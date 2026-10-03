//! The identity store on real SQLite files in a temporary data root:
//! creating, reopening, migrating and refusing a file, writing and reading
//! it, and keeping public IDs.

use std::cell::Cell;

use gunmetal_core::id::{IdError, IdKind, PublicId};
use gunmetal_core::schema::{ClassError, Column, DataClass, SchemaError, SchemaPart};
use gunmetal_core::time::Timestamp;
use gunmetal_durable::identity::error::{IdentityError, Step};
use gunmetal_durable::identity::mapping::Mapping;
use gunmetal_durable::identity::pre_principal::PrePrincipal;
use gunmetal_durable::identity::settings::{Setting, SettingKind, SettingProblem};
use gunmetal_durable::identity::store::{
    IDENTITY, IdentityStore, Invariant, Migration, Opened, Outcome, SNAPSHOT, Spec,
};
use gunmetal_fs::dataroot::{DataRoot, DataRootError, Item, Op, Policy};
use gunmetal_fs::host::HostFacts;
use gunmetal_fs::path::{DataDir, DataPath};
use gunmetal_fs::sqlite::{
    Db, DbError, DbFile, Pragmas, Query, Row, Synchronous, Value, open_db, open_untrusted,
};
use gunmetal_testkit::tempdir::TempDir;
use proptest::prelude::*;

/// A temporary data directory and its open root.
struct Data {
    /// Kept so the directory lives as long as the root.
    _dir: TempDir,
    root: DataRoot,
}

fn data() -> Data {
    let dir = TempDir::new("durable").expect("a temporary directory");
    let host = HostFacts::probe(dir.path()).expect("the host is probed");
    let root = DataRoot::open(dir.path(), &host, Policy::DEFAULT)
        .expect("the data root opens")
        .root;
    Data { _dir: dir, root }
}

/// A package's accounts, as first released.
const ACCOUNTS: SchemaPart = SchemaPart {
    name: "test.accounts",
    sql: "CREATE TABLE accounts (id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
    columns: &[
        Column {
            table: "accounts",
            name: "id",
            class: DataClass::Identity,
        },
        Column {
            table: "accounts",
            name: "name",
            class: DataClass::Identity,
        },
    ],
};

/// The same accounts after a release added a display name.
const ACCOUNTS_V1: SchemaPart = SchemaPart {
    name: "test.accounts",
    sql: "CREATE TABLE accounts (id INTEGER PRIMARY KEY, name TEXT NOT NULL, display TEXT);",
    columns: &[
        Column {
            table: "accounts",
            name: "id",
            class: DataClass::Identity,
        },
        Column {
            table: "accounts",
            name: "name",
            class: DataClass::Identity,
        },
        Column {
            table: "accounts",
            name: "display",
            class: DataClass::Identity,
        },
    ],
};

/// A security posture and a privacy choice per holder.
const SETTINGS: SchemaPart = SchemaPart {
    name: "test.settings",
    sql: "CREATE TABLE settings (holder TEXT PRIMARY KEY, posture INTEGER NOT NULL, \
          sharing INTEGER NOT NULL);",
    columns: &[
        Column {
            table: "settings",
            name: "holder",
            class: DataClass::Identity,
        },
        Column {
            table: "settings",
            name: "posture",
            class: DataClass::Identity,
        },
        Column {
            table: "settings",
            name: "sharing",
            class: DataClass::Identity,
        },
    ],
};

const POSTURE: Setting = Setting {
    name: "posture",
    kind: SettingKind::Security,
    since: 0,
    read: Query::new("SELECT holder, posture FROM settings ORDER BY holder"),
    order: &[0, 5, 9],
};

const SHARING: Setting = Setting {
    name: "sharing",
    kind: SettingKind::Privacy,
    since: 0,
    read: Query::new("SELECT holder, sharing FROM settings ORDER BY holder"),
    order: &[1, 0],
};

const ADD_ACCOUNT: Query = Query::new("INSERT INTO accounts (name) VALUES (?1)");
const ACCOUNT_NAMES: Query = Query::new("SELECT name FROM accounts ORDER BY id");
const ADD_SETTING: Query =
    Query::new("INSERT INTO settings (holder, posture, sharing) VALUES (?1, ?2, ?3)");
const TABLES: Query =
    Query::new("SELECT name FROM sqlite_schema WHERE type = 'table' ORDER BY name");
const FORMAT: Query = Query::new("SELECT format FROM store_meta");

const AN_ACCOUNT_EXISTS: Invariant = Invariant {
    name: "an account exists",
    holds: Query::new("SELECT count(*) >= 1 FROM accounts"),
};

fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}

fn row(values: &[Value]) -> Row {
    Row(values.to_vec())
}

fn names(list: &[&str]) -> Vec<Row> {
    list.iter().map(|name| row(&[text(name)])).collect()
}

fn add_account(name: &str) -> Query {
    ADD_ACCOUNT.bind(text(name))
}

fn add_setting(holder: &str, posture: i64, sharing: i64) -> Query {
    ADD_SETTING
        .bind(text(holder))
        .bind(Value::Integer(posture))
        .bind(Value::Integer(sharing))
}

fn spec<'a>(parts: &'a [SchemaPart], migrations: &'a [Migration<'a>]) -> Spec<'a> {
    Spec {
        parts,
        migrations,
        invariants: &[],
        settings: &[],
    }
}

fn open(root: &DataRoot, spec: &Spec<'_>) -> Result<Opened, IdentityError> {
    IdentityStore::open(root, spec)
}

fn store(root: &DataRoot, spec: &Spec<'_>) -> IdentityStore {
    open(root, spec).expect("the store opens").store
}

/// Reads through the pre-principal door, which every caller outside the
/// crate has.
fn read(store: &IdentityStore, query: &Query) -> Result<Vec<Row>, IdentityError> {
    store.read_pre_principal(PrePrincipal::Credential, query)
}

fn outcome(root: &DataRoot, spec: &Spec<'_>) -> Result<Outcome, IdentityError> {
    open(root, spec).map(|opened| opened.outcome)
}

