//! Opening one file beneath a library root, read-only.
//!
//! [`Root::open_file`] follows the path's links by the root's policy, then
//! opens what they lead to beneath the handle with `O_NONBLOCK`, `O_NOCTTY`
//! and `O_CLOEXEC`, so that a FIFO cannot block the open and a terminal
//! cannot become the server's controlling terminal. The type is then read
//! from the open handle, never from the path, and anything that is not a
//! regular file is refused (SEC-MED-035).
//!
//! A [`MediaFile`] can only be read. There is no way to ask for a handle
//! that can write: the open options are fixed here, and the root's
//! directory handle is private (SEC-MED-038, SEC-TM-042, SEC-OPS-054).
//!
//! [`Root::open_verified`] is the check before bytes are served: the file
//! at the recorded path must still be the one the index recorded
//! (SEC-MED-036, SEC-API-018).

use std::fs::File;
use std::io;
use std::os::fd::{AsFd, BorrowedFd};
use std::os::unix::fs::FileExt as _;

use cap_std::fs::{
    FileType, FileTypeExt as _, Metadata, MetadataExt as _, OpenOptions, OpenOptionsExt as _,
};
use gunmetal_core::path::RelPath;
use rustix::fs::OFlags;
use rustix::io::Errno;

use crate::root::{Base, FsError, Op, Root, io_error, os_path};

/// The flags every file is opened with, besides read-only: a FIFO must not
/// block the open, a terminal must not become the controlling terminal, the
/// descriptor must not leak into a program the server starts, and the path
/// the links were followed to must not have become a link since.
const FLAGS: i32 = i32::from_ne_bytes(
    OFlags::NONBLOCK
        .union(OFlags::NOCTTY)
        .union(OFlags::CLOEXEC)
        .union(OFlags::NOFOLLOW)
        .bits()
        .to_ne_bytes(),
);

/// What kind of filesystem object something is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    /// A regular file.
    File,
    /// A directory.
    Dir,
    /// A symbolic link.
    Symlink,
    /// A FIFO, also called a named pipe.
    Fifo,
    /// A Unix socket.
    Socket,
    /// A device, or anything else.
    Other,
}

impl FileKind {
    /// Reads the kind from what the operating system reported.
    fn of(kind: FileType) -> Self {
        if kind.is_file() {
            Self::File
        } else if kind.is_dir() {
            Self::Dir
        } else if kind.is_symlink() {
            Self::Symlink
        } else if kind.is_fifo() {
            Self::Fifo
        } else if kind.is_socket() {
            Self::Socket
        } else {
            Self::Other
        }
    }
}

/// When a file was last modified, as the filesystem records it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Modified {
    /// Whole seconds since the Unix epoch.
    pub seconds: i64,
    /// The nanoseconds past that second.
    pub nanoseconds: i64,
}

/// What the index records about a file so that it can tell, before serving
/// bytes, that the file at a path is still the one it scanned
/// (SEC-MED-036).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Identity {
    /// The device the file is on.
    pub device: u64,
    /// The file's inode number.
    pub inode: u64,
    /// The file's size in bytes.
    pub size: u64,
    /// When the file was last modified.
    pub modified: Modified,
}

/// An object's kind, identity and number of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Facts {
    /// What kind of object it is.
    pub(crate) kind: FileKind,
    /// Its identity.
    pub(crate) identity: Identity,
    /// How many hard links name it.
    pub(crate) links: u64,
}

impl Facts {
    /// Reads the facts from what the operating system reported.
    pub(crate) fn of(metadata: &Metadata) -> Self {
        Self {
            kind: FileKind::of(metadata.file_type()),
            identity: Identity {
                device: metadata.dev(),
                inode: metadata.ino(),
                size: metadata.len(),
                modified: Modified {
                    seconds: metadata.mtime(),
                    nanoseconds: metadata.mtime_nsec(),
                },
            },
            links: metadata.nlink(),
        }
    }
}

