//! WAV files in RIFF and RF64 forms, and the chunk walk RIFF and AIFF share.
//!
//! A WAV file is a RIFF form of type `WAVE` (Microsoft's Multimedia
//! Programming Interface and Data Specifications 1.0): the ID `RIFF`, a
//! 32-bit little-endian size that counts everything after it, the form
//! type `WAVE`, then chunks back to back. A chunk is a four-octet ID, a
//! 32-bit size that counts its body alone, the body, and one pad octet
//! after a body of odd length. RF64 (EBU Tech 3306) is the same form with
//! the ID `RF64` and a first chunk, `ds64`, that holds the 64-bit sizes a
//! file over 4 GiB needs. AIFF ([`super::aiff`]) is the same structure with
//! big-endian sizes, so the walk over the chunks lives here and serves both.
//!
//! # What the parser reads
//!
//! [`Wav`] is a sans-I/O parser. It asks for the 12-octet form header, then
//! for the first 48 octets at each chunk: the chunk's header and enough of
//! its body for the longest fixed fields it reads, those of
//! `WAVE_FORMAT_EXTENSIBLE`. It skips every body by offset, so it never
//! reads the samples. It reports:
//!
//! - the first `fmt ` chunk, as a typed [`WavFormat`]; a compressed format
//!   is recorded by its codec and not described further;
//! - where the samples of the first `data` chunk are;
//! - where the sub-chunks of the first `LIST` chunk of type `INFO` are, and
//!   the body of the first `id3 ` or `ID3 ` chunk, as raw ranges for the
//!   tag parsers.
//!
//! A later chunk with one of these IDs is skipped like any other.
//!
//! # Sizes
//!
//! The walk ends where the form's size says the form does, or at the end of
//! the file if that comes first, so octets after the form are never read. A
//! `data` size of all ones in a RIFF form is the "unknown" that streaming
//! writers leave behind, and runs to the end of the form. In an RF64 form
//! the form and `data` sizes come from `ds64`; its table of further chunk
//! sizes is not read, so any other chunk must carry its real size.
//!
//! # Damaged files
//!
//! Each chunk is checked against the form before its body is used. When the
//! walk cannot go on, because a chunk header or body runs past the end of
//! the form, a read comes back short, or the children limit or the step
//! budget is reached, the parse keeps what it found: a file whose format
//! and samples were found comes back with the reason in
//! [`WavFile::stopped`], and samples cut short end where the form does.
//! Only a file without a format or samples, or one with a malformed form
//! header, `ds64` or `fmt ` chunk, fails.
//!
//! # Limits
//!
//! The form's chunks are one level below its root, counted against the
//! container depth (SEC-MED-005). They are counted against the children
//! limit (SEC-MED-006), and each costs one step of the budget
//! (SEC-MED-007). Every chunk takes at least eight octets, so a file of n
//! octets costs at most n / 8 steps: [`STEPS_PER_OCTET`] × n +
//! [`STEPS_FIXED`] is always enough. Each chunk moves the walk on by at
//! least its header (SEC-MED-008), and the parser allocates nothing.

use std::fmt::Debug;
use std::num::NonZeroU16;

use crate::parse::{
    Budget, Cursor, Depth, LimitKind, Limits, ParseFault, ReadRequest, SansIo, Step, Window,
};
use crate::problem::{Arg, Describe, Problem, ProblemCode};
use crate::values::{BitDepth, Channels, SampleRate, ValueError};

/// The step budget's charge for each octet of the file: see the module
/// documentation.
pub const STEPS_PER_OCTET: u64 = 1;

/// The step budget's charge whatever the file's length.
pub const STEPS_FIXED: u64 = 0;

/// Octets before a form's first chunk: its ID, its size and its type.
const FORM_HEADER: u32 = 12;

/// Octets of a chunk's header, its ID and its size. A form is a chunk too,
/// so its size counts from this offset.
const CHUNK_HEADER: u64 = 8;

/// A run of octets in the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteRange {
    /// Where the octets start, from the start of the file.
    pub offset: u64,
    /// How many octets there are.
    pub len: u64,
}

/// Why the walk stopped at a chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Halt<E> {
    /// The walk cannot go on, but what it found may still be playable.
    Stop(ParseFault),
    /// The file is not playable.
    Fail(E),
}

impl<E> From<ParseFault> for Halt<E> {
    fn from(fault: ParseFault) -> Self {
        Self::Stop(fault)
    }
}

/// A chunk's header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Chunk {
    /// The chunk's ID.
    pub(crate) id: [u8; 4],
    /// The size of its body, as written.
    pub(crate) size: u32,
}

/// Where the walk over a form's chunks is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Walk {
    /// Where the chunk being read starts, or the next one.
    at: u64,
    /// Where the form ends: where its size says, or where the file does if
    /// that comes first.
    end: u64,
    /// Chunks read so far.
    chunks: u64,
}

impl Walk {
    /// A walk from the first chunk to the end of a file of `file_len`
    /// octets.
    fn new(file_len: u64) -> Self {
        Self {
            at: u64::from(FORM_HEADER),
            end: file_len,
            chunks: 0,
        }
    }

    /// This walk, ending where a form of `size` octets after its header
    /// does, if that comes first.
    pub(crate) fn within(self, size: u64) -> Self {
        Self {
            end: self.end.min(size.saturating_add(CHUNK_HEADER)),
            ..self
        }
    }

    /// Where the current chunk starts.
    pub(crate) const fn at(self) -> u64 {
        self.at
    }

    /// Where the current chunk's body starts.
    pub(crate) const fn body(self) -> u64 {
        self.at.saturating_add(CHUNK_HEADER)
    }

    /// Octets of the form from the current chunk's body to its end.
    pub(crate) const fn room(self) -> u64 {
        self.end.saturating_sub(self.body())
    }

    /// The walk past the current chunk, whose body is `len` octets, and its
    /// pad octet.
    ///
    /// # Errors
    ///
    /// [`ParseFault::Truncated`] when the body runs past the end of the
    /// form.
    pub(crate) fn past(self, len: u64) -> Result<Self, ParseFault> {
        let room = self.room();
        if len > room {
            return Err(ParseFault::Truncated {
                offset: self.body(),
                needed: len,
                available: room,
            });
        }
        // The body ends inside the form, so the pad octet passes its end by
        // one at most, which ends the walk.
        Ok(Self {
            at: self.body().saturating_add(len).saturating_add(len & 1),
            ..self
        })
    }

    /// The read for the next chunk's header and its first `peek` octets in
    /// all, or `None` once the form is over.
    ///
    /// # Errors
    ///
    /// [`ParseFault::Truncated`] when the form ends inside a chunk header.
    fn request(self, peek: u32) -> Result<Option<ReadRequest>, ParseFault> {
        let left = self.end.saturating_sub(self.at);
        if left == 0 {
            return Ok(None);
        }
        if left < CHUNK_HEADER {
            return Err(ParseFault::Truncated {
                offset: self.at,
                needed: CHUNK_HEADER,
                available: left,
            });
        }
        Ok(Some(ReadRequest {
            offset: self.at,
            len: peek.min(u32::try_from(left).unwrap_or(u32::MAX)),
        }))
    }

    /// Reads the header of the chunk at the start of `window`, whose size is
    /// in the byte order `size` reads, counting it against the children
    /// limit and charging it to the budget.
    ///
    /// # Errors
    ///
    /// [`ParseFault::Truncated`] when the window holds no whole header,
    /// [`ParseFault::LimitExceeded`] for one chunk too many and
    /// [`ParseFault::BudgetExceeded`] when the budget is spent.
    fn enter(
        self,
        window: &mut Cursor<'_>,
        size: fn([u8; 4]) -> u32,
        limits: &Limits,
        budget: &mut Budget,
    ) -> Result<(Chunk, Self), ParseFault> {
        let [i0, i1, i2, i3, s0, s1, s2, s3] = window.array()?;
        let chunks = self.chunks.saturating_add(1);
        limits.check(LimitKind::Children, chunks, self.at)?;
        budget.charge(1, self.at)?;
        let chunk = Chunk {
            id: [i0, i1, i2, i3],
            size: size([s0, s1, s2, s3]),
        };
        Ok((chunk, Self { chunks, ..self }))
    }
}

/// What a form of chunks adds to the walk: its header, its byte order, and
/// what it makes of each chunk.
pub(crate) trait Layout: Copy + Debug {
    /// What a parse of the form produces.
    type File: Copy + Debug;
    /// Why a parse of the form fails.
    type Error: Copy + Debug + From<ParseFault>;
    /// Octets read at each chunk: its header and the most of its body the
    /// layout reads.
    const PEEK: u32;

    /// A chunk size, read in the form's byte order.
    fn size(octets: [u8; 4]) -> u32;

    /// Reads the form header: the layout, and the form's size after its ID
    /// and size field.
    ///
    /// # Errors
    ///
    /// The layout's error when the header is not one of its forms.
    fn open(header: [u8; 12]) -> Result<(Self, u64), Self::Error>;

    /// Handles one chunk. `body` reads the window after the chunk's header,
    /// and `walk` is at the chunk.
    ///
    /// # Errors
    ///
    /// [`Halt::Stop`] when the walk cannot go on, [`Halt::Fail`] when the
    /// file is not playable.
    fn chunk(
        &mut self,
        chunk: Chunk,
        body: Cursor<'_>,
        walk: Walk,
    ) -> Result<Walk, Halt<Self::Error>>;

