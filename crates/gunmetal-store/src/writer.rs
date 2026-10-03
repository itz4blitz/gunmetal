//! The single writer: one thread that owns the only read-write connection
//! and runs each write as one transaction, in the order the writes arrive
//! (ADM-080).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};

use gunmetal_fs::sqlite::{Db, DbError, Query, Row};

use crate::store::StoreError;

const BEGIN: &str = "BEGIN IMMEDIATE";
const COMMIT: &str = "COMMIT";
const ROLLBACK: &str = "ROLLBACK";

/// The writer's connection, inside the transaction of one write. It runs
/// statements only as a [`Query`], whose text is static, so SQL built from
/// request text cannot reach it (SEC-API-066, SEC-TM-039, SEC-HIS-038):
///
/// ```
/// use gunmetal_fs::sqlite::{Query, Value};
/// use gunmetal_store::writer::Transaction;
///
/// const RENAME: Query = Query::new("UPDATE tracks SET title = ?1 WHERE id = ?2");
///
/// fn rename(tx: &Transaction<'_>, id: i64, title: &str) {
///     let _ = tx.execute(&RENAME.bind(Value::Text(title.to_owned())).bind(Value::Integer(id)));
/// }
/// ```
///
/// A `String` is not a `Query`, so text formatted from a request does not
/// compile.
/// Verifies: SEC-API-066, SEC-TM-039, SEC-HIS-038
///
/// ```compile_fail,E0308
/// use gunmetal_store::writer::Transaction;
///
/// fn rename(tx: &Transaction<'_>, id: i64, title: &str) {
///     let text = format!("UPDATE tracks SET title = '{title}' WHERE id = {id}");
///     let _ = tx.execute(&text);
/// }
/// ```
pub struct Transaction<'a> {
    db: &'a Db,
}

impl<'a> Transaction<'a> {
    /// Lends `db` to the store's own statements.
    pub(crate) fn new(db: &'a Db) -> Self {
        Self { db }
    }

    /// Runs `query` and returns every row.
    ///
    /// # Errors
    ///
    /// Returns a [`DbError`] when SQLite refuses or fails the statement or
    /// the parameters do not match it.
    pub fn query(&self, query: &Query) -> Result<Vec<Row>, DbError> {
        self.db.query(query)
    }

    /// Runs `query`, which must not return rows, and returns how many rows
    /// it changed.
    ///
    /// # Errors
    ///
    /// Returns a [`DbError`] when SQLite refuses or fails the statement,
    /// the parameters do not match it, or it returns rows.
    pub fn execute(&self, query: &Query) -> Result<usize, DbError> {
        self.db.execute(query)
    }

    /// The store's own statements during a build, such as a part's SQL,
    /// which is a static script of several statements.
    pub(crate) fn script(&self, script: &'static str) -> Result<(), DbError> {
        self.db.execute_batch(script)
    }
}

/// Counts one write as waiting for the writer from when it is made until
/// the writer starts it, or until it is dropped unstarted because the
/// writer has stopped.
pub(crate) struct Queued(Arc<AtomicUsize>);

impl Queued {
    pub(crate) fn new(depth: &Arc<AtomicUsize>) -> Self {
        depth.fetch_add(1, Ordering::Relaxed);
        Self(Arc::clone(depth))
    }
}

impl Drop for Queued {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

/// How many writes are waiting.
pub(crate) fn depth(depth: &AtomicUsize) -> usize {
    depth.load(Ordering::Relaxed)
}

/// One write: its place in the queue and the work, which answers its own
/// reply.
pub(crate) struct WriteJob {
    pub(crate) queued: Queued,
    pub(crate) run: Box<dyn FnOnce(&Db) + Send>,
}

/// Starts the writer thread on `db`. It runs until every sender is gone.
pub(crate) fn spawn(db: Db) -> (Sender<WriteJob>, JoinHandle<()>) {
    let (jobs, queue) = mpsc::channel::<WriteJob>();
    let thread = thread::spawn(move || {
        for WriteJob { queued, run } in queue {
            drop(queued);
            run(&db);
        }
    });
    (jobs, thread)
}

/// Runs `work` in one transaction on `db`: committed when it returns `Ok`,
/// rolled back when it returns an error or the commit fails.
pub(crate) fn transact<R>(
    db: &Db,
    work: impl FnOnce(&Transaction<'_>) -> Result<R, StoreError>,
) -> Result<R, StoreError> {
    db.execute_batch(BEGIN)
        .map_err(StoreError::from)
        .and_then(|()| work(&Transaction::new(db)))
        .and_then(|result| {
            db.execute_batch(COMMIT)
                .map(|()| result)
                .map_err(StoreError::from)
        })
        .inspect_err(|_| {
            // Rolling back fails only when no transaction is open, which
            // leaves nothing to undo.
            let _ = db.execute_batch(ROLLBACK);
        })
}
