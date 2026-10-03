//! Replays the committed FLAC frame index fuzz corpus through its harness
//! on stable Rust, so that `cargo test` and the gate run every seed and
//! every reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/flac_frames` has a test here that pins its
//! exact bytes and the exact outcome the harness reports for it. Commit a
//! fuzzing reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::formats::flac::frames::{
    Blocking, FlacFrameError, FrameEntry, FrameIndex, HeaderProblem,
};
use gunmetal_core::parse::ParseFault;
use gunmetal_core::values::{BitDepth, Channels, SampleRate};
use gunmetal_fuzz::flac_frames::{Indexed, Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/flac_frames")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 8] = [
    "deferring-to-streaminfo",
    "empty",
    "false-sync-between-frames",
    "libflac-frame",
    "stream-marker-not-a-frame",
    "sync-codes-everywhere",
    "truncated-final-header",
    "variable-with-a-gap",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

/// The same result whether or not STREAMINFO is known, as for any stream
/// whose headers state their sample rate and depth.
fn either(indexed: Indexed) -> Outcome {
    Outcome {
        plain: indexed.clone(),
        cd: indexed,
    }
}

const fn entry(offset: u64, first_sample: u64) -> FrameEntry {
    FrameEntry {
        offset,
        first_sample,
    }
}

const fn ends_at(offset: u64) -> ParseFault {
    ParseFault::Truncated {
        offset,
        needed: 1,
        available: 0,
    }
}

/// The index of 4,096-sample frames of 16-bit stereo at 44.1 kHz.
fn cd_stream(entries: Vec<FrameEntry>, end_sample: u64, end: Option<ParseFault>) -> FrameIndex {
    FrameIndex {
        blocking: Blocking::Fixed,
        sample_rate: SampleRate::new(44_100).ok(),
        channels: Channels::new(2).unwrap(),
        bits: BitDepth::new(16).ok(),
        frames: u64::try_from(entries.len()).unwrap(),
        entries,
        stride: 1,
        gaps: 0,
        end_sample,
        end,
    }
}

/// The index of the one frame libFLAC 1.5.0 wrote for 64 samples of
/// 16-bit mono silence at 8 kHz.
fn libflac(end: Option<ParseFault>) -> FrameIndex {
    FrameIndex {
        blocking: Blocking::Fixed,
        sample_rate: SampleRate::new(8_000).ok(),
        channels: Channels::new(1).unwrap(),
        bits: BitDepth::new(16).ok(),
        entries: vec![entry(0, 0)],
        stride: 1,
        frames: 1,
        gaps: 0,
        end_sample: 64,
        end,
    }
}

const LIBFLAC_FRAME: [u8; 12] = [
    0xFF, 0xF8, 0x64, 0x08, 0x00, 0x3F, 0x5E, 0x00, 0x00, 0x00, 0xC6, 0x3C,
];

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

/// Verifies: SEC-MED-028, SEC-MED-001
#[test]
fn replays_the_empty_input() {
    replay(
        "empty",
        &[],
        &either(Err(FlacFrameError::Fault(ends_at(0)))),
    );
}

/// Verifies: SEC-MED-028, SEC-HIS-036
#[test]
fn replays_the_frame_libflac_wrote() {
    replay("libflac-frame", &LIBFLAC_FRAME, &either(Ok(libflac(None))));
}

/// A CD frame whose header leaves the sample rate and depth to
/// STREAMINFO.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_frame_that_defers_to_streaminfo() {
    let unknown = FrameIndex {
        sample_rate: None,
        bits: None,
        ..cd_stream(vec![entry(0, 0)], 4_096, None)
    };
    replay(
        "deferring-to-streaminfo",
        &[
            0xFF, 0xF8, 0xC0, 0x10, 0x00, 0x50, // header
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // two constant subframes
            0x70, 0x70, // CRC-16
        ],
        &Outcome {
            plain: Ok(unknown),
            cd: Ok(cd_stream(vec![entry(0, 0)], 4_096, None)),
        },
    );
}

/// Frame 0, a header for frame 1 whose CRC-8 is off by one, then the real
/// frame 1.
///
/// Verifies: SEC-MED-028, SEC-MED-001
#[test]
fn replays_a_false_sync_code_between_frames() {
    replay(
        "false-sync-between-frames",
        &[
            0xFF, 0xF8, 0xC9, 0x18, 0x00, 0xC2, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xB8, 0xEE,
            0xFF, 0xF8, 0xC9, 0x18, 0x01, 0xC4, // the false header
            0xFF, 0xF8, 0xC9, 0x18, 0x01, 0xC5, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2F, 0x9B,
        ],
        &either(Ok(cd_stream(
            vec![entry(0, 0), entry(20, 4_096)],
            8_192,
            None,
        ))),
    );
}

/// Two frames, then the first three octets of a third header.
///
/// Verifies: SEC-MED-028, SEC-MED-001, SEC-TM-032
#[test]
fn replays_a_truncated_final_header() {
    replay(
        "truncated-final-header",
        &[
            0xFF, 0xF8, 0xC9, 0x18, 0x00, 0xC2, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xB8, 0xEE,
            0xFF, 0xF8, 0xC9, 0x18, 0x01, 0xC5, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2F, 0x9B,
            0xFF, 0xF8, 0xC9,
        ],
        &either(Ok(cd_stream(
            vec![entry(0, 0), entry(14, 4_096)],
            8_192,
            Some(ends_at(31)),
        ))),
    );
}

/// Variable blocking: 100 samples from 0, 100 from 100, then 100 from
/// 1,000, after a gap.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_variable_blocking_with_a_gap() {
    replay(
        "variable-with-a-gap",
        &[
            0xFF, 0xF9, 0x79, 0x18, 0x00, 0x00, 0x63, 0x01, // header
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xCE, 0xA7, //
            0xFF, 0xF9, 0x79, 0x18, 0x64, 0x00, 0x63, 0x6F, // header
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xA1, 0xEA, //
            0xFF, 0xF9, 0x79, 0x18, 0xCF, 0xA8, 0x00, 0x63, 0x82, // header
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xE8, 0x39,
        ],
        &either(Ok(FrameIndex {
            blocking: Blocking::Variable,
            gaps: 1,
            ..cd_stream(
                vec![entry(0, 0), entry(16, 100), entry(32, 1_000)],
                1_100,
                None,
            )
        })),
    );
}

/// A FLAC file's stream marker, which is not a frame.
///
/// Verifies: SEC-MED-028, SEC-MED-001
#[test]
fn replays_the_stream_marker() {
    replay(
        "stream-marker-not-a-frame",
        b"fLaC",
        &either(Err(FlacFrameError::NotAFrame {
            offset: 0,
            problem: HeaderProblem::NoSync {
                octets: [0x66, 0x4C],
            },
        })),
    );
}

/// libFLAC's frame, then eight sync codes, each refused, the last cut
/// short by the end of the input.
///
/// Verifies: SEC-MED-028, SEC-MED-007
#[test]
fn replays_sync_codes_everywhere() {
    let mut bytes = LIBFLAC_FRAME.to_vec();
    bytes.extend([0xFF, 0xF8].repeat(8));
    replay(
        "sync-codes-everywhere",
        &bytes,
        &either(Ok(libflac(Some(ends_at(28))))),
    );
}
