//! Key rings: the keyed hashes other crates ask for, computed here so
//! that no key leaves this crate (SEC-OPS-015, SEC-HIS-044, record 9).
//!
//! Each purpose has its own key, derived from the root secret with
//! HKDF-SHA-256 under the context `gunmetal/v1/<purpose>/<generation>`.
//! The server passes a message and gets an HMAC-SHA-256 tag back: it signs
//! capability URLs, hashes session tokens and single-use secrets, and
//! applies the recovery-code pepper without holding a key. A [`KeyRing`] is
//! the [`MacProvider`] the core's token functions take.
//!
//! The URL-signing ring follows a [`KeySchedule`]: it signs with the
//! current generation's key and still answers for the generation it
//! replaced while the schedule says so (SEC-API-030). Every other ring has
//! one key, generation 0, which changes only with the root secret.

use gunmetal_core::time::Timestamp;
use gunmetal_core::token::mac::MacProvider;

use crate::crypto::mac::{KEY_LEN, TAG_LEN, hmac_sha256};
use crate::root::{Root, SecretsError};
use crate::schedule::{KeySchedule, kid};
use crate::secret::Secret;

/// The context label of the URL-signing key.
const URL_SIGNING: &str = "url_signing";

/// What a ring with one fixed key is for: a row of record 9's inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// The stored form of session tokens (`session_hash`).
    SessionHash,
    /// The stored form of invitation secrets (`secret_hash`).
    InvitationHash,
    /// The stored form of pairing codes (`secret_hash`).
    PairingCodeHash,
    /// The stored form of recovery links (`secret_hash`).
    RecoveryLinkHash,
    /// The pepper of recovery-code hashes (`recovery_pepper`).
    RecoveryPepper,
    /// The commitment to an audit record's source address
    /// (`audit_address`).
    AuditAddress,
}

impl Purpose {
    /// The purpose's label in its key's derivation context.
    const fn label(self) -> &'static str {
        match self {
            Self::SessionHash => "session_hash",
            Self::InvitationHash => "secret_hash/invitation",
            Self::PairingCodeHash => "secret_hash/pairing_code",
            Self::RecoveryLinkHash => "secret_hash/recovery_link",
            Self::RecoveryPepper => "recovery_pepper",
            Self::AuditAddress => "audit_address",
        }
    }
}

/// One key of a ring, with the key ID that selects it.
type Entry = (u8, Secret<[u8; KEY_LEN]>);

/// The keys of one purpose that answer now: the one that signs and, for a
/// rotating key, the one it replaced while that still verifies.
///
/// Inventory: `url_signing`
/// Inventory: `session_hash`
/// Inventory: `secret_hash`
/// Inventory: `recovery_pepper`
/// Inventory: `audit_address`
#[derive(Debug)]
pub struct KeyRing {
    /// The key that signs.
    current: Entry,
    /// The key it replaced, while it still verifies.
    previous: Option<Entry>,
}

impl Root {
    /// The ring of the one key for `purpose`, whose key ID is 0.
    ///
    /// # Errors
    ///
    /// Returns [`SecretsError::Primitive`] when the key cannot be derived,
    /// which no call of this crate causes.
    pub fn key_ring(&self, purpose: Purpose) -> Result<KeyRing, SecretsError> {
        self.ring(purpose.label(), 0, None)
    }

    /// The URL-signing ring at `now`: the key of the generation `schedule`
    /// signs with and, inside the overlap, the key of the generation it
    /// replaced. A generation the schedule has ended or revoked has no key
    /// in the ring, so nothing it signed verifies (SEC-API-030).
    ///
    /// # Errors
    ///
    /// Returns [`SecretsError::Primitive`] when a key cannot be derived,
    /// which no call of this crate causes.
    pub fn url_signing(
        &self,
        schedule: &KeySchedule,
        now: Timestamp,
    ) -> Result<KeyRing, SecretsError> {
        let (current, previous) = schedule.answering(now);
        self.ring(URL_SIGNING, current, previous)
    }

