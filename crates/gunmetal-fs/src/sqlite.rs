//! The one SQLite connection opener and the static-query type.
//!
//! Every database the server keeps (the cache, the identity store, the
//! audit address store and the derived-data store) is opened by
//! [`open_db`], which gives SQLite a path built from the data root and a
//! constant file name, refuses a path with a symlink anywhere in it, sets
//! the pragmas the store declares from a closed set, always including
//! `secure_delete=ON`, `foreign_keys=ON` and `trusted_schema=OFF`, and then
//! reads every pragma back and refuses the connection if one did not take
//! (SEC-PRV-050). A database the server did not create is opened only by
//! [`open_untrusted`], read-only and hardened (SEC-STD-031).
//!
//! Both then lock the connection down, so no statement can undo that: one
//! that gives a pragma a value (other than `wal_checkpoint` and
//! `quick_check`), attaches or detaches a database, or runs `VACUUM`, which
//! SQLite does through an attached copy, fails with `SQLITE_AUTH` (23).
//! Attaching would open a file by path, outside the data-root handle
//! (SEC-MED-033).
//!
//! A connection runs SQL only as a [`Query`], whose text is a
//! `&'static str` and whose values are bound parameters, so SQL built from
//! request text does not compile (SEC-API-066, SEC-TM-039, SEC-HIS-038).
//! Sort orders and filters are enums that each pick a whole static
//! statement.

use std::time::Duration;

use rusqlite::config::DbConfig;
use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use rusqlite::limits::Limit;
use rusqlite::types::{ToSql, ToSqlOutput, ValueRef};
use rusqlite::{Connection, OpenFlags, ffi, params_from_iter};

use crate::dataroot::DataRoot;
use crate::path::{DataDir, DataPath};

/// Flags for the server's own databases. `NOFOLLOW` makes SQLite refuse a
/// path with a symlink in any component.
const READ_WRITE: OpenFlags = OpenFlags::SQLITE_OPEN_READ_WRITE
    .union(OpenFlags::SQLITE_OPEN_NOFOLLOW)
    .union(OpenFlags::SQLITE_OPEN_NO_MUTEX);

/// Flags for a database the server did not create.
const READ_ONLY: OpenFlags = OpenFlags::SQLITE_OPEN_READ_ONLY
    .union(OpenFlags::SQLITE_OPEN_NOFOLLOW)
    .union(OpenFlags::SQLITE_OPEN_NO_MUTEX);

/// The pragmas every connection to the server's own databases gets.
const COMMON: &str = "PRAGMA secure_delete = ON; \
     PRAGMA foreign_keys = ON; \
     PRAGMA trusted_schema = OFF; \
     PRAGMA journal_mode = WAL;";

/// Reads back every pragma [`open_db`] sets, in the order of
/// [`Pragmas::expected`].
const READ_BACK: Query = Query::new(
    "SELECT * FROM pragma_secure_delete(), pragma_foreign_keys(), \
     pragma_trusted_schema(), pragma_journal_mode(), pragma_synchronous(), \
     pragma_query_only(), pragma_busy_timeout()",
);

/// The connection settings for a database the server did not create, each
/// with the value it must have.
const UNTRUSTED_CONFIG: [(DbConfig, bool); 4] = [
    (DbConfig::SQLITE_DBCONFIG_DEFENSIVE, true),
    (DbConfig::SQLITE_DBCONFIG_TRUSTED_SCHEMA, false),
    (DbConfig::SQLITE_DBCONFIG_ENABLE_TRIGGER, false),
    (DbConfig::SQLITE_DBCONFIG_ENABLE_VIEW, false),
];

/// The pragmas for a database the server did not create.
const UNTRUSTED: &str = "PRAGMA secure_delete = ON; \
     PRAGMA foreign_keys = ON; \
     PRAGMA trusted_schema = OFF; \
     PRAGMA cell_size_check = ON; \
     PRAGMA mmap_size = 0; \
     PRAGMA query_only = ON;";

