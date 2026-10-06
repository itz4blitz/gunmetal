//! The play queue: applying and rebasing operations through the core
//! (MUS-116 to MUS-119, MUS-122, MUS-077).
//!
//! [`queue_apply`] and [`queue_rebase`] are one call each of [`queue::apply`]
//! and [`queue::rebase`] with their types converted. Identifiers cross as
//! their canonical text (a public ID) or as 32 lower-case hex digits (an
//! entry ID). The browser calls them as `queueApply` and `queueRebase`.

use gunmetal_core::id::{IdKind, PublicId};
use gunmetal_core::queue;
use gunmetal_core::queue::{
    Context, ContinueLane, Current, Entry, EntryId, FromLane, Lane, NewEntry, Op, PendingOp,
    Position, Queue, QueueReject, Repeat, Source, StopAfter,
};
use gunmetal_core::values::Duration;
use serde::{Deserialize, Serialize};
use tsify::Tsify;

/// A named listening context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub enum ListenContext {
    /// Music, the one context R1 plays.
    Music,
}

/// A lane a person can put entries into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub enum QueueLane {
    /// The person's own picks.
    UpNext,
    /// The album, artist or playlist being played.
    From,
}

/// What happens when an item ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub enum RepeatMode {
    /// Play the next item; stop at the end of the queue.
    Off,
    /// Play the same item again. Skipping still moves on.
    One,
    /// Play the From lane again from its start once it runs out.
    All,
}

/// When playback stops of its own accord.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub enum StopAfterMode {
    /// Keep playing.
    Off,
    /// Stop when the current item ends.
    Item,
    /// Stop when the From lane's last entry ends.
    Source,
}

/// Where a moved entry goes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub enum MoveTo {
    /// Just before this upcoming entry, named by its 32 hex digits.
    Before(String),
    /// At the end of this lane.
    End(QueueLane),
}

/// One queued item. `id` is 32 lower-case hex digits; `item` and `source`
/// are canonical public identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub struct QueueEntry {
    /// Names this entry within the queue.
    pub id: String,
    /// The item to play.
    pub item: String,
    /// Where the item was chosen from.
    pub source: String,
}

/// The entry playing now, and the lane it was taken from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub struct Playing {
    /// The entry.
    pub entry: QueueEntry,
    /// Where it came from.
    pub lane: QueueLane,
}

/// The From lane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub struct FromLaneMirror {
    /// What the last Play played, or omitted until the first Play.
    pub source: Option<String>,
    /// The lane's entries before the current one.
    pub before: Vec<QueueEntry>,
    /// The lane's entries still to play.
    pub upcoming: Vec<QueueEntry>,
}

/// One profile's queue in one listening context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub struct QueueDoc {
    /// How many operations have changed the document.
    pub version: u64,
    /// The listening context this queue belongs to.
    pub context: ListenContext,
    /// The entry playing now.
    pub current: Option<Playing>,
    /// How far into the current entry, in milliseconds.
    pub position_ms: Option<u64>,
    /// The person's picks, in the order they play.
    pub up_next: Vec<QueueEntry>,
    /// Where in Up next the next "play next" goes.
    pub cursor: u32,
    /// The album, artist or playlist being played.
    pub from: FromLaneMirror,
    /// Suggestions for when the From lane ends.
    pub continue_with: Vec<QueueEntry>,
    /// What happens when an item ends.
    pub repeat: RepeatMode,
    /// When playback stops of its own accord.
    pub stop_after: StopAfterMode,
    /// The session that last issued Play or Resume.
    pub player: Option<String>,
}

/// An item an operation adds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub struct NewQueueEntry {
    /// The new entry's ID, 32 lower-case hex digits.
    pub id: String,
    /// The item to play, a canonical public identifier.
    pub item: String,
}

/// One change to a queue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub enum QueueOp {
    /// Plays `items[start]` now, with `items` as the new From lane.
    Play {
        /// What the items were chosen from.
        source: String,
        /// The whole album, artist list or playlist, in order.
        items: Vec<NewQueueEntry>,
        /// Which of them plays now.
        start: u32,
        /// The session that pressed Play.
        by: String,
    },
    /// Puts items into Up next at the insertion cursor.
    PlayNext {
        /// What the items were chosen from.
        source: String,
        /// The items, in order.
        items: Vec<NewQueueEntry>,
    },
    /// Appends items to the end of Up next.
    Add {
        /// What the items were chosen from.
        source: String,
        /// The items, in order.
        items: Vec<NewQueueEntry>,
    },
    /// Appends items after the From lane.
    PlayLast {
        /// What the items were chosen from.
        source: String,
        /// The items, in order.
        items: Vec<NewQueueEntry>,
    },
    /// Moves an upcoming entry.
    Move {
        /// The entry, 32 hex digits.
        entry: String,
        /// Where it goes.
        to: MoveTo,
    },
    /// Removes upcoming entries, or the current one.
    Remove(Vec<String>),
    /// Removes every pick from Up next.
    ClearUpNext,
    /// Empties every lane. The item playing now plays on.
    Clear,
    /// The current entry played to its end.
    Finished(String),
    /// The person skipped the current entry.
    Skip(String),
    /// The session presses Play on the queue as it is.
    Resume {
        /// The session.
        by: String,
    },
    /// Sets the repeat mode.
    SetRepeat(RepeatMode),
    /// Sets the stop-after mode.
    SetStopAfter(StopAfterMode),
}

/// Why an operation was refused, or why a value could not be converted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Tsify)]
pub enum QueueError {
    /// The operation was built on another version.
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
        start: u32,
        /// How many items there were.
        items: u32,
    },
    /// A new entry's ID is already in the queue or in the operation.
    DuplicateEntry {
        /// The ID, 32 hex digits.
        entry: String,
    },
    /// No upcoming entry has this ID.
    UnknownEntry {
        /// The ID, 32 hex digits.
        entry: String,
    },
    /// The entry an operation says is playing is not the current one.
    NotCurrent {
        /// The ID, 32 hex digits.
        entry: String,
    },
    /// Resume found nothing to play.
    NothingToPlay,
    /// A position came from a session that is not the queue's player.
    NotPlayer,
    /// A public identifier or entry ID could not be read.
    Unusable,
}

