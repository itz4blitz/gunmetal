//! The probe's state machine: detection, the container parser, then the
//! reads for the tag blocks.
//!
//! # Reads
//!
//! Every read the probe asks for, its own or one of its parsers', first
//! passes a [`ReadGuard`] of its own, the same guard a host applies. A
//! read the limits refuse is therefore never asked for: no read is longer
//! than [`LimitKind::ReadBytes`], none runs past the end of the file, and
//! together they stay within [`LimitKind::FileBytes`] (SEC-MED-010). A
//! refused read fails the file when the container parser needs it, and
//! skips the tag block it was for otherwise (SEC-MED-017). The octets
//! admitted are reported in
//! [`FileFacts::bytes_read`](crate::catalog::FileFacts::bytes_read).
//!
//! The media itself is not read: the probe reads headers, tags and, for
//! an MP3 file without a table of contents, the four-octet header of each
//! frame (LIB-019).
//!
//! # Steps
//!
//! The caller's [`Budget`] pays for everything but detection, which
//! counts its own eight reads. A FLAC file's metadata parser charges the
//! caller's budget itself; before it starts, the probe moves every step
//! beyond that parser's allowance into a budget of its own for the tag
//! blocks and the lyrics. For any other format the probe moves all of
//! them, and gives the container parser the allowance it documents out of
//! them. A budget of [`Budget::for_input`]`(n, `[`STEPS_PER_OCTET`]`,
//! `[`FIXED_STEPS`]`)` is enough for any file of `n` octets
//! (SEC-MED-007).
//!
//! Each `ID3v2` tag the probe reads costs one step on top of what its
//! parser charges, so a tag with no frames is not free. A tag is at least
//! ten octets, so that step is within the steps the file's octets allow.
//!
//! # Depth and iteration
//!
//! The probe nests nothing itself: each parser counts its own depth from
//! its own root (SEC-MED-005). Every turn of the machine either asks for
//! a read that holds at least one octet, moves to a later stage, or ends,
//! and the reads for one tag block move forward through it (SEC-MED-008).
//!
//! # Counts
//!
//! `ID3v2` tags may lie back to back, and each holds up to the tag-field
//! limit of frames, so how many are read from one place is capped: seven
//! in front of the audio, which is as many as detection skips, and one in
//! a chunk of a WAV or AIFF file, which holds one tag. The first tag past
//! the cap is left unread with every tag after it, and recorded
//! (SEC-TM-032). The pictures of all a file's tag blocks count together
//! against the per-file picture limit (SEC-MED-006).

use std::mem;

use crate::formats::aiff::Aiff;
use crate::formats::ape::{self, ApeError};
use crate::formats::detect::{Detected, Detector, Format, detect};
use crate::formats::flac::metadata::{self as flac_metadata, MetadataParser, parse_metadata};
use crate::formats::id3v1;
use crate::formats::id3v2;
use crate::formats::mp4;
use crate::formats::mpa::{self, StreamInfo, stream_info};
use crate::formats::ogg;
use crate::formats::riff::{self, Wav};
use crate::formats::vorbis_comment;
use crate::parse::{
    Budget, DriveError, LimitKind, Limits, ParseFault, ReadGuard, ReadRequest, SansIo, Step, Window,
};

use crate::values::SampleRate;

use super::draft::{Draft, Gather, Job, span};
use super::facts::{PartProblem, ProbeError, Probed, TagBlock};
use super::{flac, mp4 as mp4_facts, mpeg, ogg as ogg_facts, pcm};

/// The steps a probe may charge its budget for each octet of the file.
/// With [`FIXED_STEPS`], `Budget::for_input(file_len, STEPS_PER_OCTET,
/// FIXED_STEPS)` is always enough (SEC-MED-007).
///
/// The most any format needs is an Ogg file's: its header packets may be
/// read twice over while the read of them grows, at the two steps an
/// octet of a page costs, the end of the file once more, and the comment
/// block and its lyrics at two steps an octet each.
pub const STEPS_PER_OCTET: u64 = 16;

