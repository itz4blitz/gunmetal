//! Parsers for the HTTP request headers the server reads from untrusted
//! clients and proxies: [`range`] for `Range`, and [`forwarded`] for the
//! forwarding headers and the one path-class function that turns a socket
//! peer into a client address and its path class (SEC-OPS-037).
//!
//! This file is a registry: it holds only module lines.

pub mod forwarded;
pub mod range;
mod syntax;
