//! The data-root handle: the one way any module reads or writes a file in
//! the server's data directory.
//!
//! [`DataRoot::open`] opens the data directory once as a directory handle
//! and from then on resolves every [`DataPath`] beneath that handle
//! (`openat2` with `RESOLVE_BENEATH` through `cap-std`), so `..`, absolute
//! paths and symlinks that lead out of the root are refused even if a path
//! were built wrongly (SEC-HIS-016, SEC-MED-033, SEC-TM-043). Because it
//! works on the handle, renaming or replacing the directory's path after
//! opening does not redirect it.
//!
//! Opening also creates the layout and checks owners and modes the way
//! SEC-OPS-012 asks: the root and every layout directory must be
//! directories with mode 0700, and everything in `secrets/` must be a
//! regular file with mode 0600 or a directory with mode 0700, all owned by
//! the service account. A wrong mode on something the service account owns
//! is tightened and reported as a [`Repair`] for the audit log, once
//! inspecting it again shows the new mode; a filesystem that accepts the
//! change and keeps the old mode is refused. A temporary file that a
//! [`DataRoot::replace`] interrupted by a crash left in `secrets/` is
//! removed and reported the same way. Anything else is refused. Files are
//! created with mode 0600 and directories with mode 0700.
//!
//! The user log names its directories and segments at run time, one
//! directory for each stream and one segment for each month (ADR 3,
//! section 3), so it gets three operations no other store has:
//! [`DataRoot::log_streams`] and [`DataRoot::log_segments`] list what is
//! there, reading at most [`LIST_MAX`] entries of a directory, and
//! [`DataRoot::remove_log_stream`] removes one profile's directory. Each
//! takes typed values and builds the names itself, lists only what is
//! itself a directory or a file of such a name, and removes only what
//! [`DataPath::log_stream`] and [`DataPath::log_segment`] can name.
//!
//! None of the three follows a symbolic link. Each directory on the way,
//! `durable`, then `log`, then the stream's own, is opened by its one name
//! beneath the handle of the directory that holds it, in a single call
//! that succeeds only if the name is itself a directory (`O_DIRECTORY`
//! with `O_NOFOLLOW`). No look at a name comes before the open of it, so
//! nothing can be put in a directory's place between the two. What a
//! directory holds is then read, inspected and unlinked by name through
//! the handle that was opened, which does not follow a link either, and a
//! removal syncs that same handle of `durable/log`.
#![expect(
    clippy::disallowed_methods,
    reason = "the data-root handle is the workspace's filesystem door: it opens the root by path once and works beneath its handle (SEC-MED-033, SEC-HIS-016)"
)]

use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{self, Write};
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};

use cap_std::fs::{Dir, DirBuilder, DirBuilderExt, MetadataExt as _, OpenOptions, OpenOptionsExt};
#[cfg(not(target_os = "linux"))]
use rustix::fs::RawMode;
use rustix::fs::{AtFlags, FileType, Mode, OFlags, Stat};

use crate::host::{Holds, HostFacts, NetworkFs};
use crate::path::{self, DataDir, DataPath, LogMonth, LogStream, USER_LOG};

/// The mode of every directory in the data directory.
pub const DIR_MODE: u32 = 0o700;

/// The mode of every file the data root creates.
pub const FILE_MODE: u32 = 0o600;

/// The most entries a listing of one of the user log's directories reads.
///
/// `durable/log` holds one directory for each profile and one for the
/// household, and a stream's directory one segment for each month it was
/// written in: 4,096 entries are more profiles than a household's server
/// has, and the months of 341 years. A directory that holds more was not
/// filled by the server, and is refused rather than read to its end.
pub const LIST_MAX: usize = 4_096;

/// How the data directory itself is opened: a readable descriptor rather
/// than `cap-std`'s `O_PATH` one, so the root's own mode can be repaired
/// through it, and only if it is a directory.
const ROOT_FLAGS: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::CLOEXEC);

/// How a directory of the user log is opened beneath the handle of the
/// directory that holds it: only if the name is itself a directory, which
/// `O_DIRECTORY` with `O_NOFOLLOW` makes the one call decide, and readable,
/// so that the handle can be synced.
const DIR_FLAGS: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);

/// What to do with a wrong mode on something the service account owns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modes {
    /// Tighten it and report a [`Repair`], as SEC-OPS-012 asks at startup.
    Repair,
    /// Refuse to open, for a check that must change nothing.
    Refuse,
}

/// Whether a data directory on a network filesystem is accepted (ADM-079).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkFilesystems {
    /// Refuse it, which is the default.
    Refuse,
    /// Accept it, because the owner set the documented override. Owners and
    /// modes are still checked, so a share that cannot enforce them is
    /// refused anyway.
    Allow,
}

/// How [`DataRoot::open`] treats what it finds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    /// What to do with a wrong mode.
    pub modes: Modes,
    /// Whether a network filesystem is accepted.
    pub network: NetworkFilesystems,
}

impl Policy {
    /// Repair modes and refuse network filesystems.
    pub const DEFAULT: Self = Self {
        modes: Modes::Repair,
        network: NetworkFilesystems::Refuse,
    };
}

/// Something in the data directory that an error or a repair is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    /// The data directory itself.
    Root,
    /// One directory of the layout.
    Dir(DataDir),
    /// A path inside a layout directory.
    Path(DataPath),
    /// The temporary file that [`DataRoot::replace`] writes beside this
    /// path before renaming it into place.
    Replacement(DataPath),
}

/// What kind of filesystem object was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A regular file.
    File,
    /// A directory.
    Dir,
    /// A symbolic link.
    Symlink,
    /// A socket, FIFO or device.
    Other,
}

/// The operation that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// Examining the data directory for [`HostFacts`].
    Probe,
    /// Resolving the configured path of the data directory.
    Resolve,
    /// Opening the data directory as a handle.
    OpenRoot,
    /// Reading an object's type, owner and mode.
    Inspect,
    /// Tightening an object's mode.
    Repair,
    /// Listing a directory.
    List,
    /// Creating a directory.
    CreateDir,
    /// Creating a file.
    Create,
    /// Opening an existing file.
    Open,
    /// Writing and syncing a file's contents.
    Write,
    /// Renaming a finished file over the old one.
    Rename,
    /// Syncing the directory that holds a renamed file, or the user log's
    /// directory once a stream is gone from it.
    Sync,
    /// Removing a temporary file that an interrupted replace left behind,
    /// or a stream's directory and segments from the user log.
    Remove,
}

/// A change the data root made while opening, for the audit log
/// (SEC-OPS-012).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Repair {
    /// A mode tightened.
    Mode {
        /// What was tightened.
        item: Item,
        /// Its permission bits before.
        from: u32,
        /// Its permission bits after.
        to: u32,
    },
    /// A temporary file removed, which a replace interrupted by a crash
    /// left in `secrets/`. The file it would have replaced is untouched.
    Removed {
        /// The temporary file, an [`Item::Replacement`].
        item: Item,
    },
}

