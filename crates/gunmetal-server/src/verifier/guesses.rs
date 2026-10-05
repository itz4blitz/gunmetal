//! The guess log: the wrong guesses the delay schedule counts, kept in the
//! identity store so that a restart forgives none of them.
//!
//! A row counts one source's wrong guesses at one target of one pathway
//! since its last right one, and when the latest was made
//! ([`Failures`]). Only the pathways whose secrets can be guessed have rows
//! (SEC-API-056).
//!
//! The rows are state kept for callers who have not signed in, so the log
//! holds a fixed number of them and drops the oldest to make room
//! (SEC-NET-051). Dropping a row forgives its source early; a source that
//! could fill the log with others' rows could as well guess from them.
//!
//! The table lets nothing in that could not be read back: its types are
//! strict, a count is at least one and a time is one a timestamp can hold.
//! So a row cannot hold a source off for good, and reading one that still
//! cannot be read is a fault, which the verifier answers with a refusal.
//!
//! The log is read through the verifier's pre-authentication handle and
//! written through the identity store's one writer. The verifier registers
//! [`GUESS_DELAYS`] with the store when it is opened.

use gunmetal_core::client_context::ClientContext;
use gunmetal_core::schema::{Column, DataClass, SchemaPart};
use gunmetal_core::time::Timestamp;
use gunmetal_durable::identity::store::IdentityStore;
use gunmetal_fs::sqlite::{Query, Row, Value};

use crate::limiter::delay::{Failures, source_label};
use crate::verifier::pathway::{Fault, Pathway, Target};
use crate::verifier::preauth::PreAuth;

/// The guess log's table. Its rows say who has been guessing at what, so
/// every column is identity data.
pub const GUESS_DELAYS: SchemaPart = SchemaPart {
    name: "verifier.guess_delays",
    sql: "CREATE TABLE guess_delays (\
              pathway TEXT NOT NULL, \
              target BLOB NOT NULL, \
              source TEXT NOT NULL, \
              failures INTEGER NOT NULL CHECK (failures >= 1), \
              failed_at INTEGER NOT NULL \
                  CHECK (failed_at >= -62167219200000 AND failed_at <= 253402300799999), \
              PRIMARY KEY (pathway, target, source)) STRICT;",
    columns: &[
        Column {
            table: "guess_delays",
            name: "pathway",
            class: DataClass::Identity,
        },
        Column {
            table: "guess_delays",
            name: "target",
            class: DataClass::Identity,
        },
        Column {
            table: "guess_delays",
            name: "source",
            class: DataClass::Identity,
        },
        Column {
            table: "guess_delays",
            name: "failures",
            class: DataClass::Identity,
        },
        Column {
            table: "guess_delays",
            name: "failed_at",
            class: DataClass::Identity,
        },
    ],
};

/// Reads one row's count and time.
const READ: Query = Query::new(
    "SELECT failures, failed_at FROM guess_delays \
     WHERE pathway = ?1 AND target = ?2 AND source = ?3",
);

/// Counts one more wrong guess, made at the time given.
const RECORD: Query = Query::new(
    "INSERT INTO guess_delays (pathway, target, source, failures, failed_at) \
     VALUES (?1, ?2, ?3, 1, ?4) \
     ON CONFLICT (pathway, target, source) \
     DO UPDATE SET failures = failures + 1, failed_at = excluded.failed_at",
);

/// Drops every row past the newest ones the log has room for.
const PRUNE: Query = Query::new(
    "DELETE FROM guess_delays WHERE rowid IN (\
         SELECT rowid FROM guess_delays \
         ORDER BY failed_at DESC, rowid DESC LIMIT -1 OFFSET ?1)",
);

/// Moves a row's time.
const REBASE: Query = Query::new(
    "UPDATE guess_delays SET failed_at = ?4 \
     WHERE pathway = ?1 AND target = ?2 AND source = ?3",
);

/// Forgets a row.
const CLEAR: Query =
    Query::new("DELETE FROM guess_delays WHERE pathway = ?1 AND target = ?2 AND source = ?3");

/// Whose wrong guesses a row counts: one source's, at one target of one
/// pathway.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuessKey {
    pathway: Pathway,
    target: Option<Target>,
    source: String,
}

