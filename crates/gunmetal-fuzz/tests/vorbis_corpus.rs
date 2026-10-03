//! Replays the committed Vorbis header fuzz corpus through its harness on
//! stable Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/vorbis` has a test here that pins its exact
//! bytes and the exact outcome the harness reports for it. Commit a fuzzing
//! reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::num::NonZeroU32;
use std::path::PathBuf;

use gunmetal_core::formats::vorbis::{VorbisError, VorbisIdent};
use gunmetal_core::parse::{LimitKind, ParseFault};
use gunmetal_core::values::{Channels, Field, SampleRate, ValueError};
use gunmetal_fuzz::vorbis::{Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/vorbis")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 9] = [
    "block-sizes-reversed",
    "comments-count-past-the-limit",
    "comments-framing-bit-unset",
    "comments-libvorbis",
    "empty",
    "framing-bit-unset",
    "libvorbis-stereo",
    "sample-rate-zero",
    "version-1",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome<'_>) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

/// The outcome for an identification header, which is not a comment
/// header.
fn ident_only(ident: Result<VorbisIdent, VorbisError>) -> Outcome<'static> {
    Outcome {
        ident,
        comments: Err(VorbisError::Magic {
            offset: 0,
            found: *b"\x01vorbis",
        }),
    }
}

/// The outcome for a comment header, which is not an identification
/// header.
fn comments_only(comments: Result<&[u8], VorbisError>) -> Outcome<'_> {
    Outcome {
        ident: Err(VorbisError::Magic {
            offset: 0,
            found: *b"\x03vorbis",
        }),
        comments,
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
    let truncated = VorbisError::Fault(ParseFault::Truncated {
        offset: 0,
        needed: 7,
        available: 0,
    });
    replay(
        "empty",
        &[],
        &Outcome {
            ident: Err(truncated),
            comments: Err(truncated),
        },
    );
}

/// libvorbis at its default quality: 44.1 kHz stereo, a nominal 112,000
/// bits per second, and blocks of 256 and 2,048 samples.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_the_stereo_header_libvorbis_writes() {
    replay(
        "libvorbis-stereo",
        b"\x01vorbis\x00\x00\x00\x00\x02\x44\xAC\x00\x00\x00\x00\x00\x00\x80\xB5\x01\x00\x00\x00\x00\x00\xB8\x01",
        &ident_only(Ok(VorbisIdent {
            channels: Channels::new(2).expect("two channels are in range"),
            sample_rate: SampleRate::new(44_100).expect("44.1 kHz is in range"),
            bitrate_maximum: None,
            bitrate_nominal: NonZeroU32::new(112_000),
            bitrate_minimum: None,
            short_block: 256,
            long_block: 2_048,
        })),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_an_identification_header_whose_framing_bit_is_unset() {
    replay(
        "framing-bit-unset",
        b"\x01vorbis\x00\x00\x00\x00\x02\x44\xAC\x00\x00\x00\x00\x00\x00\x80\xB5\x01\x00\x00\x00\x00\x00\xB8\x00",
        &ident_only(Err(VorbisError::Framing {
            offset: 29,
            octet: 0,
        })),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_vorbis_version_1() {
    replay(
        "version-1",
        b"\x01vorbis\x01\x00\x00\x00\x02\x44\xAC\x00\x00\x00\x00\x00\x00\x80\xB5\x01\x00\x00\x00\x00\x00\xB8\x01",
        &ident_only(Err(VorbisError::Version {
            offset: 7,
            version: 1,
        })),
    );
}

/// A short block of 2,048 samples and a long one of 256.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_short_block_longer_than_the_long_one() {
    replay(
        "block-sizes-reversed",
        b"\x01vorbis\x00\x00\x00\x00\x02\x44\xAC\x00\x00\x00\x00\x00\x00\x80\xB5\x01\x00\x00\x00\x00\x00\x8B\x01",
        &ident_only(Err(VorbisError::BlockSizes {
            offset: 28,
            octet: 0x8B,
        })),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_sample_rate_of_zero() {
    replay(
        "sample-rate-zero",
        b"\x01vorbis\x00\x00\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x00\x80\xB5\x01\x00\x00\x00\x00\x00\xB8\x01",
        &ident_only(Err(VorbisError::Value {
            offset: 12,
            error: ValueError::OutOfRange {
                field: Field::SampleRate,
                value: 0,
            },
        })),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_the_comment_header_libvorbis_writes() {
    let block: &[u8] =
        b"\x1D\x00\x00\x00Xiph.Org libVorbis I 20200704\x01\x00\x00\x00\x07\x00\x00\x00TITLE=A";
    replay(
        "comments-libvorbis",
        &[&b"\x03vorbis"[..], block, b"\x01"].concat(),
        &comments_only(Ok(block)),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_comment_header_whose_framing_bit_is_unset() {
    replay(
        "comments-framing-bit-unset",
        b"\x03vorbis\x04\x00\x00\x00Xiph\x00\x00\x00\x00\x00",
        &comments_only(Err(VorbisError::Framing {
            offset: 19,
            octet: 0,
        })),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_comment_count_of_u32_max() {
    replay(
        "comments-count-past-the-limit",
        b"\x03vorbis\x00\x00\x00\x00\xFF\xFF\xFF\xFF\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",
        &comments_only(Err(VorbisError::Fault(ParseFault::LimitExceeded {
            limit: LimitKind::TagFields,
            value: 4_294_967_295,
            max: 4_096,
            offset: 11,
        }))),
    );
}
