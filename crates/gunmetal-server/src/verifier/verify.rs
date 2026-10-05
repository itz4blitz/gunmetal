//! The verifier itself: what happens to every sign-in attempt, in order.

use std::sync::Arc;

use gunmetal_core::audit_event::SecuritySink;
use gunmetal_core::client_context::ClientContext;
use gunmetal_core::time::Clock;
use gunmetal_durable::identity::store::IdentityStore;

use crate::limiter::rates::SourceLimiter;
use crate::log::Logger;
use crate::verifier::error::SignInError;
use crate::verifier::guesses::GuessLog;
use crate::verifier::pathway::{Pathway, PathwayCheck, Presented, Verified};
use crate::verifier::preauth::PreAuth;

/// The credential verifier.
pub struct Verifier {
    store: Arc<IdentityStore>,
}

impl Verifier {
    /// A verifier that reads stored credentials and keeps its guess log in
    /// `store`; limits attempts with `limiter`; reads the time from
    /// `clock`; writes failure lines to `log`; and emits security events
    /// into `sink`.
    #[must_use]
    pub fn new(
        store: Arc<IdentityStore>,
        limiter: SourceLimiter,
        guesses: GuessLog,
        clock: Arc<dyn Clock + Send + Sync>,
        log: Arc<Logger>,
        sink: Arc<dyn SecuritySink + Send + Sync>,
    ) -> Self {
        // Nothing is limited or reported yet.
        drop((limiter, guesses, clock, log, sink));
        Self { store }
    }

    /// Spends one attempt of `source`'s ceilings and the server's for an
    /// endpoint that starts a sign-in ceremony on `pathway`.
    ///
    /// # Errors
    ///
    /// [`SignInError::Limited`] when a ceiling is reached.
    pub fn begin(&self, _pathway: Pathway, _source: &ClientContext) -> Result<(), SignInError> {
        Ok(())
    }

    /// Verifies what a request from `source` presented on `check`'s
    /// pathway.
    ///
    /// # Errors
    ///
    /// [`SignInError::Limited`] when a limit refused the attempt, and
    /// [`SignInError::Refused`] for every credential that is not verified.
    pub fn verify(
        &self,
        check: &dyn PathwayCheck,
        presented: &Presented<'_>,
        _source: &ClientContext,
    ) -> Result<Verified, SignInError> {
        // The pathway's lookup is believed: nothing is compared, limited or
        // refused yet.
        let lookup = PreAuth::new(&self.store);
        let account = presented
            .credential()
            .and_then(|credential| check.find(credential, &lookup).ok().flatten())
            .and_then(|stored| stored.account);
        Ok(Verified {
            pathway: check.pathway(),
            account,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::sync::Arc;

    use gunmetal_core::audit_event::{SecurityEvent, SecuritySink};
    use gunmetal_core::client_context::ClientContext;
    use gunmetal_core::id::{IdKind, PublicId};
    use gunmetal_core::schema::SchemaPart;
    use gunmetal_core::time::Clock;
    use gunmetal_durable::identity::error::IdentityError;
    use gunmetal_durable::identity::store::IDENTITY;
    use gunmetal_fs::dataroot::DataRoot;
    use gunmetal_fs::sqlite::{Db, Pragmas, Query, Synchronous, open_db};
    use gunmetal_testkit::clock::ManualClock;
    use proptest::prelude::*;
    use proptest::test_runner::{Config, TestRunner};

    use super::*;
    use crate::limiter::delay::GUESS_DELAY_ROWS;
    use crate::limiter::rates::{Ceilings, SourceLimiter, TRACKED_KEYS};
    use crate::limiter::testing::source;
    use crate::log::{Level, Logger};
    use crate::testing::{self, Capture, Recording};
    use crate::verifier::error::SignInError;
    use crate::verifier::guesses::{GUESS_DELAYS, GuessLog};
    use crate::verifier::pathway::{
        Credential, Fault, Material, Pathway, PathwayCheck, Presented, Stored, Target, Verified,
    };
    use crate::verifier::preauth::PreAuth;
    use crate::verifier::testing::{Data, STAND_IN, add_name, data, open, text};

    /// The time every clock in these tests starts at, as a line writes it.
    const NOON: &str = "2026-10-03T12:00:00.000Z";

    /// The schema parts a store in these tests is opened with.
    const PARTS: [SchemaPart; 2] = [GUESS_DELAYS, STAND_IN];

    /// Each pathway's name in the failure line, in the inventory's order,
    /// written out here on its own.
    const NAMES: [&str; 7] = [
        "passkey",
        "paired_browser",
        "claim_code",
        "recovery_code",
        "recovery_link",
        "invitation",
        "pairing_code",
    ];

    const FIND: Query = Query::new("SELECT name FROM stand_in WHERE name = ?1");

    /// Where a fault is injected into the stand-in.
    #[derive(Debug, Clone, Copy)]
    enum Stage {
        Find,
        Matches,
    }

    /// The counting hook: what the stand-in was asked to do.
    #[derive(Default)]
    struct Seen {
        /// How many lookups it ran.
        finds: Cell<u32>,
        /// The material each comparison ran against.
        checked: RefCell<Vec<Vec<u8>>>,
    }

    /// What the stand-in says is stored under a name, when the name is in
    /// its table.
    #[derive(Clone, Copy)]
    struct Script {
        account: Option<PublicId>,
        kind: Pathway,
        usable: bool,
        secret: &'static [u8],
    }

    /// A pathway that looks a name up in a real table through the handle it
    /// is handed, and compares bytes.
    struct StandIn<'a> {
        pathway: Pathway,
        /// The stored credential the request names.
        name: &'a str,
        stored: Script,
        /// Whether its comparison says yes to anything.
        agrees: bool,
        fault: Option<(Stage, Fault)>,
        seen: &'a Seen,
    }

    impl PathwayCheck for StandIn<'_> {
        fn pathway(&self) -> Pathway {
            self.pathway
        }

        fn find(
            &self,
            _credential: Credential<'_>,
            lookup: &PreAuth<'_>,
        ) -> Result<Option<Stored>, Fault> {
            self.seen.finds.set(self.seen.finds.get() + 1);
            if let Some((Stage::Find, fault)) = &self.fault {
                return Err(fault.clone());
            }
            let rows = lookup.lookup(&FIND.bind(text(self.name)))?;
            Ok((!rows.is_empty()).then(|| Stored {
                account: self.stored.account,
                kind: self.stored.kind,
                usable: self.stored.usable,
                material: Material::new(self.stored.secret.to_vec()),
            }))
        }

        fn decoy(&self) -> Material {
            Material::new(b"decoy".to_vec())
        }

        fn matches(&self, credential: Credential<'_>, material: &Material) -> Result<bool, Fault> {
            self.seen
                .checked
                .borrow_mut()
                .push(material.as_bytes().to_vec());
            if let Some((Stage::Matches, fault)) = &self.fault {
                return Err(fault.clone());
            }
            Ok(self.agrees || credential.bytes() == material.as_bytes())
        }
    }

    fn account() -> PublicId {
        PublicId::parse("usr_0123456789abcdefghjkmnpqrs", IdKind::User).expect("a user ID")
    }

    /// Ada's credential for `pathway`: usable, and right when `right` is
    /// presented.
    fn ada(pathway: Pathway) -> Script {
        Script {
            account: Some(account()),
            kind: pathway,
            usable: true,
            secret: b"right",
        }
    }

    /// The stand-in for a request on `pathway` that names ada's credential.
    fn standin(pathway: Pathway, seen: &Seen) -> StandIn<'_> {
        StandIn {
            pathway,
            name: "ada",
            stored: ada(pathway),
            agrees: false,
            fault: None,
            seen,
        }
    }

