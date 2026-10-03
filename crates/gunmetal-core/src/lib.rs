//! Shared core of Gunmetal.
//!
//! Everything in this crate is pure logic with no I/O, so it can be compiled
//! into the server, the native clients and the web client alike.

mod crypto;
pub mod ebml;
pub mod schema;
