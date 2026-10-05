//! The `INFO` list of a WAV file mapped onto
//! [`TrackTags`](crate::catalog::TrackTags).
//!
//! The WAV parser ([`crate::formats::riff`]) reports where the sub-chunks
//! of the `LIST` chunk of type `INFO` are; [`from_info`] reads those octets.
//! Each sub-chunk is a four-octet ID, a 32-bit little-endian size, the
//! value, and one pad octet after a value of odd length (Microsoft's
//! Multimedia Programming Interface and Data Specifications 1.0). A value
//! is text that ends at its first zero octet. It is read as UTF-8, or as
//! ISO 8859-1 when it is not valid UTF-8.
//!
//! The sub-chunks mapped are `INAM` (title), `IART` (artist), `IPRD`
//! (album), `ICRD` (date), `IGNR` (genre), `ITRK` (track number, alone or
//! as `3/12`) and `IMUS` (composer). `ISRC` is not a recording code here:
//! the specification uses it for who supplied the file. The values follow
//! the field rules in [`super::mp4`].
//!
//! An `id3 ` or `ID3 ` chunk in a WAV or AIFF file is not read here: the
//! probe hands it to the `ID3v2` parser like any other `ID3v2` tag.
//!
//! # Damaged lists
//!
//! When a sub-chunk's header or value runs past the end of the list, the
//! list holds more sub-chunks than the tag-field limit allows
//! (SEC-MED-006) or the step budget is spent (SEC-MED-007), the walk stops
//! and what was read before is kept, with the reason in
//! [`InfoTags::stopped`]. A missing pad octet after the last value is not
//! an error. A value longer than the long-text limit is dropped before it
//! is decoded, with a recorded problem. No parser decodes these values
//! before this mapper does, so none arrives already cut: a value is dropped
//! whole here, or it is cut by the field rules, which record the cut.
//!
//! # Work
//!
//! Each sub-chunk costs one step of the [`Budget`], charged before its
//! header is read. A sub-chunk takes at least eight octets, and a final
//! run of fewer than eight costs one step more, so a list of `n` octets
//! costs at most `n / 8 + 1` steps: [`STEPS_PER_OCTET`] × `n` +
//! [`STEPS_FIXED`] is always enough. Each sub-chunk moves the walk on by
//! at least its header (SEC-MED-008), and nothing is allocated from a
//! declared size: a value is a slice of the list (SEC-MED-003).

use super::mp4::{Fields, Mapped, Reason, TagField, lookup};
use crate::catalog::Role;
use crate::parse::{Budget, Cursor, LimitKind, Limits, ParseFault};

/// The step budget's charge for each octet of the list: see the module
/// documentation.
pub const STEPS_PER_OCTET: u64 = 1;

/// The step budget's charge whatever the list's length.
pub const STEPS_FIXED: u64 = 1;

/// The sub-chunk a value was read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InfoChunk {
    /// The sub-chunk's ID.
    pub id: [u8; 4],
    /// Where the sub-chunk's header starts, from the start of the file.
    pub offset: u64,
}

/// The tags of an `INFO` list.
#[derive(Debug, Clone, PartialEq)]
pub struct InfoTags {
    /// The tags read before the walk ended.
    pub mapped: Mapped<InfoChunk>,
    /// Why the walk stopped before the end of the list, if it did.
    pub stopped: Option<ParseFault>,
}

/// The sub-chunks that are mapped.
const KEYS: &[(&str, TagField)] = &[
    ("INAM", TagField::Title),
    ("IART", TagField::Artist),
    ("IPRD", TagField::Album),
    ("ICRD", TagField::Date),
    ("IGNR", TagField::Genres),
    ("ITRK", TagField::Track),
    ("IMUS", TagField::Credit(Role::Composer)),
];

/// Maps the sub-chunks of an `INFO` list onto
/// [`TrackTags`](crate::catalog::TrackTags).
///
/// `list` holds the sub-chunks, after the list's type, and `offset` is
/// where they start in the file: the range in
/// [`WavFile::info`](crate::formats::riff::WavFile::info).
#[must_use]
pub fn from_info(list: &[u8], offset: u64, limits: &Limits, budget: &mut Budget) -> InfoTags {
    let mut fields = Fields::new(limits);
    let stopped = walk(&mut fields, Cursor::at(list, offset), limits, budget).err();
    InfoTags {
        mapped: fields.finish(),
        stopped,
    }
}

