//! Replays the committed capability-token fuzz corpus through its harness on
//! stable Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/token` has a test here that pins its exact bytes
//! and the exact outcome the harness reports for it. Commit a fuzzing
//! reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::time::Timestamp;
use gunmetal_core::token::{
    CapError, CapFields, Expiry, MalformedReason, Operation, Representation, SessionHashError,
};
use gunmetal_fuzz::token::{Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/token")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 10] = [
    "empty",
    "expired",
    "future",
    "one-character",
    "padded",
    "query-string",
    "session-token",
    "three-octets",
    "unknown-representation",
    "unknown-version",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

fn malformed(reason: MalformedReason, session_len: usize) -> Outcome {
    Outcome {
        verify: Err(CapError::Malformed { reason }),
        session: Err(SessionHashError::Malformed { len: session_len }),
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

/// Verifies: SEC-MED-028
#[test]
fn replays_the_empty_input() {
    replay(
        "empty",
        &[],
        &malformed(MalformedReason::Length { len: 0 }, 0),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_single_character() {
    replay(
        "one-character",
        b"A",
        &malformed(MalformedReason::Encoding, 1),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_three_decoded_octets() {
    replay(
        "three-octets",
        b"AAEC",
        &malformed(MalformedReason::Length { len: 3 }, 4),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_query_string() {
    replay(
        "query-string",
        b"abc?x=1",
        &malformed(MalformedReason::Encoding, 7),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_padded_spelling() {
    replay(
        "padded",
        b"AQEAAAAAAAAD6AEAAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAALA0TGHY2zhTXKivzq8L8Ss=",
        &malformed(MalformedReason::Encoding, 72),
    );
}

/// Verifies: SEC-MED-028, SEC-API-026
#[test]
fn replays_an_unknown_version() {
    replay(
        "unknown-version",
        b"AAEAAAAAAAAD6AEAAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAALA0TGHY2zhTXKivzq8L8Ss",
        &malformed(MalformedReason::Version { version: 0 }, 71),
    );
}

/// Verifies: SEC-MED-028, SEC-API-026
#[test]
fn replays_a_representation_outside_the_fixed_set() {
    replay(
        "unknown-representation",
        b"AQEAAAAAAAAD6AH__wAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAALA0TGHY2zhTXKivzq8L8Ss",
        &malformed(MalformedReason::Representation { value: 0xFFFF }, 71),
    );
}

/// A structurally valid token whose expiry is the Unix epoch, so the
/// harness's `now` of 0 ms refuses it after the MAC check.
///
/// Verifies: SEC-MED-028, SEC-API-026, SEC-API-027
#[test]
fn replays_an_expired_token() {
    let expiry = Expiry::from_unix_seconds(0).expect("the epoch is a timestamp");
    let now = Timestamp::from_millis(0).expect("the epoch is a timestamp");
    replay(
        "expired",
        b"AQEAAAAAAAAAAAEAAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAALA0TGHY2zhTXKivzq8L8Ss",
        &Outcome {
            verify: Err(CapError::Expired { expiry, now }),
            session: Err(SessionHashError::Malformed { len: 71 }),
        },
    );
}

/// A structurally valid token whose tag is the harness's fake MAC and whose
/// expiry is one thousand seconds after the epoch.
///
/// Verifies: SEC-MED-028, SEC-API-026
#[test]
fn replays_a_future_token() {
    replay(
        "future",
        b"AQEAAAAAAAAD6AEAAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAALA0TGHY2zhTXKivzq8L8Ss",
        &Outcome {
            verify: Ok(CapFields {
                kid: 1,
                expiry: Expiry::from_unix_seconds(1000).expect("1000 seconds is a timestamp"),
                operation: Operation::Stream,
                representation: Representation::Original,
                object: [0; 16],
                handle: [0; 8],
            }),
            session: Err(SessionHashError::Malformed { len: 71 }),
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_thirty_two_octet_session_token() {
    let rfc4231_case1: [u8; 32] = [
        0xb0, 0x34, 0x4c, 0x61, 0xd8, 0xdb, 0x38, 0x53, 0x5c, 0xa8, 0xaf, 0xce, 0xaf, 0x0b, 0xf1,
        0x2b, 0x88, 0x1d, 0xc1, 0x81, 0xfd, 0xd9, 0xd4, 0xa9, 0x27, 0xad, 0xb2, 0xbd, 0x57, 0xb7,
        0x99, 0xdf,
    ];
    replay(
        "session-token",
        &[0; 32],
        &Outcome {
            verify: Err(CapError::Malformed {
                reason: MalformedReason::Encoding,
            }),
            session: Ok(rfc4231_case1),
        },
    );
}
