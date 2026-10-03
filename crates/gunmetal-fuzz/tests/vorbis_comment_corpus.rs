//! Replays the committed Vorbis comment fuzz corpus through its harness on
//! stable Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/vorbis_comment` has a test here that pins its
//! exact bytes and the exact outcome the harness reports for it, under the
//! harness's lowered limits. Commit a fuzzing reproducer by adding its file
//! and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::base64::B64Error;
use gunmetal_core::formats::vorbis_comment::{
    Comments, Field, FieldProblem, Picture, PictureError,
};
use gunmetal_core::parse::{LimitKind, ParseFault};
use gunmetal_core::text::Text;
use gunmetal_fuzz::vorbis_comment::{Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/vorbis_comment")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 11] = [
    "comments-without-a-name",
    "count-past-the-bytes",
    "empty",
    "html-in-a-comment",
    "name-past-short-text",
    "nine-comments",
    "picture",
    "picture-value",
    "picture-with-an-invalid-character",
    "picture-wrapped-in-lines",
    "two-fields",
];

/// The base64 value of a 45-octet front cover: type 3, `image/png`, no
/// description, 1 × 1 pixels at 24 bits, and the four octets `\x89PNG`.
const COVER: &[u8; 60] = b"AAAAAwAAAAlpbWFnZS9wbmcAAAAAAAAAAQAAAAEAAAAYAAAAAAAAAASJUE5H";

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

/// Text that was neither cut nor repaired.
fn text(value: &str) -> Text {
    Text {
        value: value.to_owned(),
        truncated: false,
        replaced: false,
    }
}

/// A block with an empty vendor string.
fn block(
    fields: Vec<Field>,
    pictures: Vec<Picture>,
    problems: Vec<FieldProblem>,
    end: u64,
) -> Comments {
    Comments {
        vendor: text(""),
        fields,
        pictures,
        problems,
        end,
    }
}

/// The reference to [`COVER`], whose value of `len` octets starts at
/// `offset`.
fn cover_at(offset: u64, len: u64) -> Picture {
    Picture {
        offset,
        len,
        kind: 3,
        mime: text("image/png"),
        description: text(""),
        width: 1,
        height: 1,
        depth: 24,
        colours: 0,
        data_len: 4,
    }
}

/// A block of 95 octets holding one picture comment whose value is
/// `value`: no vendor, one comment of 83 octets.
fn picture_block(value: &[u8]) -> Vec<u8> {
    [b"\0\0\0\0\x01\0\0\0S\0\0\0METADATA_BLOCK_PICTURE=", value].concat()
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
    let truncated = ParseFault::Truncated {
        offset: 0,
        needed: 4,
        available: 0,
    };
    replay(
        "empty",
        &[],
        &Outcome {
            comments: Err(truncated),
            picture: Err(PictureError::Fault(truncated)),
        },
    );
}

/// As a picture's value the block is not base64: its first octet, 0x03,
/// is outside the alphabet. The tab in the second length is set aside.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_block_of_two_fields() {
    replay(
        "two-fields",
        b"\x03\0\0\0Gmt\x02\0\0\0\x07\0\0\0album=X\t\0\0\0DATE=1999",
        &Outcome {
            comments: Ok(Comments {
                vendor: text("Gmt"),
                fields: vec![
                    Field {
                        key: "ALBUM".to_owned(),
                        value: text("X"),
                    },
                    Field {
                        key: "DATE".to_owned(),
                        value: text("1999"),
                    },
                ],
                pictures: vec![],
                problems: vec![],
                end: 35,
            }),
            picture: Err(PictureError::Base64(B64Error::InvalidByte {
                offset: 0,
                byte: 0x03,
            })),
        },
    );
}

/// 2^32 − 1 comments need four octets each, and eight are left.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_count_the_bytes_cannot_hold() {
    replay(
        "count-past-the-bytes",
        b"\x04\0\0\0abcd\xFF\xFF\xFF\xFF\x04\0\0\0A=12",
        &Outcome {
            comments: Err(ParseFault::Truncated {
                offset: 12,
                needed: 17_179_869_180,
                available: 8,
            }),
            picture: Err(PictureError::Base64(B64Error::InvalidByte {
                offset: 0,
                byte: 0x04,
            })),
        },
    );
}

/// No separator, an empty name and a `~` in a name are each recorded where
/// they are. As base64, 37 characters (the 0x0B is not whitespace) cannot
/// end an encoding.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_comments_without_a_usable_name() {
    replay(
        "comments-without-a-name",
        b"\0\0\0\0\x03\0\0\0\x0B\0\0\0NOSEPARATOR\x02\0\0\0=v\x04\0\0\0A~=1",
        &Outcome {
            comments: Ok(block(
                vec![],
                vec![],
                vec![
                    FieldProblem::NoSeparator { offset: 12 },
                    FieldProblem::EmptyKey { offset: 27 },
                    FieldProblem::KeyOctet {
                        offset: 34,
                        octet: b'~',
                    },
                ],
                37,
            )),
            picture: Err(PictureError::Base64(B64Error::InvalidLength { len: 37 })),
        },
    );
}

