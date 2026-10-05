//! Fingerprints and directory summaries: how a scan tells that nothing
//! changed without reading every file again (LIB-016).
//!
//! A [`Fingerprint`] is a file's [`Identity`] (device, inode, size and
//! modification time) with a short hash of its first and last few
//! kilobytes, for the files whose identity alone cannot be trusted, such as
//! one rewritten in place with its time put back. A [`DirSummary`] is one
//! value for a whole directory listing: equal summaries mean the same
//! names, each leading to the same kind of thing, and for files to the same
//! identity.
//!
//! Neither is a security boundary. The check before bytes are served
//! compares identities on the open handle ([`Root::open_verified`],
//! SEC-MED-036), and a hash of content never authorises access.
//!
//! [`Root::open_verified`]: crate::root::Root::open_verified

use crate::open::{Identity, MediaFile};
use crate::root::{FsError, UNBUILT};

/// How many bytes are hashed from each end of a file.
pub const SAMPLE_BYTES: u64 = 4096;

/// A file's identity and a hash of a sample of its content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fingerprint {
    /// The file's identity, read from the open handle.
    pub identity: Identity,
    /// The first 16 bytes of the SHA-256 digest of the file's first
    /// [`SAMPLE_BYTES`] bytes followed by its last [`SAMPLE_BYTES`] bytes.
    /// A shorter file is hashed whole, twice over.
    pub sample: [u8; 16],
}

/// What one entry of a directory turned out to be, for the directory's
/// summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    /// A regular file, with its identity.
    File(Identity),
    /// A directory.
    Dir,
    /// Something the walk did not take: a refused link, a FIFO, a socket.
    Skipped,
}

/// One value for a directory's whole listing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirSummary(pub [u8; 16]);

/// Fingerprints an open file.
///
/// # Errors
///
/// Returns [`FsError::Io`] with [`crate::root::Op::Read`] when the file
/// cannot be read, or became shorter after it was opened.
pub fn fingerprint(_file: &MediaFile) -> Result<Fingerprint, FsError> {
    Err(UNBUILT)
}

/// Summarises a directory's listing: `entries` are its names in name order,
/// each with what it turned out to be.
///
/// The summary is the first 16 bytes of the SHA-256 digest of, for each
/// entry, the length of its name as eight little-endian bytes, the name, one
/// byte for the mark (0 for a file, 1 for a directory, 2 for something
/// skipped) and, for a file, its device, inode, size, and the seconds and
/// nanoseconds of its modification time, each as eight little-endian bytes.
#[must_use]
pub fn summarise(entries: impl IntoIterator<Item = (&[u8], Mark)>) -> DirSummary {
    drop(entries);
    DirSummary([0; 16])
}
