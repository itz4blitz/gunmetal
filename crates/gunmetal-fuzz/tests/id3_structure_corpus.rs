//! Replays the committed corpus of the structure-aware `ID3v2` harness on
//! stable Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/id3_structure` is a recipe. Its test here pins
//! the recipe's exact bytes, the tag the harness writes from it, written
//! again with the testkit's tag builder, and the exact tag the parser reads
//! back. Commit a fuzzing reproducer by adding its file and its test
//! together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::formats::id3v2::{
    Frame, FrameBody, FrameId, Header, Id3v2Tag, PictureRef, Span, TagProblem,
};
use gunmetal_core::text::Text;
use gunmetal_fuzz::id3_structure::{Outcome, run};
use gunmetal_testkit::id3v2::{Tag, Version, chapter, frame as written};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/id3_structure")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 7] = [
    "empty",
    "sixty-five-frames",
    "v22-unsynchronised-with-the-footer-bit",
    "v23-plain",
    "v23-unsynchronised",
    "v24-every-kind",
    "v24-frame-flags",
];

/// Reads seed `name`, checks that it holds exactly `recipe`, and checks
/// that the harness writes `tag` from it and reads back `read`.
fn replay(name: &str, recipe: &[u8], tag: Vec<u8>, read: Id3v2Tag) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, recipe, "seed {name} holds different bytes");
    assert_eq!(
        run(&file),
        Outcome {
            tag,
            read: Ok(read)
        },
        "seed {name}"
    );
}

fn header(major: u8, flags: u8, size: u32, len: u64) -> Header {
    Header {
        major,
        revision: 0,
        flags,
        size,
        len,
    }
}

fn plain(value: &str) -> Text {
    Text {
        value: value.to_owned(),
        truncated: false,
        replaced: false,
    }
}

fn four(id: &[u8]) -> FrameId {
    FrameId::Four(id.try_into().expect("a 2.3 or 2.4 identifier"))
}

fn frame(id: FrameId, offset: u64, flags: u16, body: FrameBody) -> Frame {
    Frame {
        id,
        offset,
        flags,
        body,
    }
}

