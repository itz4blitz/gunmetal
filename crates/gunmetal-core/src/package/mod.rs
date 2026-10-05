//! The audio packager: fragmented MP4 audio made by copying frames
//! (architecture record 4, MUS-230).
//!
//! A browser's Media Source Extensions want fragmented MP4, so that tracks
//! join without a gap and a seek lands on the sample asked for. The
//! packager writes it from a file the scan has already indexed, and never
//! decodes or re-encodes audio:
//!
//! - [`init_segment`] writes the initialisation segment of a [`PackTrack`]:
//!   the file type box and a movie box that describes the one audio track,
//!   with an edit list that carries the trim;
//! - [`media_segment`] writes media segment `n`: one movie fragment and the
//!   frames of that segment, found in the octets the host read for it.
//!
//! A [`FrameIndex`] says where each segment starts in the file and at which
//! sample, so any segment can be written on its own, in any order.
//!
//! Both functions are pure: the host reads the file and hands over octets.
//! They run only in a jailed worker process, never in the server process
//! (record 4, decisions 2 and 8). Every box is built from typed values; the
//! only octets copied from the file are whole frames into the media data
//! box, and the one data reference says the media is in the segment itself
//! (decision 6).
//!
//! This file is a registry: it holds only module lines and re-exports.

mod boxes;
mod error;
mod index;
mod init;
mod media;
#[cfg(test)]
mod testing;
mod track;

pub use error::PackError;
pub use index::{FrameIndex, IndexPoint};
pub use init::init_segment;
pub use media::{FIXED_STEPS, STEPS_PER_OCTET, media_segment};
pub use track::{PackTrack, TrackField};
