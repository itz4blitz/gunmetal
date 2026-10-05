//! Root handles: resolved and opened once, and never redirected.

use std::fs;
use std::io::ErrorKind;

use gunmetal_fs::host::Filesystem;
use gunmetal_fs::root::{LinkPolicy, Op, Root, resolve};

use crate::support::{Scratch, at, contents, io};

#[test]
fn reports_the_resolved_path_of_the_folder_it_opened() {
    let scratch = Scratch::new("fs-root-path");
    scratch.link("music", "alias");
    assert_eq!(scratch.root().path(), &scratch.raw("music"));
    let through =
        Root::open(&scratch.path("alias"), LinkPolicy::default()).expect("the root opens");
    assert_eq!(through.path(), &scratch.raw("music"));
}

#[test]
fn resolves_a_path_to_the_form_roots_are_compared_in() {
    let scratch = Scratch::new("fs-root-resolve");
    scratch.dir("music/Album");
    scratch.link("music/Album", "alias");
    assert_eq!(
        resolve(&scratch.path("alias")),
        Ok(scratch.raw("music/Album"))
    );
    assert_eq!(
        resolve(&scratch.path("music/./Album/..")),
        Ok(scratch.raw("music"))
    );
    assert_eq!(
        resolve(&scratch.path("missing")),
        Err(io(Op::Resolve, ErrorKind::NotFound))
    );
}

#[test]
fn refuses_a_root_that_is_not_there() {
    let scratch = Scratch::new("fs-root-missing");
    assert_eq!(
        Root::open(&scratch.path("films"), LinkPolicy::default()).map(|_| ()),
        Err(io(Op::Resolve, ErrorKind::NotFound))
    );
}

#[test]
fn refuses_a_root_that_is_a_file() {
    let scratch = Scratch::new("fs-root-file");
    scratch.file("track.flac", b"fLaC");
    assert_eq!(
        Root::open(&scratch.path("track.flac"), LinkPolicy::default()).map(|_| ()),
        Err(io(Op::OpenRoot, ErrorKind::NotADirectory))
    );
}

#[test]
fn refuses_an_approved_folder_that_is_not_there() {
    let scratch = Scratch::new("fs-root-approved");
    let policy = LinkPolicy {
        approved: vec![scratch.path("debrid")],
        others: Vec::new(),
    };
    assert_eq!(
        Root::open(&scratch.path("music"), policy).map(|_| ()),
        Err(io(Op::Resolve, ErrorKind::NotFound))
    );
}

#[test]
fn reports_the_filesystem_the_root_is_on() {
    let scratch = Scratch::new("fs-root-filesystem");
    assert_eq!(scratch.root().filesystem(), Ok(Filesystem::Local));
}

/// The handle is opened once, so a folder moved away and replaced after
/// that is not the library: the root still reads the folder it opened.
///
/// Verifies: SEC-MED-033, SEC-TM-043
#[test]
fn keeps_reading_the_folder_it_opened_when_its_path_is_replaced() {
    let scratch = Scratch::new("fs-root-swap");
    scratch.file("music/track.flac", b"the library");
    let root = scratch.root();
    fs::rename(scratch.path("music"), scratch.path("moved")).expect("move the folder away");
    scratch.dir("music");
    scratch.file("music/track.flac", b"an impostor");
    let file = root.open_file(&at("track.flac")).expect("the file opens");
    assert_eq!(contents(&file), b"the library");
}
