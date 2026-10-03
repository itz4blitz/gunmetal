//! Frame iteration: frame headers, the tag-field limit, the flags that
//! skip a frame or add octets before its body, the 2.4 size fallback, and
//! the walk through frames embedded in chapters.
//!
//! Every loop here reads at least one whole frame header per turn or stops,
//! and charges the budget for every frame it reads, so iteration always
//! ends (SEC-MED-008).

use crate::parse::{Budget, Cursor, Depth, LimitKind, Limits, ParseFault};

use super::body::terminated;
use super::frame::{Frame, FrameId};
use super::source::Source;
use super::tag::TagProblem;

/// How a 2.4 tag writes its frame sizes.
///
/// ID3v2.4.0 writes them as syncsafe integers, but some writers, iTunes
/// among them, wrote plain 32-bit integers as 2.3 does. A tag is read with
/// syncsafe sizes when every frame then fits, up to the end of the tag or
/// its padding; otherwise with plain sizes when every frame then fits;
/// otherwise with syncsafe sizes, as the specification says. This is the
/// fallback mutagen uses (`determine_bpi`), simplified to "all frames fit".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Sizes {
    /// Four octets of seven bits each.
    Syncsafe,
    /// A 32-bit integer, most significant octet first.
    Plain,
}

/// What one parse carries from frame to frame.
pub(super) struct Reader<'p> {
    /// The tag's major version: 2, 3 or 4.
    pub(super) major: u8,
    /// How frame sizes are written.
    pub(super) sizes: Sizes,
    /// Whether a 2.4 tag says every frame is unsynchronised.
    pub(super) unsynchronised: bool,
    /// The limits of this parse.
    pub(super) limits: &'p Limits,
    /// The steps left.
    pub(super) budget: &'p mut Budget,
    /// What was skipped or could not be read so far.
    pub(super) problems: Vec<TagProblem>,
    /// Pictures kept so far.
    pub(super) pictures: u64,
    /// Frames walked so far, kept or not, including those inside chapters.
    pub(super) walked: u64,
}

/// A frame whose header was read, with its body.
struct Found<'a> {
    /// The frame's identifier.
    id: FrameId,
    /// Where its header starts.
    offset: u64,
    /// Its flags, 0 in 2.2.
    flags: u16,
    /// Its body as stored.
    body: Source<'a>,
}

/// Why a frame header could not be read.
enum Unreadable {
    /// It was cut short, or its 2.4 size was not a syncsafe integer.
    Fault(ParseFault),
    /// Its identifier was not made of capital letters and digits.
    Id(FrameId),
}

/// 2.3 frame flags: compression, encryption and grouping identity.
const COMPRESSED_V23: u16 = 0x0080;
const ENCRYPTED_V23: u16 = 0x0040;
const GROUPED_V23: u16 = 0x0020;

/// 2.4 frame flags: grouping identity, compression, encryption,
/// unsynchronisation and data length indicator.
const GROUPED_V24: u16 = 0x0040;
const COMPRESSED_V24: u16 = 0x0008;
const ENCRYPTED_V24: u16 = 0x0004;
const UNSYNCHRONISED_V24: u16 = 0x0002;
const LENGTH_V24: u16 = 0x0001;

