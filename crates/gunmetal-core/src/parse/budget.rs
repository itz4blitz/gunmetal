//! What one parse may spend: a deterministic step budget and a nesting
//! depth.
//!
//! Both are counters, not clocks, so a parse fails at exactly the same step
//! on every machine and the failure is testable and mutation-testable.
//! Wall-clock deadlines belong to the worker host, not to the core.

use super::fault::ParseFault;
use super::limits::{LimitKind, Limits};

/// The steps a parse has left (SEC-MED-007).
///
/// A parser charges steps for every element, box, frame or text unit it
/// handles, so its work stays at or below `per_byte` × input octets +
/// `fixed`, with both constants documented next to the parser. A budget is
/// deliberately not `Copy` or `Clone`: one parse holds one budget.
#[derive(Debug, PartialEq, Eq)]
pub struct Budget {
    left: u64,
}

impl Budget {
    /// A budget of `per_byte` × `len` + `fixed` steps, saturating at
    /// `u64::MAX` instead of overflowing.
    #[must_use]
    pub const fn for_input(len: u64, per_byte: u64, fixed: u64) -> Self {
        Self {
            left: per_byte.saturating_mul(len).saturating_add(fixed),
        }
    }

    /// Spends `steps` while working at `offset`.
    ///
    /// A charge that needs more steps than are left spends the rest of the
    /// budget, so once a charge fails every later charge of one step or more
    /// fails too.
    ///
    /// # Errors
    ///
    /// Returns [`ParseFault::BudgetExceeded`] when fewer than `steps` steps
    /// are left.
    pub const fn charge(&mut self, steps: u64, offset: u64) -> Result<(), ParseFault> {
        if let Some(left) = self.left.checked_sub(steps) {
            self.left = left;
            return Ok(());
        }
        self.left = 0;
        Err(ParseFault::BudgetExceeded { offset })
    }

    /// The steps left.
    #[must_use]
    pub const fn remaining(&self) -> u64 {
        self.left
    }
}

/// How deeply nested the structure a parser is working on is (SEC-MED-005).
///
/// A depth counts levels against one depth limit, chosen by the root the
/// parser starts from: [`Depth::CONTAINER_ROOT`] for binary containers and
/// [`Depth::EMBEDDED_FRAME_ROOT`] for frames embedded in `ID3v2` `CHAP` and
/// `CTOC` frames. A structure of one kind inside another, such as an `ID3v2`
/// tag inside a container, counts from its own root. The parser calls
/// [`Depth::descend`] every time it enters a child, whether it recurses or
/// keeps an explicit stack.
///
/// The maximum is always read from [`Limits`], which never holds a depth
/// above its compiled-in ceiling, and a depth can only be made from one of
/// the two roots, so no caller can count against a number of its own or
/// against a limit that is not a depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Depth {
    /// Levels below the root.
    level: u64,
    /// The depth limit the levels count against.
    limit: LimitKind,
}

impl Depth {
    /// The top level of a binary container, counted against
    /// [`LimitKind::ContainerDepth`].
    pub const CONTAINER_ROOT: Self = Self {
        level: 0,
        limit: LimitKind::ContainerDepth,
    };

    /// The top level of an `ID3v2` tag's frames, counted against
    /// [`LimitKind::EmbeddedFrameDepth`].
    pub const EMBEDDED_FRAME_ROOT: Self = Self {
        level: 0,
        limit: LimitKind::EmbeddedFrameDepth,
    };

    /// The depth one level further in, for a child that starts at `offset`.
    ///
    /// # Errors
    ///
    /// Returns [`ParseFault::TooDeep`], naming this depth's limit, when this
    /// depth is already at that limit's value in `limits` or deeper.
    pub const fn descend(self, limits: &Limits, offset: u64) -> Result<Self, ParseFault> {
        let max = limits.get(self.limit);
        // Below `max`, so adding one cannot saturate on the success path.
        let level = self.level.saturating_add(1);
        if self.level >= max {
            return Err(ParseFault::TooDeep {
                limit: self.limit,
                depth: level,
                max,
                offset,
            });
        }
        Ok(Self {
            level,
            limit: self.limit,
        })
    }

