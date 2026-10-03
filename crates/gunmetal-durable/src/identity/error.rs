//! Why the identity store refused an operation.

use std::io;

use gunmetal_core::id::IdError;
use gunmetal_core::schema::{ClassError, SchemaError};
use gunmetal_fs::dataroot::DataRootError;
use gunmetal_fs::sqlite::{DbError, Row};

use crate::identity::pre_principal::PrePrincipal;
use crate::identity::settings::SettingProblem;

/// What the store was doing when SQLite failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Opening a connection or reading what the file holds.
    Open,
    /// Creating the schema in a new file.
    Create,
    /// Taking or checking the snapshot before a migration.
    Snapshot,
    /// Running the migrations and the checks before their commit.
    Migrate,
    /// A write.
    Write,
    /// The crate's general read.
    Read,
    /// One of the reads made before there is a principal.
    PrePrincipal(PrePrincipal),
}

/// The command that puts the snapshot taken before the last migration back
/// in place of the identity store.
pub const RESTORE: &str = "gunmetal snapshot restore identity";

/// What a development build says when the schema changed with no
/// migration. Before R1 there is no released file, so the parts are not
/// migrated (ADR 3, section 2).
pub const SCHEMA_CHANGED: &str = "the identity store's schema changed with no migration; \
     delete the development data directory and start again";

/// Why the identity store refused an operation.
#[derive(Debug, Clone, PartialEq)]
pub enum IdentityError {
    /// SQLite failed or refused a statement.
    Db {
        /// What the store was doing.
        step: Step,
        /// SQLite's error.
        error: DbError,
    },
    /// The schema parts were refused.
    Schema(SchemaError),
    /// The file's columns and the parts' data classes disagree
    /// (SEC-TM-050).
    Classes(ClassError),
    /// A table's statement could not be read, so its columns are unknown.
    UnreadableTable {
        /// The table's row from `sqlite_schema`: its name and statement.
        found: Row,
    },
    /// The file holds tables but is not an identity store.
    Foreign,
    /// The store's own record of its format and schema could not be read.
    Meta {
        /// What was read.
        found: Vec<Row>,
    },
    /// The file was written by a newer version, whose format this binary
    /// does not know. Nothing was changed; [`RESTORE`] puts back the
    /// snapshot taken before that version migrated it (SEC-OPS-051).
    NewerFormat {
        /// The file's format.
        found: usize,
        /// The newest format this binary knows.
        supported: usize,
        /// The command to run.
        restore: &'static str,
    },
    /// The schema parts changed without a migration ([`SCHEMA_CHANGED`]).
    SchemaChanged {
        /// What to do.
        advice: &'static str,
    },
    /// Moving the log into the file before the snapshot did not finish.
    Checkpoint {
        /// What `wal_checkpoint` reported.
        report: Vec<Row>,
    },
    /// Reading or writing a file for the snapshot failed.
    SnapshotFile(DataRootError),
    /// Reading the store's file for the snapshot failed.
    SnapshotRead(io::ErrorKind),
    /// The snapshot failed `integrity_check`, so nothing was migrated
    /// (SEC-OPS-048).
    SnapshotIntegrity {
        /// What `integrity_check` reported.
        report: Vec<Row>,
    },
    /// An invariant failed after the migrations, so they were rolled back.
    Invariant {
        /// The invariant's name.
        name: &'static str,
        /// What its query returned.
        found: Vec<Row>,
    },
    /// A migration loosened, changed or dropped a setting, so it was rolled
    /// back (SEC-OPS-049, SEC-PRV-023, SEC-OPS-051).
    Setting {
        /// The setting's name.
        setting: &'static str,
        /// What was wrong.
        problem: SettingProblem,
    },
    /// A public ID of another kind was given for a mapping.
    WrongKind(IdError),
    /// A stored public-ID mapping could not be read.
    Mapping {
        /// What was read.
        found: Vec<Row>,
    },
}

/// Maps a SQLite error during `step`.
pub(crate) fn failed(step: Step) -> impl FnOnce(DbError) -> IdentityError {
    move |error| IdentityError::Db { step, error }
}
