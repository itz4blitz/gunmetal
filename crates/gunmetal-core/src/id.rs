//! Public identifiers: the IDs that leave the server.
//!
//! An identifier is a kind prefix, an underscore and 26 lower-case Crockford
//! base32 symbols that carry 16 random bytes, for example
//! `trk_0123456789abcdefghjkmnpqrs` (SEC-API-023, SEC-PRV-021). The symbols
//! write the bytes as one big-endian 128-bit number, so the first symbol is
//! `0` to `7`; this is the suffix encoding of the `TypeID` specification.
//! Exactly one spelling of each identifier parses: no upper case, none of
//! the letters `i`, `l`, `o` and `u`, and no padding or whitespace.
//!
//! The bytes come only from the operating system's CSPRNG. The secrets
//! crate's minting function (WP-047) is the one caller of
//! [`Minted::from_os_random`], and a [`PublicId`] is built only from a
//! [`Minted`] value or by parsing an identifier a client sent back, so no
//! code can make one from a path, a name, a counter or a hash
//! (SEC-HIS-012). Minting and storing identifiers is not this module's
//! job: the identity store keeps the public-ID mapping (WP-046).

use core::fmt;

use crate::problem::{Describe, Problem, ProblemCode};

/// What an identifier names. Each kind has its own prefix, so an identifier
/// of one kind never parses as another (SEC-API-024).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdKind {
    /// A track: `trk_`.
    Track,
    /// An album: `alb_`.
    Album,
    /// An artist: `art_`.
    Artist,
    /// A release group: `rgp_`.
    ReleaseGroup,
    /// A playlist: `pls_`.
    Playlist,
    /// A user account: `usr_`.
    User,
    /// A profile: `prf_`.
    Profile,
    /// A device: `dev_`.
    Device,
    /// A library: `lib_`.
    Library,
    /// An invitation: `inv_`.
    Invite,
    /// A share link: `shr_`.
    Share,
    /// A token: `tok_`.
    Token,
}

impl IdKind {
    /// The text an identifier of this kind starts with: three lower-case
    /// letters and an underscore.
    const fn prefix(self) -> &'static str {
        match self {
            Self::Track => "trk_",
            Self::Album => "alb_",
            Self::Artist => "art_",
            Self::ReleaseGroup => "rgp_",
            Self::Playlist => "pls_",
            Self::User => "usr_",
            Self::Profile => "prf_",
            Self::Device => "dev_",
            Self::Library => "lib_",
            Self::Invite => "inv_",
            Self::Share => "shr_",
            Self::Token => "tok_",
        }
    }
}

/// Sixteen bytes freshly drawn from the operating system's CSPRNG, ready to
/// become one public identifier.
///
/// The secrets crate's minting function (WP-047) is the only caller of
/// [`Minted::from_os_random`]; a `disallowed-methods` entry in `clippy.toml`
/// (WP-001) rejects a call anywhere else. A `Minted` value cannot be copied,
/// so one draw makes one identifier.
pub struct Minted([u8; 16]);

impl Minted {
    /// Wraps bytes the caller has just drawn from the operating system's
    /// CSPRNG. Only the minting function may call this.
    #[must_use]
    pub const fn from_os_random(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
}

/// An identifier that leaves the server: its kind and 16 random bytes.
///
/// Its [`Display`](fmt::Display) form is the only text form, and
/// [`PublicId::parse`] accepts nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PublicId {
    kind: IdKind,
    bytes: [u8; 16],
}

/// How many symbols write the 16 bytes: 130 bits, of which the first two
/// are always zero.
const SYMBOLS: usize = 26;

/// Crockford's base32 symbols in lower case: the digits, then the letters
/// without `i`, `l`, `o` and `u`.
const ALPHABET: [u8; 32] = *b"0123456789abcdefghjkmnpqrstvwxyz";