/// Why the data root refused an operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataRootError {
    /// The data directory is on a network filesystem and the override is
    /// not set (ADM-079).
    NetworkFilesystem(NetworkFs),
    /// An operating-system call failed.
    Io {
        /// What it was about.
        item: Item,
        /// What it was doing.
        op: Op,
        /// How it failed.
        kind: io::ErrorKind,
    },
    /// Another user owns it, so the server cannot fix it and refuses to
    /// start (SEC-OPS-012).
    NotOwned {
        /// What is owned by someone else.
        item: Item,
        /// Its owner's user ID.
        owner: u32,
        /// The service account's user ID.
        uid: u32,
    },
    /// It is not the kind of object that belongs there: a symbolic link, a
    /// file where a directory belongs, or a socket.
    WrongKind {
        /// What is wrong.
        item: Item,
        /// What was found.
        found: Kind,
    },
    /// Its mode is wrong and the policy says to refuse rather than repair.
    WrongMode {
        /// What has the wrong mode.
        item: Item,
        /// Its permission bits.
        mode: u32,
        /// The permission bits it must have.
        required: u32,
    },
    /// Its mode is wrong and tightening it did not take: the filesystem
    /// accepted the change and kept the old mode, as a share or a FUSE
    /// filesystem that ignores `chmod` does. The server cannot enforce the
    /// mode there, so it refuses (SEC-OPS-012, ADM-079).
    CannotRepair {
        /// What has the wrong mode.
        item: Item,
        /// Its permission bits after the attempt.
        mode: u32,
        /// The permission bits it must have.
        required: u32,
    },
    /// A temporary file that a replace interrupted by a crash left in
    /// `secrets/`, and the policy says to refuse rather than remove it.
    Leftover {
        /// The temporary file, an [`Item::Replacement`].
        item: Item,
    },
    /// An entry in `secrets/` whose name no [`DataPath`] can hold, so the
    /// server did not create it.
    ForeignEntry {
        /// The directory it is in.
        parent: Item,
        /// Its name.
        name: OsString,
    },
}

/// Why [`DataRoot::open`] refused, with the modes it had already tightened
/// before it found the problem, so the audit log still records them
/// (SEC-OPS-012).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    /// The problem.
    pub error: DataRootError,
    /// The repairs made before it, in order.
    pub repairs: Vec<Repair>,
}

/// What a listing of one of the user log's directories found.
///
/// A listing is whole or it is an error. An entry that cannot be inspected
/// fails it, because a listing without that entry would hide a stream or a
/// month from a caller that replays what it is given. The one entry a
/// listing leaves out is an entry removed while the listing was made: it is
/// in neither list, as it is no longer in the directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing<T> {
    /// What belongs there, in the order of the names: the streams whose
    /// directory is itself a directory, or the months whose segment is
    /// itself a regular file. The order of the names is the order of the
    /// values, so the list is sorted: the household's stream comes before
    /// every profile's, profiles come by ID, and months come oldest first.
    pub entries: Vec<T>,
    /// The names of everything else, in order: an entry whose name no
    /// typed constructor builds, and an entry of such a name that is not
    /// the kind of object that belongs there, such as a symbolic link.
    /// The temporary file a [`DataRoot::replace`] of a segment left when a
    /// crash interrupted it is among them.
    pub foreign: Vec<OsString>,
}

/// Why one of the user log's directories could not be listed or removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogDirError {
    /// The data root refused: the directory is missing, is not itself a
    /// directory, or an operating-system call on it failed.
    Root(DataRootError),
    /// The directory holds more than `max` entries, so it was not read to
    /// its end, and nothing in it was listed or removed.
    TooMany {
        /// The directory.
        item: Item,
        /// The most entries a listing reads, [`LIST_MAX`].
        max: usize,
    },
    /// The household's stream was named for removal. Only a profile's
    /// stream is ever removed (ADR 3, sections 3 and 8): what the household
    /// curated stays when an account is deleted. Nothing was touched.
    Household,
}

/// Maps an operating-system error about `item` during `op`.
pub(crate) fn io_error(item: Item, op: Op) -> impl FnOnce(io::Error) -> DataRootError {
    move |error| DataRootError::Io {
        item,
        op,
        kind: error.kind(),
    }
}

/// The open data directory.
#[derive(Debug)]
pub struct DataRoot {
    dir: Dir,
    path: PathBuf,
}

/// An opened data root and the repairs that opening it made.
#[derive(Debug)]
pub struct Opened {
    /// The handle.
    pub root: DataRoot,
    /// The modes opening it tightened, in the order it tightened them, for
    /// the audit log.
    pub repairs: Vec<Repair>,
}

/// An object's type, owner and permission bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Facts {
    kind: Kind,
    owner: u32,
    mode: u32,
}

impl Facts {
    fn of(stat: &Stat) -> Self {
        let kind = match FileType::from_raw_mode(stat.st_mode) {
            FileType::RegularFile => Kind::File,
            FileType::Directory => Kind::Dir,
            FileType::Symlink => Kind::Symlink,
            _ => Kind::Other,
        };
        Self {
            kind,
            owner: stat.st_uid,
            // `mode_t` is `u32` on Linux and narrower elsewhere; the field
            // is the wider one on every platform.
            #[cfg(target_os = "linux")]
            mode: stat.st_mode & 0o7777,
            #[cfg(not(target_os = "linux"))]
            mode: u32::from(stat.st_mode & 0o7777),
        }
    }
}

/// The `Mode` for stored permission bits.
///
/// `mode_t` is `u32` on Linux, so the bits are it already. Elsewhere they
/// are wider than `mode_t`, and a value that does not fit is refused, so
/// one can never be written truncated.
#[cfg(target_os = "linux")]
fn mode_for(bits: u32) -> Mode {
    Mode::from_raw_mode(bits)
}

/// The `Mode` for stored permission bits.
///
/// `mode_t` is `u32` on Linux, so the bits are it already. Elsewhere they
/// are wider than `mode_t`, and a value that does not fit is refused, so
/// one can never be written truncated.
#[cfg(not(target_os = "linux"))]
fn mode_for(bits: u32) -> io::Result<Mode> {
    RawMode::try_from(bits)
        .map(Mode::from_raw_mode)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "mode does not fit mode_t"))
}

/// Decides what to do about `item`, given what was found there.
///
/// Directories need mode 0700. Regular files need mode 0600 and are
/// acceptable only where `files` is true. Everything must be owned by
/// `uid`. Returns the mode to set, if the mode is to be repaired.
fn verdict(
    item: &Item,
    facts: Facts,
    files: bool,
    uid: u32,
    modes: Modes,
) -> Result<Option<u32>, DataRootError> {
    let required = match (facts.kind, files) {
        (Kind::Dir, _) => DIR_MODE,
        (Kind::File, true) => FILE_MODE,
        (found, _) => {
            return Err(DataRootError::WrongKind {
                item: item.clone(),
                found,
            });
        }
    };
    if facts.owner != uid {
        return Err(DataRootError::NotOwned {
            item: item.clone(),
            owner: facts.owner,
            uid,
        });
    }
    match (facts.mode == required, modes) {
        (true, _) => Ok(None),
        (false, Modes::Repair) => Ok(Some(required)),
        (false, Modes::Refuse) => Err(DataRootError::WrongMode {
            item: item.clone(),
            mode: facts.mode,
            required,
        }),
    }
}

/// Accepts a repair of `item` to the mode `required` only when `after`,
/// what inspecting it again found, shows that mode.
fn confirm(item: &Item, after: Facts, required: u32) -> Result<(), DataRootError> {
    if after.mode == required {
        Ok(())
    } else {
        Err(DataRootError::CannotRepair {
            item: item.clone(),
            mode: after.mode,
            required,
        })
    }
}

/// Decides what to do about `item`, a temporary file that a replace
/// interrupted by a crash left behind. A regular file the service account
/// owns is removed when the policy allows repairs; anything else is
/// refused, as it would be under any other name in `secrets/`.
fn leftover(item: &Item, facts: Facts, uid: u32, modes: Modes) -> Result<(), DataRootError> {
    if facts.kind != Kind::File {
        return Err(DataRootError::WrongKind {
            item: item.clone(),
            found: facts.kind,
        });
    }
    if facts.owner != uid {
        return Err(DataRootError::NotOwned {
            item: item.clone(),
            owner: facts.owner,
            uid,
        });
    }
    match modes {
        Modes::Repair => Ok(()),
        Modes::Refuse => Err(DataRootError::Leftover { item: item.clone() }),
    }
}

/// What every step of opening needs: the host, the policy for modes, and
/// the repairs made so far, which are kept even when a later step refuses.
struct Settler {
    host: HostFacts,
    modes: Modes,
    repairs: Vec<Repair>,
}

