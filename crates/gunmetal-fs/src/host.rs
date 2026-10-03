//! Facts about the host, read by one small probe and handed to the pure
//! checks in [`crate::dataroot`], so tests can state "another user owns the
//! directory" or "the data directory is on NFS" instead of needing either.

use std::io;
use std::path::Path;

use crate::dataroot::{DataRootError, Item, Op, io_error};

/// What the data-root checks need to know about the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostFacts {
    /// The effective user ID the server runs as: the service account that
    /// must own the data directory (SEC-OPS-012).
    pub uid: u32,
    /// The kind of filesystem the data directory is on (ADM-079).
    pub filesystem: Filesystem,
}

/// The kind of filesystem holding the data directory.
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
            .map(|stat| {
                // `f_type`'s integer type depends on the target; widen it.
                #[cfg_attr(
                    target_pointer_width = "64",
                    expect(
                        clippy::useless_conversion,
                        reason = "f_type is already an i64 on the 64-bit Linux targets R1 builds for"
                    )
                )]
                let magic = i64::from(stat.f_type);
                Self {
                    uid: rustix::process::geteuid().as_raw(),
                    filesystem: Filesystem::from_magic(magic),
                }
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
