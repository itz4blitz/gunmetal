//! The ceilings every sign-in attempt passes first: per source and
//! server-wide (SEC-IAM-101).
//!
//! Every endpoint that checks a secret or starts a sign-in ceremony spends
//! one attempt here before it does anything else. The counting is the
//! core's keyed GCRA (WP-032), not a counter of this module's own: an
//! attempt is charged to the keys the core derives from the request's
//! `ClientContext`, which are the IPv4 address, or the IPv6 /64, /56 and
//! /48, or the one bucket of the unknown class, and then to the core's
//! global key, which is the server-wide count. It is allowed only when
//! every key allows it, and a refused attempt spends nothing.
//!
//! The ceilings in force are [`Ceilings::DEFAULT`], the ones `limits.toml`
//! registers. A set of ceilings is refused unless each per-source ceiling
//! is below the server-wide one, in its burst and in its rate, so that no
//! one source, network or gateway can use the server-wide ceiling up and
//! keep everyone else out (SEC-IAM-008, SEC-NET-052).
//!
//! The limiter remembers a fixed number of keys and forgets the one it
//! heard from longest ago when it is full, so that callers who have not
//! signed in cannot make it grow (SEC-NET-051).

use std::sync::{Mutex, PoisonError};

use gunmetal_core::client_context::ClientContext;
use gunmetal_core::ratelimit::{
    BoundedStore, Decision, LimitKey, Rate, RateError, Tat, check_store, keys_for,
};
use gunmetal_core::time::Timestamp;

/// How many keys the limiter remembers, as `limits.toml` registers it.
pub const TRACKED_KEYS: usize = 16_384;

/// One ceiling: `burst` attempts at once, then one more every
/// `interval_ms` milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ceiling {
    /// Milliseconds between attempts once the burst is spent.
    pub interval_ms: u64,
    /// Attempts allowed at once.
    pub burst: u32,
}

/// The ceilings on sign-in attempts, one for each kind of key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ceilings {
    /// For one IPv4 address, or one IPv6 /64.
    pub source: Ceiling,
    /// For one IPv6 /56.
    pub ipv6_56: Ceiling,
    /// For one IPv6 /48.
    pub ipv6_48: Ceiling,
    /// For every peer of the unknown class together: whoever is behind the
    /// server's own gateway.
    pub unknown: Ceiling,
    /// For the whole server.
    pub server: Ceiling,
}

impl Ceilings {
    /// The ceilings `limits.toml` registers.
    pub const DEFAULT: Self = Self {
        source: Ceiling {
            interval_ms: 6_000,
            burst: 10,
        },
        ipv6_56: Ceiling {
            interval_ms: 3_000,
            burst: 20,
        },
        ipv6_48: Ceiling {
            interval_ms: 1_500,
            burst: 40,
        },
        unknown: Ceiling {
            interval_ms: 3_000,
            burst: 20,
        },
        server: Ceiling {
            interval_ms: 600,
            burst: 100,
        },
    };
}

/// Why a set of ceilings was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CeilingError {
    /// The core cannot count with a ceiling: its interval or its burst is
    /// zero, or too large.
    Unusable {
        /// Which ceiling.
        key: &'static str,
        /// What the core said.
        error: RateError,
    },
    /// A per-source ceiling is not below the server-wide one, so one source
    /// could use the server-wide ceiling up.
    NotBelowServer {
        /// Which ceiling.
        key: &'static str,
    },
}

/// The ceilings as the core counts with them.
struct Rates {
    source: Rate,
    ipv6_56: Rate,
    ipv6_48: Rate,
    unknown: Rate,
    server: Rate,
}

/// `ceiling` as a rate the core counts with.
fn rate(key: &'static str, ceiling: Ceiling) -> Result<Rate, CeilingError> {
    Rate::new(ceiling.interval_ms, ceiling.burst)
        .map_err(|error| CeilingError::Unusable { key, error })
}

/// `ceiling` as a rate, when it is below the server-wide ceiling `server`
/// both in how many attempts it allows at once and in how fast it allows
/// more.
fn below(key: &'static str, ceiling: Ceiling, server: Ceiling) -> Result<Rate, CeilingError> {
    if ceiling.burst < server.burst && ceiling.interval_ms > server.interval_ms {
        rate(key, ceiling)
    } else {
        Err(CeilingError::NotBelowServer { key })
    }
}

