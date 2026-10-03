//! Shared core of Gunmetal.
//!
//! Everything in this crate is pure logic with no I/O, so it can be compiled
//! into the server, the native clients and the web client alike.

pub mod base64;
pub mod ebml;
pub mod link;
pub mod net;
pub mod text;
pub mod time;
pub mod untrusted;
pub mod values;