fn add_display(db: &Db) -> Result<(), DbError> {
    db.execute_batch("ALTER TABLE accounts ADD COLUMN display TEXT;")
}

/// Creates a store at format 0 with two accounts and the given settings.
fn released(root: &DataRoot, parts: &[SchemaPart], settings: &[Query]) {
    let store = store(root, &spec(parts, &[]));
    store
        .write(&[add_account("sam"), add_account("kim")])
        .expect("the accounts are written");
    store.write(settings).expect("the settings are written");
}

/// The first bytes of a SQLite file.
const SQLITE_HEADER: &[u8] = b"SQLite format 3\0";

fn file_bytes(root: &DataRoot, file: &DbFile) -> Vec<u8> {
    use std::io::Read;
    let mut bytes = Vec::new();
    root.open_read(file.path())
        .expect("the file opens")
        .read_to_end(&mut bytes)
        .expect("the file reads");
    bytes
}

#[test]
fn creates_the_store_in_the_durable_directory_and_reopens_it_as_current() {
    let data = data();
    let parts = [ACCOUNTS];
    assert_eq!(
        outcome(&data.root, &spec(&parts, &[])),
        Ok(Outcome::Created)
    );
    assert_eq!(IDENTITY, DbFile::new(DataDir::Durable, "identity.db"));
    assert!(file_bytes(&data.root, &IDENTITY).starts_with(SQLITE_HEADER));
    let store = store(&data.root, &spec(&parts, &[]));
    assert_eq!(
        read(&store, &TABLES),
        Ok(names(&["accounts", "public_ids", "store_meta"]))
    );
    drop(store);
    assert_eq!(
        outcome(&data.root, &spec(&parts, &[])),
        Ok(Outcome::Current)
    );
}

#[test]
fn creates_a_new_file_at_the_binary_s_format_without_running_migrations() {
    let data = data();
    let ran = Cell::new(0);
    let counted = |_: &Db| -> Result<(), DbError> {
        ran.set(ran.get() + 1);
        Ok(())
    };
    let migrations = [Migration(&counted), Migration(&counted)];
    let store = store(&data.root, &spec(&[ACCOUNTS], &migrations));
    assert_eq!(ran.get(), 0);
    assert_eq!(read(&store, &FORMAT), Ok(vec![row(&[Value::Integer(2)])]));
}

#[test]
fn writes_statements_in_order_and_returns_what_each_changed() {
    let data = data();
    let store = store(&data.root, &spec(&[ACCOUNTS], &[]));
    assert_eq!(
        store.write(&[
            add_account("sam"),
            add_account("kim"),
            Query::new("UPDATE accounts SET name = name || '!'"),
        ]),
        Ok(vec![1, 1, 2])
    );
    assert_eq!(read(&store, &ACCOUNT_NAMES), Ok(names(&["sam!", "kim!"])));
}

#[test]
fn a_failed_write_changes_nothing_and_leaves_the_writer_usable() {
    let data = data();
    let store = store(&data.root, &spec(&[ACCOUNTS], &[]));
    let null_name = Query::new("INSERT INTO accounts (name) VALUES (NULL)");
    assert_eq!(
        store.write(&[add_account("sam"), null_name]),
        Err(IdentityError::Db {
            step: Step::Write,
            // SQLITE_CONSTRAINT_NOTNULL
            error: DbError::Sqlite { code: 1299 },
        })
    );
    assert_eq!(
        store.write(&[add_account("kim"), ACCOUNT_NAMES]),
        Err(IdentityError::Db {
            step: Step::Write,
            error: DbError::ReturnedRows,
        })
    );
    assert_eq!(store.write(&[add_account("lee")]), Ok(vec![1]));
    assert_eq!(read(&store, &ACCOUNT_NAMES), Ok(names(&["lee"])));
}

#[test]
fn a_write_waiting_on_another_writer_fails_at_once() {
    let data = data();
    let store = store(&data.root, &spec(&[ACCOUNTS], &[]));
    let other = open_db(&data.root, &IDENTITY, Pragmas::new(Synchronous::Full))
        .expect("a second connection opens");
    other
        .execute(&Query::new("BEGIN IMMEDIATE"))
        .expect("the second connection takes the write lock");
    assert_eq!(
        store.write(&[add_account("sam")]),
        Err(IdentityError::Db {
            step: Step::Write,
            // SQLITE_BUSY
            error: DbError::Sqlite { code: 5 },
        })
    );
    other.execute(&Query::new("ROLLBACK")).expect("rolls back");
    assert_eq!(store.write(&[add_account("sam")]), Ok(vec![1]));
}

#[test]
fn a_write_the_commit_refuses_changes_nothing() {
    const DEFERRED: SchemaPart = SchemaPart {
        name: "test.devices",
        sql: "CREATE TABLE devices (id INTEGER PRIMARY KEY, \
              account INTEGER NOT NULL REFERENCES accounts (id) DEFERRABLE INITIALLY DEFERRED);",
        columns: &[
            Column {
                table: "devices",
                name: "id",
                class: DataClass::Identity,
            },
            Column {
                table: "devices",
                name: "account",
                class: DataClass::Identity,
            },
        ],
    };
    let data = data();
    let store = store(&data.root, &spec(&[ACCOUNTS, DEFERRED], &[]));
    let orphan = Query::new("INSERT INTO devices (account) VALUES (42)");
    assert_eq!(
        store.write(&[add_account("sam"), orphan]),
        Err(IdentityError::Db {
            step: Step::Write,
            // SQLITE_CONSTRAINT_FOREIGNKEY
            error: DbError::Sqlite { code: 787 },
        })
    );
    assert_eq!(read(&store, &ACCOUNT_NAMES), Ok(vec![]));
    assert_eq!(store.write(&[add_account("kim")]), Ok(vec![1]));
}

