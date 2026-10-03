//! Facts about the host, read by one small probe and handed to the pure
//! checks in [`crate::dataroot`], so tests can state "another user owns the
//! directory" or "the data directory is on NFS" instead of needing either.
#![expect(
    clippy::disallowed_methods,
    reason = "the host probe reads the filesystem type of the data directory it is given (ADM-079)"
)]

use std::io;
use std::os::fd::AsFd;
use std::path::Path;

use rustix::fs::StatFs;

use crate::dataroot::{DataRootError, Item, NetworkFilesystems, Op, io_error};

/// What the data-root checks need to know about the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostFacts {
    /// The effective user ID the server runs as: the service account that
    /// must own the data directory (SEC-OPS-012).
    pub uid: u32,
    /// The kind of filesystem the data directory is on (ADM-079).
    pub filesystem: Filesystem,
}

/// The kind of filesystem holding the data directory or a library folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filesystem {
    /// A filesystem on a disk of this machine.
    Local,
    /// A network or user-space filesystem, where SQLite's locking cannot be
    /// trusted and owners and modes may not be enforced (ADM-079).
    Network(NetworkFs),
}

/// The network and user-space filesystems ADM-079 names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkFs {
    /// NFS.
    Nfs,
    /// SMB or CIFS.
    Smb,
    /// 9p, as virtual machines and WSL share folders.
    NineP,
    /// FUSE, which also covers union mounts such as mergerfs.
    Fuse,
}

impl Filesystem {
    /// Classifies the `f_type` magic number `statfs` reports, as the Linux
    /// header `linux/magic.h` defines them. A 32-bit kernel reports the two
    /// SMB magic numbers above `i32::MAX` as negative values, so both forms
    /// are listed.
    #[must_use]
    pub const fn from_magic(magic: i64) -> Self {
        match magic {
            0x6969 => Self::Network(NetworkFs::Nfs),
            0x517B | 0xFF53_4D42 | 0xFE53_4D42 | -0x00AC_B2BE | -0x01AC_B2BE => {
                Self::Network(NetworkFs::Smb)
            }
            0x0102_1997 => Self::Network(NetworkFs::NineP),
            0x6573_5546 => Self::Network(NetworkFs::Fuse),
            _ => Self::Local,
        }
    }
}

/// What a folder holds, which decides the filesystems it may be on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Holds {
    /// The data directory, which the server writes and where SQLite keeps
    /// its databases. A network or user-space filesystem is refused unless
    /// the owner set the documented override (ADM-079).
    Data(NetworkFilesystems),
    /// A library folder, which the server only ever reads. Every
    /// filesystem is accepted, FUSE included, because Unraid's `/mnt/user`
    /// is FUSE (the owner's answer of 2026-10-03). The folder still gets the
    /// same path and symlink checks as any other, and its filesystem type
    /// is shown on the library's health page ([`Filesystem::name`]).
    Library,
}

impl Filesystem {
    /// The filesystem's name as the library's health page and `doctor`
    /// show it: `local`, `nfs`, `smb`, `9p` or `fuse`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Network(NetworkFs::Nfs) => "nfs",
            Self::Network(NetworkFs::Smb) => "smb",
            Self::Network(NetworkFs::NineP) => "9p",
            Self::Network(NetworkFs::Fuse) => "fuse",
        }
    }

    /// Reads the type of the filesystem holding `handle`, an open file or
    /// directory, such as a library folder's root handle. It asks the
    /// handle rather than a path, so it describes the folder that was
    /// opened even if its path has been swapped since.
    ///
    /// # Errors
    ///
    /// Returns the operating system's error when the handle cannot be
    /// examined.
    pub fn of(handle: impl AsFd) -> io::Result<Self> {
        rustix::fs::fstatfs(handle)
            .map(|stat| Self::of_statfs(&stat))
            .map_err(io::Error::from)
    }

    /// Classifies what `statfs` or `fstatfs` reported.
    fn of_statfs(stat: &StatFs) -> Self {
        // `f_type`'s integer type depends on the target; widen it.
        #[cfg_attr(
            target_pointer_width = "64",
            expect(
                clippy::useless_conversion,
                reason = "f_type is already an i64 on the 64-bit Linux targets R1 builds for"
            )
        )]
        let magic = i64::from(stat.f_type);
        Self::from_magic(magic)
    }

    /// Decides whether a folder holding `holds` may be on this filesystem,
    /// or names the filesystem that refuses it.
    ///
    /// # Errors
    ///
    /// Returns the kind of network or user-space filesystem when it is
    /// refused.
    pub const fn admits(self, holds: Holds) -> Result<(), NetworkFs> {
        match (self, holds) {
            (Self::Network(kind), Holds::Data(NetworkFilesystems::Refuse)) => Err(kind),
            _ => Ok(()),
        }
    }
}

