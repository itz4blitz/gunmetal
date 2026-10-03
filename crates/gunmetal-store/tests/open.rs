//! Opening the cache: fresh, reused or rebuilt, and refused when its parts
//! do not classify every column they create. Real SQLite files in a
//! temporary data root.

#[allow(dead_code, reason = "each test binary uses part of the helpers")]
mod support;

use gunmetal_core::schema::{ClassError, Column, DataClass, SchemaError, SchemaPart};
use gunmetal_fs::sqlite::{DbError, Query};
use gunmetal_store::schema::LIBRARY;
use gunmetal_store::store::{Build, Store, StoreError};
use support::{Data, FIRST, LIBRARY_WAL, SECOND, TRACKS, add, named, titles, wait};

/// [`TRACKS`] with one more column, so its SQL and its digest differ.
const TRACKS_V2: SchemaPart = SchemaPart {
    name: "tracks",
    sql: "CREATE TABLE tracks (id INTEGER PRIMARY KEY, title TEXT NOT NULL, year INTEGER) STRICT;",
    columns: &[
        Column {
            table: "tracks",
            name: "id",
            class: DataClass::Library,
        },
        Column {
            table: "tracks",
            name: "title",
            class: DataClass::Library,
        },
        Column {
            table: "tracks",
            name: "year",
            class: DataClass::Library,
        },
    ],
};

/// A part that creates a column it does not classify.
const UNCLASSIFIED: SchemaPart = SchemaPart {
    name: "plays",
    sql: "CREATE TABLE plays (track INTEGER NOT NULL, at INTEGER NOT NULL) STRICT;",
    columns: &[Column {
        table: "plays",
        name: "track",
        class: DataClass::Activity,
    }],
};

/// A part that classifies a column it does not create.
const PHANTOM: SchemaPart = SchemaPart {
    name: "plays",
    sql: "CREATE TABLE plays (track INTEGER NOT NULL) STRICT;",
    columns: &[
        Column {
            table: "plays",
            name: "track",
            class: DataClass::Activity,
        },
        Column {
            table: "plays",
            name: "at",
            class: DataClass::Activity,
        },
    ],
};

#[test]
fn builds_a_new_cache_with_the_generation_it_is_given() {
    let data = Data::new();
    let opened = data.open(&[TRACKS], FIRST);
    assert_eq!((opened.build, opened.generation), (Build::Fresh, FIRST));
    assert_eq!(add(&opened.store, "Intro"), Ok(1));
    assert_eq!(titles(&opened.store), named(&["Intro"]));
}

#[test]
fn reuses_the_cache_and_its_generation_when_no_part_changed() {
    let data = Data::new();
    let first = data.open(&[TRACKS], FIRST);
    assert_eq!(add(&first.store, "Intro"), Ok(1));
    drop(first);
    let again = data.open(&[TRACKS], SECOND);
    assert_eq!((again.build, again.generation), (Build::Reused, FIRST));
    assert_eq!(titles(&again.store), named(&["Intro"]));
}

#[test]
fn rebuilds_with_a_new_generation_when_one_parts_sql_changes() {
    let data = Data::new();
    let first = data.open(&[TRACKS], FIRST);
    assert_eq!(add(&first.store, "Intro"), Ok(1));
    drop(first);
    let changed = data.open(&[TRACKS_V2], SECOND);
    assert_eq!(
        (changed.build, changed.generation),
        (Build::Rebuilt, SECOND)
    );
    assert_eq!(titles(&changed.store), named(&[]));
}

#[test]
fn rebuilds_a_corrupt_file_rather_than_failing() {
    let data = Data::new();
    let first = data.open(&[TRACKS], FIRST);
    assert_eq!(add(&first.store, "Intro"), Ok(1));
    drop(first);
    let mut bytes = data.cache_bytes();
    assert_eq!(&bytes[..16], b"SQLite format 3\0");
    bytes[..16].copy_from_slice(b"not a database!!");
    data.root
        .replace(LIBRARY.path(), &bytes)
        .expect("overwrite the cache");
    let rebuilt = data.open(&[TRACKS], SECOND);
    assert_eq!(
        (rebuilt.build, rebuilt.generation),
        (Build::Rebuilt, SECOND)
    );
    assert_eq!(titles(&rebuilt.store), named(&[]));
    assert_eq!(add(&rebuilt.store, "Outro"), Ok(1));
    assert_eq!(titles(&rebuilt.store), named(&["Outro"]));
}

#[test]
fn rebuilds_a_corrupt_file_that_a_crash_left_with_a_write_ahead_log() {
    let data = Data::new();
    let first = data.open(&[TRACKS], FIRST);
    assert_eq!(add(&first.store, "Intro"), Ok(1));
    // While the store is open, its last commits are only in the log; a
    // crash now would leave both files as they are.
    let mut bytes = data.cache_bytes();
    let log = data.bytes(&LIBRARY_WAL);
    drop(first);
    bytes[..16].copy_from_slice(b"not a database!!");
    data.root
        .replace(LIBRARY.path(), &bytes)
        .expect("overwrite the cache");
    data.root
        .replace(&LIBRARY_WAL, &log)
        .expect("restore the log");
    let rebuilt = data.open(&[TRACKS_V2], SECOND);
    assert_eq!(
        (rebuilt.build, rebuilt.generation),
        (Build::Rebuilt, SECOND)
    );
    assert_eq!(titles(&rebuilt.store), named(&[]));
    assert_eq!(add(&rebuilt.store, "Outro"), Ok(1));
    assert_eq!(titles(&rebuilt.store), named(&["Outro"]));
}

