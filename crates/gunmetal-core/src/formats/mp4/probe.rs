//! The MP4 audio probe: the sans-I/O entry point of the MP4 parser.
//!
//! [`Probe`] walks the boxes at the top of a file one header at a time,
//! asking its host for each header and for the bodies of the `ftyp` and
//! `moov` boxes only, wherever they lie, so the media data is never read
//! and a `moov` after `mdat` costs one more request (SEC-MED-010). Every
//! box at the top must fit the file, so a file cut short anywhere inside a
//! box is reported as [`ParseFault::Truncated`] at that box.
//!
//! The `moov` box is walked depth first: the first audio track gives the
//! [`AudioTrack`] and the [`SampleTableRanges`], and the item lists in
//! `moov/udta/meta` give the [`IlstItem`]s. A part of the metadata that
//! cannot be read is skipped with a recorded [`Mp4Problem`], and so is an
//! audio track after the first (SEC-MED-017); a malformed movie or audio
//! track fails the file.
//!
//! # Budget
//!
//! The probe charges one step of its [`Budget`] for every box it reads,
//! for every compatible brand of the `ftyp` box and for every descriptor
//! of an `esds` box. A box takes at least 8 octets, a brand 4 and a
//! descriptor 2, and none is charged twice, so a probe of a file of `n`
//! octets takes at most `n / 2` steps. A budget of
//! [`Budget::for_input`]`(n, `[`STEPS_PER_BYTE`]`, `[`FIXED_STEPS`]`)`
//! is never spent by any file (SEC-MED-007).

use super::audio::{AudioTrack, SampleTableRanges, TrackParts};
use super::boxes::{
    Event, Flow, FourCc, ILST, MDIA, META, MINF, Mp4Box, STBL, TRAK, UDTA, read_header, walk,
};
use super::ilst::{IlstItem, ItemList};
use crate::parse::{
    Budget, Cursor, Depth, LimitKind, Limits, ParseFault, ReadRequest, SansIo, Step, Window,
};
use crate::values::ValueError;

/// Steps of the budget a probe may need for each octet of its file.
pub const STEPS_PER_BYTE: u64 = 1;

/// Steps of the budget a probe may need whatever the size of its file.
pub const FIXED_STEPS: u64 = 8;

/// What the probe found in an MP4 audio file.
#[derive(Debug, Clone, PartialEq)]
pub struct Mp4Audio {
    /// The file type box, when the file has one.
    pub file_type: Option<FileType>,
    /// The first audio track.
    pub track: AudioTrack,
    /// The items of the movie's item lists, in file order.
    pub items: Vec<IlstItem>,
    /// Where the first audio track's sample tables are.
    pub sample_tables: SampleTableRanges,
    /// What was skipped or dropped on the way.
    pub problems: Vec<Mp4Problem>,
}

/// The file type box.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileType {
    /// The major brand, such as `M4A ` or `isom`.
    pub major: FourCc,
    /// The minor version.
    pub minor: u32,
    /// The compatible brands.
    pub compatible: Vec<FourCc>,
}

/// Why an MP4 file could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mp4Error {
    /// A failure every parser shares: truncation, the budget, the depth or
    /// a limit.
    Fault(ParseFault),
    /// A box declared a size smaller than its own header.
    BoxTooSmall {
        /// Where the box starts.
        offset: u64,
        /// The size it declared.
        size: u64,
        /// The length of its header.
        header_len: u64,
    },
    /// A field held a value this parser does not accept.
    Unexpected {
        /// The box the field is in.
        kind: FourCc,
        /// Where the field starts.
        offset: u64,
        /// The value it held.
        found: u64,
    },
    /// A box the audio track needs is missing.
    Missing {
        /// The box that is missing.
        kind: FourCc,
        /// Where the box that should hold it starts.
        offset: u64,
    },
    /// The audio track's media header gives a timescale of zero.
    ZeroTimescale {
        /// Where the timescale field starts.
        offset: u64,
    },
    /// The movie holds no audio track.
    NoAudioTrack {
        /// Where the `moov` box starts.
        offset: u64,
    },
    /// The file holds no `moov` box.
    NoMovie {
        /// The length of the file, where the search ended.
        offset: u64,
    },
}

impl Mp4Error {
    /// Where in the file the error was found.
    #[must_use]
    pub const fn offset(&self) -> u64 {
        match *self {
            Self::Fault(fault) => fault.offset(),
            Self::BoxTooSmall { offset, .. }
            | Self::Unexpected { offset, .. }
            | Self::Missing { offset, .. }
            | Self::ZeroTimescale { offset }
            | Self::NoAudioTrack { offset }
            | Self::NoMovie { offset } => offset,
        }
    }
}

/// Something the probe skipped or dropped, while keeping the rest of the
/// file (SEC-MED-017).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mp4Problem {
    /// An audio track after the first, which is not used.
    ExtraAudioTrack {
        /// Where its `trak` box starts.
        offset: u64,
    },
    /// A value outside its range, dropped (SEC-MED-014).
    Value {
        /// Where the box that held it starts.
        offset: u64,
        /// Why it was dropped.
        error: ValueError,
    },
    /// Part of the metadata that could not be read and was skipped.
    Metadata(Mp4Error),
}

impl From<ParseFault> for Mp4Error {
    fn from(fault: ParseFault) -> Self {
        Self::Fault(fault)
    }
}

/// The file type box.
const FTYP: FourCc = FourCc(*b"ftyp");
/// The movie box.
const MOOV: FourCc = FourCc(*b"moov");

