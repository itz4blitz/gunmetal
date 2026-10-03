//! The identity store: who exists and what each may do, server settings and
//! the public-ID mapping, in `durable/identity.db` (ADR 3, section 2;
//! record 7, section 8).
//!
//! The file is opened only through the one connection opener (WP-126), with
//! `synchronous=FULL` and `secure_delete=ON` on every connection, and
//! statements reach it only as [`Query`] values (SEC-API-066, SEC-TM-039,
//! SEC-PRV-050). It lives in `durable/`, which a cache rebuild never
//! touches (SEC-IAM-004, SEC-TM-051).
//!
//! **Schema.** Each package that keeps rows here registers a
//! [`SchemaPart`] with the data class of every column, and opening compares
//! the columns the file has with those classes (SEC-TM-050). The store
//! records its format, the number of migrations applied, and the parts'
//! digest. Before R1 there are no migrations, so a changed digest is
//! refused with [`SCHEMA_CHANGED`]; from R1 every change is a numbered
//! [`Migration`].
//!
//! **Migrations** follow operations section 8 (SEC-OPS-048): a snapshot of
//! the file in `snapshots/`, checked with `integrity_check`; then, in one
//! transaction, the migrations, the store's own record, the class check,
//! every [`Invariant`] and every declared [`Setting`]; then the commit.
//! Anything that fails rolls the transaction back and leaves the file as it
//! was. A migration may not loosen a security setting, change a privacy
//! choice or drop either (SEC-OPS-049, SEC-PRV-023), and a file from a newer
//! version is refused with the command that restores the snapshot, never
//! discarded (SEC-OPS-051).
//!
//! **Reads.** Every read that returns a user-visible object takes a
//! `Permit` (SEC-TM-024, SEC-API-010). The `Permit` is WP-033's, in the same
//! wave as this package, so the store's general reader is private to this
//! crate, where WP-065 adds the reader that takes one. The only public reads
//! are the pre-principal lookups ([`PrePrincipal`]) and the public-ID
//! mapping, which returns no user-visible object.
//!
//! A migration or write that runs `COMMIT` itself would end the store's
//! transaction early; schema parts and migrations are reviewed code that
//! never does.

use std::cmp::Ordering;
use std::io::{self, Read};
use std::sync::atomic::{self, AtomicUsize};
use std::sync::{Mutex, MutexGuard, PoisonError};

use gunmetal_core::schema::{Column, DataClass, LiveColumn, Schema, SchemaPart};
use gunmetal_fs::dataroot::DataRoot;
use gunmetal_fs::path::DataDir;
use gunmetal_fs::sqlite::{
    Db, DbError, DbFile, Pragmas, Query, Row, Synchronous, Value, open_db, open_untrusted,
};

use crate::identity::columns::columns;
use crate::identity::error::{IdentityError, RESTORE, SCHEMA_CHANGED, Step, failed};
use crate::identity::mapping::PUBLIC_IDS;
use crate::identity::pre_principal::PrePrincipal;
use crate::identity::settings::{self, Setting};

/// The identity store's file.
pub const IDENTITY: DbFile = DbFile::new(DataDir::Durable, "identity.db");

/// The copy of the file taken before the last migration.
pub const SNAPSHOT: DbFile = DbFile::new(DataDir::Snapshots, "identity.db");

/// The writer's connection.
const WRITER: Pragmas = Pragmas::new(Synchronous::Full);

/// Each reader's connection, which refuses every write.
const READER: Pragmas = Pragmas::new(Synchronous::Full)
    .busy_timeout(5_000)
    .query_only();

/// How many reader connections the store keeps.
const READERS: usize = 2;

