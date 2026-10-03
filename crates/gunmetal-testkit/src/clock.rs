//! A clock that moves only when a test moves it.

use std::sync::atomic::{AtomicI64, Ordering};

/// A time source for tests, in milliseconds since the Unix epoch, the unit
/// the core's timestamps count in.
///
/// The testkit depends on nothing in the workspace, so this does not
/// implement the core's `Clock` trait. A test crate wraps it in a two-line
/// adapter instead:
///
/// ```
/// use gunmetal_testkit::clock::ManualClock;
///
/// // Stands in for the core's `Clock` trait in this example.
/// trait Clock {
///     fn now(&self) -> i64;
/// }
///
/// struct TestClock(ManualClock);
/// impl Clock for TestClock {
///     fn now(&self) -> i64 {
///         self.0.now_ms()
///     }
/// }
///
/// let clock = TestClock(ManualClock::at(1_000));
/// clock.0.advance(500);
/// assert_eq!(clock.now(), 1_500);
/// ```
///
/// It can be shared between threads, so a server under test and the test
/// driving it can hold the same clock.
#[derive(Debug)]
pub struct ManualClock {
    now_ms: AtomicI64,
}

impl ManualClock {
    /// A clock reading `ms`.
    #[must_use]
    pub fn at(ms: i64) -> Self {
        Self {
            now_ms: AtomicI64::new(ms),
        }
    }

    /// The time the clock reads.
    #[must_use]
    pub fn now_ms(&self) -> i64 {
        self.now_ms.load(Ordering::SeqCst)
    }

    /// Moves the clock by `ms`, backwards when `ms` is negative, as a wall
    /// clock can step.
    ///
    /// # Panics
    ///
    /// Panics when the move would take the reading outside `i64`.
    pub fn advance(&self, ms: i64) {
        if let Err(from) = self.update(|now| now.checked_add(ms)) {
            panic!("advancing the manual clock by {ms} ms from {from} ms overflows");
        }
    }

    /// Replaces the reading with what `next` makes of it, starting again
    /// from the new reading when another holder moved the clock in between,
    /// so no move is lost. When `next` has no answer the clock is left alone
    /// and the reading it was asked about comes back as the error.
    ///
    /// `fetch_update` is the same loop, and Rust 1.99 renames it to
    /// `try_update`. This crate's rust-version is 1.85, so the loop is
    /// written out with `compare_exchange`, which every version has.
    fn update(&self, mut next: impl FnMut(i64) -> Option<i64>) -> Result<(), i64> {
        let mut now = self.now_ms.load(Ordering::SeqCst);
        while let Some(moved) = next(now) {
            match self
                .now_ms
                .compare_exchange(now, moved, Ordering::SeqCst, Ordering::SeqCst)
            {
                Ok(_) => return Ok(()),
                Err(current) => now = current,
            }
        }
        Err(now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn reads_the_time_it_was_set_to() {
        assert_eq!(
            ManualClock::at(1_759_449_600_000).now_ms(),
            1_759_449_600_000
        );
        assert_eq!(ManualClock::at(-86_400_000).now_ms(), -86_400_000);
        assert_eq!(ManualClock::at(i64::MAX).now_ms(), i64::MAX);
        assert_eq!(ManualClock::at(i64::MIN).now_ms(), i64::MIN);
    }

    #[test]
    fn stays_put_until_advanced() {
        let clock = ManualClock::at(1_000);
        assert_eq!(clock.now_ms(), 1_000);
        assert_eq!(clock.now_ms(), 1_000);
    }

    #[test]
    fn moves_forward_and_back_by_the_milliseconds_given() {
        let clock = ManualClock::at(1_000);
        clock.advance(250);
        assert_eq!(clock.now_ms(), 1_250);
        clock.advance(0);
        assert_eq!(clock.now_ms(), 1_250);
        // A wall clock can step backwards, and tests must be able to say so.
        clock.advance(-2_000);
        assert_eq!(clock.now_ms(), -750);
    }

    #[test]
    fn reaches_the_extremes_exactly() {
        let clock = ManualClock::at(i64::MAX - 5);
        clock.advance(5);
        assert_eq!(clock.now_ms(), i64::MAX);
        let clock = ManualClock::at(i64::MIN + 5);
        clock.advance(-5);
        assert_eq!(clock.now_ms(), i64::MIN);
    }

    #[test]
    #[should_panic(
        expected = "advancing the manual clock by 2 ms from 9223372036854775806 ms overflows"
    )]
    fn refuses_to_advance_past_the_latest_time() {
        ManualClock::at(i64::MAX - 1).advance(2);
    }

    #[test]
    #[should_panic(
        expected = "advancing the manual clock by -1 ms from -9223372036854775808 ms overflows"
    )]
    fn refuses_to_step_back_past_the_earliest_time() {
        ManualClock::at(i64::MIN).advance(-1);
    }

    /// Moves the clock by `ms` while another holder sets it to each of
    /// `interruptions` in turn, each one landing after the move has read the
    /// clock and before it writes. Returns the readings the move started
    /// from and how it ended.
    fn advance_interrupted(
        clock: &ManualClock,
        ms: i64,
        interruptions: &[i64],
    ) -> (Vec<i64>, Result<(), i64>) {
        let mut pending = interruptions.iter();
        let mut seen = Vec::new();
        let outcome = clock.update(|now| {
            seen.push(now);
            if let Some(&other) = pending.next() {
                clock.now_ms.store(other, Ordering::SeqCst);
            }
            now.checked_add(ms)
        });
        (seen, outcome)
    }

    #[test]
    fn keeps_a_move_another_holder_made_while_this_one_was_moving() {
        let clock = ManualClock::at(1_000);
        assert_eq!(
            advance_interrupted(&clock, 10, &[5_000, 70_000]),
            (vec![1_000, 5_000, 70_000], Ok(()))
        );
        assert_eq!(clock.now_ms(), 70_010);
    }

    #[test]
    fn reports_an_overflow_from_the_reading_another_holder_left() {
        let clock = ManualClock::at(1_000);
        assert_eq!(
            advance_interrupted(&clock, 10, &[i64::MAX - 3]),
            (vec![1_000, i64::MAX - 3], Err(i64::MAX - 3))
        );
        assert_eq!(clock.now_ms(), i64::MAX - 3);
    }

    #[test]
    fn shows_every_holder_a_move_made_on_another_thread() {
        let clock = Arc::new(ManualClock::at(0));
        let mover = Arc::clone(&clock);
        thread::spawn(move || mover.advance(42)).join().unwrap();
        assert_eq!(clock.now_ms(), 42);
    }
}
