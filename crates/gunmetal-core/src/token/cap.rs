//! Capability-URL tokens for media, artwork, lyrics and subtitles.
//!
//! The token is a URL-safe base64 path segment: version, key ID, expiry,
//! operation, representation, object and session-or-share handle, then a
//! 128-bit truncated HMAC-SHA-256 over a domain-separation label and those
//! fields (SEC-API-026). Verification is parse, then MAC in constant time,
//! then expiry. Session lookup is not this module's job (WP-082).

use super::mac::MacProvider;
use crate::base64::{self, Alphabet};
use crate::problem::{Describe, Problem, ProblemCode};
use crate::time::Timestamp;
use crate::untrusted::Untrusted;
use crate::values::Duration;

/// Domain-separation label for capability-URL tags (record 9).
const MEDIA_LABEL: &[u8] = b"gunmetal/v1/media";

/// Layout version written in every token.
const VERSION: u8 = 1;

/// Octets before the tag: version, kid, expiry, operation, representation,
/// object, handle.
const BODY_LEN: usize = 37;

/// Truncated HMAC-SHA-256 tag, 128 bits (SEC-API-026).
const TAG_LEN: usize = 16;

/// Body plus tag.
const TOKEN_LEN: usize = 53;

/// Ten minutes, as seconds.
const TEN_MINUTES_SECS: u64 = 600;

/// Four hours, as seconds: the stream lifetime cap (SEC-API-027).
const FOUR_HOURS_SECS: u64 = 14_400;

/// One hour, as milliseconds: artwork lifetime and bucket (SEC-API-027).
const ARTWORK_BUCKET_MS: i64 = 3_600_000;

/// What a capability URL authorises.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// Play the item's bytes.
    Stream,
    /// Download the item.
    Download,
    /// Fetch artwork of a fixed size.
    Image,
    /// Fetch a subtitle file.
    Subtitle,
    /// Fetch a lyrics file.
    Lyrics,
}

impl Operation {
    const fn as_u8(self) -> u8 {
        match self {
            Self::Stream => 1,
            Self::Download => 2,
            Self::Image => 3,
            Self::Subtitle => 4,
            Self::Lyrics => 5,
        }
    }

    const fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Stream),
            2 => Some(Self::Download),
            3 => Some(Self::Image),
            4 => Some(Self::Subtitle),
            5 => Some(Self::Lyrics),
            _ => None,
        }
    }
}

/// A representation from the server's fixed set (SEC-API-026).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Representation {
    /// The original file, or a lyrics or subtitle file as stored.
    Original,
    /// Artwork resized to 64 px on the long side.
    Artwork64,
    /// Artwork resized to 128 px on the long side.
    Artwork128,
    /// Artwork resized to 256 px on the long side.
    Artwork256,
    /// Artwork resized to 512 px on the long side.
    Artwork512,
    /// Artwork resized to 1024 px on the long side.
    Artwork1024,
}

impl Representation {
    const fn as_u16(self) -> u16 {
        match self {
            Self::Original => 0x0001,
            Self::Artwork64 => 0x0040,
            Self::Artwork128 => 0x0080,
            Self::Artwork256 => 0x0100,
            Self::Artwork512 => 0x0200,
            Self::Artwork1024 => 0x0400,
        }
    }

    const fn from_u16(value: u16) -> Option<Self> {
        match value {
            0x0001 => Some(Self::Original),
            0x0040 => Some(Self::Artwork64),
            0x0080 => Some(Self::Artwork128),
            0x0100 => Some(Self::Artwork256),
            0x0200 => Some(Self::Artwork512),
            0x0400 => Some(Self::Artwork1024),
            _ => None,
        }
    }
}

/// Which lifetime table row to use (SEC-API-027).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapKind {
    /// Stream, download, lyrics or subtitles: the item's duration plus ten
    /// minutes, at most four hours.
    Stream,
    /// Artwork: one hour, aligned to a one-hour bucket.
    Artwork,
}

/// Unix-second expiry carried in a capability token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Expiry(Timestamp);

impl Expiry {
    /// The expiry `seconds` seconds after the Unix epoch.
    ///
    /// Returns [`None`] when that instant is outside [`Timestamp::MIN`] to
    /// [`Timestamp::MAX`].
    #[must_use]
    pub fn from_unix_seconds(seconds: i64) -> Option<Self> {
        let millis = seconds.checked_mul(1_000)?;
        Timestamp::from_millis(millis).ok().map(Self)
    }

    /// Whole seconds after the Unix epoch.
    #[must_use]
    pub fn unix_seconds(self) -> i64 {
        self.0.millis().div_euclid(1_000)
    }

    /// The instant this expiry names, on a second boundary.
    #[must_use]
    pub const fn timestamp(self) -> Timestamp {
        self.0
    }
}

/// Fields a capability token carries, other than the tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapFields {
    /// Key identifier the tag was made under.
    pub kid: u8,
    /// Instant the token stops being valid.
    pub expiry: Expiry,
    /// What the token authorises.
    pub operation: Operation,
    /// Which representation of the object.
    pub representation: Representation,
    /// The object the token is bound to.
    pub object: [u8; 16],
    /// Session or share handle looked up on every use (SEC-API-028).
    pub handle: [u8; 8],
}

/// Why a capability token could not be signed or verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapError {
    /// The text is not a token of the current layout.
    Malformed {
        /// What was wrong.
        reason: MalformedReason,
    },
    /// The MAC did not match, or the key identifier is unknown. One error
    /// for every MAC failure, so a caller cannot tell them apart
    /// (SEC-STD-021).
    Invalid,
    /// The MAC matched and the token's expiry is at or before `now`.
    Expired {
        /// The expiry the token carried.
        expiry: Expiry,
        /// The time the caller passed.
        now: Timestamp,
    },
}

