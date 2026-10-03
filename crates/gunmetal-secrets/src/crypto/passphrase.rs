//! The parameter floor for Argon2id, the one way to derive a key or a
//! verifier from a secret a person chose (SEC-STD-024, record 9).
//!
//! R1 has no such secret: there are no account passwords and backups need
//! no passphrase. The floor is fixed here before any caller exists, so the
//! first one cannot ask for less: at least the second recommended
//! parameter set of RFC 9106, section 4, which is 64 MiB of memory, 3
//! passes and 4 lanes.

/// Argon2id parameters at or above the floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Argon2Params {
    /// Memory in KiB.
    memory_kib: u32,
    /// Passes over the memory.
    passes: u32,
    /// Lanes.
    lanes: u32,
}

/// Parameters below the floor were asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BelowFloor;

impl Argon2Params {
    /// The floor: 64 MiB, 3 passes and 4 lanes (RFC 9106, section 4).
    pub const FLOOR: Self = Self {
        memory_kib: 65_536,
        passes: 3,
        lanes: 4,
    };

    /// Parameters of `memory_kib` KiB, `passes` passes and `lanes` lanes.
    ///
    /// # Errors
    ///
    /// Returns [`BelowFloor`] when any of them is below [`Self::FLOOR`].
    pub const fn new(memory_kib: u32, passes: u32, lanes: u32) -> Result<Self, BelowFloor> {
        if memory_kib < Self::FLOOR.memory_kib
            || passes < Self::FLOOR.passes
            || lanes < Self::FLOOR.lanes
        {
            return Err(BelowFloor);
        }
        Ok(Self {
            memory_kib,
            passes,
            lanes,
        })
    }

    /// Memory in KiB.
    #[must_use]
    pub const fn memory_kib(self) -> u32 {
        self.memory_kib
    }

    /// Passes over the memory.
    #[must_use]
    pub const fn passes(self) -> u32 {
        self.passes
    }

    /// Lanes.
    #[must_use]
    pub const fn lanes(self) -> u32 {
        self.lanes
    }
}

#[cfg(test)]
mod tests {
    use super::{Argon2Params, BelowFloor};

    /// Verifies: SEC-STD-024
    ///
    /// The parameter floor: 64 MiB, 3 passes and 4 lanes are accepted, and
    /// one step below in any of them is refused.
    #[test]
    fn parameters_below_the_floor_are_refused() {
        let floor = Argon2Params::new(65_536, 3, 4).unwrap();
        assert_eq!(floor, Argon2Params::FLOOR);
        assert_eq!(
            (floor.memory_kib(), floor.passes(), floor.lanes()),
            (65_536, 3, 4)
        );
        assert_eq!(
            [
                Argon2Params::new(65_535, 3, 4),
                Argon2Params::new(65_536, 2, 4),
                Argon2Params::new(65_536, 3, 3),
                Argon2Params::new(0, 0, 0),
            ],
            [Err(BelowFloor); 4]
        );
    }

    #[test]
    fn parameters_above_the_floor_are_kept_as_given() {
        let raised = Argon2Params::new(262_144, 4, 8).unwrap();
        assert_eq!(
            (raised.memory_kib(), raised.passes(), raised.lanes()),
            (262_144, 4, 8)
        );
    }
}
