//! The host probe on a real filesystem.
#![expect(
    clippy::disallowed_methods,
    reason = "tests of the filesystem door build and inspect hostile layouts on the real filesystem, by path (SEC-MED-033)"
)]

#[allow(dead_code, reason = "each test binary uses part of the helpers")]
mod support;

use std::fs;
use std::io::{self, ErrorKind};
use std::os::unix::fs::MetadataExt;

use gunmetal_fs::dataroot::{DataRootError, Item, Op};
use gunmetal_fs::host::{Filesystem, HostFacts};
use support::TempDir;

#[test]
fn reads_the_service_account_and_a_local_filesystem() {
    let dir = TempDir::new();
    // The kernel records the creating process's effective user ID as the
    // owner of a new file, which gives an oracle independent of the probe.
    fs::write(dir.join("witness"), b"").expect("write the witness");
    let owner = fs::metadata(dir.join("witness")).expect("metadata").uid();
    assert_eq!(
        HostFacts::probe(dir.path()),
        Ok(HostFacts {
            uid: owner,
            filesystem: Filesystem::Local
        })
    );
}

/// The filesystem type of an open folder. The expected value is Local
/// because the temp directory is on the machine's disk; this does not
/// distinguish a handle probe from a path probe on the same filesystem.
#[test]
fn reads_the_filesystem_type_of_an_open_folder_from_its_handle() {
    let dir = TempDir::new();
    fs::create_dir(dir.join("music")).expect("create the folder");
    let folder = fs::File::open(dir.join("music")).expect("open the folder");
    assert_eq!(
        Filesystem::of(&folder).as_ref().map_err(io::Error::kind),
        Ok(&Filesystem::Local)
    );
}

#[test]
fn reports_a_data_directory_it_cannot_examine() {
    let dir = TempDir::new();
    assert_eq!(
        HostFacts::probe(&dir.join("missing")),
        Err(DataRootError::Io {
            item: Item::Root,
            op: Op::Probe,
            kind: ErrorKind::NotFound
        })
    );
}
