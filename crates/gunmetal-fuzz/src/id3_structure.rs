//! The structure-aware harness for the ID3 family (SEC-MED-031). It holds
//! two generators that read the same input as two different recipes:
//! [`run`] for the tags at the end of a file, an APE tag and the `ID3v1` tag
//! after it, and [`id3v2`] for an `ID3v2` tag. The fuzz target feeds every
//! input to both.
//!
//! # The tags at the end of a file
//!
//! The fuzzer's octets are a recipe, not a file. The harness builds a file
//! from them whose framing is sound, an APE tag whose header, footer, size
//! and count agree with its items and an `ID3v1` tag in the file's last 128
//! octets, around fuzzed keys, item flags, values and `ID3v1` fields. The
//! fuzzer then spends its time in the item and field code that random
//! octets seldom reach. The harness checks that the parsers find exactly
//! the framing it built.
//!
//! A recipe reads, one octet at a time and zero once it runs out: options
//! (bit 0 a header, bit 1 version 1000 rather than 2000, bit 2 an `ID3v1`
//! tag), the length of the audio before the tag (0 to 15) and its octets,
//! the item count (0 to 8), then each item's flags (0 to 7), key length (0
//! to 11) and key, and value length (0 to 31) and value, then the `ID3v1`
//! tag's 125 octets after `TAG`.
//!
//! # `ID3v2` tags
//!
//! Random octets rarely make a tag whose frames line up, so [`id3v2`] also
//! reads its input as a recipe and writes a tag with valid framing around
//! octets the fuzzer chooses:
//!
//! - The first octet picks the version (2.2, 2.3 or 2.4 by its value
//!   modulo 3), sets the tag's unsynchronisation flag with bit `0x10`, and
//!   in 2.4 adds a footer with bit `0x20`.
//! - Every three octets after it describe one frame, up to
//!   [`MAX_FRAMES`]: which identifier it has from [`IDS`] or [`IDS_V22`],
//!   its flags, and how many of the octets that follow are its body.
//!
//! It then reads the tag back and checks that every frame it wrote was
//! found where it wrote it, or recorded as skipped there.

use std::ops::Range;

use gunmetal_core::formats::ape::{self, ApeError, ApeTag, ApeValue, ItemProblem};
use gunmetal_core::formats::id3v1::{self, Id3v1Error, Id3v1Tag};
use gunmetal_core::formats::id3v2::{
    self, BUDGET_FIXED, BUDGET_PER_OCTET, FrameId, Id3v2Error, Id3v2Tag, TagProblem,
};
use gunmetal_core::parse::{Budget, Limits, Window};

/// The tag flags of a footer whose tag has a header: bit 31.
const HAS_HEADER: u32 = 0x8000_0000;

/// The tag flags of a header: bit 31, and bit 29 for "this is the header".
const HEADER: u32 = 0xA000_0000;

/// The file a recipe built and what the parsers found in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The file the harness built from the recipe.
    pub file: Vec<u8>,
    /// Where the harness put the APE tag.
    pub ape_range: Range<u64>,
    /// What [`ape::parse_ape`] found in the whole file.
    pub ape: Result<Option<ApeTag>, ApeError>,
    /// What [`id3v1::find_v1`] found in the whole file.
    pub v1: Result<Option<Id3v1Tag>, Id3v1Error>,
    /// Whether the harness checked the parsers against what it built. It
    /// does not when a file built without an `ID3v1` tag happens to have
    /// `TAG` 128 octets from its end, because the file then truly ends
    /// with one.
    pub checked: bool,
}

/// The octets of a recipe, read one at a time.
struct Recipe<'a> {
    rest: &'a [u8],
}

impl Recipe<'_> {
    /// The next octet, or zero once the recipe has run out.
    fn octet(&mut self) -> u8 {
        let (first, rest) = self
            .rest
            .split_first()
            .map_or((0, &[][..]), |(first, rest)| (*first, rest));
        self.rest = rest;
        first
    }

    /// The next `len` octets, zeros past the end of the recipe.
    fn octets(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.octet()).collect()
    }
}

