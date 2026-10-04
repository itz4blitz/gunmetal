//! Replays the committed public-identifier fuzz corpus through its harness
//! on stable Rust, so that `cargo test` and the gate run every seed and
//! every reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/id` has a test here that pins its exact bytes
//! and the exact outcome the harness reports for it. Commit a fuzzing
//! reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::id::IdKind;
use gunmetal_fuzz::id::{Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/id")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 10] = [
    "album-of-all-ones",
    "empty",
    "letter-outside-the-alphabet",
    "more-than-128-bits",
    "one-symbol-short",
    "prefix-of-another-kind-inside",
    "track",
    "trailing-line-feed",
    "unknown-kind",
    "upper-case",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reads it as an identifier of `kind`, or of no kind.
fn replay(name: &str, bytes: &[u8], kind: Option<IdKind>) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(run(&file), Outcome { kind }, "seed {name}");
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

/// The largest identifier: 128 one bits, whose first symbol is `7`.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_an_album_identifier_of_all_ones() {
    replay(
        "album-of-all-ones",
        b"alb_7zzzzzzzzzzzzzzzzzzzzzzzzz",
        Some(IdKind::Album),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_the_empty_input() {
    replay("empty", &[], None);
}

/// Crockford's alphabet has no `l`, which reads as `1`.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_letter_outside_the_alphabet() {
    replay(
        "letter-outside-the-alphabet",
        b"trk_0000000000000000000000000l",
        None,
    );
}

/// A first symbol above `7` would need a 129th bit.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_symbols_that_spell_more_than_128_bits() {
    replay(
        "more-than-128-bits",
        b"trk_80000000000000000000000000",
        None,
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_an_identifier_one_symbol_short() {
    replay("one-symbol-short", b"trk_0000000000000000000000000", None);
}

/// Twenty-six octets after `trk_`, but `_` and `o` are not symbols.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_second_prefix_where_the_symbols_belong() {
    replay(
        "prefix-of-another-kind-inside",
        b"trk_tok_0000000000000000000000",
        None,
    );
}

/// The `TypeID` specification's example suffix, which spells the bytes
/// `0110C853 1D0952D8 D73E1194 E95B5F19`.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_track_identifier() {
    replay(
        "track",
        b"trk_0123456789abcdefghjkmnpqrs",
        Some(IdKind::Track),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_an_identifier_with_a_trailing_line_feed() {
    replay(
        "trailing-line-feed",
        b"trk_00000000000000000000000000\n",
        None,
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_an_unknown_kind() {
    replay("unknown-kind", b"xyz_00000000000000000000000000", None);
}

/// Only the lower-case spelling is an identifier.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_an_identifier_in_upper_case() {
    replay("upper-case", b"TRK_0123456789ABCDEFGHJKMNPQRS", None);
}
