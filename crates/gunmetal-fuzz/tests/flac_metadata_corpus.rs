//! Replays the committed FLAC metadata fuzz corpus through its harness on
//! stable Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/flac_metadata` has a test here that pins its
//! exact bytes, written with the testkit's FLAC builder, and the exact
//! outcome the harness reports for it. Commit a fuzzing reproducer by
//! adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::num::{NonZeroU32, NonZeroU64};
use std::path::PathBuf;

use gunmetal_core::formats::flac::metadata::{
    AudioMd5, BlockProblem, BlockType, FlacError, FlacMetadata, PictureRef, RawBlock, SeekPoint,
    StreamInfo,
};
use gunmetal_core::parse::ParseFault;
use gunmetal_core::text::Text;
use gunmetal_core::values::{BitDepth, Channels, SampleRate};
use gunmetal_fuzz::flac_metadata::{Outcome, run};
use gunmetal_testkit::bytes::Bytes;
use gunmetal_testkit::flac::{self as build, Block};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/flac_metadata")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 8] = [
    "block-past-the-end",
    "empty",
    "every-kind",
    "last-flag-never-set",
    "not-flac",
    "rfc9639-example-1",
    "rfc9639-example-2",
    "seek-table-length",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

/// The frame of RFC 9639's first example (Appendix D.1).
const FRAME: [u8; 15] = [
    0xFF, 0xF8, 0x69, 0x18, 0x00, 0x00, 0xBF, 0x03, 0x58, 0xFD, 0x03, 0x12, 0x8B, 0xAA, 0x9A,
];

/// The two frames of RFC 9639's second example (Appendix D.2).
const EXAMPLE_2_FRAMES: [u8; 91] = [
    0xFF, 0xF8, 0x69, 0x98, 0x00, 0x0F, 0x99, 0x12, 0x08, 0x67, 0x01, 0x62, 0x3D, 0x14, 0x42, 0x99,
    0x8F, 0x5D, 0xF7, 0x0D, 0x6F, 0xE0, 0x0C, 0x17, 0xCA, 0xEB, 0x21, 0x00, 0x0E, 0xE7, 0xA7, 0x7A,
    0x24, 0xA1, 0x59, 0x0C, 0x12, 0x17, 0xB6, 0x03, 0x09, 0x7B, 0x78, 0x4F, 0xAA, 0x9A, 0x33, 0xD2,
    0x85, 0xE0, 0x70, 0xAD, 0x5B, 0x1B, 0x48, 0x51, 0xB4, 0x01, 0x0D, 0x99, 0xD2, 0xCD, 0x1A, 0x68,
    0xF1, 0xE6, 0xB8, 0x10, 0xFF, 0xF8, 0x69, 0x18, 0x01, 0x02, 0xA4, 0x02, 0xC3, 0x82, 0xC4, 0x0B,
    0xC1, 0x4A, 0x03, 0xEE, 0x48, 0xDD, 0x03, 0xB6, 0x7C, 0x13, 0x30,
];

/// The MD5 signature in RFC 9639's first example.
const EXAMPLE_1_MD5: [u8; 16] = [
    0x3E, 0x84, 0xB4, 0x18, 0x07, 0xDC, 0x69, 0x03, 0x07, 0x58, 0x6A, 0x3D, 0xAD, 0x1A, 0x2E, 0x0F,
];

/// The STREAMINFO of RFC 9639's first example, for the builder.
fn built_info() -> build::StreamInfo {
    build::StreamInfo {
        min_block_size: 4_096,
        max_block_size: 4_096,
        min_frame_size: 15,
        max_frame_size: 15,
        sample_rate: 44_100,
        channels: 2,
        bits_per_sample: 16,
        total_samples: 1,
        md5: EXAMPLE_1_MD5,
    }
}

