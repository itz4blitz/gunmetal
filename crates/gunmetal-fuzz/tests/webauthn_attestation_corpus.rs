//! Replays the committed `WebAuthn` attestation-object fuzz corpus through
//! its harness on stable Rust, so that `cargo test` and the gate run every
//! seed and every reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/webauthn_attestation` has a test here that pins
//! its exact bytes and the exact outcome the harness reports for it. Commit
//! a fuzzing reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::parse::ParseFault;
use gunmetal_core::webauthn::{
    Attestation, AttestationField, AttestedCredential, CoseKey, Flags, WebauthnError,
};
use gunmetal_fuzz::webauthn_attestation::{Outcome, run};

fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/webauthn_attestation")
}

const SEEDS: [&str; 4] = ["empty", "no-credential", "none-es256", "packed"];

fn replay(name: &str, bytes: &[u8], expected: &Outcome<'_>) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

/// A text string shorter than 24 octets.
fn text(value: &str) -> Vec<u8> {
    let mut out = vec![0x60 | u8::try_from(value.len()).expect("short text")];
    out.extend_from_slice(value.as_bytes());
    out
}

/// Authenticator data with `flags`, a sign count of 7 and, when flag bit 6
/// is set, a two-octet credential id and an ES256 key.
fn auth(flags: u8) -> Vec<u8> {
    let mut out = vec![0x5A; 32];
    out.extend_from_slice(&[flags, 0, 0, 0, 7]);
    if flags & 0x40 != 0 {
        out.extend_from_slice(&[0xA7; 16]);
        out.extend_from_slice(&[0x00, 0x02, 0xC1, 0xC2]);
        out.extend_from_slice(&[0xA5, 0x01, 0x02, 0x03, 0x26, 0x20, 0x01, 0x21, 0x58, 0x20]);
        out.extend_from_slice(&[0x11; 32]);
        out.extend_from_slice(&[0x22, 0x58, 0x20]);
        out.extend_from_slice(&[0x22; 32]);
    }
    out
}

/// An attestation object in CTAP2 canonical key order.
fn object(fmt: &str, auth_data: &[u8]) -> Vec<u8> {
    let mut out = vec![0xA3];
    out.extend_from_slice(&text("fmt"));
    out.extend_from_slice(&text(fmt));
    out.extend_from_slice(&text("attStmt"));
    out.push(0xA0);
    out.extend_from_slice(&text("authData"));
    out.extend_from_slice(&[0x58, u8::try_from(auth_data.len()).expect("short data")]);
    out.extend_from_slice(auth_data);
    out
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
            attestation: Err(WebauthnError::Fault(ParseFault::Truncated {
                offset: 0,
                needed: 1,
                available: 0,
            })),
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_none_attestation_with_an_es256_key() {
    replay(
        "none-es256",
        &object("none", &auth(0x45)),
        &Outcome {
            attestation: Ok(Attestation {
                rp_id_hash: [0x5A; 32],
                flags: Flags {
                    raw: 0x45,
                    user_present: true,
                    user_verified: true,
                    backup_eligible: false,
                    backup_state: false,
                    attested_credential_data: true,
                    extension_data: false,
                },
                sign_count: 7,
                credential: AttestedCredential {
                    aaguid: [0xA7; 16],
                    credential_id: &[0xC1, 0xC2],
                    public_key: CoseKey::Es256 {
                        x: [0x11; 32],
                        y: [0x22; 32],
                    },
                },
                extensions: None,
            }),
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_packed_attestation() {
    replay(
        "packed",
        &object("packed", &auth(0x45)),
        &Outcome {
            attestation: Err(WebauthnError::Attestation {
                offset: 0,
                field: AttestationField::Fmt,
            }),
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_an_attestation_without_a_credential() {
    replay(
        "no-credential",
        &object("none", &auth(0x05)),
        &Outcome {
            attestation: Err(WebauthnError::MissingCredential { offset: 62 }),
        },
    );
}
