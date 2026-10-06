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
//! round. The first path to reach a directory in the walk's order is the
//! one it is listed under. Exclusion rules join the walk in R1.1 (WP-140).
//!
//! A directory waiting to be listed is judged again when the walk comes to
//! it, so whatever replaced it since is taken for what it is now. It is
//! opened as a directory, not through a link in its last name, and listed
//! only if the open handle is the directory just judged
//! ([`FsError::Replaced`]); what is listed is read from that handle.
//!
//! **Limits.** The walk runs in the server, and a folder an attacker can
//! write to can hold millions of entries or nest without end. So a walk
//! runs under [`WalkLimits`]. A directory with more entries than
//! [`WalkLimits::entries`] is not listed at all, and one deeper than
//! [`WalkLimits::depth`] is not entered: each is reported once, as a
//! [`Visit::Skipped`] with [`FsError::TooManyEntries`] or
//! [`FsError::TooDeep`], nothing beneath it is walked, and its parent's
//! listing still names it as a directory. A directory is never listed in
//! part: the caller gets the whole of it or the reason it got none, and
//! should treat a skipped directory as unknown, not as empty.
//!
//! What a walk holds at once is bounded by those limits. It holds the
//! listing of the directory it is reporting, each name once with what it
//! is (for a refused link, where it leads), and for each directory above
//! that one only the names of the directories in it still to be walked:
//! at most `entries × (depth + 1)` names. Reading a directory's names
//! stops one past the limit. It also keeps the device and inode of every
//! directory it has listed, so that none is listed twice.

use std::collections::BTreeSet;
use std::convert::identity;
use std::slice;
use std::vec;

use gunmetal_core::parse::{LimitError, LimitKind, Limits};
use gunmetal_core::path::RelPath;

use crate::fingerprint::{DirSummary, Mark, summarise};
use crate::open::{FileKind, Identity};
use crate::root::{Base, Found, FsError, Root, child, read_names};

/// How far a walk may go: how many entries a directory may hold, and how
/// deep it may lie, and still be listed. See [`Root::walk_with`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WalkLimits {
    /// The most entries a directory may hold and still be listed.
    entries: u64,
    /// The deepest a directory may lie and still be listed. The root lies
    /// at depth 0, and a directory in it at depth 1.
    depth: u64,
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

    /// Limits of `entries` entries a directory and `depth` levels.
    ///
    /// # Errors
    ///
    /// Returns [`LimitError::AboveCeiling`] when a limit is above its
    /// ceiling.
    pub const fn new(entries: u64, depth: u64) -> Result<Self, LimitError> {
        Ok(Self { entries, depth })
    }

    /// The most entries a directory may hold and still be listed.
    #[must_use]
    pub const fn entries(self) -> u64 {
        self.entries
    }

    /// The deepest a directory may lie and still be listed. The root lies
    /// at depth 0, and a directory in it at depth 1.
    #[must_use]
    pub const fn depth(self) -> u64 {
        self.depth
    }
}

/// One thing a walk found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visit {
    /// A directory that was listed. The root itself comes first, with no
    /// names, unless it could not be listed. Its entries follow: the files
    /// and what was skipped, in name order, and then each directory inside
    /// it in turn.
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

/// What one entry of a directory turned out to be.
enum Class {
    /// A regular file, or a link that may be followed to one.
    File {
        /// Its identity.
        identity: Identity,
        /// How many hard links name it.
        links: u64,
    },
    /// A directory, or a link that may be followed to one.
    Dir,
    /// Something the walk does not take, and why.
    Skipped(FsError),
}

impl Class {
    /// What `found`, an entry located beneath its directory, is.
    fn of(found: Result<Found<'_>, FsError>) -> Self {
        match found {
            Ok(Found { facts, .. }) => match facts.kind {
                FileKind::File => Self::File {
                    identity: facts.identity,
                    links: facts.links,
                },
                FileKind::Dir => Self::Dir,
                found => Self::Skipped(FsError::NotRegular { found }),
            },
            Err(reason) => Self::Skipped(reason),
        }
    }

    /// The entry as the directory's summary counts it.
    const fn mark(&self) -> Mark {
        match self {
            Self::File { identity, .. } => Mark::File(*identity),
            Self::Dir => Mark::Dir,
            Self::Skipped(_) => Mark::Skipped,
        }
    }
}

/// What the walk reports of an entry that is not a directory: a regular
/// file's identity and how many names it has, or why it was skipped.
type Entry = Result<(Identity, u64), FsError>;

/// A directory the walk is inside.
#[derive(Debug)]
struct Level<'r> {
    /// Its path as the walk reports it, through any links.
    shown: RelPath,
    /// The folder it is really beneath.
    base: &'r Base,
    /// Its names beneath that folder.
    names: Vec<Vec<u8>>,
    /// Its entries that are not directories, in name order, still to be
    /// reported.
    entries: vec::IntoIter<(Vec<u8>, Entry)>,
    /// The names of the directories in it, in name order, still to be
    /// walked.
    dirs: vec::IntoIter<Vec<u8>>,
}

