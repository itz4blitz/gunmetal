//! XChaCha20-Poly1305, through the `chacha20poly1305` crate: the one AEAD
//! of Gunmetal's own server code, with the 192-bit random nonces of
//! [`super::nonce`] (SEC-STD-019, SEC-STD-020, record 9).
#![expect(
    clippy::disallowed_types,
    reason = "the crypto module's one use of chacha20poly1305: XChaCha20-Poly1305 (SEC-STD-018)"
)]

use chacha20poly1305::{AeadInOut as _, KeyInit as _, XChaCha20Poly1305};

use super::nonce::NONCE_LEN;
use crate::secret::Secret;

/// An AEAD key's length in bytes: 256 bits.
pub const KEY_LEN: usize = 32;

/// A tag's length in bytes.
pub const TAG_LEN: usize = 16;

/// The one error of every failed decryption, MAC check and signature
/// check. It never says which step failed: a wrong key, a changed
/// ciphertext, a changed tag, other associated data and a cut-off value
/// all give this same value (SEC-STD-021).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rejected;

/// Encrypts `buffer` in place under `key` and `nonce`, bound to the
/// associated data `aad`, and returns the tag; `None` when the message is
/// longer than the cipher allows (about 256 GiB).
pub(crate) fn seal(
    key: &Secret<[u8; KEY_LEN]>,
    nonce: &[u8; NONCE_LEN],
    aad: &[u8],
    buffer: &mut [u8],
) -> Option<[u8; TAG_LEN]> {
    XChaCha20Poly1305::new(key.value().into())
        .encrypt_inout_detached(nonce.into(), aad, buffer.into())
        .ok()
        .map(Into::into)
}

/// Checks `tag` over `buffer` and `aad` under `key` and `nonce`, and only
/// then decrypts `buffer` in place.
///
/// # Errors
///
/// Returns [`Rejected`] when the tag does not verify; `buffer` is then
/// still the ciphertext.
pub(crate) fn open(
    key: &Secret<[u8; KEY_LEN]>,
    nonce: &[u8; NONCE_LEN],
    aad: &[u8],
    buffer: &mut [u8],
    tag: &[u8; TAG_LEN],
) -> Result<(), Rejected> {
    XChaCha20Poly1305::new(key.value().into())
        .decrypt_inout_detached(nonce.into(), aad, buffer.into(), tag.into())
        .or(Err(Rejected))
}

#[cfg(test)]
mod tests {
    use super::{Rejected, open, seal};
    use crate::secret::Secret;
    use crate::testing::hex;

    /// The example of draft-irtf-cfrg-xchacha-03, appendix A.3.1.
    const PLAINTEXT: &[u8; 114] = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";
    const AAD: [u8; 12] = [
        0x50, 0x51, 0x52, 0x53, 0xc0, 0xc1, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7,
    ];
    const CIPHERTEXT: &str = concat!(
        "bd6d179d3e83d43b9576579493c0e939572a1700252bfaccbed2902c21396cbb",
        "731c7f1b0b4aa6440bf3a82f4eda7e39ae64c6708c54c216cb96b72e1213b452",
        "2f8c9ba40db5d945b11b69b982c1bb9e3f3fac2bc369488f76b2383565d3fff9",
        "21f9664c97637da9768812f615c68b13b52e"
    );
    const TAG: [u8; 16] = [
        0xc0, 0x87, 0x59, 0x24, 0xc1, 0xc7, 0x98, 0x79, 0x47, 0xde, 0xaf, 0xd8, 0x78, 0x0a, 0xcf,
        0x49,
    ];

    /// The example's key, the bytes 0x80 to 0x9f.
    fn key() -> Secret<[u8; 32]> {
        let mut key = [0; 32];
        for (slot, byte) in key.iter_mut().zip(0x80..=0x9f) {
            *slot = byte;
        }
        Secret::new(key)
    }

    /// The example's nonce, the bytes 0x40 to 0x57.
    fn nonce() -> [u8; 24] {
        let mut nonce = [0; 24];
        for (slot, byte) in nonce.iter_mut().zip(0x40..=0x57) {
            *slot = byte;
        }
        nonce
    }

    #[test]
    fn sealing_matches_the_published_xchacha20_poly1305_example() {
        let mut buffer = *PLAINTEXT;
        assert_eq!(seal(&key(), &nonce(), &AAD, &mut buffer), Some(TAG));
        assert_eq!(hex(&buffer), CIPHERTEXT);
    }

    #[test]
    fn opening_the_published_example_gives_its_plaintext() {
        let mut buffer = *PLAINTEXT;
        seal(&key(), &nonce(), &AAD, &mut buffer).unwrap();
        assert_eq!(open(&key(), &nonce(), &AAD, &mut buffer, &TAG), Ok(()));
        assert_eq!(&buffer, PLAINTEXT);
    }

    /// Verifies: SEC-STD-021
    ///
    /// A changed tag, a changed ciphertext, other associated data, another
    /// nonce and another key each give the same error value, and none
    /// decrypts anything: the buffer still holds the ciphertext.
    #[test]
    fn every_failed_check_gives_the_same_error_and_releases_no_plaintext() {
        let mut sealed = *PLAINTEXT;
        seal(&key(), &nonce(), &AAD, &mut sealed).unwrap();
        let mut bad_tag = TAG;
        bad_tag[15] ^= 1;
        let mut bad_text = sealed;
        bad_text[0] ^= 0x80;
        let mut bad_nonce = nonce();
        bad_nonce[23] ^= 1;
        let bad_key = Secret::new([0x80; 32]);
        let mut buffers = [sealed, bad_text, sealed, sealed, sealed];
        let [tag, text, aad, other_nonce, other_key] = &mut buffers;
        assert_eq!(
            [
                open(&key(), &nonce(), &AAD, tag, &bad_tag),
                open(&key(), &nonce(), &AAD, text, &TAG),
                open(&key(), &nonce(), b"other", aad, &TAG),
                open(&key(), &bad_nonce, &AAD, other_nonce, &TAG),
                open(&bad_key, &nonce(), &AAD, other_key, &TAG),
            ],
            [Err(Rejected); 5]
        );
        assert_eq!(buffers, [sealed, bad_text, sealed, sealed, sealed]);
    }

    /// Verifies: SEC-TM-049, SEC-STD-023
    ///
    /// `XChaCha20Poly1305` stores a copy of the 32-byte key. Its `Drop`
    /// wipes that copy only under `feature = "zeroize"`, which also
    /// implements `ZeroizeOnDrop`. The bound fails to compile if the
    /// feature is off.
    #[test]
    fn the_cipher_state_implements_zeroize_on_drop() {
        use chacha20poly1305::XChaCha20Poly1305;
        use sha2::digest::zeroize::ZeroizeOnDrop;
        fn wiped_on_drop<T: ZeroizeOnDrop>() -> &'static str {
            core::any::type_name::<T>()
        }
        assert_eq!(
            wiped_on_drop::<XChaCha20Poly1305>().split('<').next(),
            Some("chacha20poly1305::ChaChaPoly1305")
        );
    }
}