/// What applying an operation gave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Tsify)]
pub enum ApplyOutcome {
    /// The queue after the operation.
    Queue(Box<QueueDoc>),
    /// The operation was refused, or a value could not be converted.
    Error(QueueError),
}

/// An operation the client has applied to its own copy and sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub struct Pending {
    /// The version the operation was built on.
    pub based_on: u64,
    /// The operation.
    pub op: QueueOp,
}

/// A pending operation that no longer fits the server's queue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Tsify)]
pub struct DroppedOp {
    /// The operation.
    pub op: QueueOp,
    /// Why it no longer fits.
    pub reason: QueueError,
}

/// The client's queue after a rebase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Tsify)]
pub struct RebasedQueue {
    /// The server's queue with the pending operations that still fit applied.
    pub queue: QueueDoc,
    /// Those operations, each built on the version before it.
    pub pending: Vec<Pending>,
    /// The operations that no longer fit.
    pub dropped: Vec<DroppedOp>,
}

/// What rebasing pending operations gave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Tsify)]
pub enum RebaseOutcome {
    /// The rebased queue.
    Rebased(Box<RebasedQueue>),
    /// A value could not be converted.
    Unusable,
}

/// A rebase request: the operations the server has not acknowledged, and
/// the server's queue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub struct RebaseRequest {
    /// The operations the client has sent, in order.
    pub pending: Vec<Pending>,
    /// The server's queue.
    pub server: QueueDoc,
}

/// An apply request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub struct ApplyRequest {
    /// The queue to change.
    pub queue: QueueDoc,
    /// The version the operation was built on.
    pub based_on: u64,
    /// The operation.
    pub op: QueueOp,
}

/// The TypeScript declarations of this module's mirror types, in the order
/// they are declared above.
pub const DECLARATIONS: [&str; 19] = [
    ListenContext::DECL,
    QueueLane::DECL,
    RepeatMode::DECL,
    StopAfterMode::DECL,
    MoveTo::DECL,
    QueueEntry::DECL,
    Playing::DECL,
    FromLaneMirror::DECL,
    QueueDoc::DECL,
    NewQueueEntry::DECL,
    QueueOp::DECL,
    QueueError::DECL,
    ApplyOutcome::DECL,
    Pending::DECL,
    DroppedOp::DECL,
    RebasedQueue::DECL,
    RebaseOutcome::DECL,
    RebaseRequest::DECL,
    ApplyRequest::DECL,
];

fn hex_digit(nibble: u8) -> char {
    if nibble < 10 {
        char::from(nibble.wrapping_add(b'0'))
    } else {
        char::from(nibble.wrapping_add(b'a').wrapping_sub(10))
    }
}

fn entry_hex(id: EntryId) -> String {
    let mut text = String::with_capacity(32);
    for byte in id.bytes() {
        text.push(hex_digit(byte >> 4));
        text.push(hex_digit(byte & 0x0f));
    }
    text
}

fn nibble(ch: u8) -> Option<u8> {
    match ch {
        b'0'..=b'9' => Some(ch - b'0'),
        b'a'..=b'f' => Some(ch - b'a' + 10),
        _ => None,
    }
}

fn hex_pair(high: u8, low: u8) -> Result<u8, QueueError> {
    let hi = nibble(high).ok_or(QueueError::Unusable)?;
    let lo = nibble(low).ok_or(QueueError::Unusable)?;
    Ok(hi.wrapping_shl(4).wrapping_add(lo))
}

fn parse_entry(text: &str) -> Result<EntryId, QueueError> {
    let mut rest = text.as_bytes();
    let mut id = [0_u8; 16];
    for slot in &mut id {
        let Some((&high, rest1)) = rest.split_first() else {
            return Err(QueueError::Unusable);
        };
        let Some((&low, rest2)) = rest1.split_first() else {
            return Err(QueueError::Unusable);
        };
        rest = rest2;
        *slot = hex_pair(high, low)?;
    }
    if rest.is_empty() {
        Ok(EntryId::new(id))
    } else {
        Err(QueueError::Unusable)
    }
}

fn index_u32(index: usize) -> u32 {
    u32::try_from(index).unwrap_or(u32::MAX)
}

fn parse_public(text: &str) -> Result<PublicId, QueueError> {
    for kind in IdKind::ALL {
        if let Ok(id) = PublicId::parse(text, kind) {
            return Ok(id);
        }
    }
    Err(QueueError::Unusable)
}

impl From<Context> for ListenContext {
    fn from(value: Context) -> Self {
        match value {
            Context::Music => Self::Music,
        }
    }
}

impl From<ListenContext> for Context {
    fn from(value: ListenContext) -> Self {
        match value {
            ListenContext::Music => Self::Music,
        }
    }
}

impl From<Lane> for QueueLane {
    fn from(value: Lane) -> Self {
        match value {
            Lane::UpNext => Self::UpNext,
            Lane::From => Self::From,
        }
    }
}

impl From<QueueLane> for Lane {
    fn from(value: QueueLane) -> Self {
        match value {
            QueueLane::UpNext => Self::UpNext,
            QueueLane::From => Self::From,
        }
    }
}

impl From<Repeat> for RepeatMode {
    fn from(value: Repeat) -> Self {
        match value {
            Repeat::Off => Self::Off,
            Repeat::One => Self::One,
            Repeat::All => Self::All,
        }
    }
}

impl From<RepeatMode> for Repeat {
    fn from(value: RepeatMode) -> Self {
        match value {
            RepeatMode::Off => Self::Off,
            RepeatMode::One => Self::One,
            RepeatMode::All => Self::All,
        }
    }
}

impl From<StopAfter> for StopAfterMode {
    fn from(value: StopAfter) -> Self {
        match value {
            StopAfter::Off => Self::Off,
            StopAfter::Item => Self::Item,
            StopAfter::Source => Self::Source,
        }
    }
}

