//! What the library tests share: a scratch directory with a library folder
//! in it, and independent readers for what a test needs to check there.

use std::fmt::Write as _;
use std::fs;
use std::os::unix::ffi::OsStrExt as _;
use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _, symlink};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::time::Duration;

use gunmetal_core::crypto::sha256;
use gunmetal_core::path::{RawPath, RelPath, normalise};
use gunmetal_core::untrusted::Untrusted;
use gunmetal_fs::fingerprint::{DirSummary, Mark};
use gunmetal_fs::open::{Identity, MediaFile, Modified};
use gunmetal_fs::root::{FsError, LinkPolicy, LinkReason, LinkRefusal, Op, Root};
use gunmetal_fs::walk::{Visit, Walk};
use gunmetal_testkit::tempdir::TempDir;
#[cfg(target_os = "linux")]
use rustix::fs::{CWD, Mode};

/// Long enough for anything that works; a test that waits this long has
/// found a hang, and fails instead of hanging itself.
pub const LONG: Duration = Duration::from_secs(30);

/// How long a test waits for a job that it knows will never finish.
pub const SHORT: Duration = Duration::from_millis(50);

/// A scratch directory that holds a library folder, `music`, and whatever
/// a test puts beside it: other libraries, approved folders, a data
/// directory, files that must stay out of reach.
pub struct Scratch {
    temp: TempDir,
}

impl Scratch {
    /// A scratch directory with an empty `music` folder in it.
    pub fn new(label: &str) -> Self {
        let scratch = Self {
            temp: TempDir::new(label).expect("a scratch directory"),
        };
        scratch.dir("music");
        scratch
    }

    /// Where `rel` is, by path.
    pub fn path(&self, rel: &str) -> PathBuf {
        self.temp.path().join(rel)
    }

    /// Where `rel` is, as the text of an absolute link: the resolved
    /// scratch directory and `rel`, with no link in `rel` followed.
    pub fn absolute(&self, rel: &str) -> PathBuf {
        fs::canonicalize(self.temp.path())
            .expect("the scratch directory resolves")
            .join(rel)
    }

    /// Where `rel` is, as link targets are compared: the resolved scratch
    /// directory and the names of `rel`, whether or not anything is there.
    pub fn raw(&self, rel: &str) -> RawPath {
        let text = self.absolute(rel);
        RawPath::parse(Untrusted::new(text.as_os_str().as_bytes())).expect("a canonical path")
    }

    /// Creates the directory `rel` and every directory above it.
    pub fn dir(&self, rel: &str) {
        fs::create_dir_all(self.path(rel)).expect("create a directory");
    }

    /// Writes the file `rel`.
    pub fn file(&self, rel: &str, bytes: &[u8]) {
        fs::write(self.path(rel), bytes).expect("write a file");
    }

    /// Makes `rel` a symbolic link whose text is `text`.
    pub fn link(&self, text: impl AsRef<Path>, rel: &str) {
        symlink(text, self.path(rel)).expect("make a symbolic link");
    }

    /// Opens `music` with the default policy, which approves no link
    /// target.
    pub fn root(&self) -> Root {
        self.root_with(LinkPolicy::default())
    }

    /// Opens `music` with `policy`.
    pub fn root_with(&self, policy: LinkPolicy) -> Root {
        Root::open(&self.path("music"), policy).expect("the root opens")
    }
}

/// The path beneath a root with these names.
pub fn rel(names: &[&[u8]]) -> RelPath {
    normalise(Untrusted::new(names)).expect("a path beneath the root")
}

/// The path beneath a root written as `text`, its names separated by `/`.
pub fn at(text: &str) -> RelPath {
    let names: Vec<&[u8]> = text.split('/').map(str::as_bytes).collect();
    rel(&names)
}

/// The canonical absolute path written as `text`.
pub fn raw(text: &str) -> RawPath {
    RawPath::parse(Untrusted::new(text.as_bytes())).expect("a canonical path")
}

/// The identity of what `path` leads to, read by path through the standard
/// library: an oracle independent of the handle the door opened.
pub fn identity(path: &Path) -> Identity {
    let metadata = fs::metadata(path).expect("metadata");
    Identity {
        device: metadata.dev(),
        inode: metadata.ino(),
        size: metadata.size(),
        modified: Modified {
            seconds: metadata.mtime(),
            nanoseconds: metadata.mtime_nsec(),
        },
    }
}

/// Everything in `file`.
pub fn contents(file: &MediaFile) -> Vec<u8> {
    let mut bytes = vec![0; 64];
    let read = file.read_at(&mut bytes, 0).expect("the file reads");
    assert!(read < bytes.len(), "a test file fits the buffer");
    bytes.truncate(read);
    bytes
}

/// Collects a walk, failing instead of hanging if it does not end.
pub fn collect(walk: Walk<'_>) -> Vec<Visit> {
    const CEILING: usize = 200;
    let visits: Vec<Visit> = walk.take(CEILING + 1).collect();
    assert!(
        visits.len() <= CEILING,
        "the walk did not end within {CEILING} visits"
    );
    visits
}

/// The summary of a listing, written out again here from the description
/// in the documentation so that it shares no code with the door.
pub fn summary(entries: &[(&[u8], Mark)]) -> DirSummary {
    let mut bytes: Vec<u8> = Vec::new();
    for (name, mark) in entries {
        let length = u64::try_from(name.len()).expect("a short name");
        bytes.extend(length.to_le_bytes());
        bytes.extend_from_slice(name);
        match mark {
            Mark::File(identity) => {
                bytes.push(0);
                for field in [identity.device, identity.inode, identity.size] {
                    bytes.extend(field.to_le_bytes());
                }
                for field in [identity.modified.seconds, identity.modified.nanoseconds] {
                    bytes.extend(field.to_le_bytes());
                }
            }
            Mark::Dir => bytes.push(1),
            Mark::Skipped => bytes.push(2),
        }
    }
    let digest = sha256(&bytes);
    DirSummary(digest[..16].try_into().expect("sixteen bytes"))
}

/// Sixteen bytes as lowercase hexadecimal, the way `sha256sum` prints them.
pub fn hex(bytes: [u8; 16]) -> String {
    bytes.iter().fold(String::new(), |mut text, byte| {
        write!(text, "{byte:02x}").expect("writing to a string");
        text
    })
}

/// The error of an operating-system call.
pub fn io(op: Op, kind: std::io::ErrorKind) -> FsError {
    FsError::Io { op, kind }
}

/// The refusal of a link that leads to `target`, which is in no library
/// and no approved folder.
pub fn outside(target: RawPath) -> FsError {
    FsError::Link(LinkRefusal {
        target,
        reason: LinkReason::Outside,
    })
}

/// Sets the permission bits of `path`.
pub fn set_mode(path: &Path, mode: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).expect("set the mode");
}

/// Makes a FIFO at `path`. rustix has no mkfifo helper off Linux, so the
/// tests that need a FIFO run there; elsewhere the socket and directory
/// entries stand in for a non-regular file.
#[cfg(target_os = "linux")]
pub fn fifo(path: &Path) {
    rustix::fs::mkfifoat(CWD, path, Mode::from_raw_mode(0o644)).expect("make a FIFO");
}

/// Makes a Unix socket at `path`. The socket file stays when the listener
/// is dropped.
pub fn socket(path: &Path) {
    drop(UnixListener::bind(path).expect("bind a socket"));
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
