//! Replays the committed recipes of the ID3 family's structure-aware
//! harness on stable Rust, so that `cargo test` and the gate run every
//! seed and every reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/id3_structure` is a recipe, not a file. Each
//! has a test here that pins its exact bytes, the exact file the harness
//! builds from it and what the parsers find in that file. Commit a fuzzing
//! reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::ops::Range;
use std::path::PathBuf;

use gunmetal_core::formats::ape::{ApeItem, ApeTag, ApeValue, ItemProblem};
use gunmetal_core::formats::id3v1::Id3v1Tag;
use gunmetal_core::formats::id3v2::{
    Frame, FrameBody, FrameId, Header, Id3v2Tag, PictureRef, Span, TagProblem,
};
use gunmetal_core::text::Text;
use gunmetal_fuzz::id3_structure::{Id3v2Outcome, Outcome, id3v2, run};
use gunmetal_testkit::id3v2::{Tag, Version, chapter, frame as written};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/id3_structure")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 13] = [
    "empty",
    "forbidden-key-in-version-1000",
    "header-two-items-and-id3v1",
    "one-item-of-each-type",
    "sixty-five-frames",
    "v22-unsynchronised-with-the-footer-bit",
    "v23-plain",
    "v23-unsynchronised",
    "v24-every-kind",
    "v24-footer-only",
    "v24-frame-flags",
    "value-that-poses-as-id3v1",
    "zero-in-a-key",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

/// A header or footer block, as the specification lays it out.
fn block(version: u32, size: u32, count: u32, flags: u32) -> Vec<u8> {
    let mut block = b"APETAGEX".to_vec();
    for value in [version, size, count, flags] {
        block.extend(value.to_le_bytes());
    }
    block.extend([0; 8]);
    block
}

/// An item, as the specification lays it out.
fn raw_item(key: &[u8], flags: u32, value: &[u8]) -> Vec<u8> {
    let len = u32::try_from(value.len()).expect("a short value");
    [
        &len.to_le_bytes()[..],
        &flags.to_le_bytes(),
        key,
        &[0],
        value,
    ]
    .concat()
}

fn text(value: &str) -> Text {
    Text {
        value: value.to_owned(),
        truncated: false,
        replaced: false,
    }
}

fn item(key: &str, value: ApeValue) -> ApeItem {
    ApeItem {
        key: key.to_owned(),
        value,
    }
}