impl HostFacts {
    /// Reads the effective user ID of this process and the filesystem type
    /// of `data_dir`.
    ///
    /// # Errors
    ///
    /// Returns [`DataRootError::Io`] for [`Item::Root`] and [`Op::Probe`]
    /// when `data_dir` cannot be examined.
    pub fn probe(data_dir: &Path) -> Result<Self, DataRootError> {
        rustix::fs::statfs(data_dir)
            .map(|stat| Self {
                uid: rustix::process::geteuid().as_raw(),
                filesystem: Filesystem::of_statfs(&stat),
            })
            .map_err(io::Error::from)
            .map_err(io_error(Item::Root, Op::Probe))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_the_network_filesystems_adm_079_names() {
        let cases = [
            (0x6969, Filesystem::Network(NetworkFs::Nfs)),
            (0x517B, Filesystem::Network(NetworkFs::Smb)),
            (0xFF53_4D42, Filesystem::Network(NetworkFs::Smb)),
            (0xFE53_4D42, Filesystem::Network(NetworkFs::Smb)),
            (-0x00AC_B2BE, Filesystem::Network(NetworkFs::Smb)),
            (-0x01AC_B2BE, Filesystem::Network(NetworkFs::Smb)),
            (0x0102_1997, Filesystem::Network(NetworkFs::NineP)),
            (0x6573_5546, Filesystem::Network(NetworkFs::Fuse)),
        ];
        for (magic, expected) in cases {
            assert_eq!(Filesystem::from_magic(magic), expected, "magic {magic:#x}");
        }
    }

    #[test]
    fn admits_a_data_directory_on_a_local_filesystem_or_an_allowed_network_one() {
        for (filesystem, network) in [
            (Filesystem::Local, NetworkFilesystems::Refuse),
            (Filesystem::Local, NetworkFilesystems::Allow),
            (
                Filesystem::Network(NetworkFs::Smb),
                NetworkFilesystems::Allow,
            ),
            (
                Filesystem::Network(NetworkFs::Fuse),
                NetworkFilesystems::Allow,
            ),
        ] {
            assert_eq!(filesystem.admits(Holds::Data(network)), Ok(()));
        }
    }

    #[test]
    fn refuses_a_data_directory_on_a_network_or_user_space_filesystem_by_default() {
        for kind in [
            NetworkFs::Nfs,
            NetworkFs::Smb,
            NetworkFs::NineP,
            NetworkFs::Fuse,
        ] {
            assert_eq!(
                Filesystem::Network(kind).admits(Holds::Data(NetworkFilesystems::Refuse)),
                Err(kind)
            );
        }
    }

    /// The owner's answer of 2026-10-03: a library folder on FUSE, such as
    /// Unraid's `/mnt/user`, is accepted, and so is one on any other
    /// filesystem, because the server only reads it.
    #[test]
    fn admits_a_library_folder_on_every_filesystem_fuse_included() {
        for filesystem in [
            Filesystem::Local,
            Filesystem::Network(NetworkFs::Fuse),
            Filesystem::Network(NetworkFs::Nfs),
            Filesystem::Network(NetworkFs::Smb),
            Filesystem::Network(NetworkFs::NineP),
        ] {
            assert_eq!(filesystem.admits(Holds::Library), Ok(()));
        }
    }

    #[test]
    fn names_each_filesystem_for_the_health_page() {
        let names = [
            Filesystem::Local,
            Filesystem::Network(NetworkFs::Nfs),
            Filesystem::Network(NetworkFs::Smb),
            Filesystem::Network(NetworkFs::NineP),
            Filesystem::Network(NetworkFs::Fuse),
        ]
        .map(Filesystem::name);
        assert_eq!(names, ["local", "nfs", "smb", "9p", "fuse"]);
    }

    #[test]
    fn treats_disk_filesystems_as_local() {
        // ext4, btrfs, XFS, tmpfs and overlayfs.
        for magic in [0xEF53, 0x9123_683E, 0x5846_5342, 0x0102_1994, 0x794C_7630] {
            assert_eq!(
                Filesystem::from_magic(magic),
                Filesystem::Local,
                "magic {magic:#x}"
            );
        }
    }
}