/// The store's own record: its format and the parts' digest. The rows are
/// nothing anyone reads through the product.
const META: SchemaPart = SchemaPart {
    name: "identity.meta",
    sql: "CREATE TABLE store_meta (\
              singleton INTEGER PRIMARY KEY CHECK (singleton = 1), \
              format INTEGER NOT NULL, \
              digest BLOB NOT NULL);",
    columns: &[
        Column {
            table: "store_meta",
            name: "singleton",
            class: DataClass::Secret,
        },
        Column {
            table: "store_meta",
            name: "format",
            class: DataClass::Secret,
        },
        Column {
            table: "store_meta",
            name: "digest",
            class: DataClass::Secret,
        },
    ],
};

/// The table [`META`] creates.
const META_TABLE: &str = "store_meta";

/// Every table the file has, with its statement.
const TABLES: Query = Query::new(
    "SELECT name, sql FROM sqlite_schema \
     WHERE type = 'table' AND name NOT LIKE 'sqlite\\_%' ESCAPE '\\' ORDER BY name",
);
const READ_META: Query = Query::new("SELECT format, digest FROM store_meta");
const INSERT_META: Query =
    Query::new("INSERT INTO store_meta (singleton, format, digest) VALUES (1, ?1, ?2)");
const UPDATE_META: Query = Query::new("UPDATE store_meta SET format = ?1, digest = ?2");
const CHECKPOINT: Query = Query::new("PRAGMA wal_checkpoint(TRUNCATE)");
const INTEGRITY: Query = Query::new("PRAGMA integrity_check");
const BEGIN: Query = Query::new("BEGIN IMMEDIATE");
const COMMIT: Query = Query::new("COMMIT");
const ROLLBACK: Query = Query::new("ROLLBACK");

/// One numbered migration: it takes a file from the format equal to its
/// place in [`Spec::migrations`] to the next. It runs inside the runner's
/// transaction and must not end it.
pub struct Migration<'a>(pub &'a dyn Fn(&Db) -> Result<(), DbError>);

/// A rule every migrated file must keep, such as "exactly one owner".
#[derive(Debug, Clone, PartialEq)]
pub struct Invariant {
    /// The rule's name, for the error that names it.
    pub name: &'static str,
    /// A query that returns exactly one row holding the integer 1 when the
    /// rule holds.
    pub holds: Query,
}

/// What the binary knows about the store: every package's schema part, the
/// migrations, and the rules a migration must keep.
pub struct Spec<'a> {
    /// The schema parts, in any order.
    pub parts: &'a [SchemaPart],
    /// The migrations, in order; the binary's format is their number.
    pub migrations: &'a [Migration<'a>],
    /// The invariants checked before a migration commits.
    pub invariants: &'a [Invariant],
    /// The security settings and privacy choices a migration must keep.
    pub settings: &'a [Setting],
}

/// What opening did to the file, for the audit log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// It created the store.
    Created,
    /// The file was already in the binary's format.
    Current,
    /// It migrated the file, after taking a snapshot.
    Migrated {
        /// The file's format before.
        from: usize,
        /// The file's format now.
        to: usize,
    },
}

/// An open store and what opening did.
#[derive(Debug)]
pub struct Opened {
    /// The store.
    pub store: IdentityStore,
    /// What opening did.
    pub outcome: Outcome,
}

/// The open identity store: one writer connection and a small pool of
/// read-only connections, each set up by the connection opener.
#[derive(Debug)]
pub struct IdentityStore {
    writer: Mutex<Db>,
    readers: Vec<Mutex<Db>>,
    next: AtomicUsize,
}

/// The store's format and digest as the file records them.
struct Meta {
    format: usize,
    digest: Vec<u8>,
}

