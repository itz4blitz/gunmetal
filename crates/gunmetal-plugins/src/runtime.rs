//! Call limits, quotas and suspension.
//!
//! Each call gets a memory cap, a fuel budget and a deadline (SEC-EXT-023).
//! Outbound requests, concurrent calls, key-value bytes and log lines are
//! refused at the quota (SEC-EXT-033). Three failures in ten minutes suspend
//! the plugin, and the backoff grows on the next suspension (SEC-EXT-024).
//!
//! The caller passes time as milliseconds. This module does not read a clock
//! and does not link a WebAssembly runtime.

/// The longest interactive call, in seconds (SEC-EXT-023).
pub const INTERACTIVE_DEADLINE_SECS: u32 = 5;

/// The longest scheduled call, in seconds (SEC-EXT-023).
pub const SCHEDULED_DEADLINE_SECS: u32 = 60;

/// Fuel available to one call (SEC-EXT-023).
pub const FUEL: u64 = 1_000_000;

/// How long a failure still counts, in milliseconds (SEC-EXT-024).
pub const WINDOW_MS: i64 = 600_000;

/// Backoff applied by the first suspension, in seconds (SEC-EXT-024).
pub const FIRST_BACKOFF_SECS: u32 = 60;

/// The longest backoff, in seconds (SEC-EXT-024).
pub const MAX_BACKOFF_SECS: u32 = 3_600;

/// One mebibyte, in bytes.
const MIB: u64 = 1024 * 1024;

/// The most linear memory a call may be given, in mebibytes.
const MAX_MEMORY_MIB: u32 = 256;

/// Milliseconds in one second.
const MS_PER_SEC: u64 = 1_000;

/// One outbound minute, in milliseconds.
const MINUTE_MS: i64 = 60_000;

/// One log window, in milliseconds.
const LOG_WINDOW_MS: i64 = 1_000;

/// Log lines allowed in one window.
const MAX_LOG_LINES: u32 = 60;

/// Calls that may run at once.
const MAX_IN_FLIGHT: u32 = 2;

/// The most key-value store a manifest may declare, in mebibytes.
const MAX_KV_MIB: u32 = 16;

/// Failures inside the window that suspend the plugin.
const SUSPEND_AT: usize = 3;

/// Whether the call is interactive or a scheduled task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallKind {
    /// A person is waiting. The deadline is at most five seconds.
    Interactive,
    /// A scheduled task. The deadline is at most sixty seconds.
    Scheduled,
}

/// The caps for one call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallBudget {
    /// Linear memory, in bytes.
    pub memory_bytes: u64,
    /// Fuel the call may consume.
    pub fuel: u64,
    /// Wall-clock deadline, in milliseconds.
    pub deadline_ms: u64,
}

/// Which cap a call passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitKind {
    /// Linear memory grew past the cap.
    Memory,
    /// Fuel was consumed past the budget.
    Fuel,
    /// The deadline passed.
    Deadline,
}

/// The memory, fuel and deadline for one call.
///
/// Memory is `memory_mib` mebibytes, and anything above 256 is 256. Fuel is
/// [`FUEL`]. An interactive deadline is at most [`INTERACTIVE_DEADLINE_SECS`]
/// seconds; a scheduled deadline is at most [`SCHEDULED_DEADLINE_SECS`].
/// Zero stays zero.
#[must_use]
pub fn budget(memory_mib: u32, deadline_secs: u32, kind: CallKind) -> CallBudget {
    CallBudget {
        memory_bytes: mib_bytes(memory_mib.min(MAX_MEMORY_MIB)),
        fuel: FUEL,
        deadline_ms: u64::from(deadline_secs.min(deadline_cap(kind))).saturating_mul(MS_PER_SEC),
    }
}

/// What one call used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Charge {
    /// Linear memory observed, in bytes.
    pub memory_bytes: u64,
    /// Fuel consumed.
    pub fuel: u64,
    /// Time since the call started, in milliseconds.
    pub elapsed_ms: u64,
}

/// Whether `used` stayed inside `budget`.
///
/// Memory is checked first, then fuel, then the deadline. Equal to a cap is
/// inside it.
///
/// # Errors
///
/// [`LimitKind::Memory`], [`LimitKind::Fuel`] or [`LimitKind::Deadline`] for
/// the first cap `used` passed.
pub const fn charge(budget: &CallBudget, used: Charge) -> Result<(), LimitKind> {
    if used.memory_bytes > budget.memory_bytes {
        return Err(LimitKind::Memory);
    }
    if used.fuel > budget.fuel {
        return Err(LimitKind::Fuel);
    }
    if used.elapsed_ms > budget.deadline_ms {
        return Err(LimitKind::Deadline);
    }
    Ok(())
}

/// Host-enforced counters for one plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuotaState {
    /// Start of the current outbound minute, in milliseconds.
    pub minute_start_ms: i64,
    /// Outbound requests counted in that minute.
    pub requests_this_minute: u32,
    /// Calls that have begun and not ended.
    pub in_flight: u32,
    /// Key-value bytes stored.
    pub kv_bytes: u64,
    /// Start of the current log second, in milliseconds.
    pub log_second_ms: i64,
    /// Log lines counted in that second.
    pub log_lines_this_second: u32,
}