/// Reads back the pragmas in [`UNTRUSTED`] that have table functions.
const UNTRUSTED_READ_BACK: Query = Query::new(
    "SELECT * FROM pragma_secure_delete(), pragma_foreign_keys(), \
     pragma_trusted_schema(), pragma_cell_size_check(), pragma_query_only()",
);

/// `mmap_size` has no table function, so it is read on its own.
const MMAP_SIZE: Query = Query::new("PRAGMA mmap_size");

const QUICK_CHECK: Query = Query::new("PRAGMA quick_check");

/// A database file in the data directory, named by constants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbFile(DataPath);

impl DbFile {
    /// Names the database `name` in layout directory `dir`. Use it in a
    /// `const` item, where an invalid name is a compile error.
    ///
    /// # Panics
    ///
    /// Panics when `name` breaks the data path rules, which in a `const`
    /// item is a compile error.
    #[must_use]
    pub const fn new(dir: DataDir, name: &'static str) -> Self {
        Self(DataPath::constant(dir, name))
    }

    /// Where the database is.
    #[must_use]
    pub const fn path(&self) -> &DataPath {
        &self.0
    }
}

/// How hard SQLite works to make a commit durable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Synchronous {
    /// `NORMAL`: for the rebuildable cache, where losing the last commits
    /// on power loss costs only a rescan.
    Normal,
    /// `FULL`: for durable stores.
    Full,
}

/// The pragmas a store declares for its connections.
///
/// Every connection also gets `secure_delete=ON`, `foreign_keys=ON`,
/// `trusted_schema=OFF` and `journal_mode=WAL`, and no method turns them
/// off:
///
/// ```
/// use gunmetal_fs::sqlite::{Pragmas, Synchronous};
///
/// let reader = Pragmas::new(Synchronous::Normal).busy_timeout(5_000).query_only();
/// assert_ne!(reader, Pragmas::new(Synchronous::Normal));
/// ```
///
/// A pragma set that leaves out `secure_delete` cannot be built.
/// Verifies: SEC-PRV-050
///
/// ```compile_fail,E0599
/// use gunmetal_fs::sqlite::{Pragmas, Synchronous};
///
/// let reader = Pragmas::new(Synchronous::Normal).secure_delete(false);
/// assert_ne!(reader, Pragmas::new(Synchronous::Normal));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pragmas {
    synchronous: Synchronous,
    busy_timeout_ms: u32,
    query_only: bool,
}

impl Pragmas {
    /// Read-write connections with no busy timeout and the given
    /// durability.
    #[must_use]
    pub const fn new(synchronous: Synchronous) -> Self {
        Self {
            synchronous,
            busy_timeout_ms: 0,
            query_only: false,
        }
    }

    /// Waits up to `ms` milliseconds for a lock instead of failing at once,
    /// which only readers should do.
    #[must_use]
    pub const fn busy_timeout(self, ms: u32) -> Self {
        Self {
            busy_timeout_ms: ms,
            ..self
        }
    }

    /// Refuses every write on the connection.
    #[must_use]
    pub const fn query_only(self) -> Self {
        Self {
            query_only: true,
            ..self
        }
    }
}

/// A value bound to a statement or read from a row.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// SQL `NULL`.
    Null,
    /// A 64-bit signed integer.
    Integer(i64),
    /// A 64-bit floating-point number.
    Real(f64),
    /// UTF-8 text.
    Text(String),
    /// Bytes.
    Blob(Vec<u8>),
}

/// One row of a result, one value per column.
#[derive(Debug, Clone, PartialEq)]
pub struct Row(pub Vec<Value>);

