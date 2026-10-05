//! Media segments: one movie fragment and the frames it describes
//! (ISO/IEC 14496-12, sections 8.8.4 to 8.8.12).
//!
//! [`media_segment`] writes segment `n` of a track from the octets of the
//! file the index says that segment takes. The segment is a `moof` box,
//! whose one run gives the size and the duration of every frame, and an
//! `mdat` box that holds the frames as they are in the file. The decode
//! time of the fragment is the sample the index gives for the segment, so
//! segments can be written and appended in any order.
//!
//! # Finding the frames
//!
//! A FLAC frame does not say how long it is, so the frames of a segment
//! are found as the scan finds them, by their headers, with the frame
//! indexer of the FLAC parser. The segment's octets must start with a
//! frame, and a frame runs to the next header that continues the stream or
//! to the end of the octets. A frame's duration is the distance to the
//! next frame's first sample, which is its block size. A segment whose
//! frames skip numbers is refused: frames are missing from the file, and
//! no gapless join can be promised across them.
//!
//! The octets are then checked against the index: they must be as many as
//! its two points for the segment are apart, and the frames must play for
//! as many samples as the points are apart. A segment that disagrees with
//! its index is refused rather than written with times that would overlap
//! the next segment or leave a hole before it.
//!
//! # What it costs
//!
//! Finding the frames costs what the frame indexer charges: one step for
//! each candidate header, and so at most one for each octet. Copying costs
//! one step for each frame, and a frame takes at least one octet. A
//! segment therefore costs at most [`STEPS_PER_OCTET`] steps for each
//! octet of its source and [`FIXED_STEPS`] more (SEC-MED-007). One call
//! takes at most the per-file read cap of octets, and one segment holds at
//! most as many frames as an index may hold entries.

use std::ops::Range;

use crate::formats::flac::frames::{
    FlacFrameError, FrameContext, FrameEntry, FrameIndex as FlacIndex, FrameIndexer,
};
use crate::parse::{Budget, LimitKind, Limits, ReadRequest, SansIo, Step, Window};

use super::boxes::{Body, TRACK_ID, count, narrow};
use super::error::PackError;
use super::index::FrameIndex;
use super::track::PackTrack;

/// Steps a segment may cost for each octet of its source (SEC-MED-007).
pub const STEPS_PER_OCTET: u64 = 2;

/// Steps a segment may cost on top of [`STEPS_PER_OCTET`] (SEC-MED-007).
pub const FIXED_STEPS: u64 = 0;

/// The octets of a media segment that are neither an entry of its run nor
/// media data: the headers and the fixed fields of `moof`, `mfhd`, `traf`,
/// `tfhd`, `tfdt` and `trun`, 88 octets, and the 8 of the `mdat` header.
const FIXED_OCTETS: u64 = 96;

/// The `tfhd` flag `default-base-is-moof`: data offsets count from the
/// first octet of the `moof` box.
const BASE_IS_MOOF: u32 = 0x02_0000;

/// The `trun` flags of a run that gives a data offset and, for every
/// sample, a duration and a size.
const RUN_FLAGS: u32 = 0x00_0301;

/// One frame of a segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Frame {
    /// How many octets it takes.
    size: u32,
    /// How many samples it plays for.
    duration: u32,
}

/// Writes media segment `n` of `track`, counting from 0, from `source`:
/// the octets of the file from the point where `index` says the segment
/// starts to the point where it says the segment ends.
///
/// # Errors
///
/// - [`PackError::NoSegment`] when `index` has no segment `n`.
/// - [`PackError::SourceLength`] when `source` is not as long as the
///   segment's two points are apart.
/// - [`PackError::Fault`] when `source` is longer than one parse may read,
///   holds more frames than an index may hold entries, is empty or ends
///   inside its first frame header, or when `budget` runs out.
/// - [`PackError::Flac`] when `source` does not start with a FLAC frame.
/// - [`PackError::Gap`] when its frames skip frame or sample numbers.
/// - [`PackError::Duration`] when its frames do not play for as many
///   samples as the segment's two points are apart.
pub fn media_segment(
    track: &PackTrack,
    index: &FrameIndex,
    n: u32,
    source: &[u8],
    limits: &Limits,
    budget: &mut Budget,
) -> Result<Vec<u8>, PackError> {
    let Some((start, end)) = index.segment(n) else {
        return Err(PackError::NoSegment {
            segment: n,
            segments: index.segments(),
        });
    };
    let len = count(source);
    if end.offset.checked_sub(start.offset) != Some(len) {
        return Err(PackError::SourceLength {
            segment: n,
            start: start.offset,
            end: end.offset,
            found: len,
        });
    }
    limits.check(LimitKind::FileBytes, len, start.offset)?;
    let span = start.offset..end.offset;
    let frames = flac_frames(track, n, source, &span, limits, budget)?;
    let played = frames.iter().fold(0_u64, |played, frame| {
        played.saturating_add(u64::from(frame.duration))
    });
    if end.sample.checked_sub(start.sample) != Some(played) {
        return Err(PackError::Duration {
            segment: n,
            start: start.sample,
            end: end.sample,
            found: played,
        });
    }
    Ok(fragment(n, start.sample, &frames, source))
}

