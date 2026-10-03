//! Replays the committed log-segment fuzz corpus through its harness on
//! stable Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/logframe` has a test here that pins its exact
//! bytes and the exact outcome the harness reports for it. Commit a fuzzing
//! reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::logframe::{HeaderError, Item, RECORD_VERSION, Record, SegmentHeader};
use gunmetal_fuzz::logframe::{Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/logframe")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 6] = [
    "empty",
    "four-gigabyte-length",
    "garbage-then-record",
    "header-january",
    "hi",
    "torn-tail",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome<'_>) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

fn rec(payload: &[u8]) -> Item<'_> {
    Item::Record(Record {
        version: RECORD_VERSION,
        payload,
    })
}

fn damaged(range: std::ops::Range<usize>) -> Item<'static> {
    Item::Damaged { range }
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
            items: vec![],
            tail: 0,
            header: Err(HeaderError::Missing),
        },
    );
}

/// Independently framed `b"hi"`: length 2, CRC-32C of `[0x01, 'h', 'i']`
/// = `0xC1D99F14`, version 1, payload.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_whole_record() {
    replay(
        "hi",
        &[
            0x02,
            0x00,
            0x00,
            0x00,
            0x14,
            0x9F,
            0xD9,
            0xC1,
            RECORD_VERSION,
            b'h',
            b'i',
        ],
        &Outcome {
            items: vec![rec(b"hi")],
            tail: 11,
            header: Err(HeaderError::InvalidLength { len: 2 }),
        },
    );
}

/// The first five octets of the `hi` record, a torn write.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_torn_tail() {
    replay(
        "torn-tail",
        &[0x02, 0x00, 0x00, 0x00, 0x14],
        &Outcome {
            items: vec![damaged(0..5)],
            tail: 0,
            header: Err(HeaderError::Damaged { range: 0..5 }),
        },
    );
}

/// Sixteen 0xFF octets: a length field of 4 GiB minus one, then more
/// garbage, and no whole record.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_four_gigabyte_length() {
    replay(
        "four-gigabyte-length",
        &[0xFF; 16],
        &Outcome {
            items: vec![damaged(0..16)],
            tail: 0,
            header: Err(HeaderError::Damaged { range: 0..16 }),
        },
    );
}

/// One garbage octet, then the independently framed `hi` record.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_garbage_then_a_record() {
    replay(
        "garbage-then-record",
        &[
            0xFF,
            0x02,
            0x00,
            0x00,
            0x00,
            0x14,
            0x9F,
            0xD9,
            0xC1,
            RECORD_VERSION,
            b'h',
            b'i',
        ],
        &Outcome {
            items: vec![damaged(0..1), rec(b"hi")],
            tail: 0,
            header: Err(HeaderError::Damaged { range: 0..1 }),
        },
    );
}

/// A header for stream 16 zero octets, year 1, January. Length 19,
/// CRC-32C of version 1 plus that payload = `0xDCACB0FE`.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_january_header() {
    replay(
        "header-january",
        &[
            0x13,
            0x00,
            0x00,
            0x00,
            0xFE,
            0xB0,
            0xAC,
            0xDC,
            RECORD_VERSION,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0x01,
            0x00,
            0x01,
        ],
        &Outcome {
            items: vec![rec(&[
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x01, 0x00, 0x01,
            ])],
            tail: 28,
            header: Ok(SegmentHeader {
                stream: [0; 16],
                year: 1,
                month: 1,
            }),
        },
    );
}