impl PublicId {
    /// The identifier of kind `kind` that a [`Minted`] value makes. It takes
    /// the value, so the same draw cannot make a second identifier.
    #[must_use]
    pub const fn new(kind: IdKind, Minted(bytes): Minted) -> Self {
        Self { kind, bytes }
    }

    /// Reads an identifier a client sent back, which must be of the kind
    /// `expected`.
    ///
    /// # Errors
    ///
    /// Returns an [`IdError`] unless `text` is exactly the canonical
    /// spelling of an identifier of kind `expected`. An identifier of another
    /// kind gets the same error as a malformed one, so the server answers
    /// both with its not-found response (SEC-API-024).
    pub fn parse(text: &str, expected: IdKind) -> Result<Self, IdError> {
        text.strip_prefix(expected.prefix())
            .and_then(|symbols| decode(symbols.as_bytes()))
            .map(|bytes| Self {
                kind: expected,
                bytes,
            })
            .ok_or(IdError { expected })
    }
}

impl fmt::Display for PublicId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut text = String::from(self.kind.prefix());
        text.extend(encode(self.bytes).map(char::from));
        f.write_str(&text)
    }
}

/// Writes 16 bytes as 26 symbols, most significant first.
fn encode(bytes: [u8; 16]) -> [u8; SYMBOLS] {
    let mut rest = u128::from_be_bytes(bytes);
    let mut symbols = [0; SYMBOLS];
    for symbol in symbols.iter_mut().rev() {
        let [low, ..] = rest.to_le_bytes();
        *symbol = symbol_for(low);
        rest = rest.wrapping_shr(5);
    }
    symbols
}

/// The symbol for the low five bits of `bits`.
#[expect(
    clippy::indexing_slicing,
    reason = "five bits index the 32-symbol alphabet"
)]
fn symbol_for(bits: u8) -> u8 {
    ALPHABET[usize::from(bits & 0x1F)]
}

/// Reads 26 symbols as one big-endian number. Returns `None` unless there
/// are exactly 26, each is in the alphabet, and the number fits in 128 bits,
/// which holds exactly when the first symbol is `0` to `7`.
fn decode(symbols: &[u8]) -> Option<[u8; 16]> {
    let symbols: &[u8; SYMBOLS] = symbols.try_into().ok()?;
    let mut value: u128 = 0;
    for &symbol in symbols {
        let digit = (0_u8..)
            .zip(ALPHABET)
            .find_map(|(digit, candidate)| (candidate == symbol).then_some(digit))?;
        // Only the last multiplication can overflow, and only when the first
        // symbol is above 7. It leaves the low five bits zero, so adding a
        // digit below 32 cannot wrap.
        value = value.checked_mul(32)?.wrapping_add(u128::from(digit));
    }
    Some(value.to_be_bytes())
}

/// Why text a client sent back is not an identifier of the kind expected.
///
/// A wrong kind, a wrong length, a symbol outside the alphabet and a
/// non-canonical spelling are deliberately one error with no detail, so no
/// caller can answer them differently (SEC-API-024).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdError {
    /// The kind the caller asked for.
    pub expected: IdKind,
}

impl Describe for IdError {
    /// Not found, exactly like an object that does not exist (SEC-API-024).
    fn problem(&self) -> Problem {
        Problem {
            code: ProblemCode::NotFound,
            args: Vec::new(),
        }
    }
}

/// Compile-fail tests: what code outside the core must not be able to do
/// with an identifier. Rustdoc on stable does not check which error a
/// compile-fail test produced, so each shares its imports with the control
/// below, which compiles; a mistake in the imports would fail the control.
#[cfg(doctest)]
mod compile_fail {
    /// Control: outside the core, a public ID comes from minting or parsing.
    ///
    /// ```
    /// use gunmetal_core::id::{IdKind, Minted, PublicId};
    ///
    /// fn minting(minted: Minted) -> PublicId {
    ///     PublicId::new(IdKind::Track, minted)
    /// }
    ///
    /// fn parsing(text: &str) -> Option<PublicId> {
    ///     PublicId::parse(text, IdKind::Track).ok()
    /// }
    /// ```
    struct Control;

