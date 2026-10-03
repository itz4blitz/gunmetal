//! Replays the committed corpus of the MP4 structure-aware harness on
//! stable Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/mp4_structure` has a test here that pins its
//! exact bytes and the exact outcome the harness reports for it. A seed
//! describes a track: the number of chunks, the size box (0 an `stsz` list,
//! 1 a constant size, 2 to 4 an `stz2` of 4, 8 and 16 bits), whether the
//! chunks lie past 4 GiB, a 16-bit timescale, the index-entry limit and the
//! constant size; for each chunk its gap and samples, each a 16-bit size
//! and delta; the octets after the last chunk; then which body to change,
//! where and to what. Missing octets read as zeros. The harness writes
//! `stts`, `stsc`, the sizes and the chunk offsets one after another from
//! offset 0.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::formats::mp4::sample_table::{SampleTableError, SeekIndex, SeekPoint};
use gunmetal_core::values::Duration;
use gunmetal_fuzz::mp4_sample_table::Outcome;
use gunmetal_fuzz::mp4_structure::run;

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/mp4_structure")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 10] = [
    "constant-sizes-past-4-gib",
    "eight-bit-sizes",
    "eight-bit-sizes-with-a-changed-chunk-offset",
    "eight-bit-sizes-with-a-changed-run",
    "empty",
    "four-bit-sizes-with-a-changed-chunk-offset",
    "four-bit-sizes-with-a-changed-delta",
    "listed-sizes",
    "listed-sizes-with-a-changed-size",
    "sixteen-bit-sizes-with-a-changed-chunk-offset",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
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

/// No chunks and no samples: empty tables and an empty index.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_the_empty_input() {
    replay("empty", &[], &Ok(index(&[], 1, 0)));
}

/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_listed_sizes() {
    replay(
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
    replay(
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
    replay(
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
    replay(
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
    replay(
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
    replay(
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
    replay(
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
    replay(
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
    replay(
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
