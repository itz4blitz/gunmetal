//! Replays the committed `INFO` list fuzz corpus through its harness on
//! stable Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/tags_riff` has a test here that pins its exact
//! bytes and the exact outcome the harness reports for it. Commit a fuzzing
//! reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::catalog::{Credit, Role, TrackPosition, TrackTags};
use gunmetal_core::parse::ParseFault;
use gunmetal_core::tags::mp4::{Mapped, Reason, TagField, TagProblem};
use gunmetal_core::tags::riff::{InfoChunk, InfoTags};
use gunmetal_core::values::{Field, PartialDate, ValueError};
use gunmetal_fuzz::tags_riff::{Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/tags_riff")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 11] = [
    "eight-zero-octets",
    "empty",
    "every-mapped-id",
    "header-cut-short",
    "last-value-without-its-pad",
    "latin1-title",
    "odd-length-value-with-its-pad",
    "size-of-four-gigabytes",
    "title-and-artist",
    "track-number-out-of-range",
    "value-past-the-end",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

/// The sub-chunk `id` whose header starts `offset` octets into the file.
/// The harness puts the list 1,000 octets in.
fn at(id: [u8; 4], offset: u64) -> InfoChunk {
    InfoChunk { id, offset }
}

/// A reading that spent `steps` and gave `tags`, each from the sub-chunk
/// beside it in `sources`, with nothing dropped.
fn read(steps: u64, tags: TrackTags, sources: Vec<(TagField, InfoChunk)>) -> Outcome {
    Outcome {
        tags: InfoTags {
            mapped: Mapped {
                tags,
                sources,
                problems: Vec::new(),
            },
            stopped: None,
        },
        steps,
    }
}

/// A reading of one `INAM` sub-chunk at the start of the list.
fn titled(title: &str) -> Outcome {
    read(
        1,
        TrackTags {
            title: Some(String::from(title)),
            ..TrackTags::default()
        },
        vec![(TagField::Title, at(*b"INAM", 1_000))],
    )
}

/// `outcome`, with the walk stopped by `fault`.
fn stopped(mut outcome: Outcome, fault: ParseFault) -> Outcome {
    outcome.tags.stopped = Some(fault);
    outcome
}

/// Verifies: SEC-MED-028
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
    replay("empty", &[], &read(0, TrackTags::default(), Vec::new()));
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_title_and_an_artist() {
    replay(
        "title-and-artist",
        b"INAM\x02\x00\x00\x00abIART\x02\x00\x00\x00cd",
        &read(
            2,
            TrackTags {
                title: Some(String::from("ab")),
                artist: vec![String::from("cd")],
                ..TrackTags::default()
            },
            vec![
                (TagField::Title, at(*b"INAM", 1_000)),
                (TagField::Artist, at(*b"IART", 1_010)),
            ],
        ),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_an_odd_length_value_with_its_pad_octet() {
    replay(
        "odd-length-value-with-its-pad",
        b"INAM\x03\x00\x00\x00abc\x00IART\x02\x00\x00\x00de",
        &read(
            2,
            TrackTags {
                title: Some(String::from("abc")),
                artist: vec![String::from("de")],
                ..TrackTags::default()
            },
            vec![
                (TagField::Title, at(*b"INAM", 1_000)),
                (TagField::Artist, at(*b"IART", 1_012)),
            ],
        ),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_last_value_without_its_pad_octet() {
    replay(
        "last-value-without-its-pad",
        b"INAM\x03\x00\x00\x00abc",
        &titled("abc"),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_every_mapped_id_and_two_that_are_not() {
    replay(
        "every-mapped-id",
        &[
            &b"INAM\x0e\x00\x00\x00Blue in Green\x00"[..],
            b"IART\x0c\x00\x00\x00Miles Davis\x00",
            b"IPRD\x0d\x00\x00\x00Kind of Blue\x00\x00",
            b"ICRD\x0b\x00\x00\x001959-08-17\x00\x00",
            b"IGNR\x05\x00\x00\x00Jazz\x00\x00",
            b"ITRK\x04\x00\x00\x003/5\x00",
            b"IMUS\x0b\x00\x00\x00Bill Evans\x00\x00",
            b"ISFT\x0b\x00\x00\x00an encoder\x00\x00",
            b"ISRC\x0d\x00\x00\x00USS1Z9900001\x00\x00",
        ]
        .concat(),
        &read(
            9,
            TrackTags {
                title: Some(String::from("Blue in Green")),
                artist: vec![String::from("Miles Davis")],
                album: Some(String::from("Kind of Blue")),
                position: TrackPosition::new(Some(3), Some(5), None, None)
                    .expect("the position is in range"),
                date: Some(PartialDate::new(1959, Some(8), Some(17)).expect("the date exists")),
                genres: vec![String::from("Jazz")],
                credits: vec![
                    Credit::new(String::from("Bill Evans"), Role::Composer, None, None)
                        .expect("the name is not blank"),
                ],
                ..TrackTags::default()
            },
            vec![
                (TagField::Title, at(*b"INAM", 1_000)),
                (TagField::Artist, at(*b"IART", 1_022)),
                (TagField::Album, at(*b"IPRD", 1_042)),
                (TagField::Date, at(*b"ICRD", 1_064)),
                (TagField::Genres, at(*b"IGNR", 1_084)),
                (TagField::Track, at(*b"ITRK", 1_098)),
                (TagField::TrackTotal, at(*b"ITRK", 1_098)),
                (TagField::Credit(Role::Composer), at(*b"IMUS", 1_110)),
            ],
        ),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_title_in_latin1() {
    replay(
        "latin1-title",
        b"INAM\x04\x00\x00\x00Caf\xe9",
        &titled("Café"),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_sub_chunk_of_eight_zero_octets() {
    replay(
        "eight-zero-octets",
        &[0; 8],
        &read(1, TrackTags::default(), Vec::new()),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_header_cut_short() {
    replay(
        "header-cut-short",
        b"INAM\x02\x00\x00\x00abIAR",
        &stopped(
            Outcome {
                steps: 2,
                ..titled("ab")
            },
            ParseFault::Truncated {
                offset: 1_010,
                needed: 4,
                available: 3,
            },
        ),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_value_that_runs_past_the_end_of_the_list() {
    replay(
        "value-past-the-end",
        b"IART\x0a\x00\x00\x00abc",
        &stopped(
            read(1, TrackTags::default(), Vec::new()),
            ParseFault::Truncated {
                offset: 1_008,
                needed: 10,
                available: 3,
            },
        ),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_declared_size_of_four_gigabytes() {
    replay(
        "size-of-four-gigabytes",
        b"INAM\xff\xff\xff\xffa",
        &stopped(
            read(1, TrackTags::default(), Vec::new()),
            ParseFault::Truncated {
                offset: 1_008,
                needed: 4_294_967_295,
                available: 1,
            },
        ),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_track_number_out_of_range() {
    let mut expected = read(1, TrackTags::default(), Vec::new());
    expected.tags.mapped.problems = vec![TagProblem::Value {
        field: TagField::Track,
        source: at(*b"ITRK", 1_000),
        reason: Reason::Invalid(ValueError::OutOfRange {
            field: Field::Number,
            value: 10_000,
        }),
    }];
    replay(
        "track-number-out-of-range",
        b"ITRK\x05\x00\x00\x0010000\x00",
        &expected,
    );
}
