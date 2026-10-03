//! A checked reader over a byte slice.
//!
//! Every read either returns its value and moves past it, or returns
//! [`ParseFault::Truncated`] (or [`ParseFault::NotSyncsafe`]) and leaves
//! the cursor where it was. Nothing indexes the slice directly, every
//! length the input declares is taken as a `u64` and converted fallibly,
//! and offsets in faults are absolute file offsets (SEC-MED-004).

use super::fault::ParseFault;

/// A position in a byte slice, reading forward.
///
/// The cursor knows the file offset of its first octet, so every fault it
/// reports names an absolute offset in the file. Offsets saturate at
/// `u64::MAX` rather than wrap; a [`Window`](super::Window) from a driver
/// always lies inside its file, so they never reach it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor<'a> {
    /// The octets not read yet.
    rest: &'a [u8],
    /// The file offset of `rest`'s first octet.
    offset: u64,
}

impl<'a> Cursor<'a> {
    /// A cursor at the start of `bytes`, which start at file offset 0.
    #[must_use]
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self::at(bytes, 0)
    }

    /// A cursor at the start of `bytes`, which start at file offset
    /// `offset`.
    #[must_use]
    pub const fn at(bytes: &'a [u8], offset: u64) -> Self {
        Self {
            rest: bytes,
            offset,
        }
    }

    /// The file offset of the next octet to read.
    #[must_use]
    pub const fn offset(&self) -> u64 {
        self.offset
    }

    /// How many octets are left.
    #[must_use]
    pub fn remaining(&self) -> u64 {
        // A slice never holds more than u64::MAX octets.
        u64::try_from(self.rest.len()).unwrap_or(u64::MAX)
    }

    /// Whether every octet has been read.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.rest.is_empty()
    }

    /// The octets left, without reading them.
    #[must_use]
    pub const fn rest(&self) -> &'a [u8] {
        self.rest
    }

    /// The fault for a read of `needed` octets at the current position.
    fn truncated(&self, needed: u64) -> ParseFault {
        ParseFault::Truncated {
            offset: self.offset,
            needed,
            available: self.remaining(),
        }
    }

    /// Moves to `rest`, which starts `read` octets further on.
    const fn advance(&mut self, rest: &'a [u8], read: u64) {
        self.rest = rest;
        self.offset = self.offset.saturating_add(read);
    }

    /// Reads the next `N` octets as an array.
    ///
    /// # Errors
    ///
    /// Returns [`ParseFault::Truncated`] when fewer than `N` octets are
    /// left.
    pub fn array<const N: usize>(&mut self) -> Result<[u8; N], ParseFault> {
        // An array never holds more than u64::MAX octets.
        let width = u64::try_from(N).unwrap_or(u64::MAX);
        let Some((head, rest)) = self.rest.split_first_chunk::<N>() else {
            return Err(self.truncated(width));
        };
        let head = *head;
        self.advance(rest, width);
        Ok(head)
    }

    /// Reads one octet.
    ///
    /// # Errors
    ///
    /// Returns [`ParseFault::Truncated`] when no octet is left.
    pub fn u8(&mut self) -> Result<u8, ParseFault> {
        self.array().map(u8::from_be_bytes)
    }

    /// Reads a big-endian 16-bit integer.
    ///
    /// # Errors
    ///
    /// Returns [`ParseFault::Truncated`] when fewer than 2 octets are left.
    pub fn u16_be(&mut self) -> Result<u16, ParseFault> {
        self.array().map(u16::from_be_bytes)
    }

    /// Reads a little-endian 16-bit integer.
    ///
    /// # Errors
    ///
    /// Returns [`ParseFault::Truncated`] when fewer than 2 octets are left.
    pub fn u16_le(&mut self) -> Result<u16, ParseFault> {
        self.array().map(u16::from_le_bytes)
    }

    /// Reads a big-endian 24-bit integer, such as a FLAC metadata block
    /// length.
    ///
    /// # Errors
    ///
    /// Returns [`ParseFault::Truncated`] when fewer than 3 octets are left.
    pub fn u24_be(&mut self) -> Result<u32, ParseFault> {
        self.array()
            .map(|[high, middle, low]| u32::from_be_bytes([0, high, middle, low]))
    }

    /// Reads a big-endian 32-bit integer.
    ///
    /// # Errors
    ///
    /// Returns [`ParseFault::Truncated`] when fewer than 4 octets are left.
    pub fn u32_be(&mut self) -> Result<u32, ParseFault> {
        self.array().map(u32::from_be_bytes)
    }

    /// Reads a little-endian 32-bit integer.
    ///
    /// # Errors
    ///
    /// Returns [`ParseFault::Truncated`] when fewer than 4 octets are left.
    pub fn u32_le(&mut self) -> Result<u32, ParseFault> {
        self.array().map(u32::from_le_bytes)
    }

    /// Reads a big-endian 64-bit integer.
    ///
    /// # Errors
    ///
    /// Returns [`ParseFault::Truncated`] when fewer than 8 octets are left.
    pub fn u64_be(&mut self) -> Result<u64, ParseFault> {
        self.array().map(u64::from_be_bytes)
    }

    /// Reads a little-endian 64-bit integer.
    ///
    /// # Errors
    ///
    /// Returns [`ParseFault::Truncated`] when fewer than 8 octets are left.
    pub fn u64_le(&mut self) -> Result<u64, ParseFault> {
        self.array().map(u64::from_le_bytes)
    }

    /// Reads a 28-bit syncsafe integer: four octets of seven bits each,
    /// most significant first, as `ID3v2` writes its sizes.
    ///
    /// # Errors
    ///
    /// Returns [`ParseFault::Truncated`] when fewer than 4 octets are left,
    /// and [`ParseFault::NotSyncsafe`] when any of the four has its high bit
    /// set.
    pub fn syncsafe_u32(&mut self) -> Result<u32, ParseFault> {
        let mut ahead = *self;
        let octets = ahead.array::<4>()?;
        if octets.iter().any(|&octet| octet > 0x7F) {
            return Err(ParseFault::NotSyncsafe {
                offset: self.offset,
                octets,
            });
        }
        *self = ahead;
        // Four seven-bit groups are 28 bits, so nothing saturates.
        Ok(octets.iter().fold(0, |value: u32, &octet| {
            value.saturating_mul(128).saturating_add(u32::from(octet))
        }))
    }

    /// Reads the next `len` octets as a slice of the input. A declared
    /// length goes straight in, without conversion.
    ///
    /// # Errors
    ///
    /// Returns [`ParseFault::Truncated`] when fewer than `len` octets are
    /// left.
    pub fn take(&mut self, len: u64) -> Result<&'a [u8], ParseFault> {
        let split = usize::try_from(len)
            .ok()
            .and_then(|at| self.rest.split_at_checked(at));
        let Some((taken, rest)) = split else {
            return Err(self.truncated(len));
        };
        self.advance(rest, len);
        Ok(taken)
    }

    /// Moves past the next `len` octets.
    ///
    /// # Errors
    ///
    /// Returns [`ParseFault::Truncated`] when fewer than `len` octets are
    /// left.
    pub fn skip(&mut self, len: u64) -> Result<(), ParseFault> {
        self.take(len).map(|_| ())
    }

    /// Splits off the next `len` octets as a cursor of their own, for a
    /// structure nested inside this one. The child cannot read past its
    /// `len` octets, and reports the same absolute offsets.
    ///
    /// # Errors
    ///
    /// Returns [`ParseFault::Truncated`] when fewer than `len` octets are
    /// left.
    pub fn sub(&mut self, len: u64) -> Result<Self, ParseFault> {
        let offset = self.offset;
        self.take(len).map(|bytes| Self::at(bytes, offset))
    }
}

