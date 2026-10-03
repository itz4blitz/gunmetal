//! Replays the committed structure-aware FLAC fuzz corpus through its
//! harness on stable Rust, so that `cargo test` and the gate run every
//! seed and every reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/flac_structure` has a test here that pins its
//! exact bytes, the exact stream the harness writes for it and the exact
//! index the frame index reports. Commit a fuzzing reproducer by adding its
//! file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::formats::flac::frames::{Blocking, FlacFrameError, FrameEntry, FrameIndex};
use gunmetal_core::parse::ParseFault;
use gunmetal_core::values::{BitDepth, Channels, SampleRate};
use gunmetal_fuzz::flac_frames::{self, Indexed};
use gunmetal_fuzz::flac_structure::{Outcome, RECIPE_LEN, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/flac_structure")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 6] = [
    "coded-number-widths",
    "empty",
    "end-of-header-fields",
    "filler-around-headers",
    "fixed-frame-numbers-masked",
    "short-of-a-recipe",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness writes `stream` for it and indexes it as `indexed`, whether
/// or not STREAMINFO is known.
fn replay(name: &str, bytes: &[u8], stream: &[u8], indexed: Indexed) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(
        run(&file),
        Outcome {
            stream: stream.to_vec(),
            indexed: flac_frames::Outcome {
                plain: indexed.clone(),
                cd: indexed,
            },
        },
        "seed {name}"
    );
}

/// One recipe, laid out as the harness documents.
fn recipe(
    variable: bool,
    data: u8,
    codes: u8,
    layout: u8,
    number: u64,
    block_size: u16,
    sample_rate: u16,
) -> Vec<u8> {
    let mut octets = vec![(data << 1) | u8::from(variable), codes, layout];
    octets.extend(&number.to_be_bytes()[3..]);
    octets.extend(block_size.to_be_bytes());
    octets.extend(sample_rate.to_be_bytes());
    assert_eq!(octets.len(), RECIPE_LEN);
    octets
}

const fn entry(offset: u64, first_sample: u64) -> FrameEntry {
    FrameEntry {
        offset,
        first_sample,
    }
}

