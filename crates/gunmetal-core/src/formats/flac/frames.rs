//! The FLAC frame index (RFC 9639, section 9): where each audio frame
//! starts and which sample it starts with.
//!
//! The audio packager copies frames by this index, and seeking uses it to
//! pick a byte range. The index decodes no samples and checks no frame's
//! CRC-16; it finds and validates frame headers.
//!
//! # Finding frames
//!
//! A FLAC frame does not say how long it is. The next frame starts at the
//! next frame header, so the indexer scans the audio for headers. The first
//! frame must start exactly where the audio does. After it, every octet
//! `0xFF` is a candidate, and a candidate is the next frame when:
//!
//! - its header is well formed: the sync code, no reserved or forbidden
//!   code, a coded number of the right width, and a CRC-8 that matches;
//! - it belongs to the same stream as the first frame: the same blocking
//!   strategy, sample rate, number of channels and bit depth; and
//! - its coded number is at least the one the previous frame implies: the
//!   next frame number for fixed blocking, or the previous sample number
//!   plus its block size for variable blocking.
//!
//! Any other candidate is part of the previous frame's data, which is how a
//! false sync code inside audio data is skipped. A number above the one
//! expected means frames are missing, usually because a header was damaged;
//! the frame is kept and the gap counted, so one damaged header does not
//! end the index.
//!
//! # Reading
//!
//! The indexer is a sans-I/O parser ([`SansIo`]). It reads the audio
//! forward in windows of [`LimitKind::ReadBytes`] octets, but never fewer
//! than [`MAX_HEADER_LEN`] unless the audio ends first, so a header that
//! runs past one window is read again whole at the start of the next, and
//! every request starts after the one before it (SEC-MED-008). It stops
//! asking before its reads would pass [`LimitKind::FileBytes`] and returns
//! what it found, marked as stopped there (SEC-MED-010, SEC-MED-006).
//!
//! # What it costs
//!
//! The indexer charges one step for each candidate it decides. Candidates
//! start at distinct octets of the audio, so a parse of `n` audio octets
//! takes at most [`STEPS_PER_OCTET`] × `n` + [`FIXED_STEPS`] steps
//! (SEC-MED-007).
//!
//! # Thinning
//!
//! The index keeps at most [`LimitKind::IndexEntries`] entries. While it has
//! room it keeps every frame. When it is full and another frame is due, it
//! drops every second entry and from then on keeps every second frame of
//! those it kept before, so the entries stay evenly spread over the whole
//! audio. [`FrameIndex::stride`] says how many frames apart they are.

use std::ops::Range;

use crate::parse::{
    Budget, Cursor, LimitKind, Limits, ParseFault, ReadRequest, SansIo, Step, Window,
};
use crate::values::{BitDepth, Channels, SampleRate};

/// Steps charged per octet of audio, at most (SEC-MED-007).
pub const STEPS_PER_OCTET: u64 = 1;

/// Steps charged per parse on top of [`STEPS_PER_OCTET`] (SEC-MED-007).
pub const FIXED_STEPS: u64 = 0;

/// The longest a frame header can be, in octets: sync code and blocking
/// strategy (2), codes (2), a seven-octet coded number, a 16-bit block
/// size, a 16-bit sample rate and the CRC-8.
pub const MAX_HEADER_LEN: u64 = 16;

/// What the stream's STREAMINFO block says, for frame headers that defer to
/// it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameContext {
    /// The stream's sample rate.
    pub stream_sample_rate: Option<SampleRate>,
    /// The stream's bits per sample.
    pub stream_bits: Option<BitDepth>,
}

/// How a stream's frames count their position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blocking {
    /// Every frame but the last has the same block size, and each header
    /// carries its frame number.
    Fixed,
    /// Block sizes vary, and each header carries the number of its first
    /// sample.
    Variable,
}

/// One frame in the index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameEntry {
    /// Where the frame's header starts, from the start of the file.
    pub offset: u64,
    /// The number of the frame's first sample, counted per channel from
    /// the start of the stream.
    pub first_sample: u64,
}

/// The frames of one FLAC stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameIndex {
    /// How the frames count their position.
    pub blocking: Blocking,
    /// The sample rate, when a header or the STREAMINFO block gives one.
    pub sample_rate: Option<SampleRate>,
    /// The number of channels.
    pub channels: Channels,
    /// The bits per sample, when a header or the STREAMINFO block gives
    /// them.
    pub bits: Option<BitDepth>,
    /// Every [`FrameIndex::stride`]-th frame, in order, starting with the
    /// first.
    pub entries: Vec<FrameEntry>,
    /// How many frames apart the entries are: 1 when every frame is kept,
    /// and a larger power of two when the index was thinned to its limit.
    pub stride: u64,
    /// How many frames were found.
    pub frames: u64,
    /// How many frames carried a number past the one expected, each a sign
    /// that frames before it are missing.
    pub gaps: u64,
    /// The number of the first sample after the last frame found.
    pub end_sample: u64,
    /// Why the index ends before the end of the audio, if it does: a
    /// [`ParseFault::Truncated`] when the audio ends inside what would be
    /// the next frame's header, or a [`ParseFault::LimitExceeded`] for
    /// [`LimitKind::FileBytes`] when reading on would pass the per-file
    /// read cap.
    pub end: Option<ParseFault>,
}

/// Why octets are not a frame header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderProblem {
    /// The first two octets are not a sync code and blocking strategy.
    NoSync {
        /// The two octets.
        octets: [u8; 2],
    },
    /// The block size code is the reserved code 0.
    ReservedBlockSize,
    /// The sample rate code is the forbidden code 15.
    ForbiddenSampleRate,
    /// The channel code is one of the reserved codes 11 to 15.
    ReservedChannels {
        /// The code.
        code: u8,
    },
    /// The bit depth code is the reserved code 3.
    ReservedBitDepth,
    /// The reserved bit after the bit depth is set.
    ReservedBit,
    /// An octet of the coded number cannot stand where it does.
    CodedNumber {
        /// Where the octet is.
        offset: u64,
        /// The octet.
        octet: u8,
    },
    /// A frame number coded in seven octets, which only a sample number may
    /// take.
    FrameNumberTooLong {
        /// Where the coded number starts.
        offset: u64,
    },
    /// A 16-bit block size field of 65,535, which would mean the forbidden
    /// block size 65,536.
    ForbiddenBlockSize,
    /// A sample rate field of zero.
    ZeroSampleRate,
    /// The CRC-8 the header states is not the one its octets give.
    Crc {
        /// The CRC-8 in the header.
        stated: u8,
        /// The CRC-8 of the octets before it.
        computed: u8,
    },
}

/// Why no frame index could be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlacFrameError {
    /// A failure every parser shares: the audio ended before its first
    /// frame header did, the first read would pass the per-file read cap,
    /// or the step budget ran out.
    Fault(ParseFault),
    /// The audio does not start with a frame header.
    NotAFrame {
        /// Where the audio starts.
        offset: u64,
        /// What is wrong with the header there.
        problem: HeaderProblem,
    },
    /// The host resumed the indexer with octets it had not asked for.
    Unrequested {
        /// What the indexer asked for.
        requested: ReadRequest,
        /// Where the octets it was given start.
        offset: u64,
        /// How many octets it was given.
        len: u64,
    },
}

/// Builds a [`FrameIndex`] as a sans-I/O parser.
///
/// The first resume, with [`Window::start`], tells the indexer how long the
/// file is; it then asks for the audio window by window. After it is done,
/// its next resume starts a new index from the beginning.
#[derive(Debug)]
pub struct FrameIndexer<'b> {
    context: FrameContext,
    audio: Range<u64>,
    limits: Limits,
    budget: &'b mut Budget,
    /// Where the audio ends, once the first resume has told the file's
    /// length.
    end: u64,
    /// The read whose octets the next resume brings, or `None` before the
    /// first resume.
    pending: Option<ReadRequest>,
    scan: Scan,
}

/// What one index has found so far.
#[derive(Debug)]
struct Scan {
    /// The stream the first frame set, and where it has got to.
    stream: Option<Stream>,
    entries: Vec<FrameEntry>,
    stride: u64,
    frames: u64,
    gaps: u64,
    /// Octets asked for so far.
    read: u64,
}

impl Scan {
    const fn new() -> Self {
        Self {
            stream: None,
            entries: Vec::new(),
            stride: 1,
            frames: 0,
            gaps: 0,
            read: 0,
        }
    }
}

/// What every frame of a stream shares, and what the next frame must
/// continue.
#[derive(Debug, Clone, Copy)]
struct Stream {
    blocking: Blocking,
    sample_rate: Option<SampleRate>,
    channels: Channels,
    bits: Option<BitDepth>,
    /// The block size of the first frame, which with fixed blocking is the
    /// number of samples per frame number.
    nominal: u64,
    /// The smallest coded number the next frame may carry.
    next: u64,
    /// The first sample after the last frame.
    end_sample: u64,
}

impl<'b> FrameIndexer<'b> {
    /// An indexer for the frames in `audio`, the octets of the file from
    /// the end of the metadata to the end of the audio, charging `budget`
    /// under `limits`. An `audio` that runs past the end of the file ends
    /// where the file does.
    #[must_use]
    pub fn new(
        context: FrameContext,
        audio: Range<u64>,
        limits: &Limits,
        budget: &'b mut Budget,
    ) -> Self {
        Self {
            context,
            audio,
            limits: *limits,
            budget,
            end: 0,
            pending: None,
            scan: Scan::new(),
        }
    }

