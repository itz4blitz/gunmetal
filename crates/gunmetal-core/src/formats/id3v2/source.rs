//! Reading a tag's octets in order, with the unsynchronisation scheme
//! reversed where it applies.
//!
//! A tag stored unsynchronised has a zero octet inserted after some `FF`
//! octets (ID3v2.4.0 structure section 6.1, ID3v2.3.0 section 5). Reading
//! it means dropping the zero after every `FF`. Sizes in the tag count the
//! octets after that, while every offset this module reports counts the
//! octets as stored, so offsets always point into the tag the caller holds.

use std::borrow::Cow;

use crate::parse::ParseFault;

use super::tag::Span;

/// Octets of a tag not read yet, and how to read them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Source<'a> {
    /// The octets as stored.
    raw: &'a [u8],
    /// Where `raw` starts, in octets from the start of the tag.
    offset: u64,
    /// Whether the scheme is reversed while reading.
    unsynchronised: bool,
}

impl<'a> Source<'a> {
    /// The octets `raw`, which start `offset` octets into the tag.
    pub(super) const fn new(raw: &'a [u8], offset: u64, unsynchronised: bool) -> Self {
        Self {
            raw,
            offset,
            unsynchronised,
        }
    }

    /// Where the next octet is, in octets from the start of the tag.
    pub(super) const fn offset(&self) -> u64 {
        self.offset
    }

    /// The next octet, which reversing the scheme never changes.
    pub(super) fn first(&self) -> Option<u8> {
        self.raw.first().copied()
    }

    /// The same octets, read unsynchronised when they already were or
    /// when `on` is set.
    pub(super) const fn unsynchronised(self, on: bool) -> Self {
        Self::new(self.raw, self.offset, self.unsynchronised || on)
    }

    /// Where the octets left are in the tag.
    pub(super) fn span(&self) -> Span {
        Span {
            start: self.offset,
            end: self.offset.saturating_add(stored_len(self.raw)),
            unsynchronised: self.unsynchronised,
        }
    }

    /// Moves past the next `len` octets as read, and returns them as a
    /// source of their own.
    ///
    /// # Errors
    ///
    /// Returns [`ParseFault::Truncated`], counting octets as read, when
    /// fewer than `len` are left; the source does not move.
    pub(super) fn split(&mut self, len: u64) -> Result<Self, ParseFault> {
        let (stored, read) = self.measure(len);
        if read < len {
            return Err(ParseFault::Truncated {
                offset: self.offset,
                needed: len,
                available: read,
            });
        }
        Ok(self.advance(stored))
    }

    /// These octets after the next `len` as read, or after all of them
    /// when fewer are left.
    pub(super) fn skip(mut self, len: u64) -> Self {
        let (stored, _) = self.measure(len);
        self.advance(stored);
        self
    }

    /// How many stored octets the next `len` octets as read take, and how
    /// many octets as read that is: `len`, or fewer when the source ends
    /// first.
    fn measure(&self, len: u64) -> (usize, u64) {
        if !self.unsynchronised {
            let stored = usize::try_from(len)
                .unwrap_or(usize::MAX)
                .min(self.raw.len());
            return (
                stored,
                stored_len(self.raw.get(..stored).unwrap_or_default()),
            );
        }
        let mut index = 0;
        let mut read: u64 = 0;
        // Reversing the scheme only drops zeros, so a turn per stored
        // octet is enough and the walk ends even if a step does not
        // move (SEC-MED-008).
        for _ in 0..self.raw.len() {
            if read >= len {
                break;
            }
            let Some((_, next)) = read_at(self.raw, index) else {
                break;
            };
            index = next;
            read = read.saturating_add(1);
        }
        (index, read)
    }

    /// Moves past the next `stored` octets as stored, which the source
    /// holds, and returns them as a source of their own.
    fn advance(&mut self, stored: usize) -> Self {
        let (taken, rest) = self.raw.split_at_checked(stored).unwrap_or((self.raw, &[]));
        let part = Self::new(taken, self.offset, self.unsynchronised);
        self.raw = rest;
        self.offset = self.offset.saturating_add(stored_len(taken));
        part
    }

