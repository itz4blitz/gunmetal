//! Opening one file beneath a library root, read-only.
//!
//! [`Root::open_file`] follows the path's links by the root's policy and
//! judges what they lead to. Anything that is not a regular file is
//! refused there, before it is opened: a FIFO, a socket, a device or a
//! directory (SEC-MED-035). A regular file is then opened beneath the
//! handle with `O_NONBLOCK`, `O_NOCTTY`, `O_CLOEXEC` and `O_NOFOLLOW`, so
//! that a FIFO swapped in since cannot block the open, a terminal cannot
//! become the server's controlling terminal, and a link swapped in since
//! is not followed where the kernel resolves the path. What was opened must
//! be the object that was judged, the same device and inode, and its type
//! is read again from the open handle, never from the path
//! ([`FsError::Replaced`], [`FsError::NotRegular`]).
//!
//! A [`MediaFile`] reads, and lends its descriptor for handing to a worker.
//! There is no way to ask for a handle that can write: the open options are
//! fixed here, and the root's directory handle is private (SEC-MED-038,
//! SEC-TM-042, SEC-OPS-054).
//!
//! [`Root::open_verified`] is the check before bytes are served: the file
//! at the recorded path must still be the one the index recorded
//! (SEC-MED-036, SEC-API-018).

use std::fs::File;
use std::os::fd::{AsFd, BorrowedFd};
use std::os::unix::fs::FileExt as _;

use cap_std::fs::{
    FileType, FileTypeExt as _, Metadata, MetadataExt as _, OpenOptions, OpenOptionsExt as _,
};
use gunmetal_core::path::RelPath;
use rustix::fs::OFlags;

use crate::root::{Base, FsError, Op, Root, io_error};

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
/// else. It reads, and lends its read-only descriptor.
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
    /// [`std::io::ErrorKind::UnexpectedEof`] when the file ends first.
    pub fn read_exact_at(&self, buffer: &mut [u8], offset: u64) -> Result<(), FsError> {
        self.file
            .read_exact_at(buffer, offset)
            .map_err(io_error(Op::Read))
    }
}

impl AsFd for MediaFile {
    /// The read-only descriptor, to hand to a worker process. Whoever holds
    /// it, or a clone of it, can read the file and not write it or change
    /// its length; as through any descriptor, the file's owner can change
    /// its mode and times.
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.file.as_fd()
    }
}

/// The only options a file is ever opened with: reading, and [`FLAGS`].
fn read_only() -> OpenOptions {
    OpenOptions::new().read(true).custom_flags(FLAGS).clone()
}

/// Refuses anything but a regular file (SEC-MED-035).
fn regular(kind: FileKind) -> Result<(), FsError> {
    if kind == FileKind::File {
        Ok(())
    } else {
        Err(FsError::NotRegular { found: kind })
    }
}

impl Base {
    /// Opens the regular file at `names`, which was judged to be `judged` a
    /// moment before. What the judgement says is not a regular file is
    /// refused without being opened. What is opened must be the object
    /// judged, and is refused again unless the open handle is a regular
    /// file (SEC-MED-034, SEC-MED-035).
    pub(crate) fn file(&self, names: &[Vec<u8>], judged: &Facts) -> Result<MediaFile, FsError> {
        regular(judged.kind)
            .and_then(|()| self.open_judged(names, &read_only(), judged, Op::Open))
            .and_then(|(file, facts)| {
                regular(facts.kind).map(|()| MediaFile {
                    file,
                    identity: facts.identity,
                    links: facts.links,
                })
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
    /// file, [`FsError::Replaced`] when what the path leads to changed
    /// between its judgement and the open, and [`FsError::Io`] when the
    /// file does not exist or may not be read.
    pub fn open_file(&self, rel: &RelPath) -> Result<MediaFile, FsError> {
        self.locate(self.own(), &[], rel.components())
            .and_then(|found| found.base.file(&found.names, &found.facts))
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
    /// back is not a writer, and does not convert into a `std::fs::File` or
    /// an `OwnedFd`.
    ///
    /// It does lend its descriptor (`AsFd`), so that it can be handed to a
    /// worker process, and a borrowed descriptor can be cloned into an owned
    /// one (`try_clone_to_owned`) and so into a `File`. That descriptor is
    /// open for reading only: it cannot write the file or change its length
    /// (`read_only::a_descriptor_cloned_from_an_open_file_still_cannot_write`
    /// in the library tests). The file's owner can change its mode and
    /// times through any descriptor (`fchmod`, `futimens`), this one too;
    /// nothing in this crate does.
    ///
    /// The probe below has its inherent constant only for a type that has
    /// the ability asked about, and the trait's constant for any other.
    /// The first six assertions show that it does see each ability, and
    /// that it can say no. Giving `MediaFile` any of the first three
    /// abilities, taking away the fourth, or giving `open_file` another
    /// argument makes this example fail.
    ///
    /// ```
    /// use std::fs::File;
    /// use std::io::Write;
    /// use std::marker::PhantomData;
    /// use std::os::fd::{AsFd, OwnedFd};
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
    ///     const LENDS_A_DESCRIPTOR: bool = false;
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
    /// impl<T: AsFd> Probe<T> {
    ///     const LENDS_A_DESCRIPTOR: bool = true;
    /// }
    ///
    /// assert!(<Probe<File>>::WRITES);
    /// assert!(<Probe<&File>>::WRITES);
    /// assert!(<Probe<File>>::BECOMES_A_FILE);
    /// assert!(<Probe<File>>::BECOMES_A_DESCRIPTOR);
    /// assert!(<Probe<File>>::LENDS_A_DESCRIPTOR);
    /// assert!(!<Probe<u8>>::LENDS_A_DESCRIPTOR);
    ///
    /// assert!(!<Probe<MediaFile>>::WRITES);
    /// assert!(!<Probe<&MediaFile>>::WRITES);
    /// assert!(!<Probe<MediaFile>>::BECOMES_A_FILE);
    /// assert!(!<Probe<MediaFile>>::BECOMES_A_DESCRIPTOR);
    /// assert!(<Probe<MediaFile>>::LENDS_A_DESCRIPTOR);
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
