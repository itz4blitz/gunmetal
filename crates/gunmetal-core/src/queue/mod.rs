//! The play queue: a versioned document per profile and listening context,
//! changed only by operations that the client applies optimistically and the
//! server orders (MUS-116 to MUS-119, MUS-122, MUS-123, MUS-077, LAT-009;
//! API-QUE-01 to API-QUE-03).
//!
//! The rules live here once, so every client and the server apply an
//! operation the same way, and a client whose operation lost a race rebuilds
//! its pending operations on the server's version with the same function the
//! server would use.
//!
//! This file is a registry: it holds only module lines and re-exports.

pub mod apply;
pub mod document;
#[cfg(test)]
mod fixtures;
pub mod op;
pub mod position;
#[cfg(test)]
mod properties;
pub mod rebase;

pub use apply::apply;
pub use document::{
    Context, ContinueLane, Current, Entry, EntryId, FromLane, Lane, Queue, Repeat, Source,
    StopAfter,
};
pub use op::{NewEntry, Op, Position, QueueReject};
pub use position::report_position;
pub use rebase::{Dropped, PendingOp, Rebased, rebase};
