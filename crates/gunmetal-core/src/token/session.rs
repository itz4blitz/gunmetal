//! Hashing of session tokens under a derived key.
//!
//! A session token is 32 random octets. The identity store keeps only the
//! HMAC-SHA-256 of those octets, keyed through [`MacProvider`], never the
//! token itself (SEC-IAM-037, SEC-OPS-016). The core adds the
//! domain-separation label and never sees key bytes.

use super::mac::MacProvider;
use crate::untrusted::Untrusted;

/// Octets in a session token (SEC-IAM-037).
pub const SESSION_LEN: usize = 32;

/// Domain-separation label for stored session-token hashes (record 9).
const SESSION_LABEL: &[u8] = b"gunmetal/v1/session";

/// Why a session token could not be hashed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionHashError {
    /// The value is not 32 octets, so it is not a session token.
    Malformed {
        /// Octets the caller supplied.
        len: usize,
    },
    /// The current MAC key is unknown or revoked.
    Invalid,
}

/// HMAC-SHA-256 of a 32-octet session token under the provider's current key.
///
/// # Errors
///
/// [`SessionHashError::Malformed`] when `token` is not [`SESSION_LEN`]
/// octets, and [`SessionHashError::Invalid`] when the provider has no key
/// for [`MacProvider::current_kid`].
pub fn hash_session(
    token: Untrusted<&[u8]>,
    mac: &dyn MacProvider,
) -> Result<[u8; 32], SessionHashError> {
    let token = token.into_inner();
    if token.len() != SESSION_LEN {
        return Err(SessionHashError::Malformed { len: token.len() });
    }
    let mut message = Vec::new();
    message.extend_from_slice(SESSION_LABEL);
    message.extend_from_slice(token);
    mac.mac(mac.current_kid(), &message)
        .ok_or(SessionHashError::Invalid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// RFC 4231 test case 1, HMAC-SHA-256 of "Hi There".
    const RFC4231_CASE1: [u8; 32] = [
        0xb0, 0x34, 0x4c, 0x61, 0xd8, 0xdb, 0x38, 0x53, 0x5c, 0xa8, 0xaf, 0xce, 0xaf, 0x0b, 0xf1,
        0x2b, 0x88, 0x1d, 0xc1, 0x81, 0xfd, 0xd9, 0xd4, 0xa9, 0x27, 0xad, 0xb2, 0xbd, 0x57, 0xb7,
        0x99, 0xdf,
    ];

    /// RFC 4231 test case 2, HMAC-SHA-256 of "what do ya want for nothing?".
    const RFC4231_CASE2: [u8; 32] = [
        0x5b, 0xdc, 0xc1, 0x46, 0xbf, 0x60, 0x75, 0x4e, 0x6a, 0x04, 0x24, 0x26, 0x08, 0x95, 0x75,
        0xc7, 0x5a, 0x00, 0x3f, 0x08, 0x9d, 0x27, 0x39, 0x83, 0x9d, 0xec, 0x58, 0xb9, 0x64, 0xec,
        0x38, 0x43,
    ];

    struct RecordingMac {
        kid: u8,
        available: Vec<u8>,
        tag: [u8; 32],
        last: RefCell<Option<(u8, Vec<u8>)>>,
    }

    impl RecordingMac {
        fn new(kid: u8, available: &[u8], tag: [u8; 32]) -> Self {
            Self {
                kid,
                available: available.to_vec(),
                tag,
                last: RefCell::new(None),
            }
        }
    }

    impl MacProvider for RecordingMac {
        fn current_kid(&self) -> u8 {
            self.kid
        }

        fn mac(&self, kid: u8, msg: &[u8]) -> Option<[u8; 32]> {
            *self.last.borrow_mut() = Some((kid, msg.to_vec()));
            self.available.contains(&kid).then_some(self.tag)
        }
    }

    /// Verifies: SEC-OPS-016
    #[test]
    fn hashes_a_session_token_as_the_label_then_the_octets() {
        let mac = RecordingMac::new(3, &[3], RFC4231_CASE1);
        let token = [0x11_u8; SESSION_LEN];
        let tag = hash_session(Untrusted::new(&token), &mac).unwrap();
        assert_eq!(tag, RFC4231_CASE1);
        let (kid, message) = mac.last.borrow().clone().unwrap();
        assert_eq!(kid, 3);
        let mut expected = b"gunmetal/v1/session".to_vec();
        expected.extend_from_slice(&token);
        assert_eq!(message, expected);
    }

    /// Verifies: SEC-OPS-016
    #[test]
    fn places_whichever_rfc_4231_tag_the_provider_returns() {
        let mac = RecordingMac::new(1, &[1], RFC4231_CASE2);
        let token = [0x22_u8; SESSION_LEN];
        assert_eq!(
            hash_session(Untrusted::new(&token), &mac).unwrap(),
            RFC4231_CASE2
        );
    }

    #[test]
    fn refuses_a_token_that_is_not_32_octets() {
        let mac = RecordingMac::new(1, &[1], RFC4231_CASE1);
        assert_eq!(
            hash_session(Untrusted::new(&[]), &mac),
            Err(SessionHashError::Malformed { len: 0 })
        );
        assert_eq!(
            hash_session(Untrusted::new(&[0_u8; 31]), &mac),
            Err(SessionHashError::Malformed { len: 31 })
        );
        assert_eq!(
            hash_session(Untrusted::new(&[0_u8; 33]), &mac),
            Err(SessionHashError::Malformed { len: 33 })
        );
    }

    #[test]
    fn refuses_an_unknown_current_key() {
        let mac = RecordingMac::new(9, &[1], RFC4231_CASE1);
        assert_eq!(
            hash_session(Untrusted::new(&[0_u8; SESSION_LEN]), &mac),
            Err(SessionHashError::Invalid)
        );
    }
}
