//! Replays the committed `ID3v2` fuzz corpus through its harness on stable
//! Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/id3v2_tag` has a test here that pins its exact
//! bytes, written again with the testkit's tag builder, and the exact
//! outcome the harness reports for it. Commit a fuzzing reproducer by
//! adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::borrow::Cow;
use std::fs;
use std::path::PathBuf;

use gunmetal_core::formats::id3v2::{
    Frame, FrameBody, FrameId, Header, Id3v2Error, Id3v2Tag, LanguageText, PictureRef, Span,
    TagProblem,
};
use gunmetal_core::parse::{LimitKind, ParseFault};
use gunmetal_core::text::Text;
use gunmetal_fuzz::id3v2_tag::{Outcome, run};
use gunmetal_testkit::id3v2::{
    Encoding, Tag, Version, chapter, comment, frame as written, picture, picture_v22, text,
    user_text,
};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/id3v2_tag")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 13] = [
    "chapters-five-deep",
    "comment-with-markup",
    "empty",
    "footer-alone",
    "picture-link-to-a-private-address",
    "picture-of-svg-markup",
    "plain-frame-sizes",
    "size-past-the-end",
    "specification-tag",
    "unsynchronised-picture",
    "user-text-with-a-path",
    "utf8-title",
    "v22-comment-and-picture",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports what `expected` builds from them.