/// The frames of segment `segment` of a FLAC track, whose octets `source`
/// lie at `span` in the file: the size and the duration of each, in order.
fn flac_frames(
    track: &PackTrack,
    segment: u32,
    source: &[u8],
    span: &Range<u64>,
    limits: &Limits,
    budget: &mut Budget,
) -> Result<Vec<Frame>, PackError> {
    let context = FrameContext {
        stream_sample_rate: Some(track.info.sample_rate),
        stream_bits: Some(track.info.bits_per_sample),
    };
    let found = find_frames(context, source, span, limits, budget)?;
    // The indexer thins what it keeps past this limit, and a thinned index
    // no longer says where every frame starts.
    limits.check(LimitKind::IndexEntries, found.frames, span.start)?;
    if found.gaps != 0 {
        return Err(PackError::Gap {
            segment,
            gaps: found.gaps,
        });
    }
    // The last frame runs to the end of the octets and of the samples.
    let end = FrameEntry {
        offset: span.end,
        first_sample: found.end_sample,
    };
    let nexts = found.entries.iter().skip(1).chain([&end]);
    let mut frames = Vec::new();
    for (frame, next) in found.entries.iter().zip(nexts) {
        budget.charge(1, frame.offset)?;
        frames.push(Frame {
            size: narrow(next.offset.saturating_sub(frame.offset)),
            duration: narrow(next.first_sample.saturating_sub(frame.first_sample)),
        });
    }
    Ok(frames)
}

/// Runs the FLAC frame indexer over `source`, the octets of the file at
/// `span`, answering its read requests from memory. Every offset it
/// reports is an offset in the file.
fn find_frames(
    context: FrameContext,
    source: &[u8],
    span: &Range<u64>,
    limits: &Limits,
    budget: &mut Budget,
) -> Result<FlacIndex, FlacFrameError> {
    let mut indexer = FrameIndexer::new(context, span.clone(), limits, budget);
    let mut window = Window::start(span.end);
    loop {
        match indexer.resume(window) {
            Step::Done(outcome) => return outcome,
            Step::Need(request) => {
                window = Window {
                    offset: request.offset,
                    bytes: requested(source, span.start, request),
                    file_len: span.end,
                };
            }
        }
    }
}

/// The octets `request` asks for, out of `source`, whose first octet lies
/// at `start` in the file. A request that reaches outside `source` gets no
/// octets, which the indexer refuses.
fn requested(source: &[u8], start: u64, request: ReadRequest) -> &[u8] {
    let from = usize::try_from(request.offset.saturating_sub(start)).unwrap_or(usize::MAX);
    let len = usize::try_from(request.len).unwrap_or(usize::MAX);
    source
        .get(from..)
        .and_then(|rest| rest.get(..len))
        .unwrap_or_default()
}

