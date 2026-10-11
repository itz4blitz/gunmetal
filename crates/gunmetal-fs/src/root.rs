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
//! **Links** (SEC-MED-034). A path is resolved one name at a time, and
//! every symbolic link on it is judged before anything is opened. For each
//! link the text is read and judged by the core's path rules (WP-024): a
//! target beneath the library's own root, or beneath a folder the admin
//! approved for this library, is followed by carrying on beneath that
//! root's handle; a target in another library or anywhere else is refused
//! with a [`LinkRefusal`] that names it, so that the scan can list it. With
//! the default [`LinkPolicy`] no folder is approved, so every link that
//! leaves the root is refused. `..` in a link's text is collapsed by name,
//! before the target is judged, as the core's rules do.
//!
//! The open itself is confined by the kernel beneath the handle the verdict
//! named, so a link swapped in after the judgement cannot lead out of that
//! handle. In the last name it is not followed either: where cap-std opens
//! with `openat2` the kernel refuses it (`O_NOFOLLOW`), and where cap-std
//! resolves the path by hand the door refuses what it opened unless it is
//! the very object judged, by device and inode ([`FsError::Replaced`]). A
//! link swapped in for a directory above the last name is followed by the
//! kernel beneath the same handle: confined, but not judged.
#![expect(
    clippy::disallowed_methods,
    reason = "a library root is resolved and opened by path once, here, and the unit tests below build scratch libraries by path; everything else is opened beneath its handle (SEC-MED-033, SEC-HIS-016)"
)]

use std::collections::VecDeque;
use std::ffi::OsString;
use std::fs::File;
use std::io;
use std::os::unix::ffi::{OsStrExt as _, OsStringExt as _};
use std::path::{Path, PathBuf};

use cap_std::fs::{Dir, OpenOptions, OpenOptionsExt as _};
use gunmetal_core::path::{
    LinkVerdict, PathError, RawPath, RelPath, classify_link, link_target, normalise,
};
use gunmetal_core::untrusted::Untrusted;
use rustix::fs::{Mode, OFlags};
use rustix::io::Errno;

use crate::host::Filesystem;
use crate::open::{Facts, FileKind, Identity};

/// The most symbolic links one path may lead through, which is the Linux
/// kernel's own limit. A loop of links ends here.
pub const MAX_LINKS: u8 = 40;

/// The longest text, in bytes, a symbolic link may hold and still be
/// judged: `_XOPEN_PATH_MAX`, the longest path POSIX lets a portable
/// program count on. A link with longer text is refused without any of it
/// being resolved ([`FsError::LinkTooLong`]), so the work one link can ask
/// for is bounded by this and not by what the filesystem will store.
pub const MAX_LINK_TEXT: usize = 1024;

/// How a root is opened: read-only, as a directory and nothing else, and
/// closed when another program starts.
const ROOT_FLAGS: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);

/// How a directory beneath a root is opened to be listed, besides
/// read-only: as a directory and nothing else, not through a link in its
/// last name, and closed when another program starts.
const LIST_FLAGS: i32 = i32::from_ne_bytes(
    OFlags::DIRECTORY
        .union(OFlags::NOFOLLOW)
        .union(OFlags::CLOEXEC)
        .bits()
        .to_ne_bytes(),
);

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
    /// A symbolic link's text is longer than [`MAX_LINK_TEXT`] bytes, so
    /// it was neither judged nor followed.
    LinkTooLong {
        /// How long its text is, in bytes.
        len: usize,
        /// The longest text that is judged.
        max: usize,
    },
    /// The file is no longer the one the index recorded, so its bytes must
    /// not be served until it has been scanned again (SEC-MED-036).
    Changed {
        /// What the index recorded.
        expected: Identity,
        /// What is there now.
        found: Identity,
    },
    /// The walk already came to this directory under another path, as a
    /// link back to a folder above it would make it do for ever.
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
    /// swapped in for it, so it was not used (SEC-MED-034). A walk also
    /// reports this for an entry that was a refused link when its directory
    /// was listed and is something else when the entry is reported. It is
    /// a passing state: the entry is still there, and is worth another
    /// look.
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