/// Navidrome rendered a song's comment tag as HTML and leaked the session
/// token (CVE-2026-25578). The comment is kept as text, exactly as written,
/// for the clients to show as text.
///
/// Verifies: SEC-MED-028, SEC-HIS-036
#[test]
fn replays_html_in_a_comment_as_inert_text() {
    replay(
        "html-in-a-comment",
        b"\0\0\0\0\x01\0\0\x002\0\0\0COMMENT=<img src=x onerror=alert(document.cookie)>",
        &Outcome {
            comments: Ok(block(
                vec![Field {
                    key: "COMMENT".to_owned(),
                    value: text("<img src=x onerror=alert(document.cookie)>"),
                }],
                vec![],
                vec![],
                62,
            )),
            picture: Err(PictureError::Base64(B64Error::InvalidByte {
                offset: 0,
                byte: 0,
            })),
        },
    );
}

/// The harness allows 32 octets of name; this one has 33.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_name_longer_than_short_text() {
    replay(
        "name-past-short-text",
        b"\0\0\0\0\x01\0\0\0#\0\0\0ABCDEFGHIJKLMNOPQRSTUVWXYZABCDEFG=1",
        &Outcome {
            comments: Ok(block(
                vec![],
                vec![],
                vec![FieldProblem::Limit(ParseFault::LimitExceeded {
                    limit: LimitKind::ShortText,
                    value: 33,
                    max: 32,
                    offset: 12,
                })],
                47,
            )),
            picture: Err(PictureError::Base64(B64Error::InvalidByte {
                offset: 0,
                byte: 0,
            })),
        },
    );
}

/// The harness allows eight comments; this block declares nine. As base64,
/// the tab in the count is set aside, and the 61 characters left end in one
/// `=`, which only a whole group of four may.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_one_comment_more_than_the_limit() {
    replay(
        "nine-comments",
        &[&b"\0\0\0\0\x09\0\0\0"[..], &b"\x02\0\0\0K=".repeat(9)].concat(),
        &Outcome {
            comments: Err(ParseFault::LimitExceeded {
                limit: LimitKind::TagFields,
                value: 9,
                max: 8,
                offset: 4,
            }),
            picture: Err(PictureError::Base64(B64Error::InvalidLength { len: 61 })),
        },
    );
}

/// As a picture's value, 95 characters would decode to 71 octets, and the
/// harness allows 64.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_block_holding_a_picture() {
    replay(
        "picture",
        &picture_block(COVER),
        &Outcome {
            comments: Ok(block(vec![], vec![cover_at(35, 60)], vec![], 95)),
            picture: Err(PictureError::Base64(B64Error::TooLong {
                needed: 71,
                max: 64,
            })),
        },
    );
}

/// As a block, "AAAA" declares a vendor string of 0x41414141 octets.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_pictures_value_on_its_own() {
    replay(
        "picture-value",
        COVER,
        &Outcome {
            comments: Err(ParseFault::Truncated {
                offset: 4,
                needed: 1_094_795_585,
                available: 56,
            }),
            picture: Ok(vec![0x89, b'P', b'N', b'G']),
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_picture_value_with_an_invalid_character() {
    let mut broken = *COVER;
    broken[10] = b'!';
    replay(
        "picture-with-an-invalid-character",
        &picture_block(&broken),
        &Outcome {
            comments: Ok(block(
                vec![],
                vec![],
                vec![FieldProblem::Picture {
                    offset: 35,
                    error: PictureError::Base64(B64Error::InvalidByte {
                        offset: 10,
                        byte: b'!',
                    }),
                }],
                95,
            )),
            picture: Err(PictureError::Base64(B64Error::TooLong {
                needed: 71,
                max: 64,
            })),
        },
    );
}

/// The value is wrapped after 30 characters with CR LF and ends with LF;
/// the reference counts all 63 octets.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_picture_value_wrapped_in_lines() {
    let wrapped = [&COVER[..30], b"\r\n", &COVER[30..], b"\n"].concat();
    replay(
        "picture-wrapped-in-lines",
        &[
            b"\0\0\0\0\x01\0\0\0V\0\0\0METADATA_BLOCK_PICTURE=",
            &wrapped[..],
        ]
        .concat(),
        &Outcome {
            comments: Ok(block(vec![], vec![cover_at(35, 63)], vec![], 98)),
            picture: Err(PictureError::Base64(B64Error::TooLong {
                needed: 71,
                max: 64,
            })),
        },
    );
}
