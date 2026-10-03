//! APE tags, versions 1 and 2: the tag that MP3, Musepack, `WavPack` and
//! Monkey's Audio files carry at their end, before any `ID3v1` tag.
//!
//! The `APEv2` specification (Hydrogenaudio Knowledgebase) lays a tag out
//! as an optional 32-octet header, the items, then a 32-octet footer.
//! Header and footer are the same block: `APETAGEX`, the version (1000 or
//! 2000), the tag size (the items and the footer, not the header), the item
//! count and the tag flags, each a little-endian 32-bit integer, then eight
//! reserved octets. Bit 31 of the flags says the tag has a header; bit 29
//! marks the header itself. Each item is its value's length and its flags,
//! little-endian 32-bit integers, then its key, a zero octet and its value.
//! A key is 2 to 255 ASCII characters from space to tilde, and is never
//! `ID3`, `TAG`, `OggS` or `MP+` in any case. Bits 1 and 2 of an item's
//! flags give its type: UTF-8 text, binary, an external locator, or the
//! reserved type.
//!
//! The parser finds a tag by its footer, which ends the file or sits just
//! before an `ID3v1` tag ([`super::id3v1`]). `APETAGEX` holds `TAG` at
//! octets 3..6, so a headed tag of 131 octets makes the file's last 128
//! start with `TAG`; when no footer sits immediately before that slot the
//! parser looks again at the file's end. It reads the tag from a
//! [`Window`] over the end of the file, and reports its byte range so the
//! identity window can skip it (LIB-028). Text values are split at zero octets,
//! decoded as UTF-8 with control characters removed, and capped at the
//! long-text limit (SEC-MED-013). Binary and reserved values are kept as
//! byte ranges, never decoded. A locator is kept as inert text that nothing
//! follows (SEC-MED-016). Every value counts as one tag field against the
//! tag-field limit (SEC-MED-006).
//!
//! Once the footer is sound the tag's range is known, so a damaged item
//! does not lose the tag: an item with a key the specification forbids is
//! skipped, and an item that runs past the tag stops the reading; either is
//! recorded in [`ApeTag::problems`] with its offset.
//!
//! Work: one step of the [`Budget`] for the footer, and one before each
//! item. Every item takes at least nine octets, so a parse of a window of
//! `n` octets spends at most 2 + n / 9 steps (SEC-MED-007). Items do not
//! nest: a value that holds another tag is a value and is never parsed, so
//! the parse needs no depth (SEC-MED-005).
//!
//! A header-only tag, which the specification allows but which no writer
//! puts at the end of a file, cannot be found by a footer. When such a
//! header is the file's last block, the parser refuses it as
//! [`ApeError::NotAFooter`].

use std::ops::Range;

use super::id3v1;
use crate::parse::{Budget, Cursor, LimitKind, Limits, ParseFault, Window, bounded_vec};
use crate::problem::{Arg, Describe, Problem, ProblemCode};
use crate::text::{self, Encoding, Text};
use crate::untrusted::Untrusted;

/// An APE tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApeTag {
    /// Where the tag lies in the file, header and footer included.
    pub range: Range<u64>,
    /// The items read, in the order of the tag.
    pub items: Vec<ApeItem>,
    /// The items skipped, and why reading stopped early, if it did.
    pub problems: Vec<ItemProblem>,
}

/// One item of an APE tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApeItem {
    /// The key as written: 2 to 255 ASCII characters from space to tilde.
    /// Keys compare without regard to case.
    pub key: String,
    /// The value.
    pub value: ApeValue,
}

/// The value of an APE item, by the type its flags give.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApeValue {
    /// UTF-8 text: one entry per value, as zero octets separate them.
    Text(Vec<Text>),
    /// Binary data, such as a picture, as its byte range in the file.
    Binary(Range<u64>),
    /// An external locator: UTF-8 text naming a URL or a path, one entry
    /// per value. It is data to show, never a place to fetch or open
    /// (SEC-MED-016).
    Locator(Vec<Text>),
    /// The item type the specification reserves, as its byte range.
    Reserved(Range<u64>),
}

/// An item that was skipped, or why reading the items stopped early.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemProblem {
    /// Reading stopped at the item starting at the fault's offset: the
    /// tag ended inside it ([`ParseFault::Truncated`]), the budget was
    /// spent ([`ParseFault::BudgetExceeded`]), or its values took the tag
    /// past the tag-field limit ([`ParseFault::LimitExceeded`]).
    Stopped(ParseFault),
    /// Reading stopped: the key starting at `offset` has no zero octet
    /// after it before the tag's items end.
    KeyUnterminated {
        /// Where the key starts.
        offset: u64,
    },
    /// The item starting at `offset` was skipped: its key is shorter than
    /// 2 or longer than 255 octets, holds an octet outside space to tilde,
    /// or is one the specification forbids.
    BadKey {
        /// Where the item starts.
        offset: u64,
    },
}

/// Why the end of a file could not be read as an APE tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApeError {
    /// The window did not hold the octets the parse needed
    /// ([`ParseFault::Truncated`]: read them and try again), or the budget
    /// was spent ([`ParseFault::BudgetExceeded`]).
    Fault(ParseFault),
    /// The block where the footer belongs is flagged as a header.
    NotAFooter {
        /// Where the block starts.
        offset: u64,
    },
    /// The footer's version is neither 1000 nor 2000.
    UnknownVersion {
        /// Where the footer starts.
        offset: u64,
        /// The version it gives.
        version: u32,
    },
    /// The footer declares a tag smaller than the footer itself.
    TooSmall {
        /// Where the footer starts.
        offset: u64,
        /// The tag size it declares.
        size: u32,
    },
    /// The footer declares a tag that would start before the file does.
    PastFileStart {
        /// Where the footer starts.
        offset: u64,
        /// Octets the tag declares, its header included.
        size: u64,
        /// Octets from the start of the file to the end of the tag.
        room: u64,
    },
    /// The tag says it has a header, and the block before its items is not
    /// a header with the footer's version, size and count.
    HeaderMismatch {
        /// Where the header should start.
        offset: u64,
    },
}

impl ApeError {
    /// Where the parser was working when it stopped, in octets from the
    /// start of the file.
    #[must_use]
    pub const fn offset(&self) -> u64 {
        match *self {
            Self::Fault(fault) => fault.offset(),
            Self::NotAFooter { offset }
            | Self::UnknownVersion { offset, .. }
            | Self::TooSmall { offset, .. }
            | Self::PastFileStart { offset, .. }
            | Self::HeaderMismatch { offset } => offset,
        }
    }
}