    /// The ring for the context label `label` with the keys of generation
    /// `current` and, when given, `previous`.
    fn ring(
        &self,
        label: &str,
        current: u64,
        previous: Option<u64>,
    ) -> Result<KeyRing, SecretsError> {
        let entry = |generation| {
            self.derive(label, generation)
                .map(|key| (kid(generation), key))
        };
        let previous = previous.map(entry).transpose();
        entry(current).and_then(|current| previous.map(|previous| KeyRing { current, previous }))
    }
}

impl MacProvider for KeyRing {
    fn current_kid(&self) -> u8 {
        self.current.0
    }

    fn mac(&self, kid: u8, msg: &[u8]) -> Option<[u8; TAG_LEN]> {
        [Some(&self.current), self.previous.as_ref()]
            .into_iter()
            .flatten()
            .find(|(id, _)| *id == kid)
            .map(|(_, key)| hmac_sha256(key, msg))
    }
}

#[cfg(test)]
mod tests {
    use gunmetal_core::time::Timestamp;
    use gunmetal_core::token::mac::MacProvider;

    use super::{KeyRing, Purpose};
    use crate::root::Root;
    use crate::schedule::KeySchedule;
    use crate::testing::{counted_root, hex};

    /// The message every expected tag below was computed over.
    const MSG: &[u8] = b"gunmetal/v1/test message";

    /// The tags of [`MSG`] under the URL-signing keys of generations 0, 1,
    /// 2 and 256 of the counted root: HMAC-SHA-256 under the HKDF-SHA-256
    /// output for `gunmetal/v1/url_signing/<generation>`, computed with
    /// Python's `hmac` and `hashlib`.
    const URL_0: &str = "67ddf6aa09de0bd6b74776160e5571948ec917f53c59c0e94f436f2a665436b2";
    const URL_1: &str = "58801ea46d52e8c938302421d67c4788b605c06a73f1fb52f146a28dc4ef5ed7";
    const URL_2: &str = "96b773789243977e9127d4927e0b6ba150fe3381ee068ad2b03c4dd2ff3eb1a0";
    const URL_256: &str = "fbbab6c5c09323dcf2c903b2183ea4e665097a9a32346c5b597aa65d7277cd85";

    /// 2026-09-21T00:00:00Z, and a day and four hours in milliseconds.
    const START: i64 = 1_790_000_000_000;
    const DAY: i64 = 86_400_000;
    const HOURS_4: i64 = 14_400_000;

    /// The moment `millis` after [`START`].
    fn at(millis: i64) -> Timestamp {
        Timestamp::from_millis(START + millis).unwrap()
    }

    /// The tags `ring` gives [`MSG`] under key IDs 0 to 3, in hexadecimal.
    fn tags(ring: &KeyRing) -> [Option<String>; 4] {
        [0, 1, 2, 3].map(|kid| ring.mac(kid, MSG).map(|tag| hex(&tag)))
    }

    /// The URL-signing ring of the counted root at `millis` after
    /// [`START`].
    fn url_ring(schedule: &KeySchedule, millis: i64) -> KeyRing {
        counted_root().url_signing(schedule, at(millis)).unwrap()
    }

    /// Verifies: SEC-OPS-015
    ///
    /// Each purpose has its own key, derived from the root secret with
    /// HKDF under a context that names the purpose and the key's
    /// generation: the six tags are the ones an independent HKDF and HMAC
    /// give for those contexts, and no two are alike.
    #[test]
    fn each_purpose_signs_with_its_own_key_derived_from_the_root() {
        let root = counted_root();
        let tag = |purpose| {
            let ring = root.key_ring(purpose).unwrap();
            assert_eq!(ring.current_kid(), 0);
            assert_eq!(ring.mac(1, MSG), None);
            assert_eq!(ring.mac(255, MSG), None);
            ring.mac(0, MSG).map(|tag| hex(&tag))
        };
        assert_eq!(
            [
                tag(Purpose::SessionHash),
                tag(Purpose::InvitationHash),
                tag(Purpose::PairingCodeHash),
                tag(Purpose::RecoveryLinkHash),
                tag(Purpose::RecoveryPepper),
                tag(Purpose::AuditAddress),
            ]
            .map(Option::unwrap),
            [
                "cfc7e7f510d4bc43afe3188d244720dc6dbb4b9048c4cd000714c77164669ba6",
                "6a67784568a5e01f7c0beb7273877407f1fab4cb1b796a8db9994577bd2ab742",
                "31360c8af905db39a97d4c29fa526e3b6c1a9e763df74aa0d52856573b62764a",
                "c485c84f3c289dec6abff4e932f507c1ffb132b70092ef8bda016f7adec4a742",
                "4c14783ebbc16517da903c093ba27ec0816216f58dae52d21994c44023265d2f",
                "8d38cbc6b723ccea353531705b39fa8d7770e1f6b77909d5bba8496e34ccfbb3",
            ]
        );
    }