/// Which quota a plugin passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaKind {
    /// Too many outbound requests this minute.
    Outbound,
    /// Two calls are already running.
    Concurrent,
    /// The key-value store would pass its cap.
    Store,
    /// Too many log lines this second.
    Log,
}

/// Counts one outbound request, or refuses it.
///
/// A minute starts at `now_ms` when `minute_start_ms` and
/// `requests_this_minute` are both zero, or when `now_ms` is at least 60
/// seconds after `minute_start_ms`. The request number `per_minute` is
/// allowed; the next is not. The input is not changed.
///
/// # Errors
///
/// [`QuotaKind::Outbound`] when the minute already holds `per_minute`
/// requests. A `per_minute` of zero refuses the first request.
pub const fn admit_outbound(
    state: &QuotaState,
    now_ms: i64,
    per_minute: u32,
) -> Result<QuotaState, QuotaKind> {
    if fresh_count(state.minute_start_ms, state.requests_this_minute)
        || window_elapsed(state.minute_start_ms, now_ms, MINUTE_MS)
    {
        if per_minute == 0 {
            return Err(QuotaKind::Outbound);
        }
        return Ok(with_requests(state, now_ms, 1));
    }
    if state.requests_this_minute >= per_minute {
        return Err(QuotaKind::Outbound);
    }
    Ok(with_requests(
        state,
        state.minute_start_ms,
        state.requests_this_minute.saturating_add(1),
    ))
}

/// Reserves one of the two concurrent calls.
///
/// # Errors
///
/// [`QuotaKind::Concurrent`] when two calls are already in flight.
pub const fn begin_call(state: &QuotaState) -> Result<QuotaState, QuotaKind> {
    if state.in_flight >= MAX_IN_FLIGHT {
        return Err(QuotaKind::Concurrent);
    }
    Ok(with_in_flight(state, state.in_flight.saturating_add(1)))
}

/// Releases one in-flight call. Zero stays zero.
#[must_use]
pub const fn end_call(state: &QuotaState) -> QuotaState {
    with_in_flight(state, state.in_flight.saturating_sub(1))
}

/// Adds `more` bytes to the key-value store, or refuses them.
///
/// The cap is `kv_mib` mebibytes, and anything above 16 is 16. A total that
/// would pass the cap, including a store that adds nothing to a total
/// already over it, is refused. The input is not changed.
///
/// # Errors
///
/// [`QuotaKind::Store`] when the bytes would pass the cap.
pub fn store(state: &QuotaState, kv_mib: u32, more: u64) -> Result<QuotaState, QuotaKind> {
    let cap = mib_bytes(kv_mib.min(MAX_KV_MIB));
    match state.kv_bytes.checked_add(more) {
        Some(kv_bytes) if kv_bytes <= cap => Ok(with_kv(state, kv_bytes)),
        Some(_) | None => Err(QuotaKind::Store),
    }
}

/// Counts one log line, or refuses it.
///
/// A second starts at `now_ms` when `log_second_ms` and
/// `log_lines_this_second` are both zero, or when `now_ms` is at least 1000
/// ms after `log_second_ms`. Sixty lines in that second are allowed; the
/// sixty-first is not. The input is not changed.
///
/// # Errors
///
/// [`QuotaKind::Log`] when the second already holds 60 lines.
pub const fn log_line(state: &QuotaState, now_ms: i64) -> Result<QuotaState, QuotaKind> {
    if fresh_count(state.log_second_ms, state.log_lines_this_second)
        || window_elapsed(state.log_second_ms, now_ms, LOG_WINDOW_MS)
    {
        return Ok(with_logs(state, now_ms, 1));
    }
    if state.log_lines_this_second >= MAX_LOG_LINES {
        return Err(QuotaKind::Log);
    }
    Ok(with_logs(
        state,
        state.log_second_ms,
        state.log_lines_this_second.saturating_add(1),
    ))
}

/// Failure history and the suspension it produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Health {
    /// Failure timestamps still inside the ten-minute window, in the order
    /// they were recorded.
    pub failures_ms: Vec<i64>,
    /// When a call may start again, if the plugin is suspended.
    pub suspended_until_ms: Option<i64>,
    /// Backoff applied by the latest suspension, or zero before the first.
    pub backoff_secs: u32,
}

