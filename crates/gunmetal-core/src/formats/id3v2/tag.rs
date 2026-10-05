//! The tag: its header and footer, its extended header, and [`parse`].

use std::borrow::Cow;

use crate::parse::{Budget, Cursor, Depth, LimitKind, Limits, ParseFault};

use super::frame::{Frame, FrameId};
use super::frames::{Reader, Sizes, is_frame_id};
use super::source::Source;

/// The steps [`parse`] may charge per octet of the tag (SEC-MED-007).
///
/// A parse charges each frame it reads the octets of its header and body.
/// A 2.4 tag's frames are walked once more, or twice, to tell how their
/// sizes are written, and the frames embedded in a chapter are charged
/// again at each of the at most four levels they are nested below the
/// tag's frames. That is at most seven charges per octet.
pub const BUDGET_PER_OCTET: u64 = 8;

/// The steps [`parse`] may charge on top of [`BUDGET_PER_OCTET`] for each
/// octet: none, so `Budget::for_input(len, BUDGET_PER_OCTET, BUDGET_FIXED)`
/// is enough for any tag of `len` octets.
pub const BUDGET_FIXED: u64 = 0;

/// The ten octets that open a tag, or close a 2.4 tag as its footer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    /// The major version: 2, 3 or 4.
    pub major: u8,
    /// The revision.
    pub revision: u8,
    /// The flags octet as written.
    pub flags: u8,
    /// The size the tag declares: the octets between the header and the
    /// footer, or the end of the tag when there is no footer.
    pub size: u32,
    /// Octets the whole tag takes: the header, `size` octets, and the
    /// footer when a 2.4 tag has one.
    pub len: u64,
}

/// Why a tag could not be read at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Id3v2Error {
    /// A failure every parser shares: the tag ends before the size its
    /// header declares, its size is not a syncsafe integer, or the step
    /// budget ran out.
    Fault(ParseFault),
    /// The octets did not start with `ID3`, or a footer with `3DI`.
    NotId3v2 {
        /// Where the marker should be.
        offset: u64,
        /// The three octets found there.
        marker: [u8; 3],
    },
    /// A version other than 2.2, 2.3 or 2.4, or a revision of `FF`.
    UnsupportedVersion {
        /// Where the version is.
        offset: u64,
        /// The major version found.
        major: u8,
        /// The revision found.
        revision: u8,
    },
    /// A 2.2 tag with its compression flag set. 2.2 defines no compression
    /// scheme and says to ignore such a tag.
    Compressed {
        /// Where the flags octet is.
        offset: u64,
    },
    /// The extended header declares a size below its minimum or past the
    /// end of the tag, so the frames cannot be found.
    ExtendedHeader {
        /// Where the extended header starts.
        offset: u64,
        /// The size it declares.
        size: u32,
    },
}

impl Id3v2Error {
    /// Where the problem is, in octets from the start of the tag.
    #[must_use]
    pub const fn offset(&self) -> u64 {
        match *self {
            Self::Fault(fault) => fault.offset(),
            Self::NotId3v2 { offset, .. }
            | Self::UnsupportedVersion { offset, .. }
            | Self::Compressed { offset }
            | Self::ExtendedHeader { offset, .. } => offset,
        }
    }
}

/// Where some octets of a tag are: a frame body kept raw, or a picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// The first octet, from the start of the tag.
    pub start: u64,
    /// One past the last octet, from the start of the tag.
    pub end: u64,
    /// Whether the octets are stored unsynchronised, so that reading them
    /// means reversing the scheme.
    pub unsynchronised: bool,
}

impl Span {
    /// The octets this span covers in `tag`, with the unsynchronisation
    /// scheme reversed when they are stored unsynchronised, or `None` when
    /// the span does not lie inside `tag`.
    #[must_use]
    pub fn read<'t>(&self, tag: &'t [u8]) -> Option<Cow<'t, [u8]>> {
        let start = usize::try_from(self.start).unwrap_or(usize::MAX);
        let end = usize::try_from(self.end).unwrap_or(usize::MAX);
        let stored = tag.get(start..end)?;
        Some(Source::new(stored, self.start, self.unsynchronised).decode())
    }
}

/// What a tag's extended header says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtendedHeader {
    /// Whether the tag updates an earlier tag in the file (2.4).
    pub update: bool,
    /// The CRC-32 of the frames, when the tag carries one. It is not
    /// checked.
    pub crc: Option<u64>,
    /// The tag restrictions octet (2.4).
    pub restrictions: Option<u8>,
}

/// A tag, read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Id3v2Tag {
    /// The tag's header.
    pub header: Header,
    /// The extended header, when the tag has one and it could be read.
    pub extended: Option<ExtendedHeader>,
    /// The frames, in the order written.
    pub frames: Vec<Frame>,
    /// What was skipped or could not be read, in the order found.
    pub problems: Vec<TagProblem>,
}

