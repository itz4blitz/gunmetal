//! Vorbis comments: the tag block that FLAC, Ogg Vorbis and Opus share, and
//! the pictures embedded in it as `METADATA_BLOCK_PICTURE` comments.
//!
//! A block is laid out as the Vorbis I comment specification
//! (<https://xiph.org/vorbis/doc/v-comment.html>) says: a 32-bit
//! little-endian length and the vendor string, a 32-bit little-endian count,
//! then each comment as a 32-bit little-endian length and `NAME=value`. The
//! name is one or more octets from 0x20 to 0x7D other than `=`, compared
//! without regard to case; the value is UTF-8. FLAC holds the block in its
//! `VORBIS_COMMENT` metadata block, and Ogg Vorbis and Opus in their comment
//! header packet, whose framing the stream-header parsers remove first.
//!
//! [`parse`] returns raw fields: what a name means is the tag mapper's
//! business. It follows the parse contract (media-and-parser-safety.md,
//! section 2):
//!
//! - The declared count is a claim. Before any comment is read, the octets
//!   left must hold one four-octet length per comment, and the count must be
//!   within [`LimitKind::TagFields`] (SEC-MED-006, SEC-TM-032).
//! - Nothing is sized from a declaration. Fields, pictures and problems grow
//!   one per comment actually read, and text is decoded from the octets that
//!   are there.
//! - A comment that cannot be read whole is skipped and recorded in
//!   [`Comments::problems`] with its offset, and the parse goes on with the
//!   next one (SEC-MED-017). Only a block whose framing fails stops the
//!   parse, with the [`ParseFault`] that says where.
//! - Each comment takes at least its four length octets, so the loop over
//!   the count always advances, and the count bounds it (SEC-MED-008).
//! - The block is flat: no comment holds others, so there is no depth to
//!   count. A picture's image data is never read as comments.
//!
//! # Steps (SEC-MED-007)
//!
//! A parse charges one step for every octet it reads, as each part is read,
//! and one more for every octet of a picture's base64 value as it decodes
//! it, so it spends at most [`STEPS_PER_OCTET`] × octets +
//! [`FIXED_STEPS`].
//!
//! # Pictures
//!
//! A `METADATA_BLOCK_PICTURE` value is a FLAC `PICTURE` block body (RFC
//! 9639, section 8.8) in base64 (RFC 4648, section 4), decoded with the
//! core's codec. The decoded picture may be at most
//! [`LimitKind::PictureBytes`] octets, so its base64 form may be 4/3 of
//! that, and the codec refuses a longer value from its length alone, before
//! decoding any of it. A block keeps at most [`LimitKind::Pictures`]
//! pictures.
//!
//! Writers that wrap base64 at 76 columns, as the `base64` tool does by
//! default, are common enough that ASCII whitespace inside a picture's value
//! is set aside before decoding, as RFC 2045 does for MIME. This is the one
//! fallback for a writer's quirk here.
//!
//! The result is a [`Picture`]: where the value is and what the picture
//! declares, never the image data, so sixteen pictures of 32 MiB do not sit
//! in memory together. [`picture_data`] decodes one value again when the
//! artwork job needs its image. The declared media type and dimensions are
//! data, never trusted: the artwork job detects the format from the image
//! itself and reads its dimensions from its own header.

use crate::base64::{self, Alphabet, B64Error};
use crate::parse::{Budget, Cursor, LimitKind, Limits, ParseFault};
use crate::text::{self, Encoding, Text};
use crate::untrusted::Untrusted;

/// Steps [`parse`] charges per octet of its block, at most (k in
/// SEC-MED-007): one for reading it, and one more if it is part of a
/// picture's value.
pub const STEPS_PER_OCTET: u64 = 2;

/// Steps [`parse`] charges once per block (c in SEC-MED-007).
pub const FIXED_STEPS: u64 = 0;

/// The name of a comment that holds a picture, in upper case.
const PICTURE: &str = "METADATA_BLOCK_PICTURE";

/// What a comment block holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comments {
    /// The vendor string, which names the encoder that wrote the block,
    /// capped as short text.
    pub vendor: Text,
    /// Every comment that is not a picture, in block order. A name may come
    /// more than once, and each of its values is kept.
    pub fields: Vec<Field>,
    /// Every picture, in block order.
    pub pictures: Vec<Picture>,
    /// Every comment that was skipped, in block order.
    pub problems: Vec<FieldProblem>,
    /// The file offset just past the last comment. Whatever follows it in
    /// the block is not a comment: the framing bit of an Ogg Vorbis header,
    /// or padding in an Opus one.
    pub end: u64,
}

impl Comments {
    /// The values of every field named `key`, compared without regard to
    /// case, in block order.
    pub fn values<'c>(&'c self, key: &'c str) -> impl Iterator<Item = &'c Text> {
        self.fields
            .iter()
            .filter(move |field| field.key.eq_ignore_ascii_case(key))
            .map(|field| &field.value)
    }
}

/// One `NAME=value` comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    /// The name, in upper case, so that names differing only in case are
    /// equal.
    pub key: String,
    /// The value, capped as long text; tab and line feed are kept.
    pub value: Text,
}

/// A picture held in a `METADATA_BLOCK_PICTURE` comment: where its value is
/// and what the picture declares about itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    /// The file offset where the base64 value starts.
    pub offset: u64,
    /// Octets of the base64 value, whitespace included. The value's octets
    /// are what [`picture_data`] takes.
    pub len: u64,
    /// The picture type as FLAC numbers them; 3 is the front cover.
    pub kind: u32,
    /// The media type the picture declares, capped as short text.
    pub mime: Text,
    /// The description, capped as long text.
    pub description: Text,
    /// Width in pixels, as declared.
    pub width: u32,
    /// Height in pixels, as declared.
    pub height: u32,
    /// Bits per pixel, as declared.
    pub depth: u32,
    /// Colours in the palette of an indexed picture, otherwise zero, as
    /// declared.
    pub colours: u32,
    /// Octets of image data the value holds.
    pub data_len: u32,
}

/// Why a comment was skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldProblem {
    /// The comment holds no `=`, so it has no name. An empty comment is one
    /// of these.
    NoSeparator {
        /// Where the comment starts, after its length.
        offset: u64,
    },
    /// The comment starts with `=`, so its name is empty.
    EmptyKey {
        /// Where the comment starts, after its length.
        offset: u64,
    },
    /// The name holds an octet outside 0x20 to 0x7D.
    KeyOctet {
        /// Where the first such octet is.
        offset: u64,
        /// The octet.
        octet: u8,
    },
    /// The name is longer than [`LimitKind::ShortText`], or the comment
    /// holds a picture past [`LimitKind::Pictures`]. Every picture comment
    /// counts, whether it could be read or not. The fault names the limit,
    /// the value and where the comment starts.
    Limit(ParseFault),
    /// The comment names a picture that could not be read.
    Picture {
        /// Where the base64 value starts.
        offset: u64,
        /// Why it could not be read.
        error: PictureError,
    },
}

/// Why a picture's value could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PictureError {
    /// The value is not base64 once whitespace is set aside, or decodes to
    /// more than [`LimitKind::PictureBytes`] octets. Offsets count the
    /// characters of the value with its whitespace set aside.
    Base64(B64Error),
    /// The decoded picture ends before a length it declares, with offsets
    /// counted in octets of the decoded picture; or, from [`picture_data`],
    /// the budget ran out.
    Fault(ParseFault),
}