impl From<StopAfterMode> for StopAfter {
    fn from(value: StopAfterMode) -> Self {
        match value {
            StopAfterMode::Off => Self::Off,
            StopAfterMode::Item => Self::Item,
            StopAfterMode::Source => Self::Source,
        }
    }
}

impl From<Entry> for QueueEntry {
    fn from(value: Entry) -> Self {
        Self {
            id: entry_hex(value.id),
            item: value.item.to_string(),
            source: value.source.0.to_string(),
        }
    }
}

impl TryFrom<QueueEntry> for Entry {
    type Error = QueueError;

    fn try_from(value: QueueEntry) -> Result<Self, QueueError> {
        Ok(Self {
            id: parse_entry(&value.id)?,
            item: parse_public(&value.item)?,
            source: Source(parse_public(&value.source)?),
        })
    }
}

impl From<&Queue> for QueueDoc {
    fn from(value: &Queue) -> Self {
        Self {
            version: value.version,
            context: value.context.into(),
            current: value.current.map(|current| Playing {
                entry: current.entry.into(),
                lane: current.lane.into(),
            }),
            position_ms: value.position.map(Duration::millis),
            up_next: value
                .up_next
                .iter()
                .copied()
                .map(QueueEntry::from)
                .collect(),
            cursor: index_u32(value.cursor),
            from: FromLaneMirror {
                source: value.from.source.map(|source| source.0.to_string()),
                before: value
                    .from
                    .before
                    .iter()
                    .copied()
                    .map(QueueEntry::from)
                    .collect(),
                upcoming: value
                    .from
                    .upcoming
                    .iter()
                    .copied()
                    .map(QueueEntry::from)
                    .collect(),
            },
            continue_with: value
                .continue_with
                .entries
                .iter()
                .copied()
                .map(QueueEntry::from)
                .collect(),
            repeat: value.repeat.into(),
            stop_after: value.stop_after.into(),
            player: value.player.map(|id| id.to_string()),
        }
    }
}

fn try_queue(doc: QueueDoc) -> Result<Queue, QueueError> {
    Ok(Queue {
        version: doc.version,
        context: doc.context.into(),
        current: match doc.current {
            None => None,
            Some(playing) => Some(Current {
                entry: playing.entry.try_into()?,
                lane: playing.lane.into(),
            }),
        },
        position: match doc.position_ms {
            None => None,
            Some(ms) => Some(Duration::from_millis(ms).map_err(|_| QueueError::Unusable)?),
        },
        up_next: doc
            .up_next
            .into_iter()
            .map(Entry::try_from)
            .collect::<Result<_, _>>()?,
        cursor: doc.cursor as usize,
        from: FromLane {
            source: match doc.from.source {
                None => None,
                Some(text) => Some(Source(parse_public(&text)?)),
            },
            before: doc
                .from
                .before
                .into_iter()
                .map(Entry::try_from)
                .collect::<Result<_, _>>()?,
            upcoming: doc
                .from
                .upcoming
                .into_iter()
                .map(Entry::try_from)
                .collect::<Result<_, _>>()?,
        },
        continue_with: ContinueLane {
            entries: doc
                .continue_with
                .into_iter()
                .map(Entry::try_from)
                .collect::<Result<_, _>>()?,
        },
        repeat: doc.repeat.into(),
        stop_after: doc.stop_after.into(),
        player: match doc.player {
            None => None,
            Some(text) => Some(parse_public(&text)?),
        },
    })
}

fn try_new_entries(items: Vec<NewQueueEntry>) -> Result<Vec<NewEntry>, QueueError> {
    items
        .into_iter()
        .map(|item| {
            Ok(NewEntry {
                id: parse_entry(&item.id)?,
                item: parse_public(&item.item)?,
            })
        })
        .collect()
}

fn try_op(op: QueueOp) -> Result<Op, QueueError> {
    Ok(match op {
        QueueOp::Play {
            source,
            items,
            start,
            by,
        } => Op::Play {
            source: Source(parse_public(&source)?),
            items: try_new_entries(items)?,
            start: start as usize,
            by: parse_public(&by)?,
        },
        QueueOp::PlayNext { source, items } => Op::PlayNext {
            source: Source(parse_public(&source)?),
            items: try_new_entries(items)?,
        },
        QueueOp::Add { source, items } => Op::Add {
            source: Source(parse_public(&source)?),
            items: try_new_entries(items)?,
        },
        QueueOp::PlayLast { source, items } => Op::PlayLast {
            source: Source(parse_public(&source)?),
            items: try_new_entries(items)?,
        },
        QueueOp::Move { entry, to } => Op::Move {
            entry: parse_entry(&entry)?,
            to: match to {
                MoveTo::Before(id) => Position::Before(parse_entry(&id)?),
                MoveTo::End(lane) => Position::End(lane.into()),
            },
        },
        QueueOp::Remove(ids) => Op::Remove(
            ids.into_iter()
                .map(|id| parse_entry(&id))
                .collect::<Result<_, _>>()?,
        ),
        QueueOp::ClearUpNext => Op::ClearUpNext,
        QueueOp::Clear => Op::Clear,
        QueueOp::Finished(id) => Op::Finished(parse_entry(&id)?),
        QueueOp::Skip(id) => Op::Skip(parse_entry(&id)?),
        QueueOp::Resume { by } => Op::Resume {
            by: parse_public(&by)?,
        },
        QueueOp::SetRepeat(mode) => Op::SetRepeat(mode.into()),
        QueueOp::SetStopAfter(mode) => Op::SetStopAfter(mode.into()),
    })
}

