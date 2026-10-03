//! Replays the committed corpus of the structure-aware FLAC harness on
//! stable Rust (SEC-MED-028, SEC-MED-031).
//!
//! Each file in `fuzz/seeds/flac_structure` is a recipe. Its test here pins
//! the recipe's bytes, the stream the harness writes from it, written
//! independently with the testkit's FLAC builder, and the exact outcome of
//! parsing that stream. Commit a fuzzing reproducer by adding its file and
//! its test together.
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
use gunmetal_core::text::Text;
use gunmetal_core::values::{BitDepth, Channels, Field, SampleRate, ValueError};
use gunmetal_fuzz::flac_metadata;
use gunmetal_fuzz::flac_structure::{Outcome, run};
use gunmetal_testkit::bytes::Bytes;
use gunmetal_testkit::flac::{self as build, Block};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/flac_structure")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 7] = [
    "empty",
    "every-kind",
    "more-blocks-than-a-recipe-writes",
    "picture-given-as-a-link",
    "second-streaminfo",
    "seek-points-out-of-order",
    "streaminfo-only",
];

/// Reads seed `name`, checks that it holds exactly `recipe`, and checks
/// that the harness writes `blocks`, flagging the last, then the start of a
/// frame, and reports `parsed` and `steps` for that stream.
fn replay(
    name: &str,
    recipe: &[u8],
    blocks: &[Block],
    parsed: Result<FlacMetadata, FlacError>,
    steps: u64,
) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, recipe, "seed {name} holds a different recipe");
    let stream = [build::stream(blocks), vec![0xFF, 0xF8]].concat();
    assert_eq!(
        run(&file),
        Outcome {
            stream,
            parsed: flac_metadata::Outcome { parsed, steps },
        },
        "seed {name}"
    );
}

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

/// A recipe that starts with the first example's STREAMINFO.
fn recipe() -> Bytes {
    let mut recipe = Bytes::new();
    recipe.bytes(&built_info().body());
    recipe
}

/// What the parser finds in a stream with the first example's STREAMINFO
/// and nothing else, its audio starting at `audio_start`.
fn metadata(audio_start: u64) -> FlacMetadata {
    FlacMetadata {
        stream_info: StreamInfo {
            min_block_size: 4_096,
            max_block_size: 4_096,
            min_frame_size: NonZeroU32::new(15),
            max_frame_size: NonZeroU32::new(15),
            sample_rate: SampleRate::new(44_100).unwrap(),
            channels: Channels::new(2).unwrap(),
            bits_per_sample: BitDepth::new(16).unwrap(),
            total_samples: NonZeroU64::new(1),
            md5: Some(AudioMd5(EXAMPLE_1_MD5)),
        },
        seek_table: vec![],
        comment: None,
        pictures: vec![],
        raw: vec![],
        problems: vec![],
        audio_start,
    }
}

fn point(sample: u64, offset: u64, samples: u16) -> build::SeekPoint {
    build::SeekPoint {
        sample,
        offset,
        samples,
    }
}

fn raw(block_type: BlockType, body: std::ops::Range<u64>) -> RawBlock {
    RawBlock { block_type, body }
}

fn padding(body: std::ops::Range<u64>) -> RawBlock {
    raw(BlockType::Padding, body)
}

fn text(value: &str) -> Text {
    Text {
        value: value.to_owned(),
        truncated: false,
        replaced: false,
    }
}

/// Two seek points in order.
fn two_points() -> Vec<build::SeekPoint> {
    vec![point(0, 0, 4_096), point(4_096, 1_000, 4_096)]
}

/// A PNG front cover of 1 by 2 pixels, with four octets of data.
fn cover() -> build::Picture {
    build::Picture {
        picture_type: 3,
        media_type: b"image/png".to_vec(),
        description: b"Cover".to_vec(),
        width: 1,
        height: 2,
        depth: 24,
        colors: 0,
        data: vec![0x89, b'P', b'N', b'G'],
    }
}

/// Verifies: SEC-MED-028, SEC-MED-031
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

/// An empty recipe is a STREAMINFO of zeros, whose sample rate of zero is
/// outside its typed range.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_the_empty_recipe() {
    let mut zeros = Bytes::new();
    zeros.zeros(34);
    replay(
        "empty",
        &[],
        &[Block::Raw {
            code: 0,
            body: zeros.into_vec(),
        }],
        Err(FlacError::StreamInfoValue {
            offset: 18,
            error: ValueError::OutOfRange {
                field: Field::SampleRate,
                value: 0,
            },
        }),
        1,
    );
}

/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_a_recipe_of_streaminfo_alone() {
    replay(
        "streaminfo-only",
        recipe().as_slice(),
        &[Block::StreamInfo(built_info())],
        Ok(metadata(42)),
        1,
    );
}

