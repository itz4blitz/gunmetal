//! Helpers shared by the store's integration tests: a data root in a
//! temporary directory, a deadline-bound way to wait for a reply, and the
//! schema parts the tests register.

use std::future::Future;
use std::io::Read;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, Thread};
use std::time::{Duration, Instant};

use gunmetal_core::schema::{Column, DataClass, SchemaPart};
use gunmetal_fs::dataroot::{DataRoot, Policy};
use gunmetal_fs::host::HostFacts;
use gunmetal_fs::path::{DataDir, DataPath};
use gunmetal_fs::sqlite::{Query, Row, Value};
use gunmetal_store::schema::LIBRARY;
use gunmetal_store::store::{Generation, Opened, Store, StoreError};
use gunmetal_testkit::tempdir::TempDir;

/// How long a test waits for anything the store does before it fails. A
/// hang is a failure, never a timeout.
pub const BOUND: Duration = Duration::from_secs(10);

/// SQLite's write-ahead log beside the cache file.
pub const LIBRARY_WAL: DataPath = DataPath::constant(DataDir::Cache, "library.db-wal");

/// The generation a test's first build gets.
pub const FIRST: Generation = Generation([1; 16]);

/// The generation a test's later opens offer.
pub const SECOND: Generation = Generation([2; 16]);

/// One table of tracks, every column classified.
pub const TRACKS: SchemaPart = SchemaPart {
    name: "tracks",
    sql: "CREATE TABLE tracks (id INTEGER PRIMARY KEY, title TEXT NOT NULL) STRICT;",
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
    ],
};

pub const INSERT_TRACK: Query = Query::new("INSERT INTO tracks (title) VALUES (?1)");
pub const TITLES: Query = Query::new("SELECT title FROM tracks ORDER BY id");
pub const COUNT_TRACKS: Query = Query::new("SELECT count(*) FROM tracks");

/// A temporary data directory and its open root.
pub struct Data {
    pub root: DataRoot,
    // Dropped last, after the root and any store.
    _dir: TempDir,
}

impl Data {
    pub fn new() -> Self {
        let dir = TempDir::new("store").expect("a temporary directory");
        let host = HostFacts::probe(dir.path()).expect("probe the host");
        let root = DataRoot::open(dir.path(), &host, Policy::DEFAULT)
            .expect("the data root opens")
            .root;
        Self { root, _dir: dir }
    }

    pub fn open(&self, parts: &[SchemaPart], generation: Generation) -> Opened {
        Store::open(&self.root, parts, generation).expect("the store opens")
    }

    /// The bytes of the file at `path`.
    pub fn bytes(&self, path: &DataPath) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.root
            .open_read(path)
            .expect("the file exists")
            .read_to_end(&mut bytes)
            .expect("the file reads");
        bytes
    }

    /// The cache file's bytes.
    pub fn cache_bytes(&self) -> Vec<u8> {
        self.bytes(LIBRARY.path())
    }
}

/// Wakes the thread that waits for a reply.
struct Unpark(Thread);

impl Wake for Unpark {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

/// Drives `future` on this thread until it is ready, failing the test once
/// [`BOUND`] has passed.
pub fn wait<F: Future>(future: F) -> F::Output {
    let deadline = Instant::now() + BOUND;
    let waker = Waker::from(Arc::new(Unpark(thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
        let left = deadline.saturating_duration_since(Instant::now());
        assert!(!left.is_zero(), "the store did not answer in time");
        thread::park_timeout(left);
    }
}

pub fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}

/// Inserts one track with `title`.
pub fn add(store: &Store, title: &'static str) -> Result<usize, StoreError> {
    wait(store.write(move |tx| Ok(tx.execute(&INSERT_TRACK.bind(text(title)))?)))
}

/// Every title, in insertion order, as a reader sees them.
pub fn titles(store: &Store) -> Vec<Row> {
    wait(store.read(|reader| reader.query(&TITLES)))
        .expect("the reader answers")
        .expect("the query runs")
}

/// The rows [`titles`] returns for `titles`.
pub fn named(titles: &[&str]) -> Vec<Row> {
    titles.iter().map(|title| Row(vec![text(title)])).collect()
}

pub fn ints(values: &[i64]) -> Row {
    Row(values.iter().copied().map(Value::Integer).collect())
}