impl IdentityStore {
    /// Opens `durable/identity.db` beneath `root`, creating or migrating it
    /// as `spec` requires.
    ///
    /// # Errors
    ///
    /// Returns an [`IdentityError`] when the parts are refused, the file is
    /// not an identity store, is from a newer version, has a changed schema
    /// with no migration, or has columns the parts do not classify; when the
    /// snapshot, a migration, an invariant or a setting check fails, which
    /// leaves the file as it was; or when SQLite fails.
    pub fn open(root: &DataRoot, spec: &Spec<'_>) -> Result<Opened, IdentityError> {
        let parts: Vec<SchemaPart> = [META, PUBLIC_IDS]
            .into_iter()
            .chain(spec.parts.iter().copied())
            .collect();
        let schema = Schema::new(&parts).map_err(IdentityError::Schema)?;
        let writer = open_db(root, &IDENTITY, WRITER).map_err(failed(Step::Open))?;
        let outcome = settle(root, &writer, spec, &schema)?;
        (0..READERS)
            .map(|_| open_db(root, &IDENTITY, READER).map(Mutex::new))
            .collect::<Result<_, _>>()
            .map_err(failed(Step::Open))
            .map(|readers| Opened {
                store: Self {
                    writer: Mutex::new(writer),
                    readers,
                    next: AtomicUsize::new(0),
                },
                outcome,
            })
    }

    /// Runs `statements` in order in one transaction and returns how many
    /// rows each changed. A statement that fails, or returns rows, rolls
    /// back all of them; no write returns what it read.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::Db`] with [`Step::Write`] for the first
    /// statement that fails, or for the commit.
    pub fn write(&self, statements: &[Query]) -> Result<Vec<usize>, IdentityError> {
        let writer = lock(&self.writer);
        transaction(&writer, Step::Write, |db| {
            statements
                .iter()
                .map(|statement| db.execute(statement).map_err(failed(Step::Write)))
                .collect()
        })
    }

    /// The general reader. It is private to this crate: a read that returns
    /// a user-visible object goes through the `Permit`-taking reader
    /// (WP-065), so a handler that has not asked the policy cannot reach the
    /// store (SEC-TM-024, SEC-API-010).
    pub(crate) fn read(&self, query: &Query) -> Result<Vec<Row>, IdentityError> {
        self.read_as(Step::Read, query)
    }

    /// Runs `query` for `lookup`, one of the three reads made before there
    /// is a principal. A `disallowed-methods` entry confines each caller to
    /// the server module that owns its lookup.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::Db`] with [`Step::PrePrincipal`] when SQLite
    /// fails or refuses the query, which includes any write.
    pub fn read_pre_principal(
        &self,
        lookup: PrePrincipal,
        query: &Query,
    ) -> Result<Vec<Row>, IdentityError> {
        self.read_as(Step::PrePrincipal(lookup), query)
    }

    /// Runs `query` on the next reader connection in turn.
    fn read_as(&self, step: Step, query: &Query) -> Result<Vec<Row>, IdentityError> {
        let turn = self.next.fetch_add(1, atomic::Ordering::Relaxed);
        lock(&self.readers[turn % self.readers.len()])
            .query(query)
            .map_err(failed(step))
    }
}

/// Locks a connection. A panic while another thread held it cannot have
/// left a transaction open, since [`transaction`] rolls back as it unwinds,
/// so the connection is still sound.
fn lock(db: &Mutex<Db>) -> MutexGuard<'_, Db> {
    db.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Rolls back whatever transaction is open when dropped. After a commit
/// there is none, and SQLite's refusal is ignored.
struct Rollback<'a>(&'a Db);

impl Drop for Rollback<'_> {
    fn drop(&mut self) {
        let _ = self.0.execute(&ROLLBACK);
    }
}

/// Runs `work` in one write transaction, committing only if it succeeds.
fn transaction<R>(
    db: &Db,
    step: Step,
    work: impl FnOnce(&Db) -> Result<R, IdentityError>,
) -> Result<R, IdentityError> {
    db.execute(&BEGIN).map_err(failed(step))?;
    let _rollback = Rollback(db);
    let value = work(db)?;
    db.execute(&COMMIT).map_err(failed(step))?;
    Ok(value)
}

