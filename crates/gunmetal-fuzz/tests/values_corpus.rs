//! Replays the committed typed-value fuzz corpus through its harness on
//! stable Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/values` has a test here that pins its exact
//! bytes and the exact outcome the harness reports for it. Commit a fuzzing
//! reproducer by adding its file and its test together.

use std::fs;
use std::path::PathBuf;

use gunmetal_core::values::{Field, ValueError};
use gunmetal_fuzz::values::{Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/values")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 8] = [
    "date-and-time",
    "empty",
    "isrc-with-hyphens",
    "mbid-in-upper-case",
    "peak-with-a-comma",
    "replaygain-with-a-comma",
    "track-of-total",
    "year-alone",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

fn malformed(field: Field) -> ValueError {
    ValueError::Malformed { field }
}

/// What every parser reports for text it cannot read at all.
fn nothing_read() -> Outcome {
    Outcome {
        mbid: Err(malformed(Field::Mbid)),
        isrc: Err(malformed(Field::Isrc)),
        number: Err(malformed(Field::Number)),
        date: Err(malformed(Field::Year)),
        gain: Err(malformed(Field::Gain)),
        peak: Err(malformed(Field::Peak)),
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

/// Verifies: SEC-MED-028, SEC-MED-014
#[test]
fn replays_the_empty_input() {
    replay("empty", &[], &nothing_read());
}

/// The example UUID of RFC 4122, in upper case.
///
/// Verifies: SEC-MED-028, SEC-MED-014
#[test]
fn replays_an_mbid_in_upper_case() {
    replay(
        "mbid-in-upper-case",
        b"F81D4FAE-7DEC-11D0-A765-00A0C91E6BF6",
        &Outcome {
            mbid: Ok("f81d4fae-7dec-11d0-a765-00a0c91e6bf6".to_owned()),
            ..nothing_read()
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-014
#[test]
fn replays_an_isrc_with_hyphens() {
    replay(
        "isrc-with-hyphens",
        b"us-s1z-99-00001",
        &Outcome {
            isrc: Ok("USS1Z9900001".to_owned()),
            ..nothing_read()
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-014
#[test]
fn replays_a_track_of_a_total() {
    replay(
        "track-of-total",
        b"03 of 12",
        &Outcome {
            number: Ok((3, Some(12))),
            ..nothing_read()
        },
    );
}

/// The time after the date is ignored, as in an ID3v2.4 timestamp.
///
/// Verifies: SEC-MED-028, SEC-MED-014
#[test]
fn replays_a_date_with_a_time() {
    replay(
        "date-and-time",
        b"2019-05-17T07:00:00Z",
        &Outcome {
            date: Ok((2019, Some(5), Some(17))),
            ..nothing_read()
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-014
#[test]
fn replays_a_replaygain_value_with_a_decimal_comma() {
    replay(
        "replaygain-with-a-comma",
        b"-6,5 dB",
        &Outcome {
            gain: Ok(-6.5),
            ..nothing_read()
        },
    );
}

/// One and a half is a peak and a gain alike.
///
/// Verifies: SEC-MED-028, SEC-MED-014
#[test]
fn replays_a_peak_with_a_decimal_comma() {
    replay(
        "peak-with-a-comma",
        b"1,5",
        &Outcome {
            gain: Ok(1.5),
            peak: Ok(1.5),
            ..nothing_read()
        },
    );
}

/// A year is also a track number, but far beyond any gain or peak.
///
/// Verifies: SEC-MED-028, SEC-MED-014
#[test]
fn replays_a_year_alone() {
    replay(
        "year-alone",
        b"2019",
        &Outcome {
            number: Ok((2019, None)),
            date: Ok((2019, None, None)),
            gain: Err(ValueError::Unusable { field: Field::Gain }),
            peak: Err(ValueError::Unusable { field: Field::Peak }),
            ..nothing_read()
        },
    );
}