impl Rates {
    fn new(ceilings: &Ceilings) -> Result<Self, CeilingError> {
        let server = rate("server", ceilings.server)?;
        Ok(Self {
            source: below("source", ceilings.source, ceilings.server)?,
            ipv6_56: below("ipv6_56", ceilings.ipv6_56, ceilings.server)?,
            ipv6_48: below("ipv6_48", ceilings.ipv6_48, ceilings.server)?,
            unknown: below("unknown", ceilings.unknown, ceilings.server)?,
            server,
        })
    }

    /// The ceiling `key` is counted against. A sign-in attempt has no
    /// principal yet; a principal's key would be held to a source's.
    fn of(&self, key: LimitKey) -> Rate {
        match key {
            LimitKey::Principal(_) | LimitKey::Ipv4(_) | LimitKey::Ipv6Slash64(_) => self.source,
            LimitKey::Ipv6Slash56(_) => self.ipv6_56,
            LimitKey::Ipv6Slash48(_) => self.ipv6_48,
            LimitKey::Unknown => self.unknown,
            LimitKey::Global => self.server,
        }
    }
}

/// The limiter every sign-in attempt passes first.
pub struct SourceLimiter {
    rates: Rates,
    store: Mutex<BoundedStore<LimitKey, Tat>>,
}

impl SourceLimiter {
    /// A limiter with `ceilings` that remembers at most `capacity` keys. A
    /// capacity too small to hold the keys of one attempt refuses every
    /// attempt.
    ///
    /// # Errors
    ///
    /// A [`CeilingError`] when the core cannot count with a ceiling, or a
    /// per-source ceiling is not below the server-wide one.
    pub fn new(ceilings: &Ceilings, capacity: usize) -> Result<Self, CeilingError> {
        Rates::new(ceilings).map(|rates| Self {
            rates,
            store: Mutex::new(BoundedStore::new(capacity)),
        })
    }

    /// Spends one attempt from `source` at `now`, when every ceiling it
    /// counts against has one left, and otherwise says how long to wait.
    #[must_use]
    pub fn admit(&self, source: &ClientContext, now: Timestamp) -> Decision {
        let keys = keys_for(None, source);
        // The store is whole whenever the lock is free: the core changes it
        // only once it has decided, so a holder that panicked left it sound.
        let mut store = self.store.lock().unwrap_or_else(PoisonError::into_inner);
        check_store(&mut store, &keys, now, |key| self.rates.of(key))
    }
}

#[cfg(test)]
mod tests {
    use gunmetal_core::ratelimit::{Decision, RateError};
    use gunmetal_core::time::Timestamp;

    use super::*;
    use crate::limiter::delay::GUESS_DELAY_ROWS;
    use crate::limiter::testing::{gateway, source};
    use crate::testing::NOON;

    fn at(ms: i64) -> Timestamp {
        Timestamp::from_millis(ms).expect("in range")
    }

    /// The limiter the server runs with.
    fn limiter() -> SourceLimiter {
        SourceLimiter::new(&Ceilings::DEFAULT, TRACKED_KEYS).expect("the registered ceilings")
    }

    /// One field of a register entry, as it is written.
    fn field(entry: &toml::de::DeTable<'_>, key: &str) -> String {
        let value = entry.get(key).expect("the field").get_ref();
        value
            .as_str()
            .map(str::to_owned)
            .or_else(|| value.as_integer().map(|number| number.as_str().to_owned()))
            .expect("text or a whole number")
    }

    /// The sign-in entries of the limits register: each one's name, scope,
    /// default, unit, requirement and enforcing test.
    fn registered() -> Vec<[String; 6]> {
        let text = include_str!("../../limits.toml");
        let register = toml::de::DeTable::parse(text).expect("the register parses");
        register
            .get_ref()
            .get("limit")
            .and_then(|value| value.get_ref().as_array())
            .expect("the limits")
            .iter()
            .map(|item| item.get_ref().as_table().expect("a table"))
            .filter(|entry| field(entry, "name").starts_with("sign_in."))
            .map(|entry| {
                ["name", "scope", "default", "unit", "requirement", "test"]
                    .map(|key| field(entry, key))
            })
            .collect()
    }

