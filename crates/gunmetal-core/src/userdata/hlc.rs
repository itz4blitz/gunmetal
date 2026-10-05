//! The hybrid logical clock every user event carries (ADR 3, section 4).
//!
//! A clock is wall time in milliseconds and a logical counter (Kulkarni and
//! others, "Logical Physical Clocks", 2014). It never runs backwards, every
//! value a node issues is later than every value it has issued or received,
//! and it stays close to wall time, so "the latest event wins" means what a
//! person expects even when devices merge offline.
//!
//! Clocks order by wall time, then by the logical counter. When the counter
//! is full it carries into the wall time, one millisecond on, instead of
//! failing, so a burst of events or a peer that sends a full counter cannot
//! stop a node issuing clocks.
//!
//! A device cannot win every "latest wins" by setting its clock ahead: the
//! server refuses a client clock more than [`MAX_SKEW_MS`] ahead of its own
//! wall time, and [`Hlc::receive`] checks that before the server's clock
//! moves (ADR 3, section 6).

use crate::problem::{Arg, Describe, Problem, ProblemCode};

/// How far ahead of the server's wall time, in milliseconds, a client's
/// clock may be: five minutes.
///
/// Devices that keep network time are within a second or two of the
/// server; five minutes also tolerates a device whose clock was set by hand.
/// A clock further ahead is refused rather than adopted, and so is a clock
/// whose adoption would leave the server's clock at the bound with a full
/// counter ([`Hlc::receive`]). So the clock the server adopts on a
/// client's word is never past the bound, and neither is the next clock
/// it issues.
///
/// That is all the bound promises. The counter still ticks once for every
/// later event at that wall time, whoever sent it, and a full counter
/// carries into the wall time. A clock adopted at the bound with its
/// counter a few ticks short of full is therefore filled by that many
/// more events and carried one millisecond past the bound by the one
/// after them, and each millisecond after that takes 2^32 events. The
/// server package that enforces the bound registers it in the limits
/// register (SEC-STD-030).
pub const MAX_SKEW_MS: u64 = 300_000;

/// A hybrid logical clock value: wall time in milliseconds since the Unix
/// epoch, UTC, and a logical counter for events within one millisecond.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hlc {
    /// Wall time in milliseconds since the Unix epoch, UTC.
    wall_ms: u64,
    /// Events issued or received at this wall time before this one.
    logical: u32,
}

impl Hlc {
    /// The earliest clock, before any event.
    pub const ZERO: Self = Self::new(0, 0);

    /// The clock with this wall time and logical counter.
    #[must_use]
    pub const fn new(wall_ms: u64, logical: u32) -> Self {
        Self { wall_ms, logical }
    }

    /// Wall time in milliseconds since the Unix epoch, UTC.
    #[must_use]
    pub const fn wall_ms(self) -> u64 {
        self.wall_ms
    }

    /// The logical counter.
    #[must_use]
    pub const fn logical(self) -> u32 {
        self.logical
    }

    /// The clock for an event this node authors at wall time `now_ms`:
    /// later than `self`, and at `now_ms` whenever `now_ms` is later.
    ///
    /// # Errors
    ///
    /// Returns [`ClockError::Exhausted`] only when `self` is the last clock
    /// there is, which no wall time reaches.
    pub fn send(self, now_ms: u64) -> Result<Self, ClockError> {
        self.advance(now_ms)
    }

    /// The clock after this node receives an event stamped `remote` at wall
    /// time `now_ms`: later than both `self` and `remote`.
    ///
    /// # Errors
    ///
    /// Returns [`ClockError::Ahead`] when `remote` is more than
    /// [`MAX_SKEW_MS`] ahead of `now_ms`, or when adopting `remote` would
    /// leave this node's clock past that bound, or at it with a full
    /// counter, which the next clock issued would carry past it; `self`
    /// is then unchanged, and the event is refused. A clock that is this
    /// node's own, later than `remote`, is not held to the bound. Returns
    /// [`ClockError::Exhausted`] only when the later of the two clocks is
    /// the last clock there is.
    pub fn receive(self, remote: Self, now_ms: u64) -> Result<Self, ClockError> {
        let bound_ms = now_ms.saturating_add(MAX_SKEW_MS);
        if remote.wall_ms > bound_ms {
            return Err(ClockError::Ahead {
                wall_ms: remote.wall_ms,
                bound_ms,
            });
        }
        let next = self.max(remote).advance(now_ms)?;
        // When the remote clock is the one adopted, the clock it leaves must
        // have a tick to spare at the bound. A clock past the bound has
        // carried a full counter over it already, and a clock at the bound
        // with a full counter would on the next clock issued, so both are
        // refused as Ahead.
        if remote >= self && next >= Self::new(bound_ms, u32::MAX) {
            return Err(ClockError::Ahead {
                wall_ms: remote.wall_ms,
                bound_ms,
            });
        }
        Ok(next)
    }