/// Records one failure and applies the ten-minute suspension rule.
///
/// Failures older than [`WINDOW_MS`] are dropped. While fewer than three
/// remain, the suspension and backoff are left unchanged. The third failure
/// in the window suspends the plugin until `now_ms` plus
/// [`FIRST_BACKOFF_SECS`] and stores that backoff. A later failure while that
/// suspension is still in force is recorded, and the deadline moves later
/// when `now_ms` plus the current backoff would end after it. The backoff
/// does not grow until the previous deadline has passed and three failures
/// are again in the window; that next suspension doubles it, up to
/// [`MAX_BACKOFF_SECS`].
#[must_use]
pub fn note_failure(health: &Health, now_ms: i64) -> Health {
    let failures_ms = kept_failures(&health.failures_ms, now_ms);
    if failures_ms.len() < SUSPEND_AT {
        return Health {
            failures_ms,
            suspended_until_ms: health.suspended_until_ms,
            backoff_secs: health.backoff_secs,
        };
    }
    if let Some(until) = still_suspended(health.suspended_until_ms, now_ms) {
        return Health {
            failures_ms,
            suspended_until_ms: Some(until.max(suspension_deadline(now_ms, health.backoff_secs))),
            backoff_secs: health.backoff_secs,
        };
    }
    let backoff_secs = next_backoff(health.backoff_secs);
    Health {
        failures_ms,
        suspended_until_ms: Some(suspension_deadline(now_ms, backoff_secs)),
        backoff_secs,
    }
}

/// Whether a call may start at `now_ms`.
///
/// # Errors
///
/// The suspension timestamp, when `now_ms` is still before it.
pub const fn may_call(health: &Health, now_ms: i64) -> Result<(), i64> {
    match health.suspended_until_ms {
        Some(until) if now_ms < until => Err(until),
        Some(_) | None => Ok(()),
    }
}

/// The deadline cap for `kind`.
const fn deadline_cap(kind: CallKind) -> u32 {
    match kind {
        CallKind::Interactive => INTERACTIVE_DEADLINE_SECS,
        CallKind::Scheduled => SCHEDULED_DEADLINE_SECS,
    }
}

/// `mib` as bytes. The product fits for every `u32` at or below the caps.
fn mib_bytes(mib: u32) -> u64 {
    u64::from(mib).saturating_mul(MIB)
}

/// Whether a zero start and a zero count have not opened a window yet.
const fn fresh_count(start_ms: i64, count: u32) -> bool {
    start_ms == 0 && count == 0
}

/// Whether `now_ms` is outside the window that began at `start_ms`.
///
/// An addition that does not fit is treated as still inside, so a counter
/// near the top of `i64` cannot wrap into a fresh allowance.
const fn window_elapsed(start_ms: i64, now_ms: i64, window_ms: i64) -> bool {
    match start_ms.checked_add(window_ms) {
        Some(end) => now_ms >= end,
        None => false,
    }
}

/// `state` with a new outbound minute.
const fn with_requests(
    state: &QuotaState,
    minute_start_ms: i64,
    requests_this_minute: u32,
) -> QuotaState {
    QuotaState {
        minute_start_ms,
        requests_this_minute,
        in_flight: state.in_flight,
        kv_bytes: state.kv_bytes,
        log_second_ms: state.log_second_ms,
        log_lines_this_second: state.log_lines_this_second,
    }
}

/// `state` with a new in-flight count.
const fn with_in_flight(state: &QuotaState, in_flight: u32) -> QuotaState {
    QuotaState {
        minute_start_ms: state.minute_start_ms,
        requests_this_minute: state.requests_this_minute,
        in_flight,
        kv_bytes: state.kv_bytes,
        log_second_ms: state.log_second_ms,
        log_lines_this_second: state.log_lines_this_second,
    }
}

/// `state` with a new key-value total.
const fn with_kv(state: &QuotaState, kv_bytes: u64) -> QuotaState {
    QuotaState {
        minute_start_ms: state.minute_start_ms,
        requests_this_minute: state.requests_this_minute,
        in_flight: state.in_flight,
        kv_bytes,
        log_second_ms: state.log_second_ms,
        log_lines_this_second: state.log_lines_this_second,
    }
}

/// `state` with a new log window.
const fn with_logs(
    state: &QuotaState,
    log_second_ms: i64,
    log_lines_this_second: u32,
) -> QuotaState {
    QuotaState {
        minute_start_ms: state.minute_start_ms,
        requests_this_minute: state.requests_this_minute,
        in_flight: state.in_flight,
        kv_bytes: state.kv_bytes,
        log_second_ms,
        log_lines_this_second,
    }
}

/// Failures still inside the window, then `now_ms`.
fn kept_failures(previous: &[i64], now_ms: i64) -> Vec<i64> {
    let bound = now_ms.checked_sub(WINDOW_MS);
    let mut failures_ms: Vec<i64> = previous
        .iter()
        .copied()
        .filter(|stamp| stamp_in_window(*stamp, bound))
        .collect();
    failures_ms.push(now_ms);
    failures_ms
}

/// Whether `stamp` is still inside the window.
///
/// A bound that cannot be represented is below every timestamp, so every
/// recorded failure stays inside.
const fn stamp_in_window(stamp: i64, bound: Option<i64>) -> bool {
    match bound {
        Some(start) => stamp >= start,
        None => true,
    }
}