/// A walk of one library root. See [`Root::walk`].
#[derive(Debug)]
pub struct Walk<'r> {
    root: &'r Root,
    limits: WalkLimits,
    /// Whether the root itself is still to be listed.
    unstarted: bool,
    /// The directories the walk is inside, the root first.
    levels: Vec<Level<'r>>,
    /// The device and inode of every directory listed.
    seen: BTreeSet<(u64, u64)>,
}

impl Root {
    /// Walks everything beneath the root under [`WalkLimits::DEFAULT`].
    /// Nothing is read until the walk is advanced, and each step lists one
    /// directory at most.
    #[must_use]
    pub fn walk(&self) -> Walk<'_> {
        self.walk_with(WalkLimits::DEFAULT)
    }

    /// Walks everything beneath the root under `limits`, as [`Root::walk`]
    /// does.
    #[must_use]
    pub fn walk_with(&self, limits: WalkLimits) -> Walk<'_> {
        Walk {
            root: self,
            limits,
            unstarted: true,
            levels: Vec::new(),
            seen: BTreeSet::new(),
        }
    }
}

/// What the walk reports for something at `path` that it did not take.
/// Every skip of a directory goes through here, and so does the case of an
/// entry whose name cannot be part of a path, which no name the kernel
/// lists can reach.
fn skipped(path: &RelPath) -> impl FnOnce(FsError) -> Visit {
    move |reason| Visit::Skipped {
        path: path.clone(),
        reason,
    }
}

/// Refuses a directory at `depth` when that is deeper than `max`.
fn within(depth: u64, max: u64) -> Result<(), FsError> {
    if depth > max {
        Err(FsError::TooDeep { depth, max })
    } else {
        Ok(())
    }
}

/// The visit of `entry`, the entry `name` of the directory at `shown`.
fn report(shown: &RelPath, name: &[u8], entry: Entry) -> Visit {
    child(shown, name).map_or_else(skipped(shown), |path| match entry {
        Ok((identity, links)) => Visit::File {
            path,
            identity,
            links,
        },
        Err(reason) => Visit::Skipped { path, reason },
    })
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

    /// Lists the directory at `path`, `depth` levels below the root, which
    /// `located` says where it is and what it was just judged to be, and
    /// reports it; or reports why it was not listed.
    fn enter(&mut self, path: RelPath, located: Result<Found<'r>, FsError>, depth: u64) -> Visit {
        let limits = self.limits;
        let listed = within(depth, limits.depth)
            .and_then(|()| located)
            .and_then(|found| {
                found
                    .base
                    .open_dir(&found.names, &found.facts)
                    .and_then(|(dir, identity)| self.fresh(identity).map(|()| dir))
                    .and_then(|dir| read_names(&dir, limits.entries))
                    .map(|names| (found, names))
            });
        match listed {
            Ok((found, names)) => self.take(path, found, names),
            Err(reason) => skipped(&path)(reason),
        }
    }

    /// Judges each of `names`, the entries in name order of the directory
    /// `found` leads to, reports that directory with the summary of its
    /// listing, and keeps its entries and the directories in it to report
    /// next.
    fn take(&mut self, path: RelPath, found: Found<'r>, names: Vec<Vec<u8>>) -> Visit {
        let root = self.root;
        let judged: Vec<(Vec<u8>, Class)> = names
            .into_iter()
            .map(|name| {
                let class =
                    Class::of(root.locate(found.base, &found.names, slice::from_ref(&name)));
                (name, class)
            })
            .collect();
        let summary = summarise(
            judged
                .iter()
                .map(|(name, class)| (name.as_slice(), class.mark())),
        );
        let mut entries: Vec<(Vec<u8>, Entry)> = Vec::new();
        let mut dirs = Vec::new();
        for (name, class) in judged {
            match class {
                Class::File { identity, links } => entries.push((name, Ok((identity, links)))),
                Class::Dir => dirs.push(name),
                Class::Skipped(reason) => entries.push((name, Err(reason))),
            }
        }
        self.levels.push(Level {
            shown: path.clone(),
            base: found.base,
            names: found.names,
            entries: entries.into_iter(),
            dirs: dirs.into_iter(),
        });
        Visit::Dir { path, summary }
    }
}

impl Iterator for Walk<'_> {
    type Item = Visit;

    fn next(&mut self) -> Option<Visit> {
        let root = self.root;
        if self.unstarted {
            self.unstarted = false;
            let located = root.locate(root.own(), &[], &[]);
            return Some(self.enter(root.top().clone(), located, 0));
        }
        loop {
            let level = self.levels.last_mut()?;
            if let Some((name, entry)) = level.entries.next() {
                return Some(report(&level.shown, &name, entry));
            }
            if let Some(name) = level.dirs.next() {
                let path = child(&level.shown, &name).map_err(skipped(&level.shown));
                let located = root.locate(level.base, &level.names, slice::from_ref(&name));
                let depth = u64::try_from(self.levels.len()).unwrap_or(u64::MAX);
                return Some(path.map_or_else(identity, |path| self.enter(path, located, depth)));
            }
            self.levels.pop();
        }
    }
}
