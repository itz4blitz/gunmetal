//! Replays the committed one-time-code fuzz corpus through its harness on
//! stable Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/otp` has a test here that pins its exact
//! bytes and the exact outcome the harness reports for it. Commit a fuzzing
//! reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::otp::{CodeError, CodeKind};
use gunmetal_fuzz::otp::{Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/otp")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 8] = [
    "claim-zeros",
    "empty",
    "first-symbol-too-wide",
    "lowercase-pairing",
    "o-for-zero",
    "pairing-zeros",
    "recovery-zeros",
    "wrong-checksum",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

fn malformed(kind: CodeKind) -> CodeError {
    CodeError::Malformed { kind }
}

fn checksum(kind: CodeKind) -> CodeError {
    CodeError::Checksum { kind }
}

fn wrong(expected: CodeKind, actual: CodeKind) -> CodeError {
    CodeError::WrongKind { expected, actual }
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

/// Verifies: SEC-MED-028, SEC-IAM-007
#[test]
fn replays_the_empty_input() {
    replay(
        "empty",
        &[],
        &Outcome {
            claim: Err(malformed(CodeKind::Claim)),
            recovery: Err(malformed(CodeKind::Recovery)),
            pairing: Err(malformed(CodeKind::Pairing)),
        },
    );
}

/// Verifies: SEC-MED-028, SEC-IAM-007
#[test]
fn replays_a_zero_claim_code() {
    replay(
        "claim-zeros",
        b"00000-00000-00000-00000-00000-00",
        &Outcome {
            claim: Ok("00000-00000-00000-00000-00000-00".to_owned()),
            recovery: Err(wrong(CodeKind::Recovery, CodeKind::Claim)),
            pairing: Err(wrong(CodeKind::Pairing, CodeKind::Claim)),
        },
    );
}

/// Verifies: SEC-MED-028, SEC-IAM-089
#[test]
fn replays_a_zero_recovery_code() {
    replay(
        "recovery-zeros",
        b"0000-0000-0000-0000-0",
        &Outcome {
            claim: Err(wrong(CodeKind::Claim, CodeKind::Recovery)),
            recovery: Ok("0000-0000-0000-0000-0".to_owned()),
            pairing: Err(wrong(CodeKind::Pairing, CodeKind::Recovery)),
        },
    );
}

/// Verifies: SEC-MED-028, SEC-IAM-056
#[test]
fn replays_a_zero_pairing_code() {
    replay(
        "pairing-zeros",
        b"BBBB-BBBB",
        &Outcome {
            claim: Err(wrong(CodeKind::Claim, CodeKind::Pairing)),
            recovery: Err(wrong(CodeKind::Recovery, CodeKind::Pairing)),
            pairing: Ok("BBBB-BBBB".to_owned()),
        },
    );
}

/// Verifies: SEC-MED-028, SEC-IAM-007
#[test]
fn replays_a_claim_code_typed_with_o_for_zero() {
    replay(
        "o-for-zero",
        b"oooo o-oooo o-oooo o-oooo o-oooo o-oo",
        &Outcome {
            claim: Ok("00000-00000-00000-00000-00000-00".to_owned()),
            recovery: Err(wrong(CodeKind::Recovery, CodeKind::Claim)),
            pairing: Err(wrong(CodeKind::Pairing, CodeKind::Claim)),
        },
    );
}

/// Verifies: SEC-MED-028, SEC-IAM-056
#[test]
fn replays_a_lowercase_pairing_code() {
    replay(
        "lowercase-pairing",
        b"bbbb-bbbb",
        &Outcome {
            claim: Err(wrong(CodeKind::Claim, CodeKind::Pairing)),
            recovery: Err(wrong(CodeKind::Recovery, CodeKind::Pairing)),
            pairing: Ok("BBBB-BBBB".to_owned()),
        },
    );
}

/// Verifies: SEC-MED-028, SEC-IAM-007
#[test]
fn replays_a_wrong_checksum() {
    replay(
        "wrong-checksum",
        b"00000-00000-00000-00000-00000-01",
        &Outcome {
            claim: Err(checksum(CodeKind::Claim)),
            recovery: Err(wrong(CodeKind::Recovery, CodeKind::Claim)),
            pairing: Err(wrong(CodeKind::Pairing, CodeKind::Claim)),
        },
    );
}

/// Verifies: SEC-MED-028, SEC-IAM-007
#[test]
fn replays_a_claim_code_with_more_than_128_bits() {
    replay(
        "first-symbol-too-wide",
        b"80000-00000-00000-00000-00000-00",
        &Outcome {
            claim: Err(malformed(CodeKind::Claim)),
            recovery: Err(wrong(CodeKind::Recovery, CodeKind::Claim)),
            pairing: Err(wrong(CodeKind::Pairing, CodeKind::Claim)),
        },
    );
}
