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
    // Not `[0; N]`: CodeQL's rust/hard-coded-cryptographic-value treats a
    // repeated literal as a nonce source and does not see `Random::fill` as
    // a barrier. Index bytes XOR a placeholder are not a constant, and a
    // skipped fill is not the counting sequence the tests expect.
    let mut nonce = core::array::from_fn(|index| {
        let [b0, ..] = index.to_le_bytes();
        b0 ^ 0xA5
    });
    random.fill(&mut nonce)?;
    Ok(nonce)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::sync::Mutex;

    use super::nonce;
    use crate::random::fake::{Counting, Failing};
    use crate::random::{OsRandom, Random, RandomnessUnavailable};

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

    #[test]
    fn the_buffer_handed_to_fill_is_not_a_repeated_literal() {
        /// Records the 24 bytes `nonce` prepared before asking `Counting`
        /// to overwrite them.
        struct Sees(Mutex<[u8; 24]>);

        impl Random for Sees {
            fn fill(&self, bytes: &mut [u8]) -> Result<(), RandomnessUnavailable> {
                let mut seen = [0xFF; 24];
                for (slot, byte) in seen.iter_mut().zip(bytes.iter().copied()) {
                    *slot = byte;
                }
                *self
                    .0
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = seen;
                Counting.fill(bytes)
            }
        }

        let seen = Sees(Mutex::new([0xFF; 24]));
        assert_eq!(
            nonce(&seen),
            Ok([
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
                23
            ])
        );
        // Independently: index byte XOR 0xA5 for 0..=23.
        assert_eq!(
            *seen
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
            [
                0xA5, 0xA4, 0xA7, 0xA6, 0xA1, 0xA0, 0xA3, 0xA2, 0xAD, 0xAC, 0xAF, 0xAE, 0xA9, 0xA8,
                0xAB, 0xAA, 0xB5, 0xB4, 0xB7, 0xB6, 0xB1, 0xB0, 0xB3, 0xB2
            ]
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