    /// The next clock after `self` at wall time `now_ms`: `now_ms` with a
    /// zero counter when that is later, and otherwise one tick on.
    fn advance(self, now_ms: u64) -> Result<Self, ClockError> {
        if now_ms > self.wall_ms {
            return Ok(Self::new(now_ms, 0));
        }
        if let Some(logical) = self.logical.checked_add(1) {
            return Ok(Self::new(self.wall_ms, logical));
        }
        // The counter is full: carry into the wall time.
        self.wall_ms
            .checked_add(1)
            .map(|wall_ms| Self::new(wall_ms, 0))
            .ok_or(ClockError::Exhausted)
    }
}

/// Why a clock could not be issued.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockError {
    /// A received clock is further ahead of this node's wall time than
    /// [`MAX_SKEW_MS`] allows.
    Ahead {
        /// The received clock's wall time.
        wall_ms: u64,
        /// The latest wall time this node accepts now.
        bound_ms: u64,
    },
    /// The clock is the last one there is: both the wall time and the
    /// counter are at their maximum.
    Exhausted,
}

impl Describe for ClockError {
    /// Both mean a clock set too far ahead; the device is asked to check
    /// its date and time.
    fn problem(&self) -> Problem {
        let args = match *self {
            Self::Ahead { wall_ms, bound_ms } => vec![
                ("wall_ms", Arg::Number(wall_ms)),
                ("bound_ms", Arg::Number(bound_ms)),
            ],
            Self::Exhausted => Vec::new(),
        };
        Problem {
            code: ProblemCode::EventClockAhead,
            args,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn exposes_its_parts() {
        let clock = Hlc::new(1_700_000_000_123, 7);
        assert_eq!((clock.wall_ms(), clock.logical()), (1_700_000_000_123, 7));
        assert_eq!(Hlc::ZERO, Hlc::new(0, 0));
    }

    #[test]
    fn orders_by_wall_time_then_counter() {
        let mut clocks = vec![
            Hlc::new(5, 0),
            Hlc::new(4, 9),
            Hlc::new(5, 1),
            Hlc::new(0, u32::MAX),
        ];
        clocks.sort();
        assert_eq!(
            clocks,
            [
                Hlc::new(0, u32::MAX),
                Hlc::new(4, 9),
                Hlc::new(5, 0),
                Hlc::new(5, 1)
            ]
        );
    }

    #[test]
    fn send_moves_to_a_later_wall_time_with_a_zero_counter() {
        assert_eq!(Hlc::new(1_000, 4).send(1_001), Ok(Hlc::new(1_001, 0)));
    }

    #[test]
    fn send_at_the_same_wall_time_ticks_the_counter() {
        assert_eq!(Hlc::new(1_000, 4).send(1_000), Ok(Hlc::new(1_000, 5)));
    }

    #[test]
    fn send_never_runs_backwards_when_the_wall_clock_does() {
        assert_eq!(Hlc::new(1_000, 4).send(10), Ok(Hlc::new(1_000, 5)));
    }

    #[test]
    fn a_full_counter_carries_into_the_wall_time() {
        assert_eq!(
            Hlc::new(1_000, u32::MAX).send(1_000),
            Ok(Hlc::new(1_001, 0))
        );
    }

    #[test]
    fn the_last_clock_is_exhausted() {
        assert_eq!(
            Hlc::new(u64::MAX, u32::MAX).send(u64::MAX),
            Err(ClockError::Exhausted)
        );
        assert_eq!(
            Hlc::new(u64::MAX, u32::MAX - 1).send(u64::MAX),
            Ok(Hlc::new(u64::MAX, u32::MAX))
        );
    }

    #[test]
    fn receive_from_a_node_ahead_adopts_its_clock_and_ticks() {
        let local = Hlc::new(1_000, 9);
        assert_eq!(
            local.receive(Hlc::new(1_500, 3), 1_200),
            Ok(Hlc::new(1_500, 4))
        );
    }

    #[test]
    fn receive_from_a_node_behind_keeps_its_own_clock_and_ticks() {
        let local = Hlc::new(1_500, 3);
        assert_eq!(
            local.receive(Hlc::new(1_000, 9), 1_200),
            Ok(Hlc::new(1_500, 4))
        );
    }

    #[test]
    fn receive_from_a_node_at_the_same_wall_time_ticks_past_the_larger_counter() {
        let local = Hlc::new(1_500, 3);
        assert_eq!(
            local.receive(Hlc::new(1_500, 8), 1_200),
            Ok(Hlc::new(1_500, 9))
        );
        assert_eq!(
            Hlc::new(1_500, 8).receive(Hlc::new(1_500, 3), 1_200),
            Ok(Hlc::new(1_500, 9))
        );
    }

    #[test]
    fn receive_moves_to_the_wall_time_when_it_is_later_than_both() {
        let local = Hlc::new(1_000, 9);
        assert_eq!(
            local.receive(Hlc::new(1_100, 2), 1_200),
            Ok(Hlc::new(1_200, 0))
        );
    }

    #[test]
    fn receive_carries_a_full_counter() {
        let local = Hlc::new(1_000, 1);
        assert_eq!(
            local.receive(Hlc::new(1_000, u32::MAX), 900),
            Ok(Hlc::new(1_001, 0))
        );
    }

    #[test]
    fn receive_accepts_a_clock_exactly_at_the_skew_bound() {
        let now = 1_000_000;
        let remote = Hlc::new(1_300_000, 0);
        assert_eq!(Hlc::ZERO.receive(remote, now), Ok(Hlc::new(1_300_000, 1)));
    }

    /// A remote clock at the bound with a full counter would carry one
    /// millisecond past it. The bound is on the clock the server would
    /// adopt, not only on the remote wall time.
    #[test]
    fn receive_refuses_a_full_counter_that_would_carry_past_the_skew_bound() {
        let now = 1_000_000;
        let full = Hlc::new(1_300_000, u32::MAX);
        let ahead = Err(ClockError::Ahead {
            wall_ms: 1_300_000,
            bound_ms: 1_300_000,
        });
        assert_eq!(Hlc::ZERO.receive(full, now), ahead);
        assert_eq!(full.receive(full, now), ahead);
    }

    /// A remote clock at the bound, one tick short of a full counter,
    /// would be adopted with a full counter, and the next clock this node
    /// issued would carry one millisecond past the bound: two steps where
    /// the test above refuses one.
    #[test]
    fn receive_refuses_a_clock_that_would_leave_a_full_counter_at_the_skew_bound() {
        let now = 1_000_000;
        let nearly_full = Hlc::new(1_300_000, u32::MAX - 1);
        let ahead = Err(ClockError::Ahead {
            wall_ms: 1_300_000,
            bound_ms: 1_300_000,
        });
        assert_eq!(Hlc::ZERO.receive(nearly_full, now), ahead);
        assert_eq!(nearly_full.receive(nearly_full, now), ahead);
    }

    #[test]
    fn receive_adopts_a_clock_that_leaves_a_tick_to_spare_at_the_skew_bound() {
        let now = 1_000_000;
        // Two ticks short of full at the bound: adopted one tick short,
        // and the next clock this node issues is at the bound still.
        assert_eq!(
            Hlc::ZERO.receive(Hlc::new(1_300_000, u32::MAX - 2), now),
            Ok(Hlc::new(1_300_000, u32::MAX - 1))
        );
        assert_eq!(
            Hlc::new(1_300_000, u32::MAX - 1).send(now),
            Ok(Hlc::new(1_300_000, u32::MAX))
        );
        // One millisecond inside the bound the counter may fill, and a
        // full one carries to the bound itself with nothing counted.
        assert_eq!(
            Hlc::ZERO.receive(Hlc::new(1_299_999, u32::MAX - 1), now),
            Ok(Hlc::new(1_299_999, u32::MAX))
        );
        assert_eq!(
            Hlc::ZERO.receive(Hlc::new(1_299_999, u32::MAX), now),
            Ok(Hlc::new(1_300_000, 0))
        );
    }

    /// The bound is on what a client's clock makes this node adopt. A
    /// clock that is this node's own, later than the remote one, still
    /// ticks at the bound and carries past it.
    #[test]
    fn receive_ticks_this_nodes_own_clock_at_the_skew_bound() {
        let now = 1_000_000;
        let behind = Hlc::new(1_300_000, 5);
        assert_eq!(
            Hlc::new(1_300_000, u32::MAX - 1).receive(behind, now),
            Ok(Hlc::new(1_300_000, u32::MAX))
        );
        assert_eq!(
            Hlc::new(1_300_000, u32::MAX).receive(behind, now),
            Ok(Hlc::new(1_300_001, 0))
        );
    }

    #[test]
    fn receive_of_the_last_clock_is_exhausted() {
        assert_eq!(
            Hlc::ZERO.receive(Hlc::new(u64::MAX, u32::MAX), u64::MAX),
            Err(ClockError::Exhausted)
        );
        assert_eq!(
            Hlc::new(u64::MAX, u32::MAX).receive(Hlc::ZERO, u64::MAX),
            Err(ClockError::Exhausted)
        );
    }

    #[test]
    fn receive_refuses_a_clock_past_the_skew_bound() {
        let now = 1_000_000;
        assert_eq!(
            Hlc::ZERO.receive(Hlc::new(1_300_001, 0), now),
            Err(ClockError::Ahead {
                wall_ms: 1_300_001,
                bound_ms: 1_300_000
            })
        );
    }

    #[test]
    fn the_skew_bound_saturates_at_the_end_of_time() {
        assert_eq!(
            Hlc::ZERO.receive(Hlc::new(u64::MAX, 0), u64::MAX - 1),
            Ok(Hlc::new(u64::MAX, 1))
        );
    }

    #[test]
    fn the_skew_bound_is_five_minutes() {
        assert_eq!(MAX_SKEW_MS, 5 * 60 * 1_000);
    }

    #[test]
    fn describes_both_errors_as_a_clock_set_ahead() {
        assert_eq!(
            ClockError::Ahead {
                wall_ms: 9,
                bound_ms: 4
            }
            .problem(),
            Problem {
                code: ProblemCode::EventClockAhead,
                args: vec![("wall_ms", Arg::Number(9)), ("bound_ms", Arg::Number(4))],
            }
        );
        assert_eq!(
            ClockError::Exhausted.problem(),
            Problem {
                code: ProblemCode::EventClockAhead,
                args: Vec::new(),
            }
        );
    }

    /// Wall times near the present and near the end of time, so carries and
    /// saturation are reached.
    fn wall() -> impl Strategy<Value = u64> {
        prop_oneof![0_u64..2_000, u64::MAX - 2_000..=u64::MAX]
    }

    /// Counters near zero and near full.
    fn logical() -> impl Strategy<Value = u32> {
        prop_oneof![0_u32..4, u32::MAX - 4..=u32::MAX]
    }

    /// Any clock but the last one, which is exhausted and tested above.
    fn clock() -> impl Strategy<Value = Hlc> {
        (wall(), logical())
            .prop_map(|(wall_ms, logical)| Hlc::new(wall_ms, logical))
            .prop_filter("not the last clock", |clock| {
                *clock != Hlc::new(u64::MAX, u32::MAX)
            })
    }

    /// What `receive` must do for one input that does not exhaust the
    /// clock: return a clock later than both and not behind the wall time,
    /// from a remote clock within the skew bound, and, when the remote
    /// clock is the one adopted, a clock short of a full counter at the
    /// bound; or refuse the remote clock as ahead.
    fn receive_holds(local: Hlc, remote: Hlc, now: u64) -> Result<(), TestCaseError> {
        let bound_ms = now.saturating_add(MAX_SKEW_MS);
        match local.receive(remote, now) {
            Ok(next) => {
                prop_assert!(
                    next > local
                        && next > remote
                        && next.wall_ms() >= now
                        && remote.wall_ms() <= bound_ms
                );
                prop_assert!(remote < local || next < Hlc::new(bound_ms, u32::MAX));
            }
            Err(error) => prop_assert_eq!(
                error,
                ClockError::Ahead {
                    wall_ms: remote.wall_ms(),
                    bound_ms
                }
            ),
        }
        Ok(())
    }

    /// The check the property makes runs every way on every run: on a
    /// clock that is adopted, on one this node's own clock is ahead of,
    /// and on one that is refused.
    #[test]
    fn the_receive_check_takes_an_adoption_a_tick_and_a_refusal() {
        let now = 1_000_000;
        receive_holds(Hlc::ZERO, Hlc::new(1_300_000, 0), now).unwrap();
        receive_holds(Hlc::new(1_300_000, 9), Hlc::new(1_300_000, 0), now).unwrap();
        receive_holds(Hlc::ZERO, Hlc::new(1_300_001, 0), now).unwrap();
    }

    proptest! {
        #[test]
        fn send_is_later_than_the_clock_and_never_behind_the_wall_time(
            local in clock(),
            now in wall(),
        ) {
            let next = local.send(now).unwrap();
            prop_assert!(next > local && next.wall_ms() >= now);
        }

        #[test]
        fn receive_is_later_than_both_clocks_or_refuses_one_too_far_ahead(
            local in clock(),
            remote in clock(),
            now in wall(),
            early in 0_u64..2_000,
            past in 0_u64..3,
            counter in logical(),
        ) {
            receive_holds(local, remote, now)?;
            // A remote clock one millisecond inside the skew bound, at it
            // and one past it (the bound is 300,000 ms after `early`),
            // with a counter near empty or near full: against a clock
            // behind it, and against itself.
            let edge = Hlc::new(early.saturating_add(299_999).saturating_add(past), counter);
            receive_holds(Hlc::ZERO, edge, early)?;
            receive_holds(edge, edge, early)?;
        }
    }
}