    /// Verifies: SEC-HIS-012, SEC-PRV-021
    ///
    /// No constructor takes raw bytes, so an identifier cannot be built from
    /// a path hash, a name or zeros.
    ///
    /// ```compile_fail
    /// use gunmetal_core::id::{IdKind, Minted, PublicId};
    ///
    /// fn from_bytes(bytes: [u8; 16]) -> PublicId {
    ///     PublicId::new(IdKind::Track, bytes)
    /// }
    /// ```
    struct NoConstructorTakesBytes;

    /// Verifies: SEC-HIS-012
    ///
    /// The fields are private, so a struct literal cannot build one either.
    ///
    /// ```compile_fail
    /// use gunmetal_core::id::{IdKind, Minted, PublicId};
    ///
    /// fn literal(bytes: [u8; 16]) -> PublicId {
    ///     PublicId { kind: IdKind::Track, bytes }
    /// }
    /// ```
    struct NoStructLiteral;

    /// Verifies: SEC-HIS-012
    ///
    /// A `Minted` value comes only from its one constructor, which only the
    /// minting function may call.
    ///
    /// ```compile_fail
    /// use gunmetal_core::id::{IdKind, Minted, PublicId};
    ///
    /// fn minted(bytes: [u8; 16]) -> Minted {
    ///     Minted(bytes)
    /// }
    /// ```
    struct NoMintedLiteral;

    /// Verifies: SEC-API-023
    ///
    /// No identifier type wraps an integer: none converts from one.
    ///
    /// ```compile_fail
    /// use gunmetal_core::id::{IdKind, Minted, PublicId};
    ///
    /// fn from_integer(sequence: u128) -> PublicId {
    ///     PublicId::from(sequence)
    /// }
    /// ```
    struct NoIntegerConversion;
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    // Qodana does not expand `proptest!` or resolve `prop_oneof!` through `prelude::*`.
    use proptest::prop_oneof;
    use proptest::sample::select;
    use proptest::test_runner::{Config, TestRunner};

    /// Every kind with its prefix, written out independently of the code
    /// under test.
    const KINDS: [(IdKind, &str); 12] = [
        (IdKind::Track, "trk_"),
        (IdKind::Album, "alb_"),
        (IdKind::Artist, "art_"),
        (IdKind::ReleaseGroup, "rgp_"),
        (IdKind::Playlist, "pls_"),
        (IdKind::User, "usr_"),
        (IdKind::Profile, "prf_"),
        (IdKind::Device, "dev_"),
        (IdKind::Library, "lib_"),
        (IdKind::Invite, "inv_"),
        (IdKind::Share, "shr_"),
        (IdKind::Token, "tok_"),
    ];

    /// Crockford's base32 symbols in lower case, written out for the
    /// reference codec below.
    const REFERENCE_SYMBOLS: &str = "0123456789abcdefghjkmnpqrstvwxyz";

