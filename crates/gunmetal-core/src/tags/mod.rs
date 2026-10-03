//! Tag mappers: one tag format onto [`TrackTags`](crate::catalog::TrackTags).
//!
//! This file is a registry: it holds only module lines and re-exports.

pub mod id3;

pub use id3::{FieldSource, FieldSources, Id3v1Field, Mapped, TagProblem, from_id3};