/// Reads the comment block `block`, whose first octet is at file offset
/// `at`, so that every offset reported is a file offset. A comment header
/// that Ogg reassembled from pages has no single file offset; pass 0, and
/// offsets count octets of the packet.
///
/// # Errors
///
/// Returns [`ParseFault::Truncated`] when the block ends inside a length,
/// the vendor string or a comment, or holds too few octets for the count it
/// declares; [`ParseFault::LimitExceeded`] when the count is over
/// [`LimitKind::TagFields`]; and [`ParseFault::BudgetExceeded`] when
/// `budget` runs out.
pub fn parse(
    block: &[u8],
    at: u64,
    limits: &Limits,
    budget: &mut Budget,
) -> Result<Comments, ParseFault> {
    let mut cursor = Cursor::at(block, at);
    let vendor_len = cursor.u32_le()?;
    let vendor = cursor.take(u64::from(vendor_len))?;
    let count_at = cursor.offset();
    let count = cursor.u32_le()?;
    budget.charge(u64::from(vendor_len).saturating_add(8), at)?;
    // Four octets for each comment's length; u32::MAX of them fit in u64.
    let needed = u64::from(count).saturating_mul(4);
    if needed > cursor.remaining() {
        return Err(ParseFault::Truncated {
            offset: cursor.offset(),
            needed,
            available: cursor.remaining(),
        });
    }
    limits.check(LimitKind::TagFields, u64::from(count), count_at)?;
    let mut comments = Comments {
        vendor: decode_text(vendor, limits, LimitKind::ShortText),
        fields: Vec::new(),
        pictures: Vec::new(),
        problems: Vec::new(),
        end: at,
    };
    let mut pictures = 0_u64;
    for _ in 0..count {
        let comment_at = cursor.offset();
        let len = cursor.u32_le()?;
        let comment = cursor.sub(u64::from(len))?;
        budget.charge(u64::from(len).saturating_add(4), comment_at)?;
        read_comment(comment, limits, budget, &mut pictures, &mut comments)?;
    }
    comments.end = cursor.offset();
    Ok(comments)
}

/// The image data that `value`, the base64 value of a picture comment as
/// [`Picture::offset`] and [`Picture::len`] locate it, holds.
///
/// # Errors
///
/// Returns the [`PictureError`] that [`parse`] recorded for the value, or
/// [`PictureError::Fault`] with [`ParseFault::BudgetExceeded`] at offset 0
/// when `budget` cannot pay one step per octet of the value.
pub fn picture_data(
    value: &[u8],
    limits: &Limits,
    budget: &mut Budget,
) -> Result<Vec<u8>, PictureError> {
    budget
        .charge(octets(value), 0)
        .map_err(PictureError::Fault)?;
    let decoded = decode_picture(value, limits)?;
    let (_, data) = read_picture(&decoded, limits).map_err(PictureError::Fault)?;
    Ok(data.to_vec())
}

/// Reads one comment, which `comment` holds after its length, into
/// `comments`, counting picture comments in `pictures`.
///
/// # Errors
///
/// Returns [`ParseFault::BudgetExceeded`] when `budget` cannot pay for
/// decoding a picture. Every other failure is the comment's own, recorded
/// in `comments`.
fn read_comment(
    comment: Cursor<'_>,
    limits: &Limits,
    budget: &mut Budget,
    pictures: &mut u64,
    comments: &mut Comments,
) -> Result<(), ParseFault> {
    let offset = comment.offset();
    let mut parts = comment.rest().splitn(2, |&octet| octet == b'=');
    let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
        comments.problems.push(FieldProblem::NoSeparator { offset });
        return Ok(());
    };
    if name.is_empty() {
        comments.problems.push(FieldProblem::EmptyKey { offset });
        return Ok(());
    }
    let outside = name
        .iter()
        .zip(0_u64..)
        .find(|&(octet, _)| !(0x20..=0x7D).contains(octet));
    if let Some((&octet, index)) = outside {
        comments.problems.push(FieldProblem::KeyOctet {
            offset: offset.saturating_add(index),
            octet,
        });
        return Ok(());
    }
    if let Err(fault) = limits.check(LimitKind::ShortText, octets(name), offset) {
        comments.problems.push(FieldProblem::Limit(fault));
        return Ok(());
    }
    let key: String = name
        .iter()
        .map(|octet| char::from(octet.to_ascii_uppercase()))
        .collect();
    if key != PICTURE {
        comments.fields.push(Field {
            key,
            value: decode_text(value, limits, LimitKind::LongText),
        });
        return Ok(());
    }
    *pictures = pictures.saturating_add(1);
    if let Err(fault) = limits.check(LimitKind::Pictures, *pictures, offset) {
        comments.problems.push(FieldProblem::Limit(fault));
        return Ok(());
    }
    // The name and its `=` come before the value.
    let value_at = offset.saturating_add(octets(name)).saturating_add(1);
    budget.charge(octets(value), value_at)?;
    let read = decode_picture(value, limits).and_then(|decoded| {
        read_picture(&decoded, limits)
            .map(|(picture, _)| picture)
            .map_err(PictureError::Fault)
    });
    match read {
        Ok(picture) => comments.pictures.push(Picture {
            offset: value_at,
            len: octets(value),
            ..picture
        }),
        Err(error) => comments.problems.push(FieldProblem::Picture {
            offset: value_at,
            error,
        }),
    }
    Ok(())
}

/// Decodes a picture's base64 `value`, with its ASCII whitespace set aside,
/// into at most [`LimitKind::PictureBytes`] octets.
fn decode_picture(value: &[u8], limits: &Limits) -> Result<Vec<u8>, PictureError> {
    let text: Vec<u8> = value
        .iter()
        .copied()
        .filter(|octet| !octet.is_ascii_whitespace())
        .collect();
    let max = usize::try_from(limits.get(LimitKind::PictureBytes)).unwrap_or(usize::MAX);
    base64::decode(Untrusted::new(&text), Alphabet::Standard, max).map_err(PictureError::Base64)
}

/// Reads a decoded picture: what it declares, with zero for its location,
/// and its image data.
fn read_picture<'a>(decoded: &'a [u8], limits: &Limits) -> Result<(Picture, &'a [u8]), ParseFault> {
    let mut cursor = Cursor::new(decoded);
    let kind = cursor.u32_be()?;
    let mime_len = cursor.u32_be()?;
    let mime = cursor.take(u64::from(mime_len))?;
    let description_len = cursor.u32_be()?;
    let description = cursor.take(u64::from(description_len))?;
    let width = cursor.u32_be()?;
    let height = cursor.u32_be()?;
    let depth = cursor.u32_be()?;
    let colours = cursor.u32_be()?;
    let data_len = cursor.u32_be()?;
    let data = cursor.take(u64::from(data_len))?;
    let picture = Picture {
        offset: 0,
        len: 0,
        kind,
        mime: decode_text(mime, limits, LimitKind::ShortText),
        description: decode_text(description, limits, LimitKind::LongText),
        width,
        height,
        depth,
        colours,
        data_len,
    };
    Ok((picture, data))
}

/// Decodes UTF-8 text from a comment, capped at the limit `cap` names.
fn decode_text(octets: &[u8], limits: &Limits, cap: LimitKind) -> Text {
    // Every text limit's ceiling is far below u32::MAX.
    let cap = u32::try_from(limits.get(cap)).unwrap_or(u32::MAX);
    text::decode(Untrusted::new(octets), Encoding::Utf8, cap)
}

