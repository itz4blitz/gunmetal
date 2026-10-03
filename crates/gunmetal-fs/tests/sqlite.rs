//! The SQLite connection opener and the static-query type on real database
//! files in a temporary data root.
#![expect(
    clippy::disallowed_methods,
    reason = "tests of the filesystem door build and inspect hostile layouts on the real filesystem, by path (SEC-MED-033)"
)]

#[allow(dead_code, reason = "each test binary uses part of the helpers")]
mod support;

use std::fs;
use std::io::{Seek, SeekFrom, Write};
use std::os::unix::fs::symlink;

use gunmetal_fs::dataroot::DataRoot;
use gunmetal_fs::path::DataDir;
use gunmetal_fs::sqlite::{
    Db, DbError, DbFile, Pragmas, Query, Row, Synchronous, Value, open_db, open_untrusted,
};
use proptest::prelude::*;
use support::{TempDir, assert_not_root, mode, names, open, set_mode};

const LIBRARY: DbFile = DbFile::new(DataDir::Cache, "library.db");
const FOREIGN: DbFile = DbFile::new(DataDir::Tmp, "restore.db");

const SECURITY_PRAGMAS: Query = Query::new(
    "SELECT * FROM pragma_secure_delete(), pragma_foreign_keys(), pragma_trusted_schema()",
);
const CREATE_NAMES: &str = "CREATE TABLE names (id INTEGER PRIMARY KEY, name TEXT NOT NULL)";
const INSERT_NAME: Query = Query::new("INSERT INTO names (name) VALUES (?1)");
const BY_NAME: Query = Query::new("SELECT id, name FROM names WHERE name = ?1");
const TABLES: Query = Query::new("SELECT name FROM sqlite_schema ORDER BY name");

fn cache(root: &DataRoot) -> Db {
    open_db(root, &LIBRARY, Pragmas::new(Synchronous::Normal)).expect("the cache opens")
}

fn ints(values: &[i64]) -> Row {
    Row(values.iter().copied().map(Value::Integer).collect())
}

fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}

/// Verifies: SEC-PRV-050
#[test]
fn every_connection_of_a_pool_of_eight_has_the_security_pragmas() {
    let dir = TempDir::new();
    let root = open(&dir).root;
    let writer = open_db(&root, &LIBRARY, Pragmas::new(Synchronous::Normal)).expect("writer");
    let mut pool = vec![writer];
    for _ in 0..7 {
        let reader = Pragmas::new(Synchronous::Normal)
            .busy_timeout(5_000)
            .query_only();
        pool.push(open_db(&root, &LIBRARY, reader).expect("reader"));
    }
    for (index, db) in pool.iter().enumerate() {
        assert_eq!(
            db.query(&SECURITY_PRAGMAS),
            Ok(vec![ints(&[1, 1, 0])]),
            "connection {index}"
        );
    }
}

/// Verifies: SEC-PRV-050
#[test]
fn reads_back_the_pragmas_each_store_declares() {
    let dir = TempDir::new();
    let root = open(&dir).root;
    let identity = DbFile::new(DataDir::Durable, "identity.db");
    let read_back = Query::new(
        "SELECT * FROM pragma_journal_mode(), pragma_synchronous(), \
         pragma_query_only(), pragma_busy_timeout()",
    );
    let durable = open_db(&root, &identity, Pragmas::new(Synchronous::Full)).expect("opens");
    assert_eq!(
        durable.query(&read_back),
        Ok(vec![Row(vec![
            text("wal"),
            Value::Integer(2),
            Value::Integer(0),
            Value::Integer(0)
        ])])
    );
    let reader = Pragmas::new(Synchronous::Normal)
        .busy_timeout(750)
        .query_only();
    let reader = open_db(&root, &identity, reader).expect("opens");
    assert_eq!(
        reader.query(&read_back),
        Ok(vec![Row(vec![
            text("wal"),
            Value::Integer(1),
            Value::Integer(1),
            Value::Integer(750)
        ])])
    );
    assert_eq!(
        reader.execute_batch(CREATE_NAMES),
        Err(DbError::Sqlite { code: 8 })
    );
}

