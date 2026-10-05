//! The frame index a packager cuts segments by.
//!
//! The index is made when a file is scanned, from the format's own frame
//! index, and holds one point for each place a segment starts. A point is
//! a frame boundary: the file offset of a frame's first octet and the
//! decode time of its first sample. Segment `n` is the frames from point
//! `n` to the point after it, so the packager can write any segment from
//! that range of the file alone, without the segments before it.

use super::boxes::count;

/// A frame boundary a segment starts or ends at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexPoint {
    /// Where the frame starts, in octets from the start of the file.
    pub offset: u64,
    /// The decode time of the frame's first sample, in units of the track's
    /// timescale: samples from the start of the stream, before any trim.
    pub sample: u64,
}

/// Where each segment of a track starts, and where the last one ends.
///
/// Nothing is checked when an index is made. A segment is checked when it
/// is written: its octets must be as many as its two points say, and its
/// frames must play for as long as they say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameIndex {
    /// Where each segment starts, in order.
    pub starts: Vec<IndexPoint>,
    /// Where the last segment ends: the end of the audio.
    pub end: IndexPoint,
}

impl FrameIndex {
    /// How many segments the index describes.
    #[must_use]
    pub fn segments(&self) -> u64 {
        count(&self.starts)
    }

    /// Where segment `n` starts and where it ends, counting segments from
    /// 0, or `None` when the index has no such segment.
    #[must_use]
    pub fn segment(&self, n: u32) -> Option<(IndexPoint, IndexPoint)> {
        // A number too large for this target's usize names no segment.
        let at = usize::try_from(n).unwrap_or(usize::MAX);
        let start = *self.starts.get(at)?;
        let end = self
            .starts
            .get(at.saturating_add(1))
            .copied()
            .unwrap_or(self.end);
        Some((start, end))
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{index_of, point};
    use super::{FrameIndex, IndexPoint};

    #[test]
    fn a_segment_runs_from_its_start_to_the_next_and_the_last_to_the_end() {
        let index = index_of(&[(100, 0), (114, 4_096), (128, 8_192)], (142, 10_240));
        assert_eq!(index.segments(), 3);
        assert_eq!(index.segment(0), Some((point(100, 0), point(114, 4_096))));
        assert_eq!(
            index.segment(1),
            Some((point(114, 4_096), point(128, 8_192)))
        );
        assert_eq!(
            index.segment(2),
            Some((point(128, 8_192), point(142, 10_240)))
        );
    }

    #[test]
    fn a_file_of_one_segment_runs_from_its_start_to_the_end() {
        let index = index_of(&[(0, 0)], (14, 4_096));
        assert_eq!(index.segments(), 1);
        assert_eq!(index.segment(0), Some((point(0, 0), point(14, 4_096))));
    }

    #[test]
    fn has_no_segment_past_the_last() {
        let index = index_of(&[(100, 0), (114, 4_096)], (128, 8_192));
        assert_eq!(index.segment(2), None);
        assert_eq!(index.segment(3), None);
        assert_eq!(index.segment(u32::MAX), None);
        let empty = index_of(&[], (0, 0));
        assert_eq!(empty.segments(), 0);
        assert_eq!(empty.segment(0), None);
    }

    #[test]
    fn holds_the_points_it_is_given_as_they_are() {
        assert_eq!(
            index_of(&[(5, 6)], (7, 8)),
            FrameIndex {
                starts: vec![IndexPoint {
                    offset: 5,
                    sample: 6,
                }],
                end: IndexPoint {
                    offset: 7,
                    sample: 8,
                },
            }
        );
    }
}