/// The longest box header: a 64-bit size and an extended type.
const LONGEST_HEADER: u64 = 32;

/// A probe of one MP4 audio file, driven through the sans-I/O protocol.
///
/// Resume it with [`Window::start`] first; it then asks for each box
/// header at the top of the file, and for the bodies of the first `ftyp`
/// and the first `moov`. Every request is at least one octet, inside the
/// file, and within the read limits, so a host that enforces them never
/// has to refuse one (SEC-MED-010). Once it is done, resuming it again
/// starts a new parse with what is left of its budget.
#[derive(Debug)]
pub struct Probe {
    /// The limits the probe runs under.
    limits: Limits,
    /// The steps it has left.
    budget: Budget,
    /// Where the parse is.
    state: State,
}

/// Where a probe's parse is.
#[derive(Debug, Default)]
enum State {
    /// Before the first window.
    #[default]
    Start,
    /// Waiting for the header of the box at `offset`.
    Header {
        /// Where the box starts.
        offset: u64,
        /// What the parse has found so far.
        found: Found,
    },
    /// Waiting for the body of the `ftyp` box.
    FileType {
        /// Where the next box starts.
        next: u64,
        /// What the parse has found so far.
        found: Found,
    },
    /// Waiting for the body of the `moov` box at `offset`.
    Movie {
        /// Where the `moov` box starts.
        offset: u64,
        /// Its depth.
        depth: Depth,
        /// Where the next box starts.
        next: u64,
        /// What the parse has found so far.
        found: Found,
    },
}

/// What a parse has found so far.
#[derive(Debug, Default)]
struct Found {
    /// Boxes at the top of the file.
    boxes: u64,
    /// Octets asked for.
    read: u64,
    /// The first file type box.
    file_type: Option<FileType>,
    /// What the first movie holds.
    audio: Option<Mp4Audio>,
}

/// What a probe does after a window.
enum Next {
    /// Asks for a read and waits in a state.
    Read(State, ReadRequest),
    /// Finishes.
    Done(Mp4Audio),
}

impl Probe {
    /// A probe under `limits`, spending `budget`.
    #[must_use]
    pub const fn new(limits: Limits, budget: Budget) -> Self {
        Self {
            limits,
            budget,
            state: State::Start,
        }
    }

    /// Takes in `window` in `state`.
    fn step(&mut self, state: State, window: Window<'_>) -> Result<Next, Mp4Error> {
        let (next, found) = match state {
            State::Start => (0, Found::default()),
            State::Header { offset, found } => return self.header(window, offset, found),
            State::FileType { next, mut found } => {
                found.file_type = Some(file_type(window.cursor(), &mut self.budget)?);
                (next, found)
            }
            State::Movie {
                offset,
                depth,
                next,
                mut found,
            } => {
                let moov = Mp4Box {
                    kind: MOOV,
                    offset,
                    depth,
                    body: window.cursor(),
                };
                found.audio = Some(movie(moov, &self.limits, &mut self.budget)?);
                (next, found)
            }
        };
        self.next_header(next, found, window.file_len)
    }

    /// Asks for the header of the box at `offset`, or finishes at the end
    /// of the file.
    fn next_header(&self, offset: u64, mut found: Found, file_len: u64) -> Result<Next, Mp4Error> {
        let rest = file_len.saturating_sub(offset);
        if rest == 0 {
            let Some(mut audio) = found.audio else {
                return Err(Mp4Error::NoMovie { offset });
            };
            audio.file_type = found.file_type;
            return Ok(Next::Done(audio));
        }
        let request = self.request(&mut found, offset, rest.min(LONGEST_HEADER))?;
        Ok(Next::Read(State::Header { offset, found }, request))
    }

    /// A request for `len` octets at `offset`, within the read limits.
    fn request(&self, found: &mut Found, offset: u64, len: u64) -> Result<ReadRequest, Mp4Error> {
        self.limits.check(LimitKind::ReadBytes, len, offset)?;
        let read = found.read.saturating_add(len);
        self.limits.check(LimitKind::FileBytes, read, offset)?;
        found.read = read;
        Ok(ReadRequest {
            offset,
            // The read limit is at most 16 MiB, so this always fits.
            len: u32::try_from(len).unwrap_or(u32::MAX),
        })
    }

    /// Reads the header of the box at `offset` from `window`, and asks for
    /// its body or for the next header.
    fn header(
        &mut self,
        window: Window<'_>,
        offset: u64,
        mut found: Found,
    ) -> Result<Next, Mp4Error> {
        self.budget.charge(1, offset)?;
        found.boxes = found.boxes.saturating_add(1);
        self.limits
            .check(LimitKind::Children, found.boxes, offset)?;
        let mut cursor = window.cursor();
        let header = read_header(&mut cursor)?;
        let depth = Depth::CONTAINER_ROOT.descend(&self.limits, offset)?;
        let start = cursor.offset();
        let available = window.file_len.saturating_sub(start);
        let len = header.len.unwrap_or(available);
        if len > available {
            return Err(ParseFault::Truncated {
                offset: start,
                needed: len,
                available,
            }
            .into());
        }
        // The body fits the file, so this never saturates.
        let next = start.saturating_add(len);
        let movie = match header.kind {
            FTYP if found.file_type.is_none() => false,
            MOOV if found.audio.is_none() => true,
            _ => return self.next_header(next, found, window.file_len),
        };
        // An empty body needs no read, and a read of nothing is refused.
        let request = if len == 0 {
            None
        } else {
            Some(self.request(&mut found, start, len)?)
        };
        let state = if movie {
            State::Movie {
                offset,
                depth,
                next,
                found,
            }
        } else {
            State::FileType { next, found }
        };
        if let Some(request) = request {
            return Ok(Next::Read(state, request));
        }
        let empty = Window {
            offset: start,
            bytes: &[],
            file_len: window.file_len,
        };
        self.step(state, empty)
    }
}

