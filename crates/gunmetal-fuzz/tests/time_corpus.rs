//! Replays the committed RFC 3339 fuzz corpus through its harness on
//! stable Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/time` has a test here that pins its exact bytes
//! and the exact outcome the harness reports for it. Commit a fuzzing
//! reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::time::TimeError;
use gunmetal_fuzz::time::{Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/time")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 7] = [
    "before-year-zero",
    "empty",
    "february-30",
    "leap-second",
    "offset-and-nanoseconds",
    "offset-past-23-hours",
    "utc-with-milliseconds",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(run(&file), expected, "seed {name}");
}

fn millis(result: Result<i64, TimeError>) -> Outcome {
    Outcome { millis: result }
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
    replay("empty", &[], millis(Err(TimeError::Syntax)));
}

/// 2009-02-13T23:31:30Z is Unix time 1,234,567,890.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_utc_time_with_milliseconds() {
    replay(
        "utc-with-milliseconds",
        b"2009-02-13T23:31:30.123Z",
        millis(Ok(1_234_567_890_123)),
    );
}

/// The same instant an hour ahead of UTC, with a lower-case separator and
/// nine digits of fraction kept to the millisecond.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_an_offset_and_nanoseconds() {
    replay(
        "offset-and-nanoseconds",
        b"2009-02-14t00:31:30.123456789+01:00",
        millis(Ok(1_234_567_890_123)),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_leap_second() {
    replay(
        "leap-second",
        b"2016-12-31T23:59:60Z",
        millis(Err(TimeError::InvalidTime {
            hour: 23,
            minute: 59,
            second: 60,
        })),
    );
}

/// Midnight of 0000-01-01 one minute ahead of UTC is a minute before the
/// first instant RFC 3339 can write, 62,167,219,200 seconds before the
/// Unix epoch.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_time_before_year_zero() {
    replay(
        "before-year-zero",
        b"0000-01-01T00:00:00+00:01",
        millis(Err(TimeError::OutOfRange {
            millis: -62_167_219_260_000,
        })),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_day_that_does_not_exist() {
    replay(
        "february-30",
        b"2023-02-30T00:00:00Z",
        millis(Err(TimeError::InvalidDate {
            year: 2023,
            month: 2,
            day: 30,
        })),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_an_offset_past_23_hours() {
    replay(
        "offset-past-23-hours",
        b"2009-02-13T23:31:30+24:00",
        millis(Err(TimeError::InvalidOffset {
            hours: 24,
            minutes: 0,
        })),
    );
}