    /// Every octet left, as read.
    pub(super) fn decode(&self) -> Cow<'a, [u8]> {
        if !self.unsynchronised {
            return Cow::Borrowed(self.raw);
        }
        let mut read = Vec::new();
        let mut index = 0;
        // One decoded octet per stored octet, then stop (SEC-MED-008).
        for _ in 0..self.raw.len() {
            let Some((octet, next)) = read_at(self.raw, index) else {
                break;
            };
            read.push(octet);
            index = next;
        }
        Cow::Owned(read)
    }

    /// Reads the next `N` octets as an array.
    ///
    /// # Errors
    ///
    /// As [`Source::split`].
    pub(super) fn array<const N: usize>(&mut self) -> Result<[u8; N], ParseFault> {
        let width = u64::try_from(N).unwrap_or(u64::MAX);
        let part = self.split(width)?;
        let mut octets = [0; N];
        for (slot, &octet) in octets.iter_mut().zip(part.decode().iter()) {
            *slot = octet;
        }
        Ok(octets)
    }
}

/// The octet unsynchronised `raw` holds at `index`, and the index of the
/// octet after it, past the zero that follows an `FF`.
fn read_at(raw: &[u8], index: usize) -> Option<(u8, usize)> {
    let octet = *raw.get(index)?;
    let next = index.saturating_add(1);
    let dropped = octet == 0xFF && raw.get(next) == Some(&0);
    Some((octet, next.saturating_add(usize::from(dropped))))
}