/// The statements a store could write to undo the opener's pragmas.
const PRAGMA_CHANGES: [&str; 7] = [
    "PRAGMA secure_delete = OFF",
    "PRAGMA secure_delete(0)",
    "pragma SECURE_DELETE = false",
    "PRAGMA main.secure_delete = OFF",
    "PRAGMA foreign_keys = OFF",
    "PRAGMA trusted_schema = ON",
    "PRAGMA query_only = OFF",
];

/// `SQLITE_AUTH`: the connection refused to prepare the statement.
const NOT_ALLOWED: DbError = DbError::Sqlite { code: 23 };

/// A store can build a pragma set only from `Pragmas::new` and its two
/// methods (the type's fields are private, so a struct literal does not
/// compile), so these are all the kinds of set there are. None of them opens
/// a connection with `secure_delete` off, and none can switch it off later.
/// The longest busy timeout the type holds, 65,535 ms, is among them: the
/// driver panics on one above 2,147,483,647 ms, which a `u16` cannot reach.
///
/// Verifies: SEC-PRV-050
#[test]
fn no_pragma_set_a_store_can_build_switches_secure_delete_off() {
    let dir = TempDir::new();
    let root = open(&dir).root;
    let read_back = Query::new("SELECT * FROM pragma_secure_delete()");
    let mut opened = 0;
    for synchronous in [Synchronous::Normal, Synchronous::Full] {
        let base = Pragmas::new(synchronous);
        for pragmas in [
            base,
            base.query_only(),
            base.busy_timeout(0),
            base.busy_timeout(5_000),
            base.busy_timeout(65_535).query_only(),
            base.query_only().busy_timeout(1),
        ] {
            let db = open_db(&root, &LIBRARY, pragmas).expect("the cache opens");
            assert_eq!(db.query(&read_back), Ok(vec![ints(&[1])]));
            for statement in [
                "PRAGMA secure_delete = OFF",
                "PRAGMA secure_delete(0)",
                "PRAGMA main.secure_delete = FAST",
            ] {
                assert_eq!(db.execute_batch(statement), Err(NOT_ALLOWED));
            }
            assert_eq!(db.query(&read_back), Ok(vec![ints(&[1])]));
            opened += 1;
        }
    }
    assert_eq!(opened, 12);
}

/// Verifies: SEC-PRV-050
#[test]
fn refuses_statements_that_change_a_pragma_after_opening() {
    let dir = TempDir::new();
    let root = open(&dir).root;
    let reader = Pragmas::new(Synchronous::Normal).query_only();
    let db = open_db(&root, &LIBRARY, reader).expect("the cache opens");
    for statement in PRAGMA_CHANGES {
        assert_eq!(
            db.execute(&Query::new(statement)),
            Err(NOT_ALLOWED),
            "{statement}"
        );
        assert_eq!(db.execute_batch(statement), Err(NOT_ALLOWED), "{statement}");
    }
    assert_eq!(
        db.query(&Query::new(
            "SELECT * FROM pragma_secure_delete(), pragma_foreign_keys(), \
             pragma_trusted_schema(), pragma_query_only()"
        )),
        Ok(vec![ints(&[1, 1, 0, 1])])
    );
}

/// Verifies: SEC-PRV-050
#[test]
fn still_checkpoints_the_log_and_reads_pragmas_after_opening() {
    let dir = TempDir::new();
    let root = open(&dir).root;
    let db = cache(&root);
    assert_eq!(
        db.query(&Query::new("PRAGMA wal_checkpoint(TRUNCATE)")),
        Ok(vec![ints(&[0, 0, 0])])
    );
    assert_eq!(
        db.query(&Query::new("PRAGMA quick_check")),
        Ok(vec![Row(vec![text("ok")])])
    );
}

/// SQL text naming `file` in the temporary directory. A test may build
/// statement text at run time only by leaking it, which is the point of
/// `Query`'s `&'static str`.
fn naming(text: &str, dir: &TempDir, file: &str) -> &'static str {
    let path = dir.join(file);
    let path = path.to_str().expect("UTF-8 temporary directory");
    Box::leak(text.replace("{path}", path).into_boxed_str())
}