/// One item a recipe asked for.
#[derive(Debug)]
struct Planned {
    flags: u32,
    key: Vec<u8>,
    value: Vec<u8>,
}

/// A file built from a recipe, and where its parts are.
struct Built {
    file: Vec<u8>,
    ape_range: Range<u64>,
    planned: Vec<Planned>,
    /// Where each planned item starts, and where its value lies.
    places: Vec<(u64, Range<u64>)>,
    /// The `ID3v1` tag's octets after `TAG`, when one was built.
    fields: Option<Vec<u8>>,
}

/// Builds a file from the recipe `data`, parses it with both parsers, and
/// checks they found the framing that was built.
///
/// # Panics
///
/// Panics when a parser does not find what the harness built: the APE tag
/// at its range, every item either read under its key, with a binary or
/// reserved value at its place, or skipped at its place for a key the
/// specification forbids, and the `ID3v1` tag in the last 128 octets with
/// its genre, or no `ID3v1` tag when none was built.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let Built {
        file,
        ape_range,
        planned,
        places,
        fields,
    } = build(data);
    let len = offset(file.len());
    let window = Window {
        offset: 0,
        bytes: &file,
        file_len: len,
    };
    let ape = ape::parse_ape(window, &Limits::DEFAULT, &mut Budget::for_input(len, 1, 2));
    let v1 = id3v1::find_v1(window, &Limits::DEFAULT, &mut Budget::for_input(0, 0, 1));
    let checked = fields.is_some()
        || file
            .len()
            .checked_sub(128)
            .and_then(|at| file.get(at..))
            .is_none_or(|last| !last.starts_with(b"TAG"));
    if checked {
        let tag = ape.as_ref().ok().and_then(Option::as_ref);
        assert!(
            tag.is_some_and(|tag| tag.range == ape_range),
            "{ape:?} where a tag was built at {ape_range:?}"
        );
        let (items, problems) = tag.map_or((&[][..], &[][..]), |tag| {
            (tag.items.as_slice(), tag.problems.as_slice())
        });
        let mut read = items.iter();
        let mut skipped = problems.iter();
        for (plan, (item_at, value_at)) in planned.iter().zip(&places) {
            if allowed(&plan.key) {
                let item = read.next();
                assert!(
                    item.is_some_and(|item| {
                        item.key.as_bytes() == plan.key
                            && match &item.value {
                                ApeValue::Text(_) => plan.flags >> 1 == 0,
                                ApeValue::Binary(at) => plan.flags >> 1 == 1 && at == value_at,
                                ApeValue::Locator(_) => plan.flags >> 1 == 2,
                                ApeValue::Reserved(at) => plan.flags >> 1 == 3 && at == value_at,
                            }
                    }),
                    "{item:?} where {plan:?} was built at {item_at}"
                );
            } else {
                let problem = skipped.next();
                assert!(
                    problem == Some(&ItemProblem::BadKey { offset: *item_at }),
                    "{problem:?} where {plan:?} was built at {item_at}"
                );
            }
        }
        assert!(
            read.next().is_none() && skipped.next().is_none(),
            "{items:?} and {problems:?} hold more than was built"
        );
        let wanted = fields.as_ref().map(|fields| {
            let genre = fields.last().copied().unwrap_or_default();
            (len.saturating_sub(128)..len, genre)
        });
        let found = v1
            .clone()
            .map(|found| found.map(|tag| (tag.range, tag.genre)));
        assert!(
            found == Ok(wanted.clone()),
            "{v1:?} where {wanted:?} was built"
        );
    }
    Outcome {
        file,
        ape_range,
        ape,
        v1,
        checked,
    }
}