    const RATES: &str = "crates/gunmetal-server/src/limiter/rates.rs";
    const GUESSES: &str = "crates/gunmetal-server/src/verifier/guesses.rs";
    const PREFIXES: &str = "an_ipv6_network_is_held_at_each_of_its_prefixes";
    const SERVER: &str = "sources_together_reach_the_server_wide_ceiling";
    const SOURCE: &str = "a_source_gets_its_burst_then_one_attempt_per_interval";
    const UNKNOWN: &str =
        "peers_behind_the_gateway_share_one_bucket_that_cannot_delay_a_direct_peer";
    const KEYS: &str = "forgets_the_source_it_heard_from_longest_ago_when_it_is_full";
    const ROWS: &str = "keeps_only_the_newest_rows_it_has_room_for";

    /// The sign-in entries the register must hold, written out here on
    /// their own: name, scope, default, unit, requirement, and the file and
    /// the name of the test that enforces the limit.
    const REGISTER: [[&str; 7]; 12] = [
        [
            "sign_in.guess_delays.rows",
            "server",
            "4096",
            "rows",
            "SEC-NET-051",
            GUESSES,
            ROWS,
        ],
        [
            "sign_in.ipv6_48.burst",
            "source",
            "40",
            "attempts",
            "SEC-NET-052",
            RATES,
            PREFIXES,
        ],
        [
            "sign_in.ipv6_48.interval_ms",
            "source",
            "1500",
            "milliseconds",
            "SEC-NET-052",
            RATES,
            PREFIXES,
        ],
        [
            "sign_in.ipv6_56.burst",
            "source",
            "20",
            "attempts",
            "SEC-NET-052",
            RATES,
            PREFIXES,
        ],
        [
            "sign_in.ipv6_56.interval_ms",
            "source",
            "3000",
            "milliseconds",
            "SEC-NET-052",
            RATES,
            PREFIXES,
        ],
        [
            "sign_in.server.burst",
            "server",
            "100",
            "attempts",
            "SEC-IAM-101",
            RATES,
            SERVER,
        ],
        [
            "sign_in.server.interval_ms",
            "server",
            "600",
            "milliseconds",
            "SEC-IAM-101",
            RATES,
            SERVER,
        ],
        [
            "sign_in.source.burst",
            "source",
            "10",
            "attempts",
            "SEC-IAM-101",
            RATES,
            SOURCE,
        ],
        [
            "sign_in.source.interval_ms",
            "source",
            "6000",
            "milliseconds",
            "SEC-IAM-101",
            RATES,
            SOURCE,
        ],
        [
            "sign_in.tracked_keys",
            "server",
            "16384",
            "keys",
            "SEC-NET-051",
            RATES,
            KEYS,
        ],
        [
            "sign_in.unknown.burst",
            "source",
            "20",
            "attempts",
            "SEC-NET-052",
            RATES,
            UNKNOWN,
        ],
        [
            "sign_in.unknown.interval_ms",
            "source",
            "3000",
            "milliseconds",
            "SEC-NET-052",
            RATES,
            UNKNOWN,
        ],
    ];

    /// Verifies: SEC-IAM-101
    #[test]
    fn the_limits_in_force_are_the_ones_the_register_documents() {
        assert_eq!(
            Ceilings::DEFAULT,
            Ceilings {
                source: Ceiling {
                    interval_ms: 6_000,
                    burst: 10
                },
                ipv6_56: Ceiling {
                    interval_ms: 3_000,
                    burst: 20
                },
                ipv6_48: Ceiling {
                    interval_ms: 1_500,
                    burst: 40
                },
                unknown: Ceiling {
                    interval_ms: 3_000,
                    burst: 20
                },
                server: Ceiling {
                    interval_ms: 600,
                    burst: 100
                },
            }
        );
        assert_eq!(TRACKED_KEYS, 16_384);
        assert_eq!(GUESS_DELAY_ROWS, 4_096);
        let documented = REGISTER.map(|[name, scope, default, unit, id, file, test]| {
            [
                name.to_owned(),
                scope.to_owned(),
                default.to_owned(),
                unit.to_owned(),
                id.to_owned(),
                format!("{file}: {test}"),
            ]
        });
        assert_eq!(registered(), documented);
    }