/// The suspension end, if `now_ms` is still before it.
const fn still_suspended(until: Option<i64>, now_ms: i64) -> Option<i64> {
    match until {
        Some(end) if now_ms < end => Some(end),
        Some(_) | None => None,
    }
}

/// Backoff for a new suspension. Zero starts at [`FIRST_BACKOFF_SECS`].
fn next_backoff(current: u32) -> u32 {
    let grown = if current == 0 {
        FIRST_BACKOFF_SECS
    } else {
        current.saturating_mul(2)
    };
    grown.min(MAX_BACKOFF_SECS)
}

/// `now_ms` plus `backoff_secs`, saturating at the top of `i64`.
fn suspension_deadline(now_ms: i64, backoff_secs: u32) -> i64 {
    now_ms.saturating_add(i64::from(backoff_secs).saturating_mul(1_000))
}

#[cfg(test)]
mod tests {
    use super::{
        CallBudget, CallKind, Charge, FIRST_BACKOFF_SECS, FUEL, Health, INTERACTIVE_DEADLINE_SECS,
        LimitKind, MAX_BACKOFF_SECS, QuotaKind, QuotaState, SCHEDULED_DEADLINE_SECS, WINDOW_MS,
        admit_outbound, begin_call, budget, charge, end_call, log_line, may_call, note_failure,
        store,
    };

    fn quota(
        minute_start_ms: i64,
        requests_this_minute: u32,
        in_flight: u32,
        kv_bytes: u64,
        log_second_ms: i64,
        log_lines_this_second: u32,
    ) -> QuotaState {
        QuotaState {
            minute_start_ms,
            requests_this_minute,
            in_flight,
            kv_bytes,
            log_second_ms,
            log_lines_this_second,
        }
    }

    fn health(failures_ms: Vec<i64>, suspended_until_ms: Option<i64>, backoff_secs: u32) -> Health {
        Health {
            failures_ms,
            suspended_until_ms,
            backoff_secs,
        }
    }

    /// Verifies: SEC-EXT-023, SEC-EXT-024
    #[test]
    fn the_published_limits_are_these_numbers() {
        assert_eq!(INTERACTIVE_DEADLINE_SECS, 5);
        assert_eq!(SCHEDULED_DEADLINE_SECS, 60);
        assert_eq!(FUEL, 1_000_000);
        assert_eq!(WINDOW_MS, 600_000);
        assert_eq!(FIRST_BACKOFF_SECS, 60);
        assert_eq!(MAX_BACKOFF_SECS, 3_600);
    }

    /// Verifies: SEC-EXT-023
    #[test]
    fn an_interactive_budget_caps_memory_and_the_five_second_deadline() {
        assert_eq!(
            budget(32, 5, CallKind::Interactive),
            CallBudget {
                memory_bytes: 33_554_432,
                fuel: 1_000_000,
                deadline_ms: 5_000,
            }
        );
        assert_eq!(
            budget(64, 5, CallKind::Interactive),
            CallBudget {
                memory_bytes: 67_108_864,
                fuel: 1_000_000,
                deadline_ms: 5_000,
            }
        );
        assert_eq!(
            budget(0, 0, CallKind::Interactive),
            CallBudget {
                memory_bytes: 0,
                fuel: 1_000_000,
                deadline_ms: 0,
            }
        );
        assert_eq!(
            budget(4, 4, CallKind::Interactive),
            CallBudget {
                memory_bytes: 4_194_304,
                fuel: 1_000_000,
                deadline_ms: 4_000,
            }
        );
        assert_eq!(
            budget(32, 6, CallKind::Interactive),
            CallBudget {
                memory_bytes: 33_554_432,
                fuel: 1_000_000,
                deadline_ms: 5_000,
            }
        );
        assert_eq!(
            budget(32, 60, CallKind::Interactive),
            CallBudget {
                memory_bytes: 33_554_432,
                fuel: 1_000_000,
                deadline_ms: 5_000,
            }
        );
        assert_eq!(
            budget(u32::MAX, u32::MAX, CallKind::Interactive),
            CallBudget {
                memory_bytes: 268_435_456,
                fuel: 1_000_000,
                deadline_ms: 5_000,
            }
        );
        assert_eq!(
            budget(255, 5, CallKind::Interactive),
            CallBudget {
                memory_bytes: 267_386_880,
                fuel: 1_000_000,
                deadline_ms: 5_000,
            }
        );
        assert_eq!(
            budget(256, 5, CallKind::Interactive),
            CallBudget {
                memory_bytes: 268_435_456,
                fuel: 1_000_000,
                deadline_ms: 5_000,
            }
        );
        assert_eq!(
            budget(257, 5, CallKind::Interactive),
            CallBudget {
                memory_bytes: 268_435_456,
                fuel: 1_000_000,
                deadline_ms: 5_000,
            }
        );
    }

