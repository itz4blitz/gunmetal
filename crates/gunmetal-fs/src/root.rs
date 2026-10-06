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
#![expect(
    clippy::disallowed_methods,
    reason = "a library root is resolved and opened by path once, here; everything else is opened beneath its handle (SEC-MED-033, SEC-HIS-016)"
)]

use std::collections::VecDeque;
use std::ffi::OsString;
use std::fs::File;
use std::io;
use std::os::unix::ffi::{OsStrExt as _, OsStringExt as _};
use std::path::{Path, PathBuf};

use cap_std::fs::Dir;
use gunmetal_core::path::{
    LinkVerdict, PathError, RawPath, RelPath, classify_link, link_target, normalise,
};
use gunmetal_core::untrusted::Untrusted;
use rustix::fs::{Mode, OFlags};

use crate::host::Filesystem;
use crate::open::{Facts, FileKind, Identity};

/// The most symbolic links one path may lead through, which is the Linux
/// kernel's own limit. A loop of links ends here.
pub const MAX_LINKS: u8 = 40;

/// How a root is opened: read-only, as a directory and nothing else, and
/// closed when another program starts.
const ROOT_FLAGS: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);

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
    /// A directory holds more entries than the walk's limit allows
    /// ([`WalkLimits::entries`]), so none of it was listed.
    ///
    /// [`WalkLimits::entries`]: crate::walk::WalkLimits::entries
    TooManyEntries {
        /// The most entries a directory may hold.
        max: u64,
    },
    /// A directory lies deeper than the walk's limit allows
    /// ([`WalkLimits::depth`]), so it was not entered.
    ///
    /// [`WalkLimits::depth`]: crate::walk::WalkLimits::depth
    TooDeep {
        /// How deep it lies. The root lies at depth 0.
        depth: u64,
        /// The deepest a directory may lie.
        max: u64,
    },
    /// What was opened is not the object whose path was judged a moment
    /// before: something replaced it in between, such as a symbolic link
    /// swapped in for it, so it was not used (SEC-MED-034).
    Replaced,
    /// A folder approved as a link target could not be opened, so the root
    /// was not opened either.
    Approved {
        /// Which of [`LinkPolicy::approved`].
        index: usize,
        /// Why it could not be opened.
        reason: Box<Self>,
    },
}

/// Maps an operating-system error during `op`.
pub(crate) fn io_error(op: Op) -> impl FnOnce(io::Error) -> FsError {
    move |error| FsError::Io {
        op,
        kind: error.kind(),
    }
}

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

/// One folder opened as a handle: the library's root or an approved link
/// target.
#[derive(Debug)]
pub(crate) struct Base {
    /// The handle everything beneath the folder is opened through.
    pub(crate) dir: Dir,
    /// The folder's resolved path, which link targets are compared with.
    pub(crate) path: RawPath,
}

/// Where a path leads once every link on it has been followed.
pub(crate) struct Found<'r> {
    /// The folder it is beneath.
    pub(crate) base: &'r Base,
    /// Its names beneath that folder, none of them a link.
    pub(crate) names: Vec<Vec<u8>>,
    /// What is there.
    pub(crate) facts: Facts,
}

/// A library root, opened once.
#[derive(Debug)]
pub struct Root {
    own: Base,
    approved: Vec<Base>,
    targets: Vec<RawPath>,
    others: Vec<RawPath>,
    top: RelPath,
}

/// Resolves `path`, following every link in it, to the canonical form that
/// roots are compared in.
///
/// # Errors
///
/// Returns [`FsError::Io`] with [`Op::Resolve`] when the path does not
/// exist or cannot be reached.
pub fn resolve(path: &Path) -> Result<RawPath, FsError> {
    canonical(path).map(|(_, raw)| raw)
}

/// The resolved form of `path`, as a path to open and as a path to compare.
fn canonical(path: &Path) -> Result<(PathBuf, RawPath), FsError> {
    std::fs::canonicalize(path)
        .map_err(io_error(Op::Resolve))
        .and_then(|resolved| {
            RawPath::parse(Untrusted::new(resolved.as_os_str().as_bytes()))
                .map(|raw| (resolved, raw))
                .map_err(FsError::Path)
        })
}

/// The path of `names` beneath a handle, or `.` for the handle's own
/// directory. The names stay bytes (SEC-MED-040).
pub(crate) fn os_path(names: &[Vec<u8>]) -> OsString {
    if names.is_empty() {
        OsString::from(".")
    } else {
        OsString::from_vec(names.join(&b'/'))
    }
}

/// `names` as a path beneath a root.
pub(crate) fn rel(names: &[Vec<u8>]) -> Result<RelPath, FsError> {
    let names: Vec<&[u8]> = names.iter().map(Vec::as_slice).collect();
    normalise(Untrusted::new(names.as_slice())).map_err(FsError::Path)
}

/// The path of the entry `name` in the directory `dir`.
pub(crate) fn child(dir: &RelPath, name: &[u8]) -> Result<RelPath, FsError> {
    let names: Vec<&[u8]> = dir
        .components()
        .iter()
        .map(Vec::as_slice)
        .chain(std::iter::once(name))
        .collect();
    normalise(Untrusted::new(names.as_slice())).map_err(FsError::Path)
}

impl Base {
    /// Resolves `path` and opens the folder there.
    fn at(path: &Path) -> Result<Self, FsError> {
        canonical(path).and_then(|(resolved, path)| {
            rustix::fs::open(resolved, ROOT_FLAGS, Mode::empty())
                .map(|fd| Self {
                    dir: Dir::from_std_file(File::from(fd)),
                    path,
                })
                .map_err(io::Error::from)
                .map_err(io_error(Op::OpenRoot))
        })
    }

