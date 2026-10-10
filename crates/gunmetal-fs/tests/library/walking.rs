//! Walking a root: every directory once, in name order, by the bytes of
//! its names, with what is skipped listed beside what is found.

#[cfg(target_os = "linux")]
use std::ffi::OsStr;
use std::fs;
use std::io::ErrorKind;
#[cfg(target_os = "linux")]
use std::os::unix::ffi::OsStrExt as _;
use std::path::Path;

use gunmetal_core::parse::{LimitError, LimitKind};
use gunmetal_core::path::RelPath;
use gunmetal_fs::fingerprint::Mark;
#[cfg(target_os = "linux")]
use gunmetal_fs::open::FileKind;
#[cfg(target_os = "linux")]
use gunmetal_fs::pool::Pool;
use gunmetal_fs::root::{FsError, LinkPolicy, LinkReason, LinkRefusal, Op};
use gunmetal_fs::walk::{Visit, WalkLimits};

#[cfg(target_os = "linux")]
use crate::support::{LONG, fifo, socket};
use crate::support::{
    Scratch, assert_not_root, at, collect, hex, identity, io, outside, raw, rel, set_mode, summary,
};

/// The visit of the regular file at `path`, which has one name and is
/// really at `real` beneath the scratch directory.
fn file_at(scratch: &Scratch, path: RelPath, real: &str) -> Visit {
    Visit::File {
        path,
        identity: identity(&scratch.path(real)),
        links: 1,
    }
}

/// The visit of the regular file at `path` beneath the library.
fn file(scratch: &Scratch, path: &str) -> Visit {
    file_at(scratch, at(path), &format!("music/{path}"))
}

/// The listing entry `name`, a regular file that is really at `real`
/// beneath the scratch directory.
fn found<'a>(scratch: &Scratch, name: &'a [u8], real: &str) -> (&'a [u8], Mark) {
    seen(name, &scratch.path(real))
}

/// The listing entry `name`, a regular file that is really at `path`.
fn seen<'a>(name: &'a [u8], path: &Path) -> (&'a [u8], Mark) {
    (name, Mark::File(identity(path)))
}

/// The listing entry `name`, a directory.
fn folder(name: &[u8]) -> (&[u8], Mark) {
    (name, Mark::Dir)
}

/// The listing entry `name`, which the walk does not take.
fn left(name: &[u8]) -> (&[u8], Mark) {
    (name, Mark::Skipped)
}

/// The visit of the directory at `path` whose listing is `entries`.
fn dir(path: RelPath, entries: &[(&[u8], Mark)]) -> Visit {
    Visit::Dir {
        path,
        summary: summary(entries),
    }
}

/// The visit of something skipped at `path`.
fn skipped(path: &str, reason: FsError) -> Visit {
    Visit::Skipped {
        path: at(path),
        reason,
    }
}

#[test]
fn lists_each_directory_then_its_files_then_the_directories_inside_it() {
    let scratch = Scratch::new("fs-walk-order");
    scratch.dir("music/Album/Disc 2");
    scratch.dir("music/Zebra");
    for path in [
        "music/a.flac",
        "music/B-side.flac",
        "music/Album/02.flac",
        "music/Album/01.flac",
        "music/Album/Disc 2/01.flac",
        "music/Zebra/z.flac",
    ] {
        scratch.file(path, path.as_bytes());
    }
    assert_eq!(
        collect(scratch.root().walk()),
        [
            dir(
                rel(&[]),
                &[
                    folder(b"Album"),
                    found(&scratch, b"B-side.flac", "music/B-side.flac"),
                    folder(b"Zebra"),
                    found(&scratch, b"a.flac", "music/a.flac"),
                ]
            ),
            file(&scratch, "B-side.flac"),
            file(&scratch, "a.flac"),
            dir(
                at("Album"),
                &[
                    found(&scratch, b"01.flac", "music/Album/01.flac"),
                    found(&scratch, b"02.flac", "music/Album/02.flac"),
                    folder(b"Disc 2"),
                ]
            ),
            file(&scratch, "Album/01.flac"),
            file(&scratch, "Album/02.flac"),
            dir(
                at("Album/Disc 2"),
                &[found(&scratch, b"01.flac", "music/Album/Disc 2/01.flac")]
            ),
            file(&scratch, "Album/Disc 2/01.flac"),
            dir(
                at("Zebra"),
                &[found(&scratch, b"z.flac", "music/Zebra/z.flac")]
            ),
            file(&scratch, "Zebra/z.flac"),
        ]
    );
}

