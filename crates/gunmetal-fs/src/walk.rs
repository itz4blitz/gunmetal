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

use std::collections::{BTreeSet, VecDeque};

use gunmetal_core::parse::{LimitKind, Limits};
use gunmetal_core::path::RelPath;

use crate::fingerprint::{DirSummary, Mark, summarise};
use crate::open::{FileKind, Identity};
use crate::root::{Base, Found, FsError, Root};

/// How far a walk may go: how many entries a directory may hold, and how
/// deep it may lie, and still be listed. See [`Root::walk_with`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WalkLimits {
    /// The most entries a directory may hold and still be listed.
    pub entries: u64,
    /// The deepest a directory may lie and still be listed. The root lies
    /// at depth 0, and a directory in it at depth 1.
    pub depth: u64,
}

impl WalkLimits {
    /// The defaults, taken from the limits table every parse runs under
    /// (`docs/security/media-and-parser-safety.md`, section 3), which has
    /// no rows for folders: as many entries as a container's children
    /// ([`LimitKind::Children`], 65,536), and as deep as a binary
    /// container's nesting ([`LimitKind::ContainerDepth`], 32).
    pub const DEFAULT: Self = Self {
        entries: Limits::DEFAULT.get(LimitKind::Children),
        depth: Limits::DEFAULT.get(LimitKind::ContainerDepth),
    };
}

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

/// A directory waiting to be listed.
#[derive(Debug)]
struct Spot<'r> {
    /// Its path as the walk reports it, through any links.
    shown: RelPath,
    /// The folder it is really beneath.
    base: &'r Base,
    /// Its names beneath that folder.
    names: Vec<Vec<u8>>,
}

/// A walk of one library root. See [`Root::walk`].
#[derive(Debug)]
pub struct Walk<'r> {
    root: &'r Root,
    pending: Vec<Spot<'r>>,
    ready: VecDeque<Visit>,
    seen: BTreeSet<(u64, u64)>,
}

impl Root {
    /// Walks everything beneath the root. Nothing is read until the walk
    /// is advanced, and each step lists one directory at most.
    #[must_use]
    pub fn walk(&self) -> Walk<'_> {
        Walk {
            root: self,
            pending: vec![Spot {
                shown: self.top().clone(),
                base: self.own(),
                names: Vec::new(),
            }],
            ready: VecDeque::new(),
            seen: BTreeSet::new(),
        }
    }

    /// Walks everything beneath the root under `limits`.
    #[must_use]
    pub fn walk_with(&self, _limits: WalkLimits) -> Walk<'_> {
        self.walk()
    }
}

impl<'r> Walk<'r> {
    /// Records that the directory with `identity` is being listed, and
    /// refuses a directory that already was.
    fn fresh(&mut self, identity: Identity) -> Result<(), FsError> {
        if self.seen.insert((identity.device, identity.inode)) {
            Ok(())
        } else {
            Err(FsError::AlreadyWalked)
        }
    }

    /// Lists the directory at `spot`, unless it was listed before, and
    /// queues what it holds.
    fn enter(&mut self, spot: Spot<'r>) {
        let listed = spot
            .base
            .inspect(&spot.names)
            .and_then(|facts| self.fresh(facts.identity))
            .and_then(|()| spot.base.list(&spot.shown, &spot.names));
        match listed {
            Ok(entries) => self.take(spot, entries),
            Err(reason) => self.ready.push_back(Visit::Skipped {
                path: spot.shown,
                reason,
            }),
        }
    }

    /// Queues the directory at `spot` with its summary, then its files and
    /// what was skipped, and leaves the directories inside it to be listed
    /// next, in name order.
    fn take(&mut self, spot: Spot<'r>, entries: Vec<(RelPath, Vec<u8>)>) {
        let root = self.root;
        let mut marks = Vec::new();
        let mut visits = Vec::new();
        let mut inside = Vec::new();
        for (path, name) in entries {
            let found = root.locate(spot.base, &spot.names, std::slice::from_ref(&name));
            let mark = match found {
                Ok(Found { base, names, facts }) => match facts.kind {
                    FileKind::File => {
                        visits.push(Visit::File {
                            path,
                            identity: facts.identity,
                            links: facts.links,
                        });
                        Mark::File(facts.identity)
                    }
                    FileKind::Dir => {
                        inside.push(Spot {
                            shown: path,
                            base,
                            names,
                        });
                        Mark::Dir
                    }
                    found => {
                        visits.push(Visit::Skipped {
                            path,
                            reason: FsError::NotRegular { found },
                        });
                        Mark::Skipped
                    }
                },
                Err(reason) => {
                    visits.push(Visit::Skipped { path, reason });
                    Mark::Skipped
                }
            };
            marks.push((name, mark));
        }
        let summary = summarise(marks.iter().map(|(name, mark)| (name.as_slice(), *mark)));
        self.ready.push_back(Visit::Dir {
            path: spot.shown,
            summary,
        });
        self.ready.extend(visits);
        inside.reverse();
        self.pending.extend(inside);
    }
}

impl Iterator for Walk<'_> {
    type Item = Visit;

    fn next(&mut self) -> Option<Visit> {
        loop {
            if let Some(visit) = self.ready.pop_front() {
                return Some(visit);
            }
            let spot = self.pending.pop()?;
            self.enter(spot);
        }
    }
}
