//! Replays the committed EBML fuzz corpus through its harness on stable
//! Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/ebml` has a test here that pins its exact bytes
//! and the exact outcome the harness reports for it. Commit a fuzzing
//! reproducer by adding its file and its test together.

use std::fs;
use std::path::PathBuf;

use gunmetal_core::ebml::{
    DataSize, Element, ElementError, ElementHeader, ElementId, HeaderError, Vint, VintError,
};
use gunmetal_fuzz::{EbmlOutcome, Visit, ebml};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/ebml")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 9] = [
    "ebml-header",
    "empty",
    "id-too-long",
    "id-without-size",
    "nested-past-the-walk-limit",
    "siblings-then-garbage",
    "size-beyond-input",
    "unknown-size-segment",
    "zero-first-octet",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &EbmlOutcome<'_>) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&ebml(&file), expected, "seed {name}");
}

fn ok(depth: usize, id: u32, body: &[u8]) -> Visit<'_> {
    Visit {
        depth,
        item: Ok(Element {
            id: ElementId(id),
            body,
        }),
    }
}

fn err(depth: usize, error: ElementError) -> Visit<'static> {
    Visit {
        depth,
        item: Err(error),
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
        &EbmlOutcome {
            vint: Err(VintError::Empty),
            header: Err(HeaderError::Id(VintError::Empty)),
            walk: vec![],
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_first_octet_with_no_length_marker() {
    replay(
        "zero-first-octet",
        &[0x00],
        &EbmlOutcome {
            vint: Err(VintError::InvalidWidth),
            header: Err(HeaderError::Id(VintError::InvalidWidth)),
            walk: vec![err(
                0,
                ElementError::Header {
                    offset: 0,
                    error: HeaderError::Id(VintError::InvalidWidth),
                },
            )],
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_an_id_with_no_size_after_it() {
    replay(
        "id-without-size",
        &[0x81],
        &EbmlOutcome {
            vint: Ok(Vint { value: 1, width: 1 }),
            header: Err(HeaderError::Size(VintError::Empty)),
            walk: vec![err(
                0,
                ElementError::Header {
                    offset: 0,
                    error: HeaderError::Size(VintError::Empty),
                },
            )],
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_an_id_longer_than_four_octets() {
    replay(
        "id-too-long",
        &[0x08, 0x00, 0x00, 0x00, 0x01, 0x81],
        &EbmlOutcome {
            vint: Ok(Vint { value: 1, width: 5 }),
            header: Err(HeaderError::IdTooLong { width: 5 }),
            walk: vec![err(
                0,
                ElementError::Header {
                    offset: 0,
                    error: HeaderError::IdTooLong { width: 5 },
                },
            )],
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_size_larger_than_any_input() {
    replay(
        "size-beyond-input",
        &[0xEC, 0x01, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFE, 0xAA],
        &EbmlOutcome {
            vint: Ok(Vint {
                value: 0x6C,
                width: 1,
            }),
            header: Ok(ElementHeader {
                id: ElementId(0xEC),
                size: DataSize::Known(72_057_594_037_927_934),
                header_len: 9,
            }),
            walk: vec![err(
                0,
                ElementError::BodyTruncated {
                    offset: 0,
                    id: ElementId(0xEC),
                    needed: 72_057_594_037_927_934,
                    available: 1,
                },
            )],
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_live_segment_of_unknown_size() {
    replay(
        "unknown-size-segment",
        &[
            0x18, 0x53, 0x80, 0x67, // Segment
            0x01, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, // unknown size
            0xEC, 0x80, // an empty Void inside it
        ],
        &EbmlOutcome {
            vint: Ok(Vint {
                value: 0x0853_8067,
                width: 4,
            }),
            header: Ok(ElementHeader {
                id: ElementId(0x1853_8067),
                size: DataSize::Unknown,
                header_len: 12,
            }),
            walk: vec![err(
                0,
                ElementError::UnknownSize {
                    offset: 0,
                    id: ElementId(0x1853_8067),
                },
            )],
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_siblings_followed_by_garbage() {
    replay(
        "siblings-then-garbage",
        &[0xEC, 0x80, 0xEC, 0x81, 0x00, 0x00],
        &EbmlOutcome {
            vint: Ok(Vint {
                value: 0x6C,
                width: 1,
            }),
            header: Ok(ElementHeader {
                id: ElementId(0xEC),
                size: DataSize::Known(0),
                header_len: 2,
            }),
            walk: vec![
                ok(0, 0xEC, &[]),
                ok(0, 0xEC, &[0x00]),
                // The second Void's body, read as children.
                err(
                    1,
                    ElementError::Header {
                        offset: 0,
                        error: HeaderError::Id(VintError::InvalidWidth),
                    },
                ),
                err(
                    0,
                    ElementError::Header {
                        offset: 5,
                        error: HeaderError::Id(VintError::InvalidWidth),
                    },
                ),
            ],
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_real_ebml_header() {
    const DOC_TYPE: &[u8] = b"matroska";
    replay(
        "ebml-header",
        &[
            0x1A, 0x45, 0xDF, 0xA3, 0x95, // EBML, 21 octets
            0x42, 0x86, 0x81, 0x01, // EBMLVersion 1
            0x42, 0xF7, 0x81, 0x01, // EBMLReadVersion 1
            0xEC, 0x80, // Void
            0x42, 0x82, 0x88, b'm', b'a', b't', b'r', b'o', b's', b'k', b'a', // DocType
        ],
        &EbmlOutcome {
            vint: Ok(Vint {
                value: 0x0A45_DFA3,
                width: 4,
            }),
            header: Ok(ElementHeader {
                id: ElementId(0x1A45_DFA3),
                size: DataSize::Known(21),
                header_len: 5,
            }),
            walk: vec![
                ok(
                    0,
                    0x1A45_DFA3,
                    &[
                        0x42, 0x86, 0x81, 0x01, 0x42, 0xF7, 0x81, 0x01, 0xEC, 0x80, 0x42, 0x82,
                        0x88, b'm', b'a', b't', b'r', b'o', b's', b'k', b'a',
                    ],
                ),
                ok(1, 0x4286, &[0x01]),
                // A lone 0x01 declares an eight-octet ID.
                err(
                    2,
                    ElementError::Header {
                        offset: 0,
                        error: HeaderError::Id(VintError::Truncated {
                            needed: 8,
                            available: 1,
                        }),
                    },
                ),
                ok(1, 0x42F7, &[0x01]),
                err(
                    2,
                    ElementError::Header {
                        offset: 0,
                        error: HeaderError::Id(VintError::Truncated {
                            needed: 8,
                            available: 1,
                        }),
                    },
                ),
                ok(1, 0xEC, &[]),
                ok(1, 0x4282, DOC_TYPE),
                // "ma" reads as an ID and "tr" as a size of 0x3472 octets,
                // with only "oska" left.
                err(
                    2,
                    ElementError::BodyTruncated {
                        offset: 0,
                        id: ElementId(0x6D61),
                        needed: 0x3472,
                        available: 4,
                    },
                ),
            ],
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_nesting_two_levels_deeper_than_the_walk_descends() {
    // Thirty-four BlockGroups, each holding the next. Level `k` starts at
    // octet 2k with a one-octet size covering every level inside it.
    let bytes: Vec<u8> = (0..34_u8)
        .flat_map(|level| [0xA0, 0x80 | (2 * (33 - level))])
        .collect();
    // The walk reports levels 0 to 32 and does not descend into level 32's
    // body, so level 33 never appears. Each level's body runs from the end
    // of its two-octet header to the end of the input.
    let walk = (0..=32)
        .map(|depth| ok(depth, 0xA0, &bytes[2 * depth + 2..]))
        .collect();
    replay(
        "nested-past-the-walk-limit",
        &bytes,
        &EbmlOutcome {
            vint: Ok(Vint {
                value: 0x20,
                width: 1,
            }),
            header: Ok(ElementHeader {
                id: ElementId(0xA0),
                size: DataSize::Known(66),
                header_len: 2,
            }),
            walk,
        },
    );
}