/// A regular file beneath a library root, open for reading and nothing
/// else.
#[derive(Debug)]
pub struct MediaFile {
    file: File,
    identity: Identity,
    links: u64,
}

impl MediaFile {
    /// The file's identity, read from the open handle.
    #[must_use]
    pub const fn identity(&self) -> Identity {
        self.identity
    }

    /// How many hard links name the file. More than one means it has a name
    /// somewhere else, which may be outside the library.
    #[must_use]
    pub const fn links(&self) -> u64 {
        self.links
    }

    /// Reads up to `buffer.len()` bytes starting `offset` bytes into the
    /// file, and returns how many were read: fewer than asked at the end of
    /// the file, and none past it.
    ///
    /// # Errors
    ///
    /// Returns [`FsError::Io`] with [`Op::Read`] when the read fails.
    pub fn read_at(&self, buffer: &mut [u8], offset: u64) -> Result<usize, FsError> {
        self.file
            .read_at(buffer, offset)
            .map_err(io_error(Op::Read))
    }

    /// Fills `buffer` with the bytes starting `offset` bytes into the file.
    ///
    /// # Errors
    ///
    /// Returns [`FsError::Io`] with [`Op::Read`] when the read fails, with
    /// [`io::ErrorKind::UnexpectedEof`] when the file ends first.
    pub fn read_exact_at(&self, buffer: &mut [u8], offset: u64) -> Result<(), FsError> {
        self.file
            .read_exact_at(buffer, offset)
            .map_err(io_error(Op::Read))
    }
}

impl AsFd for MediaFile {
    /// The read-only descriptor, to hand to a worker process.
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.file.as_fd()
    }
}

/// The only options a file is ever opened with: reading, and [`FLAGS`].
fn read_only() -> OpenOptions {
    OpenOptions::new().read(true).custom_flags(FLAGS).clone()
}

/// Maps the failure of an open of something that looked like `seen`. A
/// socket cannot be opened at all, so that failure is reported as what it
/// is: not a regular file.
fn refused(seen: FileKind) -> impl FnOnce(io::Error) -> FsError {
    move |error| {
        if error.raw_os_error() == Some(Errno::NXIO.raw_os_error()) {
            FsError::NotRegular { found: seen }
        } else {
            FsError::Io {
                op: Op::Open,
                kind: error.kind(),
            }
        }
    }
}

impl Base {
    /// Opens the file at `names`, which looked like `seen` before it was
    /// opened, and refuses it unless the open handle is a regular file.
    pub(crate) fn file(&self, names: &[Vec<u8>], seen: FileKind) -> Result<MediaFile, FsError> {
        self.dir
            .open_with(os_path(names), &read_only())
            .and_then(|file| {
                file.metadata()
                    .map(|metadata| (file.into_std(), Facts::of(&metadata)))
            })
            .map_err(refused(seen))
            .and_then(|(file, facts)| {
                if facts.kind == FileKind::File {
                    Ok(MediaFile {
                        file,
                        identity: facts.identity,
                        links: facts.links,
                    })
                } else {
                    Err(FsError::NotRegular { found: facts.kind })
                }
            })
    }
}

impl Root {
    /// Opens the regular file at `rel` for reading.
    ///
    /// # Errors
    ///
    /// Returns [`FsError::Link`] or [`FsError::TooManyLinks`] when a
    /// symbolic link on the path may not be followed,
    /// [`FsError::NotRegular`] when the path leads to anything but a regular
    /// file, and [`FsError::Io`] when the file does not exist or may not be
    /// read.
    pub fn open_file(&self, rel: &RelPath) -> Result<MediaFile, FsError> {
        self.locate(self.own(), &[], rel.components())
            .and_then(|found| found.base.file(&found.names, found.facts.kind))
    }

