//! Replays the committed AIFF fuzz corpus through its harness on stable
//! Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/aiff` has a test here that pins its exact bytes
//! and the exact outcome the harness reports for it. Commit a fuzzing
//! reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::formats::aiff::{AiffError, AiffFile, AiffFormat};
use gunmetal_core::formats::riff::ByteRange;
use gunmetal_core::parse::ParseFault;
use gunmetal_core::values::{BitDepth, Channels, Field, SampleRate, ValueError};
use gunmetal_fuzz::aiff::run;

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/aiff")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 10] = [
    "aifc-sowt",
    "empty",
    "id3-chunk",
    "infinite-sample-rate",
    "minimal-aiff",
    "not-aiff",
    "sound-cut-inside-its-offset",
    "sound-cut-short",
    "sound-offset-past-the-chunk",
    "zero-sample-rate",
];

/// The `COMM` chunk of three 8-bit mono frames at 8 kHz, as ffmpeg writes
/// it.
const MONO_COMMON: &[u8] =
    b"COMM\x00\x00\x00\x12\x00\x01\x00\x00\x00\x03\x00\x08\x40\x0b\xfa\x00\x00\x00\x00\x00\x00\x00";

/// The `COMM` chunk of [`MONO_COMMON`] up to its sample rate.
const MONO_COMMON_BEFORE_RATE: &[u8] = b"COMM\x00\x00\x00\x12\x00\x01\x00\x00\x00\x03\x00\x08";

/// The `SSND` chunk of those three frames, with its pad octet.
const MONO_SOUND: &[u8] = b"SSND\x00\x00\x00\x0b\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01\xff\x00";

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Result<AiffFile, AiffError>) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

fn range(offset: u64, len: u64) -> ByteRange {
    ByteRange { offset, len }
}

/// The format of [`MONO_COMMON`].
fn mono_8_bit() -> AiffFormat {
    AiffFormat {
        compression: None,
        channels: Channels::new(1).expect("one channel"),
        sample_frames: 3,
        sample_size: BitDepth::new(8).ok(),
        sample_rate: SampleRate::new(8_000).expect("8 kHz"),
    }
}

/// An AIFF file of `format` and `sound`, found whole.
fn found(format: AiffFormat, sound: ByteRange) -> AiffFile {
    AiffFile {
        format,
        sound,
        block_size: 0,
        id3: None,
        stopped: None,
    }
}

/// The error for a `COMM` chunk at offset 12 whose sample rate is no rate.
fn bad_rate(error: ValueError) -> Result<AiffFile, AiffError> {
    Err(AiffError::Value { offset: 12, error })
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
        &Err(AiffError::Fault(ParseFault::Truncated {
            offset: 0,
            needed: 12,
            available: 0,
        })),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_minimal_aiff_file() {
    replay(
        "minimal-aiff",
        &[&b"FORM\x00\x00\x00\x32AIFF"[..], MONO_COMMON, MONO_SOUND].concat(),
        &Ok(found(mono_8_bit(), range(54, 3))),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_little_endian_samples_in_aiff_c() {
    replay(
        "aifc-sowt",
        &[
            &b"FORM\x00\x00\x00\x38AIFC"[..],
            b"COMM\x00\x00\x00\x18\x00\x02\x00\x00\x00\x01\x00\x10",
            b"\x40\x0e\xac\x44\x00\x00\x00\x00\x00\x00sowt\x00\x00",
            b"SSND\x00\x00\x00\x0c\x00\x00\x00\x00\x00\x00\x00\x00\x01\x02\x03\x04",
        ]
        .concat(),
        &Ok(found(
            AiffFormat {
                compression: Some(*b"sowt"),
                channels: Channels::new(2).expect("two channels"),
                sample_frames: 1,
                sample_size: BitDepth::new(16).ok(),
                sample_rate: SampleRate::new(44_100).expect("44.1 kHz"),
            },
            range(60, 4),
        )),
    );
}

/// Verifies: SEC-MED-028, SEC-MED-014
#[test]
fn replays_a_sample_rate_of_infinity() {
    replay(
        "infinite-sample-rate",
        &[
            &b"FORM\x00\x00\x00\x1eAIFF"[..],
            MONO_COMMON_BEFORE_RATE,
            b"\x7f\xff\x80\x00\x00\x00\x00\x00\x00\x00",
        ]
        .concat(),
        &bad_rate(ValueError::Unusable {
            field: Field::SampleRate,
        }),
    );
}

/// Verifies: SEC-MED-028, SEC-MED-014
#[test]
fn replays_a_sample_rate_of_zero() {
    replay(
        "zero-sample-rate",
        &[
            &b"FORM\x00\x00\x00\x1eAIFF"[..],
            MONO_COMMON_BEFORE_RATE,
            &[0; 10],
        ]
        .concat(),
        &bad_rate(ValueError::OutOfRange {
            field: Field::SampleRate,
            value: 0,
        }),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_file_with_an_id3_tag() {
    replay(
        "id3-chunk",
        &[
            &b"FORM\x00\x00\x00\x3eAIFF"[..],
            b"ID3 \x00\x00\x00\x04ID3\x04",
            MONO_COMMON,
            MONO_SOUND,
        ]
        .concat(),
        &Ok(AiffFile {
            id3: Some(range(20, 4)),
            ..found(mono_8_bit(), range(66, 3))
        }),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_samples_that_would_start_after_their_chunk() {
    replay(
        "sound-offset-past-the-chunk",
        &[
            &b"FORM\x00\x00\x00\x2eAIFF"[..],
            MONO_COMMON,
            b"SSND\x00\x00\x00\x08\x00\x00\x00\x04\x00\x00\x00\x00",
        ]
        .concat(),
        &Err(AiffError::SoundOffset {
            offset: 38,
            sound_offset: 4,
            len: 8,
        }),
    );
}

/// The reproducer of a finding: a sound chunk cut inside the gap its offset
/// leaves reported its empty samples at an offset past the end of the
/// file, which the harness refuses.
///
/// Verifies: SEC-MED-028, SEC-MED-030, SEC-TM-032
#[test]
fn replays_samples_cut_inside_their_offset() {
    replay(
        "sound-cut-inside-its-offset",
        &[
            &b"FORM\x00\x00\x00\x34AIFF"[..],
            MONO_COMMON,
            b"SSND\x00\x00\x00\x0e\x00\x00\x00\x04\x00\x00\x00\x00\x00\x00",
        ]
        .concat(),
        &Ok(AiffFile {
            stopped: Some(ParseFault::Truncated {
                offset: 46,
                needed: 14,
                available: 10,
            }),
            ..found(mono_8_bit(), range(56, 0))
        }),
    );
}

/// Verifies: SEC-MED-028, SEC-TM-032
#[test]
fn replays_samples_cut_short() {
    replay(
        "sound-cut-short",
        &[
            &b"FORM\x00\x00\x00\x32AIFF"[..],
            MONO_COMMON,
            &MONO_SOUND[..17],
        ]
        .concat(),
        &Ok(AiffFile {
            stopped: Some(ParseFault::Truncated {
                offset: 46,
                needed: 11,
                available: 9,
            }),
            ..found(mono_8_bit(), range(54, 1))
        }),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_an_iff_form_that_is_not_aiff() {
    replay(
        "not-aiff",
        b"FORM\x00\x00\x00\x048SVX",
        &Err(AiffError::NotAiff { found: *b"8SVX" }),
    );
}
