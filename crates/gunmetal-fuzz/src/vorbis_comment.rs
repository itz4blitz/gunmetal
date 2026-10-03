//! The harness for the Vorbis comment parser in
//! [`gunmetal_core::formats::vorbis_comment`], which reads the tags of
//! FLAC, Ogg Vorbis and Opus files and the pictures embedded in them.

use gunmetal_core::formats::vorbis_comment::{
    self, Comments, FIXED_STEPS, FieldProblem, PictureError, STEPS_PER_OCTET,
};
use gunmetal_core::parse::{Budget, LimitKind, Limits, ParseFault};

/// Comments a block may declare: low, so fuzzing reaches the limit often.
pub const TAG_FIELDS: u64 = 8;
/// Octets of a short text field. A picture's name, `METADATA_BLOCK_PICTURE`,
/// takes 22.
pub const SHORT_TEXT: u64 = 32;
/// Octets of a long text field.
pub const LONG_TEXT: u64 = 64;
/// Octets a picture may decode to, so its base64 value may take 88.
pub const PICTURE_BYTES: u64 = 64;
/// Pictures a block may hold.
pub const PICTURES: u64 = 2;

/// What the parser reported for one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// [`vorbis_comment::parse`] of the input as a comment block at file
    /// offset 0.
    pub comments: Result<Comments, ParseFault>,
    /// [`vorbis_comment::picture_data`] of the input as one picture's
    /// base64 value.
    pub picture: Result<Vec<u8>, PictureError>,
}

/// The limits every call runs under: the defaults, with the ones the
/// parser checks lowered to this module's constants.
///
/// # Panics
///
/// Panics if a constant is above its limit's ceiling, which none is.
#[must_use]
pub fn limits() -> Limits {
    [
        (LimitKind::TagFields, TAG_FIELDS),
        (LimitKind::ShortText, SHORT_TEXT),
        (LimitKind::LongText, LONG_TEXT),
        (LimitKind::PictureBytes, PICTURE_BYTES),
        (LimitKind::Pictures, PICTURES),
    ]
    .into_iter()
    .try_fold(Limits::DEFAULT, |limits, (kind, value)| {
        limits.with_override(kind, value)
    })
    .expect("every harness limit is below its ceiling")
}

/// Feeds `data` to [`vorbis_comment::parse`] as a block under [`limits`],
/// with a budget of exactly [`STEPS_PER_OCTET`] steps per octet plus
/// [`FIXED_STEPS`], and to [`vorbis_comment::picture_data`] as a picture's
/// value with one step per octet. Every picture the parse finds is read
/// back through `picture_data`, and every field is looked up through
/// [`Comments::values`].
///
/// # Panics
///
/// Panics when a result breaks an invariant that holds for every input:
/// either call running out of the budget its documentation promises is
/// enough; an offset past the input; a declared comment not accounted for
/// exactly once among the fields, pictures and problems; more comments or
/// pictures than the limits allow; a name that is empty, longer than short
/// text, or holds an octet outside 0x20 to 0x7D or a lower-case letter;
/// text over its cap; a field its own name does not find; a picture whose
/// value does not read back to as many octets of image data as it
/// declares; or image data over the picture limit.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let limits = limits();
    let len = u64::try_from(data.len()).unwrap_or(u64::MAX);
    let mut budget = Budget::for_input(len, STEPS_PER_OCTET, FIXED_STEPS);
    let comments = vorbis_comment::parse(data, 0, &limits, &mut budget);
    if let Err(fault) = &comments {
        assert!(
            fault.offset() <= len
                && *fault
                    != ParseFault::BudgetExceeded {
                        offset: fault.offset(),
                    },
            "parse failed with {fault:?} for {len} octets"
        );
    }
    if let Ok(found) = &comments {
        // The count, read independently of the parser: the 32-bit
        // little-endian number after the vendor string.
        assert!(
            data.get(..4)
                .and_then(|octets| <[u8; 4]>::try_from(octets).ok())
                .map(|octets| u64::from(u32::from_le_bytes(octets)))
                .and_then(|vendor| {
                    data.get(
                        usize::try_from(vendor.saturating_add(4)).unwrap_or(usize::MAX)
                            ..usize::try_from(vendor.saturating_add(8)).unwrap_or(usize::MAX),
                    )
                })
                .and_then(|octets| <[u8; 4]>::try_from(octets).ok())
                .map(|octets| u64::from(u32::from_le_bytes(octets)))
                == Some((found.fields.len() + found.pictures.len() + found.problems.len()) as u64)
                && (found.fields.len() + found.pictures.len() + found.problems.len()) as u64
                    <= TAG_FIELDS
                && found.pictures.len() as u64 <= PICTURES
                && found.end <= len
                && found.vendor.value.len() as u64 <= SHORT_TEXT,
            "{found:?} from {len} octets"
        );
        for field in &found.fields {
            assert!(
                !field.key.is_empty()
                    && field.key.len() as u64 <= SHORT_TEXT
                    && field.key.bytes().all(|octet| {
                        (0x20..=0x7D).contains(&octet)
                            && octet != b'='
                            && !octet.is_ascii_lowercase()
                    })
                    && field.value.value.len() as u64 <= LONG_TEXT
                    && found.values(&field.key).any(|value| *value == field.value),
                "field {field:?}"
            );
        }
        for picture in &found.pictures {
            assert!(
                picture.offset.saturating_add(picture.len) <= found.end
                    && picture.mime.value.len() as u64 <= SHORT_TEXT
                    && picture.description.value.len() as u64 <= LONG_TEXT
                    && data
                        .get(
                            usize::try_from(picture.offset).unwrap_or(usize::MAX)
                                ..usize::try_from(picture.offset.saturating_add(picture.len))
                                    .unwrap_or(usize::MAX),
                        )
                        .map(|value| {
                            vorbis_comment::picture_data(
                                value,
                                &limits,
                                &mut Budget::for_input(picture.len, 1, 0),
                            )
                            .map(|image| image.len() as u64)
                        })
                        == Some(Ok(u64::from(picture.data_len))),
                "picture {picture:?}"
            );
        }
        for problem in &found.problems {
            assert!(
                match *problem {
                    FieldProblem::NoSeparator { offset }
                    | FieldProblem::EmptyKey { offset }
                    | FieldProblem::KeyOctet { offset, .. }
                    | FieldProblem::Picture { offset, .. } => offset,
                    FieldProblem::Limit(fault) => fault.offset(),
                } <= found.end,
                "problem {problem:?}"
            );
        }
    }
    let picture = vorbis_comment::picture_data(data, &limits, &mut Budget::for_input(len, 1, 0));
    assert!(
        match &picture {
            Ok(image) => image.len() as u64 <= PICTURE_BYTES,
            Err(error) => *error != PictureError::Fault(ParseFault::BudgetExceeded { offset: 0 }),
        },
        "picture_data returned {picture:?} for {len} octets"
    );
    Outcome { comments, picture }
}