/// Something in a tag that was skipped or could not be read. The rest of
/// the tag is still read.
///
/// Offsets count octets from the start of the tag as stored; the counts in
/// a [`ParseFault::Truncated`] count octets as read, after any
/// unsynchronisation is reversed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagProblem {
    /// A failure every parser shares. A frame header cut short or a frame
    /// larger than the rest of the tag ([`ParseFault::Truncated`]), a 2.4
    /// frame size that is not syncsafe ([`ParseFault::NotSyncsafe`]) and
    /// the tag-field limit end the frames. A tag over the in-memory limit
    /// keeps only its frames below the limit. A picture past its limit is
    /// kept raw, and the values of a frame are cut short where the values
    /// of the tag's frames, counted together, pass the child limit
    /// ([`ParseFault::LimitExceeded`]). Frames nested too deeply in a
    /// chapter are not walked ([`ParseFault::TooDeep`]).
    Fault(ParseFault),
    /// The frames end at an identifier that is not made of capital letters
    /// and digits.
    BadFrameId {
        /// Where the frame header starts.
        offset: u64,
        /// The identifier's octets.
        id: FrameId,
    },
    /// A frame with an empty body was skipped (SEC-MED-008).
    EmptyFrame {
        /// Where the frame header starts.
        offset: u64,
        /// Which frame it was.
        id: FrameId,
    },
    /// A compressed frame was skipped; frames are not decompressed.
    Compressed {
        /// Where the frame header starts.
        offset: u64,
        /// Which frame it was.
        id: FrameId,
    },
    /// An encrypted frame was skipped.
    Encrypted {
        /// Where the frame header starts.
        offset: u64,
        /// Which frame it was.
        id: FrameId,
    },
    /// A frame's fields could not be read. A frame too short for the
    /// octets its flags add is skipped; any other is kept raw.
    Malformed {
        /// Where the frame header starts.
        offset: u64,
        /// Which frame it was.
        id: FrameId,
    },
    /// The extended header's fields could not be read, so it was skipped
    /// by its size.
    ExtendedHeader {
        /// Where the extended header starts.
        offset: u64,
    },
    /// The footer does not repeat the header.
    Footer {
        /// Where the footer starts.
        offset: u64,
    },
}

/// Reads the header at the start of `bytes`.
///
/// # Errors
///
/// Returns [`Id3v2Error::Fault`] with [`ParseFault::Truncated`] when
/// `bytes` holds fewer than ten octets or with
/// [`ParseFault::NotSyncsafe`] when the size is not a syncsafe integer,
/// [`Id3v2Error::NotId3v2`] when the octets do not start with `ID3`, and
/// [`Id3v2Error::UnsupportedVersion`] for a version other than 2.2, 2.3 or
/// 2.4 or a revision of `FF`.
pub fn header(bytes: &[u8]) -> Result<Header, Id3v2Error> {
    read_header(bytes, *b"ID3")
}

/// Reads a 2.4 footer from the first ten octets of `bytes`, so that a tag
/// at the end of a file can be found: it starts `len` octets before the
/// footer's end.
///
/// # Errors
///
/// As [`header`], with `3DI` in place of `ID3`, and
/// [`Id3v2Error::UnsupportedVersion`] for any version but 2.4, the only one
/// with footers.
pub fn footer(bytes: &[u8]) -> Result<Header, Id3v2Error> {
    let found = read_header(bytes, *b"3DI")?;
    if found.major != 4 {
        return Err(Id3v2Error::UnsupportedVersion {
            offset: 3,
            major: found.major,
            revision: found.revision,
        });
    }
    Ok(found)
}

/// Reads the ten octets of a header that starts with `marker`, or of a
/// footer.
fn read_header(bytes: &[u8], marker: [u8; 3]) -> Result<Header, Id3v2Error> {
    let [a, b, c, major, revision, flags, size @ ..] = Cursor::new(bytes)
        .array::<10>()
        .map_err(Id3v2Error::Fault)?;
    if [a, b, c] != marker {
        return Err(Id3v2Error::NotId3v2 {
            offset: 0,
            marker: [a, b, c],
        });
    }
    if !(2..=4).contains(&major) || revision == 0xFF {
        return Err(Id3v2Error::UnsupportedVersion {
            offset: 3,
            major,
            revision,
        });
    }
    let size = Cursor::at(&size, 6)
        .syncsafe_u32()
        .map_err(Id3v2Error::Fault)?;
    // Ten octets of header and at most 2^28 - 1 of tag, then perhaps ten
    // of footer: nothing here comes near u64::MAX.
    let ends = if major == 4 && flags & FOOTER != 0 {
        20
    } else {
        10
    };
    Ok(Header {
        major,
        revision,
        flags,
        size,
        len: u64::from(size).saturating_add(ends),
    })
}

/// Header flags: unsynchronisation; the extended header (2.3 and 2.4) or
/// compression (2.2); a footer (2.4).
const UNSYNCHRONISED: u8 = 0x80;
const EXTENDED: u8 = 0x40;
const FOOTER: u8 = 0x10;

