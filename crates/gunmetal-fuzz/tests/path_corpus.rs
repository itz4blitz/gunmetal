//! Replays the committed path fuzz corpus through its harness on stable
//! Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/path` has a test here that pins its exact bytes
//! and the exact outcome the harness reports for it. Commit a fuzzing
//! reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::path::PathError;
use gunmetal_fuzz::path::{Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/path")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 10] = [
    "canonical-beneath-the-root",
    "dot-and-parent-inside",
    "empty",
    "filesystem-root",
    "not-canonical",
    "nul-in-a-name",
    "parent-climbs-out",
    "prefix-of-the-root-name",
    "single-name",
    "unicode-escapes-and-invalid-utf8",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

/// The names of a path, as owned byte strings.
fn names(names: &[&[u8]]) -> Vec<Vec<u8>> {
    names.iter().map(|name| name.to_vec()).collect()
}

/// The outcome for an input that is not an absolute path.
fn relative(
    split: Result<(Vec<Vec<u8>>, &str), PathError>,
    whole: Result<Vec<Vec<u8>>, PathError>,
    shown: &str,
) -> Outcome {
    Outcome {
        split: split.map(|(names, text)| (names, text.to_owned())),
        whole,
        absolute: Err(PathError::NotAbsolute),
        beneath_music: None,
        shown: shown.to_owned(),
    }
}

/// The outcome for an input that starts with `/`, which neither reading as
/// a relative path accepts.
fn absolute(
    whole: PathError,
    absolute: Result<(Vec<Vec<u8>>, &str), PathError>,
    beneath_music: Option<Vec<Vec<u8>>>,
    shown: &str,
) -> Outcome {
    Outcome {
        split: Err(PathError::EmptyName { component: 0 }),
        whole: Err(whole),
        absolute: absolute.map(|(names, text)| (names, text.to_owned())),
        beneath_music,
        shown: shown.to_owned(),
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
        b"",
        &relative(
            Err(PathError::EmptyName { component: 0 }),
            Err(PathError::EmptyName { component: 0 }),
            "",
        ),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_single_name() {
    replay(
        "single-name",
        b"cover.jpg",
        &relative(
            Ok((names(&[b"cover.jpg"]), "cover.jpg")),
            Ok(names(&[b"cover.jpg"])),
            "cover.jpg",
        ),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_dot_and_a_parent_that_stay_inside() {
    replay(
        "dot-and-parent-inside",
        b"a/./b/../c",
        &relative(
            Ok((names(&[b"a", b"c"]), "a/c")),
            Err(PathError::Separator {
                component: 0,
                offset: 1,
            }),
            "a/./b/../c",
        ),
    );
}

/// The Matroska attachment name of Jellyfin CVE-2026-49246.
///
/// Verifies: SEC-MED-028, SEC-MED-039
#[test]
fn replays_a_parent_that_climbs_out() {
    replay(
        "parent-climbs-out",
        b"../../etc/cron.d/x",
        &relative(
            Err(PathError::Escapes { component: 0 }),
            Err(PathError::Separator {
                component: 0,
                offset: 2,
            }),
            "../../etc/cron.d/x",
        ),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_nul_in_a_name() {
    replay(
        "nul-in-a-name",
        b"a/b\0c",
        &relative(
            Err(PathError::Nul {
                component: 1,
                offset: 1,
            }),
            Err(PathError::Separator {
                component: 0,
                offset: 1,
            }),
            "a/b\\u{0}c",
        ),
    );
}

/// Verifies: SEC-MED-028, SEC-MED-040
#[test]
fn replays_unicode_escapes_and_invalid_utf8() {
    replay(
        "unicode-escapes-and-invalid-utf8",
        b"Bj\xF6rk/two\nlines\x1B[0m",
        &relative(
            Ok((
                names(&[b"Bj\xF6rk", b"two\nlines\x1B[0m"]),
                "Bj\u{FFFD}rk/two\\u{a}lines\\u{1b}[0m",
            )),
            Err(PathError::Separator {
                component: 0,
                offset: 5,
            }),
            "Bj\u{FFFD}rk/two\\u{a}lines\\u{1b}[0m",
        ),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_the_filesystem_root() {
    replay(
        "filesystem-root",
        b"/",
        &absolute(
            PathError::Separator {
                component: 0,
                offset: 0,
            },
            Ok((Vec::new(), "/")),
            None,
            "/",
        ),
    );
}

/// Verifies: SEC-MED-028, SEC-MED-040
#[test]
fn replays_a_canonical_path_beneath_the_root() {
    replay(
        "canonical-beneath-the-root",
        b"/srv/music/Bj\xF6rk/x.flac",
        &absolute(
            PathError::Separator {
                component: 0,
                offset: 0,
            },
            Ok((
                names(&[b"srv", b"music", b"Bj\xF6rk", b"x.flac"]),
                "/srv/music/Bj\u{FFFD}rk/x.flac",
            )),
            Some(names(&[b"Bj\xF6rk", b"x.flac"])),
            "/srv/music/Bj\u{FFFD}rk/x.flac",
        ),
    );
}

/// Tautulli's CVE-2025-58761 came from a string-prefix check that put
/// `/srv/musicals` beneath `/srv/music`.
///
/// Verifies: SEC-MED-028, SEC-TM-043
#[test]
fn replays_a_prefix_of_the_root_name() {
    replay(
        "prefix-of-the-root-name",
        b"/srv/musicals/x",
        &absolute(
            PathError::Separator {
                component: 0,
                offset: 0,
            },
            Ok((names(&[b"srv", b"musicals", b"x"]), "/srv/musicals/x")),
            None,
            "/srv/musicals/x",
        ),
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_an_absolute_path_that_is_not_canonical() {
    replay(
        "not-canonical",
        b"/srv//music/../x",
        &absolute(
            PathError::Separator {
                component: 0,
                offset: 0,
            },
            Err(PathError::EmptyName { component: 1 }),
            None,
            "/srv//music/../x",
        ),
    );
}
