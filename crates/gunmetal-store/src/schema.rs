//! Schema registration by parts, the digest, the data-class check against
//! the live schema, and discard-and-rebuild.
//!
//! The cache is never migrated (ADM-058, ADM-077). The store records the
//! digest of the parts it was built from and the build's generation in its
//! own table, `store_meta`. On open it keeps the file only when that record
//! names the current digest and the columns SQLite reports are exactly the
//! classified ones; otherwise, or when the file cannot be read at all, it
//! empties the file and builds it again.

use gunmetal_core::schema::{Column, DataClass, LiveColumn, Schema, SchemaPart};
use gunmetal_fs::dataroot::DataRoot;
use gunmetal_fs::path::DataDir;
use gunmetal_fs::sqlite::{Db, DbFile, Pragmas, Query, Row, Synchronous, Value, open_db};

use crate::columns::column_names;
use crate::store::{Build, Generation, StoreError};
use crate::writer::{self, Transaction};

/// The cache file.
pub const LIBRARY: DbFile = DbFile::new(DataDir::Cache, "library.db");

/// The writer's pragmas: the cache is rebuildable, so `synchronous=NORMAL`,
/// and the writer never waits for a lock, since it is the only writer.
const WRITER: Pragmas = Pragmas::new(Synchronous::Normal);

/// The store's own part: the record of the build. Its name is taken, so
/// no other part may use it.
const STORE_PART: SchemaPart = SchemaPart {
    name: "store",
    sql: "CREATE TABLE store_meta (digest BLOB NOT NULL, generation BLOB NOT NULL) STRICT;",
    columns: &[
        Column {
            table: "store_meta",
            name: "digest",
            class: DataClass::Library,
        },
        Column {
            table: "store_meta",
            name: "generation",
            class: DataClass::Library,
        },
    ],
};

const TABLE_COUNT: Query = Query::new("SELECT count(*) FROM sqlite_schema");

/// Every table's name and the text that created it, as `(name, sql)`,
/// leaving out SQLite's own tables.
const TABLES: Query = Query::new(
    "SELECT name, sql FROM sqlite_schema \
     WHERE type = 'table' AND name NOT LIKE 'sqlite\\_%' ESCAPE '\\' \
     ORDER BY name",
);

/// The generations recorded for a digest.
const GENERATION: Query = Query::new("SELECT generation FROM store_meta WHERE digest = ?1");

const RECORD: Query = Query::new("INSERT INTO store_meta (digest, generation) VALUES (?1, ?2)");

/// Validates `parts` together with the store's own.
pub(crate) fn register(parts: &[SchemaPart]) -> Result<Schema, StoreError> {
    let mut all = parts.to_vec();
    all.push(STORE_PART);
    Schema::new(&all).map_err(StoreError::Schema)
}

/// What opening found in the file.
enum Found {
    /// No tables at all: a file the opener has just created.
    Empty(Db),
    /// A build of this schema, with its generation.
    Current(Db, Generation),
    /// Anything else, including a file SQLite cannot read.
    Stale,
}

/// Opens the writer's connection to a cache built from `schema`, building
/// or rebuilding it as needed.
pub(crate) fn settle(
    root: &DataRoot,
    schema: &Schema,
    generation: Generation,
) -> Result<(Db, Build, Generation), StoreError> {
    let found = open_db(root, &LIBRARY, WRITER).map_or(Found::Stale, |db| examine(db, schema));
    match found {
        Found::Current(db, kept) => Ok((db, Build::Reused, kept)),
        Found::Empty(db) => build(db, schema, generation, Build::Fresh),
        Found::Stale => root
            .replace(LIBRARY.path(), &[])
            .map_err(StoreError::DataRoot)
            .and_then(|()| open_db(root, &LIBRARY, WRITER).map_err(StoreError::from))
            .and_then(|db| build(db, schema, generation, Build::Rebuilt)),
    }
}

/// Decides what `db` holds. A stale connection is closed here, before the
/// file is emptied.
fn examine(db: Db, schema: &Schema) -> Found {
    if db.query(&TABLE_COUNT) == Ok(vec![Row(vec![Value::Integer(0)])]) {
        return Found::Empty(db);
    }
    let generation = db
        .query(&GENERATION.bind(Value::Blob(schema.digest().to_vec())))
        .ok()
        .and_then(generation_of)
        .filter(|_| check(&Transaction::new(&db), schema).is_ok());
    match generation {
        Some(generation) => Found::Current(db, generation),
        None => Found::Stale,
    }
}

