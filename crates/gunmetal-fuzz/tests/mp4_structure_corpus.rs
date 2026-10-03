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

use gunmetal_core::formats::mp4::sample_table::{SampleTableError, SeekIndex, SeekPoint};
use gunmetal_core::formats::mp4::{
    AudioEntry, AudioTrack, CodecConfig, FileType, FourCc, Mp4Audio, Mp4Error, Mp4Problem,
    SampleTableRanges,
};
use gunmetal_core::parse::{LimitKind, ParseFault};
use gunmetal_core::values::{BitDepth, Channels, Duration, SampleRate};
use gunmetal_fuzz::mp4_probe::Outcome;
use gunmetal_fuzz::mp4_sample_table::{self, Outcome as TableOutcome};
use gunmetal_fuzz::mp4_structure::run;

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/mp4_structure")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 21] = [
    "constant-sizes-past-4-gib",
    "eight-bit-sizes",
    "eight-bit-sizes-with-a-changed-chunk-offset",
    "eight-bit-sizes-with-a-changed-run",
    "empty",
    "empty-mdat-then-movie",
    "four-bit-sizes-with-a-changed-chunk-offset",
    "four-bit-sizes-with-a-changed-delta",
    "large-mdat",
    "large-mdat-empty",
    "listed-sizes",
    "listed-sizes-with-a-changed-size",
    "movie-after-mdat",
    "movie-before-mdat",
    "nested-trak-too-deep",
    "nested-udta",
    "nested-udta-too-deep",
    "open-mdat",
    "open-mdat-empty",
    "passthrough-garbage",
    "sixteen-bit-sizes-with-a-changed-chunk-offset",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

/// Reads seed `name` and checks the sample-table generator's outcome.
fn replay_tables(name: &str, bytes: &[u8], expected: &TableOutcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(
        &mp4_sample_table::structured(&file),
        expected,
        "seed {name}"
    );
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

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis).expect("in range")
}

/// An index of `points`, each a time in milliseconds and an offset.
fn index(points: &[(u64, u64)], stride: u64, duration: u64) -> SeekIndex {
    SeekIndex {
        points: points
            .iter()
            .map(|&(millis, offset)| SeekPoint {
                time: ms(millis),
                offset,
            })
            .collect(),
        stride,
        duration: ms(duration),
    }
}

/// Two chunks, the first 100 octets in, of samples of 10 and 20 octets and
/// then of 30, each 20 ticks long at 1,000 a second, sizes listed.
const LISTED: [u8; 23] = [
    2, 0, 0, // two chunks, an stsz list, under 4 GiB
    0x03, 0xE8, 10, 0, // 1,000 ticks a second, ten entries, no constant size
    100, 2, 0, 10, 0, 20, 0, 20, 0, 20, // 100 octets on, two samples
    0, 1, 0, 30, 0, 20, // straight after, one sample
];

/// No chunks and no samples: empty tables and an empty index.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_the_empty_sample_tables() {
    replay_tables("empty", &[], &Ok(index(&[], 1, 0)));
}

/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_listed_sizes() {
    replay_tables(
        "listed-sizes",
        &LISTED,
        &Ok(index(&[(0, 100), (20, 110), (40, 130)], 1, 60)),
    );
}

/// The `stsz` body's octet 15 is the low octet of the first size: 5, not
/// 10, so the second sample starts at 105.
///
/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_listed_sizes_with_a_changed_size() {
    replay_tables(
        "listed-sizes-with-a-changed-size",
        &[&LISTED[..], &[0, 3, 0, 15, 5]].concat(),
        &Ok(index(&[(0, 100), (20, 105), (40, 130)], 1, 60)),
    );
}

/// Four samples of 25 octets in a chunk at 4 GiB, 1,024 ticks apart at
/// 44,100 a second, with room for three index entries: the stride is two,
/// and 2,048 ticks is 46.4 ms.
///
/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_constant_sizes_past_4_gib() {
    replay_tables(
        "constant-sizes-past-4-gib",
        &[
            1, 1, 1, // one chunk, a constant size, past 4 GiB
            0xAC, 0x44, 3, 25, // 44,100 ticks a second, three entries, 25 octets
            0, 4, // no gap, four samples
            0, 0, 4, 0, 0, 0, 4, 0, 0, 0, 4, 0, 0, 0, 4, 0, // sizes unread, 1,024 ticks
        ],
        &Ok(index(&[(0, 0x1_0000_0000), (46, 0x1_0000_0032)], 2, 92)),
    );
}

/// Sizes of 1, 15 and 7 in 4-bit fields, from 0x11, 0xFF and 0x07 cut to
/// fit, in a chunk 8 octets in. The `stts` body's octet 23 is the low
/// octet of the second sample's delta: 5, not 10.
///
/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_four_bit_sizes_with_a_changed_delta() {
    replay_tables(
        "four-bit-sizes-with-a-changed-delta",
        &[
            1, 2, 0, // one chunk, 4-bit sizes, under 4 GiB
            0x03, 0xE8, 10, 0, // 1,000 ticks a second, ten entries
            8, 3, // 8 octets on, three samples
            0x00, 0x11, 0, 10, 0x00, 0xFF, 0, 10, 0x00, 0x07, 0, 10, //
            0, 1, 0, 23, 5, // change the stts body's octet 23 to 5
        ],
        &Ok(index(&[(0, 8), (10, 9), (15, 24)], 1, 25)),
    );
}