impl Describe for ApeError {
    /// A tag at the end of the file was skipped, at the offset where its
    /// reading stopped.
    fn problem(&self) -> Problem {
        Problem {
            code: ProblemCode::EndTagSkipped,
            args: vec![
                ("tag", Arg::Name("APE")),
                ("offset", Arg::Number(self.offset())),
            ],
        }
    }
}

impl ItemProblem {
    /// Where the item or key the problem is about starts, in octets from
    /// the start of the file.
    #[must_use]
    pub const fn offset(&self) -> u64 {
        match *self {
            Self::Stopped(fault) => fault.offset(),
            Self::KeyUnterminated { offset } | Self::BadKey { offset } => offset,
        }
    }
}

impl Describe for ItemProblem {
    /// One field of the tag was skipped, or the fields after this offset
    /// were.
    fn problem(&self) -> Problem {
        Problem {
            code: ProblemCode::ApeItemSkipped,
            args: vec![("offset", Arg::Number(self.offset()))],
        }
    }
}

/// Octets in a header or a footer.
const BLOCK: u64 = 32;

/// The tag-flags bit that says the tag has a header.
const HAS_HEADER: u32 = 1 << 31;

/// The tag-flags bit that marks a block as the header.
const IS_HEADER: u32 = 1 << 29;

/// Octets in the smallest item a key allows: its two integers, a key of
/// two octets and the terminator.
const MIN_ITEM: u64 = 11;

/// The keys the specification forbids, compared without regard to case.
const FORBIDDEN: [&[u8]; 4] = [b"ID3", b"TAG", b"OggS", b"MP+"];

/// The fields of a header or footer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Block {
    version: u32,
    size: u32,
    count: u32,
    flags: u32,
}

/// The fields of `octets` when they start with the preamble.
fn block(octets: &[u8; 32]) -> Option<Block> {
    let [
        p0,
        p1,
        p2,
        p3,
        p4,
        p5,
        p6,
        p7,
        v0,
        v1,
        v2,
        v3,
        s0,
        s1,
        s2,
        s3,
        c0,
        c1,
        c2,
        c3,
        f0,
        f1,
        f2,
        f3,
        ..,
    ] = *octets;
    ([p0, p1, p2, p3, p4, p5, p6, p7] == *b"APETAGEX").then_some(Block {
        version: u32::from_le_bytes([v0, v1, v2, v3]),
        size: u32::from_le_bytes([s0, s1, s2, s3]),
        count: u32::from_le_bytes([c0, c1, c2, c3]),
        flags: u32::from_le_bytes([f0, f1, f2, f3]),
    })
}

/// Whether `header` is the header of the tag `footer` ends: flagged as the
/// header, with the footer's version, size and count.
const fn heads(header: &Block, footer: &Block) -> bool {
    header.flags & IS_HEADER != 0
        && header.version == footer.version
        && header.size == footer.size
        && header.count == footer.count
}

/// Reads the APE tag that ends the file `window` looks into, or that ends
/// just before its `ID3v1` tag, if there is one.
///
/// The window must hold the file's last 128 octets, or the whole file when
/// it is shorter, and the whole tag.
///
/// # Errors
///
/// [`ApeError::Fault`] when the window does not hold what the parse needs,
/// or the budget is spent before the footer is read; the other variants of
/// [`ApeError`] when the footer, or the header it declares, is not sound.
/// Problems with the items do not fail the parse; they are recorded in
/// [`ApeTag::problems`].
pub fn parse_ape(
    window: Window<'_>,
    limits: &Limits,
    budget: &mut Budget,
) -> Result<Option<ApeTag>, ApeError> {
    let v1 = id3v1::locate(&window).map_err(ApeError::Fault)?;
    if let Some((at, _)) = v1
        && let Some(tag) = parse_ending_at(window, at, limits, budget)?
    {
        return Ok(Some(tag));
    }
    parse_ending_at(window, window.file_len, limits, budget)
}

/// Reads the APE tag whose footer ends at `end`, charging the budget only
/// after the 32-octet preamble is present. `Ok(None)` means the window is
/// too short for a footer or the octets there are not `APETAGEX`.
fn parse_ending_at(
    window: Window<'_>,
    end: u64,
    limits: &Limits,
    budget: &mut Budget,
) -> Result<Option<ApeTag>, ApeError> {
    let Some(footer_at) = end.checked_sub(BLOCK) else {
        return Ok(None);
    };
    let octets = id3v1::array(&window, footer_at).map_err(ApeError::Fault)?;
    let Some(footer) = block(octets) else {
        return Ok(None);
    };
    budget.charge(1, footer_at).map_err(ApeError::Fault)?;
    if footer.flags & IS_HEADER != 0 {
        return Err(ApeError::NotAFooter { offset: footer_at });
    }
    if !matches!(footer.version, 1000 | 2000) {
        return Err(ApeError::UnknownVersion {
            offset: footer_at,
            version: footer.version,
        });
    }
    let size = u64::from(footer.size);
    if size < BLOCK {
        return Err(ApeError::TooSmall {
            offset: footer_at,
            size: footer.size,
        });
    }
    let header_len = if footer.flags & HAS_HEADER == 0 {
        0
    } else {
        BLOCK
    };
    let declared = size.saturating_add(header_len);
    let Some(start) = end.checked_sub(declared) else {
        return Err(ApeError::PastFileStart {
            offset: footer_at,
            size: declared,
            room: end,
        });
    };
    let tag = id3v1::octets(&window, start, declared).map_err(ApeError::Fault)?;
    let first_is_header = tag
        .first_chunk()
        .and_then(block)
        .is_some_and(|found| heads(&found, &footer));
    if header_len == BLOCK && !first_is_header {
        return Err(ApeError::HeaderMismatch { offset: start });
    }
    // The items lie between the header, if any, and the footer, inside
    // the tag's octets.
    let area = tag
        .get(index(header_len)..)
        .and_then(|rest| rest.get(..index(size.saturating_sub(BLOCK))))
        .unwrap_or_default();
    let cursor = Cursor::at(area, start.saturating_add(header_len));
    let (items, problems) = read_items(cursor, footer.count, limits, budget);
    Ok(Some(ApeTag {
        range: start..end,
        items,
        problems,
    }))
}

/// `value` as an index into a slice; past the end of any slice when it
/// does not fit.
fn index(value: u64) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

/// The offset `len` octets after `offset`.
fn past(offset: u64, len: usize) -> u64 {
    offset.saturating_add(u64::try_from(len).unwrap_or(u64::MAX))
}

