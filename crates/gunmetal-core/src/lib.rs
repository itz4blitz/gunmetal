//! Shared core of Gunmetal.
//!
//! Everything in this crate is pure logic with no I/O, so it can be compiled
//! into the server, the native clients and the web client alike.

pub mod audit_event;
pub mod authz;
pub mod base64;
pub mod catalog;
pub mod client_context;
pub mod collate;
pub mod crypto;
pub mod ebml;
pub mod formats;
pub mod gain;
pub mod http;
pub mod id;
pub mod imagedata;
pub mod link;
pub mod logframe;
pub mod lyrics;
pub mod net;
pub mod otp;
pub mod parse;
pub mod path;
pub mod player;
pub mod problem;
pub mod queue;
pub mod ratelimit;
pub mod retention;
pub mod schema;
pub mod shuffle;
pub mod text;
pub mod time;
pub mod token;
pub mod untrusted;
pub mod values;
pub mod wire;