fn tag(range: Range<u64>, items: Vec<ApeItem>, problems: Vec<ItemProblem>) -> ApeTag {
    ApeTag {
        range,
        items,
        problems,
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

/// An empty recipe asks for nothing: an `APEv2` footer alone.
///
/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_the_empty_recipe() {
    replay(
        "empty",
        &[],
        &Outcome {
            file: block(2_000, 32, 0, 0),
            ape_range: 0..32,
            ape: Ok(Some(tag(0..32, vec![], vec![]))),
            v1: Ok(None),
            checked: true,
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_headed_tag_of_two_items_before_an_id3v1_tag() {
    let recipe = [
        &[5, 3, 0xAA, 0xBB, 0xCC, 2, 0, 5][..],
        b"Title\x04Song",
        &[2, 4],
        b"Data\x02\x01\x02",
        b"Other",
    ]
    .concat();
    let mut v1 = b"TAGOther".to_vec();
    v1.resize(128, 0);
    let file = [
        &[0xAA, 0xBB, 0xCC][..],
        &block(2_000, 65, 2, 0xA000_0000),
        &raw_item(b"Title", 0, b"Song"),
        &raw_item(b"Data", 2, &[1, 2]),
        &block(2_000, 65, 2, 0x8000_0000),
        &v1,
    ]
    .concat();
    replay(
        "header-two-items-and-id3v1",
        &recipe,
        &Outcome {
            file,
            ape_range: 3..100,
            ape: Ok(Some(tag(
                3..100,
                vec![
                    item("Title", ApeValue::Text(vec![text("Song")])),
                    item("Data", ApeValue::Binary(66..68)),
                ],
                vec![],
            ))),
            v1: Ok(Some(Id3v1Tag {
                range: 100..228,
                title: text("Other"),
                artist: text(""),
                album: text(""),
                year: text(""),
                comment: text(""),
                track: None,
                genre: 0,
            })),
            checked: true,
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_forbidden_key_in_an_apev1_tag() {
    let recipe = [&[2, 0, 2, 0, 3][..], b"TAG\x01x", &[1, 2], b"Ok\x03a\0b"].concat();
    let file = [
        &raw_item(b"TAG", 0, b"x")[..],
        &raw_item(b"Ok", 1, b"a\0b"),
        &block(1_000, 59, 2, 0),
    ]
    .concat();
    replay(
        "forbidden-key-in-version-1000",
        &recipe,
        &Outcome {
            file,
            ape_range: 0..59,
            ape: Ok(Some(tag(
                0..59,
                vec![item("Ok", ApeValue::Text(vec![text("a"), text("b")]))],
                vec![ItemProblem::BadKey { offset: 0 }],
            ))),
            v1: Ok(None),
            checked: true,
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_one_item_of_each_type_after_a_header() {
    let recipe = [
        &[1, 0, 4, 0, 2][..],
        b"T1\x01a",
        &[2, 2],
        b"B1\x02\x01\x02",
        &[4, 2],
        b"L1\x03x\0y",
        &[6, 2],
        b"R1\x01z",
    ]
    .concat();
    let file = [
        &block(2_000, 83, 4, 0xA000_0000)[..],
        &raw_item(b"T1", 0, b"a"),
        &raw_item(b"B1", 2, &[1, 2]),
        &raw_item(b"L1", 4, b"x\0y"),
        &raw_item(b"R1", 6, b"z"),
        &block(2_000, 83, 4, 0x8000_0000),
    ]
    .concat();
    replay(
        "one-item-of-each-type",
        &recipe,
        &Outcome {
            file,
            ape_range: 0..115,
            ape: Ok(Some(tag(
                0..115,
                vec![
                    item("T1", ApeValue::Text(vec![text("a")])),
                    item("B1", ApeValue::Binary(55..57)),
                    item("L1", ApeValue::Locator(vec![text("x"), text("y")])),
                    item("R1", ApeValue::Reserved(82..83)),
                ],
                vec![],
            ))),
            v1: Ok(None),
            checked: true,
        },
    );
}

/// Four binary items whose second value puts `TAG` 128 octets from the end
/// of the file. `find_v1` reports that slot; `parse_ape` still finds the
/// tag that ends the file, because no footer sits immediately before it.
/// The harness does not check the `ID3v1` fields: none were built.
///
/// Verifies: SEC-MED-001, SEC-MED-028
#[test]
fn replays_a_value_that_poses_as_an_id3v1_tag() {
    let keys: [&[u8]; 4] = [b"Ab", b"Cd", b"Ef", b"Gh"];
    let values: [&[u8]; 4] = [
        b"0123456789abcdefghijklmnopqrstu",
        b"ABCDEFGHIJKLMNOPQRSTAGabcdefghi",
        b"0123456789klmnopqrstuvwxyzKLMNO",
        b"0123456789012345678901234567xyz",
    ];
    let mut recipe = vec![0, 0, 4];
    let mut file = Vec::new();
    for (key, value) in keys.into_iter().zip(values) {
        recipe.extend([&[2, 2][..], key, &[31], value].concat());
        file.extend(raw_item(key, 2, value));
    }
    file.extend(block(2_000, 200, 4, 0));
    replay(
        "value-that-poses-as-id3v1",
        &recipe,
        &Outcome {
            file,
            ape_range: 0..200,
            ape: Ok(Some(tag(
                0..200,
                vec![
                    item("Ab", ApeValue::Binary(11..42)),
                    item("Cd", ApeValue::Binary(53..84)),
                    item("Ef", ApeValue::Binary(95..126)),
                    item("Gh", ApeValue::Binary(137..168)),
                ],
                vec![],
            ))),
            v1: Ok(Some(Id3v1Tag {
                range: 72..200,
                // The rest of the second value, up to the first zero of
                // the third item's length.
                title: text("abcdefghi"),
                artist: text("klmnopqrstuvwxyzKLMNO"),
                album: text("h"),
                year: text("xyzA"),
                // The footer from its second octet, to the first zero of
                // the version 2000 (0xD0, 0x07, 0x00, 0x00).
                comment: text("PETAGEX\u{D0}"),
                track: None,
                genre: 0,
            })),
            checked: false,
        },
    );
}

/// A zero in a key would end it early and break the framing, so the
/// recipe's zero becomes 0x01, which the key may not hold either.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_zero_asked_for_in_a_key() {
    let file = [&raw_item(b"A\x01B", 0, b"")[..], &block(2_000, 44, 1, 0)].concat();
    replay(
        "zero-in-a-key",
        &[0, 0, 1, 0, 3, b'A', 0, b'B', 0],
        &Outcome {
            file,
            ape_range: 0..44,
            ape: Ok(Some(tag(
                0..44,
                vec![],
                vec![ItemProblem::BadKey { offset: 0 }],
            ))),
            v1: Ok(None),
            checked: true,
        },
    );
}

fn replay_id3v2(name: &str, recipe: &[u8], tag: Vec<u8>, read: Id3v2Tag) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, recipe, "seed {name} holds different bytes");
    assert_eq!(
        id3v2(&file),
        Id3v2Outcome {
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

/// An empty recipe is a 2.2 tag with no frames.
///
/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_the_empty_id3v2_recipe() {
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

/// A 2.4 recipe with only the footer bit set writes a footer and no
/// unsynchronisation flag.
///
/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_a_2_4_footer_without_unsynchronisation() {
    replay(
        "v24-footer-only",
        &[0x20],
        Tag::new(Version::V24).footer().build(),
        Id3v2Tag {
            header: header(4, 0x10, 0, 20),
            extended: None,
            frames: vec![],
            problems: vec![],
        },
    );
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