/// Every block type from one recipe. Its lengths sit where the mutations
/// of the recipe's arithmetic would change the stream: a seek table of 2
/// points (2 modulo 4 is neither 2 divided by 4 nor 2 plus 4), a media type
/// of 9 octets and a description of 5, and the reserved type 7 + 125
/// modulo 120, which is 12. Eight headers and two seek points are ten
/// steps.
///
/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_a_recipe_for_every_kind_of_block() {
    let mut every_kind = recipe();
    every_kind
        .bytes(&[1, 2, 0xAA, 0xBB])
        .bytes(&[3, 2])
        .bytes(&Block::SeekTable(two_points()).body())
        .bytes(&[4, 3, 1, 2, 3])
        .bytes(&[6, 0, 0, 0, 3, 9])
        .bytes(b"image/png")
        .u8(5)
        .bytes(b"Cover")
        .u32_be(1)
        .u32_be(2)
        .u32_be(24)
        .u32_be(0)
        .u8(4)
        .bytes(&[0x89, b'P', b'N', b'G'])
        .bytes(&[2, 6])
        .bytes(b"xmcd")
        .bytes(&[1, 2])
        .bytes(&[5, 1, 0xCC])
        .bytes(&[7, 125, 1, 0xEE]);
    let blocks = [
        Block::StreamInfo(built_info()),
        Block::Raw {
            code: 1,
            body: vec![0xAA, 0xBB],
        },
        Block::SeekTable(two_points()),
        Block::VorbisComment(vec![1, 2, 3]),
        Block::Picture(cover()),
        Block::Application {
            id: *b"xmcd",
            data: vec![1, 2],
        },
        Block::CueSheet(vec![0xCC]),
        Block::Raw {
            code: 12,
            body: vec![0xEE],
        },
    ];
    let found = FlacMetadata {
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
        comment: Some(92..95),
        pictures: vec![PictureRef {
            picture_type: 3,
            media_type: text("image/png"),
            description: text("Cover"),
            width: 1,
            height: 2,
            depth: 24,
            colors: 0,
            data: 145..149,
        }],
        raw: vec![
            padding(46..48),
            raw(BlockType::Application, 153..159),
            raw(BlockType::CueSheet, 163..164),
            raw(BlockType::Reserved(12), 168..169),
        ],
        ..metadata(169)
    };
    replay("every-kind", every_kind.as_slice(), &blocks, Ok(found), 10);
}

/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_a_recipe_with_a_second_streaminfo() {
    let mut second = recipe();
    second.u8(0).bytes(&built_info().body()).bytes(&[1, 0]);
    replay(
        "second-streaminfo",
        second.as_slice(),
        &[
            Block::StreamInfo(built_info()),
            Block::StreamInfo(built_info()),
            Block::Padding(0),
        ],
        Err(FlacError::SecondStreamInfo { offset: 42 }),
        2,
    );
}

/// The seek table's framing is valid and its points are not.
///
/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_a_recipe_with_seek_points_out_of_order() {
    let points = vec![point(10, 0, 1), point(5, 0, 1)];
    let mut falling = recipe();
    falling
        .bytes(&[3, 2])
        .bytes(&Block::SeekTable(points.clone()).body());
    replay(
        "seek-points-out-of-order",
        falling.as_slice(),
        &[Block::StreamInfo(built_info()), Block::SeekTable(points)],
        Ok(FlacMetadata {
            problems: vec![BlockProblem::SeekPointOrder {
                block: 42,
                point: 64,
            }],
            ..metadata(82)
        }),
        4,
    );
}

/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_a_recipe_with_a_picture_given_as_a_link() {
    let mut link = recipe();
    link.bytes(&[6, 0, 0, 0, 3, 3]).bytes(b"-->").zeros(18);
    replay(
        "picture-given-as-a-link",
        link.as_slice(),
        &[
            Block::StreamInfo(built_info()),
            Block::Picture(build::Picture {
                picture_type: 3,
                media_type: b"-->".to_vec(),
                description: vec![],
                width: 0,
                height: 0,
                depth: 0,
                colors: 0,
                data: vec![],
            }),
        ],
        Ok(FlacMetadata {
            problems: vec![BlockProblem::LinkedPicture { block: 42 }],
            ..metadata(81)
        }),
        2,
    );
}

/// A recipe for nine more blocks writes eight.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_recipe_for_more_blocks_than_one_writes() {
    let mut many = recipe();
    for _ in 0..9 {
        many.bytes(&[1, 0]);
    }
    let blocks: Vec<Block> = std::iter::once(Block::StreamInfo(built_info()))
        .chain((0..8).map(|_| Block::Padding(0)))
        .collect();
    replay(
        "more-blocks-than-a-recipe-writes",
        many.as_slice(),
        &blocks,
        Ok(FlacMetadata {
            raw: (0..8)
                .map(|index| padding(46 + 4 * index..46 + 4 * index))
                .collect(),
            ..metadata(74)
        }),
        9,
    );
}
