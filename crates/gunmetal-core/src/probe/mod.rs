//! The file probe: one sans-I/O entry point that a worker drives for any
//! file in a library (LIB-019).
//!
//! [`probe`] detects the format from the file's content, runs that
//! format's container parser, gathers the raw tag blocks, and returns a
//! [`Probed`]: the catalogue's [`FileFacts`](crate::catalog::FileFacts)
//! (technical facts, trim, artwork references, lyrics sources, identity
//! inputs, the parser version and the octets read), the tag blocks in
//! their order of precedence, the seek index and the problems of every
//! part that was skipped. A failure in an optional part keeps the rest;
//! only a failure in what playback needs fails the file (SEC-MED-017).
//!
//! Tags are not mapped here: the mappers run on the blocks afterwards, so
//! the probe does not depend on the mapping rules. The audio is not hashed
//! here either: the probe says which octets to hash.
//!
//! This file is a registry: it holds only module lines and re-exports.

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles work with small, bounded values"
)]
mod cases;
mod draft;
mod facts;
mod flac;
mod machine;
mod mp4;
mod mpeg;
mod ogg;
mod pcm;

pub use facts::{PARSER_VERSIONS, PartProblem, ProbeError, Probed, SeekIndex, TagBlock};
pub use machine::{FIXED_STEPS, Probe, STEPS_PER_OCTET, probe};