/// Verifies: SEC-MED-033, SEC-HIS-016
#[test]
fn refuses_to_open_or_write_another_database_file() {
    let dir = TempDir::new();
    let root = open(&dir).root;
    let db = cache(&root);
    db.execute_batch(CREATE_NAMES).expect("create the table");
    let statements = [
        naming("ATTACH DATABASE '{path}' AS other", &dir, "attached.db"),
        naming("ATTACH '{path}' AS other", &dir, "names.db"),
        naming("VACUUM INTO '{path}'", &dir, "copy.db"),
        "ATTACH DATABASE ':memory:' AS other",
        "DETACH DATABASE main",
        "VACUUM",
    ];
    for statement in statements {
        assert_eq!(db.execute_batch(statement), Err(NOT_ALLOWED), "{statement}");
        assert_eq!(
            db.execute(&Query::new(statement)),
            Err(NOT_ALLOWED),
            "{statement}"
        );
    }
    assert_eq!(names(dir.path()), ["data"]);
    assert_eq!(db.query(&TABLES), Ok(vec![Row(vec![text("names")])]));
}

/// Verifies: SEC-PRV-050
#[test]
fn scrubs_deleted_rows_from_the_database_file() {
    let dir = TempDir::new();
    let root = open(&dir).root;
    let db = cache(&root);
    db.execute_batch(CREATE_NAMES).expect("create the table");
    for name in ["kept-7f3a9c-sentinel", "erased-51be04-sentinel"] {
        db.execute(&INSERT_NAME.bind(text(name))).expect("insert");
    }
    db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
        .expect("checkpoint");
    db.execute(
        &Query::new("DELETE FROM names WHERE name = ?1").bind(text("erased-51be04-sentinel")),
    )
    .expect("delete");
    db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
        .expect("checkpoint");
    let mut bytes = fs::read(dir.join("data/cache/library.db")).expect("read the file");
    bytes.extend(fs::read(dir.join("data/cache/library.db-wal")).expect("read the log"));
    let holds = |needle: &[u8]| bytes.windows(needle.len()).any(|window| window == needle);
    assert!(holds(b"kept-7f3a9c-sentinel"), "the scan finds live rows");
    assert!(!holds(b"erased-51be04-sentinel"), "the deleted row is gone");
}

#[test]
fn enforces_foreign_keys() {
    let dir = TempDir::new();
    let root = open(&dir).root;
    let db = cache(&root);
    db.execute_batch(
        "CREATE TABLE albums (id INTEGER PRIMARY KEY); \
         CREATE TABLE tracks (id INTEGER PRIMARY KEY, album INTEGER NOT NULL REFERENCES albums (id))",
    )
    .expect("create the tables");
    assert_eq!(
        db.execute(&Query::new("INSERT INTO tracks (album) VALUES (?1)").bind(Value::Integer(9))),
        Err(DbError::Sqlite { code: 787 })
    );
}

#[test]
fn creates_the_database_and_its_journal_files_owner_only() {
    assert_not_root();
    let dir = TempDir::new();
    let root = open(&dir).root;
    let db = cache(&root);
    db.execute_batch(CREATE_NAMES).expect("create the table");
    for suffix in ["", "-wal", "-shm"] {
        assert_eq!(
            mode(&dir.join(&format!("data/cache/library.db{suffix}"))),
            0o600,
            "library.db{suffix}"
        );
    }
}

#[test]
fn round_trips_every_kind_of_value() {
    let dir = TempDir::new();
    let root = open(&dir).root;
    let db = cache(&root);
    let values = vec![
        Value::Null,
        Value::Integer(i64::MIN),
        Value::Real(-1.5),
        text("caf\u{e9}"),
        Value::Blob(vec![0, 255, 7]),
    ];
    let query = values
        .iter()
        .cloned()
        .fold(Query::new("SELECT ?1, ?2, ?3, ?4, ?5"), Query::bind);
    assert_eq!(db.query(&query), Ok(vec![Row(values)]));
    assert_eq!(db.query(&Query::new("SELECT 1 WHERE 0")), Ok(vec![]));
}