/// A stream of 16-bit stereo at 44.1 kHz.
fn stereo(blocking: Blocking, entries: Vec<FrameEntry>, gaps: u64, end_sample: u64) -> FrameIndex {
    FrameIndex {
        blocking,
        sample_rate: SampleRate::new(44_100).ok(),
        channels: Channels::new(2).unwrap(),
        bits: BitDepth::new(16).ok(),
        frames: u64::try_from(entries.len()).unwrap(),
        entries,
        stride: 1,
        gaps,
        end_sample,
        end: None,
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

/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_the_empty_input() {
    replay(
        "empty",
        &[],
        &[],
        Err(FlacFrameError::Fault(ParseFault::Truncated {
            offset: 0,
            needed: 1,
            available: 0,
        })),
    );
}

/// One octet short of a recipe writes nothing.
///
/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_input_one_octet_short_of_a_recipe() {
    let mut bytes = recipe(false, 0, 0xC9, 0x18, 0, 0, 0);
    bytes.pop();
    replay(
        "short-of-a-recipe",
        &bytes,
        &[],
        Err(FlacFrameError::Fault(ParseFault::Truncated {
            offset: 0,
            needed: 1,
            available: 0,
        })),
    );
}

/// Sample numbers at both ends of every coded width, in frames of one
/// sample, the last given 40 bits of which the harness keeps 36.
///
/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_coded_numbers_of_every_width() {
    let numbers: [u64; 13] = [
        0x7F,
        0x80,
        0x7FF,
        0x800,
        0xFFFF,
        0x1_0000,
        0x1F_FFFF,
        0x20_0000,
        0x3FF_FFFF,
        0x400_0000,
        0x7FFF_FFFF,
        0x8000_0000,
        0xFF_FFFF_FFFF,
    ];
    let bytes: Vec<u8> = numbers
        .iter()
        .flat_map(|&number| recipe(true, 0, 0x69, 0x18, number, 0, 0))
        .collect();
    let stream: [&[u8]; 13] = [
        &[0xFF, 0xF9, 0x69, 0x18, 0x7F, 0x00, 0xBC],
        &[0xFF, 0xF9, 0x69, 0x18, 0xC2, 0x80, 0x00, 0xF0],
        &[0xFF, 0xF9, 0x69, 0x18, 0xDF, 0xBF, 0x00, 0xF9],
        &[0xFF, 0xF9, 0x69, 0x18, 0xE0, 0xA0, 0x80, 0x00, 0xC9],
        &[0xFF, 0xF9, 0x69, 0x18, 0xEF, 0xBF, 0xBF, 0x00, 0xC4],
        &[0xFF, 0xF9, 0x69, 0x18, 0xF0, 0x90, 0x80, 0x80, 0x00, 0x5C],
        &[0xFF, 0xF9, 0x69, 0x18, 0xF7, 0xBF, 0xBF, 0xBF, 0x00, 0xF5],
        &[
            0xFF, 0xF9, 0x69, 0x18, 0xF8, 0x88, 0x80, 0x80, 0x80, 0x00, 0x41,
        ],
        &[
            0xFF, 0xF9, 0x69, 0x18, 0xFB, 0xBF, 0xBF, 0xBF, 0xBF, 0x00, 0xA2,
        ],
        &[
            0xFF, 0xF9, 0x69, 0x18, 0xFC, 0x84, 0x80, 0x80, 0x80, 0x80, 0x00, 0xE8,
        ],
        &[
            0xFF, 0xF9, 0x69, 0x18, 0xFD, 0xBF, 0xBF, 0xBF, 0xBF, 0xBF, 0x00, 0x27,
        ],
        &[
            0xFF, 0xF9, 0x69, 0x18, 0xFE, 0x82, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00, 0xCA,
        ],
        &[
            0xFF, 0xF9, 0x69, 0x18, 0xFE, 0xBF, 0xBF, 0xBF, 0xBF, 0xBF, 0xBF, 0x00, 0x4C,
        ],
    ];
    let offsets = [0, 7, 15, 23, 32, 41, 51, 61, 72, 83, 95, 107, 120];
    let first_samples = [
        0x7F,
        0x80,
        0x7FF,
        0x800,
        0xFFFF,
        0x1_0000,
        0x1F_FFFF,
        0x20_0000,
        0x3FF_FFFF,
        0x400_0000,
        0x7FFF_FFFF,
        0x8000_0000,
        0xF_FFFF_FFFF,
    ];
    // Six numbers skip ahead of the one before plus its one sample.
    replay(
        "coded-number-widths",
        &bytes,
        &stream.concat(),
        Ok(stereo(
            Blocking::Variable,
            offsets
                .into_iter()
                .zip(first_samples)
                .map(|(offset, first)| entry(offset, first))
                .collect(),
            6,
            0x10_0000_0000,
        )),
    );
}

/// Four frames of 64 samples of 16-bit mono at 8 kHz, the rate written in
/// kilohertz, hertz, tens of hertz and as a common code.
///
/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_end_of_header_fields() {
    let bytes = [
        recipe(false, 0, 0x6C, 0x08, 0, 0x003F, 0x0008),
        recipe(false, 0, 0x7D, 0x08, 1, 0x003F, 0x1F40),
        recipe(false, 0, 0x6E, 0x08, 2, 0x003F, 0x0320),
        recipe(false, 0, 0x64, 0x08, 3, 0x003F, 0),
    ]
    .concat();
    let stream = [
        &[0xFF, 0xF8, 0x6C, 0x08, 0x00, 0x3F, 0x08, 0xBC][..],
        &[0xFF, 0xF8, 0x7D, 0x08, 0x01, 0x00, 0x3F, 0x1F, 0x40, 0x02],
        &[0xFF, 0xF8, 0x6E, 0x08, 0x02, 0x3F, 0x03, 0x20, 0x34],
        &[0xFF, 0xF8, 0x64, 0x08, 0x03, 0x3F, 0x61],
    ]
    .concat();
    replay(
        "end-of-header-fields",
        &bytes,
        &stream,
        Ok(FrameIndex {
            blocking: Blocking::Fixed,
            sample_rate: SampleRate::new(8_000).ok(),
            channels: Channels::new(1).unwrap(),
            bits: BitDepth::new(16).ok(),
            entries: vec![entry(0, 0), entry(8, 64), entry(18, 128), entry(27, 192)],
            stride: 1,
            frames: 4,
            gaps: 0,
            end_sample: 256,
            end: None,
        }),
    );
}

/// Frame numbers given 40 bits, of which the harness keeps 31.
///
/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_frame_numbers_kept_to_31_bits() {
    let bytes = [
        recipe(false, 0, 0xC9, 0x18, 0x00_8000_0000, 0, 0),
        recipe(false, 0, 0xC9, 0x18, 0xFF_8000_0001, 0, 0),
    ]
    .concat();
    replay(
        "fixed-frame-numbers-masked",
        &bytes,
        &[
            0xFF, 0xF8, 0xC9, 0x18, 0x00, 0xC2, // frame 0
            0xFF, 0xF8, 0xC9, 0x18, 0x01, 0xC5, // frame 1
        ],
        Ok(stereo(
            Blocking::Fixed,
            vec![entry(0, 0), entry(6, 4_096)],
            0,
            8_192,
        )),
    );
}

/// Frame 0 with six octets of data holding a false header for frame 7,
/// then frame 1 asking for 127 octets of data where five remain, holding
/// two more false sync codes.
///
/// Verifies: SEC-MED-028, SEC-MED-031, SEC-MED-001
#[test]
fn replays_data_around_headers() {
    let bytes = [
        recipe(false, 6, 0xC9, 0x18, 0, 0, 0),
        vec![0xFF, 0xF8, 0xC9, 0x18, 0x07, 0x00],
        recipe(false, 127, 0xC9, 0x18, 1, 0, 0),
        vec![0x00, 0xFF, 0xF8, 0xFF, 0x00],
    ]
    .concat();
    replay(
        "filler-around-headers",
        &bytes,
        &[
            0xFF, 0xF8, 0xC9, 0x18, 0x00, 0xC2, // frame 0
            0xFF, 0xF8, 0xC9, 0x18, 0x07, 0x00, // its data
            0xFF, 0xF8, 0xC9, 0x18, 0x01, 0xC5, // frame 1
            0x00, 0xFF, 0xF8, 0xFF, 0x00, // its data
        ],
        Ok(stereo(
            Blocking::Fixed,
            vec![entry(0, 0), entry(12, 4_096)],
            0,
            8_192,
        )),
    );
}
