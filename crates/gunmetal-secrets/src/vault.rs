//! The vault: secrets the server must replay to other systems, encrypted
//! at rest (SEC-OPS-017, record 9).
//!
//! In R1 that is the DNS provider token ACME's DNS-01 challenge needs; the
//! OIDC client secret joins it in R1.2. Each is sealed with
//! XChaCha20-Poly1305 under the vault key, which is derived from the root
//! secret, with a fresh 192-bit nonce from the one nonce function
//! (SEC-STD-020) and the owning record's ID as associated data, so a
//! sealed value moved to another record does not open.
//!
//! A sealed value is one version byte ([`VERSION`]), the 24-byte nonce, the
//! ciphertext and the 16-byte tag. Opening fails with one error whatever
//! is wrong with it (SEC-STD-021), and gives the plaintext in a
//! [`Secret`], which is wiped when the one call that needed it drops it.

use crate::crypto::aead::{self, KEY_LEN, Rejected, TAG_LEN};
use crate::crypto::nonce::nonce;
use crate::random::Random;
use crate::root::{Root, SecretsError};
use crate::secret::Secret;

/// The version byte every sealed value starts with.
pub const VERSION: u8 = 1;

/// The bytes before the ciphertext: the version byte and the nonce.
const HEADER_LEN: usize = 25;

/// The context label of the vault key.
const LABEL: &str = "vault";

/// Seals and opens the secrets the server replays to other systems.
///
/// Inventory: vault
#[derive(Debug)]
pub struct Vault {
    /// The vault key.
    key: Secret<[u8; KEY_LEN]>,
}

impl Root {
    /// The vault under this root secret.
    ///
    /// # Errors
    ///
    /// Returns [`SecretsError::Primitive`] when the vault key cannot be
    /// derived, which no call of this crate causes.
    pub fn vault(&self) -> Result<Vault, SecretsError> {
        self.derive(LABEL, 0).map(|key| Vault { key })
    }
}

impl Vault {
    /// Seals `plain` for the record `id`, with a nonce drawn from `random`.
    ///
    /// # Errors
    ///
    /// Returns [`SecretsError::RandomnessUnavailable`] when `random` cannot
    /// supply the nonce; nothing is sealed with any other nonce
    /// (SEC-STD-022). Returns [`SecretsError::Primitive`] when `plain` is
    /// longer than the cipher allows, about 256 GiB.
    pub fn seal(
        &self,
        id: &[u8],
        plain: &[u8],
        random: &dyn Random,
    ) -> Result<Vec<u8>, SecretsError> {
        nonce(random)
            .or(Err(SecretsError::RandomnessUnavailable))
            .and_then(|nonce| {
                let mut sealed = [&[VERSION], &nonce[..], plain].concat();
                let tag = aead::seal(&self.key, &nonce, id, &mut sealed[HEADER_LEN..]);
                tag.map(|tag| [&sealed[..], &tag].concat())
                    .ok_or(SecretsError::Primitive)
            })
    }

    /// Opens `sealed`, which [`Vault::seal`] made for the record `id`.
    ///
    /// # Errors
    ///
    /// Returns [`Rejected`] when `sealed` is not a value this vault sealed
    /// for `id`, whatever the reason.
    pub fn open(&self, id: &[u8], sealed: &[u8]) -> Result<Secret<Vec<u8>>, Rejected> {
        let Some(([VERSION, nonce @ ..], rest)) = sealed.split_first_chunk::<HEADER_LEN>() else {
            return Err(Rejected);
        };
        let Some((body, tag)) = rest.split_last_chunk::<TAG_LEN>() else {
            return Err(Rejected);
        };
        let mut plain = Secret::new(body.to_vec());
        aead::open(&self.key, nonce, id, plain.value_mut(), tag).map(|()| plain)
    }
}

#[cfg(test)]
mod tests {
    use super::Vault;
    use crate::crypto::aead::Rejected;
    use crate::random::OsRandom;
    use crate::random::fake::{Counting, Failing};
    use crate::root::{Root, SecretsError};
    use crate::testing::{counted_root, hex};

    /// What the counted root's vault seals `dns-token-0123` to for the
    /// record `rec_one` with the nonce 0 to 23: the version byte, the
    /// nonce, and libsodium's XChaCha20-Poly1305 output under the
    /// HKDF-SHA-256 key for `gunmetal/v1/vault/0`.
    const SEALED: &str = concat!(
        "01",
        "000102030405060708090a0b0c0d0e0f1011121314151617",
        "8ea1550f2416a5ec9a879d2dfa9e",
        "52ae603d7ccd4edb6893ebbe4586b813"
    );

    /// The same for an empty secret: the version byte, the nonce and a tag.
    const SEALED_EMPTY: &str = concat!(
        "01",
        "000102030405060708090a0b0c0d0e0f1011121314151617",
        "49c3cbe5635e77164392330270046663"
    );

    const PLAIN: &[u8] = b"dns-token-0123";

    /// The counted root's vault.
    fn vault() -> Vault {
        counted_root().vault().unwrap()
    }

