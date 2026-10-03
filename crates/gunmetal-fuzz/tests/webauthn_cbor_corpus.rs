//! Replays the committed `WebAuthn` CBOR fuzz corpus through its harness on
//! stable Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/webauthn_cbor` has a test here that pins its
//! exact bytes and the exact outcome the harness reports for it. Commit a
//! fuzzing reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::parse::ParseFault;
use gunmetal_core::webauthn::{Cbor, Item, WebauthnError};
use gunmetal_fuzz::webauthn_cbor::{Outcome, run};

fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/webauthn_cbor")
}

const SEEDS: [&str; 4] = [
    "duplicate-keys",
    "empty",
    "indefinite-bytes",
    "unsigned-zero",
];

fn replay(name: &str, bytes: &[u8], expected: &Outcome<'_>) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
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
            item: Err(WebauthnError::Fault(ParseFault::Truncated {
                offset: 0,
                needed: 1,
                available: 0,
            })),
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-001
#[test]
fn replays_an_unsigned_zero() {
    replay(
        "unsigned-zero",
        &[0x00],
        &Outcome {
            item: Ok(Item {
                value: Cbor::Unsigned(0),
                rest: &[],
            }),
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-001
#[test]
fn replays_indefinite_bytes() {
    replay(
        "indefinite-bytes",
        &[0x5F],
        &Outcome {
            item: Err(WebauthnError::Indefinite {
                offset: 0,
                major: 2,
            }),
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-001
#[test]
fn replays_duplicate_map_keys() {
    replay(
        "duplicate-keys",
        &[0xA2, 0x01, 0xF4, 0x01, 0xF5],
        &Outcome {
            item: Err(WebauthnError::DuplicateKey { offset: 3 }),
        },
    );
}
