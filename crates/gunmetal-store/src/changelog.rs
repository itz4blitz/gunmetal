//! The ordered change log: what changed in the catalogue, numbered in the
//! order it was committed (LIB-018).
//!
//! Every change to a synced record gets the next sequence number in the
//! same transaction as the change itself. [`append`] takes the writer's
//! [`Transaction`], which exists only inside one write, so a change and its
//! entry commit together or not at all. A device keeps a [`Cursor`], the
//! number of the last change it has seen, and [`since`] reads the changes
//! after it one page at a time, so a device that was away for a month
//! catches up from one place (API-SYNC-02). [`head`] gives the cursor a
//! fresh snapshot starts from, and [`compact`] bounds the log by forgetting
//! every change up to a horizon.
//!
//! An entry holds a record's public identifier and whether the record was
//! written or removed, never a title, a path or anything else of the
//! record. The log is one for everybody, so it names no principal either:
//! whoever builds a delta from a page decides what its reader may see, by
//! reading each record through the catalogue's readers. An entry does not
//! name the record's library, so when a record cannot be read for someone,
//! as a removed one cannot, the entry alone does not say whether they could
//! ever see it.
//!
//! # What the log does not record yet
//!
//! The log records changes to tracks, albums and artists, which are what a
//! [`CatalogChange`] can name. ADR 3 sends two more things to devices
//! through this log, and neither has a row here yet: the projections of a
//! profile's stream, such as its playlists, and the tombstones that list
//! erased event identifiers. A row for either needs a kind of record the
//! table's checks do not take and a change [`append`] can be given. The
//! package that adds them widens the table and [`append`] together, and
//! decides whether a row must also say whose profile it belongs to, which
//! no column here can.
//!
//! # A device that must start again
//!
//! A cursor is good for one build of the cache, and for as long as the log
//! still holds every change after it. For any other cursor [`since`]
//! answers [`LogError::Resnapshot`], and the device takes a fresh snapshot
//! instead of a delta that would leave something out:
//!
//! - the cursor carries the [`Generation`] of another build, whose numbers
//!   mean nothing in this one ([`Resnapshot::Generation`]);
//! - compaction has removed changes after it ([`Resnapshot::Compacted`]);
//! - it is past the newest change ([`Resnapshot::Ahead`]): a device made
//!   the number up, or the cache has lost the commits that reached it.
//!
//! # What a power loss can undo
//!
//! While the cache keeps what it committed, a number belongs to one change
//! for good: SQLite keeps the highest number it has given out, and
//! compaction does not lower it. The cache does not always keep what it
//! committed. It runs with `synchronous=NORMAL`, as a store that can be
//! rebuilt may, so a power loss or a crash of the operating system can
//! take its last commits, and the changes recorded afterwards are given
//! the lost commits' numbers.
//!
//! A device that was handed a cursor past the head that survived is
//! answered [`Resnapshot::Ahead`], but only until the log has grown past
//! its cursor again. From then on [`since`] serves the cursor, and the
//! device never hears of the new changes numbered up to it. Nothing here
//! can tell: the generation, the head and the horizon read exactly as they
//! would had nothing been lost. Closing this is a decision for the cache
//! as a whole, such as a new generation after an unclean shutdown or
//! commits that survive a power loss, and it has to be made before a route
//! hands cursors to devices.
//!
//! # What a cursor is trusted for
//!
//! Nothing. A cursor comes back from a device, and it is only a place: a
//! generation and a number. [`since`] checks both against the cache's own
//! record on every read, and a place this build could have given out names
//! the same page whoever sends it.
//!
//! That supports SEC-API-025 and is not the whole of it. A device can
//! change the number in its cursor to any other from the horizon to the
//! head, and [`since`] serves it. Making a cursor opaque or MAC-protected,
//! refusing one that was changed, and deciding who may see which change
//! all belong to the route that hands cursors to devices.
//!
//! # Use
//!
//! ```
//! use std::num::NonZeroU32;
//!
//! use gunmetal_core::catalog::CatalogChange;
//! use gunmetal_store::changelog::{self, Cursor, LogError, Page};
//! use gunmetal_store::reply::Reply;
//! use gunmetal_store::store::Store;
//!
//! /// Logs a change in the transaction that makes it.
//! fn record(store: &Store, change: CatalogChange) -> Reply<()> {
//!     store.write(move |tx| {
//!         // The change itself is written to the catalogue's tables here.
//!         changelog::append(tx, &change)
//!     })
//! }
//!
//! /// One page of what changed after `cursor`.
//! fn delta(
//!     store: &Store,
//!     cursor: Cursor,
//!     limit: NonZeroU32,
//! ) -> Reply<Result<Page, LogError>> {
//!     store.read(move |reader| changelog::since(reader, cursor, limit))
//! }
//! ```

use std::num::NonZeroU32;

use gunmetal_core::catalog::{
    AlbumId, ArtistId, CatalogChange, CatalogError, ChangeOp, RecordId, TrackId,
};
use gunmetal_core::id::{IdKind, PublicId};
use gunmetal_core::schema::{Column, DataClass, SchemaPart};
use gunmetal_fs::sqlite::{DbError, Query, Row, Value};

use crate::readers::Reader;
use crate::store::{Generation, StoreError};
use crate::writer::Transaction;

/// The change log's table, for the cache's schema.
///
/// A row holds what an entry holds: the change's number, the record's
/// public identifier and what happened to the record. All of it is library
/// data.
///
/// The table's checks name the kinds of record and the codes of what can
/// happen to one, so the table refuses a row of any other kind or code
/// when it is written. The checks are part of the text the schema's digest
/// is taken over, so a build that logs other kinds or codes builds a cache
/// of its own instead of reading this one's rows.
pub const CHANGELOG: SchemaPart = SchemaPart {
    name: "changelog",
    sql: include_str!("changelog.sql"),
    columns: &[
        Column {
            table: "changelog",
            name: "seq",
            class: DataClass::Library,
        },
        Column {
            table: "changelog",
            name: "record",
            class: DataClass::Library,
        },
        Column {
            table: "changelog",
            name: "op",
            class: DataClass::Library,
        },
    ],
};

/// Adds one change. SQLite gives it the number after the highest it has on
/// record as given out.
const APPEND: Query = Query::new("INSERT INTO changelog (record, op) VALUES (?1, ?2)");

/// Removes every change numbered up to `?1`.
const COMPACT: Query = Query::new("DELETE FROM changelog WHERE seq <= ?1");

/// Where the log stands, as one row: the cache's generation, the head (the
/// highest number given out, or 0 before the first) and the horizon (the
/// number before the oldest change still held, or the head when none is
/// held).
///
/// It is one row only when the store's record of its build is one row.
const STATE: Query = Query::new(
    "SELECT meta.generation, head.seq, \
     coalesce((SELECT min(seq) - 1 FROM changelog), head.seq) \
     FROM store_meta AS meta, \
     (SELECT coalesce(max(seq), 0) AS seq FROM sqlite_sequence \
     WHERE name = 'changelog') AS head",
);

/// The changes numbered above `?1`, oldest first, at most `?2` of them.
const AFTER: Query =
    Query::new("SELECT seq, record, op FROM changelog WHERE seq > ?1 ORDER BY seq LIMIT ?2");

/// A change's place in the log: 1 for the first change a build of the cache
/// records and one more for each change after it. 0 is the place before
/// every change.
///
/// A number stays its change's for as long as the cache keeps what it
/// committed, even after compaction has removed every change. A cache that
/// loses its last commits to a power loss gives their numbers out again,
/// as the module's notes on a power loss describe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Seq(pub u64);

impl Seq {
    /// The number as a statement takes it. SQLite's integers end at
    /// `i64::MAX`, which no log reaches, so a larger number names a place
    /// past every change, as `i64::MAX` itself does.
    fn bound(self) -> Value {
        Value::Integer(i64::try_from(self.0).unwrap_or(i64::MAX))
    }
}

/// A device's place in the log: the last change it has seen, in one build
/// of the cache.
///
/// It holds nothing else, and nothing in it is trusted (SEC-API-025):
/// [`since`] checks both fields against the cache's own record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cursor {
    /// The build of the cache the number belongs to.
    pub generation: Generation,
    /// The number of the last change seen, or 0 for none.
    pub seq: Seq,
}

/// One change and its place in the log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// The change's number.
    pub seq: Seq,
    /// What changed.
    pub change: CatalogChange,
}

