//! Gunmetal's secrets and keys door.
//!
//! Security randomness comes from the operating system's CSPRNG through the
//! one function in [`random`] (SEC-STD-022), public identifiers are minted
//! only by [`mint`] (SEC-HIS-012), the root secret is generated and loaded
//! by [`root`] through the data-root handle (SEC-OPS-011, SEC-OPS-012), and
//! every secret value is held in a [`Secret`] that cannot be printed,
//! serialised or compared with `==` (SEC-OPS-013, SEC-IAM-095).

pub mod mint;
pub mod random;
pub mod root;
pub mod secret;

pub use secret::Secret;
