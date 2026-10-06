//! The core's cryptography door (SEC-STD-018).
//!
//! This is the one module in `gunmetal-core` that uses a cryptographic
//! crate, and it holds SHA-256 and nothing else: the schema digest
//! ([`crate::schema`]) and the content-identity windows hash with it. It is
//! public, so every crate that needs SHA-256, such as the worker hashing an
//! audio window (WP-079), goes through this door rather than using `sha2`
//! itself.
//! Every other algorithm, keyed or keyless, lives in the secrets crate's
//! crypto module, never in the core.
#![expect(
    clippy::disallowed_types,
    reason = "the core's one sanctioned use of sha2: SHA-256 for the schema digest and content identity (SEC-STD-018)"
)]

use sha2::{Digest as _, Sha256};

/// The SHA-256 digest of `bytes` (FIPS 180-4).
///
/// ```
/// // The first example of FIPS 180-4.
/// assert_eq!(
///     gunmetal_core::crypto::sha256(b"abc"),
///     [
///         0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
///         0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
///         0xf2, 0x00, 0x15, 0xad,
///     ]
/// );
/// ```
#[must_use]
pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

#[cfg(test)]
mod tests {
    use super::sha256;
    use crate::test_support::hex_lower;

    /// The core's crypto module computes SHA-256 exactly as FIPS 180-4
    /// specifies. The expected digests are the specification's published
    /// examples, cross-checked with an independent implementation.
    ///
    /// This is a correctness test only, not proof of SEC-STD-018. The
    /// core's half of that requirement is that no other module uses a
    /// cryptographic crate, which WP-001's clippy ban on `sha2` outside
    /// this file is to prove; it stays open until that ban lands.
    #[test]
    fn hashes_the_fips_180_4_examples() {
        assert_eq!(
            hex_lower(sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hex_lower(sha256(
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
            )),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        assert_eq!(
            hex_lower(sha256(&(0..1_000_000).map(|_| b'a').collect::<Vec<u8>>())),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    #[test]
    fn hashes_the_empty_message() {
        assert_eq!(
            hex_lower(sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn hashes_a_message_that_spans_two_blocks() {
        // 112 octets: one full 64-octet block, then 48 octets that share
        // the second block with the padding and the length.
        assert_eq!(
            hex_lower(sha256(
                b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmn\
                  hijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu"
            )),
            "cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1"
        );
    }
}