#[test]
fn a_pre_principal_read_names_its_lookup_and_cannot_write() {
    let data = data();
    let store = store(&data.root, &spec(&[ACCOUNTS], &[]));
    store.write(&[add_account("sam")]).expect("written");
    for lookup in [
        PrePrincipal::SessionToken,
        PrePrincipal::Credential,
        PrePrincipal::Grant,
    ] {
        assert_eq!(
            store.read_pre_principal(lookup, &ACCOUNT_NAMES),
            Ok(names(&["sam"]))
        );
        assert_eq!(
            store.read_pre_principal(lookup, &add_account("kim")),
            Err(IdentityError::Db {
                step: Step::PrePrincipal(lookup),
                // SQLITE_READONLY
                error: DbError::Sqlite { code: 8 },
            })
        );
    }
    assert_eq!(read(&store, &ACCOUNT_NAMES), Ok(names(&["sam"])));
}

/// Verifies: SEC-PRV-050
#[test]
fn every_read_of_the_pool_has_secure_delete_on_and_sees_committed_writes() {
    let data = data();
    let store = store(&data.root, &spec(&[ACCOUNTS], &[]));
    let pragmas = Query::new(
        "SELECT * FROM pragma_secure_delete(), pragma_synchronous(), pragma_query_only()",
    );
    for turn in 0..9 {
        store.write(&[add_account("sam")]).expect("written");
        // secure_delete on, synchronous FULL (2), query_only on.
        assert_eq!(
            read(&store, &pragmas),
            Ok(vec![row(&[
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(1)
            ])])
        );
        assert_eq!(
            read(&store, &Query::new("SELECT count(*) FROM accounts")),
            Ok(vec![row(&[Value::Integer(turn + 1)])])
        );
    }
}

#[test]
fn refuses_parts_that_clash_with_the_store_s_own() {
    const CLASH: SchemaPart = SchemaPart {
        name: "identity.meta",
        sql: "CREATE TABLE clash (a);",
        columns: &[],
    };
    let data = data();
    assert_eq!(
        outcome(&data.root, &spec(&[CLASH], &[])),
        Err(IdentityError::Schema(SchemaError::DuplicateName {
            name: "identity.meta"
        }))
    );
}

#[test]
fn refuses_a_file_that_is_not_a_database() {
    let data = data();
    let garbage = [0x5a_u8; 4096];
    data.root
        .replace(IDENTITY.path(), &garbage)
        .expect("the file is written");
    assert_eq!(
        outcome(&data.root, &spec(&[ACCOUNTS], &[])),
        Err(IdentityError::Db {
            step: Step::Open,
            // SQLITE_NOTADB
            error: DbError::Sqlite { code: 26 },
        })
    );
    assert_eq!(file_bytes(&data.root, &IDENTITY), garbage);
}

#[test]
fn refuses_a_database_that_is_not_an_identity_store_and_leaves_it_alone() {
    let data = data();
    let other = open_db(&data.root, &IDENTITY, Pragmas::new(Synchronous::Full)).expect("opens");
    other
        .execute_batch("CREATE TABLE accounts (id INTEGER PRIMARY KEY, name TEXT NOT NULL);")
        .expect("created");
    other.execute(&add_account("sam")).expect("written");
    assert_eq!(
        outcome(&data.root, &spec(&[ACCOUNTS], &[])),
        Err(IdentityError::Foreign)
    );
    assert_eq!(other.query(&ACCOUNT_NAMES), Ok(names(&["sam"])));
}

#[test]
fn refuses_a_store_whose_own_record_is_unreadable() {
    let data = data();
    let store = store(&data.root, &spec(&[ACCOUNTS], &[]));
    store
        .write(&[Query::new(
            "UPDATE store_meta SET format = -1, digest = x'0102'",
        )])
        .expect("written");
    drop(store);
    assert_eq!(
        outcome(&data.root, &spec(&[ACCOUNTS], &[])),
        Err(IdentityError::Meta {
            found: vec![row(&[Value::Integer(-1), Value::Blob(vec![1, 2])])]
        })
    );
}

#[test]
fn refuses_a_schema_that_changed_without_a_migration() {
    let data = data();
    released(&data.root, &[ACCOUNTS], &[]);
    assert_eq!(
        outcome(&data.root, &spec(&[ACCOUNTS_V1], &[])),
        Err(IdentityError::SchemaChanged {
            advice: "the identity store's schema changed with no migration; \
                     delete the development data directory and start again",
        })
    );
    let store = store(&data.root, &spec(&[ACCOUNTS], &[]));
    assert_eq!(read(&store, &ACCOUNT_NAMES), Ok(names(&["sam", "kim"])));
}

#[test]
fn refuses_a_part_whose_statement_fails_and_creates_nothing() {
    const BROKEN: SchemaPart = SchemaPart {
        name: "test.broken",
        sql: "CREATE TABLE broken (a INTEGER) NONSENSE;",
        columns: &[],
    };
    let data = data();
    assert_eq!(
        outcome(&data.root, &spec(&[ACCOUNTS, BROKEN], &[])),
        Err(IdentityError::Db {
            step: Step::Create,
            // SQLITE_ERROR
            error: DbError::Sqlite { code: 1 },
        })
    );
    assert_eq!(
        outcome(&data.root, &spec(&[ACCOUNTS], &[])),
        Ok(Outcome::Created)
    );
}

/// Verifies: SEC-TM-050
#[test]
fn refuses_to_create_a_column_no_part_classifies() {
    const UNDECLARED: SchemaPart = SchemaPart {
        name: "test.accounts",
        sql: ACCOUNTS_V1.sql,
        columns: ACCOUNTS.columns,
    };
    let data = data();
    assert_eq!(
        outcome(&data.root, &spec(&[UNDECLARED], &[])),
        Err(IdentityError::Classes(ClassError::Unclassified {
            table: "accounts".to_owned(),
            column: "display".to_owned(),
        }))
    );
    assert_eq!(
        outcome(&data.root, &spec(&[ACCOUNTS], &[])),
        Ok(Outcome::Created)
    );
}