/// How many octets `slice` holds, as a `u64`.
fn octets(slice: &[u8]) -> u64 {
    // A slice never holds more than u64::MAX octets.
    u64::try_from(slice.len()).unwrap_or(u64::MAX)
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use super::*;
    use crate::parse::LimitKind;
    use gunmetal_testkit::vorbis_comment::{self as kit, CommentBlock};
    use proptest::collection::vec;
    use proptest::prelude::*;

    /// Steps in the budget [`read_at`] parses with: more than any test
    /// block needs.
    const PLENTY: u64 = 1 << 40;

    /// Parses `block`, which starts at file offset `at`, under `limits` with
    /// a budget of [`PLENTY`] steps, and returns the result and the steps it
    /// spent.
    fn read_at(block: &[u8], at: u64, limits: &Limits) -> (Result<Comments, ParseFault>, u64) {
        let mut budget = Budget::for_input(0, 0, PLENTY);
        let result = parse(block, at, limits, &mut budget);
        (result, PLENTY - budget.remaining())
    }

    /// Parses `block` at offset 0 under the default limits.
    fn read(block: &[u8]) -> Result<Comments, ParseFault> {
        read_at(block, 0, &Limits::DEFAULT).0
    }

    /// The default limits with each of `overrides` applied.
    fn limits(overrides: &[(LimitKind, u64)]) -> Limits {
        overrides
            .iter()
            .fold(Limits::DEFAULT, |limits, &(kind, value)| {
                limits.with_override(kind, value).unwrap()
            })
    }

    /// Text that was neither cut nor repaired.
    fn text(value: &str) -> Text {
        Text {
            value: value.to_owned(),
            truncated: false,
            replaced: false,
        }
    }

    /// Text cut at the cap.
    fn cut(value: &str) -> Text {
        Text {
            truncated: true,
            ..text(value)
        }
    }

    fn field(key: &str, value: &str) -> Field {
        Field {
            key: key.to_owned(),
            value: text(value),
        }
    }

    /// A block's parse with only a vendor and fields.
    fn plain(vendor: &str, fields: Vec<Field>, end: u64) -> Comments {
        Comments {
            vendor: text(vendor),
            fields,
            pictures: vec![],
            problems: vec![],
            end,
        }
    }

    /// A small front cover: 50 octets laid out as a FLAC picture block
    /// body, 68 characters of base64.
    fn cover() -> kit::Picture {
        kit::Picture {
            kind: 3,
            mime: "image/png".to_owned(),
            description: "Front".to_owned(),
            width: 1,
            height: 1,
            depth: 24,
            colours: 0,
            data: vec![0x89, b'P', b'N', b'G'],
        }
    }

    /// The reference to [`cover`], whose base64 value of `len` octets
    /// starts at `offset`.
    fn cover_at(offset: u64, len: u64) -> Picture {
        Picture {
            offset,
            len,
            kind: 3,
            mime: text("image/png"),
            description: text("Front"),
            width: 1,
            height: 1,
            depth: 24,
            colours: 0,
            data_len: 4,
        }
    }

    /// The vendor string libFLAC 1.4.3 writes: 32 octets.
    const LIBFLAC: &str = "reference libFLAC 1.4.3 20230623";

    /// A block with every kind of comment, and where each part starts.
    ///
    /// | Octets  | Part                                           |
    /// |---------|------------------------------------------------|
    /// | 0..40   | vendor length, vendor, count of 5              |
    /// | 40..54  | `TITLE=Song`                                   |
    /// | 54..68  | `Artist=One`                                   |
    /// | 68..82  | `ARTIST=Two`                                   |
    /// | 82..94  | `COMMENT=`                                     |
    /// | 94..189 | `METADATA_BLOCK_PICTURE=`, value at 121..189   |
    fn everything() -> Vec<u8> {
        CommentBlock::new(LIBFLAC.as_bytes())
            .field("TITLE", "Song")
            .field("Artist", "One")
            .field("ARTIST", "Two")
            .field("COMMENT", "")
            .field("METADATA_BLOCK_PICTURE", &kit::base64(&cover().build()))
            .build()
    }

    /// Verifies: SEC-MED-001
    #[test]
    fn reads_a_block_the_testkit_wrote_whole() {
        assert_eq!(
            read(&everything()),
            Ok(Comments {
                vendor: text(LIBFLAC),
                fields: vec![
                    field("TITLE", "Song"),
                    field("ARTIST", "One"),
                    field("ARTIST", "Two"),
                    field("COMMENT", ""),
                ],
                pictures: vec![cover_at(121, 68)],
                problems: vec![],
                end: 189,
            })
        );
    }

    /// A block written out octet by octet from the specification, sharing
    /// nothing with the testkit: 35 octets.
    const LITERAL: [u8; 35] = [
        3, 0, 0, 0, b'G', b'm', b't', // vendor
        2, 0, 0, 0, // two comments
        7, 0, 0, 0, b'a', b'l', b'b', b'u', b'm', b'=', b'X', // at 11
        9, 0, 0, 0, b'D', b'A', b'T', b'E', b'=', b'1', b'9', b'9', b'9', // at 22
    ];

    #[test]
    fn reads_a_block_written_out_octet_by_octet() {
        assert_eq!(
            read(&LITERAL),
            Ok(plain(
                "Gmt",
                vec![field("ALBUM", "X"), field("DATE", "1999")],
                35
            ))
        );
    }

    #[test]
    fn keys_match_in_any_case_and_values_keep_their_order() {
        let block = CommentBlock::new(b"")
            .field("Artist", "A")
            .field("TITLE", "T")
            .field("ARTIST", "B")
            .field("artist", "C")
            .build();
        let comments = read(&block).unwrap();
        assert_eq!(
            comments.fields,
            [
                field("ARTIST", "A"),
                field("TITLE", "T"),
                field("ARTIST", "B"),
                field("ARTIST", "C"),
            ]
        );
        let values = |key| comments.values(key).cloned().collect::<Vec<_>>();
        assert_eq!(values("aRtIsT"), [text("A"), text("B"), text("C")]);
        assert_eq!(values("ARTIST"), [text("A"), text("B"), text("C")]);
        assert_eq!(values("title"), [text("T")]);
        assert_eq!(values("ALBUM"), []);
        assert_eq!(values("ARTIS"), []);
        assert_eq!(values(""), []);
    }

    /// A count of 2^32 − 1 needs 4 × (2^32 − 1) octets just for the comment
    /// lengths, and eight are left.
    ///
    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn refuses_a_count_the_bytes_cannot_hold() {
        let block = [
            4, 0, 0, 0, b'a', b'b', b'c', b'd', // vendor
            0xFF, 0xFF, 0xFF, 0xFF, // 2^32 - 1 comments
            4, 0, 0, 0, b'A', b'=', b'1', b'2', // one
        ];
        assert_eq!(block.len(), 20);
        assert_eq!(
            read(&block),
            Err(ParseFault::Truncated {
                offset: 12,
                needed: 17_179_869_180,
                available: 8,
            })
        );
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn checks_the_count_against_the_bytes_at_its_boundary() {
        // Two empty comments take exactly the eight octets left.
        let mut block = CommentBlock::new(b"");
        block.raw(b"").raw(b"");
        assert_eq!(
            read(&block.build()),
            Ok(Comments {
                problems: vec![
                    FieldProblem::NoSeparator { offset: 12 },
                    FieldProblem::NoSeparator { offset: 16 },
                ],
                ..plain("", vec![], 16)
            })
        );
        // A third would need twelve.
        assert_eq!(
            read(&block.declare(3).build()),
            Err(ParseFault::Truncated {
                offset: 8,
                needed: 12,
                available: 8,
            })
        );
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn allows_as_many_comments_as_the_limit_and_refuses_one_more() {
        let two = limits(&[(LimitKind::TagFields, 2)]);
        let mut block = CommentBlock::new(b"v");
        block.field("A", "1").field("B", "2");
        assert_eq!(
            read_at(&block.build(), 0, &two).0,
            Ok(plain("v", vec![field("A", "1"), field("B", "2")], 23))
        );
        block.field("C", "3");
        assert_eq!(
            read_at(&block.build(), 0, &two).0,
            Err(ParseFault::LimitExceeded {
                limit: LimitKind::TagFields,
                value: 3,
                max: 2,
                offset: 5,
            })
        );
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn allows_4096_comments_by_default_and_refuses_4097() {
        let mut block = CommentBlock::new(b"");
        for _ in 0..4_096 {
            block.field("K", "");
        }
        let comments = read(&block.build()).unwrap();
        assert_eq!(comments.fields.len(), 4_096);
        assert!(comments.fields.iter().all(|found| *found == field("K", "")));
        assert_eq!((comments.problems, comments.end), (vec![], 8 + 4_096 * 6));
        block.field("K", "");
        assert_eq!(
            read(&block.build()),
            Err(ParseFault::LimitExceeded {
                limit: LimitKind::TagFields,
                value: 4_097,
                max: 4_096,
                offset: 4,
            })
        );
    }

    #[test]
    fn records_a_comment_without_a_separator_or_a_name_and_reads_on() {
        let block = CommentBlock::new(b"")
            .field("TITLE", "a")
            .raw(b"NOSEPARATOR")
            .raw(b"")
            .raw(b"=value")
            .field("ARTIST", "b")
            .build();
        assert_eq!(
            read(&block),
            Ok(Comments {
                problems: vec![
                    FieldProblem::NoSeparator { offset: 23 },
                    FieldProblem::NoSeparator { offset: 38 },
                    FieldProblem::EmptyKey { offset: 42 },
                ],
                ..plain("", vec![field("TITLE", "a"), field("ARTIST", "b")], 60)
            })
        );
    }

    /// A name holds octets from 0x20 to 0x7D other than `=`; the value
    /// after the first `=` may hold anything, more `=` included.
    #[test]
    fn records_a_name_octet_outside_0x20_to_0x7d() {
        let block = CommentBlock::new(b"")
            .field(" ", "space")
            .field("}", "a=b")
            .raw(b"A~B=1")
            .raw(b"A\x1FB=1")
            .raw(b"\x7F=1")
            .raw(b"\xC3\x89=1")
            .build();
        assert_eq!(
            read(&block),
            Ok(Comments {
                problems: vec![
                    FieldProblem::KeyOctet {
                        offset: 33,
                        octet: b'~',
                    },
                    FieldProblem::KeyOctet {
                        offset: 42,
                        octet: 0x1F,
                    },
                    FieldProblem::KeyOctet {
                        offset: 50,
                        octet: 0x7F,
                    },
                    FieldProblem::KeyOctet {
                        offset: 57,
                        octet: 0xC3,
                    },
                ],
                ..plain("", vec![field(" ", "space"), field("}", "a=b")], 61)
            })
        );
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn records_a_name_longer_than_a_short_text_field() {
        let three = limits(&[(LimitKind::ShortText, 3)]);
        let block = CommentBlock::new(b"")
            .field("ABC", "1")
            .field("ABCD", "2")
            .build();
        assert_eq!(
            read_at(&block, 0, &three).0,
            Ok(Comments {
                problems: vec![FieldProblem::Limit(ParseFault::LimitExceeded {
                    limit: LimitKind::ShortText,
                    value: 4,
                    max: 3,
                    offset: 21,
                })],
                ..plain("", vec![field("ABC", "1")], 27)
            })
        );
        let long: String = (0..4_097).map(|_| 'K').collect();
        let block = CommentBlock::new(b"")
            .field(&long[1..], "fits")
            .field(&long, "too long")
            .build();
        assert_eq!(
            read(&block),
            Ok(Comments {
                problems: vec![FieldProblem::Limit(ParseFault::LimitExceeded {
                    limit: LimitKind::ShortText,
                    value: 4_097,
                    max: 4_096,
                    offset: 4_117,
                })],
                ..plain("", vec![field(&long[1..], "fits")], 8_223)
            })
        );
    }

    #[test]
    fn keeps_line_feeds_and_tabs_in_values_and_drops_other_controls() {
        let block = CommentBlock::new(b"")
            .field("LYRICS", "one\r\ntwo\tend\0!")
            .build();
        assert_eq!(
            read(&block),
            Ok(plain("", vec![field("LYRICS", "one\ntwo\tend!")], 33))
        );
    }

    /// Navidrome rendered a comment as HTML and leaked a session token
    /// (CVE-2026-25578). The value is kept as text, for clients to show
    /// as text, never as markup.
    ///
    /// Verifies: SEC-HIS-036
    #[test]
    fn keeps_html_in_a_comment_as_inert_text() {
        let block = CommentBlock::new(b"")
            .field("COMMENT", "<img src=x onerror=alert(document.cookie)>")
            .build();
        assert_eq!(
            read(&block),
            Ok(plain(
                "",
                vec![field(
                    "COMMENT",
                    "<img src=x onerror=alert(document.cookie)>"
                )],
                62
            ))
        );
    }

    #[test]
    fn replaces_invalid_utf8_in_the_vendor_and_in_values() {
        let block = CommentBlock::new(b"\xFF").raw(b"A=a\xC3(").build();
        let replaced = |value: &str| Text {
            replaced: true,
            ..text(value)
        };
        assert_eq!(
            read(&block),
            Ok(Comments {
                vendor: replaced("\u{FFFD}"),
                fields: vec![Field {
                    key: "A".to_owned(),
                    value: replaced("a\u{FFFD}("),
                }],
                // Nine octets before the comment, four of length, five.
                ..plain("", vec![], 18)
            })
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn caps_the_vendor_as_short_text_and_values_as_long_text() {
        let small = limits(&[(LimitKind::ShortText, 2), (LimitKind::LongText, 4)]);
        let block = CommentBlock::new(b"xyz")
            .field("A", "abcdef")
            .field("B", "abcd")
            .build();
        assert_eq!(
            read_at(&block, 0, &small).0,
            Ok(Comments {
                vendor: cut("xy"),
                fields: vec![
                    Field {
                        key: "A".to_owned(),
                        value: cut("abcd"),
                    },
                    field("B", "abcd"),
                ],
                ..plain("", vec![], 33)
            })
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn caps_at_4_kib_and_64_kib_by_default() {
        let vendor: String = (0..4_097).map(|_| 'v').collect();
        let value: String = (0..65_537).map(|_| 'a').collect();
        let block = CommentBlock::new(vendor.as_bytes())
            .field("A", &value)
            .field("B", &value[1..])
            .build();
        let comments = read(&block).unwrap();
        assert_eq!(comments.vendor, cut(&vendor[1..]));
        assert_eq!(
            comments.fields,
            [
                Field {
                    key: "A".to_owned(),
                    value: cut(&value[1..]),
                },
                field("B", &value[1..]),
            ]
        );
        let block = CommentBlock::new(&vendor.as_bytes()[1..]).build();
        assert_eq!(read(&block).unwrap().vendor, text(&vendor[1..]));
    }

    /// The Ogg Vorbis framing bit and Opus padding follow the comments; the
    /// caller reads them from where the comments end.
    #[test]
    fn reports_where_the_comments_end_and_leaves_what_follows() {
        let mut block = LITERAL.to_vec();
        block.extend([0x01, 0xAA, 0xBB]);
        assert_eq!(
            read(&block),
            Ok(plain(
                "Gmt",
                vec![field("ALBUM", "X"), field("DATE", "1999")],
                35
            ))
        );
    }

    /// Verifies: SEC-MED-004
    #[test]
    fn reports_offsets_from_where_the_block_starts_in_its_file() {
        let block = CommentBlock::new(b"")
            .raw(b"=x")
            .raw(b"A\x7F=1")
            .field("METADATA_BLOCK_PICTURE", "!")
            .field("METADATA_BLOCK_PICTURE", &kit::base64(&cover().build()))
            .build();
        assert_eq!(
            read_at(&block, 1_000, &Limits::DEFAULT).0,
            Ok(Comments {
                pictures: vec![cover_at(1_077, 68)],
                problems: vec![
                    FieldProblem::EmptyKey { offset: 1_012 },
                    FieldProblem::KeyOctet {
                        offset: 1_019,
                        octet: 0x7F,
                    },
                    FieldProblem::Picture {
                        offset: 1_049,
                        error: PictureError::Base64(B64Error::InvalidLength { len: 1 }),
                    },
                ],
                ..plain("", vec![], 1_145)
            })
        );
        assert_eq!(
            read_at(&LITERAL[..30], 1_000, &Limits::DEFAULT).0,
            Err(ParseFault::Truncated {
                offset: 1_026,
                needed: 9,
                available: 4,
            })
        );
    }

    /// Where [`parse`] stops for each cut of a block of a vendor string of
    /// `vendor` octets and comments of `lens` octets, written out from the
    /// specification: the first part the cut leaves incomplete, or `None`
    /// when the block is whole. Before reading any comment, the parser
    /// checks that the octets left can hold one length per comment.
    fn truncation(vendor: u64, lens: &[u64], cut: u64, at: u64) -> Option<ParseFault> {
        let count = lens.len() as u64;
        let mut parts = vec![
            (0, 4),
            (4, vendor),
            (4 + vendor, 4),
            (8 + vendor, 4 * count),
        ];
        let mut start = 8 + vendor;
        for &len in lens {
            parts.extend([(start, 4), (start + 4, len)]);
            start += 4 + len;
        }
        parts
            .into_iter()
            .find(|&(start, needed)| cut < start + needed)
            .map(|(start, needed)| ParseFault::Truncated {
                offset: at + start,
                needed,
                available: cut - start,
            })
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn every_cut_of_a_valid_block_names_the_exact_part_it_truncates() {
        for cut in 0..=LITERAL.len() {
            let expected = truncation(3, &[7, 9], cut as u64, 0);
            let result = read(&LITERAL[..cut]);
            match expected {
                Some(fault) => assert_eq!(result, Err(fault), "cut at {cut}"),
                None => assert_eq!(cut, 35),
            }
        }
        assert_eq!(
            [11, 18, 19, 21, 22, 25, 26, 34].map(|cut| truncation(3, &[7, 9], cut, 0)),
            [11, 18, 19, 21, 22, 25, 26, 34].map(|cut: u64| {
                Some(match cut {
                    11..=18 => ParseFault::Truncated {
                        offset: 11,
                        needed: 8,
                        available: cut - 11,
                    },
                    19..=21 => ParseFault::Truncated {
                        offset: 15,
                        needed: 7,
                        available: cut - 15,
                    },
                    22..=25 => ParseFault::Truncated {
                        offset: 22,
                        needed: 4,
                        available: cut - 22,
                    },
                    _ => ParseFault::Truncated {
                        offset: 26,
                        needed: 9,
                        available: cut - 26,
                    },
                })
            })
        );
    }

    /// One step per octet read, and one more per octet of each picture
    /// value decoded: 189 octets and a 68-octet value.
    ///
    /// Verifies: SEC-MED-007
    #[test]
    fn charges_one_step_per_octet_read_and_one_per_picture_octet_decoded() {
        assert_eq!(read_at(&everything(), 0, &Limits::DEFAULT).1, 257);
        assert_eq!(read_at(&LITERAL, 0, &Limits::DEFAULT).1, 35);
        assert_eq!(read_at(&LITERAL, 7, &Limits::DEFAULT).1, 35);
        assert_eq!((STEPS_PER_OCTET, FIXED_STEPS), (2, 0));
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn runs_out_of_budget_at_exactly_the_part_it_cannot_pay_for() {
        let block = everything();
        let outcome = |steps: u64| {
            let mut budget = Budget::for_input(0, 0, steps);
            let result = parse(&block, 0, &Limits::DEFAULT, &mut budget);
            (result.map(|comments| comments.end), budget.remaining())
        };
        assert_eq!(outcome(257), (Ok(189), 0));
        let out_at = |offset| (Err(ParseFault::BudgetExceeded { offset }), 0);
        // The picture's value is the last thing paid for.
        assert_eq!(outcome(256), out_at(121));
        assert_eq!(outcome(189), out_at(121));
        // The picture's comment, 95 octets after 94.
        assert_eq!(outcome(188), out_at(94));
        assert_eq!(outcome(94), out_at(94));
        assert_eq!(outcome(93), out_at(82));
        // The 40 octets before the first comment.
        assert_eq!(outcome(39), out_at(0));
        assert_eq!(outcome(0), out_at(0));
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn charges_the_header_at_the_block_offset() {
        let mut budget = Budget::for_input(0, 0, 10);
        assert_eq!(
            parse(&LITERAL, 500, &Limits::DEFAULT, &mut budget),
            Err(ParseFault::BudgetExceeded { offset: 500 })
        );
        let mut budget = Budget::for_input(0, 0, 11);
        assert_eq!(
            parse(&LITERAL, 500, &Limits::DEFAULT, &mut budget),
            Err(ParseFault::BudgetExceeded { offset: 511 })
        );
    }

    /// A comment of no octets is shorter than any name and separator, so
    /// each is recorded as a problem; the parse still moves on by its four
    /// length octets and stops after the count.
    ///
    /// Verifies: SEC-MED-008
    #[test]
    fn steps_past_a_thousand_empty_comments_and_stops() {
        let mut block = CommentBlock::new(b"");
        for _ in 0..1_000 {
            block.raw(b"");
        }
        let mut bytes = block.build();
        bytes.extend([0; 64]);
        let (result, steps) = read_at(&bytes, 0, &Limits::DEFAULT);
        let expected: Vec<_> = (0..1_000)
            .map(|index| FieldProblem::NoSeparator {
                offset: 12 + 4 * index,
            })
            .collect();
        assert_eq!(
            result,
            Ok(Comments {
                problems: expected,
                ..plain("", vec![], 4_008)
            })
        );
        assert_eq!(steps, 4_008);
    }

    /// The cover's base64 value, cut into lines of 30 characters with CR LF,
    /// with spaces and a tab about, as `base64` without `--wrap=0` and some
    /// scripts write it.
    fn wrapped(value: &str) -> String {
        let (first, rest) = value.split_at(30);
        let (second, third) = rest.split_at(30);
        format!(" {first}\r\n{second} \t\n{third}\n")
    }

    #[test]
    fn decodes_a_picture_into_a_reference_in_any_case_with_or_without_padding() {
        let value = kit::base64(&cover().build());
        // "NG", the last two octets, are "Tkc=".
        assert_eq!((value.len(), value.ends_with("Tkc=")), (68, true));
        let unpadded = value.trim_end_matches('=');
        let block = CommentBlock::new(b"")
            .field("metadata_block_picture", &value)
            .field("Metadata_Block_Picture", unpadded)
            .build();
        assert_eq!(
            read(&block),
            Ok(Comments {
                pictures: vec![cover_at(35, 68), cover_at(130, 67)],
                ..plain("", vec![], 197)
            })
        );
    }

    /// Writers that wrap base64 at 76 columns are common enough that
    /// whitespace inside a picture's value is skipped, as RFC 2045 does for
    /// MIME. The reference counts the value as it is in the block.
    #[test]
    fn decodes_a_picture_whose_value_is_wrapped_in_lines() {
        let value = wrapped(&kit::base64(&cover().build()));
        assert_eq!(value.len(), 75);
        let block = CommentBlock::new(b"")
            .field("METADATA_BLOCK_PICTURE", &value)
            .build();
        assert_eq!(
            read(&block),
            Ok(Comments {
                pictures: vec![cover_at(35, 75)],
                ..plain("", vec![], 110)
            })
        );
        let mut budget = Budget::for_input(0, 0, PLENTY);
        assert_eq!(
            picture_data(value.as_bytes(), &Limits::DEFAULT, &mut budget),
            Ok(cover().data)
        );
    }

    /// Offsets in a base64 error count the characters of the value with its
    /// whitespace set aside; the problem names where the value starts.
    #[test]
    fn records_a_picture_value_that_is_not_base64() {
        let mut broken = wrapped(&kit::base64(&cover().build()));
        broken.replace_range(12..13, "!");
        let block = CommentBlock::new(b"")
            .field("METADATA_BLOCK_PICTURE", &broken)
            .field("METADATA_BLOCK_PICTURE", "AAAAA")
            .field("METADATA_BLOCK_PICTURE", "Zh==")
            .field("TITLE", "kept")
            .build();
        assert_eq!(
            read(&block),
            Ok(Comments {
                problems: vec![
                    FieldProblem::Picture {
                        offset: 35,
                        error: PictureError::Base64(B64Error::InvalidByte {
                            offset: 11,
                            byte: b'!',
                        }),
                    },
                    FieldProblem::Picture {
                        offset: 137,
                        error: PictureError::Base64(B64Error::InvalidLength { len: 5 }),
                    },
                    FieldProblem::Picture {
                        offset: 169,
                        error: PictureError::Base64(B64Error::TrailingBits { offset: 1 }),
                    },
                ],
                ..plain("", vec![field("TITLE", "kept")], 187)
            })
        );
    }

    /// Offsets in a picture's fault count octets of the decoded picture.
    #[test]
    fn records_a_picture_whose_lengths_run_past_its_end() {
        let mut long_data = cover().build();
        // The data length is the last field before the four data octets.
        long_data[45] = 5;
        let short_mime = [0, 0, 0, 3, 0, 0, 0, 9, b'i'];
        let block = CommentBlock::new(b"")
            .field("METADATA_BLOCK_PICTURE", &kit::base64(&long_data))
            .field("METADATA_BLOCK_PICTURE", &kit::base64(&short_mime))
            .field("METADATA_BLOCK_PICTURE", "")
            .build();
        let fault = |offset, needed, available| {
            PictureError::Fault(ParseFault::Truncated {
                offset,
                needed,
                available,
            })
        };
        assert_eq!(
            read(&block),
            Ok(Comments {
                problems: vec![
                    FieldProblem::Picture {
                        offset: 35,
                        error: fault(46, 5, 4),
                    },
                    FieldProblem::Picture {
                        offset: 130,
                        error: fault(8, 9, 1),
                    },
                    FieldProblem::Picture {
                        offset: 169,
                        error: fault(0, 4, 0),
                    },
                ],
                ..plain("", vec![], 169)
            })
        );
    }

    /// A picture of 48 octets with no media type or description:
    /// `data` octets of data after 32 octets of lengths and numbers.
    fn picture_of(data: usize) -> String {
        kit::base64(
            &kit::Picture {
                data: (0..data).map(|_| 0xAB).collect(),
                ..kit::Picture::default()
            }
            .build(),
        )
    }

    /// The base64 form may take 4/3 of the picture limit: as many
    /// characters as a picture of that many octets needs.
    ///
    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn allows_a_picture_value_of_four_thirds_the_limit_and_refuses_more() {
        let small = limits(&[(LimitKind::PictureBytes, 48)]);
        let fits = picture_of(16);
        let over = picture_of(17);
        assert_eq!((fits.len(), over.len()), (64, 68));
        let block = CommentBlock::new(b"")
            .field("METADATA_BLOCK_PICTURE", &fits)
            .field("METADATA_BLOCK_PICTURE", &over)
            .build();
        let empty_at = |offset, len| Picture {
            offset,
            len,
            kind: 0,
            mime: text(""),
            description: text(""),
            width: 0,
            height: 0,
            depth: 0,
            colours: 0,
            data_len: 16,
        };
        assert_eq!(
            read_at(&block, 0, &small).0,
            Ok(Comments {
                pictures: vec![empty_at(35, 64)],
                problems: vec![FieldProblem::Picture {
                    offset: 126,
                    error: PictureError::Base64(B64Error::TooLong {
                        needed: 49,
                        max: 48,
                    }),
                }],
                ..plain("", vec![], 194)
            })
        );
        let mut budget = Budget::for_input(0, 0, PLENTY);
        assert_eq!(
            picture_data(over.as_bytes(), &small, &mut budget),
            Err(PictureError::Base64(B64Error::TooLong {
                needed: 49,
                max: 48,
            }))
        );
    }

    /// Every picture comment counts, read or not.
    ///
    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn reads_as_many_pictures_as_the_limit_and_records_the_rest() {
        let one = limits(&[(LimitKind::Pictures, 1)]);
        let value = kit::base64(&cover().build());
        let block = CommentBlock::new(b"")
            .field("METADATA_BLOCK_PICTURE", "!")
            .field("METADATA_BLOCK_PICTURE", &value)
            .build();
        let (result, steps) = read_at(&block, 0, &one);
        assert_eq!(
            result,
            Ok(Comments {
                problems: vec![
                    FieldProblem::Picture {
                        offset: 35,
                        error: PictureError::Base64(B64Error::InvalidLength { len: 1 }),
                    },
                    FieldProblem::Limit(ParseFault::LimitExceeded {
                        limit: LimitKind::Pictures,
                        value: 2,
                        max: 1,
                        offset: 40,
                    }),
                ],
                ..plain("", vec![], 131)
            })
        );
        // The refused picture's value is never decoded, so never paid for.
        assert_eq!(steps, 132);
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn reads_16_pictures_by_default_and_records_the_17th() {
        let value = kit::base64(&cover().build());
        let mut block = CommentBlock::new(b"");
        for _ in 0..17 {
            block.field("METADATA_BLOCK_PICTURE", &value);
        }
        let comments = read(&block.build()).unwrap();
        let expected: Vec<_> = (0..16).map(|index| cover_at(35 + 95 * index, 68)).collect();
        assert_eq!(comments.pictures, expected);
        assert_eq!(
            comments.problems,
            [FieldProblem::Limit(ParseFault::LimitExceeded {
                limit: LimitKind::Pictures,
                value: 17,
                max: 16,
                offset: 12 + 95 * 16,
            })]
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn caps_a_pictures_media_type_as_short_text_and_description_as_long_text() {
        // The name METADATA_BLOCK_PICTURE is 22 octets, so short text stays
        // at least that long here.
        let small = limits(&[(LimitKind::ShortText, 22), (LimitKind::LongText, 3)]);
        let picture = kit::Picture {
            mime: "image/x-a-very-long-media-type".to_owned(),
            description: "Front\u{1}".to_owned(),
            ..cover()
        };
        let value = kit::base64(&picture.build());
        let block = CommentBlock::new(b"")
            .field("METADATA_BLOCK_PICTURE", &value)
            .build();
        let comments = read_at(&block, 0, &small).0.unwrap();
        assert_eq!(
            comments.pictures,
            [Picture {
                mime: cut("image/x-a-very-long-me"),
                description: cut("Fro"),
                ..cover_at(35, 96)
            }]
        );
        let picture = kit::Picture {
            mime: "image/\u{1}png".to_owned(),
            description: "Front\u{1}".to_owned(),
            ..cover()
        };
        let value = kit::base64(&picture.build());
        let block = CommentBlock::new(b"")
            .field("METADATA_BLOCK_PICTURE", &value)
            .build();
        assert_eq!(
            read(&block).unwrap().pictures,
            [Picture {
                len: 72,
                ..cover_at(35, 68)
            }]
        );
    }

    #[test]
    fn hands_over_the_image_data_a_picture_value_holds() {
        let value = kit::base64(&cover().build());
        let mut budget = Budget::for_input(0, 0, 68);
        assert_eq!(
            picture_data(value.as_bytes(), &Limits::DEFAULT, &mut budget),
            Ok(cover().data)
        );
        assert_eq!(budget.remaining(), 0);
        let mut budget = Budget::for_input(0, 0, 67);
        assert_eq!(
            picture_data(value.as_bytes(), &Limits::DEFAULT, &mut budget),
            Err(PictureError::Fault(ParseFault::BudgetExceeded {
                offset: 0
            }))
        );
        let mut budget = Budget::for_input(0, 0, PLENTY);
        assert_eq!(
            picture_data(b"AAAAA", &Limits::DEFAULT, &mut budget),
            Err(PictureError::Base64(B64Error::InvalidLength { len: 5 }))
        );
        assert_eq!(
            picture_data(
                &kit::base64(&[0, 0, 0, 3, 0, 0]).into_bytes(),
                &Limits::DEFAULT,
                &mut budget
            ),
            Err(PictureError::Fault(ParseFault::Truncated {
                offset: 4,
                needed: 4,
                available: 2,
            }))
        );
        let empty = picture_of(0);
        assert_eq!(
            picture_data(empty.as_bytes(), &Limits::DEFAULT, &mut budget),
            Ok(vec![])
        );
        // An extra octet after the declared data is not image data.
        let mut trailing = cover().build();
        trailing.push(0xFF);
        let value = kit::base64(&trailing);
        assert_eq!(
            picture_data(value.as_bytes(), &Limits::DEFAULT, &mut budget),
            Ok(cover().data)
        );
        let block = CommentBlock::new(b"")
            .field("METADATA_BLOCK_PICTURE", &value)
            .build();
        assert_eq!(
            read(&block),
            Ok(Comments {
                pictures: vec![cover_at(35, 68)],
                ..plain("", vec![], 103)
            })
        );
    }

    /// Every cut of [`cover`]'s 50 octets leaves one part incomplete: the
    /// parts of a FLAC picture, from RFC 9639 section 8.8, with the
    /// cover's 9-octet media type, 5-octet description and 4 octets of
    /// data.
    ///
    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn every_cut_of_a_picture_names_the_exact_part_it_truncates() {
        const PARTS: [(u64, u64); 11] = [
            (0, 4),  // type
            (4, 4),  // media type length
            (8, 9),  // media type
            (17, 4), // description length
            (21, 5), // description
            (26, 4), // width
            (30, 4), // height
            (34, 4), // depth
            (38, 4), // colours
            (42, 4), // data length
            (46, 4), // data
        ];
        let whole = cover().build();
        for cut in 0..whole.len() {
            let cut_at = cut as u64;
            let (start, needed) = PARTS
                .into_iter()
                .find(|&(start, needed)| cut_at < start + needed)
                .unwrap();
            let fault = PictureError::Fault(ParseFault::Truncated {
                offset: start,
                needed,
                available: cut_at - start,
            });
            let value = kit::base64(&whole[..cut]);
            let mut budget = Budget::for_input(0, 0, PLENTY);
            assert_eq!(
                picture_data(value.as_bytes(), &Limits::DEFAULT, &mut budget),
                Err(fault),
                "cut at {cut}"
            );
            let block = CommentBlock::new(b"")
                .field("METADATA_BLOCK_PICTURE", &value)
                .build();
            assert_eq!(
                read(&block).map(|comments| comments.problems),
                Ok(vec![FieldProblem::Picture {
                    offset: 35,
                    error: fault,
                }]),
                "cut at {cut}"
            );
        }
    }

    /// A comment block whose one picture's image data is another comment
    /// block with a picture, `levels` deep.
    fn nested(levels: usize) -> Vec<u8> {
        (0..levels).fold(CommentBlock::new(b"innermost").build(), |inner, _| {
            let picture = kit::Picture {
                data: inner,
                ..cover()
            };
            CommentBlock::new(b"")
                .field("METADATA_BLOCK_PICTURE", &kit::base64(&picture.build()))
                .build()
        })
    }

    /// A comment block holds no nested structure: a picture's image data is
    /// never read as comments, however many blocks it holds inside each
    /// other, so the parse cannot descend, recurse or deepen its stack.
    ///
    /// Verifies: SEC-MED-001, SEC-MED-005
    #[test]
    fn never_descends_into_a_picture_that_holds_comment_blocks() {
        let outer = nested(12);
        let inner = nested(11);
        let picture = kit::base64(
            &kit::Picture {
                data: inner.clone(),
                ..cover()
            }
            .build(),
        );
        let result = on_small_stack(move || read(&outer));
        assert_eq!(
            result,
            Ok(Comments {
                pictures: vec![Picture {
                    data_len: u32::try_from(inner.len()).unwrap(),
                    ..cover_at(35, picture.len() as u64)
                }],
                ..plain("", vec![], 35 + picture.len() as u64)
            })
        );
    }

    /// Runs `work` on a thread with the 256 KiB stack SEC-MED-001 names, so
    /// a parse that recursed deeply would fail here rather than pass on the
    /// test runner's larger stack.
    fn on_small_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(262_144)
            .spawn(work)
            .unwrap()
            .join()
            .unwrap()
    }

    /// One comment a generated block holds.
    #[derive(Debug, Clone)]
    enum Generated {
        /// `key=value`, valid.
        Text(String, String),
        /// A valid picture.
        Picture(kit::Picture),
        /// Any octets.
        Raw(Vec<u8>),
    }

    /// Names from the octets a name may hold.
    fn key() -> impl Strategy<Value = String> {
        vec(0x20_u8..=0x7D, 1..8).prop_map(|octets| {
            octets
                .into_iter()
                .map(|octet| char::from(if octet == b'=' { b'_' } else { octet }))
                .collect()
        })
    }

    /// Values with no control character, so they decode to themselves.
    fn value() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 =é𝄞.,]{0,12}"
    }

    fn picture() -> impl Strategy<Value = kit::Picture> {
        (
            any::<u32>(),
            "[a-z/]{0,10}",
            "[a-zA-Z é]{0,10}",
            any::<[u32; 4]>(),
            vec(any::<u8>(), 0..40),
        )
            .prop_map(
                |(kind, mime, description, [width, height, depth, colours], data)| kit::Picture {
                    kind,
                    mime,
                    description,
                    width,
                    height,
                    depth,
                    colours,
                    data,
                },
            )
    }

    fn generated() -> impl Strategy<Value = Generated> {
        prop_oneof![
            4 => (key(), value()).prop_map(|(key, value)| Generated::Text(key, value)),
            1 => picture().prop_map(Generated::Picture),
            1 => vec(any::<u8>(), 0..12).prop_map(Generated::Raw),
        ]
    }

    /// A generated block: its vendor and comments.
    fn block() -> impl Strategy<Value = (Vec<u8>, Vec<Generated>)> {
        (vec(any::<u8>(), 0..8), vec(generated(), 0..8))
    }

    /// The octets of a generated block, through the testkit.
    fn build(vendor: &[u8], comments: &[Generated]) -> Vec<u8> {
        let mut block = CommentBlock::new(vendor);
        for comment in comments {
            match comment {
                Generated::Text(key, value) => block.field(key, value),
                Generated::Picture(picture) => {
                    block.field("METADATA_BLOCK_PICTURE", &kit::base64(&picture.build()))
                }
                Generated::Raw(octets) => block.raw(octets),
            };
        }
        block.build()
    }

    /// The octets each comment takes, and the charge for decoding it if it
    /// is a picture, as the generator wrote them.
    fn charges(comments: &[Generated]) -> Vec<(u64, u64)> {
        comments
            .iter()
            .map(|comment| match comment {
                Generated::Text(key, value) => ((key.len() + 1 + value.len()) as u64, 0),
                Generated::Picture(picture) => {
                    let value = kit::base64(&picture.build()).len() as u64;
                    (23 + value, value)
                }
                Generated::Raw(octets) => (octets.len() as u64, 0),
            })
            .collect()
    }

    proptest! {
        /// Every input returns, on a small stack, with each comment
        /// accounted for exactly once, every offset inside the block and at
        /// most two steps spent per octet.
        ///
        /// Verifies: SEC-MED-001, SEC-MED-007, SEC-MED-008, SEC-TM-032, SEC-HIS-036
        #[test]
        fn returns_for_any_input_within_two_steps_per_octet(
            bytes in prop_oneof![
                vec(any::<u8>(), 0..96),
                (block(), vec((any::<usize>(), any::<u8>()), 0..4)).prop_map(
                    |((vendor, comments), flips)| {
                        let mut bytes = build(&vendor, &comments);
                        for (at, octet) in flips {
                            let at = at % bytes.len();
                            bytes[at] = octet;
                        }
                        bytes
                    }
                ),
            ],
            at in prop_oneof![0_u64..1_000, Just(u64::MAX - 4_096)],
        ) {
            let len = bytes.len() as u64;
            let copy = bytes.clone();
            let (result, steps) = on_small_stack(move || read_at(&copy, at, &Limits::DEFAULT));
            prop_assert!(steps <= 2 * len, "{} steps for {} octets", steps, len);
            match result {
                Ok(comments) => {
                    let vendor = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
                    let count = u32::from_le_bytes(bytes[4 + vendor..8 + vendor].try_into().unwrap());
                    let accounted = comments.fields.len() + comments.pictures.len() + comments.problems.len();
                    prop_assert_eq!(accounted as u64, u64::from(count));
                    prop_assert!(comments.end <= at + len);
                    for found in &comments.fields {
                        prop_assert!(!found.key.is_empty());
                        prop_assert!(found.key.bytes().all(|octet| (0x20..=0x7D).contains(&octet)
                            && octet != b'=' && !octet.is_ascii_lowercase()));
                    }
                    for picture in &comments.pictures {
                        prop_assert!(picture.offset + picture.len <= comments.end);
                    }
                }
                Err(fault) => prop_assert!(fault.offset() <= at + len, "{:?}", fault),
            }
        }

        /// A block the testkit wrote reads back as exactly what it holds,
        /// for exactly the steps its octets and pictures cost.
        #[test]
        fn reads_back_every_block_the_testkit_writes((vendor, comments) in block()) {
            let bytes = build(&vendor, &comments);
            let (result, steps) = read_at(&bytes, 0, &Limits::DEFAULT);
            let mut fields = vec![];
            let mut pictures = vec![];
            let mut problems = vec![];
            let mut start = 12 + vendor.len() as u64;
            let mut decoded = 0;
            for (comment, (len, decode)) in comments.iter().zip(charges(&comments)) {
                match comment {
                    Generated::Text(key, value) => fields.push(field(&key.to_ascii_uppercase(), value)),
                    Generated::Picture(picture) => pictures.push(Picture {
                        offset: start + 23,
                        len: decode,
                        kind: picture.kind,
                        mime: text(&picture.mime),
                        description: text(&picture.description),
                        width: picture.width,
                        height: picture.height,
                        depth: picture.depth,
                        colours: picture.colours,
                        data_len: u32::try_from(picture.data.len()).unwrap(),
                    }),
                    // Under eleven octets, too short to name a picture.
                    Generated::Raw(octets) => match octets.iter().position(|&octet| octet == b'=') {
                        None => problems.push(FieldProblem::NoSeparator { offset: start }),
                        Some(0) => problems.push(FieldProblem::EmptyKey { offset: start }),
                        Some(end) => match octets[..end].iter().position(|octet| !(0x20..=0x7D).contains(octet)) {
                            Some(bad) => problems.push(FieldProblem::KeyOctet {
                                offset: start + bad as u64,
                                octet: octets[bad],
                            }),
                            None => fields.push(Field {
                                key: String::from_utf8(octets[..end].to_ascii_uppercase()).unwrap(),
                                value: crate::text::decode(
                                    crate::untrusted::Untrusted::new(&octets[end + 1..]),
                                    crate::text::Encoding::Utf8,
                                    65_536,
                                ),
                            }),
                        },
                    },
                }
                decoded += decode;
                start += 4 + len;
            }
            let end = bytes.len() as u64;
            prop_assert_eq!(steps, end + decoded);
            prop_assert_eq!(
                result,
                Ok(Comments {
                    vendor: crate::text::decode(
                        crate::untrusted::Untrusted::new(vendor.as_slice()),
                        crate::text::Encoding::Utf8,
                        4_096,
                    ),
                    fields,
                    pictures,
                    problems,
                    end,
                })
            );
        }

        /// Verifies: SEC-MED-001, SEC-TM-032
        #[test]
        fn every_cut_of_a_block_the_testkit_writes_names_the_part_it_truncates(
            (vendor, comments) in block(),
            cut_seed in any::<usize>(),
            at in 0_u64..1_000,
        ) {
            let bytes = build(&vendor, &comments);
            let cut = cut_seed % bytes.len();
            let lens: Vec<u64> = charges(&comments).into_iter().map(|(len, _)| len).collect();
            let expected = truncation(vendor.len() as u64, &lens, cut as u64, at);
            prop_assert!(expected.is_some());
            prop_assert_eq!(read_at(&bytes[..cut], at, &Limits::DEFAULT).0, Err(expected.unwrap()));
        }

        /// A parse succeeds exactly when the budget covers every charge, and
        /// otherwise fails at the first charge it cannot pay.
        ///
        /// Verifies: SEC-MED-007
        #[test]
        fn runs_out_of_budget_exactly_at_the_first_charge_it_cannot_pay(
            (vendor, comments) in block(),
            budget_seed in any::<u64>(),
            at in 0_u64..1_000,
        ) {
            let bytes = build(&vendor, &comments);
            // Independently of the parser: what it charges, and where.
            let mut paid = vec![(at, 8 + vendor.len() as u64)];
            let mut start = 8 + vendor.len() as u64;
            for (comment, (len, decode)) in comments.iter().zip(charges(&comments)) {
                paid.push((at + start, 4 + len));
                if matches!(comment, Generated::Picture(_)) {
                    paid.push((at + start + 27, decode));
                }
                start += 4 + len;
            }
            let total: u64 = paid.iter().map(|&(_, steps)| steps).sum();
            let steps = budget_seed % (total + 2);
            let mut left = steps;
            let failed = paid.iter().find(|&&(_, cost)| {
                let short = cost > left;
                left = left.saturating_sub(cost);
                short
            });
            let mut budget = Budget::for_input(0, 0, steps);
            let result = parse(&bytes, at, &Limits::DEFAULT, &mut budget).map(|comments| comments.end);
            if let Some(&(offset, _)) = failed {
                prop_assert_eq!(result, Err(ParseFault::BudgetExceeded { offset }));
                prop_assert_eq!(budget.remaining(), 0);
            } else {
                prop_assert_eq!(result, Ok(at + bytes.len() as u64));
                prop_assert_eq!(budget.remaining(), steps - total);
            }
        }

        /// Any picture, with whitespace anywhere in its value, reads back as
        /// its reference and hands back its image data.
        #[test]
        fn reads_back_any_picture_with_whitespace_anywhere(
            picture in picture(),
            spaces in vec((any::<usize>(), prop_oneof![Just(' '), Just('\t'), Just('\r'), Just('\n')]), 0..6),
        ) {
            let mut value = kit::base64(&picture.build());
            for (at, space) in spaces {
                value.insert(at % (value.len() + 1), space);
            }
            let block = CommentBlock::new(b"").field("METADATA_BLOCK_PICTURE", &value).build();
            prop_assert_eq!(
                read(&block).map(|comments| comments.pictures),
                Ok(vec![Picture {
                    offset: 35,
                    len: value.len() as u64,
                    kind: picture.kind,
                    mime: text(&picture.mime),
                    description: text(&picture.description),
                    width: picture.width,
                    height: picture.height,
                    depth: picture.depth,
                    colours: picture.colours,
                    data_len: u32::try_from(picture.data.len()).unwrap(),
                }])
            );
            let mut budget = Budget::for_input(0, 0, PLENTY);
            prop_assert_eq!(picture_data(value.as_bytes(), &Limits::DEFAULT, &mut budget), Ok(picture.data));
        }
    }
}
