//! Gunmetal's worker process.
//!
//! Untrusted media is parsed only in a separate worker process, never in
//! the server (SEC-MED-018). This crate holds the one place a process
//! starts, the sandbox launcher, and the confinement the worker applies to
//! itself before it reads a job (SEC-MED-022, SEC-MED-063).
//!
//! The crate builds everywhere so the workspace builds on development
//! machines. Its confinement is implemented for Linux, the only system
//! R1's server runs on (owner decision D-09): elsewhere every floor step
//! is refused, so a worker ends without reading a job rather than serve
//! unconfined (SEC-MED-024).

pub mod host;
pub mod ipc;
pub mod sandbox;
