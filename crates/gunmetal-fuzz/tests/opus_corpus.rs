//! Replays the committed Opus header fuzz corpus through its harness on
//! stable Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/opus` has a test here that pins its exact
//! bytes and the exact outcome the harness reports for it. Commit a fuzzing
//! reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::formats::opus::{ChannelMapping, OpusError, OpusHead};
use gunmetal_core::parse::{LimitKind, ParseFault};
use gunmetal_core::values::{Channels, Field, GainDb, SampleRate, ValueError};
use gunmetal_fuzz::opus::{Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/opus")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 10] = [
    "discrete-with-silence",
    "empty",
    "mapping-entry-past-the-streams",
    "no-channels",
    "opusenc-stereo",
    "surround-5-1",
    "tags-comment-past-the-end",
    "tags-count-past-the-limit",
    "tags-with-padding",
    "version-16",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome<'_>) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

/// What the stereo header `opusenc` writes holds.
fn stereo() -> OpusHead {
    OpusHead {
        version: 1,
        channels: Channels::new(2).expect("two channels are in range"),
        pre_skip: 312,
        input_sample_rate: SampleRate::new(48_000),
        output_gain: GainDb::new(0.0).expect("no gain is in range"),
        mapping: ChannelMapping::Single,
    }
}

/// The outcome for an identification header, which is not a comment
/// header.
fn head_only(head: Result<OpusHead, OpusError>) -> Outcome<'static> {
    Outcome {
        head,
        tags: Err(OpusError::Magic {
            offset: 0,
            found: *b"OpusHead",
        }),
    }
}

/// The outcome for a comment header, which is not an identification
/// header.
fn tags_only(tags: Result<&[u8], OpusError>) -> Outcome<'_> {
    Outcome {
        head: Err(OpusError::Magic {
            offset: 0,
            found: *b"OpusTags",
        }),
        tags,
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
    let truncated = OpusError::Fault(ParseFault::Truncated {
        offset: 0,
        needed: 8,
        available: 0,
    });
    replay(
        "empty",
        &[],
        &Outcome {
            head: Err(truncated),
            tags: Err(truncated),
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_the_stereo_header_opusenc_writes() {
    replay(
        "opusenc-stereo",
        b"OpusHead\x01\x02\x38\x01\x80\xBB\x00\x00\x00\x00\x00",
        &head_only(Ok(stereo())),
    );
}

/// libopus's 5.1 layout: four streams, two coupled, in Vorbis channel
/// order, at 44.1 kHz with -1.5 dB of gain.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_5_1_header_with_its_mapping_table() {
    replay(
        "surround-5-1",
        b"OpusHead\x01\x06\x00\x0F\x44\xAC\x00\x00\x80\xFE\x01\x04\x02\x00\x04\x01\x02\x03\x05",
        &head_only(Ok(OpusHead {
            channels: Channels::new(6).expect("six channels are in range"),
            pre_skip: 3_840,
            input_sample_rate: SampleRate::new(44_100),
            output_gain: GainDb::new(-1.5).expect("-1.5 dB is in range"),
            mapping: ChannelMapping::Table {
                family: 1,
                streams: 4,
                coupled: 2,
                mapping: vec![0, 4, 1, 2, 3, 5],
            },
            ..stereo()
        })),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_discrete_header_with_a_silent_channel() {
    replay(
        "discrete-with-silence",
        b"OpusHead\x01\x03\x38\x01\x80\xBB\x00\x00\x00\x00\xFF\x02\x01\x00\x02\xFF",
        &head_only(Ok(OpusHead {
            channels: Channels::new(3).expect("three channels are in range"),
            mapping: ChannelMapping::Table {
                family: 255,
                streams: 2,
                coupled: 1,
                mapping: vec![0, 2, 255],
            },
            ..stereo()
        })),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_header_of_major_version_1() {
    replay(
        "version-16",
        b"OpusHead\x10\x02\x38\x01\x80\xBB\x00\x00\x00\x00\x00",
        &head_only(Err(OpusError::Version {
            offset: 8,
            version: 16,
        })),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_header_with_no_channels() {
    replay(
        "no-channels",
        b"OpusHead\x01\x00\x38\x01\x80\xBB\x00\x00\x00\x00\x00",
        &head_only(Err(OpusError::Value {
            offset: 9,
            error: ValueError::OutOfRange {
                field: Field::Channels,
                value: 0,
            },
        })),
    );
}

/// One stream and one coupled stream decode to two channels, 0 and 1, so
/// an entry of 2 names none of them.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_mapping_entry_past_the_decoded_channels() {
    replay(
        "mapping-entry-past-the-streams",
        b"OpusHead\x01\x02\x38\x01\x80\xBB\x00\x00\x00\x00\x01\x01\x01\x00\x02",
        &head_only(Err(OpusError::MappingEntry {
            offset: 22,
            entry: 2,
            decoded: 2,
        })),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_comment_header_with_padding_after_its_comments() {
    let bytes: &[u8] =
        b"OpusTags\x0B\x00\x00\x00libopus 1.4\x01\x00\x00\x00\x07\x00\x00\x00TITLE=A\x00\x00\x00\x00";
    replay(
        "tags-with-padding",
        bytes,
        &tags_only(Ok(
            &b"\x0B\x00\x00\x00libopus 1.4\x01\x00\x00\x00\x07\x00\x00\x00TITLE=A"[..],
        )),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_comment_count_of_u32_max() {
    replay(
        "tags-count-past-the-limit",
        b"OpusTags\x00\x00\x00\x00\xFF\xFF\xFF\xFF\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",
        &tags_only(Err(OpusError::Fault(ParseFault::LimitExceeded {
            limit: LimitKind::TagFields,
            value: 4_294_967_295,
            max: 4_096,
            offset: 12,
        }))),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_comment_longer_than_the_packet() {
    replay(
        "tags-comment-past-the-end",
        b"OpusTags\x00\x00\x00\x00\x01\x00\x00\x00\x10\x00\x00\x00ab",
        &tags_only(Err(OpusError::Fault(ParseFault::Truncated {
            offset: 20,
            needed: 16,
            available: 2,
        }))),
    );
}