/// Reads up to `count` items from `cursor`, which holds the tag's items
/// and nothing else.
fn read_items(
    mut cursor: Cursor<'_>,
    count: u32,
    limits: &Limits,
    budget: &mut Budget,
) -> (Vec<ApeItem>, Vec<ItemProblem>) {
    let fields = limits.get(LimitKind::TagFields);
    let mut items = bounded_vec(u64::from(count), MIN_ITEM, cursor.remaining(), fields);
    let mut problems = Vec::new();
    let mut counted = Fields { used: 0, limits };
    let mut seen = 0_u32;
    // Every item read moves the cursor on by at least nine octets. A
    // problem that is not a bad key, or a turn that does not move, ends
    // the loop, so a mutant that kept going after a truncation cannot hang.
    while seen < count {
        let offset = cursor.offset();
        let before = cursor.remaining();
        let read = budget
            .charge(1, offset)
            .map_err(ItemProblem::Stopped)
            .and_then(|()| next_item(&mut cursor));
        let mut stop = false;
        match read {
            Err(problem) => {
                problems.push(problem);
                stop = true;
            }
            Ok(raw) => {
                seen = seen.saturating_add(1);
                if valid_key(raw.key) {
                    match counted.value(&raw, offset) {
                        Ok(value) => items.push(ApeItem {
                            key: raw.key.iter().copied().map(char::from).collect(),
                            value,
                        }),
                        Err(fault) => {
                            problems.push(ItemProblem::Stopped(fault));
                            stop = true;
                        }
                    }
                } else {
                    problems.push(ItemProblem::BadKey { offset });
                }
            }
        }
        if stop || cursor.remaining() >= before {
            break;
        }
    }
    (items, problems)
}

/// One item as it lies in the tag, before its key and value are checked.
struct Raw<'a> {
    /// The item flags.
    flags: u32,
    /// The key's octets, without the terminator.
    key: &'a [u8],
    /// The value's octets.
    value: &'a [u8],
    /// Where the value starts in the file.
    value_at: u64,
}

/// Reads the item at `cursor` and moves past it.
fn next_item<'a>(cursor: &mut Cursor<'a>) -> Result<Raw<'a>, ItemProblem> {
    let [s0, s1, s2, s3, f0, f1, f2, f3] = cursor.array().map_err(ItemProblem::Stopped)?;
    let key_at = cursor.offset();
    let rest = cursor.rest();
    let Some(end) = rest.iter().position(|&octet| octet == 0) else {
        return Err(ItemProblem::KeyUnterminated { offset: key_at });
    };
    let key = rest.get(..end).unwrap_or_default();
    let after = rest.get(end.saturating_add(1)..).unwrap_or_default();
    let mut rest = Cursor::at(after, past(key_at, end).saturating_add(1));
    let value_at = rest.offset();
    let size = u32::from_le_bytes([s0, s1, s2, s3]);
    let value = rest.take(u64::from(size)).map_err(ItemProblem::Stopped)?;
    *cursor = rest;
    Ok(Raw {
        flags: u32::from_le_bytes([f0, f1, f2, f3]),
        key,
        value,
        value_at,
    })
}

/// Whether the specification allows `key`.
fn valid_key(key: &[u8]) -> bool {
    (2..=255).contains(&key.len())
        && key.iter().all(|octet| (0x20..=0x7E).contains(octet))
        && !FORBIDDEN
            .iter()
            .any(|forbidden| key.eq_ignore_ascii_case(forbidden))
}

/// The tag fields read so far, counted against the tag-field limit.
struct Fields<'a> {
    used: u64,
    limits: &'a Limits,
}

