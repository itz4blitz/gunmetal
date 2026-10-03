//! The context of a request: where it came from and what changed.
//!
//! Every signal here can only add a restriction: a step-up, or a refusal.
//! None can let a principal do something its capabilities do not allow,
//! because [`decide`](super::decide::decide) consults the context only after
//! the capability, scope and resource checks have passed (SEC-IAM-013,
//! SEC-API-075, SEC-HIS-001). Each field's first variant is the one that
//! restricts least.

use crate::client_context::PathClass;

/// Whether the request came from a network other than the one where the
/// session last passed user verification (SEC-IAM-013).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Network {
    /// The same network.
    Same,
    /// A different network: anything that runs the server needs fresh user
    /// verification.
    Changed,
}

/// Whether the owner allows administration from outside the home network
/// (A-116).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RemoteAdmin {
    /// Administration works on every path.
    Allowed,
    /// Administration is refused unless the request came from the server
    /// itself or the home network.
    Refused,
}

/// The context of one request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    /// How the request reached the server, from its
    /// [`ClientContext`](crate::client_context::ClientContext).
    pub path: PathClass,
    /// Whether the network changed since the last user verification.
    pub network: Network,
    /// Whether the owner allows administration from outside.
    pub remote_admin: RemoteAdmin,
}

impl Context {
    /// Whether the request came from the server itself or the home network.
    #[must_use]
    pub const fn is_home(&self) -> bool {
        matches!(self.path, PathClass::Loopback | PathClass::Home)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_loopback_and_home_paths_are_home() {
        let home: Vec<(PathClass, bool)> = [
            PathClass::Loopback,
            PathClass::Home,
            PathClass::Unknown,
            PathClass::Internet,
        ]
        .into_iter()
        .map(|path| {
            let context = Context {
                path,
                network: Network::Same,
                remote_admin: RemoteAdmin::Allowed,
            };
            (path, context.is_home())
        })
        .collect();
        assert_eq!(
            home,
            [
                (PathClass::Loopback, true),
                (PathClass::Home, true),
                (PathClass::Unknown, false),
                (PathClass::Internet, false),
            ]
        );
    }
}