    /// How many levels below its root this is.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.level
    }
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use super::*;
    use proptest::collection::vec;
    use proptest::prelude::*;

    /// Verifies: SEC-MED-007
    #[test]
    fn runs_out_at_exactly_the_step_it_should() {
        // 3 steps per octet of a 10-octet input, plus 5.
        let mut budget = Budget::for_input(10, 3, 5);
        assert_eq!(budget.remaining(), 35);
        assert_eq!(budget.charge(34, 0), Ok(()));
        assert_eq!(budget.remaining(), 1);
        assert_eq!(budget.charge(1, 2), Ok(()));
        assert_eq!(budget.remaining(), 0);
        assert_eq!(
            budget.charge(1, 7),
            Err(ParseFault::BudgetExceeded { offset: 7 })
        );
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn a_charge_larger_than_what_is_left_spends_the_rest() {
        let mut budget = Budget::for_input(0, 0, 10);
        assert_eq!(
            budget.charge(11, 3),
            Err(ParseFault::BudgetExceeded { offset: 3 })
        );
        assert_eq!(budget.remaining(), 0);
        // Ten steps were left before the failed charge; none are now.
        assert_eq!(
            budget.charge(1, 4),
            Err(ParseFault::BudgetExceeded { offset: 4 })
        );
    }

    #[test]
    fn charging_nothing_always_succeeds() {
        let mut budget = Budget::for_input(0, 0, 0);
        assert_eq!(budget.charge(0, 0), Ok(()));
        assert_eq!(budget.remaining(), 0);
        assert_eq!(
            budget.charge(1, 1),
            Err(ParseFault::BudgetExceeded { offset: 1 })
        );
        assert_eq!(budget.charge(0, 2), Ok(()));
    }

    /// Verifies: SEC-MED-004, SEC-MED-007
    #[test]
    fn saturates_instead_of_overflowing_for_huge_inputs() {
        assert_eq!(Budget::for_input(u64::MAX, 2, 0).remaining(), u64::MAX);
        assert_eq!(Budget::for_input(1, 1, u64::MAX).remaining(), u64::MAX);
        assert_eq!(Budget::for_input(u64::MAX / 2, 2, 1).remaining(), u64::MAX);
        assert_eq!(Budget::for_input(u64::MAX / 2, 2, 2).remaining(), u64::MAX);
        assert_eq!(
            Budget::for_input(u64::MAX / 2, 2, 0).remaining(),
            u64::MAX - 1
        );
    }

    /// The depth `level` levels below the root of `limit`, written out
    /// rather than reached through [`Depth::descend`].
    const fn at(level: u64, limit: LimitKind) -> Depth {
        Depth { level, limit }
    }

    /// Descends from `root` once per offset in `offsets`, stopping at the
    /// first refusal.
    fn descend_through(
        root: Depth,
        limits: &Limits,
        offsets: std::ops::RangeInclusive<u64>,
    ) -> Result<Depth, ParseFault> {
        offsets
            .into_iter()
            .try_fold(root, |depth, offset| depth.descend(limits, offset))
    }

    /// Verifies: SEC-MED-005
    #[test]
    fn a_container_nests_32_levels_and_refuses_the_33rd() {
        assert_eq!(Depth::CONTAINER_ROOT, at(0, LimitKind::ContainerDepth));
        assert_eq!(Depth::CONTAINER_ROOT.get(), 0);
        let one_below = descend_through(Depth::CONTAINER_ROOT, &Limits::DEFAULT, 1..=31);
        assert_eq!(one_below, Ok(at(31, LimitKind::ContainerDepth)));
        let at_max = one_below.and_then(|depth| depth.descend(&Limits::DEFAULT, 32));
        assert_eq!(at_max, Ok(at(32, LimitKind::ContainerDepth)));
        assert_eq!(at_max.map(Depth::get), Ok(32));
        assert_eq!(
            at_max.and_then(|depth| depth.descend(&Limits::DEFAULT, 100)),
            Err(ParseFault::TooDeep {
                limit: LimitKind::ContainerDepth,
                depth: 33,
                max: 32,
                offset: 100,
            })
        );
    }

    /// Verifies: SEC-MED-005
    #[test]
    fn embedded_frames_nest_4_levels_and_refuse_the_5th() {
        assert_eq!(
            Depth::EMBEDDED_FRAME_ROOT,
            at(0, LimitKind::EmbeddedFrameDepth)
        );
        assert_eq!(Depth::EMBEDDED_FRAME_ROOT.get(), 0);
        let one_below = descend_through(Depth::EMBEDDED_FRAME_ROOT, &Limits::DEFAULT, 1..=3);
        assert_eq!(one_below, Ok(at(3, LimitKind::EmbeddedFrameDepth)));
        let at_max = one_below.and_then(|depth| depth.descend(&Limits::DEFAULT, 4));
        assert_eq!(at_max, Ok(at(4, LimitKind::EmbeddedFrameDepth)));
        assert_eq!(at_max.map(Depth::get), Ok(4));
        assert_eq!(
            at_max.and_then(|depth| depth.descend(&Limits::DEFAULT, 200)),
            Err(ParseFault::TooDeep {
                limit: LimitKind::EmbeddedFrameDepth,
                depth: 5,
                max: 4,
                offset: 200,
            })
        );
    }

    /// Verifies: SEC-MED-005
    #[test]
    fn a_tag_deep_inside_a_container_still_nests_its_own_four_levels() {
        // An ID3v2 tag in the deepest box a container may open counts its
        // embedded frames from their own root, and the container's count
        // carries on unchanged beside it.
        let container = descend_through(Depth::CONTAINER_ROOT, &Limits::DEFAULT, 1..=31);
        let frames = descend_through(Depth::EMBEDDED_FRAME_ROOT, &Limits::DEFAULT, 40..=43);
        assert_eq!(frames, Ok(at(4, LimitKind::EmbeddedFrameDepth)));
        assert_eq!(
            frames.and_then(|depth| depth.descend(&Limits::DEFAULT, 44)),
            Err(ParseFault::TooDeep {
                limit: LimitKind::EmbeddedFrameDepth,
                depth: 5,
                max: 4,
                offset: 44,
            })
        );
        assert_eq!(
            container.and_then(|depth| depth.descend(&Limits::DEFAULT, 50)),
            Ok(at(32, LimitKind::ContainerDepth))
        );
    }

    /// Verifies: SEC-MED-005
    #[test]
    fn reads_each_depth_from_its_own_limit_in_the_limits_given() {
        // Lowering one depth limit to zero forbids nesting of that kind and
        // leaves the other kind alone.
        let no_containers = Limits::DEFAULT.with_override(LimitKind::ContainerDepth, 0);
        assert_eq!(
            no_containers.map(|limits| Depth::CONTAINER_ROOT.descend(&limits, 7)),
            Ok(Err(ParseFault::TooDeep {
                limit: LimitKind::ContainerDepth,
                depth: 1,
                max: 0,
                offset: 7,
            }))
        );
        assert_eq!(
            no_containers.map(|limits| Depth::EMBEDDED_FRAME_ROOT.descend(&limits, 7)),
            Ok(Ok(at(1, LimitKind::EmbeddedFrameDepth)))
        );
        let no_frames = Limits::DEFAULT.with_override(LimitKind::EmbeddedFrameDepth, 0);
        assert_eq!(
            no_frames.map(|limits| Depth::EMBEDDED_FRAME_ROOT.descend(&limits, 8)),
            Ok(Err(ParseFault::TooDeep {
                limit: LimitKind::EmbeddedFrameDepth,
                depth: 1,
                max: 0,
                offset: 8,
            }))
        );
        assert_eq!(
            no_frames.map(|limits| Depth::CONTAINER_ROOT.descend(&limits, 8)),
            Ok(Ok(at(1, LimitKind::ContainerDepth)))
        );
    }

    proptest! {
        /// Verifies: SEC-MED-007
        #[test]
        fn starts_with_exactly_per_byte_times_len_plus_fixed(
            len in 0_u64..1 << 40,
            per_byte in 0_u64..1 << 16,
            fixed in 0_u64..1 << 40,
        ) {
            let expected = u128::from(per_byte) * u128::from(len) + u128::from(fixed);
            prop_assert_eq!(
                u128::from(Budget::for_input(len, per_byte, fixed).remaining()),
                expected
            );
        }

        /// Verifies: SEC-MED-007
        #[test]
        fn never_spends_more_than_it_started_with(
            fixed in 0_u64..1_000,
            charges in vec((0_u64..300, any::<u64>()), 0..40),
        ) {
            let mut budget = Budget::for_input(0, 0, fixed);
            // Independent model: what is left, and the outcome of each charge.
            let mut left = u128::from(fixed);
            for (steps, offset) in charges {
                let wanted = if u128::from(steps) <= left {
                    left -= u128::from(steps);
                    Ok(())
                } else {
                    left = 0;
                    Err(ParseFault::BudgetExceeded { offset })
                };
                prop_assert_eq!(budget.charge(steps, offset), wanted);
                prop_assert_eq!(u128::from(budget.remaining()), left);
            }
        }

        /// Verifies: SEC-MED-005
        #[test]
        fn allows_exactly_as_many_levels_as_the_limit_holds(
            // Each depth limit, at any value up to its ceiling.
            (root, limit, max) in prop_oneof![
                (0_u64..=32).prop_map(|max| (Depth::CONTAINER_ROOT, LimitKind::ContainerDepth, max)),
                (0_u64..=4).prop_map(|max| (Depth::EMBEDDED_FRAME_ROOT, LimitKind::EmbeddedFrameDepth, max)),
            ],
            extra in 1_u64..8,
        ) {
            let limits = Limits::DEFAULT.with_override(limit, max);
            prop_assert_eq!(limits.map(|limits| limits.get(limit)), Ok(max));
            let limits = limits.unwrap_or(Limits::DEFAULT);
            let mut depth = root;
            for level in 1..=max + extra {
                let next = depth.descend(&limits, level);
                if level <= max {
                    prop_assert_eq!(next, Ok(at(level, limit)));
                } else {
                    prop_assert_eq!(
                        next,
                        Err(ParseFault::TooDeep { limit, depth: max + 1, max, offset: level })
                    );
                }
                depth = next.unwrap_or(depth);
            }
            prop_assert_eq!(depth, at(max, limit));
        }
    }
}