    /// What verifying ada's credential on `pathway` answers.
    fn verified(pathway: Pathway) -> Verified {
        Verified {
            pathway,
            account: Some(account()),
        }
    }

    const REFUSED: Result<Verified, SignInError> = Err(SignInError::Refused);

    fn wait(retry_after_ms: u64) -> Result<Verified, SignInError> {
        Err(SignInError::Limited { retry_after_ms })
    }

    fn fail(source: &ClientContext, account: Option<PublicId>) -> SecurityEvent {
        SecurityEvent::AuthnLoginFail {
            source: *source,
            account,
        }
    }

    fn limited(source: &ClientContext) -> SecurityEvent {
        SecurityEvent::ExcessRateLimitExceeded { source: *source }
    }

    /// The failure line for an attempt at `ts` from `addr`.
    fn line(ts: &str, addr: &str, pathway: &str, cause: &str) -> String {
        format!(
            "{{\"ts\":\"{ts}\",\"level\":\"error\",\"event\":\"authn_login_fail\",\"addr\":\"{addr}\",\"v\":1,\"pathway\":\"{pathway}\",\"cause\":\"{cause}\"}}"
        )
    }

    /// A verifier with the limits the server runs with, on a real identity
    /// store, with a clock the test moves and a log and a sink it reads.
    struct Bench {
        data: Data,
        clock: Arc<ManualClock>,
        out: Capture,
        sink: Arc<Recording>,
        verifier: Verifier,
        /// How many lines and events the test has already looked at.
        lines: Cell<usize>,
        events: Cell<usize>,
    }

    impl Bench {
        /// The lines logged since the test last asked.
        fn new_lines(&self) -> Vec<String> {
            let lines: Vec<String> = self.out.text().lines().map(str::to_owned).collect();
            let seen = self.lines.replace(lines.len());
            lines[seen..].to_vec()
        }

        /// The security events recorded since the test last asked.
        fn new_events(&self) -> Vec<SecurityEvent> {
            let events = self.sink.events();
            let seen = self.events.replace(events.len());
            events[seen..].to_vec()
        }
    }

