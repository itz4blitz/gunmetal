//! Replays the committed corpus of the structure-aware Ogg harness on
//! stable Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding (SEC-MED-028, SEC-MED-031).
//!
//! Each seed is a description of a stream: a layout octet, a damage octet,
//! then packets as a two-octet little-endian length and that many octets. Each file in
//! `fuzz/seeds/ogg_structure` has a test here that pins its exact bytes and
//! the exact outcome the harness reports for it. Commit a fuzzing
//! reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::formats::ogg::{PacketError, PageError};
use gunmetal_core::parse::ParseFault;
use gunmetal_fuzz::ogg_structure::{Damage, Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/ogg_structure")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 12] = [
    "a-continuation-cleared",
    "a-continuation-from-nowhere",
    "a-damaged-checksum",
    "a-later-sequence-number",
    "a-lost-page",
    "a-packet-over-three-pages",
    "a-packet-over-two-pages",
    "another-stream",
    "cut-short",
    "empty",
    "no-granule",
    "three-packets-on-one-page",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

/// One page per segment and no damage or `damage`, then a packet of 255
/// octets of `A` and one of `B`: three pages of 283, 28 and 29 octets, the
/// second holding only the empty segment that ends the first packet.
fn two_pages(damage: u8) -> Vec<u8> {
    [&[0x00, damage, 255, 0][..], &[b'A'; 255], &[1, 0, b'B']].concat()
}

/// One page per segment and `damage`, then the packets `a`, `b` and `c`:
/// three pages of 29 octets each.
fn singles(damage: u8) -> [u8; 11] {
    [0x00, damage, 1, 0, b'a', 1, 0, b'b', 1, 0, b'c']
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

#[test]
fn numbers_each_kind_of_damage_by_the_low_three_bits() {
    let kinds = [
        Damage::None,
        Damage::Checksum,
        Damage::Dropped,
        Damage::Serial,
        Damage::Sequence,
        Damage::Continued,
        Damage::Granule,
        Damage::Cut,
    ];
    for (octet, kind) in (0_u8..).zip(kinds) {
        assert_eq!(Damage::of(octet), kind);
        assert_eq!(Damage::of(octet | 0xF8), kind);
    }
}

/// Verifies: SEC-MED-028
#[test]
fn replays_the_empty_input() {
    replay(
        "empty",
        &[],
        &Outcome {
            pages: 0,
            octets: 0,
            read: vec![],
            others: vec![],
            last: Ok(None),
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_three_packets_on_one_page() {
    // Up to eight segments a page: 4, 0 and 2 octets on one 36-octet page,
    // whose granule position belongs to the last packet.
    replay(
        "three-packets-on-one-page",
        b"\x07\x00\x04\x00abcd\x00\x00\x02\x00xy",
        &Outcome {
            pages: 1,
            octets: 36,
            read: vec![Ok((0, None, 4)), Ok((0, None, 0)), Ok((0, Some(3), 2))],
            others: vec![],
            last: Ok(Some(3)),
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_a_packet_over_two_pages() {
    replay(
        "a-packet-over-two-pages",
        &two_pages(0x00),
        &Outcome {
            pages: 3,
            octets: 340,
            read: vec![Ok((0, Some(1), 255)), Ok((311, Some(2), 1))],
            others: vec![],
            last: Ok(Some(2)),
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_a_packet_over_three_pages() {
    // 600 octets counting up modulo 251, so no two segments hold the same
    // octets: segments of 255, 255 and 90 on pages of 283, 283 and 118.
    let packet: Vec<u8> = (0..600_u16)
        .map(|n| u8::try_from(n % 251).expect("below 251"))
        .collect();
    replay(
        "a-packet-over-three-pages",
        &[&[0x00, 0x00, 0x58, 0x02][..], &packet].concat(),
        &Outcome {
            pages: 3,
            octets: 684,
            read: vec![Ok((0, Some(1), 600))],
            others: vec![],
            last: Ok(Some(1)),
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_a_damaged_checksum() {
    // The second page's checksum, 0xB9756403, has its lowest bit flipped.
    let crc = PageError::Crc {
        offset: 31,
        stored: 0xB975_6402,
        computed: 0xB975_6403,
    };
    replay(
        "a-damaged-checksum",
        b"\x00\x09\x03\x00abc\x02\x00de\x01\x00f",
        &Outcome {
            pages: 3,
            octets: 90,
            read: vec![
                Ok((0, Some(1), 3)),
                Err(PacketError::Page(crc)),
                Err(PacketError::Page(PageError::Skipped {
                    offset: 32,
                    len: 29,
                })),
                Err(PacketError::Gap {
                    offset: 61,
                    expected: 1,
                    found: 2,
                }),
                Ok((61, Some(3), 1)),
            ],
            others: vec![],
            last: Ok(Some(3)),
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_a_lost_page() {
    // The page that ended the first packet is gone.
    replay(
        "a-lost-page",
        &two_pages(0x0A),
        &Outcome {
            pages: 3,
            octets: 312,
            read: vec![
                Err(PacketError::Gap {
                    offset: 283,
                    expected: 1,
                    found: 2,
                }),
                Ok((283, Some(2), 1)),
            ],
            others: vec![],
            last: Ok(Some(2)),
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_a_first_page_of_another_stream() {
    replay(
        "another-stream",
        &singles(0x03),
        &Outcome {
            pages: 3,
            octets: 87,
            read: vec![Ok((29, Some(2), 1)), Ok((58, Some(3), 1))],
            others: vec![2],
            last: Ok(Some(3)),
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_a_later_sequence_number() {
    // The second page says 2, so the third, also 2, is out of order too.
    replay(
        "a-later-sequence-number",
        &singles(0x0C),
        &Outcome {
            pages: 3,
            octets: 87,
            read: vec![
                Ok((0, Some(1), 1)),
                Err(PacketError::Gap {
                    offset: 29,
                    expected: 1,
                    found: 2,
                }),
                Ok((29, Some(2), 1)),
                Err(PacketError::Gap {
                    offset: 58,
                    expected: 3,
                    found: 2,
                }),
                Ok((58, Some(3), 1)),
            ],
            others: vec![],
            last: Ok(Some(3)),
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_a_continuation_cleared() {
    // The page that should finish the first packet says it does not.
    replay(
        "a-continuation-cleared",
        &two_pages(0x0D),
        &Outcome {
            pages: 3,
            octets: 340,
            read: vec![
                Err(PacketError::Unfinished { offset: 0 }),
                Ok((283, Some(1), 0)),
                Ok((311, Some(2), 1)),
            ],
            others: vec![],
            last: Ok(Some(2)),
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_a_continuation_from_nowhere() {
    replay(
        "a-continuation-from-nowhere",
        &singles(0x0D),
        &Outcome {
            pages: 3,
            octets: 87,
            read: vec![
                Ok((0, Some(1), 1)),
                Err(PacketError::Orphan { offset: 29 }),
                Ok((58, Some(3), 1)),
            ],
            others: vec![],
            last: Ok(Some(3)),
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_a_page_with_no_granule() {
    replay(
        "no-granule",
        b"\x00\x06\x01\x00a\x01\x00b",
        &Outcome {
            pages: 2,
            octets: 58,
            read: vec![Ok((0, None, 1)), Ok((29, Some(2), 1))],
            others: vec![],
            last: Ok(Some(2)),
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-031
#[test]
fn replays_a_stream_cut_halfway_through_a_page() {
    // The stream ends 14 octets into the second page.
    replay(
        "cut-short",
        &singles(0x0F),
        &Outcome {
            pages: 3,
            octets: 43,
            read: vec![
                Ok((0, Some(1), 1)),
                Err(PacketError::Page(PageError::Fault(ParseFault::Truncated {
                    offset: 29,
                    needed: 27,
                    available: 14,
                }))),
                Err(PacketError::Page(PageError::Skipped {
                    offset: 30,
                    len: 13,
                })),
            ],
            others: vec![],
            last: Ok(Some(1)),
        },
    );
}
