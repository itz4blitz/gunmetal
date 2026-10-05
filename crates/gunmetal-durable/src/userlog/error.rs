//! Why the user log refused an operation.

use std::io;
use std::ops::Range;

use gunmetal_core::parse::ParseFault;
use gunmetal_core::userdata::event::Stream;
use gunmetal_fs::dataroot::{DataRootError, LogDirError};
use gunmetal_fs::path::LogMonth;

/// Why the user log refused an operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogError {
    /// The data root refused to open, create or replace one of the log's
    /// files.
    Root(DataRootError),
    /// The data root refused to list or remove one of the log's
    /// directories.
    Dir(LogDirError),
    /// Reading, writing or syncing a file the log had open failed.
    Io(io::ErrorKind),
    /// A segment holds a record in a format a newer version writes. Nothing
    /// was changed; this version does not start on it (ADR 3, section 9).
    NewerRecord {
        /// The stream the segment belongs to.
        stream: Stream,
        /// The segment's month.
        month: LogMonth,
        /// Where the record starts in the segment.
        offset: usize,
        /// The record format version it carries.
        version: u8,
    },
    /// The erasure ledger cannot be read, so the log cannot tell what was
    /// erased and does not open (ADR 3, section 8).
    Ledger(LedgerFlaw),
    /// An earlier operation failed part of the way through, so what the log
    /// holds in memory may not be what is on disk. It refuses everything
    /// until it is opened again.
    Halted,
    /// The permit was not decided for reading a profile's own data, or the
    /// profile it names has no stream the server knows.
    Denied,
}

/// What is wrong with the erasure ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerFlaw {
    /// Octets before its end that are not a whole record.
    Damaged {
        /// Where they are in the ledger.
        range: Range<usize>,
    },
    /// A whole record that is not an entry this version reads.
    NotAnEntry {
        /// Where it is in the ledger.
        range: Range<usize>,
    },
    /// Reading the ledger spent its step budget.
    Fault(ParseFault),
}
