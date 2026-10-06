//! The user log: what people and the household authored, kept in
//! `durable/log/<stream>/<yyyy-mm>.seg` (ADR 3, sections 3 to 8).
//!
//! This file is a registry: it holds only `mod` lines.

pub mod error;
mod ledger;
pub mod log;
mod place;
pub mod record;
pub mod scan;
#[cfg(test)]
mod testing;