/// A statement: static SQL text and the values bound to its parameters.
///
/// ```
/// use gunmetal_fs::sqlite::{Query, Value};
///
/// const BY_NAME: Query = Query::new("SELECT id FROM users WHERE name = ?1");
/// let query = BY_NAME.bind(Value::Text("sam".to_owned()));
/// assert_ne!(query, BY_NAME);
/// ```
///
/// SQL built from a `String` does not compile.
/// Verifies: SEC-API-066, SEC-TM-039, SEC-HIS-038
///
/// ```compile_fail,E0597
/// use gunmetal_fs::sqlite::{Query, Value};
///
/// const BY_NAME: Query = Query::new("SELECT id FROM users WHERE name = ?1");
/// let name = String::from("sam");
/// let text = format!("SELECT id FROM users WHERE name = '{name}'");
/// let query = Query::new(&text);
/// assert_ne!(query, BY_NAME);
/// ```
///
/// Nor does SQL from a borrowed `&str`, such as a column name a request
/// chose; a sort order or filter is an enum that picks a whole static
/// statement.
/// Verifies: SEC-API-066, SEC-TM-039, SEC-HIS-038
///
/// ```compile_fail,E0521
/// use gunmetal_fs::sqlite::{Query, Value};
///
/// const BY_NAME: Query = Query::new("SELECT id FROM users WHERE name = ?1");
/// fn sorted_by(column: &str) -> Query {
///     Query::new(column)
/// }
/// assert_ne!(sorted_by("name"), BY_NAME);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    text: &'static str,
    params: Vec<Value>,
}

impl Query {
    /// A statement with no parameters bound yet.
    #[must_use]
    pub const fn new(text: &'static str) -> Self {
        Self {
            text,
            params: Vec::new(),
        }
    }

    /// Binds `value` to the next parameter.
    #[must_use]
    pub fn bind(mut self, value: Value) -> Self {
        self.params.push(value);
        self
    }
}

/// Why a database operation failed.
#[derive(Debug, Clone, PartialEq)]
pub enum DbError {
    /// SQLite reported an error, with its extended result code. A misuse
    /// of the driver that SQLite never saw is reported as `SQLITE_MISUSE`
    /// (21).
    Sqlite {
        /// The extended result code.
        code: i32,
    },
    /// The statement has a different number of parameters than were bound.
    ParameterCount {
        /// Parameters in the statement.
        expected: usize,
        /// Values bound, counting at most one more than expected.
        given: usize,
    },
    /// The text held more than one statement.
    MultipleStatements,
    /// [`Db::execute`] ran a statement that returns rows.
    ReturnedRows,
    /// A text value in the result was not UTF-8.
    NotUtf8 {
        /// The column it was in.
        column: usize,
    },
    /// A pragma or setting did not take, so the connection was refused.
    Settings {
        /// What the opener set.
        expected: Vec<Row>,
        /// What the connection reported.
        found: Vec<Row>,
    },
    /// A database the server did not create failed `PRAGMA quick_check`.
    QuickCheck,
}

/// An open database connection.
#[derive(Debug)]
pub struct Db {
    conn: rusqlite::Connection,
}

impl Db {
    /// Runs `query` and returns every row.
    ///
    /// # Errors
    ///
    /// Returns a [`DbError`] when SQLite refuses or fails the statement,
    /// the parameters do not match it, or a text value is not UTF-8.
    pub fn query(&self, query: &Query) -> Result<Vec<Row>, DbError> {
        let mut statement = self.conn.prepare(query.text).map_err(driver_error)?;
        let columns = statement.column_count();
        let mut rows = statement
            .query(params_from_iter(query.params.iter().map(Bind)))
            .map_err(driver_error)?;
        let mut found = Vec::new();
        while let Some(row) = rows.next().map_err(driver_error)? {
            let values = (0..columns)
                .map(|column| {
                    row.get_ref(column)
                        .map_err(driver_error)
                        .and_then(|value| Value::read(column, value))
                })
                .collect::<Result<_, _>>()?;
            found.push(Row(values));
        }
        Ok(found)
    }

