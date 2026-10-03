//! `WebAuthn` authenticator data, COSE keys, and the CBOR subset they use.
//!
//! This file is a registry: it holds only module lines and re-exports.

pub mod attestation;
pub mod authdata;
pub mod cbor;
pub mod cose;
#[cfg(test)]
mod test_support;

pub use attestation::{Attestation, attestation_object};
pub use authdata::{AttestedCredential, AuthData, Flags, auth_data};
pub use cbor::{AttestationField, Cbor, Item, WebauthnError, decode};
pub use cose::{CoseKey, cose_key};