    /// Stands in for the secrets crate's minting function (WP-047), which is
    /// the one caller of [`Minted::from_os_random`] outside tests.
    #[expect(
        clippy::disallowed_methods,
        reason = "tests stand in for the minting function"
    )]
    fn minted(bytes: [u8; 16]) -> Minted {
        Minted::from_os_random(bytes)
    }

    fn id(kind: IdKind, bytes: [u8; 16]) -> PublicId {
        PublicId::new(kind, minted(bytes))
    }

    /// Reference encoder used only as a test oracle; it shares no code with
    /// the encoder under test. It writes two zero bits and then the 128 bits
    /// of the bytes, most significant first, as 26 groups of five.
    fn reference_text(prefix: &str, bytes: [u8; 16]) -> String {
        let bits: Vec<u8> = [0, 0]
            .into_iter()
            .chain(
                bytes
                    .iter()
                    .flat_map(|byte| (0..8).rev().map(move |place| (byte >> place) & 1)),
            )
            .collect();
        let symbols: String = bits
            .chunks(5)
            .map(|group| {
                let value = group
                    .iter()
                    .fold(0, |value, bit| value * 2 + usize::from(*bit));
                REFERENCE_SYMBOLS
                    .chars()
                    .nth(value)
                    .expect("five bits index 32 symbols")
            })
            .collect();
        format!("{prefix}{symbols}")
    }

    /// Reference decoder used only as a test oracle: the bytes that 26
    /// canonical symbols spell, read back as bits.
    fn reference_bytes(symbols: &str) -> [u8; 16] {
        let bits: Vec<u8> = symbols
            .chars()
            .map(|symbol| REFERENCE_SYMBOLS.find(symbol).expect("a canonical symbol"))
            .flat_map(|value| {
                (0..5)
                    .rev()
                    .map(move |place| u8::from(value >> place & 1 == 1))
            })
            .collect();
        assert_eq!(&bits[..2], [0, 0], "{symbols} holds more than 128 bits");
        let bytes: Vec<u8> = bits[2..]
            .chunks(8)
            .map(|octet| octet.iter().fold(0, |byte, bit| byte * 2 + bit))
            .collect();
        bytes.try_into().expect("128 bits make 16 bytes")
    }

    /// Picks any kind together with its prefix.
    fn any_kind() -> impl Strategy<Value = (IdKind, &'static str)> {
        select(KINDS.to_vec())
    }

    /// Verifies: SEC-API-024
    #[test]
    fn writes_each_kind_with_its_own_prefix() {
        for (kind, prefix) in KINDS {
            assert_eq!(
                id(kind, [0; 16]).to_string(),
                format!("{prefix}00000000000000000000000000")
            );
        }
    }

    /// Verifies: SEC-API-023, SEC-PRV-021
    #[test]
    fn writes_and_reads_known_byte_patterns_as_literal_text() {
        let cases: [([u8; 16], &str); 6] = [
            ([0x00; 16], "trk_00000000000000000000000000"),
            ([0xFF; 16], "trk_7zzzzzzzzzzzzzzzzzzzzzzzzz"),
            // The TypeID specification's example, which uses 26 of the 32
            // symbols.
            (
                [
                    0x01, 0x10, 0xC8, 0x53, 0x1D, 0x09, 0x52, 0xD8, 0xD7, 0x3E, 0x11, 0x94, 0xE9,
                    0x5B, 0x5F, 0x19,
                ],
                "trk_0123456789abcdefghjkmnpqrs",
            ),
            // The other six.
            (
                [
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x35,
                    0xBE, 0x77, 0xDF,
                ],
                "trk_00000000000000000000tvwxyz",
            ),
            (
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
                "trk_00000000000000000000000001",
            ),
            (
                [0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                "trk_40000000000000000000000000",
            ),
        ];
        for (bytes, text) in cases {
            assert_eq!(id(IdKind::Track, bytes).to_string(), text);
            assert_eq!(
                PublicId::parse(text, IdKind::Track),
                Ok(id(IdKind::Track, bytes)),
                "{text}"
            );
        }
    }

    /// Verifies: SEC-API-023
    #[test]
    fn refuses_every_spelling_but_the_canonical_one() {
        let refused = [
            "",
            "trk_",
            "00000000000000000000000000",
            "trk00000000000000000000000000",
            "trk-00000000000000000000000000",
            "TRK_00000000000000000000000000",
            // 25 and 27 symbols.
            "trk_0000000000000000000000000",
            "trk_000000000000000000000000000",
            // Upper case.
            "trk_7ZZZZZZZZZZZZZZZZZZZZZZZZZ",
            "trk_0000000000000000000000000A",
            // The letters Crockford's alphabet leaves out.
            "trk_0000000000000000000000000i",
            "trk_0000000000000000000000000l",
            "trk_0000000000000000000000000o",
            "trk_0000000000000000000000000u",
            // A first symbol above 7 spells more than 128 bits.
            "trk_80000000000000000000000000",
            "trk_z0000000000000000000000000",
            // Whitespace and padding.
            "trk_00000000000000000000000000\n",
            " trk_00000000000000000000000000",
            "trk_00000000000000000000000000 ",
            "trk_0000000000000000000000000=",
            // 26 bytes, but not 26 symbols.
            "trk_000000000000000000000000é",
        ];
        for text in refused {
            assert_eq!(
                PublicId::parse(text, IdKind::Track),
                Err(IdError {
                    expected: IdKind::Track
                }),
                "{text:?}"
            );
        }
    }

    /// Verifies: SEC-API-024
    #[test]
    fn refuses_an_identifier_of_another_kind_exactly_like_a_malformed_one() {
        for (expected, _) in KINDS {
            let malformed = PublicId::parse("not an identifier", expected);
            assert_eq!(malformed, Err(IdError { expected }));
            for (other, _) in KINDS.into_iter().filter(|(kind, _)| *kind != expected) {
                let text = id(other, [0x5A; 16]).to_string();
                assert_eq!(
                    PublicId::parse(&text, expected),
                    malformed,
                    "{text} as {expected:?}"
                );
            }
        }
    }

    /// Verifies: SEC-API-024, SEC-API-072
    #[test]
    fn describes_an_unparseable_identifier_as_not_found() {
        for (expected, _) in KINDS {
            assert_eq!(
                IdError { expected }.problem(),
                Problem {
                    code: ProblemCode::NotFound,
                    args: vec![],
                }
            );
        }
    }

    /// Verifies: SEC-API-023, SEC-PRV-021
    #[test]
    fn writes_what_the_reference_encoder_writes() {
        TestRunner::new(Config::default())
            .run(
                &(any_kind(), any::<[u8; 16]>()),
                |((kind, prefix), bytes)| {
                    prop_assert_eq!(id(kind, bytes).to_string(), reference_text(prefix, bytes));
                    Ok(())
                },
            )
            .unwrap();
    }

    /// Verifies: SEC-API-023, SEC-PRV-021
    #[test]
    fn reads_back_every_identifier_it_writes() {
        TestRunner::new(Config::default())
            .run(&(any_kind(), any::<[u8; 16]>()), |((kind, _), bytes)| {
                let written = id(kind, bytes);
                prop_assert_eq!(PublicId::parse(&written.to_string(), kind), Ok(written));
                Ok(())
            })
            .unwrap();
    }

    /// Verifies: SEC-API-023
    #[test]
    fn accepts_any_text_only_in_its_canonical_spelling() {
        TestRunner::new(Config::default())
            .run(&(any_kind(), "(?s).{0,40}"), |((kind, _), text)| {
                if let Ok(parsed) = PublicId::parse(&text, kind) {
                    prop_assert_eq!(parsed.to_string(), text);
                }
                Ok(())
            })
            .unwrap();
    }

    /// Verifies: SEC-API-023, SEC-API-024
    #[test]
    fn accepts_a_near_miss_only_when_it_is_canonical() {
        // Random letters and digits rarely spell a canonical identifier,
        // so force the canonical alphabet half of the time.
        TestRunner::new(Config::default())
            .run(
                &(
                    any_kind(),
                    prop_oneof!["[0-9a-zA-Z]{26}", "[0-7][0-9a-hjkmnp-tv-z]{25}"],
                ),
                |((kind, prefix), symbols)| {
                    let canonical = symbols.starts_with(|c: char| ('0'..='7').contains(&c))
                        && symbols.chars().all(|c| REFERENCE_SYMBOLS.contains(c));
                    let expected = if canonical {
                        Ok(id(kind, reference_bytes(&symbols)))
                    } else {
                        Err(IdError { expected: kind })
                    };
                    prop_assert_eq!(
                        PublicId::parse(&format!("{prefix}{symbols}"), kind),
                        expected
                    );
                    Ok(())
                },
            )
            .unwrap();
    }
}