/// Creates, migrates or checks the file the writer has open.
fn settle(
    root: &DataRoot,
    db: &Db,
    spec: &Spec<'_>,
    schema: &Schema,
) -> Result<Outcome, IdentityError> {
    let version = spec.migrations.len();
    db.query(&TABLES)
        .map_err(failed(Step::Open))
        .and_then(|tables| {
            if tables.is_empty() {
                create(db, schema, version).map(|()| Outcome::Created)
            } else {
                read_meta(db, &tables)
                    .and_then(|meta| settle_existing(root, db, spec, schema, &meta))
            }
        })
}

/// Checks or migrates a file whose record is `meta`.
fn settle_existing(
    root: &DataRoot,
    db: &Db,
    spec: &Spec<'_>,
    schema: &Schema,
    meta: &Meta,
) -> Result<Outcome, IdentityError> {
    let version = spec.migrations.len();
    match meta.format.cmp(&version) {
        Ordering::Greater => Err(IdentityError::NewerFormat {
            found: meta.format,
            supported: version,
            restore: RESTORE,
        }),
        Ordering::Less => {
            migrate(root, db, spec, schema, meta.format)?;
            Ok(Outcome::Migrated {
                from: meta.format,
                to: version,
            })
        }
        Ordering::Equal if meta.digest == schema.digest() => {
            check_classes(db, schema, Step::Open)?;
            Ok(Outcome::Current)
        }
        Ordering::Equal => Err(IdentityError::SchemaChanged {
            advice: SCHEMA_CHANGED,
        }),
    }
}

/// The format as SQLite stores it. A format is a count of migrations, far
/// below `i64::MAX`.
fn format_value(format: usize) -> Value {
    Value::Integer(i64::try_from(format).unwrap_or(i64::MAX))
}

/// Creates every part's tables and the store's record in a new file.
fn create(db: &Db, schema: &Schema, version: usize) -> Result<(), IdentityError> {
    transaction(db, Step::Create, |db| {
        for part in schema.parts() {
            db.execute_batch(part.sql).map_err(failed(Step::Create))?;
        }
        db.execute(
            &INSERT_META
                .bind(format_value(version))
                .bind(Value::Blob(schema.digest().to_vec())),
        )
        .map_err(failed(Step::Create))
        .and_then(|_| check_classes(db, schema, Step::Create))
    })
}

/// Reads the store's record from a file that has `tables`.
fn read_meta(db: &Db, tables: &[Row]) -> Result<Meta, IdentityError> {
    let ours = Value::Text(META_TABLE.to_owned());
    if !tables
        .iter()
        .any(|Row(values)| values.first() == Some(&ours))
    {
        return Err(IdentityError::Foreign);
    }
    db.query(&READ_META)
        .map_err(failed(Step::Open))
        .and_then(parse_meta)
}

/// The record, which must be one row of a non-negative format and a digest.
fn parse_meta(found: Vec<Row>) -> Result<Meta, IdentityError> {
    let meta = match found.as_slice() {
        [Row(values)] => match values.as_slice() {
            [Value::Integer(format), Value::Blob(digest)] => {
                usize::try_from(*format).ok().map(|format| Meta {
                    format,
                    digest: digest.clone(),
                })
            }
            _ => None,
        },
        _ => None,
    };
    meta.ok_or(IdentityError::Meta { found })
}

/// Takes the file from format `from` to the binary's, as the module
/// documentation describes.
fn migrate(
    root: &DataRoot,
    db: &Db,
    spec: &Spec<'_>,
    schema: &Schema,
    from: usize,
) -> Result<(), IdentityError> {
    snapshot(root, db)?;
    let version = spec.migrations.len();
    transaction(db, Step::Migrate, |db| {
        let before = read_settings(db, spec.settings, from)?;
        for migration in spec.migrations.iter().skip(from) {
            (migration.0)(db).map_err(failed(Step::Migrate))?;
        }
        db.execute(
            &UPDATE_META
                .bind(format_value(version))
                .bind(Value::Blob(schema.digest().to_vec())),
        )
        .map_err(failed(Step::Migrate))?;
        check_classes(db, schema, Step::Migrate)?;
        for invariant in spec.invariants {
            let found = db.query(&invariant.holds).map_err(failed(Step::Migrate))?;
            holds(invariant.name, found)?;
        }
        let after = read_settings(db, spec.settings, version)?;
        for ((setting, before), after) in spec.settings.iter().zip(&before).zip(&after) {
            settings::check(setting, before, after).map_err(|problem| IdentityError::Setting {
                setting: setting.name,
                problem,
            })?;
        }
        Ok(())
    })
}