/// A size of 2, from 0x0102 cut to 8 bits, and one of 3, a tick apart at
/// one tick a second.
///
/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_eight_bit_sizes() {
    replay_tables(
        "eight-bit-sizes",
        &[
            1, 3, 0, // one chunk, 8-bit sizes, under 4 GiB
            0, 1, 10, 0, // one tick a second, ten entries
            0, 2, // no gap, two samples
            0x01, 0x02, 0, 1, 0x00, 0x03, 0, 1,
        ],
        &Ok(index(&[(0, 0), (1_000, 2)], 1, 2_000)),
    );
}

/// The `stsc` body, at 24 after the `stts` body, holds two runs; its octet
/// 23 is the low octet of the second run's first chunk: 1, the first run's.
///
/// Verifies: SEC-MED-028, SEC-MED-031, SEC-MED-008
#[test]
fn replays_eight_bit_sizes_with_a_changed_run() {
    replay_tables(
        "eight-bit-sizes-with-a-changed-run",
        &[
            2, 3, 0, // two chunks, 8-bit sizes, under 4 GiB
            0, 1, 10, 0, // one tick a second, ten entries
            0, 1, 0x01, 0x02, 0, 1, // no gap, one sample
            0, 1, 0x00, 0x03, 0, 1, // no gap, one sample
            0, 2, 0, 23, 1, // change the stsc body's octet 23 to 1
        ],
        &Err(SampleTableError::ChunkRun {
            run: 1,
            first_chunk: 1,
            chunks: 2,
            offset: 44,
        }),
    );
}

/// Sizes of 256 and 1 in 16-bit fields, in a chunk 10 octets into a file
/// of 270. The `stco` body, at 60, has its octet 11, the low octet of the
/// chunk's offset, changed to 255, so the chunk's 257 octets end at 512.
///
/// Verifies: SEC-MED-028, SEC-MED-031, SEC-MED-008
#[test]
fn replays_sixteen_bit_sizes_with_a_changed_chunk_offset() {
    replay_tables(
        "sixteen-bit-sizes-with-a-changed-chunk-offset",
        &[
            1, 4, 0, // one chunk, 16-bit sizes, under 4 GiB
            0, 1, 10, 0, // one tick a second, ten entries
            10, 2, // 10 octets on, two samples
            0x01, 0x00, 0, 1, 0x00, 0x01, 0, 1, //
            3, 4, 0, 11, 0xFF, // 3 octets after; change the stco body's octet 11
        ],
        &Err(SampleTableError::ChunkPastEnd {
            chunk: 1,
            offset: 68,
            start: 255,
            len: 257,
            file_len: 270,
        }),
    );
}

/// One chunk at the start of a file of 6 octets, of samples of 1, 2 and 3
/// octets a tick apart, with the low octet of its offset, octet 11 of the
/// `stco` body, changed to 255; `small_sizes` names the size box.
fn small_sizes(kind: u8) -> Vec<u8> {
    vec![
        1, kind, 0, // one chunk, the size box, under 4 GiB
        0, 1, 10, 0, // one tick a second, ten entries
        0, 3, // no gap, three samples
        0, 1, 0, 1, 0, 2, 0, 1, 0, 3, 0, 1, //
        0, 4, 0, 11, 0xFF, // no octets after; change the stco body's octet 11
    ]
}

/// The `stz2` body packs three 4-bit sizes into 2 octets after its 12, so
/// the `stco` body starts at 66 and the chunk's offset entry at 74.
///
/// Verifies: SEC-MED-028, SEC-MED-031, SEC-MED-008
#[test]
fn replays_four_bit_sizes_with_a_changed_chunk_offset() {
    replay_tables(
        "four-bit-sizes-with-a-changed-chunk-offset",
        &small_sizes(2),
        &Err(SampleTableError::ChunkPastEnd {
            chunk: 1,
            offset: 74,
            start: 255,
            len: 6,
            file_len: 6,
        }),
    );
}

/// The `stz2` body holds three 8-bit sizes in 3 octets after its 12, so
/// the `stco` body starts at 67 and the chunk's offset entry at 75.
///
/// Verifies: SEC-MED-028, SEC-MED-031, SEC-MED-008
#[test]
fn replays_eight_bit_sizes_with_a_changed_chunk_offset() {
    replay_tables(
        "eight-bit-sizes-with-a-changed-chunk-offset",
        &small_sizes(3),
        &Err(SampleTableError::ChunkPastEnd {
            chunk: 1,
            offset: 75,
            start: 255,
            len: 6,
            file_len: 6,
        }),
    );
}