/// Why the text was not a capability token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MalformedReason {
    /// Not canonical URL-safe base64, or not this layout's length after
    /// decoding.
    Encoding,
    /// Decoded to `len` octets rather than 53.
    Length {
        /// Octets the text decoded to.
        len: usize,
    },
    /// The version octet is not 1.
    Version {
        /// The octet.
        version: u8,
    },
    /// The operation octet is not in the closed set.
    Operation {
        /// The octet.
        value: u8,
    },
    /// The representation is not in the server's fixed set.
    Representation {
        /// The two octets as a big-endian integer.
        value: u16,
    },
    /// The expiry seconds do not name a [`Timestamp`].
    Expiry {
        /// The seconds field as decoded.
        seconds: i64,
    },
}

impl Describe for CapError {
    fn problem(&self) -> Problem {
        match self {
            Self::Expired { .. } => Problem {
                code: ProblemCode::MediaUrlExpired,
                args: Vec::new(),
            },
            Self::Malformed { .. } | Self::Invalid => Problem {
                code: ProblemCode::NotFound,
                args: Vec::new(),
            },
        }
    }
}

/// Stream and artwork expiry from SEC-API-027.
///
/// A stream lasts `item_duration` plus ten minutes, at most four hours.
/// Missing duration is treated as zero, so the token lasts ten minutes.
/// Artwork lasts until the end of the one-hour bucket that contains `now`,
/// so two requests in the same hour reuse one URL.
#[must_use]
pub fn lifetime(kind: CapKind, item_duration: Option<Duration>, now: Timestamp) -> Expiry {
    match kind {
        CapKind::Stream => stream_expiry(item_duration, now),
        CapKind::Artwork => artwork_expiry(now),
    }
}

/// Signs `fields` under the provider's current key and returns the path
/// segment. [`CapFields::kid`] is ignored: the current key is always used.
///
/// # Errors
///
/// [`CapError::Invalid`] when the provider has no key for
/// [`MacProvider::current_kid`].
pub fn sign(fields: &CapFields, mac: &dyn MacProvider) -> Result<String, CapError> {
    let kid = mac.current_kid();
    let body = encode_body(kid, fields);
    let Some(tag) = mac.mac(kid, &mac_message(&body)) else {
        return Err(CapError::Invalid);
    };
    let mut token = [0_u8; TOKEN_LEN];
    for (slot, value) in token.iter_mut().zip(body.iter().chain(tag.iter())) {
        *slot = *value;
    }
    Ok(base64::encode(&token, Alphabet::UrlSafe))
}

/// Parses `token`, checks the MAC in constant time, then checks expiry
/// (SEC-API-026).
///
/// # Errors
///
/// A [`CapError`]: malformed text, a MAC or key failure, or an expiry at or
/// before `now`.
pub fn verify(
    token: Untrusted<&str>,
    mac: &dyn MacProvider,
    now: Timestamp,
) -> Result<CapFields, CapError> {
    let token = token.into_inner();
    let decoded = base64::decode(
        Untrusted::new(token.as_bytes()),
        Alphabet::UrlSafe,
        TOKEN_LEN,
    )
    .map_err(|_| CapError::Malformed {
        reason: MalformedReason::Encoding,
    })?;
    if base64::encode(&decoded, Alphabet::UrlSafe) != token {
        return Err(CapError::Malformed {
            reason: MalformedReason::Encoding,
        });
    }
    let len = decoded.len();
    let whole: [u8; TOKEN_LEN] = decoded.try_into().map_err(|_| CapError::Malformed {
        reason: MalformedReason::Length { len },
    })?;
    let version = octet(&whole, 0);
    if version != VERSION {
        return Err(CapError::Malformed {
            reason: MalformedReason::Version { version },
        });
    }
    let kid = octet(&whole, 1);
    let expiry_secs = i64::from_be_bytes(copy_n(&whole, 2));
    let Some(expiry) = Expiry::from_unix_seconds(expiry_secs) else {
        return Err(CapError::Malformed {
            reason: MalformedReason::Expiry {
                seconds: expiry_secs,
            },
        });
    };
    let operation = octet(&whole, 10);
    let Some(operation) = Operation::from_u8(operation) else {
        return Err(CapError::Malformed {
            reason: MalformedReason::Operation { value: operation },
        });
    };
    let representation = u16::from_be_bytes(copy_n(&whole, 11));
    let Some(representation) = Representation::from_u16(representation) else {
        return Err(CapError::Malformed {
            reason: MalformedReason::Representation {
                value: representation,
            },
        });
    };
    let object = copy_n(&whole, 13);
    let handle = copy_n(&whole, 29);
    let claimed = copy_n::<TAG_LEN>(&whole, 37);
    let body = copy_n::<BODY_LEN>(&whole, 0);
    let (computed, known_key) = match mac.mac(kid, &mac_message(&body)) {
        Some(tag) => (tag, true),
        None => ([0_u8; 32], false),
    };
    if !tag_matches(&claimed, &computed) || !known_key {
        return Err(CapError::Invalid);
    }
    if now >= expiry.timestamp() {
        return Err(CapError::Expired { expiry, now });
    }
    Ok(CapFields {
        kid,
        expiry,
        operation,
        representation,
        object,
        handle,
    })
}