impl Settler {
    /// Inspects one object with `inspect`, decides with [`verdict`], and
    /// tightens its mode with `repair` when that is the decision. A repair
    /// counts only once inspecting again shows the new mode, because some
    /// filesystems report success and keep the old one; then it is
    /// recorded. Returns what kind of object it is.
    fn settle(
        &mut self,
        inspect: impl Fn() -> rustix::io::Result<Stat>,
        repair: impl FnOnce(Mode) -> rustix::io::Result<()>,
        item: &Item,
        files: bool,
    ) -> Result<Kind, DataRootError> {
        let look = || {
            inspect()
                .map(|stat| Facts::of(&stat))
                .map_err(io::Error::from)
                .map_err(io_error(item.clone(), Op::Inspect))
        };
        look()
            .and_then(|facts| {
                verdict(item, facts, files, self.host.uid, self.modes).map(|change| (facts, change))
            })
            .and_then(|(facts, change)| match change {
                None => Ok(facts.kind),
                Some(required) => {
                    // Linux `mode_for` returns the mode directly. Wrapping
                    // it in `Ok` here keeps one repair path; a helper that
                    // only returned `Ok` would not pass Clippy on Linux.
                    #[cfg(target_os = "linux")]
                    let mode: io::Result<Mode> = Ok(mode_for(required));
                    #[cfg(not(target_os = "linux"))]
                    let mode = mode_for(required);
                    mode.and_then(|mode| repair(mode).map_err(io::Error::from))
                        .map_err(io_error(item.clone(), Op::Repair))
                        .and_then(|()| look())
                        .and_then(|after| confirm(item, after, required))
                        .map(|()| {
                            self.repairs.push(Repair::Mode {
                                item: item.clone(),
                                from: facts.mode,
                                to: required,
                            });
                            facts.kind
                        })
                }
            })
    }

    /// Inspects `item`, a temporary file an interrupted replace left
    /// behind, decides with [`leftover`], and removes it with `remove` when
    /// that is the decision, recording the removal.
    fn clear(
        &mut self,
        inspect: impl FnOnce() -> rustix::io::Result<Stat>,
        remove: impl FnOnce() -> rustix::io::Result<()>,
        item: Item,
    ) -> Result<(), DataRootError> {
        inspect()
            .map_err(io::Error::from)
            .map_err(io_error(item.clone(), Op::Inspect))
            .and_then(|stat| leftover(&item, Facts::of(&stat), self.host.uid, self.modes))
            .and_then(|()| {
                remove()
                    .map_err(io::Error::from)
                    .map_err(io_error(item.clone(), Op::Remove))
            })
            .map(|()| self.repairs.push(Repair::Removed { item }))
    }

    /// Settles the data directory itself, then creates and settles each
    /// layout directory in turn, stopping at the first refusal.
    fn layout(&mut self, root: &Dir) -> Result<(), DataRootError> {
        self.settle(
            || rustix::fs::fstat(root),
            |mode| rustix::fs::fchmod(root, mode),
            &Item::Root,
            false,
        )
        .and_then(|_| {
            DataDir::ALL.into_iter().try_for_each(|dir| {
                // An entry that already exists is inspected next. Any other
                // reason the create fails shows up there as well, as the
                // error the inspection reports, so its result needs no
                // handling here.
                // `map` keeps the error type and never uses it, so the `Ok`
                // has to name one. Linux `mode_for` returns `Mode` alone.
                #[cfg(target_os = "linux")]
                let mode: io::Result<Mode> = Ok(mode_for(DIR_MODE));
                #[cfg(not(target_os = "linux"))]
                let mode = mode_for(DIR_MODE);
                let _ = mode.map(|mode| rustix::fs::mkdirat(root, dir.name(), mode));
                self.settle(
                    || rustix::fs::statat(root, dir.name(), AtFlags::SYMLINK_NOFOLLOW),
                    |mode| rustix::fs::chmodat(root, dir.name(), mode, AtFlags::empty()),
                    &Item::Dir(dir),
                    false,
                )
                .map(drop)
            })
        })
    }

    /// Settles every entry of `dir`, in name order, descending into
    /// directories. `parent` names `dir` itself.
    fn walk(&mut self, dir: &Dir, parent: &Item) -> Result<(), DataRootError> {
        dir.entries()
            .and_then(|entries| {
                entries
                    .map(|entry| entry.map(|entry| entry.file_name()))
                    .collect::<io::Result<Vec<OsString>>>()
            })
            .map_err(io_error(parent.clone(), Op::List))
            .and_then(|mut names| {
                names.sort();
                names
                    .iter()
                    .try_for_each(|name| self.visit(dir, parent, name))
            })
    }

    /// Settles the entry `name` of `dir`, and everything inside it when it
    /// is a directory. A temporary file an interrupted replace left behind
    /// is cleared instead.
    fn visit(&mut self, dir: &Dir, parent: &Item, name: &OsStr) -> Result<(), DataRootError> {
        let inspect = || rustix::fs::statat(dir, name, AtFlags::SYMLINK_NOFOLLOW);
        child(parent, name).and_then(|entry| match entry {
            Entry::Path(path) => {
                let item = Item::Path(path);
                self.settle(
                    inspect,
                    |mode| rustix::fs::chmodat(dir, name, mode, AtFlags::empty()),
                    &item,
                    true,
                )
                .and_then(|kind| match kind {
                    Kind::Dir => dir
                        .open_dir(name)
                        .map_err(io_error(item.clone(), Op::List))
                        .and_then(|inner| self.walk(&inner, &item)),
                    _ => Ok(()),
                })
            }
            Entry::Leftover(path) => self.clear(
                inspect,
                || rustix::fs::unlinkat(dir, name, AtFlags::empty()),
                Item::Replacement(path),
            ),
        })
    }
}

/// What an entry below `secrets/` is.
enum Entry {
    /// A path the server may have created.
    Path(DataPath),
    /// The temporary file of a replace of this path, which a crash
    /// interrupted.
    Leftover(DataPath),
}

/// What the entry `name` inside `parent` is; `parent` is `secrets/` or a
/// directory below it. An entry that is neither a path a [`DataPath`] can
/// name nor the temporary file of a replace of one was not created by the
/// server.
fn child(parent: &Item, name: &OsStr) -> Result<Entry, DataRootError> {
    let foreign = || DataRootError::ForeignEntry {
        parent: parent.clone(),
        name: name.to_os_string(),
    };
    let inside = |rel: &str| match parent {
        Item::Path(path) => path.join(rel),
        _ => DataPath::new(DataDir::Secrets, rel),
    };
    let rel = name.to_str().ok_or_else(foreign)?;
    inside(rel).map(Entry::Path).or_else(|_| {
        path::replaced_by(rel)
            .and_then(|target| inside(target).ok())
            .map(Entry::Leftover)
            .ok_or_else(foreign)
    })
}

impl DataRoot {
    /// Opens the data directory at `path`, creates the layout and checks
    /// owners and modes.
    ///
    /// # Errors
    ///
    /// Returns [`Refused`] when the directory is on a refused network
    /// filesystem, cannot be opened, holds something it must not, or has an
    /// owner or mode that cannot or may not be repaired.
    pub fn open(path: &Path, host: &HostFacts, policy: Policy) -> Result<Opened, Refused> {
        let mut settler = Settler {
            host: *host,
            modes: policy.modes,
            repairs: Vec::new(),
        };
        match Self::open_with(path, policy.network, &mut settler) {
            Ok(root) => Ok(Opened {
                root,
                repairs: settler.repairs,
            }),
            Err(error) => Err(Refused {
                error,
                repairs: settler.repairs,
            }),
        }
    }

