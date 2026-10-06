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
    /// An erasure named the household's whole stream. Only a profile's
    /// stream is ever erased whole (ADR 3, sections 3 and 8): what the
    /// household curated stays when an account is deleted. Nothing was
    /// written to the ledger, and the log goes on.
    Household,
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

impl From<io::Error> for LogError {
    /// Keeps the kind of a failed read, write or sync.
    fn from(error: io::Error) -> Self {
        Self::Io(error.kind())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_kind_of_a_failed_read_write_or_sync() {
        assert_eq!(
            LogError::from(io::Error::from(io::ErrorKind::StorageFull)),
            LogError::Io(io::ErrorKind::StorageFull)
        );
        assert_eq!(
            LogError::from(io::Error::from(io::ErrorKind::UnexpectedEof)),
            LogError::Io(io::ErrorKind::UnexpectedEof)
        );
    }
}