impl From<&Op> for QueueOp {
    fn from(value: &Op) -> Self {
        match value {
            Op::Play {
                source,
                items,
                start,
                by,
            } => Self::Play {
                source: source.0.to_string(),
                items: items
                    .iter()
                    .map(|item| NewQueueEntry {
                        id: entry_hex(item.id),
                        item: item.item.to_string(),
                    })
                    .collect(),
                start: index_u32(*start),
                by: by.to_string(),
            },
            Op::PlayNext { source, items } => Self::PlayNext {
                source: source.0.to_string(),
                items: items
                    .iter()
                    .map(|item| NewQueueEntry {
                        id: entry_hex(item.id),
                        item: item.item.to_string(),
                    })
                    .collect(),
            },
            Op::Add { source, items } => Self::Add {
                source: source.0.to_string(),
                items: items
                    .iter()
                    .map(|item| NewQueueEntry {
                        id: entry_hex(item.id),
                        item: item.item.to_string(),
                    })
                    .collect(),
            },
            Op::PlayLast { source, items } => Self::PlayLast {
                source: source.0.to_string(),
                items: items
                    .iter()
                    .map(|item| NewQueueEntry {
                        id: entry_hex(item.id),
                        item: item.item.to_string(),
                    })
                    .collect(),
            },
            Op::Move { entry, to } => Self::Move {
                entry: entry_hex(*entry),
                to: match *to {
                    Position::Before(id) => MoveTo::Before(entry_hex(id)),
                    Position::End(lane) => MoveTo::End(lane.into()),
                },
            },
            Op::Remove(ids) => Self::Remove(ids.iter().copied().map(entry_hex).collect()),
            Op::ClearUpNext => Self::ClearUpNext,
            Op::Clear => Self::Clear,
            Op::Finished(id) => Self::Finished(entry_hex(*id)),
            Op::Skip(id) => Self::Skip(entry_hex(*id)),
            Op::Resume { by } => Self::Resume { by: by.to_string() },
            Op::SetRepeat(mode) => Self::SetRepeat((*mode).into()),
            Op::SetStopAfter(mode) => Self::SetStopAfter((*mode).into()),
        }
    }
}

impl From<QueueReject> for QueueError {
    fn from(value: QueueReject) -> Self {
        match value {
            QueueReject::Stale { current } => Self::Stale { current },
            QueueReject::VersionExhausted => Self::VersionExhausted,
            QueueReject::NoItems => Self::NoItems,
            QueueReject::StartOutOfRange { start, items } => Self::StartOutOfRange {
                start: index_u32(start),
                items: index_u32(items),
            },
            QueueReject::DuplicateEntry { entry } => Self::DuplicateEntry {
                entry: entry_hex(entry),
            },
            QueueReject::UnknownEntry { entry } => Self::UnknownEntry {
                entry: entry_hex(entry),
            },
            QueueReject::NotCurrent { entry } => Self::NotCurrent {
                entry: entry_hex(entry),
            },
            QueueReject::NothingToPlay => Self::NothingToPlay,
            QueueReject::NotPlayer => Self::NotPlayer,
        }
    }
}

/// Applies `request.op` to `request.queue`, built on `request.based_on`.
///
/// Verifies: SEC-CLI-021
#[must_use]
pub fn queue_apply(request: ApplyRequest) -> ApplyOutcome {
    let ApplyRequest {
        queue,
        based_on,
        op,
    } = request;
    let queue = match try_queue(queue) {
        Ok(queue) => queue,
        Err(error) => return ApplyOutcome::Error(error),
    };
    let op = match try_op(op) {
        Ok(op) => op,
        Err(error) => return ApplyOutcome::Error(error),
    };
    match queue::apply(&queue, based_on, &op) {
        Ok(queue) => ApplyOutcome::Queue(Box::new(QueueDoc::from(&queue))),
        Err(reason) => ApplyOutcome::Error(reason.into()),
    }
}

/// Rebuilds `request.pending` on `request.server`.
///
/// Verifies: SEC-CLI-021
#[must_use]
pub fn queue_rebase(request: RebaseRequest) -> RebaseOutcome {
    let RebaseRequest { pending, server } = request;
    let Ok(server) = try_queue(server) else {
        return RebaseOutcome::Unusable;
    };
    let mut core_pending = Vec::new();
    for Pending { based_on, op } in pending {
        let Ok(op) = try_op(op) else {
            return RebaseOutcome::Unusable;
        };
        core_pending.push(PendingOp { based_on, op });
    }
    let rebased = queue::rebase(&core_pending, &server);
    RebaseOutcome::Rebased(Box::new(RebasedQueue {
        queue: QueueDoc::from(&rebased.queue),
        pending: rebased
            .pending
            .iter()
            .map(|pending| Pending {
                based_on: pending.based_on,
                op: QueueOp::from(&pending.op),
            })
            .collect(),
        dropped: rebased
            .dropped
            .iter()
            .map(|dropped| DroppedOp {
                op: QueueOp::from(&dropped.op),
                reason: dropped.reason.into(),
            })
            .collect(),
    }))
}

crate::export::export! {
    /// The browser's `queueApply`: [`queue_apply`], with its types converted.
    "queueApply": fn queue_apply_export = queue_apply(; request: ApplyRequest) -> ApplyOutcome
}

crate::export::export! {
    /// The browser's `queueRebase`: [`queue_rebase`], with its types converted.
    "queueRebase": fn queue_rebase_export = queue_rebase(; request: RebaseRequest) -> RebaseOutcome
}

#[cfg(test)]
mod tests {
    use gunmetal_core::id::{IdKind, PublicId};
    use gunmetal_core::queue::{
        self, Context, ContinueLane, Current, Entry, EntryId, FromLane, Lane, NewEntry, Op, Queue,
        Repeat, Source, StopAfter,
    };
    use gunmetal_core::values::Duration;

    use super::{
        ApplyOutcome, ApplyRequest, DECLARATIONS, DroppedOp, FromLaneMirror, ListenContext,
        NewQueueEntry, Pending, Playing, QueueDoc, QueueEntry, QueueError, QueueLane, QueueOp,
        RebaseOutcome, RebaseRequest, RepeatMode, StopAfterMode, queue_apply, queue_rebase,
    };

    fn track(n: u8) -> String {
        PublicId::parse(&format!("trk_{n:026}"), IdKind::Track)
            .expect("canonical")
            .to_string()
    }

    fn album(n: u8) -> String {
        PublicId::parse(&format!("alb_{n:026}"), IdKind::Album)
            .expect("canonical")
            .to_string()
    }

    fn session(n: u8) -> String {
        PublicId::parse(&format!("dev_{n:026}"), IdKind::Device)
            .expect("canonical")
            .to_string()
    }

