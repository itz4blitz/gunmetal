//! Whatever names a path is made of, it opens nothing outside the root.

use gunmetal_core::path::normalise;
use gunmetal_core::untrusted::Untrusted;
use proptest::prelude::*;

use crate::support::{Scratch, contents};

/// One name of a hostile path: traversal, separators and NUL, the names of
/// the links that lead out, and the names of what lies outside.
fn piece() -> impl Strategy<Value = &'static [u8]> {
    prop_oneof![
        Just(b"..".as_slice()),
        Just(b".".as_slice()),
        Just(b"".as_slice()),
        Just(b"/".as_slice()),
        Just(b"\0".as_slice()),
        Just(b"../outside/secret".as_slice()),
        Just(b"track.flac".as_slice()),
        Just(b"Album".as_slice()),
        Just(b"escape".as_slice()),
        Just(b"absolute".as_slice()),
        Just(b"climb".as_slice()),
        Just(b"inside".as_slice()),
        Just(b"up".as_slice()),
        Just(b"outside".as_slice()),
        Just(b"secret".as_slice()),
        Just(b"music".as_slice()),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Verifies: SEC-HIS-016, SEC-TM-043, SEC-OPS-055
    #[test]
    fn no_path_reads_the_file_outside_the_root(
        pieces in proptest::collection::vec(piece(), 0..7),
    ) {
        let scratch = Scratch::new("fs-confined");
        scratch.dir("outside");
        scratch.file("outside/secret", b"outside");
        scratch.dir("music/Album");
        scratch.file("music/track.flac", b"inside");
        scratch.file("music/Album/track.flac", b"inside");
        scratch.link("../outside", "music/escape");
        scratch.link(scratch.absolute("outside/secret"), "music/absolute");
        scratch.link("../../outside/secret", "music/Album/climb");
        scratch.link("../track.flac", "music/Album/inside");
        scratch.link("..", "music/Album/up");
        let root = scratch.root();
        // A path either cannot be written at all, or does not open, or
        // opens a file of the library.
        let read = normalise(Untrusted::new(pieces.as_slice()))
            .ok()
            .and_then(|path| root.open_file(&path).ok())
            .map(|file| contents(&file));
        prop_assert!(
            read.is_none() || read == Some(b"inside".to_vec()),
            "{:?} read {:?}",
            pieces,
            read
        );
    }
}
