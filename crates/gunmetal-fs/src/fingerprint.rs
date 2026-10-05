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

use gunmetal_core::crypto::sha256;

use crate::open::{Identity, MediaFile};
use crate::root::FsError;

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

/// The first 16 bytes of a digest.
fn short(digest: [u8; 32]) -> [u8; 16] {
    digest.first_chunk().copied().unwrap_or_default()
}

/// Fingerprints an open file.
///
/// # Errors
///
/// Returns [`FsError::Io`] with [`crate::root::Op::Read`] when the file
/// cannot be read, or became shorter after it was opened.
pub fn fingerprint(file: &MediaFile) -> Result<Fingerprint, FsError> {
    let identity = file.identity();
    let take = identity.size.min(SAMPLE_BYTES);
    let length = usize::try_from(take).unwrap_or_default();
    let mut head = vec![0; length];
    let mut tail = vec![0; length];
    file.read_exact_at(&mut head, 0)
        .and_then(|()| file.read_exact_at(&mut tail, identity.size.saturating_sub(take)))
        .map(|()| {
            head.extend_from_slice(&tail);
            Fingerprint {
                identity,
                sample: short(sha256(&head)),
            }
        })
}

/// Writes one entry of a listing as [`summarise`] describes it.
fn encode(bytes: &mut Vec<u8>, name: &[u8], mark: Mark) {
    let length = u64::try_from(name.len()).unwrap_or(u64::MAX);
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(name);
    match mark {
        Mark::File(identity) => {
            bytes.push(0);
            bytes.extend_from_slice(&identity.device.to_le_bytes());
            bytes.extend_from_slice(&identity.inode.to_le_bytes());
            bytes.extend_from_slice(&identity.size.to_le_bytes());
            bytes.extend_from_slice(&identity.modified.seconds.to_le_bytes());
            bytes.extend_from_slice(&identity.modified.nanoseconds.to_le_bytes());
        }
        Mark::Dir => bytes.push(1),
        Mark::Skipped => bytes.push(2),
    }
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
pub fn summarise<'a>(entries: impl IntoIterator<Item = (&'a [u8], Mark)>) -> DirSummary {
    let mut bytes = Vec::new();
    entries
        .into_iter()
        .for_each(|(name, mark)| encode(&mut bytes, name, mark));
    DirSummary(short(sha256(&bytes)))
}