    /// Runs `query`, which must not return rows, and returns how many rows
    /// it changed.
    ///
    /// # Errors
    ///
    /// Returns a [`DbError`] when SQLite refuses or fails the statement,
    /// the parameters do not match it, or it returns rows.
    pub fn execute(&self, query: &Query) -> Result<usize, DbError> {
        self.conn
            .prepare(query.text)
            .and_then(|mut statement| {
                statement.execute(params_from_iter(query.params.iter().map(Bind)))
            })
            .map_err(driver_error)
    }

    /// Runs `script`, a static text of statements with no parameters, such
    /// as a schema part.
    ///
    /// # Errors
    ///
    /// Returns [`DbError::Sqlite`] for the first statement that fails.
    pub fn execute_batch(&self, script: &'static str) -> Result<(), DbError> {
        self.conn.execute_batch(script).map_err(driver_error)
    }
}

impl Value {
    /// Copies a value out of a row. Text must be UTF-8, because a database
    /// the server did not create may hold anything.
    fn read(column: usize, value: ValueRef<'_>) -> Result<Self, DbError> {
        match value {
            ValueRef::Null => Ok(Self::Null),
            ValueRef::Integer(value) => Ok(Self::Integer(value)),
            ValueRef::Real(value) => Ok(Self::Real(value)),
            ValueRef::Text(bytes) => std::str::from_utf8(bytes)
                .map(|text| Self::Text(text.to_owned()))
                .map_err(|_| DbError::NotUtf8 { column }),
            ValueRef::Blob(bytes) => Ok(Self::Blob(bytes.to_vec())),
        }
    }
}

/// Lends a [`Value`] to `rusqlite` as a bound parameter, keeping
/// `rusqlite`'s traits out of this crate's public types.
struct Bind<'a>(&'a Value);

impl ToSql for Bind<'_> {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Borrowed(match self.0 {
            Value::Null => ValueRef::Null,
            Value::Integer(value) => ValueRef::Integer(*value),
            Value::Real(value) => ValueRef::Real(*value),
            Value::Text(value) => ValueRef::Text(value.as_bytes()),
            Value::Blob(value) => ValueRef::Blob(value),
        }))
    }
}

/// The one call that gives SQLite a path: the data root's resolved
/// directory joined with `db`'s constant name.
fn connect(root: &DataRoot, db: &DbFile, flags: OpenFlags) -> Result<Db, DbError> {
    Connection::open_with_flags(root.sqlite_path(db.path()), flags)
        .map(|conn| Db { conn })
        .map_err(driver_error)
}

/// Opens the database `db` beneath `root` with `pragmas`, creating it with
/// mode 0600 if it does not exist.
///
/// # Errors
///
/// Returns a [`DbError`] when SQLite cannot open the file (it is missing
/// and cannot be created, is not a database, or a symlink is anywhere in
/// its path), or when a pragma did not take.
///
/// The connection is then locked down as the [module documentation](self)
/// describes.
pub fn open_db(root: &DataRoot, db: &DbFile, pragmas: Pragmas) -> Result<Db, DbError> {
    // The first open creates the file through the data root, so it has mode
    // 0600, and SQLite gives its `-wal` and `-shm` files the same mode. An
    // existing file is left as it is; any other failure to create it makes
    // the open below fail, since SQLite is not allowed to create the file.
    let _ = root.create_new(db.path());
    let db = connect(root, db, READ_WRITE)?;
    db.execute_batch(COMMON)
        .and_then(|()| db.execute_batch(pragmas.synchronous.statement()))
        .and_then(|()| db.execute_batch(query_only_statement(pragmas.query_only)))
        .and_then(|()| {
            db.conn
                .busy_timeout(Duration::from_millis(u64::from(pragmas.busy_timeout_ms)))
                .map_err(driver_error)
        })
        .and_then(|()| db.query(&READ_BACK))
        .and_then(|found| check(pragmas.expected(), found))
        .and_then(|()| lock_down(&db.conn).map_err(driver_error))
        .map(|()| db)
}

