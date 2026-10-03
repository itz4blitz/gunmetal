//! The one nonce function of the one AEAD key strategy (SEC-STD-020,
//! record 9 decision 4): every XChaCha20-Poly1305 nonce is 192 bits drawn
//! fresh from the operating system's CSPRNG, never a counter, a clock or a
//! value derived from the message.

use crate::random::{Random, RandomnessUnavailable};

/// A nonce's length in bytes: 192 bits.
pub const NONCE_LEN: usize = 24;

/// A fresh nonce from `random`.
///
/// # Errors
///
/// Returns [`RandomnessUnavailable`] when `random` cannot supply it; the
/// encryption that needed it must stop.
pub fn nonce(random: &dyn Random) -> Result<[u8; NONCE_LEN], RandomnessUnavailable> {
    let mut nonce = [0; NONCE_LEN];
    random.fill(&mut nonce)?;
    Ok(nonce)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::nonce;
    use crate::random::fake::{Counting, Failing};
    use crate::random::{OsRandom, RandomnessUnavailable};

    #[test]
    fn a_nonce_is_the_twenty_four_bytes_drawn() {
        assert_eq!(
            nonce(&Counting),
            Ok([
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
                23
            ])
        );
    }

    /// Verifies: SEC-STD-022
    ///
    /// Without randomness there is no nonce, and so no encryption.
    #[test]
    fn a_randomness_failure_gives_no_nonce() {
        assert_eq!(nonce(&Failing), Err(RandomnessUnavailable));
    }

    /// Verifies: SEC-STD-020
    ///
    /// The nonce generator never repeats over a large sample: 100,000
    /// nonces are all different.
    #[test]
    fn the_nonce_generator_never_repeats() {
        let nonces = (0..100_000)
            .map(|_| nonce(&OsRandom))
            .collect::<Result<HashSet<_>, _>>();
        assert_eq!(nonces.map(|nonces| nonces.len()), Ok(100_000));
    }
}