/// Reads every sub-chunk `list` holds into `fields`.
fn walk(
    fields: &mut Fields<'_, InfoChunk>,
    mut list: Cursor<'_>,
    limits: &Limits,
    budget: &mut Budget,
) -> Result<(), ParseFault> {
    let mut count = 0_u64;
    while !list.is_empty() {
        let offset = list.offset();
        budget.charge(1, offset)?;
        count = count.saturating_add(1);
        limits.check(LimitKind::TagFields, count, offset)?;
        let id = list.array()?;
        let size = u64::from(list.u32_le()?);
        let body = list.take(size)?;
        // The pad octet after a value of odd length. The last value of a
        // list may lack it.
        let _ = list.skip(size & 1);
        let field = core::str::from_utf8(&id)
            .ok()
            .and_then(|id| lookup(KEYS, id));
        if let Some(field) = field {
            value(fields, field, body, InfoChunk { id, offset }, limits);
        }
    }
    Ok(())
}

/// Gives `field` the text of the sub-chunk body `body`.
fn value(
    fields: &mut Fields<'_, InfoChunk>,
    field: TagField,
    body: &[u8],
    source: InfoChunk,
    limits: &Limits,
) {
    let octets = body.split(|octet| *octet == 0).next().unwrap_or_default();
    let len = u64::try_from(octets.len()).unwrap_or(u64::MAX);
    if limits
        .check(LimitKind::LongText, len, source.offset)
        .is_err()
    {
        fields.note(field, source, Reason::Limit(LimitKind::LongText));
        return;
    }
    if let Ok(text) = core::str::from_utf8(octets) {
        fields.set(field, text, source);
    } else {
        let text: String = octets.iter().copied().map(char::from).collect();
        fields.set(field, &text, source);
    }
}

#[cfg(test)]
mod tests {
    use super::super::mp4::TagProblem;
    use super::*;
    use crate::catalog::{Credit, TrackPosition, TrackTags};
    use crate::values::PartialDate;
    use proptest::collection::vec;
    use proptest::prelude::*;

    /// One sub-chunk, built here from the specification's layout: the ID,
    /// the size, the value and a pad octet after an odd length.
    fn chunk(id: [u8; 4], value: &[u8]) -> Vec<u8> {
        let size = u32::try_from(value.len()).expect("the test's value is short");
        let mut bytes = id.to_vec();
        bytes.extend_from_slice(&size.to_le_bytes());
        bytes.extend_from_slice(value);
        if value.len() % 2 == 1 {
            bytes.push(0);
        }
        bytes
    }

    fn read_at(list: &[u8], offset: u64) -> InfoTags {
        let mut budget = Budget::for_input(0, 0, 1_000);
        from_info(list, offset, &Limits::DEFAULT, &mut budget)
    }

    fn read(list: &[u8]) -> InfoTags {
        read_at(list, 0)
    }

    fn lowered(kind: LimitKind, value: u64) -> Limits {
        Limits::DEFAULT
            .with_override(kind, value)
            .expect("the test lowers a limit")
    }

    fn at(id: [u8; 4], offset: u64) -> InfoChunk {
        InfoChunk { id, offset }
    }

    /// What a walk to the end of the list gives when only `tags` were
    /// read, each from the sub-chunk beside it.
    fn whole(tags: TrackTags, sources: Vec<(TagField, InfoChunk)>) -> InfoTags {
        InfoTags {
            mapped: Mapped {
                tags,
                sources,
                problems: Vec::new(),
            },
            stopped: None,
        }
    }

    fn titled(title: &str, offset: u64) -> InfoTags {
        whole(
            TrackTags {
                title: Some(String::from(title)),
                ..TrackTags::default()
            },
            vec![(TagField::Title, at(*b"INAM", offset))],
        )
    }

    #[test]
    fn the_chunk_builder_writes_the_specifications_layout() {
        assert_eq!(chunk(*b"INAM", b"ab"), b"INAM\x02\x00\x00\x00ab");
        assert_eq!(chunk(*b"IART", b"abc"), b"IART\x03\x00\x00\x00abc\x00");
    }

