//! Search on the device (DIS-083 to DIS-085, MUS-061).
//!
//! An [`Index`] is built in memory from the synced library, which holds
//! only what the profile may see, so no query reaches the server and none
//! is ever stored there. A query is cut into folded tokens
//! ([`collate::fold`](crate::collate::fold)) and matched against the tokens
//! of each document's title, artist, album, credits, genres and labels by
//! equality, by prefix and, for longer terms, within one edit. It is never
//! compiled into a pattern of any kind (SEC-STD-011), and only its first
//! [`MAX_QUERY_CHARS`] characters and [`MAX_TERMS`] terms are used
//! (SEC-API-063).
//!
//! A text with no letter or digit folds to no token. In the index and in a
//! query alike it is cut into its whitespace-separated words as they are
//! written instead, so that "÷", "!!!" and "( )" are found by their own
//! names ([`index`] has the rule).
//!
//! [`Index::to_bytes`] writes the compact segment the server can ship when
//! a device is too slow to build its own (API-SYNC-07), and
//! [`Index::from_bytes`] reads one back as untrusted input.
//!
//! Recent searches stay with the client. The mood field, role filters and
//! library scope are WP-147.
//!
//! This file is a registry: it holds only module lines and re-exports.

pub mod doc;
pub mod index;
pub mod query;
pub mod segment;
#[cfg(test)]
mod testing;

pub use doc::{DocKind, DocRef, SearchDoc};
pub use index::{Index, MAX_TOKEN_CHARS};
pub use query::{Hit, KindFilter, MAX_QUERY_CHARS, MAX_TERMS, MIN_TYPO_CHARS, Match};
pub use segment::{FIXED_STEPS, IndexError, STEPS_PER_OCTET};