#[cfg(test)]
mod tests {
    use super::super::small_stack::on_small_stack;
    use super::*;
    use proptest::collection::vec;
    use proptest::prelude::*;

    const BYTES: [u8; 8] = [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0];

    #[test]
    fn reads_every_integer_width_in_both_byte_orders() {
        let at = || Cursor::at(&BYTES, 100);
        let mut cursor = at();
        assert_eq!((cursor.u8(), cursor.offset()), (Ok(0x12), 101));
        let mut cursor = at();
        assert_eq!((cursor.u16_be(), cursor.offset()), (Ok(0x1234), 102));
        let mut cursor = at();
        assert_eq!((cursor.u16_le(), cursor.offset()), (Ok(0x3412), 102));
        let mut cursor = at();
        assert_eq!((cursor.u24_be(), cursor.offset()), (Ok(0x0012_3456), 103));
        let mut cursor = at();
        assert_eq!((cursor.u32_be(), cursor.offset()), (Ok(0x1234_5678), 104));
        let mut cursor = at();
        assert_eq!((cursor.u32_le(), cursor.offset()), (Ok(0x7856_3412), 104));
        let mut cursor = at();
        assert_eq!(
            (cursor.u64_be(), cursor.offset()),
            (Ok(0x1234_5678_9ABC_DEF0), 108)
        );
        let mut cursor = at();
        assert_eq!(
            (cursor.u64_le(), cursor.offset()),
            (Ok(0xF0DE_BC9A_7856_3412), 108)
        );
        let mut cursor = at();
        assert_eq!(
            (cursor.array::<3>(), cursor.offset()),
            (Ok([0x12, 0x34, 0x56]), 103)
        );
    }

