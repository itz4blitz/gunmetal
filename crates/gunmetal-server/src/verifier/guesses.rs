//! The guess log: the wrong guesses the delay schedule counts, kept in the
//! identity store so that a restart forgives none of them.

use gunmetal_core::client_context::ClientContext;
use gunmetal_core::schema::{Column, DataClass, SchemaPart};
use gunmetal_core::time::Timestamp;
use gunmetal_durable::identity::store::IdentityStore;
use gunmetal_fs::sqlite::Row;

use crate::limiter::delay::Failures;
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

/// Whose wrong guesses a row counts: one source's, at one target of one
/// pathway.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuessKey;

impl GuessKey {
    /// The key for guesses from `source` on `pathway`, aimed at `target`
    /// when the pathway has more than one thing to guess.
    #[must_use]
    pub fn new(_pathway: Pathway, _target: Option<Target>, _source: &ClientContext) -> Self {
        Self
    }
}

/// What reading a key returned, as the wrong guesses on record: no row, or
/// one row of a count and a time.
///
/// # Errors
///
/// [`Fault::Missing`] when the rows are anything else.
pub fn decode(_rows: &[Row]) -> Result<Option<Failures>, Fault> {
    Ok(None)
}

/// The guess log.
#[derive(Debug, Clone, Copy)]
pub struct GuessLog;

impl GuessLog {
    /// A log that keeps the `capacity` rows written last.
    #[must_use]
    pub fn new(_capacity: u32) -> Self {
        Self
    }

    /// The wrong guesses on record for `key`.
    ///
    /// # Errors
    ///
    /// A [`Fault`] when the store cannot be read.
    pub fn read(self, _lookup: &PreAuth<'_>, _key: &GuessKey) -> Result<Option<Failures>, Fault> {
        Ok(None)
    }

    /// Counts one more wrong guess for `key`, made at `now`.
    ///
    /// # Errors
    ///
    /// [`Fault::Storage`] when the store cannot be written.
    pub fn record(
        self,
        _store: &IdentityStore,
        _key: &GuessKey,
        _now: Timestamp,
    ) -> Result<(), Fault> {
        Ok(())
    }

    /// Makes the wrong guesses on record for `key` count from `now`.
    ///
    /// # Errors
    ///
    /// [`Fault::Storage`] when the store cannot be written.
    pub fn rebase(
        self,
        _store: &IdentityStore,
        _key: &GuessKey,
        _now: Timestamp,
    ) -> Result<(), Fault> {
        Ok(())
    }

    /// Forgets the wrong guesses on record for `key`.
    ///
    /// # Errors
    ///
    /// [`Fault::Storage`] when the store cannot be written.
    pub fn clear(self, _store: &IdentityStore, _key: &GuessKey) -> Result<(), Fault> {
        Ok(())
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

    fn failed(count: u32, ms: i64) -> Result<Option<Failures>, Fault> {
        Ok(Some(Failures {
            count,
            last_at: at(ms),
        }))
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
        assert_eq!(log.read(&lookup, &key), failed(1, NOON));
        assert_eq!(log.record(&store, &key, at(NOON + 30_000)), Ok(()));
        assert_eq!(log.record(&store, &key, at(NOON + 90_000)), Ok(()));
        assert_eq!(log.read(&lookup, &key), failed(3, NOON + 90_000));
        // Moving the time keeps the count.
        assert_eq!(log.rebase(&store, &key, at(NOON - 5)), Ok(()));
        assert_eq!(log.read(&lookup, &key), failed(3, NOON - 5));
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
                failed(1, NOON),
                failed(2, NOON + 11),
                failed(3, NOON + 22),
                failed(4, NOON + 33),
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
                failed(1, NOON),
                failed(2, NOON + 11),
                Ok(None),
                failed(4, NOON + 33),
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
            failed(2, NOON + 30_000)
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
        assert_eq!(none.read(&lookup, &first), failed(1, NOON + 5));
    }

    #[test]
    fn reads_only_one_row_of_a_count_and_a_time() {
        let row = |count: i64, ms: i64| Row(vec![Value::Integer(count), Value::Integer(ms)]);
        assert_eq!(decode(&[]), Ok(None));
        assert_eq!(decode(&[row(3, NOON)]), failed(3, NOON));
        // A count past what the schedule's type holds is its largest.
        assert_eq!(decode(&[row(i64::MAX, NOON)]), failed(u32::MAX, NOON));
        assert_eq!(decode(&[row(4_294_967_296, NOON)]), failed(u32::MAX, NOON));
        assert_eq!(decode(&[row(4_294_967_295, NOON)]), failed(u32::MAX, NOON));
        assert_eq!(decode(&[row(-1, NOON)]), failed(u32::MAX, NOON));
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
        assert_eq!(log.read(&PreAuth::new(&store), &key), failed(1, NOON));
    }
}
