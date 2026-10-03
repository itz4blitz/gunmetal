//! Replays the committed wire codec fuzz corpus through its harness on
//! stable Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/wire` has a test here that pins its exact bytes
//! and the exact outcome the harness reports for it. Commit a fuzzing
//! reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::parse::{LimitKind, ParseFault};
use gunmetal_core::wire::{FrameKind, PostcardError, ProtocolVersion, Upgrade, WireError};
use gunmetal_fuzz::wire::{Outcome, Read};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/wire")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 9] = [
    "empty",
    "newer-version-with-a-trailing-octet",
    "over-the-cap",
    "sample",
    "stream-then-unknown-kind",
    "string-past-the-limit",
    "too-many-entries",
    "too-short",
    "truncated",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&gunmetal_fuzz::wire::run(&file), expected, "seed {name}");
}

/// What a client that offered no version, or only versions older than 1,
/// is told by a server that speaks version 1.
const CLIENT_MUST_UPDATE: Result<ProtocolVersion, Upgrade> = Err(Upgrade::Client {
    server_newest: Some(ProtocolVersion(1)),
});

/// No frame read, stopped by `stop`.
fn refused(stop: WireError) -> Outcome {
    Outcome {
        frames: vec![],
        stop: Some(stop),
        agreed: CLIENT_MUST_UPDATE,
    }
}

/// Verifies: SEC-MED-028, SEC-MED-030, SEC-HIS-036
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
            frames: vec![],
            stop: None,
            agreed: CLIENT_MUST_UPDATE,
        },
    );
}

/// A version 1 response holding `(300, "hi", Some(true), [Ok(5),
/// Err("no")])`: 300 as the varint AC 02, a two-octet string, `Some` and
/// `true`, a list of two, variant 0 with the octet 5, and variant 1 with a
/// two-octet string. Seventeen octets follow the length.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_frame_holding_a_sample() {
    replay(
        "sample",
        &[
            0x11, 0, 0, 0, 1, 0, 2, 0xAC, 0x02, 2, b'h', b'i', 1, 1, 2, 0, 5, 1, 2, b'n', b'o',
        ],
        &Outcome {
            frames: vec![Read {
                version: ProtocolVersion(1),
                kind: FrameKind::Response,
                len: 21,
                decoded: Ok((
                    300,
                    "hi".to_owned(),
                    Some(true),
                    vec![Ok(5), Err("no".to_owned())],
                )),
            }],
            stop: None,
            agreed: Ok(ProtocolVersion(1)),
        },
    );
}

/// An empty `End` frame, whose payload ends before the sample's first
/// field, then a frame of kind 9.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_stream_that_ends_in_an_unknown_kind() {
    replay(
        "stream-then-unknown-kind",
        &[3, 0, 0, 0, 1, 0, 4, 3, 0, 0, 0, 1, 0, 9],
        &Outcome {
            frames: vec![Read {
                version: ProtocolVersion(1),
                kind: FrameKind::End,
                len: 7,
                decoded: Err(WireError::Malformed {
                    offset: 0,
                    reason: PostcardError::DeserializeUnexpectedEnd,
                }),
            }],
            stop: Some(WireError::UnknownKind { kind: 9 }),
            agreed: Ok(ProtocolVersion(1)),
        },
    );
}

/// A version 2 request: a zero, an empty string, `None` and an empty list
/// take the four octets 0, then one octet more is left over. The server
/// speaks only version 1, so it is the one that must update.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_newer_version_with_a_trailing_octet() {
    replay(
        "newer-version-with-a-trailing-octet",
        &[8, 0, 0, 0, 2, 0, 1, 0, 0, 0, 0, 0xFF],
        &Outcome {
            frames: vec![Read {
                version: ProtocolVersion(2),
                kind: FrameKind::Request,
                len: 12,
                decoded: Err(WireError::Trailing { offset: 4, len: 1 }),
            }],
            stop: None,
            agreed: Err(Upgrade::Server {
                client_newest: ProtocolVersion(2),
            }),
        },
    );
}

/// A length of 65, one over the harness's cap of 64.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_length_over_the_cap() {
    replay(
        "over-the-cap",
        &[65, 0, 0, 0],
        &refused(WireError::TooLarge { len: 65, max: 64 }),
    );
}

/// A length of 2, which cannot cover the version and the kind.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_length_shorter_than_the_header() {
    replay(
        "too-short",
        &[2, 0, 0, 0, 1, 0],
        &refused(WireError::TooShort { len: 2 }),
    );
}

/// A length of 10 with only two octets after it.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_truncated_frame() {
    replay(
        "truncated",
        &[10, 0, 0, 0, 1, 0],
        &refused(WireError::Fault(ParseFault::Truncated {
            offset: 4,
            needed: 10,
            available: 2,
        })),
    );
}

/// A part whose list declares five entries, `Ok(1)` to `Ok(5)`; the
/// harness allows four. The entries start after the four octets of the
/// zero, the empty string, `None` and the count.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_list_past_the_entry_limit() {
    replay(
        "too-many-entries",
        &[
            0x11, 0, 0, 0, 1, 0, 3, 0, 0, 0, 5, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5,
        ],
        &Outcome {
            frames: vec![Read {
                version: ProtocolVersion(1),
                kind: FrameKind::Part,
                len: 21,
                decoded: Err(WireError::Fault(ParseFault::LimitExceeded {
                    limit: LimitKind::Children,
                    value: 5,
                    max: 4,
                    offset: 4,
                })),
            }],
            stop: None,
            agreed: Ok(ProtocolVersion(1)),
        },
    );
}

/// A refusal whose string holds nine octets; the harness allows eight. The
/// string's octets start at offset 2, after the zero and its length.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_string_past_the_length_limit() {
    replay(
        "string-past-the-limit",
        &[
            0x10, 0, 0, 0, 1, 0, 5, 0, 9, b'a', b'b', b'c', b'd', b'e', b'f', b'g', b'h', b'i', 0,
            0,
        ],
        &Outcome {
            frames: vec![Read {
                version: ProtocolVersion(1),
                kind: FrameKind::Refusal,
                len: 20,
                decoded: Err(WireError::Fault(ParseFault::LimitExceeded {
                    limit: LimitKind::LongText,
                    value: 9,
                    max: 8,
                    offset: 2,
                })),
            }],
            stop: None,
            agreed: Ok(ProtocolVersion(1)),
        },
    );
}
