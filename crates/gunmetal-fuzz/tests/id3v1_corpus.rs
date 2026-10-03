//! Replays the committed `ID3v1` fuzz corpus through its harness on stable
//! Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/id3v1` has a test here that pins its exact
//! bytes and the exact outcome the harness reports for it. Commit a fuzzing
//! reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::num::NonZeroU8;
use std::ops::Range;
use std::path::PathBuf;

use gunmetal_core::formats::id3v1::{Id3v1Error, Id3v1Tag};
use gunmetal_core::parse::ParseFault;
use gunmetal_core::text::Text;
use gunmetal_fuzz::id3v1::{Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/id3v1")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 6] = [
    "empty",
    "latin1-and-controls",
    "lower-case-marker",
    "one-octet-short-of-a-tag",
    "v1-1-track-padded-with-spaces",
    "v1-tag",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

fn text(value: &str) -> Text {
    Text {
        value: value.to_owned(),
        truncated: false,
        replaced: false,
    }
}

/// The tail case's window does not reach back to the file's last 128
/// octets: it starts 1,000 octets into a file `len` octets longer.
fn short_tail(len: u64) -> Result<Option<Id3v1Tag>, Id3v1Error> {
    Err(Id3v1Error::Fault(ParseFault::Truncated {
        offset: 872 + len,
        needed: 128,
        available: len,
    }))
}

/// `tag` read from the whole input, and from the tail 1,000 octets on.
fn both(tag: impl Fn(Range<u64>) -> Id3v1Tag) -> Outcome {
    Outcome {
        whole: Ok(Some(tag(0..128))),
        tail: Ok(Some(tag(1_000..1_128))),
    }
}

/// `field`, padded with zeros to `width` octets.
fn zeros(field: &[u8], width: usize) -> Vec<u8> {
    let mut padded = field.to_vec();
    padded.resize(width, 0);
    padded
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

/// Verifies: SEC-MED-028
#[test]
fn replays_the_empty_input() {
    replay(
        "empty",
        &[],
        &Outcome {
            whole: Ok(None),
            tail: short_tail(0),
        },
    );
}

/// Verifies: SEC-MED-028, SEC-HIS-036
#[test]
fn replays_a_tag_padded_with_zeros() {
    let bytes = [
        &b"TAG"[..],
        &zeros(b"Title", 30),
        &zeros(b"Artist", 30),
        &zeros(b"Album", 30),
        b"2003",
        &zeros(b"Comment", 30),
        &[17],
    ]
    .concat();
    replay(
        "v1-tag",
        &bytes,
        &both(|range| Id3v1Tag {
            range,
            title: text("Title"),
            artist: text("Artist"),
            album: text("Album"),
            year: text("2003"),
            comment: text("Comment"),
            track: None,
            genre: 17,
        }),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_an_id3v1_1_track_padded_with_spaces() {
    let bytes = [
        &b"TAG"[..],
        &[b' '; 94],
        b"Comment",
        &[b' '; 21],
        &[0, 7, 255],
    ]
    .concat();
    replay(
        "v1-1-track-padded-with-spaces",
        &bytes,
        &both(|range| Id3v1Tag {
            range,
            title: text(""),
            artist: text(""),
            album: text(""),
            year: text(""),
            comment: text("Comment"),
            track: NonZeroU8::new(7),
            genre: 255,
        }),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_marker_one_octet_short_of_a_tag() {
    let bytes = [&b"TAG"[..], &[0; 124]].concat();
    replay(
        "one-octet-short-of-a-tag",
        &bytes,
        &Outcome {
            whole: Ok(None),
            tail: Err(Id3v1Error::Fault(ParseFault::Truncated {
                offset: 999,
                needed: 128,
                available: 127,
            })),
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_latin1_text_with_controls_in_it() {
    let bytes = [
        &b"TAG"[..],
        &zeros(b"Caf\xE9", 30),
        &zeros(b"a\x1Bb\x85c", 30),
        &zeros(b"\0Hidden", 30),
        b" 99 ",
        &zeros(b"Tab\tand\nmore", 30),
        &[0],
    ]
    .concat();
    replay(
        "latin1-and-controls",
        &bytes,
        &both(|range| Id3v1Tag {
            range,
            title: text("Café"),
            artist: text("abc"),
            album: text(""),
            year: text("99"),
            comment: text("Tab\tand\nmore"),
            track: None,
            genre: 0,
        }),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_marker_in_lower_case() {
    let bytes = [&b"tag"[..], &[b'x'; 125]].concat();
    replay(
        "lower-case-marker",
        &bytes,
        &Outcome {
            whole: Ok(None),
            tail: Ok(None),
        },
    );
}
