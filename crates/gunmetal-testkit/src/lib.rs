//! Test support for Gunmetal's crates.
//!
//! Every test builds the bytes it parses in code, through the builders here
//! or a reference encoder written for that test. A builder shares no code
//! with the parser it feeds, so this crate depends on nothing in the
//! workspace. It is used only as a dev-dependency and is never published,
//! but it is covered and mutation-tested like any other crate.
//!
//! This file is a registry: it holds only `pub mod` lines, one per module.

pub mod ape;
pub mod bytes;
pub mod checksum;
pub mod clock;
pub mod flac;
pub mod flac_frames;
pub mod id3v1;
pub mod mp4;
pub mod mp4_samples;
pub mod mpa;
pub mod opus;
pub mod riff;
pub mod tempdir;
pub mod vorbis_comment;
