//! The guess counts: the wrong guesses the delay schedule counts, kept in
//! memory and nowhere else.
//!
//! An entry counts one source's wrong guesses at one target of one pathway
//! since its last right one, and when the latest was made ([`Failures`]).
//! Only the pathways whose secrets can be guessed have entries
//! (SEC-API-056).
//!
//! A guess is counted before it is looked at. [`GuessCounts::charge`]
//! decides whether the schedule lets the guess through and, when it does,
//! counts it as wrong, in one step under one lock, so that of guesses made
//! at the same moment only the first is looked at. The verifier clears the
//! count when the guess turns out right. Counting needs nothing that can
//! fail, so no wrong guess goes uncounted. The lock is held for that step
//! only: never while a guess is looked at, a line is logged or an event is
//! emitted.
//!
//! Nothing here is written anywhere, so no client address, and nothing
//! made from one, is kept at rest (SEC-PRV-003; architecture record 3
//! counts the limiter's counters as no one's user state). A restart
//! forgets every count, as it forgets the ceilings.
//!
//! The counts are state kept for callers who have not signed in, so they
//! have room for a fixed number of keys (SEC-NET-051). A full store never
//! drops a count whose wait is still running, since that would let its
//! source guess again at once. A new key takes the place of the count
//! whose wait ended longest ago, and while every wait is still running the
//! new key's guess is refused until the first of them ends. With the
//! registered limits the store is never full of running waits: in the
//! longest wait, fifteen minutes, the server-wide ceiling lets fewer
//! attempts through than there is room for.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use gunmetal_core::client_context::ClientContext;
use gunmetal_core::ratelimit::{Decision, next_guess_at};
use gunmetal_core::time::Timestamp;

use crate::limiter::delay::{Failures, guess_allowed, source_label};
use crate::verifier::pathway::{Pathway, Target};

/// Whose wrong guesses an entry counts: one source's, at one target of one
/// pathway. Only the verifier makes one.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(in crate::verifier) struct GuessKey {
    pathway: Pathway,
    target: Option<Target>,
    source: String,
}

impl GuessKey {
    /// The key for guesses from `source` on `pathway`, aimed at `target`
    /// when the pathway has more than one thing to guess.
    #[must_use]
    pub(in crate::verifier) fn new(
        pathway: Pathway,
        target: Option<Target>,
        source: &ClientContext,
    ) -> Self {
        Self {
            pathway,
            target,
            source: source_label(source),
        }
    }
}

/// `failures`, counted from no later than `now`. A clock set back would
/// otherwise leave a wrong guess in the future, and its source waiting for
/// the clock to catch up before it waited out the schedule.
fn settled(failures: Failures, now: Timestamp) -> Failures {
    Failures {
        count: failures.count,
        last_at: failures.last_at.min(now),
    }
}

/// Makes room in `counts`, which has room for `capacity` keys, for one more
/// key at `now`. A full store gives up the count whose wait ended first,
/// once it has ended, and otherwise says how long until it ends; it never
/// gives up a count whose wait is still running.
fn room(counts: &mut HashMap<GuessKey, Failures>, capacity: usize, now: Timestamp) -> Decision {
    if counts.len() < capacity {
        return Decision::Allow;
    }
    let first = counts
        .iter()
        .map(|(key, failures)| (settled(*failures, now), key))
        .min_by_key(|(failures, _)| next_guess_at(failures.count, failures.last_at))
        .map(|(failures, key)| (failures, key.clone()));
    let decision = guess_allowed(first.as_ref().map(|(failures, _)| *failures), now);
    if let (Decision::Allow, Some((_, key))) = (decision, first) {
        counts.remove(&key);
    }
    decision
}

/// The wrong guesses the delay schedule counts, for a fixed number of keys.
pub struct GuessCounts {
    capacity: usize,
    counts: Mutex<HashMap<GuessKey, Failures>>,
}

