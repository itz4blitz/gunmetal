//! Gunmetal's durable state: what a rescan of the media files cannot
//! recreate, kept apart from the rebuildable cache (ADR 3).
//!
//! This file is a registry: it holds only `mod` lines.

pub mod audit;
pub mod identity;
pub mod userlog;
