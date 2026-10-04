//! Replays the committed MP4-probe fuzz corpus through its harness on
//! stable Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/mp4_probe` has a test here that pins its exact
//! bytes and the exact outcome the harness reports for it. Commit a fuzzing
//! reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::num::NonZeroU32;
use std::path::PathBuf;

use gunmetal_core::formats::mp4::{
    AudioEntry, AudioTrack, CodecConfig, FileType, FourCc, Mp4Audio, Mp4Error, SampleTableRanges,
};
use gunmetal_core::parse::ParseFault;
use gunmetal_core::values::{BitDepth, Channels, SampleRate};
use gunmetal_fuzz::mp4_probe::{Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/mp4_probe")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 7] = [
    "empty",
    "four-octets",
    "ftyp-only",
    "movie-after-mdat",
    "movie-before-mdat",
    "size-7-box",
    "zero-timescale",
];

/// The `ftyp` box shared by every non-empty seed: major `M4A `, minor 512,
/// compatible `M4A `, `mp42`, `isom`.
const FTYP: &[u8] = &[
    0, 0, 0, 28, b'f', b't', b'y', b'p', b'M', b'4', b'A', b' ', 0, 0, 2, 0, b'M', b'4', b'A',
    b' ', b'm', b'p', b'4', b'2', b'i', b's', b'o', b'm',
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

fn m4a() -> FileType {
    FileType {
        major: FourCc(*b"M4A "),
        minor: 512,
        compatible: vec![FourCc(*b"M4A "), FourCc(*b"mp42"), FourCc(*b"isom")],
    }
}

/// The stereo `mp4a` track the movie-before and movie-after seeds hold,
/// with no codec configuration box.
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

fn audio(track_offset: u64) -> Outcome {
    Outcome::Audio(Box::new(Mp4Audio {
        file_type: Some(m4a()),
        track: stereo_track(track_offset),
        items: vec![],
        sample_tables: SampleTableRanges::default(),
        problems: vec![],
    }))
}

/// Sixteen octets of media data.
const MDAT: &[u8] = &[
    0, 0, 0, 24, b'm', b'd', b'a', b't', 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA,
    0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA,
];

/// The movie box of the movie-before, movie-after and zero-timescale
/// seeds: one `soun` track and an empty item list.
const MOVIE: &[u8] = &[
    0, 0, 0, 218, b'm', b'o', b'o', b'v', 0, 0, 0, 149, b't', b'r', b'a', b'k', 0, 0, 0, 141, b'm',
    b'd', b'i', b'a', 0, 0, 0, 32, b'm', b'd', b'h', b'd', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 172, 68, 0, 6, 186, 168, 85, 196, 0, 0, 0, 0, 0, 33, b'h', b'd', b'l', b'r', 0, 0, 0, 0, 0,
    0, 0, 0, b's', b'o', b'u', b'n', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 68, b'm',
    b'i', b'n', b'f', 0, 0, 0, 60, b's', b't', b'b', b'l', 0, 0, 0, 52, b's', b't', b's', b'd', 0,
    0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 36, b'm', b'p', b'4', b'a', 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 2, 0, 16, 0, 0, 0, 0, 172, 68, 0, 0, 0, 0, 0, 61, b'u', b'd', b't', b'a', 0, 0,
    0, 53, b'm', b'e', b't', b'a', 0, 0, 0, 0, 0, 0, 0, 33, b'h', b'd', b'l', b'r', 0, 0, 0, 0, 0,
    0, 0, 0, b'm', b'd', b'i', b'r', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, b'i', b'l',
    b's', b't',
];

/// The movie of the zero-timescale seed: the same as [`MOVIE`], with the
/// timescale field zeroed.
fn zero_timescale_movie() -> Vec<u8> {
    let mut movie = MOVIE.to_vec();
    // mdhd body: version and flags at 32, timestamps at 36, timescale at 44.
    movie[44] = 0;
    movie[45] = 0;
    movie[46] = 0;
    movie[47] = 0;
    movie
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

/// Verifies: SEC-HIS-036, SEC-MED-027, SEC-MED-028
#[test]
fn replays_the_empty_input() {
    replay(
        "empty",
        &[],
        &Outcome::Error(Mp4Error::NoMovie { offset: 0 }),
    );
}

/// Four octets are a size with no type, so the header is truncated.
///
/// Verifies: SEC-HIS-036, SEC-MED-001, SEC-MED-027, SEC-MED-028
#[test]
fn replays_four_octets() {
    replay("four-octets", &[0, 0, 0, 16], &truncated(4, 4, 0));
}

/// A file type box alone is not a movie.
///
/// Verifies: SEC-HIS-036, SEC-MED-027, SEC-MED-028
#[test]
fn replays_a_file_type_box_with_no_movie() {
    replay(
        "ftyp-only",
        FTYP,
        &Outcome::Error(Mp4Error::NoMovie { offset: 28 }),
    );
}

/// A box of size 7 is smaller than its own header.
///
/// Verifies: SEC-HIS-036, SEC-MED-008, SEC-MED-027, SEC-MED-028
#[test]
fn replays_a_box_of_size_7() {
    let bytes = [FTYP, &[0, 0, 0, 7, b'f', b'r', b'e', b'e'][..]].concat();
    replay(
        "size-7-box",
        &bytes,
        &Outcome::Error(Mp4Error::BoxTooSmall {
            offset: 28,
            size: 7,
            header_len: 8,
        }),
    );
}

/// A movie with a zero timescale is refused at the field.
///
/// Verifies: SEC-HIS-036, SEC-MED-014, SEC-MED-027, SEC-MED-028
#[test]
fn replays_a_movie_with_a_zero_timescale() {
    let bytes = [FTYP, &zero_timescale_movie()].concat();
    replay(
        "zero-timescale",
        &bytes,
        &Outcome::Error(Mp4Error::ZeroTimescale { offset: 72 }),
    );
}

/// The movie comes first; the media data is never read. The track starts
/// eight octets into the movie, at 36.
///
/// Verifies: SEC-HIS-036, SEC-MED-010, SEC-MED-027, SEC-MED-028
#[test]
fn replays_a_movie_before_the_media_data() {
    let bytes = [FTYP, MOVIE, MDAT].concat();
    replay("movie-before-mdat", &bytes, &audio(36));
}

/// The media data comes first; the probe still finds the movie by offset.
/// The track starts at 60.
///
/// Verifies: SEC-HIS-036, SEC-MED-010, SEC-MED-027, SEC-MED-028
#[test]
fn replays_a_movie_after_the_media_data() {
    let bytes = [FTYP, MDAT, MOVIE].concat();
    replay("movie-after-mdat", &bytes, &audio(60));
}