#[test]
fn counts_the_rows_a_statement_changed() {
    let dir = TempDir::new();
    let root = open(&dir).root;
    let db = cache(&root);
    db.execute_batch(CREATE_NAMES).expect("create the table");
    for name in ["a", "b", "c"] {
        assert_eq!(db.execute(&INSERT_NAME.bind(text(name))), Ok(1));
    }
    assert_eq!(
        db.execute(
            &Query::new("UPDATE names SET name = ?1 WHERE id > ?2")
                .bind(text("z"))
                .bind(Value::Integer(1))
        ),
        Ok(2)
    );
}

#[test]
fn reports_text_that_is_not_utf8_with_its_column() {
    let dir = TempDir::new();
    let root = open(&dir).root;
    let db = cache(&root);
    assert_eq!(
        db.query(&Query::new("SELECT 1, CAST(x'FF' AS TEXT)")),
        Err(DbError::NotUtf8 { column: 1 })
    );
}

#[test]
fn reports_each_way_a_statement_can_be_refused() {
    let dir = TempDir::new();
    let root = open(&dir).root;
    let db = cache(&root);
    let cases = [
        (
            db.query(&Query::new("SELEC 1")).map(|_| ()),
            DbError::Sqlite { code: 1 },
        ),
        (
            db.query(&Query::new("SELECT ?1, ?2")).map(|_| ()),
            DbError::ParameterCount {
                expected: 2,
                given: 0,
            },
        ),
        (
            db.query(&Query::new("SELECT 1; SELECT 2")).map(|_| ()),
            DbError::MultipleStatements,
        ),
        (
            db.query(&Query::new("SELECT abs(-9223372036854775807 - 1)"))
                .map(|_| ()),
            DbError::Sqlite { code: 1 },
        ),
        (
            db.execute(&Query::new("SELECT 1")).map(|_| ()),
            DbError::ReturnedRows,
        ),
        (
            db.execute(&Query::new("INSERT INTO missing VALUES (1)"))
                .map(|_| ()),
            DbError::Sqlite { code: 1 },
        ),
        (
            db.execute_batch("CREATE TABLE t (x); CREATE TABLE t (x)"),
            DbError::Sqlite { code: 1 },
        ),
    ];
    for (index, (found, expected)) in cases.into_iter().enumerate() {
        assert_eq!(found, Err(expected), "case {index}");
    }
}

fn hostile_text() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("'; DROP TABLE names; --".to_owned()),
        Just("\" OR 1=1 --".to_owned()),
        Just("x'00'".to_owned()),
        Just("?1 :name @name $name".to_owned()),
        Just("%_\\".to_owned()),
        Just("\0".to_owned()),
        "[ -~]{0,32}",
        "\\PC{0,16}",
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Verifies: SEC-API-066, SEC-TM-039, SEC-HIS-038
    #[test]
    fn binds_hostile_text_as_plain_data(name in hostile_text()) {
        let dir = TempDir::new();
        let root = open(&dir).root;
        let db = cache(&root);
        db.execute_batch(CREATE_NAMES).expect("create the table");
        prop_assert_eq!(db.execute(&INSERT_NAME.bind(text(&name))), Ok(1));
        prop_assert_eq!(
            db.query(&BY_NAME.bind(text(&name))),
            Ok(vec![Row(vec![Value::Integer(1), text(&name)])])
        );
        prop_assert_eq!(db.query(&TABLES), Ok(vec![Row(vec![text("names")])]));
    }
}

#[test]
fn refuses_a_file_that_is_not_a_database() {
    let dir = TempDir::new();
    let root = open(&dir).root;
    root.replace(LIBRARY.path(), &[0x5A; 4096])
        .expect("write garbage");
    assert_eq!(
        open_db(&root, &LIBRARY, Pragmas::new(Synchronous::Normal)).map(|_| ()),
        Err(DbError::Sqlite { code: 26 })
    );
}

#[test]
fn refuses_a_database_it_can_neither_find_nor_create() {
    assert_not_root();
    let dir = TempDir::new();
    let root = open(&dir).root;
    set_mode(&dir.join("data/cache"), 0o500);
    assert_eq!(
        open_db(&root, &LIBRARY, Pragmas::new(Synchronous::Normal)).map(|_| ()),
        Err(DbError::Sqlite { code: 14 })
    );
}

