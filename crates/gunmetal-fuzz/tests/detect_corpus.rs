//! Replays the committed format-detection fuzz corpus through its harness
//! on stable Rust, so that `cargo test` and the gate run every seed and
//! every reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/detect` has a test here that pins its exact
//! bytes and the exact outcome the harness reports for it. A seed's first
//! octet chooses the extension hint from `HINTS`. Commit a fuzzing
//! reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::formats::detect::{DetectError, Detected, Format};
use gunmetal_core::parse::{ParseFault, ReadRequest};
use gunmetal_fuzz::detect::{Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/detect")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 15] = [
    "eight-leading-tags",
    "empty",
    "flac-named-mp3",
    "id3v2-past-the-end",
    "id3v2-then-flac",
    "lyrics-named-lrc",
    "mp3-named-flac",
    "mp4-named-m4a",
    "one-byte",
    "playlist-named-m3u",
    "png-named-flac",
    "png-named-jpg",
    "size-not-syncsafe",
    "text-named-flac",
    "wav-named-txt",
];

/// The start of a FLAC stream: the marker and a last STREAMINFO block for
/// 4,096-sample blocks at 44.1 kHz, two channels and 16 bits. 42 octets.
const FLAC: [&[u8]; 4] = [
    b"fLaC\x80\x00\x00\x22",
    b"\x10\x00\x10\x00\x00\x00\x00\x00\x00\x00",
    b"\x0A\xC4\x42\xF0\x00\x00\x00\x00",
    &[0; 16],
];

/// An MPEG-1 Layer III frame header at 128 kbit/s and 44.1 kHz, and 32
/// octets of side information. 36 octets.
const MP3: [&[u8]; 2] = [b"\xFF\xFB\x90\x44", &[0; 32]];

/// The PNG signature and the IHDR chunk of a 1×1 RGBA image. 33 octets.
const PNG: [&[u8]; 4] = [
    b"\x89PNG\r\n\x1A\n",
    b"\x00\x00\x00\x0DIHDR",
    b"\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00",
    &[0; 4],
];

/// Text such as `/etc/passwd`. 32 octets.
const TEXT: &[u8] = b"root:x:0:0:root:/root:/bin/bash\n";

/// An extended M3U playlist. 59 octets.
const PLAYLIST: &[u8] = b"#EXTM3U\n#EXTINF:215,Artist - Title\nMusic/Artist/Title.flac\n";

/// LRC lyrics. 61 octets.
const LYRICS: &[u8] = b"[ar:Artist]\n[ti:Title]\n[00:12.34]First line\n[00:15.00]Second\n";

/// An `ID3v2.3` tag with an empty body. 10 octets.
const EMPTY_TAG: &[u8] = b"ID3\x03\x00\x00\x00\x00\x00\x00";

/// The seed whose first octet is `hint` followed by `parts`.
fn seed(hint: u8, parts: &[&[u8]]) -> Vec<u8> {
    let mut bytes = vec![hint];
    bytes.extend(parts.concat());
    bytes
}

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

const fn read(offset: u64, len: u32) -> ReadRequest {
    ReadRequest { offset, len }
}

/// The outcome of one read of `len` octets at the start of the file, named
/// `hint`, that returned `result`.
fn once(hint: Option<&'static str>, len: u32, result: Result<Detected, DetectError>) -> Outcome {
    Outcome {
        hint,
        reads: vec![read(0, len)],
        result,
    }
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "it builds the whole result a detection is compared with"
)]
const fn found(format: Format, start: u64) -> Result<Detected, DetectError> {
    Ok(Detected { format, start })
}

const fn unknown(offset: u64) -> Result<Detected, DetectError> {
    Err(DetectError::Unknown { offset })
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

/// An empty input is an empty file with no hint, which is never read.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_the_empty_input() {
    replay(
        "empty",
        &[],
        &Outcome {
            hint: None,
            reads: vec![],
            result: unknown(0),
        },
    );
}

/// The plan's FLAC named `.mp3`.
///
/// Verifies: SEC-MED-028, SEC-MED-011, SEC-HIS-017
#[test]
fn replays_flac_named_mp3() {
    replay(
        "flac-named-mp3",
        &seed(2, &FLAC),
        &once(Some("MP3"), 42, found(Format::Flac, 0)),
    );
}

/// The plan's MP3 named `.flac`.
///
/// Verifies: SEC-MED-028, SEC-MED-011, SEC-HIS-017
#[test]
fn replays_mp3_named_flac() {
    replay(
        "mp3-named-flac",
        &seed(1, &MP3),
        &once(Some("flac"), 36, found(Format::Mpeg, 0)),
    );
}

/// The plan's text file named `.flac`, and SEC-HIS-017's `passwd.flac`.
///
/// Verifies: SEC-MED-028, SEC-MED-011, SEC-HIS-017
#[test]
fn replays_text_named_flac() {
    replay(
        "text-named-flac",
        &seed(1, &[TEXT]),
        &once(Some("flac"), 32, unknown(0)),
    );
}

