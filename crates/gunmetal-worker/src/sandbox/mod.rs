//! The worker sandbox: the launcher that starts a worker, and the
//! confinement a worker applies to itself before it reads a job.
//!
//! This file is a registry: module lines and re-exports only.

mod args;
mod confine;
mod exit;
mod kernel;
mod launch;
mod limits;
pub mod programs;
mod self_test;
mod syscalls;
mod tier;

pub use args::{ENTRY, Job, TypedArgs};
pub use confine::{ConfineError, Step, apply_landlock, apply_seccomp, confine};
pub use exit::{Cause, Exit};
pub use launch::{Child, Inherited, SpawnError, launch};
pub use limits::Profile;
pub use programs::Program;
pub use self_test::{answer_self_test, self_test};
pub use tier::{Enforced, Tier, TierReport};