    /// What the parse produces once the walk is over, stopped early by
    /// `stopped` or not.
    ///
    /// # Errors
    ///
    /// The layout's error when what was found is not playable.
    fn finish(self, stopped: Option<ParseFault>) -> Result<Self::File, Self::Error>;
}

/// The result of a parse of a form of layout `L`.
type Parsed<L> = Result<<L as Layout>::File, <L as Layout>::Error>;

/// The sans-I/O walk over a form of chunks, for any [`Layout`].
#[derive(Debug)]
pub(crate) struct Parser<L: Layout> {
    /// The limits of the parse.
    limits: Limits,
    /// The steps left.
    budget: Budget,
    /// What the parser is waiting for.
    state: State<L>,
}

/// What a [`Parser`] is waiting for.
#[derive(Debug, Clone, Copy)]
enum State<L: Layout> {
    /// The first window, which says how long the file is.
    Start,
    /// The form header.
    Form,
    /// The chunk the walk is at.
    Chunks {
        /// Where the walk is.
        walk: Walk,
        /// What it found so far.
        layout: L,
    },
    /// Nothing: the parse is over.
    Done(Parsed<L>),
}

impl<L: Layout> Parser<L> {
    /// A parse under `limits`, spending `budget`.
    pub(crate) const fn new(limits: &Limits, budget: Budget) -> Self {
        Self {
            limits: *limits,
            budget,
            state: State::Start,
        }
    }

    /// Continues the parse with `window`; see [`SansIo::resume`]. A parse
    /// that is over answers its result again.
    pub(crate) fn resume(&mut self, window: Window<'_>) -> Step<Parsed<L>> {
        match self.state {
            State::Start => self.start(window.file_len),
            State::Form => self.form(window),
            State::Chunks { walk, layout } => self.chunk(window, walk, layout),
            State::Done(result) => Step::Done(result),
        }
    }

    /// Asks for the form header of a file of `file_len` octets.
    fn start(&mut self, file_len: u64) -> Step<Parsed<L>> {
        let needed = u64::from(FORM_HEADER);
        if file_len < needed {
            return self.end(Err(ParseFault::Truncated {
                offset: 0,
                needed,
                available: file_len,
            }
            .into()));
        }
        self.state = State::Form;
        Step::Need(ReadRequest {
            offset: 0,
            len: FORM_HEADER,
        })
    }

    /// Reads the form header in `window` and starts the walk.
    fn form(&mut self, window: Window<'_>) -> Step<Parsed<L>> {
        match self.open(window) {
            Ok((walk, layout)) => self.next(walk, layout),
            Err(error) => self.end(Err(error)),
        }
    }

    /// The walk over the form whose header is in `window`, and its layout.
    fn open(&self, window: Window<'_>) -> Result<(Walk, L), L::Error> {
        let header = window.cursor().array()?;
        let (layout, size) = L::open(header)?;
        Depth::CONTAINER_ROOT.descend(&self.limits, u64::from(FORM_HEADER))?;
        Ok((Walk::new(window.file_len).within(size), layout))
    }

    /// Asks for the chunk `walk` is at, or ends the parse when the form is
    /// over.
    fn next(&mut self, walk: Walk, layout: L) -> Step<Parsed<L>> {
        match walk.request(L::PEEK) {
            Ok(Some(request)) => {
                self.state = State::Chunks { walk, layout };
                Step::Need(request)
            }
            Ok(None) => self.end(layout.finish(None)),
            Err(fault) => self.end(layout.finish(Some(fault))),
        }
    }

    /// Handles the chunk at the start of `window` and moves on.
    fn chunk(&mut self, window: Window<'_>, walk: Walk, mut layout: L) -> Step<Parsed<L>> {
        match self.visit(window, walk, &mut layout) {
            Ok(walk) => self.next(walk, layout),
            Err(Halt::Stop(fault)) => self.end(layout.finish(Some(fault))),
            Err(Halt::Fail(error)) => self.end(Err(error)),
        }
    }

    /// Reads the header of the chunk at the start of `window` and lets the
    /// layout handle it.
    fn visit(
        &mut self,
        window: Window<'_>,
        walk: Walk,
        layout: &mut L,
    ) -> Result<Walk, Halt<L::Error>> {
        let mut body = window.cursor();
        let (chunk, walk) = walk.enter(&mut body, L::size, &self.limits, &mut self.budget)?;
        layout.chunk(chunk, body, walk)
    }

    /// Ends the parse with `result`.
    fn end(&mut self, result: Parsed<L>) -> Step<Parsed<L>> {
        self.state = State::Done(result);
        Step::Done(result)
    }
}

/// A problem of the code `code`, for the reason `reason`, found at
/// `offset` when the error has one.
pub(crate) fn problem(code: ProblemCode, reason: &'static str, offset: Option<u64>) -> Problem {
    let mut args = vec![("reason", Arg::Name(reason))];
    args.extend(offset.map(|offset| ("offset", Arg::Number(offset))));
    Problem { code, args }
}

/// The reason a problem gives for `fault`.
pub(crate) const fn fault_reason(fault: ParseFault) -> &'static str {
    match fault {
        ParseFault::Truncated { .. } => "truncated",
        ParseFault::BudgetExceeded { .. } => "budget_exceeded",
        ParseFault::TooDeep { .. } => "too_deep",
        ParseFault::LimitExceeded { .. } => "limit_exceeded",
        ParseFault::NotSyncsafe { .. } => "not_syncsafe",
    }
}

/// The ID of a RIFF form.
const RIFF: [u8; 4] = *b"RIFF";
/// The ID of an RF64 form.
const RF64: [u8; 4] = *b"RF64";
/// The form type of a WAV file.
const WAVE: [u8; 4] = *b"WAVE";
/// The chunk with an RF64 form's 64-bit sizes.
const DS64: [u8; 4] = *b"ds64";
/// The chunk that describes the samples.
const FMT: [u8; 4] = *b"fmt ";
/// The chunk that holds the samples.
const DATA: [u8; 4] = *b"data";
/// A chunk of sub-chunks, whose first four octets give its type.
const LIST: [u8; 4] = *b"LIST";
/// The `LIST` type whose sub-chunks are text tags.
const INFO: [u8; 4] = *b"INFO";
/// An `ID3v2` tag, as most writers name its chunk.
const ID3: [u8; 4] = *b"id3 ";
/// An `ID3v2` tag, as some writers name its chunk.
const ID3_UPPER: [u8; 4] = *b"ID3 ";

/// The format tag of `WAVE_FORMAT_EXTENSIBLE`.
const EXTENSIBLE: u16 = 0xFFFE;
/// Octets of the `PCMWAVEFORMAT` fields every `fmt ` chunk starts with.
const FMT_FIELDS: u64 = 16;
/// Octets of a `WAVEFORMATEXTENSIBLE` `fmt ` chunk.
const FMT_EXTENSIBLE: u64 = 40;
/// Octets of the fixed fields of a `ds64` chunk: three sizes and the
/// length of its table.
const DS64_FIELDS: u64 = 28;
/// The last 14 octets of every sub-format GUID that stands for a format
/// tag, `xxxxxxxx-0000-0010-8000-00AA00389B71`, after the tag's two.
const TAG_GUID_TAIL: [u8; 14] = [
    0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
];

/// Which RIFF form a WAV file uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WavForm {
    /// `RIFF`, with 32-bit sizes.
    Riff,
    /// `RF64` (EBU Tech 3306), with 64-bit sizes in its `ds64` chunk.
    Rf64,
}

/// How the samples of a WAV file are coded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WavCodec {
    /// Integer PCM: format tag 1, or the extensible PCM sub-format.
    Pcm,
    /// IEEE 754 floating point: format tag 3, or the extensible float
    /// sub-format.
    Float,
    /// Any other format tag, written plainly or as an extensible
    /// sub-format; its samples are not described further.
    Other(u16),
    /// An extensible sub-format that stands for no format tag; the GUID is
    /// in [`Extensible::sub_format`].
    Unknown,
}

/// The extension of `WAVE_FORMAT_EXTENSIBLE`, format tag 0xFFFE.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extensible {
    /// Bits of each sample that carry signal, when from 1 to 64.
    pub valid_bits: Option<BitDepth>,
    /// Which speaker position each channel feeds, one bit each, as written.
    pub channel_mask: u32,
    /// The sub-format GUID, as its 16 octets are written.
    pub sub_format: [u8; 16],
}

/// What a `fmt ` chunk says about the samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WavFormat {
    /// How the samples are coded.
    pub codec: WavCodec,
    /// How many channels there are.
    pub channels: Channels,
    /// Frames a second.
    pub sample_rate: SampleRate,
    /// The average octets a second, as written.
    pub bytes_per_second: u32,
    /// Octets in one frame, unless written as zero.
    pub block_align: Option<NonZeroU16>,
    /// Bits in one sample's container, when from 1 to 64. Compressed
    /// formats often write zero.
    pub bits_per_sample: Option<BitDepth>,
    /// The fields of `WAVE_FORMAT_EXTENSIBLE`, when the format tag is
    /// 0xFFFE.
    pub extensible: Option<Extensible>,
}

/// What a WAV file holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WavFile {
    /// Which form the file uses.
    pub form: WavForm,
    /// The first `fmt ` chunk.
    pub format: WavFormat,
    /// The samples of the first `data` chunk, ending where the form does
    /// if the chunk runs past it.
    pub data: ByteRange,
    /// The sub-chunks of the first `LIST` chunk of type `INFO`.
    pub info: Option<ByteRange>,
    /// The body of the first `id3 ` or `ID3 ` chunk: an `ID3v2` tag.
    pub id3: Option<ByteRange>,
    /// Why the walk stopped before the end of the form, if it did.
    pub stopped: Option<ParseFault>,
}