    /// A bench on `data`, whose store is opened with `parts` and holds a
    /// stand-in credential under each of `names`.
    fn bench_on(
        data: Data,
        parts: &[SchemaPart],
        names: &[&str],
        accept: bool,
        level: Level,
    ) -> Bench {
        let store = open(&data.root, parts);
        let stored: Vec<Query> = names.iter().copied().map(add_name).collect();
        store.write(&stored).expect("the names are stored");
        let (clock, manual) = testing::clock();
        let clock: Arc<dyn Clock + Send + Sync> = clock;
        let out = Capture::default();
        let log = Arc::new(Logger::new(clock.clone(), level, Box::new(out.clone())));
        let sink = Arc::new(Recording::new(accept));
        let recorder: Arc<dyn SecuritySink + Send + Sync> = sink.clone();
        let limiter = SourceLimiter::new(&Ceilings::DEFAULT, TRACKED_KEYS).expect("usable");
        let guesses = GuessLog::new(GUESS_DELAY_ROWS);
        let verifier = Verifier::new(store, limiter, guesses, clock, log, recorder);
        Bench {
            data,
            clock: manual,
            out,
            sink,
            verifier,
            lines: Cell::new(0),
            events: Cell::new(0),
        }
    }

    /// A bench on a new store that holds ada's credential.
    fn bench() -> Bench {
        bench_on(data("verifier"), &PARTS, &["ada"], true, Level::Info)
    }

    /// Takes the identity store's write lock on a second connection, so
    /// that the store's own writes are refused until it is given back.
    fn lock_writes(root: &DataRoot) -> Db {
        let other =
            open_db(root, &IDENTITY, Pragmas::new(Synchronous::Full)).expect("a second connection");
        other
            .execute(&Query::new("BEGIN IMMEDIATE"))
            .expect("the write lock");
        other
    }

    fn unlock_writes(other: &Db) {
        other.execute(&Query::new("ROLLBACK")).expect("released");
    }

    #[test]
    fn a_right_credential_verifies_and_leaves_nothing_behind() {
        let bench = bench();
        let seen = Seen::default();
        let check = standin(Pathway::Passkey, &seen);
        let home = source("192.168.1.20");
        assert_eq!(
            bench
                .verifier
                .verify(&check, &Presented::new(b"right"), &home),
            Ok(verified(Pathway::Passkey))
        );
        // The lookup ran through the handle, and the comparison once,
        // against what is stored.
        assert_eq!(seen.finds.get(), 1);
        assert_eq!(*seen.checked.borrow(), [b"right".to_vec()]);
        assert_eq!(bench.new_lines(), [""; 0]);
        assert_eq!(bench.new_events(), []);
    }

    /// Verifies: SEC-API-058, SEC-IAM-022, SEC-HIS-047
    #[test]
    fn unknown_disabled_and_wrong_get_one_answer_after_the_same_check() {
        let bench = bench();
        let from = source("203.0.113.7");
        let disabled = Script {
            usable: false,
            ..ada(Pathway::Passkey)
        };
        // The name the request gives, what is stored for ada, what is
        // presented, the material the comparison must have run against and
        // the account the event names.
        let (right, wrong, decoy) = (
            b"right".as_slice(),
            b"wrong".as_slice(),
            b"decoy".as_slice(),
        );
        let cases = [
            ("nobody", ada(Pathway::Passkey), right, decoy, None),
            ("ada", disabled, right, right, Some(account())),
            ("ada", ada(Pathway::Passkey), wrong, right, Some(account())),
        ];
        let mut answers = Vec::new();
        for (name, stored, presented, compared, named) in cases {
            let seen = Seen::default();
            let check = StandIn {
                name,
                stored,
                ..standin(Pathway::Passkey, &seen)
            };
            answers.push(
                bench
                    .verifier
                    .verify(&check, &Presented::new(presented), &from),
            );
            assert_eq!(seen.finds.get(), 1, "{name}");
            assert_eq!(*seen.checked.borrow(), [compared.to_vec()], "{name}");
            assert_eq!(
                bench.new_lines(),
                [line(NOON, "203.0.113.7", "passkey", "credential")]
            );
            assert_eq!(bench.new_events(), [fail(&from, named)]);
        }
        assert_eq!(answers, [REFUSED; 3]);
    }

    /// Verifies: SEC-HIS-047
    #[test]
    fn an_unknown_credential_is_refused_even_when_the_decoy_would_match() {
        let bench = bench();
        let seen = Seen::default();
        let check = StandIn {
            name: "nobody",
            ..standin(Pathway::RecoveryCode, &seen)
        };
        let from = source("203.0.113.7");
        assert_eq!(
            bench
                .verifier
                .verify(&check, &Presented::new(b"decoy"), &from),
            REFUSED
        );
        assert_eq!(*seen.checked.borrow(), [b"decoy".to_vec()]);
        assert_eq!(bench.new_events(), [fail(&from, None)]);
    }

