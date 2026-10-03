//! The structure-aware harness for `ID3v2` tags (SEC-MED-031).
//!
//! Random octets rarely make a tag whose frames line up, so this harness
//! reads its input as a recipe instead and writes a tag with valid framing
//! around octets the fuzzer chooses:
//!
//! - The first octet picks the version (2.2, 2.3 or 2.4 by its value
//!   modulo 3), sets the tag's unsynchronisation flag with bit `0x10`, and
//!   in 2.4 adds a footer with bit `0x20`.
//! - Every three octets after it describe one frame, up to
//!   [`MAX_FRAMES`]: which identifier it has from [`IDS`] or [`IDS_V22`],
//!   its flags, and how many of the octets that follow are its body.
//!
//! The harness then reads the tag back and checks that every frame it
//! wrote was found where it wrote it, or recorded as skipped there.

use gunmetal_core::formats::id3v2::{
    self, BUDGET_FIXED, BUDGET_PER_OCTET, FrameId, Id3v2Error, Id3v2Tag, TagProblem,
};
use gunmetal_core::parse::{Budget, Limits};

/// The most frames one recipe writes, well under the tag-field limit.
pub const MAX_FRAMES: usize = 64;

/// The identifiers a 2.3 or 2.4 recipe picks from: one of each kind of
/// body the parser reads, the chapter frames it walks, and frames it keeps
/// raw.
pub const IDS: [[u8; 4]; 12] = [
    *b"TIT2", *b"TXXX", *b"TIPL", *b"COMM", *b"USLT", *b"SYLT", *b"APIC", *b"UFID", *b"CHAP",
    *b"CTOC", *b"POPM", *b"PRIV",
];

/// The identifiers a 2.2 recipe picks from.
pub const IDS_V22: [[u8; 3]; 10] = [
    *b"TT2", *b"TXX", *b"IPL", *b"COM", *b"ULT", *b"SLT", *b"PIC", *b"UFI", *b"POP", *b"PRV",
];

/// What the harness wrote and what the parser read back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The tag the recipe describes.
    pub tag: Vec<u8>,
    /// [`id3v2::parse`] of it, under the default limits and the budget the
    /// parser documents for its length.
    pub read: Result<Id3v2Tag, Id3v2Error>,
}

/// Writes the tag `data` describes, then reads it with [`id3v2::parse`].
///
/// # Panics
///
/// Panics when the parser refuses the tag, whose framing is always valid,
/// or when a frame the harness wrote is neither among the frames read at
/// the offset it was written nor recorded there as skipped.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let (&first, mut rest) = data.split_first().unwrap_or((&0, &[]));
    let major = 2 + first % 3;
    let unsynchronised = first & 0x10 != 0;
    let footer = major == 4 && first & 0x20 != 0;
    let mut plain = Vec::new();
    let mut written = Vec::new();
    while written.len() < MAX_FRAMES
        && let Some((&[kind, flags, len], after)) = rest.split_first_chunk::<3>()
    {
        let (body, after) = after.split_at(usize::from(len).min(after.len()));
        let (id, frame) = frame(major, unsynchronised, kind, flags, body);
        written.push((id, plain.len()));
        plain.extend(frame);
        rest = after;
    }
    // In 2.2 and 2.3 the tag's flag unsynchronises everything after the
    // header, frame headers included, which moves every frame along.
    let whole = major < 4 && unsynchronised;
    let frames = if whole {
        unsynchronise(&plain)
    } else {
        plain.clone()
    };
    let flags = match (unsynchronised, footer) {
        (false, false) => 0,
        (true, false) => 0x80,
        (false, true) => 0x10,
        (true, true) => 0x90,
    };
    let size = syncsafe(frames.len());
    let mut tag = [&b"ID3"[..], &[major, 0, flags], &size, &frames].concat();
    if footer {
        tag.extend([&b"3DI"[..], &[major, 0, flags], &size].concat());
    }
    let expected: Vec<(FrameId, u64)> = written
        .iter()
        .map(|&(id, start)| {
            let moved = if whole { inserted(&plain, start) } else { 0 };
            (id, to_u64(10 + start + moved))
        })
        .collect();
    let len = to_u64(tag.len());
    let read = id3v2::parse(
        &tag,
        &Limits::DEFAULT,
        &mut Budget::for_input(len, BUDGET_PER_OCTET, BUDGET_FIXED),
    );
    assert!(
        read.as_ref()
            .is_ok_and(|found| handled(found, &expected) == expected),
        "wrote {expected:?}, read {read:?}"
    );
    Outcome { tag, read }
}