    /// Verifies: SEC-EXT-023
    #[test]
    fn a_scheduled_budget_caps_the_deadline_at_sixty_seconds() {
        assert_eq!(
            budget(32, 90, CallKind::Scheduled),
            CallBudget {
                memory_bytes: 33_554_432,
                fuel: 1_000_000,
                deadline_ms: 60_000,
            }
        );
        assert_eq!(
            budget(32, 60, CallKind::Scheduled),
            CallBudget {
                memory_bytes: 33_554_432,
                fuel: 1_000_000,
                deadline_ms: 60_000,
            }
        );
        assert_eq!(
            budget(32, 59, CallKind::Scheduled),
            CallBudget {
                memory_bytes: 33_554_432,
                fuel: 1_000_000,
                deadline_ms: 59_000,
            }
        );
        assert_eq!(
            budget(32, 61, CallKind::Scheduled),
            CallBudget {
                memory_bytes: 33_554_432,
                fuel: 1_000_000,
                deadline_ms: 60_000,
            }
        );
        assert_eq!(
            budget(32, 0, CallKind::Scheduled),
            CallBudget {
                memory_bytes: 33_554_432,
                fuel: 1_000_000,
                deadline_ms: 0,
            }
        );
        assert_eq!(
            budget(32, 5, CallKind::Scheduled),
            CallBudget {
                memory_bytes: 33_554_432,
                fuel: 1_000_000,
                deadline_ms: 5_000,
            }
        );
        assert_eq!(
            budget(257, 90, CallKind::Scheduled),
            CallBudget {
                memory_bytes: 268_435_456,
                fuel: 1_000_000,
                deadline_ms: 60_000,
            }
        );
        assert_eq!(
            budget(u32::MAX, u32::MAX, CallKind::Scheduled),
            CallBudget {
                memory_bytes: 268_435_456,
                fuel: 1_000_000,
                deadline_ms: 60_000,
            }
        );
    }

    /// Verifies: SEC-EXT-023
    #[test]
    fn charge_accepts_the_cap_and_reports_the_earliest_limit() {
        let limits = CallBudget {
            memory_bytes: 100,
            fuel: 100,
            deadline_ms: 100,
        };
        assert_eq!(
            charge(
                &limits,
                Charge {
                    memory_bytes: 100,
                    fuel: 100,
                    elapsed_ms: 100,
                }
            ),
            Ok(())
        );
        assert_eq!(
            charge(
                &limits,
                Charge {
                    memory_bytes: 99,
                    fuel: 99,
                    elapsed_ms: 99,
                }
            ),
            Ok(())
        );
        assert_eq!(
            charge(
                &limits,
                Charge {
                    memory_bytes: 101,
                    fuel: 101,
                    elapsed_ms: 101,
                }
            ),
            Err(LimitKind::Memory)
        );
        assert_eq!(
            charge(
                &limits,
                Charge {
                    memory_bytes: 100,
                    fuel: 101,
                    elapsed_ms: 101,
                }
            ),
            Err(LimitKind::Fuel)
        );
        assert_eq!(
            charge(
                &limits,
                Charge {
                    memory_bytes: 100,
                    fuel: 100,
                    elapsed_ms: 101,
                }
            ),
            Err(LimitKind::Deadline)
        );
        assert_eq!(
            charge(
                &limits,
                Charge {
                    memory_bytes: 0,
                    fuel: 101,
                    elapsed_ms: 0,
                }
            ),
            Err(LimitKind::Fuel)
        );
    }

    /// Verifies: SEC-EXT-023
    #[test]
    fn a_zero_deadline_rejects_any_positive_elapsed_time() {
        let limits = CallBudget {
            memory_bytes: 33_554_432,
            fuel: 1_000_000,
            deadline_ms: 0,
        };
        assert_eq!(
            budget(32, 0, CallKind::Interactive),
            CallBudget {
                memory_bytes: 33_554_432,
                fuel: 1_000_000,
                deadline_ms: 0,
            }
        );
        assert_eq!(
            charge(
                &limits,
                Charge {
                    memory_bytes: 0,
                    fuel: 0,
                    elapsed_ms: 0,
                }
            ),
            Ok(())
        );
        assert_eq!(
            charge(
                &limits,
                Charge {
                    memory_bytes: 0,
                    fuel: 0,
                    elapsed_ms: 1,
                }
            ),
            Err(LimitKind::Deadline)
        );
        let closed = CallBudget {
            memory_bytes: 0,
            fuel: 0,
            deadline_ms: 0,
        };
        assert_eq!(
            charge(
                &closed,
                Charge {
                    memory_bytes: 1,
                    fuel: 1,
                    elapsed_ms: 1,
                }
            ),
            Err(LimitKind::Memory)
        );
    }

    /// Verifies: SEC-EXT-033
    #[test]
    fn outbound_allows_sixty_in_a_minute_and_refuses_the_sixty_first() {
        let minute = quota(1_000, 59, 1, 8, 9, 2);
        assert_eq!(
            admit_outbound(&minute, 1_000, 60),
            Ok(quota(1_000, 60, 1, 8, 9, 2))
        );
        let full = quota(1_000, 60, 1, 8, 9, 2);
        let before = full.clone();
        assert_eq!(admit_outbound(&full, 60_999, 60), Err(QuotaKind::Outbound));
        assert_eq!(full, before);
        assert_eq!(
            admit_outbound(&full, 61_000, 60),
            Ok(quota(61_000, 1, 1, 8, 9, 2))
        );
    }

