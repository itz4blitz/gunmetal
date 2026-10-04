//! The serialised index: the segment a server can build for one profile
//! and ship to a device that is too slow to build its own (API-SYNC-07,
//! DIS-084).
//!
//! ```text
//! magic:    "GMSI"
//! version:  u8, 1
//! docs:     varint count, then per document
//!             id:        30 octets, the public identifier's text
//!             plays:     varint
//!             title_len: u8
//! terms:    varint count, then per term, in ascending order
//!             len:       u8, octets of the term
//!             term:      UTF-8, 1 to 32 characters
//!             postings:  varint count, then per posting
//!               doc:     varint, added to the posting before it
//!               place:   u8
//! ```
//!
//! A varint is a `u32` in groups of seven bits, lowest first, the high bit
//! of each octet saying that another follows, in the fewest octets that
//! hold it. Every index therefore has exactly one written form, and
//! [`Index::from_bytes`] accepts no other.
//!
//! A segment is untrusted input. A count it declares sizes nothing: each
//! document, term and posting is read and kept one at a time, and each
//! takes at least two octets, so memory stays in proportion to the input.
//!
//! # Steps (SEC-MED-007)
//!
//! [`Index::from_bytes`] spends one step per document, per term and per
//! posting. Each takes at least two octets, so [`STEPS_PER_OCTET`] steps
//! per octet plus [`FIXED_STEPS`] always suffice.

use std::str;

use crate::parse::{Budget, Cursor, ParseFault};

use super::doc::DocRef;
use super::index::{Entry, Index, MAX_TOKEN_CHARS, Posting};

/// Steps a segment's budget needs per octet (k in SEC-MED-007).
pub const STEPS_PER_OCTET: u64 = 1;

/// Steps a segment's budget needs on top (c in SEC-MED-007).
pub const FIXED_STEPS: u64 = 0;

/// What every segment starts with.
const MAGIC: [u8; 4] = *b"GMSI";

/// The one version of the layout.
const VERSION: u8 = 1;

/// Octets of a public identifier's text.
const ID_OCTETS: u64 = 30;

/// How far each of a varint's five groups is shifted.
const SHIFTS: [u32; 5] = [0, 7, 14, 21, 28];

/// Why octets are not a segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexError {
    /// The segment ended early, or reading it ran out of steps.
    Fault(ParseFault),
    /// The first four octets are not the segment's magic.
    NotASegment {
        /// The octets found instead.
        found: [u8; 4],
    },
    /// The segment is written in a layout this build does not read.
    UnknownVersion {
        /// The version it declares.
        version: u8,
    },
    /// A number is over 32 bits, runs past five octets, or is not written
    /// in the fewest octets that hold it.
    BadNumber {
        /// Where the number starts.
        offset: u64,
    },
    /// A document's identifier is not the text of an artist's, album's,
    /// track's or playlist's public identifier.
    BadId {
        /// Where the document starts.
        offset: u64,
    },
    /// A term is not UTF-8.
    TermNotText {
        /// Where the term's entry starts.
        offset: u64,
    },
    /// A term is empty or longer than a token can be.
    TermLength {
        /// Where the term's entry starts.
        offset: u64,
        /// How many characters it has.
        chars: usize,
    },
    /// A term does not sort after the term before it.
    TermOrder {
        /// Where the term's entry starts.
        offset: u64,
    },
    /// A posting names a document the segment does not hold.
    UnknownDoc {
        /// Where the posting starts.
        offset: u64,
    },
    /// Octets follow the last term.
    TrailingBytes {
        /// Where they start.
        offset: u64,
        /// How many there are.
        extra: u64,
    },
}

impl From<ParseFault> for IndexError {
    fn from(fault: ParseFault) -> Self {
        Self::Fault(fault)
    }
}