/// Runs every part's SQL in name order, checks the classes and records the
/// build, all in one transaction, so a failed build leaves the file empty.
fn build(
    db: Db,
    schema: &Schema,
    generation: Generation,
    how: Build,
) -> Result<(Db, Build, Generation), StoreError> {
    writer::transact(&db, |tx| {
        schema
            .parts()
            .iter()
            .try_for_each(|part| tx.script(part.sql))
            .map_err(StoreError::from)
            .and_then(|()| check(tx, schema))
            .and_then(|()| {
                tx.execute(
                    &RECORD
                        .bind(Value::Blob(schema.digest().to_vec()))
                        .bind(Value::Blob(generation.0.to_vec())),
                )
                .map_err(StoreError::from)
            })
    })
    .map(|_| (db, how, generation))
}

/// Compares the columns the live tables have with the classified ones
/// (SEC-TM-050).
fn check(tx: &Transaction<'_>, schema: &Schema) -> Result<(), StoreError> {
    tx.query(&TABLES)
        .map_err(StoreError::from)
        .and_then(columns)
        .and_then(|live| schema.check_classes(&live).map_err(StoreError::Classes))
}

/// Reads every column of every `(name, sql)` row, in row order and then
/// column order.
fn columns(rows: Vec<Row>) -> Result<Vec<LiveColumn>, StoreError> {
    rows.into_iter()
        .map(table_columns)
        .try_fold(Vec::new(), |mut all, table| {
            all.extend(table?);
            Ok(all)
        })
}

/// Reads the columns of one `(name, sql)` row.
fn table_columns(row: Row) -> Result<Vec<LiveColumn>, StoreError> {
    match <[Value; 2]>::try_from(row.0) {
        Ok([Value::Text(table), Value::Text(sql)]) => column_names(&sql)
            .map(|names| {
                names
                    .into_iter()
                    .map(|name| LiveColumn {
                        table: table.clone(),
                        name,
                    })
                    .collect()
            })
            .ok_or(StoreError::Catalogue),
        _ => Err(StoreError::Catalogue),
    }
}

/// The generation in `rows`, which must be exactly one row holding exactly
/// one 16-byte blob.
fn generation_of(rows: Vec<Row>) -> Option<Generation> {
    match <[Row; 1]>::try_from(rows) {
        Ok([Row(values)]) => match <[Value; 1]>::try_from(values) {
            Ok([Value::Blob(bytes)]) => <[u8; 16]>::try_from(bytes).ok().map(Generation),
            _ => None,
        },
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: &str) -> Value {
        Value::Text(value.to_owned())
    }

    fn live(table: &str, name: &str) -> LiveColumn {
        LiveColumn {
            table: table.to_owned(),
            name: name.to_owned(),
        }
    }

    #[test]
    fn reads_every_column_of_every_table_in_order() {
        assert_eq!(
            columns(vec![
                Row(vec![text("albums"), text("CREATE TABLE albums (id, name)")]),
                Row(vec![text("empty"), text("CREATE TABLE empty (CHECK (1))")]),
                Row(vec![text("tracks"), text("CREATE TABLE tracks (title)")]),
            ]),
            Ok(vec![
                live("albums", "id"),
                live("albums", "name"),
                live("tracks", "title"),
            ])
        );
        assert_eq!(columns(vec![]), Ok(vec![]));
    }

    #[test]
    fn reads_columns_only_from_a_table_name_and_its_create_table_text() {
        let table = || text("CREATE TABLE t (a)");
        for row in [
            vec![],
            vec![text("t")],
            vec![text("t"), table(), text("extra")],
            vec![text("t"), Value::Null],
            vec![Value::Integer(1), table()],
            vec![text("t"), text("CREATE TABLE t AS SELECT 1 AS a")],
        ] {
            assert_eq!(table_columns(Row(row)), Err(StoreError::Catalogue));
        }
        // One table the store cannot read fails the whole catalogue.
        assert_eq!(
            columns(vec![
                Row(vec![text("t"), table()]),
                Row(vec![text("u"), Value::Null]),
                Row(vec![text("v"), text("CREATE TABLE v (b)")]),
            ]),
            Err(StoreError::Catalogue)
        );
    }

    #[test]
    fn reads_a_generation_only_from_one_row_of_one_sixteen_byte_blob() {
        let blob = |len| Value::Blob(vec![7; len]);
        assert_eq!(
            generation_of(vec![Row(vec![blob(16)])]),
            Some(Generation([7; 16]))
        );
        for rows in [
            vec![],
            vec![Row(vec![blob(16)]), Row(vec![blob(16)])],
            vec![Row(vec![])],
            vec![Row(vec![blob(16), blob(16)])],
            vec![Row(vec![blob(15)])],
            vec![Row(vec![blob(17)])],
            vec![Row(vec![text("sixteen bytes!!!")])],
        ] {
            assert_eq!(generation_of(rows), None);
        }
    }
}
