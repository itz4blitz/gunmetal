//! Replays the committed `Range` header fuzz corpus through its harness on
//! stable Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/http_range` has a test here that pins its exact
//! bytes and the exact answers the harness reports for it, one for each of
//! the representation lengths 0, 1, 1,000 and `u64::MAX`. Commit a fuzzing
//! reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::http::range::{Ignored, Refusal};
use gunmetal_fuzz::http_range::{Answer, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/http_range")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 10] = [
    "backwards",
    "cve-2011-3192-overlapping-ranges",
    "empty",
    "empty-suffix",
    "other-unit",
    "single-range",
    "suffix",
    "thirty-digits",
    "two-ranges",
    "upper-case-unit",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness answers `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: [Answer; 4]) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(run(&file), expected, "seed {name}");
}

/// The same answer for every length.
const fn always(answer: Answer) -> [Answer; 4] {
    [answer; 4]
}

const fn part(first: u64, last: u64) -> Answer {
    Answer::Partial { first, last }
}

/// Verifies: SEC-MED-028
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
        b"",
        always(Answer::Whole(Ignored::Malformed { offset: 0 })),
    );
}

/// Verifies: SEC-MED-028, SEC-MED-060
#[test]
fn replays_a_single_range() {
    replay(
        "single-range",
        b"bytes=0-499",
        [
            Answer::NotSatisfiable(Refusal::StartsPastEnd { first: 0, len: 0 }),
            part(0, 0),
            part(0, 499),
            part(0, 499),
        ],
    );
}

/// Verifies: SEC-MED-028, SEC-MED-060
#[test]
fn replays_a_suffix() {
    replay(
        "suffix",
        b"bytes=-500",
        [
            Answer::Whole(Ignored::EmptyRepresentation),
            part(0, 0),
            part(500, 999),
            part(u64::MAX - 500, u64::MAX - 1),
        ],
    );
}

/// Verifies: SEC-MED-028, SEC-NET-050, SEC-API-031
#[test]
fn replays_two_ranges() {
    replay(
        "two-ranges",
        b"bytes=0-1,5-9",
        always(Answer::NotSatisfiable(Refusal::SeveralRanges { count: 2 })),
    );
}

/// The shape of the killapache request: one open range, then overlapping
/// ranges from the same start, some of them backwards.
///
/// Verifies: SEC-MED-028, SEC-NET-050, SEC-API-031, SEC-HIS-036
#[test]
fn replays_the_overlapping_ranges_of_cve_2011_3192() {
    replay(
        "cve-2011-3192-overlapping-ranges",
        b"bytes=0-,5-0,5-1,5-2,5-3,5-4,5-5,5-6",
        always(Answer::NotSatisfiable(Refusal::SeveralRanges { count: 8 })),
    );
}

/// Verifies: SEC-MED-028, SEC-MED-060
#[test]
fn replays_a_position_of_thirty_digits() {
    replay(
        "thirty-digits",
        b"bytes=0-999999999999999999999999999999",
        [
            Answer::NotSatisfiable(Refusal::StartsPastEnd { first: 0, len: 0 }),
            part(0, 0),
            part(0, 999),
            part(0, u64::MAX - 1),
        ],
    );
}

/// Verifies: SEC-MED-028, SEC-MED-060
#[test]
fn replays_a_backwards_range() {
    replay(
        "backwards",
        b"bytes=500-100",
        always(Answer::Whole(Ignored::Backwards {
            first: 500,
            last: 100,
        })),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_an_upper_case_unit() {
    replay(
        "upper-case-unit",
        b"BYTES=0-",
        [
            Answer::NotSatisfiable(Refusal::StartsPastEnd { first: 0, len: 0 }),
            part(0, 0),
            part(0, 999),
            part(0, u64::MAX - 1),
        ],
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_another_unit() {
    replay(
        "other-unit",
        b"items=0-1",
        always(Answer::Whole(Ignored::OtherUnit)),
    );
}

/// Verifies: SEC-MED-028, SEC-MED-060
#[test]
fn replays_an_empty_suffix() {
    replay(
        "empty-suffix",
        b"bytes=-0",
        always(Answer::NotSatisfiable(Refusal::EmptySuffix)),
    );
}
