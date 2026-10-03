//! Rate-limit arithmetic: GCRA, key derivation, the guessable-secret delay
//! schedule, and a fixed-capacity store.
//!
//! The core never reads a clock. [`check`] takes `now` as a
//! [`Timestamp`](crate::time::Timestamp), so tests inject time. Later
//! packages (the credential verifier, the request-limit hook) look up a
//! [`Tat`] per [`LimitKey`] in a [`BoundedStore`] and call [`check`] for
//! each key [`keys`] or [`keys_for`] returned. The server-wide counter is
//! the same GCRA under [`LimitKey::Global`].

use core::hash::Hash;
use core::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::collections::{HashMap, VecDeque};

use crate::client_context::{ClientContext, PathClass};
use crate::time::Timestamp;

/// How fast a key may be used: one request every `interval_ms`
/// milliseconds, with a burst of `burst` requests at once.
///
/// Constructed only by [`Rate::new`], so a zero interval, a zero burst, or
/// a pair whose `burst × interval` does not fit in an `i64` cannot reach
/// [`check`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rate {
    interval_ms: i64,
    tau_ms: i64,
}

/// Why a [`Rate`] could not be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateError {
    /// The emission interval was zero.
    ZeroInterval,
    /// The burst was zero.
    ZeroBurst,
    /// The interval, or `burst × interval_ms`, does not fit in an `i64`
    /// of milliseconds, so it cannot be added to a [`Timestamp`].
    TooLarge,
}

impl Rate {
    /// A rate of one request every `interval_ms` milliseconds, allowing
    /// `burst` requests at once.
    ///
    /// # Errors
    ///
    /// [`RateError::ZeroInterval`] when `interval_ms` is 0,
    /// [`RateError::ZeroBurst`] when `burst` is 0,
    /// [`RateError::TooLarge`] when the interval or `burst × interval_ms`
    /// does not fit in an `i64`.
    pub fn new(interval_ms: u64, burst: u32) -> Result<Self, RateError> {
        if interval_ms == 0 {
            return Err(RateError::ZeroInterval);
        }
        if burst == 0 {
            return Err(RateError::ZeroBurst);
        }
        let Ok(interval) = i64::try_from(interval_ms) else {
            return Err(RateError::TooLarge);
        };
        let Some(product) = u64::from(burst).checked_mul(interval_ms) else {
            return Err(RateError::TooLarge);
        };
        let Ok(span) = i64::try_from(product) else {
            return Err(RateError::TooLarge);
        };
        let tau_ms = span.saturating_sub(interval);
        Ok(Self {
            interval_ms: interval,
            tau_ms,
        })
    }
}

/// The GCRA theoretical arrival time of the next cell, plus the latest
/// `now` this key has observed, so a clock that steps backwards is treated
/// as no time having passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tat {
    theoretical_ms: i64,
    observed_ms: i64,
}

/// Whether a request may proceed, and when to retry if not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// The request is within the rate.
    Allow,
    /// The request is over the rate. `retry_after_ms` milliseconds must
    /// pass before the next request on this key would be allowed.
    Deny {
        /// Milliseconds until the earliest time this key would allow a
        /// request, counting from the (possibly clamped) `now` passed to
        /// [`check`].
        retry_after_ms: u64,
    },
}

/// Applies GCRA at `now` to `state`.
///
/// An empty `state` is a first request and is always allowed. A `now`
/// earlier than the last time this key observed is treated as that last
/// time, so stepping the clock backwards grants no extra requests and
/// does not postpone recovery.
///
/// The returned [`Tat`] is the state to store for the next call, whether
/// the decision is [`Decision::Allow`] or [`Decision::Deny`]. On deny the
/// theoretical arrival time is unchanged, so a refused request does not
/// spend a cell.
#[must_use]
pub fn check(state: Option<Tat>, now: Timestamp, rate: Rate) -> (Decision, Tat) {
    let now_ms = now.millis();
    let Some(tat) = state else {
        return (
            Decision::Allow,
            Tat {
                theoretical_ms: saturating_add(now_ms, rate.interval_ms),
                observed_ms: now_ms,
            },
        );
    };
    let now_ms = now_ms.max(tat.observed_ms);
    // A saturated TAT must not subtract tau: TAT - tau would sit near 0
    // and every later `now >= 0` would Allow.
    let earliest = if tat.theoretical_ms == i64::MAX {
        i64::MAX
    } else {
        tat.theoretical_ms.saturating_sub(rate.tau_ms)
    };
    if now_ms < earliest {
        let retry_after_ms = earliest.saturating_sub(now_ms).unsigned_abs();
        return (
            Decision::Deny { retry_after_ms },
            Tat {
                theoretical_ms: tat.theoretical_ms,
                observed_ms: now_ms,
            },
        );
    }
    let base = now_ms.max(tat.theoretical_ms);
    (
        Decision::Allow,
        Tat {
            theoretical_ms: saturating_add(base, rate.interval_ms),
            observed_ms: now_ms,
        },
    )
}

/// Milliseconds to wait after `failures` consecutive wrong guesses of a
/// guessable secret (SEC-API-056).
///
/// The schedule is 30 seconds, 1 minute, 5 minutes, then 15 minutes. It
/// never grows past 15 minutes and is never a permanent lockout: every
/// `failures` value, including zero and [`u32::MAX`], returns a finite
/// delay.
#[must_use]
pub const fn secret_delay_ms(failures: u32) -> u64 {
    secret_delay_i64(failures).unsigned_abs()
}

/// The earliest time another guess may be tried after `failures` wrong
/// guesses recorded at `failed_at`.
#[must_use]
pub fn next_guess_at(failures: u32, failed_at: Timestamp) -> Timestamp {
    add_millis(failed_at, secret_delay_i64(failures))
}

const fn secret_delay_i64(failures: u32) -> i64 {
    match failures {
        0 => 0,
        1 => 30_000,
        2 => 60_000,
        3 => 300_000,
        _ => 900_000,
    }
}

/// The principal a rate limit is keyed on when the request is
/// authenticated (SEC-API-057).
///
/// Sixteen opaque bytes, typically an account's public-id bytes. This
/// package does not mint identifiers; later packages construct a key from
/// the principal they already have.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PrincipalKey([u8; 16]);

impl PrincipalKey {
    /// The key that carries `bytes`.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// The bytes this key was built from.
    #[must_use]
    pub const fn as_bytes(self) -> [u8; 16] {
        self.0
    }
}

/// One counter a request is charged against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LimitKey {
    /// The authenticated principal.
    Principal(PrincipalKey),
    /// One IPv4 address.
    Ipv4(Ipv4Addr),
    /// The /64 containing an IPv6 address.
    Ipv6Slash64(Ipv6Addr),
    /// The /56 containing an IPv6 address.
    Ipv6Slash56(Ipv6Addr),
    /// The /48 containing an IPv6 address.
    Ipv6Slash48(Ipv6Addr),
    /// The single server-wide counter (SEC-IAM-101).
    Global,
    /// Every peer classified [`PathClass::Unknown`] (SEC-NET-068).
    Unknown,
}