impl GuessKey {
    /// The key for guesses from `source` on `pathway`, aimed at `target`
    /// when the pathway has more than one thing to guess.
    #[must_use]
    pub fn new(pathway: Pathway, target: Option<Target>, source: &ClientContext) -> Self {
        Self {
            pathway,
            target,
            source: source_label(source),
        }
    }

    /// Binds the key to the first three parameters of `query`.
    fn bind(&self, query: Query) -> Query {
        let target = self
            .target
            .map_or_else(Vec::new, |target| target.bytes().to_vec());
        query
            .bind(Value::Text(self.pathway.name().to_owned()))
            .bind(Value::Blob(target))
            .bind(Value::Text(self.source.clone()))
    }
}

/// What reading a key returned, as the wrong guesses on record: no row, or
/// one row of a count and a time. A count too large for the schedule's own
/// type is read as the largest it can hold.
///
/// # Errors
///
/// [`Fault::Missing`] when the rows are anything else.
pub fn decode(rows: &[Row]) -> Result<Option<Failures>, Fault> {
    let [Row(values)] = rows else {
        return if rows.is_empty() {
            Ok(None)
        } else {
            Err(Fault::Missing)
        };
    };
    let [Value::Integer(count), Value::Integer(at)] = values.as_slice() else {
        return Err(Fault::Missing);
    };
    Timestamp::from_millis(*at)
        .map(|last_at| {
            Some(Failures {
                count: u32::try_from(*count).unwrap_or(u32::MAX),
                last_at,
            })
        })
        .map_err(|_| Fault::Missing)
}

/// The guess log.
#[derive(Debug, Clone, Copy)]
pub struct GuessLog {
    capacity: u32,
}

impl GuessLog {
    /// A log that keeps the `capacity` rows written last. It always keeps
    /// one, so no setting switches the delay off.
    #[must_use]
    pub const fn new(capacity: u32) -> Self {
        Self { capacity }
    }

    /// The wrong guesses on record for `key`.
    ///
    /// # Errors
    ///
    /// A [`Fault`] when the store cannot be read, or holds a row that
    /// cannot be.
    pub fn read(self, lookup: &PreAuth<'_>, key: &GuessKey) -> Result<Option<Failures>, Fault> {
        lookup
            .lookup(&key.bind(READ))
            .and_then(|rows| decode(&rows))
    }

    /// Counts one more wrong guess for `key`, made at `now`, and drops the
    /// rows the log has no room for.
    ///
    /// # Errors
    ///
    /// [`Fault::Storage`] when the store cannot be written.
    pub fn record(
        self,
        store: &IdentityStore,
        key: &GuessKey,
        now: Timestamp,
    ) -> Result<(), Fault> {
        let record = key.bind(RECORD).bind(Value::Integer(now.millis()));
        let prune = PRUNE.bind(Value::Integer(i64::from(self.capacity.max(1))));
        store.write(&[record, prune]).map(drop).map_err(Fault::from)
    }

    /// Makes the wrong guesses on record for `key` count from `now`.
    ///
    /// # Errors
    ///
    /// [`Fault::Storage`] when the store cannot be written.
    pub fn rebase(
        self,
        store: &IdentityStore,
        key: &GuessKey,
        now: Timestamp,
    ) -> Result<(), Fault> {
        let rebase = key.bind(REBASE).bind(Value::Integer(now.millis()));
        store.write(&[rebase]).map(drop).map_err(Fault::from)
    }

    /// Forgets the wrong guesses on record for `key`.
    ///
    /// # Errors
    ///
    /// [`Fault::Storage`] when the store cannot be written.
    pub fn clear(self, store: &IdentityStore, key: &GuessKey) -> Result<(), Fault> {
        store
            .write(&[key.bind(CLEAR)])
            .map(drop)
            .map_err(Fault::from)
    }
}

#[cfg(test)]
mod tests {
    use gunmetal_core::time::Timestamp;
    use gunmetal_durable::identity::error::{IdentityError, Step};
    use gunmetal_durable::identity::pre_principal::PrePrincipal;
    use gunmetal_durable::identity::store::IDENTITY;
    use gunmetal_fs::sqlite::{DbError, Pragmas, Query, Row, Synchronous, Value, open_db};