    /// Does the work of [`DataRoot::open`], recording repairs in `settler`.
    fn open_with(
        path: &Path,
        network: NetworkFilesystems,
        settler: &mut Settler,
    ) -> Result<Self, DataRootError> {
        settler
            .host
            .filesystem
            .admits(Holds::Data(network))
            .map_err(DataRootError::NetworkFilesystem)?;
        // Resolving once gives SQLite a path with no symlinks in it, which
        // its no-follow open then insists on (see `sqlite_path`).
        let path = std::fs::canonicalize(path).map_err(io_error(Item::Root, Op::Resolve))?;
        let dir = rustix::fs::open(&path, ROOT_FLAGS, Mode::empty())
            .map(|fd| Dir::from_std_file(File::from(fd)))
            .map_err(io::Error::from)
            .map_err(io_error(Item::Root, Op::OpenRoot))?;
        let secrets = Item::Dir(DataDir::Secrets);
        settler
            .layout(&dir)
            .and_then(|()| {
                dir.open_dir(DataDir::Secrets.name())
                    .map_err(io_error(secrets.clone(), Op::List))
            })
            .and_then(|found| settler.walk(&found, &secrets))
            .map(|()| Self { dir, path })
    }

    /// Creates the file at `path` with mode 0600, failing if it exists.
    ///
    /// # Errors
    ///
    /// Returns [`DataRootError::Io`] with [`Op::Create`] when the file
    /// exists, its directory does not, or the path leads out of the root.
    pub fn create_new(&self, path: &DataPath) -> Result<File, DataRootError> {
        self.open_beneath(
            &path.beneath(),
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(FILE_MODE),
            Item::Path(path.clone()),
            Op::Create,
        )
    }

    /// Opens the file at `path` for appending, creating it with mode 0600
    /// if it does not exist.
    ///
    /// # Errors
    ///
    /// Returns [`DataRootError::Io`] with [`Op::Open`] when the directory
    /// does not exist or the path leads out of the root.
    pub fn append(&self, path: &DataPath) -> Result<File, DataRootError> {
        self.open_beneath(
            &path.beneath(),
            OpenOptions::new().append(true).create(true).mode(FILE_MODE),
            Item::Path(path.clone()),
            Op::Open,
        )
    }

    /// Opens the file at `path` for reading.
    ///
    /// # Errors
    ///
    /// Returns [`DataRootError::Io`] with [`Op::Open`] when the file does
    /// not exist or the path leads out of the root.
    pub fn open_read(&self, path: &DataPath) -> Result<File, DataRootError> {
        self.open_beneath(
            &path.beneath(),
            OpenOptions::new().read(true),
            Item::Path(path.clone()),
            Op::Open,
        )
    }

    /// Replaces the file at `path` with `bytes`, so that a crash leaves
    /// either the old contents or the new, never a mixture: the bytes go to
    /// a temporary file beside it, which is synced, renamed over it, and
    /// followed by a sync of the directory. A temporary file a crash left
    /// behind is removed by the next replace of the same path, and in
    /// `secrets/` already by the next [`DataRoot::open`].
    ///
    /// # Errors
    ///
    /// Returns [`DataRootError::Io`] naming the step that failed.
    pub fn replace(&self, path: &DataPath, bytes: &[u8]) -> Result<(), DataRootError> {
        let item = Item::Path(path.clone());
        let temp = path.temp_sibling();
        // Clear away a temporary file a crash left behind. If it cannot be
        // removed, the exclusive create below fails and reports why.
        let _ = self.dir.remove_file(&temp);
        self.open_beneath(
            &temp,
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(FILE_MODE),
            item.clone(),
            Op::Create,
        )
        .and_then(|mut file| {
            file.write_all(bytes)
                .and_then(|()| file.sync_all())
                .map_err(io_error(item.clone(), Op::Write))
        })
        .and_then(|()| {
            self.dir
                .rename(&temp, &self.dir, path.beneath())
                .map_err(io_error(item.clone(), Op::Rename))
        })
        .and_then(|()| {
            self.dir
                .open_with(path.parent(), OpenOptions::new().read(true))
                .and_then(|parent| parent.sync_all())
                .map_err(io_error(item, Op::Sync))
        })
    }

    /// Creates the directory at `path` with mode 0700.
    ///
    /// # Errors
    ///
    /// Returns [`DataRootError::Io`] with [`Op::CreateDir`] when it exists,
    /// its parent does not, or the path leads out of the root.
    pub fn create_dir(&self, path: &DataPath) -> Result<(), DataRootError> {
        self.dir
            .create_dir_with(path.beneath(), DirBuilder::new().mode(DIR_MODE))
            .map_err(io_error(Item::Path(path.clone()), Op::CreateDir))
    }

    /// The streams that have a directory in the user log, `durable/log`.
    ///
    /// # Errors
    ///
    /// Returns [`LogDirError::Root`] when `durable/log` does not exist, is
    /// not itself a directory or cannot be read, or when an entry named
    /// like a stream's directory cannot be inspected, and
    /// [`LogDirError::TooMany`] when it holds more than [`LIST_MAX`]
    /// entries.
    pub fn log_streams(&self) -> Result<Listing<LogStream>, LogDirError> {
        let item = Item::Path(USER_LOG);
        self.log_dir().map_err(LogDirError::Root).and_then(|log| {
            listing(&log, &item, Kind::Dir, LogStream::named, &|name| {
                rustix::fs::statat(&log, name, AtFlags::SYMLINK_NOFOLLOW)
            })
        })
    }

    /// The months `stream` has a segment for in the user log.
    ///
    /// # Errors
    ///
    /// Returns [`LogDirError::Root`] when the stream's directory does not
    /// exist, is not itself a directory or cannot be read, or when an
    /// entry named like a segment cannot be inspected, and
    /// [`LogDirError::TooMany`] when it holds more than [`LIST_MAX`]
    /// entries.
    pub fn log_segments(&self, stream: LogStream) -> Result<Listing<LogMonth>, LogDirError> {
        let item = Item::Path(DataPath::log_stream(stream));
        self.log_dir()
            .and_then(|log| real_dir(&log, &stream.name(), &item))
            .map_err(LogDirError::Root)
            .and_then(|dir| {
                listing(&dir, &item, Kind::File, LogMonth::named, &|name| {
                    rustix::fs::statat(&dir, name, AtFlags::SYMLINK_NOFOLLOW)
                })
            })
    }

    /// Removes a profile's `stream` from the user log: every segment in
    /// its directory, every temporary file a [`DataRoot::replace`] of a
    /// segment left beside one, then the directory itself, and syncs
    /// `durable/log` so that a crash does not bring the stream back.
    ///
    /// A stream that has no directory is already removed, and `durable/log`
    /// is synced all the same. An earlier call may have removed the
    /// directory and stopped before its sync, and the erasure job, which
    /// resumes from its ledger after a crash (ADR 3, section 8), takes `Ok`
    /// to mean that the stream stays gone.
    ///
    /// Nothing else is removed. An entry of any other name stays, and the
    /// directory with it, which the error reports. A symbolic link named
    /// like a segment is removed itself; what it leads to is not touched.
    /// An entry that cannot be removed does not keep the others: every
    /// segment and temporary file that can be removed is, one that is
    /// already gone counts as removed, and the error is the first failure.
    ///
    /// The household's stream is never removed. Erasure removes a profile's
    /// stream and nothing of the household's (ADR 3, sections 3 and 8).
    ///
    /// # Errors
    ///
    /// Returns [`LogDirError::Household`] for the household's stream,
    /// before anything is opened. Returns [`LogDirError::Root`] when
    /// `durable/log` does not exist, when the stream's name is taken by
    /// something that is not itself a directory, when an entry or the
    /// directory cannot be removed, or when `durable/log` cannot be synced,
    /// and [`LogDirError::TooMany`] when the directory holds more than
    /// [`LIST_MAX`] entries, in which case nothing is removed.
    pub fn remove_log_stream(&self, stream: LogStream) -> Result<(), LogDirError> {
        match stream {
            LogStream::Household => Err(LogDirError::Household),
            LogStream::Profile(_) => self.log_dir().map_err(LogDirError::Root).and_then(|log| {
                remove_stream(
                    &log,
                    &stream.name(),
                    &Item::Path(DataPath::log_stream(stream)),
                    &|dir, entry| rustix::fs::unlinkat(dir, entry, AtFlags::empty()),
                    &|| rustix::fs::fsync(&log).map_err(io::Error::from),
                )
            }),
        }
    }

