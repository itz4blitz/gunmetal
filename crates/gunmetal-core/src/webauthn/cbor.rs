//! A bounded reader for the CBOR subset `WebAuthn` and COSE keys use.
//!
//! The reader accepts definite-length unsigned and negative integers at
//! every width, byte strings, UTF-8 text, arrays, maps, `true`, `false`
//! and `null`. It refuses indefinite lengths, reserved additional-info
//! values, tags, floating-point numbers, and maps with two keys that
//! decode to the same value.
//!
//! [`decode`] spends one step of the budget per data item. An input of
//! `n` octets holds at most `n` items, because each item starts with a
//! type octet. Arrays and maps each count as one nesting level against
//! [`crate::parse::LimitKind::ContainerDepth`], and their declared
//! lengths count against [`crate::parse::LimitKind::Children`]. Byte and
//! text string lengths count against [`crate::parse::LimitKind::LongText`].

use crate::parse::{
    Budget, Cursor, Depth, LimitKind, Limits, ParseFault, bounded_capacity, bounded_vec,
};
use crate::problem::{Arg, Describe, Problem, ProblemCode};

/// A decoded CBOR data item that borrows strings and byte strings from
/// the input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cbor<'a> {
    /// Major type 0: an unsigned integer.
    Unsigned(u64),
    /// Major type 1: a negative integer whose value is `-1 - n`.
    Negative(u64),
    /// Major type 2: a definite-length byte string.
    Bytes(&'a [u8]),
    /// Major type 3: a definite-length UTF-8 text string.
    Text(&'a str),
    /// Major type 4: a definite-length array.
    Array(Vec<Cbor<'a>>),
    /// Major type 5: a definite-length map, in the order the pairs
    /// appeared.
    Map(Vec<(Cbor<'a>, Cbor<'a>)>),
    /// Major type 7, additional info 20 or 21.
    Bool(bool),
    /// Major type 7, additional info 22.
    Null,
}

/// One CBOR data item and the octets that follow it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item<'a> {
    /// The item.
    pub value: Cbor<'a>,
    /// The unused suffix of the input.
    pub rest: &'a [u8],
}

/// Why `WebAuthn` CBOR, a COSE key or authenticator data could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebauthnError {
    /// A failure every parser shares: the input ended early, or the parse
    /// ran out of budget, nesting depth or a size limit.
    Fault(ParseFault),
    /// Additional info 31: an indefinite-length item, or a break.
    Indefinite {
        /// Where the type octet is.
        offset: u64,
        /// The major type of that octet.
        major: u8,
    },
    /// Additional info 28, 29 or 30, which RFC 8949 reserves.
    ReservedInfo {
        /// Where the type octet is.
        offset: u64,
        /// The major type of that octet.
        major: u8,
        /// The additional-info value.
        additional: u8,
    },
    /// A major-type-7 value this subset does not accept: undefined,
    /// unassigned simples, an 8-bit simple, or a floating-point number.
    Simple {
        /// Where the type octet is.
        offset: u64,
        /// The additional-info value.
        additional: u8,
    },
    /// A CBOR tag (major type 6).
    Tag {
        /// Where the type octet is.
        offset: u64,
        /// The tag number.
        tag: u64,
    },
    /// A map key that decodes to the same value as an earlier key.
    DuplicateKey {
        /// Where the second key starts.
        offset: u64,
    },
    /// A text string that is not UTF-8.
    Text {
        /// Where the text's type octet is.
        offset: u64,
    },
    /// Octets left after a value that must stand alone.
    Trailing {
        /// Where the extra octets start.
        offset: u64,
        /// How many extra octets remain.
        remaining: u64,
    },
    /// A COSE map field that is missing, the wrong type, or the wrong
    /// length. `label` is the COSE label: 1 (`kty`), 3 (`alg`), -1
    /// (`crv`), -2 (`x`) or -3 (`y`).
    CoseField {
        /// Where the key or the field starts.
        offset: u64,
        /// The COSE label.
        label: i64,
    },
    /// A `kty`, `alg` or `crv` this package does not accept, including
    /// RS256.
    Algorithm {
        /// Where the COSE map starts.
        offset: u64,
        /// The `kty` label's value, when it was an integer.
        kty: Option<i64>,
        /// The `alg` label's value, when it was an integer.
        alg: Option<i64>,
        /// The `crv` label's value, when it was an integer.
        crv: Option<i64>,
    },
    /// Authenticator-data flag bit 4 (backup state) set without bit 3
    /// (backup eligible), which `WebAuthn` Level 3 section 6.1.3 forbids.
    BackupState {
        /// Where the flags octet is.
        offset: u64,
        /// The flags octet.
        flags: u8,
    },
    /// A credential id length of 0 or more than 1023 (`WebAuthn` Level 3
    /// section 6.5.1).
    CredentialId {
        /// Where the length integer is.
        offset: u64,
        /// The declared length.
        length: u16,
    },
}

impl WebauthnError {
    /// Where the parser was working when it stopped, in octets from the
    /// start of the input.
    #[must_use]
    pub const fn offset(&self) -> u64 {
        match *self {
            Self::Fault(fault) => fault.offset(),
            Self::Indefinite { offset, .. }
            | Self::ReservedInfo { offset, .. }
            | Self::Simple { offset, .. }
            | Self::Tag { offset, .. }
            | Self::DuplicateKey { offset }
            | Self::Text { offset }
            | Self::Trailing { offset, .. }
            | Self::CoseField { offset, .. }
            | Self::Algorithm { offset, .. }
            | Self::BackupState { offset, .. }
            | Self::CredentialId { offset, .. } => offset,
        }
    }
}

impl From<ParseFault> for WebauthnError {
    fn from(fault: ParseFault) -> Self {
        Self::Fault(fault)
    }
}