impl Index {
    /// The index as one segment.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::from(MAGIC);
        out.push(VERSION);
        put_varint(&mut out, count(self.docs.len()));
        for entry in &self.docs {
            out.extend_from_slice(entry.doc.id().to_string().as_bytes());
            put_varint(&mut out, entry.plays);
            out.push(entry.title_len);
        }
        put_varint(&mut out, count(self.terms.len()));
        for (term, postings) in &self.terms {
            // A token is at most 32 characters of at most four octets.
            out.push(u8::try_from(term.len()).unwrap_or(u8::MAX));
            out.extend_from_slice(term.as_bytes());
            put_varint(&mut out, count(postings.len()));
            let mut before = 0_u32;
            for posting in postings {
                // Postings are in ascending order of document.
                put_varint(&mut out, posting.doc.saturating_sub(before));
                out.push(posting.place);
                before = posting.doc;
            }
        }
        out
    }

    /// Reads a segment, spending `budget`.
    ///
    /// # Errors
    ///
    /// Returns an [`IndexError`] unless `bytes` is exactly what
    /// [`Index::to_bytes`] writes for some index, or when `budget` runs
    /// out first.
    pub fn from_bytes(bytes: &[u8], budget: &mut Budget) -> Result<Self, IndexError> {
        let mut cursor = Cursor::new(bytes);
        let found = cursor.array::<4>()?;
        if found != MAGIC {
            return Err(IndexError::NotASegment { found });
        }
        let version = cursor.u8()?;
        if version != VERSION {
            return Err(IndexError::UnknownVersion { version });
        }
        let mut index = Self::default();
        let docs = varint(&mut cursor)?;
        for _ in 0..docs {
            let offset = cursor.offset();
            budget.charge(1, offset)?;
            let doc = str::from_utf8(cursor.take(ID_OCTETS)?)
                .ok()
                .and_then(DocRef::from_text)
                .ok_or(IndexError::BadId { offset })?;
            let plays = varint(&mut cursor)?;
            let title_len = cursor.u8()?;
            index.docs.push(Entry {
                doc,
                plays,
                title_len,
            });
        }
        let terms = varint(&mut cursor)?;
        for _ in 0..terms {
            let offset = cursor.offset();
            budget.charge(1, offset)?;
            let len = cursor.u8()?;
            let term = str::from_utf8(cursor.take(u64::from(len))?)
                .ok()
                .ok_or(IndexError::TermNotText { offset })?;
            let chars = term.chars().count();
            if chars == 0 || chars > MAX_TOKEN_CHARS {
                return Err(IndexError::TermLength { offset, chars });
            }
            let after_last = index
                .terms
                .last_key_value()
                .is_none_or(|(last, _)| last.as_str() < term);
            if !after_last {
                return Err(IndexError::TermOrder { offset });
            }
            let mut postings = Vec::new();
            let mut doc = 0_u32;
            for _ in 0..varint(&mut cursor)? {
                let offset = cursor.offset();
                budget.charge(1, offset)?;
                doc = doc
                    .checked_add(varint(&mut cursor)?)
                    .filter(|doc| *doc < docs)
                    .ok_or(IndexError::UnknownDoc { offset })?;
                let place = cursor.u8()?;
                postings.push(Posting { doc, place });
            }
            index.terms.insert(term.to_owned(), postings);
        }
        if !cursor.is_empty() {
            return Err(IndexError::TrailingBytes {
                offset: cursor.offset(),
                extra: cursor.remaining(),
            });
        }
        Ok(index)
    }
}

/// How many of something there are, as a segment writes it.
fn count(len: usize) -> u32 {
    u32::try_from(len).unwrap_or(u32::MAX)
}

/// Appends `value` as a varint.
fn put_varint(out: &mut Vec<u8>, value: u32) {
    let groups = SHIFTS.map(|shift| value.wrapping_shr(shift));
    // The group after the last is zero, so the loop always ends at its
    // `break`, after the last group that holds a set bit.
    for (group, next) in groups.iter().zip(groups.iter().skip(1).chain(&[0])) {
        let [low, ..] = group.to_le_bytes();
        if *next == 0 {
            out.push(low);
            break;
        }
        out.push(low | 0x80);
    }
}