/// The keys a request must pass, in a fixed order: the per-source keys,
/// then [`LimitKey::Global`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LimitKeys {
    keys: [LimitKey; 4],
    count: u8,
}

impl LimitKeys {
    fn new(keys: [LimitKey; 4], count: u8) -> Self {
        Self { keys, count }
    }

    /// The keys in the order they should be checked.
    pub fn iter(self) -> impl Iterator<Item = LimitKey> {
        self.keys.into_iter().take(usize::from(self.count))
    }
}

/// The keys for `principal` when there is one, otherwise for `addr`, plus
/// the server-wide [`LimitKey::Global`] (SEC-API-057, SEC-NET-052,
/// SEC-IAM-101).
///
/// An IPv4-mapped IPv6 address is keyed as IPv4. An IPv6 address produces
/// three prefix keys (/64, /56, /48), each of which later packages give
/// its own ceiling.
#[must_use]
pub fn keys(principal: Option<PrincipalKey>, addr: IpAddr) -> LimitKeys {
    if let Some(principal) = principal {
        return LimitKeys::new(
            [
                LimitKey::Principal(principal),
                LimitKey::Global,
                LimitKey::Global,
                LimitKey::Global,
            ],
            2,
        );
    }
    match addr.to_canonical() {
        IpAddr::V4(v4) => LimitKeys::new(
            [
                LimitKey::Ipv4(v4),
                LimitKey::Global,
                LimitKey::Global,
                LimitKey::Global,
            ],
            2,
        ),
        IpAddr::V6(v6) => LimitKeys::new(
            [
                LimitKey::Ipv6Slash64(mask_v6(v6, 64)),
                LimitKey::Ipv6Slash56(mask_v6(v6, 56)),
                LimitKey::Ipv6Slash48(mask_v6(v6, 48)),
                LimitKey::Global,
            ],
            4,
        ),
    }
}

/// The keys for a request whose address and path class have already been
/// resolved into a [`ClientContext`].
///
/// An unauthenticated request classified [`PathClass::Unknown`] uses
/// [`LimitKey::Unknown`] rather than the gateway address, so exhausting
/// that bucket cannot share a per-source key with loopback or a
/// local-direct peer (SEC-NET-052, SEC-NET-068). Authenticated requests
/// still key on the principal.
#[must_use]
pub fn keys_for(principal: Option<PrincipalKey>, context: &ClientContext) -> LimitKeys {
    if principal.is_some() {
        return keys(principal, context.addr());
    }
    if context.class() == PathClass::Unknown {
        return LimitKeys::new(
            [
                LimitKey::Unknown,
                LimitKey::Global,
                LimitKey::Global,
                LimitKey::Global,
            ],
            2,
        );
    }
    keys(None, context.addr())
}

/// A keyed map that never holds more than `capacity` entries (SEC-NET-051).
///
/// Inserting a new key at capacity evicts the least recently used entry.
/// [`BoundedStore::get`] and a replacement [`BoundedStore::insert`] both
/// count as a use. A capacity of zero stores nothing; [`check_store`]
/// fails closed on a grant whose keys do not all fit.
#[derive(Debug)]
pub struct BoundedStore<K, V> {
    capacity: usize,
    map: HashMap<K, V>,
    order: VecDeque<K>,
}

impl<K: Eq + Hash + Clone, V> BoundedStore<K, V> {
    /// An empty store that will hold at most `capacity` keys.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            map: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    /// The maximum number of keys this store will hold.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// How many keys are stored.
    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Whether the store holds no keys.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// The value for `key`, recording a use, or `None` if it is absent or
    /// has been evicted.
    pub fn get(&mut self, key: &K) -> Option<&V> {
        if self.map.contains_key(key) {
            self.touch(key);
        }
        self.map.get(key)
    }

    /// Stores `value` at `key`. Returns the previous value when `key` was
    /// already present. A new key at capacity evicts the least recently
    /// used entry; a capacity of zero ignores the insert and returns
    /// `None`.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if let Some(slot) = self.map.get_mut(&key) {
            let previous = core::mem::replace(slot, value);
            self.touch(&key);
            return Some(previous);
        }
        if self.map.len() == self.capacity {
            let evicted = self.order.pop_front()?;
            self.map.remove(&evicted);
        }
        self.order.push_back(key.clone());
        self.map.insert(key, value)
    }

    fn touch(&mut self, key: &K) {
        self.order.retain(|held| held != key);
        self.order.push_back(key.clone());
    }

    /// Stores every entry as one eviction transaction: members of the
    /// group never evict each other, and only least recently used keys
    /// outside the group are evicted to make room.
    ///
    /// Returns `false` without storing anything when the group is larger
    /// than [`Self::capacity`] (including any group at capacity zero), so a
    /// grant that cannot be recorded in full does not proceed.
    fn insert_group(&mut self, entries: Vec<(K, V)>) -> bool {
        if entries.len() > self.capacity {
            return false;
        }
        let new_count = entries
            .iter()
            .filter(|(key, _)| !self.map.contains_key(key))
            .count();
        let free = self.capacity.saturating_sub(self.map.len());
        let mut need = new_count.saturating_sub(free);
        let map = &mut self.map;
        self.order.retain(|key| {
            if need == 0 || entries.iter().any(|(held, _)| held == key) {
                return true;
            }
            map.remove(key);
            need = need.saturating_sub(1);
            false
        });
        for (key, value) in entries {
            if self.map.insert(key.clone(), value).is_some() {
                self.touch(&key);
            } else {
                self.order.push_back(key);
            }
        }
        true
    }
}

/// Checks every key in `keys` against `store`. Allows only when every key
/// allows, and stores the new [`Tat`] for each key only then, so a deny on
/// the global key does not spend a per-source cell (SEC-IAM-101).
///
/// The Tats are committed as one eviction transaction. If the store cannot
/// record every key of the grant (its capacity is smaller than the number
/// of keys), the request is denied and the store is left unchanged.
///
/// `rate_of` supplies each key's ceiling: later packages give the /64, /56
/// and /48 counters, and [`LimitKey::Global`], rates of their own.
#[must_use]
pub fn check_store<F>(
    store: &mut BoundedStore<LimitKey, Tat>,
    keys: &LimitKeys,
    now: Timestamp,
    mut rate_of: F,
) -> Decision
where
    F: FnMut(LimitKey) -> Rate,
{
    // Record coverage on one body, not on every rate_of monomorphization.
    check_store_dyn(store, keys, now, &mut rate_of)
}