impl SansIo for Probe {
    type Output = Result<Mp4Audio, Mp4Error>;

    fn resume(&mut self, window: Window<'_>) -> Step<Self::Output> {
        let state = std::mem::take(&mut self.state);
        match self.step(state, window) {
            Ok(Next::Read(state, request)) => {
                self.state = state;
                Step::Need(request)
            }
            Ok(Next::Done(audio)) => Step::Done(Ok(audio)),
            Err(error) => Step::Done(Err(error)),
        }
    }
}

/// Reads the body of a file type box, charging a step for each
/// compatible brand. Octets after the last whole brand are ignored.
fn file_type(mut body: Cursor<'_>, budget: &mut Budget) -> Result<FileType, Mp4Error> {
    let major = FourCc(body.array()?);
    let minor = body.u32_be()?;
    let mut compatible = Vec::new();
    loop {
        let at = body.offset();
        let Ok(brand) = body.array() else {
            break;
        };
        budget.charge(1, at)?;
        compatible.push(FourCc(brand));
    }
    Ok(FileType {
        major,
        minor,
        compatible,
    })
}

/// What a walk of a movie has found.
#[derive(Debug, Default)]
struct Movie<'a> {
    /// The parts of the track being walked.
    track: TrackParts<'a>,
    /// The first audio track and its sample tables.
    audio: Option<(AudioTrack, SampleTableRanges)>,
    /// The items of the item lists.
    items: ItemList,
    /// What was skipped or dropped.
    problems: Vec<Mp4Problem>,
}

impl<'a> Movie<'a> {
    /// Takes in one event of the walk, inside the open boxes `path`.
    fn visit(
        &mut self,
        path: &[FourCc],
        event: Event<'a>,
        limits: &Limits,
        budget: &mut Budget,
    ) -> Result<Flow, Mp4Error> {
        match (path, event) {
            ([MOOV], Event::Enter(trak @ Mp4Box { kind: TRAK, .. })) => {
                self.track = TrackParts::new(trak.offset);
            }
            ([MOOV], Event::Leave(Mp4Box { kind: TRAK, .. })) => self.end_track(limits, budget)?,
            ([MOOV, TRAK, MDIA], Event::Leaf(child)) => self.track.media(child)?,
            ([MOOV, TRAK, MDIA, MINF, STBL], Event::Leaf(child)) => self.track.table(child),
            ([MOOV, UDTA, META, ILST], Event::Enter(item)) => {
                return Ok(self.items.enter(item, limits, &mut self.problems));
            }
            ([MOOV, UDTA, META, ILST], Event::Leave(_)) => self.items.leave(&mut self.problems),
            ([MOOV, UDTA, META, ILST, _], Event::Leaf(child)) => {
                return Ok(self.items.leaf(child, limits, &mut self.problems));
            }
            ([MOOV, UDTA, ..], Event::Broken(error)) => {
                self.problems.push(Mp4Problem::Metadata(error));
            }
            (_, Event::Broken(error)) => return Err(error),
            _ => {}
        }
        Ok(Flow::Continue)
    }

    /// Reads the track just left if it is the first audio track, and
    /// records it if it is a later one.
    fn end_track(&mut self, limits: &Limits, budget: &mut Budget) -> Result<(), Mp4Error> {
        let track = std::mem::take(&mut self.track);
        if !track.is_audio() {
            return Ok(());
        }
        if self.audio.is_some() {
            self.problems.push(Mp4Problem::ExtraAudioTrack {
                offset: track.offset,
            });
            return Ok(());
        }
        self.audio = Some(track.audio(limits, budget, &mut self.problems)?);
        Ok(())
    }
}

