//! The symbolic-link policy: a link is followed only when its whole chain
//! stays beneath the library's root or a folder approved for it.

use std::io::ErrorKind;

use gunmetal_fs::root::{FsError, LinkPolicy, LinkReason, LinkRefusal, MAX_LINKS, Op};

use crate::support::{Scratch, at, contents, identity, io, outside, raw};

/// What opening `path` beneath `scratch`'s library gives under the default
/// policy, without the file.
fn opening(scratch: &Scratch, path: &str) -> Result<(), FsError> {
    scratch.root().open_file(&at(path)).map(|_| ())
}

/// The policy that approves `folder` as a link target and knows no other
/// library.
fn approving(scratch: &Scratch, folder: &str) -> LinkPolicy {
    LinkPolicy {
        approved: vec![scratch.path(folder)],
        others: Vec::new(),
    }
}

/// Verifies: SEC-MED-034, SEC-TM-043
#[test]
fn follows_links_that_stay_beneath_the_root() {
    let scratch = Scratch::new("fs-link-inside");
    scratch.dir("music/Artist/Album");
    scratch.file("music/Artist/Album/01.flac", b"the track");
    // A link to a file, a link to a directory, a link that climbs and
    // comes back down, and an absolute link that leads into the root.
    scratch.link("Artist/Album/01.flac", "music/favourite.flac");
    scratch.link("Artist/Album", "music/Latest");
    scratch.link("../../favourite.flac", "music/Artist/Album/again.flac");
    scratch.link(scratch.absolute("music/Artist"), "music/Absolute");
    let root = scratch.root();
    for path in [
        "favourite.flac",
        "Latest/01.flac",
        "Artist/Album/again.flac",
        "Latest/again.flac",
        "Absolute/Album/01.flac",
    ] {
        let file = root.open_file(&at(path)).expect("the link is followed");
        assert_eq!(contents(&file), b"the track", "{path}");
        assert_eq!(
            file.identity(),
            identity(&scratch.path("music/Artist/Album/01.flac")),
            "{path}"
        );
    }
}

/// Navidrome's GHSA-r5qr-m328-qcf4: `passwd.wav`, a link to `/etc/passwd`,
/// became a playable track.
///
/// Verifies: SEC-MED-034, SEC-TM-043, SEC-HIS-016, SEC-API-018
#[test]
fn refuses_a_link_to_a_system_file_whatever_it_is_called() {
    let scratch = Scratch::new("fs-link-passwd");
    scratch.link("/etc/passwd", "music/passwd.wav");
    assert_eq!(
        opening(&scratch, "passwd.wav"),
        Err(outside(raw("/etc/passwd")))
    );
}

/// Verifies: SEC-MED-034, SEC-TM-043, SEC-HIS-016, SEC-API-018
#[test]
fn refuses_links_that_lead_out_of_the_root() {
    let scratch = Scratch::new("fs-link-escape");
    scratch.dir("outside");
    scratch.file("outside/secret", b"outside");
    scratch.dir("music/Album");
    scratch.link("../outside/secret", "music/relative.flac");
    scratch.link("../../outside/secret", "music/Album/deeper.flac");
    scratch.link(scratch.absolute("outside/secret"), "music/absolute.flac");
    scratch.link("../outside", "music/Folder");
    for path in ["relative.flac", "Album/deeper.flac", "absolute.flac"] {
        assert_eq!(
            opening(&scratch, path),
            Err(outside(scratch.raw("outside/secret"))),
            "{path}"
        );
    }
    // A link to a directory outside is refused where the link is, before
    // anything inside that directory is looked at.
    assert_eq!(
        opening(&scratch, "Folder/secret"),
        Err(outside(scratch.raw("outside")))
    );
}

/// Verifies: SEC-MED-034
#[test]
fn refuses_a_link_into_another_library_and_names_the_library() {
    let scratch = Scratch::new("fs-link-other");
    scratch.dir("films");
    scratch.dir("kids");
    scratch.file("kids/song.flac", b"another library");
    scratch.link("../kids/song.flac", "music/borrowed.flac");
    let root = scratch.root_with(LinkPolicy {
        approved: Vec::new(),
        others: vec![scratch.raw("films"), scratch.raw("kids")],
    });
    assert_eq!(
        root.open_file(&at("borrowed.flac")).map(|_| ()),
        Err(FsError::Link(LinkRefusal {
            target: scratch.raw("kids/song.flac"),
            reason: LinkReason::OtherLibrary { index: 1 },
        }))
    );
}

