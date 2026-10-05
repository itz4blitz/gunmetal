//! Replays the committed search-segment fuzz corpus through its harness on
//! stable Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/search_segment` has a test here that pins its
//! exact bytes and the exact outcome the harness reports for it. Commit a
//! fuzzing reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::id::{IdKind, PublicId};
use gunmetal_core::parse::ParseFault;
use gunmetal_core::search::{DocRef, Hit, IndexError, Match};
use gunmetal_fuzz::search_segment::{Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/search_segment")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 10] = [
    "backwards-posting",
    "empty",
    "four-billion-docs",
    "long-number",
    "no-docs",
    "no-postings",
    "not-a-segment",
    "repeated-posting",
    "two-docs",
    "unknown-doc",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Result<Vec<Hit>, IndexError>) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!((name, file.as_slice()), (name, bytes));
    assert_eq!(
        (name, run(&file)),
        (
            name,
            Outcome {
                hits: expected.clone()
            }
        )
    );
}

/// The hit for the document whose identifier is written `text`.
fn hit(text: &str, kind: IdKind, matched: Match) -> Hit {
    let id = PublicId::parse(text, kind).expect("the identifier is well formed");
    Hit {
        doc: DocRef::new(id).expect("the identifier is a document's"),
        matched,
    }
}

/// Verifies: SEC-MED-028
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
        &Err(IndexError::Fault(ParseFault::Truncated {
            offset: 0,
            needed: 4,
            available: 0,
        })),
    );
}

/// A track "Go" played 300 times, with "ol" elsewhere, and an artist
/// "Go Go": the probe "go" is the track's whole title and the start of the
/// artist's.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_segment_of_two_documents() {
    replay(
        "two-docs",
        b"GMSI\x01\
          \x02\
          trk_00000000000000000000000001\xAC\x02\x01\
          art_00000000000000000000000002\x00\x02\
          \x02\
          \x02go\x03\x00\x00\x01\x00\x00\x01\
          \x02ol\x01\x00\xFF",
        &Ok(vec![
            hit(
                "trk_00000000000000000000000001",
                IdKind::Track,
                Match::WholeTitle,
            ),
            hit(
                "art_00000000000000000000000002",
                IdKind::Artist,
                Match::TitleStart,
            ),
        ]),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_segment_with_no_documents() {
    replay("no-docs", b"GMSI\x01\x00\x00", &Ok(vec![]));
}

/// The first eight octets of a FLAC file.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_octets_that_are_not_a_segment() {
    replay(
        "not-a-segment",
        b"fLaC\x00\x00\x00\x22",
        &Err(IndexError::NotASegment { found: *b"fLaC" }),
    );
}

/// A count of `u32::MAX` documents and not one of them: the reader sizes
/// nothing from the count and stops at the first document that is missing.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_count_of_four_billion_documents() {
    replay(
        "four-billion-docs",
        b"GMSI\x01\xFF\xFF\xFF\xFF\x0F",
        &Err(IndexError::Fault(ParseFault::Truncated {
            offset: 10,
            needed: 30,
            available: 0,
        })),
    );
}

/// No documents, and a term "a" with a posting for document 0.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_posting_for_a_document_that_is_not_there() {
    replay(
        "unknown-doc",
        b"GMSI\x01\x00\x01\x01a\x01\x00\x00",
        &Err(IndexError::UnknownDoc { offset: 10 }),
    );
}

/// One track, and a term "a" with a count of no postings.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_term_with_no_postings() {
    replay(
        "no-postings",
        b"GMSI\x01\
          \x01\
          trk_00000000000000000000000001\x00\x01\
          \x01\
          \x01a\x00",
        &Err(IndexError::NoPostings { offset: 39 }),
    );
}

/// One track "Go", and the term "go" with the posting for the title's
/// first place written twice.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_posting_written_twice() {
    replay(
        "repeated-posting",
        b"GMSI\x01\
          \x01\
          trk_00000000000000000000000001\x00\x01\
          \x01\
          \x02go\x02\x00\x00\x00\x00",
        &Err(IndexError::PostingOrder { offset: 45 }),
    );
}

/// One track "Go Go", and the term "go" with the posting for the title's
/// second place before the posting for its first.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_postings_that_go_backwards() {
    replay(
        "backwards-posting",
        b"GMSI\x01\
          \x01\
          trk_00000000000000000000000001\x00\x02\
          \x01\
          \x02go\x02\x00\x01\x00\x00",
        &Err(IndexError::PostingOrder { offset: 45 }),
    );
}

/// A document count of zero written in two octets.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_number_with_a_needless_octet() {
    replay(
        "long-number",
        b"GMSI\x01\x80\x00\x00",
        &Err(IndexError::BadNumber { offset: 5 }),
    );
}
