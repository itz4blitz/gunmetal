//! Session tokens: 256 bits from the operating system's CSPRNG
//! (SEC-IAM-037).
//!
//! A token is drawn through the secrets crate's one randomness function,
//! written into a cookie as unpadded URL-safe base64, and kept by the
//! server only as a keyed hash. A [`Token`] cannot be printed, compared or
//! copied: the only ways out of it are the cookie text and the bytes handed
//! to the hash.

use gunmetal_core::base64::{self, Alphabet};
use gunmetal_core::token::SESSION_LEN;
use gunmetal_core::untrusted::Untrusted;
use gunmetal_secrets::random::Random;

use super::error::SessionError;

/// A session token's 32 bytes.
pub(super) struct Token([u8; SESSION_LEN]);

impl Token {
    /// A fresh token from `random`.
    pub(super) fn draw(random: &dyn Random) -> Result<Self, SessionError> {
        let mut bytes = [0; SESSION_LEN];
        random
            .fill(&mut bytes)
            .or(Err(SessionError::RandomnessUnavailable))
            .map(|()| Self(bytes))
    }

    /// The token a cookie value spells, if it is the one written form of
    /// 32 bytes: unpadded URL-safe base64 with no stray bits.
    pub(super) fn parse(presented: &[u8]) -> Option<Self> {
        base64::decode(Untrusted::new(presented), Alphabet::UrlSafe, SESSION_LEN)
            .ok()
            .filter(|bytes| base64::encode(bytes, Alphabet::UrlSafe).as_bytes() == presented)
            .and_then(|bytes| <[u8; SESSION_LEN]>::try_from(bytes).ok())
            .map(Self)
    }

    /// The token as a cookie value.
    pub(super) fn text(&self) -> String {
        base64::encode(&self.0, Alphabet::UrlSafe)
    }

    /// The token's bytes, for the keyed hash.
    pub(super) const fn bytes(&self) -> &[u8] {
        &self.0
    }
}
