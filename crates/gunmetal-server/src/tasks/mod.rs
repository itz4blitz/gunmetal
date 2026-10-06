//! The task runner: named kinds, one-off requests with de-duplication,
//! checkpoints, cancel, and expiry sweeps (WP-070, ADM-095).
//!
//! Heavy work runs on operating-system threads. Persistence goes through
//! the cache's [`Store`](gunmetal_store::store::Store) and its [`Reply`]
//! type, so this module adds no async runtime. The R1 kinds are a closed
//! enum; a package may request a kind before a handler for it has merged.
//! A request for a kind with no handler stays queued as waiting. A repeated
//! request for the same kind, principal and path joins the running or
//! waiting job and returns its identifier (SEC-API-064).
//!
//! This file is a registry: it holds only `mod` lines and re-exports.

mod ctx;
mod error;
mod handler;
mod kind;
mod persist;
mod runner;
mod schema;
mod wait;

#[cfg(test)]
mod tests;

pub use ctx::TaskCtx;
pub use error::{Outcome, TaskError, TaskStatus};
pub use handler::Handler;
pub use kind::TaskKind;
pub use runner::{Limits, MAX_PATH, Runner, TaskHandle, TaskId, TaskSnapshot};
pub use schema::SCHEMA;