/// Verifies: SEC-MED-034, SEC-TM-043
#[test]
fn follows_links_into_a_folder_approved_for_the_library() {
    let scratch = Scratch::new("fs-link-approved");
    scratch.dir("debrid/Album");
    scratch.file("debrid/Album/01.flac", b"approved");
    scratch.link("01.flac", "debrid/Album/same.flac");
    scratch.link(scratch.absolute("debrid/Album/01.flac"), "music/01.flac");
    scratch.link(scratch.absolute("debrid/Album"), "music/Album");
    scratch.link("../debrid/Album/same.flac", "music/relative.flac");
    let root = scratch.root_with(approving(&scratch, "debrid"));
    for path in [
        "01.flac",
        "Album/01.flac",
        "Album/same.flac",
        "relative.flac",
    ] {
        let file = root.open_file(&at(path)).expect("the link is followed");
        assert_eq!(contents(&file), b"approved", "{path}");
    }
    // Until the admin approves the folder, the same links are refused.
    assert_eq!(
        opening(&scratch, "01.flac"),
        Err(outside(scratch.raw("debrid/Album/01.flac")))
    );
    assert_eq!(
        opening(&scratch, "Album/01.flac"),
        Err(outside(scratch.raw("debrid/Album")))
    );
}

/// The whole chain has to stay inside: a link in an approved folder that
/// leads on to somewhere else is refused there.
///
/// Verifies: SEC-MED-034
#[test]
fn refuses_a_chain_that_leaves_through_an_approved_folder() {
    let scratch = Scratch::new("fs-link-onward");
    scratch.dir("debrid");
    scratch.link("/etc/passwd", "debrid/onward.flac");
    scratch.link(scratch.absolute("debrid/onward.flac"), "music/track.flac");
    let root = scratch.root_with(approving(&scratch, "debrid"));
    assert_eq!(
        root.open_file(&at("track.flac")).map(|_| ()),
        Err(outside(raw("/etc/passwd")))
    );
}

/// Verifies: SEC-MED-034
#[test]
fn refuses_a_chain_whose_middle_link_has_an_audio_name() {
    let scratch = Scratch::new("fs-link-chain");
    scratch.link("hop.flac", "music/song.flac");
    scratch.link("/etc/passwd", "music/hop.flac");
    assert_eq!(
        opening(&scratch, "song.flac"),
        Err(outside(raw("/etc/passwd")))
    );
}

/// Verifies: SEC-OPS-055, SEC-MED-034
#[test]
fn refuses_links_into_the_servers_own_secrets_backups_and_database() {
    let scratch = Scratch::new("fs-link-data");
    for dir in ["data/secrets", "data/backups", "data/cache"] {
        scratch.dir(dir);
    }
    scratch.file("data/secrets/root.key", b"the root secret");
    scratch.file("data/backups/2026-10-05.gmb", b"a backup");
    scratch.file("data/cache/library.db", b"the database");
    scratch.link("../data/secrets/root.key", "music/key.flac");
    scratch.link(
        scratch.absolute("data/backups/2026-10-05.gmb"),
        "music/backup.flac",
    );
    scratch.link("../data/cache/library.db", "music/library.flac");
    scratch.link("../data/secrets", "music/Secrets");
    for (path, target) in [
        ("key.flac", "data/secrets/root.key"),
        ("backup.flac", "data/backups/2026-10-05.gmb"),
        ("library.flac", "data/cache/library.db"),
        ("Secrets/root.key", "data/secrets"),
    ] {
        assert_eq!(
            opening(&scratch, path),
            Err(outside(scratch.raw(target))),
            "{path}"
        );
    }
}

#[test]
fn reports_a_link_that_leads_nowhere() {
    let scratch = Scratch::new("fs-link-dangling");
    scratch.link("missing.flac", "music/dangling.flac");
    assert_eq!(
        opening(&scratch, "dangling.flac"),
        Err(io(Op::Inspect, ErrorKind::NotFound))
    );
}

#[test]
fn follows_as_many_links_as_the_kernel_would_and_no_more() {
    assert_eq!(MAX_LINKS, 40);
    let scratch = Scratch::new("fs-link-limit");
    scratch.file("music/track.flac", b"the end of the chain");
    // `link-39` leads to the track and every link before it to the next,
    // so `link-0` leads through forty links.
    scratch.link("track.flac", "music/link-39");
    for n in 0..39 {
        scratch.link(format!("link-{}", n + 1), &format!("music/link-{n}"));
    }
    let root = scratch.root();
    let file = root
        .open_file(&at("link-0"))
        .expect("forty links are followed");
    assert_eq!(contents(&file), b"the end of the chain");
    scratch.link("link-0", "music/one-more");
    assert_eq!(
        root.open_file(&at("one-more")).map(|_| ()),
        Err(FsError::TooManyLinks)
    );
}

#[test]
fn gives_up_on_links_that_lead_to_each_other() {
    let scratch = Scratch::new("fs-link-loop");
    scratch.link("b.flac", "music/a.flac");
    scratch.link("a.flac", "music/b.flac");
    scratch.link("itself", "music/itself");
    for path in ["a.flac", "itself", "itself/track.flac"] {
        assert_eq!(
            opening(&scratch, path),
            Err(FsError::TooManyLinks),
            "{path}"
        );
    }
}