    /// Verifies: SEC-HIS-047
    #[test]
    fn nothing_presented_verifies_for_an_unknown_disabled_or_foreign_credential() {
        // The text of the MD5 of nothing, which a rival took for a password.
        const MD5_OF_NOTHING: &[u8] = b"d41d8cd98f00b204e9800998ecf8427e";
        let bench = bench();
        let from = source("203.0.113.7");
        let stored = [
            ada(Pathway::Passkey),
            Script {
                usable: false,
                ..ada(Pathway::Passkey)
            },
            Script {
                kind: Pathway::Invitation,
                ..ada(Pathway::Passkey)
            },
        ];
        let names = ["nobody", "ada", "ada"];
        let presented = prop_oneof![
            Just(Vec::<u8>::new()),
            Just(MD5_OF_NOTHING.to_vec()),
            prop::collection::vec(any::<u8>(), 1..24),
        ];
        let mut runner = TestRunner::new(Config::with_cases(48));
        let ran = runner.run(&(presented, 0_usize..3), |(presented, case)| {
            let seen = Seen::default();
            // The stand-in says that whatever is presented matches, as a
            // pathway would that compared with an empty or default secret.
            let check = StandIn {
                name: names[case],
                stored: stored[case],
                agrees: true,
                ..standin(Pathway::Passkey, &seen)
            };
            let answer = bench
                .verifier
                .verify(&check, &Presented::new(&presented), &from);
            assert_eq!(answer, REFUSED);
            // Nothing empty reaches the comparison; anything else reaches it
            // once.
            let compared = seen.checked.borrow().len();
            assert_eq!(compared, usize::from(!presented.is_empty()));
            // A minute on, so the source's ceiling plays no part.
            bench.clock.advance(60_000);
            Ok(())
        });
        assert_eq!(ran, Ok(()));
    }

    /// Verifies: SEC-EXT-007
    #[test]
    fn a_pathway_accepts_only_a_credential_issued_for_it() {
        let bench = bench();
        for (row, pathway) in Pathway::ALL.iter().copied().enumerate() {
            for (column, kind) in Pathway::ALL.iter().copied().enumerate() {
                let seen = Seen::default();
                let check = StandIn {
                    stored: ada(kind),
                    ..standin(pathway, &seen)
                };
                let from = source(&format!("198.51.{row}.{column}"));
                // Every kind against every pathway: only its own verifies.
                let expected = if row == column {
                    Ok(verified(pathway))
                } else {
                    REFUSED
                };
                assert_eq!(
                    bench
                        .verifier
                        .verify(&check, &Presented::new(b"right"), &from),
                    expected,
                    "{kind:?} on {pathway:?}"
                );
                assert_eq!(*seen.checked.borrow(), [b"right".to_vec()]);
            }
        }
    }

    /// Verifies: SEC-HIS-004
    #[test]
    fn an_empty_or_missing_credential_is_refused_on_every_pathway_unchecked() {
        let bench = bench();
        for (index, pathway) in Pathway::ALL.iter().copied().enumerate() {
            let seen = Seen::default();
            // What is stored is empty too, and the stand-in says anything
            // matches: only the verifier stands in the way.
            let check = StandIn {
                stored: Script {
                    secret: b"",
                    ..ada(pathway)
                },
                agrees: true,
                ..standin(pathway, &seen)
            };
            let nothing = [Presented::new(b""), Presented::missing()];
            for (attempt, presented) in nothing.iter().enumerate() {
                let addr = format!("198.51.{index}.{attempt}");
                let from = source(&addr);
                assert_eq!(
                    bench.verifier.verify(&check, presented, &from),
                    REFUSED,
                    "{pathway:?}"
                );
                assert_eq!(bench.new_events(), [fail(&from, None)]);
                assert_eq!(
                    bench.new_lines(),
                    [line(NOON, &addr, NAMES[index], "credential")]
                );
            }
            assert_eq!(seen.finds.get(), 0, "{pathway:?}");
            assert_eq!(*seen.checked.borrow(), Vec::<Vec<u8>>::new(), "{pathway:?}");
        }
    }

    /// Verifies: SEC-HIS-004, SEC-API-056
    #[test]
    fn presenting_nothing_for_a_short_secret_counts_as_a_wrong_guess() {
        let bench = bench();
        let seen = Seen::default();
        let check = standin(Pathway::ClaimCode, &seen);
        let from = source("192.168.1.66");
        assert_eq!(
            bench.verifier.verify(&check, &Presented::missing(), &from),
            REFUSED
        );
        assert_eq!(
            bench
                .verifier
                .verify(&check, &Presented::new(b"right"), &from),
            wait(30_000)
        );
        assert_eq!(seen.finds.get(), 0);
    }

