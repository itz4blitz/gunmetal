//! The core's cryptography door (SEC-STD-018).
//!
//! This is the one module in `gunmetal-core` that uses a cryptographic
//! crate, and it holds SHA-256 and nothing else: the schema digest
//! ([`crate::schema`]) and the content-identity windows hash with it. Every
//! other algorithm, keyed or keyless, lives in the secrets crate's crypto
//! module, never in the core.

use sha2::{Digest as _, Sha256};

/// The SHA-256 digest of `bytes` (FIPS 180-4).
pub(crate) fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

#[cfg(test)]
mod tests {
    use super::sha256;
    use std::fmt::Write as _;

    /// Writes a digest as lowercase hexadecimal, the way the specification
    /// prints its examples.
    fn hex(digest: [u8; 32]) -> String {
        digest.iter().fold(String::new(), |mut out, byte| {
            write!(out, "{byte:02x}").unwrap();
            out
        })
    }

    /// The core's crypto module computes SHA-256 exactly as FIPS 180-4
    /// specifies, the one algorithm the cryptographic inventory lists for
    /// it. The expected digests are the specification's published examples,
    /// cross-checked with an independent implementation.
    ///
    /// Verifies: SEC-STD-018
    #[test]
    fn hashes_the_fips_180_4_examples() {
        assert_eq!(
            hex(sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hex(sha256(
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
            )),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        assert_eq!(
            hex(sha256(&vec![b'a'; 1_000_000])),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    #[test]
    fn hashes_the_empty_message() {
        assert_eq!(
            hex(sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn hashes_a_message_that_spans_two_blocks() {
        // 112 octets: one full 64-octet block, then 48 octets that share
        // the second block with the padding and the length.
        assert_eq!(
            hex(sha256(
                b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmn\
                  hijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu"
            )),
            "cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1"
        );
    }
}