/// Some of the changes after a cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    /// The changes, oldest first, with none left out between the cursor and
    /// the last of them. Empty when nothing has changed since the cursor.
    pub entries: Vec<Entry>,
    /// The cursor to read on from: the place of the last change here, or
    /// the cursor that was given when there is none.
    pub next: Cursor,
}

/// Why a device must take a fresh snapshot instead of a delta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resnapshot {
    /// The cursor is from another build of the cache, so its number says
    /// nothing about this log.
    Generation {
        /// The generation the cursor carries.
        cursor: Generation,
        /// The generation of this build.
        cache: Generation,
    },
    /// Compaction has removed changes after the cursor.
    Compacted {
        /// The cursor's number.
        cursor: Seq,
        /// The oldest place the log can still be read from.
        horizon: Seq,
    },
    /// The cursor is past the newest change. A cursor a device made up
    /// reads so. So does one from before the cache lost its last commits,
    /// but only until new changes have taken the lost numbers: from then on
    /// the log cannot tell it from a good cursor.
    Ahead {
        /// The cursor's number.
        cursor: Seq,
        /// The number of the newest change.
        head: Seq,
    },
}

/// Why the log refused a cursor or could not be read.
#[derive(Debug, Clone, PartialEq)]
pub enum LogError {
    /// The log cannot serve the cursor, so the device takes a fresh
    /// snapshot.
    Resnapshot(Resnapshot),
    /// What the cache holds is not a log this build can read. The table
    /// refuses a kind of record or a code this build does not log, so what
    /// reads so is a cache that was damaged or written to by something
    /// else.
    Unreadable {
        /// The rows that could not be read: every row of the log's
        /// standing, or the one row that is not a change.
        found: Vec<Row>,
    },
    /// SQLite failed or refused a statement.
    Db(DbError),
}

/// Where the log stands in one read.
struct State {
    generation: Generation,
    head: Seq,
    horizon: Seq,
}

impl State {
    /// Accepts `cursor` when it carries this build's generation and its
    /// number is from the horizon to the head: a place this build can have
    /// given out and can still be read from.
    fn admit(&self, cursor: Cursor) -> Result<(), Resnapshot> {
        if cursor.generation != self.generation {
            return Err(Resnapshot::Generation {
                cursor: cursor.generation,
                cache: self.generation,
            });
        }
        if cursor.seq > self.head {
            return Err(Resnapshot::Ahead {
                cursor: cursor.seq,
                head: self.head,
            });
        }
        if cursor.seq < self.horizon {
            return Err(Resnapshot::Compacted {
                cursor: cursor.seq,
                horizon: self.horizon,
            });
        }
        Ok(())
    }
}

/// Records `change` as the newest in the log, in the transaction that makes
/// the change.
///
/// # Errors
///
/// Returns [`StoreError::Db`] when SQLite fails or refuses the statement,
/// as it does in a cache built without [`CHANGELOG`].
pub fn append(tx: &Transaction<'_>, change: &CatalogChange) -> Result<(), StoreError> {
    tx.execute(
        &APPEND
            .bind(Value::Text(change.record.public_id().to_string()))
            .bind(Value::Integer(i64::from(change.op.code()))),
    )
    .map(|_| ())
    .map_err(StoreError::from)
}

/// Forgets every change numbered up to `horizon` and returns how many that
/// removed.
///
/// A cursor older than the horizon is told to take a fresh snapshot from
/// then on. The horizon only moves forward and never passes the newest
/// change: compacting to an older place removes nothing, and compacting
/// past the newest change removes them all and leaves the numbering where
/// it was.
///
/// Every change up to the horizon is removed in the caller's one write,
/// however many there are, so a job that compacts a long log moves the
/// horizon forward in steps. A row holds no time: a job that compacts by
/// age keeps its own record of when a number was given out.
///
/// # Errors
///
/// Returns [`StoreError::Db`] when SQLite fails or refuses the statement.
pub fn compact(tx: &Transaction<'_>, horizon: Seq) -> Result<usize, StoreError> {
    tx.execute(&COMPACT.bind(horizon.bound()))
        .map_err(StoreError::from)
}

/// The cursor of the newest change: where a snapshot taken in the same read
/// is up to.
///
/// # Errors
///
/// Returns [`LogError::Unreadable`] when the log's standing cannot be read,
/// and [`LogError::Db`] when SQLite fails.
pub fn head(reader: &Reader<'_>) -> Result<Cursor, LogError> {
    state(reader).map(|state| Cursor {
        generation: state.generation,
        seq: state.head,
    })
}

/// Reads at most `limit` of the changes after `cursor`, oldest first.
///
/// Reading on from each page's `next` until a page comes back empty gives
/// every change after the cursor exactly once, in order.
///
/// `limit` is the only bound on a page. The log sets no ceiling of its own,
/// and a page is held in memory whole, first as rows and then as entries,
/// so the caller passes the page size its route declares (SEC-API-063) and
/// never a number as a request sent it.
///
/// # Errors
///
/// Returns [`LogError::Resnapshot`] when the cursor is from another build
/// of the cache, older than the horizon or past the newest change;
/// [`LogError::Unreadable`] when the log's standing or one of its rows
/// cannot be read; and [`LogError::Db`] when SQLite fails.
pub fn since(reader: &Reader<'_>, cursor: Cursor, limit: NonZeroU32) -> Result<Page, LogError> {
    state(reader)
        .and_then(|state| state.admit(cursor).map_err(LogError::Resnapshot))
        .and_then(|()| {
            reader
                .query(
                    &AFTER
                        .bind(cursor.seq.bound())
                        .bind(Value::Integer(i64::from(limit.get()))),
                )
                .map_err(LogError::Db)
        })
        .and_then(entries)
        .map(|found| {
            let next = found.last().map_or(cursor, |last| Cursor {
                generation: cursor.generation,
                seq: last.seq,
            });
            Page {
                entries: found,
                next,
            }
        })
}

/// Reads where the log stands.
fn state(reader: &Reader<'_>) -> Result<State, LogError> {
    reader
        .query(&STATE)
        .map_err(LogError::Db)
        .and_then(read_state)
}

/// Reads what [`STATE`] returned, which must be exactly one row of a
/// 16-byte generation and two numbers.
fn read_state(rows: Vec<Row>) -> Result<State, LogError> {
    let state = match rows.as_slice() {
        [Row(values)] => match values.as_slice() {
            [
                Value::Blob(generation),
                Value::Integer(head),
                Value::Integer(horizon),
            ] => <[u8; 16]>::try_from(generation.as_slice())
                .ok()
                .zip(seq_of(*head))
                .zip(seq_of(*horizon))
                .map(|((generation, head), horizon)| State {
                    generation: Generation(generation),
                    head,
                    horizon,
                }),
            _ => None,
        },
        _ => None,
    };
    state.ok_or(LogError::Unreadable { found: rows })
}

/// Reads every row [`AFTER`] returned. One row that is not a change fails
/// them all, so a page never leaves a change out.
fn entries(rows: Vec<Row>) -> Result<Vec<Entry>, LogError> {
    rows.into_iter().map(entry).collect()
}

/// Reads one row of [`AFTER`]: a number, a record's public identifier and
/// the code of what happened to it.
fn entry(row: Row) -> Result<Entry, LogError> {
    let read = match row.0.as_slice() {
        [Value::Integer(seq), Value::Text(record), Value::Integer(op)] => seq_of(*seq)
            .zip(record_of(record))
            .zip(op_of(*op))
            .map(|((seq, record), op)| Entry {
                seq,
                change: CatalogChange { record, op },
            }),
        _ => None,
    };
    read.ok_or_else(|| LogError::Unreadable { found: vec![row] })
}

/// The place `number` names, unless it is negative.
fn seq_of(number: i64) -> Option<Seq> {
    u64::try_from(number).ok().map(Seq)
}

/// What happened to a record, from its stored code.
fn op_of(code: i64) -> Option<ChangeOp> {
    u8::try_from(code).ok().and_then(ChangeOp::from_code)
}

/// The record `text` identifies: the one text form of a track's, an
/// album's or an artist's public identifier.
fn record_of(text: &str) -> Option<RecordId> {
    typed(text, IdKind::Track, TrackId::new, RecordId::Track)
        .or_else(|| typed(text, IdKind::Album, AlbumId::new, RecordId::Album))
        .or_else(|| typed(text, IdKind::Artist, ArtistId::new, RecordId::Artist))
}

