//! The host probe on a real filesystem.

#[allow(dead_code, reason = "each test binary uses part of the helpers")]
mod support;

use std::fs;
use std::io::ErrorKind;
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