fn stream_expiry(item_duration: Option<Duration>, now: Timestamp) -> Expiry {
    let now_secs = now.millis().div_euclid(1_000);
    let duration_secs = item_duration.map_or(0, |duration| duration.millis().saturating_div(1_000));
    let ttl = duration_secs
        .saturating_add(TEN_MINUTES_SECS)
        .min(FOUR_HOURS_SECS);
    expiry_from_secs(now_secs.saturating_add_unsigned(ttl))
}

fn artwork_expiry(now: Timestamp) -> Expiry {
    let start = now
        .millis()
        .div_euclid(ARTWORK_BUCKET_MS)
        .saturating_mul(ARTWORK_BUCKET_MS);
    let end = start.saturating_add(ARTWORK_BUCKET_MS);
    expiry_from_secs(end.div_euclid(1_000))
}

fn expiry_from_secs(seconds: i64) -> Expiry {
    let min = Timestamp::MIN.millis().div_euclid(1_000);
    let max = Timestamp::MAX.millis().div_euclid(1_000);
    let seconds = seconds.clamp(min, max);
    Expiry::from_unix_seconds(seconds).unwrap_or(Expiry(Timestamp::MIN))
}

fn encode_body(kid: u8, fields: &CapFields) -> [u8; BODY_LEN] {
    let expiry = fields.expiry.unix_seconds().to_be_bytes();
    let representation = fields.representation.as_u16().to_be_bytes();
    let mut encoded = Vec::new();
    encoded.push(VERSION);
    encoded.push(kid);
    encoded.extend_from_slice(&expiry);
    encoded.push(fields.operation.as_u8());
    encoded.extend_from_slice(&representation);
    encoded.extend_from_slice(&fields.object);
    encoded.extend_from_slice(&fields.handle);
    let mut body = [0_u8; BODY_LEN];
    for (slot, value) in body.iter_mut().zip(encoded) {
        *slot = value;
    }
    body
}

fn mac_message(body: &[u8]) -> Vec<u8> {
    let mut message = Vec::new();
    message.extend_from_slice(MEDIA_LABEL);
    message.extend_from_slice(body);
    message
}

fn octet(whole: &[u8; TOKEN_LEN], index: usize) -> u8 {
    whole.get(index).copied().unwrap_or(0)
}

fn copy_n<const N: usize>(whole: &[u8; TOKEN_LEN], start: usize) -> [u8; N] {
    let mut out = [0_u8; N];
    for (slot, index) in out.iter_mut().zip(start..) {
        *slot = octet(whole, index);
    }
    out
}