impl GuessCounts {
    /// Counts with room for `capacity` keys. There is always room for one,
    /// so no setting switches the delay off.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            counts: Mutex::new(HashMap::new()),
        }
    }

    /// The counts. They are whole whenever the lock is free: each step
    /// changes them only once it has decided, so a holder that panicked
    /// left them sound.
    fn lock(&self) -> MutexGuard<'_, HashMap<GuessKey, Failures>> {
        self.counts.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Lets a guess for `key` at `now` through when the delay schedule
    /// allows it, and then counts it as wrong before anyone has looked at
    /// it; otherwise says how long to wait.
    #[must_use]
    pub(in crate::verifier) fn charge(&self, key: &GuessKey, now: Timestamp) -> Decision {
        // Red: decides on what is held, and counts nothing.
        let mut counts = self.lock();
        let before = counts.get(key).map(|found| settled(*found, now));
        before.map_or_else(
            || room(&mut counts, self.capacity, now),
            |failures| guess_allowed(Some(failures), now),
        )
    }

    /// Forgets the wrong guesses counted for `key`.
    pub(in crate::verifier) fn clear(&self, key: &GuessKey) {
        self.lock().remove(key);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use gunmetal_core::ratelimit::Decision;
    use gunmetal_core::time::Timestamp;

    use super::*;
    use crate::limiter::delay::{Failures, GUESS_DELAY_KEYS};
    use crate::limiter::rates::Ceilings;
    use crate::limiter::testing::source;
    use crate::testing::NOON;
    use crate::verifier::pathway::{Pathway, Target};

    fn at(ms: i64) -> Timestamp {
        Timestamp::from_millis(ms).expect("in range")
    }

    fn counted(count: u32, ms: i64) -> Failures {
        Failures {
            count,
            last_at: at(ms),
        }
    }

    fn wait(retry_after_ms: u64) -> Decision {
        Decision::Deny { retry_after_ms }
    }

    /// The key for guesses at the claim code from `addr`.
    fn claim_from(addr: &str) -> GuessKey {
        GuessKey::new(Pathway::ClaimCode, None, &source(addr))
    }

    /// Every count the store holds, by key.
    fn held(counts: &GuessCounts) -> HashMap<GuessKey, Failures> {
        counts.lock().clone()
    }

    #[test]
    fn counts_each_guess_it_lets_through_until_it_is_told_to_forget_them() {
        let counts = GuessCounts::new(8);
        let key = claim_from("192.168.1.66");
        // The first guess is let through, and counted as it goes.
        assert_eq!(counts.charge(&key, at(NOON)), Decision::Allow);
        assert_eq!(
            held(&counts),
            HashMap::from([(key.clone(), counted(1, NOON))])
        );
        // The next waits out the schedule, and waiting is not counted.
        assert_eq!(counts.charge(&key, at(NOON + 29_999)), wait(1));
        assert_eq!(counts.charge(&key, at(NOON + 30_000)), Decision::Allow);
        assert_eq!(counts.charge(&key, at(NOON + 30_000)), wait(60_000));
        assert_eq!(counts.charge(&key, at(NOON + 90_000)), Decision::Allow);
        assert_eq!(
            held(&counts),
            HashMap::from([(key.clone(), counted(3, NOON + 90_000))])
        );
        counts.clear(&key);
        assert_eq!(held(&counts), HashMap::new());
        // Forgetting what is not there changes nothing, and the next guess
        // is a first one.
        counts.clear(&key);
        assert_eq!(counts.charge(&key, at(NOON + 90_001)), Decision::Allow);
        assert_eq!(
            held(&counts),
            HashMap::from([(key, counted(1, NOON + 90_001))])
        );
    }

    #[test]
    fn a_count_a_clock_set_back_left_in_the_future_counts_from_now() {
        let counts = GuessCounts::new(8);
        let key = claim_from("192.168.1.66");
        assert_eq!(counts.charge(&key, at(NOON)), Decision::Allow);
        // An hour back: the wait is the schedule's from here, and the count
        // keeps the time it was moved to.
        assert_eq!(counts.charge(&key, at(NOON - 3_600_000)), wait(30_000));
        assert_eq!(
            held(&counts),
            HashMap::from([(key.clone(), counted(1, NOON - 3_600_000))])
        );
        assert_eq!(counts.charge(&key, at(NOON - 3_570_001)), wait(1));
        assert_eq!(counts.charge(&key, at(NOON - 3_570_000)), Decision::Allow);
        assert_eq!(
            held(&counts),
            HashMap::from([(key, counted(2, NOON - 3_570_000))])
        );
    }

    #[test]
    fn keeps_each_pathway_target_and_source_apart() {
        let counts = GuessCounts::new(8);
        let approver = Target::from_bytes([0xab; 16]);
        let from = source("203.0.113.7");
        let keys = [
            GuessKey::new(Pathway::ClaimCode, None, &from),
            GuessKey::new(Pathway::PairingCode, None, &from),
            GuessKey::new(Pathway::PairingCode, Some(approver), &from),
            GuessKey::new(
                Pathway::PairingCode,
                Some(approver),
                &source("2001:db8:0:1234:aaaa:bbbb:cccc:dddd"),
            ),
        ];
        // Each key's first guess is let through, whatever the others did.
        let first: Vec<_> = keys
            .iter()
            .map(|key| counts.charge(key, at(NOON)))
            .collect();
        assert_eq!(first, [Decision::Allow; 4]);
        // Another address of the same /64 is the same source, and an IPv4
        // address written as IPv6 is that address.
        let same = [
            GuessKey::new(
                Pathway::PairingCode,
                Some(approver),
                &source("2001:db8:0:1234::1"),
            ),
            GuessKey::new(Pathway::ClaimCode, None, &source("::ffff:203.0.113.7")),
        ];
        let again: Vec<_> = same
            .iter()
            .map(|key| counts.charge(key, at(NOON + 1)))
            .collect();
        assert_eq!(again, [wait(29_999); 2]);
        // Forgetting one leaves the others.
        counts.clear(&keys[2]);
        assert_eq!(
            held(&counts),
            HashMap::from([
                (keys[0].clone(), counted(1, NOON)),
                (keys[1].clone(), counted(1, NOON)),
                (keys[3].clone(), counted(1, NOON)),
            ])
        );
    }

    /// Verifies: SEC-API-056
    #[test]
    fn a_full_store_never_drops_a_wait_that_is_still_running() {
        let counts = GuessCounts::new(3);
        let [first, second, third, fourth] =
            ["192.0.2.1", "192.0.2.2", "192.0.2.3", "192.0.2.4"].map(claim_from);
        // Three sources guess once each, a millisecond apart, so their waits
        // end 30 seconds on in the same order. The third takes the last
        // place there is.
        assert_eq!(counts.charge(&first, at(NOON)), Decision::Allow);
        assert_eq!(counts.charge(&second, at(NOON + 1)), Decision::Allow);
        assert_eq!(counts.charge(&third, at(NOON + 2)), Decision::Allow);
        // Full, with every wait still running: however many new keys come,
        // each is refused until the first wait ends, and nothing is dropped.
        let flood: Vec<_> = (0..1_000_u128)
            .map(|n| {
                let target = Target::from_bytes(n.to_be_bytes());
                let key = GuessKey::new(Pathway::PairingCode, Some(target), &source("192.0.2.9"));
                counts.charge(&key, at(NOON + 3))
            })
            .collect();
        assert_eq!(flood, vec![wait(29_997); 1_000]);
        let waiting = HashMap::from([
            (first.clone(), counted(1, NOON)),
            (second.clone(), counted(1, NOON + 1)),
            (third.clone(), counted(1, NOON + 2)),
        ]);
        assert_eq!(held(&counts), waiting);
        // The first source still waits out its own 30 seconds, to the
        // millisecond.
        assert_eq!(counts.charge(&first, at(NOON + 29_999)), wait(1));
        // Once that wait is over, a new key takes its place, and it is the
        // only count given up.
        assert_eq!(counts.charge(&fourth, at(NOON + 30_000)), Decision::Allow);
        assert_eq!(
            held(&counts),
            HashMap::from([
                (second, counted(1, NOON + 1)),
                (third, counted(1, NOON + 2)),
                (fourth, counted(1, NOON + 30_000)),
            ])
        );
        // And it came back too early to have a place of its own.
        assert_eq!(counts.charge(&first, at(NOON + 30_000)), wait(1));
    }

    #[test]
    fn a_new_key_takes_the_place_of_the_wait_that_ended_longest_ago() {
        let counts = GuessCounts::new(3);
        let [persistent, early, late, newcomer, follower, straggler] = [
            "192.0.2.1",
            "192.0.2.2",
            "192.0.2.3",
            "192.0.2.4",
            "192.0.2.5",
            "192.0.2.6",
        ]
        .map(claim_from);
        // A source made four wrong guesses as soon as it could, the last at
        // 390 seconds: it waits fifteen minutes, until 1,290 seconds. Its
        // count is the oldest there is, and the one heard from longest ago.
        for ms in [0, 30_000, 90_000, 390_000] {
            assert_eq!(counts.charge(&persistent, at(NOON + ms)), Decision::Allow);
        }
        // Two more guess once each, later: their waits end at 1,030 and at
        // 1,040 seconds.
        assert_eq!(counts.charge(&early, at(NOON + 1_000_000)), Decision::Allow);
        assert_eq!(counts.charge(&late, at(NOON + 1_010_000)), Decision::Allow);
        // At 1,050 seconds both of theirs are over, and a new key takes the
        // place of the one that ended first; the next new key, the other.
        assert_eq!(
            counts.charge(&newcomer, at(NOON + 1_050_000)),
            Decision::Allow
        );
        assert_eq!(
            counts.charge(&follower, at(NOON + 1_050_000)),
            Decision::Allow
        );
        // Now every wait is running, the shortest until 1,080 seconds.
        assert_eq!(
            counts.charge(&straggler, at(NOON + 1_050_000)),
            wait(30_000)
        );
        assert_eq!(
            held(&counts),
            HashMap::from([
                (persistent.clone(), counted(4, NOON + 390_000)),
                (newcomer, counted(1, NOON + 1_050_000)),
                (follower, counted(1, NOON + 1_050_000)),
            ])
        );
        // The persistent source kept its count and its wait.
        assert_eq!(counts.charge(&persistent, at(NOON + 1_289_999)), wait(1));
    }

    #[test]
    fn counts_told_to_keep_nothing_keep_one() {
        let counts = GuessCounts::new(0);
        let [first, second] = ["192.0.2.1", "192.0.2.2"].map(claim_from);
        assert_eq!(counts.charge(&first, at(NOON)), Decision::Allow);
        assert_eq!(counts.charge(&first, at(NOON)), wait(30_000));
        assert_eq!(counts.charge(&second, at(NOON)), wait(30_000));
        assert_eq!(held(&counts), HashMap::from([(first, counted(1, NOON))]));
    }

    #[test]
    fn the_server_wide_ceiling_lets_too_few_guesses_through_to_fill_the_counts() {
        // A count's wait runs fifteen minutes at the longest, and only an
        // attempt the server-wide ceiling let through makes or renews a
        // count. In fifteen minutes that ceiling lets through its burst and
        // then one attempt per interval: 100 and 1,500.
        let server = Ceilings::DEFAULT.server;
        let through = u64::from(server.burst) + 900_000 / server.interval_ms;
        assert_eq!(through, 1_600);
        // So the store is never full of running waits, and no source is
        // refused for want of room.
        let places = u64::try_from(GUESS_DELAY_KEYS).expect("a small number");
        assert!(through < places, "{through} attempts, room for {places}");
    }
}