    #[test]
    fn another_root_gives_other_keys() {
        let other = Root::read(&mut &[7_u8; 32][..]).unwrap();
        let tag = other
            .key_ring(Purpose::SessionHash)
            .unwrap()
            .mac(0, MSG)
            .map(|tag| hex(&tag));
        // HMAC-SHA-256 under HKDF-SHA-256 of 32 bytes of 7, from the same
        // Python oracle.
        assert_eq!(
            tag.as_deref(),
            Some("38e71c9458948db72dff211433c1b05de824e8e698b7964c77c4fb70695f167f")
        );
    }

    /// Verifies: SEC-OPS-015, SEC-API-030
    ///
    /// The URL-signing ring signs with the current generation's key; after
    /// a rotation the key it replaced still verifies, selected by its key
    /// ID, for the four hours of the longest stream capability and never
    /// after.
    #[test]
    fn the_url_signing_ring_answers_for_the_previous_key_only_in_the_overlap() {
        let mut schedule = KeySchedule::new(at(0));
        let ring = url_ring(&schedule, 0);
        assert_eq!(ring.current_kid(), 0);
        assert_eq!(tags(&ring), [Some(URL_0.to_owned()), None, None, None]);
        schedule.rotate_if_due(at(DAY));
        let ring = url_ring(&schedule, DAY + HOURS_4 - 1);
        assert_eq!(ring.current_kid(), 1);
        assert_eq!(
            tags(&ring),
            [Some(URL_0.to_owned()), Some(URL_1.to_owned()), None, None]
        );
        let ring = url_ring(&schedule, DAY + HOURS_4);
        assert_eq!(ring.current_kid(), 1);
        assert_eq!(tags(&ring), [None, Some(URL_1.to_owned()), None, None]);
    }

    /// Verifies: SEC-API-030
    ///
    /// Revoking a key ID takes its key out of the ring at once, so no URL
    /// it signed verifies: the previous key inside its overlap, and the
    /// current key, which the next generation's key replaces.
    #[test]
    fn a_revoked_key_id_has_no_key_in_the_ring() {
        let mut schedule = KeySchedule::new(at(0));
        schedule.rotate_if_due(at(DAY));
        schedule.revoke(0, at(DAY + 1));
        assert_eq!(
            tags(&url_ring(&schedule, DAY + 1)),
            [None, Some(URL_1.to_owned()), None, None]
        );
        schedule.revoke(1, at(DAY + 2));
        let ring = url_ring(&schedule, DAY + 2);
        assert_eq!(ring.current_kid(), 2);
        assert_eq!(tags(&ring), [None, None, Some(URL_2.to_owned()), None]);
    }

    /// Verifies: SEC-API-030
    ///
    /// When key ID 0 comes round again after 256 generations it selects a
    /// different key, so nothing generation 0 signed verifies under it.
    #[test]
    fn a_key_id_that_comes_round_again_selects_another_key() {
        let mut schedule = KeySchedule::new(at(0));
        for day in 1..=256 {
            schedule.rotate_if_due(at(day * DAY));
        }
        let ring = url_ring(&schedule, 256 * DAY + HOURS_4);
        assert_eq!(ring.current_kid(), 0);
        assert_eq!(tags(&ring), [Some(URL_256.to_owned()), None, None, None]);
    }

    #[test]
    fn a_ring_formats_without_its_keys() {
        let ring = counted_root().key_ring(Purpose::SessionHash).unwrap();
        assert_eq!(
            format!("{ring:?}"),
            "KeyRing { current: (0, Secret([redacted])), previous: None }"
        );
    }
}
