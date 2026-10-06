//! What the session layer asks about an account on every request.
//!
//! A session says who signed in and from which device. What that account
//! may do is not the session's to keep: its role, its grants and whether it
//! is still enabled can change while the session lives, and a change must
//! apply from the next request (SEC-IAM-076). So the session layer holds no
//! copy. It asks a [`Directory`] each time, and a session whose account the
//! directory no longer knows resolves to no principal at all.
//!
//! The accounts table arrives with the packages that create accounts; they
//! supply the directory.

use gunmetal_core::authz::{CapabilitySet, PrincipalKind, Reach};
use gunmetal_core::id::PublicId;

/// What an account is and may do at this moment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Standing {
    /// The kind of principal the account is.
    pub kind: PrincipalKind,
    /// The profile the account acts as, if it has one.
    pub profile: Option<PublicId>,
    /// The capabilities the account holds: its role's preset and any it was
    /// granted.
    pub capabilities: CapabilitySet,
    /// The libraries the account was granted.
    pub libraries: Vec<PublicId>,
    /// Where the person may use the server from.
    pub reach: Reach,
}

/// The server's accounts, as far as the session layer needs them.
pub trait Directory: Send + Sync {
    /// The standing of `account` now, or `None` when the account does not
    /// exist, is disabled or is being deleted. An implementation that
    /// cannot tell answers `None`: no standing, no principal.
    fn standing(&self, account: PublicId) -> Option<Standing>;
}