    /// Opens the regular file at `rel` for serving, and refuses it unless
    /// it is still the file the index recorded as `expected`: the same
    /// device and inode, size and modification time (SEC-MED-036).
    ///
    /// # Errors
    ///
    /// Returns [`FsError::Changed`] when the file is not the one recorded,
    /// and otherwise as [`Root::open_file`].
    pub fn open_verified(&self, rel: &RelPath, expected: &Identity) -> Result<MediaFile, FsError> {
        self.open_file(rel).and_then(|file| {
            if file.identity == *expected {
                Ok(file)
            } else {
                Err(FsError::Changed {
                    expected: *expected,
                    found: file.identity,
                })
            }
        })
    }
}

/// What code outside this crate cannot do with a file beneath a library
/// root. These are documentation tests because they are about what
/// compiles.
#[cfg(doctest)]
mod read_only {
    /// Verifies: SEC-MED-038, SEC-TM-042, SEC-OPS-054
    ///
    /// Opening takes a root and a path and nothing else, so there are no
    /// options with which to ask for a handle that can write. What comes
    /// back is not a writer, and does not turn into a `std::fs::File` or an
    /// owned descriptor, whose methods can change a file's length, times
    /// and mode.
    ///
    /// The probe below has its inherent constant only for a type that has
    /// the ability asked about, and the trait's constant for any other.
    /// The first four assertions show that it does see each ability.
    /// Giving `MediaFile` one of them, or `open_file` another argument,
    /// makes this example fail.
    ///
    /// ```
    /// use std::fs::File;
    /// use std::io::Write;
    /// use std::marker::PhantomData;
    /// use std::os::fd::OwnedFd;
    ///
    /// use gunmetal_core::path::RelPath;
    /// use gunmetal_fs::open::MediaFile;
    /// use gunmetal_fs::root::{FsError, Root};
    ///
    /// let _open: fn(&Root, &RelPath) -> Result<MediaFile, FsError> = Root::open_file;
    ///
    /// struct Probe<T>(PhantomData<T>);
    ///
    /// trait Lacks {
    ///     const WRITES: bool = false;
    ///     const BECOMES_A_FILE: bool = false;
    ///     const BECOMES_A_DESCRIPTOR: bool = false;
    /// }
    /// impl<T> Lacks for Probe<T> {}
    ///
    /// impl<T: Write> Probe<T> {
    ///     const WRITES: bool = true;
    /// }
    /// impl<T: Into<File>> Probe<T> {
    ///     const BECOMES_A_FILE: bool = true;
    /// }
    /// impl<T: Into<OwnedFd>> Probe<T> {
    ///     const BECOMES_A_DESCRIPTOR: bool = true;
    /// }
    ///
    /// assert!(<Probe<File>>::WRITES);
    /// assert!(<Probe<&File>>::WRITES);
    /// assert!(<Probe<File>>::BECOMES_A_FILE);
    /// assert!(<Probe<File>>::BECOMES_A_DESCRIPTOR);
    ///
    /// assert!(!<Probe<MediaFile>>::WRITES);
    /// assert!(!<Probe<&MediaFile>>::WRITES);
    /// assert!(!<Probe<MediaFile>>::BECOMES_A_FILE);
    /// assert!(!<Probe<MediaFile>>::BECOMES_A_DESCRIPTOR);
    /// ```
    struct OnlyReads;

    /// Control for the example below: writing through a shared reference
    /// to a `std::fs::File` compiles.
    ///
    /// ```
    /// use std::io::Write;
    ///
    /// fn write(file: &std::fs::File) -> std::io::Result<()> {
    ///     let mut writer = file;
    ///     writer.write_all(b"x")
    /// }
    /// ```
    struct Control;

    /// The same through an open library file does not. Rustdoc on stable
    /// does not check which error stopped an example compiling, so this
    /// shows only that this one call is refused; the proof is the example
    /// above.
    ///
    /// ```compile_fail
    /// use std::io::Write;
    ///
    /// fn write(file: &gunmetal_fs::open::MediaFile) -> std::io::Result<()> {
    ///     let mut writer = file;
    ///     writer.write_all(b"x")
    /// }
    /// ```
    struct NoWriting;
}