    #[test]
    fn a_source_gets_its_burst_then_one_attempt_per_interval() {
        let limiter = limiter();
        let peer = source("203.0.113.7");
        let burst: Vec<Decision> = (0..10).map(|_| limiter.admit(&peer, at(NOON))).collect();
        assert_eq!(burst, [Decision::Allow; 10]);
        assert_eq!(
            limiter.admit(&peer, at(NOON)),
            Decision::Deny {
                retry_after_ms: 6_000
            }
        );
        assert_eq!(
            limiter.admit(&peer, at(NOON + 5_999)),
            Decision::Deny { retry_after_ms: 1 }
        );
        assert_eq!(limiter.admit(&peer, at(NOON + 6_000)), Decision::Allow);
        assert_eq!(
            limiter.admit(&peer, at(NOON + 6_000)),
            Decision::Deny {
                retry_after_ms: 6_000
            }
        );
        // Another source has a burst of its own.
        assert_eq!(
            limiter.admit(&source("203.0.113.8"), at(NOON)),
            Decision::Allow
        );
    }

    /// Verifies: SEC-IAM-101
    #[test]
    fn sources_together_reach_the_server_wide_ceiling() {
        let limiter = limiter();
        let each: Vec<Decision> = (0..100)
            .map(|n| limiter.admit(&source(&format!("198.51.100.{n}")), at(NOON)))
            .collect();
        assert_eq!(each, [Decision::Allow; 100]);
        // A source that has spent nothing of its own is refused all the same.
        let late = source("198.51.100.200");
        assert_eq!(
            limiter.admit(&late, at(NOON)),
            Decision::Deny {
                retry_after_ms: 600
            }
        );
        assert_eq!(
            limiter.admit(&late, at(NOON + 599)),
            Decision::Deny { retry_after_ms: 1 }
        );
        assert_eq!(limiter.admit(&late, at(NOON + 600)), Decision::Allow);
    }

    /// Verifies: SEC-IAM-008
    #[test]
    fn one_source_cannot_use_up_the_server_wide_ceiling() {
        let limiter = limiter();
        let hostile = source("203.0.113.66");
        let allowed = (0..1_000)
            .filter(|_| limiter.admit(&hostile, at(NOON)) == Decision::Allow)
            .count();
        assert_eq!(allowed, 10);
        // The other ninety attempts of the server-wide burst are still there
        // for everyone else.
        let others: Vec<Decision> = (0..90)
            .map(|n| limiter.admit(&source(&format!("198.51.100.{n}")), at(NOON)))
            .collect();
        assert_eq!(others, [Decision::Allow; 90]);
        assert_eq!(
            limiter.admit(&source("198.51.100.200"), at(NOON)),
            Decision::Deny {
                retry_after_ms: 600
            }
        );
    }

    #[test]
    fn an_ipv6_network_is_held_at_each_of_its_prefixes() {
        let limiter = limiter();
        // Twenty /64s of one /56, one attempt each, are that /56's burst.
        let in_56: Vec<Decision> = (0..20)
            .map(|n| limiter.admit(&source(&format!("2001:db8:0:{n:x}::1")), at(NOON)))
            .collect();
        assert_eq!(in_56, [Decision::Allow; 20]);
        assert_eq!(
            limiter.admit(&source("2001:db8:0:ff::1"), at(NOON)),
            Decision::Deny {
                retry_after_ms: 3_000
            }
        );
        // The next /56 of the same /48 has a burst of its own, which takes
        // the /48 to its forty.
        let in_48: Vec<Decision> = (0..20)
            .map(|n| limiter.admit(&source(&format!("2001:db8:0:1{n:02x}::1")), at(NOON)))
            .collect();
        assert_eq!(in_48, [Decision::Allow; 20]);
        assert_eq!(
            limiter.admit(&source("2001:db8:0:200::1"), at(NOON)),
            Decision::Deny {
                retry_after_ms: 1_500
            }
        );
        // Another /48 has spent nothing.
        assert_eq!(
            limiter.admit(&source("2001:db8:1::1"), at(NOON)),
            Decision::Allow
        );
    }

