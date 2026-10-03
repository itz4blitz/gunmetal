//! The system clock: the one place the server reads the time of day.
//!
//! Everything else takes a [`Clock`], so tests drive time by hand.

use std::time::{SystemTime, UNIX_EPOCH};

use gunmetal_core::time::{Clock, Timestamp};

/// The host's wall clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        timestamp(SystemTime::now())
    }
}

/// Whole milliseconds, or `None` when the count does not fit in an `i64`.
fn i64_millis(ms: u128) -> Option<i64> {
    i64::try_from(ms).ok()
}

/// `time` as a timestamp, in whole milliseconds towards the epoch. A time
/// outside the range a timestamp holds becomes the nearest end of it.
fn timestamp(time: SystemTime) -> Timestamp {
    let (ms, limit) = match time.duration_since(UNIX_EPOCH) {
        Ok(after) => (i64_millis(after.as_millis()), Timestamp::MAX),
        Err(before) => (
            i64_millis(before.duration().as_millis()).map(i64::wrapping_neg),
            Timestamp::MIN,
        ),
    };
    ms.and_then(|ms| Timestamp::from_millis(ms).ok())
        .unwrap_or(limit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn at(ms: i64) -> Timestamp {
        Timestamp::from_millis(ms).expect("in range")
    }

    fn after(duration: Duration) -> SystemTime {
        UNIX_EPOCH.checked_add(duration).expect("representable")
    }

    fn before(duration: Duration) -> SystemTime {
        UNIX_EPOCH.checked_sub(duration).expect("representable")
    }

    #[test]
    fn counts_milliseconds_on_both_sides_of_the_epoch() {
        assert_eq!(timestamp(UNIX_EPOCH), at(0));
        assert_eq!(
            timestamp(after(Duration::from_micros(1_500_999))),
            at(1_500)
        );
        assert_eq!(
            timestamp(before(Duration::from_micros(1_500_999))),
            at(-1_500)
        );
        assert_eq!(
            timestamp(after(Duration::from_secs(1_791_028_800))),
            at(1_791_028_800_000)
        );
    }

    #[test]
    fn a_time_outside_the_timestamp_range_becomes_its_nearest_end() {
        // The year 10000 and the year -1, each one second past the range.
        assert_eq!(
            timestamp(after(Duration::from_secs(253_402_300_800))),
            Timestamp::MAX
        );
        assert_eq!(
            timestamp(before(Duration::from_secs(62_167_219_201))),
            Timestamp::MIN
        );
        assert_eq!(
            timestamp(after(Duration::from_millis(253_402_300_799_999))),
            Timestamp::MAX
        );
        assert_eq!(
            timestamp(before(Duration::from_secs(62_167_219_200))),
            Timestamp::MIN
        );
        assert_eq!(i64_millis(0), Some(0));
        assert_eq!(i64_millis(u128::from(u64::MAX)), None);
    }

    #[test]
    fn reads_the_hosts_clock() {
        let reading = || {
            let since = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("after 1970");
            i64::try_from(since.as_millis()).expect("before the year 10000")
        };
        let earliest = reading();
        let now = SystemClock.now().millis();
        let latest = reading();
        // 2026-01-01T00:00:00Z: a clock that reads earlier is not set.
        assert!(earliest > 1_767_225_600_000);
        assert!(earliest <= now);
        assert!(now <= latest);
    }
}
