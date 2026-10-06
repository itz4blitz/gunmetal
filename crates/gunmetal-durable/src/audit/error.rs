//! Why the audit log refused an operation.

use std::io;

use gunmetal_fs::dataroot::DataRootError;
use gunmetal_fs::sqlite::DbError;
use gunmetal_secrets::random::RandomnessUnavailable;
use gunmetal_secrets::root::SecretsError;

/// Why the audit log refused an operation.
#[derive(Debug, Clone, PartialEq)]
pub enum AuditError {
    /// The data root refused to open, create or replace a file.
    Root(DataRootError),
    /// Reading, writing or syncing a file the log had open failed.
    Io(io::ErrorKind),
    /// The address side store refused a statement.
    Db(DbError),
    /// A cryptographic primitive or the root secret could not be used.
    Secrets(SecretsError),
    /// The operating system could not supply a per-record salt.
    Random(RandomnessUnavailable),
    /// A HMAC key the log needed was missing or revoked.
    MacUnavailable,
    /// The disk is full, and this action is not on the recovery list
    /// (SEC-OPS-020).
    DiskFull,
    /// The caller’s `Permit` does not allow this read.
    Denied,
    /// A segment or the head file could not be parsed.
    Corrupt {
        /// Where the damaged record starts, 1-based, or 0 for the head.
        seq: u64,
    },
    /// An earlier operation failed part of the way through.
    Halted,
}

/// The first record at which the hash chain, a checkpoint MAC or the
/// sequence numbers break.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrokenAt {
    /// The sequence number of the first bad record, 1-based.
    pub seq: u64,
}

impl From<DataRootError> for AuditError {
    fn from(error: DataRootError) -> Self {
        Self::Root(error)
    }
}

impl From<io::Error> for AuditError {
    fn from(error: io::Error) -> Self {
        Self::Io(error.kind())
    }
}

impl From<DbError> for AuditError {
    fn from(error: DbError) -> Self {
        Self::Db(error)
    }
}

impl From<SecretsError> for AuditError {
    fn from(error: SecretsError) -> Self {
        Self::Secrets(error)
    }
}

impl From<RandomnessUnavailable> for AuditError {
    fn from(error: RandomnessUnavailable) -> Self {
        Self::Random(error)
    }
}

#[cfg(test)]
mod tests {
    use super::AuditError;
    use gunmetal_secrets::random::RandomnessUnavailable;
    use gunmetal_secrets::root::SecretsError;

    #[test]
    fn maps_secrets_and_randomness() {
        assert_eq!(
            AuditError::from(SecretsError::Malformed),
            AuditError::Secrets(SecretsError::Malformed)
        );
        assert_eq!(
            AuditError::from(RandomnessUnavailable),
            AuditError::Random(RandomnessUnavailable)
        );
        assert_eq!(AuditError::Denied, AuditError::Denied);
        assert_eq!(AuditError::Halted, AuditError::Halted);
        assert_eq!(AuditError::DiskFull, AuditError::DiskFull);
        assert_eq!(AuditError::MacUnavailable, AuditError::MacUnavailable);
        assert_eq!(
            AuditError::Corrupt { seq: 3 },
            AuditError::Corrupt { seq: 3 }
        );
    }
}