    /// Verifies: SEC-TM-014, SEC-HIS-046, SEC-IAM-101
    #[test]
    fn every_pathway_is_limited_and_reported_by_the_one_verifier() {
        // For each pathway of the inventory, in order: how many wrong
        // guesses in a row one source gets, and the wait it is then told.
        // The ceiling stops a strong secret's source after its burst of
        // ten; the schedule stops a guessable one's after its first.
        const LIMITS: [(usize, u64); 7] = [
            (10, 6_000),
            (10, 6_000),
            (1, 30_000),
            (10, 6_000),
            (10, 6_000),
            (10, 6_000),
            (1, 30_000),
        ];
        let bench = bench();
        assert_eq!(Pathway::ALL.len(), 7);
        for (index, pathway) in Pathway::ALL.iter().copied().enumerate() {
            let (guesses, retry_after_ms) = LIMITS[index];
            let seen = Seen::default();
            let check = standin(pathway, &seen);
            let addr = format!("198.51.100.{index}");
            let from = source(&addr);
            let answers: Vec<_> = (0..=guesses)
                .map(|_| {
                    bench
                        .verifier
                        .verify(&check, &Presented::new(b"wrong"), &from)
                })
                .collect();
            let mut expected = vec![REFUSED; guesses];
            expected.push(wait(retry_after_ms));
            assert_eq!(answers, expected, "{pathway:?}");
            // Only the attempts that were let through reached the check, and
            // each of those left a line and an event.
            assert_eq!(seen.checked.borrow().len(), guesses, "{pathway:?}");
            let failure = line(NOON, &addr, NAMES[index], "credential");
            assert_eq!(bench.new_lines(), vec![failure; guesses]);
            let mut events = vec![fail(&from, Some(account())); guesses];
            events.push(limited(&from));
            assert_eq!(bench.new_events(), events);
        }
    }