/// A part whose table counts its keys in SQLite's own `sqlite_sequence`
/// table, which no part creates or classifies.
const COUNTED: SchemaPart = SchemaPart {
    name: "counted",
    sql: "CREATE TABLE counted (id INTEGER PRIMARY KEY AUTOINCREMENT);",
    columns: &[Column {
        table: "counted",
        name: "id",
        class: DataClass::Library,
    }],
};

#[test]
fn leaves_sqlites_own_tables_out_of_the_class_check() {
    let data = Data::new();
    let first = data.open(&[TRACKS, COUNTED], FIRST);
    assert_eq!((first.build, first.generation), (Build::Fresh, FIRST));
    let count = Query::new("INSERT INTO counted DEFAULT VALUES");
    assert_eq!(
        wait(first.store.write(move |tx| Ok(tx.execute(&count)?))),
        Ok(1)
    );
    drop(first);
    let again = data.open(&[TRACKS, COUNTED], SECOND);
    assert_eq!((again.build, again.generation), (Build::Reused, FIRST));
}

/// Statements that leave the cache's own record of its build, or its live
/// columns, different from what its parts declare.
const TAMPERING: [&str; 8] = [
    // A column no part classifies, however it is written.
    "ALTER TABLE tracks ADD COLUMN path TEXT",
    "ALTER TABLE tracks ADD \"a \"\"quoted\"\" path\" /* , ) */ TEXT",
    // A table no part classifies.
    "CREATE TABLE extra (secret BLOB)",
    // No record of the build.
    "DROP TABLE store_meta",
    "DELETE FROM store_meta",
    // Two records of the build.
    "INSERT INTO store_meta (digest, generation) SELECT digest, generation FROM store_meta",
    // A generation of the wrong length.
    "UPDATE store_meta SET generation = x'0102'",
    // A different digest.
    "UPDATE store_meta SET digest = x'00'",
];

#[test]
fn rebuilds_a_cache_whose_build_record_or_columns_were_tampered_with() {
    for statement in TAMPERING {
        let data = Data::new();
        let first = data.open(&[TRACKS], FIRST);
        assert_eq!(add(&first.store, "Intro"), Ok(1));
        let tamper = Query::new(statement);
        assert_eq!(
            (
                statement,
                wait(first.store.write(move |tx| Ok(tx.execute(&tamper)?)))
            ),
            // The count of the last row-changing statement, which a schema
            // change leaves as it was: the track added above.
            (statement, Ok(1))
        );
        drop(first);
        let rebuilt = data.open(&[TRACKS], SECOND);
        assert_eq!(
            (
                statement,
                rebuilt.build,
                rebuilt.generation,
                titles(&rebuilt.store)
            ),
            (statement, Build::Rebuilt, SECOND, named(&[]))
        );
    }
}

/// Why opening the cache in `data` with `parts` was refused, if it was.
fn refusal(data: &Data, parts: &[SchemaPart]) -> Option<StoreError> {
    Store::open(&data.root, parts, FIRST).err()
}

/// Verifies: SEC-TM-050
#[test]
fn refuses_to_open_with_a_part_that_creates_an_unclassified_column() {
    let data = Data::new();
    assert_eq!(
        refusal(&data, &[TRACKS, UNCLASSIFIED]),
        Some(StoreError::Classes(ClassError::Unclassified {
            table: "plays".to_owned(),
            column: "at".to_owned(),
        }))
    );
    // Nothing of the refused build was kept.
    let opened = data.open(&[TRACKS], SECOND);
    assert_eq!((opened.build, opened.generation), (Build::Fresh, SECOND));
}

/// Verifies: SEC-TM-050
#[test]
fn refuses_to_rebuild_with_a_part_that_creates_an_unclassified_column() {
    let data = Data::new();
    drop(data.open(&[TRACKS], FIRST));
    assert_eq!(
        refusal(&data, &[TRACKS, UNCLASSIFIED]),
        Some(StoreError::Classes(ClassError::Unclassified {
            table: "plays".to_owned(),
            column: "at".to_owned(),
        }))
    );
}

#[test]
fn refuses_to_open_with_a_part_that_classifies_a_column_it_does_not_create() {
    let data = Data::new();
    assert_eq!(
        refusal(&data, &[PHANTOM, TRACKS]),
        Some(StoreError::Classes(ClassError::Missing {
            part: "plays",
            table: "plays",
            column: "at",
        }))
    );
}

#[test]
fn refuses_a_part_that_takes_the_stores_own_name() {
    let data = Data::new();
    let rival = SchemaPart {
        name: "store",
        ..TRACKS
    };
    assert_eq!(
        refusal(&data, &[rival]),
        Some(StoreError::Schema(SchemaError::DuplicateName {
            name: "store"
        }))
    );
}

#[test]
fn refuses_a_part_whose_sql_fails_and_keeps_nothing_of_it() {
    let data = Data::new();
    let broken = SchemaPart {
        name: "broken",
        sql: "CREATE TABLE broken (x) STRICT;",
        columns: &[],
    };
    assert_eq!(
        refusal(&data, &[TRACKS, broken]),
        Some(StoreError::Db(DbError::Sqlite { code: 1 }))
    );
    let opened = data.open(&[TRACKS], SECOND);
    assert_eq!((opened.build, opened.generation), (Build::Fresh, SECOND));
}

#[test]
fn refuses_a_part_whose_tables_columns_cannot_be_read() {
    let data = Data::new();
    let words = SchemaPart {
        name: "words",
        sql: "CREATE VIRTUAL TABLE words USING fts5(word);",
        columns: &[Column {
            table: "words",
            name: "word",
            class: DataClass::Library,
        }],
    };
    assert_eq!(
        refusal(&data, &[TRACKS, words]),
        Some(StoreError::Catalogue)
    );
    let opened = data.open(&[TRACKS], SECOND);
    assert_eq!((opened.build, opened.generation), (Build::Fresh, SECOND));
}
