//! Library root handles and the symbolic-link policy.
//!
//! [`Root::open`] resolves a library folder's configured path once and opens
//! it as a directory handle. From then on every file in that library is
//! reached beneath the handle (`openat2` with `RESOLVE_BENEATH` through
//! `cap-std`, SEC-MED-033, SEC-HIS-016, SEC-TM-043), so renaming or
//! replacing the folder's path afterwards does not redirect it, and no path
//! can climb out of it. The handle is private and the only operations on it
//! read: nothing in this crate can create, change, rename or delete inside a
//! library root (SEC-MED-038, SEC-TM-042, SEC-OPS-054).
//!
//! **Links** (SEC-MED-034). A path is resolved one name at a time, and no
//! symbolic link is left to the kernel. For each link the text is read and
//! judged by the core's path rules (WP-024): a target beneath the library's
//! own root, or beneath a folder the admin approved for this library, is
//! followed by carrying on beneath that root's handle; a target in another
//! library or anywhere else is refused with a [`LinkRefusal`] that names
//! it, so that the scan can list it. With the default [`LinkPolicy`] no
//! folder is approved, so every link that leaves the root is refused. The
//! final open is still confined by the kernel beneath the handle the
//! verdict named, so a link swapped during the check cannot redirect it.

use std::io;
use std::path::{Path, PathBuf};

use gunmetal_core::path::{PathError, RawPath};

use crate::host::Filesystem;
use crate::open::{FileKind, Identity};

/// The most symbolic links one path may lead through, which is the Linux
/// kernel's own limit. A loop of links ends here.
pub const MAX_LINKS: u8 = 40;

/// The operation that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// Resolving the configured path of a root.
    Resolve,
    /// Opening a root as a handle.
    OpenRoot,
    /// Reading the type of the filesystem a root is on.
    Probe,
    /// Reading an object's type and identity without opening it.
    Inspect,
    /// Reading where a symbolic link leads.
    ReadLink,
    /// Listing a directory.
    List,
    /// Opening a file.
    Open,
    /// Reading from an open file.
    Read,
}

/// Why a symbolic link was not followed (SEC-MED-034).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkReason {
    /// It leads into another library, whose files this library's users may
    /// not be allowed to see.
    OtherLibrary {
        /// Which of [`LinkPolicy::others`].
        index: usize,
    },
    /// It leads anywhere else: a system file, the server's own data, or a
    /// folder nobody approved.
    Outside,
}

/// A symbolic link that was not followed, with where it leads, so that the
/// problem list can group skipped links by target (SEC-MED-034, LIB-204).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkRefusal {
    /// Where the link leads, read from its text without touching the
    /// target.
    pub target: RawPath,
    /// Why it was refused.
    pub reason: LinkReason,
}

/// Why the library door refused an operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsError {
    /// An operating-system call failed.
    Io {
        /// What it was doing.
        op: Op,
        /// How it failed.
        kind: io::ErrorKind,
    },
    /// A name could not be part of a path beneath a root.
    Path(PathError),
    /// It is not a regular file: a directory, a FIFO, a socket or a device
    /// (SEC-MED-035).
    NotRegular {
        /// What was found.
        found: FileKind,
    },
    /// A symbolic link on the path may not be followed (SEC-MED-034).
    Link(LinkRefusal),
    /// The path led through more than [`MAX_LINKS`] symbolic links.
    TooManyLinks,
    /// The file is no longer the one the index recorded, so its bytes must
    /// not be served until it has been scanned again (SEC-MED-036).
    Changed {
        /// What the index recorded.
        expected: Identity,
        /// What is there now.
        found: Identity,
    },
    /// The walk already listed this directory under another path, as a link
    /// back to a folder above it would make it do for ever.
    AlreadyWalked,
}

/// What every operation answers until the door is built.
pub(crate) const UNBUILT: FsError = FsError::Io {
    op: Op::Open,
    kind: io::ErrorKind::Unsupported,
};

/// Which symbolic links a library's root follows (SEC-MED-034). A link that
/// stays beneath the root is always followed. The default approves nothing
/// else, so every link that leaves the root is refused (SEC-TM-043).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LinkPolicy {
    /// Folders the admin approved as link targets for this library. Each is
    /// opened as a handle of its own when the root opens.
    pub approved: Vec<PathBuf>,
    /// The resolved roots of every other library, as [`resolve`] reports
    /// them. A link into one is refused and named as such.
    pub others: Vec<RawPath>,
}

/// A library root, opened once.
#[derive(Debug)]
pub struct Root {
    path: RawPath,
}

/// Resolves `path`, following every link in it, to the canonical form that
/// roots are compared in.
///
/// # Errors
///
/// Returns [`FsError::Io`] with [`Op::Resolve`] when the path does not
/// exist or cannot be reached.
pub fn resolve(_path: &Path) -> Result<RawPath, FsError> {
    Err(UNBUILT)
}

impl Root {
    /// Resolves and opens the library folder at `path`, and each folder
    /// `policy` approves as a link target.
    ///
    /// # Errors
    ///
    /// Returns [`FsError::Io`] with [`Op::Resolve`] when a folder does not
    /// exist, and with [`Op::OpenRoot`] when it is not a directory or may
    /// not be read.
    pub fn open(_path: &Path, policy: LinkPolicy) -> Result<Self, FsError> {
        drop(policy);
        Err(UNBUILT)
    }

    /// The root's resolved path.
    #[must_use]
    pub const fn path(&self) -> &RawPath {
        &self.path
    }

    /// The type of the filesystem the root is on, for the library's health
    /// page. Every type is accepted, FUSE included (ADM-079).
    ///
    /// # Errors
    ///
    /// Returns [`FsError::Io`] with [`Op::Probe`] when the handle cannot be
    /// examined.
    pub fn filesystem(&self) -> Result<Filesystem, FsError> {
        Err(UNBUILT)
    }
}