#[test]
fn refuses_a_part_that_classifies_a_column_it_does_not_create() {
    const PHANTOM: SchemaPart = SchemaPart {
        name: "test.accounts",
        sql: ACCOUNTS.sql,
        columns: ACCOUNTS_V1.columns,
    };
    let data = data();
    assert_eq!(
        outcome(&data.root, &spec(&[PHANTOM], &[])),
        Err(IdentityError::Classes(ClassError::Missing {
            part: "test.accounts",
            table: "accounts",
            column: "display",
        }))
    );
}

/// Verifies: SEC-TM-050
#[test]
fn refuses_to_open_a_file_with_a_column_no_part_classifies() {
    let data = data();
    let store = store(&data.root, &spec(&[ACCOUNTS], &[]));
    store
        .write(&[Query::new("ALTER TABLE accounts ADD COLUMN note TEXT")])
        .expect("altered");
    drop(store);
    assert_eq!(
        outcome(&data.root, &spec(&[ACCOUNTS], &[])),
        Err(IdentityError::Classes(ClassError::Unclassified {
            table: "accounts".to_owned(),
            column: "note".to_owned(),
        }))
    );
}

/// Verifies: SEC-OPS-051
#[test]
fn refuses_a_file_from_a_newer_version_with_the_restore_command() {
    let data = data();
    let no_migration = |_: &Db| -> Result<(), DbError> { Ok(()) };
    let newer = [Migration(&no_migration), Migration(&add_display)];
    let store = store(&data.root, &spec(&[ACCOUNTS_V1], &newer));
    store.write(&[add_account("sam")]).expect("written");
    drop(store);
    let before = file_bytes(&data.root, &IDENTITY);
    assert_eq!(
        outcome(&data.root, &spec(&[ACCOUNTS], &newer[..1])),
        Err(IdentityError::NewerFormat {
            found: 2,
            supported: 1,
            restore: "gunmetal snapshot restore identity",
        })
    );
    assert_eq!(file_bytes(&data.root, &IDENTITY), before);
    let store = self::store(&data.root, &spec(&[ACCOUNTS_V1], &newer));
    assert_eq!(read(&store, &ACCOUNT_NAMES), Ok(names(&["sam"])));
}

/// Verifies: SEC-OPS-048
#[test]
fn migrates_after_a_snapshot_that_passes_the_integrity_check() {
    let data = data();
    released(&data.root, &[ACCOUNTS], &[]);
    let fill = |db: &Db| {
        add_display(db)?;
        db.execute(&Query::new("UPDATE accounts SET display = upper(name)"))
            .map(drop)
    };
    let migrations = [Migration(&fill)];
    let opened = open(&data.root, &spec(&[ACCOUNTS_V1], &migrations)).expect("migrates");
    assert_eq!(opened.outcome, Outcome::Migrated { from: 0, to: 1 });
    assert_eq!(
        read(
            &opened.store,
            &Query::new("SELECT name, display FROM accounts ORDER BY id")
        ),
        Ok(vec![
            row(&[text("sam"), text("SAM")]),
            row(&[text("kim"), text("KIM")]),
        ])
    );
    assert_eq!(
        read(&opened.store, &FORMAT),
        Ok(vec![row(&[Value::Integer(1)])])
    );
    drop(opened);

    assert_eq!(SNAPSHOT, DbFile::new(DataDir::Snapshots, "identity.db"));
    let snapshot = open_untrusted(&data.root, &SNAPSHOT).expect("the snapshot opens");
    assert_eq!(
        snapshot.query(&Query::new("PRAGMA integrity_check")),
        Ok(vec![row(&[text("ok")])])
    );
    assert_eq!(snapshot.query(&FORMAT), Ok(vec![row(&[Value::Integer(0)])]));
    assert_eq!(
        snapshot.query(&Query::new("SELECT * FROM accounts ORDER BY id")),
        Ok(vec![
            row(&[Value::Integer(1), text("sam")]),
            row(&[Value::Integer(2), text("kim")]),
        ])
    );
    assert_eq!(
        outcome(&data.root, &spec(&[ACCOUNTS_V1], &migrations)),
        Ok(Outcome::Current)
    );
}

#[test]
fn runs_only_the_migrations_the_file_has_not_had() {
    let data = data();
    let ran = Cell::new(Vec::new());
    let first = |_: &Db| -> Result<(), DbError> {
        let mut log = ran.take();
        log.push(1);
        ran.set(log);
        Ok(())
    };
    let second = |_: &Db| -> Result<(), DbError> {
        let mut log = ran.take();
        log.push(2);
        ran.set(log);
        Ok(())
    };
    let third = |_: &Db| -> Result<(), DbError> {
        let mut log = ran.take();
        log.push(3);
        ran.set(log);
        Ok(())
    };
    let all = [Migration(&first), Migration(&second), Migration(&third)];
    drop(store(&data.root, &spec(&[ACCOUNTS], &all[..1])));
    assert_eq!(
        outcome(&data.root, &spec(&[ACCOUNTS], &all)),
        Ok(Outcome::Migrated { from: 1, to: 3 })
    );
    assert_eq!(ran.take(), vec![2, 3]);
}

/// Verifies: SEC-OPS-048
#[test]
fn a_failed_migration_leaves_the_file_as_it_was() {
    let data = data();
    released(&data.root, &[ACCOUNTS], &[]);
    let before = file_bytes(&data.root, &IDENTITY);
    let half = |db: &Db| {
        add_display(db)?;
        db.execute(&Query::new("UPDATE accounts SET missing = 1"))
            .map(drop)
    };
    assert_eq!(
        outcome(&data.root, &spec(&[ACCOUNTS_V1], &[Migration(&half)])),
        Err(IdentityError::Db {
            step: Step::Migrate,
            // SQLITE_ERROR: no such column
            error: DbError::Sqlite { code: 1 },
        })
    );
    assert_eq!(file_bytes(&data.root, &IDENTITY), before);
    let store = store(&data.root, &spec(&[ACCOUNTS], &[]));
    assert_eq!(read(&store, &ACCOUNT_NAMES), Ok(names(&["sam", "kim"])));
}

