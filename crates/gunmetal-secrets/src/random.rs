//! Security randomness: the one function over the operating system's CSPRNG
//! (SEC-STD-022).
//!
//! [`OsRandom::fill`] is the only call to `getrandom` in the workspace; a
//! `disallowed-methods` entry in `clippy.toml` rejects one anywhere else,
//! and with it every seeded or user-space generator. Code that needs random
//! bytes takes a [`Random`] handle, so a test can hand it a source that
//! fails, and the real handle is always [`OsRandom`].
//!
//! When the operating system cannot supply randomness, the operation stops
//! with [`RandomnessUnavailable`]. Nothing falls back to a weaker source, a
//! fixed value or a key built into the binary (SEC-STD-022, SEC-HIS-043).

/// The operating system could not supply random bytes. The operation that
/// needed them must stop, and at start-up the server with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RandomnessUnavailable;

/// A source of security randomness.
///
/// The server only ever uses [`OsRandom`]. The trait exists so that tests
/// can force a failure and check that the caller stops (SEC-STD-022).
pub trait Random: Sync {
    /// Fills `bytes` with random bytes.
    ///
    /// # Errors
    ///
    /// Returns [`RandomnessUnavailable`] when no randomness can be had. The
    /// contents of `bytes` are then unspecified and must not be used.
    fn fill(&self, bytes: &mut [u8]) -> Result<(), RandomnessUnavailable>;
}

/// The operating system's CSPRNG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OsRandom;

impl Random for OsRandom {
    #[expect(
        clippy::disallowed_methods,
        reason = "the workspace's one function over the OS CSPRNG (SEC-STD-022)"
    )]
    fn fill(&self, bytes: &mut [u8]) -> Result<(), RandomnessUnavailable> {
        getrandom::fill(bytes).map_err(|_| RandomnessUnavailable)
    }
}

/// Sources for tests: one that always fails and one that writes a known
/// sequence, so a test can see where each byte went.
#[cfg(test)]
pub(crate) mod fake {
    use super::{Random, RandomnessUnavailable};

    /// A source whose randomness is never available.
    pub struct Failing;

    impl Random for Failing {
        fn fill(&self, _: &mut [u8]) -> Result<(), RandomnessUnavailable> {
            Err(RandomnessUnavailable)
        }
    }

    /// A source that writes 0, 1, 2 and so on, wrapping after 255.
    pub struct Counting;

    impl Random for Counting {
        fn fill(&self, bytes: &mut [u8]) -> Result<(), RandomnessUnavailable> {
            for (byte, value) in bytes.iter_mut().zip((0..=u8::MAX).cycle()) {
                *byte = value;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{OsRandom, Random};

    #[test]
    fn the_operating_system_fills_every_byte_with_fresh_values() {
        // 64 zero bytes from a working CSPRNG has probability 2^-512, and so
        // has a repeat of the same 64 bytes.
        let mut first = [0_u8; 64];
        let mut second = [0_u8; 64];
        assert_eq!(OsRandom.fill(&mut first), Ok(()));
        assert_eq!(OsRandom.fill(&mut second), Ok(()));
        assert_ne!(first, [0; 64]);
        assert_ne!(second, [0; 64]);
        assert_ne!(first, second);
        // Every one of 256 possible values turns up in 64 KiB.
        let mut many = vec![0_u8; 65_536];
        assert_eq!(OsRandom.fill(&mut many), Ok(()));
        assert_eq!(many.iter().collect::<HashSet<_>>().len(), 256);
    }

    #[test]
    fn filling_nothing_succeeds() {
        assert_eq!(OsRandom.fill(&mut []), Ok(()));
    }
}