#[test]
fn an_empty_root_is_one_empty_directory() {
    let scratch = Scratch::new("fs-walk-empty");
    let visits = collect(scratch.root().walk());
    assert_eq!(visits, [dir(rel(&[]), &[])]);
    // The summary of nothing is the digest of no bytes.
    let [Visit::Dir { summary, .. }] = visits.as_slice() else {
        panic!("one directory, not {visits:?}");
    };
    assert_eq!(hex(summary.0), "e3b0c44298fc1c149afbf4c8996fb924");
}

/// The walk runs on a pool so that a door that opened the FIFO, and waited
/// for a writer that never comes, would fail this test instead of hanging
/// it. The FIFO helper is Linux's, so the test runs there; elsewhere the
/// refusals are proven by the socket and directory cases in opening.rs,
/// whose opens cannot wait.
///
/// Verifies: SEC-MED-035
#[cfg(target_os = "linux")]
#[test]
fn skips_a_fifo_and_a_socket_whatever_they_are_called_and_finishes() {
    let scratch = Scratch::new("fs-walk-special");
    fifo(&scratch.path("music/track.flac"));
    socket(&scratch.path("music/cover.jpg"));
    scratch.file("music/real.flac", b"fLaC");
    let root = scratch.root();
    let walked = Pool::new(1).run(LONG, move || collect(root.walk()));
    assert_eq!(
        walked,
        Ok(vec![
            dir(
                rel(&[]),
                &[
                    left(b"cover.jpg"),
                    found(&scratch, b"real.flac", "music/real.flac"),
                    left(b"track.flac"),
                ]
            ),
            skipped(
                "cover.jpg",
                FsError::NotRegular {
                    found: FileKind::Socket
                }
            ),
            file(&scratch, "real.flac"),
            skipped(
                "track.flac",
                FsError::NotRegular {
                    found: FileKind::Fifo
                }
            ),
        ])
    );
}

/// Verifies: SEC-MED-034, SEC-API-018
#[test]
fn follows_a_link_that_stays_inside_and_lists_the_ones_it_refuses() {
    let scratch = Scratch::new("fs-walk-links");
    scratch.dir("films");
    scratch.file("films/film.flac", b"another library");
    scratch.file("music/track.flac", b"fLaC");
    scratch.link("track.flac", "music/again.flac");
    scratch.link("../films/film.flac", "music/borrowed.flac");
    scratch.link("missing.flac", "music/dangling.flac");
    scratch.link("/etc/passwd", "music/passwd.wav");
    let root = scratch.root_with(LinkPolicy {
        approved: Vec::new(),
        others: vec![scratch.raw("films")],
    });
    assert_eq!(
        collect(root.walk()),
        [
            dir(
                rel(&[]),
                &[
                    found(&scratch, b"again.flac", "music/track.flac"),
                    left(b"borrowed.flac"),
                    left(b"dangling.flac"),
                    left(b"passwd.wav"),
                    found(&scratch, b"track.flac", "music/track.flac"),
                ]
            ),
            file_at(&scratch, at("again.flac"), "music/track.flac"),
            skipped(
                "borrowed.flac",
                FsError::Link(LinkRefusal {
                    target: scratch.raw("films/film.flac"),
                    reason: LinkReason::OtherLibrary { index: 0 },
                })
            ),
            skipped("dangling.flac", io(Op::Inspect, ErrorKind::NotFound)),
            skipped("passwd.wav", outside(raw("/etc/passwd"))),
            file(&scratch, "track.flac"),
        ]
    );
}

