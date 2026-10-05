//! The guess delay: when a source that guessed a short secret wrong may
//! guess again (SEC-API-056).

use gunmetal_core::client_context::ClientContext;
use gunmetal_core::ratelimit::{Decision, LimitKey};
use gunmetal_core::time::Timestamp;

/// How many rows the guess log keeps, as `limits.toml` registers it.
pub const GUESS_DELAY_ROWS: u32 = 0;

/// The wrong guesses one source has made at one target of one pathway
/// since its last right one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Failures {
    /// How many.
    pub count: u32,
    /// When the latest was made.
    pub last_at: Timestamp,
}

/// Whether a source with `failures` on record may guess again at `now`, and
/// how long it must wait if not.
#[must_use]
pub fn guess_allowed(_failures: Option<Failures>, _now: Timestamp) -> Decision {
    Decision::Allow
}

/// The name the guess log keeps `source` under.
#[must_use]
pub fn source_label(_source: &ClientContext) -> String {
    String::new()
}

/// A limiter key as text.
#[must_use]
pub fn label(_key: LimitKey) -> String {
    String::new()
}

#[cfg(test)]
mod tests {
    use core::net::{Ipv4Addr, Ipv6Addr};

    use gunmetal_core::ratelimit::{Decision, LimitKey, PrincipalKey};
    use gunmetal_core::time::Timestamp;
    use proptest::prelude::*;

    use super::*;
    use crate::limiter::testing::{gateway, source};
    use crate::testing::NOON;

    fn at(ms: i64) -> Timestamp {
        Timestamp::from_millis(ms).expect("in range")
    }

    fn failed(count: u32, ms: i64) -> Option<Failures> {
        Some(Failures {
            count,
            last_at: at(ms),
        })
    }

    #[test]
    fn a_source_with_no_wrong_guess_does_not_wait() {
        assert_eq!(guess_allowed(None, at(NOON)), Decision::Allow);
    }

    /// Verifies: SEC-API-056
    #[test]
    fn each_wrong_guess_waits_out_the_next_step_of_the_schedule() {
        // How many wrong guesses, and the wait after the last, written out
        // from the requirement: 30 seconds, 1 minute, 5 minutes, then
        // 15 minutes however many more there are.
        let schedule: [(u32, i64); 8] = [
            (1, 30_000),
            (2, 60_000),
            (3, 300_000),
            (4, 900_000),
            (5, 900_000),
            (10, 900_000),
            (1_000, 900_000),
            (u32::MAX, 900_000),
        ];
        for (count, wait) in schedule {
            let failures = failed(count, NOON);
            assert_eq!(
                guess_allowed(failures, at(NOON)),
                Decision::Deny {
                    retry_after_ms: wait.unsigned_abs()
                },
                "{count}"
            );
            assert_eq!(
                guess_allowed(failures, at(NOON + wait - 1)),
                Decision::Deny { retry_after_ms: 1 },
                "{count}"
            );
            assert_eq!(
                guess_allowed(failures, at(NOON + wait)),
                Decision::Allow,
                "{count}"
            );
            assert_eq!(
                guess_allowed(failures, at(NOON + wait + 1)),
                Decision::Allow,
                "{count}"
            );
        }
    }

    proptest! {
        /// Verifies: SEC-API-056
        #[test]
        fn no_number_of_wrong_guesses_waits_longer_than_fifteen_minutes(
            count in any::<u32>(),
            since in 0_i64..2_000_000,
        ) {
            let failures = failed(count, NOON);
            let waited = match guess_allowed(failures, at(NOON + since)) {
                Decision::Allow => 0,
                Decision::Deny { retry_after_ms } => retry_after_ms,
            };
            prop_assert!(waited <= 900_000);
            // Never for good: once the wait it was told is over, it may guess.
            let later = NOON + since + i64::try_from(waited).expect("a short wait");
            prop_assert_eq!(guess_allowed(failures, at(later)), Decision::Allow);
        }
    }

    #[test]
    fn names_each_kind_of_key() {
        let prefix = Ipv6Addr::new(0x2001, 0xdb8, 0, 0x1200, 0, 0, 0, 0);
        let mut principal = [0_u8; 16];
        principal[0] = 0x0a;
        principal[15] = 0xff;
        let labels = [
            label(LimitKey::Principal(PrincipalKey::from_bytes(principal))),
            label(LimitKey::Ipv4(Ipv4Addr::new(203, 0, 113, 7))),
            label(LimitKey::Ipv6Slash64(prefix)),
            label(LimitKey::Ipv6Slash56(prefix)),
            label(LimitKey::Ipv6Slash48(prefix)),
            label(LimitKey::Global),
            label(LimitKey::Unknown),
        ];
        assert_eq!(
            labels,
            [
                "principal:0a0000000000000000000000000000ff",
                "203.0.113.7",
                "2001:db8:0:1200::/64",
                "2001:db8:0:1200::/56",
                "2001:db8:0:1200::/48",
                "server",
                "unknown",
            ]
        );
    }

    #[test]
    fn keeps_a_source_under_its_narrowest_key() {
        let labels = [
            source_label(&source("203.0.113.7")),
            source_label(&source("127.0.0.1")),
            // Every address of one /64 is one source.
            source_label(&source("2001:db8:0:1234:aaaa:bbbb:cccc:dddd")),
            source_label(&source("2001:db8:0:1234::1")),
            // An IPv4 address written as IPv6 is that IPv4 address.
            source_label(&source("::ffff:203.0.113.7")),
            // Whoever is behind the server's gateway shares one name.
            source_label(&gateway("172.17.0.1")),
            source_label(&gateway("10.0.0.1")),
        ];
        assert_eq!(
            labels,
            [
                "203.0.113.7",
                "127.0.0.1",
                "2001:db8:0:1234::/64",
                "2001:db8:0:1234::/64",
                "203.0.113.7",
                "unknown",
                "unknown",
            ]
        );
    }
}
