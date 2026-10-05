//! Sessions and revocation epochs (WP-062, ADR 7): the browser sessions
//! held in the session cookie, the devices they belong to, the epoch that
//! ends them, and the principal extractor the request pipeline calls.
//!
//! This file is a registry: it holds only module lines.

pub mod cookie;
pub mod directory;
pub mod epoch;
pub mod error;
pub mod hook;
pub mod kind;
pub mod lifetime;
mod record;
pub mod schema;
pub mod sessions;
#[cfg(test)]
mod tests;
mod token;