    /// Asks for the window that starts at `offset`, or finishes when the
    /// audio ends there or the read would pass the per-file cap.
    fn read_from(&mut self, offset: u64) -> Step<Result<FrameIndex, FlacFrameError>> {
        let left = self.end.saturating_sub(offset);
        if left == 0 {
            return self.finish(None);
        }
        let window = self.limits.get(LimitKind::ReadBytes).max(MAX_HEADER_LEN);
        let len = left.min(window);
        let read = self.scan.read.saturating_add(len);
        let cap = self.limits.get(LimitKind::FileBytes);
        if read > cap {
            return self.finish(Some(ParseFault::LimitExceeded {
                limit: LimitKind::FileBytes,
                value: read,
                max: cap,
                offset,
            }));
        }
        self.scan.read = read;
        // The read limit's ceiling is 16 MiB, so the length fits.
        let request = ReadRequest {
            offset,
            len: u32::try_from(len).unwrap_or(u32::MAX),
        };
        self.pending = Some(request);
        Step::Need(request)
    }

    /// Looks for frames in `window`, the octets of the pending request.
    fn search(&mut self, window: Window<'_>) -> Step<Result<FrameIndex, FlacFrameError>> {
        let window_end = window
            .cursor()
            .offset()
            .saturating_add(window.cursor().remaining());
        let last = window_end == self.end;
        let mut rest = window.cursor();
        while let Some((&octet, tail)) = rest.rest().split_first() {
            let here = rest;
            let offset = here.offset();
            rest = Cursor::at(tail, offset.saturating_add(1));
            // Before the first frame, the audio's first octet must start
            // one; after it, only 0xFF can.
            if self.scan.stream.is_some() && octet != 0xFF {
                continue;
            }
            let decided = match parse_header(here, self.context) {
                Err(Rejection::Incomplete(fault)) if last => return self.finish(Some(fault)),
                Err(Rejection::Incomplete(_)) => return self.read_from(offset),
                decided => decided,
            };
            if let Err(fault) = self.budget.charge(1, offset) {
                return self.done(Err(FlacFrameError::Fault(fault)));
            }
            match decided {
                Ok((header, after)) if self.continues(&header) => {
                    self.record(offset, &header);
                    rest = after;
                }
                Err(Rejection::Problem(problem)) if self.scan.stream.is_none() => {
                    return self.done(Err(FlacFrameError::NotAFrame { offset, problem }));
                }
                _ => {}
            }
        }
        if last {
            self.finish(None)
        } else {
            self.read_from(window_end)
        }
    }

    /// Whether `header` is the next frame of the stream, or the first.
    fn continues(&self, header: &FrameHeader) -> bool {
        self.scan.stream.as_ref().is_none_or(|stream| {
            header.blocking == stream.blocking
                && header.sample_rate == stream.sample_rate
                && header.channels == stream.channels
                && header.bits == stream.bits
                && header.number >= stream.next
        })
    }

    /// Adds the frame whose `header` starts at `offset`, thinning the
    /// entries when they are full.
    fn record(&mut self, offset: u64, header: &FrameHeader) {
        let limit = self.limits.get(LimitKind::IndexEntries);
        let scan = &mut self.scan;
        let nominal = scan
            .stream
            .as_ref()
            .map_or(header.block_size, |stream| stream.nominal);
        let (first_sample, next) = match header.blocking {
            Blocking::Fixed => (
                header.number.saturating_mul(nominal),
                header.number.saturating_add(1),
            ),
            Blocking::Variable => (
                header.number,
                header.number.saturating_add(header.block_size),
            ),
        };
        let gap = scan
            .stream
            .as_ref()
            .is_some_and(|stream| header.number != stream.next);
        scan.gaps = scan.gaps.saturating_add(u64::from(gap));
        scan.stream = Some(Stream {
            blocking: header.blocking,
            sample_rate: header.sample_rate,
            channels: header.channels,
            bits: header.bits,
            nominal,
            next,
            end_sample: first_sample.saturating_add(header.block_size),
        });
        let ordinal = scan.frames;
        scan.frames = ordinal.saturating_add(1);
        if ordinal.checked_rem(scan.stride) != Some(0) {
            return;
        }
        if count(&scan.entries) >= limit {
            // Keep the first entry, the third, the fifth...
            let mut keep = false;
            scan.entries.retain(|_| {
                keep = !keep;
                keep
            });
            scan.stride = scan.stride.saturating_mul(2);
        }
        if ordinal.checked_rem(scan.stride) == Some(0) && count(&scan.entries) < limit {
            scan.entries.push(FrameEntry {
                offset,
                first_sample,
            });
        }
    }

    /// Ends the index, with `end` saying why it stops early, if it does.
    fn finish(&mut self, end: Option<ParseFault>) -> Step<Result<FrameIndex, FlacFrameError>> {
        let scan = std::mem::replace(&mut self.scan, Scan::new());
        let outcome = match (scan.stream, end) {
            (Some(stream), end) => Ok(FrameIndex {
                blocking: stream.blocking,
                sample_rate: stream.sample_rate,
                channels: stream.channels,
                bits: stream.bits,
                entries: scan.entries,
                stride: scan.stride,
                frames: scan.frames,
                gaps: scan.gaps,
                end_sample: stream.end_sample,
                end,
            }),
            (None, Some(fault)) => Err(FlacFrameError::Fault(fault)),
            // The audio is empty: it ends where it starts.
            (None, None) => Err(FlacFrameError::Fault(ParseFault::Truncated {
                offset: self.audio.start,
                needed: 1,
                available: 0,
            })),
        };
        self.done(outcome)
    }

    /// Returns `outcome` and makes ready to start again.
    fn done(
        &mut self,
        outcome: Result<FrameIndex, FlacFrameError>,
    ) -> Step<Result<FrameIndex, FlacFrameError>> {
        self.pending = None;
        self.scan = Scan::new();
        Step::Done(outcome)
    }
}

impl SansIo for FrameIndexer<'_> {
    type Output = Result<FrameIndex, FlacFrameError>;

    fn resume(&mut self, window: Window<'_>) -> Step<Self::Output> {
        let Some(requested) = self.pending else {
            self.end = self.audio.end.min(window.file_len);
            return self.read_from(self.audio.start);
        };
        let len = window.cursor().remaining();
        if window.offset != requested.offset || len != u64::from(requested.len) {
            return self.done(Err(FlacFrameError::Unrequested {
                requested,
                offset: window.offset,
                len,
            }));
        }
        self.search(window)
    }
}

/// How many items `items` holds.
fn count<T>(items: &[T]) -> u64 {
    // A slice never holds more than u64::MAX items.
    u64::try_from(items.len()).unwrap_or(u64::MAX)
}

/// A frame header, as the index needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FrameHeader {
    blocking: Blocking,
    block_size: u64,
    sample_rate: Option<SampleRate>,
    channels: Channels,
    bits: Option<BitDepth>,
    number: u64,
}

/// Why a candidate is not a frame header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rejection {
    /// The octets show it is not one.
    Problem(HeaderProblem),
    /// The octets end before it could be decided.
    Incomplete(ParseFault),
}

/// The refusal of a header for `problem`.
const fn refuse<T>(problem: HeaderProblem) -> Result<T, Rejection> {
    Err(Rejection::Problem(problem))
}

/// Reads a header an octet at a time, keeping its CRC-8.
struct Reader<'a> {
    cursor: Cursor<'a>,
    crc: u8,
}

impl Reader<'_> {
    fn octet(&mut self) -> Result<u8, Rejection> {
        let octet = self.cursor.u8().map_err(Rejection::Incomplete)?;
        self.crc = crc8(self.crc, octet);
        Ok(octet)
    }

    fn u16(&mut self) -> Result<u16, Rejection> {
        Ok(u16::from_be_bytes([self.octet()?, self.octet()?]))
    }
}

