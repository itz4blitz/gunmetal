//! Literal values for the queue's tests.
//!
//! By convention entry `n` has the entry ID made of the byte `n` and plays
//! track `n`, so an expected queue can be written out by number. Nothing
//! here calls the code under test except the identifier parser, which
//! `id.rs` tests on its own.

use crate::id::{IdKind, PublicId};

use super::document::{
    Context, ContinueLane, Entry, EntryId, FromLane, Queue, Repeat, Source, StopAfter,
};
use super::op::NewEntry;

/// The identifier of kind `kind` whose 26 symbols spell `n` in decimal.
fn public(prefix: &str, kind: IdKind, n: u8) -> PublicId {
    PublicId::parse(&format!("{prefix}{n:026}"), kind).expect("a canonical identifier")
}

/// Track `n`.
pub fn track(n: u8) -> PublicId {
    public("trk_", IdKind::Track, n)
}

/// Album `n` as a source.
pub fn album(n: u8) -> Source {
    Source(public("alb_", IdKind::Album, n))
}

/// Playlist `n` as a source.
pub fn playlist(n: u8) -> Source {
    Source(public("pls_", IdKind::Playlist, n))
}

/// Stands in for the opaque identifier of a profile's session `n`.
pub fn session(n: u8) -> PublicId {
    public("dev_", IdKind::Device, n)
}

/// The ID of entry `n`.
pub fn id(n: u8) -> EntryId {
    EntryId::new([n; 16])
}

/// Entry `n`, chosen from `source`.
pub fn entry(n: u8, source: Source) -> Entry {
    Entry {
        id: id(n),
        item: track(n),
        source,
    }
}

/// Entries numbered `numbers`, all chosen from `source`.
pub fn entries(numbers: &[u8], source: Source) -> Vec<Entry> {
    numbers.iter().map(|&n| entry(n, source)).collect()
}

/// New entries numbered `numbers`, as an operation names them.
pub fn new_entries(numbers: &[u8]) -> Vec<NewEntry> {
    numbers
        .iter()
        .map(|&n| NewEntry {
            id: id(n),
            item: track(n),
        })
        .collect()
}

/// The empty music queue at version zero, written out.
pub fn empty() -> Queue {
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
}
