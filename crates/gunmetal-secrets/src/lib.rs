//! Gunmetal's secrets and keys door.
//!
//! Security randomness comes from the operating system's CSPRNG through the
//! one function in [`random`] (SEC-STD-022), public identifiers are minted
//! only by [`mint`] (SEC-HIS-012), the root secret is generated and loaded
//! by [`root`] through the data-root handle (SEC-OPS-011, SEC-OPS-012),
//! rotating keys follow a [`schedule`] (SEC-API-030), and every secret
//! value is held in a [`Secret`] that cannot be printed, serialised or
//! compared with `==` (SEC-OPS-013, SEC-IAM-095).
//!
//! Every other key is derived from the root secret with HKDF, one per
//! purpose (SEC-OPS-015). A [`keyring`] computes the keyed hashes other
//! crates ask for, and the [`vault`] seals the secrets the server must
//! replay to other systems (SEC-OPS-017); no key leaves this crate. The
//! algorithms themselves are used only in [`crypto`] (SEC-STD-018).

pub mod crypto;
pub mod keyring;
pub mod mint;
pub mod random;
pub mod root;
pub mod schedule;
pub mod secret;
#[cfg(test)]
mod testing;
pub mod vault;

pub use secret::Secret;