impl Synchronous {
    const fn statement(self) -> &'static str {
        match self {
            Self::Normal => "PRAGMA synchronous = NORMAL",
            Self::Full => "PRAGMA synchronous = FULL",
        }
    }
}

const fn query_only_statement(on: bool) -> &'static str {
    if on {
        "PRAGMA query_only = ON"
    } else {
        "PRAGMA query_only = OFF"
    }
}

/// Opens `db`, a database the server did not create (a restored backup, an
/// import), read-only and hardened: defensive mode, triggers and views
/// disabled, `trusted_schema` off, `cell_size_check` on, memory mapping
/// off, and `PRAGMA quick_check` passed before any query (SEC-STD-031).
/// Callers run it in the jailed worker.
///
/// # Errors
///
/// Returns a [`DbError`] when SQLite cannot open the file, a setting did
/// not take, or the file fails the quick check.
pub fn open_untrusted(root: &DataRoot, db: &DbFile) -> Result<Db, DbError> {
    let db = connect(root, db, READ_ONLY)?;
    UNTRUSTED_CONFIG
        .iter()
        .map(|&(config, on)| {
            db.conn
                .set_db_config(config, on)
                .map(|now| Value::Integer(i64::from(now)))
        })
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(driver_error)
        .and_then(|configs| db.execute_batch(UNTRUSTED).map(|()| configs))
        .and_then(|configs| {
            db.query(&UNTRUSTED_READ_BACK).and_then(|mut found| {
                db.query(&MMAP_SIZE).map(|mmap| {
                    found.extend(mmap);
                    found.push(Row(configs));
                    found
                })
            })
        })
        .and_then(|found| check(untrusted_expected(), found))
        .and_then(|()| lock_down(&db.conn).map_err(driver_error))
        .and_then(|()| db.query(&QUICK_CHECK))
        .and_then(|report| passes_quick_check(&report))
        .map(|()| db)
}

impl Pragmas {
    /// What [`READ_BACK`] must return for these pragmas.
    fn expected(self) -> Vec<Row> {
        vec![Row(vec![
            Value::Integer(1),
            Value::Integer(1),
            Value::Integer(0),
            Value::Text("wal".to_owned()),
            Value::Integer(match self.synchronous {
                Synchronous::Normal => 1,
                Synchronous::Full => 2,
            }),
            Value::Integer(i64::from(self.query_only)),
            Value::Integer(i64::from(self.busy_timeout_ms)),
        ])]
    }
}

/// What [`open_untrusted`] must read back: the pragmas, `mmap_size`, then
/// the connection settings in the order of [`UNTRUSTED_CONFIG`].
fn untrusted_expected() -> Vec<Row> {
    let int = |values: &[i64]| Row(values.iter().copied().map(Value::Integer).collect());
    vec![int(&[1, 1, 0, 1, 1]), int(&[0]), int(&[1, 0, 0, 0])]
}

/// Maps a `rusqlite` error to a [`DbError`].
fn driver_error(error: rusqlite::Error) -> DbError {
    match error {
        rusqlite::Error::SqlInputError { error, .. } => DbError::Sqlite {
            code: error.extended_code,
        },
        rusqlite::Error::InvalidParameterCount(given, expected) => {
            DbError::ParameterCount { expected, given }
        }
        rusqlite::Error::MultipleStatement => DbError::MultipleStatements,
        rusqlite::Error::ExecuteReturnedResults => DbError::ReturnedRows,
        other => DbError::Sqlite {
            code: other
                .sqlite_error()
                .map_or(ffi::SQLITE_MISUSE, |error| error.extended_code),
        },
    }
}

/// The only pragmas a locked-down connection may give a value. Neither
/// changes a setting: one checks the database and one moves the log into
/// it, which the erasure job needs (SEC-PRV-050).
const ALLOWED_PRAGMAS: [&str; 2] = ["quick_check", "wal_checkpoint"];