/// Reads every setting a file of `format` holds; a setting newer than the
/// format reads as no rows.
fn read_settings(
    db: &Db,
    settings: &[Setting],
    format: usize,
) -> Result<Vec<Vec<Row>>, IdentityError> {
    settings
        .iter()
        .map(|setting| {
            if format >= setting.since {
                db.query(&setting.read).map_err(failed(Step::Migrate))
            } else {
                Ok(Vec::new())
            }
        })
        .collect()
}

/// Copies the file to [`SNAPSHOT`] and checks the copy. The writer is the
/// only connection while the store opens, so once the log has been moved
/// into the file, the file alone is the whole store.
fn snapshot(root: &DataRoot, db: &Db) -> Result<(), IdentityError> {
    db.query(&CHECKPOINT)
        .map_err(failed(Step::Snapshot))
        .and_then(checkpointed)
        .and_then(|()| {
            root.open_read(IDENTITY.path())
                .map_err(IdentityError::SnapshotFile)
        })
        .and_then(|mut file| {
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)
                .map(|_| bytes)
                .map_err(IdentityError::from)
        })
        .and_then(|bytes| {
            root.replace(SNAPSHOT.path(), &bytes)
                .map_err(IdentityError::SnapshotFile)
        })
        .and_then(|()| open_untrusted(root, &SNAPSHOT).map_err(failed(Step::Snapshot)))
        .and_then(|copy| copy.query(&INTEGRITY).map_err(failed(Step::Snapshot)))
        .and_then(intact)
}

impl From<io::Error> for IdentityError {
    /// Reading the file for the snapshot failed.
    fn from(error: io::Error) -> Self {
        Self::SnapshotRead(error.kind())
    }
}

/// Accepts a checkpoint that moved every frame and emptied the log.
fn checkpointed(report: Vec<Row>) -> Result<(), IdentityError> {
    if report == [Row(vec![Value::Integer(0); 3])] {
        Ok(())
    } else {
        Err(IdentityError::Checkpoint { report })
    }
}

/// Accepts exactly the one-row "ok" of `integrity_check`.
fn intact(report: Vec<Row>) -> Result<(), IdentityError> {
    if report == [Row(vec![Value::Text("ok".to_owned())])] {
        Ok(())
    } else {
        Err(IdentityError::SnapshotIntegrity { report })
    }
}

/// Accepts an invariant whose query returned exactly the integer 1.
fn holds(name: &'static str, found: Vec<Row>) -> Result<(), IdentityError> {
    if found == [Row(vec![Value::Integer(1)])] {
        Ok(())
    } else {
        Err(IdentityError::Invariant { name, found })
    }
}

/// Compares the file's columns with the classes the parts declare.
fn check_classes(db: &Db, schema: &Schema, step: Step) -> Result<(), IdentityError> {
    db.query(&TABLES)
        .map_err(failed(step))
        .and_then(|tables| live_columns(&tables))
        .and_then(|live| schema.check_classes(&live).map_err(IdentityError::Classes))
}

/// The columns of the tables [`TABLES`] listed.
fn live_columns(tables: &[Row]) -> Result<Vec<LiveColumn>, IdentityError> {
    let mut live = Vec::new();
    for row in tables {
        let read = match row.0.as_slice() {
            [Value::Text(table), Value::Text(sql)] => columns(sql).map(|names| (table, names)),
            _ => None,
        };
        let (table, names) =
            read.ok_or_else(|| IdentityError::UnreadableTable { found: row.clone() })?;
        live.extend(names.into_iter().map(|name| LiveColumn {
            table: table.clone(),
            name,
        }));
    }
    Ok(live)
}

