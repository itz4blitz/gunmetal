//! Authorisation: the one deny-by-default policy function, the `Permit`
//! only it can mint, and the types the server's sessions, authorisation
//! layer and stores share (WP-033).
//!
//! This file is a registry: it holds only module lines and re-exports.

pub mod action;
pub mod capability;
pub mod context;
pub mod decide;
#[cfg(test)]
mod fixtures;
pub mod library;
#[cfg(test)]
mod matrix;
pub mod principal;
#[cfg(test)]
mod properties;

pub use action::{Action, HOST_EQUIVALENT};
pub use capability::{Capability, CapabilitySet, Role, Tier};
pub use context::{Context, Network, RemoteAdmin};
pub use decide::{Denial, Owner, Permit, ResourceFacts, decide, may_issue};
pub use library::{HasLibrary, LibrarySet};
pub use principal::{
    DeviceClass, Elevation, Epoch, Principal, PrincipalFacts, PrincipalKind, Reach, Scope,
    SessionHandle, UserVerification,
};