/// Verifies: SEC-HIS-016, SEC-TM-043
#[test]
fn refuses_a_symlink_planted_for_the_database() {
    let dir = TempDir::new();
    let root = open(&dir).root;
    fs::create_dir(dir.join("elsewhere")).expect("create the target directory");
    symlink(
        dir.join("elsewhere/stolen.db"),
        dir.join("data/cache/library.db"),
    )
    .expect("plant");
    assert_eq!(
        open_db(&root, &LIBRARY, Pragmas::new(Synchronous::Normal)).map(|_| ()),
        Err(DbError::Sqlite { code: 1550 })
    );
    assert_eq!(
        fs::read_dir(dir.join("elsewhere")).expect("list").count(),
        0
    );
}

/// Moves the data directory `open` opened to `moved` and puts a new, empty
/// directory holding `layout_dir` at its path, as someone who can write to
/// the directory above it could. The data root's handle still holds the
/// directory that was opened.
fn swap_the_data_directory(dir: &TempDir, layout_dir: &str) {
    fs::rename(dir.join("data"), dir.join("moved")).expect("move the data directory");
    fs::create_dir_all(dir.join("data").join(layout_dir)).expect("plant a data directory");
}

/// SQLite opens a database by path, so the opener compares the device and
/// inode of what that path names with what the data-root handle holds, and
/// refuses the connection before it writes anything.
#[test]
fn refuses_a_database_planted_where_the_data_directory_was() {
    let dir = TempDir::new();
    let root = open(&dir).root;
    swap_the_data_directory(&dir, "cache");
    fs::write(dir.join("data/cache/library.db"), b"").expect("plant a database");
    assert_eq!(
        open_db(&root, &LIBRARY, Pragmas::new(Synchronous::Normal)).map(|_| ()),
        Err(DbError::OutsideRoot)
    );
    assert_eq!(names(&dir.join("data/cache")), ["library.db"]);
    assert_eq!(
        fs::read(dir.join("data/cache/library.db")).expect("read the planted file"),
        b""
    );
    assert_eq!(names(&dir.join("moved/cache")), ["library.db"]);
}

/// A hard link to the real database is the same file, but SQLite would
/// keep its `-wal` and `-shm` files beside the link, outside the data
/// root, so the directory holding the database is compared as well.
#[test]
fn refuses_a_hard_link_to_the_database_in_a_planted_directory() {
    let dir = TempDir::new();
    let root = open(&dir).root;
    drop(cache(&root));
    swap_the_data_directory(&dir, "cache");
    fs::hard_link(
        dir.join("moved/cache/library.db"),
        dir.join("data/cache/library.db"),
    )
    .expect("link the database");
    assert_eq!(
        open_db(&root, &LIBRARY, Pragmas::new(Synchronous::Normal)).map(|_| ()),
        Err(DbError::OutsideRoot)
    );
    assert_eq!(names(&dir.join("data/cache")), ["library.db"]);
}

/// The database is at the path SQLite opens and nowhere beneath the handle.
#[test]
fn refuses_a_foreign_database_the_data_root_handle_does_not_hold() {
    let dir = TempDir::new();
    let root = open(&dir).root;
    foreign(&root);
    swap_the_data_directory(&dir, "tmp");
    fs::rename(
        dir.join("moved/tmp/restore.db"),
        dir.join("data/tmp/restore.db"),
    )
    .expect("move the database into the planted directory");
    assert_eq!(
        open_untrusted(&root, &FOREIGN).map(|_| ()),
        Err(DbError::OutsideRoot)
    );
}

/// A data directory that was only moved leaves nothing at the path, and
/// SQLite may not create a file there.
#[test]
fn refuses_to_create_a_database_where_the_data_directory_was() {
    let dir = TempDir::new();
    let root = open(&dir).root;
    swap_the_data_directory(&dir, "cache");
    assert_eq!(
        open_db(&root, &LIBRARY, Pragmas::new(Synchronous::Normal)).map(|_| ()),
        Err(DbError::Sqlite { code: 14 })
    );
    assert_eq!(names(&dir.join("data/cache")), [""; 0]);
}