/// Maps the failure of an open during `op`. The opens beneath a root do not
/// follow a link in the last name, and the kernel says so with `ELOOP`: the
/// path was judged to hold no link there, so something has replaced what
/// was judged (SEC-MED-034).
fn opening(op: Op) -> impl FnOnce(io::Error) -> FsError {
    move |error| {
        if error.raw_os_error() == Some(Errno::LOOP.raw_os_error()) {
            FsError::Replaced
        } else {
            FsError::Io {
                op,
                kind: error.kind(),
            }
        }
    }
}

/// Whether `opened` is the object `judged` describes: the same device and
/// inode.
const fn same(opened: &Facts, judged: &Facts) -> bool {
    opened.identity.device == judged.identity.device
        && opened.identity.inode == judged.identity.inode
}

/// The options a directory is opened with to be listed.
fn listing() -> OpenOptions {
    OpenOptions::new()
        .read(true)
        .custom_flags(LIST_FLAGS)
        .clone()
}

/// The names of the entries of `dir`, in name order: all of them, or
/// [`FsError::TooManyEntries`] when it holds more than `max`, which is known
/// once one more than `max` has been read. No more are read.
pub(crate) fn read_names(dir: &Dir, max: u64) -> Result<Vec<Vec<u8>>, FsError> {
    let most = usize::try_from(max).unwrap_or(usize::MAX);
    dir.entries()
        .and_then(|entries| {
            entries
                .take(most.saturating_add(1))
                .map(|entry| entry.map(|entry| entry.file_name().into_vec()))
                .collect::<io::Result<Vec<Vec<u8>>>>()
        })
        .map_err(io_error(Op::List))
        .and_then(|mut names| {
            if names.len() > most {
                Err(FsError::TooManyEntries { max })
            } else {
                names.sort();
                Ok(names)
            }
        })
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

    /// Opens what is at `names` beneath this folder with `options`, during
    /// `op`, and refuses it unless it is the object `judged` describes, the
    /// one whose path was judged a moment before: the same device and
    /// inode. A link swapped in for the last name since is refused by the
    /// kernel where cap-std opens with `openat2`; where cap-std resolves the
    /// path by hand, it is resolved beneath this folder and what it leads to
    /// is refused here (SEC-MED-034).
    pub(crate) fn open_judged(
        &self,
        names: &[Vec<u8>],
        options: &OpenOptions,
        judged: &Facts,
        op: Op,
    ) -> Result<(File, Facts), FsError> {
        self.dir
            .open_with(os_path(names), options)
            .and_then(|file| {
                file.metadata()
                    .map(|metadata| (file.into_std(), Facts::of(&metadata)))
            })
            .map_err(opening(op))
            .and_then(|(file, facts)| {
                if same(&facts, judged) {
                    Ok((file, facts))
                } else {
                    Err(FsError::Replaced)
                }
            })
    }

    /// Opens the directory at `names`, judged a moment before to be
    /// `judged`, to list it, with its identity read from the open handle.
    pub(crate) fn open_dir(
        &self,
        names: &[Vec<u8>],
        judged: &Facts,
    ) -> Result<(Dir, Identity), FsError> {
        self.open_judged(names, &listing(), judged, Op::List)
            .map(|(file, facts)| (Dir::from_std_file(file), facts.identity))
    }
}

impl Root {
    /// Resolves and opens the library folder at `path`, and each folder
    /// `policy` approves as a link target.
    ///
    /// # Errors
    ///
    /// Returns [`FsError::Io`] with [`Op::Resolve`] when the folder does not
    /// exist, and with [`Op::OpenRoot`] when it is not a directory or may
    /// not be read. Returns [`FsError::Approved`], naming the folder and
    /// holding one of those, when an approved folder cannot be opened.
    pub fn open(path: &Path, policy: LinkPolicy) -> Result<Self, FsError> {
        let own = Base::at(path)?;
        let approved = policy
            .approved
            .iter()
            .enumerate()
            .map(|(index, target)| {
                Base::at(target).map_err(|reason| FsError::Approved {
                    index,
                    reason: Box::new(reason),
                })
            })
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
                if text.len() > MAX_LINK_TEXT {
                    Err(FsError::LinkTooLong {
                        len: text.len(),
                        max: MAX_LINK_TEXT,
                    })
                } else {
                    Ok(text)
                }
            })
            .and_then(|text| {
                rel(dir).and_then(|dir| {
                    link_target(&base.path, &dir, Untrusted::new(text.as_slice()))
                        .map_err(FsError::Path)
                })
            })
            .and_then(|target| self.admit(target))
    }

    /// Decides whether a link that leads to `target` may be followed
    /// (SEC-MED-034). An approved folder's index always names one of
    /// `approved`, which [`Root::open`] fills from the same list as
    /// `targets`; were it ever not, the link would be refused like any
    /// other that leads outside, by the same code.
    fn admit(&self, target: RawPath) -> Result<(&Base, RelPath), FsError> {
        let verdict = classify_link(&target, &self.own.path, &self.targets, &self.others);
        let refuse = |reason| FsError::Link(LinkRefusal { target, reason });
        let followed = match verdict {
            LinkVerdict::Own(rest) => Some((&self.own, rest)),
            LinkVerdict::Approved { index, rest } => {
                self.approved.get(index).map(|base| (base, rest))
            }
            LinkVerdict::OtherLibrary { index } => {
                return Err(refuse(LinkReason::OtherLibrary { index }));
            }
            LinkVerdict::Outside => None,
        };
        followed.ok_or_else(|| refuse(LinkReason::Outside))
    }
}