impl Reader<'_> {
    /// How the 2.4 frames in `area` write their sizes (see [`Sizes`]).
    ///
    /// # Errors
    ///
    /// Returns [`ParseFault::BudgetExceeded`] when the budget runs out.
    pub(super) fn sizes_of(&mut self, area: Source<'_>) -> Result<Sizes, ParseFault> {
        if self.fits(area, Sizes::Syncsafe)? {
            return Ok(Sizes::Syncsafe);
        }
        if self.fits(area, Sizes::Plain)? {
            return Ok(Sizes::Plain);
        }
        Ok(Sizes::Syncsafe)
    }

    /// Whether every 2.4 frame in `area` lies whole inside it when sizes
    /// are read as `sizes`, up to the end or to padding.
    fn fits(&mut self, mut area: Source<'_>, sizes: Sizes) -> Result<bool, ParseFault> {
        while area.first().is_some_and(|octet| octet != 0) {
            let offset = area.offset();
            let Ok((_, size, _)) = header(&mut area, 4, sizes) else {
                return Ok(false);
            };
            if area.split(size).is_err() {
                return Ok(false);
            }
            self.budget.charge(size.saturating_add(10), offset)?;
        }
        Ok(true)
    }

    /// Reads every frame in `source`, at `depth` below the tag's frames,
    /// keeping each one in `kept` when it is given and walking the frames
    /// embedded in chapters either way.
    ///
    /// # Errors
    ///
    /// Returns [`ParseFault::BudgetExceeded`] when the budget runs out.
    /// Everything else is recorded in the problems.
    pub(super) fn frames(
        &mut self,
        mut source: Source<'_>,
        depth: Depth,
        mut kept: Option<&mut Vec<Frame>>,
    ) -> Result<(), ParseFault> {
        while let Some(found) = self.next(&mut source)? {
            let Some(payload) = self.payload(&found) else {
                continue;
            };
            self.container(&found, payload, depth)?;
            if let Some(kept) = kept.as_deref_mut() {
                let body = self.body(found.id, found.offset, payload);
                kept.push(Frame {
                    id: found.id,
                    offset: found.offset,
                    flags: found.flags,
                    body,
                });
            }
        }
        Ok(())
    }

    /// Reads the next frame header in `source` and splits off the frame's
    /// body, or records why it cannot and returns `None`. At the end of the
    /// frames or at padding it returns `None`. One more past the tag-field
    /// limit, counting every frame walked so far including those inside
    /// chapters, is refused.
    fn next<'a>(&mut self, source: &mut Source<'a>) -> Result<Option<Found<'a>>, ParseFault> {
        if source.first().is_none_or(|octet| octet == 0) {
            return Ok(None);
        }
        let offset = source.offset();
        let count = self.walked.saturating_add(1);
        if let Err(fault) = self.limits.check(LimitKind::TagFields, count, offset) {
            self.problems.push(TagProblem::Fault(fault));
            return Ok(None);
        }
        let (id, size, flags) = match header(source, self.major, self.sizes) {
            Ok(header) => header,
            Err(Unreadable::Fault(fault)) => {
                self.problems.push(TagProblem::Fault(fault));
                return Ok(None);
            }
            Err(Unreadable::Id(id)) => {
                self.problems.push(TagProblem::BadFrameId { offset, id });
                return Ok(None);
            }
        };
        let body = match source.split(size) {
            Ok(body) => body,
            Err(fault) => {
                self.problems.push(TagProblem::Fault(fault));
                return Ok(None);
            }
        };
        let header_len = if self.major == 2 { 6 } else { 10 };
        self.budget
            .charge(size.saturating_add(header_len), offset)?;
        self.walked = count;
        Ok(Some(Found {
            id,
            offset,
            flags,
            body,
        }))
    }

    /// The frame's octets after those its flags add, read as its flags
    /// say, or `None` with the reason recorded for a frame that is
    /// skipped: empty, compressed, encrypted, or too short for what its
    /// flags add.
    fn payload<'a>(&mut self, found: &Found<'a>) -> Option<Source<'a>> {
        let Found {
            id, offset, flags, ..
        } = *found;
        if found.body.first().is_none() {
            self.problems.push(TagProblem::EmptyFrame { offset, id });
            return None;
        }
        let set = |flag: u16| flags & flag != 0;
        let (compressed, encrypted, extras, unsynchronised) = if self.major == 4 {
            let length = if set(LENGTH_V24) { 4 } else { 0 };
            (
                set(COMPRESSED_V24),
                set(ENCRYPTED_V24),
                u64::from(set(GROUPED_V24)).saturating_add(length),
                self.unsynchronised || set(UNSYNCHRONISED_V24),
            )
        } else {
            (
                set(COMPRESSED_V23),
                set(ENCRYPTED_V23),
                u64::from(set(GROUPED_V23)),
                false,
            )
        };
        if compressed {
            self.problems.push(TagProblem::Compressed { offset, id });
            return None;
        }
        if encrypted {
            self.problems.push(TagProblem::Encrypted { offset, id });
            return None;
        }
        let mut payload = found.body;
        if payload.split(extras).is_err() {
            self.problems.push(TagProblem::Malformed { offset, id });
            return None;
        }
        Some(payload.unsynchronised(unsynchronised))
    }

    /// Walks the frames embedded in a chapter (`CHAP`) or table of
    /// contents (`CTOC`) one level deeper than `depth`, and records a
    /// chapter whose own fields cannot be read. Other frames are left
    /// alone.
    fn container(
        &mut self,
        found: &Found<'_>,
        payload: Source<'_>,
        depth: Depth,
    ) -> Result<(), ParseFault> {
        let prefix: fn(&[u8]) -> Option<usize> = match found.id {
            FrameId::Four(id) if id == *b"CHAP" => chapter_prefix,
            FrameId::Four(id) if id == *b"CTOC" => contents_prefix,
            _ => return Ok(()),
        };
        let Some(len) = prefix(&payload.decode()) else {
            self.problems.push(TagProblem::Malformed {
                offset: found.offset,
                id: found.id,
            });
            return Ok(());
        };
        let embedded = payload.skip(u64::try_from(len).unwrap_or(u64::MAX));
        if embedded.first().is_none() {
            return Ok(());
        }
        match depth.descend(self.limits, embedded.offset()) {
            Ok(depth) => self.frames(embedded, depth, None),
            Err(fault) => {
                self.problems.push(TagProblem::Fault(fault));
                Ok(())
            }
        }
    }
}