/// The plan's PNG named `cover.jpg`.
///
/// Verifies: SEC-MED-028, SEC-MED-011, SEC-HIS-017
#[test]
fn replays_png_named_jpg() {
    replay(
        "png-named-jpg",
        &seed(4, &PNG),
        &once(Some("JPG"), 33, found(Format::Png, 0)),
    );
}

/// SEC-MED-011's PNG named `.flac`.
///
/// Verifies: SEC-MED-028, SEC-MED-011
#[test]
fn replays_png_named_flac() {
    replay(
        "png-named-flac",
        &seed(1, &PNG),
        &once(Some("flac"), 33, unknown(0)),
    );
}

/// The plan's `ID3v2` tag followed by FLAC: a version 2.4 tag with a
/// four-octet body.
///
/// Verifies: SEC-MED-028, SEC-MED-011
#[test]
fn replays_an_id3v2_tag_then_flac() {
    let tag: &[u8] = b"ID3\x04\x00\x00\x00\x00\x00\x04\x00\x00\x00\x00";
    let mut parts = vec![tag];
    parts.extend(FLAC);
    replay(
        "id3v2-then-flac",
        &seed(0, &parts),
        &Outcome {
            hint: None,
            reads: vec![read(0, 56), read(14, 42)],
            result: found(Format::Flac, 14),
        },
    );
}

/// The plan's `ID3v2` tag whose declared size, 100 octets, runs past the
/// file.
///
/// Verifies: SEC-MED-028, SEC-TM-032
#[test]
fn replays_an_id3v2_tag_that_runs_past_the_end() {
    replay(
        "id3v2-past-the-end",
        &seed(2, &[b"ID3\x03\x00\x00\x00\x00\x00\x64", &[0; 5]]),
        &once(
            Some("MP3"),
            15,
            Err(DetectError::Fault(ParseFault::Truncated {
                offset: 0,
                needed: 110,
                available: 15,
            })),
        ),
    );
}

/// The plan's file of one byte.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_file_of_one_byte() {
    replay("one-byte", b"\x00f", &once(None, 1, unknown(0)));
}

/// Verifies: SEC-MED-028, SEC-MED-011
#[test]
fn replays_a_playlist_named_m3u() {
    replay(
        "playlist-named-m3u",
        &seed(6, &[PLAYLIST]),
        &once(Some("m3u"), 59, found(Format::M3u, 0)),
    );
}

/// Verifies: SEC-MED-028, SEC-MED-011
#[test]
fn replays_lyrics_named_lrc() {
    replay(
        "lyrics-named-lrc",
        &seed(5, &[LYRICS]),
        &once(Some("lrc"), 61, found(Format::Lrc, 0)),
    );
}

/// Eight empty tags in front of FLAC: the eight reads the budget allows
/// are spent on the tags. The first octet, 8, chooses no hint, as 0 does.
///
/// Verifies: SEC-MED-028, SEC-MED-007
#[test]
fn replays_eight_leading_tags() {
    let mut parts = vec![EMPTY_TAG; 8];
    parts.push(b"fLaC");
    replay(
        "eight-leading-tags",
        &seed(8, &parts),
        &Outcome {
            hint: None,
            reads: vec![
                read(0, 84),
                read(10, 74),
                read(20, 64),
                read(30, 54),
                read(40, 44),
                read(50, 34),
                read(60, 24),
                read(70, 14),
            ],
            result: Err(DetectError::Fault(ParseFault::BudgetExceeded {
                offset: 80,
            })),
        },
    );
}

/// An `ftyp` box with major brand `M4A `, then an empty `free` box.
///
/// Verifies: SEC-MED-028, SEC-MED-011
#[test]
fn replays_mp4_named_m4a() {
    replay(
        "mp4-named-m4a",
        &seed(
            3,
            &[
                b"\x00\x00\x00\x1CftypM4A \x00\x00\x00\x00M4A mp42isom",
                b"\x00\x00\x00\x08free",
            ],
        ),
        &once(Some("m4a"), 36, found(Format::Mp4, 0)),
    );
}

/// An `ID3v2.4` tag whose size has an octet with its high bit set.
///
/// Verifies: SEC-MED-028, SEC-TM-032
#[test]
fn replays_a_tag_size_that_is_not_syncsafe() {
    replay(
        "size-not-syncsafe",
        &seed(0, &[b"ID3\x04\x00\x00\x00\x80\x00\x05", &[0; 5]]),
        &once(
            None,
            15,
            Err(DetectError::Fault(ParseFault::NotSyncsafe {
                offset: 6,
                octets: [0, 0x80, 0, 5],
            })),
        ),
    );
}

/// A WAV file named `.txt`, a name outside the allowlist.
///
/// Verifies: SEC-MED-028, SEC-MED-011
#[test]
fn replays_wav_named_txt() {
    replay(
        "wav-named-txt",
        &seed(
            7,
            &[
                b"RIFF\x24\x00\x00\x00WAVE",
                b"fmt \x10\x00\x00\x00\x01\x00\x02\x00\x44\xAC\x00\x00\x10\xB1\x02\x00\x04\x00\x10\x00",
                b"data\x00\x00\x00\x00",
            ],
        ),
        &once(Some("txt"), 44, unknown(0)),
    );
}