fn tag_matches(claimed: &[u8; TAG_LEN], computed: &[u8; 32]) -> bool {
    let mut diff = 0_u8;
    for (left, right) in claimed.iter().zip(computed) {
        diff |= left ^ right;
    }
    diff == 0
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "tests work with small, bounded values"
)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::cell::RefCell;

    /// RFC 4231 test case 1 HMAC-SHA-256 tag.
    const RFC4231_CASE1: [u8; 32] = [
        0xb0, 0x34, 0x4c, 0x61, 0xd8, 0xdb, 0x38, 0x53, 0x5c, 0xa8, 0xaf, 0xce, 0xaf, 0x0b, 0xf1,
        0x2b, 0x88, 0x1d, 0xc1, 0x81, 0xfd, 0xd9, 0xd4, 0xa9, 0x27, 0xad, 0xb2, 0xbd, 0x57, 0xb7,
        0x99, 0xdf,
    ];

    /// RFC 4231 test case 2 HMAC-SHA-256 tag.
    const RFC4231_CASE2: [u8; 32] = [
        0x5b, 0xdc, 0xc1, 0x46, 0xbf, 0x60, 0x75, 0x4e, 0x6a, 0x04, 0x24, 0x26, 0x08, 0x95, 0x75,
        0xc7, 0x5a, 0x00, 0x3f, 0x08, 0x9d, 0x27, 0x39, 0x83, 0x9d, 0xec, 0x58, 0xb9, 0x64, 0xec,
        0x38, 0x43,
    ];

    const NOW_MS: i64 = 1_700_000_000_000;
    const OBJECT: [u8; 16] = [
        0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54, 0x32,
        0x10,
    ];
    const HANDLE: [u8; 8] = [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x11, 0x22];
    const URL_SAFE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

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

    /// A provider whose tag depends on both the key id and the message, so
    /// a flipped field or a substituted field fails verification.
    struct MixMac {
        current: u8,
        available: Vec<u8>,
    }

    impl MixMac {
        fn new(current: u8, available: &[u8]) -> Self {
            Self {
                current,
                available: available.to_vec(),
            }
        }
    }

    impl MacProvider for MixMac {
        fn current_kid(&self) -> u8 {
            self.current
        }

        fn mac(&self, kid: u8, msg: &[u8]) -> Option<[u8; 32]> {
            self.available.contains(&kid).then_some(mix(kid, msg))
        }
    }

    fn mix(kid: u8, msg: &[u8]) -> [u8; 32] {
        let mut a: u64 = 0x9E37_79B9_7F4A_7C15 ^ u64::from(kid);
        let mut b: u64 = 0xBF58_476D_1CE4_E5B9;
        let mut c: u64 = 0x94D0_49BB_1331_11EB;
        let mut d: u64 = 0xC2B2_AE3D_27D4_EB4F;
        for &octet in msg {
            a = a.rotate_left(7) ^ u64::from(octet).wrapping_mul(0xD6E8_FEB8_66D2_2BE0);
            b = b.wrapping_add(u64::from(octet)).rotate_left(11) ^ a;
            c = c.wrapping_mul(0xA24B_AED4_96E9_C13F) ^ b;
            d = d.rotate_left(u32::from(octet % 63).saturating_add(1)) ^ u64::from(octet);
        }
        d ^= u64::try_from(msg.len()).unwrap_or(u64::MAX);
        let mut out = [0_u8; 32];
        let words = [
            a.to_be_bytes(),
            b.to_be_bytes(),
            c.to_be_bytes(),
            d.to_be_bytes(),
        ];
        for (chunk, word) in out.chunks_mut(8).zip(words) {
            chunk.copy_from_slice(&word);
        }
        out
    }

    fn ts(millis: i64) -> Timestamp {
        Timestamp::from_millis(millis).unwrap()
    }

    fn now() -> Timestamp {
        ts(NOW_MS)
    }

    fn expiry_secs(seconds: i64) -> Expiry {
        Expiry::from_unix_seconds(seconds).unwrap()
    }

    fn sample_fields(expiry: Expiry) -> CapFields {
        CapFields {
            kid: 0,
            expiry,
            operation: Operation::Stream,
            representation: Representation::Original,
            object: OBJECT,
            handle: HANDLE,
        }
    }

    fn decode_token(token: &str) -> Vec<u8> {
        base64::decode(
            Untrusted::new(token.as_bytes()),
            Alphabet::UrlSafe,
            TOKEN_LEN,
        )
        .unwrap()
    }

    fn encode_token(bytes: &[u8]) -> String {
        base64::encode(bytes, Alphabet::UrlSafe)
    }

    fn malformed(reason: MalformedReason) -> CapError {
        CapError::Malformed { reason }
    }

    /// Independent of [`verify`]: a one-bit edit of a 53-octet body is either
    /// an unknown version, operation, representation or expiry, or a MAC
    /// failure on a still-well-formed token.
    fn flipped_token_error(bytes: &[u8]) -> CapError {
        let version = bytes[0];
        if version != 1 {
            return malformed(MalformedReason::Version { version });
        }
        let mut expiry = [0_u8; 8];
        expiry.copy_from_slice(&bytes[2..10]);
        let seconds = i64::from_be_bytes(expiry);
        if seconds
            .checked_mul(1_000)
            .and_then(|millis| Timestamp::from_millis(millis).ok())
            .is_none()
        {
            return malformed(MalformedReason::Expiry { seconds });
        }
        let operation = bytes[10];
        if !matches!(operation, 1..=5) {
            return malformed(MalformedReason::Operation { value: operation });
        }
        let mut representation = [0_u8; 2];
        representation.copy_from_slice(&bytes[11..13]);
        let representation = u16::from_be_bytes(representation);
        if !matches!(
            representation,
            0x0001 | 0x0040 | 0x0080 | 0x0100 | 0x0200 | 0x0400
        ) {
            return malformed(MalformedReason::Representation {
                value: representation,
            });
        }
        CapError::Invalid
    }

    /// Independent of [`verify`]: a proper prefix is either not canonical
    /// URL-safe base64, or it decodes to fewer than 53 octets.
    fn truncated_token_error(prefix: &str) -> CapError {
        match base64::decode(
            Untrusted::new(prefix.as_bytes()),
            Alphabet::UrlSafe,
            TOKEN_LEN,
        ) {
            Ok(decoded) if base64::encode(&decoded, Alphabet::UrlSafe) == prefix => {
                malformed(MalformedReason::Length { len: decoded.len() })
            }
            _ => malformed(MalformedReason::Encoding),
        }
    }

    /// Verifies: SEC-API-026
    #[test]
    fn signs_by_passing_the_label_and_fields_and_placing_the_rfc_4231_tag() {
        let expiry = expiry_secs(1_700_000_780);
        let fields = sample_fields(expiry);
        let mac = RecordingMac::new(1, &[1], RFC4231_CASE1);
        let token = sign(&fields, &mac).unwrap();
        assert_eq!(token.len(), 71);
        assert!(
            token.bytes().all(|octet| URL_SAFE.contains(&octet)),
            "{token}"
        );
        let bytes = decode_token(&token);
        assert_eq!(bytes.len(), TOKEN_LEN);
        assert_eq!(bytes[0], 1);
        assert_eq!(bytes[1], 1);
        assert_eq!(&bytes[2..10], &1_700_000_780_i64.to_be_bytes());
        assert_eq!(bytes[10], 1);
        assert_eq!(&bytes[11..13], &[0x00, 0x01]);
        assert_eq!(&bytes[13..29], &OBJECT);
        assert_eq!(&bytes[29..37], &HANDLE);
        assert_eq!(&bytes[37..53], &RFC4231_CASE1[..16]);
        let (kid, message) = mac.last.borrow().clone().unwrap();
        assert_eq!(kid, 1);
        let mut expected = b"gunmetal/v1/media".to_vec();
        expected.extend_from_slice(&bytes[..37]);
        assert_eq!(message, expected);
    }

    /// Verifies: SEC-API-026
    #[test]
    fn places_the_second_rfc_4231_tag_when_the_provider_returns_it() {
        let fields = sample_fields(expiry_secs(1_700_000_780));
        let mac = RecordingMac::new(2, &[2], RFC4231_CASE2);
        let bytes = decode_token(&sign(&fields, &mac).unwrap());
        assert_eq!(bytes[1], 2);
        assert_eq!(&bytes[37..53], &RFC4231_CASE2[..16]);
    }

    /// Verifies: SEC-API-026
    #[test]
    fn signs_with_the_current_key_not_the_fields_kid() {
        let mut fields = sample_fields(expiry_secs(1_700_000_780));
        fields.kid = 99;
        let mac = MixMac::new(4, &[4]);
        let verified = verify(Untrusted::new(&sign(&fields, &mac).unwrap()), &mac, now()).unwrap();
        assert_eq!(verified.kid, 4);
        assert_eq!(verified.object, OBJECT);
        assert_eq!(verified.handle, HANDLE);
        assert_eq!(verified.operation, Operation::Stream);
        assert_eq!(verified.representation, Representation::Original);
        assert_eq!(verified.expiry, fields.expiry);
    }

    /// Verifies: SEC-API-026
    #[test]
    fn verifies_a_token_signed_with_key_1_during_rotation_overlap() {
        let fields = sample_fields(expiry_secs(1_700_000_780));
        let signer = MixMac::new(1, &[1]);
        let token = sign(&fields, &signer).unwrap();
        let overlap = MixMac::new(2, &[1, 2]);
        let verified = verify(Untrusted::new(&token), &overlap, now()).unwrap();
        assert_eq!(verified.kid, 1);
        assert_eq!(verified.object, OBJECT);
    }

    /// Verifies: SEC-API-026
    #[test]
    fn refuses_a_token_whose_key_is_no_longer_answered() {
        let fields = sample_fields(expiry_secs(1_700_000_780));
        let signer = MixMac::new(1, &[1]);
        let token = sign(&fields, &signer).unwrap();
        let rotated = MixMac::new(2, &[2]);
        assert_eq!(
            verify(Untrusted::new(&token), &rotated, now()),
            Err(CapError::Invalid)
        );
    }

    /// Verifies: SEC-API-026
    #[test]
    fn refuses_an_unknown_key_even_when_the_claimed_tag_is_all_zeros() {
        let mut bytes = [0_u8; TOKEN_LEN];
        bytes[0] = 1;
        bytes[10] = 1;
        bytes[12] = 1;
        let expiry = 1_700_000_780_i64.to_be_bytes();
        bytes[2..10].copy_from_slice(&expiry);
        let mac = MixMac::new(1, &[1]);
        assert_eq!(
            verify(Untrusted::new(&encode_token(&bytes)), &mac, now()),
            Err(CapError::Invalid)
        );
        let empty = RecordingMac::new(1, &[], RFC4231_CASE1);
        assert_eq!(
            sign(&sample_fields(expiry_secs(1)), &empty),
            Err(CapError::Invalid)
        );
    }

    /// Verifies: SEC-API-026
    #[test]
    fn a_flipped_bit_in_every_field_is_rejected() {
        let fields = sample_fields(expiry_secs(1_700_000_780));
        let mac = MixMac::new(1, &[1]);
        let token = sign(&fields, &mac).unwrap();
        let accepted = verify(Untrusted::new(&token), &mac, now()).unwrap();
        assert_eq!(accepted, CapFields { kid: 1, ..fields });
        let bytes = decode_token(&token);
        for (index, octet) in bytes.iter().enumerate() {
            for bit in 0..8_u8 {
                let mut flipped = bytes.clone();
                flipped[index] = octet ^ (1 << bit);
                let text = encode_token(&flipped);
                assert_eq!(
                    verify(Untrusted::new(&text), &mac, now()),
                    Err(flipped_token_error(&flipped)),
                    "index {index} bit {bit}"
                );
            }
        }
    }

    /// Verifies: SEC-API-026
    #[test]
    fn a_field_substituted_from_another_valid_token_is_rejected() {
        let mac = MixMac::new(1, &[1]);
        let a = sample_fields(expiry_secs(1_700_000_780));
        let mut b = a;
        b.object = [0x99; 16];
        b.handle = [0x77; 8];
        b.operation = Operation::Download;
        b.representation = Representation::Artwork256;
        b.expiry = expiry_secs(1_700_014_400);
        let a_bytes = decode_token(&sign(&a, &mac).unwrap());
        let b_bytes = decode_token(&sign(&b, &mac).unwrap());
        let ranges = [2..10, 10..11, 11..13, 13..29, 29..37];
        for range in ranges {
            let mut mixed = a_bytes.clone();
            mixed[range.clone()].copy_from_slice(&b_bytes[range.clone()]);
            assert_eq!(
                verify(Untrusted::new(&encode_token(&mixed)), &mac, now()),
                Err(CapError::Invalid),
                "range {range:?}"
            );
        }
    }

    /// Verifies: SEC-API-026, SEC-API-027
    #[test]
    fn expiry_at_the_boundary_second() {
        let expiry = expiry_secs(1_700_000_780);
        let fields = sample_fields(expiry);
        let mac = MixMac::new(1, &[1]);
        let token = sign(&fields, &mac).unwrap();
        let instant = expiry.timestamp();
        assert_eq!(
            verify(Untrusted::new(&token), &mac, instant),
            Err(CapError::Expired {
                expiry,
                now: instant
            })
        );
        assert_eq!(
            verify(Untrusted::new(&token), &mac, ts(instant.millis() - 1))
                .unwrap()
                .expiry,
            expiry
        );
        assert_eq!(
            verify(Untrusted::new(&token), &mac, ts(instant.millis() + 1)),
            Err(CapError::Expired {
                expiry,
                now: ts(instant.millis() + 1)
            })
        );
    }

    /// Verifies: SEC-API-026
    #[test]
    fn a_bad_mac_on_an_expired_token_is_invalid_not_expired() {
        let expiry = expiry_secs(1);
        let fields = sample_fields(expiry);
        let mac = MixMac::new(1, &[1]);
        let mut bytes = decode_token(&sign(&fields, &mac).unwrap());
        bytes[37] ^= 1;
        assert_eq!(
            verify(Untrusted::new(&encode_token(&bytes)), &mac, now()),
            Err(CapError::Invalid)
        );
    }

    /// Verifies: SEC-API-026
    #[test]
    fn unknown_version_is_refused() {
        let mac = MixMac::new(1, &[1]);
        let mut bytes =
            decode_token(&sign(&sample_fields(expiry_secs(1_700_000_780)), &mac).unwrap());
        bytes[0] = 0;
        assert_eq!(
            verify(Untrusted::new(&encode_token(&bytes)), &mac, now()),
            Err(malformed(MalformedReason::Version { version: 0 }))
        );
        bytes[0] = 2;
        assert_eq!(
            verify(Untrusted::new(&encode_token(&bytes)), &mac, now()),
            Err(malformed(MalformedReason::Version { version: 2 }))
        );
    }

    /// Verifies: SEC-API-026
    #[test]
    fn a_representation_outside_the_fixed_set_is_refused() {
        let mac = MixMac::new(1, &[1]);
        let mut bytes =
            decode_token(&sign(&sample_fields(expiry_secs(1_700_000_780)), &mac).unwrap());
        bytes[11] = 0xFF;
        bytes[12] = 0xFF;
        assert_eq!(
            verify(Untrusted::new(&encode_token(&bytes)), &mac, now()),
            Err(malformed(MalformedReason::Representation { value: 0xFFFF }))
        );
        bytes[11] = 0;
        bytes[12] = 0;
        assert_eq!(
            verify(Untrusted::new(&encode_token(&bytes)), &mac, now()),
            Err(malformed(MalformedReason::Representation { value: 0 }))
        );
    }

    /// Verifies: SEC-API-026
    #[test]
    fn an_unknown_operation_is_refused() {
        let mac = MixMac::new(1, &[1]);
        let mut bytes =
            decode_token(&sign(&sample_fields(expiry_secs(1_700_000_780)), &mac).unwrap());
        bytes[10] = 0;
        assert_eq!(
            verify(Untrusted::new(&encode_token(&bytes)), &mac, now()),
            Err(malformed(MalformedReason::Operation { value: 0 }))
        );
        bytes[10] = 6;
        assert_eq!(
            verify(Untrusted::new(&encode_token(&bytes)), &mac, now()),
            Err(malformed(MalformedReason::Operation { value: 6 }))
        );
    }

    /// Verifies: SEC-API-026
    #[test]
    fn truncation_at_every_length_is_refused() {
        let mac = MixMac::new(1, &[1]);
        let token = sign(&sample_fields(expiry_secs(1_700_000_780)), &mac).unwrap();
        let accepted = verify(Untrusted::new(&token), &mac, now()).unwrap();
        assert_eq!(accepted.kid, 1);
        for len in 0..token.len() {
            let prefix = &token[..len];
            assert_eq!(
                verify(Untrusted::new(prefix), &mac, now()),
                Err(truncated_token_error(prefix)),
                "prefix length {len}"
            );
        }
        let mut extra = token.clone();
        extra.push('A');
        assert_eq!(
            verify(Untrusted::new(&extra), &mac, now()),
            Err(malformed(MalformedReason::Encoding))
        );
    }

    /// Verifies: SEC-API-026
    #[test]
    fn padded_and_standard_alphabet_spellings_are_refused() {
        let mac = MixMac::new(1, &[1]);
        let token = sign(&sample_fields(expiry_secs(1_700_000_780)), &mac).unwrap();
        let padded = format!("{token}=");
        assert_eq!(
            verify(Untrusted::new(&padded), &mac, now()),
            Err(malformed(MalformedReason::Encoding))
        );
        let mut bytes = decode_token(&token);
        bytes[52] = 0xFF;
        let url = encode_token(&bytes);
        let standard = base64::encode(&bytes, Alphabet::Standard);
        assert_ne!(standard, url);
        assert_eq!(
            verify(Untrusted::new(&standard), &mac, now()),
            Err(malformed(MalformedReason::Encoding))
        );
    }

    /// Verifies: SEC-API-026
    #[test]
    fn an_unusable_expiry_field_is_refused() {
        let mac = MixMac::new(1, &[1]);
        let mut bytes =
            decode_token(&sign(&sample_fields(expiry_secs(1_700_000_780)), &mac).unwrap());
        bytes[2..10].copy_from_slice(&i64::MAX.to_be_bytes());
        assert_eq!(
            verify(Untrusted::new(&encode_token(&bytes)), &mac, now()),
            Err(malformed(MalformedReason::Expiry { seconds: i64::MAX }))
        );
    }

    /// Verifies: SEC-API-026
    #[test]
    fn each_operation_and_representation_round_trips() {
        let mac = MixMac::new(1, &[1]);
        let expiry = expiry_secs(1_700_000_780);
        for operation in [
            Operation::Stream,
            Operation::Download,
            Operation::Image,
            Operation::Subtitle,
            Operation::Lyrics,
        ] {
            for representation in [
                Representation::Original,
                Representation::Artwork64,
                Representation::Artwork128,
                Representation::Artwork256,
                Representation::Artwork512,
                Representation::Artwork1024,
            ] {
                let fields = CapFields {
                    kid: 0,
                    expiry,
                    operation,
                    representation,
                    object: OBJECT,
                    handle: HANDLE,
                };
                let verified =
                    verify(Untrusted::new(&sign(&fields, &mac).unwrap()), &mac, now()).unwrap();
                assert_eq!(verified.operation, operation);
                assert_eq!(verified.representation, representation);
            }
        }
        assert_eq!(Operation::from_u8(1).unwrap().as_u8(), 1);
        assert_eq!(Representation::Original.as_u16(), 0x0001);
        assert_eq!(Representation::Artwork64.as_u16(), 64);
        assert_eq!(Representation::Artwork128.as_u16(), 128);
        assert_eq!(Representation::Artwork256.as_u16(), 256);
        assert_eq!(Representation::Artwork512.as_u16(), 512);
        assert_eq!(Representation::Artwork1024.as_u16(), 1024);
    }

    /// Verifies: SEC-API-027
    #[test]
    fn stream_lifetime_matches_the_literal_table() {
        let origin = now();
        let three_minutes = Duration::from_millis(180_000).unwrap();
        let five_hours = Duration::from_millis(18_000_000).unwrap();
        assert_eq!(
            lifetime(CapKind::Stream, Some(three_minutes), origin).unix_seconds(),
            origin.millis() / 1_000 + 780
        );
        assert_eq!(
            lifetime(CapKind::Stream, Some(five_hours), origin).unix_seconds(),
            origin.millis() / 1_000 + 14_400
        );
        assert_eq!(
            lifetime(CapKind::Stream, None, origin).unix_seconds(),
            origin.millis() / 1_000 + 600
        );
        let just_under_cap = Duration::from_millis(13_800_000).unwrap();
        assert_eq!(
            lifetime(CapKind::Stream, Some(just_under_cap), origin).unix_seconds(),
            origin.millis() / 1_000 + 14_400
        );
        let four_hours_less_ten = Duration::from_millis(13_799_000).unwrap();
        assert_eq!(
            lifetime(CapKind::Stream, Some(four_hours_less_ten), origin).unix_seconds(),
            origin.millis() / 1_000 + 14_399
        );
    }

    /// Verifies: SEC-API-027
    #[test]
    fn artwork_lifetime_reuses_one_url_inside_a_bucket() {
        let origin = now();
        let first = lifetime(CapKind::Artwork, None, origin);
        let later = lifetime(
            CapKind::Artwork,
            Some(Duration::from_millis(1).unwrap()),
            ts(origin.millis() + 10_000),
        );
        assert_eq!(first, later);
        assert_eq!(first.unix_seconds(), 1_700_002_800);
        let across = lifetime(CapKind::Artwork, None, first.timestamp());
        assert_ne!(across, first);
        assert_eq!(across.unix_seconds(), 1_700_006_400);
        assert_eq!(
            lifetime(CapKind::Artwork, None, ts(origin.millis() - 1)).unix_seconds(),
            1_700_002_800
        );
    }

    /// Verifies: SEC-API-027
    #[test]
    fn lifetime_clamps_at_the_timestamp_range() {
        let max_stream = lifetime(
            CapKind::Stream,
            Some(Duration::from_millis(18_000_000).unwrap()),
            Timestamp::MAX,
        );
        assert_eq!(
            max_stream.timestamp(),
            ts(Timestamp::MAX.millis().div_euclid(1_000) * 1_000)
        );
        let min_art = lifetime(CapKind::Artwork, None, Timestamp::MIN);
        assert!(min_art.timestamp() >= Timestamp::MIN);
        assert!(Expiry::from_unix_seconds(i64::MAX).is_none());
        assert!(Expiry::from_unix_seconds(i64::MIN).is_none());
        assert_eq!(
            expiry_from_secs(i64::MAX).unix_seconds(),
            Timestamp::MAX.millis().div_euclid(1_000)
        );
        assert_eq!(
            expiry_from_secs(i64::MIN).unix_seconds(),
            Timestamp::MIN.millis().div_euclid(1_000)
        );
    }

    /// Verifies: SEC-API-026
    #[test]
    fn tag_comparison_accumulates_every_octet() {
        let zeros = [0_u8; TAG_LEN];
        let ones = [0xFF_u8; 32];
        assert!(tag_matches(&zeros, &[0_u8; 32]));
        assert!(!tag_matches(&zeros, &ones));
        let mut claimed = [0xA5_u8; TAG_LEN];
        let mut computed = [0xA5_u8; 32];
        assert!(tag_matches(&claimed, &computed));
        claimed[0] ^= 1;
        assert!(!tag_matches(&claimed, &computed));
        claimed[0] = 0xA5;
        claimed[15] ^= 1;
        assert!(!tag_matches(&claimed, &computed));
        claimed[15] = 0xA5;
        computed[0] ^= 1;
        assert!(!tag_matches(&claimed, &computed));
    }

    /// Verifies: SEC-API-026
    #[test]
    fn expired_tokens_describe_as_media_url_expired_and_the_rest_as_not_found() {
        let expiry = expiry_secs(1);
        assert_eq!(
            CapError::Expired { expiry, now: now() }.problem().code,
            ProblemCode::MediaUrlExpired
        );
        assert_eq!(
            malformed(MalformedReason::Encoding).problem().code,
            ProblemCode::NotFound
        );
        assert_eq!(CapError::Invalid.problem().code, ProblemCode::NotFound);
        assert!(CapError::Invalid.problem().args.is_empty());
    }

    /// Verifies: SEC-API-026
    #[test]
    fn decode_errors_become_encoding_failures() {
        let mac = MixMac::new(1, &[1]);
        assert_eq!(
            verify(Untrusted::new("???"), &mac, now()),
            Err(malformed(MalformedReason::Encoding))
        );
        assert_eq!(
            verify(Untrusted::new("A"), &mac, now()),
            Err(malformed(MalformedReason::Encoding))
        );
        let three = encode_token(&[0, 1, 2]);
        assert_eq!(
            verify(Untrusted::new(&three), &mac, now()),
            Err(malformed(MalformedReason::Length { len: 3 }))
        );
        let empty = encode_token(&[]);
        assert_eq!(
            verify(Untrusted::new(&empty), &mac, now()),
            Err(malformed(MalformedReason::Length { len: 0 }))
        );
        let mut whole = [7_u8; TOKEN_LEN];
        whole[52] = 9;
        assert_eq!(octet(&whole, TOKEN_LEN), 0);
        assert_eq!(octet(&whole, usize::MAX), 0);
        assert_eq!(copy_n::<2>(&whole, 52), [9, 0]);
    }

    fn any_operation() -> impl Strategy<Value = Operation> {
        prop_oneof![
            Just(Operation::Stream),
            Just(Operation::Download),
            Just(Operation::Image),
            Just(Operation::Subtitle),
            Just(Operation::Lyrics),
        ]
    }

    fn any_representation() -> impl Strategy<Value = Representation> {
        prop_oneof![
            Just(Representation::Original),
            Just(Representation::Artwork64),
            Just(Representation::Artwork128),
            Just(Representation::Artwork256),
            Just(Representation::Artwork512),
            Just(Representation::Artwork1024),
        ]
    }

    fn any_fields() -> impl Strategy<Value = CapFields> {
        (
            1_i64..2_000_000_000,
            any_operation(),
            any_representation(),
            any::<[u8; 16]>(),
            any::<[u8; 8]>(),
        )
            .prop_map(
                |(seconds, operation, representation, object, handle)| CapFields {
                    kid: 0,
                    expiry: expiry_secs(seconds),
                    operation,
                    representation,
                    object,
                    handle,
                },
            )
    }

    proptest! {
        /// Verifies: SEC-API-026
        #[test]
        fn sign_then_verify_returns_the_fields(fields in any_fields()) {
            let mac = MixMac::new(7, &[7]);
            let token = sign(&fields, &mac).unwrap();
            let now = ts(fields.expiry.timestamp().millis() - 1);
            let verified = verify(Untrusted::new(&token), &mac, now).unwrap();
            prop_assert_eq!(verified.kid, 7);
            prop_assert_eq!(verified.expiry, fields.expiry);
            prop_assert_eq!(verified.operation, fields.operation);
            prop_assert_eq!(verified.representation, fields.representation);
            prop_assert_eq!(verified.object, fields.object);
            prop_assert_eq!(verified.handle, fields.handle);
        }

        /// Verifies: SEC-API-026
        #[test]
        fn any_single_bit_change_fails(fields in any_fields(), index in 0usize..TOKEN_LEN, bit in 0u8..8) {
            let mac = MixMac::new(3, &[3]);
            let token = sign(&fields, &mac).unwrap();
            let now = ts(fields.expiry.timestamp().millis() - 1);
            let accepted = verify(Untrusted::new(&token), &mac, now).unwrap();
            prop_assert_eq!(accepted, CapFields { kid: 3, ..fields });
            let mut bytes = decode_token(&token);
            bytes[index] ^= 1 << bit;
            prop_assert_eq!(
                verify(Untrusted::new(&encode_token(&bytes)), &mac, now),
                Err(flipped_token_error(&bytes))
            );
        }

        /// Verifies: SEC-API-027
        #[test]
        fn stream_lifetime_never_exceeds_four_hours_and_grows_with_duration(
            millis in 0u64..=Duration::MAX_MILLIS,
            now_ms in -62_000_000_000_000_i64..250_000_000_000_000,
        ) {
            let now = ts(now_ms);
            let duration = Duration::from_millis(millis).unwrap();
            let expiry = lifetime(CapKind::Stream, Some(duration), now);
            let now_secs = now.millis().div_euclid(1_000);
            let ttl = expiry.unix_seconds().saturating_sub(now_secs);
            prop_assert!(ttl <= 14_400);
            prop_assert!(ttl >= 600 || now_secs > Timestamp::MAX.millis().div_euclid(1_000) - 600);
            let longer = Duration::from_millis(millis.saturating_add(1_000).min(Duration::MAX_MILLIS)).unwrap();
            let later = lifetime(CapKind::Stream, Some(longer), now);
            prop_assert!(later >= expiry);
        }

        /// Verifies: SEC-API-027
        #[test]
        fn artwork_bucket_alignment(
            now_ms in -62_000_000_000_000_i64..250_000_000_000_000,
            delta in 0i64..3_599_999,
        ) {
            let now = ts(now_ms);
            let first = lifetime(CapKind::Artwork, None, now);
            let bucket_start = now.millis().div_euclid(ARTWORK_BUCKET_MS) * ARTWORK_BUCKET_MS;
            let in_bucket = ts(now.millis().saturating_add(delta).min(bucket_start + ARTWORK_BUCKET_MS - 1).max(bucket_start));
            if in_bucket.millis().div_euclid(ARTWORK_BUCKET_MS) == now.millis().div_euclid(ARTWORK_BUCKET_MS) {
                prop_assert_eq!(lifetime(CapKind::Artwork, None, in_bucket), first);
            }
            prop_assert!(first.timestamp().millis().saturating_sub(now.millis()) <= ARTWORK_BUCKET_MS);
        }
    }
}
