//! Values the policy's tests share.

use super::capability::CapabilitySet;
use super::context::{Context, Network, RemoteAdmin};
use super::principal::{
    DeviceClass, Elevation, PrincipalFacts, PrincipalKind, Reach, UserVerification,
};
use crate::client_context::PathClass;
use crate::id::{IdKind, PublicId};

/// The identifier of kind `kind` whose symbols are `n` in decimal, padded
/// with zeros.
fn id(prefix: &str, kind: IdKind, n: u8) -> PublicId {
    PublicId::parse(&format!("{prefix}_{n:026}"), kind).unwrap()
}

/// Library number `n`.
pub(crate) fn library(n: u8) -> PublicId {
    id("lib", IdKind::Library, n)
}

/// Account number `n`.
pub(crate) fn account(n: u8) -> PublicId {
    id("usr", IdKind::User, n)
}

/// Profile number `n`.
pub(crate) fn profile(n: u8) -> PublicId {
    id("prf", IdKind::Profile, n)
}

/// A principal of kind `kind` holding `capabilities`, acting as account 1
/// and profile 1, granted library 1, on a personal device in an elevated
/// session with a fresh user verification, with no location restriction
/// and no credential scope.
pub(crate) fn facts(kind: PrincipalKind, capabilities: CapabilitySet) -> PrincipalFacts {
    PrincipalFacts {
        kind,
        account: Some(account(1)),
        profile: Some(profile(1)),
        capabilities,
        libraries: vec![library(1)],
        device: DeviceClass::Personal,
        elevation: Elevation::Elevated,
        verification: UserVerification::Fresh,
        reach: Reach::Anywhere,
        scope: None,
    }
}

/// A request from the server itself that restricts nothing.
pub(crate) fn at_home() -> Context {
    at(PathClass::Loopback)
}

/// A request over `path` from the same network, with remote administration
/// allowed.
pub(crate) fn at(path: PathClass) -> Context {
    Context {
        path,
        network: Network::Same,
        remote_admin: RemoteAdmin::Allowed,
    }
}
