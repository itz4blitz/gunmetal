//! Replays the committed APE fuzz corpus through its harness on stable
//! Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/ape` has a test here that pins its exact bytes
//! and the exact outcome the harness reports for it. Commit a fuzzing
//! reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::ops::Range;
use std::path::PathBuf;

use gunmetal_core::formats::ape::{ApeError, ApeItem, ApeTag, ApeValue, ItemProblem};
use gunmetal_core::parse::ParseFault;
use gunmetal_core::text::Text;
use gunmetal_fuzz::ape::{Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/ape")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 11] = [
    "apev2-with-a-header",
    "before-an-id3v1-tag",
    "binary-locator-and-reserved",
    "count-that-cannot-fit",
    "empty",
    "forbidden-and-good-keys",
    "header-at-the-end",
    "size-past-the-file-start",
    "text-with-a-tab-and-a-line-feed",
    "two-forbidden-keys",
    "unterminated-key",
];

/// Ten octets standing in for the audio before a tag.
const AUDIO: [u8; 10] = [0xFF; 10];

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

/// The tail case's window does not reach back to the file's last 128
/// octets: it starts 1,000 octets into a file `len` octets longer.
fn short_tail(len: u64) -> Result<Option<ApeTag>, ApeError> {
    Err(ApeError::Fault(ParseFault::Truncated {
        offset: 872 + len,
        needed: 128,
        available: len,
    }))
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

/// Verifies: SEC-MED-028
#[test]
fn replays_the_empty_input() {
    replay(
        "empty",
        &[],
        &Outcome {
            whole: Ok(None),
            tail: short_tail(0),
            steps: [0, 0],
        },
    );
}

/// Verifies: SEC-MED-028, SEC-HIS-036
#[test]
fn replays_an_apev2_tag_with_a_header() {
    let bytes = [
        &AUDIO[..],
        &block(2_000, 50, 1, 0xA000_0000),
        &raw_item(b"Title", 0, b"Song"),
        &block(2_000, 50, 1, 0x8000_0000),
    ]
    .concat();
    replay(
        "apev2-with-a-header",
        &bytes,
        &Outcome {
            whole: Ok(Some(tag(
                10..92,
                vec![item("Title", ApeValue::Text(vec![text("Song")]))],
                vec![],
            ))),
            tail: short_tail(92),
            steps: [2, 0],
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_tag_before_an_id3v1_tag() {
    let mut v1 = b"TAGOther".to_vec();
    v1.resize(128, 0);
    let bytes = [
        &AUDIO[..],
        &raw_item(b"Title", 0, b"Song"),
        &block(2_000, 50, 1, 0),
        &v1,
    ]
    .concat();
    let title = || vec![item("Title", ApeValue::Text(vec![text("Song")]))];
    replay(
        "before-an-id3v1-tag",
        &bytes,
        &Outcome {
            whole: Ok(Some(tag(10..60, title(), vec![]))),
            tail: Ok(Some(tag(1_010..1_060, title(), vec![]))),
            steps: [2, 2],
        },
    );
}

/// Verifies: SEC-MED-028, SEC-TM-032
#[test]
fn replays_a_size_past_the_start_of_the_file() {
    replay(
        "size-past-the-file-start",
        &block(2_000, 0xFFFF_FFF0, 0, 0),
        &Outcome {
            whole: Err(ApeError::PastFileStart {
                offset: 0,
                size: 4_294_967_280,
                room: 32,
            }),
            tail: short_tail(32),
            steps: [1, 0],
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-003
#[test]
fn replays_a_count_that_cannot_fit() {
    let bytes = [
        &AUDIO[..],
        &raw_item(b"Title", 0, b"Song"),
        &block(2_000, 50, u32::MAX, 0),
    ]
    .concat();
    replay(
        "count-that-cannot-fit",
        &bytes,
        &Outcome {
            whole: Ok(Some(tag(
                10..60,
                vec![item("Title", ApeValue::Text(vec![text("Song")]))],
                vec![ItemProblem::Stopped(ParseFault::Truncated {
                    offset: 28,
                    needed: 8,
                    available: 0,
                })],
            ))),
            tail: short_tail(60),
            steps: [3, 0],
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_forbidden_key_before_a_good_one() {
    let bytes = [
        &AUDIO[..],
        &raw_item(b"TAG", 0, b"x"),
        &raw_item(b"Ok", 0, b"y"),
        &block(2_000, 57, 2, 0),
    ]
    .concat();
    replay(
        "forbidden-and-good-keys",
        &bytes,
        &Outcome {
            whole: Ok(Some(tag(
                10..67,
                vec![item("Ok", ApeValue::Text(vec![text("y")]))],
                vec![ItemProblem::BadKey { offset: 10 }],
            ))),
            tail: short_tail(67),
            steps: [3, 0],
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_two_forbidden_keys_before_a_good_one() {
    let bytes = [
        &AUDIO[..],
        &raw_item(b"TAG", 0, b"x"),
        &raw_item(b"ID3", 0, b"y"),
        &raw_item(b"Ok", 0, b"z"),
        &block(2_000, 70, 3, 0),
    ]
    .concat();
    replay(
        "two-forbidden-keys",
        &bytes,
        &Outcome {
            whole: Ok(Some(tag(
                10..80,
                vec![item("Ok", ApeValue::Text(vec![text("z")]))],
                vec![
                    ItemProblem::BadKey { offset: 10 },
                    ItemProblem::BadKey { offset: 23 },
                ],
            ))),
            tail: short_tail(80),
            steps: [4, 0],
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_header_where_the_footer_belongs() {
    let bytes = [&AUDIO[..], &block(2_000, 32, 0, 0xA000_0000)].concat();
    replay(
        "header-at-the-end",
        &bytes,
        &Outcome {
            whole: Err(ApeError::NotAFooter { offset: 10 }),
            tail: short_tail(42),
            steps: [1, 0],
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_key_without_a_terminator() {
    let bytes = [&AUDIO[..], &[0; 8], b"Endless", &block(2_000, 47, 1, 0)].concat();
    replay(
        "unterminated-key",
        &bytes,
        &Outcome {
            whole: Ok(Some(tag(
                10..57,
                vec![],
                vec![ItemProblem::KeyUnterminated { offset: 18 }],
            ))),
            tail: short_tail(57),
            steps: [2, 0],
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_binary_locator_and_reserved_values() {
    let bytes = [
        &AUDIO[..],
        &raw_item(b"Cover", 2, b"\x01\x02"),
        &raw_item(b"Link", 4, b"http://x.invalid/\0../y"),
        &raw_item(b"Odd", 6, b"z"),
        &block(2_000, 96, 3, 0),
    ]
    .concat();
    replay(
        "binary-locator-and-reserved",
        &bytes,
        &Outcome {
            whole: Ok(Some(tag(
                10..106,
                vec![
                    item("Cover", ApeValue::Binary(24..26)),
                    item(
                        "Link",
                        ApeValue::Locator(vec![text("http://x.invalid/"), text("../y")]),
                    ),
                    item("Odd", ApeValue::Reserved(73..74)),
                ],
                vec![],
            ))),
            tail: short_tail(106),
            steps: [4, 0],
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-013
#[test]
fn replays_text_with_a_tab_and_a_line_feed() {
    let bytes = [
        &AUDIO[..],
        &raw_item(b"Comment", 0, b"one\ttwo\nthree"),
        &block(2_000, 61, 1, 0),
    ]
    .concat();
    replay(
        "text-with-a-tab-and-a-line-feed",
        &bytes,
        &Outcome {
            whole: Ok(Some(tag(
                10..71,
                vec![item(
                    "Comment",
                    ApeValue::Text(vec![text("one\ttwo\nthree")]),
                )],
                vec![],
            ))),
            tail: short_tail(71),
            steps: [2, 0],
        },
    );
}