/// Reads a frame header of the tag's `major` version from `source`.
fn header(
    source: &mut Source<'_>,
    major: u8,
    sizes: Sizes,
) -> Result<(FrameId, u64, u16), Unreadable> {
    let offset = source.offset();
    if major == 2 {
        let [a, b, c, size @ ..] = source.array::<6>().map_err(Unreadable::Fault)?;
        let id = FrameId::Three([a, b, c]);
        if !is_frame_id(&[a, b, c]) {
            return Err(Unreadable::Id(id));
        }
        let [high, middle, low] = size;
        return Ok((id, u64::from(u32::from_be_bytes([0, high, middle, low])), 0));
    }
    let [a, b, c, d, s0, s1, s2, s3, f0, f1] = source.array::<10>().map_err(Unreadable::Fault)?;
    let id = FrameId::Four([a, b, c, d]);
    if !is_frame_id(&[a, b, c, d]) {
        return Err(Unreadable::Id(id));
    }
    let size = [s0, s1, s2, s3];
    let size = match sizes {
        Sizes::Syncsafe => Cursor::at(&size, offset.saturating_add(4))
            .syncsafe_u32()
            .map_err(Unreadable::Fault)?,
        Sizes::Plain => u32::from_be_bytes(size),
    };
    Ok((id, u64::from(size), u16::from_be_bytes([f0, f1])))
}

/// Whether `octets` are a frame identifier: capital letters and digits.
pub(super) fn is_frame_id(octets: &[u8]) -> bool {
    octets
        .iter()
        .all(|octet| octet.is_ascii_uppercase() || octet.is_ascii_digit())
}

/// How many octets of a chapter's body come before its embedded frames:
/// the terminated element ID, then four 32-bit times and offsets.
fn chapter_prefix(body: &[u8]) -> Option<usize> {
    let (_, rest) = terminated(body, 1);
    let rest = rest?.get(16..)?;
    Some(body.len().saturating_sub(rest.len()))
}

/// How many octets of a table of contents come before its embedded
/// frames: the terminated element ID, the flags, the entry count, then
/// that many terminated child element IDs.
fn contents_prefix(body: &[u8]) -> Option<usize> {
    let (_, rest) = terminated(body, 1);
    let (&[_, count], mut rest) = rest?.split_first_chunk::<2>()?;
    for _ in 0..count {
        let (_, after) = terminated(rest, 1);
        rest = after?;
    }
    Some(body.len().saturating_sub(rest.len()))
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test layouts add small, bounded offsets"
)]
mod tests {
    use super::super::testing::{
        contents, frame, id, limits, raw, read_with, span, steps, text_body, truncated,
    };
    use super::super::{FrameBody, Id3v2Error, TagProblem};
    use crate::parse::{LimitKind, Limits, ParseFault};
    use gunmetal_testkit::id3v2::{
        self as kit, Encoding, Tag, Version, chapter, table_of_contents, unsynchronise,
    };

    /// A text frame body of one ISO-8859-1 value.
    fn latin1(value: &str) -> Vec<u8> {
        kit::text(Encoding::Latin1, &[value])
    }

    #[test]
    fn reads_the_frames_of_each_version_up_to_the_padding() {
        // 2.2: TT2 at 10 (6 + 2 octets), PRV at 18 (6 + 2), body 24 to 26.
        let v22 = Tag::new(Version::V22)
            .frame(b"TT2", 0, &latin1("A"))
            .frame(b"PRV", 0, b"xy")
            .padding(5)
            .build();
        assert_eq!(
            contents(&v22),
            (
                vec![
                    frame(b"TT2", 10, 0, text_body(&["A"])),
                    frame(b"PRV", 18, 0, raw(24, 26)),
                ],
                vec![]
            )
        );
        // 2.3 and 2.4: TIT2 at 10 (10 + 2), PRIV at 22 (10 + 2), body 32 to
        // 34, with status flags that change nothing.
        for version in [Version::V23, Version::V24] {
            let tag = Tag::new(version)
                .frame(b"TIT2", 0, &latin1("A"))
                .frame(b"PRIV", 0x6000, b"xy")
                .padding(5)
                .build();
            assert_eq!(
                contents(&tag),
                (
                    vec![
                        frame(b"TIT2", 10, 0, text_body(&["A"])),
                        frame(b"PRIV", 22, 0x6000, raw(32, 34)),
                    ],
                    vec![]
                ),
                "{version:?}"
            );
        }
    }

    #[test]
    fn reads_no_frames_from_a_tag_of_padding_alone() {
        for tag in [
            Tag::new(Version::V24).build(),
            Tag::new(Version::V23).padding(30).build(),
        ] {
            assert_eq!(contents(&tag), (vec![], vec![]));
        }
    }

