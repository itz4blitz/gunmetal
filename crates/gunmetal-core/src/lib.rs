//! Shared core of Gunmetal.
//!
//! Everything in this crate is pure logic with no I/O, so it can be compiled
//! into the server, the native clients and the web client alike.

pub mod audit_event;
pub mod base64;
pub mod client_context;
pub mod collate;
pub mod crypto;
pub mod ebml;
pub mod id;
pub mod link;
pub mod net;
pub mod parse;
pub mod problem;
pub mod schema;
pub mod text;
pub mod time;
pub mod untrusted;
pub mod values;