    #[test]
    fn maps_every_sub_chunk_with_where_it_was_read() {
        let list = [
            chunk(*b"INAM", b"Blue in Green\0"),
            chunk(*b"IART", b"Miles Davis\0"),
            chunk(*b"IPRD", b"Kind of Blue\0"),
            chunk(*b"ICRD", b"1959-08-17\0"),
            chunk(*b"IGNR", b"Jazz\0"),
            chunk(*b"ITRK", b"3/5\0"),
            chunk(*b"IMUS", b"Bill Evans\0"),
            chunk(*b"ISFT", b"an encoder\0"),
            chunk(*b"ISRC", b"USS1Z9900001\0"),
        ]
        .concat();
        let composer = TagField::Credit(Role::Composer);
        assert_eq!(
            read_at(&list, 100),
            whole(
                TrackTags {
                    title: Some(String::from("Blue in Green")),
                    artist: vec![String::from("Miles Davis")],
                    album: Some(String::from("Kind of Blue")),
                    position: TrackPosition::new(Some(3), Some(5), None, None)
                        .expect("the position is in range"),
                    date: Some(PartialDate::new(1959, Some(8), Some(17)).expect("the date exists")),
                    genres: vec![String::from("Jazz")],
                    credits: vec![
                        Credit::new(String::from("Bill Evans"), Role::Composer, None, None)
                            .expect("the name is not blank")
                    ],
                    ..TrackTags::default()
                },
                vec![
                    (TagField::Title, at(*b"INAM", 100)),
                    (TagField::Artist, at(*b"IART", 122)),
                    (TagField::Album, at(*b"IPRD", 142)),
                    (TagField::Date, at(*b"ICRD", 164)),
                    (TagField::Genres, at(*b"IGNR", 184)),
                    (TagField::Track, at(*b"ITRK", 198)),
                    (TagField::TrackTotal, at(*b"ITRK", 198)),
                    (composer, at(*b"IMUS", 210)),
                ],
            )
        );
    }

    #[test]
    fn skips_the_pad_octet_after_an_odd_length_value() {
        let list = b"INAM\x03\x00\x00\x00abc\x00IART\x02\x00\x00\x00de";
        assert_eq!(
            read(list),
            whole(
                TrackTags {
                    title: Some(String::from("abc")),
                    artist: vec![String::from("de")],
                    ..TrackTags::default()
                },
                vec![
                    (TagField::Title, at(*b"INAM", 0)),
                    (TagField::Artist, at(*b"IART", 12)),
                ],
            )
        );
    }

    #[test]
    fn an_even_length_value_has_no_pad_octet() {
        let list = b"IART\x02\x00\x00\x00deINAM\x04\x00\x00\x00abcd";
        assert_eq!(
            read(list),
            whole(
                TrackTags {
                    title: Some(String::from("abcd")),
                    artist: vec![String::from("de")],
                    ..TrackTags::default()
                },
                vec![
                    (TagField::Artist, at(*b"IART", 0)),
                    (TagField::Title, at(*b"INAM", 10)),
                ],
            )
        );
    }

    #[test]
    fn the_last_value_may_lack_its_pad_octet() {
        assert_eq!(read(b"INAM\x03\x00\x00\x00abc"), titled("abc", 0));
    }

    #[test]
    fn a_value_ends_at_its_first_zero_octet() {
        assert_eq!(read(&chunk(*b"INAM", b"ab\0zz\0")), titled("ab", 0));
        assert_eq!(
            read(&chunk(*b"INAM", b"\0ab")),
            whole(TrackTags::default(), Vec::new())
        );
    }

    #[test]
    fn reads_utf8_and_falls_back_to_latin1() {
        assert_eq!(read(&chunk(*b"INAM", b"Caf\xC3\xA9")), titled("Café", 0));
        assert_eq!(read(&chunk(*b"INAM", b"Caf\xE9")), titled("Café", 0));
    }

    #[test]
    fn ignores_ids_it_does_not_map() {
        let list = [
            chunk(*b"\xFF\xFE\xFD\xFC", b"ab"),
            chunk(*b"IENG", b"cd"),
            chunk(*b"INAM", b""),
            chunk(*b"INAM", b"ef"),
        ]
        .concat();
        assert_eq!(read(&list), titled("ef", 28));
    }

    #[test]
    fn an_empty_list_maps_to_no_tags() {
        assert_eq!(read(b""), whole(TrackTags::default(), Vec::new()));
    }

    #[test]
    fn keeps_what_it_read_before_a_header_cut_short() {
        let mut list = chunk(*b"INAM", b"ab");
        list.extend_from_slice(b"IAR");
        let mut expected = titled("ab", 50);
        expected.stopped = Some(ParseFault::Truncated {
            offset: 60,
            needed: 4,
            available: 3,
        });
        assert_eq!(read_at(&list, 50), expected);

        let mut list = chunk(*b"INAM", b"ab");
        list.extend_from_slice(b"IART\x02\x00");
        let mut expected = titled("ab", 50);
        expected.stopped = Some(ParseFault::Truncated {
            offset: 64,
            needed: 4,
            available: 2,
        });
        assert_eq!(read_at(&list, 50), expected);
    }