/// The file the recipe `data` asks for, and where its parts are.
fn build(data: &[u8]) -> Built {
    let mut recipe = Recipe { rest: data };
    let options = recipe.octet();
    let version: u32 = if options & 2 == 0 { 2_000 } else { 1_000 };
    let audio_len = usize::from(recipe.octet() % 16);
    let mut file = recipe.octets(audio_len);
    let count = recipe.octet() % 9;
    let planned: Vec<Planned> = (0..count)
        .map(|_| {
            let flags = u32::from(recipe.octet() & 0b111);
            let key_len = usize::from(recipe.octet() % 12);
            // A zero would end the key early and break the framing.
            let key = recipe
                .octets(key_len)
                .into_iter()
                .map(|octet| octet.max(1))
                .collect();
            let value_len = usize::from(recipe.octet() % 32);
            let value = recipe.octets(value_len);
            Planned { flags, key, value }
        })
        .collect();

    // Each item takes its two integers, its key, a zero and its value; the
    // size counts the footer too.
    let items_len: usize = planned
        .iter()
        .map(|plan| plan.key.len() + plan.value.len() + 9)
        .sum();
    let size = u32::try_from(items_len + 32).unwrap_or(u32::MAX);
    let start = offset(file.len());
    let header = options & 1 != 0;
    if header {
        file.extend(block(version, size, count, HEADER));
    }
    let mut places = Vec::new();
    for plan in &planned {
        let item_at = offset(file.len());
        file.extend(
            u32::try_from(plan.value.len())
                .unwrap_or(u32::MAX)
                .to_le_bytes(),
        );
        file.extend(plan.flags.to_le_bytes());
        file.extend(&plan.key);
        file.push(0);
        let value_at = offset(file.len());
        file.extend(&plan.value);
        places.push((item_at, value_at..offset(file.len())));
    }
    file.extend(block(
        version,
        size,
        count,
        if header { HAS_HEADER } else { 0 },
    ));
    let ape_range = start..offset(file.len());
    let fields = (options & 4 != 0).then(|| recipe.octets(125));
    if let Some(fields) = &fields {
        file.extend(b"TAG");
        file.extend(fields);
    }
    Built {
        file,
        ape_range,
        planned,
        places,
        fields,
    }
}

/// `at` as a file offset.
fn offset(at: usize) -> u64 {
    u64::try_from(at).unwrap_or(u64::MAX)
}

/// A header or footer block.
fn block(version: u32, size: u32, count: u8, flags: u32) -> Vec<u8> {
    let mut block = b"APETAGEX".to_vec();
    for value in [version, size, u32::from(count), flags] {
        block.extend(value.to_le_bytes());
    }
    block.extend([0; 8]);
    block
}

/// Whether the specification allows `key`, worked out here rather than
/// taken from the parser.
fn allowed(key: &[u8]) -> bool {
    (2..=255).contains(&key.len())
        && key.iter().all(|&octet| (0x20..=0x7E).contains(&octet))
        && !["ID3", "TAG", "OGGS", "MP+"]
            .iter()
            .any(|forbidden| key.eq_ignore_ascii_case(forbidden.as_bytes()))
}

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
pub struct Id3v2Outcome {
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
pub fn id3v2(data: &[u8]) -> Id3v2Outcome {
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
    Id3v2Outcome { tag, read }
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
        let with_flag = id3v2(&[0x02, 11, 0x02, 2, 0xFF, 0x00]);
        assert_eq!(
            with_flag.tag,
            Tag::new(Version::V24)
                .frame(b"PRIV", 0x0002, b"\xFF\x00\x00")
                .build()
        );
        let without = id3v2(&[0x02, 11, 0x00, 2, 0xFF, 0x00]);
        assert_eq!(
            without.tag,
            Tag::new(Version::V24)
                .frame(b"PRIV", 0, b"\xFF\x00")
                .build()
        );
        // 0x11: 2.4, tag unsynchronised, no footer.
        let tag_flag = id3v2(&[0x11, 11, 0x00, 2, 0xFF, 0x00]);
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
        assert_eq!(id3v2(&[0x20]).tag, Tag::new(Version::V24).footer().build());
        assert_eq!(
            id3v2(&[0x32]).tag,
            Tag::new(Version::V24).unsynchronised().footer().build()
        );
    }
}