    fn entry_id(n: u8) -> String {
        format!("{n:02x}").repeat(16)
    }

    fn empty_doc() -> QueueDoc {
        QueueDoc {
            version: 0,
            context: ListenContext::Music,
            current: None,
            position_ms: None,
            up_next: vec![],
            cursor: 0,
            from: FromLaneMirror {
                source: None,
                before: vec![],
                upcoming: vec![],
            },
            continue_with: vec![],
            repeat: RepeatMode::Off,
            stop_after: StopAfterMode::Off,
            player: None,
        }
    }

    fn play(start: u32, items: &[u8]) -> QueueOp {
        QueueOp::Play {
            source: album(1),
            items: items
                .iter()
                .map(|&n| NewQueueEntry {
                    id: entry_id(n),
                    item: track(n),
                })
                .collect(),
            start,
            by: session(1),
        }
    }

    #[test]
    fn an_empty_queue_converts_field_by_field() {
        let core = Queue::new(Context::Music);
        assert_eq!(QueueDoc::from(&core), empty_doc());
    }

    /// Play on an empty queue matches the core: the start item is current,
    /// the rest are the From lane, version is 1.
    #[test]
    fn play_on_an_empty_queue_matches_the_core() {
        let doc = must_apply(empty_doc(), 0, play(0, &[1, 2, 3]));
        assert_eq!(doc.version, 1);
        let current = doc.current.expect("something is playing");
        assert_eq!(current.entry.id, entry_id(1));
        assert_eq!(current.entry.item, track(1));
        assert_eq!(doc.from.upcoming.len(), 2);
        assert_eq!(doc.from.upcoming[0].id, entry_id(2));
        assert_eq!(doc.from.upcoming[1].id, entry_id(3));
        assert_eq!(doc.player.as_deref(), Some(session(1).as_str()));

        let core = queue::apply(
            &Queue::new(Context::Music),
            0,
            &Op::Play {
                source: Source(PublicId::parse(&album(1), IdKind::Album).expect("canonical")),
                items: vec![
                    NewEntry {
                        id: EntryId::new([1; 16]),
                        item: PublicId::parse(&track(1), IdKind::Track).expect("canonical"),
                    },
                    NewEntry {
                        id: EntryId::new([2; 16]),
                        item: PublicId::parse(&track(2), IdKind::Track).expect("canonical"),
                    },
                    NewEntry {
                        id: EntryId::new([3; 16]),
                        item: PublicId::parse(&track(3), IdKind::Track).expect("canonical"),
                    },
                ],
                start: 0,
                by: PublicId::parse(&session(1), IdKind::Device).expect("canonical"),
            },
        )
        .expect("play on empty");
        assert_eq!(QueueDoc::from(&core).version, 1);
        assert_eq!(QueueDoc::from(&core).current.unwrap().entry.id, entry_id(1));
    }

    /// A stale based-on is the typed stale error with the current version.
    #[test]
    fn a_stale_operation_is_the_current_version() {
        assert_eq!(
            applied(empty_doc(), 1, play(0, &[1])),
            Err(QueueError::Stale { current: 0 })
        );
    }

    /// A malformed identifier is a conversion error, not a guess.
    ///
    /// Verifies: SEC-CLI-021
    #[test]
    fn a_malformed_identifier_is_unusable() {
        assert_eq!(
            queue_apply(ApplyRequest {
                queue: empty_doc(),
                based_on: 0,
                op: QueueOp::Play {
                    source: album(1),
                    items: new_items(1),
                    start: 0,
                    by: "not-an-id".to_owned(),
                },
            }),
            ApplyOutcome::Error(QueueError::Unusable)
        );
        assert_eq!(
            queue_apply(ApplyRequest {
                queue: empty_doc(),
                based_on: 0,
                op: QueueOp::Finished("zz".to_owned()),
            }),
            ApplyOutcome::Error(QueueError::Unusable)
        );
    }

    /// Rebase with nothing pending is the server's queue.
    #[test]
    fn rebase_with_nothing_pending_is_the_server_queue() {
        let rebased = rebase_of(vec![], empty_doc()).unwrap();
        assert_eq!(rebased.queue, empty_doc());
        assert!(rebased.pending.is_empty());
        assert!(rebased.dropped.is_empty());
    }

    #[test]
    fn every_operation_converts_to_the_core_and_back() {
        let items = vec![NewQueueEntry {
            id: entry_id(0xab),
            item: track(1),
        }];
        let ops = [
            play(0, &[1]),
            QueueOp::PlayNext {
                source: album(1),
                items: items.clone(),
            },
            QueueOp::Add {
                source: album(1),
                items: items.clone(),
            },
            QueueOp::PlayLast {
                source: album(1),
                items: items.clone(),
            },
            QueueOp::Move {
                entry: entry_id(1),
                to: super::MoveTo::Before(entry_id(2)),
            },
            QueueOp::Move {
                entry: entry_id(1),
                to: super::MoveTo::End(super::QueueLane::From),
            },
            QueueOp::Remove(vec![entry_id(1)]),
            QueueOp::ClearUpNext,
            QueueOp::Clear,
            QueueOp::Finished(entry_id(1)),
            QueueOp::Skip(entry_id(1)),
            QueueOp::Resume { by: session(1) },
            QueueOp::SetRepeat(RepeatMode::One),
            QueueOp::SetRepeat(RepeatMode::All),
            QueueOp::SetStopAfter(StopAfterMode::Item),
            QueueOp::SetStopAfter(StopAfterMode::Source),
            QueueOp::Move {
                entry: entry_id(1),
                to: super::MoveTo::End(super::QueueLane::UpNext),
            },
        ];
        for op in ops {
            let core = super::try_op(op.clone()).expect("canonical identifiers");
            assert_eq!(QueueOp::from(&core), op);
        }
    }