    #[test]
    fn stops_at_a_value_that_runs_past_the_list() {
        let mut list = chunk(*b"INAM", b"ab");
        list.extend_from_slice(b"IART\x0A\x00\x00\x00abc");
        let mut expected = titled("ab", 0);
        expected.stopped = Some(ParseFault::Truncated {
            offset: 18,
            needed: 10,
            available: 3,
        });
        assert_eq!(read(&list), expected);
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn reads_as_many_sub_chunks_as_the_tag_field_limit_and_no_more() {
        let limits = lowered(LimitKind::TagFields, 2);
        let two = [chunk(*b"ISFT", b"ab"), chunk(*b"INAM", b"cd")].concat();
        let mut budget = Budget::for_input(0, 0, 100);
        assert_eq!(from_info(&two, 0, &limits, &mut budget), titled("cd", 10));

        let three = [two, chunk(*b"IART", b"ef")].concat();
        let mut expected = titled("cd", 10);
        expected.stopped = Some(ParseFault::LimitExceeded {
            limit: LimitKind::TagFields,
            value: 3,
            max: 2,
            offset: 20,
        });
        let mut budget = Budget::for_input(0, 0, 100);
        assert_eq!(from_info(&three, 0, &limits, &mut budget), expected);
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn drops_a_value_longer_than_the_long_text_limit_before_decoding_it() {
        let limits = lowered(LimitKind::LongText, 4);
        let list = [
            chunk(*b"INAM", b"abcd\0"),
            chunk(*b"IPRD", b"abcde"),
            chunk(*b"IART", b"ef"),
        ]
        .concat();
        let mut budget = Budget::for_input(0, 0, 100);
        assert_eq!(
            from_info(&list, 0, &limits, &mut budget),
            InfoTags {
                mapped: Mapped {
                    tags: TrackTags {
                        title: Some(String::from("abcd")),
                        artist: vec![String::from("ef")],
                        ..TrackTags::default()
                    },
                    sources: vec![
                        (TagField::Title, at(*b"INAM", 0)),
                        (TagField::Artist, at(*b"IART", 28)),
                    ],
                    problems: vec![TagProblem::Value {
                        field: TagField::Album,
                        source: at(*b"IPRD", 14),
                        reason: Reason::Limit(LimitKind::LongText),
                    }],
                },
                stopped: None,
            }
        );
    }

    #[test]
    fn charges_one_step_for_each_sub_chunk() {
        let list = [chunk(*b"INAM", b"ab"), chunk(*b"IART", b"cd")].concat();
        let mut enough = Budget::for_input(0, 0, 2);
        assert_eq!(
            from_info(&list, 0, &Limits::DEFAULT, &mut enough),
            whole(
                TrackTags {
                    title: Some(String::from("ab")),
                    artist: vec![String::from("cd")],
                    ..TrackTags::default()
                },
                vec![
                    (TagField::Title, at(*b"INAM", 0)),
                    (TagField::Artist, at(*b"IART", 10)),
                ],
            )
        );
        assert_eq!(enough.remaining(), 0);

        let mut short = Budget::for_input(0, 0, 1);
        let mut expected = titled("ab", 0);
        expected.stopped = Some(ParseFault::BudgetExceeded { offset: 10 });
        assert_eq!(from_info(&list, 0, &Limits::DEFAULT, &mut short), expected);
    }

    proptest! {
        #[test]
        fn any_octets_are_read_within_the_documented_steps(
            list in prop_oneof![
                vec(any::<u8>(), 0..96),
                vec(
                    (
                        prop_oneof![Just(*b"INAM"), Just(*b"IGNR"), Just(*b"ITRK"), any::<[u8; 4]>()],
                        vec(any::<u8>(), 0..12),
                    ),
                    0..8,
                )
                .prop_map(|chunks| chunks.iter().flat_map(|(id, value)| chunk(*id, value)).collect()),
            ]
        ) {
            let len = u64::try_from(list.len()).expect("the list is short");
            let allowed = Budget::for_input(len, STEPS_PER_OCTET, STEPS_FIXED).remaining();
            let mut budget = Budget::for_input(len, STEPS_PER_OCTET, STEPS_FIXED);
            let read = from_info(&list, 0, &Limits::DEFAULT, &mut budget);
            let spent = allowed.checked_sub(budget.remaining()).expect("a budget only shrinks");
            prop_assert!(spent <= (len / 8).saturating_add(1));
            let over = matches!(read.stopped, Some(ParseFault::BudgetExceeded { .. }));
            prop_assert!(!over);
        }
    }
}