/// The same STREAMINFO as the parser reads it.
fn example_info() -> StreamInfo {
    StreamInfo {
        min_block_size: 4_096,
        max_block_size: 4_096,
        min_frame_size: NonZeroU32::new(15),
        max_frame_size: NonZeroU32::new(15),
        sample_rate: SampleRate::new(44_100).unwrap(),
        channels: Channels::new(2).unwrap(),
        bits_per_sample: BitDepth::new(16).unwrap(),
        total_samples: NonZeroU64::new(1),
        md5: Some(AudioMd5(EXAMPLE_1_MD5)),
    }
}

/// The marker, then the first example's STREAMINFO, not flagged as the
/// last block.
fn unfinished() -> Bytes {
    let mut stream = Bytes::new();
    stream
        .bytes(&build::MARKER)
        .bytes(&Block::StreamInfo(built_info()).encode(false));
    stream
}

/// Metadata with the first example's STREAMINFO and nothing else, its
/// audio starting at `audio_start`.
fn metadata(audio_start: u64) -> FlacMetadata {
    FlacMetadata {
        stream_info: example_info(),
        seek_table: vec![],
        comment: None,
        pictures: vec![],
        raw: vec![],
        problems: vec![],
        audio_start,
    }
}

fn text(value: &str) -> Text {
    Text {
        value: value.to_owned(),
        truncated: false,
        replaced: false,
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
            parsed: Err(FlacError::Fault(ParseFault::Truncated {
                offset: 0,
                needed: 4,
                available: 0,
            })),
            steps: 0,
        },
    );
}

/// RFC 9639, Appendix D.1: one STREAMINFO block and one frame.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_the_first_example_of_rfc_9639() {
    let example = [
        build::stream(&[Block::StreamInfo(built_info())]),
        FRAME.to_vec(),
    ]
    .concat();
    replay(
        "rfc9639-example-1",
        &example,
        &Outcome {
            parsed: Ok(metadata(42)),
            steps: 1,
        },
    );
}

/// RFC 9639, Appendix D.2: STREAMINFO, a seek table of one point, a
/// Vorbis comment and padding, then two frames. Four headers and one seek
/// point are five steps.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_the_second_example_of_rfc_9639() {
    let md5 = [
        0xD5, 0xB0, 0x56, 0x49, 0x75, 0xE9, 0x8B, 0x8D, 0x8B, 0x93, 0x04, 0x22, 0x75, 0x7B, 0x81,
        0x03,
    ];
    let info = build::StreamInfo {
        min_block_size: 16,
        max_block_size: 16,
        min_frame_size: 23,
        max_frame_size: 68,
        total_samples: 19,
        md5,
        ..built_info()
    };
    let mut comment = Bytes::new();
    comment
        .u32_le(32)
        .bytes(b"reference libFLAC 1.3.3 20190804")
        .u32_le(1)
        .u32_le(14)
        .bytes("TITLE=\u{5E9}\u{5DC}\u{5D5}\u{5DD}".as_bytes());
    let example = [
        build::stream(&[
            Block::StreamInfo(info),
            Block::SeekTable(vec![build::SeekPoint {
                sample: 0,
                offset: 0,
                samples: 16,
            }]),
            Block::VorbisComment(comment.into_vec()),
            Block::Padding(6),
        ]),
        EXAMPLE_2_FRAMES.to_vec(),
    ]
    .concat();
    replay(
        "rfc9639-example-2",
        &example,
        &Outcome {
            parsed: Ok(FlacMetadata {
                stream_info: StreamInfo {
                    min_block_size: 16,
                    max_block_size: 16,
                    min_frame_size: NonZeroU32::new(23),
                    max_frame_size: NonZeroU32::new(68),
                    total_samples: NonZeroU64::new(19),
                    md5: Some(AudioMd5(md5)),
                    ..example_info()
                },
                seek_table: vec![SeekPoint {
                    sample: 0,
                    offset: 0,
                    samples: 16,
                }],
                comment: Some(68..126),
                raw: vec![RawBlock {
                    block_type: BlockType::Padding,
                    body: 130..136,
                }],
                ..metadata(136)
            }),
            steps: 5,
        },
    );
}