    /// Verifies: SEC-EXT-033
    #[test]
    fn outbound_opens_a_minute_on_the_zero_state_and_on_the_boundary() {
        let idle = quota(0, 0, 4, 8, 9, 2);
        let before = idle.clone();
        assert_eq!(
            admit_outbound(&idle, 30_000, 60),
            Ok(quota(30_000, 1, 4, 8, 9, 2))
        );
        assert_eq!(idle, before);
        assert_eq!(
            admit_outbound(&quota(0, 5, 4, 8, 9, 2), 1_000, 60),
            Ok(quota(0, 6, 4, 8, 9, 2))
        );
        assert_eq!(
            admit_outbound(&quota(5_000, 0, 4, 8, 9, 2), 6_000, 60),
            Ok(quota(5_000, 1, 4, 8, 9, 2))
        );
        assert_eq!(
            admit_outbound(&quota(100_000, 10, 4, 8, 9, 2), 0, 60),
            Ok(quota(100_000, 11, 4, 8, 9, 2))
        );
        assert_eq!(
            admit_outbound(&quota(-100_000, 59, 4, 8, 9, 2), -40_001, 60),
            Ok(quota(-100_000, 60, 4, 8, 9, 2))
        );
        assert_eq!(
            admit_outbound(&quota(-100_000, 60, 4, 8, 9, 2), -40_000, 60),
            Ok(quota(-40_000, 1, 4, 8, 9, 2))
        );
    }

    /// Verifies: SEC-EXT-033
    #[test]
    fn outbound_uses_the_callers_limit_and_does_not_panic_on_overflow() {
        let open = quota(5_000, 0, 4, 8, 9, 2);
        let before = open.clone();
        assert_eq!(admit_outbound(&open, 6_000, 0), Err(QuotaKind::Outbound));
        assert_eq!(open, before);
        assert_eq!(
            admit_outbound(&quota(0, 0, 4, 8, 9, 2), 30_000, 0),
            Err(QuotaKind::Outbound)
        );
        assert_eq!(
            admit_outbound(&quota(1_000, 60, 4, 8, 9, 2), 61_000, 0),
            Err(QuotaKind::Outbound)
        );
        assert_eq!(
            admit_outbound(&quota(5_000, 1, 4, 8, 9, 2), 5_000, 2),
            Ok(quota(5_000, 2, 4, 8, 9, 2))
        );
        assert_eq!(
            admit_outbound(&quota(5_000, 2, 4, 8, 9, 2), 5_000, 2),
            Err(QuotaKind::Outbound)
        );
        assert_eq!(
            admit_outbound(&quota(5_000, 1, 4, 8, 9, 2), 5_000, 1),
            Err(QuotaKind::Outbound)
        );
        assert_eq!(
            admit_outbound(&quota(1, u32::MAX - 1, 4, 8, 9, 2), 1, u32::MAX),
            Ok(quota(1, u32::MAX, 4, 8, 9, 2))
        );
        assert_eq!(
            admit_outbound(&quota(1, u32::MAX, 4, 8, 9, 2), 1, u32::MAX),
            Err(QuotaKind::Outbound)
        );
        assert_eq!(
            admit_outbound(&quota(i64::MAX - 1_000, 4, 3, 8, 9, 2), i64::MAX, 60),
            Ok(quota(i64::MAX - 1_000, 5, 3, 8, 9, 2))
        );
    }

    /// Verifies: SEC-EXT-033
    #[test]
    fn two_concurrent_calls_are_allowed_and_the_third_is_refused() {
        assert_eq!(
            begin_call(&quota(1, 9, 0, 8, 3, 4)),
            Ok(quota(1, 9, 1, 8, 3, 4))
        );
        assert_eq!(
            begin_call(&quota(1, 9, 1, 8, 3, 4)),
            Ok(quota(1, 9, 2, 8, 3, 4))
        );
        let busy = quota(1, 9, 2, 8, 3, 4);
        let before = busy.clone();
        assert_eq!(begin_call(&busy), Err(QuotaKind::Concurrent));
        assert_eq!(busy, before);
        assert_eq!(
            begin_call(&quota(1, 9, 3, 8, 3, 4)),
            Err(QuotaKind::Concurrent)
        );
        assert_eq!(
            begin_call(&quota(1, 9, u32::MAX, 8, 3, 4)),
            Err(QuotaKind::Concurrent)
        );
    }

    /// Verifies: SEC-EXT-033
    #[test]
    fn ending_a_call_stops_at_zero() {
        assert_eq!(end_call(&quota(1, 9, 2, 8, 3, 4)), quota(1, 9, 1, 8, 3, 4));
        assert_eq!(end_call(&quota(1, 9, 1, 8, 3, 4)), quota(1, 9, 0, 8, 3, 4));
        assert_eq!(end_call(&quota(1, 9, 0, 8, 3, 4)), quota(1, 9, 0, 8, 3, 4));
    }