/// Decides whether a connection may prepare a statement that does `context`.
/// Attaching or detaching a database would open a file by path, outside the
/// data-root handle (SEC-MED-033), and a pragma given a value could undo
/// what the opener set (SEC-PRV-050), so both are denied. A pragma with no
/// value only reads its setting (SQLite authorises the `pragma_...()` table
/// functions the same way), and everything else is for the store's own
/// statements to do.
fn authorize(context: AuthContext<'_>) -> Authorization {
    match context.action {
        AuthAction::Attach { .. } | AuthAction::Detach { .. } => Authorization::Deny,
        AuthAction::Pragma {
            pragma_name,
            pragma_value: Some(_),
        } if !ALLOWED_PRAGMAS
            .iter()
            .any(|allowed| allowed.eq_ignore_ascii_case(pragma_name)) =>
        {
            Authorization::Deny
        }
        _ => Authorization::Allow,
    }
}

/// Locks `conn` down once its opener has set it up and read it back: from
/// then on it refuses to prepare a statement [`authorize`] denies, which
/// fails with `SQLITE_AUTH`, and it can attach no database at all, which
/// also stops `VACUUM`, since SQLite runs that through an attached copy.
fn lock_down(conn: &Connection) -> rusqlite::Result<()> {
    conn.set_limit(Limit::SQLITE_LIMIT_ATTACHED, 0)
        .and_then(|_| conn.authorizer(Some(authorize)))
}

/// Refuses a connection whose settings read back differently.
fn check(expected: Vec<Row>, found: Vec<Row>) -> Result<(), DbError> {
    if found == expected {
        Ok(())
    } else {
        Err(DbError::Settings { expected, found })
    }
}

