//! Replays the committed WAV fuzz corpus through its harness on stable
//! Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/riff` has a test here that pins its exact bytes
//! and the exact outcome the harness reports for it. Commit a fuzzing
//! reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::num::NonZeroU16;
use std::path::PathBuf;

use gunmetal_core::formats::riff::{
    ByteRange, Extensible, RiffError, WavCodec, WavFile, WavForm, WavFormat,
};
use gunmetal_core::parse::ParseFault;
use gunmetal_core::values::{BitDepth, Channels, Field, SampleRate, ValueError};
use gunmetal_fuzz::riff::run;

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/riff")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 11] = [
    "data-past-the-end",
    "empty",
    "extensible-float",
    "fmt-after-data",
    "info-and-id3",
    "minimal-pcm",
    "missing-pad-at-the-end",
    "not-wave",
    "rf64-with-ds64",
    "streaming-sizes",
    "zero-channels",
];

/// The `fmt ` chunk of 8-bit mono PCM at 8 kHz, as ffmpeg writes it.
const MONO_FORMAT: &[u8] =
    b"fmt \x10\x00\x00\x00\x01\x00\x01\x00\x40\x1f\x00\x00\x40\x1f\x00\x00\x01\x00\x08\x00";

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Result<WavFile, RiffError>) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

fn range(offset: u64, len: u64) -> ByteRange {
    ByteRange { offset, len }
}

fn depth(bits: u32) -> Option<BitDepth> {
    BitDepth::new(bits).ok()
}

/// The format of [`MONO_FORMAT`].
fn mono_8_bit() -> WavFormat {
    WavFormat {
        codec: WavCodec::Pcm,
        channels: Channels::new(1).expect("one channel"),
        sample_rate: SampleRate::new(8_000).expect("8 kHz"),
        bytes_per_second: 8_000,
        block_align: NonZeroU16::new(1),
        bits_per_sample: depth(8),
        extensible: None,
    }
}

