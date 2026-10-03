//! Replays the committed path fuzz corpus through its harness on stable
//! Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/path` has a test here that pins its exact bytes
//! and the exact outcome the harness reports for it. Commit a fuzzing
//! reproducer by adding its file and its test together. The `stored`
//! digests were computed with `sha256sum` over the seed files.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::path::{Overlap, PathError, RootRefusal};
use gunmetal_fuzz::path::{Outcome, Verdict, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/path")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 20] = [
    "another-library",
    "approved-target",
    "canonical-beneath-the-root",
    "climbs-out-to-the-approved-root",
    "dot",
    "dot-and-parent-inside",
    "empty",
    "filesystem-root",
    "inside-the-data-directory",
    "not-canonical",
    "nul-in-a-name",
    "parent-climbs-out",
    "parent-of-the-data-directory",
    "passwd-link",
    "prefix-of-the-root-name",
    "single-name",
    "system-directory",
    "the-data-directory",
    "track-identifier",
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

/// The reading of an input that starts with `/` as a relative path: its
/// first name is empty.
const SPLIT_FROM_ROOT: PathError = PathError::EmptyName { component: 0 };

/// The reading of an input that starts with `/` as one name.
const WHOLE_FROM_ROOT: PathError = PathError::Separator {
    component: 0,
    offset: 0,
};

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

/// An empty link leads to its own directory.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_the_empty_input() {
    replay(
        "empty",
        b"",
        &Outcome {
            split: Err(PathError::EmptyName { component: 0 }),
            whole: Err(PathError::EmptyName { component: 0 }),
            absolute: Err(PathError::NotAbsolute),
            beneath_music: None,
            refusal: None,
            link: Ok(names(&[b"srv", b"music", b"Rock", b"Album"])),
            verdict: Some(Verdict::Own(names(&[b"Rock", b"Album"]))),
            shown: String::new(),
            stored: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_owned(),
            stored_id: None,
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_single_name() {
    replay(
        "single-name",
        b"cover.jpg",
        &Outcome {
            split: Ok((names(&[b"cover.jpg"]), "cover.jpg".to_owned())),
            whole: Ok(names(&[b"cover.jpg"])),
            absolute: Err(PathError::NotAbsolute),
            beneath_music: None,
            refusal: None,
            link: Ok(names(&[b"srv", b"music", b"Rock", b"Album", b"cover.jpg"])),
            verdict: Some(Verdict::Own(names(&[b"Rock", b"Album", b"cover.jpg"]))),
            shown: "cover.jpg".to_owned(),
            stored: "57aaf135e3916d328888bca57e4e8dcdf4ea65db066dd2e8d6dc0b29f6a35fc8".to_owned(),
            stored_id: None,
        },
    );
}

/// `.` alone is the root itself, read either way, and a link holding it
/// leads to its own directory.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_a_dot() {
    replay(
        "dot",
        b".",
        &Outcome {
            split: Ok((Vec::new(), String::new())),
            whole: Ok(Vec::new()),
            absolute: Err(PathError::NotAbsolute),
            beneath_music: None,
            refusal: None,
            link: Ok(names(&[b"srv", b"music", b"Rock", b"Album"])),
            verdict: Some(Verdict::Own(names(&[b"Rock", b"Album"]))),
            shown: ".".to_owned(),
            stored: "cdb4ee2aea69cc6a83331bbe96dc2caa9a299d21329efb0336fc02a82e1839a8".to_owned(),
            stored_id: None,
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_dot_and_a_parent_that_stay_inside() {
    replay(
        "dot-and-parent-inside",
        b"a/./b/../c",
        &Outcome {
            split: Ok((names(&[b"a", b"c"]), "a/c".to_owned())),
            whole: Err(PathError::Separator {
                component: 0,
                offset: 1,
            }),
            absolute: Err(PathError::NotAbsolute),
            beneath_music: None,
            refusal: None,
            link: Ok(names(&[b"srv", b"music", b"Rock", b"Album", b"a", b"c"])),
            verdict: Some(Verdict::Own(names(&[b"Rock", b"Album", b"a", b"c"]))),
            shown: "a/./b/../c".to_owned(),
            stored: "bce8bdca6e27450b4c5537b51d3350f0e1a222208df8b5f83baaa76bba4e37d7".to_owned(),
            stored_id: None,
        },
    );
}

/// The Matroska attachment name of Jellyfin CVE-2026-49246. As a link two
/// directories down, it stays inside the library.
///
/// Verifies: SEC-MED-028, SEC-MED-039
#[test]
fn replays_a_parent_that_climbs_out() {
    replay(
        "parent-climbs-out",
        b"../../etc/cron.d/x",
        &Outcome {
            split: Err(PathError::Escapes { component: 0 }),
            whole: Err(PathError::Separator {
                component: 0,
                offset: 2,
            }),
            absolute: Err(PathError::NotAbsolute),
            beneath_music: None,
            refusal: None,
            link: Ok(names(&[b"srv", b"music", b"etc", b"cron.d", b"x"])),
            verdict: Some(Verdict::Own(names(&[b"etc", b"cron.d", b"x"]))),
            shown: "../../etc/cron.d/x".to_owned(),
            stored: "ea88c59c899b43a4d4fae1388fbb74066132bb92dc7a870987a00aeda58e2111".to_owned(),
            stored_id: None,
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_nul_in_a_name() {
    replay(
        "nul-in-a-name",
        b"a/b\0c",
        &Outcome {
            split: Err(PathError::Nul {
                component: 1,
                offset: 1,
            }),
            whole: Err(PathError::Separator {
                component: 0,
                offset: 1,
            }),
            absolute: Err(PathError::NotAbsolute),
            beneath_music: None,
            refusal: None,
            link: Err(PathError::Nul {
                component: 1,
                offset: 1,
            }),
            verdict: None,
            shown: "a/b\\u{0}c".to_owned(),
            stored: "51a83b4c5b73e29f34005cdf40dd6e6f39d8d610ffe5b0f3fb3729d2b2a34023".to_owned(),
            stored_id: None,
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-040
#[test]
fn replays_unicode_escapes_and_invalid_utf8() {
    replay(
        "unicode-escapes-and-invalid-utf8",
        b"Bj\xF6rk/two\nlines\x1B[0m",
        &Outcome {
            split: Ok((
                names(&[b"Bj\xF6rk", b"two\nlines\x1B[0m"]),
                "Bj\u{FFFD}rk/two\\u{a}lines\\u{1b}[0m".to_owned(),
            )),
            whole: Err(PathError::Separator {
                component: 0,
                offset: 5,
            }),
            absolute: Err(PathError::NotAbsolute),
            beneath_music: None,
            refusal: None,
            link: Ok(names(&[
                b"srv",
                b"music",
                b"Rock",
                b"Album",
                b"Bj\xF6rk",
                b"two\nlines\x1B[0m",
            ])),
            verdict: Some(Verdict::Own(names(&[
                b"Rock",
                b"Album",
                b"Bj\xF6rk",
                b"two\nlines\x1B[0m",
            ]))),
            shown: "Bj\u{FFFD}rk/two\\u{a}lines\\u{1b}[0m".to_owned(),
            stored: "ef3418ceb1549d6ae7a1e672496763acc3310aade1efd24cec3924b3dd969d9b".to_owned(),
            stored_id: None,
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-037
#[test]
fn replays_the_filesystem_root() {
    replay(
        "filesystem-root",
        b"/",
        &Outcome {
            split: Err(SPLIT_FROM_ROOT),
            whole: Err(WHOLE_FROM_ROOT),
            absolute: Ok((Vec::new(), "/".to_owned())),
            beneath_music: None,
            refusal: Some(RootRefusal::FilesystemRoot),
            link: Ok(Vec::new()),
            verdict: Some(Verdict::Outside),
            shown: "/".to_owned(),
            stored: "8a5edab282632443219e051e4ade2d1d5bbc671c781051bf1437897cbdfea0f1".to_owned(),
            stored_id: None,
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-040
#[test]
fn replays_a_canonical_path_beneath_the_root() {
    replay(
        "canonical-beneath-the-root",
        b"/srv/music/Bj\xF6rk/x.flac",
        &Outcome {
            split: Err(SPLIT_FROM_ROOT),
            whole: Err(WHOLE_FROM_ROOT),
            absolute: Ok((
                names(&[b"srv", b"music", b"Bj\xF6rk", b"x.flac"]),
                "/srv/music/Bj\u{FFFD}rk/x.flac".to_owned(),
            )),
            beneath_music: Some(names(&[b"Bj\xF6rk", b"x.flac"])),
            refusal: None,
            link: Ok(names(&[b"srv", b"music", b"Bj\xF6rk", b"x.flac"])),
            verdict: Some(Verdict::Own(names(&[b"Bj\xF6rk", b"x.flac"]))),
            shown: "/srv/music/Bj\u{FFFD}rk/x.flac".to_owned(),
            stored: "baf016f901534bce8fc90c15f3e5d7fa7509531647cba722941be21726d20ba0".to_owned(),
            stored_id: None,
        },
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
        &Outcome {
            split: Err(SPLIT_FROM_ROOT),
            whole: Err(WHOLE_FROM_ROOT),
            absolute: Ok((
                names(&[b"srv", b"musicals", b"x"]),
                "/srv/musicals/x".to_owned(),
            )),
            beneath_music: None,
            refusal: None,
            link: Ok(names(&[b"srv", b"musicals", b"x"])),
            verdict: Some(Verdict::Outside),
            shown: "/srv/musicals/x".to_owned(),
            stored: "b45d8c4cb59618778dec16b177a2ca535a8c85c60f30db3f6c4295e4663fa9b9".to_owned(),
            stored_id: None,
        },
    );
}

/// A root must be canonical, but a link may hold any spelling.
///
/// Verifies: SEC-MED-028
#[test]
fn replays_an_absolute_path_that_is_not_canonical() {
    replay(
        "not-canonical",
        b"/srv//music/../x",
        &Outcome {
            split: Err(SPLIT_FROM_ROOT),
            whole: Err(WHOLE_FROM_ROOT),
            absolute: Err(PathError::EmptyName { component: 1 }),
            beneath_music: None,
            refusal: None,
            link: Ok(names(&[b"srv", b"x"])),
            verdict: Some(Verdict::Outside),
            shown: "/srv//music/../x".to_owned(),
            stored: "33770f158dafae8858c693729b5885d0b708e3ed77a6bca0a451ea6c5c2fb499".to_owned(),
            stored_id: None,
        },
    );
}

/// `/proc/self/root` leads back to `/`.
///
/// Verifies: SEC-MED-028, SEC-MED-037
#[test]
fn replays_a_system_directory() {
    replay(
        "system-directory",
        b"/proc/self/root",
        &Outcome {
            split: Err(SPLIT_FROM_ROOT),
            whole: Err(WHOLE_FROM_ROOT),
            absolute: Ok((
                names(&[b"proc", b"self", b"root"]),
                "/proc/self/root".to_owned(),
            )),
            beneath_music: None,
            refusal: Some(RootRefusal::SystemDirectory { dir: "/proc" }),
            link: Ok(names(&[b"proc", b"self", b"root"])),
            verdict: Some(Verdict::Outside),
            shown: "/proc/self/root".to_owned(),
            stored: "438b2a168f661e8dcf9330f4debcd5ba5f6ab4d95141385fc4a5b6bf79643f2c".to_owned(),
            stored_id: None,
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-037
#[test]
fn replays_a_folder_inside_the_data_directory() {
    replay(
        "inside-the-data-directory",
        b"/var/lib/gunmetal/secrets",
        &Outcome {
            split: Err(SPLIT_FROM_ROOT),
            whole: Err(WHOLE_FROM_ROOT),
            absolute: Ok((
                names(&[b"var", b"lib", b"gunmetal", b"secrets"]),
                "/var/lib/gunmetal/secrets".to_owned(),
            )),
            beneath_music: None,
            refusal: Some(RootRefusal::DataDirectory {
                index: 0,
                overlap: Overlap::Inside,
            }),
            link: Ok(names(&[b"var", b"lib", b"gunmetal", b"secrets"])),
            verdict: Some(Verdict::Outside),
            shown: "/var/lib/gunmetal/secrets".to_owned(),
            stored: "10826ddc828d4b8aa684728042345fe542a041eab927bc9a60c0d66b5a0785bd".to_owned(),
            stored_id: None,
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-037
#[test]
fn replays_a_parent_of_the_data_directory() {
    replay(
        "parent-of-the-data-directory",
        b"/var/lib",
        &Outcome {
            split: Err(SPLIT_FROM_ROOT),
            whole: Err(WHOLE_FROM_ROOT),
            absolute: Ok((names(&[b"var", b"lib"]), "/var/lib".to_owned())),
            beneath_music: None,
            refusal: Some(RootRefusal::DataDirectory {
                index: 0,
                overlap: Overlap::Contains,
            }),
            link: Ok(names(&[b"var", b"lib"])),
            verdict: Some(Verdict::Outside),
            shown: "/var/lib".to_owned(),
            stored: "018076f20178b3bf76d8270b980ca52fcb8d4d20b397e43df5db5d59874fbf58".to_owned(),
            stored_id: None,
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-037
#[test]
fn replays_the_data_directory_itself() {
    replay(
        "the-data-directory",
        b"/var/lib/gunmetal",
        &Outcome {
            split: Err(SPLIT_FROM_ROOT),
            whole: Err(WHOLE_FROM_ROOT),
            absolute: Ok((
                names(&[b"var", b"lib", b"gunmetal"]),
                "/var/lib/gunmetal".to_owned(),
            )),
            beneath_music: None,
            refusal: Some(RootRefusal::DataDirectory {
                index: 0,
                overlap: Overlap::Equal,
            }),
            link: Ok(names(&[b"var", b"lib", b"gunmetal"])),
            verdict: Some(Verdict::Outside),
            shown: "/var/lib/gunmetal".to_owned(),
            stored: "4d3efef353af9ab1f9fd691c7bb9c5ed00ea7472ee25d3cd6e564a2bd5782bf5".to_owned(),
            stored_id: None,
        },
    );
}

/// Navidrome's GHSA-r5qr-m328-qcf4: `passwd.wav`, a link to
/// `/etc/passwd`, became a playable track.
///
/// Verifies: SEC-MED-028, SEC-MED-034, SEC-MED-037
#[test]
fn replays_a_link_to_the_password_file() {
    replay(
        "passwd-link",
        b"/etc/passwd",
        &Outcome {
            split: Err(SPLIT_FROM_ROOT),
            whole: Err(WHOLE_FROM_ROOT),
            absolute: Ok((names(&[b"etc", b"passwd"]), "/etc/passwd".to_owned())),
            beneath_music: None,
            refusal: Some(RootRefusal::SystemDirectory { dir: "/etc" }),
            link: Ok(names(&[b"etc", b"passwd"])),
            verdict: Some(Verdict::Outside),
            shown: "/etc/passwd".to_owned(),
            stored: "74acf31844532670be412c65b8251ee55d072549080b1cffdbea6b1a192230a0".to_owned(),
            stored_id: None,
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-034
#[test]
fn replays_a_link_into_the_approved_root() {
    replay(
        "approved-target",
        b"/mnt/debrid/Album/x.flac",
        &Outcome {
            split: Err(SPLIT_FROM_ROOT),
            whole: Err(WHOLE_FROM_ROOT),
            absolute: Ok((
                names(&[b"mnt", b"debrid", b"Album", b"x.flac"]),
                "/mnt/debrid/Album/x.flac".to_owned(),
            )),
            beneath_music: None,
            refusal: None,
            link: Ok(names(&[b"mnt", b"debrid", b"Album", b"x.flac"])),
            verdict: Some(Verdict::Approved {
                index: 0,
                rest: names(&[b"Album", b"x.flac"]),
            }),
            shown: "/mnt/debrid/Album/x.flac".to_owned(),
            stored: "f81ba5eeaec6f0ffb30cc5c149097aea57bef9113275cf1d76c3d5488ffd04ac".to_owned(),
            stored_id: None,
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-034
#[test]
fn replays_a_relative_link_that_climbs_out_to_the_approved_root() {
    replay(
        "climbs-out-to-the-approved-root",
        b"../../../../mnt/debrid/x.flac",
        &Outcome {
            split: Err(PathError::Escapes { component: 0 }),
            whole: Err(PathError::Separator {
                component: 0,
                offset: 2,
            }),
            absolute: Err(PathError::NotAbsolute),
            beneath_music: None,
            refusal: None,
            link: Ok(names(&[b"mnt", b"debrid", b"x.flac"])),
            verdict: Some(Verdict::Approved {
                index: 0,
                rest: names(&[b"x.flac"]),
            }),
            shown: "../../../../mnt/debrid/x.flac".to_owned(),
            stored: "7caf01ac9c70a9ea160c5953e82cf8b69b14fae4a00cb2990fa5a1151256ad3c".to_owned(),
            stored_id: None,
        },
    );
}

/// Verifies: SEC-MED-028, SEC-MED-034
#[test]
fn replays_a_link_into_another_library() {
    replay(
        "another-library",
        b"/srv/films/x.mkv",
        &Outcome {
            split: Err(SPLIT_FROM_ROOT),
            whole: Err(WHOLE_FROM_ROOT),
            absolute: Ok((
                names(&[b"srv", b"films", b"x.mkv"]),
                "/srv/films/x.mkv".to_owned(),
            )),
            beneath_music: None,
            refusal: None,
            link: Ok(names(&[b"srv", b"films", b"x.mkv"])),
            verdict: Some(Verdict::OtherLibrary { index: 0 }),
            shown: "/srv/films/x.mkv".to_owned(),
            stored: "d30697a31c0bb39a26d2c0282b798bba42f786618f85019636dab011f1b72f82".to_owned(),
            stored_id: None,
        },
    );
}

/// A file the server writes for a track is named by its identifier.
///
/// Verifies: SEC-MED-028, SEC-MED-039
#[test]
fn replays_a_track_identifier() {
    replay(
        "track-identifier",
        b"trk_0123456789abcdefghjkmnpqrs",
        &Outcome {
            split: Ok((
                names(&[b"trk_0123456789abcdefghjkmnpqrs"]),
                "trk_0123456789abcdefghjkmnpqrs".to_owned(),
            )),
            whole: Ok(names(&[b"trk_0123456789abcdefghjkmnpqrs"])),
            absolute: Err(PathError::NotAbsolute),
            beneath_music: None,
            refusal: None,
            link: Ok(names(&[
                b"srv",
                b"music",
                b"Rock",
                b"Album",
                b"trk_0123456789abcdefghjkmnpqrs",
            ])),
            verdict: Some(Verdict::Own(names(&[
                b"Rock",
                b"Album",
                b"trk_0123456789abcdefghjkmnpqrs",
            ]))),
            shown: "trk_0123456789abcdefghjkmnpqrs".to_owned(),
            stored: "b90ce200ffc8942cd06b76bf835b78174108dd2dd080d4fcf7448fd6abc17d98".to_owned(),
            stored_id: Some("trk_0123456789abcdefghjkmnpqrs".to_owned()),
        },
    );
}
