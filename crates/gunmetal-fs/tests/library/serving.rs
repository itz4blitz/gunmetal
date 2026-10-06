//! The check before bytes are served: the file at the recorded path must
//! still be the file the index recorded.

use std::fs;

use gunmetal_fs::root::FsError;

use crate::support::{Scratch, at, contents, identity, outside};

/// Verifies: SEC-MED-036
#[test]
fn serves_a_file_that_is_still_the_one_recorded() {
    let scratch = Scratch::new("fs-serve-same");
    scratch.file("music/track.flac", b"fLaC");
    let root = scratch.root();
    let recorded = root
        .open_file(&at("track.flac"))
        .expect("the file opens")
        .identity();
    assert_eq!(recorded, identity(&scratch.path("music/track.flac")));
    let file = root
        .open_verified(&at("track.flac"), &recorded)
        .expect("the file is the one recorded");
    assert_eq!(contents(&file), b"fLaC");
}

/// The two files are the same size, so what gives the swap away is that the
/// path now names another inode.
///
/// Verifies: SEC-MED-036, SEC-API-018
#[test]
fn refuses_a_file_swapped_for_another_after_the_scan() {
    let scratch = Scratch::new("fs-serve-swap");
    scratch.file("music/track.flac", b"the scanned file");
    scratch.file("music/other.flac", b"a different file");
    let root = scratch.root();
    let recorded = identity(&scratch.path("music/track.flac"));
    fs::rename(
        scratch.path("music/other.flac"),
        scratch.path("music/track.flac"),
    )
    .expect("swap the file");
    let found = identity(&scratch.path("music/track.flac"));
    assert_eq!((recorded.size, found.size), (16, 16));
    assert_eq!(
        root.open_verified(&at("track.flac"), &recorded).map(|_| ()),
        Err(FsError::Changed {
            expected: recorded,
            found
        })
    );
}

/// Verifies: SEC-MED-036
#[test]
fn refuses_a_file_rewritten_in_place_after_the_scan() {
    let scratch = Scratch::new("fs-serve-rewrite");
    scratch.file("music/track.flac", b"fLaC");
    let root = scratch.root();
    let recorded = identity(&scratch.path("music/track.flac"));
    scratch.file("music/track.flac", b"fLaC and more");
    let found = identity(&scratch.path("music/track.flac"));
    // It is the same inode on the same device, so its size gave it away.
    assert_eq!(
        (found.device, found.inode, recorded.size, found.size),
        (recorded.device, recorded.inode, 4, 13)
    );
    assert_eq!(
        root.open_verified(&at("track.flac"), &recorded).map(|_| ()),
        Err(FsError::Changed {
            expected: recorded,
            found
        })
    );
}

/// Verifies: SEC-MED-036, SEC-API-018, SEC-OPS-055
#[test]
fn refuses_a_file_swapped_for_a_link_after_the_scan() {
    let scratch = Scratch::new("fs-serve-link");
    scratch.dir("outside");
    scratch.file("outside/secret", b"outside");
    scratch.file("music/track.flac", b"fLaC");
    scratch.file("music/other.flac", b"another track");
    let root = scratch.root();
    let recorded = identity(&scratch.path("music/track.flac"));
    // A link that leads out of the library is refused as a link.
    fs::remove_file(scratch.path("music/track.flac")).expect("remove the file");
    scratch.link("../outside/secret", "music/track.flac");
    assert_eq!(
        root.open_verified(&at("track.flac"), &recorded).map(|_| ()),
        Err(outside(scratch.raw("outside/secret")))
    );
    // A link that stays inside is followed, and leads to another file.
    fs::remove_file(scratch.path("music/track.flac")).expect("remove the link");
    scratch.link("other.flac", "music/track.flac");
    assert_eq!(
        root.open_verified(&at("track.flac"), &recorded).map(|_| ()),
        Err(FsError::Changed {
            expected: recorded,
            found: identity(&scratch.path("music/other.flac"))
        })
    );
}