impl Describe for WebauthnError {
    /// Unreadable passkey data, at the offset where reading stopped.
    fn problem(&self) -> Problem {
        Problem {
            code: ProblemCode::WebauthnDataUnreadable,
            args: vec![("offset", Arg::Number(self.offset()))],
        }
    }
}

/// Reads one CBOR data item from the start of `bytes` and returns it
/// together with the unused suffix.
///
/// `depth` is the nesting depth of the structure that holds this item;
/// an array or map counts as one level below it.
///
/// # Errors
///
/// Returns a typed error for truncated input, indefinite lengths,
/// reserved additional info, tags, floating-point numbers, duplicate map
/// keys, non-UTF-8 text, a nesting or size limit, or a spent budget.
pub fn decode<'a>(
    bytes: &'a [u8],
    limits: &Limits,
    budget: &mut Budget,
    depth: Depth,
) -> Result<Item<'a>, WebauthnError> {
    let mut cursor = Cursor::new(bytes);
    let value = decode_from(&mut cursor, limits, budget, depth)?;
    Ok(Item {
        value,
        rest: cursor.rest(),
    })
}

/// Refuses any octet left in `cursor` after a value that must stand alone.
pub(super) fn require_empty(cursor: &Cursor<'_>) -> Result<(), WebauthnError> {
    if cursor.is_empty() {
        Ok(())
    } else {
        Err(WebauthnError::Trailing {
            offset: cursor.offset(),
            remaining: cursor.remaining(),
        })
    }
}

/// The integer a CBOR unsigned or negative data item holds, when it fits
/// in `i64`. Major type 1 with argument `n` is `-1 - n`.
pub(super) fn integer(value: &Cbor<'_>) -> Option<i64> {
    match *value {
        Cbor::Unsigned(n) => i64::try_from(n).ok(),
        Cbor::Negative(n) => i64::try_from(n).ok().and_then(|n| (-1_i64).checked_sub(n)),
        _ => None,
    }
}

/// Reads one data item from `cursor`, reporting offsets from that cursor's
/// file position.
pub(super) fn decode_from<'a>(
    cursor: &mut Cursor<'a>,
    limits: &Limits,
    budget: &mut Budget,
    depth: Depth,
) -> Result<Cbor<'a>, WebauthnError> {
    let offset = cursor.offset();
    budget.charge(1, offset)?;
    let octet = cursor.u8()?;
    let major = octet >> 5;
    let additional = octet & 0x1F;
    match major {
        0 => read_length(cursor, major, additional, offset).map(Cbor::Unsigned),
        1 => read_length(cursor, major, additional, offset).map(Cbor::Negative),
        2 => read_bytes(cursor, limits, major, additional, offset).map(Cbor::Bytes),
        3 => read_text(cursor, limits, additional, offset),
        4 => read_array(cursor, limits, budget, depth, additional, offset),
        5 => read_map(cursor, limits, budget, depth, additional, offset),
        6 => {
            let tag = read_length(cursor, major, additional, offset)?;
            Err(WebauthnError::Tag { offset, tag })
        }
        _ => read_simple(additional, offset),
    }
}

/// Decodes the argument of a definite-length head: the integer, tag
/// number, or the length of a string, array or map.
fn read_length(
    cursor: &mut Cursor<'_>,
    major: u8,
    additional: u8,
    offset: u64,
) -> Result<u64, WebauthnError> {
    match additional {
        24 => Ok(u64::from(cursor.u8()?)),
        25 => Ok(u64::from(cursor.u16_be()?)),
        26 => Ok(u64::from(cursor.u32_be()?)),
        27 => cursor.u64_be().map_err(WebauthnError::from),
        28..=30 => Err(WebauthnError::ReservedInfo {
            offset,
            major,
            additional,
        }),
        31 => Err(WebauthnError::Indefinite { offset, major }),
        _ => Ok(u64::from(additional)),
    }
}

/// Reads a definite-length byte string.
fn read_bytes<'a>(
    cursor: &mut Cursor<'a>,
    limits: &Limits,
    major: u8,
    additional: u8,
    offset: u64,
) -> Result<&'a [u8], WebauthnError> {
    let len = read_length(cursor, major, additional, offset)?;
    limits.check(LimitKind::LongText, len, offset)?;
    Ok(cursor.take(len)?)
}

/// Reads a definite-length UTF-8 text string.
fn read_text<'a>(
    cursor: &mut Cursor<'a>,
    limits: &Limits,
    additional: u8,
    offset: u64,
) -> Result<Cbor<'a>, WebauthnError> {
    let bytes = read_bytes(cursor, limits, 3, additional, offset)?;
    match core::str::from_utf8(bytes) {
        Ok(text) => Ok(Cbor::Text(text)),
        Err(_) => Err(WebauthnError::Text { offset }),
    }
}

/// Reads a definite-length array, descending one nesting level.
fn read_array<'a>(
    cursor: &mut Cursor<'a>,
    limits: &Limits,
    budget: &mut Budget,
    depth: Depth,
    additional: u8,
    offset: u64,
) -> Result<Cbor<'a>, WebauthnError> {
    let count = read_length(cursor, 4, additional, offset)?;
    limits.check(LimitKind::Children, count, offset)?;
    let nested = depth.descend(limits, offset)?;
    let mut items = bounded_vec(
        count,
        1,
        cursor.remaining(),
        limits.get(LimitKind::Children),
    );
    for _ in 0..bounded_capacity(count, 1, u64::MAX, count) {
        items.push(decode_from(cursor, limits, budget, nested)?);
    }
    Ok(Cbor::Array(items))
}

