//! The structure-aware harness for the ID3 family (SEC-MED-031). It covers
//! the tags at the end of a file so far: an APE tag and the `ID3v1` tag
//! after it.
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

use std::ops::Range;

use gunmetal_core::formats::ape::{self, ApeError, ApeTag, ApeValue, ItemProblem};
use gunmetal_core::formats::id3v1::{self, Id3v1Error, Id3v1Tag};
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