/// What code outside this crate must not be able to do with the store.
/// Rustdoc on stable does not check which error a compile-fail test
/// produced, so each shares its imports with the control, which compiles.
#[cfg(doctest)]
mod compile_fail {
    /// Control: outside this crate, a read without a `Permit` names its
    /// pre-principal lookup, and a write takes static queries.
    ///
    /// ```
    /// use gunmetal_durable::identity::error::IdentityError;
    /// use gunmetal_durable::identity::pre_principal::PrePrincipal;
    /// use gunmetal_durable::identity::store::IdentityStore;
    /// use gunmetal_fs::sqlite::{Query, Row};
    ///
    /// fn lookup(store: &IdentityStore, query: &Query) -> Result<Vec<Row>, IdentityError> {
    ///     store.read_pre_principal(PrePrincipal::SessionToken, query)
    /// }
    ///
    /// fn write(store: &IdentityStore) -> Result<Vec<usize>, IdentityError> {
    ///     store.write(&[Query::new("DELETE FROM sessions")])
    /// }
    /// ```
    struct Control;

    /// Verifies: SEC-TM-024, SEC-API-010
    ///
    /// The general reader is not callable from outside the crate, so a
    /// handler cannot read a user-visible object without a `Permit`.
    ///
    /// ```compile_fail,E0624
    /// use gunmetal_durable::identity::error::IdentityError;
    /// use gunmetal_durable::identity::pre_principal::PrePrincipal;
    /// use gunmetal_durable::identity::store::IdentityStore;
    /// use gunmetal_fs::sqlite::{Query, Row};
    ///
    /// fn read(store: &IdentityStore, query: &Query) -> Result<Vec<Row>, IdentityError> {
    ///     store.read(query)
    /// }
    /// ```
    struct NoGeneralReader;

    /// Verifies: SEC-API-066, SEC-TM-039
    ///
    /// A write takes statements only as static queries, never as text.
    ///
    /// ```compile_fail,E0308
    /// use gunmetal_durable::identity::error::IdentityError;
    /// use gunmetal_durable::identity::pre_principal::PrePrincipal;
    /// use gunmetal_durable::identity::store::IdentityStore;
    /// use gunmetal_fs::sqlite::{Query, Row};
    ///
    /// fn write(store: &IdentityStore, name: &str) -> Result<Vec<usize>, IdentityError> {
    ///     store.write(&[format!("DELETE FROM sessions WHERE name = '{name}'")])
    /// }
    /// ```
    struct NoTextStatements;
}

#[cfg(test)]
mod tests {
    use super::*;
    use gunmetal_fs::dataroot::Policy;
    use gunmetal_fs::host::HostFacts;
    use gunmetal_testkit::tempdir::TempDir;

    fn text(value: &str) -> Value {
        Value::Text(value.to_owned())
    }

    /// Verifies: SEC-PRV-050
    #[test]
    fn every_pooled_connection_has_secure_delete_on() {
        let dir = TempDir::new("durable-pool").expect("a temporary directory");
        let host = HostFacts::probe(dir.path()).expect("the host is probed");
        let root = DataRoot::open(dir.path(), &host, Policy::DEFAULT)
            .expect("the data root opens")
            .root;
        let spec = Spec {
            parts: &[],
            migrations: &[],
            invariants: &[],
            settings: &[],
        };
        let store = IdentityStore::open(&root, &spec).expect("opens").store;
        let pragmas = Query::new(
            "SELECT * FROM pragma_secure_delete(), pragma_foreign_keys(), \
             pragma_trusted_schema(), pragma_synchronous(), pragma_query_only()",
        );
        // secure_delete on, foreign_keys on, trusted_schema off, synchronous
        // FULL (2), then query_only: off for the writer, on for each reader.
        let row = |query_only| {
            Row(vec![
                Value::Integer(1),
                Value::Integer(1),
                Value::Integer(0),
                Value::Integer(2),
                Value::Integer(query_only),
            ])
        };
        let pool: Vec<Vec<Row>> = std::iter::once(&store.writer)
            .chain(&store.readers)
            .map(|db| lock(db).query(&pragmas).expect("the pragmas read"))
            .collect();
        assert_eq!(pool, vec![vec![row(0)], vec![row(1)], vec![row(1)]]);
    }

