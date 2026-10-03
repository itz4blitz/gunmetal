//! The single writer and the reader pool: transactions, serialised writes,
//! snapshot reads, the write-queue depth and the pragmas of every pooled
//! connection. Real SQLite files in a temporary data root.

#[allow(dead_code, reason = "each test binary uses part of the helpers")]
mod support;

use std::sync::mpsc;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

use gunmetal_fs::sqlite::{DbError, Query, Row, Value};
use gunmetal_store::readers::READERS;
use gunmetal_store::store::StoreError;
use support::{
    BOUND, COUNT_TRACKS, Data, FIRST, INSERT_TRACK, TITLES, TRACKS, add, ints, named, text, titles,
    wait,
};

const SECURITY_PRAGMAS: Query = Query::new(
    "SELECT * FROM pragma_secure_delete(), pragma_foreign_keys(), pragma_trusted_schema()",
);

/// Verifies: SEC-PRV-050
#[test]
fn every_pooled_connection_has_secure_delete_on() {
    let data = Data::new();
    let store = data.open(&[TRACKS], FIRST).store;
    assert_eq!(
        wait(store.write(|tx| Ok(tx.query(&SECURITY_PRAGMAS)?))),
        Ok(vec![ints(&[1, 1, 0])])
    );
    // Each read waits until every reader holds one, so each runs on a
    // different pooled connection.
    let all_held = Arc::new(Barrier::new(READERS));
    let replies: Vec<_> = (0..READERS)
        .map(|_| {
            let all_held = Arc::clone(&all_held);
            store.read(move |reader| {
                all_held.wait();
                reader.query(&SECURITY_PRAGMAS)
            })
        })
        .collect();
    for reply in replies {
        assert_eq!(wait(reply), Ok(Ok(vec![ints(&[1, 1, 0])])));
    }
}

#[test]
fn a_hundred_concurrent_writes_are_serialised_without_a_locked_error() {
    let data = Data::new();
    let store = data.open(&[TRACKS], FIRST).store;
    let results: Vec<_> = thread::scope(|scope| {
        let writers: Vec<_> = (0..100)
            .map(|_| scope.spawn(|| add(&store, "Intro")))
            .collect();
        writers
            .into_iter()
            .map(|writer| writer.join().expect("the writing thread"))
            .collect()
    });
    assert_eq!(results, vec![Ok(1); 100]);
    assert_eq!(
        wait(store.read(|reader| reader.query(&COUNT_TRACKS))),
        Ok(Ok(vec![ints(&[100])]))
    );
}

#[test]
fn readers_see_committed_data_and_never_a_half_written_batch() {
    let data = Data::new();
    let store = data.open(&[TRACKS], FIRST).store;
    assert_eq!(add(&store, "Intro"), Ok(1));
    let (started, has_started) = mpsc::channel();
    let (finish, may_finish) = mpsc::channel::<()>();
    let batch = store.write(move |tx| {
        tx.execute(&INSERT_TRACK.bind(text("Verse")))?;
        let _ = started.send(());
        let _ = may_finish.recv_timeout(BOUND);
        tx.execute(&INSERT_TRACK.bind(text("Chorus")))?;
        Ok(())
    });
    has_started.recv_timeout(BOUND).expect("the batch started");
    assert_eq!(titles(&store), named(&["Intro"]));
    finish.send(()).expect("the batch is waiting");
    assert_eq!(wait(batch), Ok(()));
    assert_eq!(titles(&store), named(&["Intro", "Verse", "Chorus"]));
}

#[test]
fn a_read_sees_one_snapshot_from_its_first_query_to_its_last() {
    let data = Data::new();
    let store = data.open(&[TRACKS], FIRST).store;
    assert_eq!(add(&store, "Intro"), Ok(1));
    let (commit, may_commit) = mpsc::channel::<()>();
    let write = store.write(move |tx| {
        let _ = may_commit.recv_timeout(BOUND);
        Ok(tx.execute(&INSERT_TRACK.bind(text("Verse")))?)
    });
    // The write commits between the read's two queries.
    let read = store.read(move |reader| {
        let before = reader.query(&TITLES);
        let _ = commit.send(());
        let committed = wait(write);
        (before, committed, reader.query(&TITLES))
    });
    assert_eq!(
        wait(read),
        Ok((Ok(named(&["Intro"])), Ok(1), Ok(named(&["Intro"]))))
    );
    assert_eq!(titles(&store), named(&["Intro", "Verse"]));
}

