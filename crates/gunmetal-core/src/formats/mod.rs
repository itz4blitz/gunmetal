//! The file formats on the allowlist: detecting which one a file is
//! (SEC-MED-011), and one parser module per format.
//!
//! This file is a registry: it holds only module lines and re-exports.

pub mod aiff;
pub mod detect;
pub mod riff;