/// Verifies: SEC-MED-040
///
/// The names need a byte-transparent filesystem, which the macOS ones
/// are not: those refuse to create such a name at all (`EILSEQ`), so the
/// test runs on Linux, where the hostile input can exist.
#[cfg(target_os = "linux")]
#[test]
fn walks_names_that_are_not_utf8_or_hold_controls_by_their_bytes() {
    let scratch = Scratch::new("fs-walk-bytes");
    let music = scratch.path("music");
    let artist = music.join(OsStr::from_bytes(b"Bj\xF6rk"));
    let inner = artist.join(OsStr::from_bytes(b"two\nlines.flac"));
    let outer = music.join(OsStr::from_bytes(b"\x1B[31mred.flac"));
    fs::create_dir(artist).expect("create the folder");
    fs::write(&inner, b"fLaC").expect("write a file");
    fs::write(&outer, b"fLaC").expect("write a file");
    let deep = rel(&[b"Bj\xF6rk", b"two\nlines.flac"]);
    assert_eq!(
        collect(scratch.root().walk()),
        [
            dir(
                rel(&[]),
                &[seen(b"\x1B[31mred.flac", &outer), folder(b"Bj\xF6rk"),]
            ),
            Visit::File {
                path: rel(&[b"\x1B[31mred.flac"]),
                identity: identity(&outer),
                links: 1,
            },
            dir(rel(&[b"Bj\xF6rk"]), &[seen(b"two\nlines.flac", &inner)]),
            Visit::File {
                path: deep.clone(),
                identity: identity(&inner),
                links: 1,
            },
        ]
    );
    // What people see is decoded and escaped; the bytes above open the file.
    assert_eq!(deep.display(), "Bj\u{FFFD}rk/two\\u{a}lines.flac");
}

#[test]
fn skips_a_directory_that_becomes_unreadable_and_carries_on() {
    assert_not_root();
    let scratch = Scratch::new("fs-walk-unreadable");
    for path in ["music/a", "music/b", "music/c"] {
        scratch.dir(path);
        scratch.file(&format!("{path}/track.flac"), b"fLaC");
    }
    let root = scratch.root();
    let mut walk = root.walk();
    assert_eq!(
        walk.next(),
        Some(dir(rel(&[]), &[folder(b"a"), folder(b"b"), folder(b"c")]))
    );
    set_mode(&scratch.path("music/b"), 0o000);
    let rest = collect(walk);
    set_mode(&scratch.path("music/b"), 0o700);
    assert_eq!(
        rest,
        [
            dir(
                at("a"),
                &[found(&scratch, b"track.flac", "music/a/track.flac")]
            ),
            file(&scratch, "a/track.flac"),
            skipped("b", io(Op::List, ErrorKind::PermissionDenied)),
            dir(
                at("c"),
                &[found(&scratch, b"track.flac", "music/c/track.flac")]
            ),
            file(&scratch, "c/track.flac"),
        ]
    );
}

/// A link back to a folder above it would send a walk round for ever. Each
/// directory is listed once, whichever path reaches it first.
#[test]
fn lists_a_directory_once_however_many_links_lead_to_it() {
    let scratch = Scratch::new("fs-walk-loop");
    scratch.dir("music/Album");
    scratch.file("music/Album/01.flac", b"fLaC");
    scratch.link("..", "music/Album/up");
    scratch.link(".", "music/Album/same");
    scratch.link("Album", "music/Copy");
    assert_eq!(
        collect(scratch.root().walk()),
        [
            dir(rel(&[]), &[folder(b"Album"), folder(b"Copy")]),
            dir(
                at("Album"),
                &[
                    found(&scratch, b"01.flac", "music/Album/01.flac"),
                    folder(b"same"),
                    folder(b"up"),
                ]
            ),
            file(&scratch, "Album/01.flac"),
            skipped("Album/same", FsError::AlreadyWalked),
            skipped("Album/up", FsError::AlreadyWalked),
            skipped("Copy", FsError::AlreadyWalked),
        ]
    );
}