/// The record `text` identifies when it is an identifier of kind `kind`.
fn typed<T>(
    text: &str,
    kind: IdKind,
    new: fn(PublicId) -> Result<T, CatalogError>,
    wrap: fn(T) -> RecordId,
) -> Option<RecordId> {
    PublicId::parse(text, kind)
        .ok()
        .and_then(|id| new(id).ok())
        .map(wrap)
}

#[cfg(test)]
mod tests {
    use std::iter;
    use std::num::NonZeroU32;
    use std::ops::RangeInclusive;

    use gunmetal_core::catalog::{AlbumId, ArtistId, CatalogChange, ChangeOp, RecordId, TrackId};
    use gunmetal_core::id::{IdKind, PublicId};
    use gunmetal_core::schema::{Column, DataClass, SchemaPart};
    use gunmetal_fs::dataroot::{DataRoot, Policy};
    use gunmetal_fs::host::HostFacts;
    use gunmetal_fs::sqlite::{Db, DbError, Query, Row, Value};
    use gunmetal_testkit::tempdir::TempDir;
    use proptest::collection::vec;
    use proptest::prelude::*;

    use super::{
        CHANGELOG, Cursor, Entry, LogError, Page, Resnapshot, Seq, append, compact, head, since,
    };
    use crate::readers::{self, Reader};
    use crate::schema;
    use crate::store::{Build, Generation, StoreError};
    use crate::writer::{self, Transaction};

    /// The generation a test's first build gets.
    const FIRST: Generation = Generation([1; 16]);

    /// The generation a test's later opens offer.
    const SECOND: Generation = Generation([2; 16]);

    /// More pages than any test's log holds. A read that has not ended by
    /// then fails instead of hanging.
    const CEILING: usize = 64;

    /// A table standing in for the catalogue's, so a test can change a
    /// record and log the change in one transaction.
    const ITEMS: SchemaPart = SchemaPart {
        name: "items",
        sql: "CREATE TABLE items (name TEXT NOT NULL) STRICT;",
        columns: &[Column {
            table: "items",
            name: "name",
            class: DataClass::Library,
        }],
    };

    const ADD_ITEM: Query = Query::new("INSERT INTO items (name) VALUES (?1)");
    const ITEM_NAMES: Query = Query::new("SELECT name FROM items ORDER BY rowid");

    /// A table that numbers its own rows with `AUTOINCREMENT`, as another
    /// part of the cache may, so that SQLite's `sqlite_sequence` table
    /// holds a row beside the log's.
    const COUNTED: SchemaPart = SchemaPart {
        name: "counted",
        sql: "CREATE TABLE counted (id INTEGER PRIMARY KEY AUTOINCREMENT) STRICT;",
        columns: &[Column {
            table: "counted",
            name: "id",
            class: DataClass::Library,
        }],
    };

    const COUNT: Query = Query::new("INSERT INTO counted DEFAULT VALUES");

    /// Writes a row to the log as it is given, whether or not it is a
    /// change.
    const ADD_ROW: Query = Query::new("INSERT INTO changelog (record, op) VALUES (?1, ?2)");

    /// A data root in a temporary directory.
    struct Data {
        root: DataRoot,
        // Dropped last, after the root and every connection.
        _dir: TempDir,
    }

    impl Data {
        fn new() -> Self {
            let dir = TempDir::new("changelog").expect("a temporary directory");
            let host = HostFacts::probe(dir.path()).expect("probe the host");
            let root = DataRoot::open(dir.path(), &host, Policy::DEFAULT)
                .expect("the data root opens")
                .root;
            Self { root, _dir: dir }
        }

        /// Opens the cache as the store does: built, kept or rebuilt from
        /// `parts`, with the writer's connection and one of the pool's
        /// read-only connections.
        fn open(&self, parts: &[SchemaPart], generation: Generation) -> Log {
            let schema = schema::register(parts).expect("the parts register");
            let (writer, build, generation) =
                schema::settle(&self.root, &schema, generation).expect("the cache opens");
            let reader = readers::connect(&self.root)
                .expect("the readers connect")
                .pop()
                .expect("a reader");
            Log {
                reader,
                writer,
                build,
                generation,
            }
        }
    }

    /// An open cache: real SQLite connections to a file in a temporary
    /// directory, used as the store's writer and readers use theirs.
    struct Log {
        // Closed first, so the writer's close moves the log into the file.
        reader: Db,
        writer: Db,
        build: Build,
        generation: Generation,
    }

