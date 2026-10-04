//! The identity store: `durable/identity.db`.
//!
//! This file is a registry: it holds only `mod` lines.

mod columns;
pub mod error;
pub mod mapping;
pub mod pre_principal;
pub mod settings;
pub mod store;
