//! One event as a segment holds it (ADR 3, section 4).
//!
//! The writer stamps every event with its stream's next sequence number.
//! The number is not part of the event, which a device authors without one,
//! so it goes in front of the event in the record's payload:
//!
//! ```text
//! sequence: u64 LE   strictly increasing in a stream, never reused
//! event:    the octets `gunmetal_core::userdata::codec` writes
//! ```

use gunmetal_core::userdata::event::Event;

/// One event of a stream, with the sequence number the writer gave it.
///
/// Projections and damage reports refer to a record by its number. Erasure
/// leaves gaps, and no number is used twice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stamped {
    /// The record's sequence number in its stream, from 1.
    pub seq: u64,
    /// The event.
    pub event: Event,
}