/// One block of every kind; the seek table ends in a placeholder. Eight
/// headers and three seek points are eleven steps.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_stream_with_every_kind_of_block() {
    let point = |sample, offset, samples| build::SeekPoint {
        sample,
        offset,
        samples,
    };
    let every_kind = [
        build::stream(&[
            Block::StreamInfo(built_info()),
            Block::SeekTable(vec![
                point(0, 0, 4_096),
                point(4_096, 1_000, 4_096),
                point(build::PLACEHOLDER, 0, 0),
            ]),
            Block::VorbisComment(vec![1, 2, 3, 4, 5]),
            Block::Picture(build::Picture {
                picture_type: 3,
                media_type: b"image/png".to_vec(),
                description: b"Cover".to_vec(),
                width: 1,
                height: 2,
                depth: 24,
                colors: 0,
                data: vec![0x89, b'P', b'N', b'G'],
            }),
            Block::Application {
                id: *b"xmcd",
                data: vec![1, 2],
            },
            Block::CueSheet(vec![0xCC, 0xCC, 0xCC]),
            Block::Raw {
                code: 77,
                body: vec![0xEE],
            },
            Block::Padding(2),
        ]),
        FRAME.to_vec(),
    ]
    .concat();
    replay(
        "every-kind",
        &every_kind,
        &Outcome {
            parsed: Ok(FlacMetadata {
                seek_table: vec![
                    SeekPoint {
                        sample: 0,
                        offset: 0,
                        samples: 4_096,
                    },
                    SeekPoint {
                        sample: 4_096,
                        offset: 1_000,
                        samples: 4_096,
                    },
                ],
                comment: Some(104..109),
                pictures: vec![PictureRef {
                    picture_type: 3,
                    media_type: text("image/png"),
                    description: text("Cover"),
                    width: 1,
                    height: 2,
                    depth: 24,
                    colors: 0,
                    data: 159..163,
                }],
                raw: vec![
                    RawBlock {
                        block_type: BlockType::Application,
                        body: 167..173,
                    },
                    RawBlock {
                        block_type: BlockType::CueSheet,
                        body: 177..180,
                    },
                    RawBlock {
                        block_type: BlockType::Reserved(77),
                        body: 184..185,
                    },
                    RawBlock {
                        block_type: BlockType::Padding,
                        body: 189..191,
                    },
                ],
                ..metadata(191)
            }),
            steps: 11,
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_an_ogg_page_as_not_flac() {
    replay(
        "not-flac",
        b"OggS\x00\x02",
        &Outcome {
            parsed: Err(FlacError::NotFlac {
                offset: 0,
                found: *b"OggS",
            }),
            steps: 0,
        },
    );
}

/// The frame after a chain whose last block is not flagged reads as a
/// header of the forbidden type 127.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_chain_whose_last_block_flag_is_never_set() {
    let mut stream = unfinished();
    stream.bytes(&FRAME[..4]);
    replay(
        "last-flag-never-set",
        stream.as_slice(),
        &Outcome {
            parsed: Err(FlacError::ForbiddenBlockType { offset: 42 }),
            steps: 2,
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_block_that_runs_past_the_end_of_the_file() {
    let mut stream = unfinished();
    stream.bytes(&build::header(true, 1, 100)).zeros(10);
    replay(
        "block-past-the-end",
        stream.as_slice(),
        &Outcome {
            parsed: Err(FlacError::Fault(ParseFault::Truncated {
                offset: 46,
                needed: 100,
                available: 10,
            })),
            steps: 2,
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_seek_table_whose_length_is_not_a_multiple_of_18() {
    let mut body = Bytes::new();
    body.zeros(19);
    let mut stream = unfinished();
    stream
        .bytes(
            &Block::Raw {
                code: 3,
                body: body.into_vec(),
            }
            .encode(true),
        )
        .bytes(&FRAME[..4]);
    replay(
        "seek-table-length",
        stream.as_slice(),
        &Outcome {
            parsed: Ok(FlacMetadata {
                problems: vec![BlockProblem::SeekTableLength {
                    block: 42,
                    length: 19,
                }],
                ..metadata(65)
            }),
            steps: 2,
        },
    );
}