/// Reads the frame header at the start of `cursor`, resolving codes that
/// defer to STREAMINFO from `context`, and returns it with a cursor just
/// past it.
///
/// Each field is checked as soon as it is read, so octets that cannot be a
/// header are refused even when they end early.
fn parse_header(
    cursor: Cursor<'_>,
    context: FrameContext,
) -> Result<(FrameHeader, Cursor<'_>), Rejection> {
    let mut reader = Reader { cursor, crc: 0 };
    let octets = [reader.octet()?, reader.octet()?];
    let [sync, strategy] = octets;
    if sync != 0xFF || strategy & 0xFE != 0xF8 {
        return refuse(HeaderProblem::NoSync { octets });
    }
    let blocking = if strategy & 0x01 == 0 {
        Blocking::Fixed
    } else {
        Blocking::Variable
    };
    let codes = reader.octet()?;
    let block_code = codes >> 4;
    let rate_code = codes & 0x0F;
    if block_code == 0 {
        return refuse(HeaderProblem::ReservedBlockSize);
    }
    if rate_code == 0x0F {
        return refuse(HeaderProblem::ForbiddenSampleRate);
    }
    let layout = reader.octet()?;
    let channel_code = layout >> 4;
    // A reserved code counts no channels, which `Channels` refuses.
    let count = match channel_code {
        0..=7 => u32::from(channel_code).saturating_add(1),
        8..=10 => 2,
        _ => 0,
    };
    let channels = Channels::new(count)
        .map_err(|_| Rejection::Problem(HeaderProblem::ReservedChannels { code: channel_code }))?;
    let bits = match (layout >> 1) & 0x07 {
        0 => context.stream_bits,
        1 => BitDepth::new(8).ok(),
        2 => BitDepth::new(12).ok(),
        3 => return refuse(HeaderProblem::ReservedBitDepth),
        4 => BitDepth::new(16).ok(),
        5 => BitDepth::new(20).ok(),
        6 => BitDepth::new(24).ok(),
        _ => BitDepth::new(32).ok(),
    };
    if layout & 0x01 != 0 {
        return refuse(HeaderProblem::ReservedBit);
    }
    let number = coded_number(&mut reader, blocking)?;
    let block_size = match block_code {
        1 => 192,
        2 => 576,
        3 => 1_152,
        4 => 2_304,
        5 => 4_608,
        6 => u64::from(reader.octet()?).saturating_add(1),
        7 => match reader.u16()? {
            u16::MAX => return refuse(HeaderProblem::ForbiddenBlockSize),
            minus_one => u64::from(minus_one).saturating_add(1),
        },
        8 => 256,
        9 => 512,
        10 => 1_024,
        11 => 2_048,
        12 => 4_096,
        13 => 8_192,
        14 => 16_384,
        _ => 32_768,
    };
    let sample_rate = match rate_code {
        0 => context.stream_sample_rate,
        1 => SampleRate::new(88_200).ok(),
        2 => SampleRate::new(176_400).ok(),
        3 => SampleRate::new(192_000).ok(),
        4 => SampleRate::new(8_000).ok(),
        5 => SampleRate::new(16_000).ok(),
        6 => SampleRate::new(22_050).ok(),
        7 => SampleRate::new(24_000).ok(),
        8 => SampleRate::new(32_000).ok(),
        9 => SampleRate::new(44_100).ok(),
        10 => SampleRate::new(48_000).ok(),
        11 => SampleRate::new(96_000).ok(),
        12 => stated_rate(u32::from(reader.octet()?).saturating_mul(1_000))?,
        13 => stated_rate(u32::from(reader.u16()?))?,
        _ => stated_rate(u32::from(reader.u16()?).saturating_mul(10))?,
    };
    let computed = reader.crc;
    let stated = reader.octet()?;
    if stated != computed {
        return refuse(HeaderProblem::Crc { stated, computed });
    }
    let header = FrameHeader {
        blocking,
        block_size,
        sample_rate,
        channels,
        bits,
        number,
    };
    Ok((header, reader.cursor))
}

/// A sample rate the header states at its end, which must not be zero.
fn stated_rate(hz: u32) -> Result<Option<SampleRate>, Rejection> {
    SampleRate::new(hz)
        .map(Some)
        .map_err(|_| Rejection::Problem(HeaderProblem::ZeroSampleRate))
}

/// Reads the coded number: a frame number of up to 31 bits in one to six
/// octets with fixed blocking, or a sample number of up to 36 bits in one
/// to seven octets with variable blocking, laid out as UTF-8 lays out a
/// code point (RFC 9639, section 9.1.5). A form longer than it needs to be
/// is accepted, as the RFC does not forbid it.
fn coded_number(reader: &mut Reader<'_>, blocking: Blocking) -> Result<u64, Rejection> {
    let start = reader.cursor.offset();
    let lead = reader.octet()?;
    let (continuations, high) = match lead {
        0x00..=0x7F => (0, lead),
        0xC0..=0xDF => (1, lead & 0x1F),
        0xE0..=0xEF => (2, lead & 0x0F),
        0xF0..=0xF7 => (3, lead & 0x07),
        0xF8..=0xFB => (4, lead & 0x03),
        0xFC..=0xFD => (5, lead & 0x01),
        0xFE => (6, 0),
        _ => {
            return refuse(HeaderProblem::CodedNumber {
                offset: start,
                octet: lead,
            });
        }
    };
    if continuations == 6 && blocking == Blocking::Fixed {
        return refuse(HeaderProblem::FrameNumberTooLong { offset: start });
    }
    let mut number = u64::from(high);
    for _ in 0..continuations {
        let offset = reader.cursor.offset();
        let octet = reader.octet()?;
        if octet & 0xC0 != 0x80 {
            return refuse(HeaderProblem::CodedNumber { offset, octet });
        }
        // At most 36 bits in all, so nothing saturates.
        number = number
            .saturating_mul(64)
            .saturating_add(u64::from(octet & 0x3F));
    }
    Ok(number)
}