    /// What opening `sealed` for `id` gives: the plaintext, or the error.
    fn opened(vault: &Vault, id: &[u8], sealed: &[u8]) -> Result<Vec<u8>, Rejected> {
        vault.open(id, sealed).map(|plain| plain.value().clone())
    }

    /// Verifies: SEC-OPS-017, SEC-STD-020
    ///
    /// A replayed secret is sealed with XChaCha20-Poly1305 under a key
    /// derived from the root secret, with the record's ID as associated
    /// data and the 192-bit nonce the one nonce function drew: the sealed
    /// value is exactly what libsodium gives for that key, nonce and ID.
    #[test]
    fn sealing_gives_the_version_the_drawn_nonce_and_the_ciphertext() {
        let sealed = vault().seal(b"rec_one", PLAIN, &Counting);
        assert_eq!(sealed.as_deref().map(hex).as_deref(), Ok(SEALED));
        let empty = vault().seal(b"rec_one", b"", &Counting);
        assert_eq!(empty.as_deref().map(hex).as_deref(), Ok(SEALED_EMPTY));
    }

    /// Verifies: SEC-OPS-017
    #[test]
    fn a_sealed_secret_opens_to_its_plaintext_for_its_record() {
        let vault = vault();
        let sealed = vault.seal(b"rec_one", PLAIN, &OsRandom).unwrap();
        assert_eq!(sealed.len(), 1 + 24 + 14 + 16);
        assert_eq!(opened(&vault, b"rec_one", &sealed), Ok(PLAIN.to_vec()));
        let empty = vault.seal(b"rec_one", b"", &OsRandom).unwrap();
        assert_eq!(empty.len(), 1 + 24 + 16);
        assert_eq!(opened(&vault, b"rec_one", &empty), Ok(Vec::new()));
    }

    /// Verifies: SEC-STD-020
    ///
    /// Sealing the same secret twice draws two nonces, so the two sealed
    /// values share neither nonce nor ciphertext.
    #[test]
    fn sealing_twice_draws_two_nonces() {
        let vault = vault();
        let first = vault.seal(b"rec_one", PLAIN, &OsRandom).unwrap();
        let second = vault.seal(b"rec_one", PLAIN, &OsRandom).unwrap();
        assert_ne!(first[1..25], second[1..25]);
        assert_ne!(first[25..], second[25..]);
    }

    /// Verifies: SEC-STD-022
    ///
    /// Without randomness there is no nonce and nothing is sealed.
    #[test]
    fn a_randomness_failure_seals_nothing() {
        assert_eq!(
            vault().seal(b"rec_one", PLAIN, &Failing),
            Err(SecretsError::RandomnessUnavailable)
        );
    }

    /// Verifies: SEC-OPS-017, SEC-STD-021
    ///
    /// A sealed value moved to another record's ID fails with the same
    /// error as one whose tag, ciphertext, nonce or version was changed,
    /// one that was cut short, and one sealed under another root secret.
    #[test]
    fn every_value_the_vault_did_not_seal_for_the_record_gives_the_same_error() {
        let vault = vault();
        let sealed = vault.seal(b"rec_one", PLAIN, &Counting).unwrap();
        let changed = |at: usize| {
            let mut bytes = sealed.clone();
            bytes[at] ^= 1;
            bytes
        };
        let outcomes = [
            // Another record, and a record ID that only starts the same.
            opened(&vault, b"rec_two", &sealed),
            opened(&vault, b"rec_one2", &sealed),
            opened(&vault, b"", &sealed),
            // The last byte of the tag, the first of the ciphertext, the
            // last of the nonce and the version byte.
            opened(&vault, b"rec_one", &changed(54)),
            opened(&vault, b"rec_one", &changed(25)),
            opened(&vault, b"rec_one", &changed(24)),
            opened(&vault, b"rec_one", &changed(0)),
            // Cut short: no tag, part of a tag, part of a header, nothing.
            opened(&vault, b"rec_one", &sealed[..54]),
            opened(&vault, b"rec_one", &sealed[..40]),
            opened(&vault, b"rec_one", &sealed[..25]),
            opened(&vault, b"rec_one", &sealed[..24]),
            opened(&vault, b"rec_one", &[]),
            // Longer by one byte.
            opened(&vault, b"rec_one", &[&sealed[..], &[0]].concat()),
        ];
        assert_eq!(outcomes.to_vec(), vec![Err(Rejected); 13]);
        let other = Root::read(&mut &[7_u8; 32][..]).unwrap().vault().unwrap();
        assert_eq!(opened(&other, b"rec_one", &sealed), Err(Rejected));
        // The unchanged value still opens, so each refusal above came from
        // the one change made.
        assert_eq!(opened(&vault, b"rec_one", &sealed), Ok(PLAIN.to_vec()));
    }

    #[test]
    fn a_vault_formats_without_its_key() {
        assert_eq!(
            format!("{:?}", vault()),
            "Vault { key: Secret([redacted]) }"
        );
    }
}