    /// Verifies: SEC-API-056
    #[test]
    fn wrong_guesses_wait_out_the_schedule_and_the_right_one_ends_it() {
        let bench = bench();
        let seen = Seen::default();
        let check = standin(Pathway::ClaimCode, &seen);
        let from = source("192.168.1.66");
        let guess = |secret: &'static [u8]| {
            bench
                .verifier
                .verify(&check, &Presented::new(secret), &from)
        };
        assert_eq!(guess(b"wrong"), REFUSED);
        // 30 seconds, 1 minute, 5 minutes, then 15 minutes and no longer.
        for delay in [30_000_i64, 60_000, 300_000, 900_000, 900_000] {
            // Even the right code waits, and is not looked at.
            assert_eq!(guess(b"right"), wait(delay.unsigned_abs()));
            bench.clock.advance(delay - 1);
            assert_eq!(guess(b"right"), wait(1));
            bench.clock.advance(1);
            assert_eq!(guess(b"wrong"), REFUSED);
        }
        // Never for good: after six wrong guesses and one more wait, the
        // right code is accepted.
        assert_eq!(guess(b"right"), wait(900_000));
        bench.clock.advance(900_000);
        assert_eq!(guess(b"right"), Ok(verified(Pathway::ClaimCode)));
        // And it ended the count: the next wrong guess is a first one.
        assert_eq!(guess(b"wrong"), REFUSED);
        assert_eq!(guess(b"right"), wait(30_000));
        assert_eq!(seen.finds.get(), 8);
    }

    /// Verifies: SEC-API-056
    #[test]
    fn guesses_made_at_the_same_moment_are_counted_one_at_a_time() {
        let bench = bench();
        let verifier = &bench.verifier;
        let from = source("192.168.1.66");
        // Four wrong guesses at one code, from one source, on four threads.
        let answers: Vec<_> = std::thread::scope(|scope| {
            let guessers: Vec<_> = (0..4)
                .map(|_| {
                    scope.spawn(|| {
                        let seen = Seen::default();
                        let check = standin(Pathway::PairingCode, &seen);
                        verifier.verify(&check, &Presented::new(b"wrong"), &from)
                    })
                })
                .collect();
            guessers
                .into_iter()
                .map(|guesser| guesser.join().expect("the guess returned"))
                .collect()
        });
        // Whichever came first was looked at and counted before any other
        // was let through, so the other three wait.
        let looked_at = answers.iter().filter(|answer| **answer == REFUSED).count();
        let waiting = answers
            .iter()
            .filter(|answer| **answer == wait(30_000))
            .count();
        assert_eq!((looked_at, waiting), (1, 3));
    }

    #[test]
    fn a_different_target_from_the_same_address_is_a_first_guess_under_the_same_ceiling() {
        let bench = bench();
        let seen = Seen::default();
        let check = standin(Pathway::PairingCode, &seen);
        let from = source("192.168.1.66");
        let guess = |target: u8| {
            let aimed = Presented::new(b"wrong").aimed_at(Target::from_bytes([target; 16]));
            bench.verifier.verify(&check, &aimed, &from)
        };
        assert_eq!(guess(1), REFUSED);
        assert_eq!(guess(1), wait(30_000));
        // Another person's code from the same address is not delayed by
        // the first one's wrong guess.
        assert_eq!(guess(2), REFUSED);
        assert_eq!(guess(2), wait(30_000));
        // But the address has one ceiling for all of them: ten attempts,
        // four of them made, then none for six seconds.
        let fresh: Vec<_> = (3..=8).map(guess).collect();
        assert_eq!(fresh, [REFUSED; 6]);
        assert_eq!(guess(9), wait(6_000));
        assert_eq!(seen.finds.get(), 8);
    }

    /// Verifies: SEC-IAM-008
    #[test]
    fn a_hostile_source_never_delays_the_claim_code_for_another_source_or_the_host() {
        let bench = bench();
        let seen = Seen::default();
        let check = standin(Pathway::ClaimCode, &seen);
        let guess = |secret: &'static [u8], from: &ClientContext| {
            bench.verifier.verify(&check, &Presented::new(secret), from)
        };
        // Fifty wrong guesses from one address on the home network: the
        // first is looked at, nine wait on the schedule, and the rest are
        // past the address's ceiling.
        let hostile = source("192.168.1.66");
        let answers: Vec<_> = (0..50).map(|_| guess(b"wrong", &hostile)).collect();
        let mut expected = vec![REFUSED];
        expected.extend([wait(30_000); 9]);
        expected.extend([wait(6_000); 40]);
        assert_eq!(answers, expected);
        assert_eq!(seen.finds.get(), 1);
        // The owner is not delayed, on another device or at the host.
        let host = source("127.0.0.1");
        assert_eq!(
            guess(b"right", &source("192.168.1.20")),
            Ok(verified(Pathway::ClaimCode))
        );
        assert_eq!(guess(b"right", &host), Ok(verified(Pathway::ClaimCode)));
        // And the host is never delayed, whatever it has got wrong itself.
        let at_host: Vec<_> = (0..50).map(|_| guess(b"wrong", &host)).collect();
        assert_eq!(at_host, [REFUSED; 50]);
        assert_eq!(guess(b"right", &host), Ok(verified(Pathway::ClaimCode)));
        assert_eq!(seen.finds.get(), 54);
    }

    /// Verifies: SEC-IAM-101
    #[test]
    fn attempts_from_many_sources_reach_the_server_wide_ceiling() {
        let bench = bench();
        let seen = Seen::default();
        let check = standin(Pathway::Invitation, &seen);
        let guess = |secret: &'static [u8], from: &ClientContext| {
            bench.verifier.verify(&check, &Presented::new(secret), from)
        };
        let answers: Vec<_> = (0..100)
            .map(|n| guess(b"wrong", &source(&format!("198.51.100.{n}"))))
            .collect();
        assert_eq!(answers, [REFUSED; 100]);
        bench.new_events();
        // The hundred-and-first source has spent nothing of its own, and
        // is refused unseen all the same, for 600 milliseconds.
        let late = source("198.51.100.200");
        assert_eq!(guess(b"right", &late), wait(600));
        assert_eq!(bench.new_events(), [limited(&late)]);
        bench.clock.advance(600);
        assert_eq!(guess(b"right", &late), Ok(verified(Pathway::Invitation)));
        assert_eq!(seen.finds.get(), 101);
    }

    /// Verifies: SEC-IAM-101
    #[test]
    fn starting_a_ceremony_spends_the_same_ceilings() {
        let bench = bench();
        let from = source("203.0.113.7");
        let mut expected = vec![Ok(()); 10];
        expected.push(Err(SignInError::Limited {
            retry_after_ms: 6_000,
        }));
        let starts: Vec<_> = (0..11)
            .map(|_| bench.verifier.begin(Pathway::Passkey, &from))
            .collect();
        assert_eq!(starts, expected);
        assert_eq!(bench.new_events(), [limited(&from)]);
        assert_eq!(bench.new_lines(), [""; 0]);
        // Checking a credential draws on the ceiling that starting spent.
        let seen = Seen::default();
        let check = standin(Pathway::Passkey, &seen);
        assert_eq!(
            bench
                .verifier
                .verify(&check, &Presented::new(b"right"), &from),
            wait(6_000)
        );
        // Only the claim is free of the ceilings at the host; any other
        // pathway is held to them there as anywhere.
        let host = source("127.0.0.1");
        let claims = (0..50)
            .filter(|_| bench.verifier.begin(Pathway::ClaimCode, &host) == Ok(()))
            .count();
        assert_eq!(claims, 50);
        let at_host: Vec<_> = (0..11)
            .map(|_| bench.verifier.begin(Pathway::Passkey, &host))
            .collect();
        assert_eq!(at_host, expected);
    }

    /// Verifies: SEC-IAM-069
    #[test]
    fn a_fault_in_the_check_is_a_denial_and_exactly_one_security_event() {
        let bench = bench();
        let faults = [
            (Fault::Timeout, "timeout"),
            (Fault::Missing, "missing_data"),
            (Fault::from(IdentityError::Foreign), "storage"),
        ];
        // A fault in the lookup comes before any account is known; one in
        // the comparison, after ada's credential was found.
        let stages = [(Stage::Find, None), (Stage::Matches, Some(account()))];
        for (row, (fault, cause)) in faults.iter().enumerate() {
            for (column, (stage, named)) in stages.iter().enumerate() {
                let seen = Seen::default();
                let check = StandIn {
                    fault: Some((*stage, fault.clone())),
                    ..standin(Pathway::RecoveryCode, &seen)
                };
                let addr = format!("198.51.{row}.{column}");
                let from = source(&addr);
                // The credential is the right one, and is still refused.
                assert_eq!(
                    bench
                        .verifier
                        .verify(&check, &Presented::new(b"right"), &from),
                    REFUSED,
                    "{cause} at {stage:?}"
                );
                assert_eq!(bench.new_events(), [fail(&from, *named)]);
                assert_eq!(
                    bench.new_lines(),
                    [line(NOON, &addr, "recovery_code", cause)]
                );
            }
        }
    }

    /// Verifies: SEC-IAM-069
    #[test]
    fn a_store_the_check_cannot_read_is_a_denial() {
        // A store with no table for the stand-in's lookup: SQLite refuses
        // the query the pathway runs through the handle.
        let bench = bench_on(
            data("verifier-bare"),
            &[GUESS_DELAYS],
            &[],
            true,
            Level::Info,
        );
        let seen = Seen::default();
        let check = standin(Pathway::Passkey, &seen);
        let from = source("203.0.113.7");
        assert_eq!(
            bench
                .verifier
                .verify(&check, &Presented::new(b"right"), &from),
            REFUSED
        );
        assert_eq!(bench.new_events(), [fail(&from, None)]);
        assert_eq!(
            bench.new_lines(),
            [line(NOON, "203.0.113.7", "passkey", "storage")]
        );
        assert_eq!(*seen.checked.borrow(), Vec::<Vec<u8>>::new());
    }

    /// Verifies: SEC-IAM-069
    #[test]
    fn a_guess_log_that_cannot_be_read_is_a_denial_before_the_check() {
        // A store opened without the guess log's table.
        let bench = bench_on(
            data("verifier-no-log"),
            &[STAND_IN],
            &["ada"],
            true,
            Level::Info,
        );
        let seen = Seen::default();
        let claim = standin(Pathway::ClaimCode, &seen);
        let from = source("192.168.1.20");
        assert_eq!(
            bench
                .verifier
                .verify(&claim, &Presented::new(b"right"), &from),
            REFUSED
        );
        assert_eq!(seen.finds.get(), 0);
        assert_eq!(bench.new_events(), [fail(&from, None)]);
        assert_eq!(
            bench.new_lines(),
            [line(NOON, "192.168.1.20", "claim_code", "storage")]
        );
        // What needs no guess log is not affected: a strong secret, and
        // the claim at the host.
        let passkey = standin(Pathway::Passkey, &seen);
        assert_eq!(
            bench
                .verifier
                .verify(&passkey, &Presented::new(b"right"), &from),
            Ok(verified(Pathway::Passkey))
        );
        let host = source("127.0.0.1");
        assert_eq!(
            bench
                .verifier
                .verify(&claim, &Presented::new(b"right"), &host),
            Ok(verified(Pathway::ClaimCode))
        );
    }

    /// Verifies: SEC-IAM-069
    #[test]
    fn a_guess_log_that_cannot_be_written_never_lets_a_guess_through() {
        let bench = bench();
        let seen = Seen::default();
        let check = standin(Pathway::PairingCode, &seen);
        let from = source("192.168.1.66");
        let guess = |secret: &'static [u8]| {
            bench
                .verifier
                .verify(&check, &Presented::new(secret), &from)
        };
        let lock = lock_writes(&bench.data.root);
        // A first right guess has nothing to clear, so it needs no write.
        assert_eq!(guess(b"right"), Ok(verified(Pathway::PairingCode)));
        // A wrong guess is refused although it could not be counted.
        assert_eq!(guess(b"wrong"), REFUSED);
        unlock_writes(&lock);
        // This one is counted.
        assert_eq!(guess(b"wrong"), REFUSED);
        bench.clock.advance(30_000);
        bench.new_events();
        bench.new_lines();
        // With a wrong guess on record the right one must clear it first,
        // and is refused while that cannot be written.
        let lock = lock_writes(&bench.data.root);
        assert_eq!(guess(b"right"), REFUSED);
        assert_eq!(bench.new_events(), [fail(&from, Some(account()))]);
        assert_eq!(
            bench.new_lines(),
            [line(
                "2026-10-03T12:00:30.000Z",
                "192.168.1.66",
                "pairing_code",
                "storage"
            )]
        );
        unlock_writes(&lock);
        assert_eq!(guess(b"right"), Ok(verified(Pathway::PairingCode)));
    }

    /// Verifies: SEC-API-056
    #[test]
    fn a_clock_set_back_never_makes_a_guess_wait_longer_than_the_schedule() {
        let bench = bench();
        let seen = Seen::default();
        let check = standin(Pathway::ClaimCode, &seen);
        let from = source("192.168.1.66");
        let guess = |secret: &'static [u8]| {
            bench
                .verifier
                .verify(&check, &Presented::new(secret), &from)
        };
        assert_eq!(guess(b"wrong"), REFUSED);
        // The clock is set back an hour: the wrong guess is now in the
        // future. It waits the schedule's 30 seconds from here, not an hour
        // and 30 seconds.
        bench.clock.advance(-3_600_000);
        assert_eq!(guess(b"right"), wait(30_000));
        bench.clock.advance(29_999);
        assert_eq!(guess(b"right"), wait(1));
        bench.clock.advance(1);
        assert_eq!(guess(b"right"), Ok(verified(Pathway::ClaimCode)));
    }

    /// Verifies: SEC-IAM-069
    #[test]
    fn a_guess_whose_time_cannot_be_moved_is_refused() {
        let bench = bench();
        let seen = Seen::default();
        let check = standin(Pathway::ClaimCode, &seen);
        let from = source("192.168.1.66");
        let guess = |secret: &'static [u8]| {
            bench
                .verifier
                .verify(&check, &Presented::new(secret), &from)
        };
        assert_eq!(guess(b"wrong"), REFUSED);
        bench.clock.advance(-3_600_000);
        bench.new_events();
        let lock = lock_writes(&bench.data.root);
        assert_eq!(guess(b"right"), REFUSED);
        assert_eq!(bench.new_events(), [fail(&from, None)]);
        unlock_writes(&lock);
        // Nothing was looked at, and the wait still starts from here.
        assert_eq!(seen.finds.get(), 1);
        assert_eq!(guess(b"right"), wait(30_000));
    }

    #[test]
    fn a_refusal_stands_when_the_audit_log_cannot_record_it() {
        let bench = bench_on(
            data("verifier-no-audit"),
            &PARTS,
            &["ada"],
            false,
            Level::Info,
        );
        let seen = Seen::default();
        let check = standin(Pathway::Passkey, &seen);
        let from = source("203.0.113.7");
        let answers: Vec<_> = (0..11)
            .map(|_| {
                bench
                    .verifier
                    .verify(&check, &Presented::new(b"wrong"), &from)
            })
            .collect();
        let mut expected = vec![REFUSED; 10];
        expected.push(wait(6_000));
        assert_eq!(answers, expected);
        // The lines are written whatever the audit log answers.
        let failure = line(NOON, "203.0.113.7", "passkey", "credential");
        assert_eq!(bench.new_lines(), vec![failure; 10]);
        assert_eq!(bench.new_events(), []);
    }

    /// Verifies: SEC-IAM-099, SEC-OPS-028
    #[test]
    fn a_failure_is_one_line_in_the_documented_format_at_any_log_level() {
        // A server set to log only errors still writes the line.
        let bench = bench_on(data("verifier-line"), &PARTS, &["ada"], true, Level::Error);
        let seen = Seen::default();
        let check = standin(Pathway::ClaimCode, &seen);
        for addr in ["203.0.113.7", "2001:db8::7"] {
            assert_eq!(
                bench
                    .verifier
                    .verify(&check, &Presented::new(b"wrong"), &source(addr)),
                REFUSED
            );
        }
        // Format version 1, as the verifier's documentation gives it.
        assert_eq!(
            bench.out.text(),
            concat!(
                r#"{"ts":"2026-10-03T12:00:00.000Z","level":"error","event":"authn_login_fail","addr":"203.0.113.7","v":1,"pathway":"claim_code","cause":"credential"}"#,
                "\n",
                r#"{"ts":"2026-10-03T12:00:00.000Z","level":"error","event":"authn_login_fail","addr":"2001:db8::7","v":1,"pathway":"claim_code","cause":"credential"}"#,
                "\n",
            )
        );
        // What the published fail2ban filter anchors on.
        let anchored = bench
            .out
            .text()
            .matches(r#""event":"authn_login_fail","addr":""#)
            .count();
        assert_eq!(anchored, 2);
    }

    #[test]
    fn a_restart_forgives_no_wrong_guess() {
        let first = bench();
        let seen = Seen::default();
        let check = standin(Pathway::ClaimCode, &seen);
        let from = source("192.168.1.66");
        assert_eq!(
            first
                .verifier
                .verify(&check, &Presented::new(b"wrong"), &from),
            REFUSED
        );
        // Stop the server and start it again on the same data directory:
        // its limiter is new, its guess log is not.
        let Bench { data, verifier, .. } = first;
        drop(verifier);
        let again = bench_on(data, &PARTS, &["ada"], true, Level::Info);
        assert_eq!(
            again
                .verifier
                .verify(&check, &Presented::new(b"right"), &from),
            wait(30_000)
        );
        again.clock.advance(30_000);
        assert_eq!(
            again
                .verifier
                .verify(&check, &Presented::new(b"right"), &from),
            Ok(verified(Pathway::ClaimCode))
        );
    }
}
