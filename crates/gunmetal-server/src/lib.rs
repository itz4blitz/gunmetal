//! The Gunmetal server: the `gunmetal` binary's command line, its
//! configuration, the data directory, the logger, the event bus and the
//! application state that every feature module registers with.

pub mod access;
pub mod app;
pub mod bus;
pub mod cli;
pub mod clock;
pub mod config;
pub mod datadir;
pub mod host;
pub mod limiter;
pub mod listener;
pub mod log;
pub mod routes;
pub mod session;
#[cfg(test)]
mod testing;
pub mod verifier;
