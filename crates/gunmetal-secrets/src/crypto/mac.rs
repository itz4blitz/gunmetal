//! HMAC-SHA-256 (RFC 2104), through the `hmac` crate: capability URLs,
//! the keyed hashes of stored secrets and the audit address commitment
//! (record 9).
#![expect(
    clippy::disallowed_types,
    reason = "the crypto module's one use of hmac: HMAC-SHA-256 (SEC-STD-018)"
)]

use hmac::{Hmac, KeyInit as _, Mac as _};
use sha2::Sha256;

use crate::secret::Secret;

/// A MAC key's length in bytes: 256 bits.
pub const KEY_LEN: usize = 32;

/// A tag's length in bytes.
pub const TAG_LEN: usize = 32;

/// The HMAC-SHA-256 tag of `msg` under `key`.
///
/// HMAC pads a key shorter than SHA-256's 64-byte block with zeros, so the
/// key is handed over already padded: that constructor takes exactly one
/// block and cannot fail, and the padded copy is wiped when it is dropped.
pub(crate) fn hmac_sha256(key: &Secret<[u8; KEY_LEN]>, msg: &[u8]) -> [u8; TAG_LEN] {
    let mut block = Secret::new([0; 64]);
    for (slot, byte) in block.value_mut().iter_mut().zip(key.value()) {
        *slot = *byte;
    }
    let mut mac = Hmac::<Sha256>::new(block.value().into());
    mac.update(msg);
    mac.finalize().into_bytes().into()
}

#[cfg(test)]
mod tests {
    use super::{KEY_LEN, hmac_sha256};
    use crate::secret::Secret;
    use crate::testing::hex;

    /// The tag under the key that starts with `key` and is zero after it,
    /// in hexadecimal. HMAC pads a short key with zeros itself, so this is
    /// the tag under `key`.
    fn tag(key: &[u8], msg: &[u8]) -> String {
        let mut padded = [0; KEY_LEN];
        for (slot, byte) in padded.iter_mut().zip(key) {
            *slot = *byte;
        }
        hex(&hmac_sha256(&Secret::new(padded), msg))
    }

    /// Test cases 1 to 4 of RFC 4231, section 4, the ones whose keys fit in
    /// 256 bits; cases 6 and 7 use 131-byte keys, which no key of this
    /// crate is.
    #[test]
    fn the_tag_matches_the_rfc_4231_vectors() {
        assert_eq!(
            tag(&[0x0b; 20], b"Hi There"),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
        assert_eq!(
            tag(b"Jefe", b"what do ya want for nothing?"),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
        assert_eq!(
            tag(&[0xaa; 20], &[0xdd; 50]),
            "773ea91e36800e46854db8ebd09181a72959098b3ef8c122d9635514ced565fe"
        );
        let key: Vec<u8> = (0x01..=0x19).collect();
        assert_eq!(
            tag(&key, &[0xcd; 50]),
            "82558a389a443c0ea4cc819899f2083a85f0faa3e578f8077a2e3ff46729665b"
        );
    }
}