    #[test]
    fn peers_behind_the_gateway_share_one_bucket_that_cannot_delay_a_direct_peer() {
        let limiter = limiter();
        let nat = gateway("172.17.0.1");
        let allowed = (0..1_000)
            .filter(|_| limiter.admit(&nat, at(NOON)) == Decision::Allow)
            .count();
        assert_eq!(allowed, 20);
        // Another gateway of the server's is the same bucket.
        assert_eq!(
            limiter.admit(&gateway("10.0.0.1"), at(NOON)),
            Decision::Deny {
                retry_after_ms: 3_000
            }
        );
        // The server itself and a peer on the home network are not held up.
        assert_eq!(
            limiter.admit(&source("127.0.0.1"), at(NOON)),
            Decision::Allow
        );
        assert_eq!(
            limiter.admit(&source("192.168.1.20"), at(NOON)),
            Decision::Allow
        );
    }

    /// Whether the first source is still held to its spent burst after two
    /// other sources have been heard from, in a limiter with room for
    /// `capacity` keys.
    fn first_source_after_two_others(capacity: usize) -> Decision {
        let limiter = SourceLimiter::new(&Ceilings::DEFAULT, capacity).expect("usable");
        let first = source("203.0.113.1");
        let spent = (0..11)
            .filter(|_| limiter.admit(&first, at(NOON)) == Decision::Allow)
            .count();
        assert_eq!(spent, 10);
        for other in ["203.0.113.2", "203.0.113.3"] {
            assert_eq!(limiter.admit(&source(other), at(NOON)), Decision::Allow);
        }
        limiter.admit(&first, at(NOON))
    }

    #[test]
    fn forgets_the_source_it_heard_from_longest_ago_when_it_is_full() {
        // Four keys: the server-wide one and all three sources.
        assert_eq!(
            first_source_after_two_others(4),
            Decision::Deny {
                retry_after_ms: 6_000
            }
        );
        // Three keys: the third source pushed the first out, and it starts
        // again with a whole burst.
        assert_eq!(first_source_after_two_others(3), Decision::Allow);
        // No room for the two keys of one attempt: nothing is allowed.
        let cramped = SourceLimiter::new(&Ceilings::DEFAULT, 1).expect("usable");
        assert_eq!(
            cramped.admit(&source("203.0.113.1"), at(NOON)),
            Decision::Deny {
                retry_after_ms: 6_000
            }
        );
    }

    #[test]
    fn refuses_ceilings_that_let_one_source_use_up_the_server_wide_one() {
        // The server-wide ceiling is a burst of 100 and one attempt every
        // 600 ms. Each of these is at or above it in one of the two.
        let loose = [
            Ceiling {
                interval_ms: 6_000,
                burst: 100,
            },
            Ceiling {
                interval_ms: 6_000,
                burst: 101,
            },
            Ceiling {
                interval_ms: 600,
                burst: 10,
            },
            Ceiling {
                interval_ms: 599,
                burst: 10,
            },
        ];
        for ceiling in loose {
            let cases = [
                (
                    "source",
                    Ceilings {
                        source: ceiling,
                        ..Ceilings::DEFAULT
                    },
                ),
                (
                    "ipv6_56",
                    Ceilings {
                        ipv6_56: ceiling,
                        ..Ceilings::DEFAULT
                    },
                ),
                (
                    "ipv6_48",
                    Ceilings {
                        ipv6_48: ceiling,
                        ..Ceilings::DEFAULT
                    },
                ),
                (
                    "unknown",
                    Ceilings {
                        unknown: ceiling,
                        ..Ceilings::DEFAULT
                    },
                ),
            ];
            for (key, ceilings) in cases {
                assert_eq!(
                    SourceLimiter::new(&ceilings, 16).err(),
                    Some(CeilingError::NotBelowServer { key }),
                    "{ceiling:?}"
                );
            }
        }
    }

    #[test]
    fn refuses_a_ceiling_the_core_cannot_count_with() {
        let no_burst = Ceilings {
            source: Ceiling {
                interval_ms: 6_000,
                burst: 0,
            },
            ..Ceilings::DEFAULT
        };
        assert_eq!(
            SourceLimiter::new(&no_burst, 16).err(),
            Some(CeilingError::Unusable {
                key: "source",
                error: RateError::ZeroBurst,
            })
        );
        let no_interval = Ceilings {
            server: Ceiling {
                interval_ms: 0,
                burst: 100,
            },
            ..Ceilings::DEFAULT
        };
        assert_eq!(
            SourceLimiter::new(&no_interval, 16).err(),
            Some(CeilingError::Unusable {
                key: "server",
                error: RateError::ZeroInterval,
            })
        );
    }
}
