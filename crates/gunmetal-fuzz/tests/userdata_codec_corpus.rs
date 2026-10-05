//! Replays the committed user-event fuzz corpus through its harness on
//! stable Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/userdata_codec` has a test here that pins its
//! exact bytes and the exact outcome the harness reports for it. Commit a
//! fuzzing reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::parse::ParseFault;
use gunmetal_core::userdata::codec::{EventError, Field};
use gunmetal_core::userdata::event::{
    Body, BodyType, ContentId, DeviceId, DocumentId, Event, EventId, ItemRef, Play, ProfileId,
    Stream, UnknownBody,
};
use gunmetal_core::userdata::hlc::Hlc;
use gunmetal_fuzz::userdata_codec::{Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/userdata_codec")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 6] = [
    "empty",
    "love-playlist",
    "overlong-varint",
    "play",
    "trailing",
    "unknown-type",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: Result<Event, EventError>) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes);
    assert_eq!(run(&file), Outcome { event: expected });
}

/// The envelope every seed shares: event ID `11…`, clock
/// 1,700,000,000,123 ms with counter 2, device `22…`, profile `33…`.
fn envelope() -> Vec<u8> {
    [
        &[0x11; 16][..],
        &[0xfb, 0xd0, 0x95, 0xff, 0xbc, 0x31, 0x02],
        &[0x22; 16],
        &[0x00],
        &[0x33; 16],
    ]
    .concat()
}

/// A play of item `44…` for 215,000 ms, to the end.
fn play() -> Vec<u8> {
    [
        &envelope()[..],
        &[0x01, 0x01, 0x01, 36],
        &[0x44; 32],
        &[0xd8, 0x8f, 0x0d, 0x01],
    ]
    .concat()
}

fn event(body: Body) -> Event {
    Event {
        id: EventId::new([0x11; 16]),
        clock: Hlc::new(1_700_000_000_123, 2),
        device: DeviceId::new([0x22; 16]),
        stream: Stream::Profile(ProfileId::new([0x33; 16])),
        body,
    }
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
        Err(EventError::Fault(ParseFault::Truncated {
            offset: 0,
            needed: 16,
            available: 0,
        })),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_play() {
    let played = event(Body::Play(Play {
        item: ContentId::new([0x44; 32]),
        position_ms: 215_000,
        completed: true,
    }));
    replay("play", &play(), Ok(played));
}

/// Verifies: SEC-MED-028
#[test]
fn replays_an_unlove_of_a_playlist() {
    let bytes = [&envelope()[..], &[0x04, 0x01, 0x01, 17, 0x01], &[0x66; 16]].concat();
    let unloved = event(Body::Unlove(ItemRef::Document(DocumentId::new([0x66; 16]))));
    replay("love-playlist", &bytes, Ok(unloved));
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_body_of_an_unknown_type() {
    let bytes = [&envelope()[..], &[0x28, 0x01, 0x00, 3, 0xde, 0xad, 0x00]].concat();
    let body_type = BodyType {
        tag: 40,
        version: 1,
        skippable: false,
    };
    let kept = UnknownBody::opaque(body_type, vec![0xde, 0xad, 0x00]).expect("type 40 is unknown");
    replay("unknown-type", &bytes, Ok(event(Body::Unknown(kept))));
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_wall_time_written_longer_than_it_needs() {
    let bytes = [&[0x11; 16][..], &[0x80, 0x00], &play()[22..]].concat();
    replay(
        "overlong-varint",
        &bytes,
        Err(EventError::Invalid {
            field: Field::Varint,
            offset: 16,
        }),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_octets_after_an_event() {
    let bytes = [&play()[..], &[0, 0]].concat();
    replay(
        "trailing",
        &bytes,
        Err(EventError::Trailing {
            offset: 96,
            count: 2,
        }),
    );
}
