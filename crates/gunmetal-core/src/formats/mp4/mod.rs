//! MP4 audio: the ISO base media file format (ISO/IEC 14496-12) as music
//! files use it.
//!
//! [`Probe`] is the entry point. It walks the boxes of a file through the
//! sans-I/O protocol, reading the `moov` box wherever it lies, and returns
//! the first audio track, its sample entry and codec configuration, the
//! iTunes-style item list and where the sample tables are.
//!
//! This file is a registry: it holds only module lines and re-exports.

pub mod audio;
pub mod boxes;
pub mod ilst;
pub mod probe;
pub mod sample_table;

pub use audio::{
    Alac, AudioEntry, AudioSpecificConfig, AudioTrack, ChunkOffsets, CodecConfig, Esds, Extension,
    Flac, Opus, SampleSizes, SampleTableRanges, Specific,
};
pub use boxes::FourCc;
pub use ilst::{IlstItem, ItemKey, ItemValue, PictureRef};
pub use probe::{FIXED_STEPS, FileType, Mp4Audio, Mp4Error, Mp4Problem, Probe, STEPS_PER_BYTE};
