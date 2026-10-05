//! Reading one segment back (ADR 3, section 4).
//!
//! What comes back from storage is untrusted (boundary TB10): a damaged
//! disk or a restored backup can hold anything. A scan therefore keeps only
//! what is a whole record, of this stream, numbered after the record before
//! it, and reports everything else as [`Damage`] with its byte range.
//! Damage is never repaired in place. The one exception is a torn tail, a
//! record cut off at the end of a stream's newest segment, which was never
//! acknowledged; the scan says where it starts and the log cuts it.
//!
//! A record of another stream is damage, never an event of this one, so a
//! reader of one person's stream cannot be handed another person's event by
//! whatever put it in the file (SEC-TM-024).

use std::ops::Range;

use gunmetal_core::userdata::event::Stream;
use gunmetal_fs::path::LogMonth;

/// A part of a segment that is not a record of its stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Damage {
    /// The stream the segment belongs to.
    pub stream: Stream,
    /// The segment's month.
    pub month: LogMonth,
    /// Where the part is in the segment, in octets.
    pub range: Range<usize>,
    /// The sequence number of the last record of the stream before it, if
    /// there is one. The numbers lost are after this one and before the
    /// next record the stream holds.
    pub after: Option<u64>,
    /// What is wrong with it.
    pub problem: Problem,
}

/// What is wrong with a damaged part of a segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    /// Octets that are not a whole record: a length that cannot be right, a
    /// checksum that does not match, or a part the scan had no steps left
    /// to read.
    Unreadable,
    /// The segment's first record is not the header of its stream and
    /// month.
    Header,
    /// A whole record that does not hold a sequence number and one event.
    NotAnEvent,
    /// A whole record that holds an event of another stream.
    Foreign,
    /// A whole record whose sequence number is not after the one before
    /// it.
    OutOfOrder,
}
