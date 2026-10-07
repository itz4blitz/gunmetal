//! The security audit log: hash-chained JSON-lines, an address side store,
//! and signed checkpoints (WP-069).
//!
//! This file is a registry: it holds only `mod` lines.

mod addresses;
mod chain;
mod encode;
pub mod error;
pub mod log;
mod parse;
pub mod record;
#[cfg(test)]
mod testing;