    use super::*;
    use crate::limiter::delay::Failures;
    use crate::limiter::testing::source;
    use crate::testing::NOON;
    use crate::verifier::pathway::{Fault, Pathway, Target};
    use crate::verifier::preauth::PreAuth;
    use crate::verifier::testing::{STAND_IN, data, open, text};

    /// Every row of the log as it is stored, oldest guess first.
    const ROWS: Query = Query::new(
        "SELECT pathway, target, source, failures, failed_at FROM guess_delays \
         ORDER BY failed_at, source",
    );

    fn at(ms: i64) -> Timestamp {
        Timestamp::from_millis(ms).expect("in range")
    }

    fn counted(count: u32, ms: i64) -> Failures {
        Failures {
            count,
            last_at: at(ms),
        }
    }

    /// The key for guesses at the claim code from `addr`.
    fn claim_from(addr: &str) -> GuessKey {
        GuessKey::new(Pathway::ClaimCode, None, &source(addr))
    }

    /// A row as the store holds it: the pathway's name, the target's bytes
    /// or none, the source's narrowest key, the count and the time.
    fn stored(pathway: &str, target: &[u8], source: &str, count: i64, ms: i64) -> Row {
        Row(vec![
            text(pathway),
            Value::Blob(target.to_vec()),
            text(source),
            Value::Integer(count),
            Value::Integer(ms),
        ])
    }

    /// The stored row of a claim-code guesser at `addr`.
    fn claim_row(addr: &str, count: i64, ms: i64) -> Row {
        stored("claim_code", &[], addr, count, ms)
    }

    /// What a write refused for want of the write lock looks like.
    fn busy() -> Result<(), Fault> {
        Err(Fault::Storage(Box::new(IdentityError::Db {
            step: Step::Write,
            // SQLITE_BUSY
            error: DbError::Sqlite { code: 5 },
        })))
    }

    #[test]
    fn counts_each_wrong_guess_until_it_is_told_to_forget_them() {
        let data = data("guesses");
        let store = open(&data.root, &[GUESS_DELAYS]);
        let lookup = PreAuth::new(&store);
        let log = GuessLog::new(8);
        let key = claim_from("192.168.1.66");
        assert_eq!(log.read(&lookup, &key), Ok(None));
        assert_eq!(log.record(&store, &key, at(NOON)), Ok(()));
        assert_eq!(log.read(&lookup, &key), Ok(Some(counted(1, NOON))));
        assert_eq!(log.record(&store, &key, at(NOON + 30_000)), Ok(()));
        assert_eq!(log.record(&store, &key, at(NOON + 90_000)), Ok(()));
        assert_eq!(log.read(&lookup, &key), Ok(Some(counted(3, NOON + 90_000))));
        // Moving the time keeps the count.
        assert_eq!(log.rebase(&store, &key, at(NOON - 5)), Ok(()));
        assert_eq!(log.read(&lookup, &key), Ok(Some(counted(3, NOON - 5))));
        assert_eq!(log.clear(&store, &key), Ok(()));
        assert_eq!(log.read(&lookup, &key), Ok(None));
        // Forgetting, or moving, what is not there is not a fault.
        assert_eq!(log.clear(&store, &key), Ok(()));
        assert_eq!(log.rebase(&store, &key, at(NOON)), Ok(()));
        assert_eq!(log.read(&lookup, &key), Ok(None));
    }

