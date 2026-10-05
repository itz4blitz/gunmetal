//! How long a request may take and how much it may read.
//!
//! Every request has a connect timeout, a deadline for the whole exchange
//! and a cap on the response body (SEC-EXT-004, SEC-API-078). The defaults
//! are the security baseline's: 5 seconds to connect, 30 seconds in all and
//! 8 MiB of body. The arithmetic lives here, with the time that has passed
//! and the octets read as inputs, so that it is tested without a clock or a
//! socket; the client applies its answers to every step it takes.

use core::time::Duration;

/// The time and size limits of one request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The longest one connection attempt may take.
    pub connect: Duration,
    /// The longest the whole request may take, from before its name is
    /// resolved until the last octet of the body is read.
    pub total: Duration,
    /// The most octets of response body that are read.
    pub body: u64,
}

/// A response body is larger than its request allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TooLarge {
    /// The most octets the body may have.
    pub limit: u64,
}

impl Limits {
    /// The baseline's defaults: 5 seconds to connect, 30 seconds in all and
    /// 8 MiB of body.
    pub const DEFAULT: Self = Self {
        connect: Duration::from_secs(5),
        total: Duration::from_secs(30),
        body: 8_388_608,
    };

    /// How long the request may still run when `elapsed` has passed since
    /// it started, or `None` once its deadline has passed.
    #[must_use]
    pub fn remaining(&self, _elapsed: Duration) -> Option<Duration> {
        Some(self.total)
    }

    /// How long a connection attempt may take when it starts `elapsed`
    /// after the request did: the connect timeout, or what is left of the
    /// whole request if that is shorter. `None` once the deadline has
    /// passed.
    #[must_use]
    pub fn connect_budget(&self, _elapsed: Duration) -> Option<Duration> {
        Some(self.connect)
    }

    /// The octets of body read once `more` arrive after `received`.
    ///
    /// # Errors
    ///
    /// [`TooLarge`] when that is more than the body may have. The caller
    /// stops reading and keeps nothing of the body.
    pub fn body_after(&self, _received: u64, _more: u64) -> Result<u64, TooLarge> {
        Err(TooLarge { limit: self.body })
    }
}

#[cfg(test)]
mod tests {
    use super::{Limits, TooLarge};
    use core::time::Duration;

    /// The baseline's times, with a body small enough to count by hand.
    fn limits() -> Limits {
        Limits {
            connect: Duration::from_secs(5),
            total: Duration::from_secs(30),
            body: 1_000,
        }
    }

    #[test]
    fn the_defaults_are_five_seconds_thirty_seconds_and_eight_mebibytes() {
        assert_eq!(
            Limits::DEFAULT,
            Limits {
                connect: Duration::from_secs(5),
                total: Duration::from_secs(30),
                body: 8 * 1024 * 1024,
            }
        );
    }

    #[test]
    fn a_request_may_run_until_its_deadline_and_no_longer() {
        let cases = [
            (Duration::ZERO, Some(Duration::from_secs(30))),
            (Duration::from_secs(12), Some(Duration::from_secs(18))),
            (
                Duration::from_millis(29_999),
                Some(Duration::from_millis(1)),
            ),
            (Duration::from_secs(30), None),
            (Duration::from_secs(31), None),
            (Duration::MAX, None),
        ];
        for (elapsed, left) in cases {
            assert_eq!(limits().remaining(elapsed), left, "{elapsed:?}");
        }
    }

    #[test]
    fn a_connection_attempt_gets_the_connect_timeout_or_what_is_left_if_that_is_shorter() {
        let cases = [
            (Duration::ZERO, Some(Duration::from_secs(5))),
            (Duration::from_secs(25), Some(Duration::from_secs(5))),
            (Duration::from_secs(26), Some(Duration::from_secs(4))),
            (
                Duration::from_millis(29_999),
                Some(Duration::from_millis(1)),
            ),
            (Duration::from_secs(30), None),
            (Duration::from_secs(45), None),
        ];
        for (elapsed, budget) in cases {
            assert_eq!(limits().connect_budget(elapsed), budget, "{elapsed:?}");
        }
    }

    #[test]
    fn a_body_is_read_up_to_its_limit_and_refused_past_it() {
        let too_large = Err(TooLarge { limit: 1_000 });
        let cases = [
            (0, 0, Ok(0)),
            (0, 400, Ok(400)),
            (400, 600, Ok(1_000)),
            (400, 601, too_large),
            (1_000, 1, too_large),
            (0, 1_001, too_large),
            (1, u64::MAX, too_large),
            (u64::MAX, 1, too_large),
        ];
        for (received, more, read) in cases {
            assert_eq!(
                limits().body_after(received, more),
                read,
                "{received} and {more}"
            );
        }
    }
}
