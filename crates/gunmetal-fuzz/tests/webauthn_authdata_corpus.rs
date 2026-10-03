//! Replays the committed `WebAuthn` authenticator-data fuzz corpus through
//! its harness on stable Rust, so that `cargo test` and the gate run every
//! seed and every reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/webauthn_authdata` has a test here that pins
//! its exact bytes and the exact outcome the harness reports for it. Commit
//! a fuzzing reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::parse::ParseFault;
use gunmetal_core::webauthn::{AuthData, Flags, WebauthnError};
use gunmetal_fuzz::webauthn_authdata::{Outcome, run};

fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/webauthn_authdata")
}

const SEEDS: [&str; 3] = ["backup-state-without-eligible", "empty", "user-present"];

fn replay(name: &str, bytes: &[u8], expected: &Outcome<'_>) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

fn prefix(flags: u8) -> [u8; 37] {
    let mut bytes = [0u8; 37];
    bytes[32] = flags;
    bytes
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
            data: Err(WebauthnError::Fault(ParseFault::Truncated {
                offset: 0,
                needed: 32,
                available: 0,
            })),
        },
    );
}

/// Verifies: SEC-MED-028, SEC-IAM-018, SEC-IAM-020
#[test]
fn replays_user_present_prefix() {
    replay(
        "user-present",
        &prefix(0x01),
        &Outcome {
            data: Ok(AuthData {
                rp_id_hash: [0; 32],
                flags: Flags {
                    raw: 0x01,
                    user_present: true,
                    user_verified: false,
                    backup_eligible: false,
                    backup_state: false,
                    attested_credential_data: false,
                    extension_data: false,
                },
                sign_count: 0,
                attested: None,
                extensions: None,
            }),
        },
    );
}

/// Verifies: SEC-MED-028, SEC-IAM-021
#[test]
fn replays_backup_state_without_eligible() {
    replay(
        "backup-state-without-eligible",
        &prefix(0x10),
        &Outcome {
            data: Err(WebauthnError::BackupState {
                offset: 32,
                flags: 0x10,
            }),
        },
    );
}