    #[test]
    fn keeps_each_pathway_target_and_source_apart() {
        let data = data("guesses-keys");
        let store = open(&data.root, &[GUESS_DELAYS]);
        let lookup = PreAuth::new(&store);
        let log = GuessLog::new(8);
        let approver = Target::from_bytes([0xab; 16]);
        let lan = source("192.168.1.66");
        let v6 = source("2001:db8:0:1234:aaaa:bbbb:cccc:dddd");
        let keys = [
            GuessKey::new(Pathway::ClaimCode, None, &lan),
            GuessKey::new(Pathway::PairingCode, None, &lan),
            GuessKey::new(Pathway::PairingCode, Some(approver), &lan),
            GuessKey::new(Pathway::PairingCode, Some(approver), &v6),
        ];
        // The first key guesses wrong once, the second twice, and so on.
        for (index, key) in keys.iter().enumerate() {
            for guess in 0..=index {
                let offset = i64::try_from(index * 10 + guess).expect("small");
                assert_eq!(log.record(&store, key, at(NOON + offset)), Ok(()));
            }
        }
        let read: Vec<_> = keys.iter().map(|key| log.read(&lookup, key)).collect();
        assert_eq!(
            read,
            [
                Ok(Some(counted(1, NOON))),
                Ok(Some(counted(2, NOON + 11))),
                Ok(Some(counted(3, NOON + 22))),
                Ok(Some(counted(4, NOON + 33))),
            ]
        );
        let home = "192.168.1.66";
        let network = "2001:db8:0:1234::/64";
        assert_eq!(
            lookup.lookup(&ROWS),
            Ok(vec![
                stored("claim_code", &[], home, 1, NOON),
                stored("pairing_code", &[], home, 2, NOON + 11),
                stored("pairing_code", &[0xab; 16], home, 3, NOON + 22),
                stored("pairing_code", &[0xab; 16], network, 4, NOON + 33),
            ])
        );
        // Forgetting one leaves the others.
        assert_eq!(log.clear(&store, &keys[2]), Ok(()));
        let read: Vec<_> = keys.iter().map(|key| log.read(&lookup, key)).collect();
        assert_eq!(
            read,
            [
                Ok(Some(counted(1, NOON))),
                Ok(Some(counted(2, NOON + 11))),
                Ok(None),
                Ok(Some(counted(4, NOON + 33))),
            ]
        );
    }

    #[test]
    fn a_restart_keeps_what_was_counted() {
        let data = data("guesses-restart");
        let key = claim_from("192.168.1.66");
        let log = GuessLog::new(8);
        let store = open(&data.root, &[GUESS_DELAYS]);
        assert_eq!(log.record(&store, &key, at(NOON)), Ok(()));
        assert_eq!(log.record(&store, &key, at(NOON + 30_000)), Ok(()));
        drop(store);
        let reopened = open(&data.root, &[GUESS_DELAYS]);
        assert_eq!(
            log.read(&PreAuth::new(&reopened), &key),
            Ok(Some(counted(2, NOON + 30_000)))
        );
    }

    #[test]
    fn keeps_only_the_newest_rows_it_has_room_for() {
        let data = data("guesses-room");
        let store = open(&data.root, &[GUESS_DELAYS]);
        let lookup = PreAuth::new(&store);
        let log = GuessLog::new(2);
        let [first, second, third, fourth] =
            ["192.0.2.1", "192.0.2.2", "192.0.2.3", "192.0.2.4"].map(claim_from);
        assert_eq!(log.record(&store, &first, at(NOON)), Ok(()));
        assert_eq!(log.record(&store, &second, at(NOON + 1)), Ok(()));
        assert_eq!(log.record(&store, &third, at(NOON + 2)), Ok(()));
        assert_eq!(
            lookup.lookup(&ROWS),
            Ok(vec![
                claim_row("192.0.2.2", 1, NOON + 1),
                claim_row("192.0.2.3", 1, NOON + 2),
            ])
        );
        // A source that guesses again is the newest again, and keeps its
        // count; the one it overtook is the next to go.
        assert_eq!(log.record(&store, &second, at(NOON + 3)), Ok(()));
        assert_eq!(log.record(&store, &fourth, at(NOON + 4)), Ok(()));
        assert_eq!(
            lookup.lookup(&ROWS),
            Ok(vec![
                claim_row("192.0.2.2", 2, NOON + 3),
                claim_row("192.0.2.4", 1, NOON + 4),
            ])
        );
        // A log told to keep nothing still keeps the newest row.
        let none = GuessLog::new(0);
        assert_eq!(none.record(&store, &first, at(NOON + 5)), Ok(()));
        assert_eq!(
            lookup.lookup(&ROWS),
            Ok(vec![claim_row("192.0.2.1", 1, NOON + 5)])
        );
        assert_eq!(none.read(&lookup, &first), Ok(Some(counted(1, NOON + 5))));
    }

