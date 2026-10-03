//! The parse contract every parser in the core follows (the parsing contract
//! in `docs/security/media-and-parser-safety.md`, section 2).
//!
//! Every parse runs under three explicit inputs: [`Limits`], a step
//! [`Budget`] and a nesting [`Depth`]. It reads through a checked
//! [`Cursor`], sizes any allocation from a declared count only through
//! [`bounded_vec`], reports the failures every format shares as a typed
//! [`ParseFault`], and reaches file octets only through the sans-I/O
//! protocol in [`sansio`], whose [`ReadGuard`] refuses any read the limits
//! forbid.
//!
//! This file is a registry: it holds only module lines and re-exports.

pub mod budget;
pub mod capacity;
pub mod cursor;
pub mod fault;
pub mod limits;
pub mod sansio;
#[cfg(test)]
mod small_stack;

pub use budget::{Budget, Depth};
pub use capacity::{bounded_capacity, bounded_vec};
pub use cursor::Cursor;
pub use fault::ParseFault;
pub use limits::{LimitError, LimitKind, Limits};
pub use sansio::{DriveError, ReadGuard, ReadRequest, SansIo, Step, Window, drive};