    /// Verifies: SEC-EXT-033
    #[test]
    fn the_store_accepts_bytes_up_to_the_cap_and_sixteen_mebibytes() {
        assert_eq!(
            store(&quota(1, 2, 3, 0, 4, 5), 1, 1_048_576),
            Ok(quota(1, 2, 3, 1_048_576, 4, 5))
        );
        assert_eq!(
            store(&quota(1, 2, 3, 1_048_575, 4, 5), 1, 1),
            Ok(quota(1, 2, 3, 1_048_576, 4, 5))
        );
        let full = quota(1, 2, 3, 1_048_576, 4, 5);
        let before = full.clone();
        assert_eq!(store(&full, 1, 1), Err(QuotaKind::Store));
        assert_eq!(full, before);
        assert_eq!(
            store(&quota(1, 2, 3, 1_048_577, 4, 5), 1, 0),
            Err(QuotaKind::Store)
        );
        assert_eq!(
            store(&quota(1, 2, 3, 0, 4, 5), 0, 0),
            Ok(quota(1, 2, 3, 0, 4, 5))
        );
        assert_eq!(store(&quota(1, 2, 3, 0, 4, 5), 0, 1), Err(QuotaKind::Store));
        assert_eq!(
            store(&quota(1, 2, 3, 0, 4, 5), 16, 16_777_216),
            Ok(quota(1, 2, 3, 16_777_216, 4, 5))
        );
        assert_eq!(
            store(&quota(1, 2, 3, 0, 4, 5), 16, 16_777_217),
            Err(QuotaKind::Store)
        );
        assert_eq!(
            store(&quota(1, 2, 3, 0, 4, 5), 17, 16_777_216),
            Ok(quota(1, 2, 3, 16_777_216, 4, 5))
        );
        assert_eq!(
            store(&quota(1, 2, 3, 0, 4, 5), 17, 16_777_217),
            Err(QuotaKind::Store)
        );
        assert_eq!(
            store(&quota(1, 2, 3, 0, 4, 5), u32::MAX, 16_777_216),
            Ok(quota(1, 2, 3, 16_777_216, 4, 5))
        );
        assert_eq!(
            store(&quota(1, 2, 3, u64::MAX, 4, 5), 16, 1),
            Err(QuotaKind::Store)
        );
    }

    /// Verifies: SEC-EXT-033
    #[test]
    fn sixty_log_lines_are_allowed_in_one_second() {
        assert_eq!(
            log_line(&quota(1, 2, 3, 4, 0, 0), 500),
            Ok(quota(1, 2, 3, 4, 500, 1))
        );
        assert_eq!(
            log_line(&quota(1, 2, 3, 4, 0, 5), 500),
            Ok(quota(1, 2, 3, 4, 0, 6))
        );
        assert_eq!(
            log_line(&quota(1, 2, 3, 4, 5_000, 0), 5_500),
            Ok(quota(1, 2, 3, 4, 5_000, 1))
        );
        assert_eq!(
            log_line(&quota(1, 2, 3, 4, 5_000, 59), 5_999),
            Ok(quota(1, 2, 3, 4, 5_000, 60))
        );
        let full = quota(1, 2, 3, 4, 5_000, 60);
        let before = full.clone();
        assert_eq!(log_line(&full, 5_999), Err(QuotaKind::Log));
        assert_eq!(full, before);
        assert_eq!(
            log_line(&quota(1, 2, 3, 4, 5_000, 3), 5_999),
            Ok(quota(1, 2, 3, 4, 5_000, 4))
        );
        assert_eq!(
            log_line(&quota(1, 2, 3, 4, 5_000, 60), 6_000),
            Ok(quota(1, 2, 3, 4, 6_000, 1))
        );
        assert_eq!(
            log_line(&quota(1, 2, 3, 4, 5_000, 3), 4_000),
            Ok(quota(1, 2, 3, 4, 5_000, 4))
        );
        assert_eq!(
            log_line(&quota(1, 2, 3, 4, i64::MAX - 100, 3), i64::MAX),
            Ok(quota(1, 2, 3, 4, i64::MAX - 100, 4))
        );
    }

    /// Verifies: SEC-EXT-024
    #[test]
    fn the_third_failure_in_ten_minutes_suspends_for_sixty_seconds() {
        let idle = health(vec![], None, 0);
        let before = idle.clone();
        let once = note_failure(&idle, 0);
        assert_eq!(idle, before);
        assert_eq!(once, health(vec![0], None, 0));
        let twice = note_failure(&once, 1_000);
        assert_eq!(twice, health(vec![0, 1_000], None, 0));
        assert_eq!(
            note_failure(&twice, 2_000),
            health(vec![0, 1_000, 2_000], Some(62_000), 60)
        );
    }