    /// What is at `names`, without following a link there and without
    /// opening it.
    pub(crate) fn inspect(&self, names: &[Vec<u8>]) -> Result<Facts, FsError> {
        self.dir
            .symlink_metadata(os_path(names))
            .map(|metadata| Facts::of(&metadata))
            .map_err(io_error(Op::Inspect))
    }

    /// The text of the symbolic link at `names`.
    fn link_text(&self, names: &[Vec<u8>]) -> Result<Vec<u8>, FsError> {
        self.dir
            .read_link_contents(os_path(names))
            .map(|text| text.into_os_string().into_vec())
            .map_err(io_error(Op::ReadLink))
    }

    /// The entries of the directory at `names`, in name order, each with
    /// its path beneath the directory shown as `shown`.
    pub(crate) fn list(
        &self,
        shown: &RelPath,
        names: &[Vec<u8>],
    ) -> Result<Vec<(RelPath, Vec<u8>)>, FsError> {
        self.dir
            .read_dir(os_path(names))
            .and_then(|entries| {
                entries
                    .map(|entry| entry.map(|entry| entry.file_name().into_vec()))
                    .collect::<io::Result<Vec<Vec<u8>>>>()
            })
            .map_err(io_error(Op::List))
            .and_then(|mut found| {
                found.sort();
                found
                    .into_iter()
                    .map(|name| child(shown, &name).map(|path| (path, name)))
                    .collect()
            })
    }
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
    pub fn open(path: &Path, policy: LinkPolicy) -> Result<Self, FsError> {
        let own = Base::at(path)?;
        let approved = policy
            .approved
            .iter()
            .map(|target| Base::at(target))
            .collect::<Result<Vec<_>, _>>()?;
        let targets = approved.iter().map(|base| base.path.clone()).collect();
        rel(&[]).map(|top| Self {
            own,
            approved,
            targets,
            others: policy.others,
            top,
        })
    }

    /// The root's resolved path.
    #[must_use]
    pub const fn path(&self) -> &RawPath {
        &self.own.path
    }

    /// The type of the filesystem the root is on, for the library's health
    /// page. Every type is accepted, FUSE included (ADM-079).
    ///
    /// # Errors
    ///
    /// Returns [`FsError::Io`] with [`Op::Probe`] when the handle cannot be
    /// examined.
    pub fn filesystem(&self) -> Result<Filesystem, FsError> {
        Filesystem::of(&self.own.dir).map_err(io_error(Op::Probe))
    }

    /// The library's own folder.
    pub(crate) const fn own(&self) -> &Base {
        &self.own
    }

    /// The path of the root itself: no names.
    pub(crate) const fn top(&self) -> &RelPath {
        &self.top
    }

    /// Follows `names` from the directory `dir` beneath `start`, one name
    /// at a time, judging every symbolic link on the way, and reports where
    /// they lead and what is there.
    pub(crate) fn locate<'r>(
        &'r self,
        start: &'r Base,
        dir: &[Vec<u8>],
        names: &[Vec<u8>],
    ) -> Result<Found<'r>, FsError> {
        let mut base = start;
        let mut done = dir.to_vec();
        let mut todo: VecDeque<Vec<u8>> = names.iter().cloned().collect();
        let mut last = None;
        let mut fuel = MAX_LINKS;
        while let Some(name) = todo.pop_front() {
            done.push(name);
            let facts = base.inspect(&done)?;
            if facts.kind == FileKind::Symlink {
                fuel = fuel.checked_sub(1).ok_or(FsError::TooManyLinks)?;
                let (next, rest) = self.follow(base, &done)?;
                base = next;
                done.clear();
                todo = rest.components().iter().cloned().chain(todo).collect();
                last = None;
            } else {
                last = Some(facts);
            }
        }
        last.map_or_else(|| base.inspect(&done), Ok)
            .map(|facts| Found {
                base,
                names: done,
                facts,
            })
    }

    /// Reads the symbolic link at `link` beneath `base` and decides where
    /// to carry on: the folder its target is beneath, and the path of the
    /// target there.
    fn follow(&self, base: &Base, link: &[Vec<u8>]) -> Result<(&Base, RelPath), FsError> {
        let dir = link.split_last().map_or(link, |(_, dir)| dir);
        base.link_text(link)
            .and_then(|text| {
                rel(dir).and_then(|dir| {
                    link_target(&base.path, &dir, Untrusted::new(text.as_slice()))
                        .map_err(FsError::Path)
                })
            })
            .and_then(|target| self.admit(target))
    }

    /// Decides whether a link that leads to `target` may be followed
    /// (SEC-MED-034).
    fn admit(&self, target: RawPath) -> Result<(&Base, RelPath), FsError> {
        let verdict = classify_link(&target, &self.own.path, &self.targets, &self.others);
        let refuse = |reason| FsError::Link(LinkRefusal { target, reason });
        match verdict {
            LinkVerdict::Own(rest) => Ok((&self.own, rest)),
            LinkVerdict::Approved { index, rest } => self
                .approved
                .get(index)
                .map(|base| (base, rest))
                .ok_or(refuse(LinkReason::Outside)),
            LinkVerdict::OtherLibrary { index } => Err(refuse(LinkReason::OtherLibrary { index })),
            LinkVerdict::Outside => Err(refuse(LinkReason::Outside)),
        }
    }
}
