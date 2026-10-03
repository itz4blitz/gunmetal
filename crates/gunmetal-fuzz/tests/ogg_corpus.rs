//! Replays the committed Ogg fuzz corpus through its harness on stable
//! Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/ogg` has a test here that pins its exact bytes
//! and the exact outcome the harness reports for it. Commit a fuzzing
//! reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::formats::ogg::{IndexEntry, PacketError, Page, PageError};
use gunmetal_core::parse::ParseFault;
use gunmetal_fuzz::ogg::{Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/ogg")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 7] = [
    "another-version",
    "damaged-checksum",
    "empty",
    "ffmpeg-first-and-last-pages",
    "junk-then-a-cut-page",
    "orphan-then-gap",
    "two-streams",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome<'_>) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

/// A page's octets, field by field as RFC 3533 lays them out, with the
/// checksum written out.
fn page(
    flags: u8,
    granule: u64,
    serial: u32,
    sequence: u32,
    crc: u32,
    lacing: &[u8],
    body: &[u8],
) -> Vec<u8> {
    [
        &b"OggS\x00"[..],
        &[flags],
        &granule.to_le_bytes(),
        &serial.to_le_bytes(),
        &sequence.to_le_bytes(),
        &crc.to_le_bytes(),
        &[u8::try_from(lacing.len()).expect("a page holds 255 segments")],
        lacing,
        body,
    ]
    .concat()
}

/// The outcome for an input that holds no page at all.
fn no_pages(pages: Vec<Result<Page<'_>, PageError>>) -> Outcome<'_> {
    Outcome {
        packets: pages
            .iter()
            .map(|item| Err(PacketError::Page(*item.as_ref().unwrap_err())))
            .collect(),
        pages,
        serial: 0,
        others: vec![],
        last: Ok(None),
        index: (vec![], 1),
        seek: None,
    }
}

fn entry(offset: u64, granule: u64) -> IndexEntry {
    IndexEntry { offset, granule }
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
    replay("empty", &[], &no_pages(vec![]));
}

/// The 51-octet packet on the first page `ffmpeg -c:a flac -f ogg` wrote
/// for 64 frames of 16-bit mono silence at 8 kHz.
const FLAC_HEADER: [u8; 51] = [
    0x7F, b'F', b'L', b'A', b'C', 0x01, 0x00, 0x00, 0x01, b'f', b'L', b'a', b'C', 0x00, 0x00, 0x00,
    0x22, 0x00, 0x40, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x95, 0x01, 0xF4, 0x00, 0xF0, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00,
];

/// The one FLAC frame on the last page of the same file.
const FLAC_FRAME: [u8; 12] = [
    0xFF, 0xF8, 0x64, 0x08, 0x00, 0x3F, 0x5E, 0x00, 0x00, 0x00, 0xC6, 0x3C,
];

/// Verifies: SEC-MED-028
#[test]
fn replays_the_first_and_last_pages_ffmpeg_wrote() {
    // The page between them is missing, so the walk reports a gap.
    let bytes = [
        page(0x02, 0, 0x4CA9_4C6A, 0, 0x964E_C25E, &[51], &FLAC_HEADER),
        page(0x04, 64, 0x4CA9_4C6A, 2, 0xC76F_3F1D, &[12], &FLAC_FRAME),
    ]
    .concat();
    replay(
        "ffmpeg-first-and-last-pages",
        &bytes,
        &Outcome {
            pages: vec![
                Ok(Page {
                    offset: 0,
                    continued: false,
                    first: true,
                    last: false,
                    granule: Some(0),
                    serial: 0x4CA9_4C6A,
                    sequence: 0,
                    lacing: &[51],
                    body: &FLAC_HEADER,
                }),
                Ok(Page {
                    offset: 79,
                    continued: false,
                    first: false,
                    last: true,
                    granule: Some(64),
                    serial: 0x4CA9_4C6A,
                    sequence: 2,
                    lacing: &[12],
                    body: &FLAC_FRAME,
                }),
            ],
            serial: 0x4CA9_4C6A,
            packets: vec![
                Ok((0, Some(0), 51)),
                Err(PacketError::Gap {
                    offset: 79,
                    expected: 1,
                    found: 2,
                }),
                Ok((79, Some(64), 12)),
            ],
            others: vec![],
            last: Ok(Some(64)),
            index: (vec![entry(0, 0), entry(79, 64)], 1),
            seek: Some(entry(79, 64)),
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_junk_then_a_page_cut_short() {
    replay(
        "junk-then-a-cut-page",
        b"ID3\x04OggS\x00\x02\x00\x00\x00\x00",
        &no_pages(vec![
            Err(PageError::Skipped { offset: 0, len: 4 }),
            Err(PageError::Fault(ParseFault::Truncated {
                offset: 4,
                needed: 27,
                available: 10,
            })),
            Err(PageError::Skipped { offset: 5, len: 9 }),
        ]),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_page_of_another_version() {
    let bytes = [&b"OggS\x01"[..], &[0; 22]].concat();
    replay(
        "another-version",
        &bytes,
        &no_pages(vec![
            Err(PageError::Version {
                offset: 0,
                version: 1,
            }),
            Err(PageError::Skipped { offset: 1, len: 26 }),
        ]),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_page_whose_checksum_fails() {
    let bytes = [
        page(0x02, 5, 1, 0, 0xE308_FD2A, &[3], b"abc"),
        page(0x00, 6, 1, 1, 0, &[3], b"def"),
    ]
    .concat();
    let crc = PageError::Crc {
        offset: 31,
        stored: 0,
        computed: 0xBBAF_1BEE,
    };
    let skipped = PageError::Skipped {
        offset: 32,
        len: 30,
    };
    replay(
        "damaged-checksum",
        &bytes,
        &Outcome {
            pages: vec![
                Ok(Page {
                    offset: 0,
                    continued: false,
                    first: true,
                    last: false,
                    granule: Some(5),
                    serial: 1,
                    sequence: 0,
                    lacing: &[3],
                    body: b"abc",
                }),
                Err(crc),
                Err(skipped),
            ],
            serial: 1,
            packets: vec![
                Ok((0, Some(5), 3)),
                Err(PacketError::Page(crc)),
                Err(PacketError::Page(skipped)),
            ],
            others: vec![],
            last: Ok(Some(5)),
            index: (vec![entry(0, 5)], 1),
            seek: Some(entry(0, 5)),
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_two_interleaved_streams() {
    let bytes = [
        page(0x02, 10, 1, 0, 0x98B0_858B, &[1], b"a"),
        page(0x02, 20, 2, 0, 0x240F_FD65, &[2], b"bb"),
        page(0x04, 30, 1, 1, 0x5A97_1B69, &[3], b"ccc"),
    ]
    .concat();
    replay(
        "two-streams",
        &bytes,
        &Outcome {
            pages: vec![
                Ok(Page {
                    offset: 0,
                    continued: false,
                    first: true,
                    last: false,
                    granule: Some(10),
                    serial: 1,
                    sequence: 0,
                    lacing: &[1],
                    body: b"a",
                }),
                Ok(Page {
                    offset: 29,
                    continued: false,
                    first: true,
                    last: false,
                    granule: Some(20),
                    serial: 2,
                    sequence: 0,
                    lacing: &[2],
                    body: b"bb",
                }),
                Ok(Page {
                    offset: 59,
                    continued: false,
                    first: false,
                    last: true,
                    granule: Some(30),
                    serial: 1,
                    sequence: 1,
                    lacing: &[3],
                    body: b"ccc",
                }),
            ],
            serial: 1,
            packets: vec![Ok((0, Some(10), 1)), Ok((59, Some(30), 3))],
            others: vec![2],
            last: Ok(Some(30)),
            index: (vec![entry(0, 10), entry(59, 30)], 1),
            seek: Some(entry(59, 30)),
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_continued_page_with_nothing_to_continue_then_a_gap() {
    let bytes = [
        page(0x01, 40, 7, 4, 0xC296_24EF, &[2, 1], b"xxy"),
        page(0x00, 90, 7, 9, 0x4C91_CE90, &[1], b"z"),
    ]
    .concat();
    replay(
        "orphan-then-gap",
        &bytes,
        &Outcome {
            pages: vec![
                Ok(Page {
                    offset: 0,
                    continued: true,
                    first: false,
                    last: false,
                    granule: Some(40),
                    serial: 7,
                    sequence: 4,
                    lacing: &[2, 1],
                    body: b"xxy",
                }),
                Ok(Page {
                    offset: 32,
                    continued: false,
                    first: false,
                    last: false,
                    granule: Some(90),
                    serial: 7,
                    sequence: 9,
                    lacing: &[1],
                    body: b"z",
                }),
            ],
            serial: 7,
            packets: vec![
                Err(PacketError::Orphan { offset: 0 }),
                Ok((0, Some(40), 1)),
                Err(PacketError::Gap {
                    offset: 32,
                    expected: 5,
                    found: 9,
                }),
                Ok((32, Some(90), 1)),
            ],
            others: vec![],
            last: Ok(Some(90)),
            index: (vec![entry(0, 40), entry(32, 90)], 1),
            seek: Some(entry(32, 90)),
        },
    );
}