    /// Opens `durable/log`: `durable` beneath the root, and `log` beneath
    /// that. Each must itself be a directory.
    fn log_dir(&self) -> Result<Dir, DataRootError> {
        real_dir(
            &self.dir,
            DataDir::Durable.name(),
            &Item::Dir(DataDir::Durable),
        )
        .and_then(|durable| real_dir(&durable, path::LOG, &Item::Path(USER_LOG)))
    }

    /// Opens `rel`, a path relative to the root, beneath the handle. Every
    /// file the data root opens goes through here, and the handle refuses a
    /// path that would leave the root even when `rel` was not built from a
    /// [`DataPath`].
    fn open_beneath(
        &self,
        rel: &Path,
        options: &OpenOptions,
        item: Item,
        op: Op,
    ) -> Result<File, DataRootError> {
        self.dir
            .open_with(rel, options)
            .map(cap_std::fs::File::into_std)
            .map_err(io_error(item, op))
    }

    /// The one sanctioned way to give SQLite a path: the resolved data
    /// directory joined with a path built from constants.
    pub(crate) fn sqlite_path(&self, path: &DataPath) -> PathBuf {
        self.path.join(path.beneath())
    }

    /// Whether the path [`DataRoot::sqlite_path`] gives for `path` names the
    /// file the handle holds there, in the directory the handle holds it
    /// in: both have the same device and inode whether they are reached by
    /// that path or beneath the handle. SQLite opens by path, so its
    /// opener asks this once the file is open and before it writes
    /// anything, and refuses a data directory whose path was moved or
    /// replaced after [`DataRoot::open`]. A link in place of the file is
    /// never followed on either side.
    ///
    /// The check and SQLite's open are separate steps, so it does not
    /// catch a path that was swapped for the open and swapped back before
    /// the check; only an open beneath the handle could, and SQLite has
    /// none. SQLite also opens `-wal` and `-shm` by path at the first
    /// statement, which is after the check, so a data directory swapped
    /// after the opener returns still gets those files created outside the
    /// handle.
    pub(crate) fn holds_sqlite_path(&self, path: &DataPath) -> bool {
        let file = path.beneath();
        let parent = path.parent();
        let beneath = |rel: &Path| {
            self.dir
                .symlink_metadata(rel)
                .map(|found| (found.dev(), found.ino()))
                .ok()
        };
        let by_path = |rel: &Path| {
            std::fs::symlink_metadata(self.path.join(rel))
                .map(|found| (found.dev(), found.ino()))
                .ok()
        };
        beneath(&parent)
            .zip(beneath(&file))
            .zip(by_path(&parent).zip(by_path(&file)))
            .is_some_and(|(held, named)| held == named)
    }
}

/// Opens the entry `name` of `parent`, which must itself be a directory: a
/// symbolic link to one is refused, never followed. `name` is one name, so
/// nothing but that entry is resolved.
///
/// The open is the check. One system call opens the name only if it is a
/// directory and not a link ([`DIR_FLAGS`]). A look at the name before the
/// open would leave a moment in which a link could take the directory's
/// place and be followed.
fn real_dir(parent: &Dir, name: &str, item: &Item) -> Result<Dir, DataRootError> {
    rustix::fs::openat(parent, name, DIR_FLAGS, Mode::empty())
        .map(|fd| Dir::from_std_file(File::from(fd)))
        .map_err(|refused| {
            refusal(
                refused,
                rustix::fs::statat(parent, name, AtFlags::SYMLINK_NOFOLLOW),
                item,
            )
        })
}

/// Why the open of `item` was refused with `refused`, given what a look at
/// its name without following a link found once the open had refused. The
/// look decides nothing: by then nothing is opened, whatever it finds.
///
/// Something that is not itself a directory is a
/// [`DataRootError::WrongKind`]. A directory after all is the open's own
/// failure, which a directory the service account may not read gives.
/// Nothing that can be looked at is the failure of the look, which a name
/// with nothing behind it gives.
fn refusal(
    refused: rustix::io::Errno,
    found: rustix::io::Result<Stat>,
    item: &Item,
) -> DataRootError {
    match found.map(|stat| Facts::of(&stat).kind) {
        Ok(Kind::Dir) => io_error(item.clone(), Op::List)(refused.into()),
        Ok(found) => DataRootError::WrongKind {
            item: item.clone(),
            found,
        },
        Err(unseen) => io_error(item.clone(), Op::Inspect)(unseen.into()),
    }
}

/// The names of the entries of `dir`, in order, when it holds at most
/// [`LIST_MAX`] of them. It reads one entry more than that and no further,
/// so a directory someone else filled cannot make the server hold a list
/// without end.
fn names(dir: &Dir, item: &Item) -> Result<Vec<OsString>, LogDirError> {
    dir.entries()
        .and_then(|entries| {
            entries
                .take(LIST_MAX.saturating_add(1))
                .map(|entry| entry.map(|entry| entry.file_name()))
                .collect::<io::Result<Vec<OsString>>>()
        })
        .map_err(io_error(item.clone(), Op::List))
        .map_err(LogDirError::Root)
        .and_then(|mut names| {
            if names.len() > LIST_MAX {
                Err(LogDirError::TooMany {
                    item: item.clone(),
                    max: LIST_MAX,
                })
            } else {
                names.sort();
                Ok(names)
            }
        })
}

/// Lists `dir`: the entries whose name `named` reads and that `inspect`
/// finds to be themselves a `wanted`, and the names of the rest. `inspect`
/// reads what the entry of a name is without following a link, so a
/// symbolic link is a link, whatever it leads to.
///
/// An entry `inspect` cannot read fails the listing, as [`Listing`] says,
/// unless it is gone, in which case it is in neither list. A name `named`
/// does not read is not inspected: it is foreign whatever it is.
///
/// [`DataRoot::log_streams`] and [`DataRoot::log_segments`] pass the system
/// call. A test passes a look that fails, as it passes [`Settler::clear`] a
/// removal that fails. The look is a trait object so that the system call
/// and a test's look go through the same compiled function.
fn listing<T>(
    dir: &Dir,
    item: &Item,
    wanted: Kind,
    named: fn(&str) -> Option<T>,
    inspect: &dyn Fn(&OsStr) -> rustix::io::Result<Stat>,
) -> Result<Listing<T>, LogDirError> {
    names(dir, item).and_then(|names| {
        let mut found = Listing {
            entries: Vec::new(),
            foreign: Vec::new(),
        };
        for name in names {
            let Some(entry) = name.to_str().and_then(named) else {
                found.foreign.push(name);
                continue;
            };
            match inspect(&name) {
                Ok(stat) if Facts::of(&stat).kind == wanted => found.entries.push(entry),
                Ok(_) => found.foreign.push(name),
                // Gone since the directory was read.
                Err(rustix::io::Errno::NOENT) => {}
                Err(errno) => {
                    return Err(LogDirError::Root(io_error(item.clone(), Op::List)(
                        errno.into(),
                    )));
                }
            }
        }
        Ok(found)
    })
}