    /// Verifies: SEC-MED-008
    #[test]
    fn stops_at_an_identifier_that_is_not_capital_letters_and_digits() {
        for bad in [
            *b"tit2",
            *b"TI-2",
            *b"TIT\x01",
            *b"T\x00\x00\x00",
            *b"\xC0IT2",
        ] {
            let tag = Tag::new(Version::V24)
                .frame(b"TIT2", 0, &latin1(""))
                .frame(&bad, 0, b"\x00A")
                .frame(b"TALB", 0, &latin1("B"))
                .build();
            assert_eq!(
                contents(&tag),
                (
                    vec![frame(b"TIT2", 10, 0, text_body(&[]))],
                    vec![TagProblem::BadFrameId {
                        offset: 21,
                        id: id(&bad)
                    }]
                ),
                "{bad:02X?}"
            );
        }
        let v22 = Tag::new(Version::V22).frame(b"tt2", 0, b"\x00A").build();
        assert_eq!(
            contents(&v22),
            (
                vec![],
                vec![TagProblem::BadFrameId {
                    offset: 10,
                    id: id(b"tt2")
                }]
            )
        );
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn records_a_frame_header_cut_short() {
        let v24 = Tag::new(Version::V24)
            .frame(b"TIT2", 0, &latin1(""))
            .bytes(b"TAL")
            .build();
        assert_eq!(
            contents(&v24),
            (
                vec![frame(b"TIT2", 10, 0, text_body(&[]))],
                vec![truncated(21, 10, 3)]
            )
        );
        let v22 = Tag::new(Version::V22).bytes(b"TA").build();
        assert_eq!(contents(&v22), (vec![], vec![truncated(10, 6, 2)]));
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn records_a_frame_larger_than_the_rest_of_the_tag() {
        let whole = kit::frame(Version::V24, b"TALB", 0, &[0x41; 100]);
        let tag = Tag::new(Version::V24)
            .frame(b"TIT2", 0, &latin1(""))
            .bytes(&whole[..13])
            .build();
        assert_eq!(
            contents(&tag),
            (
                vec![frame(b"TIT2", 10, 0, text_body(&[]))],
                vec![truncated(31, 100, 3)]
            )
        );
    }

    /// Verifies: SEC-MED-008
    #[test]
    fn skips_and_records_an_empty_frame() {
        for version in [Version::V22, Version::V24] {
            let (empty, next): (&[u8], &[u8]) = if version == Version::V22 {
                (b"TT2", b"TAL")
            } else {
                (b"TIT2", b"TALB")
            };
            let header_len = if version == Version::V22 { 6 } else { 10 };
            let tag = Tag::new(version)
                .frame(empty, 0, &[])
                .frame(next, 0, &latin1("B"))
                .build();
            assert_eq!(
                contents(&tag),
                (
                    vec![frame(next, 10 + header_len, 0, text_body(&["B"]))],
                    vec![TagProblem::EmptyFrame {
                        offset: 10,
                        id: id(empty)
                    }]
                ),
                "{version:?}"
            );
        }
    }

    /// Eleven-octet TIT2 frames with an empty value, `count` of them.
    fn many_frames(count: usize) -> Vec<u8> {
        let one = kit::frame(Version::V24, b"TIT2", 0, &latin1(""));
        let frames: Vec<u8> = (0..count).flat_map(|_| one.iter().copied()).collect();
        Tag::new(Version::V24).bytes(&frames).build()
    }

    /// The `count` frames [`many_frames`] writes, as read.
    fn many_read(count: u64) -> Vec<super::Frame> {
        (0..count)
            .map(|index| frame(b"TIT2", 10 + 11 * index, 0, text_body(&[])))
            .collect()
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn keeps_4096_frames_and_stops_at_the_4097th() {
        assert_eq!(contents(&many_frames(4_096)), (many_read(4_096), vec![]));
        assert_eq!(
            contents(&many_frames(4_097)),
            (
                many_read(4_096),
                vec![TagProblem::Fault(ParseFault::LimitExceeded {
                    limit: LimitKind::TagFields,
                    value: 4_097,
                    max: 4_096,
                    offset: 10 + 11 * 4_096,
                })]
            )
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn counts_frames_against_a_lowered_tag_field_limit() {
        let read = read_with(&many_frames(3), &limits(LimitKind::TagFields, 2));
        assert_eq!(
            read.map(|tag| (tag.frames, tag.problems)),
            Ok((
                many_read(2),
                vec![TagProblem::Fault(ParseFault::LimitExceeded {
                    limit: LimitKind::TagFields,
                    value: 3,
                    max: 2,
                    offset: 32,
                })]
            ))
        );
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn counts_frames_inside_a_chapter_against_the_tag_field_limit() {
        let empty = kit::frame(Version::V24, b"TIT2", 0, &[]);
        let inner: Vec<u8> = (0..4_097).flat_map(|_| empty.iter().copied()).collect();
        // CHAP at 10; body 20 to 41_008 (18 octets of fields, 4,097 empty
        // TIT2 frames of 10 octets). Embedded frames start at 38.
        let (frames, problems) = contents(&nested(1, &inner));
        assert_eq!(frames, vec![frame(b"CHAP", 10, 0, raw(20, 41_008))]);
        // The chapter is the first walked frame. The next 4,095 empty
        // inner frames fill the 4,096-field limit; the 4,097th overall
        // is refused, so the remaining empty frames are not recorded.
        let mut expected: Vec<TagProblem> = (0..4_095)
            .map(|index| TagProblem::EmptyFrame {
                offset: 38 + 10 * index,
                id: id(b"TIT2"),
            })
            .collect();
        expected.push(TagProblem::Fault(ParseFault::LimitExceeded {
            limit: LimitKind::TagFields,
            value: 4_097,
            max: 4_096,
            offset: 38 + 10 * 4_095,
        }));
        assert_eq!(problems, expected);
    }

    #[test]
    fn skips_and_records_compressed_and_encrypted_frames() {
        // 2.3: compression %10000000 (TIT2 at 10, 15 octets), encryption
        // %01000000 (TALB at 25, 12 octets), then TPE1 at 37.
        let v23 = Tag::new(Version::V23)
            .frame(b"TIT2", 0x0080, b"\x00\x00\x00\x09x")
            .frame(b"TALB", 0x0040, b"\x80\x01")
            .frame(b"TPE1", 0x0000, &latin1("C"))
            .build();
        // 2.4: compression %00001001 with its data length indicator,
        // encryption %00000100.
        let v24 = Tag::new(Version::V24)
            .frame(b"TIT2", 0x0009, b"\x00\x00\x00\x09x")
            .frame(b"TALB", 0x0004, b"\x80\x01")
            .frame(b"TPE1", 0x0000, &latin1("C"))
            .build();
        for tag in [v23, v24] {
            assert_eq!(
                contents(&tag),
                (
                    vec![frame(b"TPE1", 37, 0, text_body(&["C"]))],
                    vec![
                        TagProblem::Compressed {
                            offset: 10,
                            id: id(b"TIT2")
                        },
                        TagProblem::Encrypted {
                            offset: 25,
                            id: id(b"TALB")
                        },
                    ]
                )
            );
        }
    }

    #[test]
    fn skips_the_octets_the_flags_add_before_the_body() {
        // 2.3 grouping identity: one group octet.
        let v23 = Tag::new(Version::V23)
            .frame(b"TIT2", 0x0020, b"\x80\x00A")
            .frame(b"PRIV", 0x0020, b"\x81xy")
            .build();
        assert_eq!(
            contents(&v23),
            (
                vec![
                    frame(b"TIT2", 10, 0x0020, text_body(&["A"])),
                    frame(b"PRIV", 23, 0x0020, raw(34, 36)),
                ],
                vec![]
            )
        );
        // 2.4 grouping identity, then a data length indicator, alone and
        // together.
        let v24 = Tag::new(Version::V24)
            .frame(b"TIT2", 0x0040, b"\x80\x00A")
            .frame(b"TALB", 0x0001, b"\x00\x00\x00\x02\x00B")
            .frame(b"TPE1", 0x0041, b"\x80\x00\x00\x00\x02\x00C")
            .build();
        assert_eq!(
            contents(&v24),
            (
                vec![
                    frame(b"TIT2", 10, 0x0040, text_body(&["A"])),
                    frame(b"TALB", 23, 0x0001, text_body(&["B"])),
                    frame(b"TPE1", 39, 0x0041, text_body(&["C"])),
                ],
                vec![]
            )
        );
    }

    #[test]
    fn skips_and_records_a_frame_too_short_for_what_its_flags_add() {
        let v24 = Tag::new(Version::V24)
            .frame(b"TIT2", 0x0041, b"\x80\x00\x00\x00")
            .frame(b"TALB", 0, &latin1("B"))
            .build();
        let v23 = Tag::new(Version::V23)
            .frame(b"TIT2", 0x0020, b"")
            .bytes(&kit::frame(Version::V23, b"TIT2", 0x0020, b"\x80"))
            .build();
        assert_eq!(
            contents(&v24),
            (
                vec![frame(b"TALB", 24, 0, text_body(&["B"]))],
                vec![TagProblem::Malformed {
                    offset: 10,
                    id: id(b"TIT2")
                }]
            )
        );
        // A group octet and nothing after it leaves an empty payload, read
        // as a text frame with no encoding octet.
        assert_eq!(
            contents(&v23),
            (
                vec![frame(b"TIT2", 20, 0x0020, raw(31, 31))],
                vec![
                    TagProblem::EmptyFrame {
                        offset: 10,
                        id: id(b"TIT2")
                    },
                    TagProblem::Malformed {
                        offset: 20,
                        id: id(b"TIT2")
                    },
                ]
            )
        );
    }

    /// ID3v2.4.0 structure section 6.1: the scheme applies frame by frame,
    /// by the frame's own flag or by the tag's.
    #[test]
    fn reverses_unsynchronisation_in_2_4_frames() {
        // "ÿà" in ISO-8859-1 is FF E0, stored as FF 00 E0.
        let title = unsynchronise(&latin1("ÿà"));
        assert_eq!(title, [0x00, 0xFF, 0x00, 0xE0]);
        let private = unsynchronise(&[0xFF, 0x00]);
        let by_frame = Tag::new(Version::V24)
            .frame(b"TIT2", 0x0002, &title)
            .frame(b"PRIV", 0x0002, &private)
            .build();
        let by_tag = Tag::new(Version::V24)
            .unsynchronised()
            .frame(b"TIT2", 0, &title)
            .frame(b"PRIV", 0, &private)
            .build();
        for (tag, flags) in [(by_frame, 0x0002), (by_tag, 0)] {
            assert_eq!(
                contents(&tag),
                (
                    vec![
                        frame(b"TIT2", 10, flags, text_body(&["ÿà"])),
                        frame(b"PRIV", 24, flags, FrameBody::Raw(span(34, 37, true))),
                    ],
                    vec![]
                )
            );
        }
    }

    /// ID3v2.3.0 section 5: the scheme covers the whole tag, frame headers
    /// included, and offsets still count the octets as stored.
    #[test]
    fn reverses_unsynchronisation_across_a_whole_2_3_tag() {
        // A PRIV of 255 octets has a size of 00 00 00 FF, stored with a
        // zero after the FF, so its header takes 11 octets.
        let tag = Tag::new(Version::V23)
            .unsynchronised()
            .frame(b"TIT2", 0, &latin1("ÿà"))
            .frame(b"PRIV", 0, &[0x41; 255])
            .frame(b"TALB", 0, &[0x00, 0xFF])
            .build();
        // TIT2 at 10: header 10, body 00 FF 00 E0. PRIV at 24: header 11,
        // body 35 to 290. TALB at 290: header 10, body 00 FF 00 at 300.
        assert_eq!(
            contents(&tag),
            (
                vec![
                    frame(b"TIT2", 10, 0, text_body(&["ÿà"])),
                    frame(b"PRIV", 24, 0, FrameBody::Raw(span(35, 290, true))),
                    frame(b"TALB", 290, 0, text_body(&["ÿ"])),
                ],
                vec![]
            )
        );
        let v22 = Tag::new(Version::V22)
            .unsynchronised()
            .frame(b"TT2", 0, &latin1("ÿà"))
            .build();
        assert_eq!(
            contents(&v22),
            (vec![frame(b"TT2", 10, 0, text_body(&["ÿà"]))], vec![])
        );
    }

    /// The documented fallback for 2.4 tags whose writers wrote 2.3 sizes.
    #[test]
    fn reads_2_4_sizes_written_as_plain_integers() {
        let title = kit::frame(Version::V24, b"TIT2", 0, &latin1("A"));
        // 200 is 00 00 00 C8, which is not syncsafe at all.
        let plain_200 = kit::frame(Version::V23, b"PRIV", 0, &[0x41; 200]);
        let tag = Tag::new(Version::V24)
            .bytes(&title)
            .bytes(&plain_200)
            .build();
        assert_eq!(
            contents(&tag),
            (
                vec![
                    frame(b"TIT2", 10, 0, text_body(&["A"])),
                    frame(b"PRIV", 22, 0, raw(32, 232)),
                ],
                vec![]
            )
        );
        // 300 is 00 00 01 2C, which reads as 172 when taken as syncsafe and
        // lands inside the body.
        let plain_300 = kit::frame(Version::V23, b"PRIV", 0, &[0x41; 300]);
        let tag = Tag::new(Version::V24)
            .bytes(&title)
            .bytes(&plain_300)
            .build();
        assert_eq!(
            contents(&tag),
            (
                vec![
                    frame(b"TIT2", 10, 0, text_body(&["A"])),
                    frame(b"PRIV", 22, 0, raw(32, 332)),
                ],
                vec![]
            )
        );
    }

    #[test]
    fn keeps_syncsafe_sizes_when_both_readings_fit() {
        // 00 00 01 00 is 128 as syncsafe and 256 as plain. With 128 octets
        // of padding after the body, both readings fit the tag.
        let tag = Tag::new(Version::V24)
            .frame(b"PRIV", 0, &[0x41; 128])
            .padding(128)
            .build();
        assert_eq!(
            contents(&tag),
            (vec![frame(b"PRIV", 10, 0, raw(20, 148))], vec![])
        );
    }

    #[test]
    fn keeps_syncsafe_sizes_when_neither_reading_fits() {
        let title = kit::frame(Version::V24, b"TIT2", 0, &latin1("A"));
        let plain_200 = kit::frame(Version::V23, b"PRIV", 0, &[0x41; 200]);
        let tag = Tag::new(Version::V24)
            .bytes(&title)
            .bytes(&plain_200)
            .frame(b"tit2", 0, b"x")
            .build();
        assert_eq!(
            contents(&tag),
            (
                vec![frame(b"TIT2", 10, 0, text_body(&["A"]))],
                vec![TagProblem::Fault(ParseFault::NotSyncsafe {
                    offset: 26,
                    octets: [0x00, 0x00, 0x00, 0xC8],
                })]
            )
        );
    }

    /// A chapter frame of `element_id` holding `frames`.
    fn chap(element_id: &str, frames: &[u8]) -> Vec<u8> {
        kit::frame(
            Version::V24,
            b"CHAP",
            0,
            &chapter(element_id, [0, 1_000, u32::MAX, u32::MAX], frames),
        )
    }

    /// `levels` chapters, each holding the next, the innermost holding
    /// `innermost`. The chapter at level `k` starts at 10 + 28k in a tag,
    /// and its embedded frames at 38 + 28k.
    fn nested(levels: usize, innermost: &[u8]) -> Vec<u8> {
        let mut frames = innermost.to_vec();
        for _ in 0..levels {
            frames = chap("c", &frames);
        }
        Tag::new(Version::V24).bytes(&frames).build()
    }

    #[test]
    fn walks_chapters_and_tables_of_contents_and_keeps_them_raw() {
        let title = kit::frame(Version::V24, b"TIT2", 0, &latin1("One"));
        let toc_body = table_of_contents("toc", 0x03, &["ch1", "ch2"], &title);
        let tag = Tag::new(Version::V24)
            .frame(b"CTOC", 0, &toc_body)
            .bytes(&chap("ch1", &title))
            .frame(b"CHAP", 0, &chapter("ch2", [1_000, 2_000, 0, 0], &[]))
            .build();
        // CTOC at 10: body 20 to 48 (14 octets of fields, 14 of TIT2).
        // CHAP at 48: body 58 to 92. CHAP at 92: body 102 to 122.
        assert_eq!(
            contents(&tag),
            (
                vec![
                    frame(b"CTOC", 10, 0, raw(20, 48)),
                    frame(b"CHAP", 48, 0, raw(58, 92)),
                    frame(b"CHAP", 92, 0, raw(102, 122)),
                ],
                vec![]
            )
        );
    }

    /// Verifies: SEC-MED-005, SEC-TM-032
    #[test]
    fn walks_frames_nested_four_levels_deep() {
        let title = kit::frame(Version::V24, b"TIT2", 0, &latin1("A"));
        // Four chapters put the title four levels below the tag's frames.
        assert_eq!(
            contents(&nested(4, &title)),
            (vec![frame(b"CHAP", 10, 0, raw(20, 134))], vec![])
        );
    }

    /// Verifies: SEC-MED-005, SEC-TM-032
    #[test]
    fn refuses_a_chapter_nested_five_levels_deep() {
        let title = kit::frame(Version::V24, b"TIT2", 0, &latin1("A"));
        // The fifth chapter starts at 10 + 4 * 28 = 122; its embedded title
        // would be at level 5.
        assert_eq!(
            contents(&nested(5, &title)),
            (
                vec![frame(b"CHAP", 10, 0, raw(20, 162))],
                vec![TagProblem::Fault(ParseFault::TooDeep {
                    limit: LimitKind::EmbeddedFrameDepth,
                    depth: 5,
                    max: 4,
                    offset: 150,
                })]
            )
        );
        // A fifth chapter with nothing inside goes no deeper.
        assert_eq!(
            contents(&nested(5, &[])),
            (vec![frame(b"CHAP", 10, 0, raw(20, 150))], vec![])
        );
    }

    /// Verifies: SEC-MED-005
    #[test]
    fn reads_the_depth_limit_from_the_limits_given() {
        let title = kit::frame(Version::V24, b"TIT2", 0, &latin1("A"));
        let read = read_with(
            &nested(1, &title),
            &limits(LimitKind::EmbeddedFrameDepth, 0),
        );
        assert_eq!(
            read.map(|tag| (tag.frames, tag.problems)),
            Ok((
                vec![frame(b"CHAP", 10, 0, raw(20, 50))],
                vec![TagProblem::Fault(ParseFault::TooDeep {
                    limit: LimitKind::EmbeddedFrameDepth,
                    depth: 1,
                    max: 0,
                    offset: 38,
                })]
            ))
        );
    }

    #[test]
    fn records_what_is_wrong_inside_a_chapter_at_its_own_offset() {
        let inside = [
            kit::frame(Version::V24, b"TIT2", 0x0004, b"\x80x"),
            kit::frame(Version::V24, b"tit2", 0, b"x"),
        ]
        .concat();
        // The chapter's frames start at 38: an encrypted TIT2 (12 octets),
        // then a bad identifier at 50.
        assert_eq!(
            contents(&nested(1, &inside)),
            (
                vec![frame(b"CHAP", 10, 0, raw(20, 61))],
                vec![
                    TagProblem::Encrypted {
                        offset: 38,
                        id: id(b"TIT2")
                    },
                    TagProblem::BadFrameId {
                        offset: 50,
                        id: id(b"tit2")
                    },
                ]
            )
        );
    }

    #[test]
    fn records_a_chapter_or_table_of_contents_whose_fields_are_cut_short() {
        let cases: [(&[u8], &[u8]); 7] = [
            (b"CHAP", b"c"),
            (
                b"CHAP",
                b"c\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",
            ),
            (b"CTOC", b"t"),
            (b"CTOC", b"t\x00\x03"),
            (b"CTOC", b"t\x00\x03\x02c1\x00"),
            (b"CTOC", b"t\x00\x03\x01c1"),
            (b"CHAP", b"\xFF"),
        ];
        for (kind, body) in cases {
            let tag = Tag::new(Version::V24).frame(kind, 0, body).build();
            let end = 20 + u64::try_from(body.len()).unwrap();
            assert_eq!(
                contents(&tag),
                (
                    vec![frame(kind, 10, 0, raw(20, end))],
                    vec![TagProblem::Malformed {
                        offset: 10,
                        id: id(kind)
                    }]
                ),
                "{body:02X?}"
            );
        }
    }

    /// Chapters in an unsynchronised 2.3 tag are walked in the octets as
    /// read, and report offsets in the octets as stored.
    #[test]
    fn walks_a_chapter_in_an_unsynchronised_2_3_tag() {
        // The end time FF 00 00 00 is stored as FF 00 00 00 00.
        let inside = kit::frame(Version::V23, b"tit2", 0, b"x");
        let body = chapter("c", [0, 0xFF00_0000, 0, 0], &inside);
        let tag = Tag::new(Version::V23)
            .unsynchronised()
            .frame(b"CHAP", 0, &body)
            .build();
        // The chapter's body is 29 octets as read and 30 as stored, from
        // 20. Its 18 octets of fields take 19 as stored, so the embedded
        // frame starts at 39.
        assert_eq!(
            contents(&tag),
            (
                vec![frame(b"CHAP", 10, 0, FrameBody::Raw(span(20, 50, true)))],
                vec![TagProblem::BadFrameId {
                    offset: 39,
                    id: id(b"tit2")
                }]
            )
        );
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn charges_each_frame_its_length_and_walks_2_4_frames_twice() {
        let pair = |version| {
            Tag::new(version)
                .frame(b"TIT2", 0, &latin1(""))
                .frame(b"PRIV", 0, b"xy")
                .build()
        };
        // 11 + 12 octets of frames, read once in 2.3 and twice in 2.4.
        assert_eq!(steps(&pair(Version::V23), &Limits::DEFAULT), 23);
        assert_eq!(steps(&pair(Version::V24), &Limits::DEFAULT), 46);
        // 2.2 headers are six octets.
        let v22 = Tag::new(Version::V22).frame(b"TT2", 0, &latin1("")).build();
        assert_eq!(steps(&v22, &Limits::DEFAULT), 7);
        // A chapter of 40 octets read twice, and its 12-octet title walked
        // once more.
        let title = kit::frame(Version::V24, b"TIT2", 0, &latin1("A"));
        assert_eq!(steps(&nested(1, &title), &Limits::DEFAULT), 92);
        // Plain 2.4 sizes: the syncsafe walk charges the title before it
        // fails, then the plain walk and the reading charge both frames.
        let plain_200 = kit::frame(Version::V23, b"PRIV", 0, &[0x41; 200]);
        let tag = Tag::new(Version::V24)
            .bytes(&title)
            .bytes(&plain_200)
            .build();
        assert_eq!(steps(&tag, &Limits::DEFAULT), 12 + 222 + 222);
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn fails_at_the_frame_where_the_budget_runs_out() {
        use crate::parse::Budget;
        let tag = Tag::new(Version::V24)
            .frame(b"TIT2", 0, &latin1(""))
            .frame(b"PRIV", 0, b"xy")
            .build();
        let parse = |steps| {
            super::super::parse(&tag, &Limits::DEFAULT, &mut Budget::for_input(0, 0, steps))
        };
        assert_eq!(parse(46).map(|tag| tag.frames.len()), Ok(2));
        // 45 steps: the walk takes 23, the title 11, and the private frame
        // at 21 needs 12 with 11 left.
        assert_eq!(
            parse(45),
            Err(Id3v2Error::Fault(ParseFault::BudgetExceeded { offset: 21 }))
        );
        // 22 steps: the walk itself runs out at the private frame.
        assert_eq!(
            parse(22),
            Err(Id3v2Error::Fault(ParseFault::BudgetExceeded { offset: 21 }))
        );
        // 34 steps: the reading runs out at the private frame; 33, at the
        // title.
        assert_eq!(
            parse(34),
            Err(Id3v2Error::Fault(ParseFault::BudgetExceeded { offset: 21 }))
        );
        assert_eq!(
            parse(33),
            Err(Id3v2Error::Fault(ParseFault::BudgetExceeded { offset: 10 }))
        );
        let chaptered = nested(1, &kit::frame(Version::V24, b"TIT2", 0, &latin1("A")));
        let parse = |steps| {
            super::super::parse(
                &chaptered,
                &Limits::DEFAULT,
                &mut Budget::for_input(0, 0, steps),
            )
        };
        assert_eq!(parse(92).map(|tag| tag.frames.len()), Ok(1));
        assert_eq!(
            parse(91),
            Err(Id3v2Error::Fault(ParseFault::BudgetExceeded { offset: 38 }))
        );
        // Plain 2.4 sizes: the syncsafe walk charges the title, then the
        // plain walk runs out at the private frame at 22.
        let title = kit::frame(Version::V24, b"TIT2", 0, &latin1("A"));
        let plain_200 = kit::frame(Version::V23, b"PRIV", 0, &[0x41; 200]);
        let plain = Tag::new(Version::V24)
            .bytes(&title)
            .bytes(&plain_200)
            .build();
        let parse = |steps| {
            super::super::parse(
                &plain,
                &Limits::DEFAULT,
                &mut Budget::for_input(0, 0, steps),
            )
        };
        assert_eq!(parse(456).map(|tag| tag.frames.len()), Ok(2));
        assert_eq!(
            parse(233),
            Err(Id3v2Error::Fault(ParseFault::BudgetExceeded { offset: 22 }))
        );
    }
}
