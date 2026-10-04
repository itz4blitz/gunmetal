//! The operations that change a queue, and why one can be refused.
//!
//! The set is closed and carries no text: every operation names entries,
//! items and sources by identifier, and a session only by the identifier
//! the server takes from the request (SEC-HIS-014 is proved where the
//! server accepts them, WP-085). An operation that adds several items, or
//! removes several entries, is one operation: it is applied whole or
//! refused whole.

use crate::id::PublicId;
use crate::problem::{Arg, Describe, Problem, ProblemCode};

use super::document::{EntryId, Lane, Repeat, Source, StopAfter};

/// An item an operation adds, with the ID its entry will have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewEntry {
    /// The new entry's ID, drawn by the device that issues the operation.
    pub id: EntryId,
    /// The item to play.
    pub item: PublicId,
}

/// Where a moved entry goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    /// Just before this upcoming entry, in its lane.
    Before(EntryId),
    /// At the end of this lane.
    End(Lane),
}

/// One change to a queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// Plays `items[start]` now, with `items` as the new From lane. Up next
    /// is kept: picks survive a new Play (owner decision 28).
    Play {
        /// What the items were chosen from.
        source: Source,
        /// The whole album, artist list or playlist, in order.
        items: Vec<NewEntry>,
        /// Which of them plays now.
        start: usize,
        /// The session that pressed Play; it becomes the queue's player.
        by: PublicId,
    },
    /// Puts items into Up next at the insertion cursor, so several "play
    /// next" picks play in the order they were chosen (MUS-117).
    PlayNext {
        /// What the items were chosen from.
        source: Source,
        /// The items, in order.
        items: Vec<NewEntry>,
    },
    /// Appends items to the end of Up next (MUS-118).
    Add {
        /// What the items were chosen from.
        source: Source,
        /// The items, in order.
        items: Vec<NewEntry>,
    },
    /// Appends items after the From lane, at the very end before Continue
    /// with (MUS-118).
    PlayLast {
        /// What the items were chosen from.
        source: Source,
        /// The items, in order.
        items: Vec<NewEntry>,
    },
    /// Moves an upcoming entry, within its lane or to the other one. The
    /// entry keeps its own source (MUS-123).
    Move {
        /// The entry.
        entry: EntryId,
        /// Where it goes.
        to: Position,
    },
    /// Removes upcoming entries, or the current one, which skips to the
    /// next item.
    Remove(Vec<EntryId>),
    /// Removes every pick from Up next ("Clear" on its header).
    ClearUpNext,
    /// Empties every lane ("Clear queue"). The item playing now plays on.
    Clear,
    /// The current entry played to its end. The repeat and stop-after
    /// modes decide what comes next, and the cursor goes back to the top
    /// of Up next when the current entry changes.
    Finished(EntryId),
    /// The person skipped the current entry. Repeat one and stop-after do
    /// not apply.
    Skip(EntryId),
    /// The session presses Play on the queue as it is, and takes it from
    /// any other session: the last Play wins (A-535). With nothing current,
    /// the next entry starts, and a finished From lane starts again.
    Resume {
        /// The session; it becomes the queue's player.
        by: PublicId,
    },
    /// Sets the repeat mode.
    SetRepeat(Repeat),
    /// Sets the stop-after mode.
    SetStopAfter(StopAfter),
}

/// Why an operation was refused. A refused operation changes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueReject {
    /// The operation was built on another version. The client rebuilds it
    /// on this one and tries again.
    Stale {
        /// The queue's version now.
        current: u64,
    },
    /// The version cannot go any higher.
    VersionExhausted,
    /// The operation names no items or entries.
    NoItems,
    /// Play's start is not one of its items.
    StartOutOfRange {
        /// The start asked for.
        start: usize,
        /// How many items there were.
        items: usize,
    },
    /// A new entry's ID is already in the queue or in the operation.
    DuplicateEntry {
        /// The ID.
        entry: EntryId,
    },
    /// No upcoming entry has this ID: it was never queued, has played or
    /// been removed, or is playing now and cannot move.
    UnknownEntry {
        /// The ID.
        entry: EntryId,
    },
    /// The entry an operation says is playing is not the current one: the
    /// queue moved on, perhaps on another device.
    NotCurrent {
        /// The ID.
        entry: EntryId,
    },
    /// Resume found nothing to play.
    NothingToPlay,
    /// A position came from a session that is not the queue's player, such
    /// as one that lost the queue to a later Play.
    NotPlayer,
}

impl Describe for QueueReject {
    /// `queue_stale` with the current version, which the client rebases
    /// on, and `queue_refused` with the reason for everything else.
    fn problem(&self) -> Problem {
        let reason = match self {
            Self::Stale { current } => {
                return Problem {
                    code: ProblemCode::QueueStale,
                    args: vec![("current", Arg::Number(*current))],
                };
            }
            Self::VersionExhausted => "version_exhausted",
            Self::NoItems => "no_items",
            Self::StartOutOfRange { .. } => "start_out_of_range",
            Self::DuplicateEntry { .. } => "duplicate_entry",
            Self::UnknownEntry { .. } => "unknown_entry",
            Self::NotCurrent { .. } => "not_current",
            Self::NothingToPlay => "nothing_to_play",
            Self::NotPlayer => "not_player",
        };
        Problem {
            code: ProblemCode::QueueRefused,
            args: vec![("reason", Arg::Name(reason))],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stale_operation_is_described_with_the_current_version() {
        assert_eq!(
            QueueReject::Stale { current: 42 }.problem(),
            Problem {
                code: ProblemCode::QueueStale,
                args: vec![("current", Arg::Number(42))],
            }
        );
    }

    #[test]
    fn every_other_refusal_is_described_with_its_reason() {
        let entry = EntryId::new([7; 16]);
        let cases = [
            (QueueReject::VersionExhausted, "version_exhausted"),
            (QueueReject::NoItems, "no_items"),
            (
                QueueReject::StartOutOfRange { start: 3, items: 2 },
                "start_out_of_range",
            ),
            (QueueReject::DuplicateEntry { entry }, "duplicate_entry"),
            (QueueReject::UnknownEntry { entry }, "unknown_entry"),
            (QueueReject::NotCurrent { entry }, "not_current"),
            (QueueReject::NothingToPlay, "nothing_to_play"),
            (QueueReject::NotPlayer, "not_player"),
        ];
        for (reject, reason) in cases {
            assert_eq!(
                reject.problem(),
                Problem {
                    code: ProblemCode::QueueRefused,
                    args: vec![("reason", Arg::Name(reason))],
                },
                "{reject:?}"
            );
        }
    }
}