    impl Log {
        /// Runs `work` in one write transaction, as the store's writer does.
        fn write<R>(
            &self,
            work: impl FnOnce(&Transaction<'_>) -> Result<R, StoreError>,
        ) -> Result<R, StoreError> {
            writer::transact(&self.writer, work)
        }

        /// Runs `work` in one read transaction on the read-only connection,
        /// as one of the store's readers does.
        fn read<R>(&self, work: impl FnOnce(&Reader<'_>) -> R) -> R {
            readers::snapshot(&self.reader, work).expect("the read transaction")
        }

        /// Appends `changes` in one transaction.
        fn append(&self, changes: &[CatalogChange]) -> Result<(), StoreError> {
            self.write(|tx| changes.iter().try_for_each(|change| append(tx, change)))
        }

        fn compact(&self, horizon: u64) -> Result<usize, StoreError> {
            self.write(|tx| compact(tx, Seq(horizon)))
        }

        fn head(&self) -> Result<Cursor, LogError> {
            self.read(head)
        }

        /// The page of at most `limit` changes after `cursor`.
        fn page(&self, cursor: Cursor, limit: u32) -> Result<Page, LogError> {
            let limit = NonZeroU32::new(limit).expect("a page size above zero");
            self.read(|reader| since(reader, cursor, limit))
        }

        /// The page of at most `limit` changes after number `after` of this
        /// build.
        fn since(&self, after: u64, limit: u32) -> Result<Page, LogError> {
            self.page(at(self.generation, after), limit)
        }

        /// Runs one statement of the test's own on the writer, as something
        /// other than this module would.
        fn tamper(&self, statement: &'static str) {
            self.write(|tx| tx.execute(&Query::new(statement)).map_err(StoreError::from))
                .expect("the statement runs");
        }
    }

    fn text(value: &str) -> Value {
        Value::Text(value.to_owned())
    }

    /// The public identifier of kind `kind` that `prefix` and `number`
    /// spell.
    fn id(kind: IdKind, prefix: &str, number: u64) -> PublicId {
        PublicId::parse(&format!("{prefix}_{number:026}"), kind).expect("a public identifier")
    }

    fn track(number: u64) -> RecordId {
        RecordId::Track(TrackId::new(id(IdKind::Track, "trk", number)).expect("a track's"))
    }

    fn album(number: u64) -> RecordId {
        RecordId::Album(AlbumId::new(id(IdKind::Album, "alb", number)).expect("an album's"))
    }

    fn artist(number: u64) -> RecordId {
        RecordId::Artist(ArtistId::new(id(IdKind::Artist, "art", number)).expect("an artist's"))
    }

    fn upsert(record: RecordId) -> CatalogChange {
        CatalogChange {
            record,
            op: ChangeOp::Upsert,
        }
    }

    fn removal(record: RecordId) -> CatalogChange {
        CatalogChange {
            record,
            op: ChangeOp::Removal,
        }
    }

    /// A change unlike its neighbours: its record's kind and what happened
    /// to the record both follow `number`.
    fn change(number: u64) -> CatalogChange {
        let record = match number % 3 {
            0 => track(number),
            1 => album(number),
            _ => artist(number),
        };
        if number % 2 == 0 {
            upsert(record)
        } else {
            removal(record)
        }
    }

    fn entry(seq: u64, change: CatalogChange) -> Entry {
        Entry {
            seq: Seq(seq),
            change,
        }
    }

    fn at(generation: Generation, seq: u64) -> Cursor {
        Cursor {
            generation,
            seq: Seq(seq),
        }
    }

    /// An upsert of track `n` for each `n` in `numbers`.
    fn tracks(numbers: RangeInclusive<u64>) -> Vec<CatalogChange> {
        numbers.map(|number| upsert(track(number))).collect()
    }

    /// The entries [`tracks`] makes when they are the first changes a log
    /// records: the upsert of track `n` is change `n`.
    fn track_entries(numbers: RangeInclusive<u64>) -> Vec<Entry> {
        numbers
            .map(|number| entry(number, upsert(track(number))))
            .collect()
    }

    fn other_generation(cursor: Generation, cache: Generation) -> LogError {
        LogError::Resnapshot(Resnapshot::Generation { cursor, cache })
    }

    fn compacted(cursor: u64, horizon: u64) -> LogError {
        LogError::Resnapshot(Resnapshot::Compacted {
            cursor: Seq(cursor),
            horizon: Seq(horizon),
        })
    }

    fn ahead(cursor: u64, head: u64) -> LogError {
        LogError::Resnapshot(Resnapshot::Ahead {
            cursor: Seq(cursor),
            head: Seq(head),
        })
    }

    /// Adds an item named `name`, as a change to the catalogue would. An
    /// item with no name is refused.
    fn add_item(tx: &Transaction<'_>, name: Option<&str>) -> Result<(), StoreError> {
        let name = name.map_or(Value::Null, text);
        tx.execute(&ADD_ITEM.bind(name))
            .map(|_| ())
            .map_err(StoreError::from)
    }

    /// The items' names in the order they were added, as a reader sees
    /// them.
    fn items(log: &Log) -> Vec<Row> {
        log.read(|reader| reader.query(&ITEM_NAMES))
            .expect("the items read")
    }

    /// The rows [`items`] returns for items named `names`.
    fn named(names: &[&str]) -> Vec<Row> {
        names.iter().map(|name| Row(vec![text(name)])).collect()
    }

    /// Writes `record` and `op` to the log as its next row.
    fn add_row(log: &Log, record: &str, op: i64) {
        let row = ADD_ROW.bind(text(record)).bind(Value::Integer(op));
        assert_eq!(
            log.write(|tx| tx.execute(&row).map_err(StoreError::from)),
            Ok(1)
        );
    }

    /// Offers `record` and `op` to the log's table as its next row, and
    /// returns what the table answered.
    fn offer_row(log: &Log, record: &str, op: i64) -> Result<usize, StoreError> {
        let row = ADD_ROW.bind(text(record)).bind(Value::Integer(op));
        log.write(|tx| tx.execute(&row).map_err(StoreError::from))
    }

    /// Replaces the log's table with one of the same columns and types and
    /// none of its checks, so that a test can plant a row the table would
    /// refuse. Damage could leave such a row, and so could a writer that
    /// is not this build, in a cache whose record of its build still
    /// matches.
    fn drop_the_checks(log: &Log) {
        log.tamper("DROP TABLE changelog");
        log.tamper(
            "CREATE TABLE changelog (seq INTEGER PRIMARY KEY AUTOINCREMENT, \
             record TEXT NOT NULL, op INTEGER NOT NULL) STRICT",
        );
    }

    /// Adds `rows` rows to [`COUNTED`]'s table, each numbered by SQLite.
    fn count(log: &Log, rows: usize) {
        for _ in 0..rows {
            assert_eq!(
                log.write(|tx| tx.execute(&COUNT).map_err(StoreError::from)),
                Ok(1)
            );
        }
    }

    /// Every page one read returned, the empty one last, and the cursor
    /// they end at.
    type Pages = (Vec<Vec<Entry>>, Cursor);

    /// Reads page after page of `limit` from `cursor` until one comes back
    /// empty.
    fn read_pages(log: &Log, cursor: Cursor, limit: u32) -> Result<Pages, LogError> {
        let mut pages = Vec::new();
        let mut next = cursor;
        let mut ended = false;
        while !ended {
            assert!(
                pages.len() < CEILING,
                "the log did not end within {CEILING} pages"
            );
            let page = log.page(next, limit)?;
            ended = page.entries.is_empty();
            next = page.next;
            pages.push(page.entries);
        }
        Ok((pages, next))
    }

    #[test]
    fn numbers_the_changes_in_the_order_they_were_appended() {
        let data = Data::new();
        let log = data.open(&[CHANGELOG], FIRST);
        assert_eq!((log.build, log.generation), (Build::Fresh, FIRST));
        assert_eq!(log.head(), Ok(at(FIRST, 0)));
        assert_eq!(
            log.since(0, 10),
            Ok(Page {
                entries: vec![],
                next: at(FIRST, 0),
            })
        );
        assert_eq!(log.append(&[upsert(track(7)), removal(album(8))]), Ok(()));
        assert_eq!(log.append(&[upsert(artist(9))]), Ok(()));
        assert_eq!(log.append(&[removal(track(7))]), Ok(()));
        assert_eq!(
            log.since(0, 10),
            Ok(Page {
                entries: vec![
                    entry(1, upsert(track(7))),
                    entry(2, removal(album(8))),
                    entry(3, upsert(artist(9))),
                    entry(4, removal(track(7))),
                ],
                next: at(FIRST, 4),
            })
        );
        assert_eq!(log.head(), Ok(at(FIRST, 4)));
    }

    #[test]
    fn a_change_and_its_log_entry_commit_or_roll_back_together() {
        let data = Data::new();
        let log = data.open(&[CHANGELOG, ITEMS], FIRST);
        let one = Ok(Page {
            entries: vec![entry(1, upsert(track(1)))],
            next: at(FIRST, 1),
        });
        // SQLITE_CONSTRAINT_NOTNULL: an item must have a name.
        let refused = Err(StoreError::Db(DbError::Sqlite { code: 1299 }));

        // Written in one transaction, both are there once it commits.
        assert_eq!(
            log.write(|tx| {
                add_item(tx, Some("Intro")).and_then(|()| append(tx, &upsert(track(1))))
            }),
            Ok(())
        );
        assert_eq!(items(&log), named(&["Intro"]));
        assert_eq!(log.since(0, 10), one);

        // The entry is written and then its change fails: neither is kept.
        assert_eq!(
            log.write(|tx| append(tx, &upsert(track(2))).and_then(|()| add_item(tx, None))),
            refused
        );
        assert_eq!(items(&log), named(&["Intro"]));
        assert_eq!(log.since(0, 10), one);

        // A change and its entry are written and a later statement fails:
        // neither is kept.
        assert_eq!(
            log.write(|tx| {
                add_item(tx, Some("Verse"))
                    .and_then(|()| append(tx, &upsert(track(2))))
                    .and_then(|()| add_item(tx, None))
            }),
            refused
        );
        assert_eq!(items(&log), named(&["Intro"]));
        assert_eq!(log.since(0, 10), one);
        assert_eq!(log.head(), Ok(at(FIRST, 1)));

        // The numbers the failed writes took are given out again, so the
        // log has no gap.
        assert_eq!(
            log.write(|tx| {
                add_item(tx, Some("Verse")).and_then(|()| append(tx, &removal(album(2))))
            }),
            Ok(())
        );
        assert_eq!(items(&log), named(&["Intro", "Verse"]));
        assert_eq!(
            log.since(1, 10),
            Ok(Page {
                entries: vec![entry(2, removal(album(2)))],
                next: at(FIRST, 2),
            })
        );
    }

    #[test]
    fn pages_are_contiguous_and_never_repeat() {
        let data = Data::new();
        let log = data.open(&[CHANGELOG], FIRST);
        assert_eq!(log.append(&tracks(1..=7)), Ok(()));
        let page = |numbers: RangeInclusive<u64>| -> Result<Page, LogError> {
            let next = at(FIRST, *numbers.end());
            Ok(Page {
                entries: track_entries(numbers),
                next,
            })
        };
        // Each page starts where the one before it ended.
        assert_eq!(log.since(0, 3), page(1..=3));
        assert_eq!(log.since(3, 3), page(4..=6));
        assert_eq!(log.since(6, 3), page(7..=7));
        // After the newest change there is nothing, and the cursor stays.
        assert_eq!(
            log.since(7, 3),
            Ok(Page {
                entries: vec![],
                next: at(FIRST, 7),
            })
        );
        // A page of one, a page that ends exactly at the newest change, and
        // pages with room to spare.
        assert_eq!(log.since(2, 1), page(3..=3));
        assert_eq!(log.since(4, 3), page(5..=7));
        assert_eq!(log.since(5, 500), page(6..=7));
        assert_eq!(log.since(0, u32::MAX), page(1..=7));
    }

    #[test]
    fn a_cursor_from_another_generation_is_refused() {
        let data = Data::new();
        let log = data.open(&[CHANGELOG], FIRST);
        assert_eq!(log.append(&tracks(1..=2)), Ok(()));
        // Whatever its number: before the log, inside it, at its end or
        // past it.
        for number in [0, 1, 2, 3, u64::MAX] {
            assert_eq!(
                log.page(at(SECOND, number), 10),
                Err(other_generation(SECOND, FIRST)),
                "number {number}"
            );
        }
        assert_eq!(
            log.since(1, 10),
            Ok(Page {
                entries: track_entries(2..=2),
                next: at(FIRST, 2),
            })
        );
    }

    #[test]
    fn a_rebuilt_cache_refuses_the_cursors_of_the_one_it_replaced() {
        let data = Data::new();
        let first = data.open(&[CHANGELOG], FIRST);
        assert_eq!(first.append(&tracks(1..=2)), Ok(()));
        assert_eq!(first.head(), Ok(at(FIRST, 2)));
        drop(first);

        // Another part changes the schema's digest, so the cache is
        // discarded and built again with the generation offered.
        let rebuilt = data.open(&[CHANGELOG, ITEMS], SECOND);
        assert_eq!(
            (rebuilt.build, rebuilt.generation),
            (Build::Rebuilt, SECOND)
        );
        assert_eq!(
            rebuilt.page(at(FIRST, 2), 10),
            Err(other_generation(FIRST, SECOND))
        );
        // The new build numbers its changes from 1 again.
        assert_eq!(rebuilt.head(), Ok(at(SECOND, 0)));
        assert_eq!(rebuilt.append(&[removal(artist(5))]), Ok(()));
        assert_eq!(
            rebuilt.since(0, 10),
            Ok(Page {
                entries: vec![entry(1, removal(artist(5)))],
                next: at(SECOND, 1),
            })
        );
    }

    #[test]
    fn a_reopened_cache_keeps_its_log_its_horizon_and_its_cursors() {
        let data = Data::new();
        let first = data.open(&[CHANGELOG], FIRST);
        assert_eq!(first.append(&tracks(1..=3)), Ok(()));
        assert_eq!(first.compact(1), Ok(1));
        drop(first);

        let again = data.open(&[CHANGELOG], SECOND);
        assert_eq!((again.build, again.generation), (Build::Reused, FIRST));
        assert_eq!(again.head(), Ok(at(FIRST, 3)));
        assert_eq!(
            again.since(1, 10),
            Ok(Page {
                entries: track_entries(2..=3),
                next: at(FIRST, 3),
            })
        );
        assert_eq!(again.since(0, 10), Err(compacted(0, 1)));
        // A cursor of the generation that was offered and not used is
        // another build's.
        assert_eq!(
            again.page(at(SECOND, 3), 10),
            Err(other_generation(SECOND, FIRST))
        );
        assert_eq!(again.append(&tracks(4..=4)), Ok(()));
        assert_eq!(
            again.since(3, 10),
            Ok(Page {
                entries: track_entries(4..=4),
                next: at(FIRST, 4),
            })
        );
    }

    #[test]
    fn a_cursor_older_than_the_horizon_is_told_to_resnapshot() {
        let data = Data::new();
        let log = data.open(&[CHANGELOG], FIRST);
        assert_eq!(log.append(&tracks(1..=5)), Ok(()));
        assert_eq!(log.compact(3), Ok(3));
        for number in [0, 1, 2] {
            assert_eq!(
                log.since(number, 10),
                Err(compacted(number, 3)),
                "number {number}"
            );
        }
        // A cursor at the horizon has seen everything that was removed.
        assert_eq!(
            log.since(3, 10),
            Ok(Page {
                entries: track_entries(4..=5),
                next: at(FIRST, 5),
            })
        );
        assert_eq!(
            log.since(4, 10),
            Ok(Page {
                entries: track_entries(5..=5),
                next: at(FIRST, 5),
            })
        );
        assert_eq!(log.head(), Ok(at(FIRST, 5)));

        // The horizon does not move back: compacting to an older place, or
        // to the same one again, removes nothing.
        assert_eq!(log.compact(2), Ok(0));
        assert_eq!(log.compact(3), Ok(0));
        assert_eq!(log.since(2, 10), Err(compacted(2, 3)));
        assert_eq!(
            log.since(3, 1),
            Ok(Page {
                entries: track_entries(4..=4),
                next: at(FIRST, 4),
            })
        );
    }

    #[test]
    fn compacting_past_the_newest_change_removes_them_all_and_keeps_the_numbering() {
        let data = Data::new();
        let log = data.open(&[CHANGELOG], FIRST);
        // Nothing to remove in an empty log, and its horizon stays at 0.
        assert_eq!(log.compact(9), Ok(0));
        assert_eq!(
            log.since(0, 10),
            Ok(Page {
                entries: vec![],
                next: at(FIRST, 0),
            })
        );

        assert_eq!(log.append(&tracks(1..=2)), Ok(()));
        assert_eq!(log.compact(u64::MAX), Ok(2));
        // The horizon is the newest change, not the place asked for.
        assert_eq!(log.head(), Ok(at(FIRST, 2)));
        assert_eq!(log.since(1, 10), Err(compacted(1, 2)));
        assert_eq!(
            log.since(2, 10),
            Ok(Page {
                entries: vec![],
                next: at(FIRST, 2),
            })
        );

        // The next change gets the next number, never one given out before.
        assert_eq!(log.append(&[removal(album(6))]), Ok(()));
        assert_eq!(log.head(), Ok(at(FIRST, 3)));
        assert_eq!(
            log.since(2, 10),
            Ok(Page {
                entries: vec![entry(3, removal(album(6)))],
                next: at(FIRST, 3),
            })
        );
    }

    #[test]
    fn a_cursor_past_the_newest_change_is_refused() {
        let data = Data::new();
        let log = data.open(&[CHANGELOG], FIRST);
        assert_eq!(log.since(1, 10), Err(ahead(1, 0)));
        assert_eq!(log.append(&tracks(1..=2)), Ok(()));
        for number in [3, 4, u64::MAX] {
            assert_eq!(
                log.since(number, 10),
                Err(ahead(number, 2)),
                "number {number}"
            );
        }
        assert_eq!(
            log.since(2, 10),
            Ok(Page {
                entries: vec![],
                next: at(FIRST, 2),
            })
        );
        // Compaction does not make a place that was never given out good.
        assert_eq!(log.compact(2), Ok(2));
        assert_eq!(log.since(3, 10), Err(ahead(3, 2)));
    }

    /// SQLite keeps one row in `sqlite_sequence` for each table that
    /// numbers its rows with `AUTOINCREMENT`, and the log's head is its own
    /// row's number. Here another table is always ahead of the log, so a
    /// head read from any row but the log's would be the other table's
    /// number.
    #[test]
    fn the_head_is_the_logs_own_number_whatever_another_table_counts() {
        let data = Data::new();
        let log = data.open(&[CHANGELOG, COUNTED], FIRST);

        // The other table has numbered three rows and the log none: the
        // head is still the place before every change.
        count(&log, 3);
        assert_eq!(log.head(), Ok(at(FIRST, 0)));
        assert_eq!(
            log.since(0, 10),
            Ok(Page {
                entries: vec![],
                next: at(FIRST, 0),
            })
        );
        assert_eq!(log.since(3, 10), Err(ahead(3, 0)));

        // The log records two changes while the other table reaches five.
        assert_eq!(log.append(&tracks(1..=2)), Ok(()));
        count(&log, 2);
        assert_eq!(log.head(), Ok(at(FIRST, 2)));
        assert_eq!(
            log.since(0, 10),
            Ok(Page {
                entries: track_entries(1..=2),
                next: at(FIRST, 2),
            })
        );
        // The other table's numbers are past the newest change.
        for number in [3, 5] {
            assert_eq!(
                log.since(number, 10),
                Err(ahead(number, 2)),
                "number {number}"
            );
        }

        // With every change removed the horizon is the log's own head too.
        assert_eq!(log.compact(5), Ok(2));
        assert_eq!(log.head(), Ok(at(FIRST, 2)));
        assert_eq!(log.since(1, 10), Err(compacted(1, 2)));
        assert_eq!(
            log.since(2, 10),
            Ok(Page {
                entries: vec![],
                next: at(FIRST, 2),
            })
        );

        // The next change takes the log's next number, not the other
        // table's.
        assert_eq!(log.append(&[removal(album(6))]), Ok(()));
        assert_eq!(
            log.since(2, 10),
            Ok(Page {
                entries: vec![entry(3, removal(album(6)))],
                next: at(FIRST, 3),
            })
        );
    }

    /// A cursor is a place and nothing more: a generation and a number,
    /// with no principal, library or filter in it, so there is nothing in
    /// one to trust for who may see what. The log checks both against the
    /// cache's own record on every read. A cursor this build could not
    /// have given out is refused, never clamped or answered with a guess,
    /// and a place it could have given out reads the same page whoever
    /// sends it.
    ///
    /// Supports SEC-API-025 and does not prove it, so this comment has no
    /// line the traceability check counts. The requirement asks for
    /// cursors that are opaque or MAC-protected, for a changed cursor to be
    /// refused, and for a cursor replayed by another principal to return
    /// only that principal's data. The log has no wire form and no
    /// principal, and the end of this test shows it serving a number a
    /// device wrote itself. The requirement is proved where cursors get
    /// their wire form: the route that hands them to devices.
    #[test]
    fn a_cursor_holds_only_a_place_and_one_the_log_did_not_give_out_is_refused() {
        let data = Data::new();
        let log = data.open(&[CHANGELOG], FIRST);
        assert_eq!(log.append(&tracks(1..=5)), Ok(()));
        assert_eq!(log.compact(2), Ok(2));

        // Every field of a cursor is named here, so a cursor that held
        // anything more would not compile.
        let Cursor { generation, seq } = log.head().expect("the log's place");
        assert_eq!((generation, seq), (FIRST, Seq(5)));

        // The generation is the cache's own or the cursor is refused: a
        // change to any one bit of it is, at every number.
        for bit in 0..128_usize {
            let mut forged = FIRST.0;
            forged[bit / 8] ^= 1 << (bit % 8);
            for number in [0, 2, 3, 5, 6] {
                assert_eq!(
                    log.page(at(Generation(forged), number), 10),
                    Err(other_generation(Generation(forged), FIRST)),
                    "bit {bit}, number {number}"
                );
            }
        }

        // The number is one this build gave out and can still read from, or
        // the cursor is refused.
        for number in [0, 1] {
            assert_eq!(log.since(number, 10), Err(compacted(number, 2)));
        }
        for number in [6, 7, u64::MAX] {
            assert_eq!(log.since(number, 10), Err(ahead(number, 5)));
        }

        // Any number in between is a place the log gave out, and it reads
        // that place's page and no more, whether the log returned the
        // cursor or a device wrote it.
        assert_eq!(
            log.since(2, 1),
            Ok(Page {
                entries: track_entries(3..=3),
                next: at(FIRST, 3),
            })
        );
        for number in [3, 4, 5] {
            assert_eq!(
                log.since(number, 10),
                Ok(Page {
                    entries: track_entries(number + 1..=5),
                    next: at(FIRST, 5),
                }),
                "number {number}"
            );
        }
    }

    /// What the log should hold, kept by the test: every change ever
    /// appended, in order, and the horizon.
    struct Model {
        entries: Vec<Entry>,
        horizon: u64,
    }

    impl Model {
        /// The number of the newest change.
        fn head(&self) -> u64 {
            u64::try_from(self.entries.len()).expect("a count that fits")
        }

        /// Appends `changes`, numbering them on from the newest.
        fn append(&mut self, changes: &[CatalogChange]) {
            for change in changes {
                let number = self.head() + 1;
                self.entries.push(entry(number, *change));
            }
        }

        /// Compacts to `target` and returns how many changes that removes.
        fn compact(&mut self, target: u64) -> usize {
            let before = self.horizon;
            self.horizon = before.max(target.min(self.head()));
            usize::try_from(self.horizon - before).expect("a count that fits")
        }

        /// What reading every page of `limit` after number `cursor` should
        /// give, or why the cursor should be refused.
        fn pages(&self, cursor: u64, limit: u32) -> Result<Pages, LogError> {
            if cursor > self.head() {
                return Err(ahead(cursor, self.head()));
            }
            if cursor < self.horizon {
                return Err(compacted(cursor, self.horizon));
            }
            let rest: Vec<Entry> = self
                .entries
                .iter()
                .copied()
                .filter(|held| held.seq > Seq(cursor))
                .collect();
            let size = usize::try_from(limit).expect("a page size that fits");
            let pages = rest
                .chunks(size)
                .map(<[Entry]>::to_vec)
                .chain(iter::once(Vec::new()))
                .collect();
            Ok((pages, at(FIRST, self.head())))
        }
    }

    /// Where one step of the sweep compacts to, if it compacts.
    enum Compaction {
        Skip,
        BelowHorizon,
        Middle,
        Head,
        PastHead,
    }

    /// Reads from every place worth trying, in several page sizes, and
    /// compares what comes back with `model`.
    fn check_every_place(log: &Log, model: &Model) {
        let places = [
            model.horizon.saturating_sub(1),
            model.horizon,
            u64::midpoint(model.horizon, model.head()),
            model.head(),
            model.head() + 1,
        ];
        for place in places {
            for limit in [1, 2, 7] {
                assert_eq!(
                    read_pages(log, at(FIRST, place), limit),
                    model.pages(place, limit),
                    "from {place} in pages of {limit}"
                );
            }
        }
    }

    /// The plan's property, as a sweep: after every mix of appends of
    /// several sizes and compactions to every kind of place, the pages read
    /// from a cursor, joined end to end, are exactly the changes after it,
    /// each page full but the last, or the cursor is refused for the reason
    /// the model gives. The model shares no code with the log.
    #[test]
    fn concatenated_pages_equal_the_log_after_every_mix_of_appends_and_compactions() {
        let data = Data::new();
        let log = data.open(&[CHANGELOG], FIRST);
        let mut model = Model {
            entries: Vec::new(),
            horizon: 0,
        };
        for _ in 0..2 {
            for count in [1, 2, 3, 0] {
                for compaction in [
                    Compaction::Skip,
                    Compaction::BelowHorizon,
                    Compaction::Middle,
                    Compaction::Head,
                    Compaction::PastHead,
                ] {
                    let fresh: Vec<CatalogChange> = (0..count)
                        .map(|offset| change(model.head() + offset + 1))
                        .collect();
                    assert_eq!(log.append(&fresh), Ok(()));
                    model.append(&fresh);
                    let target = match compaction {
                        Compaction::Skip => None,
                        Compaction::BelowHorizon => Some(model.horizon.saturating_sub(1)),
                        Compaction::Middle => Some(u64::midpoint(model.horizon, model.head())),
                        Compaction::Head => Some(model.head()),
                        Compaction::PastHead => Some(model.head() + 3),
                    };
                    if let Some(target) = target {
                        assert_eq!(log.compact(target), Ok(model.compact(target)));
                    }
                    assert_eq!(log.head(), Ok(at(FIRST, model.head())));
                    check_every_place(&log, &model);
                }
            }
        }
    }

    /// The most rounds a generated history has.
    const ROUNDS: usize = 16;

    /// The most changes one round appends. A history then holds at most 48
    /// changes, which pages of one read in 49 pages: fewer than
    /// [`CEILING`].
    const MOST: u8 = 3;

    /// A place in the log, named against where the log stands when the
    /// place is used, so that every run reads from the edges and compacts
    /// to them, and not only where a number happens to fall.
    #[derive(Debug, Clone, Copy)]
    enum Place {
        /// The place before the horizon, which compaction has made too
        /// old, or the horizon itself while that is 0.
        BeforeHorizon,
        /// The oldest place the log can still be read from.
        Horizon,
        /// The newest change.
        Head,
        /// The first place past the newest change.
        PastHead,
        /// One of the places from 0 to two past the newest change, picked
        /// by this number.
        Within(u8),
        /// This number, whatever the log holds: most often one far past
        /// the newest change.
        Number(u64),
    }

    impl Place {
        /// The number the place has in a log that stands where `model`
        /// does.
        fn number(self, model: &Model) -> u64 {
            match self {
                Self::BeforeHorizon => model.horizon.saturating_sub(1),
                Self::Horizon => model.horizon,
                Self::Head => model.head(),
                Self::PastHead => model.head() + 1,
                Self::Within(pick) => u64::from(pick) % (model.head() + 3),
                Self::Number(number) => number,
            }
        }
    }

    /// One round of a history: some changes appended in one transaction,
    /// then a compaction, then a read of every page after a place. A round
    /// that appends nothing, or compacts to a place that removes nothing,
    /// leaves that step out, so the rounds of a history spell every order
    /// of the three.
    #[derive(Debug, Clone, Copy)]
    struct Round {
        /// How many changes to append.
        append: u8,
        /// Where to compact to.
        compact: Place,
        /// Where to read from.
        from: Place,
        /// The size of the pages to read.
        limit: u32,
    }

    /// Plays `round` on the log and on the model, and compares each answer
    /// of the log's with the model's.
    fn play(log: &Log, model: &mut Model, round: Round) {
        let fresh: Vec<CatalogChange> = (0..u64::from(round.append))
            .map(|offset| change(model.head() + offset + 1))
            .collect();
        assert_eq!(log.append(&fresh), Ok(()));
        model.append(&fresh);
        assert_eq!(log.head(), Ok(at(FIRST, model.head())));

        let target = round.compact.number(model);
        assert_eq!(
            log.compact(target),
            Ok(model.compact(target)),
            "compacting to {target}"
        );
        assert_eq!(log.head(), Ok(at(FIRST, model.head())));

        let from = round.from.number(model);
        let limit = round.limit;
        assert_eq!(
            read_pages(log, at(FIRST, from), limit),
            model.pages(from, limit),
            "from {from} in pages of {limit}"
        );
    }

    /// Any place: each edge by name, the places in and around the log, and
    /// numbers as they come.
    fn place() -> impl Strategy<Value = Place> {
        prop_oneof![
            1 => Just(Place::BeforeHorizon),
            1 => Just(Place::Horizon),
            1 => Just(Place::Head),
            1 => Just(Place::PastHead),
            4 => any::<u8>().prop_map(Place::Within),
            1 => any::<u64>().prop_map(Place::Number),
        ]
    }

    /// A page size: the smallest, a few that take several pages to read a
    /// log, and the largest.
    fn limit() -> impl Strategy<Value = u32> {
        prop_oneof![
            1 => Just(1_u32),
            4 => 2_u32..=9,
            1 => Just(u32::MAX),
        ]
    }

    /// Any round.
    fn round() -> impl Strategy<Value = Round> {
        (0..=MOST, place(), place(), limit()).prop_map(|(append, compact, from, limit)| Round {
            append,
            compact,
            from,
            limit,
        })
    }

    #[test]
    fn a_place_is_named_against_where_the_log_stands() {
        // A log that has recorded nine changes and compacted the first
        // four.
        let model = Model {
            entries: track_entries(1..=9),
            horizon: 4,
        };
        let numbers = [
            Place::BeforeHorizon,
            Place::Horizon,
            Place::Head,
            Place::PastHead,
            Place::Within(0),
            Place::Within(11),
            Place::Within(12),
            Place::Within(255),
            Place::Number(0),
            Place::Number(u64::MAX),
        ]
        .map(|place| place.number(&model));
        assert_eq!(numbers, [3, 4, 9, 10, 0, 11, 0, 3, 0, u64::MAX]);

        // A log that has recorded nothing has no place before its horizon.
        let empty = Model {
            entries: Vec::new(),
            horizon: 0,
        };
        let numbers = [
            Place::BeforeHorizon,
            Place::Horizon,
            Place::Head,
            Place::PastHead,
            Place::Within(2),
            Place::Within(3),
        ]
        .map(|place| place.number(&empty));
        assert_eq!(numbers, [0, 0, 0, 1, 2, 0]);
    }

    /// One history written out, as the property below generates them. Each
    /// round is compared with the model as it is played, and what the
    /// rounds leave in the log is compared here with literal values.
    #[test]
    fn a_history_is_played_on_the_log_and_on_the_model_alike() {
        let data = Data::new();
        let log = data.open(&[CHANGELOG], FIRST);
        let mut model = Model {
            entries: Vec::new(),
            horizon: 0,
        };
        let history = [
            // Three changes and no compaction, read from the start in
            // pages of two.
            Round {
                append: 3,
                compact: Place::Horizon,
                from: Place::Horizon,
                limit: 2,
            },
            // A compaction on its own, to place 2 of the six places from 0
            // to two past the head, and a read from the place it made too
            // old.
            Round {
                append: 0,
                compact: Place::Within(2),
                from: Place::BeforeHorizon,
                limit: 1,
            },
            // Two more changes, a compaction that removes nothing and a
            // read from past the head.
            Round {
                append: 2,
                compact: Place::BeforeHorizon,
                from: Place::PastHead,
                limit: u32::MAX,
            },
            // A read on its own, of everything the log still holds.
            Round {
                append: 0,
                compact: Place::Horizon,
                from: Place::Horizon,
                limit: 1,
            },
        ];
        for round in history {
            play(&log, &mut model, round);
        }
        assert_eq!((model.head(), model.horizon), (5, 2));
        assert_eq!(log.since(1, 10), Err(compacted(1, 2)));
        assert_eq!(
            log.since(2, 10),
            Ok(Page {
                entries: vec![
                    entry(3, removal(track(3))),
                    entry(4, upsert(album(4))),
                    entry(5, removal(artist(5))),
                ],
                next: at(FIRST, 5),
            })
        );
        assert_eq!(log.since(6, 10), Err(ahead(6, 5)));
    }

    proptest! {
        // Every case opens a real cache, so there are fewer cases than the
        // default 256, as in the other properties that open a file. A
        // failing run stops shrinking after four times as many steps, each
        // of which opens another cache, so it still ends well inside the
        // time the gate allows a mutant.
        #![proptest_config(ProptestConfig::with_cases(64))]

        /// The plan's property, generated: after any history of appends,
        /// compactions and reads, the pages read from any place, joined end
        /// to end, are exactly the changes after it, each page full but the
        /// last, or the place is refused for the reason the model gives.
        /// The sweep above is the worked example, and the model is the
        /// same one, which shares no code with the log.
        #[test]
        fn concatenated_pages_equal_the_log_after_any_generated_history(
            history in vec(round(), 1..=ROUNDS),
        ) {
            let data = Data::new();
            let log = data.open(&[CHANGELOG], FIRST);
            let mut model = Model {
                entries: Vec::new(),
                horizon: 0,
            };
            for round in history {
                play(&log, &mut model, round);
            }
            check_every_place(&log, &model);
        }
    }

    /// A record and an op code that together are not a change.
    const NOT_CHANGES: [(&str, i64); 12] = [
        // An op code this build does not know, on either side of the two
        // it does.
        ("trk_00000000000000000000000001", 0),
        ("trk_00000000000000000000000001", 3),
        // An op code that is no code at all. Cut to eight bits, 257 and 258
        // would read as the two codes there are.
        ("trk_00000000000000000000000001", -1),
        ("trk_00000000000000000000000001", 257),
        ("trk_00000000000000000000000001", 258),
        ("trk_00000000000000000000000001", i64::MAX),
        // An identifier of a kind the log does not hold.
        ("lib_00000000000000000000000001", 1),
        ("pls_00000000000000000000000001", 2),
        // Not an identifier.
        ("", 1),
        ("trk_1", 2),
        ("TRK_00000000000000000000000001", 1),
        ("trk_00000000000000000000000001 ", 2),
    ];

    #[test]
    fn a_row_that_is_not_a_change_is_reported_with_what_it_holds() {
        let data = Data::new();
        let log = data.open(&[CHANGELOG], FIRST);
        // The table's own checks refuse most of these rows, so they are
        // planted in a table without them.
        drop_the_checks(&log);
        for (before, (record, op)) in (0_u64..).zip(NOT_CHANGES) {
            add_row(&log, record, op);
            let number = i64::try_from(before + 1).expect("a small number");
            let found = Row(vec![
                Value::Integer(number),
                text(record),
                Value::Integer(op),
            ]);
            assert_eq!(
                log.since(before, 1),
                Err(LogError::Unreadable { found: vec![found] }),
                "{record:?} with op {op}"
            );
        }
    }

    #[test]
    fn one_row_that_is_not_a_change_fails_its_page_and_no_other() {
        let data = Data::new();
        let log = data.open(&[CHANGELOG], FIRST);
        // The table's own checks refuse a code of 7, so the row is planted
        // in a table without them.
        drop_the_checks(&log);
        assert_eq!(log.append(&tracks(1..=2)), Ok(()));
        add_row(&log, "trk_00000000000000000000000003", 7);
        assert_eq!(log.append(&tracks(4..=4)), Ok(()));
        let found = Row(vec![
            Value::Integer(3),
            text("trk_00000000000000000000000003"),
            Value::Integer(7),
        ]);
        let refused = Err(LogError::Unreadable { found: vec![found] });
        // The pages that end before the row and start after it read.
        assert_eq!(
            log.since(0, 2),
            Ok(Page {
                entries: track_entries(1..=2),
                next: at(FIRST, 2),
            })
        );
        assert_eq!(
            log.since(3, 10),
            Ok(Page {
                entries: track_entries(4..=4),
                next: at(FIRST, 4),
            })
        );
        // A page that would hold it gives none of its changes.
        assert_eq!(log.since(0, 3), refused);
        assert_eq!(log.since(2, 10), refused);
        assert_eq!(log.since(0, 10), refused);
    }

    /// Every kind of public identifier, with its prefix and whether the
    /// log records changes to records of that kind.
    const KINDS: [(IdKind, &str, bool); 12] = [
        (IdKind::Track, "trk", true),
        (IdKind::Album, "alb", true),
        (IdKind::Artist, "art", true),
        (IdKind::ReleaseGroup, "rgp", false),
        (IdKind::Playlist, "pls", false),
        (IdKind::User, "usr", false),
        (IdKind::Profile, "prf", false),
        (IdKind::Device, "dev", false),
        (IdKind::Library, "lib", false),
        (IdKind::Invite, "inv", false),
        (IdKind::Share, "shr", false),
        (IdKind::Token, "tok", false),
    ];

    /// Codes on both sides of the two the log stores, and far from them,
    /// with whether the table takes each.
    const CODES: [(i64, bool); 9] = [
        (i64::MIN, false),
        (-1, false),
        (0, false),
        (1, true),
        (2, true),
        (3, false),
        (257, false),
        (258, false),
        (i64::MAX, false),
    ];

    /// Text that starts as no kind of identifier does.
    const NO_KIND: [&str; 5] = [
        "",
        "trk",
        "TRK_00000000000000000000000001",
        " trk_00000000000000000000000001",
        "track_0000000000000000000000001",
    ];

    /// The kinds of record and the codes a row may hold are in the table's
    /// own checks, so the table refuses a row of any other kind or code
    /// when it is written, and a build that logs other kinds or codes
    /// creates the table with other text, which is another schema digest
    /// and so another cache.
    #[test]
    fn the_table_takes_only_the_kinds_and_codes_this_build_logs() {
        let data = Data::new();
        let log = data.open(&[CHANGELOG], FIRST);
        // SQLITE_CONSTRAINT_CHECK: the row fails one of the table's checks.
        let refused: Result<usize, StoreError> = Err(StoreError::Db(DbError::Sqlite { code: 275 }));

        // The list holds every kind there is, so a kind added to the core
        // fails here until it is decided whether the log records it.
        assert_eq!(KINDS.map(|(kind, _, _)| kind), IdKind::ALL);
        for (kind, prefix, logged) in KINDS {
            let record = id(kind, prefix, 1).to_string();
            let expected = if logged { Ok(1) } else { refused.clone() };
            assert_eq!(offer_row(&log, &record, 1), expected, "{record}");
        }
        for record in NO_KIND {
            assert_eq!(offer_row(&log, record, 1), refused, "{record:?}");
        }
        for (code, known) in CODES {
            let expected = if known { Ok(1) } else { refused.clone() };
            assert_eq!(
                offer_row(&log, "trk_00000000000000000000000002", code),
                expected,
                "code {code}"
            );
        }

        // A refused row leaves nothing and takes no number: the log holds
        // the five rows the table took, numbered in the order they came.
        assert_eq!(
            log.since(0, 10),
            Ok(Page {
                entries: vec![
                    entry(1, upsert(track(1))),
                    entry(2, upsert(album(1))),
                    entry(3, upsert(artist(1))),
                    entry(4, upsert(track(2))),
                    entry(5, removal(track(2))),
                ],
                next: at(FIRST, 5),
            })
        );

        // Every change the core can name is one the table takes.
        for op in ChangeOp::ALL {
            for record in [track(3), album(3), artist(3)] {
                assert_eq!(log.append(&[CatalogChange { record, op: *op }]), Ok(()));
            }
        }
        assert_eq!(log.head(), Ok(at(FIRST, 11)));
    }

    /// The table's checks name kinds and codes, not the whole form of an
    /// identifier. A row they take can still be no change, and reading it
    /// says so.
    #[test]
    fn a_row_the_table_takes_that_is_not_a_change_is_still_unreadable() {
        let data = Data::new();
        let log = data.open(&[CHANGELOG], FIRST);
        let rows = [
            ("trk_", 1),
            ("alb_1", 2),
            ("art_00000000000000000000000001 ", 1),
            ("trk_0000000000000000000000000i", 2),
            ("alb_80000000000000000000000000", 1),
        ];
        for (before, (record, op)) in (0_u64..).zip(rows) {
            add_row(&log, record, op);
            let number = i64::try_from(before + 1).expect("a small number");
            let found = Row(vec![
                Value::Integer(number),
                text(record),
                Value::Integer(op),
            ]);
            assert_eq!(
                log.since(before, 1),
                Err(LogError::Unreadable { found: vec![found] }),
                "{record:?} with op {op}"
            );
        }
    }

    #[test]
    fn a_log_whose_table_holds_other_types_is_unreadable() {
        let data = Data::new();
        let log = data.open(&[CHANGELOG], FIRST);
        // The same table without its types, so a record can be a number.
        log.tamper("DROP TABLE changelog");
        log.tamper("CREATE TABLE changelog (seq INTEGER PRIMARY KEY AUTOINCREMENT, record, op)");
        log.tamper("INSERT INTO changelog (record, op) VALUES (7, 1)");
        let found = Row(vec![
            Value::Integer(1),
            Value::Integer(7),
            Value::Integer(1),
        ]);
        assert_eq!(
            log.since(0, 10),
            Err(LogError::Unreadable { found: vec![found] })
        );
    }

    /// What a log holding one change answers for its head and for a read
    /// from the start, once `statement` has been run on its cache.
    fn after_tampering(
        statement: &'static str,
    ) -> (Result<Cursor, LogError>, Result<Page, LogError>) {
        let data = Data::new();
        let log = data.open(&[CHANGELOG], FIRST);
        assert_eq!(log.append(&tracks(1..=1)), Ok(()));
        log.tamper(statement);
        (log.head(), log.since(0, 10))
    }

    /// The log's standing as a row: the bytes of the generation, the head
    /// and a horizon of 0.
    fn standing(generation: &[u8], head: Value) -> Row {
        Row(vec![
            Value::Blob(generation.to_vec()),
            head,
            Value::Integer(0),
        ])
    }

    /// Both answers of [`after_tampering`] when the log's standing read as
    /// `found`.
    fn unreadable(found: Vec<Row>) -> (Result<Cursor, LogError>, Result<Page, LogError>) {
        let error = LogError::Unreadable { found };
        (Err(error.clone()), Err(error))
    }

    #[test]
    fn a_cache_whose_build_record_is_not_one_generation_is_unreadable() {
        let one = || Value::Integer(1);
        // No record of the build.
        assert_eq!(
            after_tampering("DELETE FROM store_meta"),
            unreadable(vec![])
        );
        // Two records of the build.
        assert_eq!(
            after_tampering(
                "INSERT INTO store_meta (digest, generation) \
                 SELECT digest, generation FROM store_meta"
            ),
            unreadable(vec![standing(&FIRST.0, one()), standing(&FIRST.0, one())])
        );
        // A generation one byte short and one byte long.
        assert_eq!(
            after_tampering("UPDATE store_meta SET generation = zeroblob(15)"),
            unreadable(vec![standing(&[0; 15], one())])
        );
        assert_eq!(
            after_tampering("UPDATE store_meta SET generation = zeroblob(17)"),
            unreadable(vec![standing(&[0; 17], one())])
        );
    }

    #[test]
    fn a_log_whose_highest_number_is_not_a_place_is_unreadable() {
        assert_eq!(
            after_tampering("UPDATE sqlite_sequence SET seq = 'many' WHERE name = 'changelog'"),
            unreadable(vec![standing(&FIRST.0, text("many"))])
        );
        assert_eq!(
            after_tampering("UPDATE sqlite_sequence SET seq = -4 WHERE name = 'changelog'"),
            unreadable(vec![standing(&FIRST.0, Value::Integer(-4))])
        );
    }

    #[test]
    fn a_cache_built_without_the_log_fails_every_call_with_sqlites_error() {
        let data = Data::new();
        let log = data.open(&[ITEMS], FIRST);
        // SQLITE_ERROR: there is no such table.
        let missing = DbError::Sqlite { code: 1 };
        assert_eq!(
            log.append(&[upsert(track(1))]),
            Err(StoreError::Db(missing.clone()))
        );
        assert_eq!(log.compact(1), Err(StoreError::Db(missing.clone())));
        assert_eq!(log.head(), Err(LogError::Db(missing.clone())));
        assert_eq!(log.since(0, 10), Err(LogError::Db(missing)));
    }
}
