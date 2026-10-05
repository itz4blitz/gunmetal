//! Why the packager refused a track or could not write a segment.

use crate::formats::flac::frames::FlacFrameError;
use crate::parse::ParseFault;
use crate::problem::{Describe, Problem, ProblemCode};

use super::track::TrackField;

/// Why the packager refused a track or could not write a segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackError {
    /// A failure every parser shares: the step budget ran out, a limit was
    /// passed, or the segment's octets ended inside its first frame header.
    Fault(ParseFault),
    /// A value of the track does not fit the field of the box that holds
    /// it.
    Field {
        /// Which value it was.
        field: TrackField,
        /// The value.
        value: u64,
        /// The largest value the field holds.
        max: u64,
    },
    /// The index has no segment with this number.
    NoSegment {
        /// The segment asked for, counting from 0.
        segment: u32,
        /// How many segments the index has.
        segments: u64,
    },
    /// The octets given for a segment are not as many as the index says the
    /// segment takes.
    SourceLength {
        /// The segment, counting from 0.
        segment: u32,
        /// Where the index says the segment starts in the file.
        start: u64,
        /// Where the index says it ends.
        end: u64,
        /// How many octets were given.
        found: u64,
    },
    /// The segment's octets do not start with a FLAC frame.
    Flac(FlacFrameError),
    /// The frames of a segment skip frame or sample numbers, so frames are
    /// missing from the file and the segment cannot be played without a
    /// gap.
    Gap {
        /// The segment, counting from 0.
        segment: u32,
        /// How many frames carry a number past the one expected.
        gaps: u64,
    },
    /// The frames of a segment do not play for as long as the index says
    /// the segment does.
    Duration {
        /// The segment, counting from 0.
        segment: u32,
        /// The sample the index says the segment starts at.
        start: u64,
        /// The sample the index says it ends before.
        end: u64,
        /// How many samples its frames hold.
        found: u64,
    },
}

impl From<ParseFault> for PackError {
    fn from(fault: ParseFault) -> Self {
        Self::Fault(fault)
    }
}

impl From<FlacFrameError> for PackError {
    fn from(error: FlacFrameError) -> Self {
        Self::Flac(error)
    }
}

impl Describe for PackError {
    /// A file that could not be packaged.
    fn problem(&self) -> Problem {
        Problem {
            code: ProblemCode::FileUnreadable,
            args: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PackError;
    use crate::formats::flac::frames::{FlacFrameError, HeaderProblem};
    use crate::package::TrackField;
    use crate::parse::{LimitKind, ParseFault, ReadRequest};
    use crate::problem::{Arg, Describe, Problem, ProblemCode};

    const FAULT: ParseFault = ParseFault::BudgetExceeded { offset: 7 };

    #[test]
    fn keeps_a_shared_fault_as_it_is() {
        assert_eq!(PackError::from(FAULT), PackError::Fault(FAULT));
        let limit = ParseFault::LimitExceeded {
            limit: LimitKind::IndexEntries,
            value: 3,
            max: 2,
            offset: 9,
        };
        assert_eq!(PackError::from(limit), PackError::Fault(limit));
    }

    #[test]
    fn unwraps_a_fault_the_frame_index_reports_and_keeps_its_other_errors() {
        assert_eq!(
            PackError::from(FlacFrameError::Fault(FAULT)),
            PackError::Fault(FAULT)
        );
        let not_a_frame = FlacFrameError::NotAFrame {
            offset: 100,
            problem: HeaderProblem::ReservedBlockSize,
        };
        assert_eq!(PackError::from(not_a_frame), PackError::Flac(not_a_frame));
        let unrequested = FlacFrameError::Unrequested {
            requested: ReadRequest { offset: 4, len: 2 },
            offset: 4,
            len: 1,
        };
        assert_eq!(PackError::from(unrequested), PackError::Flac(unrequested));
    }

    #[test]
    fn describes_every_error_as_an_unreadable_file_with_its_reason() {
        let cases = [
            (PackError::Fault(FAULT), "fault"),
            (
                PackError::Field {
                    field: TrackField::Channels,
                    value: 9,
                    max: 8,
                },
                "track_field",
            ),
            (
                PackError::NoSegment {
                    segment: 3,
                    segments: 3,
                },
                "no_segment",
            ),
            (
                PackError::SourceLength {
                    segment: 0,
                    start: 100,
                    end: 114,
                    found: 13,
                },
                "source_length",
            ),
            (
                PackError::Flac(FlacFrameError::NotAFrame {
                    offset: 100,
                    problem: HeaderProblem::ReservedBitDepth,
                }),
                "flac_frames",
            ),
            (
                PackError::Gap {
                    segment: 1,
                    gaps: 2,
                },
                "frame_gap",
            ),
            (
                PackError::Duration {
                    segment: 0,
                    start: 0,
                    end: 8_000,
                    found: 8_192,
                },
                "duration",
            ),
        ];
        for (error, reason) in cases {
            assert_eq!(
                error.problem(),
                Problem {
                    code: ProblemCode::FileUnreadable,
                    args: vec![("reason", Arg::Name(reason))],
                },
                "{error:?}"
            );
        }
    }
}