/// Verifies: SEC-OPS-048
#[test]
fn a_migration_that_breaks_an_invariant_rolls_back_and_keeps_the_snapshot() {
    let data = data();
    released(&data.root, &[ACCOUNTS], &[]);
    let before = file_bytes(&data.root, &IDENTITY);
    let wipe = |db: &Db| db.execute(&Query::new("DELETE FROM accounts")).map(drop);
    let migrations = [Migration(&wipe)];
    let breaking = Spec {
        invariants: &[AN_ACCOUNT_EXISTS],
        ..spec(&[ACCOUNTS], &migrations)
    };
    assert_eq!(
        outcome(&data.root, &breaking),
        Err(IdentityError::Invariant {
            name: "an account exists",
            found: vec![row(&[Value::Integer(0)])],
        })
    );
    assert_eq!(file_bytes(&data.root, &IDENTITY), before);
    let snapshot = open_untrusted(&data.root, &SNAPSHOT).expect("the snapshot opens");
    assert_eq!(snapshot.query(&ACCOUNT_NAMES), Ok(names(&["sam", "kim"])));
}

#[test]
fn checks_every_invariant_after_the_migrations() {
    let data = data();
    released(&data.root, &[ACCOUNTS], &[]);
    let keep = |db: &Db| db.execute(&add_account("lee")).map(drop);
    let migrations = [Migration(&keep)];
    let unreadable = Invariant {
        name: "two rows",
        holds: Query::new("SELECT 1 UNION ALL SELECT 1"),
    };
    let kept = Spec {
        invariants: &[AN_ACCOUNT_EXISTS, unreadable],
        ..spec(&[ACCOUNTS], &migrations)
    };
    assert_eq!(
        outcome(&data.root, &kept),
        Err(IdentityError::Invariant {
            name: "two rows",
            found: vec![row(&[Value::Integer(1)]), row(&[Value::Integer(1)])],
        })
    );
    let failing = Invariant {
        name: "broken",
        holds: Query::new("SELECT missing FROM accounts"),
    };
    assert_eq!(
        outcome(
            &data.root,
            &Spec {
                invariants: &[failing],
                ..spec(&[ACCOUNTS], &migrations)
            }
        ),
        Err(IdentityError::Db {
            step: Step::Migrate,
            error: DbError::Sqlite { code: 1 },
        })
    );
    assert_eq!(
        outcome(
            &data.root,
            &Spec {
                invariants: &[AN_ACCOUNT_EXISTS],
                ..spec(&[ACCOUNTS], &migrations)
            }
        ),
        Ok(Outcome::Migrated { from: 0, to: 1 })
    );
}

/// Verifies: SEC-TM-050
#[test]
fn a_migration_that_adds_an_unclassified_column_rolls_back() {
    let data = data();
    released(&data.root, &[ACCOUNTS], &[]);
    let before = file_bytes(&data.root, &IDENTITY);
    assert_eq!(
        outcome(&data.root, &spec(&[ACCOUNTS], &[Migration(&add_display)])),
        Err(IdentityError::Classes(ClassError::Unclassified {
            table: "accounts".to_owned(),
            column: "display".to_owned(),
        }))
    );
    assert_eq!(file_bytes(&data.root, &IDENTITY), before);
}

fn loosen(db: &Db) -> Result<(), DbError> {
    db.execute(&Query::new("UPDATE settings SET posture = 0"))
        .map(drop)
}

/// Verifies: SEC-OPS-049, SEC-OPS-048
#[test]
fn a_migration_that_loosens_a_security_setting_rolls_back() {
    let data = data();
    released(
        &data.root,
        &[ACCOUNTS, SETTINGS],
        &[add_setting("library", 0, 0), add_setting("server", 9, 1)],
    );
    let before = file_bytes(&data.root, &IDENTITY);
    let migrations = [Migration(&loosen)];
    let checked = Spec {
        settings: &[POSTURE, SHARING],
        ..spec(&[ACCOUNTS, SETTINGS], &migrations)
    };
    assert_eq!(
        outcome(&data.root, &checked),
        Err(IdentityError::Setting {
            setting: "posture",
            problem: SettingProblem::Loosened {
                key: text("server"),
                before: 9,
                after: 0,
            },
        })
    );
    assert_eq!(file_bytes(&data.root, &IDENTITY), before);
}

/// Verifies: SEC-PRV-023
#[test]
fn a_migration_that_changes_a_privacy_choice_rolls_back() {
    let data = data();
    released(
        &data.root,
        &[ACCOUNTS, SETTINGS],
        &[add_setting("kim", 9, 0), add_setting("sam", 9, 1)],
    );
    let before = file_bytes(&data.root, &IDENTITY);
    let private = |db: &Db| {
        db.execute(&Query::new("UPDATE settings SET sharing = 0"))
            .map(drop)
    };
    let migrations = [Migration(&private)];
    let checked = Spec {
        settings: &[POSTURE, SHARING],
        ..spec(&[ACCOUNTS, SETTINGS], &migrations)
    };
    assert_eq!(
        outcome(&data.root, &checked),
        Err(IdentityError::Setting {
            setting: "sharing",
            problem: SettingProblem::Changed {
                key: text("sam"),
                before: 1,
                after: 0,
            },
        })
    );
    assert_eq!(file_bytes(&data.root, &IDENTITY), before);
}

/// Verifies: SEC-OPS-051
#[test]
fn a_migration_that_drops_a_setting_rolls_back() {
    let data = data();
    released(
        &data.root,
        &[ACCOUNTS, SETTINGS],
        &[add_setting("kim", 9, 0), add_setting("sam", 9, 1)],
    );
    let before = file_bytes(&data.root, &IDENTITY);
    let drop_sam = |db: &Db| {
        db.execute(&Query::new("DELETE FROM settings WHERE holder = 'sam'"))
            .map(drop)
    };
    let migrations = [Migration(&drop_sam)];
    let checked = Spec {
        settings: &[SHARING],
        ..spec(&[ACCOUNTS, SETTINGS], &migrations)
    };
    assert_eq!(
        outcome(&data.root, &checked),
        Err(IdentityError::Setting {
            setting: "sharing",
            problem: SettingProblem::Dropped { key: text("sam") },
        })
    );
    assert_eq!(file_bytes(&data.root, &IDENTITY), before);
}

