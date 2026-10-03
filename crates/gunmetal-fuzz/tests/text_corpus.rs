//! Replays the committed text fuzz corpus through its harness on stable
//! Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/text` has a test here that pins its exact bytes
//! and the exact outcome the harness reports for it. Commit a fuzzing
//! reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::text::Text;
use gunmetal_fuzz::text::{Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/text")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 5] = [
    "bidi-tab-and-newline",
    "empty",
    "invalid-utf8-and-controls",
    "past-the-cap",
    "utf16-mark-and-lone-surrogate",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

fn kept(value: &str) -> Text {
    Text {
        value: value.to_owned(),
        truncated: false,
        replaced: false,
    }
}

fn replaced(value: &str) -> Text {
    Text {
        value: value.to_owned(),
        truncated: false,
        replaced: true,
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
        &Outcome {
            decoded: [kept(""), kept(""), kept(""), kept(""), kept("")],
            single: kept(""),
            multi: kept(""),
        },
    );
}

/// A title holding a right-to-left override, a tab and a line feed: the
/// single-line normaliser removes all three, the multi-line one keeps them.
/// Read as UTF-16, the same octets are four CJK, Hangul or other letters.
///
/// Verifies: SEC-MED-028, SEC-API-048, SEC-MED-013
#[test]
fn replays_a_bidi_override_with_a_tab_and_a_newline() {
    replay(
        "bidi-tab-and-newline",
        b"a\xE2\x80\xAEb\tc\n",
        &Outcome {
            decoded: [
                kept("a\u{202E}b\tc\n"),
                kept("\u{61E2}\u{80AE}\u{6209}\u{630A}"),
                kept("\u{61E2}\u{80AE}\u{6209}\u{630A}"),
                kept("\u{E261}\u{AE80}\u{962}\u{A63}"),
                // 0x80 is a C1 control in Latin-1.
                kept("a\u{E2}\u{AE}b\tc\n"),
            ],
            single: kept("abc"),
            multi: kept("a\u{202E}b\tc\n"),
        },
    );
}

/// Verifies: SEC-MED-028, SEC-API-048, SEC-MED-013
#[test]
fn replays_invalid_utf8_among_controls() {
    replay(
        "invalid-utf8-and-controls",
        b"a\xFF\0b\r\x1B\xC3(",
        &Outcome {
            decoded: [
                replaced("a\u{FFFD}b\u{FFFD}("),
                kept("\u{61FF}b\u{D1B}\u{C328}"),
                kept("\u{61FF}b\u{D1B}\u{C328}"),
                kept("\u{FF61}\u{6200}\u{1B0D}\u{28C3}"),
                kept("a\u{FF}b\u{C3}("),
            ],
            single: replaced("a\u{FFFD}b\u{FFFD}("),
            multi: replaced("a\u{FFFD}b\u{FFFD}("),
        },
    );
}

/// A little-endian byte-order mark, a letter, an unpaired high surrogate,
/// a line feed and a left-to-right mark.
///
/// Verifies: SEC-MED-028, SEC-API-048, SEC-MED-013
#[test]
fn replays_a_utf16_mark_and_a_lone_surrogate() {
    replay(
        "utf16-mark-and-lone-surrogate",
        b"\xFF\xFEa\0\0\xD8\n\0\x0E\x20",
        &Outcome {
            decoded: [
                replaced("\u{FFFD}\u{FFFD}a\u{FFFD}\n "),
                replaced("a\u{FFFD}\n\u{200E}"),
                kept("\u{FFFE}\u{6100}\u{D8}\u{A00}\u{E20}"),
                replaced("a\u{FFFD}\n\u{200E}"),
                kept("\u{FF}\u{FE}a\u{D8}\n "),
            ],
            single: replaced("\u{FFFD}\u{FFFD}a\u{FFFD} "),
            multi: replaced("\u{FFFD}\u{FFFD}a\u{FFFD}\n "),
        },
    );
}

/// Forty octets of 0xE9: invalid UTF-8 that grows to three octets each,
/// twenty private-use characters in UTF-16, and forty accented letters of
/// two octets each in Latin-1, against the harness's 64-octet cap.
///
/// Verifies: SEC-MED-028, SEC-API-048, SEC-MED-013
#[test]
fn replays_text_that_grows_past_the_cap() {
    let replacements = Text {
        value: "\u{FFFD}".repeat(21),
        truncated: true,
        replaced: true,
    };
    let private_use = kept(&"\u{E9E9}".repeat(20));
    replay(
        "past-the-cap",
        &[0xE9; 40],
        &Outcome {
            decoded: [
                replacements.clone(),
                private_use.clone(),
                private_use.clone(),
                private_use,
                Text {
                    value: "\u{E9}".repeat(32),
                    truncated: true,
                    replaced: false,
                },
            ],
            single: replacements.clone(),
            multi: replacements,
        },
    );
}
