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
//!             postings:  varint count of 1 or more, then per posting, in
//!                        ascending order of document and then place
//!               doc:     varint, added to the posting before it
//!               place:   u8
//! ```
//!
//! A varint is a `u32` in groups of seven bits, lowest first, the high bit
//! of each octet saying that another follows, in the fewest octets that
//! hold it. A term's postings are a set: each pair of document and place
//! is written once, in strictly ascending order of document and then of
//! place, and a term with no posting is not written at all. Every set of
//! documents, terms and postings therefore has exactly one written form,
//! and [`Index::from_bytes`] accepts no other.
//!
//! One written form is not a promise that the index is one
//! [`Index::build`] makes. The reader does not check that a term is a
//! token the index would cut from any text, or that a document's
//! `title_len` agrees with the places its postings name. A query takes
//! both as it finds them.
//!
//! A segment is untrusted input. A count it declares sizes nothing: each
//! document, term and posting is read and kept one at a time, and each
//! takes at least two octets, so memory stays in proportion to the input.
//!
//! # Memory
//!
//! [`Index::from_bytes`] takes a step budget and no other limit. Nothing
//! caps the documents, terms or postings of a segment, or its size, so the
//! caller must cap the size of a segment before it reads one: at the
//! memory it can spare for one index, divided by the multiple below.
//!
//! The index is held in proportion to the segment, but it is larger. On a
//! 64-bit target, worked out from the layout of what is held and not
//! measured:
//!
//! - A posting is 2 octets or more in the segment and 8 in memory, in a
//!   vector that may have room for twice as many: up to 8 times its
//!   octets.
//! - A document is 32 octets or more in the segment and 24 in memory, in
//!   a vector of the same kind: under twice its octets.
//! - A term is 5 octets or more in the segment with its first posting. It
//!   takes 48 octets in the map of terms, which may have about as much
//!   room again to spare, a copy of its text, and a vector with room for
//!   four postings (32 octets), the last two each in an allocation of
//!   their own: about 190 octets with the allocator's overhead, or 40
//!   times its octets.
//!
//! So count on **40 octets of memory for each octet of segment**. That is
//! the worst case, a segment of nothing but short terms with one posting
//! each; a segment that is mostly postings stays under 8. While a vector
//! grows, its old and new allocations are both held for a moment, which
//! can add half as much again for the largest term.
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
    /// A term has no postings.
    NoPostings {
        /// Where the term's entry starts.
        offset: u64,
    },
    /// A posting names a document the segment does not hold.
    UnknownDoc {
        /// Where the posting starts.
        offset: u64,
    },
    /// A posting does not sort after the posting before it: it names the
    /// same document and place again, or an earlier place of the same
    /// document.
    PostingOrder {
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
    /// `budget` bounds the steps and nothing else. No limit caps the
    /// documents, terms or postings of a segment, or its size, and the
    /// index is held in up to 40 octets of memory for each octet of
    /// `bytes`; the module documentation has the sum, under "Memory". The
    /// caller must cap the size of `bytes` before it calls.
    ///
    /// # Errors
    ///
    /// Returns an [`IndexError`] unless `bytes` is exactly what
    /// [`Index::to_bytes`] writes for some index whose every term has a
    /// posting, with its postings in strictly ascending order of document
    /// and then place, or when `budget` runs out first.
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
            let listed = varint(&mut cursor)?;
            if listed == 0 {
                return Err(IndexError::NoPostings { offset });
            }
            let mut postings: Vec<Posting> = Vec::new();
            let mut doc = 0_u32;
            for _ in 0..listed {
                let offset = cursor.offset();
                budget.charge(1, offset)?;
                doc = doc
                    .checked_add(varint(&mut cursor)?)
                    .filter(|doc| *doc < docs)
                    .ok_or(IndexError::UnknownDoc { offset })?;
                let place = cursor.u8()?;
                // One written form: each pair once, in ascending order.
                let ascends = postings
                    .last()
                    .is_none_or(|last| (last.doc, last.place) < (doc, place));
                if !ascends {
                    return Err(IndexError::PostingOrder { offset });
                }
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
    use std::collections::{BTreeMap, BTreeSet};

    use super::super::doc::{DocKind, SearchDoc};
    use super::super::index::{ELSEWHERE, tokens};
    use super::super::query::{Hit, KindFilter, Match};
    use super::super::testing::{doc_ref, id_text, owned, titled};
    use super::*;
    use proptest::collection::{btree_map, vec};
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
    fn two_items() -> [SearchDoc; 2] {
        let mut track = titled(DocKind::Track, 1, "Go");
        track.artist = String::from("Öl");
        track.credits = owned(&["Öl"]);
        track.plays = 300;
        [track, titled(DocKind::Artist, 2, "Go Go")]
    }

    /// The index of [`two_items`].
    fn two_docs() -> Index {
        Index::build(two_items().into_iter())
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

    /// `value` as a varint, written a second way for the tests: the low
    /// seven bits of what is left, for as long as more than seven are left.
    fn reference_varint(value: u32) -> Vec<u8> {
        let mut out = Vec::new();
        let mut rest = value;
        while rest >= 128 {
            out.push(u8::try_from(rest.rem_euclid(128)).unwrap() | 0x80);
            rest = rest.div_euclid(128);
        }
        out.push(u8::try_from(rest).unwrap());
        out
    }

    /// A segment written from its parts as they are given, whether or not a
    /// reader would take them: for each document its identifier, plays and
    /// title length, and for each term its postings, each a step from the
    /// document of the posting before and a place.
    fn raw_segment(
        docs: &[(String, u32, u8)],
        terms: &BTreeMap<String, Vec<(u32, u8)>>,
    ) -> Vec<u8> {
        let mut out = joined(&[HEAD, &reference_varint(u32::try_from(docs.len()).unwrap())]);
        for (id, plays, title_len) in docs {
            out.extend(id.bytes());
            out.extend(reference_varint(*plays));
            out.push(*title_len);
        }
        out.extend(reference_varint(u32::try_from(terms.len()).unwrap()));
        for (term, postings) in terms {
            out.push(u8::try_from(term.len()).unwrap());
            out.extend(term.bytes());
            out.extend(reference_varint(u32::try_from(postings.len()).unwrap()));
            for (step, place) in postings {
                out.extend(reference_varint(*step));
                out.push(*place);
            }
        }
        out
    }

    /// The segment of the index of `docs`, worked out apart from
    /// [`Index::build`] and [`Index::to_bytes`]: a token's postings are the
    /// set of its pairs of document and place, in ascending order. The
    /// place of a title's token is its position, held at 255, and the place
    /// of any other field's token is 255.
    fn reference_segment(docs: &[SearchDoc]) -> Vec<u8> {
        let mut sets: BTreeMap<String, BTreeSet<(u32, u8)>> = BTreeMap::new();
        let mut entries = Vec::new();
        for (number, doc) in (0_u32..).zip(docs) {
            let title = tokens(&doc.title);
            for (position, token) in (0_usize..).zip(&title) {
                let place = u8::try_from(position).unwrap_or(u8::MAX);
                sets.entry(token.clone())
                    .or_default()
                    .insert((number, place));
            }
            let rest = [&doc.artist, &doc.album]
                .into_iter()
                .chain(&doc.credits)
                .chain(&doc.genres)
                .chain(&doc.labels);
            for text in rest {
                for token in tokens(text) {
                    sets.entry(token).or_default().insert((number, u8::MAX));
                }
            }
            entries.push((
                doc.doc.id().to_string(),
                doc.plays,
                u8::try_from(title.len()).unwrap_or(u8::MAX),
            ));
        }
        let terms: BTreeMap<String, Vec<(u32, u8)>> = sets
            .into_iter()
            .map(|(term, pairs)| {
                let mut before = 0_u32;
                let steps: Vec<(u32, u8)> = pairs
                    .into_iter()
                    .map(|(number, place)| {
                        let step = number.checked_sub(before).unwrap();
                        before = number;
                        (step, place)
                    })
                    .collect();
                (term, steps)
            })
            .collect();
        raw_segment(&entries, &terms)
    }

    /// Whether every term of `index` has a posting, and has its postings
    /// in strictly ascending order of document and then place.
    fn in_one_form(index: &Index) -> bool {
        index.terms.values().all(|postings| {
            let pairs: Vec<(u32, u8)> = postings
                .iter()
                .map(|posting| (posting.doc, posting.place))
                .collect();
            !pairs.is_empty()
                && pairs
                    .iter()
                    .zip(pairs.iter().skip(1))
                    .all(|(before, after)| before < after)
        })
    }

    #[test]
    fn writes_the_segment_octet_for_octet() {
        assert_eq!(two_docs().to_bytes(), SEGMENT);
    }

    #[test]
    fn the_reference_writers_write_the_segment_octet_for_octet() {
        let docs = [
            (id_text(DocKind::Track, 1), 300, 1),
            (id_text(DocKind::Artist, 2), 0, 2),
        ];
        // "go" at (0, 0), (1, 0) and (1, 1), as steps of 0, 1 and 0.
        let terms = BTreeMap::from([
            (String::from("go"), vec![(0, 0), (1, 0), (0, 1)]),
            (String::from("ol"), vec![(0, 0xFF)]),
        ]);
        assert_eq!(raw_segment(&docs, &terms), SEGMENT);
        assert_eq!(reference_segment(&two_items()), SEGMENT);
    }

    #[test]
    fn tells_postings_in_their_one_form_from_the_others() {
        let with = |pairs: &[(u32, u8)]| Index {
            docs: Vec::new(),
            terms: BTreeMap::from([(
                String::from("a"),
                pairs
                    .iter()
                    .map(|&(doc, place)| Posting { doc, place })
                    .collect(),
            )]),
        };
        assert!(in_one_form(&Index::default()));
        assert!(in_one_form(&two_docs()));
        assert!(in_one_form(&with(&[(0, 0), (0, 1), (1, 0)])));
        // No posting, a pair twice, an earlier place, an earlier document.
        assert!(!in_one_form(&with(&[])));
        assert!(!in_one_form(&with(&[(0, 0), (0, 0)])));
        assert!(!in_one_form(&with(&[(0, 1), (0, 0)])));
        assert!(!in_one_form(&with(&[(1, 0), (0, 0)])));
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

    /// An album "÷" and an artist "!!!": names with no letter or digit,
    /// whose terms are the names as they are written.
    #[test]
    fn writes_and_reads_names_with_no_letter_or_digit() {
        let index = Index::build(
            [
                titled(DocKind::Album, 1, "÷"),
                titled(DocKind::Artist, 2, "!!!"),
            ]
            .into_iter(),
        );
        let segment: &[u8] = b"GMSI\x01\
            \x02\
            alb_00000000000000000000000001\x00\x01\
            art_00000000000000000000000002\x00\x01\
            \x02\
            \x03!!!\x01\x01\x00\
            \x02\xC3\xB7\x01\x00\x00";
        assert_eq!(index.to_bytes(), segment);
        let back = read(segment).unwrap();
        assert_eq!(
            back.docs,
            [
                Entry {
                    doc: doc_ref(DocKind::Album, 1),
                    plays: 0,
                    title_len: 1
                },
                Entry {
                    doc: doc_ref(DocKind::Artist, 2),
                    plays: 0,
                    title_len: 1
                },
            ]
        );
        assert_eq!(
            back.terms,
            BTreeMap::from([
                (String::from("!!!"), vec![Posting { doc: 1, place: 0 }]),
                (String::from("÷"), vec![Posting { doc: 0, place: 0 }]),
            ])
        );
        assert_eq!(back, index);
        assert_eq!(
            back.query("÷", KindFilter::All, u16::MAX),
            [Hit {
                doc: doc_ref(DocKind::Album, 1),
                matched: Match::WholeTitle
            }]
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
            // The tests' second writer agrees with the table too.
            assert_eq!(reference_varint(value), octets);
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

    /// One document, the track [`TRACK_1`], and one term for it: `len`
    /// octets of length, `term`, and a posting for the title's first place.
    fn one_term(len: u8, term: &[u8]) -> Vec<u8> {
        joined(&[HEAD, &[1], TRACK_1, &[0, 1], &[1], &[len], term, &[1, 0, 0]])
    }

    /// The terms of a segment that holds `term` alone, with the posting
    /// [`one_term`] gives it.
    fn only(term: &str) -> BTreeMap<String, Vec<Posting>> {
        BTreeMap::from([(String::from(term), vec![Posting { doc: 0, place: 0 }])])
    }

    #[test]
    fn reads_a_term_of_1_to_32_characters() {
        assert_eq!(
            read(&one_term(1, b"a")).map(|index| index.terms),
            Ok(only("a"))
        );
        let longest = "abcdefghijklmnopqrstuvwxyzabcdef";
        assert_eq!(
            read(&one_term(32, longest.as_bytes())).map(|index| index.terms),
            Ok(only(longest))
        );
        // Characters, not octets: twenty of two octets each.
        let wide = "éééééééééééééééééééé";
        assert_eq!(
            read(&one_term(40, wide.as_bytes())).map(|index| index.terms),
            Ok(only(wide))
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
        // After a term that reads, so the offset is the long one's.
        let long = joined(&[
            HEAD,
            &[1],
            TRACK_1,
            &[0, 1],
            &[2],
            &[1],
            b"a",
            &[1, 0, 0],
            &[33],
            b"abcdefghijklmnopqrstuvwxyzabcdefg",
            &[1, 0, 0],
        ]);
        assert_eq!(
            read(&long),
            Err(IndexError::TermLength {
                offset: 44,
                chars: 33
            })
        );
    }

    #[test]
    fn refuses_a_term_with_no_postings() {
        // No documents, and one term with a count of no postings.
        let alone = joined(&[HEAD, &[0], &[1], &[1], b"a", &[0]]);
        assert_eq!(read(&alone), Err(IndexError::NoPostings { offset: 7 }));
        // After a term that has one, so the offset is the second term's.
        let second = joined(&[
            HEAD,
            &[1],
            TRACK_1,
            &[0, 1],
            &[2],
            &[1],
            b"a",
            &[1, 0, 0],
            &[1],
            b"b",
            &[0],
        ]);
        assert_eq!(read(&second), Err(IndexError::NoPostings { offset: 44 }));
    }

    #[test]
    fn refuses_a_term_that_is_not_text() {
        let bytes = joined(&[HEAD, &[0], &[1], &[2], &[b'a', 0xFF], &[0]]);
        assert_eq!(read(&bytes), Err(IndexError::TermNotText { offset: 7 }));
    }

    #[test]
    fn refuses_terms_that_are_not_in_ascending_order() {
        // One document, and two terms with a posting for it each.
        let two = |first: &[u8], second: &[u8]| {
            joined(&[
                HEAD,
                &[1],
                TRACK_1,
                &[0, 1],
                &[2],
                &[1],
                first,
                &[1, 0, 0],
                &[1],
                second,
                &[1, 0, 0],
            ])
        };
        assert_eq!(
            read(&two(b"a", b"b")).map(|index| index.terms),
            Ok(BTreeMap::from([
                (String::from("a"), vec![Posting { doc: 0, place: 0 }]),
                (String::from("b"), vec![Posting { doc: 0, place: 0 }]),
            ]))
        );
        assert_eq!(
            read(&two(b"b", b"a")),
            Err(IndexError::TermOrder { offset: 44 })
        );
        assert_eq!(
            read(&two(b"b", b"b")),
            Err(IndexError::TermOrder { offset: 44 })
        );
    }

    #[test]
    fn reads_postings_only_in_ascending_order_of_document_and_place() {
        // One document, and a term with the postings given: a count, then
        // a step and a place for each.
        let one =
            |postings: &[u8]| joined(&[HEAD, &[1], TRACK_1, &[0, 3], &[1], &[1], b"a", postings]);
        // Places 0 and then 1 of document 0.
        assert_eq!(
            read(&one(&[2, 0, 0, 0, 1])).map(|index| index.terms),
            Ok(BTreeMap::from([(
                String::from("a"),
                vec![Posting { doc: 0, place: 0 }, Posting { doc: 0, place: 1 }]
            )]))
        );
        // The same place twice.
        assert_eq!(
            read(&one(&[2, 0, 0, 0, 0])),
            Err(IndexError::PostingOrder { offset: 44 })
        );
        // Place 1 and then place 0.
        assert_eq!(
            read(&one(&[2, 0, 1, 0, 0])),
            Err(IndexError::PostingOrder { offset: 44 })
        );
        // The third posting repeats the second, so the offset is its own.
        assert_eq!(
            read(&one(&[3, 0, 0, 0, 1, 0, 1])),
            Err(IndexError::PostingOrder { offset: 46 })
        );
        // Place 255, which is a title's 256th place and every other
        // field's, is written once like any other.
        assert_eq!(
            read(&one(&[2, 0, 0xFF, 0, 0xFF])),
            Err(IndexError::PostingOrder { offset: 44 })
        );
    }

    #[test]
    fn a_later_document_may_have_any_place() {
        // Two documents, and a term with the postings given.
        let two = |postings: &[u8]| {
            joined(&[
                HEAD,
                &[2],
                TRACK_1,
                &[0, 3],
                TRACK_2,
                &[0, 3],
                &[1],
                &[1],
                b"a",
                postings,
            ])
        };
        // Place 9 of document 0, then places 0 and 9 of document 1: the
        // place goes back where the document goes on.
        assert_eq!(
            read(&two(&[3, 0, 9, 1, 0, 0, 9])).map(|index| index.terms),
            Ok(BTreeMap::from([(
                String::from("a"),
                vec![
                    Posting { doc: 0, place: 9 },
                    Posting { doc: 1, place: 0 },
                    Posting { doc: 1, place: 9 },
                ]
            )]))
        );
        // Place 0 of document 1 twice.
        assert_eq!(
            read(&two(&[3, 0, 9, 1, 0, 0, 0])),
            Err(IndexError::PostingOrder { offset: 78 })
        );
    }

    /// A title of 257 words "a" by the artist "a": the word is at places 0
    /// to 254, and at place 255 for the title's 256th word, for its 257th
    /// and for the artist, which is one posting.
    #[test]
    fn writes_a_word_past_a_titles_255th_place_once_and_in_order() {
        let title = (0..257).map(|_| "a").collect::<Vec<&str>>().join(" ");
        let mut track = titled(DocKind::Track, 1, &title);
        track.artist = String::from("a");
        let index = Index::build([track].into_iter());
        // 256 postings, each a step of 0 and a place.
        let postings: Vec<u8> = (0..=u8::MAX).flat_map(|place| [0, place]).collect();
        let segment = joined(&[
            HEAD,
            &[1],
            TRACK_1,
            &[0, 0xFF],
            &[1],
            &[1],
            b"a",
            &[0x80, 0x02],
            &postings,
        ]);
        assert_eq!(index.to_bytes(), segment);
        assert_eq!(read(&segment), Ok(index));
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

    /// One generated document: its kind, title, artist, genres and plays.
    type Fields = (DocKind, String, String, Vec<String>, u32);

    /// `fields` as documents, numbered from 0.
    fn numbered(fields: Vec<Fields>) -> Vec<SearchDoc> {
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
        .prop_map(numbered)
    }

    /// Fewer than `most` words with spaces between, from six short ones.
    fn words(most: usize) -> impl Strategy<Value = String> {
        vec("[ab]{1,2}", 0..most).prop_map(|words| words.join(" "))
    }

    /// Documents whose fields share six words, with titles of up to 299
    /// of them, so that a word comes more than once in a title, past the
    /// title's 255th place, and in another field of the same document.
    fn wordy_docs() -> impl Strategy<Value = Vec<SearchDoc>> {
        vec(
            (
                kind(),
                words(300),
                words(4),
                vec(words(3), 0..3),
                any::<u32>(),
            ),
            0..4,
        )
        .prop_map(numbered)
    }

    /// One posting as a hostile sender writes it: a step from the document
    /// before, nearly always none, and a place, often one of the first few.
    fn near() -> impl Strategy<Value = (u32, u8)> {
        (
            prop_oneof![6 => Just(0_u32), 1 => Just(1_u32)],
            prop_oneof![0_u8..4, any::<u8>()],
        )
    }

    /// A segment written from parts a hostile sender picks: up to three
    /// documents, and up to five terms in ascending order, each with a few
    /// postings or hundreds. Their steps are mostly none, so that pairs of
    /// document and place repeat and places go backwards, and now and then
    /// any number at all, past the documents or past 32 bits.
    fn hostile() -> impl Strategy<Value = Vec<u8>> {
        let postings = prop_oneof![
            3 => vec(near(), 0..6),
            1 => vec(near(), 0..300),
            1 => vec(prop_oneof![9 => near(), 1 => (any::<u32>(), any::<u8>())], 0..40),
        ];
        (
            vec((kind(), any::<u32>(), any::<u8>()), 0..4),
            btree_map("[a-c]{1,3}", postings, 0..6),
        )
            .prop_map(|(docs, terms)| {
                let docs: Vec<(String, u32, u8)> = (0_u32..)
                    .zip(docs)
                    .map(|(number, (kind, plays, title_len))| {
                        (id_text(kind, number), plays, title_len)
                    })
                    .collect();
                raw_segment(&docs, &terms)
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

    /// Runs `work` on a fresh thread with the 256 KiB stack SEC-MED-001
    /// names, so a read that recursed too deeply would fail here.
    fn on_small_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(262_144)
            .spawn(work)
            .unwrap()
            .join()
            .unwrap()
    }

    /// What reading `bytes` on a small stack under the documented budget
    /// must satisfy: an index with its postings in their one form that
    /// writes back as exactly `bytes`, or an error other than a spent
    /// budget.
    fn check(bytes: Vec<u8>) -> Result<(), TestCaseError> {
        let (spent, malformed, written, bytes) = on_small_stack(move || {
            let result = Index::from_bytes(&bytes, &mut documented(&bytes));
            let spent = matches!(
                result,
                Err(IndexError::Fault(ParseFault::BudgetExceeded { .. }))
            );
            let malformed = result.as_ref().is_ok_and(|index| !in_one_form(index));
            let written = result.map(|index| index.to_bytes());
            (spent, malformed, written, bytes)
        });
        prop_assert!(!spent);
        prop_assert!(!malformed);
        prop_assert!(written.is_err() || written.as_deref() == Ok(bytes.as_slice()));
        Ok(())
    }

    /// The check the properties make runs both ways on every run: on a
    /// segment that reads, and on octets that are refused.
    #[test]
    fn the_check_takes_a_segment_and_a_refusal() {
        check(SEGMENT.to_vec()).unwrap();
        check(b"fLaC\x00\x00\x00\x22".to_vec()).unwrap();
    }

    /// The figures the module's notes on memory are worked out from.
    #[test]
    fn holds_a_posting_in_8_octets_and_a_document_in_24() {
        assert_eq!(std::mem::size_of::<Posting>(), 8);
        assert_eq!(std::mem::size_of::<Entry>(), 24);
    }

    proptest! {
        #[test]
        fn an_index_reads_back_from_its_segment(docs in docs()) {
            let index = Index::build(docs.into_iter());
            let bytes = index.to_bytes();
            prop_assert_eq!(Index::from_bytes(&bytes, &mut documented(&bytes)), Ok(index));
        }

        /// Every index `build` makes is written as the tests' second
        /// writer writes it, with each term's postings a set in ascending
        /// order, so the reader takes it.
        #[test]
        fn an_index_is_written_as_its_postings_in_order_and_reads_back(docs in wordy_docs()) {
            let index = Index::build(docs.clone().into_iter());
            let bytes = index.to_bytes();
            prop_assert_eq!(&bytes, &reference_segment(&docs));
            prop_assert!(in_one_form(&index));
            prop_assert_eq!(Index::from_bytes(&bytes, &mut documented(&bytes)), Ok(index));
        }

        /// Verifies: SEC-MED-001, SEC-MED-007
        #[test]
        fn any_octets_are_read_or_refused_within_the_documented_budget(
            bytes in vec(any::<u8>(), 0..200),
        ) {
            check(joined(&[HEAD, &bytes]))?;
            // After one document, so that the octets are read as terms
            // and postings and not as a document's identifier.
            check(joined(&[HEAD, &[1], TRACK_1, &[0, 1], &bytes]))?;
            check(bytes)?;
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
            check(bytes)?;
        }

        /// Verifies: SEC-MED-001, SEC-MED-007
        #[test]
        fn a_hostile_segment_is_read_or_refused_within_the_documented_budget(
            bytes in hostile(),
        ) {
            check(bytes)?;
        }
    }
}