/// The steps a probe may charge beyond [`STEPS_PER_OCTET`]: the fixed
/// steps of the container parsers, of which the MP3 parser's 128 are the
/// most, and those of the tag and lyrics parsers.
pub const FIXED_STEPS: u64 = 256;

/// How many octets of an Ogg file are read first for its header packets.
/// The read doubles until it holds them.
const OGG_HEAD: u64 = 8_192;

/// The most `ID3v2` tags read from the front of a file: as many as
/// detection skips there. No row of the limits table counts tags, so the
/// probe keeps to the one bound the project has for them.
const LEADING_TAGS: u64 = 7;

/// The most `ID3v2` tags read from a chunk of a WAV or AIFF file: the
/// chunk holds one tag.
const CHUNK_TAGS: u64 = 1;

/// What a probe answers with at its end.
type Outcome = Result<Probed, ProbeError>;

/// A turn of the machine: the stage to go on with from a start window, or
/// the step to answer the host with, boxed because a result is large.
type Turn<'b> = Result<Stage<'b>, Box<Step<Outcome>>>;

/// A probe of one file, driven through the sans-I/O protocol. Made by
/// [`probe`].
///
/// Resume it with [`Window::start`] first. A probe reads one file once:
/// resumed after its result, it answers [`ProbeError::Finished`].
#[derive(Debug)]
pub struct Probe<'b> {
    /// The limits the probe runs under.
    limits: Limits,
    /// The steps left for the tag blocks, the lyrics and the container
    /// parsers that take a budget of their own.
    work: Budget,
    /// The guard every read passes, once the file's length is known.
    guard: Option<ReadGuard>,
    /// Where the probe is.
    stage: Stage<'b>,
}

/// Where a probe is.
#[derive(Debug)]
enum Stage<'b> {
    /// Detecting the format, with the caller's budget still whole.
    Detect(Detector, &'b mut Budget),
    /// Reading FLAC metadata that starts after this many octets of tags.
    Flac(u64, MetadataParser<'b>),
    /// Reading an MP3 stream after this many octets of tags.
    Mpeg(u64, StreamInfo),
    /// Reading an MP4 file's boxes.
    Mp4(Box<mp4::Probe>),
    /// Reading a WAV file's chunks.
    Wav(Wav),
    /// Reading an AIFF file's chunks.
    Aiff(Aiff),
    /// Reading the start of an Ogg file.
    OggHead(Gather),
    /// Reading the octets for a tag block.
    Jobs(Box<Draft>, Job, Gather),
    /// The result has been given.
    Finished,
}

/// A probe of one file whose name has the extension `ext_hint` (without
/// the dot), or no extension, under `limits`, charging `budget`.
///
/// The extension only narrows which formats the content may be detected
/// as (SEC-MED-011).
#[must_use]
pub fn probe<'b>(ext_hint: Option<&str>, limits: Limits, budget: &'b mut Budget) -> Probe<'b> {
    Probe {
        limits,
        work: Budget::for_input(0, 0, 0),
        guard: None,
        stage: Stage::Detect(detect(ext_hint), budget),
    }
}

impl SansIo for Probe<'_> {
    type Output = Outcome;

    fn resume(&mut self, window: Window<'_>) -> Step<Outcome> {
        let file_len = window.file_len;
        let mut window = window;
        loop {
            let stage = mem::replace(&mut self.stage, Stage::Finished);
            match self.turn(stage, window) {
                Ok(next) => {
                    self.stage = next;
                    window = Window::start(file_len);
                }
                Err(step) => return *step,
            }
        }
    }
}

/// The step that ends a probe with `error`.
fn fail(error: ProbeError) -> Box<Step<Outcome>> {
    Box::new(Step::Done(Err(error)))
}