    #[test]
    fn every_refusal_converts() {
        use gunmetal_core::queue::QueueReject;
        let entry = EntryId::new([3; 16]);
        let pairs = [
            (
                QueueReject::Stale { current: 9 },
                QueueError::Stale { current: 9 },
            ),
            (QueueReject::VersionExhausted, QueueError::VersionExhausted),
            (QueueReject::NoItems, QueueError::NoItems),
            (
                QueueReject::StartOutOfRange { start: 2, items: 1 },
                QueueError::StartOutOfRange { start: 2, items: 1 },
            ),
            (
                QueueReject::DuplicateEntry { entry },
                QueueError::DuplicateEntry { entry: entry_id(3) },
            ),
            (
                QueueReject::UnknownEntry { entry },
                QueueError::UnknownEntry { entry: entry_id(3) },
            ),
            (
                QueueReject::NotCurrent { entry },
                QueueError::NotCurrent { entry: entry_id(3) },
            ),
            (QueueReject::NothingToPlay, QueueError::NothingToPlay),
            (QueueReject::NotPlayer, QueueError::NotPlayer),
        ];
        for (core, mirrored) in pairs {
            assert_eq!(QueueError::from(core), mirrored);
        }
    }

    #[test]
    fn the_declarations_name_the_apply_and_rebase_types() {
        assert_eq!(DECLARATIONS.len(), 19);
        assert!(DECLARATIONS[8].contains("export interface QueueDoc"));
        assert!(DECLARATIONS[10].contains("export type QueueOp"));
        assert!(DECLARATIONS[12].contains("export type ApplyOutcome"));
        assert!(DECLARATIONS[16].contains("export type RebaseOutcome"));
    }

    fn core_entry(n: u8) -> Entry {
        Entry {
            id: EntryId::new([n; 16]),
            item: PublicId::parse(&track(n), IdKind::Track).expect("canonical"),
            source: Source(PublicId::parse(&album(1), IdKind::Album).expect("canonical")),
        }
    }

    fn queue_entry(n: u8) -> QueueEntry {
        QueueEntry {
            id: entry_id(n),
            item: track(n),
            source: album(1),
        }
    }

    fn applied(queue: QueueDoc, based_on: u64, op: QueueOp) -> Result<QueueDoc, QueueError> {
        match queue_apply(ApplyRequest {
            queue,
            based_on,
            op,
        }) {
            ApplyOutcome::Queue(doc) => Ok(*doc),
            ApplyOutcome::Error(error) => Err(error),
        }
    }

    fn must_apply(queue: QueueDoc, based_on: u64, op: QueueOp) -> QueueDoc {
        applied(queue, based_on, op).unwrap()
    }

    fn rebase_of(
        pending: Vec<Pending>,
        server: QueueDoc,
    ) -> Result<super::RebasedQueue, RebaseOutcome> {
        match queue_rebase(RebaseRequest { pending, server }) {
            RebaseOutcome::Rebased(rebased) => Ok(*rebased),
            RebaseOutcome::Unusable => Err(RebaseOutcome::Unusable),
        }
    }

    fn refuse_clear(queue: QueueDoc) {
        assert_eq!(
            queue_apply(ApplyRequest {
                queue,
                based_on: 0,
                op: QueueOp::Clear,
            }),
            ApplyOutcome::Error(QueueError::Unusable)
        );
    }

    fn new_items(n: u8) -> Vec<NewQueueEntry> {
        vec![NewQueueEntry {
            id: entry_id(n),
            item: track(n),
        }]
    }

    #[test]
    fn a_filled_queue_converts_field_by_field_and_back() {
        let source = Source(PublicId::parse(&album(1), IdKind::Album).expect("canonical"));
        let core = Queue {
            version: 4,
            context: Context::Music,
            current: Some(Current {
                entry: core_entry(1),
                lane: Lane::UpNext,
            }),
            position: Some(Duration::from_millis(1_500).expect("in range")),
            up_next: vec![core_entry(2)],
            cursor: 1,
            from: FromLane {
                source: Some(source),
                before: vec![core_entry(3)],
                upcoming: vec![core_entry(4)],
            },
            continue_with: ContinueLane {
                entries: vec![core_entry(5)],
            },
            repeat: Repeat::All,
            stop_after: StopAfter::Source,
            player: Some(PublicId::parse(&session(1), IdKind::Device).expect("canonical")),
        };
        let doc = QueueDoc::from(&core);
        assert_eq!(
            doc,
            QueueDoc {
                version: 4,
                context: ListenContext::Music,
                current: Some(Playing {
                    entry: queue_entry(1),
                    lane: QueueLane::UpNext,
                }),
                position_ms: Some(1_500),
                up_next: vec![queue_entry(2)],
                cursor: 1,
                from: FromLaneMirror {
                    source: Some(album(1)),
                    before: vec![queue_entry(3)],
                    upcoming: vec![queue_entry(4)],
                },
                continue_with: vec![queue_entry(5)],
                repeat: RepeatMode::All,
                stop_after: StopAfterMode::Source,
                player: Some(session(1)),
            }
        );
        let roundtrip = super::try_queue(doc.clone()).expect("canonical identifiers");
        assert_eq!(QueueDoc::from(&roundtrip), doc);
        assert_eq!(QueueEntry::from(core_entry(0xab)).id, entry_id(0xab));
    }

    #[test]
    fn indices_that_do_not_fit_in_u32_saturate_and_hex_roundtrips() {
        let mixed = [
            0x0a, 0x1b, 0x2c, 0x3d, 0x4e, 0x5f, 0x60, 0x71, 0x82, 0x93, 0xa4, 0xb5, 0xc6, 0xd7,
            0xe8, 0xf9,
        ];
        assert_eq!(
            super::entry_hex(EntryId::new(mixed)),
            "0a1b2c3d4e5f60718293a4b5c6d7e8f9"
        );
        assert_eq!(
            super::parse_entry("0a1b2c3d4e5f60718293a4b5c6d7e8f9").expect("lower-case hex"),
            EntryId::new(mixed)
        );
        assert_eq!(super::index_u32(0), 0);
        assert_eq!(super::index_u32(u32::MAX as usize), u32::MAX);
        assert_eq!(super::index_u32(usize::MAX), u32::MAX);
        let mut wide = Queue::new(Context::Music);
        wide.cursor = usize::MAX;
        assert_eq!(QueueDoc::from(&wide).cursor, u32::MAX);
        let source = Source(PublicId::parse(&album(1), IdKind::Album).expect("canonical"));
        assert_eq!(
            QueueOp::from(&Op::Play {
                source,
                items: vec![NewEntry {
                    id: EntryId::new([1; 16]),
                    item: PublicId::parse(&track(1), IdKind::Track).expect("canonical"),
                }],
                start: usize::MAX,
                by: PublicId::parse(&session(1), IdKind::Device).expect("canonical"),
            }),
            play(u32::MAX, &[1])
        );
        assert_eq!(
            QueueError::from(gunmetal_core::queue::QueueReject::StartOutOfRange {
                start: usize::MAX,
                items: usize::MAX,
            }),
            QueueError::StartOutOfRange {
                start: u32::MAX,
                items: u32::MAX,
            }
        );
    }

