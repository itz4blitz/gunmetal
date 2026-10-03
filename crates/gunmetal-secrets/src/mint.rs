//! The one function that mints public identifiers (SEC-HIS-012,
//! SEC-API-023, SEC-PRV-021).
//!
//! A [`PublicId`] is built only from a [`Minted`] value, and
//! [`Minted::from_os_random`] is called only here, with 16 bytes just drawn
//! through the one CSPRNG function ([`crate::random`]); a
//! `disallowed-methods` entry in `clippy.toml` rejects a call anywhere else.
//! So every identifier the server hands out carries 128 bits from the
//! operating system's CSPRNG, and none is derived from a path, a name, a
//! counter or a clock.

use gunmetal_core::id::{IdKind, Minted, PublicId};

use crate::random::{Random, RandomnessUnavailable};

/// A fresh identifier of kind `kind`, from 16 bytes drawn from `random`.
///
/// # Errors
///
/// Returns [`RandomnessUnavailable`] when `random` cannot supply the bytes;
/// no identifier is made from anything else (SEC-STD-022).
pub fn mint(kind: IdKind, random: &dyn Random) -> Result<PublicId, RandomnessUnavailable> {
    let mut bytes = [0; 16];
    random.fill(&mut bytes)?;
    Ok(PublicId::new(kind, from_os_random(bytes)))
}

/// Wraps bytes [`mint`] has just drawn.
#[expect(
    clippy::disallowed_methods,
    reason = "the one sanctioned caller: the minting function, with bytes it just drew from the OS CSPRNG (SEC-HIS-012)"
)]
const fn from_os_random(bytes: [u8; 16]) -> Minted {
    Minted::from_os_random(bytes)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::thread;

    use gunmetal_core::id::{IdKind, PublicId};

    use super::mint;
    use crate::random::fake::{Counting, Failing};
    use crate::random::{OsRandom, RandomnessUnavailable};

    /// Verifies: SEC-HIS-012, SEC-PRV-021
    ///
    /// The "from the OS CSPRNG" part of these requirements: the
    /// identifier's 128 bits are exactly the 16 bytes the randomness source
    /// gave, in order, and nothing else goes into it. The expected text is
    /// the bytes 0 to 15 written in Crockford base32 by hand.
    #[test]
    fn an_identifier_carries_exactly_the_sixteen_bytes_drawn() {
        let expected = PublicId::parse("trk_00041061050r3gg28a1c60t3gf", IdKind::Track);
        assert_eq!(mint(IdKind::Track, &Counting).map(Ok), Ok(expected));
        let device = PublicId::parse("dev_00041061050r3gg28a1c60t3gf", IdKind::Device);
        assert_eq!(mint(IdKind::Device, &Counting).map(Ok), Ok(device));
    }

    /// Verifies: SEC-STD-022
    ///
    /// A randomness failure stops minting: there is an error and no
    /// identifier, never one built from a fallback.
    #[test]
    fn a_randomness_failure_mints_nothing() {
        assert_eq!(mint(IdKind::User, &Failing), Err(RandomnessUnavailable));
    }

    /// Verifies: SEC-STD-022
    ///
    /// 100,000 identifiers minted on eight threads at once are all
    /// different.
    #[test]
    fn a_hundred_thousand_identifiers_minted_across_threads_are_distinct() {
        let minted: Vec<PublicId> = thread::scope(|scope| {
            let workers: Vec<_> = (0..8)
                .map(|_| {
                    scope.spawn(|| {
                        (0..12_500)
                            .map(|_| mint(IdKind::Token, &OsRandom))
                            .collect::<Result<Vec<_>, _>>()
                    })
                })
                .collect();
            workers
                .into_iter()
                .flat_map(|worker| worker.join().unwrap().unwrap())
                .collect()
        });
        assert_eq!(minted.len(), 100_000);
        assert_eq!(minted.iter().collect::<HashSet<_>>().len(), 100_000);
    }
}
