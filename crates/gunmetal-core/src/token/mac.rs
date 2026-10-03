//! The MAC door the token layer uses: the core never sees key bytes.
//!
//! [`MacProvider`] is implemented by the secrets crate's key ring (WP-047).
//! Tests supply a fake that returns published HMAC-SHA-256 tags, so this
//! crate does not compute a MAC of its own (SEC-STD-018).

/// Computes HMAC-SHA-256 tags for a key identifier, without exposing the key.
///
/// `mac` returns [`None`] when `kid` is unknown or revoked. Callers treat
/// that the same as a tag mismatch, so verification does not distinguish
/// an unknown key from a bad token (SEC-API-026).
pub trait MacProvider {
    /// The key identifier new tokens are signed with.
    fn current_kid(&self) -> u8;

    /// The HMAC-SHA-256 tag of `msg` under key `kid`, or [`None`] if that
    /// key is not available.
    fn mac(&self, kid: u8, msg: &[u8]) -> Option<[u8; 32]>;
}