    /// A number or identifier the core refuses is a typed conversion error.
    ///
    /// Verifies: SEC-CLI-021
    #[test]
    fn a_malformed_queue_field_is_unusable() {
        let mut doc = empty_doc();
        doc.position_ms = Some(2_592_000_001);
        refuse_clear(doc);
        doc = empty_doc();
        doc.current = Some(Playing {
            entry: QueueEntry {
                id: entry_id(1),
                item: "not-an-id".to_owned(),
                source: album(1),
            },
            lane: QueueLane::From,
        });
        refuse_clear(doc);
        doc = empty_doc();
        doc.up_next = vec![QueueEntry {
            id: "gg".repeat(16),
            item: track(1),
            source: album(1),
        }];
        refuse_clear(doc);
        doc = empty_doc();
        doc.from.source = Some("not-an-id".to_owned());
        refuse_clear(doc);
        doc = empty_doc();
        doc.from.before = vec![queue_entry(1)];
        doc.from.before[0].id = "AA".repeat(16);
        refuse_clear(doc);
        doc = empty_doc();
        doc.from.upcoming = vec![queue_entry(1)];
        doc.from.upcoming[0].source = "not-an-id".to_owned();
        refuse_clear(doc);
        doc = empty_doc();
        doc.continue_with = vec![queue_entry(1)];
        doc.continue_with[0].item = "not-an-id".to_owned();
        refuse_clear(doc);
        doc = empty_doc();
        doc.player = Some("not-an-id".to_owned());
        refuse_clear(doc);
    }

    #[test]
    fn entry_hex_is_thirty_two_lower_case_digits() {
        assert_eq!(super::parse_entry(""), Err(QueueError::Unusable));
        assert_eq!(
            super::parse_entry(&"0".repeat(31)),
            Err(QueueError::Unusable)
        );
        assert_eq!(
            super::parse_entry(&"0".repeat(33)),
            Err(QueueError::Unusable)
        );
        assert_eq!(
            super::parse_entry(&"0".repeat(32)),
            Ok(EntryId::new([0; 16]))
        );
        assert_eq!(super::nibble(b'0'), Some(0));
        assert_eq!(super::nibble(b'9'), Some(9));
        assert_eq!(super::nibble(b'a'), Some(10));
        assert_eq!(super::nibble(b'f'), Some(15));
        assert_eq!(super::nibble(b'A'), None);
        assert_eq!(super::nibble(b'g'), None);
        assert_eq!(super::nibble(b'/'), None);
        assert_eq!(super::nibble(b':'), None);
        assert_eq!(
            super::parse_entry(&"0g".repeat(16)),
            Err(QueueError::Unusable)
        );
        assert_eq!(super::hex_digit(0), '0');
        assert_eq!(super::hex_digit(9), '9');
        assert_eq!(super::hex_digit(10), 'a');
        assert_eq!(super::hex_digit(15), 'f');
    }

    #[test]
    fn each_operation_with_a_malformed_identifier_is_unusable() {
        let items = vec![NewQueueEntry {
            id: entry_id(1),
            item: track(1),
        }];
        let bad_item = vec![NewQueueEntry {
            id: "zz".repeat(16),
            item: track(1),
        }];
        let ops = [
            QueueOp::Play {
                source: "not-an-id".to_owned(),
                items: items.clone(),
                start: 0,
                by: session(1),
            },
            QueueOp::Play {
                source: album(1),
                items: vec![NewQueueEntry {
                    id: entry_id(1),
                    item: "not-an-id".to_owned(),
                }],
                start: 0,
                by: session(1),
            },
            QueueOp::PlayNext {
                source: "not-an-id".to_owned(),
                items: items.clone(),
            },
            QueueOp::PlayNext {
                source: album(1),
                items: bad_item.clone(),
            },
            QueueOp::Add {
                source: "not-an-id".to_owned(),
                items: items.clone(),
            },
            QueueOp::Add {
                source: album(1),
                items: bad_item.clone(),
            },
            QueueOp::PlayLast {
                source: "not-an-id".to_owned(),
                items: items.clone(),
            },
            QueueOp::PlayLast {
                source: album(1),
                items: bad_item,
            },
            QueueOp::Move {
                entry: "not-hex".to_owned(),
                to: super::MoveTo::End(QueueLane::From),
            },
            QueueOp::Move {
                entry: entry_id(1),
                to: super::MoveTo::Before("not-hex".to_owned()),
            },
            QueueOp::Remove(vec!["not-hex".to_owned()]),
            QueueOp::Skip("not-hex".to_owned()),
            QueueOp::Resume {
                by: "not-an-id".to_owned(),
            },
        ];
        for op in ops {
            assert_eq!(
                queue_apply(ApplyRequest {
                    queue: empty_doc(),
                    based_on: 0,
                    op,
                }),
                ApplyOutcome::Error(QueueError::Unusable)
            );
        }
    }

