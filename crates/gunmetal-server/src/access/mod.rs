//! The authorisation layer: the way from a request's principal to a stored
//! object (SEC-TM-024, SEC-API-010).
//!
//! The decision itself is the core's, one deny-by-default pure function
//! (SEC-IAM-068). This layer loads the facts that function needs, the
//! principal's library grants, asks it, and enforces its answer; it never
//! decides anything the policy did not. A handler asks [`permit`] for a
//! `Permit`, hands it to the storage readers, which take nothing else, and
//! passes what they return through [`check`] and its siblings, which wrap
//! a row only when the permit's library set holds it.
//!
//! That it is the only way is held in parts. The compiler holds the readers
//! that take a `Permit`, since only the policy mints one. The architecture
//! test (`tests/storage_access.rs`) holds the ways round them that it
//! watches, the pre-principal lookups, the SQLite door's openers and the
//! policy function itself, to the modules listed for each, by reading the
//! server's sources as text. Nothing but review holds the identity store's
//! writer, which takes no `Permit`. [`policy`], [`grants`] and [`visible`]
//! each say what holds them.
//!
//! This file is a registry: it holds only module lines and re-exports.

pub mod grants;
pub mod policy;
#[cfg(test)]
mod testing;
pub mod visible;

pub use grants::{GrantError, LIBRARY_GRANTS, grant, granted, revoke};
pub use policy::{Ask, permit};
pub use visible::{Editable, Visible, check, check_all, check_edit, only_visible, visible_to};