    #[test]
    fn reads_the_store_s_record_only_as_one_row_of_a_format_and_a_digest() {
        let good = || Row(vec![Value::Integer(3), Value::Blob(vec![7, 8])]);
        assert!(matches!(
            parse_meta(vec![good()]),
            Ok(Meta { format: 3, ref digest }) if *digest == [7, 8]
        ));
        for found in [
            vec![],
            vec![good(), good()],
            vec![Row(vec![Value::Integer(3)])],
            vec![Row(vec![text("3"), Value::Blob(vec![7, 8])])],
            vec![Row(vec![Value::Integer(3), text("78")])],
            vec![Row(vec![Value::Integer(-1), Value::Blob(vec![7, 8])])],
        ] {
            assert!(matches!(
                parse_meta(found.clone()),
                Err(IdentityError::Meta { found: ref got }) if *got == found
            ));
        }
    }

    #[test]
    fn lists_the_columns_of_every_table_it_can_read() {
        let tables = [
            Row(vec![text("a"), text("CREATE TABLE a (x, y)")]),
            Row(vec![text("b"), text("CREATE TABLE b (z)")]),
        ];
        let live = |table: &str, name: &str| LiveColumn {
            table: table.to_owned(),
            name: name.to_owned(),
        };
        assert_eq!(
            live_columns(&tables),
            Ok(vec![live("a", "x"), live("a", "y"), live("b", "z")])
        );
        for found in [
            Row(vec![text("v"), text("CREATE VIRTUAL TABLE v USING x")]),
            Row(vec![text("n"), Value::Null]),
            Row(vec![text("n")]),
        ] {
            assert_eq!(
                live_columns(&[tables[0].clone(), found.clone()]),
                Err(IdentityError::UnreadableTable { found })
            );
        }
    }

    #[test]
    fn accepts_only_the_one_ok_row_of_integrity_check() {
        assert_eq!(intact(vec![Row(vec![text("ok")])]), Ok(()));
        for report in [
            vec![],
            vec![Row(vec![text("ok")]), Row(vec![text("ok")])],
            vec![Row(vec![text("*** in database main ***")])],
        ] {
            assert_eq!(
                intact(report.clone()),
                Err(IdentityError::SnapshotIntegrity { report })
            );
        }
    }

    #[test]
    fn accepts_only_a_checkpoint_that_moved_everything() {
        let report = |values: [i64; 3]| vec![Row(values.map(Value::Integer).to_vec())];
        assert_eq!(checkpointed(report([0, 0, 0])), Ok(()));
        for values in [[1, 0, 0], [0, 1, 0], [0, 0, 1], [0, 3, 3]] {
            assert_eq!(
                checkpointed(report(values)),
                Err(IdentityError::Checkpoint {
                    report: report(values)
                })
            );
        }
    }

    #[test]
    fn keeps_the_kind_of_a_failed_read_of_the_file() {
        assert_eq!(
            IdentityError::from(io::Error::from(io::ErrorKind::UnexpectedEof)),
            IdentityError::SnapshotRead(io::ErrorKind::UnexpectedEof)
        );
    }

    #[test]
    fn stores_a_format_as_its_integer() {
        assert_eq!(format_value(0), Value::Integer(0));
        assert_eq!(format_value(41), Value::Integer(41));
    }
}