/// Does the work of [`DataRoot::remove_log_stream`] in `log`, the open
/// `durable/log`, for the stream whose directory is called `name`.
///
/// `unlink` removes one entry of the stream's directory and `sync` makes
/// what was done in `log` durable. [`DataRoot::remove_log_stream`] passes
/// the system calls. A test passes a step that fails, as it passes
/// [`Settler::clear`] a removal that fails. Both are trait objects so that
/// the system calls and a test's steps go through the same compiled
/// function.
fn remove_stream(
    log: &Dir,
    name: &str,
    item: &Item,
    unlink: &dyn Fn(&Dir, &OsStr) -> rustix::io::Result<()>,
    sync: &dyn Fn() -> io::Result<()>,
) -> Result<(), LogDirError> {
    let gone = match real_dir(log, name, item) {
        // No directory: an earlier call removed it, and may have stopped
        // before its sync, so this call still syncs.
        Err(DataRootError::Io {
            op: Op::Inspect,
            kind: io::ErrorKind::NotFound,
            ..
        }) => Ok(()),
        found => found.map_err(LogDirError::Root).and_then(|dir| {
            names(&dir, item).and_then(|entries| {
                // A failure does not keep the entries after it: each one is
                // tried, and the first failure is the one reported. An
                // entry that is already gone is removed.
                let mut emptied: rustix::io::Result<()> = Ok(());
                for entry in entries.iter().filter(|entry| removable(entry)) {
                    emptied = emptied.and(match unlink(&dir, entry.as_os_str()) {
                        Err(rustix::io::Errno::NOENT) => Ok(()),
                        outcome => outcome,
                    });
                }
                emptied
                    .and_then(|()| rustix::fs::unlinkat(log, name, AtFlags::REMOVEDIR))
                    .map_err(io::Error::from)
                    .map_err(io_error(item.clone(), Op::Remove))
                    .map_err(LogDirError::Root)
            })
        }),
    };
    // The removal is durable once `durable/log` is synced.
    gone.and_then(|()| {
        sync()
            .map_err(io_error(Item::Path(USER_LOG), Op::Sync))
            .map_err(LogDirError::Root)
    })
}

