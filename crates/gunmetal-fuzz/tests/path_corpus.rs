//! Replays the committed path fuzz corpus through its harness on stable
//! Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/path` has a test here that pins its exact bytes
//! and the exact outcome the harness reports for it. Commit a fuzzing
//! reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::path::PathError;
use gunmetal_fuzz::path::{Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/path")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 6] = [
    "dot-and-parent-inside",
    "empty",
    "nul-in-a-name",
    "parent-climbs-out",
    "single-name",
    "unicode-escapes-and-invalid-utf8",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

/// The names of a normalised path, as owned byte strings.
fn names(names: &[&[u8]]) -> Vec<Vec<u8>> {
    names.iter().map(|name| name.to_vec()).collect()
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
        b"",
        &Outcome {
            split: Err(PathError::EmptyName { component: 0 }),
            whole: Err(PathError::EmptyName { component: 0 }),
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_single_name() {
    replay(
        "single-name",
        b"cover.jpg",
        &Outcome {
            split: Ok(names(&[b"cover.jpg"])),
            whole: Ok(names(&[b"cover.jpg"])),
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_dot_and_a_parent_that_stay_inside() {
    replay(
        "dot-and-parent-inside",
        b"a/./b/../c",
        &Outcome {
            split: Ok(names(&[b"a", b"c"])),
            whole: Err(PathError::Separator {
                component: 0,
                offset: 1,
            }),
        },
    );
}

/// The Matroska attachment name of Jellyfin CVE-2026-49246.
///
/// Verifies: SEC-MED-028, SEC-MED-039
#[test]
fn replays_a_parent_that_climbs_out() {
    replay(
        "parent-climbs-out",
        b"../../etc/cron.d/x",
        &Outcome {
            split: Err(PathError::Escapes { component: 0 }),
            whole: Err(PathError::Separator {
                component: 0,
                offset: 2,
            }),
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_nul_in_a_name() {
    replay(
        "nul-in-a-name",
        b"a/b\0c",
        &Outcome {
            split: Err(PathError::Nul {
                component: 1,
                offset: 1,
            }),
            whole: Err(PathError::Separator {
                component: 0,
                offset: 1,
            }),
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-040
#[test]
fn replays_unicode_escapes_and_invalid_utf8() {
    replay(
        "unicode-escapes-and-invalid-utf8",
        b"Bj\xF6rk/two\nlines\x1B[0m",
        &Outcome {
            split: Ok(names(&[b"Bj\xF6rk", b"two\nlines\x1B[0m"])),
            whole: Err(PathError::Separator {
                component: 0,
                offset: 5,
            }),
        },
    );
}