fn check_store_dyn(
    store: &mut BoundedStore<LimitKey, Tat>,
    keys: &LimitKeys,
    now: Timestamp,
    rate_of: &mut dyn FnMut(LimitKey) -> Rate,
) -> Decision {
    let mut allowed = Vec::new();
    let mut denied = false;
    let mut retry_after_ms = 0_u64;
    for key in keys.iter() {
        let (decision, tat) = check(store.get(&key).copied(), now, rate_of(key));
        match decision {
            Decision::Allow => allowed.push((key, tat)),
            Decision::Deny {
                retry_after_ms: retry,
            } => {
                denied = true;
                retry_after_ms = retry_after_ms.max(retry);
            }
        }
    }
    if denied {
        Decision::Deny { retry_after_ms }
    } else {
        let now_ms = now.millis();
        let mut grant_retry_ms = 0_u64;
        for (_, tat) in &allowed {
            grant_retry_ms =
                grant_retry_ms.max(tat.theoretical_ms.saturating_sub(now_ms).unsigned_abs());
        }
        if store.insert_group(allowed) {
            Decision::Allow
        } else {
            Decision::Deny {
                retry_after_ms: grant_retry_ms,
            }
        }
    }
}

fn mask_v6(addr: Ipv6Addr, prefix: u8) -> Ipv6Addr {
    let bits = addr.to_bits();
    let masked = match prefix {
        64 => bits & 0xffff_ffff_ffff_ffff_0000_0000_0000_0000,
        56 => bits & 0xffff_ffff_ffff_ff00_0000_0000_0000_0000,
        48 => bits & 0xffff_ffff_ffff_0000_0000_0000_0000_0000,
        _ => 0,
    };
    Ipv6Addr::from_bits(masked)
}

fn saturating_add(base: i64, delta: i64) -> i64 {
    base.checked_add(delta).unwrap_or(i64::MAX)
}

