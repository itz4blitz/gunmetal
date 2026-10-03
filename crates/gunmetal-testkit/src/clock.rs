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
        if let Err(from) = self
            .now_ms
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |now| {
                now.checked_add(ms)
            })
        {
            panic!("advancing the manual clock by {ms} ms from {from} ms overflows");
        }
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

    #[test]
    fn shows_every_holder_a_move_made_on_another_thread() {
        let clock = Arc::new(ManualClock::at(0));
        let mover = Arc::clone(&clock);
        thread::spawn(move || mover.advance(42)).join().unwrap();
        assert_eq!(clock.now_ms(), 42);
    }
}