#[test]
fn a_read_that_panics_says_so_and_the_other_readers_go_on() {
    let data = Data::new();
    let store = data.open(&[TRACKS], FIRST).store;
    assert_eq!(add(&store, "Intro"), Ok(1));
    assert_eq!(
        wait(store.read(|_| -> () { panic!("a bug in a read") })),
        Err(StoreError::Closed)
    );
    // More reads than the pool has readers, all answered by those left.
    let all: Vec<_> = (0..2 * READERS).map(|_| titles(&store)).collect();
    assert_eq!(all, vec![named(&["Intro"]); 2 * READERS]);
}

#[test]
fn a_write_that_fails_is_rolled_back_and_the_next_one_succeeds() {
    let data = Data::new();
    let store = data.open(&[TRACKS], FIRST).store;
    let failing = Query::new("INSERT INTO tracks (title) VALUES (NULL)");
    assert_eq!(
        wait(store.write(move |tx| {
            tx.execute(&INSERT_TRACK.bind(text("Verse")))?;
            Ok(tx.execute(&failing)?)
        })),
        Err(StoreError::Db(DbError::Sqlite { code: 1299 }))
    );
    assert_eq!(add(&store, "Intro"), Ok(1));
    assert_eq!(titles(&store), named(&["Intro"]));
}

/// A table whose foreign key is checked only at commit.
const DEFERRED: gunmetal_core::schema::SchemaPart = gunmetal_core::schema::SchemaPart {
    name: "credits",
    sql: "CREATE TABLE credits (track INTEGER NOT NULL \
          REFERENCES tracks (id) DEFERRABLE INITIALLY DEFERRED) STRICT;",
    columns: &[gunmetal_core::schema::Column {
        table: "credits",
        name: "track",
        class: gunmetal_core::schema::DataClass::Library,
    }],
};

#[test]
fn a_write_whose_commit_fails_is_rolled_back_and_the_next_one_succeeds() {
    let data = Data::new();
    let store = data.open(&[TRACKS, DEFERRED], FIRST).store;
    let orphan = Query::new("INSERT INTO credits (track) VALUES (42)");
    assert_eq!(
        wait(store.write(move |tx| {
            tx.execute(&INSERT_TRACK.bind(text("Verse")))?;
            Ok(tx.execute(&orphan)?)
        })),
        // SQLITE_CONSTRAINT_FOREIGNKEY, reported by COMMIT.
        Err(StoreError::Db(DbError::Sqlite { code: 787 }))
    );
    assert_eq!(add(&store, "Intro"), Ok(1));
    assert_eq!(titles(&store), named(&["Intro"]));
}

#[test]
fn a_write_returns_what_its_closure_returns() {
    let data = Data::new();
    let store = data.open(&[TRACKS], FIRST).store;
    let last = Query::new("SELECT last_insert_rowid()");
    assert_eq!(
        wait(store.write(move |tx| {
            tx.execute(&INSERT_TRACK.bind(text("Intro")))?;
            tx.execute(&INSERT_TRACK.bind(text("Verse")))?;
            Ok(tx.query(&last)?)
        })),
        Ok(vec![Row(vec![Value::Integer(2)])])
    );
}

#[test]
fn a_reader_cannot_write() {
    let data = Data::new();
    let store = data.open(&[TRACKS], FIRST).store;
    assert_eq!(
        wait(store.read(|reader| reader.query(&INSERT_TRACK.bind(text("Intro"))))),
        // SQLITE_READONLY: the connection is query-only.
        Ok(Err(DbError::Sqlite { code: 8 }))
    );
    assert_eq!(titles(&store), named(&[]));
}