/// Reads a varint.
fn varint(cursor: &mut Cursor<'_>) -> Result<u32, IndexError> {
    let offset = cursor.offset();
    let mut value = 0_u64;
    for shift in SHIFTS {
        let octet = cursor.u8()?;
        // The groups do not overlap, and five of seven bits fit in 64.
        value = value.saturating_add(u64::from(octet & 0x7F).wrapping_shl(shift));
        if octet & 0x80 == 0 {
            // One written form: only a lone octet may be zero.
            return u32::try_from(value)
                .ok()
                .filter(|_| octet != 0 || shift == 0)
                .ok_or(IndexError::BadNumber { offset });
        }
    }
    Err(IndexError::BadNumber { offset })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::super::doc::{DocKind, SearchDoc};
    use super::super::index::ELSEWHERE;
    use super::super::query::{Hit, KindFilter, Match};
    use super::super::testing::{doc_ref, owned, titled};
    use super::*;
    use proptest::collection::vec;
    use proptest::prelude::*;

    /// The segment of [`two_docs`], written out by hand.
    const SEGMENT: [u8; 88] = *b"GMSI\x01\
        \x02\
        trk_00000000000000000000000001\xAC\x02\x01\
        art_00000000000000000000000002\x00\x02\
        \x02\
        \x02go\x03\x00\x00\x01\x00\x00\x01\
        \x02ol\x01\x00\xFF";

    /// Where each field of [`SEGMENT`] starts and how many octets it has,
    /// each varint octet as a field of its own.
    const LAYOUT: [(u64, u64); 26] = [
        (0, 4),
        (4, 1),
        (5, 1),
        (6, 30),
        (36, 1),
        (37, 1),
        (38, 1),
        (39, 30),
        (69, 1),
        (70, 1),
        (71, 1),
        (72, 1),
        (73, 2),
        (75, 1),
        (76, 1),
        (77, 1),
        (78, 1),
        (79, 1),
        (80, 1),
        (81, 1),
        (82, 1),
        (83, 2),
        (85, 1),
        (86, 1),
        (87, 1),
        (88, 0),
    ];

    /// A track "Go" by "Öl", credited to "Öl" too and played 300 times,
    /// and an artist "Go Go".
    fn two_docs() -> Index {
        let mut track = titled(DocKind::Track, 1, "Go");
        track.artist = String::from("Öl");
        track.credits = owned(&["Öl"]);
        track.plays = 300;
        Index::build([track, titled(DocKind::Artist, 2, "Go Go")].into_iter())
    }

    /// More steps than any test segment needs.
    fn plenty() -> Budget {
        Budget::for_input(0, 0, 1_000_000)
    }

    fn read(bytes: &[u8]) -> Result<Index, IndexError> {
        Index::from_bytes(bytes, &mut plenty())
    }

    /// `parts` one after another.
    fn joined(parts: &[&[u8]]) -> Vec<u8> {
        parts.concat()
    }

    const HEAD: &[u8] = b"GMSI\x01";
    const TRACK_1: &[u8] = b"trk_00000000000000000000000001";
    const TRACK_2: &[u8] = b"trk_00000000000000000000000002";

    #[test]
    fn writes_the_segment_octet_for_octet() {
        assert_eq!(two_docs().to_bytes(), SEGMENT);
    }

    #[test]
    fn writes_an_empty_index_as_a_header_and_two_zero_counts() {
        assert_eq!(Index::default().to_bytes(), b"GMSI\x01\x00\x00");
        assert_eq!(read(b"GMSI\x01\x00\x00"), Ok(Index::default()));
    }

    #[test]
    fn reads_the_segment_back_as_the_index_that_wrote_it() {
        let index = read(&SEGMENT).unwrap();
        assert_eq!(
            index.docs,
            [
                Entry {
                    doc: doc_ref(DocKind::Track, 1),
                    plays: 300,
                    title_len: 1
                },
                Entry {
                    doc: doc_ref(DocKind::Artist, 2),
                    plays: 0,
                    title_len: 2
                },
            ]
        );
        assert_eq!(
            index.terms,
            BTreeMap::from([
                (
                    String::from("go"),
                    vec![
                        Posting { doc: 0, place: 0 },
                        Posting { doc: 1, place: 0 },
                        Posting { doc: 1, place: 1 },
                    ]
                ),
                (
                    String::from("ol"),
                    vec![Posting {
                        doc: 0,
                        place: ELSEWHERE
                    }]
                ),
            ])
        );
        assert_eq!(index, two_docs());
    }

    #[test]
    fn a_segment_that_was_read_answers_queries() {
        let index = read(&SEGMENT).unwrap();
        assert_eq!(
            index.query("go", KindFilter::All, u16::MAX),
            [
                Hit {
                    doc: doc_ref(DocKind::Track, 1),
                    matched: Match::WholeTitle
                },
                Hit {
                    doc: doc_ref(DocKind::Artist, 2),
                    matched: Match::TitleStart
                },
            ]
        );
    }

    #[test]
    fn writes_numbers_in_groups_of_seven_bits_lowest_first() {
        let cases: [(u32, &[u8]); 10] = [
            (0, &[0x00]),
            (1, &[0x01]),
            (127, &[0x7F]),
            (128, &[0x80, 0x01]),
            (300, &[0xAC, 0x02]),
            (16_383, &[0xFF, 0x7F]),
            (16_384, &[0x80, 0x80, 0x01]),
            (2_097_152, &[0x80, 0x80, 0x80, 0x01]),
            (268_435_456, &[0x80, 0x80, 0x80, 0x80, 0x01]),
            (u32::MAX, &[0xFF, 0xFF, 0xFF, 0xFF, 0x0F]),
        ];
        for (value, octets) in cases {
            let mut out = vec![0x55];
            put_varint(&mut out, value);
            assert_eq!(out, joined(&[&[0x55], octets]));
            let mut cursor = Cursor::new(octets);
            assert_eq!(varint(&mut cursor), Ok(value));
            assert!(cursor.is_empty());
        }
    }

    #[test]
    fn reads_a_number_and_stops_after_it() {
        let mut cursor = Cursor::at(&[0xAC, 0x02, 0xFF], 40);
        assert_eq!(varint(&mut cursor), Ok(300));
        assert_eq!(cursor.offset(), 42);
    }

    #[test]
    fn refuses_a_number_that_is_not_in_its_one_written_form() {
        let bad: [&[u8]; 8] = [
            // Zero, and one, with a needless second octet.
            &[0x80, 0x00],
            &[0x81, 0x00],
            // A needless fifth octet.
            &[0x80, 0x80, 0x80, 0x80, 0x00],
            // Bit 32 set, and bit 35.
            &[0x80, 0x80, 0x80, 0x80, 0x10],
            &[0xFF, 0xFF, 0xFF, 0xFF, 0x7F],
            // A sixth octet promised, and given.
            &[0xFF, 0xFF, 0xFF, 0xFF, 0x8F],
            &[0x80, 0x80, 0x80, 0x80, 0x80, 0x01],
            &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
        ];
        for octets in bad {
            let mut cursor = Cursor::at(octets, 9);
            assert_eq!(
                (octets, varint(&mut cursor)),
                (octets, Err(IndexError::BadNumber { offset: 9 }))
            );
        }
        assert_eq!(
            read(b"GMSI\x01\x80\x00\x00"),
            Err(IndexError::BadNumber { offset: 5 })
        );
    }

    #[test]
    fn a_number_cut_short_is_truncated() {
        let mut cursor = Cursor::at(&[0x80, 0x80], 9);
        assert_eq!(
            varint(&mut cursor),
            Err(IndexError::Fault(ParseFault::Truncated {
                offset: 11,
                needed: 1,
                available: 0
            }))
        );
    }

    #[test]
    fn the_layout_table_tiles_the_segment() {
        let mut next = 0;
        for (offset, len) in LAYOUT {
            assert_eq!(offset, next);
            next = offset + len;
        }
        assert_eq!(next, 88);
    }

    /// Verifies: SEC-MED-001
    #[test]
    fn every_cut_of_the_segment_is_truncated_at_the_field_it_cuts() {
        for cut in 0..SEGMENT.len() {
            let have = u64::try_from(cut).unwrap();
            let (offset, needed) = LAYOUT
                .into_iter()
                .find(|(offset, len)| offset + len > have)
                .unwrap();
            assert_eq!(
                (cut, read(&SEGMENT[..cut])),
                (
                    cut,
                    Err(IndexError::Fault(ParseFault::Truncated {
                        offset,
                        needed,
                        available: have - offset
                    }))
                )
            );
        }
    }

    #[test]
    fn refuses_octets_that_do_not_start_with_the_magic() {
        assert_eq!(
            read(b"GMSJ\x01\x00\x00"),
            Err(IndexError::NotASegment { found: *b"GMSJ" })
        );
        assert_eq!(
            read(b"fLaC\x00\x00\x00\x22"),
            Err(IndexError::NotASegment { found: *b"fLaC" })
        );
    }

    #[test]
    fn refuses_a_version_it_does_not_read() {
        assert_eq!(
            read(b"GMSI\x02\x00\x00"),
            Err(IndexError::UnknownVersion { version: 2 })
        );
        assert_eq!(
            read(b"GMSI\x00\x00\x00"),
            Err(IndexError::UnknownVersion { version: 0 })
        );
    }

    #[test]
    fn refuses_an_identifier_that_is_not_a_documents() {
        let ids: [&[u8]; 5] = [
            // A user's identifier.
            b"usr_00000000000000000000000001",
            // Upper case.
            b"TRK_00000000000000000000000001",
            // A symbol outside the alphabet.
            b"trk_0000000000000000000000000l",
            // Not UTF-8.
            &[0xFF; 30],
            // The right length in octets, but not in symbols.
            b"trk_000000000000000000000000\xC3\xA9",
        ];
        for id in ids {
            // One good document first, so the offset is the bad one's.
            let bytes = joined(&[HEAD, &[2], TRACK_1, &[0, 0], id, &[0, 0], &[0]]);
            assert_eq!(read(&bytes), Err(IndexError::BadId { offset: 38 }));
        }
    }

    #[test]
    fn reads_a_term_of_1_to_32_characters() {
        let one = joined(&[HEAD, &[0], &[1], &[1], b"a", &[0]]);
        assert_eq!(
            read(&one).map(|index| index.terms),
            Ok(BTreeMap::from([(String::from("a"), Vec::new())]))
        );
        let longest = "abcdefghijklmnopqrstuvwxyzabcdef";
        let thirty_two = joined(&[HEAD, &[0], &[1], &[32], longest.as_bytes(), &[0]]);
        assert_eq!(
            read(&thirty_two).map(|index| index.terms),
            Ok(BTreeMap::from([(String::from(longest), Vec::new())]))
        );
        // Characters, not octets: twenty of two octets each.
        let wide = "éééééééééééééééééééé";
        let forty_octets = joined(&[HEAD, &[0], &[1], &[40], wide.as_bytes(), &[0]]);
        assert_eq!(
            read(&forty_octets).map(|index| index.terms),
            Ok(BTreeMap::from([(String::from(wide), Vec::new())]))
        );
    }

    #[test]
    fn refuses_a_term_that_is_empty_or_over_32_characters() {
        let empty = joined(&[HEAD, &[0], &[1], &[0], &[0]]);
        assert_eq!(
            read(&empty),
            Err(IndexError::TermLength {
                offset: 7,
                chars: 0
            })
        );
        let long = joined(&[
            HEAD,
            &[0],
            &[2],
            &[1],
            b"a",
            &[0],
            &[33],
            b"abcdefghijklmnopqrstuvwxyzabcdefg",
            &[0],
        ]);
        assert_eq!(
            read(&long),
            Err(IndexError::TermLength {
                offset: 10,
                chars: 33
            })
        );
    }

    #[test]
    fn refuses_a_term_that_is_not_text() {
        let bytes = joined(&[HEAD, &[0], &[1], &[2], &[b'a', 0xFF], &[0]]);
        assert_eq!(read(&bytes), Err(IndexError::TermNotText { offset: 7 }));
    }

    #[test]
    fn refuses_terms_that_are_not_in_ascending_order() {
        let two = |first: &[u8], second: &[u8]| {
            joined(&[HEAD, &[0], &[2], &[1], first, &[0], &[1], second, &[0]])
        };
        assert_eq!(
            read(&two(b"a", b"b")).map(|index| index.terms),
            Ok(BTreeMap::from([
                (String::from("a"), Vec::new()),
                (String::from("b"), Vec::new()),
            ]))
        );
        assert_eq!(
            read(&two(b"b", b"a")),
            Err(IndexError::TermOrder { offset: 10 })
        );
        assert_eq!(
            read(&two(b"b", b"b")),
            Err(IndexError::TermOrder { offset: 10 })
        );
    }

    #[test]
    fn refuses_a_posting_for_a_document_the_segment_does_not_hold() {
        // No documents at all.
        let none = joined(&[HEAD, &[0], &[1], &[1], b"a", &[1], &[0, 0]]);
        assert_eq!(read(&none), Err(IndexError::UnknownDoc { offset: 10 }));
        // One document: 0 is held, 1 is not.
        let one =
            |postings: &[u8]| joined(&[HEAD, &[1], TRACK_1, &[0, 1], &[1], &[1], b"a", postings]);
        assert_eq!(
            read(&one(&[1, 0, 9])).map(|index| index.terms),
            Ok(BTreeMap::from([(
                String::from("a"),
                vec![Posting { doc: 0, place: 9 }]
            )]))
        );
        assert_eq!(
            read(&one(&[1, 1, 9])),
            Err(IndexError::UnknownDoc { offset: 42 })
        );
        // The second posting adds to the first: 0, then 0 + 1.
        assert_eq!(
            read(&one(&[2, 0, 9, 1, 9])),
            Err(IndexError::UnknownDoc { offset: 44 })
        );
    }

    #[test]
    fn refuses_a_posting_whose_document_number_overflows() {
        // Two documents; postings for 1, then 1 + u32::MAX.
        let bytes = joined(&[
            HEAD,
            &[2],
            TRACK_1,
            &[0, 1],
            TRACK_2,
            &[0, 1],
            &[1],
            &[1],
            b"a",
            &[2],
            &[1, 0],
            &[0xFF, 0xFF, 0xFF, 0xFF, 0x0F, 0],
        ]);
        assert_eq!(read(&bytes), Err(IndexError::UnknownDoc { offset: 76 }));
    }

    #[test]
    fn refuses_octets_after_the_last_term() {
        let bytes = joined(&[&SEGMENT, &[0]]);
        assert_eq!(
            read(&bytes),
            Err(IndexError::TrailingBytes {
                offset: 88,
                extra: 1
            })
        );
        assert_eq!(
            read(b"GMSI\x01\x00\x00GMSI\x01\x00\x00"),
            Err(IndexError::TrailingBytes {
                offset: 7,
                extra: 7
            })
        );
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn spends_one_step_per_document_term_and_posting() {
        // Two documents, then "go" with three postings, then "ol" with one.
        let charged_at = [6, 39, 72, 76, 78, 80, 82, 86];
        for (steps, offset) in (0_u64..).zip(charged_at) {
            let mut budget = Budget::for_input(0, 0, steps);
            assert_eq!(
                (steps, Index::from_bytes(&SEGMENT, &mut budget)),
                (
                    steps,
                    Err(IndexError::Fault(ParseFault::BudgetExceeded { offset }))
                )
            );
        }
        let mut exact = Budget::for_input(0, 0, 8);
        assert_eq!(Index::from_bytes(&SEGMENT, &mut exact), Ok(two_docs()));
        assert_eq!(exact.remaining(), 0);
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn the_documented_budget_covers_a_segment() {
        assert_eq!((STEPS_PER_OCTET, FIXED_STEPS), (1, 0));
        let mut budget = Budget::for_input(88, STEPS_PER_OCTET, FIXED_STEPS);
        assert_eq!(Index::from_bytes(&SEGMENT, &mut budget), Ok(two_docs()));
        assert_eq!(budget.remaining(), 80);
    }

    #[test]
    fn a_count_of_four_billion_documents_is_only_a_truncation() {
        let bytes = joined(&[HEAD, &[0xFF, 0xFF, 0xFF, 0xFF, 0x0F]]);
        assert_eq!(
            read(&bytes),
            Err(IndexError::Fault(ParseFault::Truncated {
                offset: 10,
                needed: 30,
                available: 0
            }))
        );
        let terms = joined(&[HEAD, &[0], &[0xFF, 0xFF, 0xFF, 0xFF, 0x0F]]);
        assert_eq!(
            read(&terms),
            Err(IndexError::Fault(ParseFault::Truncated {
                offset: 11,
                needed: 1,
                available: 0
            }))
        );
        let postings = joined(&[
            HEAD,
            &[0],
            &[1],
            &[1],
            b"a",
            &[0xFF, 0xFF, 0xFF, 0xFF, 0x0F],
        ]);
        assert_eq!(
            read(&postings),
            Err(IndexError::Fault(ParseFault::Truncated {
                offset: 14,
                needed: 1,
                available: 0
            }))
        );
    }

    #[test]
    fn counts_as_a_segment_writes_them() {
        assert_eq!(count(0), 0);
        assert_eq!(count(300), 300);
        assert_eq!(count(usize::MAX), u32::MAX);
    }

    fn kind() -> impl Strategy<Value = DocKind> {
        prop_oneof![
            Just(DocKind::Artist),
            Just(DocKind::Album),
            Just(DocKind::Track),
            Just(DocKind::Playlist)
        ]
    }

    /// Documents of any text, numbered from 0.
    fn docs() -> impl Strategy<Value = Vec<SearchDoc>> {
        vec(
            (
                kind(),
                ".{0,24}",
                "[a-d ]{0,12}",
                vec(".{0,8}", 0..3),
                any::<u32>(),
            ),
            0..8,
        )
        .prop_map(|fields| {
            fields
                .into_iter()
                .zip(0_u32..)
                .map(|((kind, title, artist, genres, plays), number)| SearchDoc {
                    doc: doc_ref(kind, number),
                    title,
                    artist,
                    album: String::new(),
                    credits: Vec::new(),
                    genres,
                    labels: Vec::new(),
                    plays,
                })
                .collect()
        })
    }

    /// The documented budget for `bytes`.
    fn documented(bytes: &[u8]) -> Budget {
        Budget::for_input(
            u64::try_from(bytes.len()).unwrap(),
            STEPS_PER_OCTET,
            FIXED_STEPS,
        )
    }

    /// What reading `bytes` under the documented budget must satisfy: an
    /// index that writes back as exactly `bytes`, or an error other than a
    /// spent budget.
    fn check(bytes: &[u8]) -> Result<(), TestCaseError> {
        let result = Index::from_bytes(bytes, &mut documented(bytes));
        let spent = matches!(
            result,
            Err(IndexError::Fault(ParseFault::BudgetExceeded { .. }))
        );
        prop_assert!(!spent);
        let written = result.map(|index| index.to_bytes());
        prop_assert!(written.is_err() || written.as_deref() == Ok(bytes));
        Ok(())
    }

    proptest! {
        #[test]
        fn an_index_reads_back_from_its_segment(docs in docs()) {
            let index = Index::build(docs.into_iter());
            let bytes = index.to_bytes();
            prop_assert_eq!(Index::from_bytes(&bytes, &mut documented(&bytes)), Ok(index));
        }

        /// Verifies: SEC-MED-001, SEC-MED-007
        #[test]
        fn any_octets_are_read_or_refused_within_the_documented_budget(
            bytes in vec(any::<u8>(), 0..200),
        ) {
            check(&bytes)?;
            check(&joined(&[HEAD, &bytes]))?;
        }

        /// Verifies: SEC-MED-001, SEC-MED-007
        #[test]
        fn a_segment_with_one_octet_changed_is_read_or_refused(
            docs in docs(),
            at in any::<prop::sample::Index>(),
            octet in any::<u8>(),
        ) {
            let mut bytes = Index::build(docs.into_iter()).to_bytes();
            let at = at.index(bytes.len());
            bytes[at] = octet;
            check(&bytes)?;
        }
    }
}
