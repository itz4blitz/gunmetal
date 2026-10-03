//! Helpers shared by the integration tests: a temporary directory on the
//! real filesystem, and readers for what a test needs to check there.
//!
//! The testkit's temporary directories (WP-007) are built in the same wave
//! as this crate, so these tests carry their own until both have merged.

use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use gunmetal_fs::dataroot::{DataRoot, Opened, Policy};
use gunmetal_fs::host::{Filesystem, HostFacts};

static NEXT: AtomicUsize = AtomicUsize::new(0);

/// A directory under the system's temporary directory, with mode 0700,
/// removed with everything in it when dropped.
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "gunmetal-fs-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir(&path).expect("create the temporary directory");
        set_mode(&path, 0o700);
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn join(&self, rel: &str) -> PathBuf {
        self.path.join(rel)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        unlock(&self.path);
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Gives the owner full access to every directory below `path`, so that a
/// test that took permissions away can still be cleaned up.
fn unlock(path: &Path) {
    let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o700));
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                unlock(&entry.path());
            }
        }
    }
}

/// Fails loudly when the tests run as root, which ignores file modes and
/// would let a permission test pass for the wrong reason.
pub fn assert_not_root() {
    assert_ne!(
        rustix::process::geteuid().as_raw(),
        0,
        "these tests check file modes and must not run as root"
    );
}

/// The effective user ID of this process.
pub fn uid() -> u32 {
    rustix::process::geteuid().as_raw()
}

/// The host as these tests see it: this process's user on a local disk.
pub fn local_host() -> HostFacts {
    HostFacts {
        uid: uid(),
        filesystem: Filesystem::Local,
    }
}

/// Opens the data root in `dir/data`, creating that directory with mode
/// 0700 first if it does not exist.
pub fn open(dir: &TempDir) -> Opened {
    let data = dir.join("data");
    if !data.exists() {
        fs::create_dir(&data).expect("create the data directory");
        set_mode(&data, 0o700);
    }
    DataRoot::open(&data, &local_host(), Policy::DEFAULT).expect("the data root opens")
}

/// The permission bits of `path`, without following a final symlink.
pub fn mode(path: &Path) -> u32 {
    fs::symlink_metadata(path).expect("metadata").mode() & 0o7777
}

/// The owner of `path`, without following a final symlink.
pub fn owner(path: &Path) -> u32 {
    fs::symlink_metadata(path).expect("metadata").uid()
}

pub fn set_mode(path: &Path, mode: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).expect("set the mode");
}

/// The names in directory `path`, sorted.
pub fn names(path: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(path)
        .expect("list the directory")
        .map(|entry| {
            entry
                .expect("directory entry")
                .file_name()
                .into_string()
                .expect("UTF-8 name")
        })
        .collect();
    names.sort();
    names
}