fn add_millis(time: Timestamp, delta: i64) -> Timestamp {
    Timestamp::from_millis(saturating_add(time.millis(), delta)).unwrap_or(Timestamp::MAX)
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use super::*;
    use crate::client_context::ClientContext;
    use proptest::prelude::*;

    const START_MS: i64 = 1_700_000_000_000;

    fn ts(ms: i64) -> Timestamp {
        Timestamp::from_millis(ms).unwrap()
    }

    fn rate(interval_ms: u64, burst: u32) -> Rate {
        Rate::new(interval_ms, burst).unwrap()
    }

    fn allow(state: Option<Tat>, now: i64, rate: Rate) -> Tat {
        let (decision, tat) = check(state, ts(now), rate);
        assert_eq!(decision, Decision::Allow);
        tat
    }

    fn deny(state: Option<Tat>, now: i64, rate: Rate, retry_after_ms: u64) -> Tat {
        let (decision, tat) = check(state, ts(now), rate);
        assert_eq!(decision, Decision::Deny { retry_after_ms });
        tat
    }

    fn collected(keys: LimitKeys) -> Vec<LimitKey> {
        keys.iter().collect()
    }

    fn principal(byte: u8) -> PrincipalKey {
        PrincipalKey::from_bytes([byte; 16])
    }

    fn context(addr: IpAddr, class: PathClass) -> ClientContext {
        ClientContext::new(addr, class, false)
    }

    fn v4(a: u8, b: u8, c: u8, d: u8) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(a, b, c, d))
    }

    fn v6(segments: [u16; 8]) -> Ipv6Addr {
        Ipv6Addr::new(
            segments[0],
            segments[1],
            segments[2],
            segments[3],
            segments[4],
            segments[5],
            segments[6],
            segments[7],
        )
    }

    /// Independent prefix mask: keep the first `prefix` bits of each
    /// 16-bit segment, written without the production shift-on-u128.
    fn reference_prefix(addr: Ipv6Addr, prefix: u8) -> Ipv6Addr {
        let segs = addr.segments();
        let mut out = [0_u16; 8];
        let mut remaining = u32::from(prefix.min(128));
        for (i, seg) in segs.iter().enumerate() {
            if remaining >= 16 {
                out[i] = *seg;
                remaining -= 16;
            } else if remaining > 0 {
                let keep = remaining;
                let mask = 0xFFFF_u16
                    .checked_shl(16_u32.saturating_sub(keep))
                    .unwrap_or_default();
                out[i] = seg & mask;
                remaining = 0;
            }
        }
        v6(out)
    }

    #[test]
    fn refuses_a_zero_interval_or_burst() {
        assert_eq!(Rate::new(0, 1), Err(RateError::ZeroInterval));
        assert_eq!(Rate::new(0, 0), Err(RateError::ZeroInterval));
        assert_eq!(Rate::new(1, 0), Err(RateError::ZeroBurst));
        assert_eq!(Rate::new(1_000, 0), Err(RateError::ZeroBurst));
    }

    #[test]
    fn refuses_an_interval_or_tolerance_that_does_not_fit_an_i64() {
        assert_eq!(
            Rate::new(u64::try_from(i64::MAX).unwrap() + 1, 1),
            Err(RateError::TooLarge)
        );
        assert_eq!(Rate::new(u64::MAX, 1), Err(RateError::TooLarge));
        assert_eq!(Rate::new(u64::MAX / 2, 3), Err(RateError::TooLarge));
        assert_eq!(
            Rate::new(u64::try_from(i64::MAX).unwrap(), 2),
            Err(RateError::TooLarge)
        );
        assert_eq!(
            Rate::new(u64::try_from(i64::MAX).unwrap(), 3),
            Err(RateError::TooLarge)
        );
        assert_eq!(
            Rate::new(u64::try_from(i64::MAX / 2).unwrap(), 3),
            Err(RateError::TooLarge)
        );
        assert!(Rate::new(u64::try_from(i64::MAX).unwrap(), 1).is_ok());
        assert!(Rate::new(1, u32::MAX).is_ok());
    }

    #[test]
    fn a_burst_at_the_limit_is_allowed_and_one_over_is_not() {
        let rate = rate(1_000, 3);
        let first = allow(None, START_MS, rate);
        let second = allow(Some(first), START_MS, rate);
        let third = allow(Some(second), START_MS, rate);
        deny(Some(third), START_MS, rate, 1_000);
        deny(Some(third), START_MS + 999, rate, 1);
    }

    #[test]
    fn recovers_after_exactly_the_emission_interval() {
        let rate = rate(1_000, 3);
        let first = allow(None, START_MS, rate);
        let second = allow(Some(first), START_MS, rate);
        let third = allow(Some(second), START_MS, rate);
        deny(Some(third), START_MS + 999, rate, 1);
        let fourth = allow(Some(third), START_MS + 1_000, rate);
        deny(Some(fourth), START_MS + 1_000, rate, 1_000);
        allow(Some(fourth), START_MS + 2_000, rate);
    }

    #[test]
    fn a_burst_of_one_needs_a_full_interval_between_requests() {
        let rate = rate(250, 1);
        let first = allow(None, START_MS, rate);
        deny(Some(first), START_MS, rate, 250);
        deny(Some(first), START_MS + 249, rate, 1);
        allow(Some(first), START_MS + 250, rate);
    }

    #[test]
    fn a_clock_going_backwards_is_treated_as_no_time_passed() {
        let rate = rate(1_000, 3);
        let first = allow(None, START_MS, rate);
        let second = allow(Some(first), START_MS, rate);
        // Two of three burst cells spent; stepping back must not lose the
        // remaining cell and must not mint extra ones.
        let third = allow(Some(second), START_MS - 500, rate);
        deny(Some(third), START_MS - 1_000, rate, 1_000);
        deny(Some(third), START_MS + 999, rate, 1);
        allow(Some(third), START_MS + 1_000, rate);
    }

    #[test]
    fn a_clock_going_backwards_after_the_burst_does_not_reset_the_limiter() {
        let rate = rate(1_000, 2);
        let first = allow(None, START_MS, rate);
        let second = allow(Some(first), START_MS, rate);
        let denied = deny(Some(second), START_MS - 5_000, rate, 1_000);
        deny(Some(denied), START_MS, rate, 1_000);
        deny(Some(denied), START_MS + 999, rate, 1);
        allow(Some(denied), START_MS + 1_000, rate);
    }

    #[test]
    fn a_denied_request_does_not_spend_a_cell() {
        let rate = rate(1_000, 1);
        let first = allow(None, START_MS, rate);
        let denied = deny(Some(first), START_MS + 100, rate, 900);
        deny(Some(denied), START_MS + 100, rate, 900);
        allow(Some(denied), START_MS + 1_000, rate);
    }

    #[test]
    fn saturates_a_theoretical_arrival_that_would_overflow_i64() {
        let rate = rate(u64::try_from(i64::MAX).unwrap(), 1);
        let first = allow(None, START_MS, rate);
        let (decision, _) = check(Some(first), ts(START_MS + 1), rate);
        assert_eq!(
            decision,
            Decision::Deny {
                retry_after_ms: u64::try_from(i64::MAX - (START_MS + 1)).unwrap(),
            }
        );
    }

    #[test]
    fn a_saturated_tat_denies_instead_of_treating_earliest_as_zero() {
        let rate = rate(1_000, 3);
        let saturated = Tat {
            theoretical_ms: i64::MAX,
            observed_ms: START_MS,
        };
        let (decision, next) = check(Some(saturated), ts(START_MS), rate);
        assert_eq!(
            decision,
            Decision::Deny {
                retry_after_ms: u64::try_from(i64::MAX - START_MS).unwrap(),
            }
        );
        assert_eq!(
            next,
            Tat {
                theoretical_ms: i64::MAX,
                observed_ms: START_MS,
            }
        );
    }

    /// Verifies: SEC-API-056
    #[test]
    fn the_guessable_secret_delay_is_the_literal_schedule() {
        const STEPS: [(u32, u64); 8] = [
            (0, 0),
            (1, 30_000),
            (2, 60_000),
            (3, 300_000),
            (4, 900_000),
            (5, 900_000),
            (10, 900_000),
            (u32::MAX, 900_000),
        ];
        for (failures, delay_ms) in STEPS {
            assert_eq!(secret_delay_ms(failures), delay_ms, "failures={failures}");
            assert_eq!(
                next_guess_at(failures, ts(START_MS)),
                ts(START_MS + i64::try_from(delay_ms).unwrap()),
                "failures={failures}"
            );
        }
    }

    /// Verifies: SEC-API-056
    #[test]
    fn the_delay_schedule_never_locks_out_permanently() {
        assert_eq!(secret_delay_ms(u32::MAX), 900_000);
        assert_ne!(secret_delay_ms(u32::MAX), u64::MAX);
        let at_end = next_guess_at(4, Timestamp::MAX);
        assert_eq!(at_end, Timestamp::MAX);
    }

    /// Verifies: SEC-API-057
    #[test]
    fn a_principal_replaces_the_address_and_still_carries_the_global_key() {
        let one = principal(1);
        let two = principal(2);
        let addr_a = v4(203, 0, 113, 7);
        let addr_b = v4(198, 51, 100, 9);
        assert_eq!(
            collected(keys(Some(one), addr_a)),
            [LimitKey::Principal(one), LimitKey::Global]
        );
        assert_eq!(
            collected(keys(Some(one), addr_b)),
            collected(keys(Some(one), addr_a))
        );
        assert_ne!(
            collected(keys(Some(one), addr_a)),
            collected(keys(Some(two), addr_a))
        );
        assert_eq!(collected(keys(Some(one), addr_a)).len(), 2);
        assert_eq!(one.as_bytes(), [1; 16]);
        let mixed =
            PrincipalKey::from_bytes([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
        assert_eq!(
            mixed.as_bytes(),
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
        );
        assert_ne!(mixed.as_bytes(), [0; 16]);
        assert_ne!(mixed.as_bytes(), [1; 16]);
    }

    /// Verifies: SEC-NET-052, SEC-API-057
    #[test]
    fn an_ipv4_address_is_its_own_key() {
        let a = v4(203, 0, 113, 7);
        let b = v4(203, 0, 113, 8);
        assert_eq!(
            collected(keys(None, a)),
            [
                LimitKey::Ipv4(Ipv4Addr::new(203, 0, 113, 7)),
                LimitKey::Global
            ]
        );
        assert_ne!(collected(keys(None, a)), collected(keys(None, b)));
        assert_eq!(
            collected(keys(None, a))[0],
            collected(keys(
                None,
                IpAddr::V6(Ipv4Addr::new(203, 0, 113, 7).to_ipv6_mapped())
            ))[0],
            "IPv4-mapped IPv6 must share the IPv4 key"
        );
    }

    /// Verifies: SEC-NET-052
    #[test]
    fn every_address_in_one_slash_64_shares_that_slash_64_key() {
        let a = v6([0x2001, 0x0DB8, 0x0001, 0x0002, 0, 0, 0, 1]);
        let b = v6([
            0x2001, 0x0DB8, 0x0001, 0x0002, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF,
        ]);
        let other = v6([0x2001, 0x0DB8, 0x0001, 0x0003, 0, 0, 0, 1]);
        let keys_a = collected(keys(None, IpAddr::V6(a)));
        let keys_b = collected(keys(None, IpAddr::V6(b)));
        let keys_other = collected(keys(None, IpAddr::V6(other)));
        assert_eq!(keys_a[0], keys_b[0]);
        assert_eq!(keys_a[0], LimitKey::Ipv6Slash64(reference_prefix(a, 64)));
        assert_ne!(keys_a[0], keys_other[0]);
        assert_eq!(keys_a[1], keys_b[1]);
        assert_eq!(keys_a[2], keys_b[2]);
        assert_eq!(keys_a[3], LimitKey::Global);
        assert_eq!(collected(keys(None, IpAddr::V6(a))).len(), 4);
        assert_eq!(mask_v6(a, 0), Ipv6Addr::UNSPECIFIED);
        assert_eq!(mask_v6(a, 128), Ipv6Addr::UNSPECIFIED);
    }

    /// Verifies: SEC-NET-052
    #[test]
    fn rotating_slash_64s_inside_one_slash_48_share_the_slash_48_key() {
        let first = v6([0x2001, 0x0DB8, 0xABCD, 0x0000, 0, 0, 0, 1]);
        let rotated = v6([0x2001, 0x0DB8, 0xABCD, 0xFFFF, 0, 0, 0, 2]);
        let outside = v6([0x2001, 0x0DB8, 0xABCE, 0x0000, 0, 0, 0, 1]);
        let a = collected(keys(None, IpAddr::V6(first)));
        let b = collected(keys(None, IpAddr::V6(rotated)));
        let c = collected(keys(None, IpAddr::V6(outside)));
        assert_ne!(a[0], b[0], "distinct /64s");
        assert_eq!(a[2], b[2], "same /48");
        assert_eq!(a[2], LimitKey::Ipv6Slash48(reference_prefix(first, 48)));
        assert_ne!(a[2], c[2], "a neighbouring /48 is a different key");
    }

    /// Verifies: SEC-NET-052
    #[test]
    fn an_unknown_peer_has_its_own_bucket_that_loopback_does_not_share() {
        let unknown = context(v4(172, 17, 0, 1), PathClass::Unknown);
        let loopback = context(v4(127, 0, 0, 1), PathClass::Loopback);
        let home = context(v4(192, 168, 1, 20), PathClass::Home);
        let internet = context(v4(203, 0, 113, 7), PathClass::Internet);
        let unknown_keys = collected(keys_for(None, &unknown));
        let loopback_keys = collected(keys_for(None, &loopback));
        let home_keys = collected(keys_for(None, &home));
        let internet_keys = collected(keys_for(None, &internet));
        assert_eq!(unknown_keys, [LimitKey::Unknown, LimitKey::Global]);
        assert_eq!(
            loopback_keys,
            [LimitKey::Ipv4(Ipv4Addr::LOCALHOST), LimitKey::Global]
        );
        assert_ne!(unknown_keys[0], loopback_keys[0]);
        assert_ne!(unknown_keys[0], home_keys[0]);
        assert_ne!(unknown_keys[0], internet_keys[0]);
        assert_eq!(
            collected(keys_for(Some(principal(9)), &unknown)),
            [LimitKey::Principal(principal(9)), LimitKey::Global]
        );
    }

    /// Verifies: SEC-NET-052, SEC-IAM-101
    #[test]
    fn exhausting_a_slash_48_stops_new_slash_64s_while_another_slash_48_passes() {
        let tight = rate(1_000, 2);
        let generous = rate(1, 255);
        let rate_of = |key: LimitKey| match key {
            LimitKey::Ipv6Slash48(_) => tight,
            _ => generous,
        };
        let mut store = BoundedStore::new(32);
        let inside_one = v6([0x2001, 0x0DB8, 0xAAAA, 0x0001, 0, 0, 0, 1]);
        let inside_two = v6([0x2001, 0x0DB8, 0xAAAA, 0x0002, 0, 0, 0, 1]);
        let outside = v6([0x2001, 0x0DB8, 0xBBBB, 0x0001, 0, 0, 0, 1]);
        assert_eq!(
            check_store(
                &mut store,
                &keys(None, IpAddr::V6(inside_one)),
                ts(START_MS),
                rate_of
            ),
            Decision::Allow
        );
        assert_eq!(
            check_store(
                &mut store,
                &keys(None, IpAddr::V6(inside_two)),
                ts(START_MS),
                rate_of
            ),
            Decision::Allow
        );
        assert_eq!(
            check_store(
                &mut store,
                &keys(None, IpAddr::V6(inside_one)),
                ts(START_MS),
                rate_of
            ),
            Decision::Deny {
                retry_after_ms: 1_000
            }
        );
        assert_eq!(
            check_store(
                &mut store,
                &keys(None, IpAddr::V6(outside)),
                ts(START_MS),
                rate_of
            ),
            Decision::Allow
        );
    }

    /// Verifies: SEC-IAM-101, SEC-API-057
    #[test]
    fn the_global_key_caps_the_server_whatever_each_source_still_has_left() {
        let per_source = rate(1_000, 5);
        let global = rate(1_000, 2);
        let rate_of = |key: LimitKey| match key {
            LimitKey::Global => global,
            _ => per_source,
        };
        let mut store = BoundedStore::new(16);
        let a = v4(203, 0, 113, 1);
        let b = v4(203, 0, 113, 2);
        assert_eq!(
            check_store(&mut store, &keys(None, a), ts(START_MS), rate_of),
            Decision::Allow
        );
        assert_eq!(
            check_store(&mut store, &keys(None, b), ts(START_MS), rate_of),
            Decision::Allow
        );
        assert_eq!(
            check_store(&mut store, &keys(None, a), ts(START_MS), rate_of),
            Decision::Deny {
                retry_after_ms: 1_000
            }
        );
        assert_eq!(
            check_store(&mut store, &keys(None, b), ts(START_MS), rate_of),
            Decision::Deny {
                retry_after_ms: 1_000
            }
        );
    }

    /// Verifies: SEC-IAM-101
    #[test]
    fn a_global_deny_does_not_store_a_per_source_tat() {
        let per_source = rate(60_000, 1);
        let global = rate(1_000, 1);
        let rate_of = |key: LimitKey| match key {
            LimitKey::Global => global,
            _ => per_source,
        };
        let mut store = BoundedStore::new(8);
        let a_addr = Ipv4Addr::new(203, 0, 113, 1);
        let b_addr = Ipv4Addr::new(198, 51, 100, 1);
        let a = IpAddr::V4(a_addr);
        let b = IpAddr::V4(b_addr);
        assert_eq!(
            check_store(&mut store, &keys(None, a), ts(START_MS), rate_of),
            Decision::Allow
        );
        assert_eq!(
            store.get(&LimitKey::Ipv4(a_addr)).copied(),
            Some(Tat {
                theoretical_ms: START_MS + 60_000,
                observed_ms: START_MS,
            })
        );
        assert_eq!(
            store.get(&LimitKey::Global).copied(),
            Some(Tat {
                theoretical_ms: START_MS + 1_000,
                observed_ms: START_MS,
            })
        );
        assert_eq!(
            check_store(&mut store, &keys(None, b), ts(START_MS), rate_of),
            Decision::Deny {
                retry_after_ms: 1_000
            }
        );
        assert_eq!(store.get(&LimitKey::Ipv4(b_addr)), None);
        assert_eq!(
            store.get(&LimitKey::Ipv4(a_addr)).copied(),
            Some(Tat {
                theoretical_ms: START_MS + 60_000,
                observed_ms: START_MS,
            })
        );
        assert_eq!(
            store.get(&LimitKey::Global).copied(),
            Some(Tat {
                theoretical_ms: START_MS + 1_000,
                observed_ms: START_MS,
            })
        );
        assert_eq!(
            check_store(&mut store, &keys(None, b), ts(START_MS + 1_000), rate_of),
            Decision::Allow
        );
        assert_eq!(
            store.get(&LimitKey::Ipv4(b_addr)).copied(),
            Some(Tat {
                theoretical_ms: START_MS + 61_000,
                observed_ms: START_MS + 1_000,
            })
        );
    }

    /// Verifies: SEC-IAM-101
    #[test]
    fn a_per_source_deny_does_not_spend_the_global_cell() {
        let per_source = rate(1_000, 1);
        let global = rate(1_000, 5);
        let rate_of = |key: LimitKey| match key {
            LimitKey::Global => global,
            _ => per_source,
        };
        let mut store = BoundedStore::new(8);
        let a_addr = Ipv4Addr::new(203, 0, 113, 1);
        let c_addr = Ipv4Addr::new(198, 51, 100, 1);
        let a = IpAddr::V4(a_addr);
        let c = IpAddr::V4(c_addr);
        assert_eq!(
            check_store(&mut store, &keys(None, a), ts(START_MS), rate_of),
            Decision::Allow
        );
        let spent_once = Tat {
            theoretical_ms: START_MS + 1_000,
            observed_ms: START_MS,
        };
        assert_eq!(
            store.get(&LimitKey::Ipv4(a_addr)).copied(),
            Some(spent_once)
        );
        assert_eq!(store.get(&LimitKey::Global).copied(), Some(spent_once));
        assert_eq!(
            check_store(&mut store, &keys(None, a), ts(START_MS), rate_of),
            Decision::Deny {
                retry_after_ms: 1_000
            }
        );
        assert_eq!(
            store.get(&LimitKey::Ipv4(a_addr)).copied(),
            Some(spent_once)
        );
        assert_eq!(store.get(&LimitKey::Global).copied(), Some(spent_once));
        assert_eq!(
            check_store(&mut store, &keys(None, c), ts(START_MS), rate_of),
            Decision::Allow
        );
        assert_eq!(
            store.get(&LimitKey::Ipv4(c_addr)).copied(),
            Some(Tat {
                theoretical_ms: START_MS + 1_000,
                observed_ms: START_MS,
            })
        );
        assert_eq!(
            store.get(&LimitKey::Global).copied(),
            Some(Tat {
                theoretical_ms: START_MS + 2_000,
                observed_ms: START_MS,
            })
        );
    }

    /// Verifies: SEC-IAM-101
    #[test]
    fn a_deny_returns_the_longest_retry_among_the_keys() {
        let a = v4(203, 0, 113, 1);
        let longer_first = |key: LimitKey| match key {
            LimitKey::Global => rate(1_000, 1),
            _ => rate(5_000, 1),
        };
        let mut store = BoundedStore::new(8);
        assert_eq!(
            check_store(&mut store, &keys(None, a), ts(START_MS), longer_first),
            Decision::Allow
        );
        assert_eq!(
            check_store(&mut store, &keys(None, a), ts(START_MS + 1), longer_first),
            Decision::Deny {
                retry_after_ms: 4_999
            }
        );

        let longer_last = |key: LimitKey| match key {
            LimitKey::Global => rate(5_000, 1),
            _ => rate(1_000, 1),
        };
        let mut store = BoundedStore::new(8);
        assert_eq!(
            check_store(&mut store, &keys(None, a), ts(START_MS), longer_last),
            Decision::Allow
        );
        assert_eq!(
            check_store(&mut store, &keys(None, a), ts(START_MS + 1), longer_last),
            Decision::Deny {
                retry_after_ms: 4_999
            }
        );
    }

    /// Verifies: SEC-NET-051
    #[test]
    fn a_full_store_evicts_the_least_recently_used_key() {
        let mut store = BoundedStore::new(2);
        assert_eq!(store.capacity(), 2);
        assert_eq!(store.len(), 0);
        assert!(store.is_empty());
        assert_eq!(store.insert("a", 1), None);
        assert_eq!(store.insert("b", 2), None);
        assert_eq!(store.len(), 2);
        assert!(!store.is_empty());
        assert_eq!(store.insert("c", 3), None);
        assert_eq!(store.get(&"a"), None);
        assert_eq!(store.get(&"b"), Some(&2));
        assert_eq!(store.get(&"c"), Some(&3));
        assert_eq!(store.len(), 2);
    }

    /// Verifies: SEC-NET-051
    #[test]
    fn getting_a_key_makes_it_more_recent_than_its_neighbours() {
        let mut store = BoundedStore::new(2);
        store.insert("a", 1);
        store.insert("b", 2);
        assert_eq!(store.get(&"a"), Some(&1));
        store.insert("c", 3);
        assert_eq!(store.get(&"b"), None);
        assert_eq!(store.get(&"a"), Some(&1));
        assert_eq!(store.get(&"c"), Some(&3));
    }

    /// Verifies: SEC-NET-051
    #[test]
    fn replacing_a_key_does_not_grow_the_store_and_counts_as_a_use() {
        let mut store = BoundedStore::new(2);
        store.insert("a", 1);
        store.insert("b", 2);
        assert_eq!(store.insert("a", 9), Some(1));
        assert_eq!(store.len(), 2);
        store.insert("c", 3);
        assert_eq!(store.get(&"b"), None);
        assert_eq!(store.get(&"a"), Some(&9));
        assert_eq!(store.get(&"c"), Some(&3));
    }

    /// Verifies: SEC-NET-051
    #[test]
    fn a_zero_capacity_store_holds_nothing() {
        let mut store = BoundedStore::new(0);
        assert_eq!(store.capacity(), 0);
        assert_eq!(store.insert("a", 1), None);
        assert_eq!(store.get(&"a"), None);
        assert_eq!(store.len(), 0);
        assert!(store.is_empty());
    }

    /// Verifies: SEC-NET-051, SEC-IAM-101
    #[test]
    fn check_store_on_a_zero_capacity_store_denies() {
        let rate_of = |key: LimitKey| match key {
            LimitKey::Ipv6Slash56(_) => rate(7_000, 1),
            LimitKey::Ipv6Slash48(_) => rate(2_000, 1),
            LimitKey::Global => rate(3_000, 1),
            _ => rate(1_000, 1),
        };
        let mut store = BoundedStore::new(0);
        let addr = IpAddr::V6(v6([0x2001, 0x0DB8, 0xAAAA, 0x0001, 0, 0, 0, 1]));
        assert_eq!(
            check_store(&mut store, &keys(None, addr), ts(START_MS), rate_of),
            Decision::Deny {
                retry_after_ms: 7_000
            }
        );
        assert_eq!(store.get(&LimitKey::Global), None);
        assert_eq!(
            store.get(&LimitKey::Ipv6Slash48(reference_prefix(
                v6([0x2001, 0x0DB8, 0xAAAA, 0x0001, 0, 0, 0, 1]),
                48
            ))),
            None
        );
        assert!(store.is_empty());
        assert_eq!(
            check_store(&mut store, &keys(None, addr), ts(START_MS + 1_000), rate_of),
            Decision::Deny {
                retry_after_ms: 7_000
            }
        );
    }

    /// Verifies: SEC-IAM-101
    #[test]
    fn a_store_smaller_than_one_requests_keys_denies_and_records_nothing() {
        let rate_of = |key: LimitKey| match key {
            LimitKey::Global => rate(3_000, 1),
            _ => rate(1_000, 1),
        };
        let mut store = BoundedStore::new(1);
        assert_eq!(
            check_store(
                &mut store,
                &keys(None, v4(203, 0, 113, 1)),
                ts(START_MS),
                rate_of
            ),
            Decision::Deny {
                retry_after_ms: 3_000
            }
        );
        assert!(store.is_empty());

        let mut store = BoundedStore::new(3);
        let addr = IpAddr::V6(v6([0x2001, 0x0DB8, 0xAAAA, 0x0001, 0, 0, 0, 1]));
        assert_eq!(
            check_store(&mut store, &keys(None, addr), ts(START_MS), rate_of),
            Decision::Deny {
                retry_after_ms: 3_000
            }
        );
        assert!(store.is_empty());
        assert_eq!(
            check_store(&mut store, &keys(None, addr), ts(START_MS + 5_000), rate_of),
            Decision::Deny {
                retry_after_ms: 3_000
            }
        );
        assert!(store.is_empty());

        let mut store = BoundedStore::new(2);
        let a_addr = Ipv4Addr::new(203, 0, 113, 1);
        assert_eq!(
            check_store(
                &mut store,
                &keys(None, IpAddr::V4(a_addr)),
                ts(START_MS),
                rate_of
            ),
            Decision::Allow
        );
        assert_eq!(store.len(), 2);
        assert_eq!(
            store.get(&LimitKey::Ipv4(a_addr)).copied(),
            Some(Tat {
                theoretical_ms: START_MS + 1_000,
                observed_ms: START_MS,
            })
        );
    }

    /// Verifies: SEC-NET-051
    #[test]
    fn a_repeat_grant_on_a_full_store_evicts_nothing() {
        let generous = rate(1, 255);
        let mut store = BoundedStore::new(4);
        for host in 1_u8..=3 {
            assert_eq!(
                check_store(
                    &mut store,
                    &keys(None, v4(203, 0, 113, host)),
                    ts(START_MS),
                    |_| generous
                ),
                Decision::Allow
            );
        }
        assert_eq!(store.len(), 4);
        assert_eq!(
            check_store(
                &mut store,
                &keys(None, v4(203, 0, 113, 3)),
                ts(START_MS),
                |_| generous
            ),
            Decision::Allow
        );
        assert_eq!(store.len(), 4);
        assert_eq!(
            store
                .get(&LimitKey::Ipv4(Ipv4Addr::new(203, 0, 113, 1)))
                .copied(),
            Some(Tat {
                theoretical_ms: START_MS + 1,
                observed_ms: START_MS,
            })
        );
        assert_eq!(
            store
                .get(&LimitKey::Ipv4(Ipv4Addr::new(203, 0, 113, 3)))
                .copied(),
            Some(Tat {
                theoretical_ms: START_MS + 2,
                observed_ms: START_MS,
            })
        );
    }

    /// Verifies: SEC-NET-051, SEC-NET-052
    #[test]
    fn ipv6_rotation_with_capacity_four_still_exhausts_the_slash_48() {
        rotating_slash_64s_exhaust_slash_48(4);
    }

    /// Verifies: SEC-NET-051, SEC-NET-052
    #[test]
    fn ipv6_rotation_with_capacity_five_still_exhausts_the_slash_48() {
        rotating_slash_64s_exhaust_slash_48(5);
    }

    fn rotating_slash_64s_exhaust_slash_48(capacity: usize) {
        let tight = rate(1_000, 2);
        let generous = rate(1, 255);
        let rate_of = |key: LimitKey| match key {
            LimitKey::Ipv6Slash48(_) => tight,
            _ => generous,
        };
        let mut store = BoundedStore::new(capacity);
        let inside_one = v6([0x2001, 0x0DB8, 0xAAAA, 0x0001, 0, 0, 0, 1]);
        let inside_two = v6([0x2001, 0x0DB8, 0xAAAA, 0x0002, 0, 0, 0, 1]);
        let inside_three = v6([0x2001, 0x0DB8, 0xAAAA, 0x0003, 0, 0, 0, 1]);
        let outside = v6([0x2001, 0x0DB8, 0xBBBB, 0x0001, 0, 0, 0, 1]);
        let slash48 = LimitKey::Ipv6Slash48(reference_prefix(inside_one, 48));
        assert_eq!(
            check_store(
                &mut store,
                &keys(None, IpAddr::V6(inside_one)),
                ts(START_MS),
                rate_of
            ),
            Decision::Allow
        );
        assert_eq!(store.len(), 4);
        assert_eq!(
            store.get(&slash48).copied(),
            Some(Tat {
                theoretical_ms: START_MS + 1_000,
                observed_ms: START_MS,
            })
        );
        assert_eq!(
            store.get(&LimitKey::Global).copied(),
            Some(Tat {
                theoretical_ms: START_MS + 1,
                observed_ms: START_MS,
            })
        );
        assert_eq!(
            check_store(
                &mut store,
                &keys(None, IpAddr::V6(inside_two)),
                ts(START_MS),
                rate_of
            ),
            Decision::Allow
        );
        assert_eq!(store.len(), capacity);
        assert_eq!(
            store.get(&slash48).copied(),
            Some(Tat {
                theoretical_ms: START_MS + 2_000,
                observed_ms: START_MS,
            })
        );
        assert_eq!(
            check_store(
                &mut store,
                &keys(None, IpAddr::V6(inside_three)),
                ts(START_MS),
                rate_of
            ),
            Decision::Deny {
                retry_after_ms: 1_000
            }
        );
        assert_eq!(
            store.get(&slash48).copied(),
            Some(Tat {
                theoretical_ms: START_MS + 2_000,
                observed_ms: START_MS,
            })
        );
        assert_eq!(
            check_store(
                &mut store,
                &keys(None, IpAddr::V6(outside)),
                ts(START_MS),
                rate_of
            ),
            Decision::Allow
        );
    }

    /// Verifies: SEC-NET-051, SEC-IAM-101, SEC-API-057
    #[test]
    fn flooding_unique_ipv4s_does_not_reset_the_global_ceiling() {
        let per_source = rate(1, 255);
        let global = rate(1_000, 5);
        let rate_of = |key: LimitKey| match key {
            LimitKey::Global => global,
            _ => per_source,
        };
        let mut store = BoundedStore::new(4);
        for host in 1_u8..=5 {
            assert_eq!(
                check_store(
                    &mut store,
                    &keys(None, v4(203, 0, 113, host)),
                    ts(START_MS),
                    rate_of
                ),
                Decision::Allow
            );
            assert_eq!(store.len(), usize::from(host).min(3) + 1);
            assert_eq!(
                store.get(&LimitKey::Global).copied(),
                Some(Tat {
                    theoretical_ms: START_MS + i64::from(host) * 1_000,
                    observed_ms: START_MS,
                })
            );
        }
        for host in 6_u8..=10 {
            assert_eq!(
                check_store(
                    &mut store,
                    &keys(None, v4(203, 0, 113, host)),
                    ts(START_MS),
                    rate_of
                ),
                Decision::Deny {
                    retry_after_ms: 1_000
                }
            );
            assert_eq!(
                store.get(&LimitKey::Global).copied(),
                Some(Tat {
                    theoretical_ms: START_MS + 5_000,
                    observed_ms: START_MS,
                })
            );
        }
        assert_eq!(
            check_store(
                &mut store,
                &keys(None, v4(198, 51, 100, 1)),
                ts(START_MS + 999),
                rate_of
            ),
            Decision::Deny { retry_after_ms: 1 }
        );
        assert_eq!(
            store.get(&LimitKey::Global).copied(),
            Some(Tat {
                theoretical_ms: START_MS + 5_000,
                observed_ms: START_MS,
            })
        );
    }

    /// Verifies: SEC-NET-051
    #[test]
    fn a_missing_key_is_absent_and_a_capacity_of_one_keeps_only_the_latest() {
        let mut store = BoundedStore::new(1);
        assert_eq!(store.get(&"missing"), None);
        store.insert("a", 1);
        store.insert("b", 2);
        assert_eq!(store.get(&"a"), None);
        assert_eq!(store.get(&"b"), Some(&2));
        assert_eq!(store.len(), 1);
    }

    fn ip_addr_strategy() -> impl Strategy<Value = IpAddr> {
        prop_oneof![
            any::<u32>().prop_map(|bits| IpAddr::V4(Ipv4Addr::from(bits))),
            any::<u128>().prop_map(|bits| IpAddr::V6(Ipv6Addr::from(bits))),
            any::<u32>().prop_map(|bits| IpAddr::V6(Ipv4Addr::from(bits).to_ipv6_mapped())),
        ]
    }

    proptest! {
        #[test]
        fn allowed_requests_never_exceed_burst_plus_rate_times_elapsed(
            interval_ms in 1_u64..200,
            burst in 1_u32..8,
            start in 0_i64..1_000_000,
            deltas in proptest::collection::vec(0_i64..400, 1..40),
        ) {
            let rate = rate(interval_ms, burst);
            let mut now = start;
            let mut state = None;
            let mut allowed = 0_u64;
            let first = now;
            for delta in deltas {
                now += delta;
                let (decision, tat) = check(state, ts(now), rate);
                state = Some(tat);
                if decision == Decision::Allow {
                    allowed += 1;
                }
            }
            let elapsed = u64::try_from(now - first).unwrap();
            let extra = elapsed / interval_ms;
            let ceiling = u64::from(burst) + extra;
            prop_assert!(allowed <= ceiling, "allowed={allowed} ceiling={ceiling}");
        }

        /// Verifies: SEC-NET-052
        #[test]
        fn every_generated_ipv6_address_maps_to_the_reference_prefixes(
            bits in any::<u128>(),
        ) {
            let addr = Ipv6Addr::from(bits);
            let derived = collected(keys(None, IpAddr::V6(addr)));
            prop_assert_eq!(derived[0], LimitKey::Ipv6Slash64(reference_prefix(addr, 64)));
            prop_assert_eq!(derived[1], LimitKey::Ipv6Slash56(reference_prefix(addr, 56)));
            prop_assert_eq!(derived[2], LimitKey::Ipv6Slash48(reference_prefix(addr, 48)));
            prop_assert_eq!(derived[3], LimitKey::Global);
            let same_slash_64 = collected(keys(None, IpAddr::V6(
                Ipv6Addr::from(bits & 0xFFFF_FFFF_FFFF_FFFF_0000_0000_0000_0000 | 1),
            )));
            prop_assert_eq!(derived[0], same_slash_64[0]);
        }

        /// Verifies: SEC-NET-052
        #[test]
        fn rotating_slash_64s_of_one_slash_48_share_the_slash_48(
            net48 in any::<u16>(),
            net64_a in any::<u16>(),
            net64_b in any::<u16>(),
            host_a in any::<u64>(),
            host_b in any::<u64>(),
        ) {
            // 0x2001_0DB8 (32 bits) plus net48 (16 bits) is the /48; net64
            // occupies the next 16 bits down to the /64 boundary; host is
            // the low 64 bits.
            let prefix48 = 0x2001_0DB8_u128 << 96 | u128::from(net48) << 80;
            let addr_a = Ipv6Addr::from(prefix48 | u128::from(net64_a) << 64 | u128::from(host_a));
            let addr_b = Ipv6Addr::from(prefix48 | u128::from(net64_b) << 64 | u128::from(host_b));
            let a = collected(keys(None, IpAddr::V6(addr_a)));
            let b = collected(keys(None, IpAddr::V6(addr_b)));
            prop_assert_eq!(a[2], b[2]);
            if net64_a == net64_b {
                prop_assert_eq!(a[0], b[0]);
            } else {
                prop_assert_ne!(a[0], b[0]);
            }
        }

        /// Verifies: SEC-NET-051
        #[test]
        fn the_store_never_exceeds_its_capacity(
            capacity in 0_usize..8,
            ops in proptest::collection::vec((any::<u8>(), any::<u8>(), any::<bool>()), 0..40),
        ) {
            let mut store = BoundedStore::new(capacity);
            for (key, value, do_get) in ops {
                if do_get {
                    let _ = store.get(&key);
                } else {
                    let _ = store.insert(key, value);
                }
                prop_assert!(store.len() <= capacity);
                prop_assert!(store.len() <= store.capacity());
                if capacity == 0 {
                    prop_assert!(store.is_empty());
                    prop_assert_eq!(store.get(&key), None);
                }
            }
        }

        /// Verifies: SEC-IAM-101, SEC-API-057
        #[test]
        fn the_global_key_never_allows_more_than_its_ceiling(
            n_sources in 1_usize..6,
            global_burst in 1_u32..4,
            interval_ms in 50_u64..200,
            deltas in proptest::collection::vec(0_i64..80, 1..24),
            source_picks in proptest::collection::vec(0_usize..6, 1..24),
        ) {
            let per_source = rate(1, 255);
            let global = rate(interval_ms, global_burst);
            let rate_of = |key: LimitKey| match key {
                LimitKey::Global => global,
                _ => per_source,
            };
            let mut store = BoundedStore::new(64);
            let sources: Vec<IpAddr> = (0..n_sources)
                .map(|i| v4(203, 0, 113, u8::try_from(i).unwrap() + 1))
                .collect();
            let mut now = START_MS;
            let first = now;
            let mut allowed = 0_u64;
            for (delta, pick) in deltas.into_iter().zip(source_picks.into_iter()) {
                now += delta;
                let idx = pick % n_sources;
                let addr = sources[idx];
                if check_store(&mut store, &keys(None, addr), ts(now), rate_of) == Decision::Allow {
                    allowed += 1;
                }
            }
            let elapsed = u64::try_from(now - first).unwrap();
            let ceiling = u64::from(global_burst) + elapsed / interval_ms;
            prop_assert!(allowed <= ceiling, "allowed={allowed} ceiling={ceiling}");
        }

        /// Verifies: SEC-API-057, SEC-NET-052
        #[test]
        fn a_principal_key_ignores_the_address(
            byte in any::<u8>(),
            addr in ip_addr_strategy(),
            other in ip_addr_strategy(),
        ) {
            let p = principal(byte);
            prop_assert_eq!(collected(keys(Some(p), addr)), collected(keys(Some(p), other)));
            prop_assert_eq!(
                collected(keys(Some(p), addr)),
                [LimitKey::Principal(p), LimitKey::Global]
            );
        }
    }
}