#[test]
fn counts_the_writes_waiting_for_the_writer() {
    let data = Data::new();
    let store = data.open(&[TRACKS], FIRST).store;
    assert_eq!(store.write_queue_depth(), 0);
    let (started, has_started) = mpsc::channel();
    let (finish, may_finish) = mpsc::channel::<()>();
    let first = store.write(move |_| {
        let _ = started.send(());
        let _ = may_finish.recv_timeout(BOUND);
        Ok(())
    });
    has_started
        .recv_timeout(BOUND)
        .expect("the first write started");
    // The running write is no longer waiting.
    assert_eq!(store.write_queue_depth(), 0);
    let queued: Vec<_> = (0..3).map(|_| store.write(|_| Ok(()))).collect();
    assert_eq!(store.write_queue_depth(), 3);
    finish.send(()).expect("the first write is waiting");
    assert_eq!(wait(first), Ok(()));
    for reply in queued {
        assert_eq!(wait(reply), Ok(()));
    }
    assert_eq!(store.write_queue_depth(), 0);
}

#[test]
fn a_write_that_panics_closes_the_writer_and_every_later_write_says_so() {
    let data = Data::new();
    let store = data.open(&[TRACKS], FIRST).store;
    assert_eq!(
        wait(store.write(|_| -> Result<(), StoreError> {
            panic!("a bug in a write");
        })),
        Err(StoreError::Closed)
    );
    assert_eq!(add(&store, "Intro"), Err(StoreError::Closed));
    assert_eq!(store.write_queue_depth(), 0);
    // Reads go on.
    assert_eq!(titles(&store), named(&[]));
}

#[test]
fn dropping_the_store_finishes_every_write_it_accepted() {
    let data = Data::new();
    let opened = data.open(&[TRACKS], FIRST);
    let slow = opened.store.write(|tx| {
        thread::sleep(Duration::from_millis(300));
        Ok(tx.execute(&INSERT_TRACK.bind(text("Intro")))?)
    });
    drop(opened.store);
    drop(slow);
    let again = data.open(&[TRACKS], FIRST);
    assert_eq!(titles(&again.store), named(&["Intro"]));
}

/// How many rows the long batch in the load test writes.
const LONG_BATCH: i64 = 5_000;

/// The stated bound on a reader's latency while a long batch commits.
const READ_LATENCY: Duration = Duration::from_secs(5);

/// How many reads the load test makes at most while waiting for the batch
/// to become visible.
const MAX_READS: usize = 1_000_000;

/// A short load test: while one long batch commits, readers keep answering
/// within [`READ_LATENCY`], and each sees either none of the batch or all
/// of it.
#[test]
fn readers_answer_quickly_while_a_long_batch_commits() {
    let data = Data::new();
    let store = data.open(&[TRACKS], FIRST).store;
    let (written, has_written) = mpsc::channel();
    let (commit, may_commit) = mpsc::channel::<()>();
    let batch = store.write(move |tx| {
        for _ in 0..LONG_BATCH {
            tx.execute(&INSERT_TRACK.bind(text("Track")))?;
        }
        let _ = written.send(());
        let _ = may_commit.recv_timeout(BOUND);
        Ok(())
    });
    has_written
        .recv_timeout(BOUND)
        .expect("the batch was written");
    let none = Ok(Ok(vec![ints(&[0])]));
    let all = Ok(Ok(vec![ints(&[LONG_BATCH])]));
    let mut latencies = Vec::new();
    let mut count = || {
        let asked = Instant::now();
        let rows = wait(store.read(|reader| reader.query(&COUNT_TRACKS)));
        latencies.push(asked.elapsed());
        rows
    };
    // Before the commit, a reader sees none of the batch.
    let mut seen = vec![count()];
    commit.send(()).expect("the batch is waiting to commit");
    for _ in 0..MAX_READS {
        let rows = count();
        let done = rows == all;
        seen.push(rows);
        if done {
            break;
        }
    }
    assert_eq!(seen.first(), Some(&none));
    assert_eq!(seen.last(), Some(&all));
    assert!(seen.iter().all(|rows| *rows == none || *rows == all));
    assert!(latencies.iter().all(|latency| *latency < READ_LATENCY));
    assert_eq!(wait(batch), Ok(()));
}