/// Verifies: SEC-MED-034
#[test]
fn walks_through_a_link_into_a_folder_once_it_is_approved() {
    let scratch = Scratch::new("fs-walk-approved");
    scratch.dir("debrid/Album");
    scratch.file("debrid/Album/01.flac", b"approved");
    scratch.link(scratch.absolute("debrid/Album"), "music/Album");
    // Refused, the link is listed with where it leads.
    assert_eq!(
        collect(scratch.root().walk()),
        [
            dir(rel(&[]), &[left(b"Album")]),
            skipped("Album", outside(scratch.raw("debrid/Album"))),
        ]
    );
    // Approved, what it leads to is walked under the link's own path, and
    // the root's summary is no longer the one it had.
    let root = scratch.root_with(LinkPolicy {
        approved: vec![scratch.path("debrid")],
        others: Vec::new(),
    });
    assert_eq!(
        collect(root.walk()),
        [
            dir(rel(&[]), &[folder(b"Album")]),
            dir(
                at("Album"),
                &[found(&scratch, b"01.flac", "debrid/Album/01.flac")]
            ),
            file_at(&scratch, at("Album/01.flac"), "debrid/Album/01.flac"),
        ]
    );
}

/// A hard link is a file like any other, listed with how many names it
/// has.
#[test]
fn lists_a_hard_link_with_the_number_of_names_the_file_has() {
    let scratch = Scratch::new("fs-walk-hard-link");
    scratch.dir("outside");
    scratch.file("outside/original.flac", b"fLaC");
    fs::hard_link(
        scratch.path("outside/original.flac"),
        scratch.path("music/linked.flac"),
    )
    .expect("make a hard link");
    assert_eq!(
        collect(scratch.root().walk()),
        [
            dir(
                rel(&[]),
                &[found(&scratch, b"linked.flac", "outside/original.flac")]
            ),
            Visit::File {
                path: at("linked.flac"),
                identity: identity(&scratch.path("outside/original.flac")),
                links: 2,
            },
        ]
    );
}

/// The summaries are what lets a rescan skip a directory: walking an
/// unchanged library twice gives the same visits, and a change shows in
/// the summary of the directory it happened in and in no other (LIB-016).
#[test]
fn a_change_shows_in_the_summary_of_its_own_directory_only() {
    let scratch = Scratch::new("fs-walk-rescan");
    scratch.dir("music/Album");
    scratch.dir("music/Other");
    scratch.file("music/Album/01.flac", b"fLaC");
    scratch.file("music/Other/01.flac", b"fLaC");
    let root = scratch.root();
    let first = collect(root.walk());
    assert_eq!(collect(root.walk()), first);

    scratch.file("music/Album/02.flac", b"fLaC");
    assert_eq!(
        collect(root.walk()),
        [
            dir(rel(&[]), &[folder(b"Album"), folder(b"Other")]),
            dir(
                at("Album"),
                &[
                    found(&scratch, b"01.flac", "music/Album/01.flac"),
                    found(&scratch, b"02.flac", "music/Album/02.flac"),
                ]
            ),
            file(&scratch, "Album/01.flac"),
            file(&scratch, "Album/02.flac"),
            dir(
                at("Other"),
                &[found(&scratch, b"01.flac", "music/Other/01.flac")]
            ),
            file(&scratch, "Other/01.flac"),
        ]
    );
    let summaries = |visits: &[Visit]| -> Vec<String> {
        visits
            .iter()
            .filter_map(|visit| match visit {
                Visit::Dir { summary, .. } => Some(hex(summary.0)),
                Visit::File { .. } | Visit::Skipped { .. } => None,
            })
            .collect()
    };
    let before = summaries(&first);
    let after = summaries(&collect(root.walk()));
    assert_eq!(
        (
            before[0] == after[0],
            before[1] == after[1],
            before[2] == after[2]
        ),
        (true, false, true)
    );
}

/// The path of the folder `depth` levels of `d` below the root.
fn nested(depth: usize) -> RelPath {
    let names: Vec<&[u8]> = std::iter::repeat_n(b"d".as_slice(), depth).collect();
    rel(&names)
}

/// The limits that change only how many entries a directory may hold.
fn entries(entries: u64) -> WalkLimits {
    WalkLimits::new(entries, WalkLimits::DEFAULT.depth()).expect("limits within their ceilings")
}