impl<'b> Probe<'b> {
    /// Takes `window` in at `stage`.
    fn turn(&mut self, stage: Stage<'b>, window: Window<'_>) -> Turn<'b> {
        let file_len = window.file_len;
        match stage {
            Stage::Detect(mut detector, budget) => match detector.resume(window) {
                Step::Need(request) => {
                    Err(self.ask(request, file_len, Stage::Detect(detector, budget)))
                }
                Step::Done(Ok(detected)) => self.begin(detected, budget, file_len),
                Step::Done(Err(error)) => Err(fail(ProbeError::Detect(error))),
            },
            Stage::Flac(lead, mut parser) => match parser.resume(window) {
                Step::Need(request) => Err(self.ask(request, file_len, Stage::Flac(lead, parser))),
                Step::Done(Ok(metadata)) => {
                    self.next(flac::draft(metadata, lead, file_len, &self.limits))
                }
                Step::Done(Err(error)) => Err(fail(ProbeError::Flac(error))),
            },
            Stage::Mpeg(lead, mut parser) => match parser.resume(window) {
                Step::Need(request) => Err(self.ask(request, file_len, Stage::Mpeg(lead, parser))),
                Step::Done(Ok(stream)) => {
                    self.next(mpeg::draft(stream, lead, file_len, &self.limits))
                }
                Step::Done(Err(error)) => Err(fail(ProbeError::Mpeg(error))),
            },
            Stage::Mp4(mut parser) => match parser.resume(window) {
                Step::Need(request) => Err(self.ask(request, file_len, Stage::Mp4(parser))),
                Step::Done(Ok(audio)) => self.next(mp4_facts::draft(audio, file_len)),
                Step::Done(Err(error)) => Err(fail(ProbeError::Mp4(error))),
            },
            Stage::Wav(mut parser) => match parser.resume(window) {
                Step::Need(request) => Err(self.ask(request, file_len, Stage::Wav(parser))),
                Step::Done(Ok(file)) => self.next(pcm::wav(file, &self.limits)),
                Step::Done(Err(error)) => Err(fail(ProbeError::Wav(error))),
            },
            Stage::Aiff(mut parser) => match parser.resume(window) {
                Step::Need(request) => Err(self.ask(request, file_len, Stage::Aiff(parser))),
                Step::Done(Ok(file)) => self.next(pcm::aiff(file, &self.limits)),
                Step::Done(Err(error)) => Err(fail(ProbeError::Aiff(error))),
            },
            Stage::OggHead(mut gather) => match self.fill(&mut gather, window) {
                Ok(Some(request)) => {
                    self.stage = Stage::OggHead(gather);
                    Err(Box::new(Step::Need(request)))
                }
                Ok(None) => self.ogg_head(gather, file_len),
                Err(error) => Err(fail(ProbeError::Read(error))),
            },
            Stage::Jobs(mut draft, job, mut gather) => match self.fill(&mut gather, window) {
                Ok(Some(request)) => {
                    self.stage = Stage::Jobs(draft, job, gather);
                    Err(Box::new(Step::Need(request)))
                }
                Ok(None) => {
                    self.job(&mut draft, job, gather, file_len);
                    self.next(*draft)
                }
                Err(error) => {
                    draft.problems.push(PartProblem::Read(error));
                    self.next(*draft)
                }
            },
            Stage::Finished => Err(fail(ProbeError::Finished)),
        }
    }

    /// The step that asks the host for `request` while the probe waits at
    /// `stage`, or the step that ends the probe when the limits refuse the
    /// read.
    fn ask(&mut self, request: ReadRequest, file_len: u64, stage: Stage<'b>) -> Box<Step<Outcome>> {
        match self.admit(request, file_len) {
            Ok(()) => {
                self.stage = stage;
                Box::new(Step::Need(request))
            }
            Err(error) => fail(ProbeError::Read(error)),
        }
    }

    /// Reads the header packets of an Ogg file from the octets gathered,
    /// or doubles the read when it does not hold them yet. Each attempt
    /// costs a step on top of the parsers' own, so the attempts end.
    fn ogg_head(&mut self, mut gather: Gather, file_len: u64) -> Turn<'b> {
        self.work
            .charge(1, gather.end)
            .map_err(|fault| fail(ProbeError::Fault(fault)))?;
        let more = gather.end < file_len;
        let found = ogg_facts::head(&gather.octets, more, file_len, &self.limits, &mut self.work);
        match found {
            None => {
                gather.end = gather.end.saturating_mul(2).min(file_len);
                Ok(Stage::OggHead(gather))
            }
            Some(Ok(draft)) => self.next(draft),
            Some(Err(error)) => Err(fail(error)),
        }
    }

    /// Admits `request` through the probe's own guard, which the first
    /// request makes for a file of `file_len` octets.
    fn admit(&mut self, request: ReadRequest, file_len: u64) -> Result<(), DriveError> {
        self.guard
            .get_or_insert_with(|| ReadGuard::new(file_len, &self.limits))
            .admit(request)
    }

    /// Starts the container parser of the format that was detected.
    fn begin(&mut self, detected: Detected, budget: &'b mut Budget, file_len: u64) -> Turn<'b> {
        let Detected { format, start } = detected;
        let keep = if format == Format::Flac {
            flac_metadata::STEPS_PER_OCTET
                .saturating_mul(file_len)
                .saturating_add(flac_metadata::STEPS_FIXED)
        } else {
            0
        };
        // Every step beyond what is kept moves to the probe's own budget.
        let moved = budget.remaining().saturating_sub(keep);
        // A budget holds at least what it has left, so this charge passes.
        let _ = budget.charge(moved, start);
        self.work = Budget::for_input(0, 0, moved);
        match format {
            Format::Flac => Ok(Stage::Flac(
                start,
                parse_metadata(start, &self.limits, budget),
            )),
            Format::Mpeg => {
                let budget = self.carve(mpa::STEPS_PER_OCTET, mpa::FIXED_STEPS, file_len)?;
                Ok(Stage::Mpeg(start, stream_info(&self.limits, budget)))
            }
            Format::Mp4 => {
                let budget = self.carve(mp4::STEPS_PER_BYTE, mp4::FIXED_STEPS, file_len)?;
                Ok(Stage::Mp4(Box::new(mp4::Probe::new(self.limits, budget))))
            }
            Format::Wav => {
                let budget = self.carve(riff::STEPS_PER_OCTET, riff::STEPS_FIXED, file_len)?;
                Ok(Stage::Wav(Wav::new(&self.limits, budget)))
            }
            Format::Aiff => {
                let budget = self.carve(riff::STEPS_PER_OCTET, riff::STEPS_FIXED, file_len)?;
                Ok(Stage::Aiff(Aiff::new(&self.limits, budget)))
            }
            Format::Ogg => Ok(Stage::OggHead(Gather::new(0, OGG_HEAD.min(file_len)))),
            Format::Jpeg | Format::Png | Format::Webp | Format::Gif | Format::Lrc | Format::M3u => {
                Err(fail(ProbeError::NotAudio { format }))
            }
        }
    }

    /// Takes the allowance of a container parser, `per_octet` steps for
    /// each octet of the file and `fixed` more, out of the probe's budget.
    ///
    /// # Errors
    ///
    /// The end of the probe, with [`ParseFault::BudgetExceeded`], when the
    /// budget holds less than the allowance.
    fn carve(
        &mut self,
        per_octet: u64,
        fixed: u64,
        file_len: u64,
    ) -> Result<Budget, Box<Step<Outcome>>> {
        let allowance = Budget::for_input(file_len, per_octet, fixed);
        self.work
            .charge(allowance.remaining(), 0)
            .map_err(|fault| fail(ProbeError::Fault(fault)))?;
        Ok(allowance)
    }

    /// Goes on with the next read `draft` still needs, or ends the probe
    /// with its result.
    fn next(&mut self, mut draft: Draft) -> Turn<'b> {
        if let Some((job, gather)) = draft.jobs.pop() {
            return Ok(Stage::Jobs(Box::new(draft), job, gather));
        }
        let read = self.guard.as_ref().map_or(0, ReadGuard::bytes_read);
        Err(Box::new(Step::Done(draft.finish(
            &self.limits,
            &mut self.work,
            read,
        ))))
    }

    /// Adds the octets of `window` to `gather` and asks for the next read
    /// of it: `None` once it holds every octet.
    ///
    /// # Errors
    ///
    /// The [`DriveError`] of a read the limits refuse.
    fn fill(
        &mut self,
        gather: &mut Gather,
        window: Window<'_>,
    ) -> Result<Option<ReadRequest>, DriveError> {
        gather.octets.extend_from_slice(window.bytes);
        let held = u64::try_from(gather.octets.len()).unwrap_or(u64::MAX);
        let at = gather.start.saturating_add(held);
        let left = gather.end.saturating_sub(at);
        if left == 0 {
            return Ok(None);
        }
        let len = left.min(self.limits.get(LimitKind::ReadBytes));
        let request = ReadRequest {
            offset: at,
            // A read is at most 16 MiB, which fits.
            len: u32::try_from(len).unwrap_or(u32::MAX),
        };
        self.admit(request, window.file_len).map(|()| Some(request))
    }

    /// Reads the tag block `job` asked for from the octets gathered.
    fn job(&mut self, draft: &mut Draft, job: Job, gather: Gather, file_len: u64) {
        match job {
            Job::Id3v2 => self.id3v2(draft, gather.window(file_len), LEADING_TAGS),
            Job::ChunkTag => self.id3v2(draft, gather.window(file_len), CHUNK_TAGS),
            Job::Comment => {
                match vorbis_comment::parse(
                    &gather.octets,
                    gather.start,
                    &self.limits,
                    &mut self.work,
                ) {
                    Ok(comments) => draft.tags.push(TagBlock::Vorbis(comments)),
                    Err(fault) => draft.problems.push(PartProblem::Comment(fault)),
                }
            }
            Job::Info => draft.tags.push(TagBlock::RiffInfo {
                offset: gather.start,
                octets: gather.octets,
            }),
            Job::Tail => self.tail(draft, gather.window(file_len)),
            Job::OggTail { serial, skip } => {
                let tail = gather.window(file_len).cursor();
                match ogg::last_granule(tail, serial, &mut self.work) {
                    Ok(granule) => {
                        draft.audio.duration = span(
                            granule.map(|granule| granule.saturating_sub(skip)),
                            draft.audio.sample_rate.map(SampleRate::hz),
                            &mut draft.problems,
                        );
                    }
                    Err(fault) => draft.problems.push(PartProblem::Fault(fault)),
                }
            }
        }
    }

    /// Reads the `ID3v2` tags that lie back to back in `window`, `most`
    /// of them at the most. A tag that cannot be read ends them, and is
    /// recorded.
    ///
    /// Each tag costs one step on top of what its parser charges, so a
    /// tag with no frames, which costs its parser nothing, is paid for
    /// too (SEC-MED-007). The tags have a budget of their own as well,
    /// of `most` tags, as detection has one of eight reads: the tag that
    /// would be one more is left unread with every tag after it, and the
    /// spent budget is recorded where that tag starts (SEC-TM-032). A tag
    /// that is not read costs no step.
    fn id3v2(&mut self, draft: &mut Draft, window: Window<'_>, most: u64) {
        let mut cursor = window.cursor();
        let mut tags = Budget::for_input(0, 0, most);
        loop {
            let offset = cursor.offset();
            if let Err(fault) = tags.charge(1, offset) {
                draft.problems.push(PartProblem::Fault(fault));
                break;
            }
            // A tag's own faults count octets from where the tag starts.
            let read = self
                .work
                .charge(1, 0)
                .map_err(id3v2::Id3v2Error::Fault)
                .and_then(|()| id3v2::parse(cursor.rest(), &self.limits, &mut self.work));
            match read {
                Ok(tag) => {
                    // A tag is at least its ten-octet header, so every
                    // turn moves on.
                    let len = tag.header.len;
                    draft.tags.push(TagBlock::Id3v2 { offset, tag });
                    if cursor.skip(len).is_err() || !cursor.rest().starts_with(b"ID3") {
                        break;
                    }
                }
                Err(error) => {
                    draft.problems.push(PartProblem::Id3v2 { offset, error });
                    break;
                }
            }
        }
    }

    /// Reads the APE and `ID3v1` tags at the end of the file from
    /// `window`, and ends the audio window where the first of them starts.
    ///
    /// An APE tag that starts before the window is read again from its
    /// start, unless it is larger than a tag may be in memory.
    fn tail(&mut self, draft: &mut Draft, window: Window<'_>) {
        match ape::parse_ape(window, &self.limits, &mut self.work) {
            Err(ApeError::Fault(ParseFault::Truncated { offset, .. }))
                if offset < window.offset =>
            {
                let len = window.file_len.saturating_sub(offset);
                match self.limits.check(LimitKind::Id3v2TagBytes, len, offset) {
                    Ok(()) => {
                        draft
                            .jobs
                            .push((Job::Tail, Gather::new(offset, window.file_len)));
                        return;
                    }
                    Err(fault) => draft.problems.push(PartProblem::Fault(fault)),
                }
            }
            Ok(Some(tag)) => {
                draft.window.1 = draft.window.1.min(tag.range.start);
                draft.tags.push(TagBlock::Ape(tag));
            }
            Ok(None) => {}
            Err(error) => draft.problems.push(PartProblem::Ape(error)),
        }
        match id3v1::find_v1(window, &self.limits, &mut self.work) {
            Ok(Some(tag)) => {
                draft.window.1 = draft.window.1.min(tag.range.start);
                draft.tags.push(TagBlock::Id3v1(tag));
            }
            Ok(None) => {}
            Err(error) => draft.problems.push(PartProblem::Id3v1(error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{AudioFormat, Codec, Container};
    use crate::formats::id3v1::Id3v1Error;
    use crate::formats::id3v2::{Header, Id3v2Error, Id3v2Tag};
    use crate::probe::SeekIndex;

    fn empty_draft() -> Draft {
        Draft {
            format: Format::Mpeg,
            codec: Some(Codec::Mp3),
            container: Container::Mpeg,
            at: 0,
            audio: AudioFormat {
                sample_rate: None,
                bit_depth: None,
                channels: None,
                bitrate: None,
                duration: None,
            },
            trim: None,
            md5: None,
            window: (0, 200),
            seek: SeekIndex::None,
            pictures: vec![],
            tags: vec![],
            problems: vec![],
            jobs: vec![],
        }
    }

    /// A retry must extend the tail toward the start of the file. A
    /// missing read at the same or a later offset cannot make progress.
    /// The final 128 octets of this file start at 72.
    ///
    /// Verifies: SEC-MED-008, SEC-MED-017
    #[test]
    fn records_truncated_tail_reads_that_cannot_extend_the_window() {
        for offset in [40, 72] {
            let mut budget = Budget::for_input(0, 0, 0);
            let mut probe = probe(None, Limits::DEFAULT, &mut budget);
            let mut draft = empty_draft();
            probe.tail(
                &mut draft,
                Window {
                    offset,
                    bytes: &[],
                    file_len: 200,
                },
            );
            assert_eq!(draft.jobs, []);
            assert_eq!(draft.tags, []);
            assert_eq!(draft.window, (0, 200));
            assert_eq!(
                draft.problems,
                [
                    PartProblem::Ape(ApeError::Fault(ParseFault::Truncated {
                        offset: 72,
                        needed: 128,
                        available: 0,
                    })),
                    PartProblem::Id3v1(Id3v1Error::Fault(ParseFault::Truncated {
                        offset: 72,
                        needed: 128,
                        available: 0,
                    })),
                ]
            );
        }
    }

    /// A tail starting at 100 lacks the start of the final 128 octets.
    /// Retrying from 72 extends it, so the missing data can be supplied.
    ///
    /// Verifies: SEC-MED-008, SEC-MED-017
    #[test]
    fn retries_a_truncated_tail_only_from_an_earlier_offset() {
        let mut budget = Budget::for_input(0, 0, 0);
        let mut probe = probe(None, Limits::DEFAULT, &mut budget);
        let mut draft = empty_draft();
        probe.tail(
            &mut draft,
            Window {
                offset: 100,
                bytes: &[],
                file_len: 200,
            },
        );
        assert_eq!(draft.jobs, [(Job::Tail, Gather::new(72, 200))]);
        assert_eq!(draft.tags, []);
        assert_eq!(draft.window, (0, 200));
        assert_eq!(draft.problems, []);
    }

    /// What an `ID3v2.3` tag with no frames, ten octets that start at
    /// `offset`, is read as.
    fn empty_tag_at(offset: u64) -> TagBlock {
        TagBlock::Id3v2 {
            offset,
            tag: Id3v2Tag {
                header: Header {
                    major: 3,
                    revision: 0,
                    flags: 0,
                    size: 0,
                    len: 10,
                },
                extended: None,
                frames: vec![],
                problems: vec![],
            },
        }
    }

    /// Reads the tags in front of the audio of a file of 4,000 octets that
    /// starts with `count` tags with no frames, back to back, with `steps`
    /// steps to spend. Returns the tags read, the problems recorded and
    /// the steps left.
    fn leading(count: usize, steps: u64) -> (Vec<TagBlock>, Vec<PartProblem>, u64) {
        let mut budget = Budget::for_input(0, 0, 0);
        let mut probe = probe(None, Limits::DEFAULT, &mut budget);
        probe.work = Budget::for_input(0, 0, steps);
        let mut draft = empty_draft();
        let octets: Vec<u8> = (0..count).flat_map(|_| *b"ID3\x03\0\0\0\0\0\0").collect();
        let gather = Gather {
            start: 0,
            end: u64::try_from(octets.len()).unwrap(),
            octets,
        };
        probe.job(&mut draft, Job::Id3v2, gather, 4_000);
        (draft.tags, draft.problems, probe.work.remaining())
    }

    /// Detection skips at most seven tags in front of the audio, so at
    /// most seven are read from there. One more is left unread with every
    /// tag after it, and what is recorded is where it starts: at 70. A
    /// file that changes between its two reads cannot make the probe keep
    /// 300 tags.
    ///
    /// Verifies: SEC-MED-017, SEC-TM-032
    #[test]
    fn reads_seven_tags_in_front_of_the_audio_and_records_an_eighth() {
        let seven = [0, 10, 20, 30, 40, 50, 60].map(empty_tag_at).to_vec();
        let eighth = vec![PartProblem::Fault(ParseFault::BudgetExceeded {
            offset: 70,
        })];
        let read = |count| {
            let (tags, problems, _) = leading(count, 1_000);
            (tags, problems)
        };
        assert_eq!(read(7), (seven.clone(), vec![]));
        assert_eq!(read(8), (seven.clone(), eighth.clone()));
        assert_eq!(read(300), (seven, eighth));
    }

    /// A tag costs one step on top of what its parser charges, which is
    /// nothing for a tag with no frames. With two steps the third tag, at
    /// 20, is not read. The tag past the seventh is not paid for.
    ///
    /// Verifies: SEC-MED-007, SEC-MED-017, SEC-TM-032
    #[test]
    fn charges_a_step_for_each_tag_it_reads() {
        let three = [0, 10, 20].map(empty_tag_at).to_vec();
        let spent = PartProblem::Id3v2 {
            offset: 20,
            error: Id3v2Error::Fault(ParseFault::BudgetExceeded { offset: 0 }),
        };
        assert_eq!(
            leading(3, 2),
            ([0, 10].map(empty_tag_at).to_vec(), vec![spent], 0)
        );
        assert_eq!(leading(3, 3), (three.clone(), vec![], 0));
        assert_eq!(leading(3, 4), (three, vec![], 1));
        let (_, _, left) = leading(8, 20);
        assert_eq!(left, 13);
    }
}