    #[test]
    fn reads_only_one_row_of_a_count_and_a_time() {
        let row = |count: i64, ms: i64| Row(vec![Value::Integer(count), Value::Integer(ms)]);
        assert_eq!(decode(&[]), Ok(None));
        assert_eq!(decode(&[row(3, NOON)]), Ok(Some(counted(3, NOON))));
        // A count past what the schedule's type holds is its largest.
        assert_eq!(
            decode(&[row(i64::MAX, NOON)]),
            Ok(Some(counted(u32::MAX, NOON)))
        );
        assert_eq!(
            decode(&[row(4_294_967_296, NOON)]),
            Ok(Some(counted(u32::MAX, NOON)))
        );
        assert_eq!(
            decode(&[row(4_294_967_295, NOON)]),
            Ok(Some(counted(u32::MAX, NOON)))
        );
        assert_eq!(decode(&[row(-1, NOON)]), Ok(Some(counted(u32::MAX, NOON))));
        for unreadable in [
            vec![row(1, NOON), row(1, NOON)],
            vec![Row(vec![Value::Integer(1)])],
            vec![Row(vec![text("1"), Value::Integer(NOON)])],
            vec![Row(vec![Value::Integer(1), Value::Null])],
            vec![Row(vec![
                Value::Integer(1),
                Value::Integer(NOON),
                Value::Integer(0),
            ])],
            // A time no timestamp holds.
            vec![row(1, i64::MAX)],
        ] {
            assert_eq!(decode(&unreadable), Err(Fault::Missing), "{unreadable:?}");
        }
    }

    #[test]
    fn the_table_refuses_a_row_that_could_not_be_read_back() {
        let data = data("guesses-strict");
        let store = open(&data.root, &[GUESS_DELAYS]);
        let insert = |failures: Value, failed_at: Value| {
            Query::new(
                "INSERT INTO guess_delays (pathway, target, source, failures, failed_at) \
                 VALUES ('claim_code', x'', '192.0.2.1', ?1, ?2)",
            )
            .bind(failures)
            .bind(failed_at)
        };
        let refused = |code: i32| {
            Err(IdentityError::Db {
                step: Step::Write,
                error: DbError::Sqlite { code },
            })
        };
        // SQLITE_CONSTRAINT_DATATYPE (3091) for a value that is not a whole
        // number, and SQLITE_CONSTRAINT_CHECK (275) for one out of range.
        let cases = [
            (text("many"), Value::Integer(NOON), 3091),
            (Value::Real(1.5), Value::Integer(NOON), 3091),
            (Value::Integer(1), Value::Real(1.5), 3091),
            (Value::Integer(0), Value::Integer(NOON), 275),
            (Value::Integer(1), Value::Integer(i64::MAX), 275),
            (Value::Integer(1), Value::Integer(i64::MIN), 275),
        ];
        for (failures, failed_at, code) in cases {
            assert_eq!(store.write(&[insert(failures, failed_at)]), refused(code));
        }
        assert_eq!(PreAuth::new(&store).lookup(&ROWS), Ok(vec![]));
    }

    #[test]
    fn a_store_that_cannot_be_read_or_written_is_a_fault() {
        // A store opened without the log's table: SQLite knows no such table.
        let bare = data("guesses-bare");
        let store = open(&bare.root, &[STAND_IN]);
        let log = GuessLog::new(8);
        let key = claim_from("192.168.1.66");
        assert_eq!(
            log.read(&PreAuth::new(&store), &key),
            Err(Fault::Storage(Box::new(IdentityError::Db {
                step: Step::PrePrincipal(PrePrincipal::Credential),
                // SQLITE_ERROR
                error: DbError::Sqlite { code: 1 },
            })))
        );
        // A store whose write lock another connection holds.
        let data = data("guesses-busy");
        let store = open(&data.root, &[GUESS_DELAYS]);
        assert_eq!(log.record(&store, &key, at(NOON)), Ok(()));
        let other = open_db(&data.root, &IDENTITY, Pragmas::new(Synchronous::Full))
            .expect("a second connection");
        other
            .execute(&Query::new("BEGIN IMMEDIATE"))
            .expect("the write lock");
        assert_eq!(log.record(&store, &key, at(NOON + 1)), busy());
        assert_eq!(log.rebase(&store, &key, at(NOON + 1)), busy());
        assert_eq!(log.clear(&store, &key), busy());
        other.execute(&Query::new("ROLLBACK")).expect("released");
        // None of the refused writes took.
        assert_eq!(
            log.read(&PreAuth::new(&store), &key),
            Ok(Some(counted(1, NOON)))
        );
    }
}