/// `crc` advanced over `octet`: FLAC's header CRC-8, polynomial 0x07,
/// initial value 0, most significant bit first.
fn crc8(crc: u8, octet: u8) -> u8 {
    (0..8).fold(crc ^ octet, |crc, _| {
        let shifted = crc.wrapping_shl(1);
        if crc & 0x80 == 0 {
            shifted
        } else {
            shifted ^ 0x07
        }
    })
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use super::*;
    use crate::parse::{DriveError, drive};
    use gunmetal_testkit::checksum::crc8_flac;
    use gunmetal_testkit::flac_frames::{Header, frame};
    use proptest::collection::vec;
    use proptest::prelude::*;

    /// A context for a stream whose STREAMINFO block is not known.
    const NO_STREAM_INFO: FrameContext = FrameContext {
        stream_sample_rate: None,
        stream_bits: None,
    };

    fn hz(rate: u32) -> Option<SampleRate> {
        SampleRate::new(rate).ok()
    }

    fn depth(bits: u32) -> Option<BitDepth> {
        BitDepth::new(bits).ok()
    }

    fn channels(count: u32) -> Channels {
        Channels::new(count).unwrap()
    }

    const fn entry(offset: u64, first_sample: u64) -> FrameEntry {
        FrameEntry {
            offset,
            first_sample,
        }
    }

    /// The fault for audio that ends at `offset`, where one more octet was
    /// needed: every header field is read an octet at a time.
    const fn ends_at(offset: u64) -> ParseFault {
        ParseFault::Truncated {
            offset,
            needed: 1,
            available: 0,
        }
    }

    fn len(bytes: &[u8]) -> u64 {
        count(bytes)
    }

    fn count<T>(items: &[T]) -> u64 {
        u64::try_from(items.len()).unwrap()
    }

    /// The test thread stack SEC-MED-001 names: 256 KiB.
    fn on_small_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(262_144)
            .spawn(work)
            .expect("the test thread starts")
            .join()
            .expect("the code under test returned instead of panicking")
    }

    /// `Limits::DEFAULT` with each listed limit changed.
    fn limits(changes: &[(LimitKind, u64)]) -> Limits {
        changes
            .iter()
            .fold(Limits::DEFAULT, |limits, &(kind, value)| {
                limits.with_override(kind, value).unwrap()
            })
    }

    /// An indexer that fails the test the moment a request does not start
    /// past the one before it, so an indexer that stops advancing fails
    /// fast instead of hanging (SEC-MED-008).
    struct Advancing<'b> {
        inner: FrameIndexer<'b>,
        last: Option<u64>,
    }

    impl SansIo for Advancing<'_> {
        type Output = Result<FrameIndex, FlacFrameError>;

        fn resume(&mut self, window: Window<'_>) -> Step<Self::Output> {
            let step = self.inner.resume(window);
            if let Step::Need(request) = step {
                assert!(
                    self.last.is_none_or(|last| request.offset > last),
                    "{request:?} does not start past {:?}",
                    self.last
                );
                self.last = Some(request.offset);
            }
            step
        }
    }

    /// The budget [`STEPS_PER_OCTET`] and [`FIXED_STEPS`] allow for the
    /// part of `file` that `audio` covers.
    fn documented_budget(file: &[u8], audio: &Range<u64>) -> Budget {
        let octets = audio.end.min(len(file)).saturating_sub(audio.start);
        Budget::for_input(octets, STEPS_PER_OCTET, FIXED_STEPS)
    }

    /// Indexes `audio` in `file` through [`drive`], under `budget`.
    fn drive_index(
        file: &[u8],
        audio: Range<u64>,
        context: FrameContext,
        limits: &Limits,
        budget: &mut Budget,
    ) -> Result<Result<FrameIndex, FlacFrameError>, DriveError> {
        let indexer = Advancing {
            inner: FrameIndexer::new(context, audio, limits, budget),
            last: None,
        };
        drive(indexer, file, limits)
    }

    /// Indexes `audio` in `file` under the documented budget.
    fn index_audio(
        file: &[u8],
        audio: Range<u64>,
        context: FrameContext,
        limits: &Limits,
    ) -> Result<FrameIndex, FlacFrameError> {
        let mut budget = documented_budget(file, &audio);
        drive_index(file, audio, context, limits, &mut budget)
            .expect("the host serves every request")
    }

    /// Indexes all of `file` as audio, knowing no STREAMINFO.
    fn index(file: &[u8]) -> Result<FrameIndex, FlacFrameError> {
        index_audio(file, 0..len(file), NO_STREAM_INFO, &Limits::DEFAULT)
    }

    /// The first frame libFLAC 1.5.0 wrote for 64 samples of 16-bit mono
    /// silence at 8 kHz: header, constant subframe, CRC-16.
    const LIBFLAC_FRAME: [u8; 12] = [
        0xFF, 0xF8, 0x64, 0x08, 0x00, 0x3F, 0x5E, 0x00, 0x00, 0x00, 0xC6, 0x3C,
    ];

    /// Fixed-blocking frames of 4,096 samples of 16-bit stereo silence at
    /// 44.1 kHz, numbered `numbers`, each 14 octets for numbers below 128.
    fn cd_frames(numbers: impl IntoIterator<Item = u64>) -> Vec<Vec<u8>> {
        numbers
            .into_iter()
            .map(|number| frame(&Header::cd(false, number), &[16, 16]))
            .collect()
    }

    /// Where each of `frames` starts once they are written one after
    /// another from `base`.
    fn starts(base: u64, frames: &[Vec<u8>]) -> Vec<u64> {
        frames
            .iter()
            .scan(base, |at, frame| {
                let start = *at;
                *at += len(frame);
                Some(start)
            })
            .collect()
    }

    /// What a CD stream's index holds, given its entries and the rest.
    fn cd_index(entries: Vec<FrameEntry>, frames: u64, end_sample: u64) -> FrameIndex {
        FrameIndex {
            blocking: Blocking::Fixed,
            sample_rate: hz(44_100),
            channels: channels(2),
            bits: depth(16),
            entries,
            stride: 1,
            frames,
            gaps: 0,
            end_sample,
            end: None,
        }
    }

    // -----------------------------------------------------------------
    // The header CRC-8.
    // -----------------------------------------------------------------

    #[test]
    fn computes_the_crc_8_of_the_crc_catalogue_and_of_libflac() {
        // CRC-8/SMBUS's check value, and the CRC-8 libFLAC wrote.
        let crc = |octets: &[u8]| octets.iter().fold(0, |crc, &octet| crc8(crc, octet));
        assert_eq!(crc(b"123456789"), 0xF4);
        assert_eq!(crc(&LIBFLAC_FRAME[..6]), 0x5E);
        assert_eq!(crc(&[]), 0x00);
    }

    // -----------------------------------------------------------------
    // One header at a time.
    // -----------------------------------------------------------------

    /// Parses the header at the start of `bytes`, placed at file offset
    /// 100, and returns it with the offset just past it.
    fn header(bytes: &[u8], context: FrameContext) -> Result<(FrameHeader, u64), Rejection> {
        parse_header(Cursor::at(bytes, 100), context)
            .map(|(header, after)| (header, after.offset()))
    }

    /// The header [`Header::cd`] writes, as the parser should read it.
    fn cd_header(blocking: Blocking, number: u64) -> FrameHeader {
        FrameHeader {
            blocking,
            block_size: 4_096,
            sample_rate: hz(44_100),
            channels: channels(2),
            bits: depth(16),
            number,
        }
    }

    const fn refused(problem: HeaderProblem) -> Result<(FrameHeader, u64), Rejection> {
        Err(Rejection::Problem(problem))
    }

    #[test]
    fn reads_the_header_libflac_wrote() {
        assert_eq!(
            header(&LIBFLAC_FRAME, NO_STREAM_INFO),
            Ok((
                FrameHeader {
                    blocking: Blocking::Fixed,
                    block_size: 64,
                    sample_rate: hz(8_000),
                    channels: channels(1),
                    bits: depth(16),
                    number: 0,
                },
                107
            ))
        );
    }

    #[test]
    fn reads_both_blocking_strategies() {
        assert_eq!(
            header(&Header::cd(false, 3).bytes(), NO_STREAM_INFO),
            Ok((cd_header(Blocking::Fixed, 3), 106))
        );
        assert_eq!(
            header(&Header::cd(true, 3).bytes(), NO_STREAM_INFO),
            Ok((cd_header(Blocking::Variable, 3), 106))
        );
    }

    #[test]
    fn refuses_octets_without_a_sync_code() {
        for octets in [
            [0xFF, 0xFA],
            [0xFF, 0xF0],
            [0xFE, 0xF8],
            [0x7F, 0xF8],
            [0xFF, 0x78],
        ] {
            let mut bytes = Header::cd(false, 0).bytes();
            bytes[..2].copy_from_slice(&octets);
            assert_eq!(
                header(&bytes, NO_STREAM_INFO),
                refused(HeaderProblem::NoSync { octets }),
                "{octets:02X?}"
            );
        }
    }

    #[test]
    fn reads_every_block_size_code() {
        // RFC 9639, section 9.1.1: 192, 144 × 2^n for codes 2 to 5, and
        // 2^n for codes 8 to 15.
        let codes: [(u8, u64); 13] = [
            (0b0001, 192),
            (0b0010, 576),
            (0b0011, 1_152),
            (0b0100, 2_304),
            (0b0101, 4_608),
            (0b1000, 256),
            (0b1001, 512),
            (0b1010, 1_024),
            (0b1011, 2_048),
            (0b1100, 4_096),
            (0b1101, 8_192),
            (0b1110, 16_384),
            (0b1111, 32_768),
        ];
        for (code, block_size) in codes {
            let bytes = Header {
                block_size: code,
                ..Header::cd(false, 0)
            }
            .bytes();
            assert_eq!(
                header(&bytes, NO_STREAM_INFO),
                Ok((
                    FrameHeader {
                        block_size,
                        ..cd_header(Blocking::Fixed, 0)
                    },
                    106
                )),
                "code {code}"
            );
        }
    }

    #[test]
    fn reads_block_sizes_from_the_end_of_the_header() {
        // Codes 6 and 7 store the block size minus one in 8 or 16 bits.
        let cases: [(u8, u16, u64, u64); 6] = [
            (0b0110, 0, 1, 107),
            (0b0110, 0x7F, 128, 107),
            (0b0110, 0xFF, 256, 107),
            (0b0111, 0, 1, 108),
            (0b0111, 0x1234, 0x1235, 108),
            (0b0111, 0xFFFE, 65_535, 108),
        ];
        for (code, field, block_size, end) in cases {
            let bytes = Header {
                block_size: code,
                block_size_field: field,
                ..Header::cd(false, 0)
            }
            .bytes();
            assert_eq!(
                header(&bytes, NO_STREAM_INFO),
                Ok((
                    FrameHeader {
                        block_size,
                        ..cd_header(Blocking::Fixed, 0)
                    },
                    end
                )),
                "code {code}, field {field}"
            );
        }
    }

    #[test]
    fn refuses_the_reserved_and_forbidden_block_sizes() {
        let reserved = Header {
            block_size: 0b0000,
            ..Header::cd(false, 0)
        };
        assert_eq!(
            header(&reserved.bytes(), NO_STREAM_INFO),
            refused(HeaderProblem::ReservedBlockSize)
        );
        let forbidden = Header {
            block_size: 0b0111,
            block_size_field: 0xFFFF,
            ..Header::cd(false, 0)
        };
        assert_eq!(
            header(&forbidden.bytes(), NO_STREAM_INFO),
            refused(HeaderProblem::ForbiddenBlockSize)
        );
    }

    #[test]
    fn reads_every_sample_rate_code() {
        // RFC 9639, section 9.1.2.
        let codes: [(u8, u32); 11] = [
            (0b0001, 88_200),
            (0b0010, 176_400),
            (0b0011, 192_000),
            (0b0100, 8_000),
            (0b0101, 16_000),
            (0b0110, 22_050),
            (0b0111, 24_000),
            (0b1000, 32_000),
            (0b1001, 44_100),
            (0b1010, 48_000),
            (0b1011, 96_000),
        ];
        for (code, rate) in codes {
            let bytes = Header {
                sample_rate: code,
                ..Header::cd(false, 0)
            }
            .bytes();
            assert_eq!(
                header(&bytes, NO_STREAM_INFO),
                Ok((
                    FrameHeader {
                        sample_rate: hz(rate),
                        ..cd_header(Blocking::Fixed, 0)
                    },
                    106
                )),
                "code {code}"
            );
        }
    }

    #[test]
    fn reads_sample_rates_from_the_end_of_the_header() {
        // Code 12 in kilohertz (8 bits), 13 in hertz and 14 in tens of
        // hertz (16 bits).
        let cases: [(u8, u16, u32, u64); 6] = [
            (0b1100, 1, 1_000, 107),
            (0b1100, 0xFF, 255_000, 107),
            (0b1101, 1, 1, 108),
            (0b1101, 0xFFFF, 65_535, 108),
            (0b1110, 1, 10, 108),
            (0b1110, 0xFFFF, 655_350, 108),
        ];
        for (code, field, rate, end) in cases {
            let bytes = Header {
                sample_rate: code,
                sample_rate_field: field,
                ..Header::cd(false, 0)
            }
            .bytes();
            assert_eq!(
                header(&bytes, NO_STREAM_INFO),
                Ok((
                    FrameHeader {
                        sample_rate: hz(rate),
                        ..cd_header(Blocking::Fixed, 0)
                    },
                    end
                )),
                "code {code}, field {field}"
            );
        }
    }

    #[test]
    fn refuses_a_sample_rate_of_zero_and_the_forbidden_code() {
        for code in [0b1100, 0b1101, 0b1110] {
            let bytes = Header {
                sample_rate: code,
                sample_rate_field: 0,
                ..Header::cd(false, 0)
            }
            .bytes();
            assert_eq!(
                header(&bytes, NO_STREAM_INFO),
                refused(HeaderProblem::ZeroSampleRate),
                "code {code}"
            );
        }
        let forbidden = Header {
            sample_rate: 0b1111,
            ..Header::cd(false, 0)
        };
        assert_eq!(
            header(&forbidden.bytes(), NO_STREAM_INFO),
            refused(HeaderProblem::ForbiddenSampleRate)
        );
    }

    #[test]
    fn takes_the_sample_rate_and_depth_from_streaminfo_when_the_header_defers() {
        let deferring = Header {
            sample_rate: 0b0000,
            bits: 0b000,
            ..Header::cd(false, 0)
        }
        .bytes();
        let known = FrameContext {
            stream_sample_rate: hz(12_345),
            stream_bits: depth(4),
        };
        assert_eq!(
            header(&deferring, known),
            Ok((
                FrameHeader {
                    sample_rate: hz(12_345),
                    bits: depth(4),
                    ..cd_header(Blocking::Fixed, 0)
                },
                106
            ))
        );
        assert_eq!(
            header(&deferring, NO_STREAM_INFO),
            Ok((
                FrameHeader {
                    sample_rate: None,
                    bits: None,
                    ..cd_header(Blocking::Fixed, 0)
                },
                106
            ))
        );
        // A header that states them ignores STREAMINFO.
        assert_eq!(
            header(&Header::cd(false, 0).bytes(), known),
            Ok((cd_header(Blocking::Fixed, 0), 106))
        );
    }

    #[test]
    fn reads_every_channel_code_and_refuses_the_reserved_ones() {
        // RFC 9639, section 9.1.3: codes 0 to 7 are one to eight channels,
        // 8 to 10 are stereo with decorrelation.
        let counts: [(u8, u32); 11] = [
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 4),
            (4, 5),
            (5, 6),
            (6, 7),
            (7, 8),
            (8, 2),
            (9, 2),
            (10, 2),
        ];
        for (code, count) in counts {
            let bytes = Header {
                channels: code,
                ..Header::cd(false, 0)
            }
            .bytes();
            assert_eq!(
                header(&bytes, NO_STREAM_INFO),
                Ok((
                    FrameHeader {
                        channels: channels(count),
                        ..cd_header(Blocking::Fixed, 0)
                    },
                    106
                )),
                "code {code}"
            );
        }
        for code in 11..=15 {
            let bytes = Header {
                channels: code,
                ..Header::cd(false, 0)
            }
            .bytes();
            assert_eq!(
                header(&bytes, NO_STREAM_INFO),
                refused(HeaderProblem::ReservedChannels { code }),
                "code {code}"
            );
        }
    }

    #[test]
    fn reads_every_bit_depth_code_and_refuses_the_reserved_one() {
        // RFC 9639, section 9.1.4.
        let codes: [(u8, u32); 6] = [(1, 8), (2, 12), (4, 16), (5, 20), (6, 24), (7, 32)];
        for (code, bits) in codes {
            let bytes = Header {
                bits: code,
                ..Header::cd(false, 0)
            }
            .bytes();
            assert_eq!(
                header(&bytes, NO_STREAM_INFO),
                Ok((
                    FrameHeader {
                        bits: depth(bits),
                        ..cd_header(Blocking::Fixed, 0)
                    },
                    106
                )),
                "code {code}"
            );
        }
        let reserved = Header {
            bits: 0b011,
            ..Header::cd(false, 0)
        };
        assert_eq!(
            header(&reserved.bytes(), NO_STREAM_INFO),
            refused(HeaderProblem::ReservedBitDepth)
        );
    }

    #[test]
    fn refuses_the_reserved_bit() {
        let bytes = Header {
            reserved: true,
            ..Header::cd(false, 0)
        }
        .bytes();
        assert_eq!(
            header(&bytes, NO_STREAM_INFO),
            refused(HeaderProblem::ReservedBit)
        );
    }

    #[test]
    fn reads_a_coded_number_at_each_width() {
        // The widths' boundaries, with bits set in every octet. A frame
        // number takes at most six octets, a sample number seven.
        let cases: [(u64, u64); 9] = [
            (0x55, 106),
            (0x7FF, 107),
            (0xA5A5, 108),
            (0x15_5555, 109),
            (0x2AA_AAAA, 110),
            (0x5555_5555, 111),
            (0x7FFF_FFFF, 111),
            (0x8000_0000, 112),
            (0xF_FFFF_FFFF, 112),
        ];
        for (number, end) in cases {
            let variable = Header::cd(true, number).bytes();
            assert_eq!(
                header(&variable, NO_STREAM_INFO),
                Ok((cd_header(Blocking::Variable, number), end)),
                "sample number {number:#X}"
            );
            if number <= 0x7FFF_FFFF {
                let fixed = Header::cd(false, number).bytes();
                assert_eq!(
                    header(&fixed, NO_STREAM_INFO),
                    Ok((cd_header(Blocking::Fixed, number), end)),
                    "frame number {number:#X}"
                );
            }
        }
    }

    #[test]
    fn refuses_a_frame_number_of_seven_octets() {
        let bytes = Header::cd(false, 0x8000_0000).bytes();
        assert_eq!(
            header(&bytes, NO_STREAM_INFO),
            refused(HeaderProblem::FrameNumberTooLong { offset: 104 })
        );
    }

    /// A CD header's first four octets, then `number` as raw octets, then a
    /// CRC-8 that matches them.
    fn with_raw_number(variable: bool, number: &[u8]) -> Vec<u8> {
        let mut bytes = Header::cd(variable, 0).bytes();
        bytes.truncate(4);
        bytes.extend(number);
        bytes.push(crc8_flac(&bytes));
        bytes
    }

    #[test]
    fn refuses_coded_numbers_with_misplaced_octets() {
        // A continuation octet or 0xFF cannot lead, and only a
        // continuation octet can follow the lead.
        let cases: [(&[u8], u64, u8); 6] = [
            (&[0x80], 104, 0x80),
            (&[0xBF], 104, 0xBF),
            (&[0xFF], 104, 0xFF),
            (&[0xC2, 0x00], 105, 0x00),
            (&[0xE0, 0xA0, 0xC0], 106, 0xC0),
            (&[0xFE, 0x82, 0x80, 0x80, 0x80, 0x80, 0x7F], 110, 0x7F),
        ];
        for (number, offset, octet) in cases {
            assert_eq!(
                header(&with_raw_number(true, number), NO_STREAM_INFO),
                refused(HeaderProblem::CodedNumber { offset, octet }),
                "{number:02X?}"
            );
        }
    }

    #[test]
    fn accepts_a_coded_number_written_longer_than_it_needs() {
        // Zero in two octets: RFC 9639 does not ask for the shortest form.
        assert_eq!(
            header(&with_raw_number(false, &[0xC0, 0x80]), NO_STREAM_INFO),
            Ok((cd_header(Blocking::Fixed, 0), 107))
        );
    }

    #[test]
    fn refuses_a_header_whose_crc_8_does_not_match() {
        let mut bytes = Header::cd(false, 0).bytes();
        assert_eq!(bytes[5], 0xC2);
        bytes[5] = 0xC3;
        assert_eq!(
            header(&bytes, NO_STREAM_INFO),
            refused(HeaderProblem::Crc {
                stated: 0xC3,
                computed: 0xC2,
            })
        );
    }

    #[test]
    fn reports_where_a_header_runs_out() {
        // Every field is read one octet at a time, so a header cut short
        // needs one more octet where the cut is.
        let bytes = Header {
            variable: true,
            block_size: 0b0111,
            sample_rate: 0b1101,
            number: 0x8000_0000,
            block_size_field: 0x1000,
            sample_rate_field: 0x2000,
            ..Header::cd(true, 0)
        }
        .bytes();
        assert_eq!(bytes.len(), 16);
        for cut in 0..bytes.len() {
            assert_eq!(
                header(&bytes[..cut], NO_STREAM_INFO),
                Err(Rejection::Incomplete(ends_at(100 + len(&bytes[..cut])))),
                "cut at {cut}"
            );
        }
        // 8-bit fields at the end, and a 16-bit rate in tens of hertz.
        for (block_size, sample_rate) in [(0b0110, 0b1100), (0b0111, 0b1110)] {
            let short = Header {
                block_size,
                sample_rate,
                block_size_field: 7,
                sample_rate_field: 7,
                ..Header::cd(false, 0)
            }
            .bytes();
            for cut in 0..short.len() {
                assert_eq!(
                    header(&short[..cut], NO_STREAM_INFO),
                    Err(Rejection::Incomplete(ends_at(100 + len(&short[..cut])))),
                    "codes {block_size} and {sample_rate}, cut at {cut}"
                );
            }
        }
    }

    // -----------------------------------------------------------------
    // Whole indexes.
    // -----------------------------------------------------------------

    /// Verifies: SEC-MED-001
    #[test]
    fn indexes_the_frame_libflac_wrote() {
        assert_eq!(
            index(&LIBFLAC_FRAME),
            Ok(FrameIndex {
                blocking: Blocking::Fixed,
                sample_rate: hz(8_000),
                channels: channels(1),
                bits: depth(16),
                entries: vec![entry(0, 0)],
                stride: 1,
                frames: 1,
                gaps: 0,
                end_sample: 64,
                end: None,
            })
        );
    }

    #[test]
    fn indexes_fixed_blocking_frames_where_the_audio_starts() {
        // Ten octets of metadata before the audio, which the indexer must
        // not read as frames.
        let frames = cd_frames(0..3);
        let mut file = [0xFF; 10].to_vec();
        file.extend(frames.concat());
        let offsets = starts(10, &frames);
        assert_eq!(offsets, [10, 24, 38]);
        assert_eq!(
            index_audio(&file, 10..len(&file), NO_STREAM_INFO, &Limits::DEFAULT),
            Ok(cd_index(
                vec![entry(10, 0), entry(24, 4_096), entry(38, 8_192)],
                3,
                12_288
            ))
        );
    }

    #[test]
    fn places_a_short_last_frame_after_the_full_ones() {
        let mut frames = cd_frames(0..2);
        frames.push(frame(
            &Header {
                block_size: 0b0110,
                block_size_field: 99,
                ..Header::cd(false, 2)
            },
            &[16, 16],
        ));
        assert_eq!(
            index(&frames.concat()),
            Ok(cd_index(
                vec![entry(0, 0), entry(14, 4_096), entry(28, 8_192)],
                3,
                8_292
            ))
        );
    }

    /// A variable-blocking frame of `block_size` samples starting at sample
    /// `number`, otherwise of the CD shape.
    fn variable_frame(number: u64, block_size: u16) -> Vec<u8> {
        frame(
            &Header {
                block_size: 0b0111,
                block_size_field: block_size - 1,
                ..Header::cd(true, number)
            },
            &[16, 16],
        )
    }

    #[test]
    fn indexes_variable_blocking_frames_by_their_sample_numbers() {
        let frames = [
            variable_frame(0, 1_000),
            variable_frame(1_000, 2_000),
            variable_frame(3_000, 500),
        ];
        let offsets = starts(0, &frames);
        assert_eq!(
            index(&frames.concat()),
            Ok(FrameIndex {
                blocking: Blocking::Variable,
                entries: vec![
                    entry(offsets[0], 0),
                    entry(offsets[1], 1_000),
                    entry(offsets[2], 3_000),
                ],
                ..cd_index(Vec::new(), 3, 3_500)
            })
        );
    }

    #[test]
    fn a_stream_may_start_past_sample_zero() {
        // Coded numbers of every width start a stream: frame 2^30 at 4,096
        // samples per frame, and sample 2^35.
        assert_eq!(
            index(&cd_frames([1 << 30]).concat()),
            Ok(cd_index(vec![entry(0, 1 << 42)], 1, (1 << 42) + 4_096))
        );
        assert_eq!(
            index(&variable_frame(1 << 35, 16)),
            Ok(FrameIndex {
                blocking: Blocking::Variable,
                ..cd_index(vec![entry(0, 1 << 35)], 1, (1 << 35) + 16)
            })
        );
    }

    #[test]
    fn resolves_a_deferring_stream_from_streaminfo() {
        let file = frame(
            &Header {
                sample_rate: 0b0000,
                bits: 0b000,
                ..Header::cd(false, 0)
            },
            &[16, 16],
        );
        let known = FrameContext {
            stream_sample_rate: hz(44_100),
            stream_bits: depth(16),
        };
        assert_eq!(
            index_audio(&file, 0..len(&file), known, &Limits::DEFAULT),
            Ok(cd_index(vec![entry(0, 0)], 1, 4_096))
        );
        assert_eq!(
            index(&file),
            Ok(FrameIndex {
                sample_rate: None,
                bits: None,
                ..cd_index(vec![entry(0, 0)], 1, 4_096)
            })
        );
    }

    /// Verifies: SEC-MED-001
    #[test]
    fn refuses_audio_that_does_not_start_with_a_frame() {
        let mut file = [0x00; 4].to_vec();
        file.extend(LIBFLAC_FRAME);
        assert_eq!(
            index(&file),
            Err(FlacFrameError::NotAFrame {
                offset: 0,
                problem: HeaderProblem::NoSync {
                    octets: [0x00, 0x00],
                },
            })
        );
        let mut damaged = LIBFLAC_FRAME;
        damaged[6] = 0x5F;
        assert_eq!(
            index(&damaged),
            Err(FlacFrameError::NotAFrame {
                offset: 0,
                problem: HeaderProblem::Crc {
                    stated: 0x5F,
                    computed: 0x5E,
                },
            })
        );
    }

    /// Verifies: SEC-MED-001
    #[test]
    fn refuses_empty_audio_as_truncated_where_it_starts() {
        assert_eq!(index(&[]), Err(FlacFrameError::Fault(ends_at(0))));
        assert_eq!(
            index_audio(&LIBFLAC_FRAME, 5..5, NO_STREAM_INFO, &Limits::DEFAULT),
            Err(FlacFrameError::Fault(ends_at(5)))
        );
        // Audio said to start past the end of the file.
        assert_eq!(
            index_audio(&LIBFLAC_FRAME, 20..30, NO_STREAM_INFO, &Limits::DEFAULT),
            Err(FlacFrameError::Fault(ends_at(20)))
        );
    }

    #[test]
    fn ends_the_audio_where_the_file_ends() {
        assert_eq!(
            index_audio(&LIBFLAC_FRAME, 0..1_000, NO_STREAM_INFO, &Limits::DEFAULT),
            index(&LIBFLAC_FRAME)
        );
    }

    #[test]
    fn stops_where_the_audio_is_said_to_end() {
        // An ID3v1 tag after the audio is not read as frame data.
        let frames = cd_frames(0..3);
        let file = frames.concat();
        assert_eq!(
            index_audio(&file, 0..30, NO_STREAM_INFO, &Limits::DEFAULT),
            Ok(FrameIndex {
                end: Some(ends_at(30)),
                ..cd_index(vec![entry(0, 0), entry(14, 4_096)], 2, 8_192)
            })
        );
    }

    /// Verifies: SEC-MED-001
    #[test]
    fn skips_a_false_sync_code_whose_crc_8_fails() {
        // A header for frame 1 inside frame 0's audio data, with its CRC-8
        // off by one, then the real frame 1.
        let frames = cd_frames(0..2);
        let mut fake = Header::cd(false, 1).bytes();
        fake[5] ^= 0x01;
        let file = [frames[0].clone(), fake, frames[1].clone()].concat();
        assert_eq!(
            index(&file),
            Ok(cd_index(vec![entry(0, 0), entry(20, 4_096)], 2, 8_192))
        );
    }

    #[test]
    fn skips_well_formed_headers_that_do_not_continue_the_stream() {
        // Each of these is a valid header with a matching CRC-8 inside
        // frame 0's data, but differs from the stream in one way.
        let fakes: [(&str, Header); 6] = [
            ("an earlier frame number", Header::cd(false, 0)),
            ("the other blocking strategy", Header::cd(true, 1)),
            (
                "another sample rate",
                Header {
                    sample_rate: 0b1010,
                    ..Header::cd(false, 1)
                },
            ),
            (
                "another channel count",
                Header {
                    channels: 0b0000,
                    ..Header::cd(false, 1)
                },
            ),
            (
                "another bit depth",
                Header {
                    bits: 0b110,
                    ..Header::cd(false, 1)
                },
            ),
            (
                "a reserved code",
                Header {
                    channels: 0b1011,
                    ..Header::cd(false, 1)
                },
            ),
        ];
        let frames = cd_frames(0..2);
        for (why, fake) in fakes {
            let fake = fake.bytes();
            let after = 14 + len(&fake);
            let file = [frames[0].clone(), fake, frames[1].clone()].concat();
            assert_eq!(
                index(&file),
                Ok(cd_index(vec![entry(0, 0), entry(after, 4_096)], 2, 8_192)),
                "{why}"
            );
        }
    }

    #[test]
    fn keeps_a_stereo_frame_whatever_its_decorrelation() {
        // Left/side, side/right and mid/side are all two channels.
        let mut frames = cd_frames(0..1);
        for (number, code) in [(1, 0b1000), (2, 0b1001), (3, 0b1010)] {
            frames.push(frame(
                &Header {
                    channels: code,
                    ..Header::cd(false, number)
                },
                &[16, 17],
            ));
        }
        let offsets = starts(0, &frames);
        assert_eq!(
            index(&frames.concat()),
            Ok(cd_index(
                offsets
                    .iter()
                    .zip(0..)
                    .map(|(&offset, n)| entry(offset, 4_096 * n))
                    .collect(),
                4,
                16_384
            ))
        );
    }

    #[test]
    fn keeps_a_frame_after_missing_ones_and_counts_the_gap() {
        // Frame 2 is missing; frames 3 and 5 still carry their places.
        let frames = cd_frames([0, 1, 3, 5]);
        assert_eq!(
            index(&frames.concat()),
            Ok(FrameIndex {
                gaps: 2,
                ..cd_index(
                    vec![
                        entry(0, 0),
                        entry(14, 4_096),
                        entry(28, 12_288),
                        entry(42, 20_480)
                    ],
                    4,
                    24_576
                )
            })
        );
        let variable = [
            variable_frame(0, 100),
            variable_frame(100, 100),
            variable_frame(1_000, 100),
        ];
        let offsets = starts(0, &variable);
        assert_eq!(
            index(&variable.concat()),
            Ok(FrameIndex {
                blocking: Blocking::Variable,
                gaps: 1,
                ..cd_index(
                    vec![
                        entry(offsets[0], 0),
                        entry(offsets[1], 100),
                        entry(offsets[2], 1_000),
                    ],
                    3,
                    1_100
                )
            })
        );
    }

    /// The index an independent model expects for `frames`, a clean CD
    /// stream numbered from 0, cut after `cut` octets: every frame whose
    /// header is whole, and a truncation where a header is cut.
    fn cut_model(frames: &[Vec<u8>], cut: u64) -> Result<FrameIndex, FlacFrameError> {
        let offsets = starts(0, frames);
        let whole: Vec<u64> = offsets
            .iter()
            .copied()
            .filter(|&start| start + 6 <= cut)
            .collect();
        let cut_inside = offsets.iter().any(|&start| start < cut && cut < start + 6);
        if whole.is_empty() {
            return Err(FlacFrameError::Fault(ends_at(cut)));
        }
        let found = count(&whole);
        Ok(FrameIndex {
            end: cut_inside.then(|| ends_at(cut)),
            ..cd_index(
                whole
                    .iter()
                    .zip(0..)
                    .map(|(&start, n)| entry(start, 4_096 * n))
                    .collect(),
                found,
                4_096 * found,
            )
        })
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn every_cut_of_a_valid_stream_reports_the_exact_truncation() {
        let frames = cd_frames(0..4);
        let file = frames.concat();
        // Only the sync codes hold 0xFF, so no other octet is a candidate.
        let candidates: Vec<usize> = file
            .iter()
            .enumerate()
            .filter(|&(_, &octet)| octet == 0xFF)
            .map(|(at, _)| at)
            .collect();
        assert_eq!(candidates, [0, 14, 28, 42]);
        for cut in 0..=len(&file) {
            let audio = &file[..usize::try_from(cut).unwrap()];
            assert_eq!(index(audio), cut_model(&frames, cut), "cut at {cut}");
        }
    }

    // -----------------------------------------------------------------
    // Windows, the read caps and the budget.
    // -----------------------------------------------------------------

    /// Verifies: SEC-MED-008, SEC-TM-032
    #[test]
    fn reads_headers_that_straddle_windows_again_whole() {
        // Windows of 16 to 20 octets split headers at every point; the
        // index is the one a single window gives.
        let frames = cd_frames(0..6);
        let file = frames.concat();
        let whole = index(&file);
        assert_eq!(
            whole,
            Ok(cd_index(
                (0..6).map(|n| entry(14 * n, 4_096 * n)).collect(),
                6,
                24_576
            ))
        );
        for read in 16..=20 {
            assert_eq!(
                index_audio(
                    &file,
                    0..len(&file),
                    NO_STREAM_INFO,
                    &limits(&[(LimitKind::ReadBytes, read)])
                ),
                whole,
                "windows of {read} octets"
            );
        }
    }

    /// Verifies: SEC-MED-008
    #[test]
    fn asks_for_one_header_at_least_even_under_a_smaller_read_limit() {
        // A window shorter than a header could leave a header split at its
        // start, and the next request would start there again.
        let lowered = limits(&[(LimitKind::ReadBytes, 15)]);
        // Audio shorter than one header is asked for whole.
        assert_eq!(
            index_audio(&LIBFLAC_FRAME, 0..12, NO_STREAM_INFO, &lowered),
            index(&LIBFLAC_FRAME)
        );
        // Longer audio is asked for a whole header at a time, which a host
        // holding to the lower limit refuses.
        let file = cd_frames(0..2).concat();
        let mut budget = documented_budget(&file, &(0..28));
        assert_eq!(
            drive_index(&file, 0..28, NO_STREAM_INFO, &lowered, &mut budget),
            Err(DriveError::TooLong {
                offset: 0,
                len: 16,
                max: 15,
            })
        );
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn stops_before_its_reads_pass_the_per_file_cap() {
        // Windows of 16 octets over three 14-octet frames: [0, 16) finds
        // frame 0 and the start of frame 1, [14, 30) reads frame 1 whole,
        // and the window from 28 would take the reads to 46 octets.
        let file = cd_frames(0..3).concat();
        let capped = limits(&[(LimitKind::ReadBytes, 16), (LimitKind::FileBytes, 45)]);
        assert_eq!(
            index_audio(&file, 0..42, NO_STREAM_INFO, &capped),
            Ok(FrameIndex {
                end: Some(ParseFault::LimitExceeded {
                    limit: LimitKind::FileBytes,
                    value: 46,
                    max: 45,
                    offset: 28,
                }),
                ..cd_index(vec![entry(0, 0), entry(14, 4_096)], 2, 8_192)
            })
        );
        // At exactly 46 octets every frame is read.
        let enough = limits(&[(LimitKind::ReadBytes, 16), (LimitKind::FileBytes, 46)]);
        assert_eq!(
            index_audio(&file, 0..42, NO_STREAM_INFO, &enough),
            Ok(cd_index(
                vec![entry(0, 0), entry(14, 4_096), entry(28, 8_192)],
                3,
                12_288
            ))
        );
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn refuses_audio_whose_first_read_would_pass_the_per_file_cap() {
        let file = cd_frames(0..3).concat();
        let capped = limits(&[(LimitKind::FileBytes, 41)]);
        assert_eq!(
            index_audio(&file, 0..42, NO_STREAM_INFO, &capped),
            Err(FlacFrameError::Fault(ParseFault::LimitExceeded {
                limit: LimitKind::FileBytes,
                value: 42,
                max: 41,
                offset: 0,
            }))
        );
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn charges_one_step_per_candidate_and_fails_at_exactly_the_last() {
        // Frame 0, then two stray 0xFF octets in its data, then frame 1:
        // four candidates.
        let frames = cd_frames(0..2);
        let file = [frames[0].clone(), vec![0xFF, 0x00, 0xFF], frames[1].clone()].concat();
        let mut budget = Budget::for_input(0, 0, 4);
        assert_eq!(
            drive_index(
                &file,
                0..len(&file),
                NO_STREAM_INFO,
                &Limits::DEFAULT,
                &mut budget
            ),
            Ok(Ok(cd_index(vec![entry(0, 0), entry(17, 4_096)], 2, 8_192)))
        );
        assert_eq!(budget.remaining(), 0);
        let mut short = Budget::for_input(0, 0, 3);
        assert_eq!(
            drive_index(
                &file,
                0..len(&file),
                NO_STREAM_INFO,
                &Limits::DEFAULT,
                &mut short
            ),
            Ok(Err(FlacFrameError::Fault(ParseFault::BudgetExceeded {
                offset: 17
            })))
        );
        let mut none = Budget::for_input(0, 0, 0);
        assert_eq!(
            drive_index(
                &file,
                0..len(&file),
                NO_STREAM_INFO,
                &Limits::DEFAULT,
                &mut none
            ),
            Ok(Err(FlacFrameError::Fault(ParseFault::BudgetExceeded {
                offset: 0
            })))
        );
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn a_window_full_of_sync_codes_stays_within_one_step_per_octet() {
        // After one real frame, 4,000 octets of FF F8: the most candidates
        // the audio can hold, each one refused.
        let mut file = LIBFLAC_FRAME.to_vec();
        for _ in 0..2_000 {
            file.extend([0xFF, 0xF8]);
        }
        let mut budget = documented_budget(&file, &(0..len(&file)));
        let before = budget.remaining();
        assert_eq!(
            drive_index(
                &file,
                0..len(&file),
                NO_STREAM_INFO,
                &Limits::DEFAULT,
                &mut budget
            ),
            Ok(Ok(FrameIndex {
                end: Some(ends_at(len(&file))),
                ..index(&LIBFLAC_FRAME).unwrap()
            }))
        );
        // The real frame and 2,000 refused candidates; the last one runs
        // out of octets and is not charged.
        assert_eq!(before - budget.remaining(), 2_000);
    }

    // -----------------------------------------------------------------
    // Thinning.
    // -----------------------------------------------------------------

    /// The stride an independent model gives `count` frames under a limit
    /// of `limit` entries: the smallest power of two that leaves at most
    /// `limit` of them.
    fn model_stride(count: u64, limit: u64) -> u64 {
        let mut stride = 1;
        while count.div_ceil(stride) > limit {
            stride *= 2;
        }
        stride
    }

    /// Indexes `count` CD frames under an entry limit of `limit`.
    fn thinned(count: u64, limit: u64) -> Result<FrameIndex, FlacFrameError> {
        let file = cd_frames(0..count).concat();
        index_audio(
            &file,
            0..len(&file),
            NO_STREAM_INFO,
            &limits(&[(LimitKind::IndexEntries, limit)]),
        )
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn keeps_every_frame_up_to_the_limit_and_thins_one_past_it() {
        assert_eq!(
            thinned(4, 4),
            Ok(cd_index(
                (0..4).map(|n| entry(14 * n, 4_096 * n)).collect(),
                4,
                16_384
            ))
        );
        assert_eq!(
            thinned(5, 4),
            Ok(FrameIndex {
                stride: 2,
                ..cd_index(
                    vec![entry(0, 0), entry(28, 8_192), entry(56, 16_384)],
                    5,
                    20_480
                )
            })
        );
        assert_eq!(
            thinned(7, 3),
            Ok(FrameIndex {
                stride: 4,
                ..cd_index(vec![entry(0, 0), entry(56, 16_384)], 7, 28_672)
            })
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn a_limit_of_one_keeps_the_first_frame_and_zero_keeps_none() {
        assert_eq!(
            thinned(5, 1),
            Ok(FrameIndex {
                stride: 8,
                ..cd_index(vec![entry(0, 0)], 5, 20_480)
            })
        );
        assert_eq!(
            thinned(5, 0),
            Ok(FrameIndex {
                stride: 8,
                ..cd_index(Vec::new(), 5, 20_480)
            })
        );
        assert_eq!(
            thinned(1, 0),
            Ok(FrameIndex {
                stride: 2,
                ..cd_index(Vec::new(), 1, 4_096)
            })
        );
    }

    /// Ten hours of 8-bit mono silence at 8 kHz in fixed blocks of 65,535
    /// samples: 4,395 frames, the last of 39,210 samples, and where each
    /// frame starts.
    fn ten_hours() -> (Vec<u8>, Vec<u64>) {
        let samples: u64 = 10 * 3_600 * 8_000;
        let full = samples / 65_535;
        let mut file = Vec::new();
        let mut offsets = Vec::new();
        for number in 0..=full {
            let size = if number == full {
                samples - full * 65_535
            } else {
                65_535
            };
            offsets.push(len(&file));
            file.extend(frame(
                &Header {
                    variable: false,
                    block_size: 0b0111,
                    sample_rate: 0b0100,
                    channels: 0b0000,
                    bits: 0b001,
                    reserved: false,
                    number,
                    block_size_field: u16::try_from(size - 1).unwrap(),
                    sample_rate_field: 0,
                },
                &[8],
            ));
        }
        (file, offsets)
    }

    /// Verifies: SEC-MED-005, SEC-MED-006
    #[test]
    fn thins_ten_hours_of_frames_to_the_index_limit_on_a_small_stack() {
        let (file, offsets) = ten_hours();
        assert_eq!(offsets.len(), 4_395);
        let expected = |stride: u64| FrameIndex {
            blocking: Blocking::Fixed,
            sample_rate: hz(8_000),
            channels: channels(1),
            bits: depth(8),
            entries: offsets
                .iter()
                .zip(0..)
                .filter(|&(_, n)| n % stride == 0)
                .map(|(&offset, n)| entry(offset, n * 65_535))
                .collect(),
            stride,
            frames: 4_395,
            gaps: 0,
            end_sample: 288_000_000,
            end: None,
        };
        assert_eq!(model_stride(4_395, 1_000), 8);
        let thinned_to = |limit: u64| {
            let file = file.clone();
            on_small_stack(move || {
                index_audio(
                    &file,
                    0..len(&file),
                    NO_STREAM_INFO,
                    &limits(&[(LimitKind::IndexEntries, limit)]),
                )
            })
        };
        assert_eq!(thinned_to(1_000), Ok(expected(8)));
        // The default limit of a million entries keeps every frame.
        assert_eq!(thinned_to(1_000_000), Ok(expected(1)));
    }

    // -----------------------------------------------------------------
    // The protocol.
    // -----------------------------------------------------------------

    #[test]
    fn refuses_octets_it_did_not_ask_for() {
        let mut budget = Budget::for_input(100, 1, 0);
        let mut indexer = FrameIndexer::new(NO_STREAM_INFO, 0..12, &Limits::DEFAULT, &mut budget);
        let asked = ReadRequest { offset: 0, len: 12 };
        assert_eq!(indexer.resume(Window::start(12)), Step::Need(asked));
        let elsewhere = Window {
            offset: 1,
            bytes: &LIBFLAC_FRAME[1..],
            file_len: 12,
        };
        assert_eq!(
            indexer.resume(elsewhere),
            Step::Done(Err(FlacFrameError::Unrequested {
                requested: asked,
                offset: 1,
                len: 11,
            }))
        );
        // Done, it starts again; this time the octets are short.
        assert_eq!(indexer.resume(Window::start(12)), Step::Need(asked));
        let short = Window {
            offset: 0,
            bytes: &LIBFLAC_FRAME[..11],
            file_len: 12,
        };
        assert_eq!(
            indexer.resume(short),
            Step::Done(Err(FlacFrameError::Unrequested {
                requested: asked,
                offset: 0,
                len: 11,
            }))
        );
    }

    #[test]
    fn starts_a_new_index_after_it_is_done() {
        let mut budget = Budget::for_input(100, 1, 0);
        let mut indexer = FrameIndexer::new(NO_STREAM_INFO, 0..12, &Limits::DEFAULT, &mut budget);
        let whole = Window {
            offset: 0,
            bytes: &LIBFLAC_FRAME,
            file_len: 12,
        };
        for _ in 0..2 {
            assert_eq!(
                indexer.resume(Window::start(12)),
                Step::Need(ReadRequest { offset: 0, len: 12 })
            );
            assert_eq!(indexer.resume(whole), Step::Done(index(&LIBFLAC_FRAME)));
        }
    }

    // -----------------------------------------------------------------
    // Properties.
    // -----------------------------------------------------------------

    /// One frame of a generated stream: a block size code with its field,
    /// a channel code, frame numbers to skip before it, and octets of data
    /// to put after it.
    #[derive(Debug, Clone)]
    struct Spec {
        block_size: (u8, u16),
        stereo_code: u8,
        skip: u64,
        data: Vec<u8>,
    }

    fn spec(junk: bool) -> impl Strategy<Value = Spec> {
        (
            prop_oneof![
                (prop_oneof![1_u8..=5, 8_u8..=15], Just(0_u16)),
                (Just(6_u8), any::<u8>().prop_map(u16::from)),
                (Just(7_u8), 0_u16..0xFFFF),
            ],
            prop_oneof![Just(1_u8), Just(8), Just(9), Just(10)],
            prop_oneof![4 => Just(0_u64), 1 => 1_u64..4],
            if junk {
                vec(prop_oneof![Just(0xFF_u8), Just(0xF8), any::<u8>()], 0..24).boxed()
            } else {
                Just(Vec::new()).boxed()
            },
        )
            .prop_map(|(block_size, stereo_code, skip, data)| Spec {
                block_size,
                stereo_code,
                skip,
                data,
            })
    }

    /// The block size a code and field mean, written out from the table in
    /// RFC 9639, section 9.1.1.
    fn model_block_size((code, field): (u8, u16)) -> u64 {
        match code {
            1 => 192,
            2..=5 => 144 << code,
            6 | 7 => u64::from(field) + 1,
            _ => 1 << code,
        }
    }

    /// A stereo stream with the given blocking, starting at `first`, its
    /// file, and what an independent model expects its index to hold when
    /// no frame's data holds a candidate.
    fn stream(variable: bool, first: u64, specs: &[Spec]) -> (Vec<u8>, FrameIndex) {
        let mut file = Vec::new();
        let mut entries = Vec::new();
        let mut number = first;
        let mut gaps = 0;
        let mut end_sample = 0;
        let nominal = specs
            .first()
            .map_or(1, |spec| model_block_size(spec.block_size));
        for (index, spec) in specs.iter().enumerate() {
            let size = model_block_size(spec.block_size);
            if index > 0 && spec.skip > 0 {
                gaps += 1;
                number += if variable {
                    spec.skip * 1_000
                } else {
                    spec.skip
                };
            }
            let first_sample = if variable { number } else { number * nominal };
            entries.push(entry(len(&file), first_sample));
            end_sample = first_sample + size;
            let side_bit = u32::from(spec.stereo_code != 1);
            file.extend(frame(
                &Header {
                    block_size: spec.block_size.0,
                    block_size_field: spec.block_size.1,
                    channels: spec.stereo_code,
                    ..Header::cd(variable, number)
                },
                &[16, 16 + side_bit],
            ));
            file.extend(&spec.data);
            number += if variable { size } else { 1 };
        }
        // The last frame's CRC-16 may end the file with 0xFF, or with a
        // sync code, which could be the start of another header.
        let ends_in_a_sync = match file.as_slice() {
            [.., 0xFF] => true,
            [.., 0xFF, last] => last & 0xFE == 0xF8,
            _ => false,
        };
        let index = FrameIndex {
            blocking: if variable {
                Blocking::Variable
            } else {
                Blocking::Fixed
            },
            frames: count(&entries),
            entries,
            gaps,
            end_sample,
            end: ends_in_a_sync.then(|| ends_at(len(&file))),
            ..cd_index(Vec::new(), 0, 0)
        };
        (file, index)
    }

    fn any_stream(junk: bool) -> impl Strategy<Value = (Vec<u8>, FrameIndex)> {
        (any::<bool>(), 0_u64..2_000, vec(spec(junk), 1..10))
            .prop_map(|(variable, first, specs)| stream(variable, first, &specs))
    }

    proptest! {
        /// Verifies: SEC-MED-001, SEC-HIS-036
        #[test]
        fn returns_for_any_octets_on_a_small_stack(
            bytes in vec(any::<u8>(), 0..256),
            read in 16_u64..64,
        ) {
            let result = on_small_stack(move || {
                let audio = 0..len(&bytes);
                let whole = index_audio(&bytes, audio.clone(), NO_STREAM_INFO, &Limits::DEFAULT);
                let windowed = index_audio(
                    &bytes,
                    audio,
                    NO_STREAM_INFO,
                    &limits(&[(LimitKind::ReadBytes, read)]),
                );
                (whole, windowed)
            });
            prop_assert_eq!(&result.0, &result.1);
            // The documented budget always suffices.
            prop_assert!(
                !matches!(result.0, Err(FlacFrameError::Fault(ParseFault::BudgetExceeded { .. }))),
                "{:?}",
                result.0
            );
        }

        /// Verifies: SEC-MED-001
        #[test]
        fn indexes_every_frame_of_a_clean_stream(stream in any_stream(false)) {
            let (file, expected) = stream;
            prop_assert_eq!(index(&file), Ok(expected));
        }

        /// Verifies: SEC-MED-001, SEC-MED-007, SEC-MED-008
        #[test]
        fn sample_positions_strictly_increase_whatever_lies_between_frames(
            stream in any_stream(true),
            read in 16_u64..64,
            limit in 0_u64..12,
        ) {
            let (file, _) = stream;
            let lowered = limits(&[(LimitKind::ReadBytes, read), (LimitKind::IndexEntries, limit)]);
            let (windowed, whole) = on_small_stack(move || {
                let audio = 0..len(&file);
                (
                    index_audio(&file, audio.clone(), NO_STREAM_INFO, &lowered),
                    index_audio(
                        &file,
                        audio,
                        NO_STREAM_INFO,
                        &limits(&[(LimitKind::IndexEntries, limit)]),
                    ),
                )
            });
            // The documented budget always suffices, and the window size
            // changes nothing.
            prop_assert_eq!(&windowed, &whole);
            let found = windowed.unwrap();
            for pair in found.entries.windows(2) {
                prop_assert!(pair[0].offset < pair[1].offset, "{:?}", pair);
                prop_assert!(pair[0].first_sample < pair[1].first_sample, "{:?}", pair);
            }
            prop_assert!(count(&found.entries) <= limit);
            prop_assert!(found.entries.last().is_none_or(|last| last.first_sample < found.end_sample));
        }

        /// Verifies: SEC-MED-006
        #[test]
        fn thins_to_the_smallest_stride_that_fits(
            count in 1_u64..40,
            limit in 1_u64..12,
        ) {
            let stride = model_stride(count, limit);
            let found = thinned(count, limit).unwrap();
            prop_assert_eq!(found.stride, stride);
            prop_assert_eq!(
                found.entries,
                (0..count)
                    .filter(|n| n % stride == 0)
                    .map(|n| entry(14 * n, 4_096 * n))
                    .collect::<Vec<_>>()
            );
        }
    }
}