/// What cannot be shown through the public interface without a race: what
/// the door does when the object at a path changes between the moment the
/// path is judged and the moment it is opened. Each test hands the door a
/// judgement made before the change, as a swap in that moment would.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::pool::Pool;
    use gunmetal_testkit::tempdir::TempDir;
    use std::time::Duration;

    /// A scratch directory holding a library folder with two regular
    /// files, `track.flac` and `other.flac`, and the root opened on it.
    fn library(label: &str) -> (TempDir, Root) {
        let temp = TempDir::new(label).expect("a scratch directory");
        let music = temp.path().join("music");
        std::fs::create_dir(&music).expect("create the library");
        std::fs::write(music.join("track.flac"), b"the track").expect("write a file");
        std::fs::write(music.join("other.flac"), b"another").expect("write a file");
        let root = Root::open(&music, LinkPolicy::default()).expect("the root opens");
        (temp, root)
    }

    /// The same refusals the library tests check, from this binary, so the
    /// error paths of [`Root::open`] are covered in every instantiation.
    #[test]
    fn refuses_a_root_or_an_approved_folder_that_cannot_be_opened() {
        let temp = TempDir::new("fs-unit-open-refuse").expect("a scratch directory");
        assert_eq!(
            Root::open(&temp.path().join("missing"), LinkPolicy::default()).map(|_| ()),
            Err(FsError::Io {
                op: Op::Resolve,
                kind: std::io::ErrorKind::NotFound,
            })
        );
        std::fs::write(temp.path().join("track.flac"), b"fLaC").expect("a file");
        assert_eq!(
            Root::open(&temp.path().join("track.flac"), LinkPolicy::default()).map(|_| ()),
            Err(FsError::Io {
                op: Op::OpenRoot,
                kind: std::io::ErrorKind::NotADirectory,
            })
        );
        let music = temp.path().join("music");
        std::fs::create_dir(&music).expect("create the library");
        assert_eq!(
            Root::open(
                &music,
                LinkPolicy {
                    approved: vec![temp.path().join("debrid")],
                    others: Vec::new(),
                }
            )
            .map(|_| ()),
            Err(FsError::Approved {
                index: 0,
                reason: Box::new(FsError::Io {
                    op: Op::Resolve,
                    kind: std::io::ErrorKind::NotFound,
                }),
            })
        );
        std::fs::create_dir(temp.path().join("debrid")).expect("an approved folder");
        assert_eq!(
            Root::open(
                &music,
                LinkPolicy {
                    approved: vec![temp.path().join("debrid"), temp.path().join("track.flac")],
                    others: Vec::new(),
                }
            )
            .map(|_| ()),
            Err(FsError::Approved {
                index: 1,
                reason: Box::new(FsError::Io {
                    op: Op::OpenRoot,
                    kind: std::io::ErrorKind::NotADirectory,
                }),
            })
        );
    }

    /// The names of the entry `name` of the root.
    fn names(name: &str) -> Vec<Vec<u8>> {
        vec![name.as_bytes().to_vec()]
    }

    /// What the entry `name` of the root is now, as a walk or an open
    /// judges it before opening it.
    fn judge(root: &Root, name: &str) -> Facts {
        root.own()
            .inspect(&names(name))
            .expect("the entry is there")
    }

    /// A link swapped in for a file after the file was judged is not
    /// followed to what it names. Where the kernel resolves the path the
    /// open refuses the link itself; where cap-std resolves it by hand the
    /// open reaches `other.flac`, which is not the file judged, or gives up
    /// on the link that leads only to itself. The door reports each as the
    /// same swap.
    ///
    /// Verifies: SEC-MED-034
    #[test]
    fn refuses_a_link_swapped_in_for_a_file_after_it_was_judged() {
        let (temp, root) = library("fs-unit-swapped-link");
        let judged = judge(&root, "track.flac");
        let track = temp.path().join("music/track.flac");
        for text in ["other.flac", "track.flac"] {
            std::fs::remove_file(&track).expect("remove what is there");
            std::os::unix::fs::symlink(text, &track).expect("make a link");
            assert_eq!(
                root.own().file(&names("track.flac"), &judged).err(),
                Some(FsError::Replaced),
                "{text}"
            );
        }
    }

    /// Verifies: SEC-MED-034
    #[test]
    fn refuses_a_file_that_is_not_the_one_judged() {
        let (_temp, root) = library("fs-unit-another-file");
        let judged = judge(&root, "other.flac");
        assert_eq!(
            root.own().file(&names("track.flac"), &judged).err(),
            Some(FsError::Replaced)
        );
    }

    /// A file removed after it was judged fails the open as I/O, not as a
    /// replacement: the kernel says the name is gone, not that a link is
    /// there. Covers that branch of the open-error map in this binary.
    ///
    /// Verifies: SEC-MED-034
    #[test]
    fn reports_a_file_removed_after_it_was_judged_as_io() {
        let (temp, root) = library("fs-unit-removed");
        let judged = judge(&root, "track.flac");
        std::fs::remove_file(temp.path().join("music/track.flac")).expect("remove");
        assert_eq!(
            root.own().file(&names("track.flac"), &judged).err(),
            Some(FsError::Io {
                op: Op::Open,
                kind: std::io::ErrorKind::NotFound,
            })
        );
    }

    /// Link policy refusals and the too-many-links bound, from this binary.
    ///
    /// Verifies: SEC-MED-034
    #[test]
    fn refuses_links_the_policy_forbids_and_a_chain_that_is_too_long() {
        let temp = TempDir::new("fs-unit-policy").expect("a scratch directory");
        let music = temp.path().join("music");
        let other = temp.path().join("films");
        std::fs::create_dir(&music).expect("library");
        std::fs::create_dir(&other).expect("other library");
        std::fs::write(music.join("track.flac"), b"the track").expect("write");
        std::fs::write(other.join("x.flac"), b"other").expect("write");
        std::os::unix::fs::symlink("/etc/passwd", music.join("out.flac")).expect("outside");
        std::os::unix::fs::symlink("../films/x.flac", music.join("lib.flac")).expect("other lib");
        let root = Root::open(
            &music,
            LinkPolicy {
                approved: Vec::new(),
                others: vec![resolve(&other).expect("resolve the other library")],
            },
        )
        .expect("open");
        assert_eq!(
            root.open_file(&rel(&names("out.flac")).expect("path"))
                .map(|_| ()),
            Err(FsError::Link(LinkRefusal {
                target: RawPath::parse(Untrusted::new(b"/etc/passwd")).expect("path"),
                reason: LinkReason::Outside,
            }))
        );
        assert_eq!(
            root.open_file(&rel(&names("lib.flac")).expect("path"))
                .map(|_| ()),
            Err(FsError::Link(LinkRefusal {
                target: resolve(&other.join("x.flac")).expect("resolve"),
                reason: LinkReason::OtherLibrary { index: 0 },
            }))
        );
        // Links that lead only to each other trip the same bound.
        std::os::unix::fs::symlink("b.flac", music.join("a.flac")).expect("loop a");
        std::os::unix::fs::symlink("a.flac", music.join("b.flac")).expect("loop b");
        assert_eq!(
            root.open_file(&rel(&names("a.flac")).expect("path"))
                .map(|_| ()),
            Err(FsError::TooManyLinks)
        );
        // An approved folder is followed from this binary too.
        let debrid = temp.path().join("debrid");
        std::fs::create_dir(&debrid).expect("approved");
        std::fs::write(debrid.join("ok.flac"), b"ok").expect("write");
        std::os::unix::fs::symlink("../debrid/ok.flac", music.join("approved.flac"))
            .expect("approved link");
        let with_approved = Root::open(
            &music,
            LinkPolicy {
                approved: vec![debrid],
                others: Vec::new(),
            },
        )
        .expect("open with approved");
        with_approved
            .open_file(&rel(&names("approved.flac")).expect("path"))
            .expect("the approved link is followed");
    }

    /// The same length bound the library tests check, from this binary, so
    /// the path that refuses over-long link text is covered here as well.
    /// A link at the limit is followed, so the path that accepts one is
    /// covered in this instantiation too.
    ///
    /// Verifies: SEC-MED-034
    #[test]
    fn judges_link_text_up_to_the_limit_from_this_binary() {
        let (temp, root) = library("fs-unit-link-length");
        assert_eq!(
            root.own().inspect(&[]).map(|facts| facts.kind),
            Ok(FileKind::Dir)
        );
        // A link at the limit is followed, and one over it is refused.
        // Linux's own limit on a link's text is 4096, so a text at and
        // over Gunmetal's 1024 can exist there. Where the kernel's
        // PATH_MAX is 1024, its own limit is one below Gunmetal's, so the
        // longest link that can exist is made instead; the refusal is
        // proven where an over-limit link can exist.
        #[cfg(target_os = "linux")]
        {
            let at_the_limit = format!("{}track.flac", "./".repeat(507));
            let over = format!("{}/track.flac", "./".repeat(507));
            assert_eq!((at_the_limit.len(), over.len()), (1024, 1025));
            std::os::unix::fs::symlink(&at_the_limit, temp.path().join("music/limit.flac"))
                .expect("make a link at the limit");
            std::os::unix::fs::symlink(&over, temp.path().join("music/over.flac"))
                .expect("make a long link");
            root.open_file(&rel(&names("limit.flac")).expect("path"))
                .expect("the link at the limit is followed");
            assert_eq!(
                root.open_file(&rel(&names("over.flac")).expect("path"))
                    .map(|_| ()),
                Err(FsError::LinkTooLong {
                    len: over.len(),
                    max: MAX_LINK_TEXT
                })
            );
        }
        #[cfg(not(target_os = "linux"))]
        {
            let at_the_limit = format!("{}12345678.flac", "./".repeat(505));
            assert_eq!(at_the_limit.len(), 1023);
            std::fs::write(temp.path().join("music/12345678.flac"), b"fLaC")
                .expect("the link's target");
            std::os::unix::fs::symlink(&at_the_limit, temp.path().join("music/limit.flac"))
                .expect("make a link at the limit");
            root.open_file(&rel(&names("limit.flac")).expect("path"))
                .expect("the link at the limit is followed");
        }
    }

    /// The type is read again from the open handle: an entry the judgement
    /// took for a regular file is opened without blocking and refused. On
    /// Linux the entry is a FIFO, whose open would wait for a writer that
    /// never comes, so the open runs on a pool: a door that waited for one
    /// would fail this test instead of hanging it. Where rustix has no
    /// FIFO helper, the entry is a directory, which refuses through the
    /// same read of the open handle.
    ///
    /// Verifies: SEC-MED-035
    #[test]
    fn refuses_what_the_open_handle_shows_is_not_a_regular_file() {
        let (temp, root) = library("fs-unit-handle-kind");
        #[cfg(target_os = "linux")]
        rustix::fs::mkfifoat(
            rustix::fs::CWD,
            temp.path().join("music/pipe.flac"),
            Mode::from_raw_mode(0o644),
        )
        .expect("make a FIFO");
        #[cfg(not(target_os = "linux"))]
        std::fs::create_dir(temp.path().join("music/pipe.flac")).expect("make a directory");
        #[cfg(target_os = "linux")]
        let found = FileKind::Fifo;
        #[cfg(not(target_os = "linux"))]
        let found = FileKind::Dir;
        let mut judged = judge(&root, "pipe.flac");
        judged.kind = FileKind::File;
        let opened = Pool::new(1).run(Duration::from_secs(30), move || {
            root.own().file(&names("pipe.flac"), &judged).err()
        });
        assert_eq!(opened, Ok(Some(FsError::NotRegular { found })));
    }
}