fn raw(start: u64, end: u64, unsynchronised: bool) -> FrameBody {
    FrameBody::Raw(Span {
        start,
        end,
        unsynchronised,
    })
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

/// An empty recipe is a 2.2 tag with no frames.
///
/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_the_empty_recipe() {
    replay(
        "empty",
        &[],
        Tag::new(Version::V22).build(),
        Id3v2Tag {
            header: header(2, 0, 0, 10),
            extended: None,
            frames: vec![],
            problems: vec![],
        },
    );
}

/// One frame of each kind in 2.4. The chapter holds an empty title, which
/// is recorded at its own offset and is not one of the frames written.
///
/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_a_2_4_tag_with_every_kind_of_frame() {
    let embedded = written(Version::V24, b"TIT2", 0, &[]);
    let chapter_body = chapter("c", [0; 4], &embedded);
    let recipe = [
        &[0x02][..],
        &[12, 0, 3, 0x03, b'H', b'i'],
        &[1, 0, 4, 0x03, b'k', 0x00, b'v'],
        &[13, 0, 2, 0x09, b'x'],
        &[6, 0, 5, 0x00, 0x00, 0x03, 0x00, b'x'],
        &[8, 0, 28],
        &chapter_body,
        &[11, 0, 2, b'a', b'b'],
    ]
    .concat();
    let tag = Tag::new(Version::V24)
        .frame(b"TIT2", 0, b"\x03Hi")
        .frame(b"TXXX", 0, b"\x03k\x00v")
        .frame(b"TXXX", 0, b"\x09x")
        .frame(b"APIC", 0, b"\x00\x00\x03\x00x")
        .frame(b"CHAP", 0, &chapter_body)
        .frame(b"PRIV", 0, b"ab")
        .build();
    let read = Id3v2Tag {
        header: header(4, 0, 104, 114),
        extended: None,
        frames: vec![
            frame(four(b"TIT2"), 10, 0, FrameBody::Text(vec![plain("Hi")])),
            frame(
                four(b"TXXX"),
                23,
                0,
                FrameBody::UserText {
                    description: plain("k"),
                    values: vec![plain("v")],
                },
            ),
            frame(four(b"TXXX"), 37, 0, raw(47, 49, false)),
            frame(
                four(b"APIC"),
                49,
                0,
                FrameBody::Picture(PictureRef {
                    mime: plain(""),
                    picture_type: 3,
                    description: plain(""),
                    data: Span {
                        start: 63,
                        end: 64,
                        unsynchronised: false,
                    },
                }),
            ),
            frame(four(b"CHAP"), 64, 0, raw(74, 102, false)),
            frame(four(b"PRIV"), 102, 0, raw(112, 114, false)),
        ],
        problems: vec![
            TagProblem::Malformed {
                offset: 37,
                id: four(b"TXXX"),
            },
            TagProblem::EmptyFrame {
                offset: 92,
                id: four(b"TIT2"),
            },
        ],
    };
    replay("v24-every-kind", &recipe, tag, read);
}

/// The tag's unsynchronisation moves every frame after a size of 255 and
/// a body of `FF` octets.
///
/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_an_unsynchronised_2_3_tag() {
    let ffs = [0xFF; 255];
    let recipe = [
        &[0x10, 11, 0, 255][..],
        &ffs,
        &[0, 0x20, 3, 0x80, 0x00, b'A'],
    ]
    .concat();
    let tag = Tag::new(Version::V23)
        .unsynchronised()
        .frame(b"PRIV", 0, &ffs)
        .frame(b"TIT2", 0x0020, b"\x80\x00A")
        .build();
    // PRIV's header takes 11 octets as stored and its body 509: each FF
    // but the last is followed by another.
    let read = Id3v2Tag {
        header: header(3, 0x80, 533, 543),
        extended: None,
        frames: vec![
            frame(four(b"PRIV"), 10, 0, raw(21, 530, true)),
            frame(
                four(b"TIT2"),
                530,
                0x0020,
                FrameBody::Text(vec![plain("A")]),
            ),
        ],
        problems: vec![],
    };
    replay("v23-unsynchronised", &recipe, tag, read);
}

/// Without the tag's flag, a 2.3 tag is stored as written.
///
/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_a_2_3_tag_stored_as_written() {
    let recipe = [0x01, 11, 0, 2, 0xFF, 0x00];
    let tag = Tag::new(Version::V23)
        .frame(b"PRIV", 0, b"\xFF\x00")
        .build();
    let read = Id3v2Tag {
        header: header(3, 0, 12, 22),
        extended: None,
        frames: vec![frame(four(b"PRIV"), 10, 0, raw(20, 22, false))],
        problems: vec![],
    };
    replay("v23-plain", &recipe, tag, read);
}

/// Every frame flag 2.4 acts on, with the tag's unsynchronisation and a
/// footer.
///
/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_2_4_frame_flags() {
    let recipe = [
        &[0x32][..],
        &[0, 0x41, 7, 0x80, 0x00, 0x00, 0x00, 0x02, 0x03, b'A'],
        &[3, 0x08, 1, b'x'],
        &[4, 0x04, 1, b'y'],
        &[5, 0x00, 0],
        &[7, 0x01, 2, 0x00, 0x00],
        &[11, 0x02, 2, 0xFF, 0x00],
        &[10, 0x00, 1, 0xFF],
    ]
    .concat();
    // The tag's flag makes every body unsynchronised.
    let tag = Tag::new(Version::V24)
        .unsynchronised()
        .footer()
        .frame(b"TIT2", 0x0041, b"\x80\x00\x00\x00\x02\x03A")
        .frame(b"COMM", 0x0008, b"x")
        .frame(b"USLT", 0x0004, b"y")
        .frame(b"SYLT", 0x0000, b"")
        .frame(b"UFID", 0x0001, b"\x00\x00")
        .frame(b"PRIV", 0x0002, b"\xFF\x00\x00")
        .frame(b"POPM", 0x0000, b"\xFF\x00")
        .build();
    let read = Id3v2Tag {
        header: header(4, 0x90, 86, 106),
        extended: None,
        frames: vec![
            frame(four(b"TIT2"), 10, 0x0041, FrameBody::Text(vec![plain("A")])),
            frame(four(b"PRIV"), 71, 0x0002, raw(81, 84, true)),
            frame(four(b"POPM"), 84, 0, raw(94, 96, true)),
        ],
        problems: vec![
            TagProblem::Compressed {
                offset: 27,
                id: four(b"COMM"),
            },
            TagProblem::Encrypted {
                offset: 38,
                id: four(b"USLT"),
            },
            TagProblem::EmptyFrame {
                offset: 49,
                id: four(b"SYLT"),
            },
            TagProblem::Malformed {
                offset: 59,
                id: four(b"UFID"),
            },
        ],
    };
    replay("v24-frame-flags", &recipe, tag, read);
}

/// 2.2 ignores frame flags and has no footer; its tag's unsynchronisation
/// moves the second frame, and a final `FF` takes a zero after it.
///
/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_an_unsynchronised_2_2_tag_with_the_footer_bit() {
    let recipe = [0x30, 10, 0xFF, 4, 0x00, 0xFF, 0xE0, b'A', 9, 0, 1, 0xFF];
    let tag = Tag::new(Version::V22)
        .unsynchronised()
        .frame(b"TT2", 0, b"\x00\xFF\xE0A")
        .frame(b"PRV", 0, b"\xFF")
        .build();
    let read = Id3v2Tag {
        header: header(2, 0x80, 19, 29),
        extended: None,
        frames: vec![
            frame(
                FrameId::Three(*b"TT2"),
                10,
                0,
                FrameBody::Text(vec![plain("ÿàA")]),
            ),
            frame(FrameId::Three(*b"PRV"), 21, 0, raw(27, 29, true)),
        ],
        problems: vec![],
    };
    replay("v22-unsynchronised-with-the-footer-bit", &recipe, tag, read);
}

/// A recipe of 65 frames writes 64.
///
/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_a_recipe_of_more_frames_than_are_written() {
    let recipe: Vec<u8> = [0x01]
        .into_iter()
        .chain((0..65).flat_map(|_| [0, 0, 0]))
        .collect();
    let tag = (0..64)
        .fold(Tag::new(Version::V23), |tag, _| tag.frame(b"TIT2", 0, &[]))
        .build();
    let read = Id3v2Tag {
        header: header(3, 0, 640, 650),
        extended: None,
        frames: vec![],
        problems: (0..64)
            .map(|index| TagProblem::EmptyFrame {
                offset: 10 + 10 * index,
                id: four(b"TIT2"),
            })
            .collect(),
    };
    replay("sixty-five-frames", &recipe, tag, read);
}
