//! The queue document: what one profile has queued in one listening
//! context, as every client and the server hold it.
//!
//! The queue has three lanes, always played in this order (MUS-116): **Up
//! next**, the person's own picks; **From**, the album, artist or playlist
//! being played; and **Continue with**, suggestions, which stay empty until
//! library radio fills them in R1.3 (MUS-129). The item playing now is held
//! apart from the lanes, together with the lane it was taken from, so the
//! lanes hold only what is still to come.
//!
//! Every entry carries the source it was chosen from (MUS-123) and an
//! [`EntryId`] that names it within the queue, so the same track can be
//! queued twice and each copy can be moved or removed on its own.
//!
//! Which of the profile's sessions plays the queue is recorded in
//! [`Queue::player`]: Play and Resume set it to the session that issued them,
//! so the last Play wins and every other session pauses when it sees the new
//! version (decision register A-535). It is an opaque identifier, never a
//! credential and never a device name.
//!
//! The fields are public so the server can store and restore a document
//! and a client can show it. Nothing here trusts them: the operations never
//! panic on a document whose fields disagree, such as a cursor past the end
//! of Up next.

use crate::id::PublicId;
use crate::values::Duration;

/// A named listening context: which of a profile's queues a document is
/// (LAT-009). Video (VID-181) and spoken word (LAT-039) join as variants
/// later, each keeping its own queue and place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Context {
    /// Music, the one context R1 plays.
    Music,
}

/// Names one entry within one queue.
///
/// It is 128 bits that the device issuing the operation draws from its
/// CSPRNG, as it does for the ID of a play event, so the device's own copy
/// and the server agree on every entry without a round trip
/// (SEC-API-023). The core only compares entry IDs: an operation that
/// would give two entries one ID is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntryId([u8; 16]);

impl EntryId {
    /// The entry ID made of these random bytes.
    #[must_use]
    pub const fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
}

/// What an entry was chosen from, shown as "Playing from" (MUS-123): the
/// album, artist or playlist, by its public identifier, whose kind tells the
/// client which of them it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Source(pub PublicId);

/// One queued item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// Names this entry within the queue.
    pub id: EntryId,
    /// The item to play, by its public identifier.
    pub item: PublicId,
    /// Where the item was chosen from. It stays with the entry when the
    /// entry moves to another lane (MUS-123).
    pub source: Source,
}

/// A lane a person can put entries into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lane {
    /// The person's own picks.
    UpNext,
    /// The album, artist or playlist being played.
    From,
}

/// The entry playing now, and the lane it was taken from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Current {
    /// The entry.
    pub entry: Entry,
    /// Where it came from. An entry from the From lane goes back to the
    /// lane's start when it finishes, so repeat all can play it again; a
    /// pick from Up next is done once it has played.
    pub lane: Lane,
}

/// The From lane: the album, artist or playlist being played.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FromLane {
    /// What the last Play played, shown in the lane's header. `None` until
    /// the first Play, and after the queue is cleared.
    pub source: Option<Source>,
    /// The lane's entries before the current one, in order. They are not
    /// shown, and no operation can address them, but repeat all and playing
    /// a finished queue again start the lane from them.
    pub before: Vec<Entry>,
    /// The lane's entries still to play, in order.
    pub upcoming: Vec<Entry>,
}

/// The Continue with lane: suggestions that play when the From lane ends.
/// Part of the document from R1, empty until library radio fills it in R1.3
/// (MUS-129).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContinueLane {
    /// The suggestions, in order.
    pub entries: Vec<Entry>,
}

/// What happens when an item ends (MUS-077).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Repeat {
    /// Play the next item; stop at the end of the queue.
    Off,
    /// Play the same item again. Skipping still moves on.
    One,
    /// Play the From lane again from its start once it runs out. Up next
    /// picks still play once.
    All,
}

/// When playback stops of its own accord (MUS-077). The mode applies once
/// and then returns to [`StopAfter::Off`]; skipping an item never triggers
/// it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StopAfter {
    /// Keep playing.
    Off,
    /// Stop when the current item ends.
    Item,
    /// Stop when the From lane's last entry ends: "stop after this album"
    /// while an album plays.
    Source,
}

/// One profile's queue in one listening context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Queue {
    /// How many operations have changed the document. Every operation adds
    /// one, and an operation built on any other version is refused.
    pub version: u64,
    /// The listening context this queue belongs to.
    pub context: Context,
    /// The entry playing now, or paused at [`Queue::position`].
    pub current: Option<Current>,
    /// How far into the current entry its player has reported, or `None`
    /// from its start until the first report.
    pub position: Option<Duration>,
    /// The person's picks, in the order they play.
    pub up_next: Vec<Entry>,
    /// Where in Up next the next "play next" goes: after the last one added
    /// since the current entry started, back at the top when the current
    /// entry changes (MUS-117).
    pub cursor: usize,
    /// The album, artist or playlist being played.
    pub from: FromLane,
    /// Suggestions for when the From lane ends.
    pub continue_with: ContinueLane,
    /// What happens when an item ends.
    pub repeat: Repeat,
    /// When playback stops of its own accord.
    pub stop_after: StopAfter,
    /// The session that last issued Play or Resume, the only one that
    /// should be playing; `None` before the first Play and once playback has
    /// stopped of its own accord. An opaque identifier the server takes from
    /// the request's session, never a credential (A-535).
    pub player: Option<PublicId>,
}

impl Queue {
    /// An empty queue in `context`, at version zero.
    #[must_use]
    pub const fn new(context: Context) -> Self {
        Self {
            version: 0,
            context,
            current: None,
            position: None,
            up_next: Vec::new(),
            cursor: 0,
            from: FromLane {
                source: None,
                before: Vec::new(),
                upcoming: Vec::new(),
            },
            continue_with: ContinueLane {
                entries: Vec::new(),
            },
            repeat: Repeat::Off,
            stop_after: StopAfter::Off,
            player: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_queue_is_empty_at_version_zero() {
        assert_eq!(
            Queue::new(Context::Music),
            Queue {
                version: 0,
                context: Context::Music,
                current: None,
                position: None,
                up_next: vec![],
                cursor: 0,
                from: FromLane {
                    source: None,
                    before: vec![],
                    upcoming: vec![],
                },
                continue_with: ContinueLane { entries: vec![] },
                repeat: Repeat::Off,
                stop_after: StopAfter::Off,
                player: None,
            }
        );
    }
}