/// A part a release adds, with a setting that did not exist before it.
const LOCKS: SchemaPart = SchemaPart {
    name: "test.locks",
    sql: "CREATE TABLE locks (holder TEXT PRIMARY KEY, lock INTEGER NOT NULL);",
    columns: &[
        Column {
            table: "locks",
            name: "holder",
            class: DataClass::Identity,
        },
        Column {
            table: "locks",
            name: "lock",
            class: DataClass::Identity,
        },
    ],
};

/// Exists from format 1, when the release adding [`LOCKS`] migrates.
const LOCK: Setting = Setting {
    name: "lock",
    kind: SettingKind::Security,
    since: 1,
    read: Query::new("SELECT holder, lock FROM locks ORDER BY holder"),
    order: &[0, 1],
};

/// Verifies: SEC-OPS-049
#[test]
fn a_setting_a_migration_adds_starts_at_its_strictest() {
    let data = data();
    released(&data.root, &[ACCOUNTS], &[]);
    let add = |value: i64| {
        move |db: &Db| {
            db.execute_batch(LOCKS.sql)?;
            db.execute(
                &Query::new("INSERT INTO locks (holder, lock) VALUES ('server', ?1)")
                    .bind(Value::Integer(value)),
            )
            .map(drop)
        }
    };
    let (loose, strict) = (add(0), add(1));
    let loose_migrations = [Migration(&loose)];
    assert_eq!(
        outcome(
            &data.root,
            &Spec {
                settings: &[LOCK],
                ..spec(&[ACCOUNTS, LOCKS], &loose_migrations)
            }
        ),
        Err(IdentityError::Setting {
            setting: "lock",
            problem: SettingProblem::NotStrictest {
                key: text("server"),
                value: 0,
            },
        })
    );
    let strict_migrations = [Migration(&strict)];
    assert_eq!(
        outcome(
            &data.root,
            &Spec {
                settings: &[LOCK],
                ..spec(&[ACCOUNTS, LOCKS], &strict_migrations)
            }
        ),
        Ok(Outcome::Migrated { from: 0, to: 1 })
    );
}

#[test]
fn reads_a_setting_from_the_format_it_was_added_in() {
    let data = data();
    let first = |db: &Db| db.execute_batch(LOCKS.sql);
    let second = |db: &Db| {
        db.execute(&Query::new("UPDATE locks SET lock = 0"))
            .map(drop)
    };
    let first_only = [Migration(&first)];
    let store = store(&data.root, &spec(&[ACCOUNTS, LOCKS], &first_only));
    store
        .write(&[
            add_account("sam"),
            Query::new("INSERT INTO locks (holder, lock) VALUES ('server', 1)"),
        ])
        .expect("written");
    drop(store);
    let both = [Migration(&first), Migration(&second)];
    assert_eq!(
        outcome(
            &data.root,
            &Spec {
                settings: &[LOCK],
                ..spec(&[ACCOUNTS, LOCKS], &both)
            }
        ),
        Err(IdentityError::Setting {
            setting: "lock",
            problem: SettingProblem::Loosened {
                key: text("server"),
                before: 1,
                after: 0,
            },
        })
    );
}

#[test]
fn a_migration_whose_checks_cannot_run_rolls_back() {
    let data = data();
    released(&data.root, &[ACCOUNTS], &[]);
    let before = file_bytes(&data.root, &IDENTITY);
    let keep = |_: &Db| -> Result<(), DbError> { Ok(()) };
    let kept = [Migration(&keep)];
    // SQLITE_ERROR: no such table.
    let no_such_table = Err(IdentityError::Db {
        step: Step::Migrate,
        error: DbError::Sqlite { code: 1 },
    });
    // A setting of the old format that cannot be read before the migration.
    let unreadable = Setting { since: 0, ..LOCK };
    assert_eq!(
        outcome(
            &data.root,
            &Spec {
                settings: &[unreadable],
                ..spec(&[ACCOUNTS], &kept)
            }
        ),
        no_such_table
    );
    // A setting of the new format the migration did not create.
    assert_eq!(
        outcome(
            &data.root,
            &Spec {
                settings: &[LOCK],
                ..spec(&[ACCOUNTS], &kept)
            }
        ),
        no_such_table
    );
    // A migration that removes the store's own record.
    let unrecorded = |db: &Db| db.execute_batch("DROP TABLE store_meta;");
    assert_eq!(
        outcome(&data.root, &spec(&[ACCOUNTS], &[Migration(&unrecorded)])),
        no_such_table
    );
    assert_eq!(file_bytes(&data.root, &IDENTITY), before);
    assert_eq!(
        outcome(&data.root, &spec(&[ACCOUNTS], &[])),
        Ok(Outcome::Current)
    );
}

/// Verifies: SEC-OPS-048
#[test]
fn a_snapshot_that_cannot_be_written_stops_the_migration() {
    let data = data();
    released(&data.root, &[ACCOUNTS], &[]);
    let before = file_bytes(&data.root, &IDENTITY);
    data.root
        .create_dir(&DataPath::constant(DataDir::Snapshots, "identity.db"))
        .expect("a directory takes the snapshot's name");
    let ran = Cell::new(false);
    let watched = |_: &Db| -> Result<(), DbError> {
        ran.set(true);
        Ok(())
    };
    assert_eq!(
        outcome(&data.root, &spec(&[ACCOUNTS], &[Migration(&watched)])),
        Err(IdentityError::SnapshotFile(DataRootError::Io {
            item: Item::Path(DataPath::constant(DataDir::Snapshots, "identity.db")),
            op: Op::Rename,
            kind: std::io::ErrorKind::IsADirectory,
        }))
    );
    assert!(!ran.get());
    assert_eq!(file_bytes(&data.root, &IDENTITY), before);
}