/// Reads the tag at the start of `tag`, under `limits`, charging `budget`.
///
/// # Errors
///
/// Returns an [`Id3v2Error`] when the header cannot be read, when the tag
/// is shorter than its header declares, when its extended header hides
/// where the frames start, or when the budget runs out. Everything else
/// that is wrong is recorded in [`Id3v2Tag::problems`].
pub fn parse(tag: &[u8], limits: &Limits, budget: &mut Budget) -> Result<Id3v2Tag, Id3v2Error> {
    let header = header(tag)?;
    let Header {
        major, flags, size, ..
    } = header;
    if major == 2 && flags & EXTENDED != 0 {
        return Err(Id3v2Error::Compressed { offset: 5 });
    }
    let mut problems = Vec::new();
    let size = u64::from(size);
    let capped = limits.check(LimitKind::Id3v2TagBytes, size, 0);
    if let Err(fault) = capped {
        problems.push(TagProblem::Fault(fault));
    }
    let mut cursor = Cursor::at(tag.get(10..).unwrap_or_default(), 10);
    let area = cursor
        .take(size.min(limits.get(LimitKind::Id3v2TagBytes)))
        .map_err(Id3v2Error::Fault)?;
    if capped.is_ok() && major == 4 && flags & FOOTER != 0 {
        let offset = cursor.offset();
        let found = cursor.take(10).map_err(Id3v2Error::Fault)?;
        if footer(found) != Ok(header) {
            problems.push(TagProblem::Footer { offset });
        }
    }
    let unsynchronised = flags & UNSYNCHRONISED != 0;
    let mut source = Source::new(area, 10, major < 4 && unsynchronised);
    // A 2.2 tag with this flag already returned [`Id3v2Error::Compressed`].
    let extended = if flags & EXTENDED != 0 {
        extended_header(&mut source, major, &mut problems)?
    } else {
        None
    };
    let mut reader = Reader {
        major,
        sizes: Sizes::Plain,
        unsynchronised: major == 4 && unsynchronised,
        limits,
        budget,
        problems,
        pictures: 0,
        values: 0,
        lines: 0,
        walked: 0,
    };
    if major == 4 {
        reader.sizes = reader.sizes_of(source).map_err(Id3v2Error::Fault)?;
    }
    let mut frames = Vec::new();
    reader
        .frames(source, Depth::EMBEDDED_FRAME_ROOT, Some(&mut frames))
        .map_err(Id3v2Error::Fault)?;
    Ok(Id3v2Tag {
        header,
        extended,
        frames,
        problems: reader.problems,
    })
}

/// Reads the extended header at the start of `source`, leaving `source`
/// after it.
///
/// Some writers set the extended header flag without writing one (mutagen
/// documents this from Quod Libet issue 126). When the four octets where
/// its size belongs are a frame identifier, there is no extended header
/// and the frames start there.
fn extended_header(
    source: &mut Source<'_>,
    major: u8,
    problems: &mut Vec<TagProblem>,
) -> Result<Option<ExtendedHeader>, Id3v2Error> {
    let start = *source;
    let offset = source.offset();
    let size = source.array::<4>().map_err(Id3v2Error::Fault)?;
    if is_frame_id(&size) {
        *source = start;
        return Ok(None);
    }
    // 2.3 counts the octets after the size; 2.4 counts the whole extended
    // header, at least six octets, as a syncsafe integer.
    let (declared, after) = if major == 3 {
        let declared = u32::from_be_bytes(size);
        (declared, u64::from(declared))
    } else {
        let declared = Cursor::at(&size, offset)
            .syncsafe_u32()
            .map_err(Id3v2Error::Fault)?;
        (declared, u64::from(declared).saturating_sub(4))
    };
    let refused = Id3v2Error::ExtendedHeader {
        offset,
        size: declared,
    };
    if major == 4 && declared < 6 {
        return Err(refused);
    }
    let fields = source.split(after).map_err(|_| refused)?.decode();
    let read = if major == 3 {
        extended_v23(&fields)
    } else {
        extended_v24(&fields)
    };
    if read.is_none() {
        problems.push(TagProblem::ExtendedHeader { offset });
    }
    Ok(read)
}

/// The fields of a 2.3 extended header after its size: two flag octets,
/// the padding size, then the CRC when the first flag is set.
fn extended_v23(fields: &[u8]) -> Option<ExtendedHeader> {
    let (&[flags, ..], rest) = fields.split_first_chunk::<6>()?;
    let crc = if flags & 0x80 == 0 {
        None
    } else {
        Some(u64::from(u32::from_be_bytes(*rest.first_chunk::<4>()?)))
    };
    Some(ExtendedHeader {
        update: false,
        crc,
        restrictions: None,
    })
}

/// The fields of a 2.4 extended header after its size: one flag octet
/// counted by the octet before it, then for each flag set, in order, a
/// length octet and the flag's data: none for an update, a 35-bit syncsafe
/// CRC, and the restrictions octet.
fn extended_v24(fields: &[u8]) -> Option<ExtendedHeader> {
    let Some((&[1, flags], mut rest)) = fields.split_first_chunk::<2>() else {
        return None;
    };
    let update = flags & 0x40 != 0;
    if update {
        let Some((&[0], after)) = rest.split_first_chunk::<1>() else {
            return None;
        };
        rest = after;
    }
    let mut crc = None;
    if flags & 0x20 != 0 {
        let Some((&[5, crc_octets @ ..], after)) = rest.split_first_chunk::<6>() else {
            return None;
        };
        crc = Some(crc_octets.iter().fold(0_u64, |value, &octet| {
            value
                .saturating_mul(128)
                .saturating_add(u64::from(octet & 0x7F))
        }));
        rest = after;
    }
    let mut restrictions = None;
    if flags & 0x10 != 0 {
        let Some((&[1, octet], _)) = rest.split_first_chunk::<2>() else {
            return None;
        };
        restrictions = Some(octet);
    }
    Some(ExtendedHeader {
        update,
        crc,
        restrictions,
    })
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test layouts and bounds add small, bounded values"
)]
mod tests {
    use super::super::testing::{
        contents, frame, limits, on_small_stack, raw, read, read_with, steps, text_body, truncated,
    };
    use super::super::{Frame, FrameBody};
    use super::*;
    use gunmetal_testkit::id3v2::{self as kit, Encoding, Tag, Version, chapter, unsynchronise};
    use proptest::collection::vec;
    use proptest::prelude::*;

