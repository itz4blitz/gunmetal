//! Replays the committed MP4 sample table fuzz corpus through its harness
//! on stable Rust, so that `cargo test` and the gate run every seed and
//! every reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/mp4_sample_table` has a test here that pins its
//! exact bytes and the exact outcome the harness reports for it. Commit a
//! fuzzing reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::formats::mp4::sample_table::{SampleTableError, SeekIndex, SeekPoint};
use gunmetal_core::parse::ParseFault;
use gunmetal_core::values::Duration;
use gunmetal_fuzz::mp4_sample_table::{Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/mp4_sample_table")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 6] = [
    "chunk-past-the-end",
    "compact-sizes-and-large-offsets-thinned",
    "cut-short",
    "empty",
    "minimal-track",
    "stagefright-stsc-count",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

fn point(millis: u64, offset: u64) -> SeekPoint {
    SeekPoint {
        time: Duration::from_millis(millis).expect("in range"),
        offset,
    }
}

/// The bodies of a track of three samples of 10, 20 and 30 octets, 20 ticks
/// apart, in chunks at 100 and 130: `stts`, `stsc`, `stsz` and `stco`.
const MINIMAL_TABLES: [&[u8]; 4] = [
    &[
        0, 0, 0, 0, // version and flags
        0, 0, 0, 1, // one run
        0, 0, 0, 3, 0, 0, 0, 20, // three samples of 20 ticks
    ],
    &[
        0, 0, 0, 0, // version and flags
        0, 0, 0, 2, // two runs
        0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 1, // from chunk 1, two samples each
        0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 1, // from chunk 2, one sample each
    ],
    &[
        0, 0, 0, 0, // version and flags
        0, 0, 0, 0, // sizes listed
        0, 0, 0, 3, // three samples
        0, 0, 0, 10, 0, 0, 0, 20, 0, 0, 0, 30,
    ],
    &[
        0, 0, 0, 0, // version and flags
        0, 0, 0, 2, // two chunks
        0, 0, 0, 100, 0, 0, 0, 130,
    ],
];

/// The settings for `MINIMAL_TABLES`, in a file of `file_len` octets.
fn minimal(file_len: u8) -> Vec<u8> {
    [
        &[
            0, // stsz and stco
            0x03, 0xE8, // 1,000 ticks a second
            10,   // ten index entries
            0, 0, 0, file_len, // the file's length
            16, 32, 24, // the stts, stsc and stsz lengths
        ][..],
        MINIMAL_TABLES[0],
        MINIMAL_TABLES[1],
        MINIMAL_TABLES[2],
        MINIMAL_TABLES[3],
    ]
    .concat()
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

/// The settings read as zeros, so the `stts` body is empty at offset 11.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_the_empty_input() {
    replay(
        "empty",
        &[],
        &Err(SampleTableError::Fault(ParseFault::Truncated {
            offset: 11,
            needed: 1,
            available: 0,
        })),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_minimal_track() {
    replay(
        "minimal-track",
        &minimal(160),
        &Ok(SeekIndex {
            points: vec![point(0, 100), point(20, 110), point(40, 130)],
            stride: 1,
            duration: Duration::from_millis(60).expect("in range"),
        }),
    );
}

/// The second chunk's 30 octets end at 160, one past the file. Its offset
/// entry is the second of the `stco` body, which starts at 83.
///
/// Verifies: SEC-MED-028, SEC-MED-008
#[test]
fn replays_a_chunk_past_the_end_of_the_file() {
    replay(
        "chunk-past-the-end",
        &minimal(159),
        &Err(SampleTableError::ChunkPastEnd {
            chunk: 2,
            offset: 95,
            start: 130,
            len: 30,
            file_len: 159,
        }),
    );
}

/// Two samples of 16 octets, one tick apart at one tick a second, in an
/// 8-bit `stz2` and one chunk at 100 in a `co64`, with room for one index
/// entry: the stride is two, so only the first sample is a point.
///
/// Verifies: SEC-MED-028, SEC-MED-006
#[test]
fn replays_compact_sizes_and_large_offsets_thinned_to_one_entry() {
    replay(
        "compact-sizes-and-large-offsets-thinned",
        &[
            &[
                3, // stz2 and co64
                0, 0, // a timescale of 0, which counts as 1
                1, // one index entry
                0, 0, 0, 200, // the file's length
                16, 20, 14, // the stts, stsc and stz2 lengths
            ][..],
            &[0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 1], // two samples of 1 tick
            &[0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 1], // two per chunk
            &[0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 2, 16, 16],     // 8-bit sizes of 16
            &[0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 100], // one chunk at 100
        ]
        .concat(),
        &Ok(SeekIndex {
            points: vec![point(0, 100)],
            stride: 2,
            duration: Duration::from_millis(2_000).expect("in range"),
        }),
    );
}

/// The pattern of Stagefright's CVE-2015-1538, rebuilt: an `stsc` count of
/// 0x15555556 runs of 12 octets is 2^32 + 8 octets, which a 32-bit product
/// wraps to the 8 octets the body holds after the count at 27.
///
/// Verifies: SEC-MED-028, SEC-HIS-036, SEC-TM-032
#[test]
fn replays_the_stagefright_stsc_count() {
    replay(
        "stagefright-stsc-count",
        &[
            &[0, 0, 0, 0, 0, 0, 0, 0, 8, 16, 0][..], // an 8-octet stts, a 16-octet stsc
            &[0, 0, 0, 0, 0, 0, 0, 0],               // an stts of no runs
            &[0, 0, 0, 0, 0x15, 0x55, 0x55, 0x56],   // the stsc count
            &[1, 1, 1, 1, 1, 1, 1, 1],
        ]
        .concat(),
        &Err(SampleTableError::Fault(ParseFault::Truncated {
            offset: 27,
            needed: 0x1_0000_0008,
            available: 8,
        })),
    );
}

/// The settings ask for an `stts` of 200 octets and five follow, so the
/// count at 15 has one of its four.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_body_cut_short_by_the_end_of_the_input() {
    replay(
        "cut-short",
        &[0, 0, 0, 0, 0, 0, 0, 0, 200, 0, 0, 0, 0, 0, 0, 0],
        &Err(SampleTableError::Fault(ParseFault::Truncated {
            offset: 15,
            needed: 4,
            available: 1,
        })),
    );
}