impl Fields<'_> {
    /// Counts one more field, for the item at `offset`.
    const fn count(&mut self, offset: u64) -> Result<(), ParseFault> {
        self.used = self.used.saturating_add(1);
        self.limits.check(LimitKind::TagFields, self.used, offset)
    }

    /// The value of `raw`, the item at `offset`, by its type.
    fn value(&mut self, raw: &Raw<'_>, offset: u64) -> Result<ApeValue, ParseFault> {
        let range = raw.value_at..past(raw.value_at, raw.value.len());
        match (raw.flags >> 1) & 3 {
            0 => self.texts(raw.value, offset).map(ApeValue::Text),
            1 => self.count(offset).map(|()| ApeValue::Binary(range)),
            2 => self.texts(raw.value, offset).map(ApeValue::Locator),
            _ => self.count(offset).map(|()| ApeValue::Reserved(range)),
        }
    }

    /// The values of a text or locator item at `offset`: split at zero
    /// octets, decoded as UTF-8 and capped at the long-text limit, each
    /// counted before it is decoded.
    fn texts(&mut self, value: &[u8], offset: u64) -> Result<Vec<Text>, ParseFault> {
        let cap = u32::try_from(self.limits.get(LimitKind::LongText)).unwrap_or(u32::MAX);
        value
            .split(|&octet| octet == 0)
            .map(|part| {
                self.count(offset)
                    .map(|()| text::decode(Untrusted::new(part), Encoding::Utf8, cap))
            })
            .collect()
    }
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use super::*;
    use gunmetal_testkit::ape::{Ape, BINARY, LOCATOR, READ_ONLY, RESERVED};
    use gunmetal_testkit::id3v1::Id3v1;
    use proptest::collection::vec;
    use proptest::prelude::*;
    use proptest::sample::select;

    /// Ten octets standing in for the audio before a tag.
    const AUDIO: [u8; 10] = [0xFF; 10];

    /// Runs `work` on a thread with the 256 KiB stack SEC-MED-001 names, so
    /// a parse that recursed would fail here.
    fn on_small_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(262_144)
            .spawn(work)
            .expect("the test thread starts")
            .join()
            .expect("the parse returned instead of panicking")
    }

    /// `AUDIO` followed by `tag`.
    fn file(tag: &[u8]) -> Vec<u8> {
        [&AUDIO[..], tag].concat()
    }

    fn len(bytes: &[u8]) -> u64 {
        u64::try_from(bytes.len()).unwrap()
    }

    /// A window over the whole of `bytes`.
    fn whole(bytes: &[u8]) -> Window<'_> {
        Window {
            offset: 0,
            bytes,
            file_len: len(bytes),
        }
    }

    /// The window over `bytes` from file offset `start` to its end.
    fn tail(bytes: &[u8], start: u64) -> Window<'_> {
        Window {
            offset: start,
            bytes: &bytes[usize::try_from(start).unwrap()..],
            file_len: len(bytes),
        }
    }

    /// A budget no test exhausts.
    fn ample() -> Budget {
        Budget::for_input(0, 0, u64::MAX)
    }

    /// Parses the whole of `bytes` under the default limits.
    fn parse(bytes: &[u8]) -> Result<Option<ApeTag>, ApeError> {
        parse_ape(whole(bytes), &Limits::DEFAULT, &mut ample())
    }

    fn text(value: &str) -> Text {
        Text {
            value: value.to_owned(),
            truncated: false,
            replaced: false,
        }
    }

    fn texts(values: &[&str]) -> ApeValue {
        ApeValue::Text(values.iter().map(|value| text(value)).collect())
    }

    fn item(key: &str, value: ApeValue) -> ApeItem {
        ApeItem {
            key: key.to_owned(),
            value,
        }
    }

    fn tag(range: Range<u64>, items: Vec<ApeItem>, problems: Vec<ItemProblem>) -> ApeTag {
        ApeTag {
            range,
            items,
            problems,
        }
    }

    /// Octets an item takes: its two integers, key, terminator and value.
    fn item_len(key: &[u8], value: &[u8]) -> u64 {
        8 + len(key) + 1 + len(value)
    }

    #[test]
    fn reads_an_apev2_tag_with_a_header_and_a_footer() {
        let bytes = file(
            &Ape::new()
                .text("Title", "Song")
                .text("Artist", "Band")
                .build(),
        );
        // 32 header, 18 and 19 for the items, 32 footer.
        assert_eq!(
            parse(&bytes),
            Ok(Some(tag(
                10..111,
                vec![
                    item("Title", texts(&["Song"])),
                    item("Artist", texts(&["Band"])),
                ],
                vec![],
            )))
        );
    }

    #[test]
    fn reads_tags_without_a_header_in_both_versions() {
        for version in [1000, 2000] {
            let bytes = file(
                &Ape::new()
                    .without_header()
                    .version(version)
                    .text("Title", "Song")
                    .build(),
            );
            assert_eq!(
                parse(&bytes),
                Ok(Some(tag(
                    10..60,
                    vec![item("Title", texts(&["Song"]))],
                    vec![]
                ))),
                "version {version}"
            );
        }
    }

    #[test]
    fn reads_an_empty_tag_that_is_the_whole_file() {
        let bytes = Ape::new().without_header().build();
        assert_eq!(parse(&bytes), Ok(Some(tag(0..32, vec![], vec![]))));
        let bytes = Ape::new().build();
        assert_eq!(parse(&bytes), Ok(Some(tag(0..64, vec![], vec![]))));
    }

    /// Verifies: SEC-MED-016
    #[test]
    fn keeps_binary_and_reserved_values_as_ranges_and_locators_as_text() {
        let bytes = file(
            &Ape::new()
                .without_header()
                .item(b"Cover Art (Front)", BINARY, b"a.jpg\0\xFF\xD8")
                .item(
                    b"Related",
                    LOCATOR,
                    b"http://example.invalid/x\0../../etc/passwd",
                )
                .item(b"Odd", RESERVED, b"xyz")
                .build(),
        );
        // Items at 10, 44 and 101; values at 36, 60 and 113; footer at 116.
        assert_eq!(
            parse(&bytes),
            Ok(Some(tag(
                10..148,
                vec![
                    item("Cover Art (Front)", ApeValue::Binary(36..44)),
                    item(
                        "Related",
                        ApeValue::Locator(vec![
                            text("http://example.invalid/x"),
                            text("../../etc/passwd"),
                        ]),
                    ),
                    item("Odd", ApeValue::Reserved(113..116)),
                ],
                vec![],
            )))
        );
    }

    #[test]
    fn splits_text_values_at_every_zero_octet() {
        let bytes = file(
            &Ape::new()
                .item(b"Artist", 0, b"One\0Two\0")
                .item(b"Genre", 0, b"")
                .item(b"Album", 0, b"\0")
                .build(),
        );
        let found = parse(&bytes).map(|found| found.map(|tag| tag.items));
        assert_eq!(
            found,
            Ok(Some(vec![
                item("Artist", texts(&["One", "Two", ""])),
                item("Genre", texts(&[""])),
                item("Album", texts(&["", ""])),
            ]))
        );
    }

    #[test]
    fn reads_the_type_bits_alone_and_ignores_the_others() {
        let bytes = file(
            &Ape::new()
                .item(b"Title", READ_ONLY | 0x0100_0008, b"Song")
                .item(b"Data", BINARY | READ_ONLY | 0x8000_0000, b"\x01")
                .build(),
        );
        // The header takes 10 to 42, "Title" 42 to 60 and "Data" 60 to 74.
        let found = parse(&bytes).map(|found| found.map(|tag| tag.items));
        assert_eq!(
            found,
            Ok(Some(vec![
                item("Title", texts(&["Song"])),
                item("Data", ApeValue::Binary(73..74)),
            ]))
        );
    }

    /// Verifies: SEC-MED-013
    #[test]
    fn decodes_utf8_replacing_invalid_sequences_and_removing_controls() {
        let bytes = file(
            &Ape::new()
                .item(b"Title", 0, b"Caf\xC3\xA9 \xFF\x1B[1m\tx\n")
                .item(b"Album", 0, b"\xEF\xBB\xBFMarked\0\r\xC2\x85")
                .build(),
        );
        let found = parse(&bytes).map(|found| found.map(|tag| tag.items));
        assert_eq!(
            found,
            Ok(Some(vec![
                item(
                    "Title",
                    ApeValue::Text(vec![Text {
                        value: "Café \u{FFFD}[1m\tx\n".to_owned(),
                        truncated: false,
                        replaced: true,
                    }]),
                ),
                item("Album", texts(&["Marked", ""])),
            ]))
        );
    }

    /// The cap counts decoded UTF-8, so "Café" needs five octets.
    ///
    /// Verifies: SEC-MED-006, SEC-MED-013
    #[test]
    fn caps_each_text_value_at_the_long_text_limit() {
        let limits = Limits::DEFAULT
            .with_override(LimitKind::LongText, 4)
            .unwrap();
        let bytes = file(
            &Ape::new()
                .item(b"Title", 0, "Café\0abcd\0abcde".as_bytes())
                .item(b"Link", LOCATOR, b"12345")
                .build(),
        );
        let cut = |value: &str| Text {
            value: value.to_owned(),
            truncated: true,
            replaced: false,
        };
        let found = parse_ape(whole(&bytes), &limits, &mut ample());
        assert_eq!(
            found.map(|found| found.map(|tag| tag.items)),
            Ok(Some(vec![
                item(
                    "Title",
                    ApeValue::Text(vec![cut("Caf"), text("abcd"), cut("abcd")])
                ),
                item("Link", ApeValue::Locator(vec![cut("1234")])),
            ]))
        );
    }

    #[test]
    fn reads_a_tag_that_comes_before_an_id3v1_tag() {
        let ape = Ape::new().text("Title", "Song").build();
        let bytes = [&file(&ape)[..], &Id3v1::new().title(b"Other").build()].concat();
        assert_eq!(
            parse(&bytes),
            Ok(Some(tag(
                10..92,
                vec![item("Title", texts(&["Song"]))],
                vec![]
            )))
        );
        // An ID3v1 tag with no APE tag before it.
        let bytes = file(&Id3v1::new().build());
        assert_eq!(parse(&bytes), Ok(None));
        // A footer that is not sound sits before ID3v1: the parse fails
        // there and does not look again at the file's end.
        let ape = Ape::new().without_header().version(3).build();
        let bytes = [&file(&ape)[..], &Id3v1::new().build()].concat();
        assert_eq!(
            parse(&bytes),
            Err(ApeError::UnknownVersion {
                offset: 10,
                version: 3
            })
        );
    }

    /// `APETAGEX` holds `TAG` at octets 3..6. A headed tag of 131 octets
    /// therefore makes the last 128 start with `TAG`, which is not an
    /// `ID3v1` tag: the parser still finds the APE tag that ends the file.
    ///
    /// Verifies: SEC-MED-001
    #[test]
    fn finds_a_headed_tag_whose_header_looks_like_id3v1() {
        let value = "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";
        let ape = Ape::new().text("Title", value).build();
        assert_eq!(ape.len(), 131);
        assert_eq!(&ape[3..6], b"TAG");
        assert_eq!(
            parse(&ape),
            Ok(Some(tag(
                0..131,
                vec![item("Title", texts(&[value]))],
                vec![]
            )))
        );
        let bytes = [&[0x55_u8; 40][..], &ape].concat();
        assert_eq!(&bytes[43..46], b"TAG");
        assert_eq!(
            parse(&bytes),
            Ok(Some(tag(
                40..171,
                vec![item("Title", texts(&[value]))],
                vec![]
            )))
        );
    }

    #[test]
    fn finds_nothing_without_a_footer() {
        let empty = Ape::new().without_header().build();
        for (at, octet) in [(0, b'X'), (6, b'e'), (7, b'Y')] {
            let mut bytes = file(&empty);
            bytes[10 + at] = octet;
            assert_eq!(parse(&bytes), Ok(None), "octet {at}");
        }
        assert_eq!(parse(&[]), Ok(None));
        assert_eq!(parse(&empty[1..]), Ok(None));
        assert_eq!(parse(&[0x41; 300]), Ok(None));
    }

    /// A header-only tag at the end of a file cannot be found by its
    /// footer: with no items its header is the last block and is refused;
    /// with items its last octets are an item's.
    #[test]
    fn refuses_a_header_where_the_footer_belongs() {
        let bytes = file(&Ape::new().header());
        assert_eq!(parse(&bytes), Err(ApeError::NotAFooter { offset: 10 }));
        let ape = Ape::new().text("Title", "Song");
        let bytes = file(&[&ape.header()[..], b"\x04\0\0\0\0\0\0\0Title\0Song"].concat());
        assert_eq!(parse(&bytes), Ok(None));
    }

    #[test]
    fn refuses_a_version_other_than_1000_and_2000() {
        for version in [0, 999, 1001, 1999, 2001, 3000, u32::MAX] {
            let bytes = file(&Ape::new().without_header().version(version).build());
            assert_eq!(
                parse(&bytes),
                Err(ApeError::UnknownVersion {
                    offset: 10,
                    version
                }),
                "version {version}"
            );
        }
    }

    /// Verifies: SEC-MED-008
    #[test]
    fn refuses_a_footer_that_declares_less_than_itself() {
        for size in [0, 1, 31] {
            let bytes = file(&Ape::new().without_header().declared_size(size).build());
            assert_eq!(
                parse(&bytes),
                Err(ApeError::TooSmall { offset: 10, size }),
                "size {size}"
            );
        }
        let bytes = file(&Ape::new().without_header().declared_size(32).build());
        assert_eq!(parse(&bytes), Ok(Some(tag(10..42, vec![], vec![]))));
    }

    /// Verifies: SEC-TM-032, SEC-MED-004
    #[test]
    fn refuses_a_tag_that_would_start_before_the_file() {
        // Without a header: 42 octets end the tag, and 42 may be declared.
        let sized = |size| file(&Ape::new().without_header().declared_size(size).build());
        assert_eq!(
            parse(&sized(43)),
            Err(ApeError::PastFileStart {
                offset: 10,
                size: 43,
                room: 42,
            })
        );
        assert_eq!(
            parse(&sized(u32::MAX)),
            Err(ApeError::PastFileStart {
                offset: 10,
                size: 4_294_967_295,
                room: 42,
            })
        );
        assert_eq!(parse(&sized(42)), Ok(Some(tag(0..42, vec![], vec![]))));
        // With a header, its 32 octets count too.
        let headed = |size| file(&Ape::new().declared_size(size).build());
        assert_eq!(
            parse(&headed(43)),
            Err(ApeError::PastFileStart {
                offset: 42,
                size: 75,
                room: 74,
            })
        );
        assert_eq!(
            parse(&headed(u32::MAX)),
            Err(ApeError::PastFileStart {
                offset: 42,
                size: 4_294_967_327,
                room: 74,
            })
        );
        // Exactly the whole file: the ten audio octets are not a header.
        assert_eq!(
            parse(&headed(42)),
            Err(ApeError::HeaderMismatch { offset: 0 })
        );
    }

    #[test]
    fn refuses_a_header_that_does_not_match_its_footer() {
        let good = file(&Ape::new().text("Title", "Song").build());
        // The header runs from 10 to 42: preamble, version, size, count,
        // flags.
        for (at, octet) in [
            (10, b'X'),
            (17, b'x'),
            (18, 0xE8),
            (22, 0x37),
            (26, 2),
            (33, 0x80),
        ] {
            let mut bytes = good.clone();
            bytes[at] = octet;
            assert_eq!(
                parse(&bytes),
                Err(ApeError::HeaderMismatch { offset: 10 }),
                "octet {at}"
            );
        }
        // The header's other flags and its reserved octets are not checked.
        let mut bytes = good;
        bytes[33] = 0xE0;
        bytes[34] = 0xFF;
        bytes[41] = 0xFF;
        assert_eq!(
            parse(&bytes).map(|found| found.map(|tag| tag.range)),
            Ok(Some(10..92))
        );
    }

    /// A count of `u32::MAX` in a tag of one item reserves room for the one
    /// item the octets can hold, and reading stops where the items end.
    ///
    /// Verifies: SEC-MED-003, SEC-MED-008, SEC-TM-032
    #[test]
    fn reads_the_items_there_are_when_the_count_cannot_fit() {
        let bytes = file(
            &Ape::new()
                .without_header()
                .text("Title", "Song")
                .declared_count(u32::MAX)
                .build(),
        );
        let found = parse(&bytes);
        assert_eq!(
            found,
            Ok(Some(tag(
                10..60,
                vec![item("Title", texts(&["Song"]))],
                vec![ItemProblem::Stopped(ParseFault::Truncated {
                    offset: 28,
                    needed: 8,
                    available: 0,
                })],
            )))
        );
        assert_eq!(
            found.map(|found| found.map(|tag| tag.items.capacity())),
            Ok(Some(1))
        );
    }

    /// Two items of eleven octets each: the smallest a key allows, so the
    /// reserved capacity is two, and one more octet of minimum size would
    /// not fit.
    ///
    /// Verifies: SEC-MED-003
    #[test]
    fn reserves_what_the_smallest_item_can_fit() {
        let found = parse(
            &Ape::new()
                .without_header()
                .item(b"Ab", BINARY, b"")
                .item(b"Cd", BINARY, b"")
                .declared_count(u32::MAX)
                .build(),
        );
        assert_eq!(
            found.map(|found| found.map(|tag| tag.items.capacity())),
            Ok(Some(2))
        );
        let found = parse(
            &Ape::new()
                .without_header()
                .item(b"Ab", BINARY, b"")
                .raw(&[0; 10])
                .declared_count(u32::MAX)
                .build(),
        );
        assert_eq!(
            found.map(|found| found.map(|tag| tag.items.capacity())),
            Ok(Some(1))
        );
    }

    /// Verifies: SEC-MED-003
    #[test]
    fn reserves_no_more_items_than_the_tag_field_limit() {
        let limits = Limits::DEFAULT
            .with_override(LimitKind::TagFields, 2)
            .unwrap();
        let mut ape = Ape::new().without_header().declared_count(u32::MAX);
        for _ in 0..5 {
            ape = ape.item(b"Ab", BINARY, b"");
        }
        let found = parse_ape(whole(&ape.build()), &limits, &mut ample());
        assert_eq!(
            found.map(|found| found.map(|tag| (tag.items.len(), tag.items.capacity()))),
            Ok(Some((2, 2)))
        );
    }

    #[test]
    fn stops_at_an_item_that_runs_past_the_tag() {
        let base = || Ape::new().without_header().text("Artist", "Band");
        // The second item starts at 29; its key at 37 and its value at 43.
        let long_value = file(&base().raw(b"\x64\0\0\0\0\0\0\0Title\0Song").build());
        assert_eq!(
            parse(&long_value),
            Ok(Some(tag(
                10..79,
                vec![item("Artist", texts(&["Band"]))],
                vec![ItemProblem::Stopped(ParseFault::Truncated {
                    offset: 43,
                    needed: 100,
                    available: 4,
                })],
            )))
        );
        let cut_header = file(&base().raw(b"\x01\0\0").build());
        assert_eq!(
            parse(&cut_header).map(|found| found.map(|tag| tag.problems)),
            Ok(Some(vec![ItemProblem::Stopped(ParseFault::Truncated {
                offset: 29,
                needed: 8,
                available: 3,
            })]))
        );
    }

    #[test]
    fn stops_at_a_key_without_a_terminator() {
        let bytes = file(
            &Ape::new()
                .without_header()
                .text("Artist", "Band")
                .raw(b"\0\0\0\0\0\0\0\0Endless")
                .build(),
        );
        assert_eq!(
            parse(&bytes),
            Ok(Some(tag(
                10..76,
                vec![item("Artist", texts(&["Band"]))],
                vec![ItemProblem::KeyUnterminated { offset: 37 }],
            )))
        );
    }

    /// Each bad key is skipped where it stands and the next item is read.
    #[test]
    fn skips_each_item_whose_key_the_specification_forbids() {
        let long = [b'a'; 256];
        let bad: [&[u8]; 11] = [
            b"",
            b"A",
            &long,
            b"Ti\x1Ftle",
            b"Ti\x7Ftle",
            b"Caf\xC3\xA9",
            b"ID3",
            b"tag",
            b"OGGS",
            b"Mp+",
            b"oggs",
        ];
        let mut ape = Ape::new().without_header();
        let mut problems = Vec::new();
        let mut offset = 10;
        for key in bad {
            ape = ape.item(key, 0, b"v").text("Ok", "w");
            problems.push(ItemProblem::BadKey { offset });
            offset += item_len(key, b"v") + item_len(b"Ok", b"w");
        }
        let bytes = file(&ape.build());
        let found = parse(&bytes)
            .map(|found| found.map(|tag| (tag.items.len(), tag.items[0].clone(), tag.problems)));
        assert_eq!(found, Ok(Some((11, item("Ok", texts(&["w"])), problems))));
    }

    #[test]
    fn reads_keys_at_each_edge_of_what_is_allowed() {
        let longest = ["k"; 255].concat();
        let keys = ["Ab", longest.as_str(), " ~", "ID3x", "TAGS", "MP+3", "Ogg"];
        let mut ape = Ape::new();
        for key in keys {
            ape = ape.text(key, "v");
        }
        let found = parse(&file(&ape.build())).map(|found| found.map(|tag| tag.items));
        assert_eq!(
            found,
            Ok(Some(
                keys.iter().map(|key| item(key, texts(&["v"]))).collect()
            ))
        );
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn counts_every_value_against_the_tag_field_limit() {
        let limits = Limits::DEFAULT
            .with_override(LimitKind::TagFields, 3)
            .unwrap();
        let parse_under = |ape: Ape| parse_ape(whole(&file(&ape.build())), &limits, &mut ample());
        let three = || {
            Ape::new()
                .without_header()
                .item(b"Artist", 0, b"a\0b")
                .item(b"Cover", BINARY, b"")
        };
        assert_eq!(
            parse_under(three()).map(|found| found.map(|tag| (tag.items.len(), tag.problems))),
            Ok(Some((2, vec![])))
        );
        // The fourth field is the value of the item at 10 + 18 + 14.
        let found = parse_under(three().text("Title", "x"));
        assert_eq!(
            found.map(|found| found.map(|tag| (tag.items.len(), tag.problems))),
            Ok(Some((
                2,
                vec![ItemProblem::Stopped(ParseFault::LimitExceeded {
                    limit: LimitKind::TagFields,
                    value: 4,
                    max: 3,
                    offset: 42,
                })]
            )))
        );
        let found = parse_under(Ape::new().without_header().item(b"Genre", 0, b"a\0b\0c\0d"));
        assert_eq!(
            found.map(|found| found.map(|tag| (tag.items.len(), tag.problems))),
            Ok(Some((
                0,
                vec![ItemProblem::Stopped(ParseFault::LimitExceeded {
                    limit: LimitKind::TagFields,
                    value: 4,
                    max: 3,
                    offset: 10,
                })]
            )))
        );
        let found = parse_under(three().item(b"More", RESERVED, b""));
        assert_eq!(
            found.map(|found| found.map(|tag| tag.problems)),
            Ok(Some(vec![ItemProblem::Stopped(
                ParseFault::LimitExceeded {
                    limit: LimitKind::TagFields,
                    value: 4,
                    max: 3,
                    offset: 42,
                }
            )]))
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn reads_4096_fields_by_default_and_refuses_the_4097th() {
        let mut ape = Ape::new().without_header();
        for _ in 0..4_096 {
            ape = ape.item(b"Ab", BINARY, b"");
        }
        let at_limit = parse(&ape.build());
        assert_eq!(
            at_limit.map(|found| found.map(|tag| (tag.items.len(), tag.problems))),
            Ok(Some((4_096, vec![])))
        );
        let over = parse(&ape.item(b"Ab", LOCATOR, b"").build());
        assert_eq!(
            over.map(|found| found.map(|tag| (tag.items.len(), tag.problems))),
            Ok(Some((
                4_096,
                vec![ItemProblem::Stopped(ParseFault::LimitExceeded {
                    limit: LimitKind::TagFields,
                    value: 4_097,
                    max: 4_096,
                    offset: 45_056,
                })]
            )))
        );
    }

    /// Verifies: SEC-MED-007, SEC-TM-032
    #[test]
    fn spends_one_step_for_the_footer_and_one_before_each_item() {
        // Items at 42 and 54, footer at 66.
        let bytes = file(&Ape::new().text("A1", "x").text("B2", "y").build());
        let run = |steps| {
            let mut budget = Budget::for_input(0, 0, steps);
            let found = parse_ape(whole(&bytes), &Limits::DEFAULT, &mut budget);
            (found, budget.remaining())
        };
        let both = vec![item("A1", texts(&["x"])), item("B2", texts(&["y"]))];
        let stopped = |offset| vec![ItemProblem::Stopped(ParseFault::BudgetExceeded { offset })];
        assert_eq!(
            run(0),
            (
                Err(ApeError::Fault(ParseFault::BudgetExceeded { offset: 66 })),
                0
            )
        );
        assert_eq!(run(1), (Ok(Some(tag(10..98, vec![], stopped(42)))), 0));
        assert_eq!(
            run(2),
            (Ok(Some(tag(10..98, both[..1].to_vec(), stopped(54)))), 0)
        );
        assert_eq!(run(3), (Ok(Some(tag(10..98, both.clone(), vec![]))), 0));
        assert_eq!(run(5), (Ok(Some(tag(10..98, both, vec![]))), 2));
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn spends_nothing_without_a_footer() {
        let mut budget = Budget::for_input(0, 0, 4);
        assert_eq!(
            parse_ape(whole(&AUDIO), &Limits::DEFAULT, &mut budget),
            Ok(None)
        );
        assert_eq!(budget.remaining(), 4);
    }

    /// A tag inside a binary value is a value; nothing descends into it.
    ///
    /// Verifies: SEC-MED-005
    #[test]
    fn never_reads_a_tag_held_inside_a_value() {
        let inner = Ape::new().text("Title", "Inner").build();
        let mut ape = Ape::new();
        for _ in 0..40 {
            ape = Ape::new().item(b"Nested", BINARY, &ape.item(b"Inner", 0, &inner).build());
        }
        let bytes = file(&ape.build());
        let found =
            on_small_stack(move || parse(&bytes).map(|found| found.map(|tag| tag.items.len())));
        assert_eq!(found, Ok(Some(1)));
    }

    #[test]
    fn reports_absolute_offsets_from_a_tail() {
        let ape = Ape::new()
            .without_header()
            .item(b"Data", BINARY, b"\x01\x02")
            .build();
        let bytes = [&[0x55; 200][..], &ape].concat();
        assert_eq!(
            parse_ape(tail(&bytes, 72), &Limits::DEFAULT, &mut ample()),
            Ok(Some(tag(
                200..247,
                vec![item("Data", ApeValue::Binary(213..215))],
                vec![]
            )))
        );
    }

    /// Every tail that starts inside the tag is reported at the tag's
    /// first octet with the octets it holds, and every tail too short to
    /// show whether an `ID3v1` tag ends the file is reported there.
    ///
    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn reports_every_tail_that_cuts_what_the_parse_needs() {
        // The tag runs from 100 to 378, its footer from 346.
        let ape = Ape::new().item(b"Title", 0, &[b'a'; 200]).build();
        let bytes = [&[0x55; 100][..], &ape].concat();
        for start in 101..=250 {
            assert_eq!(
                parse_ape(tail(&bytes, start), &Limits::DEFAULT, &mut ample()),
                Err(ApeError::Fault(ParseFault::Truncated {
                    offset: 100,
                    needed: 278,
                    available: 378 - start,
                })),
                "tail from {start}"
            );
        }
        for start in 251..=378 {
            assert_eq!(
                parse_ape(tail(&bytes, start), &Limits::DEFAULT, &mut ample()),
                Err(ApeError::Fault(ParseFault::Truncated {
                    offset: 250,
                    needed: 128,
                    available: 378 - start,
                })),
                "tail from {start}"
            );
        }
        let early = Window {
            offset: 0,
            bytes: &bytes[..300],
            file_len: 378,
        };
        assert_eq!(
            parse_ape(early, &Limits::DEFAULT, &mut ample()),
            Err(ApeError::Fault(ParseFault::Truncated {
                offset: 250,
                needed: 128,
                available: 50,
            }))
        );
    }

    /// Verifies: SEC-MED-001
    #[test]
    fn reports_every_tail_that_cuts_the_footer_of_a_short_file() {
        let bytes = file(&Ape::new().without_header().build());
        for start in 11..=42 {
            assert_eq!(
                parse_ape(tail(&bytes, start), &Limits::DEFAULT, &mut ample()),
                Err(ApeError::Fault(ParseFault::Truncated {
                    offset: 10,
                    needed: 32,
                    available: 42 - start,
                })),
                "tail from {start}"
            );
        }
    }

    #[test]
    fn says_where_each_error_and_problem_happened() {
        let errors = [
            ApeError::Fault(ParseFault::BudgetExceeded { offset: 1 }),
            ApeError::NotAFooter { offset: 2 },
            ApeError::UnknownVersion {
                offset: 3,
                version: 9,
            },
            ApeError::TooSmall { offset: 4, size: 9 },
            ApeError::PastFileStart {
                offset: 5,
                size: 9,
                room: 8,
            },
            ApeError::HeaderMismatch { offset: 6 },
        ];
        assert_eq!(errors.map(|error| error.offset()), [1, 2, 3, 4, 5, 6]);
        let problems = [
            ItemProblem::Stopped(ParseFault::BudgetExceeded { offset: 7 }),
            ItemProblem::KeyUnterminated { offset: 8 },
            ItemProblem::BadKey { offset: 9 },
        ];
        assert_eq!(problems.map(|problem| problem.offset()), [7, 8, 9]);
    }

    #[test]
    fn describes_a_skipped_tag_and_a_skipped_item_at_their_offsets() {
        assert_eq!(
            ApeError::TooSmall {
                offset: 77,
                size: 3,
            }
            .problem(),
            Problem {
                code: ProblemCode::EndTagSkipped,
                args: vec![("tag", Arg::Name("APE")), ("offset", Arg::Number(77))],
            }
        );
        assert_eq!(
            ItemProblem::BadKey { offset: 88 }.problem(),
            Problem {
                code: ProblemCode::ApeItemSkipped,
                args: vec![("offset", Arg::Number(88))],
            }
        );
    }

    /// One item as the property tests generate it: a valid key, a type, and
    /// values with no zero octet in them.
    #[derive(Debug, Clone)]
    struct Spec {
        key: String,
        kind: u32,
        values: Vec<String>,
    }

    fn any_key() -> impl Strategy<Value = String> {
        "[ -SU-~]{2,40}".prop_filter("not a forbidden key", |key| {
            !["ID3", "TAG", "OGGS", "MP+"].contains(&key.to_ascii_uppercase().as_str())
        })
    }

    fn any_spec() -> impl Strategy<Value = Spec> {
        (
            any_key(),
            select(vec![0_u32, BINARY, LOCATOR, RESERVED]),
            vec("[ -SU-~é]{0,12}", 1..4),
        )
            .prop_map(|(key, kind, values)| Spec { key, kind, values })
    }

    /// The tag a list of items should read as, worked out from the
    /// specification's layout rather than from the parser: each item's
    /// value starts 9 octets plus its key's length after the item.
    fn expected(start: u64, header: bool, specs: &[Spec]) -> (Vec<u8>, ApeTag) {
        let mut ape = Ape::new();
        if !header {
            ape = ape.without_header();
        }
        let mut at = start + if header { 32 } else { 0 };
        let mut items = Vec::new();
        for spec in specs {
            let value = spec.values.join("\0");
            ape = ape.item(spec.key.as_bytes(), spec.kind, value.as_bytes());
            let value_at = at + 9 + len(spec.key.as_bytes());
            let range = value_at..value_at + len(value.as_bytes());
            let next = range.end;
            let decoded = || spec.values.iter().map(|value| text(value)).collect();
            items.push(item(
                &spec.key,
                match spec.kind {
                    0 => ApeValue::Text(decoded()),
                    BINARY => ApeValue::Binary(range),
                    LOCATOR => ApeValue::Locator(decoded()),
                    _ => ApeValue::Reserved(range),
                },
            ));
            at = next;
        }
        let tag = ApeTag {
            range: start..at + 32,
            items,
            problems: vec![],
        };
        (ape.build(), tag)
    }

    /// Windows over hostile ends of files: random octets, footers with
    /// random fields, and real tags with random octets changed.
    fn any_window() -> impl Strategy<Value = (u64, Vec<u8>, u64)> {
        let footer = (
            prop_oneof![Just(1_000_u32), Just(2_000), any::<u32>()],
            prop_oneof![0_u32..400, any::<u32>()],
            prop_oneof![0_u32..8, any::<u32>()],
            prop_oneof![Just(0_u32), Just(1 << 31), any::<u32>()],
        )
            .prop_map(|(version, size, count, flags)| {
                let mut block = b"APETAGEX".to_vec();
                for value in [version, size, count, flags] {
                    block.extend(value.to_le_bytes());
                }
                block.extend([0; 8]);
                block
            });
        let tag = (
            vec(any_spec(), 0..4),
            any::<bool>(),
            vec((any::<usize>(), any::<u8>()), 0..4),
        )
            .prop_map(|(specs, header, changes)| {
                let mut bytes = expected(0, header, &specs).0;
                let width = bytes.len();
                for (at, octet) in changes {
                    bytes[at % width] = octet;
                }
                bytes
            });
        let end = prop_oneof![footer, tag, vec(any::<u8>(), 0..64)];
        (
            vec(any::<u8>(), 0..64),
            end,
            any::<bool>(),
            0_u64..48,
            0_u64..3,
        )
            .prop_map(|(before, end, v1, offset, shape)| {
                let v1 = if v1 {
                    Id3v1::new().title(b"x").build()
                } else {
                    vec![]
                };
                let bytes = [before, end, v1].concat();
                let len = len(&bytes);
                let file_len = match shape {
                    0 => offset + len,
                    1 => offset + len + 1,
                    _ => offset + len / 2,
                };
                (offset, bytes, file_len)
            })
    }

    /// Asserts what holds for every parse, whatever the input.
    fn assert_sound(window: Window<'_>, found: &Result<Option<ApeTag>, ApeError>, spent: u64) {
        let span = len(window.bytes);
        assert!(spent <= 2 + span / 9, "{spent} steps for {span} octets");
        match found {
            Ok(Some(tag)) => {
                assert!(tag.range.start < tag.range.end && tag.range.end <= window.file_len);
                for found in &tag.items {
                    if let ApeValue::Binary(range) | ApeValue::Reserved(range) = &found.value {
                        assert!(tag.range.start <= range.start && range.end <= tag.range.end);
                    }
                    assert!((2..=255).contains(&found.key.len()));
                }
                for problem in &tag.problems {
                    assert!(tag.range.contains(&problem.offset()), "{problem:?}");
                }
            }
            Ok(None) => {}
            Err(error) => assert!(error.offset() < window.file_len.max(1), "{error:?}"),
        }
    }

    proptest! {
        /// Verifies: SEC-MED-001, SEC-MED-007, SEC-MED-008
        #[test]
        fn returns_within_its_step_bound_for_any_window(
            (offset, bytes, file_len) in any_window(),
        ) {
            let owned = bytes.clone();
            let (found, left) = on_small_stack(move || {
                let window = Window { offset, bytes: &owned, file_len };
                let mut budget = ample();
                let found = parse_ape(window, &Limits::DEFAULT, &mut budget);
                (found, budget.remaining())
            });
            assert_sound(Window { offset, bytes: &bytes, file_len }, &found, u64::MAX - left);
        }

        /// Verifies: SEC-MED-001
        #[test]
        fn reads_back_whatever_the_builder_writes(
            specs in vec(any_spec(), 0..6),
            header in any::<bool>(),
            v1 in any::<bool>(),
            before in 0_usize..40,
        ) {
            let (ape, wanted) = expected(u64::try_from(before).unwrap(), header, &specs);
            let v1 = if v1 { Id3v1::new().build() } else { vec![] };
            let bytes = [&[0x55_u8; 40][..before], &ape, &v1].concat();
            prop_assert_eq!(parse(&bytes), Ok(Some(wanted)));
        }
    }
}