/// Why a WAV file cannot be played.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiffError {
    /// A failure every parser shares, before the format and the samples
    /// were both found.
    Fault(ParseFault),
    /// The file does not start with `RIFF` or `RF64`.
    NotRiff {
        /// The first four octets.
        found: [u8; 4],
    },
    /// The form type, at offset 8, is not `WAVE`.
    NotWave {
        /// The form type.
        found: [u8; 4],
    },
    /// The first chunk of an RF64 form, at offset 12, is not `ds64`.
    NoDs64 {
        /// The chunk's ID.
        found: [u8; 4],
    },
    /// A chunk is shorter than the fixed fields it must hold.
    Short {
        /// The chunk's ID.
        id: [u8; 4],
        /// Where the chunk starts.
        offset: u64,
        /// The size of its body.
        len: u64,
        /// The size its fields need.
        needed: u64,
    },
    /// A `fmt ` chunk holds a channel count or sample rate out of range.
    Value {
        /// Where the chunk starts.
        offset: u64,
        /// Which value, and why.
        error: ValueError,
    },
    /// The form has no chunk with this ID, which a playable file needs.
    Missing {
        /// The chunk's ID.
        id: [u8; 4],
    },
}

impl From<ParseFault> for RiffError {
    fn from(fault: ParseFault) -> Self {
        Self::Fault(fault)
    }
}

impl Describe for RiffError {
    /// An unreadable WAV file, with the reason and where it was found.
    fn problem(&self) -> Problem {
        let (reason, offset) = match *self {
            Self::Fault(fault) => (fault_reason(fault), Some(fault.offset())),
            Self::NotRiff { .. } => ("not_riff", Some(0)),
            Self::NotWave { .. } => ("not_wave", Some(8)),
            Self::NoDs64 { .. } => ("no_ds64", Some(u64::from(FORM_HEADER))),
            Self::Short { offset, .. } => ("short_chunk", Some(offset)),
            Self::Value { offset, .. } => ("bad_value", Some(offset)),
            Self::Missing { .. } => ("missing_chunk", None),
        };
        problem(ProblemCode::WavUnreadable, reason, offset)
    }
}

/// The parser for one WAV file: see the module documentation.
#[derive(Debug)]
pub struct Wav(Parser<Wave>);

impl Wav {
    /// A parser for one file, under `limits`, spending `budget`.
    /// `Budget::for_input(file_len, STEPS_PER_OCTET, STEPS_FIXED)` is always
    /// enough.
    #[must_use]
    pub const fn new(limits: &Limits, budget: Budget) -> Self {
        Self(Parser::new(limits, budget))
    }
}

impl SansIo for Wav {
    type Output = Result<WavFile, RiffError>;

    fn resume(&mut self, window: Window<'_>) -> Step<Self::Output> {
        self.0.resume(window)
    }
}

/// What the walk over a WAV file has found so far.
#[derive(Debug, Clone, Copy)]
struct Wave {
    /// Which form the file uses.
    form: WavForm,
    /// The `data` size from an RF64 form's `ds64` chunk, once read.
    data_size: Option<u64>,
    /// The first `fmt ` chunk.
    format: Option<WavFormat>,
    /// The first `data` chunk's samples.
    data: Option<ByteRange>,
    /// The first `INFO` list's sub-chunks.
    info: Option<ByteRange>,
    /// The first `ID3v2` tag.
    id3: Option<ByteRange>,
}

impl Layout for Wave {
    type File = WavFile;
    type Error = RiffError;
    const PEEK: u32 = 48;

    fn size(octets: [u8; 4]) -> u32 {
        u32::from_le_bytes(octets)
    }

    fn open(header: [u8; 12]) -> Result<(Self, u64), RiffError> {
        let [i0, i1, i2, i3, s0, s1, s2, s3, t0, t1, t2, t3] = header;
        let (form, size) = match [i0, i1, i2, i3] {
            RIFF => (
                WavForm::Riff,
                u64::from(u32::from_le_bytes([s0, s1, s2, s3])),
            ),
            // The form's size is in its ds64 chunk.
            RF64 => (WavForm::Rf64, u64::MAX),
            found => return Err(RiffError::NotRiff { found }),
        };
        let found = [t0, t1, t2, t3];
        if found != WAVE {
            return Err(RiffError::NotWave { found });
        }
        let wave = Self {
            form,
            data_size: None,
            format: None,
            data: None,
            info: None,
            id3: None,
        };
        Ok((wave, size))
    }

    fn chunk(
        &mut self,
        chunk: Chunk,
        mut body: Cursor<'_>,
        walk: Walk,
    ) -> Result<Walk, Halt<RiffError>> {
        if self.form == WavForm::Rf64 && self.data_size.is_none() {
            return self.ds64(chunk, body, walk);
        }
        let size = u64::from(chunk.size);
        match chunk.id {
            FMT if self.format.is_none() => {
                short(FMT, size, FMT_FIELDS, walk)?;
                let next = walk.past(size)?;
                self.format = Some(format(size, &mut body, walk)?);
                Ok(next)
            }
            DATA if self.data.is_none() => {
                let len = match (chunk.size, self.data_size) {
                    (u32::MAX, Some(len)) => len,
                    // A streaming writer could not go back to fill it in.
                    (u32::MAX, None) => walk.room(),
                    _ => size,
                };
                self.data = Some(ByteRange {
                    offset: walk.body(),
                    len: len.min(walk.room()),
                });
                Ok(walk.past(len)?)
            }
            LIST if self.info.is_none() => {
                let next = walk.past(size)?;
                if size >= 4 && body.array()? == INFO {
                    self.info = Some(ByteRange {
                        offset: walk.body().saturating_add(4),
                        len: size.saturating_sub(4),
                    });
                }
                Ok(next)
            }
            ID3 | ID3_UPPER if self.id3.is_none() => {
                let next = walk.past(size)?;
                self.id3 = Some(ByteRange {
                    offset: walk.body(),
                    len: size,
                });
                Ok(next)
            }
            _ => Ok(walk.past(size)?),
        }
    }

    fn finish(self, stopped: Option<ParseFault>) -> Result<WavFile, RiffError> {
        let missing = |id| stopped.map_or(RiffError::Missing { id }, RiffError::Fault);
        let format = self.format.ok_or_else(|| missing(FMT))?;
        let data = self.data.ok_or_else(|| missing(DATA))?;
        Ok(WavFile {
            form: self.form,
            format,
            data,
            info: self.info,
            id3: self.id3,
            stopped,
        })
    }
}

impl Wave {
    /// Reads the `ds64` chunk an RF64 form must start with, and ends the
    /// walk where its form size says the form does.
    fn ds64(
        &mut self,
        chunk: Chunk,
        mut body: Cursor<'_>,
        walk: Walk,
    ) -> Result<Walk, Halt<RiffError>> {
        if chunk.id != DS64 {
            return Err(Halt::Fail(RiffError::NoDs64 { found: chunk.id }));
        }
        let size = u64::from(chunk.size);
        short(DS64, size, DS64_FIELDS, walk)?;
        let next = walk.past(size)?;
        let [
            r0,
            r1,
            r2,
            r3,
            r4,
            r5,
            r6,
            r7,
            d0,
            d1,
            d2,
            d3,
            d4,
            d5,
            d6,
            d7,
        ] = body.array()?;
        self.data_size = Some(u64::from_le_bytes([d0, d1, d2, d3, d4, d5, d6, d7]));
        Ok(next.within(u64::from_le_bytes([r0, r1, r2, r3, r4, r5, r6, r7])))
    }
}

/// Refuses the chunk `walk` is at, of ID `id` and size `size`, when it is
/// shorter than the `needed` octets of its fields.
fn short(id: [u8; 4], size: u64, needed: u64, walk: Walk) -> Result<(), Halt<RiffError>> {
    if size < needed {
        return Err(Halt::Fail(RiffError::Short {
            id,
            offset: walk.at(),
            len: size,
            needed,
        }));
    }
    Ok(())
}

/// A typed value from the `fmt ` chunk `walk` is at, or the error for it.
fn value<T>(read: Result<T, ValueError>, walk: Walk) -> Result<T, Halt<RiffError>> {
    read.map_err(|error| {
        Halt::Fail(RiffError::Value {
            offset: walk.at(),
            error,
        })
    })
}

/// Bits per sample, when from 1 to 64.
fn depth(bits: u16) -> Option<BitDepth> {
    BitDepth::new(u32::from(bits)).ok()
}

/// The codec of format tag `tag`.
const fn codec(tag: u16) -> WavCodec {
    match tag {
        1 => WavCodec::Pcm,
        3 => WavCodec::Float,
        other => WavCodec::Other(other),
    }
}

/// The codec of an extensible sub-format GUID.
fn sub_codec(guid: [u8; 16]) -> WavCodec {
    let [t0, t1, tail @ ..] = guid;
    if tail == TAG_GUID_TAIL {
        codec(u16::from_le_bytes([t0, t1]))
    } else {
        WavCodec::Unknown
    }
}