/// The media segment numbered `segment`, counting from 0: a movie fragment
/// that starts at `decode_time` and describes `frames`, and the media data
/// box that holds `source`, their octets.
fn fragment(segment: u32, decode_time: u64, frames: &[Frame], source: &[u8]) -> Vec<u8> {
    // Every entry of the run takes 8 octets, and the media data starts
    // after the `moof` box and the `mdat` header.
    let data_offset = count(frames).saturating_mul(8).saturating_add(FIXED_OCTETS);
    let run = frames
        .iter()
        .fold(
            Body::full(0, RUN_FLAGS)
                .u32(narrow(count(frames)))
                .u32(narrow(data_offset)),
            |run, frame| run.u32(frame.duration).u32(frame.size),
        )
        .boxed(*b"trun");
    let header = Body::full(0, BASE_IS_MOOF).u32(TRACK_ID).boxed(*b"tfhd");
    let time = Body::full(1, 0).u64(decode_time).boxed(*b"tfdt");
    let track = Body::new()
        .bytes(&header)
        .bytes(&time)
        .bytes(&run)
        .boxed(*b"traf");
    // Fragments are numbered from 1.
    let number = Body::full(0, 0)
        .u32(segment.saturating_add(1))
        .boxed(*b"mfhd");
    let mut whole = Body::new().bytes(&number).bytes(&track).boxed(*b"moof");
    whole.extend_from_slice(&narrow(count(source).saturating_add(8)).to_be_bytes());
    whole.extend_from_slice(b"mdat");
    whole.extend_from_slice(source);
    whole
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use super::super::testing::{cd_frame, cd_frames, cd_track, index_of, on_small_stack, point};
    use super::{FIXED_STEPS, STEPS_PER_OCTET, media_segment};
    use crate::formats::flac::frames::{FlacFrameError, FrameContext, FrameIndexer, HeaderProblem};
    use crate::package::{FrameIndex, PackError};
    use crate::parse::{Budget, LimitKind, Limits, ParseFault, drive};
    use gunmetal_testkit::flac_frames::{Header, frame};
    use gunmetal_testkit::fmp4::{Fragment, FragmentError, Sample, read_fragment};
    use proptest::collection::vec;
    use proptest::prelude::*;

    /// `parts` joined, so a literal box reads one field per line.
    fn cat(parts: &[&[u8]]) -> Vec<u8> {
        parts.concat()
    }

    fn len(octets: &[u8]) -> u64 {
        u64::try_from(octets.len()).unwrap()
    }

    /// `Limits::DEFAULT` with `kind` lowered to `value`.
    fn lowered(kind: LimitKind, value: u64) -> Limits {
        Limits::DEFAULT
            .with_override(kind, value)
            .expect("the test lowers a limit")
    }

    /// Writes segment `n` of `index` from `source` under `limits`, with the
    /// budget the module documents for that many octets.
    fn package_under(
        index: &FrameIndex,
        n: u32,
        source: &[u8],
        limits: &Limits,
    ) -> Result<Vec<u8>, PackError> {
        let mut budget = Budget::for_input(len(source), STEPS_PER_OCTET, FIXED_STEPS);
        media_segment(&cd_track(), index, n, source, limits, &mut budget)
    }

    /// Writes segment `n` of `index` from `source` under the default
    /// limits.
    fn package(index: &FrameIndex, n: u32, source: &[u8]) -> Result<Vec<u8>, PackError> {
        package_under(index, n, source, &Limits::DEFAULT)
    }

    /// Writes segment `n` of `index` from `source` with `steps` steps to
    /// spend, and returns what it wrote with the steps left over.
    fn package_with(
        index: &FrameIndex,
        n: u32,
        source: &[u8],
        steps: u64,
    ) -> (Result<Vec<u8>, PackError>, u64) {
        let mut budget = Budget::for_input(0, 0, steps);
        let written = media_segment(&cd_track(), index, n, source, &Limits::DEFAULT, &mut budget);
        (written, budget.remaining())
    }

    /// What the testkit's reader makes of what [`package`] wrote.
    fn read_back(
        index: &FrameIndex,
        n: u32,
        source: &[u8],
    ) -> Result<Result<Fragment, FragmentError>, PackError> {
        package(index, n, source).map(|segment| read_fragment(&segment))
    }

    /// The fragment numbered `sequence` that starts at `decode_time` and
    /// holds `samples`, each a duration and its octets.
    fn fragment(sequence: u32, decode_time: u64, samples: &[(u32, &[u8])]) -> Fragment {
        Fragment {
            sequence,
            track: 1,
            decode_time,
            samples: samples
                .iter()
                .map(|&(duration, data)| Sample {
                    duration,
                    data: data.to_vec(),
                })
                .collect(),
        }
    }

    /// The steps the FLAC frame indexer alone spends on `source`, measured
    /// through the indexer's own entry point.
    fn finding_steps(source: &[u8]) -> u64 {
        let context = FrameContext {
            stream_sample_rate: None,
            stream_bits: None,
        };
        let mut budget = Budget::for_input(0, 0, 1_000_000);
        let indexer = FrameIndexer::new(context, 0..len(source), &Limits::DEFAULT, &mut budget);
        drive(indexer, source, &Limits::DEFAULT)
            .expect("the indexer reads within the limits")
            .expect("the test's source starts with a frame");
        1_000_000 - budget.remaining()
    }

    /// The test file: its three frames, and all 42 of its octets, which lie
    /// at 100 to 142 of the imagined file.
    fn cd_file() -> ([Vec<u8>; 3], Vec<u8>) {
        let frames = cd_frames();
        let file = frames.concat();
        (frames, file)
    }

    #[test]
    fn the_test_frames_are_fourteen_octets_each() {
        let (frames, file) = cd_file();
        let lengths: Vec<usize> = frames.iter().map(Vec::len).collect();
        assert_eq!((lengths, file.len()), (vec![14, 14, 14], 42));
    }

    #[test]
    fn writes_two_frames_as_one_fragment_and_its_media_data() {
        let ([first, second, _], file) = cd_file();
        let index = index_of(&[(100, 0), (128, 8_192)], (142, 10_240));
        let expected = cat(&[
            b"\x00\x00\x00\x68moof",
            b"\x00\x00\x00\x10mfhd\x00\x00\x00\x00",
            &[0, 0, 0, 1], // segment 0 is fragment 1
            b"\x00\x00\x00\x50traf",
            b"\x00\x00\x00\x10tfhd\x00\x02\x00\x00", // default-base-is-moof
            &[0, 0, 0, 1],                           // track ID
            b"\x00\x00\x00\x14tfdt\x01\x00\x00\x00", // version 1
            &[0; 8],                                 // decode time 0
            b"\x00\x00\x00\x24trun\x00\x00\x03\x01", // offset, durations, sizes
            &[0, 0, 0, 2],                           // two samples
            &[0, 0, 0, 0x70],                        // data offset: 104 + 8
            &[0, 0, 0x10, 0, 0, 0, 0, 14],           // 4,096 samples, 14 octets
            &[0, 0, 0x10, 0, 0, 0, 0, 14],
            b"\x00\x00\x00\x24mdat",
            &first,
            &second,
        ]);
        assert_eq!(expected.len(), 140);
        assert_eq!(package(&index, 0, &file[..28]), Ok(expected));
    }

    #[test]
    fn writes_the_last_segment_with_its_short_frame() {
        let ([_, _, last], file) = cd_file();
        let index = index_of(&[(100, 0), (128, 8_192)], (142, 10_240));
        let expected = cat(&[
            b"\x00\x00\x00\x60moof",
            b"\x00\x00\x00\x10mfhd\x00\x00\x00\x00",
            &[0, 0, 0, 2], // segment 1 is fragment 2
            b"\x00\x00\x00\x48traf",
            b"\x00\x00\x00\x10tfhd\x00\x02\x00\x00",
            &[0, 0, 0, 1], // track ID
            b"\x00\x00\x00\x14tfdt\x01\x00\x00\x00",
            &[0, 0, 0, 0, 0, 0, 0x20, 0], // decode time 8,192
            b"\x00\x00\x00\x1Ctrun\x00\x00\x03\x01",
            &[0, 0, 0, 1],                 // one sample
            &[0, 0, 0, 0x68],              // data offset: 96 + 8
            &[0, 0, 0x08, 0, 0, 0, 0, 14], // 2,048 samples, 14 octets
            b"\x00\x00\x00\x16mdat",
            &last,
        ]);
        assert_eq!(expected.len(), 118);
        assert_eq!(package(&index, 1, &file[28..]), Ok(expected));
    }

    /// Verifies: SEC-MED-032
    #[test]
    fn every_segment_reads_back_to_the_frames_that_went_in() {
        let ([first, second, last], file) = cd_file();
        // One frame a segment: boundaries at the first and at the last
        // frame.
        let each = index_of(&[(100, 0), (114, 4_096), (128, 8_192)], (142, 10_240));
        assert_eq!(
            read_back(&each, 0, &file[..14]),
            Ok(Ok(fragment(1, 0, &[(4_096, first.as_slice())])))
        );
        assert_eq!(
            read_back(&each, 1, &file[14..28]),
            Ok(Ok(fragment(2, 4_096, &[(4_096, second.as_slice())])))
        );
        assert_eq!(
            read_back(&each, 2, &file[28..]),
            Ok(Ok(fragment(3, 8_192, &[(2_048, last.as_slice())])))
        );
        // The whole file as one segment.
        let whole = index_of(&[(100, 0)], (142, 10_240));
        assert_eq!(
            read_back(&whole, 0, &file),
            Ok(Ok(fragment(
                1,
                0,
                &[
                    (4_096, first.as_slice()),
                    (4_096, second.as_slice()),
                    (2_048, last.as_slice())
                ]
            )))
        );
    }

    #[test]
    fn writes_a_file_of_one_frame() {
        // The frame is the file's first and its last, and so may be short.
        let only = cd_frame(0, 0b1011);
        let index = index_of(&[(0, 0)], (14, 2_048));
        assert_eq!(
            read_back(&index, 0, &only),
            Ok(Ok(fragment(1, 0, &[(2_048, only.as_slice())])))
        );
    }

    #[test]
    fn starts_a_fragment_at_a_time_past_32_bits() {
        let ([first, ..], _) = cd_file();
        // Sample 2^32 + 5, as a stream of more than a day at 44.1 kHz has.
        let index = index_of(&[(100, 4_294_967_301)], (114, 4_294_971_397));
        assert_eq!(
            read_back(&index, 0, &first),
            Ok(Ok(fragment(1, 4_294_967_301, &[(4_096, first.as_slice())])))
        );
    }

    #[test]
    fn numbers_a_fragment_one_past_its_segment() {
        let ([first, ..], _) = cd_file();
        let mut index = index_of(&[], (0, 0));
        // 300 segments of one frame each, back to back.
        for n in 0..300 {
            index.starts.push(point(n * 14, n * 4_096));
        }
        index.end = point(300 * 14, 300 * 4_096);
        assert_eq!(
            read_back(&index, 299, &first),
            Ok(Ok(fragment(300, 299 * 4_096, &[(4_096, first.as_slice())])))
        );
    }

    #[test]
    fn finds_the_same_frames_when_the_indexer_reads_in_small_windows() {
        let ([first, second, last], file) = cd_file();
        let whole = index_of(&[(100, 0)], (142, 10_240));
        // Windows of 16 octets: each frame header is met in a later read
        // than the one before it.
        let windows = lowered(LimitKind::ReadBytes, 16);
        assert_eq!(
            package_under(&whole, 0, &file, &windows).map(|segment| read_fragment(&segment)),
            Ok(Ok(fragment(
                1,
                0,
                &[
                    (4_096, first.as_slice()),
                    (4_096, second.as_slice()),
                    (2_048, last.as_slice())
                ]
            )))
        );
    }

    #[test]
    fn gives_the_last_frame_the_octets_up_to_the_end_of_the_segment() {
        let ([first, ..], _) = cd_file();
        // A frame, then an octet that could start a header but is cut
        // short: nothing says the frame ends before it.
        let source = cat(&[&first, &[0xFF]]);
        let index = index_of(&[(100, 0)], (115, 4_096));
        assert_eq!(
            read_back(&index, 0, &source),
            Ok(Ok(fragment(1, 0, &[(4_096, source.as_slice())])))
        );
    }

    #[test]
    fn refuses_a_segment_the_index_does_not_have() {
        let index = index_of(&[(100, 0), (114, 4_096), (128, 8_192)], (142, 10_240));
        for n in [3, 4, u32::MAX] {
            assert_eq!(
                package(&index, n, &[]),
                Err(PackError::NoSegment {
                    segment: n,
                    segments: 3,
                })
            );
        }
    }

    #[test]
    fn refuses_octets_that_are_not_as_long_as_the_segment() {
        let (_, file) = cd_file();
        let index = index_of(&[(100, 0), (114, 4_096)], (128, 8_192));
        // Segment 0 takes the 14 octets from 100 to 114.
        let cases: [(&[u8], u64); 3] = [(&file[..13], 13), (&file[..15], 15), (&[], 0)];
        for (source, found) in cases {
            assert_eq!(
                package(&index, 0, source),
                Err(PackError::SourceLength {
                    segment: 0,
                    start: 100,
                    end: 114,
                    found,
                })
            );
        }
        // Points that run backwards are no length at all.
        let backwards = index_of(&[(114, 0)], (100, 4_096));
        assert_eq!(
            package(&backwards, 0, &file[..14]),
            Err(PackError::SourceLength {
                segment: 0,
                start: 114,
                end: 100,
                found: 14,
            })
        );
    }

    #[test]
    fn refuses_octets_that_do_not_start_with_a_frame() {
        let index = index_of(&[(100, 0)], (114, 4_096));
        assert_eq!(
            package(&index, 0, &[0; 14]),
            Err(PackError::Flac(FlacFrameError::NotAFrame {
                offset: 100,
                problem: HeaderProblem::NoSync { octets: [0, 0] },
            }))
        );
    }

    #[test]
    fn refuses_a_segment_with_no_octets_or_with_part_of_a_header() {
        let empty = index_of(&[(100, 0)], (100, 0));
        assert_eq!(
            package(&empty, 0, &[]),
            Err(PackError::Fault(ParseFault::Truncated {
                offset: 100,
                needed: 1,
                available: 0,
            }))
        );
        // The first three octets of a frame header: the fourth is missing.
        let ([first, ..], _) = cd_file();
        let cut = index_of(&[(100, 0)], (103, 4_096));
        assert_eq!(
            package(&cut, 0, &first[..3]),
            Err(PackError::Fault(ParseFault::Truncated {
                offset: 103,
                needed: 1,
                available: 0,
            }))
        );
    }

    #[test]
    fn refuses_frames_that_skip_numbers() {
        // Frame 0 and then frame 3: frames 1 and 2 are not in the file.
        let source = cat(&[&cd_frame(0, 0b1100), &cd_frame(3, 0b1100)]);
        let index = index_of(&[(100, 0)], (128, 16_384));
        assert_eq!(
            package(&index, 0, &source),
            Err(PackError::Gap {
                segment: 0,
                gaps: 1,
            })
        );
    }

    #[test]
    fn refuses_frames_that_do_not_play_for_as_long_as_the_index_says() {
        let (_, file) = cd_file();
        // Two frames play 8,192 samples; the index says 8,191 and 8,193.
        for end in [8_191, 8_193] {
            let index = index_of(&[(100, 0)], (128, end));
            assert_eq!(
                package(&index, 0, &file[..28]),
                Err(PackError::Duration {
                    segment: 0,
                    start: 0,
                    end,
                    found: 8_192,
                })
            );
        }
        // Samples that run backwards are no length at all.
        let backwards = index_of(&[(100, 9_000)], (128, 8_192));
        assert_eq!(
            package(&backwards, 0, &file[..28]),
            Err(PackError::Duration {
                segment: 0,
                start: 9_000,
                end: 8_192,
                found: 8_192,
            })
        );
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn stops_with_the_budget_error_at_the_frame_it_cannot_pay_for() {
        let (_, file) = cd_file();
        let whole = index_of(&[(100, 0)], (142, 10_240));
        // With no steps, the first header cannot be paid for.
        assert_eq!(
            package_with(&whole, 0, &file, 0),
            (
                Err(PackError::Fault(ParseFault::BudgetExceeded { offset: 100 })),
                0
            )
        );
        // Enough to find the three frames and to copy two: the third,
        // which starts at 128, is not paid for.
        let finding = finding_steps(&file);
        assert_eq!(
            package_with(&whole, 0, &file, finding + 2),
            (
                Err(PackError::Fault(ParseFault::BudgetExceeded { offset: 128 })),
                0
            )
        );
        // One step more pays for it, and nothing is left over.
        let (written, left) = package_with(&whole, 0, &file, finding + 3);
        assert_eq!((written.map(|segment| segment.len()), left), (Ok(162), 0));
        // Steps that are not needed are not spent.
        let (written, left) = package_with(&whole, 0, &file, finding + 10);
        assert_eq!((written.map(|segment| segment.len()), left), (Ok(162), 7));
    }

    #[test]
    fn refuses_more_octets_than_one_parse_may_read() {
        let (_, file) = cd_file();
        let index = index_of(&[(100, 0)], (128, 8_192));
        let source = &file[..28];
        // 28 octets under a cap of 28 are written as under any cap.
        assert_eq!(
            package_under(&index, 0, source, &lowered(LimitKind::FileBytes, 28)),
            package(&index, 0, source)
        );
        assert_eq!(
            package(&index, 0, source).map(|segment| segment.len()),
            Ok(140)
        );
        assert_eq!(
            package_under(&index, 0, source, &lowered(LimitKind::FileBytes, 27)),
            Err(PackError::Fault(ParseFault::LimitExceeded {
                limit: LimitKind::FileBytes,
                value: 28,
                max: 27,
                offset: 100,
            }))
        );
    }

    #[test]
    fn refuses_more_frames_than_an_index_may_hold() {
        let (_, file) = cd_file();
        let index = index_of(&[(100, 0)], (128, 8_192));
        let source = &file[..28];
        // Two frames under a limit of two are written as under any limit.
        assert_eq!(
            package_under(&index, 0, source, &lowered(LimitKind::IndexEntries, 2)),
            package(&index, 0, source)
        );
        assert_eq!(
            package_under(&index, 0, source, &lowered(LimitKind::IndexEntries, 1)),
            Err(PackError::Fault(ParseFault::LimitExceeded {
                limit: LimitKind::IndexEntries,
                value: 2,
                max: 1,
                offset: 100,
            }))
        );
    }

    /// A frame with variable blocking that starts at sample `number` and
    /// holds the samples of block size code `code`, with how many those are.
    fn variable_frame(number: u64, code: u8) -> (Vec<u8>, u32) {
        let header = Header {
            block_size: code,
            ..Header::cd(true, number)
        };
        (frame(&header, &[16, 16]), 256 << (code - 8))
    }

    #[test]
    fn the_generated_frames_hold_the_samples_their_codes_stand_for() {
        // RFC 9639, table 14: codes 8 to 15 are 256 samples times a power
        // of two.
        let samples: Vec<u32> = (8..=14).map(|code| variable_frame(0, code).1).collect();
        assert_eq!(samples, [256, 512, 1_024, 2_048, 4_096, 8_192, 16_384]);
    }

    proptest! {
        /// Verifies: SEC-MED-001, SEC-MED-007
        #[test]
        fn returns_within_the_step_bound_for_any_octets(
            framed in any::<bool>(),
            tail in prop_oneof![
                vec(any::<u8>(), 0..200),
                // Every octet a candidate header: the costliest input.
                vec(Just(0xFF_u8), 0..200),
            ],
            start in 0_u64..1_000_000,
        ) {
            let source = if framed {
                cat(&[&cd_frame(0, 0b1100), &tail])
            } else {
                tail
            };
            let (written, octets) = on_small_stack(move || {
                let index = index_of(&[(start, 0)], (start + len(&source), 4_096));
                (package(&index, 0, &source), source)
            });
            // The documented budget is never spent, whatever the octets.
            prop_assert!(
                !matches!(written, Err(PackError::Fault(ParseFault::BudgetExceeded { .. }))),
                "{written:?}"
            );
            // What is written holds every octet that went in, in order.
            if let Ok(segment) = written {
                let read = read_fragment(&segment).map(|fragment| {
                    fragment
                        .samples
                        .iter()
                        .flat_map(|sample| sample.data.clone())
                        .collect::<Vec<u8>>()
                });
                prop_assert_eq!(read, Ok(octets));
            }
        }

        /// Verifies: SEC-MED-032
        #[test]
        fn any_run_of_frames_reads_back_wherever_the_segments_are_cut(
            codes in vec(8_u8..=14, 1..10),
            cuts in vec(any::<bool>(), 9),
            first_sample in 0_u64..1_000_000,
            first_octet in 0_u64..1_000_000,
        ) {
            // The model: each segment's points and its frames, written out
            // from the block sizes chosen, with no help from the packager.
            let mut segments: Vec<Vec<(u32, Vec<u8>)>> = Vec::new();
            let mut starts = Vec::new();
            let (mut sample, mut octet) = (first_sample, first_octet);
            for (at, &code) in codes.iter().enumerate() {
                let (octets, duration) = variable_frame(sample, code);
                if at == 0 || cuts[at - 1] {
                    segments.push(Vec::new());
                    starts.push((octet, sample));
                }
                sample += u64::from(duration);
                octet += len(&octets);
                segments
                    .last_mut()
                    .expect("the first frame starts a segment")
                    .push((duration, octets));
            }
            let index = index_of(&starts, (octet, sample));
            for (n, (&(_, decode_time), frames)) in starts.iter().zip(&segments).enumerate() {
                let n = u32::try_from(n).unwrap();
                let source = frames
                    .iter()
                    .flat_map(|(_, octets)| octets.clone())
                    .collect::<Vec<u8>>();
                let samples = frames
                    .iter()
                    .map(|(duration, octets)| (*duration, octets.as_slice()))
                    .collect::<Vec<_>>();
                prop_assert_eq!(
                    read_back(&index, n, &source),
                    Ok(Ok(fragment(n + 1, decode_time, &samples)))
                );
            }
        }
    }
}