    /// Verifies: SEC-EXT-024
    #[test]
    fn a_failure_outside_the_ten_minute_window_does_not_suspend() {
        let once = note_failure(&health(vec![], None, 0), 0);
        assert_eq!(note_failure(&once, 700_000), health(vec![700_000], None, 0));
        assert_eq!(
            note_failure(&health(vec![100_000, 200_000], None, 0), 700_000),
            health(vec![100_000, 200_000, 700_000], Some(760_000), 60)
        );
        assert_eq!(
            note_failure(&health(vec![99_999, 200_000], None, 0), 700_000),
            health(vec![200_000, 700_000], None, 0)
        );
        assert_eq!(
            note_failure(&health(vec![0, 500_000, 100], Some(50), 60), 700_000),
            health(vec![500_000, 700_000], Some(50), 60)
        );
        assert_eq!(
            note_failure(&health(vec![1_000, 800_000], None, 0), 700_000),
            health(vec![800_000, 700_000], None, 0)
        );
        assert_eq!(
            note_failure(&health(vec![-700_000, -100_000], None, 0), -50_000),
            health(vec![-100_000, -50_000], None, 0)
        );
        assert_eq!(
            note_failure(&health(vec![0], None, 0), i64::MIN),
            health(vec![0, i64::MIN], None, 0)
        );
    }

    /// Verifies: SEC-EXT-024
    #[test]
    fn a_failure_while_suspended_is_kept_and_can_extend_the_deadline() {
        let suspended = health(vec![0, 1_000, 2_000], Some(62_000), 60);
        let before = suspended.clone();
        assert_eq!(
            note_failure(&suspended, 50_000),
            health(vec![0, 1_000, 2_000, 50_000], Some(110_000), 60)
        );
        assert_eq!(suspended, before);
        assert_eq!(
            note_failure(&suspended, 1_500),
            health(vec![0, 1_000, 2_000, 1_500], Some(62_000), 60)
        );
        assert_eq!(
            note_failure(
                &health(vec![i64::MAX - 10, i64::MAX - 9], Some(i64::MAX), 60),
                i64::MAX - 1
            ),
            health(
                vec![i64::MAX - 10, i64::MAX - 9, i64::MAX - 1],
                Some(i64::MAX),
                60
            )
        );
    }

    /// Verifies: SEC-EXT-024
    #[test]
    fn the_next_suspension_doubles_backoff_and_the_hour_cap_holds() {
        let suspended = health(vec![0, 1_000, 2_000], Some(62_000), 60);
        assert_eq!(
            note_failure(&suspended, 62_000),
            health(vec![0, 1_000, 2_000, 62_000], Some(182_000), 120)
        );
        let aged = note_failure(&suspended, 700_000);
        assert_eq!(aged, health(vec![700_000], Some(62_000), 60));
        let still = note_failure(&aged, 701_000);
        assert_eq!(still, health(vec![700_000, 701_000], Some(62_000), 60));
        assert_eq!(
            note_failure(&still, 702_000),
            health(vec![700_000, 701_000, 702_000], Some(822_000), 120)
        );
        assert_eq!(
            note_failure(
                &health(vec![1_000_000, 1_001_000], Some(1_000_000), 1_920),
                1_002_000
            ),
            health(
                vec![1_000_000, 1_001_000, 1_002_000],
                Some(4_602_000),
                3_600
            )
        );
        assert_eq!(
            note_failure(
                &health(vec![5_000_000, 5_001_000], Some(4_602_000), 3_600),
                5_002_000
            ),
            health(
                vec![5_000_000, 5_001_000, 5_002_000],
                Some(8_602_000),
                3_600
            )
        );
        assert_eq!(
            note_failure(&health(vec![0, 1_000], None, u32::MAX), 2_000),
            health(vec![0, 1_000, 2_000], Some(3_602_000), 3_600)
        );
        assert_eq!(
            note_failure(&health(vec![i64::MAX - 2, i64::MAX - 1], None, 0), i64::MAX),
            health(
                vec![i64::MAX - 2, i64::MAX - 1, i64::MAX],
                Some(i64::MAX),
                60
            )
        );
    }

    /// Verifies: SEC-EXT-024
    #[test]
    fn a_call_waits_until_the_suspension_timestamp() {
        assert_eq!(may_call(&health(vec![0, 1, 2], None, 0), 0), Ok(()));
        let held = health(vec![0, 1_000, 2_000], Some(62_000), 60);
        assert_eq!(may_call(&held, 61_999), Err(62_000));
        assert_eq!(may_call(&held, 62_000), Ok(()));
        assert_eq!(may_call(&held, 62_001), Ok(()));
        assert_eq!(
            may_call(&health(vec![], Some(i64::MAX), 3_600), i64::MAX - 1),
            Err(i64::MAX)
        );
        assert_eq!(
            may_call(&health(vec![], Some(i64::MAX), 3_600), i64::MAX),
            Ok(())
        );
        assert_eq!(
            may_call(&health(vec![], Some(i64::MIN), 60), i64::MIN),
            Ok(())
        );
    }
}