/// One frame of the recipe: its identifier, and its header and body as
/// `major` writes them. 2.3 takes the flags `0xE0` of `flags`, the
/// compression, encryption and grouping flags; 2.4 takes `0x4F`, those and
/// the unsynchronisation and data length flags, and unsynchronises the
/// body when the frame or the tag says so.
fn frame(major: u8, unsynchronised: bool, kind: u8, flags: u8, body: &[u8]) -> (FrameId, Vec<u8>) {
    let kind = usize::from(kind);
    if major == 2 {
        let id = IDS_V22[kind % IDS_V22.len()];
        let [_, size @ ..] = to_u32(body.len()).to_be_bytes();
        return (FrameId::Three(id), [&id[..], &size, body].concat());
    }
    let id = IDS[kind % IDS.len()];
    let (flags, size, body) = if major == 3 {
        (
            flags & 0xE0,
            to_u32(body.len()).to_be_bytes(),
            body.to_vec(),
        )
    } else {
        let flags = flags & 0x4F;
        let body = if unsynchronised || flags & 0x02 != 0 {
            unsynchronise(body)
        } else {
            body.to_vec()
        };
        (flags, syncsafe(body.len()), body)
    };
    (
        FrameId::Four(id),
        [&id[..], &size, &[0, flags], &body].concat(),
    )
}

/// The frames the parser handled at the offsets the harness wrote frames
/// at: those it read, and those it recorded as skipped or malformed there.
/// Problems inside a chapter's body lie at other offsets and are left out.
fn handled(found: &Id3v2Tag, written: &[(FrameId, u64)]) -> Vec<(FrameId, u64)> {
    let mut handled: Vec<(FrameId, u64)> = found
        .frames
        .iter()
        .map(|frame| (frame.id, frame.offset))
        .collect();
    for problem in &found.problems {
        if let TagProblem::EmptyFrame { offset, id }
        | TagProblem::Compressed { offset, id }
        | TagProblem::Encrypted { offset, id }
        | TagProblem::Malformed { offset, id } = *problem
            && written.contains(&(id, offset))
        {
            handled.push((id, offset));
        }
    }
    handled.sort_by_key(|&(_, offset)| offset);
    handled.dedup();
    handled
}

/// `plain` with the unsynchronisation scheme applied: a zero octet after
/// every `FF` followed by a zero or by `E0` or more, and after a final `FF`.
fn unsynchronise(plain: &[u8]) -> Vec<u8> {
    let mut stored = Vec::new();
    for (index, &octet) in plain.iter().enumerate() {
        stored.push(octet);
        if needs_zero(plain, index) {
            stored.push(0);
        }
    }
    stored
}

/// How many zero octets [`unsynchronise`] puts before octet `end` of
/// `plain`.
fn inserted(plain: &[u8], end: usize) -> usize {
    (0..end).filter(|&index| needs_zero(plain, index)).count()
}

/// Whether the scheme puts a zero octet after octet `index` of `plain`.
fn needs_zero(plain: &[u8], index: usize) -> bool {
    plain[index] == 0xFF
        && plain
            .get(index + 1)
            .is_none_or(|&next| next == 0 || next >= 0xE0)
}

/// `value` as a syncsafe integer: four octets of seven bits each.
fn syncsafe(value: usize) -> [u8; 4] {
    let value = to_u32(value);
    [21, 14, 7, 0].map(|shift| u8::try_from((value >> shift) & 0x7F).unwrap_or(0))
}

/// `value`, which a recipe keeps far below `u32::MAX`, as a `u32`.
fn to_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// `value` as a `u64`.
fn to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gunmetal_testkit::id3v2::{Tag, Version};

    /// A 2.4 frame with the unsynchronisation flag, or a 2.4 tag with the
    /// tag's flag, stores a zero after `FF`; the same body without either
    /// flag is stored as written.
    ///
    /// Verifies: SEC-MED-031
    #[test]
    fn unsynchronises_a_2_4_frame_only_when_the_frame_or_tag_says_so() {
        let with_flag = run(&[0x02, 11, 0x02, 2, 0xFF, 0x00]);
        assert_eq!(
            with_flag.tag,
            Tag::new(Version::V24)
                .frame(b"PRIV", 0x0002, b"\xFF\x00\x00")
                .build()
        );
        let without = run(&[0x02, 11, 0x00, 2, 0xFF, 0x00]);
        assert_eq!(
            without.tag,
            Tag::new(Version::V24)
                .frame(b"PRIV", 0, b"\xFF\x00")
                .build()
        );
        // 0x11: 2.4, tag unsynchronised, no footer.
        let tag_flag = run(&[0x11, 11, 0x00, 2, 0xFF, 0x00]);
        assert_eq!(
            tag_flag.tag,
            Tag::new(Version::V24)
                .unsynchronised()
                .frame(b"PRIV", 0, b"\xFF\x00\x00")
                .build()
        );
    }

    /// The footer bit is independent of unsynchronisation: a 2.4 recipe
    /// with only that bit set writes a footer and no unsynchronisation
    /// flag.
    ///
    /// Verifies: SEC-MED-031
    #[test]
    fn writes_a_2_4_footer_without_unsynchronising() {
        assert_eq!(run(&[0x20]).tag, Tag::new(Version::V24).footer().build());
        assert_eq!(
            run(&[0x32]).tag,
            Tag::new(Version::V24).unsynchronised().footer().build()
        );
    }
}