    #[test]
    fn play_refusals_and_resume_on_empty_are_the_core_s_errors() {
        assert_eq!(
            queue_apply(ApplyRequest {
                queue: empty_doc(),
                based_on: 0,
                op: play(0, &[]),
            }),
            ApplyOutcome::Error(QueueError::NoItems)
        );
        assert_eq!(
            queue_apply(ApplyRequest {
                queue: empty_doc(),
                based_on: 0,
                op: play(2, &[1]),
            }),
            ApplyOutcome::Error(QueueError::StartOutOfRange { start: 2, items: 1 })
        );
        assert_eq!(
            queue_apply(ApplyRequest {
                queue: empty_doc(),
                based_on: 0,
                op: QueueOp::Resume { by: session(1) },
            }),
            ApplyOutcome::Error(QueueError::NothingToPlay)
        );
        let played = must_apply(empty_doc(), 0, play(0, &[1]));
        assert_eq!(
            queue_apply(ApplyRequest {
                queue: played.clone(),
                based_on: 1,
                op: play(0, &[1]),
            }),
            ApplyOutcome::Error(QueueError::DuplicateEntry { entry: entry_id(1) })
        );
        assert_eq!(
            queue_apply(ApplyRequest {
                queue: played.clone(),
                based_on: 1,
                op: QueueOp::Move {
                    entry: entry_id(9),
                    to: super::MoveTo::End(QueueLane::From),
                },
            }),
            ApplyOutcome::Error(QueueError::UnknownEntry { entry: entry_id(9) })
        );
        assert_eq!(
            queue_apply(ApplyRequest {
                queue: played.clone(),
                based_on: 1,
                op: QueueOp::Skip(entry_id(9)),
            }),
            ApplyOutcome::Error(QueueError::NotCurrent { entry: entry_id(9) })
        );
        let mut exhausted = empty_doc();
        exhausted.version = u64::MAX;
        assert_eq!(
            queue_apply(ApplyRequest {
                queue: exhausted,
                based_on: u64::MAX,
                op: QueueOp::SetRepeat(RepeatMode::One),
            }),
            ApplyOutcome::Error(QueueError::VersionExhausted)
        );
    }

    #[test]
    fn play_next_add_move_remove_and_clear_apply_through_the_facade() {
        let played = must_apply(empty_doc(), 0, play(0, &[1, 2]));
        let with_next = must_apply(
            played,
            1,
            QueueOp::PlayNext {
                source: album(1),
                items: new_items(8),
            },
        );
        assert_eq!(with_next.up_next[0].id, entry_id(8));
        let with_add = must_apply(
            with_next,
            2,
            QueueOp::Add {
                source: album(1),
                items: new_items(9),
            },
        );
        assert_eq!(with_add.up_next[1].id, entry_id(9));
        let with_last = must_apply(
            with_add,
            3,
            QueueOp::PlayLast {
                source: album(1),
                items: new_items(7),
            },
        );
        assert_eq!(
            with_last.from.upcoming.last().map(|e| &e.id),
            Some(&entry_id(7))
        );
        let moved = must_apply(
            with_last,
            4,
            QueueOp::Move {
                entry: entry_id(8),
                to: super::MoveTo::End(QueueLane::UpNext),
            },
        );
        assert_eq!(moved.up_next.last().map(|e| &e.id), Some(&entry_id(8)));
        let removed = must_apply(moved, 5, QueueOp::Remove(vec![entry_id(9)]));
        assert!(removed.up_next.iter().all(|e| e.id != entry_id(9)));
        let cleared_up = must_apply(removed, 6, QueueOp::ClearUpNext);
        assert!(cleared_up.up_next.is_empty());
        let repeated = must_apply(cleared_up, 7, QueueOp::SetRepeat(RepeatMode::One));
        assert_eq!(repeated.repeat, RepeatMode::One);
        let stopped = must_apply(repeated, 8, QueueOp::SetStopAfter(StopAfterMode::Item));
        assert_eq!(stopped.stop_after, StopAfterMode::Item);
        let cleared = must_apply(stopped, 9, QueueOp::Clear);
        assert!(cleared.from.upcoming.is_empty());
        assert_eq!(
            cleared.current.as_ref().map(|c| &c.entry.id),
            Some(&entry_id(1))
        );
    }

    #[test]
    fn finished_then_resume_applies_through_the_facade() {
        let finished = must_apply(empty_doc(), 0, play(0, &[1, 2]));
        let after = must_apply(finished, 1, QueueOp::Finished(entry_id(1)));
        assert_eq!(
            after.current.as_ref().map(|c| &c.entry.id),
            Some(&entry_id(2))
        );
        let resumed = must_apply(after, 2, QueueOp::Resume { by: session(2) });
        assert_eq!(resumed.player.as_deref(), Some(session(2).as_str()));
    }

    /// Rebase keeps operations that still fit and drops the rest with the
    /// core's reason; a value the core refuses is Unusable.
    ///
    /// Verifies: SEC-CLI-021
    #[test]
    fn rebase_keeps_what_fits_and_drops_the_rest() {
        let played = must_apply(empty_doc(), 0, play(0, &[1, 2]));
        let rebased = rebase_of(
            vec![
                Pending {
                    based_on: 0,
                    op: QueueOp::SetRepeat(RepeatMode::All),
                },
                Pending {
                    based_on: 0,
                    op: QueueOp::Skip(entry_id(9)),
                },
            ],
            played,
        )
        .unwrap();
        assert_eq!(rebased.queue.repeat, RepeatMode::All);
        assert_eq!(rebased.queue.version, 2);
        assert_eq!(
            rebased.pending,
            vec![Pending {
                based_on: 1,
                op: QueueOp::SetRepeat(RepeatMode::All),
            }]
        );
        assert_eq!(
            rebased.dropped,
            vec![DroppedOp {
                op: QueueOp::Skip(entry_id(9)),
                reason: QueueError::NotCurrent { entry: entry_id(9) },
            }]
        );
        let mut bad = empty_doc();
        bad.player = Some("not-an-id".to_owned());
        assert_eq!(rebase_of(vec![], bad), Err(RebaseOutcome::Unusable));
        assert_eq!(
            rebase_of(
                vec![Pending {
                    based_on: 0,
                    op: QueueOp::Finished("zz".to_owned()),
                }],
                empty_doc(),
            ),
            Err(RebaseOutcome::Unusable)
        );
    }
}