/// Builds a database at `FOREIGN` with a view and a trigger, as a hostile
/// backup might carry.
fn foreign(root: &DataRoot) {
    let db = open_db(root, &FOREIGN, Pragmas::new(Synchronous::Full)).expect("opens");
    db.execute_batch(
        "CREATE TABLE t (x); \
         CREATE VIEW v AS SELECT x FROM t; \
         CREATE TRIGGER tr AFTER INSERT ON t BEGIN SELECT 1; END; \
         INSERT INTO t VALUES (1);",
    )
    .expect("build the foreign database");
}

#[test]
fn opens_a_foreign_database_read_only_with_views_and_triggers_off() {
    let dir = TempDir::new();
    let root = open(&dir).root;
    foreign(&root);
    let db = open_untrusted(&root, &FOREIGN).expect("opens");
    assert_eq!(
        db.query(&Query::new("SELECT x FROM t")),
        Ok(vec![ints(&[1])])
    );
    assert_eq!(
        db.query(&Query::new("SELECT x FROM v")).map(|_| ()),
        Err(DbError::Sqlite { code: 1 })
    );
    assert_eq!(
        db.execute(&Query::new("INSERT INTO t VALUES (2)")),
        Err(DbError::Sqlite { code: 8 })
    );
    assert_eq!(
        db.query(&Query::new(
            "SELECT * FROM pragma_secure_delete(), pragma_trusted_schema(), \
             pragma_cell_size_check(), pragma_query_only()"
        )),
        Ok(vec![ints(&[1, 0, 1, 1])])
    );
}

/// Verifies: SEC-PRV-050, SEC-MED-033
#[test]
fn locks_down_a_foreign_database_connection_as_well() {
    let dir = TempDir::new();
    let root = open(&dir).root;
    foreign(&root);
    let db = open_untrusted(&root, &FOREIGN).expect("opens");
    let attach = naming("ATTACH DATABASE '{path}' AS other", &dir, "attached.db");
    for statement in PRAGMA_CHANGES.into_iter().chain([attach]) {
        assert_eq!(db.execute_batch(statement), Err(NOT_ALLOWED), "{statement}");
    }
    assert_eq!(
        db.query(&Query::new(
            "SELECT * FROM pragma_secure_delete(), pragma_foreign_keys(), \
             pragma_trusted_schema(), pragma_query_only()"
        )),
        Ok(vec![ints(&[1, 1, 0, 1])])
    );
    assert_eq!(names(dir.path()), ["data"]);
}

#[test]
fn refuses_a_foreign_database_that_is_missing_or_not_a_database() {
    let dir = TempDir::new();
    let root = open(&dir).root;
    assert_eq!(
        open_untrusted(&root, &FOREIGN).map(|_| ()),
        Err(DbError::Sqlite { code: 14 })
    );
    root.replace(FOREIGN.path(), &[0x5A; 4096])
        .expect("write garbage");
    assert_eq!(
        open_untrusted(&root, &FOREIGN).map(|_| ()),
        Err(DbError::Sqlite { code: 26 })
    );
}

#[test]
fn refuses_a_foreign_database_that_fails_the_quick_check() {
    let dir = TempDir::new();
    let root = open(&dir).root;
    let db = open_db(&root, &FOREIGN, Pragmas::new(Synchronous::Full)).expect("opens");
    db.execute_batch(
        "CREATE TABLE t (x TEXT); \
         WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 200) \
         INSERT INTO t SELECT printf('%0100d', i) FROM n;",
    )
    .expect("fill several pages");
    drop(db);
    // Bytes 36 to 39 of the header count the pages on the freelist. Claiming
    // nine when there are none is damage that `quick_check` reports as a
    // row rather than failing on, once the file has more than a few pages.
    let mut file = fs::OpenOptions::new()
        .write(true)
        .open(dir.join("data/tmp/restore.db"))
        .expect("open the file");
    file.seek(SeekFrom::Start(36)).expect("seek");
    file.write_all(&[0, 0, 0, 9]).expect("damage the header");
    drop(file);
    assert_eq!(
        open_untrusted(&root, &FOREIGN).map(|_| ()),
        Err(DbError::QuickCheck)
    );
}
