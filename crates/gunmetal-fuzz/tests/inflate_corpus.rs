//! Replays the committed corpus of the decompression helper's harness on
//! stable Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/inflate` has a test here that pins its exact
//! bytes and the exact outcome the harness reports for it. The expected
//! outcomes come from RFC 1950 and RFC 1951 worked by hand; the bombs'
//! inflated sizes were checked against Python's `zlib`. Commit a fuzzing
//! reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::inflate::InflateError;
use gunmetal_core::parse::ParseFault;
use gunmetal_fuzz::inflate::{Inflated, Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/inflate")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 8] = [
    "bad-checksum",
    "deflate-bomb",
    "empty",
    "hello-zlib",
    "reserved-block-type",
    "stored-hello",
    "truncated-checksum",
    "zlib-bomb",
];

/// `zlib.compress(b"hello")`: a zlib header for the default level, one
/// fixed-Huffman block and the Adler-32 0x062C0215.
const HELLO_ZLIB: [u8; 13] = [
    0x78, 0x9C, 0xCB, 0x48, 0xCD, 0xC9, 0xC9, 0x07, 0x00, 0x06, 0x2C, 0x02, 0x15,
];

/// A raw deflate bomb: one dynamic-Huffman block whose code makes each
/// 258-octet back-reference two zero bits, holding 255 of them, so it
/// inflates to 1 + 255 × 258 = 65,791 zero octets, 255 more than the
/// harness's cap. Its 78 octets are a 14-octet header and code table, 63
/// zero octets of matches, and the end-of-block code.
fn bomb() -> Vec<u8> {
    let head = [
        0xED, 0xC1, 0x81, 0x00, 0x00, 0x00, 0x00, 0x80, 0xA0, 0xFD, 0xA9, 0x17, 0xA9, 0x02,
    ];
    [&head[..], &[0; 63], &[0x06]].concat()
}

/// What a bomb leaves in the buffer: the declared 65,536 octets, all zero.
fn cap_of_zeros() -> Vec<u8> {
    std::iter::repeat_n(0, 65_536).collect()
}

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes);
    assert_eq!(&run(&file), expected);
}

/// A call that failed with `error`, leaving `output` in its buffer.
fn failed(error: InflateError, output: &[u8]) -> Inflated {
    Inflated {
        result: Err(error),
        output: output.to_vec(),
    }
}

/// The input ends before the stream does, after `offset` octets.
fn truncated(offset: u64) -> InflateError {
    InflateError::Fault(ParseFault::Truncated {
        offset,
        needed: 1,
        available: 0,
    })
}

/// Verifies: SEC-MED-028
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
            zlib: failed(truncated(0), &[]),
            deflate: failed(truncated(0), &[]),
        },
    );
}

/// As zlib, the stream inflates. As deflate, 0x78 opens a stored block
/// (BFINAL 0, BTYPE 00) whose length 0xCB9C is not the complement of
/// 0xCD48, found once the five octets of its header are read.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_what_zlib_itself_wrote() {
    replay(
        "hello-zlib",
        &HELLO_ZLIB,
        &Outcome {
            zlib: Inflated {
                result: Ok(13),
                output: b"hello".to_vec(),
            },
            deflate: failed(InflateError::Corrupt { offset: 5 }, &[]),
        },
    );
}

/// As deflate, one final stored block of five octets. As zlib, the header
/// 0x01 0x05 names compression method 1, which RFC 1950 does not define.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_stored_block() {
    replay(
        "stored-hello",
        &[0x01, 0x05, 0x00, 0xFA, 0xFF, b'h', b'e', b'l', b'l', b'o'],
        &Outcome {
            zlib: failed(InflateError::Corrupt { offset: 2 }, &[]),
            deflate: Inflated {
                result: Ok(10),
                output: b"hello".to_vec(),
            },
        },
    );
}

/// As deflate, BFINAL 1 with the reserved block type 11. As zlib, one
/// octet is half a header.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_reserved_block_type() {
    replay(
        "reserved-block-type",
        &[0x07],
        &Outcome {
            zlib: failed(truncated(1), &[]),
            deflate: failed(InflateError::Corrupt { offset: 1 }, &[]),
        },
    );
}

/// "abc" in a stored block, closed with the Adler-32 of "abd"
/// (0x024E0128 instead of 0x024D0127). The data is inflated before the
/// checksum is read, so the buffer holds it when the mismatch is found.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_wrong_checksum() {
    replay(
        "bad-checksum",
        &[
            0x78, 0x01, 0x01, 0x03, 0x00, 0xFC, 0xFF, b'a', b'b', b'c', 0x02, 0x4E, 0x01, 0x28,
        ],
        &Outcome {
            zlib: failed(InflateError::ChecksumMismatch { offset: 14 }, b"abc"),
            deflate: failed(InflateError::Corrupt { offset: 5 }, &[]),
        },
    );
}

/// `zlib.compress(b"hello")` without the checksum's last octet.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_stream_cut_inside_its_checksum() {
    replay(
        "truncated-checksum",
        &HELLO_ZLIB[..12],
        &Outcome {
            zlib: failed(truncated(12), b"hello"),
            deflate: failed(InflateError::Corrupt { offset: 5 }, &[]),
        },
    );
}

/// 78 octets that inflate to 65,791: the buffer stops at the declared
/// 65,536. As zlib, the first octet names compression method 13.
///
/// Verifies: SEC-MED-009, SEC-MED-028
#[test]
fn replays_a_deflate_bomb() {
    replay(
        "deflate-bomb",
        &bomb(),
        &Outcome {
            zlib: failed(InflateError::Corrupt { offset: 2 }, &[]),
            deflate: failed(
                InflateError::LongerThanDeclared {
                    declared: 65_536,
                    offset: 0,
                },
                &cap_of_zeros(),
            ),
        },
    );
}

/// The deflate bomb in zlib framing, with the Adler-32 of 65,791 zeros
/// (0x010E0001). As deflate, 0x78 opens a stored block whose length 0xED01
/// is not the complement of 0x81C1.
///
/// Verifies: SEC-MED-009, SEC-MED-028
#[test]
fn replays_a_zlib_bomb() {
    let stream = [&[0x78, 0x01][..], &bomb(), &[0x01, 0x0E, 0x00, 0x01]].concat();
    replay(
        "zlib-bomb",
        &stream,
        &Outcome {
            zlib: failed(
                InflateError::LongerThanDeclared {
                    declared: 65_536,
                    offset: 0,
                },
                &cap_of_zeros(),
            ),
            deflate: failed(InflateError::Corrupt { offset: 5 }, &[]),
        },
    );
}