/// Walks the movie box `moov`.
fn movie(moov: Mp4Box<'_>, limits: &Limits, budget: &mut Budget) -> Result<Mp4Audio, Mp4Error> {
    let mut movie = Movie::default();
    walk(moov, limits, budget, &mut |path, event, budget| {
        movie.visit(path, event, limits, budget)
    })?;
    let Some((track, sample_tables)) = movie.audio else {
        return Err(Mp4Error::NoAudioTrack {
            offset: moov.offset,
        });
    };
    Ok(Mp4Audio {
        file_type: None,
        track,
        items: movie.items.into_items(),
        sample_tables,
        problems: movie.problems,
    })
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use std::num::NonZeroU32;

    use super::*;
    use crate::formats::mp4::audio::{
        AudioEntry, AudioSpecificConfig, ChunkOffsets, CodecConfig, Esds, SampleSizes, Specific,
    };
    use crate::formats::mp4::ilst::{ItemKey, ItemValue, PictureRef};
    use crate::parse::{DriveError, LimitKind, ReadRequest, drive};
    use crate::text::Text;
    use crate::values::{BitDepth, Channels, SampleRate};
    use gunmetal_testkit::mp4::{
        self as kit, SampleEntry, data, freeform, ftyp, full_box, large_box, mp4_box, open_box,
        stsd, trak, udta,
    };
    use proptest::collection::vec;
    use proptest::prelude::*;

    /// Probes `file` under `limits` with `budget`, serving its requests
    /// from memory.
    fn run(
        file: &[u8],
        limits: &Limits,
        budget: Budget,
    ) -> Result<Result<Mp4Audio, Mp4Error>, DriveError> {
        drive(Probe::new(*limits, budget), file, limits)
    }

    /// The budget the module documentation promises is enough for `file`.
    fn enough(file: &[u8]) -> Budget {
        Budget::for_input(file.len() as u64, STEPS_PER_BYTE, FIXED_STEPS)
    }

    /// Probes `file` under the default limits with the promised budget.
    fn probe(file: &[u8]) -> Result<Mp4Audio, Mp4Error> {
        run(file, &Limits::DEFAULT, enough(file))
            .expect("the probe asks only for what the host allows")
    }

    /// `Limits::DEFAULT` with `kind` lowered to `value`.
    fn lowered(kind: LimitKind, value: u64) -> Limits {
        Limits::DEFAULT
            .with_override(kind, value)
            .expect("the test lowers a limit")
    }

    fn fault(fault: ParseFault) -> Mp4Error {
        Mp4Error::Fault(fault)
    }

    fn truncated(offset: u64, needed: u64, available: u64) -> Mp4Error {
        fault(ParseFault::Truncated {
            offset,
            needed,
            available,
        })
    }

    fn text(value: &str) -> Text {
        Text {
            value: value.to_owned(),
            truncated: false,
            replaced: false,
        }
    }

    /// The file type box of the standard file: 28 octets.
    fn brand() -> Vec<u8> {
        ftyp(*b"M4A ", 0x200, &[*b"M4A ", *b"mp42", *b"isom"])
    }

    fn brand_read() -> FileType {
        FileType {
            major: FourCc(*b"M4A "),
            minor: 0x200,
            compatible: vec![FourCc(*b"M4A "), FourCc(*b"mp42"), FourCc(*b"isom")],
        }
    }

    /// Media data of 24 octets.
    fn mdat() -> Vec<u8> {
        mp4_box(*b"mdat", &[0xAA; 16])
    }

    /// The sample description and tables of the standard track: AAC LC at
    /// 44.1 kHz in stereo, ten samples of 16 octets in one chunk.
    fn tables() -> Vec<u8> {
        let esds = kit::esds(&kit::Esds {
            object_type: 0x40,
            max_bitrate: 128_000,
            avg_bitrate: 96_000,
            specific: Some(&kit::audio_specific_config(2, 4, 2)),
            width: 4,
        });
        let entry = kit::sample_entry(&SampleEntry {
            format: *b"mp4a",
            version: 0,
            channels: 2,
            bits: 16,
            rate: 44_100,
            children: &esds,
        });
        [
            stsd(&[&entry]),
            full_box(*b"stts", 0, 0, &[0, 0, 0, 1, 0, 0, 0, 10, 0, 0, 4, 0]),
            full_box(
                *b"stsc",
                0,
                0,
                &[0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 10, 0, 0, 0, 1],
            ),
            full_box(*b"stsz", 0, 0, &[0, 0, 0, 16, 0, 0, 0, 10]),
            full_box(*b"stco", 0, 0, &[0, 0, 0, 1, 0, 0, 0, 0]),
        ]
        .concat()
    }

    /// A track of 292 octets with `handler` and the standard tables.
    fn track(handler: [u8; 4], timescale: u32) -> Vec<u8> {
        trak(handler, &kit::mdhd(timescale, 441_000), &tables())
    }

    /// The standard item list: a title, a freeform item and artwork.
    fn items() -> Vec<u8> {
        [
            mp4_box(*b"\xA9nam", &data(1, b"Song")),
            freeform(Some("com.apple.iTunes"), Some("iTunSMPB"), &[data(1, b"x")]),
            mp4_box(*b"covr", &data(13, &[0xFF, 0xD8])),
        ]
        .concat()
    }

    /// The standard movie, 488 octets: an audio track, then user data with
    /// the standard item list in a full `meta` box.
    fn movie() -> Vec<u8> {
        mp4_box(
            *b"moov",
            &[track(*b"soun", 44_100), udta(false, &items())].concat(),
        )
    }

    /// What the probe should find in the standard movie when its body
    /// starts at `body` - 8, as it does for a movie with an 8-octet header
    /// at `body` - 8.
    fn expected(at: u64, file_type: Option<FileType>) -> Mp4Audio {
        // The track starts at `t`: its sample description 97 octets in,
        // with the specific info 95 octets into that, and its tables after
        // it. The user data starts 292 octets after the track, and the
        // artwork's octets 186 octets into that.
        let t = at + 8;
        let u = t + 292;
        Mp4Audio {
            file_type,
            track: AudioTrack {
                offset: t,
                timescale: NonZeroU32::new(44_100).expect("non-zero"),
                duration: Some(441_000),
                entry: AudioEntry {
                    format: FourCc(*b"mp4a"),
                    channels: Channels::new(2).ok(),
                    bits: BitDepth::new(16).ok(),
                    sample_rate: SampleRate::new(44_100).ok(),
                    config: CodecConfig::Esds(Esds {
                        object_type: 0x40,
                        max_bitrate: 128_000,
                        avg_bitrate: 96_000,
                        specific: Some(Specific {
                            range: t + 97 + 95..t + 97 + 97,
                            audio: Some(AudioSpecificConfig {
                                object_type: 2,
                                sample_rate: SampleRate::new(44_100).ok(),
                                channel_config: 2,
                                extension: None,
                            }),
                        }),
                    }),
                },
            },
            items: vec![
                IlstItem {
                    key: ItemKey::Atom(FourCc(*b"\xA9nam")),
                    values: vec![ItemValue::Text(text("Song"))],
                },
                IlstItem {
                    key: ItemKey::Freeform {
                        mean: Some(text("com.apple.iTunes")),
                        name: text("iTunSMPB"),
                    },
                    values: vec![ItemValue::Text(text("x"))],
                },
                IlstItem {
                    key: ItemKey::Atom(FourCc(*b"covr")),
                    values: vec![ItemValue::Picture(PictureRef {
                        type_code: 13,
                        offset: u + 186,
                        len: 2,
                    })],
                },
            ],
            sample_tables: SampleTableRanges {
                stts: Some(t + 208..t + 224),
                stsc: Some(t + 232..t + 252),
                sizes: Some(SampleSizes::Stsz(t + 260..t + 272)),
                offsets: Some(ChunkOffsets::Stco(t + 280..t + 292)),
            },
            problems: vec![],
        }
    }

    /// Verifies: SEC-MED-001
    #[test]
    fn reads_a_file_with_the_movie_before_the_media_data() {
        let file = [brand(), movie(), mdat()].concat();
        assert_eq!(movie().len(), 488);
        assert_eq!(probe(&file), Ok(expected(28, Some(brand_read()))));
    }

    #[test]
    fn reads_a_file_with_the_movie_after_the_media_data() {
        let file = [brand(), mdat(), movie()].concat();
        assert_eq!(probe(&file), Ok(expected(52, Some(brand_read()))));
    }

    /// Verifies: SEC-MED-010
    #[test]
    fn asks_only_for_box_headers_and_the_bodies_it_reads() {
        // The media data comes first and is never read.
        let file = [brand(), mdat(), movie()].concat();
        let mut probe = Probe::new(Limits::DEFAULT, enough(&file));
        let mut window = Window::start(file.len() as u64);
        let mut asked = Vec::new();
        let result = loop {
            match probe.resume(window) {
                Step::Done(result) => break result,
                Step::Need(request) => {
                    asked.push(request);
                    let start = usize::try_from(request.offset).expect("the offset fits");
                    window = Window {
                        offset: request.offset,
                        bytes: &file[start..start + request.len as usize],
                        file_len: file.len() as u64,
                    };
                }
            }
        };
        assert_eq!(result, Ok(expected(52, Some(brand_read()))));
        let request = |offset, len| ReadRequest { offset, len };
        assert_eq!(
            asked,
            vec![
                request(0, 32),   // the ftyp header and what follows it
                request(8, 20),   // its body
                request(28, 32),  // the mdat header
                request(52, 32),  // the moov header
                request(60, 480), // its body
            ]
        );
    }

    /// Verifies: SEC-MED-008
    #[test]
    fn reads_boxes_of_size_zero_at_the_end_of_the_file() {
        let open_mdat = open_box(*b"mdat", &[0xAA; 16]);
        let file = [brand(), movie(), open_mdat].concat();
        assert_eq!(probe(&file), Ok(expected(28, Some(brand_read()))));
        let open_movie = open_box(*b"moov", &movie()[8..]);
        let file = [brand(), mdat(), open_movie].concat();
        assert_eq!(probe(&file), Ok(expected(52, Some(brand_read()))));
    }

    /// Verifies: SEC-MED-008
    #[test]
    fn reads_boxes_with_64_bit_sizes() {
        let file = [brand(), large_box(*b"mdat", &[0xAA; 16]), movie()].concat();
        assert_eq!(probe(&file), Ok(expected(60, Some(brand_read()))));
        // A movie with a 16-octet header: its body starts 8 octets later.
        let file = [brand(), large_box(*b"moov", &movie()[8..])].concat();
        assert_eq!(probe(&file), Ok(expected(36, Some(brand_read()))));
    }

    /// Verifies: SEC-MED-008
    #[test]
    fn refuses_a_box_of_size_7() {
        let file = [brand(), b"\x00\x00\x00\x07free".to_vec(), movie()].concat();
        assert_eq!(
            probe(&file),
            Err(Mp4Error::BoxTooSmall {
                offset: 28,
                size: 7,
                header_len: 8,
            })
        );
    }

    #[test]
    fn reads_a_file_without_a_file_type_box() {
        let file = [movie(), mdat()].concat();
        assert_eq!(probe(&file), Ok(expected(0, None)));
    }

    #[test]
    fn keeps_the_first_file_type_box_and_the_first_movie() {
        let other_brand = ftyp(*b"isom", 1, &[]);
        let other_movie = mp4_box(*b"moov", &track(*b"soun", 8_000));
        let file = [brand(), other_brand, movie(), other_movie].concat();
        assert_eq!(probe(&file), Ok(expected(44, Some(brand_read()))));
    }

    #[test]
    fn reads_a_file_type_box_with_no_brands_or_a_partial_one() {
        let file = [ftyp(*b"isom", 7, &[]), movie()].concat();
        let read = probe(&file).map(|audio| audio.file_type);
        assert_eq!(
            read,
            Ok(Some(FileType {
                major: FourCc(*b"isom"),
                minor: 7,
                compatible: vec![],
            }))
        );
        // Two octets after the last brand are not one.
        let file = [mp4_box(*b"ftyp", b"M4A \x00\x00\x00\x00mp42is"), movie()].concat();
        let read = probe(&file).map(|audio| audio.file_type);
        assert_eq!(
            read,
            Ok(Some(FileType {
                major: FourCc(*b"M4A "),
                minor: 0,
                compatible: vec![FourCc(*b"mp42")],
            }))
        );
    }

    /// Verifies: SEC-MED-001, SEC-MED-008
    #[test]
    fn reads_boxes_with_empty_bodies_without_asking_for_nothing() {
        // An empty ftyp lacks its major brand; an empty moov has no track.
        let file = [mp4_box(*b"ftyp", &[]), movie()].concat();
        assert_eq!(probe(&file), Err(truncated(8, 4, 0)));
        let file = [brand(), mp4_box(*b"moov", &[])].concat();
        assert_eq!(probe(&file), Err(Mp4Error::NoAudioTrack { offset: 28 }));
    }

    #[test]
    fn reads_a_plain_meta_box() {
        let file = [
            brand(),
            mp4_box(
                *b"moov",
                &[track(*b"soun", 44_100), udta(true, &items())].concat(),
            ),
        ]
        .concat();
        let mut wanted = expected(28, Some(brand_read()));
        // Without its version and flags the artwork is 4 octets earlier.
        wanted.items[2].values[0] = ItemValue::Picture(PictureRef {
            type_code: 13,
            offset: 28 + 8 + 292 + 182,
            len: 2,
        });
        assert_eq!(probe(&file), Ok(wanted));
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn refuses_an_audio_track_with_a_timescale_of_zero() {
        let file = [brand(), mp4_box(*b"moov", &track(*b"soun", 0))].concat();
        // moov 28, trak 36, mdia 44, mdhd 52 with its timescale 20 in.
        assert_eq!(probe(&file), Err(Mp4Error::ZeroTimescale { offset: 72 }));
    }

    #[test]
    fn uses_the_first_audio_track_and_records_the_second() {
        let file = [
            brand(),
            mp4_box(
                *b"moov",
                &[
                    track(*b"soun", 44_100),
                    track(*b"soun", 8_000),
                    udta(false, &items()),
                ]
                .concat(),
            ),
        ]
        .concat();
        let mut wanted = expected(28, Some(brand_read()));
        // The second track moves the artwork 292 octets on.
        wanted.items[2].values[0] = ItemValue::Picture(PictureRef {
            type_code: 13,
            offset: 28 + 8 + 292 + 292 + 186,
            len: 2,
        });
        wanted.problems = vec![Mp4Problem::ExtraAudioTrack { offset: 36 + 292 }];
        assert_eq!(probe(&file), Ok(wanted));
    }

    #[test]
    fn passes_over_tracks_that_are_not_audio() {
        let file = [
            brand(),
            mp4_box(
                *b"moov",
                &[
                    track(*b"vide", 0),
                    mp4_box(*b"trak", &[]),
                    track(*b"soun", 44_100),
                    udta(false, &items()),
                ]
                .concat(),
            ),
        ]
        .concat();
        // The video track and an empty track come first: 300 octets.
        assert_eq!(probe(&file), Ok(expected(28 + 300, Some(brand_read()))));
    }

    #[test]
    fn refuses_a_file_without_a_movie_or_an_audio_track() {
        assert_eq!(probe(&[]), Err(Mp4Error::NoMovie { offset: 0 }));
        assert_eq!(
            probe(&[brand(), mdat()].concat()),
            Err(Mp4Error::NoMovie { offset: 52 })
        );
        let file = [
            brand(),
            mp4_box(
                *b"moov",
                &[track(*b"vide", 1), udta(false, &items())].concat(),
            ),
        ]
        .concat();
        assert_eq!(probe(&file), Err(Mp4Error::NoAudioTrack { offset: 28 }));
    }

    /// `levels` boxes of type `kind`, each inside the one before, with an
    /// empty `free` box in the innermost.
    fn nested(kind: [u8; 4], levels: usize) -> Vec<u8> {
        (0..levels).fold(mp4_box(*b"free", &[]), |inner, _| mp4_box(kind, &inner))
    }

    /// Verifies: SEC-MED-005, SEC-TM-032
    #[test]
    fn refuses_tracks_nested_33_levels_deep() {
        // moov is level 1; 30 nested tracks reach level 31 and their free
        // box level 32, which is allowed.
        let file = [
            brand(),
            mp4_box(
                *b"moov",
                &[track(*b"soun", 44_100), nested(*b"trak", 30)].concat(),
            ),
        ]
        .concat();
        assert!(probe(&file).is_ok());
        // With 31 the free box, at 28 + 300 + 8 * 31, is level 33.
        let file = [
            brand(),
            mp4_box(
                *b"moov",
                &[track(*b"soun", 44_100), nested(*b"trak", 31)].concat(),
            ),
        ]
        .concat();
        assert_eq!(
            probe(&file),
            Err(fault(ParseFault::TooDeep {
                limit: LimitKind::ContainerDepth,
                depth: 33,
                max: 32,
                offset: 28 + 300 + 8 * 31,
            }))
        );
    }

    /// Verifies: SEC-MED-005
    #[test]
    fn skips_user_data_nested_33_levels_deep_and_keeps_the_rest() {
        let file = [
            brand(),
            mp4_box(
                *b"moov",
                &[track(*b"soun", 44_100), nested(*b"udta", 31)].concat(),
            ),
        ]
        .concat();
        let mut wanted = expected(28, Some(brand_read()));
        wanted.items = vec![];
        wanted.problems = vec![Mp4Problem::Metadata(fault(ParseFault::TooDeep {
            limit: LimitKind::ContainerDepth,
            depth: 33,
            max: 32,
            offset: 28 + 300 + 8 * 31,
        }))];
        assert_eq!(probe(&file), Ok(wanted));
    }

    #[test]
    fn skips_a_broken_part_of_the_metadata_and_keeps_the_rest() {
        // An item of size 7 after the standard items, at 28 + 300 + 188.
        let broken = [items(), b"\x00\x00\x00\x07\xA9day".to_vec()].concat();
        let file = [
            brand(),
            mp4_box(
                *b"moov",
                &[track(*b"soun", 44_100), udta(false, &broken)].concat(),
            ),
        ]
        .concat();
        let mut wanted = expected(28, Some(brand_read()));
        wanted.problems = vec![Mp4Problem::Metadata(Mp4Error::BoxTooSmall {
            offset: 28 + 300 + 188,
            size: 7,
            header_len: 8,
        })];
        assert_eq!(probe(&file), Ok(wanted));
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn refuses_a_short_file_type_box_or_track_handler() {
        // An ftyp with its major brand and no minor version, at 12.
        let file = [mp4_box(*b"ftyp", b"M4A "), movie()].concat();
        assert_eq!(probe(&file), Err(truncated(12, 4, 0)));
        // moov 28, trak 36, mdia 44, and an hdlr at 52 whose handler type,
        // at 68, has one octet.
        let handler = full_box(*b"hdlr", 0, 0, &[0, 0, 0, 0, b's']);
        let file = [
            brand(),
            mp4_box(*b"moov", &mp4_box(*b"trak", &mp4_box(*b"mdia", &handler))),
        ]
        .concat();
        assert_eq!(probe(&file), Err(truncated(68, 4, 1)));
    }

    #[test]
    fn refuses_a_broken_box_elsewhere_in_the_movie() {
        let file = [
            brand(),
            mp4_box(
                *b"moov",
                &[track(*b"soun", 44_100), b"\x00\x00\x00\x07free".to_vec()].concat(),
            ),
        ]
        .concat();
        assert_eq!(
            probe(&file),
            Err(Mp4Error::BoxTooSmall {
                offset: 28 + 300,
                size: 7,
                header_len: 8,
            })
        );
    }

    /// The top-level boxes of a file, as (start, length), and which one is
    /// the movie.
    struct Layout {
        file: Vec<u8>,
        boxes: Vec<(u64, u64)>,
        movie: usize,
        wanted: Mp4Audio,
    }

    fn layout(pieces: &[Vec<u8>], movie: usize, wanted: Mp4Audio) -> Layout {
        let mut boxes = Vec::new();
        let mut start = 0;
        for piece in pieces {
            boxes.push((start, piece.len() as u64));
            start += piece.len() as u64;
        }
        Layout {
            file: pieces.concat(),
            boxes,
            movie,
            wanted,
        }
    }

    /// What an independent model of the probe expects when `layout`'s file
    /// is cut after `cut` octets.
    fn cut_outcome(layout: &Layout, cut: u64) -> Result<Mp4Audio, Mp4Error> {
        let index = layout
            .boxes
            .iter()
            .position(|&(start, len)| cut < start + len)
            .expect("the cut is inside the file");
        let (start, len) = layout.boxes[index];
        let into = cut - start;
        match into {
            // A cut between boxes leaves a shorter, well-formed file.
            0 if index > layout.movie => Ok(layout.wanted.clone()),
            0 => Err(Mp4Error::NoMovie { offset: cut }),
            1..4 => Err(truncated(start, 4, into)),
            4..8 => Err(truncated(start + 4, 4, into - 4)),
            _ => Err(truncated(start + 8, len - 8, into - 8)),
        }
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn reports_every_cut_of_a_valid_file_exactly() {
        for layout in [
            layout(
                &[brand(), movie(), mdat()],
                1,
                expected(28, Some(brand_read())),
            ),
            layout(
                &[brand(), mdat(), movie()],
                2,
                expected(52, Some(brand_read())),
            ),
        ] {
            for cut in 0..layout.file.len() as u64 {
                assert_eq!(
                    probe(&layout.file[..usize::try_from(cut).expect("the cut fits")]),
                    cut_outcome(&layout, cut),
                    "cut at {cut}"
                );
            }
        }
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn runs_out_of_budget_at_exactly_the_step_it_should() {
        // Three top-level boxes, three brands, 23 boxes in the movie, the
        // sample entry, the esds box and its three descriptors: 34 steps,
        // the last for the mdat header at 516.
        let file = [brand(), movie(), mdat()].concat();
        assert_eq!(
            run(&file, &Limits::DEFAULT, Budget::for_input(0, 0, 34)),
            Ok(Ok(expected(28, Some(brand_read()))))
        );
        assert_eq!(
            run(&file, &Limits::DEFAULT, Budget::for_input(0, 0, 33)),
            Ok(Err(fault(ParseFault::BudgetExceeded { offset: 516 })))
        );
        // The brands are charged one by one: the third is at 24.
        assert_eq!(
            run(&file, &Limits::DEFAULT, Budget::for_input(0, 0, 3)),
            Ok(Err(fault(ParseFault::BudgetExceeded { offset: 24 })))
        );
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn counts_the_boxes_at_the_top_against_the_children_limit() {
        let free = mp4_box(*b"free", &[]);
        let file = [brand(), movie(), mdat(), free.clone(), free.clone(), free].concat();
        // The sample table's five children are the most any box has.
        assert!(
            run(&file, &lowered(LimitKind::Children, 6), enough(&file))
                .is_ok_and(|probe| probe.is_ok())
        );
        assert_eq!(
            run(&file, &lowered(LimitKind::Children, 5), enough(&file)),
            Ok(Err(fault(ParseFault::LimitExceeded {
                limit: LimitKind::Children,
                value: 6,
                max: 5,
                offset: 556,
            })))
        );
    }

    /// Verifies: SEC-MED-005
    #[test]
    fn counts_the_boxes_at_the_top_as_the_first_level() {
        let file = [brand(), movie()].concat();
        assert_eq!(
            run(&file, &lowered(LimitKind::ContainerDepth, 0), enough(&file)),
            Ok(Err(fault(ParseFault::TooDeep {
                limit: LimitKind::ContainerDepth,
                depth: 1,
                max: 0,
                offset: 0,
            })))
        );
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn refuses_to_ask_for_more_than_the_read_limits_allow() {
        let file = [brand(), movie(), mdat()].concat();
        // A movie body of 480 octets is longer than one read may be.
        assert_eq!(
            run(&file, &lowered(LimitKind::ReadBytes, 479), enough(&file)),
            Ok(Err(fault(ParseFault::LimitExceeded {
                limit: LimitKind::ReadBytes,
                value: 480,
                max: 479,
                offset: 36,
            })))
        );
        assert!(
            run(&file, &lowered(LimitKind::ReadBytes, 480), enough(&file))
                .is_ok_and(|probe| probe.is_ok())
        );
        // 32 + 20 + 32 octets come before the movie body: 564 in all.
        assert_eq!(
            run(&file, &lowered(LimitKind::FileBytes, 563), enough(&file)),
            Ok(Err(fault(ParseFault::LimitExceeded {
                limit: LimitKind::FileBytes,
                value: 564,
                max: 563,
                offset: 36,
            })))
        );
        // The last header read, of the 24 octets left, takes it to 588.
        assert!(
            run(&file, &lowered(LimitKind::FileBytes, 588), enough(&file))
                .is_ok_and(|probe| probe.is_ok())
        );
        assert_eq!(
            run(&file, &lowered(LimitKind::FileBytes, 587), enough(&file)),
            Ok(Err(fault(ParseFault::LimitExceeded {
                limit: LimitKind::FileBytes,
                value: 588,
                max: 587,
                offset: 516,
            })))
        );
        // A header read is refused like any other.
        assert_eq!(
            run(&file, &lowered(LimitKind::ReadBytes, 27), enough(&file)),
            Ok(Err(fault(ParseFault::LimitExceeded {
                limit: LimitKind::ReadBytes,
                value: 32,
                max: 27,
                offset: 0,
            })))
        );
    }

    #[test]
    fn reports_where_each_kind_of_error_happened() {
        let kind = FourCc(*b"mdhd");
        let errors = [
            fault(ParseFault::BudgetExceeded { offset: 1 }),
            Mp4Error::BoxTooSmall {
                offset: 2,
                size: 3,
                header_len: 8,
            },
            Mp4Error::Unexpected {
                kind,
                offset: 3,
                found: 9,
            },
            Mp4Error::Missing { kind, offset: 4 },
            Mp4Error::ZeroTimescale { offset: 5 },
            Mp4Error::NoAudioTrack { offset: 6 },
            Mp4Error::NoMovie { offset: 7 },
        ];
        assert_eq!(errors.map(|error| error.offset()), [1, 2, 3, 4, 5, 6, 7]);
    }

    /// Box headers, likely ones weighted up, so random files reach past the
    /// top level.
    fn header() -> impl Strategy<Value = Vec<u8>> {
        let kind = prop_oneof![
            Just(*b"moov"),
            Just(*b"trak"),
            Just(*b"mdia"),
            Just(*b"minf"),
            Just(*b"stbl"),
            Just(*b"stsd"),
            Just(*b"mdhd"),
            Just(*b"hdlr"),
            Just(*b"udta"),
            Just(*b"meta"),
            Just(*b"ilst"),
            Just(*b"ftyp"),
            Just(*b"uuid"),
            any::<[u8; 4]>(),
        ];
        (prop_oneof![0_u32..64, Just(0), Just(1), any::<u32>()], kind)
            .prop_map(|(size, kind)| [size.to_be_bytes().to_vec(), kind.to_vec()].concat())
    }

    proptest! {
        /// Verifies: SEC-MED-001, SEC-MED-007, SEC-MED-008, SEC-MED-010
        #[test]
        fn returns_for_any_bytes_without_a_refused_read_or_a_spent_budget(
            pieces in vec(prop_oneof![header(), vec(any::<u8>(), 0..24)], 0..24),
        ) {
            let file: Vec<u8> = pieces.concat();
            let result = crate::parse::small_stack::on_small_stack(move || {
                let budget = enough(&file);
                run(&file, &Limits::DEFAULT, budget).map(Result::err)
            });
            // The host never refuses a request, and the promised budget is
            // never spent.
            prop_assert!(result.is_ok(), "{:?}", result);
            prop_assert!(
                !matches!(result, Ok(Some(Mp4Error::Fault(ParseFault::BudgetExceeded { .. })))),
                "{:?}",
                result
            );
        }
    }
}
