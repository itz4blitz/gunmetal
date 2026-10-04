//! The cache handle: open it, write through the one writer, read through
//! the reader pool.

use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;

use gunmetal_core::schema::{ClassError, SchemaError, SchemaPart};
use gunmetal_fs::dataroot::{DataRoot, DataRootError};
use gunmetal_fs::sqlite::DbError;

use crate::readers::{self, ReadJob, Reader};
use crate::reply::{self, Reply};
use crate::schema;
use crate::writer::{self, Queued, Transaction, WriteJob};

/// The random identity of one build of the cache. Change-log cursors carry
/// it, so a device holding a cursor from an older build is told to take a
/// fresh snapshot rather than a delta that no longer means anything.
///
/// The caller draws it from the server's CSPRNG; the store only keeps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Generation(pub [u8; 16]);

/// What [`Store::open`] found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Build {
    /// There was no cache, so it was built.
    Fresh,
    /// The cache was built from the same parts, so it was kept, with its
    /// generation.
    Reused,
    /// The cache was built from different parts, or could not be read, so
    /// it was discarded and built again.
    Rebuilt,
}

/// An open cache and how it was opened.
#[derive(Debug)]
pub struct Opened {
    /// The cache.
    pub store: Store,
    /// Whether it was built, kept or rebuilt.
    pub build: Build,
    /// Its generation: the one offered to [`Store::open`] when it was built
    /// or rebuilt, and the one it already had when it was kept.
    pub generation: Generation,
}

/// Why the store refused or failed a call.
#[derive(Debug, Clone, PartialEq)]
pub enum StoreError {
    /// The schema parts were refused.
    Schema(SchemaError),
    /// The parts do not classify exactly the columns they create
    /// (SEC-TM-050).
    Classes(ClassError),
    /// SQLite failed or refused a statement.
    Db(DbError),
    /// The cache file could not be discarded.
    DataRoot(DataRootError),
    /// A row of SQLite's own catalogue did not have the expected shape.
    Catalogue,
    /// The thread that would have done the work has stopped, because work
    /// given to it panicked.
    Closed,
}

impl From<DbError> for StoreError {
    fn from(error: DbError) -> Self {
        Self::Db(error)
    }
}

/// Joins threads when dropped, after the sender declared before it has
/// gone, so every call they accepted finishes first.
#[derive(Debug)]
struct Threads(Vec<JoinHandle<()>>);

impl Drop for Threads {
    fn drop(&mut self) {
        for thread in self.0.drain(..) {
            // A thread that panicked has nothing left to finish.
            let _ = thread.join();
        }
    }
}

/// The rebuildable cache, `cache/library.db`.
///
/// One thread owns the only read-write connection and runs each write as
/// one transaction, in the order the writes were made, so writers never
/// meet a "database is locked" error (ADM-080). A small pool of threads own
/// read-only connections and run reads concurrently with the writer, each
/// on a consistent snapshot. Every connection comes from the one opener in
/// `gunmetal-fs`, with `secure_delete` on (SEC-PRV-050), and runs SQL only
/// as a static `Query` (SEC-API-066, SEC-TM-039, SEC-HIS-038).
///
/// Dropping the store waits for every call it accepted to finish. The
/// readers close first and the writer last, so the writer's close
/// checkpoints the write-ahead log into the file and removes it, leaving
/// one self-contained file.
#[derive(Debug)]
pub struct Store {
    depth: Arc<AtomicUsize>,
    // Fields drop in this order: each sender closes its queue before its
    // threads are joined, and the readers go before the writer.
    reads: Sender<ReadJob>,
    _readers: Threads,
    writes: Sender<WriteJob>,
    _writer: Threads,
}

impl Store {
    /// Opens the cache beneath `root`, built from `parts`.
    ///
    /// A cache built from the same parts is kept. Otherwise, or when the
    /// file cannot be read, it is discarded and built again from the parts
    /// in name order, in one transaction, with `generation`. Before that
    /// transaction commits, the columns SQLite reports must be exactly the
    /// columns the parts classify.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Schema`] when the parts are refused (a part
    /// may not be called `store`, which is the store's own),
    /// [`StoreError::Classes`] when they do not classify exactly the columns
    /// they create, and [`StoreError::Db`] or [`StoreError::DataRoot`] when
    /// a part's SQL fails or the file cannot be opened or discarded.
    pub fn open(
        root: &DataRoot,
        parts: &[SchemaPart],
        generation: Generation,
    ) -> Result<Opened, StoreError> {
        schema::register(parts)
            .and_then(|schema| schema::settle(root, &schema, generation))
            .and_then(|(db, build, generation)| {
                readers::connect(root).map(|pool| (db, pool, build, generation))
            })
            .map(|(db, pool, build, generation)| {
                let depth = Arc::new(AtomicUsize::new(0));
                let (writes, writing) = writer::spawn(db);
                let (reads, reading) = readers::spawn(pool);
                Opened {
                    store: Self {
                        depth,
                        reads,
                        _readers: Threads(reading),
                        writes,
                        _writer: Threads(vec![writing]),
                    },
                    build,
                    generation,
                }
            })
    }

    /// Runs `work` on the writer's connection inside one transaction, after
    /// every write made before it. The transaction commits when `work`
    /// returns `Ok` and rolls back when it returns an error or the commit
    /// fails.
    ///
    /// The write is queued now, not when the reply is first polled, and a
    /// dropped reply does not cancel it. A panic in `work` stops the writer:
    /// this write and every later one resolve to [`StoreError::Closed`].
    pub fn write<R: Send + 'static>(
        &self,
        work: impl FnOnce(&Transaction<'_>) -> Result<R, StoreError> + Send + 'static,
    ) -> Reply<R> {
        let (completer, reply) = reply::channel();
        let job = WriteJob {
            queued: Queued::new(&self.depth),
            run: Box::new(move |db| completer.send(writer::transact(db, work))),
        };
        // A writer that has stopped hands the job back, and dropping it
        // answers the reply with `Closed` and leaves the queue.
        let _ = self.writes.send(job);
        reply
    }

    /// Runs `work` on one of the pooled read-only connections, inside one
    /// read transaction: every query it makes sees the same snapshot, which
    /// holds every write committed before its first query and nothing of a
    /// write still in progress or committed later.
    ///
    /// The read is queued now, not when the reply is first polled. A panic
    /// in `work` stops that reader and resolves this read to
    /// [`StoreError::Closed`]; the rest of the pool goes on.
    pub fn read<R: Send + 'static>(
        &self,
        work: impl FnOnce(&Reader<'_>) -> R + Send + 'static,
    ) -> Reply<R> {
        let (completer, reply) = reply::channel();
        let job: ReadJob = Box::new(move |db| completer.send(readers::snapshot(db, work)));
        let _ = self.reads.send(job);
        reply
    }

    /// How many writes are waiting for the writer, not counting the one it
    /// is running, for Admin > Diagnostics (ADM-080).
    #[must_use]
    pub fn write_queue_depth(&self) -> usize {
        writer::depth(&self.depth)
    }
}
