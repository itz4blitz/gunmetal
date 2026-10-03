//! The parse contract every parser in the core follows (the parsing contract
//! in `docs/security/media-and-parser-safety.md`, section 2).
//!
//! This file is a registry: it holds only module lines and re-exports.

pub mod budget;
pub mod capacity;
pub mod cursor;
pub mod fault;
pub mod limits;
#[cfg(test)]
mod small_stack;

pub use budget::{Budget, Depth};
pub use capacity::{bounded_capacity, bounded_vec};
pub use cursor::Cursor;
pub use fault::ParseFault;
pub use limits::{LimitError, LimitKind, Limits};
