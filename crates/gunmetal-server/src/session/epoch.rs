//! Revocation epochs: the number every session carries, and whose sessions
//! a bump ends (SEC-IAM-043).
//!
//! Each enrolled device has an epoch. A session carries the epoch its
//! device had when the session was issued, and so does the principal a
//! request resolves to. Whatever ends a session moves its device's epoch
//! on: signing out, signing in again on the same device, running out of
//! time, a revoked device, and everything that ends all of an account's
//! sessions at once.

use gunmetal_core::id::PublicId;

/// Whose sessions an epoch bump ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpochScope {
    /// Every session of the account, on every device: a credential added or
    /// removed, sign out everywhere, a disabled account.
    Account(PublicId),
    /// Every session of one device: a revoked device.
    Device(PublicId),
}