/// The limits that change only how deep a directory may lie.
fn depth(depth: u64) -> WalkLimits {
    WalkLimits::new(WalkLimits::DEFAULT.entries(), depth).expect("limits within their ceilings")
}

/// A plain walk runs under the limits the core's limits table gives a
/// container: 65,536 children and 32 levels. A folder 32 levels below the
/// root is listed, and the one 33 levels below it is reported and not
/// entered.
#[test]
fn walks_folders_as_deep_as_the_default_limit_and_no_deeper() {
    assert_eq!(
        (WalkLimits::DEFAULT.entries(), WalkLimits::DEFAULT.depth()),
        (65_536, 32)
    );
    let scratch = Scratch::new("fs-walk-default-depth");
    scratch.dir(&format!("music/{}", ["d"; 33].join("/")));
    let mut expected: Vec<Visit> = (0..=32)
        .map(|level| dir(nested(level), &[folder(b"d")]))
        .collect();
    expected.push(Visit::Skipped {
        path: nested(33),
        reason: FsError::TooDeep { depth: 33, max: 32 },
    });
    assert_eq!(collect(scratch.root().walk()), expected);
}

/// A directory with exactly as many entries as the limit is listed whole.
/// One with a single entry more is not listed at all: it is reported once,
/// with the limit, and nothing in it is reported as if it were all of it.
/// Its parent still lists it as a directory.
#[test]
fn lists_a_directory_at_the_entry_limit_and_skips_one_over_it_whole() {
    let scratch = Scratch::new("fs-walk-entries");
    for name in ["a.flac", "b.flac", "c.flac"] {
        for place in ["Full", "Over"] {
            scratch.dir(&format!("music/{place}"));
            scratch.file(&format!("music/{place}/{name}"), b"fLaC");
        }
    }
    scratch.file("music/Over/d.flac", b"fLaC");
    let root = scratch.root();
    assert_eq!(
        collect(root.walk_with(entries(3))),
        [
            dir(rel(&[]), &[folder(b"Full"), folder(b"Over")]),
            dir(
                at("Full"),
                &[
                    found(&scratch, b"a.flac", "music/Full/a.flac"),
                    found(&scratch, b"b.flac", "music/Full/b.flac"),
                    found(&scratch, b"c.flac", "music/Full/c.flac"),
                ]
            ),
            file(&scratch, "Full/a.flac"),
            file(&scratch, "Full/b.flac"),
            file(&scratch, "Full/c.flac"),
            skipped("Over", FsError::TooManyEntries { max: 3 }),
        ]
    );
    // The root is held to the limit too.
    assert_eq!(
        collect(root.walk_with(entries(1))),
        [Visit::Skipped {
            path: rel(&[]),
            reason: FsError::TooManyEntries { max: 1 },
        }]
    );
}

#[test]
fn lists_directories_at_the_depth_limit_and_skips_deeper_ones() {
    let scratch = Scratch::new("fs-walk-depth");
    scratch.dir("music/a/b/c");
    scratch.file("music/a/1.flac", b"fLaC");
    scratch.file("music/a/b/2.flac", b"fLaC");
    scratch.file("music/a/b/c/3.flac", b"fLaC");
    let root = scratch.root();
    assert_eq!(
        collect(root.walk_with(depth(2))),
        [
            dir(rel(&[]), &[folder(b"a")]),
            dir(
                at("a"),
                &[found(&scratch, b"1.flac", "music/a/1.flac"), folder(b"b")]
            ),
            file(&scratch, "a/1.flac"),
            dir(
                at("a/b"),
                &[found(&scratch, b"2.flac", "music/a/b/2.flac"), folder(b"c")]
            ),
            file(&scratch, "a/b/2.flac"),
            skipped("a/b/c", FsError::TooDeep { depth: 3, max: 2 }),
        ]
    );
    assert_eq!(
        collect(root.walk_with(depth(0))),
        [
            dir(rel(&[]), &[folder(b"a")]),
            skipped("a", FsError::TooDeep { depth: 1, max: 0 }),
        ]
    );
}