/// Verifies: SEC-OPS-048
#[test]
fn a_snapshot_taken_while_another_connection_reads_stops_the_migration() {
    let data = data();
    released(&data.root, &[ACCOUNTS], &[]);
    let reader = open_db(
        &data.root,
        &IDENTITY,
        Pragmas::new(Synchronous::Full).query_only(),
    )
    .expect("a reader opens");
    let writer =
        open_db(&data.root, &IDENTITY, Pragmas::new(Synchronous::Full)).expect("a writer opens");
    reader.execute(&Query::new("BEGIN")).expect("begins");
    assert_eq!(reader.query(&ACCOUNT_NAMES), Ok(names(&["sam", "kim"])));
    writer.execute(&add_account("lee")).expect("written");
    let ran = Cell::new(false);
    let watched = |_: &Db| -> Result<(), DbError> {
        ran.set(true);
        Ok(())
    };
    // Busy, with the one page the insert wrote in the log and none of it
    // moved, since the reader still needs the page as it was.
    assert_eq!(
        outcome(&data.root, &spec(&[ACCOUNTS], &[Migration(&watched)])),
        Err(IdentityError::Checkpoint {
            report: vec![row(&[
                Value::Integer(1),
                Value::Integer(1),
                Value::Integer(0)
            ])],
        })
    );
    assert!(!ran.get());
}

/// The store's file and its log, beneath the data root.
const STORE_FILES: [DataPath; 2] = [
    DataPath::constant(DataDir::Durable, "identity.db"),
    DataPath::constant(DataDir::Durable, "identity.db-wal"),
];

/// Copies the store's file and its log, as they are on disk at this
/// moment, into the data root `to`, as a crash at this moment would leave
/// them.
fn crash_image(from: &DataRoot, to: &DataRoot) {
    use std::io::Read;
    for path in &STORE_FILES {
        let mut bytes = Vec::new();
        from.open_read(path)
            .expect("the file opens")
            .read_to_end(&mut bytes)
            .expect("the file reads");
        to.replace(path, &bytes).expect("the copy is written");
    }
}

/// Verifies: SEC-OPS-048
#[test]
fn a_crash_between_the_snapshot_and_the_commit_leaves_a_usable_file() {
    let data = data();
    released(&data.root, &[ACCOUNTS], &[]);
    let crashed = self::data();
    let interrupted = |db: &Db| {
        add_display(db)?;
        db.execute(&Query::new("UPDATE accounts SET display = name"))?;
        crash_image(&data.root, &crashed.root);
        Ok(())
    };
    let migrations = [Migration(&interrupted)];
    assert_eq!(
        outcome(&data.root, &spec(&[ACCOUNTS_V1], &migrations)),
        Ok(Outcome::Migrated { from: 0, to: 1 })
    );

    let store = store(&crashed.root, &spec(&[ACCOUNTS], &[]));
    assert_eq!(read(&store, &ACCOUNT_NAMES), Ok(names(&["sam", "kim"])));
    assert_eq!(read(&store, &FORMAT), Ok(vec![row(&[Value::Integer(0)])]));
    drop(store);
    let retried = [Migration(&add_display)];
    assert_eq!(
        outcome(&crashed.root, &spec(&[ACCOUNTS_V1], &retried)),
        Ok(Outcome::Migrated { from: 0, to: 1 })
    );
}

/// Verifies: SEC-OPS-048
#[test]
fn a_migration_that_panics_leaves_a_usable_file() {
    let data = data();
    released(&data.root, &[ACCOUNTS], &[]);
    let before = file_bytes(&data.root, &IDENTITY);
    let panicking = |db: &Db| -> Result<(), DbError> {
        add_display(db)?;
        std::panic::panic_any("the migration was interrupted");
    };
    let migrations = [Migration(&panicking)];
    let spec_v1 = spec(&[ACCOUNTS_V1], &migrations);
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        drop(open(&data.root, &spec_v1));
    }));
    assert_eq!(
        unwound
            .err()
            .and_then(|payload| payload.downcast::<&str>().ok())
            .map(|s| *s),
        Some("the migration was interrupted")
    );
    assert_eq!(file_bytes(&data.root, &IDENTITY), before);
    assert_eq!(
        outcome(&data.root, &spec(&[ACCOUNTS], &[])),
        Ok(Outcome::Current)
    );
}

fn generated_holders(values: &[i64]) -> Vec<Query> {
    values
        .iter()
        .enumerate()
        .map(|(index, posture)| add_setting(&format!("holder {index}"), *posture, 1))
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    /// A test migration that sets every posture to its loosest value fails
    /// the strictness check for every generated configuration that held a
    /// stricter one, names the first such holder, and leaves the file as it
    /// was; on a configuration already at the loosest value it changes
    /// nothing and passes.
    ///
    /// Verifies: SEC-OPS-049
    #[test]
    fn a_loosening_migration_fails_on_every_configuration_it_loosens(
        values in prop::collection::vec(prop::sample::select(vec![0_i64, 5, 9]), 1..5),
    ) {
        let data = data();
        released(&data.root, &[ACCOUNTS, SETTINGS], &generated_holders(&values));
        let before = file_bytes(&data.root, &IDENTITY);
        let migrations = [Migration(&loosen)];
        let checked = Spec {
            settings: &[POSTURE],
            ..spec(&[ACCOUNTS, SETTINGS], &migrations)
        };
        let expected = match values.iter().position(|value| *value != 0) {
            Some(index) => Err(IdentityError::Setting {
                setting: "posture",
                problem: SettingProblem::Loosened {
                    key: text(&format!("holder {index}")),
                    before: values[index],
                    after: 0,
                },
            }),
            None => Ok(Outcome::Migrated { from: 0, to: 1 }),
        };
        let loosened = expected.is_err();
        prop_assert_eq!(outcome(&data.root, &checked), expected);
        if loosened {
            prop_assert_eq!(file_bytes(&data.root, &IDENTITY), before);
        }
    }
}

