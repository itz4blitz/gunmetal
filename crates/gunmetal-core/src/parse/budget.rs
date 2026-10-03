//! What one parse may spend: a deterministic step budget and a nesting
//! depth.
//!
//! Both are counters, not clocks, so a parse fails at exactly the same step
//! on every machine and the failure is testable and mutation-testable.
//! Wall-clock deadlines belong to the worker host, not to the core.

use super::fault::ParseFault;

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
/// A parser starts at [`Depth::ROOT`] and calls [`Depth::descend`] every
/// time it enters a child, whether it recurses or keeps an explicit stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Depth(u64);

impl Depth {
    /// The top level of a file, outside every structure.
    pub const ROOT: Self = Self(0);

    /// The depth one level further in, for a child that starts at `offset`,
    /// under a limit of `max` levels.
    ///
    /// # Errors
    ///
    /// Returns [`ParseFault::TooDeep`] when this depth is already `max` or
    /// deeper.
    pub const fn descend(self, max: u64, offset: u64) -> Result<Self, ParseFault> {
        // Below `max`, so adding one cannot saturate on the success path.
        let depth = self.0.saturating_add(1);
        if self.0 >= max {
            return Err(ParseFault::TooDeep { depth, max, offset });
        }
        Ok(Self(depth))
    }

    /// How many levels below the top this is.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[cfg(test)]
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

    /// Verifies: SEC-MED-005
    #[test]
    fn descends_to_the_limit_and_refuses_one_level_past_it() {
        let mut depth = Depth::ROOT;
        for level in 1..=32 {
            depth = Depth::descend(depth, 32, level).unwrap_or(depth);
            assert_eq!(depth.get(), level);
        }
        assert_eq!(
            depth.descend(32, 100),
            Err(ParseFault::TooDeep {
                depth: 33,
                max: 32,
                offset: 100,
            })
        );
    }

    /// Verifies: SEC-MED-005
    #[test]
    fn refuses_at_the_limit_and_not_one_below_it() {
        let below = (0..31).try_fold(Depth::ROOT, |depth, _| depth.descend(32, 0));
        assert_eq!(below.map(Depth::get), Ok(31));
        let at = below.and_then(|depth| depth.descend(32, 5));
        assert_eq!(at.map(Depth::get), Ok(32));
        assert_eq!(
            at.and_then(|depth| depth.descend(32, 6)),
            Err(ParseFault::TooDeep {
                depth: 33,
                max: 32,
                offset: 6,
            })
        );
    }

    /// Verifies: SEC-MED-005
    #[test]
    fn a_shallower_limit_refuses_a_structure_already_deeper() {
        // ID3v2 chapter frames allow 4 levels; a parser already 5 levels
        // deep in some other structure may not open one.
        let deep = (0..5).try_fold(Depth::ROOT, |depth, _| depth.descend(32, 0));
        assert_eq!(
            deep.and_then(|depth| depth.descend(4, 9)),
            Err(ParseFault::TooDeep {
                depth: 6,
                max: 4,
                offset: 9,
            })
        );
    }

    /// Verifies: SEC-MED-005
    #[test]
    fn a_limit_of_zero_allows_no_nesting() {
        assert_eq!(Depth::ROOT.get(), 0);
        assert_eq!(
            Depth::ROOT.descend(0, 0),
            Err(ParseFault::TooDeep {
                depth: 1,
                max: 0,
                offset: 0,
            })
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
        fn allows_exactly_max_levels(max in 0_u64..64, extra in 1_u64..8) {
            let mut depth = Depth::ROOT;
            for level in 1..=max + extra {
                let next = depth.descend(max, level);
                if level <= max {
                    prop_assert_eq!(next, Ok(Depth(level)));
                } else {
                    prop_assert_eq!(
                        next,
                        Err(ParseFault::TooDeep { depth: depth.get() + 1, max, offset: level })
                    );
                }
                depth = next.unwrap_or(depth);
            }
            prop_assert_eq!(depth.get(), max);
        }
    }
}
