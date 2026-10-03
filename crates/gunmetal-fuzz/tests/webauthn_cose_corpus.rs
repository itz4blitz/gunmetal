//! Replays the committed `WebAuthn` COSE-key fuzz corpus through its harness
//! on stable Rust, so that `cargo test` and the gate run every seed and
//! every reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/webauthn_cose` has a test here that pins its
//! exact bytes and the exact outcome the harness reports for it. Commit a
//! fuzzing reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::parse::ParseFault;
use gunmetal_core::webauthn::{CoseKey, WebauthnError};
use gunmetal_fuzz::webauthn_cose::{Outcome, run};

fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/webauthn_cose")
}

const SEEDS: [&str; 3] = ["empty", "es256", "rs256"];

const ES256: [u8; 77] = [
    0xA5, 0x01, 0x02, 0x03, 0x26, 0x20, 0x01, 0x21, 0x58, 0x20, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
    0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
    0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x22, 0x58, 0x20, 0x22, 0x22, 0x22,
    0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22,
    0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22,
];

fn replay(name: &str, bytes: &[u8], expected: &Outcome) {
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
            key: Err(WebauthnError::Fault(ParseFault::Truncated {
                offset: 0,
                needed: 1,
                available: 0,
            })),
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-001
#[test]
fn replays_an_es256_key() {
    replay(
        "es256",
        &ES256,
        &Outcome {
            key: Ok(CoseKey::Es256 {
                x: [0x11; 32],
                y: [0x22; 32],
            }),
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-001
#[test]
fn replays_an_rs256_key() {
    replay(
        "rs256",
        &[0xA2, 0x01, 0x03, 0x03, 0x39, 0x01, 0x00],
        &Outcome {
            key: Err(WebauthnError::Algorithm {
                offset: 0,
                kty: Some(3),
                alg: Some(-257),
                crv: None,
            }),
        },
    );
}