const TRACK_A: &str = "trk_0123456789abcdefghjkmnpqrs";
const TRACK_B: &str = "trk_00000000000000000000000001";
const ALBUM: &str = "alb_0123456789abcdefghjkmnpqrs";

fn id(text: &str, kind: IdKind) -> PublicId {
    PublicId::parse(text, kind).expect("a canonical identifier")
}

fn at(millis: i64) -> Timestamp {
    Timestamp::from_millis(millis).expect("in range")
}

/// Verifies: SEC-HIS-012, SEC-API-023, SEC-PRV-021
#[test]
fn two_fresh_stores_keep_the_different_ids_minted_for_the_same_content() {
    let (first, second) = (data(), data());
    let content = b"the same content identity";
    let one = store(&first.root, &spec(&[], &[]));
    let two = store(&second.root, &spec(&[], &[]));
    assert_eq!(
        one.assign(IdKind::Track, content, id(TRACK_A, IdKind::Track), at(10)),
        Ok(Mapping {
            id: id(TRACK_A, IdKind::Track),
            first_seen: at(10),
        })
    );
    assert_eq!(
        two.assign(IdKind::Track, content, id(TRACK_B, IdKind::Track), at(10)),
        Ok(Mapping {
            id: id(TRACK_B, IdKind::Track),
            first_seen: at(10),
        })
    );
    assert_eq!(
        one.public_id(IdKind::Track, content),
        Ok(Some(Mapping {
            id: id(TRACK_A, IdKind::Track),
            first_seen: at(10),
        }))
    );
    assert_eq!(
        two.public_id(IdKind::Track, content),
        Ok(Some(Mapping {
            id: id(TRACK_B, IdKind::Track),
            first_seen: at(10),
        }))
    );
}

#[test]
fn an_item_keeps_the_first_id_it_was_given() {
    let data = data();
    let store = store(&data.root, &spec(&[], &[]));
    let content = b"content";
    let first = Mapping {
        id: id(TRACK_A, IdKind::Track),
        first_seen: at(-5),
    };
    assert_eq!(store.public_id(IdKind::Track, content), Ok(None));
    assert_eq!(
        store.assign(IdKind::Track, content, id(TRACK_A, IdKind::Track), at(-5)),
        Ok(first)
    );
    assert_eq!(
        store.assign(IdKind::Track, content, id(TRACK_B, IdKind::Track), at(99)),
        Ok(first)
    );
    drop(store);
    let store = self::store(&data.root, &spec(&[], &[]));
    assert_eq!(store.public_id(IdKind::Track, content), Ok(Some(first)));
    // The same content identity is a different item under another kind.
    assert_eq!(store.public_id(IdKind::Album, content), Ok(None));
    assert_eq!(
        store.assign(IdKind::Album, content, id(ALBUM, IdKind::Album), at(1)),
        Ok(Mapping {
            id: id(ALBUM, IdKind::Album),
            first_seen: at(1),
        })
    );
}

#[test]
fn refuses_an_id_of_another_kind_or_one_another_item_has() {
    let data = data();
    let store = store(&data.root, &spec(&[], &[]));
    assert_eq!(
        store.assign(IdKind::Track, b"a", id(ALBUM, IdKind::Album), at(0)),
        Err(IdentityError::WrongKind(IdError {
            expected: IdKind::Track
        }))
    );
    assert_eq!(store.public_id(IdKind::Track, b"a"), Ok(None));
    store
        .assign(IdKind::Track, b"a", id(TRACK_A, IdKind::Track), at(0))
        .expect("kept");
    assert_eq!(
        store.assign(IdKind::Track, b"b", id(TRACK_A, IdKind::Track), at(0)),
        Err(IdentityError::Db {
            step: Step::Write,
            // SQLITE_CONSTRAINT_UNIQUE
            error: DbError::Sqlite { code: 2067 },
        })
    );
    assert_eq!(store.public_id(IdKind::Track, b"b"), Ok(None));
}

#[test]
fn refuses_a_stored_mapping_it_cannot_read() {
    let data = data();
    let store = store(&data.root, &spec(&[], &[]));
    store
        .write(&[Query::new(
            "INSERT INTO public_ids (kind, content, public_id, first_seen) \
             VALUES ('track', x'61', 'trk_', 0)",
        )])
        .expect("written");
    assert_eq!(
        store.public_id(IdKind::Track, b"a"),
        Err(IdentityError::Mapping {
            found: vec![row(&[text("trk_"), Value::Integer(0)])],
        })
    );
}

/// The cache's library database.
const LIBRARY: DbFile = DbFile::new(DataDir::Cache, "library.db");

/// Throws the cache's database away and starts a new, empty one, as a
/// cache rebuild does.
fn rebuild_cache(root: &DataRoot) {
    root.replace(LIBRARY.path(), &[])
        .expect("the cache is emptied");
    let cache =
        open_db(root, &LIBRARY, Pragmas::new(Synchronous::Normal)).expect("the new cache opens");
    assert_eq!(cache.query(&TABLES), Ok(vec![]));
}

#[test]
fn a_cache_rebuild_keeps_every_identity_row_and_public_id() {
    let data = data();
    let store = store(&data.root, &spec(&[ACCOUNTS], &[]));
    store.write(&[add_account("sam")]).expect("written");
    store
        .assign(IdKind::Track, b"a", id(TRACK_A, IdKind::Track), at(7))
        .expect("kept");
    drop(store);
    let cache =
        open_db(&data.root, &LIBRARY, Pragmas::new(Synchronous::Normal)).expect("the cache opens");
    cache
        .execute_batch("CREATE TABLE tracks (id INTEGER PRIMARY KEY);")
        .expect("created");
    drop(cache);

    rebuild_cache(&data.root);

    let store = self::store(&data.root, &spec(&[ACCOUNTS], &[]));
    assert_eq!(read(&store, &ACCOUNT_NAMES), Ok(names(&["sam"])));
    assert_eq!(
        store.public_id(IdKind::Track, b"a"),
        Ok(Some(Mapping {
            id: id(TRACK_A, IdKind::Track),
            first_seen: at(7),
        }))
    );
}
