//! Replays the committed recipes of the ID3 family's structure-aware
//! harness on stable Rust, so that `cargo test` and the gate run every
//! seed and every reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/id3_structure` is a recipe, not a file. Each
//! has a test here that pins its exact bytes, the exact file the harness
//! builds from it and what the parsers find in that file. Commit a fuzzing
//! reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::ops::Range;
use std::path::PathBuf;

use gunmetal_core::formats::ape::{ApeItem, ApeTag, ApeValue, ItemProblem};
use gunmetal_core::formats::id3v1::Id3v1Tag;
use gunmetal_core::text::Text;
use gunmetal_fuzz::id3_structure::{Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/id3_structure")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 6] = [
    "empty",
    "forbidden-key-in-version-1000",
    "header-two-items-and-id3v1",
    "one-item-of-each-type",
    "value-that-poses-as-id3v1",
    "zero-in-a-key",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

/// A header or footer block, as the specification lays it out.
fn block(version: u32, size: u32, count: u32, flags: u32) -> Vec<u8> {
    let mut block = b"APETAGEX".to_vec();
    for value in [version, size, count, flags] {
        block.extend(value.to_le_bytes());
    }
    block.extend([0; 8]);
    block
}

/// An item, as the specification lays it out.
fn raw_item(key: &[u8], flags: u32, value: &[u8]) -> Vec<u8> {
    let len = u32::try_from(value.len()).expect("a short value");
    [
        &len.to_le_bytes()[..],
        &flags.to_le_bytes(),
        key,
        &[0],
        value,
    ]
    .concat()
}

fn text(value: &str) -> Text {
    Text {
        value: value.to_owned(),
        truncated: false,
        replaced: false,
    }
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

/// Verifies: SEC-MED-028, SEC-MED-030
#[test]
fn the_corpus_holds_exactly_the_seeds_tested_here() {
    let mut names: Vec<String> = fs::read_dir(seeds_dir())
        .expect("seed directory is readable")
        .map(|entry| {
            entry
                .expect("seed directory entry is readable")
                .file_name()
                .into_string()
                .expect("seed names are UTF-8")
        })
        .collect();
    names.sort();
    assert_eq!(names, SEEDS);
}

/// An empty recipe asks for nothing: an `APEv2` footer alone.
///
/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_the_empty_recipe() {
    replay(
        "empty",
        &[],
        &Outcome {
            file: block(2_000, 32, 0, 0),
            ape_range: 0..32,
            ape: Ok(Some(tag(0..32, vec![], vec![]))),
            v1: Ok(None),
            checked: true,
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_headed_tag_of_two_items_before_an_id3v1_tag() {
    let recipe = [
        &[5, 3, 0xAA, 0xBB, 0xCC, 2, 0, 5][..],
        b"Title\x04Song",
        &[2, 4],
        b"Data\x02\x01\x02",
        b"Other",
    ]
    .concat();
    let mut v1 = b"TAGOther".to_vec();
    v1.resize(128, 0);
    let file = [
        &[0xAA, 0xBB, 0xCC][..],
        &block(2_000, 65, 2, 0xA000_0000),
        &raw_item(b"Title", 0, b"Song"),
        &raw_item(b"Data", 2, &[1, 2]),
        &block(2_000, 65, 2, 0x8000_0000),
        &v1,
    ]
    .concat();
    replay(
        "header-two-items-and-id3v1",
        &recipe,
        &Outcome {
            file,
            ape_range: 3..100,
            ape: Ok(Some(tag(
                3..100,
                vec![
                    item("Title", ApeValue::Text(vec![text("Song")])),
                    item("Data", ApeValue::Binary(66..68)),
                ],
                vec![],
            ))),
            v1: Ok(Some(Id3v1Tag {
                range: 100..228,
                title: text("Other"),
                artist: text(""),
                album: text(""),
                year: text(""),
                comment: text(""),
                track: None,
                genre: 0,
            })),
            checked: true,
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_forbidden_key_in_an_apev1_tag() {
    let recipe = [&[2, 0, 2, 0, 3][..], b"TAG\x01x", &[1, 2], b"Ok\x03a\0b"].concat();
    let file = [
        &raw_item(b"TAG", 0, b"x")[..],
        &raw_item(b"Ok", 1, b"a\0b"),
        &block(1_000, 59, 2, 0),
    ]
    .concat();
    replay(
        "forbidden-key-in-version-1000",
        &recipe,
        &Outcome {
            file,
            ape_range: 0..59,
            ape: Ok(Some(tag(
                0..59,
                vec![item("Ok", ApeValue::Text(vec![text("a"), text("b")]))],
                vec![ItemProblem::BadKey { offset: 0 }],
            ))),
            v1: Ok(None),
            checked: true,
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_one_item_of_each_type_after_a_header() {
    let recipe = [
        &[1, 0, 4, 0, 2][..],
        b"T1\x01a",
        &[2, 2],
        b"B1\x02\x01\x02",
        &[4, 2],
        b"L1\x03x\0y",
        &[6, 2],
        b"R1\x01z",
    ]
    .concat();
    let file = [
        &block(2_000, 83, 4, 0xA000_0000)[..],
        &raw_item(b"T1", 0, b"a"),
        &raw_item(b"B1", 2, &[1, 2]),
        &raw_item(b"L1", 4, b"x\0y"),
        &raw_item(b"R1", 6, b"z"),
        &block(2_000, 83, 4, 0x8000_0000),
    ]
    .concat();
    replay(
        "one-item-of-each-type",
        &recipe,
        &Outcome {
            file,
            ape_range: 0..115,
            ape: Ok(Some(tag(
                0..115,
                vec![
                    item("T1", ApeValue::Text(vec![text("a")])),
                    item("B1", ApeValue::Binary(55..57)),
                    item("L1", ApeValue::Locator(vec![text("x"), text("y")])),
                    item("R1", ApeValue::Reserved(82..83)),
                ],
                vec![],
            ))),
            v1: Ok(None),
            checked: true,
        },
    );
}

/// Four binary items whose second value puts `TAG` 128 octets from the end
/// of the file. `find_v1` reports that slot; `parse_ape` still finds the
/// tag that ends the file, because no footer sits immediately before it.
/// The harness does not check the `ID3v1` fields: none were built.
///
/// Verifies: SEC-MED-001, SEC-MED-028
#[test]
fn replays_a_value_that_poses_as_an_id3v1_tag() {
    let keys: [&[u8]; 4] = [b"Ab", b"Cd", b"Ef", b"Gh"];
    let values: [&[u8]; 4] = [
        b"0123456789abcdefghijklmnopqrstu",
        b"ABCDEFGHIJKLMNOPQRSTAGabcdefghi",
        b"0123456789klmnopqrstuvwxyzKLMNO",
        b"0123456789012345678901234567xyz",
    ];
    let mut recipe = vec![0, 0, 4];
    let mut file = Vec::new();
    for (key, value) in keys.into_iter().zip(values) {
        recipe.extend([&[2, 2][..], key, &[31], value].concat());
        file.extend(raw_item(key, 2, value));
    }
    file.extend(block(2_000, 200, 4, 0));
    replay(
        "value-that-poses-as-id3v1",
        &recipe,
        &Outcome {
            file,
            ape_range: 0..200,
            ape: Ok(Some(tag(
                0..200,
                vec![
                    item("Ab", ApeValue::Binary(11..42)),
                    item("Cd", ApeValue::Binary(53..84)),
                    item("Ef", ApeValue::Binary(95..126)),
                    item("Gh", ApeValue::Binary(137..168)),
                ],
                vec![],
            ))),
            v1: Ok(Some(Id3v1Tag {
                range: 72..200,
                // The rest of the second value, up to the first zero of
                // the third item's length.
                title: text("abcdefghi"),
                artist: text("klmnopqrstuvwxyzKLMNO"),
                album: text("h"),
                year: text("xyzA"),
                // The footer from its second octet, to the first zero of
                // the version 2000 (0xD0, 0x07, 0x00, 0x00).
                comment: text("PETAGEX\u{D0}"),
                track: None,
                genre: 0,
            })),
            checked: false,
        },
    );
}

/// A zero in a key would end it early and break the framing, so the
/// recipe's zero becomes 0x01, which the key may not hold either.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_zero_asked_for_in_a_key() {
    let file = [&raw_item(b"A\x01B", 0, b"")[..], &block(2_000, 44, 1, 0)].concat();
    replay(
        "zero-in-a-key",
        &[0, 0, 1, 0, 3, b'A', 0, b'B', 0],
        &Outcome {
            file,
            ape_range: 0..44,
            ape: Ok(Some(tag(
                0..44,
                vec![],
                vec![ItemProblem::BadKey { offset: 0 }],
            ))),
            v1: Ok(None),
            checked: true,
        },
    );
}