    fn header_of(major: u8, revision: u8, flags: u8, size: u32, len: u64) -> Header {
        Header {
            major,
            revision,
            flags,
            size,
            len,
        }
    }

    #[test]
    fn reads_the_header_of_each_version() {
        let cases: [(&[u8], Header); 4] = [
            (
                b"ID3\x02\x00\x00\x00\x00\x00\x00",
                header_of(2, 0, 0, 0, 10),
            ),
            (
                b"ID3\x03\x01\xE0\x00\x00\x02\x01",
                header_of(3, 1, 0xE0, 257, 267),
            ),
            (
                b"ID3\x04\x00\x80\x7F\x7F\x7F\x7F",
                header_of(4, 0, 0x80, 0x0FFF_FFFF, 0x1000_0009),
            ),
            // Octets after the header are not read.
            (
                b"ID3\x04\x00\x00\x00\x00\x00\x05TIT2",
                header_of(4, 0, 0, 5, 15),
            ),
        ];
        for (bytes, expected) in cases {
            assert_eq!(header(bytes), Ok(expected), "{bytes:02X?}");
        }
    }

    /// A footer adds ten octets to a 2.4 tag; the same flag means nothing
    /// in 2.2 and 2.3.
    #[test]
    fn counts_the_footer_of_a_2_4_tag_only() {
        assert_eq!(
            header(b"ID3\x04\x00\x10\x00\x00\x00\x05"),
            Ok(header_of(4, 0, 0x10, 5, 25))
        );
        assert_eq!(
            header(b"ID3\x03\x00\x10\x00\x00\x00\x05"),
            Ok(header_of(3, 0, 0x10, 5, 15))
        );
        assert_eq!(
            header(b"ID3\x02\x00\x10\x00\x00\x00\x05"),
            Ok(header_of(2, 0, 0x10, 5, 15))
        );
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn refuses_a_header_cut_short_at_every_length() {
        let whole = b"ID3\x04\x00\x00\x00\x00\x00\x00";
        for len in 0..10 {
            assert_eq!(
                header(&whole[..len]),
                Err(Id3v2Error::Fault(ParseFault::Truncated {
                    offset: 0,
                    needed: 10,
                    available: u64::try_from(len).unwrap(),
                })),
                "{len}"
            );
        }
    }

    #[test]
    fn refuses_octets_that_are_not_a_tag() {
        assert_eq!(
            header(b"3DI\x04\x00\x00\x00\x00\x00\x00"),
            Err(Id3v2Error::NotId3v2 {
                offset: 0,
                marker: *b"3DI"
            })
        );
        assert_eq!(
            header(b"fLaC\x00\x00\x00\x22\x00\x00"),
            Err(Id3v2Error::NotId3v2 {
                offset: 0,
                marker: *b"fLa"
            })
        );
    }

    #[test]
    fn refuses_versions_it_does_not_know() {
        for (major, revision) in [(0, 0), (1, 0), (5, 0), (0xFF, 0), (3, 0xFF), (4, 0xFF)] {
            assert_eq!(
                header(&[b'I', b'D', b'3', major, revision, 0, 0, 0, 0, 0]),
                Err(Id3v2Error::UnsupportedVersion {
                    offset: 3,
                    major,
                    revision
                }),
                "{major}.{revision}"
            );
        }
        // Every other revision is accepted.
        assert_eq!(
            header(b"ID3\x04\xFE\x00\x00\x00\x00\x00"),
            Ok(header_of(4, 0xFE, 0, 0, 10))
        );
    }

    /// Verifies: SEC-MED-001
    #[test]
    fn refuses_a_size_that_is_not_syncsafe() {
        assert_eq!(
            header(b"ID3\x03\x00\x00\x00\x00\x01\x80"),
            Err(Id3v2Error::Fault(ParseFault::NotSyncsafe {
                offset: 6,
                octets: [0x00, 0x00, 0x01, 0x80],
            }))
        );
    }

    #[test]
    fn reads_a_footer_to_find_a_tag_from_its_end() {
        assert_eq!(
            footer(b"3DI\x04\x00\x10\x00\x00\x02\x01"),
            Ok(header_of(4, 0, 0x10, 257, 277))
        );
        assert_eq!(
            footer(b"ID3\x04\x00\x10\x00\x00\x02\x01"),
            Err(Id3v2Error::NotId3v2 {
                offset: 0,
                marker: *b"ID3"
            })
        );
        assert_eq!(
            footer(b"3DI\x03\x00\x10\x00\x00\x02\x01"),
            Err(Id3v2Error::UnsupportedVersion {
                offset: 3,
                major: 3,
                revision: 0
            })
        );
        assert_eq!(
            footer(b"3DI\x04"),
            Err(Id3v2Error::Fault(ParseFault::Truncated {
                offset: 0,
                needed: 10,
                available: 4,
            }))
        );
    }

    /// A text frame body of one ISO-8859-1 value.
    fn latin1(value: &str) -> Vec<u8> {
        kit::text(Encoding::Latin1, &[value])
    }

    /// The tag the testkit checks octet by octet against the 2.4
    /// specification: an extended header marking an update, two text
    /// frames, padding and a footer.
    fn specification_tag() -> Vec<u8> {
        Tag::new(Version::V24)
            .extended_header(&[0x00, 0x00, 0x00, 0x07, 0x01, 0x40, 0x00])
            .frame(b"TIT2", 0x0000, &latin1("Song"))
            .frame(b"TPE1", 0x4000, &kit::text(Encoding::Utf8, &["A", "B"]))
            .padding(4)
            .footer()
            .build()
    }

    #[test]
    fn reads_a_whole_tag_as_the_specification_lays_it_out() {
        assert_eq!(
            read(&specification_tag()),
            Ok(Id3v2Tag {
                header: header_of(4, 0, 0x50, 40, 60),
                extended: Some(ExtendedHeader {
                    update: true,
                    crc: None,
                    restrictions: None,
                }),
                frames: vec![
                    frame(b"TIT2", 17, 0, text_body(&["Song"])),
                    frame(b"TPE1", 32, 0x4000, text_body(&["A", "B"])),
                ],
                problems: vec![],
            })
        );
    }

    /// The fault for `tag` cut to `len` octets when its header, `size`
    /// octets and `footer` octets of footer are needed.
    fn cut(len: usize, size: u64, footer: u64) -> Id3v2Error {
        let len = u64::try_from(len).unwrap();
        let (offset, needed) = if len < 10 {
            (0, 10)
        } else if len < 10 + size {
            (10, size)
        } else {
            (10 + size, footer)
        };
        Id3v2Error::Fault(ParseFault::Truncated {
            offset,
            needed,
            available: len - offset,
        })
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn reports_every_truncation_of_a_whole_tag_at_its_exact_offset() {
        let tag = specification_tag();
        for len in 0..tag.len() {
            assert_eq!(read(&tag[..len]), Err(cut(len, 40, 10)), "{len}");
        }
        for version in [Version::V22, Version::V23] {
            let tag = Tag::new(version)
                .frame(
                    if version == Version::V22 {
                        b"TT2"
                    } else {
                        b"TIT2"
                    },
                    0,
                    &latin1("A"),
                )
                .padding(3)
                .build();
            let size = u64::try_from(tag.len()).unwrap() - 10;
            for len in 0..tag.len() {
                assert_eq!(
                    read(&tag[..len]),
                    Err(cut(len, size, 0)),
                    "{version:?} {len}"
                );
            }
        }
    }

    #[test]
    fn refuses_a_2_2_tag_whose_compression_flag_is_set() {
        let tag = Tag::new(Version::V22)
            .flags(0x40)
            .frame(b"TT2", 0, &latin1("A"))
            .build();
        assert_eq!(read(&tag), Err(Id3v2Error::Compressed { offset: 5 }));
    }

    /// Flags a version does not define, and the experimental flag, change
    /// nothing.
    #[test]
    fn ignores_the_header_flags_it_does_not_act_on() {
        for (version, flags, id) in [
            (Version::V22, 0x3F, &b"TT2"[..]),
            (Version::V23, 0x3F, b"TIT2"),
            (Version::V24, 0x2F, b"TIT2"),
        ] {
            let tag = Tag::new(version)
                .flags(flags)
                .frame(id, 0, &latin1("A"))
                .build();
            assert_eq!(
                read(&tag).map(|tag| (tag.header.flags, tag.frames, tag.problems)),
                Ok((flags, vec![frame(id, 10, 0, text_body(&["A"]))], vec![])),
                "{version:?}"
            );
        }
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn reads_only_the_frames_below_the_in_memory_limit() {
        let tag = Tag::new(Version::V24)
            .frame(b"TIT2", 0, &latin1("A"))
            .frame(b"TALB", 0, &latin1("B"))
            .footer()
            .build();
        // 12 + 12 octets of frames. A limit of 16 keeps the title and four
        // octets of the album's header; the footer is past the limit.
        let capped = limits(LimitKind::Id3v2TagBytes, 16);
        let expected = Ok(Id3v2Tag {
            header: header_of(4, 0, 0x10, 24, 44),
            extended: None,
            frames: vec![frame(b"TIT2", 10, 0, text_body(&["A"]))],
            problems: vec![
                TagProblem::Fault(ParseFault::LimitExceeded {
                    limit: LimitKind::Id3v2TagBytes,
                    value: 24,
                    max: 16,
                    offset: 0,
                }),
                truncated(22, 10, 4),
            ],
        });
        assert_eq!(read_with(&tag, &capped), expected);
        // The octets past the limit need not be there at all.
        assert_eq!(read_with(&tag[..26], &capped), expected);
        assert_eq!(
            read_with(&tag[..25], &capped),
            Err(Id3v2Error::Fault(ParseFault::Truncated {
                offset: 10,
                needed: 16,
                available: 15,
            }))
        );
        // At the limit the whole tag is read, footer included.
        let exact = read_with(&tag, &limits(LimitKind::Id3v2TagBytes, 24));
        assert_eq!(
            exact.map(|tag| (tag.frames.len(), tag.problems)),
            Ok((2, vec![]))
        );
    }

    /// A tag of `version` with the extended header `extended`, then a
    /// title.
    fn with_extended(version: Version, extended: &[u8]) -> Vec<u8> {
        Tag::new(version)
            .extended_header(extended)
            .frame(b"TIT2", 0, &latin1("A"))
            .build()
    }

    /// What reading a tag finds besides its header.
    type Found = (Option<ExtendedHeader>, Vec<Frame>, Vec<TagProblem>);

    /// What reading `tag` finds besides its header.
    fn found(tag: &[u8]) -> Result<Found, Id3v2Error> {
        read(tag).map(|tag| (tag.extended, tag.frames, tag.problems))
    }

    fn extended(update: bool, crc: Option<u64>, restrictions: Option<u8>) -> ExtendedHeader {
        ExtendedHeader {
            update,
            crc,
            restrictions,
        }
    }

    /// The title [`with_extended`] writes, at `offset`.
    fn title_at(offset: u64) -> Vec<Frame> {
        vec![frame(b"TIT2", offset, 0, text_body(&["A"]))]
    }

    #[test]
    fn reads_a_2_3_extended_header_with_and_without_a_crc() {
        let plain = b"\x00\x00\x00\x06\x00\x00\x00\x00\x01\x00";
        assert_eq!(
            found(&with_extended(Version::V23, plain)),
            Ok((Some(extended(false, None, None)), title_at(20), vec![]))
        );
        let crc = b"\x00\x00\x00\x0A\x80\x00\x00\x00\x00\x00\x12\x34\x56\x78";
        assert_eq!(
            found(&with_extended(Version::V23, crc)),
            Ok((
                Some(extended(false, Some(0x1234_5678), None)),
                title_at(24),
                vec![]
            ))
        );
    }

    /// ID3v2.3.0 section 5: an extended header is unsynchronised with the
    /// rest of the tag.
    #[test]
    fn reads_a_2_3_extended_header_in_an_unsynchronised_tag() {
        // The CRC FF 00 FF 00 is stored as FF 00 00 FF 00 00.
        let tag = Tag::new(Version::V23)
            .unsynchronised()
            .extended_header(b"\x00\x00\x00\x0A\x80\x00\x00\x00\x00\x00\xFF\x00\xFF\x00")
            .frame(b"TIT2", 0, &latin1("A"))
            .build();
        assert_eq!(
            found(&tag),
            Ok((
                Some(extended(false, Some(0xFF00_FF00), None)),
                title_at(26),
                vec![]
            ))
        );
    }

    #[test]
    fn skips_a_2_3_extended_header_whose_fields_are_cut_short() {
        // A CRC flag with no CRC, and fewer octets than the flags and the
        // padding size take.
        for (fields, title) in [
            (&b"\x00\x00\x00\x06\x80\x00\x00\x00\x00\x00"[..], 20),
            (b"\x00\x00\x00\x02\x00\x00", 16),
        ] {
            assert_eq!(
                found(&with_extended(Version::V23, fields)),
                Ok((
                    None,
                    title_at(title),
                    vec![TagProblem::ExtendedHeader { offset: 10 }]
                ))
            );
        }
    }

    #[test]
    fn reads_each_flag_of_a_2_4_extended_header() {
        let cases: [(&[u8], ExtendedHeader); 5] = [
            (b"\x00\x00\x00\x06\x01\x00", extended(false, None, None)),
            (b"\x00\x00\x00\x07\x01\x40\x00", extended(true, None, None)),
            // 35 bits of CRC, all set.
            (
                b"\x00\x00\x00\x0C\x01\x20\x05\x7F\x7F\x7F\x7F\x7F",
                extended(false, Some(0x7_FFFF_FFFF), None),
            ),
            (
                b"\x00\x00\x00\x08\x01\x10\x01\xA5",
                extended(false, None, Some(0xA5)),
            ),
            // Every flag, in order; the CRC's high bits are not part of it.
            (
                b"\x00\x00\x00\x0F\x01\x70\x00\x05\x8F\xFF\xFF\xFF\xFF\x01\xA5",
                extended(true, Some(0xFFFF_FFFF), Some(0xA5)),
            ),
        ];
        for (fields, expected) in cases {
            let title = 10 + u64::try_from(fields.len()).unwrap();
            assert_eq!(
                found(&with_extended(Version::V24, fields)),
                Ok((Some(expected), title_at(title), vec![])),
                "{fields:02X?}"
            );
        }
    }

    #[test]
    fn skips_a_2_4_extended_header_whose_fields_are_wrong() {
        let cases: [&[u8]; 7] = [
            // Two flag octets; an update with data; a CRC of four octets;
            // restrictions of two octets.
            b"\x00\x00\x00\x06\x02\x00",
            b"\x00\x00\x00\x08\x01\x40\x01\x00",
            b"\x00\x00\x00\x0B\x01\x20\x04\x00\x00\x00\x00",
            b"\x00\x00\x00\x09\x01\x10\x02\x00\x00",
            // Each flag's data cut short.
            b"\x00\x00\x00\x06\x01\x40",
            b"\x00\x00\x00\x0A\x01\x20\x05\x00\x00\x00",
            b"\x00\x00\x00\x07\x01\x10\x01",
        ];
        for fields in cases {
            let title = 10 + u64::try_from(fields.len()).unwrap();
            assert_eq!(
                found(&with_extended(Version::V24, fields)),
                Ok((
                    None,
                    title_at(title),
                    vec![TagProblem::ExtendedHeader { offset: 10 }]
                )),
                "{fields:02X?}"
            );
        }
    }

    #[test]
    fn refuses_an_extended_header_that_hides_where_the_frames_start() {
        let refused = |size| Err(Id3v2Error::ExtendedHeader { offset: 10, size });
        assert_eq!(
            found(&with_extended(Version::V24, b"\x00\x00\x00\x05\x01")),
            refused(5)
        );
        assert_eq!(
            found(&with_extended(Version::V24, b"\x00\x00\x00\x7F\x01\x00")),
            refused(0x7F)
        );
        assert_eq!(
            found(&with_extended(Version::V23, b"\x00\x00\x01\x00\x00\x00")),
            refused(256)
        );
        assert_eq!(
            found(&with_extended(Version::V24, b"\x00\x00\x00\x86\x01\x00")),
            Err(Id3v2Error::Fault(ParseFault::NotSyncsafe {
                offset: 10,
                octets: [0x00, 0x00, 0x00, 0x86],
            }))
        );
        // Too few octets for the size itself.
        let short = Tag::new(Version::V24).extended_header(b"\x00\x00").build();
        assert_eq!(
            found(&short),
            Err(Id3v2Error::Fault(ParseFault::Truncated {
                offset: 10,
                needed: 4,
                available: 2,
            }))
        );
    }

    /// The documented fallback for writers that set the flag but write no
    /// extended header.
    #[test]
    fn reads_frames_where_a_flagged_extended_header_is_missing() {
        for version in [Version::V23, Version::V24] {
            let tag = Tag::new(version)
                .flags(0x40)
                .frame(b"TIT2", 0, &latin1("A"))
                .build();
            assert_eq!(found(&tag), Ok((None, title_at(10), vec![])), "{version:?}");
        }
    }

    #[test]
    fn records_a_footer_that_does_not_repeat_the_header() {
        let tag = Tag::new(Version::V24)
            .frame(b"TIT2", 0, &latin1("A"))
            .footer()
            .build();
        assert_eq!(found(&tag), Ok((None, title_at(10), vec![])));
        // The marker, the version, the flags and the size must all match.
        for (at, octet) in [(22, b'I'), (25, 0x00), (27, 0x00), (31, 0x0D)] {
            let mut changed = tag.clone();
            changed[at] = octet;
            assert_eq!(
                found(&changed),
                Ok((None, title_at(10), vec![TagProblem::Footer { offset: 22 }])),
                "{at}"
            );
        }
        // The same flag in 2.3 means nothing, so no footer is read.
        let v23 = Tag::new(Version::V23)
            .flags(0x10)
            .frame(b"TIT2", 0, &latin1("A"))
            .build();
        assert_eq!(found(&v23), Ok((None, title_at(10), vec![])));
    }

    #[test]
    fn reads_the_octets_of_a_span() {
        let tag = b"\x01\xFF\x00\x02";
        let span = |start, end, unsynchronised| Span {
            start,
            end,
            unsynchronised,
        };
        assert_eq!(span(1, 3, false).read(tag), Some(Cow::Borrowed(&tag[1..3])));
        assert_eq!(
            span(1, 4, true).read(tag),
            Some(Cow::Owned(vec![0xFF, 0x02]))
        );
        assert_eq!(span(4, 4, false).read(tag), Some(Cow::Borrowed(&[][..])));
        assert_eq!(span(3, 5, false).read(tag), None);
        assert_eq!(span(3, 2, false).read(tag), None);
        assert_eq!(span(u64::MAX, u64::MAX, false).read(tag), None);
    }

    /// Octets that are often not a tag at all, and often a header that
    /// declares exactly the octets after it, around any frames.
    fn any_octets() -> impl Strategy<Value = Vec<u8>> {
        let declared =
            (2_u8..=4, any::<u8>(), vec(any::<u8>(), 0..200)).prop_map(|(major, flags, body)| {
                let size = u32::try_from(body.len()).unwrap();
                let mut tag = b"ID3".to_vec();
                tag.extend([major, 0, flags]);
                tag.extend([
                    0,
                    0,
                    u8::try_from(size >> 7).unwrap(),
                    u8::try_from(size & 0x7F).unwrap(),
                ]);
                tag.extend(body);
                tag
            });
        // A tag the builder wrote, with one octet changed.
        let built =
            (vec(any_frame(), 0..8), any::<u16>(), any::<u8>()).prop_map(|(frames, at, octet)| {
                let mut tag = frames
                    .iter()
                    .fold(Tag::new(Version::V24), |tag, (index, body)| {
                        tag.frame(IDS[*index], 0, body)
                    })
                    .build();
                let at = usize::from(at) % tag.len();
                tag[at] = octet;
                tag
            });
        prop_oneof![vec(any::<u8>(), 0..64), declared, built]
    }

    /// Checks what holds for every result: errors and offsets inside the
    /// input, spans that can be read, and the documented step bound.
    fn check_any(octets: &[u8]) {
        let len = u64::try_from(octets.len()).unwrap();
        match read(octets) {
            Err(error) => assert!(error.offset() <= len, "{error:?} for {len} octets"),
            Ok(tag) => {
                for found in &tag.frames {
                    assert!(found.offset < len, "{found:?} for {len} octets");
                    if let FrameBody::Raw(span) = found.body {
                        assert!(span.read(octets).is_some(), "{span:?} for {len} octets");
                    }
                }
            }
        }
        assert!(steps(octets, &Limits::DEFAULT) <= BUDGET_PER_OCTET * len + BUDGET_FIXED);
    }

    /// Tags the builder writes that make the parser work hardest per
    /// octet: many tiny frames, chapters nested past the depth limit, and
    /// unsynchronised octets.
    fn adversarial() -> impl Strategy<Value = Vec<u8>> {
        let tiny = (0_usize..300).prop_map(|count| {
            let one = kit::frame(Version::V24, b"TIT2", 0, &[0x00]);
            let frames: Vec<u8> = (0..count).flat_map(|_| one.iter().copied()).collect();
            Tag::new(Version::V24).bytes(&frames).build()
        });
        let deep = (0_usize..10, 0_usize..20).prop_map(|(levels, inner)| {
            let mut frames: Vec<u8> = (0..inner)
                .flat_map(|_| kit::frame(Version::V24, b"TIT2", 0, b"\x00"))
                .collect();
            for _ in 0..levels {
                frames = kit::frame(Version::V24, b"CHAP", 0, &chapter("c", [0; 4], &frames));
            }
            Tag::new(Version::V24).bytes(&frames).build()
        });
        let ff = (1_usize..200, any::<bool>()).prop_map(|(len, v23)| {
            let body: Vec<u8> = (0..len).map(|_| 0xFF).collect();
            if v23 {
                Tag::new(Version::V23)
                    .unsynchronised()
                    .frame(b"TIT2", 0, &body)
                    .build()
            } else {
                Tag::new(Version::V24)
                    .frame(b"TIT2", 0x0002, &unsynchronise(&body))
                    .build()
            }
        });
        prop_oneof![tiny, deep, ff]
    }

    /// One frame as the round-trip property writes it: an identifier from
    /// a pool, and a body of any octets.
    fn any_frame() -> impl Strategy<Value = (usize, Vec<u8>)> {
        (0_usize..8, vec(any::<u8>(), 1..24))
    }

    /// Frame identifiers of each kind the parser treats differently.
    const IDS: [&[u8; 4]; 8] = [
        b"TIT2", b"TXXX", b"COMM", b"SYLT", b"APIC", b"UFID", b"CTOC", b"PRIV",
    ];
    const IDS_V22: [&[u8; 3]; 8] = [
        b"TT2", b"TXX", b"COM", b"SLT", b"PIC", b"UFI", b"IPL", b"PRV",
    ];

    proptest! {
        /// Verifies: SEC-MED-001, SEC-MED-008, SEC-TM-032
        #[test]
        fn returns_for_any_octets(octets in any_octets()) {
            on_small_stack(move || check_any(&octets));
        }

        /// Verifies: SEC-MED-007, SEC-MED-008
        #[test]
        fn charges_no_more_than_the_documented_steps(octets in adversarial()) {
            let len = u64::try_from(octets.len()).unwrap();
            prop_assert!(steps(&octets, &Limits::DEFAULT) <= BUDGET_PER_OCTET * len + BUDGET_FIXED);
            prop_assert!(read(&octets).is_ok());
        }

        /// Verifies: SEC-MED-005
        #[test]
        fn walks_four_levels_of_chapters_and_refuses_a_fifth(levels in 0_u64..12) {
            let title = kit::frame(Version::V24, b"TIT2", 0, &latin1("A"));
            let mut frames = title.clone();
            for _ in 0..levels {
                frames = kit::frame(Version::V24, b"CHAP", 0, &chapter("c", [0; 4], &frames));
            }
            let tag = Tag::new(Version::V24).bytes(&frames).build();
            let (frames, problems) = on_small_stack(move || contents(&tag));
            let expected = if levels == 0 {
                vec![frame(b"TIT2", 10, 0, text_body(&["A"]))]
            } else {
                vec![frame(b"CHAP", 10, 0, raw(20, 22 + 28 * levels))]
            };
            prop_assert_eq!(frames, expected);
            let deepest = if levels > 4 {
                vec![TagProblem::Fault(ParseFault::TooDeep {
                    limit: LimitKind::EmbeddedFrameDepth,
                    depth: 5,
                    max: 4,
                    offset: 150,
                })]
            } else {
                vec![]
            };
            prop_assert_eq!(problems, deepest);
        }

        /// Every frame the builder writes is found where it was written,
        /// whatever its body holds.
        ///
        /// Verifies: SEC-MED-008
        #[test]
        fn finds_every_frame_the_builder_wrote(
            frames in vec(any_frame(), 0..12),
            version in prop_oneof![Just(Version::V22), Just(Version::V23), Just(Version::V24)],
        ) {
            let header_len = if version == Version::V22 { 6 } else { 10 };
            let mut tag = Tag::new(version);
            let mut expected = Vec::new();
            let mut offset = 10;
            for (index, body) in &frames {
                let id: &[u8] = if version == Version::V22 { IDS_V22[*index] } else { IDS[*index] };
                tag = tag.frame(id, 0, body);
                expected.push((super::super::testing::id(id), offset));
                offset += header_len + u64::try_from(body.len()).unwrap();
            }
            let read = read(&tag.build()).unwrap();
            let found: Vec<_> = read.frames.iter().map(|frame| (frame.id, frame.offset)).collect();
            prop_assert_eq!(found, expected);
        }
    }

    #[test]
    fn reports_where_each_error_is() {
        let errors = [
            Id3v2Error::Fault(ParseFault::BudgetExceeded { offset: 3 }),
            Id3v2Error::NotId3v2 {
                offset: 5,
                marker: *b"abc",
            },
            Id3v2Error::UnsupportedVersion {
                offset: 7,
                major: 9,
                revision: 0,
            },
            Id3v2Error::Compressed { offset: 11 },
            Id3v2Error::ExtendedHeader {
                offset: 13,
                size: 2,
            },
        ];
        assert_eq!(errors.map(|error| error.offset()), [3, 5, 7, 11, 13]);
    }
}
