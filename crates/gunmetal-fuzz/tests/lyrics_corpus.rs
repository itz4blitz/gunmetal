//! Replays the committed lyrics fuzz corpus through its harness on stable
//! Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/lyrics` has a test here that pins its exact
//! bytes and the exact outcome the harness reports for it. Commit a fuzzing
//! reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::lyrics::{Line, Lyrics, Parsed, Source, Tag, TagKey, Word, WordLine};
use gunmetal_core::parse::{LimitKind, ParseFault};
use gunmetal_fuzz::lyrics::{Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/lyrics")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 6] = [
    "empty",
    "enhanced",
    "past-limits",
    "plain-stanzas",
    "sylt-frames",
    "sylt-words",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

/// Checks that seed `name` holds `bytes` and that both readers read it:
/// LRC as `lrc` and `SYLT` as `sylt`.
fn replay_read(name: &str, bytes: &[u8], lrc: Parsed, sylt: Parsed) {
    replay(
        name,
        bytes,
        &Outcome {
            lrc: Ok(lrc),
            sylt: Ok(sylt),
        },
    );
}

/// `lyrics` from `source`, with no tags, no offset and no limit passed.
fn parsed(lyrics: Lyrics, source: Source) -> Parsed {
    Parsed {
        lyrics,
        source,
        tags: Vec::new(),
        offset: 0,
        over_limit: Vec::new(),
    }
}

fn plain(lines: &[&str]) -> Lyrics {
    Lyrics::Plain(lines.iter().map(|&line| line.to_owned()).collect())
}

fn lines(lines: &[(u32, &str)]) -> Lyrics {
    Lyrics::Lines(
        lines
            .iter()
            .map(|&(at, text)| Line {
                at,
                text: text.to_owned(),
            })
            .collect(),
    )
}

fn words(lines: &[(u32, &[(u32, &str)])]) -> Lyrics {
    Lyrics::Words(
        lines
            .iter()
            .map(|&(at, words)| WordLine {
                at,
                words: words
                    .iter()
                    .map(|&(at, text)| Word {
                        at,
                        text: text.to_owned(),
                    })
                    .collect(),
            })
            .collect(),
    )
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

/// An empty input is a file with no lyrics, and one `SYLT` entry at 0 with
/// no text.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_the_empty_input() {
    replay_read(
        "empty",
        b"",
        parsed(plain(&[]), Source::LrcFile),
        parsed(lines(&[(0, "")]), Source::Sylt),
    );
}

/// Enhanced LRC with a title, an offset of a quarter second and a line
/// without word stamps. Its first octet, `[`, is odd, so its `SYLT` reading
/// counts MPEG frames: 91,000 frames of 1,152 samples at 44.1 kHz are
/// 2,377,142 ms.
///
/// Verifies: SEC-MED-028, SEC-MED-049, SEC-API-090, SEC-HIS-036
#[test]
fn replays_enhanced_lrc_with_tags_and_an_offset() {
    let text = "[ti:Song]\n[offset:+250]\n[00:01.00]<00:01.00>Hel<00:01.50>lo <00:02.00>world\n[00:03.50]Plain\n";
    replay_read(
        "enhanced",
        text.as_bytes(),
        Parsed {
            lyrics: words(&[
                (750, &[(750, "Hel"), (1_250, "lo "), (1_750, "world")]),
                (3_250, &[(3_250, "Plain")]),
            ]),
            source: Source::LrcFile,
            tags: vec![
                Tag {
                    key: TagKey::Title,
                    value: "Song".to_owned(),
                },
                Tag {
                    key: TagKey::Offset,
                    value: "+250".to_owned(),
                },
            ],
            offset: 250,
            over_limit: Vec::new(),
        },
        parsed(
            lines(&[(
                2_377_142,
                "ti:Song]\n[offset:+250]\n[00:01.00]<00:01.00>Hel<00:01.50>lo <00:02.00>world\n[00:03.50]Plain\n",
            )]),
            Source::Sylt,
        ),
    );
}

/// Plain lyrics in two stanzas. `P` is even, so its `SYLT` reading is in
/// milliseconds: 80,000 for the code point 80.
///
/// Verifies: SEC-MED-028, SEC-MED-049
#[test]
fn replays_plain_stanzas() {
    replay_read(
        "plain-stanzas",
        b"Plain one\n\nPlain two\n",
        parsed(plain(&["Plain one", "", "Plain two"]), Source::LrcFile),
        parsed(lines(&[(80_000, "lain one\n\nPlain two\n")]), Source::Sylt),
    );
}

/// Lines out of order and a stamp one millisecond past 24 hours, which is
/// dropped and reported.
///
/// Verifies: SEC-MED-028, SEC-MED-049, SEC-API-090
#[test]
fn replays_a_stamp_past_24_hours() {
    replay_read(
        "past-limits",
        b"[00:02.00]b\n[1440:00.001]late\n[00:01.00]a\n",
        Parsed {
            over_limit: vec![ParseFault::LimitExceeded {
                limit: LimitKind::LyricsTimestampMs,
                value: 86_400_001,
                max: 86_400_000,
                offset: 12,
            }],
            ..parsed(lines(&[(1_000, "a"), (2_000, "b")]), Source::LrcFile)
        },
        parsed(
            lines(&[(2_377_142, "00:02.00]b\n[1440:00.001]late\n[00:01.00]a\n")]),
            Source::Sylt,
        ),
    );
}

/// Four `SYLT` entries in milliseconds, the last two starting a line with a
/// line feed, so the entries are syllables. As LRC the same octets are plain
/// lines without their control characters.
///
/// Verifies: SEC-MED-028, SEC-MED-049
#[test]
fn replays_sylt_syllables() {
    replay_read(
        "sylt-words",
        b"\x02Strang\x00\x03ers\x00\x05\nnight\x00\x04\nin",
        parsed(plain(&["Strangers", "night", "in"]), Source::LrcFile),
        parsed(
            words(&[
                (2_000, &[(2_000, "Strang"), (3_000, "ers")]),
                (4_000, &[(4_000, "in")]),
                (5_000, &[(5_000, "night")]),
            ]),
            Source::Sylt,
        ),
    );
}

/// Two `SYLT` entries in MPEG frames: 1,000 frames are 26,122 ms and 3,000
/// frames 78,367 ms.
///
/// Verifies: SEC-MED-028, SEC-MED-049
#[test]
fn replays_sylt_in_mpeg_frames() {
    replay_read(
        "sylt-frames",
        b"\x01a\x00\x03b",
        parsed(plain(&["ab"]), Source::LrcFile),
        parsed(lines(&[(26_122, "a"), (78_367, "b")]), Source::Sylt),
    );
}