/// Reads a `fmt ` chunk of `size` octets, at least 16, from `body`.
fn format(size: u64, body: &mut Cursor<'_>, walk: Walk) -> Result<WavFormat, Halt<RiffError>> {
    let [
        t0,
        t1,
        c0,
        c1,
        r0,
        r1,
        r2,
        r3,
        b0,
        b1,
        b2,
        b3,
        a0,
        a1,
        s0,
        s1,
    ] = body.array()?;
    let tag = u16::from_le_bytes([t0, t1]);
    let extensible = if tag == EXTENSIBLE {
        short(FMT, size, FMT_EXTENSIBLE, walk)?;
        // The two octets skipped give the extension's size, which the
        // chunk's size already bounds.
        let [_, _, v0, v1, m0, m1, m2, m3, sub_format @ ..] = body.array::<24>()?;
        Some(Extensible {
            valid_bits: depth(u16::from_le_bytes([v0, v1])),
            channel_mask: u32::from_le_bytes([m0, m1, m2, m3]),
            sub_format,
        })
    } else {
        None
    };
    Ok(WavFormat {
        codec: extensible.map_or(codec(tag), |extensible| sub_codec(extensible.sub_format)),
        channels: value(Channels::new(u32::from(u16::from_le_bytes([c0, c1]))), walk)?,
        sample_rate: value(SampleRate::new(u32::from_le_bytes([r0, r1, r2, r3])), walk)?,
        bytes_per_second: u32::from_le_bytes([b0, b1, b2, b3]),
        block_align: NonZeroU16::new(u16::from_le_bytes([a0, a1])),
        bits_per_sample: depth(u16::from_le_bytes([s0, s1])),
        extensible,
    })
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
pub(crate) mod tests {
    use super::*;
    use crate::parse::drive;
    use crate::values::Field;
    use gunmetal_testkit::bytes::Bytes;
    use gunmetal_testkit::riff::{
        FLOAT_SUBFORMAT, PCM_SUBFORMAT, ds64, extensible, format, rf64, wave,
    };
    use proptest::collection::vec;
    use proptest::prelude::*;

    /// The length of `file`.
    fn len(file: &[u8]) -> u64 {
        u64::try_from(file.len()).unwrap()
    }

    /// The budget the parser documents as always enough for `file`.
    fn enough(file: &[u8]) -> Budget {
        Budget::for_input(len(file), STEPS_PER_OCTET, STEPS_FIXED)
    }

    /// Runs `work` on a thread with the 256 KiB stack SEC-MED-001 names, so
    /// a parse that recursed would fail here.
    pub(crate) fn on_small_stack<T: Send + 'static>(
        work: impl FnOnce() -> T + Send + 'static,
    ) -> T {
        std::thread::Builder::new()
            .stack_size(262_144)
            .spawn(work)
            .expect("the test thread starts")
            .join()
            .expect("the parse returned instead of panicking")
    }

    /// Wraps a parser, recording its requests, clipping each window at
    /// `cut` as a file that shrinks while it is read would, and failing
    /// instead of hanging after `ceiling` requests.
    pub(crate) struct Recorder<P> {
        inner: P,
        cut: u64,
        ceiling: usize,
        requests: Vec<ReadRequest>,
    }

    impl<P: SansIo> SansIo for Recorder<P> {
        type Output = (P::Output, Vec<ReadRequest>);

        fn resume(&mut self, window: Window<'_>) -> Step<Self::Output> {
            let keep = usize::try_from(self.cut.saturating_sub(window.offset)).unwrap();
            let bytes = &window.bytes[..window.bytes.len().min(keep)];
            match self.inner.resume(Window { bytes, ..window }) {
                Step::Need(request) => {
                    self.requests.push(request);
                    assert!(
                        self.requests.len() <= self.ceiling,
                        "more than {} requests: {:?}",
                        self.ceiling,
                        self.requests
                    );
                    Step::Need(request)
                }
                Step::Done(output) => Step::Done((output, std::mem::take(&mut self.requests))),
            }
        }
    }

    /// Drives `parser` over `file` under `limits`, with every window
    /// clipped at `cut`, returning what it produced and the requests it
    /// made. A chunk takes at least eight octets, so a parser that makes
    /// more than a request per eight octets, plus the form header and the
    /// last chunk, has failed to move on (SEC-MED-008).
    pub(crate) fn record<P: SansIo>(
        parser: P,
        file: &[u8],
        cut: u64,
        limits: &Limits,
    ) -> (P::Output, Vec<ReadRequest>) {
        let recorder = Recorder {
            inner: parser,
            cut,
            ceiling: file.len() / 8 + 2,
            requests: Vec::new(),
        };
        drive(recorder, file, limits).expect("the parser reads only inside the file")
    }

    /// Parses `file` under `limits` and `budget`, with every window clipped
    /// at `cut`, returning the result and the requests made.
    fn run_under(
        file: &[u8],
        cut: u64,
        limits: &Limits,
        budget: Budget,
    ) -> (Result<WavFile, RiffError>, Vec<ReadRequest>) {
        record(Wav::new(limits, budget), file, cut, limits)
    }

    /// Parses all of `file` under the default limits and the documented
    /// budget.
    fn parse(file: &[u8]) -> Result<WavFile, RiffError> {
        run_under(file, u64::MAX, &Limits::DEFAULT, enough(file)).0
    }

    /// Parses `file` under `limits` and `budget`.
    fn parse_under(file: &[u8], limits: &Limits, budget: Budget) -> Result<WavFile, RiffError> {
        run_under(file, u64::MAX, limits, budget).0
    }

    /// Parses `file` as if it shrank to `cut` octets while being read.
    fn parse_shrinking(file: &[u8], cut: u64) -> Result<WavFile, RiffError> {
        run_under(file, cut, &Limits::DEFAULT, enough(file)).0
    }

    /// Chunks laid out by `write`.
    pub(crate) fn chunks(write: impl FnOnce(&mut Bytes)) -> Vec<u8> {
        let mut written = Bytes::new();
        write(&mut written);
        written.into_vec()
    }

    /// A RIFF form header declaring `size` octets after the size field.
    fn header(size: u32) -> Vec<u8> {
        chunks(|header| {
            header.bytes(b"RIFF").u32_le(size).bytes(b"WAVE");
        })
    }

    fn channels(count: u32) -> Channels {
        Channels::new(count).unwrap()
    }

    fn rate(hz: u32) -> SampleRate {
        SampleRate::new(hz).unwrap()
    }

    /// A depth of `count` bits, which the test knows is one.
    fn bits(count: u32) -> Option<BitDepth> {
        let depth = BitDepth::new(count).ok();
        assert!(depth.is_some(), "{count} bits is no depth");
        depth
    }

    fn range(offset: u64, len: u64) -> ByteRange {
        ByteRange { offset, len }
    }

    /// The format of the 8-bit mono PCM `fmt ` chunk at 8 kHz.
    fn mono_8_bit() -> WavFormat {
        WavFormat {
            codec: WavCodec::Pcm,
            channels: channels(1),
            sample_rate: rate(8_000),
            bytes_per_second: 8_000,
            block_align: NonZeroU16::new(1),
            bits_per_sample: bits(8),
            extensible: None,
        }
    }

    /// The format of the 16-bit stereo PCM `fmt ` chunk at 44.1 kHz.
    fn cd_quality() -> WavFormat {
        WavFormat {
            codec: WavCodec::Pcm,
            channels: channels(2),
            sample_rate: rate(44_100),
            bytes_per_second: 176_400,
            block_align: NonZeroU16::new(4),
            bits_per_sample: bits(16),
            extensible: None,
        }
    }

    /// A RIFF WAV file of `format` and `data`, found whole.
    fn found(format: WavFormat, data: ByteRange) -> WavFile {
        WavFile {
            form: WavForm::Riff,
            format,
            data,
            info: None,
            id3: None,
            stopped: None,
        }
    }

    /// The 8-bit mono file of three samples ffmpeg writes, 48 octets.
    fn minimal() -> Vec<u8> {
        wave(&chunks(|wave| {
            wave.riff_chunk(FMT, &format(1, 1, 8_000, 8))
                .riff_chunk(DATA, &[0x80, 0x81, 0x7F]);
        }))
    }

    fn truncated(offset: u64, needed: u64, available: u64) -> ParseFault {
        ParseFault::Truncated {
            offset,
            needed,
            available,
        }
    }

    const fn request(offset: u64, len: u32) -> ReadRequest {
        ReadRequest { offset, len }
    }

    #[test]
    fn reads_the_format_and_samples_of_a_minimal_file() {
        assert_eq!(parse(&minimal()), Ok(found(mono_8_bit(), range(44, 3))));
    }

    #[test]
    fn reads_only_the_form_header_and_the_first_octets_of_each_chunk() {
        // A thousand CD-quality frames, then a tag, so the walk has to step
        // over the samples to find it.
        let file = wave(&chunks(|wave| {
            wave.riff_chunk(FMT, &format(1, 2, 44_100, 16))
                .riff_chunk(DATA, &[0; 4_000])
                .riff_chunk(ID3, &[0x49, 0x44, 0x33]);
        }));
        let (result, requests) = run_under(&file, u64::MAX, &Limits::DEFAULT, enough(&file));
        assert_eq!(
            result,
            Ok(WavFile {
                id3: Some(range(4_052, 3)),
                ..found(cd_quality(), range(44, 4_000))
            })
        );
        assert_eq!(
            requests,
            [
                request(0, 12),
                request(12, 48),
                request(36, 48),
                request(4_044, 12),
            ]
        );
    }

    #[test]
    fn steps_over_the_pad_octet_after_an_odd_length_chunk() {
        let file = wave(&chunks(|wave| {
            wave.riff_chunk(*b"junk", &[1, 2, 3])
                .riff_chunk(FMT, &format(1, 1, 8_000, 8))
                .riff_chunk(DATA, &[0x80]);
        }));
        // The junk chunk's three octets and their pad end at 24.
        assert_eq!(parse(&file), Ok(found(mono_8_bit(), range(56, 1))));
    }

    /// Verifies: SEC-MED-008
    #[test]
    fn accepts_a_missing_pad_octet_at_the_end_of_the_file() {
        let mut file = minimal();
        // The last octet is the pad after the three samples.
        file.pop();
        assert_eq!(parse(&file), Ok(found(mono_8_bit(), range(44, 3))));
        // So does a form whose size leaves the pad out too.
        file[4] = 0x27;
        assert_eq!(parse(&file), Ok(found(mono_8_bit(), range(44, 3))));
    }

    #[test]
    fn a_data_size_of_all_ones_runs_to_the_end_of_the_form() {
        // What a writer streaming to a pipe leaves: no size it could know.
        let mut file = chunks(|wave| {
            wave.bytes(b"RIFF")
                .u32_le(u32::MAX)
                .bytes(b"WAVE")
                .riff_chunk(FMT, &format(1, 1, 8_000, 8))
                .bytes(b"data")
                .u32_le(u32::MAX);
        });
        file.extend([0x80, 0x81, 0x82, 0x83, 0x84]);
        assert_eq!(parse(&file), Ok(found(mono_8_bit(), range(44, 5))));
        // An even number of samples ends the form as exactly.
        file.pop();
        assert_eq!(parse(&file), Ok(found(mono_8_bit(), range(44, 4))));
    }

    /// Verifies: SEC-MED-004
    #[test]
    fn reads_the_sizes_of_an_rf64_file_from_its_ds64_chunk() {
        // ds64 sits at 12, fmt at 48, data at 72; the file is 86 octets.
        let file = rf64(&chunks(|wave| {
            wave.riff_chunk(DS64, &ds64(78, 6, 6))
                .riff_chunk(FMT, &format(1, 1, 8_000, 8))
                .bytes(b"data")
                .u32_le(u32::MAX)
                .bytes(&[1, 2, 3, 4, 5, 6]);
        }));
        assert_eq!(
            parse(&file),
            Ok(WavFile {
                form: WavForm::Rf64,
                ..found(mono_8_bit(), range(80, 6))
            })
        );
    }

    #[test]
    fn an_rf64_data_chunk_may_carry_its_own_32_bit_size() {
        let file = rf64(&chunks(|wave| {
            wave.riff_chunk(DS64, &ds64(76, 0, 0))
                .riff_chunk(FMT, &format(1, 1, 8_000, 8))
                .riff_chunk(DATA, &[1, 2, 3, 4]);
        }));
        assert_eq!(
            parse(&file),
            Ok(WavFile {
                form: WavForm::Rf64,
                ..found(mono_8_bit(), range(80, 4))
            })
        );
    }

    #[test]
    fn ends_an_rf64_form_where_its_ds64_size_says() {
        // The form ends after the samples; the tag after it is not read.
        let file = rf64(&chunks(|wave| {
            wave.riff_chunk(DS64, &ds64(76, 4, 4))
                .riff_chunk(FMT, &format(1, 1, 8_000, 8))
                .bytes(b"data")
                .u32_le(u32::MAX)
                .bytes(&[1, 2, 3, 4])
                .riff_chunk(ID3, b"ID3");
        }));
        assert_eq!(
            parse(&file),
            Ok(WavFile {
                form: WavForm::Rf64,
                ..found(mono_8_bit(), range(80, 4))
            })
        );
    }

    /// Verifies: SEC-MED-004, SEC-TM-032
    #[test]
    fn rf64_sizes_of_u64_max_end_at_the_end_of_the_file() {
        let file = rf64(&chunks(|wave| {
            wave.riff_chunk(DS64, &ds64(u64::MAX, u64::MAX, 0))
                .riff_chunk(FMT, &format(1, 1, 8_000, 8))
                .bytes(b"data")
                .u32_le(u32::MAX)
                .bytes(&[1, 2, 3]);
        }));
        assert_eq!(
            parse(&file),
            Ok(WavFile {
                form: WavForm::Rf64,
                stopped: Some(truncated(80, u64::MAX, 3)),
                ..found(mono_8_bit(), range(80, 3))
            })
        );
    }

    #[test]
    fn reads_a_format_chunk_that_comes_after_the_samples() {
        let file = wave(&chunks(|wave| {
            wave.riff_chunk(DATA, &[1, 2])
                .riff_chunk(FMT, &format(1, 2, 44_100, 16));
        }));
        assert_eq!(parse(&file), Ok(found(cd_quality(), range(20, 2))));
    }

    #[test]
    fn reads_wave_format_extensible() {
        // 24 valid bits in 32-bit containers, front left and right.
        let file = wave(&chunks(|wave| {
            wave.riff_chunk(FMT, &extensible(2, 48_000, 32, 24, 0x3, PCM_SUBFORMAT))
                .riff_chunk(DATA, &[0; 8]);
        }));
        assert_eq!(
            parse(&file),
            Ok(found(
                WavFormat {
                    codec: WavCodec::Pcm,
                    channels: channels(2),
                    sample_rate: rate(48_000),
                    bytes_per_second: 384_000,
                    block_align: NonZeroU16::new(8),
                    bits_per_sample: bits(32),
                    extensible: Some(Extensible {
                        valid_bits: bits(24),
                        channel_mask: 0x3,
                        sub_format: PCM_SUBFORMAT,
                    }),
                },
                range(68, 8)
            ))
        );
    }

    /// The codec and extension of an extensible `fmt ` chunk with
    /// `valid_bits` and `sub_format`.
    fn extended_codec(valid_bits: u16, sub_format: [u8; 16]) -> (WavCodec, Option<Extensible>) {
        let file = wave(&chunks(|wave| {
            wave.riff_chunk(FMT, &extensible(1, 8_000, 32, valid_bits, 0x4, sub_format))
                .riff_chunk(DATA, &[]);
        }));
        let format = parse(&file).unwrap().format;
        (format.codec, format.extensible)
    }

    #[test]
    fn takes_the_codec_of_an_extensible_format_from_its_sub_format() {
        let extension = |valid_bits, sub_format| {
            Some(Extensible {
                valid_bits,
                channel_mask: 0x4,
                sub_format,
            })
        };
        assert_eq!(
            extended_codec(32, FLOAT_SUBFORMAT),
            (WavCodec::Float, extension(bits(32), FLOAT_SUBFORMAT))
        );
        // MPEG Layer III's tag, 0x0055, in the same GUID range.
        let mut mp3 = PCM_SUBFORMAT;
        mp3[..2].copy_from_slice(&[0x55, 0x00]);
        assert_eq!(
            extended_codec(0, mp3),
            (WavCodec::Other(0x55), extension(None, mp3))
        );
        // Ambisonic B-format PCM, 00000001-0721-11D3-8644-C8C1CA000000,
        // stands for no tag; nor does a tag whose upper Data1 half is set.
        let ambisonic = [
            0x01, 0x00, 0x00, 0x00, 0x21, 0x07, 0xD3, 0x11, 0x86, 0x44, 0xC8, 0xC1, 0xCA, 0x00,
            0x00, 0x00,
        ];
        assert_eq!(
            extended_codec(65, ambisonic),
            (WavCodec::Unknown, extension(None, ambisonic))
        );
        let mut wide = PCM_SUBFORMAT;
        wide[2] = 0x01;
        assert_eq!(
            extended_codec(24, wide),
            (WavCodec::Unknown, extension(bits(24), wide))
        );
    }

    #[test]
    fn records_other_format_tags_without_describing_them() {
        // An MP3 stream in a WAV file: no bits per sample, one-octet blocks.
        let mp3 = chunks(|fmt| {
            fmt.u16_le(0x55)
                .u16_le(2)
                .u32_le(44_100)
                .u32_le(16_000)
                .u16_le(1)
                .u16_le(0);
        });
        let file = wave(&chunks(|wave| {
            wave.riff_chunk(FMT, &mp3).riff_chunk(DATA, &[0xFF, 0xFB]);
        }));
        assert_eq!(
            parse(&file),
            Ok(found(
                WavFormat {
                    codec: WavCodec::Other(0x55),
                    channels: channels(2),
                    sample_rate: rate(44_100),
                    bytes_per_second: 16_000,
                    block_align: NonZeroU16::new(1),
                    bits_per_sample: None,
                    extensible: None,
                },
                range(44, 2)
            ))
        );
    }

    #[test]
    fn reads_float_samples_and_a_block_alignment_of_zero() {
        let float = chunks(|fmt| {
            fmt.u16_le(3)
                .u16_le(1)
                .u32_le(96_000)
                .u32_le(384_000)
                .u16_le(0)
                .u16_le(65);
        });
        let file = wave(&chunks(|wave| {
            wave.riff_chunk(FMT, &float).riff_chunk(DATA, &[]);
        }));
        assert_eq!(
            parse(&file),
            Ok(found(
                WavFormat {
                    codec: WavCodec::Float,
                    channels: channels(1),
                    sample_rate: rate(96_000),
                    bytes_per_second: 384_000,
                    block_align: None,
                    bits_per_sample: None,
                    extensible: None,
                },
                range(44, 0)
            ))
        );
    }

    #[test]
    fn refuses_a_file_that_is_not_a_riff_wave_form() {
        let mut file = minimal();
        file[..4].copy_from_slice(b"RIFX");
        assert_eq!(parse(&file), Err(RiffError::NotRiff { found: *b"RIFX" }));
        let mut file = minimal();
        file[8..12].copy_from_slice(b"AVI ");
        assert_eq!(parse(&file), Err(RiffError::NotWave { found: *b"AVI " }));
    }

    #[test]
    fn refuses_an_rf64_form_whose_first_chunk_is_not_ds64() {
        let file = rf64(&chunks(|wave| {
            wave.riff_chunk(FMT, &format(1, 1, 8_000, 8))
                .riff_chunk(DS64, &ds64(56, 0, 0));
        }));
        assert_eq!(parse(&file), Err(RiffError::NoDs64 { found: FMT }));
    }

    /// A WAV file whose first chunk is `fmt` with `body`, then empty data.
    fn with_format(body: &[u8]) -> Vec<u8> {
        wave(&chunks(|wave| {
            wave.riff_chunk(FMT, body).riff_chunk(DATA, &[]);
        }))
    }

    #[test]
    fn refuses_a_chunk_shorter_than_its_fields() {
        let short = |id, len, needed| {
            Err(RiffError::Short {
                id,
                offset: 12,
                len,
                needed,
            })
        };
        let pcm = format(1, 1, 8_000, 8);
        assert_eq!(parse(&with_format(&pcm[..15])), short(FMT, 15, 16));
        let full = extensible(1, 8_000, 16, 16, 0x4, PCM_SUBFORMAT);
        assert_eq!(parse(&with_format(&full[..39])), short(FMT, 39, 40));
        // WAVEFORMATEX with its two-octet extension size, but tag 0xFFFE.
        assert_eq!(parse(&with_format(&full[..18])), short(FMT, 18, 40));
        let file = rf64(&chunks(|wave| {
            wave.riff_chunk(DS64, &ds64(0, 0, 0)[..27]);
        }));
        assert_eq!(parse(&file), short(DS64, 27, 28));
    }

    #[test]
    fn reads_fields_followed_by_more_octets_than_they_need() {
        // A WAVEFORMATEX of PCM, whose extension size is zero, and an
        // extensible format with two octets of padding after its fields.
        let mut pcm = format(1, 1, 8_000, 8);
        pcm.extend([0, 0]);
        assert_eq!(
            parse(&with_format(&pcm)),
            Ok(found(mono_8_bit(), range(46, 0)))
        );
        let mut full = extensible(1, 8_000, 8, 8, 0x4, PCM_SUBFORMAT);
        full.extend([0, 0]);
        let format = parse(&with_format(&full)).map(|file| (file.format.codec, file.data));
        assert_eq!(format, Ok((WavCodec::Pcm, range(70, 0))));
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn refuses_a_channel_count_or_sample_rate_out_of_range() {
        let value = |field, value| {
            Err(RiffError::Value {
                offset: 12,
                error: ValueError::OutOfRange { field, value },
            })
        };
        assert_eq!(
            parse(&with_format(&format(1, 0, 8_000, 8))),
            value(Field::Channels, 0)
        );
        assert_eq!(
            parse(&with_format(&format(1, 256, 8_000, 8))),
            value(Field::Channels, 256)
        );
        assert_eq!(
            parse(&with_format(&format(1, 1, 0, 8))),
            value(Field::SampleRate, 0)
        );
        assert_eq!(
            parse(&with_format(&format(1, 1, 768_001, 8))),
            value(Field::SampleRate, 768_001)
        );
        // At the top of each range.
        let file = with_format(&format(1, 255, 768_000, 8));
        let format = parse(&file).map(|file| (file.format.channels, file.format.sample_rate));
        assert_eq!(format, Ok((channels(255), rate(768_000))));
    }

    #[test]
    fn a_playable_file_needs_its_format_and_its_samples() {
        let only_data = wave(&chunks(|wave| {
            wave.riff_chunk(DATA, &[1]);
        }));
        assert_eq!(parse(&only_data), Err(RiffError::Missing { id: FMT }));
        let only_format = wave(&chunks(|wave| {
            wave.riff_chunk(FMT, &format(1, 1, 8_000, 8));
        }));
        assert_eq!(parse(&only_format), Err(RiffError::Missing { id: DATA }));
        assert_eq!(parse(&wave(&[])), Err(RiffError::Missing { id: FMT }));
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn refuses_a_file_shorter_than_a_form_header() {
        assert_eq!(parse(&[]), Err(RiffError::Fault(truncated(0, 12, 0))));
        assert_eq!(
            parse(&minimal()[..11]),
            Err(RiffError::Fault(truncated(0, 12, 11)))
        );
        // A form header alone holds no chunks.
        assert_eq!(parse(&minimal()[..12]), Err(RiffError::Missing { id: FMT }));
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn keeps_samples_cut_short_and_says_where_the_walk_stopped() {
        let mut file = wave(&chunks(|wave| {
            wave.riff_chunk(FMT, &format(1, 1, 8_000, 8))
                .riff_chunk(DATA, &[0; 100]);
        }));
        file.truncate(50);
        assert_eq!(
            parse(&file),
            Ok(WavFile {
                stopped: Some(truncated(44, 100, 6)),
                ..found(mono_8_bit(), range(44, 6))
            })
        );
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn stops_at_a_chunk_header_cut_short() {
        let mut file = minimal();
        file.extend(b"LIST\x04");
        // The form now claims the five octets as well.
        file[4] += 5;
        assert_eq!(
            parse(&file),
            Ok(WavFile {
                stopped: Some(truncated(48, 8, 5)),
                ..found(mono_8_bit(), range(44, 3))
            })
        );
        // Before the samples were found, the file is not playable.
        let file = wave(&chunks(|wave| {
            wave.riff_chunk(FMT, &format(1, 1, 8_000, 8)).bytes(b"dat");
        }));
        assert_eq!(parse(&file), Err(RiffError::Fault(truncated(36, 8, 3))));
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn stops_at_a_chunk_whose_body_runs_past_the_end_of_the_form() {
        // Whatever the chunk is, nothing of it is recorded.
        for id in [LIST, ID3, *b"junk"] {
            let mut file = minimal();
            file.extend(chunks(|tail| {
                tail.bytes(&id).u32_le(16).bytes(b"INFO");
            }));
            file[4] += 12;
            assert_eq!(
                parse(&file),
                Ok(WavFile {
                    stopped: Some(truncated(56, 16, 4)),
                    ..found(mono_8_bit(), range(44, 3))
                }),
                "{id:?}"
            );
        }
        // A format chunk cut short leaves no format.
        let mut file = wave(&chunks(|wave| {
            wave.riff_chunk(FMT, &format(1, 1, 8_000, 8));
        }));
        file.truncate(30);
        assert_eq!(parse(&file), Err(RiffError::Fault(truncated(20, 16, 10))));
        // Nor does an RF64 file whose ds64 chunk is cut short.
        let mut file = rf64(&chunks(|wave| {
            wave.riff_chunk(DS64, &ds64(0, 0, 0));
        }));
        file.truncate(30);
        assert_eq!(parse(&file), Err(RiffError::Fault(truncated(20, 28, 10))));
    }

    #[test]
    fn records_the_first_info_list_and_id3_tag() {
        let file = wave(&chunks(|wave| {
            wave.riff_chunk(LIST, b"adtlnote")
                .riff_chunk(LIST, b"INFOINAM\x02\x00\x00\x00A\x00")
                .riff_chunk(LIST, b"INFOsecond")
                .riff_chunk(ID3, b"ID3\x04")
                .riff_chunk(ID3_UPPER, b"ID3\x03")
                .riff_chunk(FMT, &format(1, 1, 8_000, 8))
                .riff_chunk(DATA, &[0x80]);
        }));
        assert_eq!(
            parse(&file),
            Ok(WavFile {
                info: Some(range(40, 10)),
                id3: Some(range(76, 4)),
                ..found(mono_8_bit(), range(124, 1))
            })
        );
        // Either spelling of the tag's chunk.
        let file = wave(&chunks(|wave| {
            wave.riff_chunk(ID3_UPPER, b"ID3")
                .riff_chunk(FMT, &format(1, 1, 8_000, 8))
                .riff_chunk(DATA, &[]);
        }));
        assert_eq!(parse(&file).map(|file| file.id3), Ok(Some(range(20, 3))));
    }

    #[test]
    fn reads_no_list_type_from_a_list_shorter_than_one() {
        // An empty LIST, then a chunk whose ID happens to be INFO: reading
        // four octets of type would take the next chunk's ID.
        let file = wave(&chunks(|wave| {
            wave.riff_chunk(LIST, b"")
                .riff_chunk(INFO, b"")
                .riff_chunk(FMT, &format(1, 1, 8_000, 8))
                .riff_chunk(DATA, &[]);
        }));
        assert_eq!(parse(&file), Ok(found(mono_8_bit(), range(60, 0))));
        // A type alone is an INFO list with no sub-chunks.
        let file = wave(&chunks(|wave| {
            wave.riff_chunk(LIST, b"INFO")
                .riff_chunk(FMT, &format(1, 1, 8_000, 8))
                .riff_chunk(DATA, &[]);
        }));
        assert_eq!(parse(&file).map(|file| file.info), Ok(Some(range(24, 0))));
    }

    #[test]
    fn keeps_the_first_format_and_the_first_samples() {
        let file = wave(&chunks(|wave| {
            wave.riff_chunk(FMT, &format(1, 1, 8_000, 8))
                .riff_chunk(DATA, &[1])
                .riff_chunk(FMT, &format(1, 2, 44_100, 16))
                .riff_chunk(DATA, &[2, 3]);
        }));
        assert_eq!(parse(&file), Ok(found(mono_8_bit(), range(44, 1))));
    }

    #[test]
    fn walks_only_as_far_as_the_form_declares() {
        // A form that ends after the samples: the tag after it is not read.
        let mut file = minimal();
        file.extend(chunks(|tail| {
            tail.riff_chunk(ID3, b"ID3");
        }));
        assert_eq!(parse(&file), Ok(found(mono_8_bit(), range(44, 3))));
        // A form that claims more than the file holds ends with the file.
        let mut file = header(1_000);
        file.extend(&minimal()[12..]);
        assert_eq!(parse(&file), Ok(found(mono_8_bit(), range(44, 3))));
    }

    /// `count` empty chunks, then the minimal file's format and samples.
    fn after_empty_chunks(count: usize) -> Vec<u8> {
        wave(&chunks(|wave| {
            for _ in 0..count {
                wave.riff_chunk(*b"junk", &[]);
            }
            wave.riff_chunk(FMT, &format(1, 1, 8_000, 8))
                .riff_chunk(DATA, &[0x80]);
        }))
    }

    fn lowered(kind: LimitKind, value: u64) -> Limits {
        Limits::DEFAULT.with_override(kind, value).unwrap()
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn reads_as_many_chunks_as_the_children_limit_allows() {
        let file = after_empty_chunks(3);
        let at_five = lowered(LimitKind::Children, 5);
        assert_eq!(
            parse_under(&file, &at_five, enough(&file)),
            Ok(found(mono_8_bit(), range(68, 1)))
        );
        let at_four = lowered(LimitKind::Children, 4);
        assert_eq!(
            parse_under(&file, &at_four, enough(&file)),
            Err(RiffError::Fault(ParseFault::LimitExceeded {
                limit: LimitKind::Children,
                value: 5,
                max: 4,
                offset: 60,
            }))
        );
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn keeps_what_it_found_before_the_children_limit() {
        let mut file = minimal();
        file.extend(chunks(|tail| {
            tail.riff_chunk(*b"junk", &[]);
        }));
        file[4] += 8;
        assert_eq!(
            parse_under(&file, &lowered(LimitKind::Children, 2), enough(&file)),
            Ok(WavFile {
                stopped: Some(ParseFault::LimitExceeded {
                    limit: LimitKind::Children,
                    value: 3,
                    max: 2,
                    offset: 48,
                }),
                ..found(mono_8_bit(), range(44, 3))
            })
        );
    }

    /// Verifies: SEC-MED-007, SEC-TM-032
    #[test]
    fn charges_one_step_for_each_chunk() {
        // Three empty chunks, the format and the samples: five steps.
        let file = after_empty_chunks(3);
        assert_eq!(
            parse_under(&file, &Limits::DEFAULT, Budget::for_input(0, 0, 5)),
            Ok(found(mono_8_bit(), range(68, 1)))
        );
        assert_eq!(
            parse_under(&file, &Limits::DEFAULT, Budget::for_input(0, 0, 4)),
            Err(RiffError::Fault(ParseFault::BudgetExceeded { offset: 60 }))
        );
        // Spent after the format and samples, the walk keeps them.
        let mut file = minimal();
        file.extend(chunks(|tail| {
            tail.riff_chunk(*b"junk", &[]);
        }));
        file[4] += 8;
        assert_eq!(
            parse_under(&file, &Limits::DEFAULT, Budget::for_input(0, 0, 2)),
            Ok(WavFile {
                stopped: Some(ParseFault::BudgetExceeded { offset: 48 }),
                ..found(mono_8_bit(), range(44, 3))
            })
        );
    }

    /// Verifies: SEC-MED-005
    #[test]
    fn counts_the_chunks_of_a_form_one_level_below_its_root() {
        assert_eq!(
            parse_under(
                &minimal(),
                &lowered(LimitKind::ContainerDepth, 0),
                enough(&minimal())
            ),
            Err(RiffError::Fault(ParseFault::TooDeep {
                limit: LimitKind::ContainerDepth,
                depth: 1,
                max: 0,
                offset: 12,
            }))
        );
        assert_eq!(
            parse_under(
                &minimal(),
                &lowered(LimitKind::ContainerDepth, 1),
                enough(&minimal())
            ),
            Ok(found(mono_8_bit(), range(44, 3)))
        );
    }

    /// Verifies: SEC-MED-001
    #[test]
    fn reports_reads_cut_short_by_a_file_that_shrinks() {
        let file = minimal();
        assert_eq!(
            parse_shrinking(&file, 5),
            Err(RiffError::Fault(truncated(0, 12, 5)))
        );
        assert_eq!(
            parse_shrinking(&file, 15),
            Err(RiffError::Fault(truncated(12, 8, 3)))
        );
        assert_eq!(
            parse_shrinking(&file, 30),
            Err(RiffError::Fault(truncated(20, 16, 10)))
        );
        let file = wave(&chunks(|wave| {
            wave.riff_chunk(FMT, &extensible(1, 8_000, 8, 8, 0x4, PCM_SUBFORMAT))
                .riff_chunk(DATA, &[]);
        }));
        assert_eq!(
            parse_shrinking(&file, 50),
            Err(RiffError::Fault(truncated(36, 24, 14)))
        );
        let file = rf64(&chunks(|wave| {
            wave.riff_chunk(DS64, &ds64(0, 0, 0));
        }));
        assert_eq!(
            parse_shrinking(&file, 30),
            Err(RiffError::Fault(truncated(20, 16, 10)))
        );
        let file = wave(&chunks(|wave| {
            wave.riff_chunk(LIST, b"INFO")
                .riff_chunk(FMT, &format(1, 1, 8_000, 8))
                .riff_chunk(DATA, &[]);
        }));
        assert_eq!(
            parse_shrinking(&file, 22),
            Err(RiffError::Fault(truncated(20, 4, 2)))
        );
    }

    #[test]
    fn answers_its_result_again_once_the_parse_is_over() {
        let file = minimal();
        let mut parser = Wav::new(&Limits::DEFAULT, enough(&file));
        let mut window = Window::start(48);
        let mut reads = 0;
        while let Step::Need(ReadRequest { offset, len }) = parser.resume(window) {
            let start = usize::try_from(offset).unwrap();
            let end = start + usize::try_from(len).unwrap();
            window = Window {
                offset,
                bytes: &file[start..end],
                file_len: 48,
            };
            reads += 1;
            assert!(reads <= 3, "more reads than the file has chunks");
        }
        let done = Step::Done(Ok(found(mono_8_bit(), range(44, 3))));
        assert_eq!(parser.resume(window), done);
        assert_eq!(parser.resume(Window::start(48)), done);
    }

    #[test]
    fn asks_for_at_most_a_peek_and_never_past_the_form() {
        // Fifty octets from the end of a form near the top of u64.
        let walk = Walk {
            at: u64::MAX - 50,
            end: u64::MAX,
            chunks: 0,
        };
        assert_eq!(walk.request(48), Ok(Some(request(u64::MAX - 50, 48))));
        assert_eq!(Walk { at: 0, ..walk }.request(48), Ok(Some(request(0, 48))));
        assert_eq!(
            Walk {
                at: u64::MAX - 8,
                ..walk
            }
            .request(48),
            Ok(Some(request(u64::MAX - 8, 8)))
        );
        assert_eq!(
            Walk {
                at: u64::MAX - 7,
                ..walk
            }
            .request(48),
            Err(truncated(u64::MAX - 7, 8, 7))
        );
        assert_eq!(
            Walk {
                at: u64::MAX,
                ..walk
            }
            .request(48),
            Ok(None)
        );
    }

    #[test]
    fn moves_past_a_body_that_fills_the_form_exactly() {
        let walk = Walk {
            at: 12,
            end: 31,
            chunks: 1,
        };
        assert_eq!(walk.past(11), Ok(Walk { at: 32, ..walk }));
        assert_eq!(walk.past(10), Ok(Walk { at: 30, ..walk }));
        assert_eq!(walk.past(12), Err(truncated(20, 12, 11)));
        // Near the top of u64, the body and its pad saturate.
        let top = Walk {
            at: u64::MAX - 9,
            end: u64::MAX,
            chunks: 1,
        };
        assert_eq!(
            top.past(1),
            Ok(Walk {
                at: u64::MAX,
                ..top
            })
        );
        assert_eq!(top.room(), 1);
    }

    #[test]
    fn describes_every_error_as_an_unreadable_wav_file() {
        let fault = RiffError::Fault;
        let cases = [
            (fault(truncated(3, 8, 1)), "truncated", Some(3)),
            (
                fault(ParseFault::BudgetExceeded { offset: 5 }),
                "budget_exceeded",
                Some(5),
            ),
            (
                fault(ParseFault::TooDeep {
                    limit: LimitKind::ContainerDepth,
                    depth: 1,
                    max: 0,
                    offset: 12,
                }),
                "too_deep",
                Some(12),
            ),
            (
                fault(ParseFault::LimitExceeded {
                    limit: LimitKind::Children,
                    value: 3,
                    max: 2,
                    offset: 7,
                }),
                "limit_exceeded",
                Some(7),
            ),
            (
                fault(ParseFault::NotSyncsafe {
                    offset: 9,
                    octets: [0x80; 4],
                }),
                "not_syncsafe",
                Some(9),
            ),
            (RiffError::NotRiff { found: *b"RIFX" }, "not_riff", Some(0)),
            (RiffError::NotWave { found: *b"AVI " }, "not_wave", Some(8)),
            (RiffError::NoDs64 { found: FMT }, "no_ds64", Some(12)),
            (
                RiffError::Short {
                    id: FMT,
                    offset: 30,
                    len: 2,
                    needed: 16,
                },
                "short_chunk",
                Some(30),
            ),
            (
                RiffError::Value {
                    offset: 40,
                    error: ValueError::OutOfRange {
                        field: Field::Channels,
                        value: 0,
                    },
                },
                "bad_value",
                Some(40),
            ),
            (RiffError::Missing { id: DATA }, "missing_chunk", None),
        ];
        for (error, reason, offset) in cases {
            let mut args = vec![("reason", Arg::Name(reason))];
            args.extend(offset.map(|offset| ("offset", Arg::Number(offset))));
            assert_eq!(
                error.problem(),
                Problem {
                    code: ProblemCode::WavUnreadable,
                    args,
                },
                "{error:?}"
            );
        }
    }

    /// One chunk of a generated WAV file.
    #[derive(Debug, Clone)]
    enum Piece {
        /// A PCM `fmt ` chunk.
        Format { channels: u16, rate: u32, bits: u16 },
        /// A `data` chunk.
        Data(Vec<u8>),
        /// A `LIST` chunk, of type `INFO` or `adtl`.
        List { info: bool, body: Vec<u8> },
        /// An `id3 ` or `ID3 ` chunk.
        Id3 { upper: bool, body: Vec<u8> },
        /// A chunk this parser does not read.
        Junk(Vec<u8>),
    }

    impl Piece {
        /// The chunk's ID and body.
        fn encode(&self) -> ([u8; 4], Vec<u8>) {
            match self {
                Self::Format {
                    channels,
                    rate,
                    bits,
                } => (FMT, format(1, *channels, *rate, *bits)),
                Self::Data(body) => (DATA, body.clone()),
                Self::List { info, body } => {
                    let kind: &[u8] = if *info { b"INFO" } else { b"adtl" };
                    (LIST, [kind, body].concat())
                }
                Self::Id3 { upper, body } => (if *upper { ID3_UPPER } else { ID3 }, body.clone()),
                Self::Junk(body) => (*b"junk", body.clone()),
            }
        }
    }

    fn format_piece() -> impl Strategy<Value = Piece> {
        (
            1_u16..=8,
            1_u32..=192_000,
            prop::sample::select(vec![8_u16, 16, 24, 32]),
        )
            .prop_map(|(channels, rate, bits)| Piece::Format {
                channels,
                rate,
                bits,
            })
    }

    fn data_piece() -> impl Strategy<Value = Piece> {
        vec(any::<u8>(), 0..24).prop_map(Piece::Data)
    }

    fn piece() -> impl Strategy<Value = Piece> {
        prop_oneof![
            format_piece(),
            data_piece(),
            (any::<bool>(), vec(any::<u8>(), 0..12))
                .prop_map(|(info, body)| Piece::List { info, body }),
            (any::<bool>(), vec(any::<u8>(), 0..12))
                .prop_map(|(upper, body)| Piece::Id3 { upper, body }),
            vec(any::<u8>(), 0..12).prop_map(Piece::Junk),
        ]
    }

    /// The WAV file of `pieces`.
    fn encode(pieces: &[Piece]) -> Vec<u8> {
        wave(&chunks(|wave| {
            for piece in pieces {
                let (id, body) = piece.encode();
                wave.riff_chunk(id, &body);
            }
        }))
    }

    /// An independent model of the parse of a whole file of `pieces`: it
    /// lays the chunks out and keeps the first of each kind.
    fn model(pieces: &[Piece]) -> Result<WavFile, RiffError> {
        let mut at = 12_u64;
        let (mut format, mut data, mut info, mut id3) = (None, None, None, None);
        for piece in pieces {
            let body = at + 8;
            let len = len(&piece.encode().1);
            match piece {
                Piece::Format {
                    channels: count,
                    rate: hz,
                    bits: width,
                } if format.is_none() => {
                    let frame = u32::from(*count) * u32::from(*width) / 8;
                    format = Some(WavFormat {
                        codec: WavCodec::Pcm,
                        channels: channels(u32::from(*count)),
                        sample_rate: rate(*hz),
                        bytes_per_second: hz * frame,
                        block_align: NonZeroU16::new(u16::try_from(frame).unwrap()),
                        bits_per_sample: bits(u32::from(*width)),
                        extensible: None,
                    });
                }
                Piece::Data(_) if data.is_none() => data = Some(range(body, len)),
                Piece::List { info: true, .. } if info.is_none() => {
                    info = Some(range(body + 4, len - 4));
                }
                Piece::Id3 { .. } if id3.is_none() => id3 = Some(range(body, len)),
                _ => {}
            }
            at = body + len + len % 2;
        }
        match (format, data) {
            (None, _) => Err(RiffError::Missing { id: FMT }),
            (_, None) => Err(RiffError::Missing { id: DATA }),
            (Some(format), Some(data)) => Ok(WavFile {
                info,
                id3,
                ..found(format, data)
            }),
        }
    }

    #[test]
    fn the_model_agrees_with_the_parser_on_each_kind_of_chunk() {
        let pieces = [
            Piece::Junk(vec![1]),
            Piece::List {
                info: false,
                body: vec![2, 3],
            },
            Piece::Id3 {
                upper: true,
                body: vec![4],
            },
            Piece::Format {
                channels: 2,
                rate: 44_100,
                bits: 16,
            },
            Piece::List {
                info: true,
                body: vec![5],
            },
            Piece::Data(vec![6, 7, 8]),
            Piece::Id3 {
                upper: false,
                body: vec![],
            },
        ];
        let expected = Ok(WavFile {
            info: Some(range(82, 1)),
            id3: Some(range(44, 1)),
            ..found(cd_quality(), range(92, 3))
        });
        assert_eq!(model(&pieces), expected);
        assert_eq!(parse(&encode(&pieces)), expected);
        assert_eq!(model(&pieces[..5]), Err(RiffError::Missing { id: DATA }));
        assert_eq!(model(&pieces[5..]), Err(RiffError::Missing { id: FMT }));
    }

    /// Any octets at all: noise, noise after a form header, or a generated
    /// file that holds a format and samples, whole or with one octet
    /// changed and its end cut off.
    fn any_file() -> impl Strategy<Value = Vec<u8>> {
        prop_oneof![
            vec(any::<u8>(), 0..96),
            (
                prop::sample::select(vec![RIFF, RF64]),
                any::<u32>(),
                vec(any::<u8>(), 0..96)
            )
                .prop_map(|(id, size, noise)| {
                    let mut file = chunks(|form| {
                        form.bytes(&id).u32_le(size).bytes(b"WAVE");
                    });
                    file.extend(noise);
                    file
                }),
            (
                vec(piece(), 0..3),
                format_piece(),
                data_piece(),
                vec(piece(), 0..3),
                prop::option::of((any::<usize>(), any::<u8>(), any::<usize>()))
            )
                .prop_map(|(before, format, data, after, damage)| {
                    let mut file = encode(&[before, vec![format, data], after].concat());
                    if let Some((at, octet, cut)) = damage {
                        let at = at % file.len();
                        file[at] = octet;
                        file.truncate(cut % (file.len() + 1));
                    }
                    file
                }),
        ]
    }

    proptest! {
        /// Verifies: SEC-MED-001, SEC-MED-007, SEC-MED-008, SEC-TM-032, SEC-HIS-036
        #[test]
        fn returns_and_reads_only_inside_the_file_for_any_input(
            file in any_file(),
            cut in prop_oneof![Just(u64::MAX), 0_u64..128],
        ) {
            // run_under fails on a read outside the file and on more reads
            // than the file has chunks; the budget is the documented one.
            let length = len(&file);
            let (result, _) = on_small_stack(move || {
                run_under(&file, cut, &Limits::DEFAULT, enough(&file))
            });
            let inside = |range: ByteRange| range.offset + range.len <= length;
            match result {
                Ok(found) => {
                    prop_assert!(inside(found.data), "{:?}", found);
                    prop_assert!(found.info.is_none_or(inside), "{:?}", found);
                    prop_assert!(found.id3.is_none_or(inside), "{:?}", found);
                    prop_assert!(
                        !matches!(found.stopped, Some(ParseFault::BudgetExceeded { .. })),
                        "{:?}",
                        found
                    );
                }
                Err(error) => prop_assert!(
                    !matches!(error, RiffError::Fault(ParseFault::BudgetExceeded { .. })),
                    "{:?}",
                    error
                ),
            }
        }

        #[test]
        fn finds_the_first_of_each_chunk_wherever_it_lies(pieces in vec(piece(), 0..8)) {
            prop_assert_eq!(parse(&encode(&pieces)), model(&pieces));
        }

        /// Verifies: SEC-MED-007, SEC-MED-008
        #[test]
        fn walks_any_number_of_empty_chunks_within_the_documented_budget(count in 0_usize..300) {
            let file = after_empty_chunks(count);
            let data = 12 + 8 * u64::try_from(count).unwrap() + 24 + 8;
            prop_assert_eq!(parse(&file), Ok(found(mono_8_bit(), range(data, 1))));
        }
    }
}
