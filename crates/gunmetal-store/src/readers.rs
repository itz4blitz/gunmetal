//! The reader pool: a few threads, each owning one read-only connection,
//! taking reads from one shared queue. In WAL mode each read sees the
//! writes committed before it started and nothing of a write in progress,
//! and never waits for the writer.

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use gunmetal_fs::dataroot::DataRoot;
use gunmetal_fs::sqlite::{Db, DbError, Pragmas, Query, Row, Synchronous, open_db};

use crate::reply::lock;
use crate::schema::LIBRARY;
use crate::store::StoreError;

/// How many read-only connections the pool holds.
pub const READERS: usize = 4;

/// How long a reader waits for a lock, which in WAL mode only a checkpoint
/// or recovery holds, before failing.
const BUSY_TIMEOUT_MS: u16 = 5_000;

/// The pragmas of every pooled connection: query-only, on top of the
/// opener's own (`secure_delete` among them).
const READER: Pragmas = Pragmas::new(Synchronous::Normal)
    .busy_timeout(BUSY_TIMEOUT_MS)
    .query_only();

/// One read, which answers its own reply.
pub(crate) type ReadJob = Box<dyn FnOnce(&Db) + Send>;

/// A pooled read-only connection, lent to one read. It runs statements
/// only as a [`Query`], whose text is static, so SQL built from request
/// text cannot reach it (SEC-API-066, SEC-TM-039, SEC-HIS-038):
///
/// ```
/// use gunmetal_fs::sqlite::{Query, Value};
/// use gunmetal_store::readers::Reader;
///
/// const BY_TITLE: Query = Query::new("SELECT id FROM tracks WHERE title = ?1");
///
/// fn find(reader: &Reader<'_>, title: &str) {
///     let _ = reader.query(&BY_TITLE.bind(Value::Text(title.to_owned())));
/// }
/// ```
///
/// A `String` is not a `Query`, so text formatted from a request does not
/// compile.
/// Verifies: SEC-API-066, SEC-TM-039, SEC-HIS-038
///
/// ```compile_fail,E0308
/// use gunmetal_store::readers::Reader;
///
/// fn find(reader: &Reader<'_>, title: &str) {
///     let text = format!("SELECT id FROM tracks WHERE title = '{title}'");
///     let _ = reader.query(&text);
/// }
/// ```
pub struct Reader<'a> {
    db: &'a Db,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(db: &'a Db) -> Self {
        Self { db }
    }

    /// Runs `query` and returns every row.
    ///
    /// # Errors
    ///
    /// Returns a [`DbError`] when SQLite refuses or fails the statement,
    /// the parameters do not match it, or it would write.
    pub fn query(&self, query: &Query) -> Result<Vec<Row>, DbError> {
        self.db.query(query)
    }
}

/// Starts a read transaction, which takes its snapshot at its first query.
const BEGIN: &str = "BEGIN DEFERRED";

/// Ends a read transaction; it changed nothing, so there is nothing to keep.
const END: &str = "ROLLBACK";

/// Runs `work` on `db` inside one read transaction, so every query it makes
/// sees the same snapshot: the writes committed before its first query and
/// none after.
pub(crate) fn snapshot<R>(db: &Db, work: impl FnOnce(&Reader<'_>) -> R) -> Result<R, StoreError> {
    db.execute_batch(BEGIN)
        .map(|()| work(&Reader::new(db)))
        .and_then(|result| db.execute_batch(END).map(|()| result))
        .map_err(StoreError::from)
}

/// Opens the pool's connections.
pub(crate) fn connect(root: &DataRoot) -> Result<Vec<Db>, StoreError> {
    (0..READERS)
        .map(|_| open_db(root, &LIBRARY, READER).map_err(StoreError::from))
        .collect()
}

/// Starts one reader thread per connection, all taking from one queue.
/// Each runs until every sender is gone.
pub(crate) fn spawn(pool: Vec<Db>) -> (Sender<ReadJob>, Vec<JoinHandle<()>>) {
    let (jobs, queue) = mpsc::channel::<ReadJob>();
    let queue = Arc::new(Mutex::new(queue));
    let threads = pool
        .into_iter()
        .map(|db| {
            let queue = Arc::clone(&queue);
            thread::spawn(move || {
                while let Ok(job) = next(&queue) {
                    job(&db);
                }
            })
        })
        .collect();
    (jobs, threads)
}

/// Takes the next read, holding the queue's lock only while waiting for it.
fn next(queue: &Mutex<Receiver<ReadJob>>) -> Result<ReadJob, mpsc::RecvError> {
    lock(queue).recv()
}
