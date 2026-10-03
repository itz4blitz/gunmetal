//! Replays the committed structure-aware MP4 fuzz corpus through its
//! harness on stable Rust, so that `cargo test` and the gate run every seed
//! and every reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/mp4_structure` has a test here that pins its
//! exact bytes and the exact outcome the harness reports for it. Commit a
//! fuzzing reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::num::NonZeroU32;
use std::path::PathBuf;

use gunmetal_core::formats::mp4::{
    AudioEntry, AudioTrack, CodecConfig, FileType, FourCc, Mp4Audio, Mp4Error, Mp4Problem,
    SampleTableRanges,
};
use gunmetal_core::parse::{LimitKind, ParseFault};
use gunmetal_core::values::{BitDepth, Channels, SampleRate};
use gunmetal_fuzz::mp4_probe::Outcome;
use gunmetal_fuzz::mp4_structure::run;

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/mp4_structure")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 12] = [
    "empty",
    "empty-mdat-then-movie",
    "large-mdat",
    "large-mdat-empty",
    "movie-after-mdat",
    "movie-before-mdat",
    "nested-trak-too-deep",
    "nested-udta",
    "nested-udta-too-deep",
    "open-mdat",
    "open-mdat-empty",
    "passthrough-garbage",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

fn truncated(offset: u64, needed: u64, available: u64) -> Outcome {
    Outcome::Error(Mp4Error::Fault(ParseFault::Truncated {
        offset,
        needed,
        available,
    }))
}

fn too_deep(offset: u64) -> ParseFault {
    ParseFault::TooDeep {
        limit: LimitKind::ContainerDepth,
        depth: 33,
        max: 32,
        offset,
    }
}

fn m4a() -> FileType {
    FileType {
        major: FourCc(*b"M4A "),
        minor: 512,
        compatible: vec![FourCc(*b"M4A "), FourCc(*b"mp42"), FourCc(*b"isom")],
    }
}

fn stereo_track(offset: u64) -> AudioTrack {
    AudioTrack {
        offset,
        timescale: NonZeroU32::new(44_100).expect("non-zero"),
        duration: Some(441_000),
        entry: AudioEntry {
            format: FourCc(*b"mp4a"),
            channels: Channels::new(2).ok(),
            bits: BitDepth::new(16).ok(),
            sample_rate: SampleRate::new(44_100).ok(),
            config: CodecConfig::Missing,
        },
    }
}

fn audio(track_offset: u64, problems: Vec<Mp4Problem>) -> Outcome {
    Outcome::Audio(Box::new(Mp4Audio {
        file_type: Some(m4a()),
        track: stereo_track(track_offset),
        items: vec![],
        sample_tables: SampleTableRanges::default(),
        problems,
    }))
}

/// Verifies: SEC-MED-028, SEC-MED-030, SEC-MED-031
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

/// Verifies: SEC-HIS-036, SEC-MED-027, SEC-MED-028, SEC-MED-031
#[test]
fn replays_the_empty_input() {
    replay(
        "empty",
        &[],
        &Outcome::Error(Mp4Error::NoMovie { offset: 0 }),
    );
}

