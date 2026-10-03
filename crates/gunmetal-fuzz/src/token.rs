//! The harness for the capability-token parser and session-token hasher in
//! `gunmetal_core::token`.

use gunmetal_core::time::Timestamp;
use gunmetal_core::token::{
    CapError, CapFields, MacProvider, SESSION_LEN, SessionHashError, hash_session, verify,
};
use gunmetal_core::untrusted::Untrusted;

/// RFC 4231 test case 1 HMAC-SHA-256 tag, used as a fixed fake MAC.
const RFC4231_CASE1: [u8; 32] = [
    0xb0, 0x34, 0x4c, 0x61, 0xd8, 0xdb, 0x38, 0x53, 0x5c, 0xa8, 0xaf, 0xce, 0xaf, 0x0b, 0xf1, 0x2b,
    0x88, 0x1d, 0xc1, 0x81, 0xfd, 0xd9, 0xd4, 0xa9, 0x27, 0xad, 0xb2, 0xbd, 0x57, 0xb7, 0x99, 0xdf,
];

/// A provider that answers only key identifier 1, with the RFC 4231 case 1 tag.
struct FuzzMac;

impl MacProvider for FuzzMac {
    fn current_kid(&self) -> u8 {
        1
    }

    fn mac(&self, kid: u8, _msg: &[u8]) -> Option<[u8; 32]> {
        (kid == 1).then_some(RFC4231_CASE1)
    }
}

/// What the token layer reported for one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// [`verify`] at the Unix epoch, with the RFC 4231 case 1 tag as the MAC.
    pub verify: Result<CapFields, CapError>,
    /// [`hash_session`] of the same bytes.
    pub session: Result<[u8; 32], SessionHashError>,
}

/// Feeds `data` to [`verify`] and [`hash_session`].
///
/// # Panics
///
/// Panics when an accepted capability token names an expiry outside
/// [`Timestamp::MIN`] to [`Timestamp::MAX`], or when a session hash is
/// returned for an input that is not 32 octets.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let text = String::from_utf8_lossy(data);
    let mac = FuzzMac;
    let now = Timestamp::from_millis(0).unwrap_or(Timestamp::MIN);
    let verified = verify(Untrusted::new(text.as_ref()), &mac, now);
    if let Ok(fields) = &verified {
        assert!(
            (Timestamp::MIN..=Timestamp::MAX).contains(&fields.expiry.timestamp()),
            "{text:?} verified to {fields:?}"
        );
    }
    let hashed = hash_session(Untrusted::new(data), &mac);
    if let Ok(tag) = &hashed {
        assert!(
            data.len() == SESSION_LEN && tag == &RFC4231_CASE1,
            "{data:02X?} hashed to {hashed:?}"
        );
    } else {
        assert_eq!(
            hashed,
            Err(SessionHashError::Malformed { len: data.len() }),
            "{data:02X?} hashed to {hashed:?}"
        );
        assert_ne!(data.len(), SESSION_LEN);
    }
    if data.len() == SESSION_LEN {
        assert_eq!(
            hash_session(Untrusted::new(data), &EmptyMac),
            Err(SessionHashError::Invalid)
        );
    }
    Outcome {
        verify: verified,
        session: hashed,
    }
}

/// A provider that answers key 0 only, while its current key is 1, so a
/// 32-octet token cannot be hashed under the current key.
struct EmptyMac;

impl MacProvider for EmptyMac {
    fn current_kid(&self) -> u8 {
        1
    }

    fn mac(&self, kid: u8, _msg: &[u8]) -> Option<[u8; 32]> {
        (kid == 0).then_some(RFC4231_CASE1)
    }
}