/// A RIFF file of `format` and `data`, found whole.
fn found(format: WavFormat, data: ByteRange) -> WavFile {
    WavFile {
        form: WavForm::Riff,
        format,
        data,
        info: None,
        id3: None,
        stopped: None,
    }
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
        &Err(RiffError::Fault(ParseFault::Truncated {
            offset: 0,
            needed: 12,
            available: 0,
        })),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_minimal_pcm_file() {
    replay(
        "minimal-pcm",
        &[
            &b"RIFF\x28\x00\x00\x00WAVE"[..],
            MONO_FORMAT,
            b"data\x03\x00\x00\x00\x80\x81\x7f\x00",
        ]
        .concat(),
        &Ok(found(mono_8_bit(), range(44, 3))),
    );
}

/// Verifies: SEC-MED-028, SEC-MED-008
#[test]
fn replays_a_file_without_its_last_pad_octet() {
    replay(
        "missing-pad-at-the-end",
        &[
            &b"RIFF\x27\x00\x00\x00WAVE"[..],
            MONO_FORMAT,
            b"data\x03\x00\x00\x00\x80\x81\x7f",
        ]
        .concat(),
        &Ok(found(mono_8_bit(), range(44, 3))),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_the_sizes_a_streaming_writer_leaves() {
    replay(
        "streaming-sizes",
        &[
            &b"RIFF\xff\xff\xff\xffWAVE"[..],
            MONO_FORMAT,
            b"data\xff\xff\xff\xff\x80\x81\x82\x83\x84",
        ]
        .concat(),
        &Ok(found(mono_8_bit(), range(44, 5))),
    );
}

/// Verifies: SEC-MED-028, SEC-MED-004
#[test]
fn replays_an_rf64_file() {
    replay(
        "rf64-with-ds64",
        &[
            &b"RF64\xff\xff\xff\xffWAVE"[..],
            b"ds64\x1c\x00\x00\x00",
            b"\x4e\x00\x00\x00\x00\x00\x00\x00", // the form's size
            b"\x06\x00\x00\x00\x00\x00\x00\x00", // the data's size
            b"\x06\x00\x00\x00\x00\x00\x00\x00", // the sample count
            b"\x00\x00\x00\x00",                 // no table
            MONO_FORMAT,
            b"data\xff\xff\xff\xff\x01\x02\x03\x04\x05\x06",
        ]
        .concat(),
        &Ok(WavFile {
            form: WavForm::Rf64,
            ..found(mono_8_bit(), range(80, 6))
        }),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_format_chunk_after_the_samples() {
    replay(
        "fmt-after-data",
        &[
            &b"RIFF\x26\x00\x00\x00WAVE"[..],
            b"data\x02\x00\x00\x00\x01\x02",
            b"fmt \x10\x00\x00\x00\x01\x00\x02\x00\x44\xac\x00\x00\x10\xb1\x02\x00\x04\x00\x10\x00",
        ]
        .concat(),
        &Ok(found(
            WavFormat {
                codec: WavCodec::Pcm,
                channels: Channels::new(2).expect("two channels"),
                sample_rate: SampleRate::new(44_100).expect("44.1 kHz"),
                bytes_per_second: 176_400,
                block_align: NonZeroU16::new(4),
                bits_per_sample: depth(16),
                extensible: None,
            },
            range(20, 2),
        )),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_extensible_float_samples() {
    let float = *b"\x03\x00\x00\x00\x00\x00\x10\x00\x80\x00\x00\xaa\x00\x38\x9b\x71";
    replay(
        "extensible-float",
        &[
            &b"RIFF\x44\x00\x00\x00WAVE"[..],
            b"fmt \x28\x00\x00\x00\xfe\xff\x02\x00\x80\xbb\x00\x00\x00\xdc\x05\x00\x08\x00\x20\x00",
            b"\x16\x00\x20\x00\x03\x00\x00\x00",
            &float,
            b"data\x08\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",
        ]
        .concat(),
        &Ok(found(
            WavFormat {
                codec: WavCodec::Float,
                channels: Channels::new(2).expect("two channels"),
                sample_rate: SampleRate::new(48_000).expect("48 kHz"),
                bytes_per_second: 384_000,
                block_align: NonZeroU16::new(8),
                bits_per_sample: depth(32),
                extensible: Some(Extensible {
                    valid_bits: depth(32),
                    channel_mask: 0x3,
                    sub_format: float,
                }),
            },
            range(68, 8),
        )),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_file_with_an_info_list_and_an_id3_tag() {
    replay(
        "info-and-id3",
        &[
            &b"RIFF\x48\x00\x00\x00WAVE"[..],
            b"LIST\x0e\x00\x00\x00INFOINAM\x02\x00\x00\x00A\x00",
            b"id3 \x04\x00\x00\x00ID3\x04",
            MONO_FORMAT,
            b"data\x01\x00\x00\x00\x80\x00",
        ]
        .concat(),
        &Ok(WavFile {
            info: Some(range(24, 10)),
            id3: Some(range(42, 4)),
            ..found(mono_8_bit(), range(78, 1))
        }),
    );
}

/// Verifies: SEC-MED-028, SEC-TM-032
#[test]
fn replays_samples_that_run_past_the_end_of_the_file() {
    replay(
        "data-past-the-end",
        &[
            &b"RIFF\x8c\x00\x00\x00WAVE"[..],
            MONO_FORMAT,
            b"data\x64\x00\x00\x00\x00\x00\x00\x00\x00\x00",
        ]
        .concat(),
        &Ok(WavFile {
            stopped: Some(ParseFault::Truncated {
                offset: 44,
                needed: 100,
                available: 6,
            }),
            ..found(mono_8_bit(), range(44, 6))
        }),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_riff_form_that_is_not_wave() {
    replay(
        "not-wave",
        b"RIFF\x04\x00\x00\x00AVI ",
        &Err(RiffError::NotWave { found: *b"AVI " }),
    );
}

/// Verifies: SEC-MED-028, SEC-MED-014
#[test]
fn replays_a_format_of_no_channels() {
    replay(
        "zero-channels",
        b"RIFF\x1c\x00\x00\x00WAVEfmt \x10\x00\x00\x00\x01\x00\x00\x00\x40\x1f\x00\x00\x40\x1f\x00\x00\x01\x00\x08\x00",
        &Err(RiffError::Value {
            offset: 12,
            error: ValueError::OutOfRange {
                field: Field::Channels,
                value: 0,
            },
        }),
    );
}