fn replay(name: &str, bytes: &[u8], expected: impl FnOnce(&[u8]) -> Outcome<'_>) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(run(&file), expected(&file), "seed {name}");
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

fn frame(id: &[u8], offset: u64, body: FrameBody) -> Frame {
    Frame {
        id: FrameId::Four(id.try_into().expect("a 2.3 or 2.4 identifier")),
        offset,
        flags: 0,
        body,
    }
}

fn span(start: u64, end: u64, unsynchronised: bool) -> Span {
    Span {
        start,
        end,
        unsynchronised,
    }
}

fn picture_ref(mime: &str, data: Span) -> FrameBody {
    FrameBody::Picture(PictureRef {
        mime: plain(mime),
        picture_type: 3,
        description: plain(""),
        data,
    })
}

/// The tag `frames` make with the header `header`, and no problems.
fn tag(header: Header, frames: Vec<Frame>) -> Id3v2Tag {
    Id3v2Tag {
        header,
        extended: None,
        frames,
        problems: vec![],
    }
}

/// What `footer` reports for octets that start with `ID3`.
const NOT_A_FOOTER: Result<Header, Id3v2Error> = Err(Id3v2Error::NotId3v2 {
    offset: 0,
    marker: *b"ID3",
});

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
    let cut = Id3v2Error::Fault(ParseFault::Truncated {
        offset: 0,
        needed: 10,
        available: 0,
    });
    replay("empty", &[], |_| Outcome {
        header: Err(cut),
        footer: Err(cut),
        tag: Err(cut),
        octets: vec![],
    });
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_utf8_title() {
    let bytes = Tag::new(Version::V24)
        .frame(b"TIT2", 0, &text(Encoding::Utf8, &["Gunmetal"]))
        .build();
    let head = header(4, 0, 19, 29);
    replay("utf8-title", &bytes, |_| Outcome {
        header: Ok(head),
        footer: NOT_A_FOOTER,
        tag: Ok(tag(
            head,
            vec![frame(b"TIT2", 10, FrameBody::Text(vec![plain("Gunmetal")]))],
        )),
        octets: vec![],
    });
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_picture_in_an_unsynchronised_tag() {
    let jpeg = [0xFF, 0xD8, 0xFF, 0xE0];
    let bytes = Tag::new(Version::V23)
        .unsynchronised()
        .frame(
            b"APIC",
            0,
            &picture(Encoding::Latin1, "image/jpeg", 3, "", &jpeg),
        )
        .build();
    // The body is 18 octets as read and 19 as stored: FF E0 takes a zero.
    let head = header(3, 0x80, 29, 39);
    replay("unsynchronised-picture", &bytes, |_| Outcome {
        header: Ok(head),
        footer: NOT_A_FOOTER,
        tag: Ok(tag(
            head,
            vec![frame(
                b"APIC",
                10,
                picture_ref("image/jpeg", span(34, 39, true)),
            )],
        )),
        octets: vec![Cow::Owned(jpeg.to_vec())],
    });
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_2_2_comment_and_picture() {
    let bytes = Tag::new(Version::V22)
        .frame(b"COM", 0, &comment(Encoding::Latin1, *b"eng", "", "Hi"))
        .frame(
            b"PIC",
            0,
            &picture_v22(Encoding::Latin1, *b"PNG", 3, "", &[0x89, 0x50]),
        )
        .build();
    let head = header(2, 0, 27, 37);
    let frames = vec![
        Frame {
            id: FrameId::Three(*b"COM"),
            offset: 10,
            flags: 0,
            body: FrameBody::Comment(LanguageText {
                language: *b"eng",
                description: plain(""),
                text: plain("Hi"),
            }),
        },
        Frame {
            id: FrameId::Three(*b"PIC"),
            offset: 23,
            flags: 0,
            body: picture_ref("PNG", span(35, 37, false)),
        },
    ];
    replay("v22-comment-and-picture", &bytes, |_| Outcome {
        header: Ok(head),
        footer: NOT_A_FOOTER,
        tag: Ok(tag(head, frames)),
        octets: vec![Cow::Borrowed(&[0x89, 0x50][..])],
    });
}

/// Verifies: SEC-MED-028
#[test]
fn replays_2_4_frame_sizes_written_as_plain_integers() {
    let bytes = Tag::new(Version::V24)
        .frame(b"TIT2", 0, &text(Encoding::Latin1, &["A"]))
        .bytes(&written(Version::V23, b"PRIV", 0, &[0x41; 200]))
        .build();
    let head = header(4, 0, 222, 232);
    replay("plain-frame-sizes", &bytes, |file| Outcome {
        header: Ok(head),
        footer: NOT_A_FOOTER,
        tag: Ok(tag(
            head,
            vec![
                frame(b"TIT2", 10, FrameBody::Text(vec![plain("A")])),
                frame(b"PRIV", 22, FrameBody::Raw(span(32, 232, false))),
            ],
        )),
        octets: vec![Cow::Borrowed(&file[32..232])],
    });
}

/// Verifies: SEC-MED-005, SEC-MED-028
#[test]
fn replays_chapters_nested_five_levels_deep() {
    let mut frames = written(Version::V24, b"TIT2", 0, &text(Encoding::Latin1, &["A"]));
    for _ in 0..5 {
        frames = written(Version::V24, b"CHAP", 0, &chapter("c", [0; 4], &frames));
    }
    let bytes = Tag::new(Version::V24).bytes(&frames).build();
    let head = header(4, 0, 152, 162);
    replay("chapters-five-deep", &bytes, |file| Outcome {
        header: Ok(head),
        footer: NOT_A_FOOTER,
        tag: Ok(Id3v2Tag {
            header: head,
            extended: None,
            frames: vec![frame(b"CHAP", 10, FrameBody::Raw(span(20, 162, false)))],
            problems: vec![TagProblem::Fault(ParseFault::TooDeep {
                limit: LimitKind::EmbeddedFrameDepth,
                depth: 5,
                max: 4,
                offset: 150,
            })],
        }),
        octets: vec![Cow::Borrowed(&file[20..162])],
    });
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_size_past_the_end() {
    let bytes = b"ID3\x04\x00\x00\x00\x00\x00\x64TIT2\x00";
    replay("size-past-the-end", bytes, |_| Outcome {
        header: Ok(header(4, 0, 100, 110)),
        footer: NOT_A_FOOTER,
        tag: Err(Id3v2Error::Fault(ParseFault::Truncated {
            offset: 10,
            needed: 100,
            available: 5,
        })),
        octets: vec![],
    });
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_footer_alone() {
    let not_a_header = Id3v2Error::NotId3v2 {
        offset: 0,
        marker: *b"3DI",
    };
    replay("footer-alone", b"3DI\x04\x00\x10\x00\x00\x02\x01", |_| {
        Outcome {
            header: Err(not_a_header),
            footer: Ok(header(4, 0x10, 257, 277)),
            tag: Err(not_a_header),
            octets: vec![],
        }
    });
}

/// Verifies: SEC-MED-028
#[test]
fn replays_the_tag_the_testkit_writes_from_the_specification() {
    let bytes = Tag::new(Version::V24)
        .extended_header(&[0x00, 0x00, 0x00, 0x07, 0x01, 0x40, 0x00])
        .frame(b"TIT2", 0x0000, &text(Encoding::Latin1, &["Song"]))
        .frame(b"TPE1", 0x4000, &text(Encoding::Utf8, &["A", "B"]))
        .padding(4)
        .footer()
        .build();
    let head = header(4, 0x50, 40, 60);
    let artists = Frame {
        flags: 0x4000,
        ..frame(b"TPE1", 32, FrameBody::Text(vec![plain("A"), plain("B")]))
    };
    replay("specification-tag", &bytes, |_| Outcome {
        header: Ok(head),
        footer: NOT_A_FOOTER,
        tag: Ok(Id3v2Tag {
            header: head,
            extended: Some(gunmetal_core::formats::id3v2::ExtendedHeader {
                update: true,
                crc: None,
                restrictions: None,
            }),
            frames: vec![
                frame(b"TIT2", 17, FrameBody::Text(vec![plain("Song")])),
                artists,
            ],
            problems: vec![],
        }),
        octets: vec![],
    });
}

/// Navidrome CVE-2026-25578: a comment that a web client rendered as HTML.
/// It is read as text and nothing more.
///
/// Verifies: SEC-MED-028, SEC-HIS-036
#[test]
fn replays_a_comment_holding_markup() {
    let markup = "<img src=x onerror=alert(localStorage.token)>";
    let bytes = Tag::new(Version::V24)
        .frame(b"COMM", 0, &comment(Encoding::Utf8, *b"eng", "", markup))
        .build();
    let head = header(4, 0, 60, 70);
    replay("comment-with-markup", &bytes, |_| Outcome {
        header: Ok(head),
        footer: NOT_A_FOOTER,
        tag: Ok(tag(
            head,
            vec![frame(
                b"COMM",
                10,
                FrameBody::Comment(LanguageText {
                    language: *b"eng",
                    description: plain(""),
                    text: plain(markup),
                }),
            )],
        )),
        octets: vec![],
    });
}

/// SEC-HIS-030: an embedded picture that declares itself SVG and carries
/// script. The parser only says where its octets are.
///
/// Verifies: SEC-MED-028, SEC-HIS-036
#[test]
fn replays_a_picture_of_svg_markup() {
    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\" onload=\"alert(1)\"/>";
    let bytes = Tag::new(Version::V24)
        .frame(
            b"APIC",
            0,
            &picture(Encoding::Latin1, "image/svg+xml", 3, "", svg),
        )
        .build();
    let head = header(4, 0, 86, 96);
    replay("picture-of-svg-markup", &bytes, |_| Outcome {
        header: Ok(head),
        footer: NOT_A_FOOTER,
        tag: Ok(tag(
            head,
            vec![frame(
                b"APIC",
                10,
                picture_ref("image/svg+xml", span(37, 96, false)),
            )],
        )),
        octets: vec![Cow::Borrowed(&svg[..])],
    });
}

/// SEC-MED-016: a picture whose MIME type `-->` makes its data a link,
/// here to a cloud metadata address. The link is octets, never followed.
///
/// Verifies: SEC-MED-028, SEC-HIS-036
#[test]
fn replays_a_picture_link_to_a_private_address() {
    let link = b"http://169.254.169.254/latest/meta-data/";
    let bytes = Tag::new(Version::V23)
        .frame(b"APIC", 0, &picture(Encoding::Latin1, "-->", 3, "", link))
        .build();
    let head = header(3, 0, 57, 67);
    replay("picture-link-to-a-private-address", &bytes, |_| Outcome {
        header: Ok(head),
        footer: NOT_A_FOOTER,
        tag: Ok(tag(
            head,
            vec![frame(b"APIC", 10, picture_ref("-->", span(27, 67, false)))],
        )),
        octets: vec![Cow::Borrowed(&link[..])],
    });
}

/// Jellyfin CVE-2026-35031 reached the filesystem through a name from
/// metadata (T-HIS-05). A path in a tag is read as text and nothing more.
///
/// Verifies: SEC-MED-028, SEC-HIS-036
#[test]
fn replays_user_text_holding_a_path() {
    let path = "../../etc/cron.d/x";
    let bytes = Tag::new(Version::V24)
        .frame(b"TXXX", 0, &user_text(Encoding::Utf8, "filename", &[path]))
        .build();
    let head = header(4, 0, 38, 48);
    replay("user-text-with-a-path", &bytes, |_| Outcome {
        header: Ok(head),
        footer: NOT_A_FOOTER,
        tag: Ok(tag(
            head,
            vec![frame(
                b"TXXX",
                10,
                FrameBody::UserText {
                    description: plain("filename"),
                    values: vec![plain(path)],
                },
            )],
        )),
        octets: vec![],
    });
}