/// Whether the removal of a stream may remove the entry called `name` from
/// the stream's directory: a segment, or the temporary file a replace of a
/// segment writes beside it.
fn removable(name: &OsStr) -> bool {
    name.to_str().is_some_and(|name| {
        LogMonth::named(name)
            .or_else(|| path::replaced_by(name).and_then(LogMonth::named))
            .is_some()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::Filesystem;
    use proptest::prelude::*;
    // Direct imports: Qodana does not resolve these macros through `prelude::*`.
    use proptest::{prop_oneof, proptest};
    use std::cell::Cell;
    use std::io::Read;

    const UID: u32 = 1000;

    /// Verifies: SEC-OPS-012
    #[test]
    fn mode_for_keeps_the_permission_bits() {
        for bits in [0o700_u32, 0o600] {
            #[cfg(target_os = "linux")]
            let mode = mode_for(bits);
            #[cfg(not(target_os = "linux"))]
            let mode = mode_for(bits).expect("permission bits fit mode_t");
            // `mode_t` is already `u32` on Linux. Elsewhere it is narrower.
            #[cfg(target_os = "linux")]
            assert_eq!(mode.as_raw_mode(), bits);
            #[cfg(not(target_os = "linux"))]
            assert_eq!(u32::from(mode.as_raw_mode()), bits);
        }
    }

    /// Verifies: SEC-OPS-012
    #[cfg(not(target_os = "linux"))]
    #[test]
    fn mode_for_refuses_bits_wider_than_mode_t() {
        let bits = u32::from(RawMode::MAX) + 1;
        let error = mode_for(bits).expect_err("wider than mode_t");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(error.to_string(), "mode does not fit mode_t");
    }

    fn facts(kind: Kind, owner: u32, mode: u32) -> Facts {
        Facts { kind, owner, mode }
    }

    fn key() -> Item {
        Item::Path(DataPath::constant(DataDir::Secrets, "root.key"))
    }

    /// Verifies: SEC-OPS-012
    #[test]
    fn leaves_correct_modes_alone() {
        for modes in [Modes::Repair, Modes::Refuse] {
            assert_eq!(
                verdict(&Item::Root, facts(Kind::Dir, UID, 0o700), false, UID, modes),
                Ok(None)
            );
            assert_eq!(
                verdict(&key(), facts(Kind::File, UID, 0o600), true, UID, modes),
                Ok(None)
            );
            assert_eq!(
                verdict(&key(), facts(Kind::Dir, UID, 0o700), true, UID, modes),
                Ok(None)
            );
        }
    }

    /// Verifies: SEC-OPS-012
    #[test]
    fn repairs_any_other_mode_on_what_the_service_account_owns() {
        assert_eq!(
            verdict(
                &key(),
                facts(Kind::File, UID, 0o644),
                true,
                UID,
                Modes::Repair
            ),
            Ok(Some(0o600))
        );
        assert_eq!(
            verdict(
                &Item::Dir(DataDir::Cache),
                facts(Kind::Dir, UID, 0o2755),
                false,
                UID,
                Modes::Repair
            ),
            Ok(Some(0o700))
        );
        assert_eq!(
            verdict(
                &key(),
                facts(Kind::File, UID, 0o400),
                true,
                UID,
                Modes::Repair
            ),
            Ok(Some(0o600))
        );
    }

    /// Verifies: SEC-OPS-012
    #[test]
    fn refuses_a_wrong_mode_when_the_policy_says_so() {
        assert_eq!(
            verdict(
                &key(),
                facts(Kind::File, UID, 0o644),
                true,
                UID,
                Modes::Refuse
            ),
            Err(DataRootError::WrongMode {
                item: key(),
                mode: 0o644,
                required: 0o600
            })
        );
        assert_eq!(
            verdict(
                &Item::Root,
                facts(Kind::Dir, UID, 0o755),
                false,
                UID,
                Modes::Refuse
            ),
            Err(DataRootError::WrongMode {
                item: Item::Root,
                mode: 0o755,
                required: 0o700
            })
        );
    }

    /// Verifies: SEC-OPS-012
    #[test]
    fn refuses_what_another_user_owns_whatever_its_mode() {
        for mode in [0o600, 0o644] {
            assert_eq!(
                verdict(&key(), facts(Kind::File, 0, mode), true, UID, Modes::Repair),
                Err(DataRootError::NotOwned {
                    item: key(),
                    owner: 0,
                    uid: UID
                })
            );
        }
    }

    /// Verifies: SEC-HIS-016, SEC-OPS-012
    #[test]
    fn refuses_links_files_and_devices_where_they_do_not_belong() {
        let cases = [
            (Item::Root, Kind::File, false),
            (Item::Dir(DataDir::Secrets), Kind::Symlink, false),
            (key(), Kind::Symlink, true),
            (key(), Kind::Other, true),
        ];
        for (item, found, files) in cases {
            assert_eq!(
                verdict(&item, facts(found, UID, 0o600), files, UID, Modes::Repair),
                Err(DataRootError::WrongKind {
                    item: item.clone(),
                    found
                })
            );
        }
    }

    fn temp() -> Item {
        Item::Replacement(DataPath::constant(DataDir::Secrets, "keys.json"))
    }

    /// Verifies: SEC-OPS-012
    #[test]
    fn removes_a_leftover_temporary_file_only_when_repairs_are_allowed() {
        let file = facts(Kind::File, UID, 0o644);
        assert_eq!(leftover(&temp(), file, UID, Modes::Repair), Ok(()));
        assert_eq!(
            leftover(&temp(), file, UID, Modes::Refuse),
            Err(DataRootError::Leftover { item: temp() })
        );
    }

    /// Verifies: SEC-OPS-012
    #[test]
    fn refuses_a_leftover_that_is_not_a_file_the_service_account_owns() {
        assert_eq!(
            leftover(&temp(), facts(Kind::File, 0, 0o600), UID, Modes::Repair),
            Err(DataRootError::NotOwned {
                item: temp(),
                owner: 0,
                uid: UID
            })
        );
        for found in [Kind::Dir, Kind::Symlink, Kind::Other] {
            assert_eq!(
                leftover(&temp(), facts(found, UID, 0o600), UID, Modes::Repair),
                Err(DataRootError::WrongKind {
                    item: temp(),
                    found
                })
            );
        }
    }

    #[test]
    fn reports_a_leftover_it_cannot_remove() {
        let scratch = Scratch::new("stuck-leftover");
        let secret = scratch.0.join("outside/secret");
        let mut settler = settler(Modes::Repair);
        assert_eq!(
            settler.clear(
                || rustix::fs::stat(&secret),
                || Err(rustix::io::Errno::ACCESS),
                temp()
            ),
            Err(DataRootError::Io {
                item: temp(),
                op: Op::Remove,
                kind: io::ErrorKind::PermissionDenied
            })
        );
        assert_eq!(settler.repairs, [earlier()]);
    }

    /// A directory under the system's temporary directory, removed on drop.
    /// The integration tests have a fuller helper; these unit tests need
    /// only this.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("gunmetal-fs-unit-{}-{tag}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(path.join("root")).expect("scratch directory");
            std::fs::create_dir_all(path.join("outside")).expect("scratch directory");
            std::fs::write(path.join("outside/secret"), b"outside").expect("outside file");
            Self(path)
        }

        fn root(&self) -> DataRoot {
            let host = HostFacts {
                uid: rustix::process::geteuid().as_raw(),
                filesystem: Filesystem::Local,
            };
            DataRoot::open(&self.0.join("root"), &host, Policy::DEFAULT)
                .expect("data root opens")
                .root
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A settler for this process's user that has already made one repair.
    fn settler(modes: Modes) -> Settler {
        Settler {
            host: HostFacts {
                uid: rustix::process::geteuid().as_raw(),
                filesystem: Filesystem::Local,
            },
            modes,
            repairs: vec![earlier()],
        }
    }

    fn earlier() -> Repair {
        Repair::Mode {
            item: Item::Root,
            from: 0o755,
            to: 0o700,
        }
    }

    /// Verifies: SEC-OPS-012
    #[test]
    fn refuses_when_a_repair_reports_success_but_the_mode_did_not_change() {
        // A share or FUSE filesystem that accepts chmod and ignores it.
        let scratch = Scratch::new("ignored-chmod");
        let secret = scratch.0.join("outside/secret");
        std::fs::set_permissions(&secret, std::os::unix::fs::PermissionsExt::from_mode(0o644))
            .expect("loosen the file");
        let mut settler = settler(Modes::Repair);
        assert_eq!(
            settler.settle(|| rustix::fs::stat(&secret), |_| Ok(()), &key(), true),
            Err(DataRootError::CannotRepair {
                item: key(),
                mode: 0o644,
                required: 0o600
            })
        );
        assert_eq!(settler.repairs, [earlier()]);
    }

    #[test]
    fn reports_an_object_that_vanishes_while_it_is_repaired() {
        let scratch = Scratch::new("vanishing");
        let secret = scratch.0.join("outside/secret");
        std::fs::set_permissions(&secret, std::os::unix::fs::PermissionsExt::from_mode(0o644))
            .expect("loosen the file");
        let mut settler = settler(Modes::Repair);
        let remove = |_| {
            std::fs::remove_file(&secret).expect("remove the file");
            Ok(())
        };
        assert_eq!(
            settler.settle(|| rustix::fs::stat(&secret), remove, &key(), true),
            Err(DataRootError::Io {
                item: key(),
                op: Op::Inspect,
                kind: io::ErrorKind::NotFound
            })
        );
        assert_eq!(settler.repairs, [earlier()]);
    }

    fn read_options() -> OpenOptions {
        let mut options = OpenOptions::new();
        options.read(true);
        options
    }

    fn open_raw(root: &DataRoot, rel: &str) -> Result<File, DataRootError> {
        root.open_beneath(Path::new(rel), &read_options(), Item::Root, Op::Open)
    }

    fn denied() -> DataRootError {
        DataRootError::Io {
            item: Item::Root,
            op: Op::Open,
            kind: io::ErrorKind::PermissionDenied,
        }
    }

    /// Verifies: SEC-HIS-016, SEC-MED-033, SEC-TM-043
    #[test]
    fn the_handle_refuses_parent_and_absolute_paths_that_no_data_path_can_hold() {
        let scratch = Scratch::new("raw");
        let root = scratch.root();
        let outside = scratch.0.join("outside/secret");
        let outside = outside.to_str().expect("UTF-8 temporary directory");
        for rel in ["../outside/secret", "secrets/../../outside/secret", outside] {
            assert_eq!(open_raw(&root, rel).map(|_| ()), Err(denied()), "{rel}");
        }
    }

    /// Verifies: SEC-HIS-016, SEC-MED-033, SEC-TM-043
    #[test]
    fn the_handle_refuses_a_symlink_that_leads_out_of_the_root() {
        let scratch = Scratch::new("link");
        let root = scratch.root();
        std::os::unix::fs::symlink(
            scratch.0.join("outside"),
            scratch.0.join("root/durable/escape"),
        )
        .expect("symlink");
        assert_eq!(
            open_raw(&root, "durable/escape/secret").map(|_| ()),
            Err(denied())
        );
    }

    const ALICE: LogStream = LogStream::Profile([0xA1; 16]);

    /// The name of Alice's directory in the user log, written out.
    const ALICE_DIR: &str = "p-a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1";

    /// Alice's directory, as an error names it.
    fn alice() -> Item {
        Item::Path(DataPath::log_stream(ALICE))
    }

    fn month(year: u16, number: u8) -> LogMonth {
        LogMonth::new(year, number).expect("a month of the calendar")
    }

    /// Where the user log of `scratch` is on disk.
    fn log_path(scratch: &Scratch) -> PathBuf {
        scratch.0.join("root/durable/log")
    }

    /// Opens the data root of `scratch`, plants Alice's stream in its user
    /// log with an empty file for each of `files`, and returns the handle
    /// of `durable/log`.
    fn log_with(scratch: &Scratch, files: &[&str]) -> Dir {
        let root = scratch.root();
        let stream = log_path(scratch).join(ALICE_DIR);
        std::fs::create_dir_all(&stream).expect("the stream's directory");
        for file in files {
            std::fs::write(stream.join(file), b"").expect("a file in the stream's directory");
        }
        root.log_dir().expect("the user log opens")
    }

    /// The names in the directory at `path`, sorted.
    fn names_at(path: &Path) -> Vec<String> {
        let mut found: Vec<String> = std::fs::read_dir(path)
            .expect("list the directory")
            .map(|entry| {
                entry
                    .expect("a directory entry")
                    .file_name()
                    .into_string()
                    .expect("a UTF-8 name")
            })
            .collect();
        found.sort();
        found
    }

    /// Removes the entry `name` of `dir`, as the data root does.
    fn unlink(dir: &Dir, name: &OsStr) -> rustix::io::Result<()> {
        rustix::fs::unlinkat(dir, name, AtFlags::empty())
    }

    /// A sync that succeeds and counts itself in `syncs`.
    fn counted(syncs: &Cell<u32>) -> impl Fn() -> io::Result<()> {
        move || {
            syncs.set(syncs.get() + 1);
            Ok(())
        }
    }

    /// A sync that fails for want of space.
    fn full() -> io::Result<()> {
        Err(io::Error::from(io::ErrorKind::StorageFull))
    }

    /// What a removal reports when its sync fails for want of space.
    fn not_synced() -> LogDirError {
        LogDirError::Root(DataRootError::Io {
            item: Item::Path(USER_LOG),
            op: Op::Sync,
            kind: io::ErrorKind::StorageFull,
        })
    }

    #[test]
    fn says_why_a_directory_was_not_opened() {
        let scratch = Scratch::new("refused-open");
        let directory = scratch.0.join("outside");
        let file = scratch.0.join("outside/secret");
        let missing = scratch.0.join("outside/nothing");
        // A directory after all: the open's own failure.
        assert_eq!(
            refusal(
                rustix::io::Errno::ACCESS,
                rustix::fs::stat(&directory),
                &alice()
            ),
            DataRootError::Io {
                item: alice(),
                op: Op::List,
                kind: io::ErrorKind::PermissionDenied,
            }
        );
        // Something that is not a directory: what it is.
        assert_eq!(
            refusal(rustix::io::Errno::NOTDIR, rustix::fs::stat(&file), &alice()),
            DataRootError::WrongKind {
                item: alice(),
                found: Kind::File,
            }
        );
        // Nothing: the failure of the look.
        assert_eq!(
            refusal(
                rustix::io::Errno::NOENT,
                rustix::fs::stat(&missing),
                &alice()
            ),
            DataRootError::Io {
                item: alice(),
                op: Op::Inspect,
                kind: io::ErrorKind::NotFound,
            }
        );
    }

    #[test]
    fn a_retried_removal_syncs_the_user_log_when_the_directory_is_already_gone() {
        let scratch = Scratch::new("retried-removal");
        let log = log_with(&scratch, &["2026-10.seg"]);
        // The first call removes the directory, and its sync fails.
        assert_eq!(
            remove_stream(&log, ALICE_DIR, &alice(), &unlink, &full),
            Err(not_synced())
        );
        assert_eq!(names_at(&log_path(&scratch)), Vec::<String>::new());
        // The retry finds no directory. It cannot know that the first
        // call's removal reached the disk, so it syncs before it says so.
        let syncs = Cell::new(0);
        assert_eq!(
            remove_stream(&log, ALICE_DIR, &alice(), &unlink, &counted(&syncs)),
            Ok(())
        );
        assert_eq!(syncs.get(), 1);
        // A retry whose sync fails does not report the stream removed.
        assert_eq!(
            remove_stream(&log, ALICE_DIR, &alice(), &unlink, &full),
            Err(not_synced())
        );
    }

    #[test]
    fn removes_every_entry_it_can_and_reports_the_first_it_could_not() {
        let scratch = Scratch::new("stuck-entries");
        let log = log_with(
            &scratch,
            &["2026-07.seg", "2026-08.seg", "2026-09.seg", "2026-10.seg"],
        );
        // August's segment cannot be removed, and September's cannot for
        // another reason.
        let stuck = |dir: &Dir, name: &OsStr| match name.to_str() {
            Some("2026-08.seg") => Err(rustix::io::Errno::ACCESS),
            Some("2026-09.seg") => Err(rustix::io::Errno::BUSY),
            _ => unlink(dir, name),
        };
        let syncs = Cell::new(0);
        assert_eq!(
            remove_stream(&log, ALICE_DIR, &alice(), &stuck, &counted(&syncs)),
            Err(LogDirError::Root(DataRootError::Io {
                item: alice(),
                op: Op::Remove,
                kind: io::ErrorKind::PermissionDenied,
            }))
        );
        assert_eq!(
            names_at(&log_path(&scratch).join(ALICE_DIR)),
            ["2026-08.seg", "2026-09.seg"]
        );
        // The stream is still there, so there is no removal to sync.
        assert_eq!(syncs.get(), 0);
    }

    #[test]
    fn counts_an_entry_that_is_already_gone_as_removed() {
        let scratch = Scratch::new("vanished-segment");
        let log = log_with(&scratch, &["2026-09.seg", "2026-10.seg"]);
        let stream = log_path(&scratch).join(ALICE_DIR);
        // Something else removes October's segment once the directory has
        // been read, before the removal reaches it.
        let raced = |dir: &Dir, name: &OsStr| {
            if name == "2026-10.seg" {
                std::fs::remove_file(stream.join(name)).expect("remove the segment");
            }
            unlink(dir, name)
        };
        let syncs = Cell::new(0);
        assert_eq!(
            remove_stream(&log, ALICE_DIR, &alice(), &raced, &counted(&syncs)),
            Ok(())
        );
        assert_eq!(names_at(&log_path(&scratch)), Vec::<String>::new());
        assert_eq!(syncs.get(), 1);
    }

    #[test]
    fn fails_a_listing_when_it_cannot_tell_what_an_entry_is() {
        let scratch = Scratch::new("unread-entry");
        let log = log_with(&scratch, &["2026-08.seg", "2026-09.seg", "notes.txt"]);
        let dir = real_dir(&log, ALICE_DIR, &alice()).expect("the stream's directory opens");
        // The look at September's segment fails.
        let look = |name: &OsStr| {
            if name == "2026-09.seg" {
                Err(rustix::io::Errno::ACCESS)
            } else {
                rustix::fs::statat(&dir, name, AtFlags::SYMLINK_NOFOLLOW)
            }
        };
        assert_eq!(
            listing(&dir, &alice(), Kind::File, LogMonth::named, &look),
            Err(LogDirError::Root(DataRootError::Io {
                item: alice(),
                op: Op::List,
                kind: io::ErrorKind::PermissionDenied,
            }))
        );
    }

    #[test]
    fn leaves_an_entry_that_is_gone_when_it_is_looked_at_out_of_a_listing() {
        let scratch = Scratch::new("vanished-entry");
        let log = log_with(
            &scratch,
            &["2026-08.seg", "2026-09.seg", "2026-10.seg", "notes.txt"],
        );
        let stream = log_path(&scratch).join(ALICE_DIR);
        // A directory named like a segment is not a segment.
        std::fs::create_dir(stream.join("2026-11.seg")).expect("a directory");
        let dir = real_dir(&log, ALICE_DIR, &alice()).expect("the stream's directory opens");
        // September's segment is removed once the directory has been read,
        // before the listing looks at it.
        let look = |name: &OsStr| {
            if name == "2026-09.seg" {
                std::fs::remove_file(stream.join(name)).expect("remove the segment");
            }
            rustix::fs::statat(&dir, name, AtFlags::SYMLINK_NOFOLLOW)
        };
        assert_eq!(
            listing(&dir, &alice(), Kind::File, LogMonth::named, &look),
            Ok(Listing {
                entries: vec![month(2026, 8), month(2026, 10)],
                foreign: vec![OsString::from("2026-11.seg"), OsString::from("notes.txt")],
            })
        );
    }

    /// Pieces of relative paths aimed at the file outside the root.
    fn hostile_piece() -> impl Strategy<Value = &'static str> {
        prop_oneof![
            Just(".."),
            Just("."),
            Just(""),
            Just("outside"),
            Just("secret"),
            Just("secrets"),
            Just("durable"),
            Just("escape"),
            Just("root"),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        /// Verifies: SEC-HIS-016, SEC-TM-043
        #[test]
        fn the_handle_never_opens_the_file_outside(
            pieces in proptest::collection::vec(hostile_piece(), 1..7),
            absolute in any::<bool>(),
        ) {
            let scratch = Scratch::new("property");
            let root = scratch.root();
            std::os::unix::fs::symlink(
                scratch.0.join("outside"),
                scratch.0.join("root/durable/escape"),
            )
            .expect("symlink");
            let base = if absolute { scratch.0.to_str().expect("UTF-8") } else { "" };
            let rel = format!("{base}/{}", pieces.join("/"));
            let rel = rel.trim_start_matches(if absolute { "" } else { "/" });
            let mut contents = Vec::new();
            if let Ok(mut file) = open_raw(&root, rel) {
                let _ = file.read_to_end(&mut contents);
            }
            prop_assert_ne!(contents, b"outside".to_vec(), "{}", rel);
        }
    }
}
