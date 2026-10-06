//! Why the session layer refused an operation.

use gunmetal_durable::identity::error::IdentityError;

/// Why a session operation failed. No variant carries a token, a token's
/// hash or a client address, so an error can be logged whole.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionError {
    /// The identity store failed or refused a statement.
    Store(IdentityError),
    /// A stored device or session could not be read, so it was not trusted.
    Corrupt,
    /// The operating system could not supply randomness, so nothing was
    /// issued (SEC-STD-022).
    RandomnessUnavailable,
    /// The key that hashes session tokens could not be used.
    KeyUnavailable,
    /// The directory does not know the account: it does not exist, is
    /// disabled or is being deleted.
    UnknownAccount,
    /// The device is not enrolled for this account.
    UnknownDevice,
    /// A shared-browser session was asked for on a personal-class device. A
    /// browser in shared mode is always a limited-class device
    /// (SEC-CLI-024).
    SharedNeedsLimited,
}