/// A directory waiting to be listed is judged again when the walk comes to
/// it, so whatever replaced it since is taken for what it is now: a link
/// by the link policy, a file as no directory, and a link to a folder the
/// walk has listed as that folder, which is not listed twice.
///
/// Verifies: SEC-MED-034
#[test]
fn judges_a_directory_again_when_it_comes_to_list_it() {
    let scratch = Scratch::new("fs-walk-rejudge");
    scratch.dir("outside");
    for name in ["a", "b", "c", "d"] {
        scratch.dir(&format!("music/{name}"));
        scratch.file(&format!("music/{name}/track.flac"), b"fLaC");
    }
    let root = scratch.root();
    let mut walk = root.walk();
    assert_eq!(
        walk.next(),
        Some(dir(
            rel(&[]),
            &[folder(b"a"), folder(b"b"), folder(b"c"), folder(b"d")]
        ))
    );
    for name in ["b", "c", "d"] {
        fs::rename(scratch.path(&format!("music/{name}")), scratch.path(name))
            .expect("move the folder away");
    }
    scratch.link("../outside", "music/b");
    scratch.file("music/c", b"fLaC");
    scratch.link("a", "music/d");
    assert_eq!(
        collect(walk),
        [
            dir(
                at("a"),
                &[found(&scratch, b"track.flac", "music/a/track.flac")]
            ),
            file(&scratch, "a/track.flac"),
            skipped("b", outside(scratch.raw("outside"))),
            skipped("c", io(Op::List, ErrorKind::NotADirectory)),
            skipped("d", FsError::AlreadyWalked),
        ]
    );
}

/// The limits may be lowered freely, and raised only to the ceilings the
/// core's limits table gives the rows their numbers come from: four times
/// the default for a directory's entries, and no higher than the default
/// for depth. So no caller can switch a limit off.
#[test]
fn holds_the_limits_to_the_ceilings_of_the_core_limits_table() {
    let highest = WalkLimits::new(262_144, 32).expect("the ceilings themselves are allowed");
    assert_eq!((highest.entries(), highest.depth()), (262_144, 32));
    let lowest = WalkLimits::new(0, 0).expect("a limit may be lowered to nothing");
    assert_eq!((lowest.entries(), lowest.depth()), (0, 0));
    assert_eq!(
        WalkLimits::new(262_145, 32),
        Err(LimitError::AboveCeiling {
            limit: LimitKind::Children,
            value: 262_145,
            ceiling: 262_144,
        })
    );
    assert_eq!(
        WalkLimits::new(65_536, 33),
        Err(LimitError::AboveCeiling {
            limit: LimitKind::ContainerDepth,
            value: 33,
            ceiling: 32,
        })
    );
    assert_eq!(
        WalkLimits::new(u64::MAX, u64::MAX),
        Err(LimitError::AboveCeiling {
            limit: LimitKind::Children,
            value: u64::MAX,
            ceiling: 262_144,
        })
    );
}

/// A link the policy refuses is listed with where it leads, but the walk
/// does not hold on to that while it reports the rest of the directory, or
/// a folder of such links could make it hold their targets all at once. It
/// reads the link again when it comes to report it. So the target reported
/// is the one the link has then, and a refused link that has become
/// something else since is reported as replaced.
///
/// Verifies: SEC-MED-034
#[test]
fn reads_a_refused_link_again_when_it_comes_to_report_it() {
    let scratch = Scratch::new("fs-walk-refused-again");
    scratch.link("/outside/first", "music/a.flac");
    scratch.link("/outside/first", "music/b.flac");
    let root = scratch.root();
    let mut walk = root.walk();
    assert_eq!(
        walk.next(),
        Some(dir(rel(&[]), &[left(b"a.flac"), left(b"b.flac")]))
    );
    for name in ["a.flac", "b.flac"] {
        fs::remove_file(scratch.path(&format!("music/{name}"))).expect("remove the link");
    }
    scratch.link("/outside/second", "music/a.flac");
    scratch.file("music/b.flac", b"fLaC");
    assert_eq!(
        collect(walk),
        [
            skipped("a.flac", outside(raw("/outside/second"))),
            skipped("b.flac", FsError::Replaced),
        ]
    );
}