/// Reads a definite-length map, refusing a key that equals an earlier one.
fn read_map<'a>(
    cursor: &mut Cursor<'a>,
    limits: &Limits,
    budget: &mut Budget,
    depth: Depth,
    additional: u8,
    offset: u64,
) -> Result<Cbor<'a>, WebauthnError> {
    let count = read_length(cursor, 5, additional, offset)?;
    limits.check(LimitKind::Children, count, offset)?;
    let nested = depth.descend(limits, offset)?;
    let mut entries = bounded_vec(
        count,
        2,
        cursor.remaining(),
        limits.get(LimitKind::Children),
    );
    for _ in 0..bounded_capacity(count, 1, u64::MAX, count) {
        let key_at = cursor.offset();
        let key = decode_from(cursor, limits, budget, nested)?;
        if entries.iter().any(|(seen, _)| seen == &key) {
            return Err(WebauthnError::DuplicateKey { offset: key_at });
        }
        let value = decode_from(cursor, limits, budget, nested)?;
        entries.push((key, value));
    }
    Ok(Cbor::Map(entries))
}

/// Reads a major-type-7 simple value. Only `false`, `true` and `null`
/// are accepted.
fn read_simple(additional: u8, offset: u64) -> Result<Cbor<'static>, WebauthnError> {
    match additional {
        20 => Ok(Cbor::Bool(false)),
        21 => Ok(Cbor::Bool(true)),
        22 => Ok(Cbor::Null),
        31 => Err(WebauthnError::Indefinite { offset, major: 7 }),
        28..=30 => Err(WebauthnError::ReservedInfo {
            offset,
            major: 7,
            additional,
        }),
        _ => Err(WebauthnError::Simple { offset, additional }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::collection::vec;
    use proptest::prelude::*;

    fn on_small_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(262_144)
            .spawn(work)
            .expect("the test thread starts")
            .join()
            .expect("the code under test returned instead of panicking")
    }

    fn plenty() -> Budget {
        Budget::for_input(0, 0, u64::MAX)
    }

    fn decode_at<'a>(
        bytes: &'a [u8],
        limits: &Limits,
        depth: Depth,
    ) -> Result<Item<'a>, WebauthnError> {
        decode(bytes, limits, &mut plenty(), depth)
    }

    fn decode_default(bytes: &[u8]) -> Result<Item<'_>, WebauthnError> {
        decode_at(bytes, &Limits::DEFAULT, Depth::CONTAINER_ROOT)
    }

    #[expect(
        clippy::unnecessary_wraps,
        reason = "matches decode so assert_eq compares Results"
    )]
    fn ok_value(value: Cbor<'_>) -> Result<Item<'_>, WebauthnError> {
        Ok(Item { value, rest: &[] })
    }

    fn truncated(offset: u64, needed: u64, available: u64) -> WebauthnError {
        WebauthnError::Fault(ParseFault::Truncated {
            offset,
            needed,
            available,
        })
    }

    /// Independent encoder: writes `value` with the smallest definite
    /// head RFC 8949 allows, never calling the reader.
    fn encode(value: &Cbor<'_>) -> Vec<u8> {
        let mut out = Vec::new();
        write_item(&mut out, value);
        out
    }

    fn write_item(out: &mut Vec<u8>, value: &Cbor<'_>) {
        match *value {
            Cbor::Unsigned(n) => write_head(out, 0, n),
            Cbor::Negative(n) => write_head(out, 1, n),
            Cbor::Bytes(bytes) => {
                write_head(out, 2, u64::try_from(bytes.len()).unwrap());
                out.extend_from_slice(bytes);
            }
            Cbor::Text(text) => {
                write_head(out, 3, u64::try_from(text.len()).unwrap());
                out.extend_from_slice(text.as_bytes());
            }
            Cbor::Array(ref items) => {
                write_head(out, 4, u64::try_from(items.len()).unwrap());
                for item in items {
                    write_item(out, item);
                }
            }
            Cbor::Map(ref entries) => {
                write_head(out, 5, u64::try_from(entries.len()).unwrap());
                for (key, value) in entries {
                    write_item(out, key);
                    write_item(out, value);
                }
            }
            Cbor::Bool(false) => out.push(0xF4),
            Cbor::Bool(true) => out.push(0xF5),
            Cbor::Null => out.push(0xF6),
        }
    }

    fn write_head(out: &mut Vec<u8>, major: u8, n: u64) {
        let lead = major.checked_mul(0x20).unwrap_or(0);
        if n < 24 {
            out.push(lead | u8::try_from(n).unwrap());
        } else if let Ok(byte) = u8::try_from(n) {
            out.push(lead | 0x18);
            out.push(byte);
        } else if let Ok(wide) = u16::try_from(n) {
            out.push(lead | 0x19);
            out.extend_from_slice(&wide.to_be_bytes());
        } else if let Ok(wide) = u32::try_from(n) {
            out.push(lead | 0x1A);
            out.extend_from_slice(&wide.to_be_bytes());
        } else {
            out.push(lead | 0x1B);
            out.extend_from_slice(&n.to_be_bytes());
        }
    }

    /// Encodes `n` as a major-type-0 integer using a follow-on of `width`
    /// octets (1, 2, 4 or 8), even when a shorter head would do.
    fn unsigned_at_width(n: u64, width: u8) -> Vec<u8> {
        match width {
            1 => vec![0x18, u8::try_from(n).unwrap()],
            2 => {
                let mut out = vec![0x19];
                out.extend_from_slice(&u16::try_from(n).unwrap().to_be_bytes());
                out
            }
            4 => {
                let mut out = vec![0x1A];
                out.extend_from_slice(&u32::try_from(n).unwrap().to_be_bytes());
                out
            }
            8 => {
                let mut out = vec![0x1B];
                out.extend_from_slice(&n.to_be_bytes());
                out
            }
            _ => vec![u8::try_from(n).unwrap()],
        }
    }

    /// Verifies: SEC-MED-001, SEC-HIS-036
    #[test]
    fn reads_a_zero_unsigned_integer() {
        assert_eq!(decode_default(&[0x00]), ok_value(Cbor::Unsigned(0)));
    }

    /// Verifies: SEC-MED-001
    #[test]
    fn reads_unsigned_integers_at_every_width() {
        let cases: [(u64, &[u8]); 9] = [
            (0, &[0x00]),
            (23, &[0x17]),
            (24, &[0x18, 0x18]),
            (255, &[0x18, 0xFF]),
            (256, &[0x19, 0x01, 0x00]),
            (65_535, &[0x19, 0xFF, 0xFF]),
            (65_536, &[0x1A, 0x00, 0x01, 0x00, 0x00]),
            (u64::from(u32::MAX), &[0x1A, 0xFF, 0xFF, 0xFF, 0xFF]),
            (
                u64::MAX,
                &[0x1B, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
            ),
        ];
        for (n, bytes) in cases {
            assert_eq!(
                decode_default(bytes),
                ok_value(Cbor::Unsigned(n)),
                "{bytes:02X?}"
            );
            assert_eq!(encode(&Cbor::Unsigned(n)), bytes, "encoder {n}");
        }
        // The same values encoded at a longer width still read as that integer.
        assert_eq!(
            decode_default(&unsigned_at_width(0, 1)),
            Ok(Item {
                value: Cbor::Unsigned(0),
                rest: &[],
            })
        );
        assert_eq!(
            decode_default(&unsigned_at_width(0, 2)),
            Ok(Item {
                value: Cbor::Unsigned(0),
                rest: &[],
            })
        );
        assert_eq!(
            decode_default(&unsigned_at_width(0, 4)),
            Ok(Item {
                value: Cbor::Unsigned(0),
                rest: &[],
            })
        );
        assert_eq!(
            decode_default(&unsigned_at_width(0, 8)),
            Ok(Item {
                value: Cbor::Unsigned(0),
                rest: &[],
            })
        );
        assert_eq!(
            decode_default(&unsigned_at_width(23, 1)),
            Ok(Item {
                value: Cbor::Unsigned(23),
                rest: &[],
            })
        );
        assert_eq!(
            decode_default(&unsigned_at_width(24, 2)),
            Ok(Item {
                value: Cbor::Unsigned(24),
                rest: &[],
            })
        );
        assert_eq!(
            decode_default(&unsigned_at_width(255, 4)),
            Ok(Item {
                value: Cbor::Unsigned(255),
                rest: &[],
            })
        );
        assert_eq!(
            decode_default(&unsigned_at_width(65_535, 8)),
            Ok(Item {
                value: Cbor::Unsigned(65_535),
                rest: &[],
            })
        );
    }

    /// Verifies: SEC-MED-001
    #[test]
    fn reads_negative_integers_at_every_width() {
        // Major type 1, value n, means -1 - n. 0x20 is -1, 0x37 is -24.
        let cases: [(u64, &[u8]); 6] = [
            (0, &[0x20]),
            (23, &[0x37]),
            (24, &[0x38, 0x18]),
            (255, &[0x38, 0xFF]),
            (256, &[0x39, 0x01, 0x00]),
            (
                u64::MAX,
                &[0x3B, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
            ),
        ];
        for (n, bytes) in cases {
            assert_eq!(
                decode_default(bytes),
                ok_value(Cbor::Negative(n)),
                "{bytes:02X?}"
            );
            assert_eq!(encode(&Cbor::Negative(n)), bytes, "encoder {n}");
        }
        assert_eq!(
            decode_default(&[0x3A, 0x00, 0x00, 0x00, 0x00]),
            ok_value(Cbor::Negative(0))
        );
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn reads_byte_and_text_strings() {
        assert_eq!(decode_default(&[0x40]), ok_value(Cbor::Bytes(&[])));
        assert_eq!(
            decode_default(&[0x43, b'a', b'b', b'c']),
            ok_value(Cbor::Bytes(b"abc"))
        );
        assert_eq!(
            decode_default(&[0x58, 0x01, 0xFF]),
            ok_value(Cbor::Bytes(&[0xFF]))
        );
        assert_eq!(decode_default(&[0x60]), ok_value(Cbor::Text("")));
        assert_eq!(
            decode_default(&[0x65, b'h', b'e', b'l', b'l', b'o']),
            ok_value(Cbor::Text("hello"))
        );
        assert_eq!(
            encode(&Cbor::Text("hello")),
            [0x65, b'h', b'e', b'l', b'l', b'o']
        );
        assert_eq!(
            decode_default(&[0x62, 0x80, 0x80]),
            Err(WebauthnError::Text { offset: 0 })
        );
        let limits = Limits::DEFAULT
            .with_override(LimitKind::LongText, 4)
            .expect("4 is below the ceiling");
        assert_eq!(
            decode_at(&[0x44, 1, 2, 3, 4], &limits, Depth::CONTAINER_ROOT),
            Ok(Item {
                value: Cbor::Bytes(&[1, 2, 3, 4]),
                rest: &[],
            })
        );
        assert_eq!(
            decode_at(&[0x45, 1, 2, 3, 4, 5], &limits, Depth::CONTAINER_ROOT),
            Err(WebauthnError::Fault(ParseFault::LimitExceeded {
                limit: LimitKind::LongText,
                value: 5,
                max: 4,
                offset: 0,
            }))
        );
    }

    /// Verifies: SEC-MED-001
    #[test]
    fn reads_bool_null_array_and_map() {
        assert_eq!(decode_default(&[0xF4]), ok_value(Cbor::Bool(false)));
        assert_eq!(decode_default(&[0xF5]), ok_value(Cbor::Bool(true)));
        assert_eq!(decode_default(&[0xF6]), ok_value(Cbor::Null));
        assert_eq!(decode_default(&[0x80]), ok_value(Cbor::Array(Vec::new())));
        assert_eq!(
            decode_default(&[0x83, 0x01, 0x02, 0x03]),
            ok_value(Cbor::Array(vec![
                Cbor::Unsigned(1),
                Cbor::Unsigned(2),
                Cbor::Unsigned(3)
            ]))
        );
        assert_eq!(decode_default(&[0xA0]), ok_value(Cbor::Map(Vec::new())));
        assert_eq!(
            decode_default(&[0xA1, 0x01, 0xF5]),
            ok_value(Cbor::Map(vec![(Cbor::Unsigned(1), Cbor::Bool(true))]))
        );
        assert_eq!(
            decode_default(&[0xA2, 0x01, 0x02, 0x03, 0x04]),
            ok_value(Cbor::Map(vec![
                (Cbor::Unsigned(1), Cbor::Unsigned(2)),
                (Cbor::Unsigned(3), Cbor::Unsigned(4)),
            ]))
        );
        // A one-byte length of zero still produces an empty array.
        assert_eq!(
            decode_default(&[0x98, 0x00]),
            Ok(Item {
                value: Cbor::Array(Vec::new()),
                rest: &[],
            })
        );
        assert_eq!(decode_default(&[0x58, 0x00]), ok_value(Cbor::Bytes(&[])));
        assert_eq!(
            decode_default(&[0x59, 0x00, 0x00]),
            ok_value(Cbor::Bytes(&[]))
        );
        assert_eq!(
            decode_default(&[0x5A, 0x00, 0x00, 0x00, 0x00]),
            ok_value(Cbor::Bytes(&[]))
        );
        assert_eq!(
            decode_default(&[0x5B, 0, 0, 0, 0, 0, 0, 0, 0]),
            ok_value(Cbor::Bytes(&[]))
        );
        assert_eq!(decode_default(&[0x78, 0x00]), ok_value(Cbor::Text("")));
        assert_eq!(
            decode_default(&[0x99, 0x00, 0x00]),
            ok_value(Cbor::Array(Vec::new()))
        );
        assert_eq!(
            decode_default(&[0x9A, 0x00, 0x00, 0x00, 0x00]),
            ok_value(Cbor::Array(Vec::new()))
        );
        assert_eq!(
            decode_default(&[0xB8, 0x00]),
            ok_value(Cbor::Map(Vec::new()))
        );
        assert_eq!(
            decode_default(&[0xB9, 0x00, 0x00]),
            ok_value(Cbor::Map(Vec::new()))
        );
        assert_eq!(
            decode_default(&[0x39, 0x00, 0x00]),
            ok_value(Cbor::Negative(0))
        );
    }

    /// Verifies: SEC-MED-001, SEC-HIS-036
    #[test]
    fn returns_the_unused_suffix() {
        assert_eq!(
            decode_default(&[0x01, 0x02]),
            Ok(Item {
                value: Cbor::Unsigned(1),
                rest: &[0x02],
            })
        );
        assert_eq!(
            decode_default(&[0xA1, 0x01, 0xF4, 0x00]),
            Ok(Item {
                value: Cbor::Map(vec![(Cbor::Unsigned(1), Cbor::Bool(false))]),
                rest: &[0x00],
            })
        );
    }

    /// Verifies: SEC-MED-001
    #[test]
    fn refuses_duplicate_map_keys_including_across_widths() {
        // Two inline 1 keys.
        assert_eq!(
            decode_default(&[0xA2, 0x01, 0xF4, 0x01, 0xF5]),
            Err(WebauthnError::DuplicateKey { offset: 3 })
        );
        // Inline 1 and a one-byte 1: same decoded key.
        assert_eq!(
            decode_default(&[0xA2, 0x01, 0xF4, 0x18, 0x01, 0xF5]),
            Err(WebauthnError::DuplicateKey { offset: 3 })
        );
        // Distinct keys are kept.
        assert_eq!(
            decode_default(&[0xA2, 0x01, 0xF4, 0x02, 0xF5]).map(|item| item.value),
            Ok(Cbor::Map(vec![
                (Cbor::Unsigned(1), Cbor::Bool(false)),
                (Cbor::Unsigned(2), Cbor::Bool(true)),
            ]))
        );
        assert_eq!(WebauthnError::DuplicateKey { offset: 3 }.offset(), 3);
    }

    /// Verifies: SEC-MED-001
    #[expect(
        clippy::too_many_lines,
        reason = "each octet is an independent refusal oracle"
    )]
    #[test]
    fn refuses_indefinite_lengths_reserved_info_tags_and_simples() {
        for (bytes, error) in [
            (
                &[0x1F][..],
                WebauthnError::Indefinite {
                    offset: 0,
                    major: 0,
                },
            ),
            (
                &[0x5F],
                WebauthnError::Indefinite {
                    offset: 0,
                    major: 2,
                },
            ),
            (
                &[0x7F],
                WebauthnError::Indefinite {
                    offset: 0,
                    major: 3,
                },
            ),
            (
                &[0x9F],
                WebauthnError::Indefinite {
                    offset: 0,
                    major: 4,
                },
            ),
            (
                &[0xBF],
                WebauthnError::Indefinite {
                    offset: 0,
                    major: 5,
                },
            ),
            (
                &[0xFF],
                WebauthnError::Indefinite {
                    offset: 0,
                    major: 7,
                },
            ),
            (
                &[0x3F],
                WebauthnError::Indefinite {
                    offset: 0,
                    major: 1,
                },
            ),
            (
                &[0xDF],
                WebauthnError::Indefinite {
                    offset: 0,
                    major: 6,
                },
            ),
            (
                &[0x3C],
                WebauthnError::ReservedInfo {
                    offset: 0,
                    major: 1,
                    additional: 28,
                },
            ),
            (
                &[0x5C],
                WebauthnError::ReservedInfo {
                    offset: 0,
                    major: 2,
                    additional: 28,
                },
            ),
            (
                &[0x7C],
                WebauthnError::ReservedInfo {
                    offset: 0,
                    major: 3,
                    additional: 28,
                },
            ),
            (
                &[0x9C],
                WebauthnError::ReservedInfo {
                    offset: 0,
                    major: 4,
                    additional: 28,
                },
            ),
            (
                &[0xBC],
                WebauthnError::ReservedInfo {
                    offset: 0,
                    major: 5,
                    additional: 28,
                },
            ),
            (
                &[0xDC],
                WebauthnError::ReservedInfo {
                    offset: 0,
                    major: 6,
                    additional: 28,
                },
            ),
            (
                &[0x1C],
                WebauthnError::ReservedInfo {
                    offset: 0,
                    major: 0,
                    additional: 28,
                },
            ),
            (
                &[0x1D],
                WebauthnError::ReservedInfo {
                    offset: 0,
                    major: 0,
                    additional: 29,
                },
            ),
            (
                &[0x1E],
                WebauthnError::ReservedInfo {
                    offset: 0,
                    major: 0,
                    additional: 30,
                },
            ),
            (
                &[0xFC],
                WebauthnError::ReservedInfo {
                    offset: 0,
                    major: 7,
                    additional: 28,
                },
            ),
            (
                &[0xF7],
                WebauthnError::Simple {
                    offset: 0,
                    additional: 23,
                },
            ),
            (
                &[0xF0],
                WebauthnError::Simple {
                    offset: 0,
                    additional: 16,
                },
            ),
            (
                &[0xF8],
                WebauthnError::Simple {
                    offset: 0,
                    additional: 24,
                },
            ),
            (
                &[0xF9],
                WebauthnError::Simple {
                    offset: 0,
                    additional: 25,
                },
            ),
            (
                &[0xFA],
                WebauthnError::Simple {
                    offset: 0,
                    additional: 26,
                },
            ),
            (
                &[0xFB],
                WebauthnError::Simple {
                    offset: 0,
                    additional: 27,
                },
            ),
            (&[0xC0], WebauthnError::Tag { offset: 0, tag: 0 }),
            (&[0xC1], WebauthnError::Tag { offset: 0, tag: 1 }),
            (&[0xD8, 0x2A], WebauthnError::Tag { offset: 0, tag: 42 }),
        ] {
            assert_eq!(decode_default(bytes), Err(error), "{bytes:02X?}");
            assert_eq!(error.offset(), 0, "{error:?}");
        }
    }

    /// Verifies: SEC-MED-001, SEC-MED-004
    #[test]
    fn reports_truncation_with_exact_offsets() {
        assert_eq!(decode_default(&[]), Err(truncated(0, 1, 0)));
        assert_eq!(decode_default(&[0x18]), Err(truncated(1, 1, 0)));
        assert_eq!(decode_default(&[0x19, 0x00]), Err(truncated(1, 2, 1)));
        assert_eq!(
            decode_default(&[0x1A, 0x00, 0x00, 0x00]),
            Err(truncated(1, 4, 3))
        );
        assert_eq!(
            decode_default(&[0x1B, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
            Err(truncated(1, 8, 7))
        );
        assert_eq!(decode_default(&[0x42, 0x00]), Err(truncated(1, 2, 1)));
        assert_eq!(decode_default(&[0x81]), Err(truncated(1, 1, 0)));
        assert_eq!(decode_default(&[0xA1, 0x01]), Err(truncated(2, 1, 0)));
        assert_eq!(
            WebauthnError::Fault(ParseFault::BudgetExceeded { offset: 9 }).offset(),
            9
        );
    }

    /// Verifies: SEC-MED-005
    #[test]
    fn refuses_nesting_past_the_depth_limit() {
        // Three nested arrays around 0: 0x81 0x81 0x81 0x00. Allowed when
        // the container depth limit is 3, refused when it is 2.
        let nested = [0x81, 0x81, 0x81, 0x00];
        let two = Limits::DEFAULT
            .with_override(LimitKind::ContainerDepth, 2)
            .expect("2 is below the ceiling");
        assert_eq!(
            decode_at(&nested, &two, Depth::CONTAINER_ROOT),
            Err(WebauthnError::Fault(ParseFault::TooDeep {
                limit: LimitKind::ContainerDepth,
                depth: 3,
                max: 2,
                offset: 2,
            }))
        );
        let three = Limits::DEFAULT
            .with_override(LimitKind::ContainerDepth, 3)
            .expect("3 is below the ceiling");
        assert_eq!(
            decode_at(&nested, &three, Depth::CONTAINER_ROOT).map(|item| item.value),
            Ok(Cbor::Array(vec![Cbor::Array(vec![Cbor::Array(vec![
                Cbor::Unsigned(0)
            ])])]))
        );
        // A primitive at the root does not consume a nesting level.
        assert_eq!(
            decode_at(&[0x00], &two, Depth::CONTAINER_ROOT).map(|item| item.value),
            Ok(Cbor::Unsigned(0))
        );
        // An empty array still counts as one level.
        let none = Limits::DEFAULT
            .with_override(LimitKind::ContainerDepth, 0)
            .expect("0 is below the ceiling");
        assert_eq!(
            decode_at(&[0x80], &none, Depth::CONTAINER_ROOT),
            Err(WebauthnError::Fault(ParseFault::TooDeep {
                limit: LimitKind::ContainerDepth,
                depth: 1,
                max: 0,
                offset: 0,
            }))
        );
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn refuses_an_array_or_map_past_the_children_limit() {
        let limits = Limits::DEFAULT
            .with_override(LimitKind::Children, 1)
            .expect("1 is below the ceiling");
        assert_eq!(
            decode_at(&[0x81, 0x00], &limits, Depth::CONTAINER_ROOT).map(|item| item.value),
            Ok(Cbor::Array(vec![Cbor::Unsigned(0)]))
        );
        assert_eq!(
            decode_at(&[0x82, 0x00, 0x01], &limits, Depth::CONTAINER_ROOT),
            Err(WebauthnError::Fault(ParseFault::LimitExceeded {
                limit: LimitKind::Children,
                value: 2,
                max: 1,
                offset: 0,
            }))
        );
        assert_eq!(
            decode_at(&[0xA1, 0x01, 0x02], &limits, Depth::CONTAINER_ROOT).map(|item| item.value),
            Ok(Cbor::Map(vec![(Cbor::Unsigned(1), Cbor::Unsigned(2))]))
        );
        assert_eq!(
            decode_at(
                &[0xA2, 0x01, 0x02, 0x03, 0x04],
                &limits,
                Depth::CONTAINER_ROOT
            ),
            Err(WebauthnError::Fault(ParseFault::LimitExceeded {
                limit: LimitKind::Children,
                value: 2,
                max: 1,
                offset: 0,
            }))
        );
        // An empty array is allowed when the limit is zero.
        let none = Limits::DEFAULT
            .with_override(LimitKind::Children, 0)
            .expect("0 is below the ceiling");
        assert_eq!(
            decode_at(&[0x80], &none, Depth::CONTAINER_ROOT).map(|item| item.value),
            Ok(Cbor::Array(Vec::new()))
        );
        assert_eq!(
            decode_at(&[0x81, 0x00], &none, Depth::CONTAINER_ROOT),
            Err(WebauthnError::Fault(ParseFault::LimitExceeded {
                limit: LimitKind::Children,
                value: 1,
                max: 0,
                offset: 0,
            }))
        );
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn spends_one_step_per_data_item_and_stops_when_the_budget_runs_out() {
        let mut budget = Budget::for_input(0, 0, 1);
        assert_eq!(
            decode(
                &[0x00],
                &Limits::DEFAULT,
                &mut budget,
                Depth::CONTAINER_ROOT
            )
            .map(|item| item.value),
            Ok(Cbor::Unsigned(0))
        );
        assert_eq!(budget.remaining(), 0);
        let mut budget = Budget::for_input(0, 0, 1);
        assert_eq!(
            decode(
                &[0x81, 0x00],
                &Limits::DEFAULT,
                &mut budget,
                Depth::CONTAINER_ROOT
            ),
            Err(WebauthnError::Fault(ParseFault::BudgetExceeded {
                offset: 1
            }))
        );
        let mut budget = Budget::for_input(0, 0, 0);
        assert_eq!(
            decode(
                &[0x00],
                &Limits::DEFAULT,
                &mut budget,
                Depth::CONTAINER_ROOT
            ),
            Err(WebauthnError::Fault(ParseFault::BudgetExceeded {
                offset: 0
            }))
        );
        // A map of one pair is three items: the map, the key and the value.
        let mut budget = Budget::for_input(0, 0, 3);
        assert_eq!(
            decode(
                &[0xA1, 0x01, 0x02],
                &Limits::DEFAULT,
                &mut budget,
                Depth::CONTAINER_ROOT
            )
            .map(|item| item.value),
            Ok(Cbor::Map(vec![(Cbor::Unsigned(1), Cbor::Unsigned(2))]))
        );
        assert_eq!(budget.remaining(), 0);
    }

    /// Verifies: SEC-MED-008
    #[test]
    fn a_nested_walk_always_stops() {
        // Thirty-three nested empty arrays would hang a reader that did
        // not consume the type octet of each array. The depth limit stops
        // the walk first.
        let bytes: Vec<u8> = (0..33)
            .map(|_| 0x81)
            .chain(core::iter::once(0x80))
            .collect();
        let error = decode_default(&bytes);
        assert_eq!(
            error,
            Err(WebauthnError::Fault(ParseFault::TooDeep {
                limit: LimitKind::ContainerDepth,
                depth: 33,
                max: 32,
                offset: 32,
            }))
        );
    }

    /// Verifies: SEC-API-072
    #[test]
    fn describes_every_variant_as_the_passkey_problem() {
        let errors: [(WebauthnError, u64); 12] = [
            (truncated(1, 2, 0), 1),
            (
                WebauthnError::Indefinite {
                    offset: 2,
                    major: 4,
                },
                2,
            ),
            (
                WebauthnError::ReservedInfo {
                    offset: 3,
                    major: 0,
                    additional: 28,
                },
                3,
            ),
            (
                WebauthnError::Simple {
                    offset: 4,
                    additional: 25,
                },
                4,
            ),
            (WebauthnError::Tag { offset: 5, tag: 0 }, 5),
            (WebauthnError::DuplicateKey { offset: 6 }, 6),
            (WebauthnError::Text { offset: 7 }, 7),
            (
                WebauthnError::Trailing {
                    offset: 8,
                    remaining: 1,
                },
                8,
            ),
            (
                WebauthnError::CoseField {
                    offset: 9,
                    label: 1,
                },
                9,
            ),
            (
                WebauthnError::Algorithm {
                    offset: 10,
                    kty: Some(3),
                    alg: Some(-257),
                    crv: None,
                },
                10,
            ),
            (
                WebauthnError::BackupState {
                    offset: 11,
                    flags: 0x10,
                },
                11,
            ),
            (
                WebauthnError::CredentialId {
                    offset: 12,
                    length: 0,
                },
                12,
            ),
        ];
        for (error, offset) in errors {
            assert_eq!(error.offset(), offset, "{error:?}");
            assert_eq!(
                error.problem(),
                Problem {
                    code: ProblemCode::WebauthnDataUnreadable,
                    args: vec![("offset", Arg::Number(offset))],
                },
                "{error:?}"
            );
        }
        assert_eq!(
            WebauthnError::from(ParseFault::BudgetExceeded { offset: 4 }),
            WebauthnError::Fault(ParseFault::BudgetExceeded { offset: 4 })
        );
    }

    #[test]
    fn reads_an_integer_that_fits_in_i64_and_drops_the_rest() {
        assert_eq!(integer(&Cbor::Unsigned(0)), Some(0));
        assert_eq!(integer(&Cbor::Unsigned(24)), Some(24));
        let max = u64::try_from(i64::MAX).expect("i64::MAX fits in u64");
        assert_eq!(integer(&Cbor::Unsigned(max)), Some(i64::MAX));
        assert_eq!(integer(&Cbor::Unsigned(max.checked_add(1).unwrap())), None);
        assert_eq!(integer(&Cbor::Negative(0)), Some(-1));
        assert_eq!(integer(&Cbor::Negative(6)), Some(-7));
        assert_eq!(integer(&Cbor::Negative(max)), Some(i64::MIN));
        assert_eq!(integer(&Cbor::Negative(max.checked_add(1).unwrap())), None);
        assert_eq!(integer(&Cbor::Bytes(&[])), None);
        assert_eq!(integer(&Cbor::Text("")), None);
        assert_eq!(integer(&Cbor::Array(Vec::new())), None);
        assert_eq!(integer(&Cbor::Map(Vec::new())), None);
        assert_eq!(integer(&Cbor::Bool(false)), None);
        assert_eq!(integer(&Cbor::Bool(true)), None);
        assert_eq!(integer(&Cbor::Null), None);
    }

    /// A small owned tree the property test generates, independent of
    /// [`Cbor`].
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Tree {
        Unsigned(u64),
        Negative(u64),
        Bytes(Vec<u8>),
        Text(String),
        Array(Vec<Tree>),
        Map(Vec<(Tree, Tree)>),
        Bool(bool),
        Null,
    }

    fn tree(depth: u32) -> impl Strategy<Value = Tree> {
        let leaf = prop_oneof![
            any::<u64>().prop_map(Tree::Unsigned),
            any::<u64>().prop_map(Tree::Negative),
            vec(any::<u8>(), 0..8).prop_map(Tree::Bytes),
            "[a-z]{0,8}".prop_map(Tree::Text),
            any::<bool>().prop_map(Tree::Bool),
            Just(Tree::Null),
        ];
        leaf.prop_recursive(depth, 16, 4, |inner| {
            prop_oneof![
                vec(inner.clone(), 0..3).prop_map(Tree::Array),
                vec((inner.clone(), inner), 0..3).prop_map(|pairs| {
                    let mut keys = Vec::new();
                    let mut unique = Vec::new();
                    for (key, value) in pairs {
                        if !keys.iter().any(|seen| encoded(seen) == encoded(&key)) {
                            keys.push(key.clone());
                            unique.push((key, value));
                        }
                    }
                    Tree::Map(unique)
                }),
            ]
        })
    }

    fn encoded(tree: &Tree) -> Vec<u8> {
        let mut out = Vec::new();
        write_tree(&mut out, tree);
        out
    }

    fn write_tree(out: &mut Vec<u8>, tree: &Tree) {
        match *tree {
            Tree::Unsigned(n) => write_head(out, 0, n),
            Tree::Negative(n) => write_head(out, 1, n),
            Tree::Bytes(ref bytes) => {
                write_head(out, 2, u64::try_from(bytes.len()).unwrap());
                out.extend_from_slice(bytes);
            }
            Tree::Text(ref text) => {
                write_head(out, 3, u64::try_from(text.len()).unwrap());
                out.extend_from_slice(text.as_bytes());
            }
            Tree::Array(ref items) => {
                write_head(out, 4, u64::try_from(items.len()).unwrap());
                for item in items {
                    write_tree(out, item);
                }
            }
            Tree::Map(ref entries) => {
                write_head(out, 5, u64::try_from(entries.len()).unwrap());
                for (key, value) in entries {
                    write_tree(out, key);
                    write_tree(out, value);
                }
            }
            Tree::Bool(false) => out.push(0xF4),
            Tree::Bool(true) => out.push(0xF5),
            Tree::Null => out.push(0xF6),
        }
    }

    fn as_cbor(tree: &Tree) -> Cbor<'_> {
        match *tree {
            Tree::Unsigned(n) => Cbor::Unsigned(n),
            Tree::Negative(n) => Cbor::Negative(n),
            Tree::Bytes(ref bytes) => Cbor::Bytes(bytes),
            Tree::Text(ref text) => Cbor::Text(text),
            Tree::Array(ref items) => Cbor::Array(items.iter().map(as_cbor).collect()),
            Tree::Map(ref entries) => Cbor::Map(
                entries
                    .iter()
                    .map(|(key, value)| (as_cbor(key), as_cbor(value)))
                    .collect(),
            ),
            Tree::Bool(value) => Cbor::Bool(value),
            Tree::Null => Cbor::Null,
        }
    }

    proptest! {
        /// Verifies: SEC-MED-001, SEC-HIS-036
        #[test]
        fn never_panics_on_any_input(bytes in vec(any::<u8>(), 0..64)) {
            on_small_stack(move || {
                let _ = decode_default(&bytes);
            });
        }

        /// Verifies: SEC-MED-001, SEC-MED-008
        #[test]
        fn reading_an_encoded_tree_returns_it(
            value in tree(3),
            trailer in vec(any::<u8>(), 0..4),
        ) {
            let mut bytes = encoded(&value);
            let item_len = bytes.len();
            bytes.extend_from_slice(&trailer);
            let got = decode_default(&bytes);
            prop_assert_eq!(
                got,
                Ok(Item {
                    value: as_cbor(&value),
                    rest: &bytes[item_len..],
                })
            );
        }
    }
}