    #[test]
    fn reads_values_one_after_another() {
        let mut cursor = Cursor::new(&BYTES);
        assert_eq!(cursor.u8(), Ok(0x12));
        assert_eq!(cursor.u16_le(), Ok(0x5634));
        assert_eq!(cursor.u32_be(), Ok(0x789A_BCDE));
        assert_eq!(cursor.u8(), Ok(0xF0));
        assert_eq!(
            (cursor.offset(), cursor.remaining(), cursor.is_empty()),
            (8, 0, true)
        );
        assert_eq!(cursor.rest(), &[] as &[u8]);
    }

    #[test]
    fn shows_what_is_left_without_reading_it() {
        let mut cursor = Cursor::at(&BYTES, 7);
        assert_eq!(cursor.skip(5), Ok(()));
        assert_eq!(
            (
                cursor.rest(),
                cursor.remaining(),
                cursor.is_empty(),
                cursor.offset()
            ),
            (&BYTES[5..], 3, false, 12)
        );
        assert_eq!(cursor.rest(), &[0xBC, 0xDE, 0xF0]);
        assert_eq!(cursor.offset(), 12);
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn reports_truncation_with_exact_offsets_and_does_not_move() {
        let short = [0x01, 0x02, 0x03];
        let start = Cursor::at(&short, 40);
        let truncated = |needed| ParseFault::Truncated {
            offset: 40,
            needed,
            available: 3,
        };
        let mut cursor = start;
        assert_eq!(cursor.u32_be(), Err(truncated(4)));
        assert_eq!(cursor.u32_le(), Err(truncated(4)));
        assert_eq!(cursor.u64_be(), Err(truncated(8)));
        assert_eq!(cursor.u64_le(), Err(truncated(8)));
        assert_eq!(cursor.syncsafe_u32(), Err(truncated(4)));
        assert_eq!(cursor.array::<5>(), Err(truncated(5)));
        assert_eq!(cursor.take(4), Err(truncated(4)));
        assert_eq!(cursor.skip(4), Err(truncated(4)));
        assert_eq!(cursor.sub(4), Err(truncated(4)));
        assert_eq!(cursor, start);

        assert_eq!(cursor.u16_be(), Ok(0x0102));
        let after = cursor;
        let truncated = |needed| ParseFault::Truncated {
            offset: 42,
            needed,
            available: 1,
        };
        assert_eq!(cursor.u16_be(), Err(truncated(2)));
        assert_eq!(cursor.u16_le(), Err(truncated(2)));
        assert_eq!(cursor.u24_be(), Err(truncated(3)));
        assert_eq!(cursor, after);
    }

    /// Verifies: SEC-MED-001
    #[test]
    fn every_read_of_empty_input_is_truncated() {
        let mut cursor = Cursor::at(&[], 9);
        assert_eq!(
            (cursor.remaining(), cursor.is_empty(), cursor.rest()),
            (0, true, &[] as &[u8])
        );
        assert_eq!(
            cursor.u8(),
            Err(ParseFault::Truncated {
                offset: 9,
                needed: 1,
                available: 0,
            })
        );
        // A read of nothing succeeds even here.
        assert_eq!(cursor.take(0), Ok(&[] as &[u8]));
        assert_eq!(cursor.array::<0>(), Ok([]));
        assert_eq!(cursor.offset(), 9);
    }

    #[test]
    fn decodes_syncsafe_integers() {
        let cases: [([u8; 4], u32); 4] = [
            ([0x00, 0x00, 0x00, 0x00], 0),
            ([0x00, 0x00, 0x02, 0x01], 257),
            ([0x01, 0x02, 0x03, 0x04], 0x0020_8184),
            ([0x7F, 0x7F, 0x7F, 0x7F], 0x0FFF_FFFF),
        ];
        for (octets, value) in cases {
            let mut cursor = Cursor::at(&octets, 10);
            assert_eq!(
                (cursor.syncsafe_u32(), cursor.offset()),
                (Ok(value), 14),
                "{octets:02X?}"
            );
        }
    }

    #[test]
    fn refuses_a_syncsafe_integer_with_a_high_bit_set_and_does_not_move() {
        for octets in [
            [0x80, 0x00, 0x00, 0x00],
            [0x00, 0x80, 0x00, 0x00],
            [0x00, 0x00, 0xFF, 0x00],
            [0x00, 0x00, 0x00, 0x80],
        ] {
            let start = Cursor::at(&octets, 20);
            let mut cursor = start;
            assert_eq!(
                cursor.syncsafe_u32(),
                Err(ParseFault::NotSyncsafe { offset: 20, octets }),
                "{octets:02X?}"
            );
            assert_eq!(cursor, start);
        }
    }

    /// Verifies: SEC-MED-004, SEC-TM-032
    #[test]
    fn takes_slices_of_declared_lengths() {
        let mut cursor = Cursor::at(&BYTES, 50);
        assert_eq!(cursor.take(0), Ok(&[] as &[u8]));
        assert_eq!(cursor.offset(), 50);
        assert_eq!(cursor.take(3), Ok(&BYTES[..3]));
        assert_eq!(cursor.offset(), 53);
        let before = cursor;
        assert_eq!(
            cursor.take(6),
            Err(ParseFault::Truncated {
                offset: 53,
                needed: 6,
                available: 5,
            })
        );
        // A declared length of u64::MAX is a claim like any other.
        assert_eq!(
            cursor.take(u64::MAX),
            Err(ParseFault::Truncated {
                offset: 53,
                needed: u64::MAX,
                available: 5,
            })
        );
        assert_eq!(cursor, before);
        assert_eq!(cursor.take(5), Ok(&BYTES[3..]));
        assert_eq!((cursor.offset(), cursor.is_empty()), (58, true));
    }

    #[test]
    fn skips_declared_lengths() {
        let mut cursor = Cursor::at(&BYTES, 0);
        assert_eq!(cursor.skip(6), Ok(()));
        assert_eq!(cursor.u16_be(), Ok(0xDEF0));
        assert_eq!(
            cursor.skip(1),
            Err(ParseFault::Truncated {
                offset: 8,
                needed: 1,
                available: 0,
            })
        );
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn a_child_cursor_reads_only_its_own_octets_at_absolute_offsets() {
        let mut parent = Cursor::at(&BYTES, 200);
        assert_eq!(parent.u8(), Ok(0x12));
        let child = parent.sub(3);
        assert_eq!(child, Ok(Cursor::at(&BYTES[1..4], 201)));
        // The parent moved past the child's octets.
        assert_eq!((parent.offset(), parent.rest()), (204, &BYTES[4..]));

        let mut child = child.unwrap_or(parent);
        assert_eq!(child.u16_be(), Ok(0x3456));
        // The parent has more octets, but the child stops at its own end.
        assert_eq!(
            child.u16_be(),
            Err(ParseFault::Truncated {
                offset: 203,
                needed: 2,
                available: 1,
            })
        );
    }

    /// Verifies: SEC-MED-004
    #[test]
    fn saturates_offsets_instead_of_wrapping() {
        let mut cursor = Cursor::at(&BYTES, u64::MAX - 2);
        assert_eq!(cursor.u16_be(), Ok(0x1234));
        assert_eq!(cursor.offset(), u64::MAX);
        assert_eq!(cursor.u32_be(), Ok(0x5678_9ABC));
        assert_eq!(cursor.offset(), u64::MAX);
        assert_eq!(cursor.take(2), Ok(&BYTES[6..]));
        assert_eq!(cursor.offset(), u64::MAX);
    }

    /// One read, as the property test generates them.
    #[derive(Debug, Clone, Copy)]
    enum Read {
        U8,
        U16Be,
        U16Le,
        U24Be,
        U32Be,
        U32Le,
        U64Be,
        U64Le,
        Syncsafe,
        Take(u64),
        Skip(u64),
        Sub(u64),
    }

    /// What one read returned, owned so it can leave the small-stack thread.
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Got {
        Int(u64),
        Bytes(Vec<u8>),
        Moved,
        Child { offset: u64, bytes: Vec<u8> },
    }

    /// Lengths: mostly small, sometimes huge, sometimes exactly `u64::MAX`.
    fn length() -> impl Strategy<Value = u64> {
        prop_oneof![0_u64..24, any::<u64>(), Just(u64::MAX)]
    }

    fn read() -> impl Strategy<Value = Read> {
        prop_oneof![
            Just(Read::U8),
            Just(Read::U16Be),
            Just(Read::U16Le),
            Just(Read::U24Be),
            Just(Read::U32Be),
            Just(Read::U32Le),
            Just(Read::U64Be),
            Just(Read::U64Le),
            Just(Read::Syncsafe),
            length().prop_map(Read::Take),
            length().prop_map(Read::Skip),
            length().prop_map(Read::Sub),
        ]
    }

    /// Applies `reads` to a cursor over `bytes` at `base`, recording each
    /// result and the offset after it.
    fn run(bytes: &[u8], base: u64, reads: &[Read]) -> Vec<(Result<Got, ParseFault>, u64)> {
        let mut cursor = Cursor::at(bytes, base);
        reads
            .iter()
            .map(|read| {
                let got = match *read {
                    Read::U8 => cursor.u8().map(|v| Got::Int(v.into())),
                    Read::U16Be => cursor.u16_be().map(|v| Got::Int(v.into())),
                    Read::U16Le => cursor.u16_le().map(|v| Got::Int(v.into())),
                    Read::U24Be => cursor.u24_be().map(|v| Got::Int(v.into())),
                    Read::U32Be => cursor.u32_be().map(|v| Got::Int(v.into())),
                    Read::U32Le => cursor.u32_le().map(|v| Got::Int(v.into())),
                    Read::U64Be => cursor.u64_be().map(Got::Int),
                    Read::U64Le => cursor.u64_le().map(Got::Int),
                    Read::Syncsafe => cursor.syncsafe_u32().map(|v| Got::Int(v.into())),
                    Read::Take(len) => cursor.take(len).map(|b| Got::Bytes(b.to_vec())),
                    Read::Skip(len) => cursor.skip(len).map(|()| Got::Moved),
                    Read::Sub(len) => cursor.sub(len).map(|child| Got::Child {
                        offset: child.offset(),
                        bytes: child.rest().to_vec(),
                    }),
                };
                (got, cursor.offset())
            })
            .collect()
    }

    /// An independent model of the same reads: integers are assembled
    /// octet by octet in `u128`, and positions are tracked as `u128`.
    fn model(bytes: &[u8], base: u64, reads: &[Read]) -> Vec<(Result<Got, ParseFault>, u64)> {
        let total = bytes.len() as u128;
        let mut pos: u128 = 0;
        let offset = |pos: u128| u64::try_from(u128::from(base) + pos).unwrap_or(u64::MAX);
        let mut out = Vec::new();
        for read in reads {
            let (width, big_endian) = match *read {
                Read::U8 => (Some(1_u128), true),
                Read::U16Be => (Some(2), true),
                Read::U16Le => (Some(2), false),
                Read::U24Be => (Some(3), true),
                Read::U32Be | Read::Syncsafe => (Some(4), true),
                Read::U32Le => (Some(4), false),
                Read::U64Be => (Some(8), true),
                Read::U64Le => (Some(8), false),
                Read::Take(_) | Read::Skip(_) | Read::Sub(_) => (None, true),
            };
            let needed = match *read {
                Read::Take(len) | Read::Skip(len) | Read::Sub(len) => u128::from(len),
                _ => width.unwrap_or(0),
            };
            let truncated = ParseFault::Truncated {
                offset: offset(pos),
                needed: u64::try_from(needed).unwrap_or(u64::MAX),
                available: u64::try_from(total - pos).unwrap_or(u64::MAX),
            };
            if needed > total - pos {
                out.push((Err(truncated), offset(pos)));
                continue;
            }
            let start = usize::try_from(pos).unwrap_or(usize::MAX);
            let end = usize::try_from(pos + needed).unwrap_or(usize::MAX);
            let octets = &bytes[start..end];
            let got = match *read {
                Read::Take(_) => Ok(Got::Bytes(octets.to_vec())),
                Read::Skip(_) => Ok(Got::Moved),
                Read::Sub(_) => Ok(Got::Child {
                    offset: offset(pos),
                    bytes: octets.to_vec(),
                }),
                Read::Syncsafe if octets.iter().any(|&o| o >= 0x80) => {
                    out.push((
                        Err(ParseFault::NotSyncsafe {
                            offset: offset(pos),
                            octets: [octets[0], octets[1], octets[2], octets[3]],
                        }),
                        offset(pos),
                    ));
                    continue;
                }
                Read::Syncsafe => Ok(Got::Int(
                    octets
                        .iter()
                        .fold(0_u64, |acc, &o| (acc << 7) + u64::from(o)),
                )),
                _ => {
                    let mut ordered = octets.to_vec();
                    if !big_endian {
                        ordered.reverse();
                    }
                    let value = ordered
                        .iter()
                        .fold(0_u128, |acc, &o| (acc << 8) + u128::from(o));
                    Ok(Got::Int(u64::try_from(value).unwrap_or(u64::MAX)))
                }
            };
            pos += needed;
            out.push((got, offset(pos)));
        }
        out
    }

    proptest! {
        /// Verifies: SEC-MED-001, SEC-MED-004, SEC-TM-032
        #[test]
        fn never_reads_past_its_slice_for_any_sequence_of_reads(
            bytes in vec(any::<u8>(), 0..48),
            base in prop_oneof![0_u64..1_000, any::<u64>()],
            reads in vec(read(), 0..24),
        ) {
            let expected = model(&bytes, base, &reads);
            let actual = on_small_stack(move || run(&bytes, base, &reads));
            prop_assert_eq!(actual, expected);
        }
    }
}
