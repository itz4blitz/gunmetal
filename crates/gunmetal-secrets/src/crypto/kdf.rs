//! HKDF-SHA-256 (RFC 5869), through the `hkdf` crate: how every key is
//! derived from the root secret (SEC-OPS-015, record 9).
#![expect(
    clippy::disallowed_types,
    reason = "the crypto module's one use of hkdf: HKDF-SHA-256 (SEC-STD-018)"
)]

use hkdf::Hkdf;
use sha2::Sha256;

/// More output was asked for than HKDF-SHA-256 can give: 255 blocks of 32
/// bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TooLong;

/// Fills `okm` with the HKDF-SHA-256 output for the input key `ikm`, the
/// salt `salt` (empty for none) and the context `info`.
///
/// # Errors
///
/// Returns [`TooLong`] when `okm` is longer than 8,160 bytes.
pub(crate) fn hkdf_sha256(
    salt: &[u8],
    ikm: &[u8],
    info: &[u8],
    okm: &mut [u8],
) -> Result<(), TooLong> {
    Hkdf::<Sha256>::new(Some(salt), ikm)
        .expand(info, okm)
        .or(Err(TooLong))
}

#[cfg(test)]
mod tests {
    use super::{TooLong, hkdf_sha256};
    use crate::testing::hex;

    /// The output of `len` bytes, in hexadecimal.
    fn derived(salt: &[u8], ikm: &[u8], info: &[u8], len: usize) -> Result<String, TooLong> {
        let mut okm = vec![0; len];
        hkdf_sha256(salt, ikm, info, &mut okm).map(|()| hex(&okm))
    }

    /// Verifies: SEC-OPS-015
    ///
    /// The three SHA-256 test cases of RFC 5869, appendix A: the basic
    /// case, the case with longer inputs and outputs, and the case with no
    /// salt and no context.
    #[test]
    fn the_output_matches_the_rfc_5869_vectors() {
        let salt: Vec<u8> = (0x00..=0x0c).collect();
        let info: Vec<u8> = (0xf0..=0xf9).collect();
        assert_eq!(
            derived(&salt, &[0x0b; 22], &info, 42).as_deref(),
            Ok(
                "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"
            )
        );
        let ikm: Vec<u8> = (0x00..=0x4f).collect();
        let salt: Vec<u8> = (0x60..=0xaf).collect();
        let info: Vec<u8> = (0xb0..=0xff).collect();
        assert_eq!(
            derived(&salt, &ikm, &info, 82).as_deref(),
            Ok(concat!(
                "b11e398dc80327a1c8e7f78c596a49344f012eda2d4efad8a050cc4c19afa97c",
                "59045a99cac7827271cb41c65e590e09da3275600c2f09b8367793a9aca3db71",
                "cc30c58179ec3e87c14c01d5c1f3434f1d87"
            ))
        );
        assert_eq!(
            derived(&[], &[0x0b; 22], &[], 42).as_deref(),
            Ok(
                "8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d9d201395faa4b61a96c8"
            )
        );
    }

    #[test]
    fn the_longest_output_is_255_blocks_and_one_byte_more_is_refused() {
        let longest = derived(&[], &[0x0b; 22], &[], 8_160).unwrap();
        // The output is a stream: the first 42 bytes are test case 3's.
        assert_eq!(
            &longest[..84],
            "8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d9d201395faa4b61a96c8"
        );
        assert_eq!(longest.len(), 16_320);
        assert_eq!(derived(&[], &[0x0b; 22], &[], 8_161), Err(TooLong));
    }
}
