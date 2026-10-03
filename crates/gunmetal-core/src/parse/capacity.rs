//! The one way the core sizes an allocation from a length or count the input
//! declares (SEC-MED-003).
//!
//! A small file can declare four billion entries. Reserving room for all of
//! them before reading the first makes the allocator abort the process,
//! which no parser can catch. So a declared count never reaches an
//! allocation directly: it goes through [`bounded_capacity`], which also
//! bounds it by the bytes that could actually hold the items and by a
//! ceiling from [`Limits`](super::Limits). Most parsers need no allocation
//! at all, and borrow slices of the input instead.

/// Capacity for `declared` items of at least `min_item_len` encoded octets
/// each, given `remaining` input octets and a hard `ceiling`.
///
/// Returns the smallest of `declared`, `remaining / min_item_len` and
/// `ceiling`. Every item takes at least one octet, so a `min_item_len` of
/// zero counts as one. A result wider than `usize` (on a 32-bit target)
/// saturates to `usize::MAX`, which the ceiling keeps from ever mattering.
#[must_use]
pub fn bounded_capacity(declared: u64, min_item_len: u64, remaining: u64, ceiling: u64) -> usize {
    let fit = remaining.checked_div(min_item_len).unwrap_or(remaining);
    let smallest = declared.min(fit).min(ceiling);
    usize::try_from(smallest).unwrap_or(usize::MAX)
}

/// An empty vector with room for [`bounded_capacity`] items.
///
/// When the allocator cannot provide that room, the vector comes back empty
/// and unreserved instead of aborting the process; it still grows one item
/// at a time.
#[must_use]
#[expect(
    clippy::disallowed_methods,
    reason = "the core's one sanctioned pre-sizing call; its capacity comes from bounded_capacity (SEC-MED-003)"
)]
pub fn bounded_vec<T>(declared: u64, min_item_len: u64, remaining: u64, ceiling: u64) -> Vec<T> {
    let mut items = Vec::new();
    // A refusal leaves `items` empty and unreserved, which is the fallback.
    let _ = items.try_reserve_exact(bounded_capacity(declared, min_item_len, remaining, ceiling));
    items
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn takes_the_declared_count_when_it_is_smallest() {
        assert_eq!(bounded_capacity(10, 4, 100, 1_000), 10);
    }

    /// Verifies: SEC-MED-003
    #[test]
    fn takes_what_the_remaining_octets_can_hold_when_that_is_smallest() {
        assert_eq!(bounded_capacity(1_000, 4, 100, 1_000), 25);
        // A partial item at the end does not count.
        assert_eq!(bounded_capacity(1_000, 4, 103, 1_000), 25);
    }

    /// Verifies: SEC-MED-003
    #[test]
    fn takes_the_ceiling_when_it_is_smallest() {
        assert_eq!(bounded_capacity(1_000, 1, 1_000, 7), 7);
    }

    #[test]
    fn is_zero_when_any_bound_is_zero() {
        assert_eq!(bounded_capacity(0, 1, 100, 100), 0);
        assert_eq!(bounded_capacity(100, 1, 0, 100), 0);
        assert_eq!(bounded_capacity(100, 1, 100, 0), 0);
        // Items longer than the input hold none.
        assert_eq!(bounded_capacity(100, 101, 100, 100), 0);
    }

    /// Verifies: SEC-MED-003
    #[test]
    fn counts_a_zero_item_length_as_one_octet() {
        assert_eq!(bounded_capacity(1_000, 0, 10, 1_000), 10);
    }

    /// Verifies: SEC-MED-003, SEC-TM-032
    #[test]
    fn bounds_a_declared_count_of_u64_max_by_a_small_input() {
        // Sixty-four octets left hold at most eight 8-octet entries.
        assert_eq!(bounded_capacity(u64::MAX, 8, 64, 1_000_000), 8);
    }

    /// Verifies: SEC-MED-004
    #[test]
    fn saturates_when_every_bound_is_u64_max() {
        // Holds on 32-bit and 64-bit targets alike.
        assert_eq!(
            bounded_capacity(u64::MAX, 1, u64::MAX, u64::MAX),
            usize::MAX
        );
    }

    /// Verifies: SEC-MED-004
    #[cfg(target_pointer_width = "64")]
    #[test]
    fn keeps_a_capacity_wider_than_32_bits_on_a_64_bit_target() {
        assert_eq!(
            bounded_capacity(5_000_000_000, 1, u64::MAX, u64::MAX),
            5_000_000_000
        );
    }

    /// Verifies: SEC-MED-004
    #[cfg(target_pointer_width = "32")]
    #[test]
    fn saturates_a_capacity_wider_than_32_bits_on_a_32_bit_target() {
        assert_eq!(
            bounded_capacity(5_000_000_000, 1, u64::MAX, u64::MAX),
            usize::MAX
        );
    }

    /// Verifies: SEC-MED-003, SEC-TM-032
    #[test]
    fn reserves_only_what_the_input_can_hold() {
        let items: Vec<u32> = bounded_vec(u64::MAX, 4, 64, 1_000);
        assert_eq!((items.len(), items.capacity()), (0, 16));
    }

    /// Verifies: SEC-MED-003
    #[test]
    fn reserves_up_to_the_ceiling() {
        let items: Vec<u16> = bounded_vec(500, 2, 1 << 20, 300);
        assert_eq!((items.len(), items.capacity()), (0, 300));
    }

    /// Verifies: SEC-MED-001, SEC-MED-003
    #[test]
    fn comes_back_empty_instead_of_aborting_when_the_allocator_refuses() {
        // `usize::MAX` eight-octet items overflow any address space.
        let items: Vec<u64> = bounded_vec(u64::MAX, 1, u64::MAX, u64::MAX);
        assert_eq!((items.len(), items.capacity()), (0, 0));
    }

    proptest! {
        /// Verifies: SEC-MED-003, SEC-MED-004
        #[test]
        fn is_the_smallest_of_its_three_bounds(
            declared in prop_oneof![any::<u64>(), 0_u64..2_000],
            min_item_len in prop_oneof![any::<u64>(), 0_u64..16],
            remaining in prop_oneof![any::<u64>(), 0_u64..2_000],
            ceiling in prop_oneof![any::<u64>(), 0_u64..2_000],
        ) {
            // Independent model in u128: the three bounds, then saturate to
            // the target's usize.
            let by_octets = u128::from(remaining) / u128::from(min_item_len.max(1));
            let smallest = u128::from(declared).min(by_octets).min(u128::from(ceiling));
            let expected = usize::try_from(smallest).unwrap_or(usize::MAX);
            let capacity = bounded_capacity(declared, min_item_len, remaining, ceiling);
            prop_assert_eq!(capacity, expected);
            let capacity = u128::try_from(capacity).unwrap_or(u128::MAX);
            prop_assert!(capacity <= u128::from(declared));
            prop_assert!(capacity <= by_octets);
            prop_assert!(capacity <= u128::from(ceiling));
        }
    }
}