/// Accepts exactly the one-row, one-column "ok" report.
fn passes_quick_check(report: &[Row]) -> Result<(), DbError> {
    if report == [Row(vec![Value::Text("ok".to_owned())])] {
        Ok(())
    } else {
        Err(DbError::QuickCheck)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::ffi;

    fn row(values: &[i64]) -> Row {
        Row(values.iter().copied().map(Value::Integer).collect())
    }

    #[test]
    fn maps_each_driver_error_to_its_own_variant() {
        let cases = [
            (
                rusqlite::Error::SqliteFailure(ffi::Error::new(787), None),
                DbError::Sqlite { code: 787 },
            ),
            (
                rusqlite::Error::InvalidParameterCount(0, 2),
                DbError::ParameterCount {
                    expected: 2,
                    given: 0,
                },
            ),
            (
                rusqlite::Error::MultipleStatement,
                DbError::MultipleStatements,
            ),
            (
                rusqlite::Error::ExecuteReturnedResults,
                DbError::ReturnedRows,
            ),
            (
                rusqlite::Error::QueryReturnedNoRows,
                DbError::Sqlite { code: 21 },
            ),
        ];
        for (error, expected) in cases {
            assert_eq!(driver_error(error), expected);
        }
    }

    /// Verifies: SEC-PRV-050
    #[test]
    fn refuses_a_connection_whose_settings_did_not_take() {
        let expected = vec![row(&[1, 1, 0])];
        assert_eq!(check(expected.clone(), vec![row(&[1, 1, 0])]), Ok(()));
        assert_eq!(
            check(expected.clone(), vec![row(&[0, 1, 0])]),
            Err(DbError::Settings {
                expected,
                found: vec![row(&[0, 1, 0])]
            })
        );
    }

    fn ask(action: AuthAction<'_>) -> Authorization {
        authorize(AuthContext {
            action,
            database_name: Some("main"),
            accessor: None,
        })
    }

    fn pragma(name: &str, value: Option<&str>) -> Authorization {
        ask(AuthAction::Pragma {
            pragma_name: name,
            pragma_value: value,
        })
    }

    /// Verifies: SEC-PRV-050, SEC-MED-033
    #[test]
    fn denies_attaching_detaching_and_giving_a_pragma_a_value() {
        assert_eq!(
            ask(AuthAction::Attach {
                filename: "/etc/x.db"
            }),
            Authorization::Deny
        );
        assert_eq!(
            ask(AuthAction::Detach {
                database_name: "main"
            }),
            Authorization::Deny
        );
        for (name, value) in [
            ("secure_delete", "OFF"),
            ("foreign_keys", "0"),
            ("trusted_schema", "ON"),
            ("query_only", "OFF"),
            ("journal_mode", "DELETE"),
            ("temp_store_directory", "/tmp"),
            ("integrity_check", "1"),
        ] {
            assert_eq!(pragma(name, Some(value)), Authorization::Deny, "{name}");
        }
    }

    /// A pragma with no value only reads its setting, which is also how
    /// SQLite authorises `SELECT * FROM pragma_secure_delete()`.
    #[test]
    fn allows_reading_a_pragma_and_the_two_that_take_a_harmless_value() {
        for (name, value) in [
            ("secure_delete", None),
            ("foreign_keys", None),
            ("integrity_check", None),
            ("wal_checkpoint", Some("TRUNCATE")),
            ("WAL_Checkpoint", Some("passive")),
            ("quick_check", None),
            ("QUICK_CHECK", Some("1")),
        ] {
            assert_eq!(pragma(name, value), Authorization::Allow, "{name}");
        }
    }

    #[test]
    fn allows_reading_and_writing_rows() {
        for action in [
            AuthAction::Select,
            AuthAction::Read {
                table_name: "t",
                column_name: "x",
            },
            AuthAction::Insert { table_name: "t" },
            AuthAction::Delete { table_name: "t" },
        ] {
            assert_eq!(ask(action), Authorization::Allow, "{action:?}");
        }
    }

    /// Verifies: SEC-MED-033
    #[test]
    fn allows_no_attached_database_even_without_the_authorizer() {
        let conn = Connection::open_in_memory().expect("open");
        lock_down(&conn).expect("lock down");
        assert_eq!(conn.limit(Limit::SQLITE_LIMIT_ATTACHED), Ok(0));
        // Take the authorizer away to show the limit holds on its own.
        conn.authorizer(None::<fn(AuthContext<'_>) -> Authorization>)
            .expect("remove the authorizer");
        assert_eq!(
            conn.execute_batch("ATTACH DATABASE ':memory:' AS other")
                .map_err(driver_error),
            Err(DbError::Sqlite { code: 1 })
        );
    }

    #[test]
    fn passes_only_a_quick_check_that_says_ok() {
        let ok = || Row(vec![Value::Text("ok".to_owned())]);
        assert_eq!(passes_quick_check(&[ok()]), Ok(()));
        for report in [
            vec![],
            vec![Row(vec![Value::Text(
                "*** in database main ***".to_owned(),
            )])],
            vec![ok(), ok()],
        ] {
            assert_eq!(passes_quick_check(&report), Err(DbError::QuickCheck));
        }
    }

    #[test]
    fn expects_what_each_pragma_set_declares() {
        let wal = Value::Text("wal".to_owned());
        assert_eq!(
            Pragmas::new(Synchronous::Full).expected(),
            vec![Row(vec![
                Value::Integer(1),
                Value::Integer(1),
                Value::Integer(0),
                wal.clone(),
                Value::Integer(2),
                Value::Integer(0),
                Value::Integer(0),
            ])]
        );
        assert_eq!(
            Pragmas::new(Synchronous::Normal)
                .busy_timeout(250)
                .query_only()
                .expected(),
            vec![Row(vec![
                Value::Integer(1),
                Value::Integer(1),
                Value::Integer(0),
                wal,
                Value::Integer(1),
                Value::Integer(1),
                Value::Integer(250),
            ])]
        );
    }
}