/// Selector 0 copies the rest as the file. Eight 0xFF octets are a size of
/// `u32::MAX` with no body.
///
/// Verifies: SEC-HIS-036, SEC-MED-001, SEC-MED-027, SEC-MED-028, SEC-MED-031
#[test]
fn replays_passthrough_garbage() {
    replay(
        "passthrough-garbage",
        &[0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
        &truncated(8, 4_294_967_287, 0),
    );
}

/// Selector 1: a movie, then a 32-bit media-data box. The track starts at
/// 36.
///
/// Verifies: SEC-HIS-036, SEC-MED-010, SEC-MED-027, SEC-MED-028, SEC-MED-031
#[test]
fn replays_a_generated_movie_before_the_media_data() {
    replay("movie-before-mdat", &[1], &audio(36, vec![]));
}

/// Selector 2 with no payload: an empty media-data box, then the movie.
/// The track starts at 44.
///
/// Verifies: SEC-HIS-036, SEC-MED-010, SEC-MED-027, SEC-MED-028, SEC-MED-031
#[test]
fn replays_a_generated_empty_media_data_then_the_movie() {
    replay("empty-mdat-then-movie", &[2], &audio(44, vec![]));
}

/// Selector 2 with two payload octets: they become a truncated child of
/// the sample entry, and the movie fails.
///
/// Verifies: SEC-HIS-036, SEC-MED-001, SEC-MED-027, SEC-MED-028, SEC-MED-031
#[test]
fn replays_a_generated_movie_after_the_media_data_with_a_truncated_child() {
    replay("movie-after-mdat", &[2, 0x11, 0x22], &truncated(195, 4, 1));
}

/// Selector 3 with depth 30 of `udta` stays inside the depth limit.
///
/// Verifies: SEC-HIS-036, SEC-MED-005, SEC-MED-027, SEC-MED-028, SEC-MED-031
#[test]
fn replays_nested_user_data_inside_the_depth_limit() {
    replay("nested-udta", &[3, 61], &audio(36, vec![]));
}

/// Nested `trak` boxes 31 levels past the movie fail the file at depth 33.
///
/// Verifies: SEC-HIS-036, SEC-MED-005, SEC-TM-032, SEC-MED-027, SEC-MED-028, SEC-MED-031
#[test]
fn replays_nested_tracks_past_the_depth_limit() {
    replay(
        "nested-trak-too-deep",
        &[3, 62],
        &Outcome::Error(Mp4Error::Fault(too_deep(433))),
    );
}

/// Nested `udta` boxes past the depth limit skip the user data and keep
/// the track.
///
/// Verifies: SEC-HIS-036, SEC-MED-005, SEC-MED-017, SEC-MED-027, SEC-MED-028, SEC-MED-031
#[test]
fn replays_nested_user_data_past_the_depth_limit() {
    replay(
        "nested-udta-too-deep",
        &[3, 63],
        &audio(
            36,
            vec![Mp4Problem::Metadata(Mp4Error::Fault(too_deep(433)))],
        ),
    );
}

/// Selector 4 with two payload octets: a size-0 media-data box last, and a
/// truncated child of the sample entry.
///
/// Verifies: SEC-HIS-036, SEC-MED-001, SEC-MED-008, SEC-MED-027, SEC-MED-028, SEC-MED-031
#[test]
fn replays_an_open_media_data_box_with_a_truncated_child() {
    replay("open-mdat", &[4, 0xAA, 0xBB], &truncated(185, 4, 1));
}

/// Selector 4 with no payload: a size-0 media-data box last. The track
/// starts at 36, as with a 32-bit empty media-data box after the movie.
///
/// Verifies: SEC-HIS-036, SEC-MED-008, SEC-MED-027, SEC-MED-028, SEC-MED-031
#[test]
fn replays_an_open_empty_media_data_box() {
    replay("open-mdat-empty", &[4], &audio(36, vec![]));
}

/// Selector 5 with one payload octet: a 64-bit media-data box first, and a
/// truncated item-list child recorded as a problem.
///
/// Verifies: SEC-HIS-036, SEC-MED-008, SEC-MED-017, SEC-MED-027, SEC-MED-028, SEC-MED-031
#[test]
fn replays_a_64_bit_media_data_box_with_a_truncated_item() {
    replay(
        "large-mdat",
        &[5, 0xCC],
        &audio(
            53,
            vec![Mp4Problem::Metadata(Mp4Error::Fault(
                ParseFault::Truncated {
                    offset: 263,
                    needed: 4,
                    available: 1,
                },
            ))],
        ),
    );
}

/// Selector 5 with no payload: a 64-bit empty media-data box first. The
/// track starts at 52.
///
/// Verifies: SEC-HIS-036, SEC-MED-008, SEC-MED-010, SEC-MED-027, SEC-MED-028, SEC-MED-031
#[test]
fn replays_a_64_bit_empty_media_data_box() {
    replay("large-mdat-empty", &[5], &audio(52, vec![]));
}
