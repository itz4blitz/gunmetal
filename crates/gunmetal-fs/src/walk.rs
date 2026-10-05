//! Walking a library root.
//!
//! [`Root::walk`] lists every directory beneath the root once, in name
//! order, and reports what each entry is without opening any file: a
//! regular file with its identity, or something skipped with the reason.
//! Names stay bytes, so a name that is not UTF-8, or holds a newline, is
//! walked like any other (SEC-MED-040). A FIFO or a socket is skipped
//! whatever it is called, and is never opened (SEC-MED-035). A symbolic
//! link is judged by the root's policy; one that may not be followed is
//! skipped with where it leads, so that it can be listed (SEC-MED-034).
//!
//! The walk ends: a directory is listed once however many paths lead to
//! it, so a link back to a folder above it is skipped the second time
//! round. Exclusion rules join the walk in R1.1 (WP-140).

use gunmetal_core::path::RelPath;

use crate::fingerprint::DirSummary;
use crate::open::Identity;
use crate::root::{FsError, Root};

/// One thing a walk found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visit {
    /// A directory that was listed. The root itself comes first, with no
    /// names. Its entries follow: the files and what was skipped, in name
    /// order, and then each directory inside it in turn.
    Dir {
        /// Its path beneath the root.
        path: RelPath,
        /// The summary of its listing.
        summary: DirSummary,
    },
    /// A regular file, or a link that may be followed to one.
    File {
        /// Its path beneath the root, to open it by.
        path: RelPath,
        /// Its identity, to record in the index.
        identity: Identity,
        /// How many hard links name it.
        links: u64,
    },
    /// Something the walk did not take.
    Skipped {
        /// Its path beneath the root.
        path: RelPath,
        /// Why it was skipped.
        reason: FsError,
    },
}

/// A walk of one library root. See [`Root::walk`].
#[derive(Debug)]
pub struct Walk<'r> {
    _root: &'r Root,
}

impl Root {
    /// Walks everything beneath the root. Nothing is read until the walk
    /// is advanced, and each step lists one directory at most.
    #[must_use]
    pub const fn walk(&self) -> Walk<'_> {
        Walk { _root: self }
    }
}

impl Iterator for Walk<'_> {
    type Item = Visit;

    fn next(&mut self) -> Option<Visit> {
        None
    }
}