/// How many octets `raw` holds.
fn stored_len(raw: &[u8]) -> u64 {
    // A slice never holds more than u64::MAX octets.
    u64::try_from(raw.len()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gunmetal_testkit::id3v2::unsynchronise;
    use proptest::collection::vec;
    use proptest::prelude::*;

    fn span(start: u64, end: u64, unsynchronised: bool) -> Span {
        Span {
            start,
            end,
            unsynchronised,
        }
    }

    #[test]
    fn reads_octets_as_stored_when_not_unsynchronised() {
        let raw = [0x01, 0xFF, 0x00, 0x02, 0x03];
        let mut source = Source::new(&raw, 40, false);
        assert_eq!((source.offset(), source.first()), (40, Some(0x01)));
        let part = source.split(3);
        assert_eq!(part, Ok(Source::new(&raw[..3], 40, false)));
        assert_eq!(part.map(|part| part.decode()), Ok(Cow::Borrowed(&raw[..3])));
        assert_eq!(source, Source::new(&raw[3..], 43, false));
        assert_eq!(source.span(), span(43, 45, false));
        assert_eq!(source.array::<2>(), Ok([0x02, 0x03]));
        assert_eq!((source.offset(), source.first()), (45, None));
        assert_eq!(source.span(), span(45, 45, false));
    }

    #[test]
    fn drops_the_zero_after_each_ff_and_counts_offsets_as_stored() {
        // FF 00 FF FF 00 01 FF reads as FF FF FF 01 FF.
        let raw = [0xFF, 0x00, 0xFF, 0xFF, 0x00, 0x01, 0xFF];
        let mut source = Source::new(&raw, 100, true);
        assert_eq!(
            source.decode(),
            Cow::<[u8]>::Owned(vec![0xFF, 0xFF, 0xFF, 0x01, 0xFF])
        );
        // The first octet read takes its dropped zero with it.
        let part = source.split(1);
        assert_eq!(part, Ok(Source::new(&raw[..2], 100, true)));
        assert_eq!(part.map(|part| part.decode().into_owned()), Ok(vec![0xFF]));
        assert_eq!((source.offset(), source.first()), (102, Some(0xFF)));
        // An FF before another FF keeps the octet after it.
        assert_eq!(source.array::<3>(), Ok([0xFF, 0xFF, 0x01]));
        assert_eq!(source.offset(), 106);
        // An FF at the very end stays.
        assert_eq!(source.span(), span(106, 107, true));
        assert_eq!(source.array::<1>(), Ok([0xFF]));
        assert_eq!(source.offset(), 107);
    }

    #[test]
    fn keeps_a_zero_that_does_not_follow_ff() {
        let raw = [0x00, 0x7F, 0x00, 0xFF, 0x00, 0x00];
        let source = Source::new(&raw, 0, true);
        assert_eq!(source.decode().into_owned(), [0x00, 0x7F, 0x00, 0xFF, 0x00]);
    }

    /// Verifies: SEC-MED-001, SEC-MED-008, SEC-TM-032
    #[test]
    fn reports_a_short_read_in_octets_as_read_and_does_not_move() {
        let raw = [0xFF, 0x00, 0x41];
        let mut source = Source::new(&raw, 7, true);
        assert_eq!(
            source.split(3),
            Err(ParseFault::Truncated {
                offset: 7,
                needed: 3,
                available: 2,
            })
        );
        assert_eq!(
            source.array::<4>(),
            Err(ParseFault::Truncated {
                offset: 7,
                needed: 4,
                available: 2,
            })
        );
        assert_eq!(source, Source::new(&raw, 7, true));
        let mut plain = Source::new(&raw, 7, false);
        assert_eq!(
            plain.split(u64::MAX),
            Err(ParseFault::Truncated {
                offset: 7,
                needed: u64::MAX,
                available: 3,
            })
        );
        assert_eq!(plain, Source::new(&raw, 7, false));
        assert_eq!(
            source.split(u64::MAX),
            Err(ParseFault::Truncated {
                offset: 7,
                needed: u64::MAX,
                available: 2,
            })
        );
        assert_eq!(source, Source::new(&raw, 7, true));
        // Reading nothing always works.
        assert_eq!(source.split(0), Ok(Source::new(&[], 7, true)));
        assert_eq!(plain.split(0), Ok(Source::new(&[], 7, false)));
    }

    #[test]
    fn skips_octets_as_read_and_stops_at_the_end() {
        let raw = [0xFF, 0x00, 0x41, 0x42];
        let source = Source::new(&raw, 20, true);
        assert_eq!(source.skip(0), source);
        assert_eq!(source.skip(1), Source::new(&raw[2..], 22, true));
        assert_eq!(source.skip(2), Source::new(&raw[3..], 23, true));
        assert_eq!(source.skip(4), Source::new(&[], 24, true));
        assert_eq!(source.skip(u64::MAX), Source::new(&[], 24, true));
        let plain = Source::new(&raw, 20, false);
        assert_eq!(plain.skip(1), Source::new(&raw[1..], 21, false));
        assert_eq!(plain.skip(9), Source::new(&[], 24, false));
        assert_eq!(plain.skip(u64::MAX), Source::new(&[], 24, false));
    }

    #[test]
    fn turns_unsynchronisation_on_but_never_off() {
        let raw = [0xFF, 0x00];
        let cases = [
            (false, false, false),
            (false, true, true),
            (true, false, true),
            (true, true, true),
        ];
        for (was, on, is) in cases {
            assert_eq!(
                Source::new(&raw, 3, was).unsynchronised(on),
                Source::new(&raw, 3, is),
                "{was} {on}"
            );
        }
    }

    proptest! {
        /// Reading an unsynchronised copy in pieces of any length gives
        /// back the original octets, and each piece starts where the
        /// stored octets of the one before ended.
        ///
        /// Verifies: SEC-MED-001
        #[test]
        fn reverses_the_scheme_as_the_reference_writer_applies_it(
            plain in vec(prop_oneof![Just(0xFF_u8), Just(0x00), Just(0xE0), any::<u8>()], 0..48),
            pieces in vec(0_u64..6, 0..12),
        ) {
            let stored = unsynchronise(&plain);
            let mut source = Source::new(&stored, 1_000, true);
            prop_assert_eq!(source.decode().into_owned(), plain.clone());
            let mut read = Vec::new();
            let mut end = 1_000;
            for len in pieces {
                if let Ok(piece) = source.split(len) {
                    prop_assert_eq!(piece.offset(), end);
                    prop_assert_eq!(u64::try_from(piece.decode().len()).unwrap(), len);
                    end = piece.span().end;
                    read.extend_from_slice(&piece.decode());
                }
                prop_assert_eq!(source.offset(), end);
            }
            read.extend_from_slice(&source.decode());
            prop_assert_eq!(read, plain);
        }
    }
}
