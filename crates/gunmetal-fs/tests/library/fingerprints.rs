//! Fingerprints and directory summaries. Every expected digest here was
//! made with `sha256sum` from the bytes the documentation says are hashed,
//! and cut to its first sixteen bytes.

use std::collections::BTreeSet;
use std::fs;
use std::io::ErrorKind;

use gunmetal_fs::fingerprint::{
    DirSummary, Fingerprint, Mark, SAMPLE_BYTES, fingerprint, summarise,
};
use gunmetal_fs::open::{Identity, Modified};
use gunmetal_fs::root::{FsError, Op};

use crate::support::{Scratch, at, hex, identity, io, summary};

/// The sample of a file holding `bytes`, as hexadecimal.
fn sample_of(bytes: &[u8]) -> String {
    let scratch = Scratch::new("fs-fingerprint-sample");
    scratch.file("music/track.flac", bytes);
    let file = scratch
        .root()
        .open_file(&at("track.flac"))
        .expect("the file opens");
    hex(fingerprint(&file).expect("the file reads").sample)
}

/// A file of `head`, `middle` and `tail` repeated to the lengths given.
fn layered(head: (u8, usize), middle: (u8, usize), tail: (u8, usize)) -> Vec<u8> {
    [head, middle, tail]
        .iter()
        .flat_map(|&(byte, count)| std::iter::repeat_n(byte, count))
        .collect()
}

#[test]
fn hashes_the_first_and_last_four_kilobytes_of_a_file() {
    assert_eq!(SAMPLE_BYTES, 4096);
    // Nothing; `fLaC` twice; one byte more than a sample, whose two samples
    // overlap in all but a byte each; and a file with a middle that is not
    // read at all.
    for (bytes, expected) in [
        (Vec::new(), "e3b0c44298fc1c149afbf4c8996fb924"),
        (b"fLaC".to_vec(), "8fb0fadb74842e4b7548da593b5013ba"),
        (
            layered((b'a', 1), (b'b', 4095), (b'c', 1)),
            "0348ff1c4e98c34f14f1aa0a24ff12a6",
        ),
        (
            layered((b'a', 4096), (b'b', 1000), (b'c', 4096)),
            "9afd3bbf26fe493fe898736a0d0e9802",
        ),
    ] {
        assert_eq!(sample_of(&bytes), expected, "{} bytes", bytes.len());
    }
}

#[test]
fn the_sample_sees_the_ends_of_a_file_and_not_its_middle() {
    let reference = sample_of(&layered((b'a', 4096), (b'b', 1000), (b'c', 4096)));
    assert_eq!(
        sample_of(&layered((b'a', 4096), (b'x', 9000), (b'c', 4096))),
        reference
    );
    assert_eq!(
        sample_of(&layered((b'a', 4096), (b'b', 1000), (b'd', 4096))),
        "e54dbb39b66e9613954ade97d9aa12d6"
    );
    assert_ne!(
        sample_of(&layered((b'z', 4096), (b'b', 1000), (b'c', 4096))),
        reference
    );
}

#[test]
fn a_fingerprint_carries_the_identity_of_the_open_file() {
    let scratch = Scratch::new("fs-fingerprint-identity");
    scratch.file("music/track.flac", b"fLaC");
    let file = scratch
        .root()
        .open_file(&at("track.flac"))
        .expect("the file opens");
    assert_eq!(
        fingerprint(&file),
        Ok(Fingerprint {
            identity: identity(&scratch.path("music/track.flac")),
            sample: [
                0x8f, 0xb0, 0xfa, 0xdb, 0x74, 0x84, 0x2e, 0x4b, 0x75, 0x48, 0xda, 0x59, 0x3b, 0x50,
                0x13, 0xba,
            ],
        })
    );
}

#[test]
fn reports_a_file_that_became_shorter_while_it_was_open() {
    let scratch = Scratch::new("fs-fingerprint-short");
    scratch.file("music/track.flac", b"fLaC and the rest");
    let file = scratch
        .root()
        .open_file(&at("track.flac"))
        .expect("the file opens");
    scratch.file("music/track.flac", b"");
    assert_eq!(
        fingerprint(&file),
        Err(io(Op::Read, ErrorKind::UnexpectedEof))
    );
}

/// Verifies: SEC-MED-036
#[test]
fn a_file_swapped_after_it_was_fingerprinted_is_refused_before_serving() {
    let scratch = Scratch::new("fs-fingerprint-swap");
    scratch.file("music/track.flac", b"the scanned file");
    scratch.file("music/other.flac", b"a different file");
    let root = scratch.root();
    let scanned = root.open_file(&at("track.flac")).expect("the file opens");
    let recorded = fingerprint(&scanned).expect("the file reads").identity;
    drop(scanned);
    fs::rename(
        scratch.path("music/other.flac"),
        scratch.path("music/track.flac"),
    )
    .expect("swap the file");
    assert_eq!(
        root.open_verified(&at("track.flac"), &recorded).map(|_| ()),
        Err(FsError::Changed {
            expected: recorded,
            found: identity(&scratch.path("music/track.flac")),
        })
    );
}

/// An identity with every field different from the others.
const IDENTITY: Identity = Identity {
    device: 1,
    inode: 2,
    size: 3,
    modified: Modified {
        seconds: 4,
        nanoseconds: 5,
    },
};

#[test]
fn summarises_a_listing_as_the_documentation_describes() {
    let listing: [(&[u8], Mark); 3] = [
        (b"a.flac".as_slice(), Mark::File(IDENTITY)),
        (b"b".as_slice(), Mark::Dir),
        (b"c\n".as_slice(), Mark::Skipped),
    ];
    assert_eq!(
        hex(summarise(listing).0),
        "d8a627aff4714daf49232681af9426b1"
    );
    assert_eq!(summarise(listing), summary(&listing));
    let nothing: [(&[u8], Mark); 0] = [];
    assert_eq!(
        hex(summarise(nothing).0),
        "e3b0c44298fc1c149afbf4c8996fb924"
    );
}

#[test]
fn a_summary_changes_with_every_part_of_a_listing() {
    let with = |change: fn(&mut Identity)| {
        let mut identity = IDENTITY;
        change(&mut identity);
        vec![(b"a".as_slice(), Mark::File(identity))]
    };
    let listings: [Vec<(&[u8], Mark)>; 13] = [
        with(|_| ()),
        with(|identity| identity.device = 9),
        with(|identity| identity.inode = 9),
        with(|identity| identity.size = 9),
        with(|identity| identity.modified.seconds = 9),
        with(|identity| identity.modified.nanoseconds = 9),
        vec![(b"b".as_slice(), Mark::File(IDENTITY))],
        vec![(b"a".as_slice(), Mark::Dir)],
        vec![(b"a".as_slice(), Mark::Skipped)],
        vec![
            (b"a".as_slice(), Mark::File(IDENTITY)),
            (b"b".as_slice(), Mark::Dir),
        ],
        // The same bytes, split into names at another place.
        vec![(b"ab".as_slice(), Mark::Dir), (b"c".as_slice(), Mark::Dir)],
        vec![(b"a".as_slice(), Mark::Dir), (b"bc".as_slice(), Mark::Dir)],
        Vec::new(),
    ];
    let summaries: Vec<DirSummary> = listings
        .iter()
        .map(|listing| summarise(listing.iter().copied()))
        .collect();
    let expected: Vec<DirSummary> = listings.iter().map(|listing| summary(listing)).collect();
    assert_eq!(summaries, expected);
    let distinct: BTreeSet<[u8; 16]> = summaries.iter().map(|summary| summary.0).collect();
    assert_eq!(distinct.len(), 13);
}
