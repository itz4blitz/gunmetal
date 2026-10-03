//! Replays the committed base64 fuzz corpus through its harness on stable
//! Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/base64` has a test here that pins its exact
//! bytes and the exact outcome the harness reports for it. Commit a fuzzing
//! reproducer by adding its file and its test together.

use std::fs;
use std::path::PathBuf;

use gunmetal_core::base64::B64Error;
use gunmetal_fuzz::base64::{Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/base64")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 8] = [
    "empty",
    "five-characters",
    "newline-inside",
    "padded",
    "past-the-cap",
    "rfc4648-foobar",
    "trailing-bits",
    "url-safe-characters",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

/// The same result in both alphabets.
fn both(result: Result<Vec<u8>, B64Error>) -> Outcome {
    Outcome {
        standard: result.clone(),
        url_safe: result,
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
    replay("empty", &[], &both(Ok(vec![])));
}

/// RFC 4648, section 10.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_the_rfc_4648_vector_for_foobar() {
    replay("rfc4648-foobar", b"Zm9vYmFy", &both(Ok(b"foobar".to_vec())));
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_padded_value() {
    replay("padded", b"Zm8=", &both(Ok(b"fo".to_vec())));
}

/// `-`, `_` and `8` are 62, 63 and 60: the bits 111110 111111 111100, or
/// the octets 0xFB and 0xFF and two zero bits. The standard alphabet has
/// neither `-` nor `_`.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_the_characters_only_the_url_safe_alphabet_has() {
    replay(
        "url-safe-characters",
        b"-_8",
        &Outcome {
            standard: Err(B64Error::InvalidByte {
                offset: 0,
                byte: b'-',
            }),
            url_safe: Ok(vec![0xFB, 0xFF]),
        },
    );
}

/// `Z` and `h` are 25 and 33, 011001 100001: the octet `f` and four bits
/// that are not zero, so `Zg==` is the only way to write `f`.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_last_character_with_bits_left_over() {
    replay(
        "trailing-bits",
        b"Zh==",
        &both(Err(B64Error::TrailingBits { offset: 1 })),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_newline_inside_the_value() {
    replay(
        "newline-inside",
        b"Zm9\nYmFy",
        &both(Err(B64Error::InvalidByte {
            offset: 3,
            byte: b'\n',
        })),
    );
}

/// No encoding ends with a single character in its last group of four.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_length_no_encoding_produces() {
    replay(
        "five-characters",
        b"Zm9vY",
        &both(Err(B64Error::InvalidLength { len: 5 })),
    );
}

/// Sixty-eight characters are seventeen groups of four, which decode to
/// fifty-one octets, three more than the harness allows.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_value_that_decodes_past_the_cap() {
    replay(
        "past-the-cap",
        &[b'A'; 68],
        &both(Err(B64Error::TooLong {
            needed: 51,
            max: 48,
        })),
    );
}
