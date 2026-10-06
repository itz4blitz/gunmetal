//! Gunmetal's rebuildable SQLite cache (`cache/library.db`).
//!
//! One writer thread serialises every write, a small pool of read-only
//! connections serves reads from consistent snapshots, and the schema is
//! registered by parts whose digest decides whether the cache is kept or
//! discarded and rebuilt (ADM-058, ADM-077, ADM-080). Every connection is
//! opened by the one SQLite opener in `gunmetal-fs`, and every statement is
//! a static `Query` (SEC-API-066, SEC-TM-039, SEC-HIS-038, SEC-PRV-050).
//!
//! This file is a registry: it holds only `pub mod` lines.

pub mod catalog;
pub mod changelog;
mod columns;
pub mod readers;
pub mod reply;
pub mod schema;
pub mod store;
pub mod writer;
