//! FLAC metadata (RFC 9639, section 8): the `fLaC` marker, the chain of
//! metadata blocks after it, and where the audio frames start.
//!
//! [`parse_metadata`] returns a sans-I/O parser. It reads the marker, every
//! block header, the STREAMINFO block, the seek table, and each picture's
//! fields up to its data, and nothing more: the picture data, the Vorbis
//! comment and every other block come back as byte ranges of the file, for
//! the parsers and jobs that read them.
//!
//! # What fails the file
//!
//! Playback needs STREAMINFO and the block chain, so a fault in either
//! fails the parse with a [`FlacError`]: no marker, a first block that is
//! not STREAMINFO or a second one, a STREAMINFO of the wrong length or with
//! a value outside its typed range, a block of the forbidden type 127, a
//! block that runs past the end of the file, more blocks than
//! [`LimitKind::Children`] allows, or a spent budget. A last-block flag
//! that is never set fails too: at the end of the file, or at the first
//! frame, whose sync code reads as a header of type 127.
//!
//! Everything else is optional (SEC-MED-017). A seek table or picture that
//! is malformed or over a limit, a picture given as a link, and a second
//! seek table or Vorbis comment are skipped and recorded as a
//! [`BlockProblem`], and the parse carries on with the next block.
//!
//! # Reads, steps and depth
//!
//! The parser never asks for a read past the end of the file, longer than
//! [`LimitKind::ReadBytes`], or past [`LimitKind::FileBytes`] in all
//! (SEC-MED-010); a read that would is a truncation or a limit fault
//! instead. It charges its budget one step per block header and one per
//! seek point. A header takes four octets and a seek point eighteen, so a
//! parse spends at most one step per four octets of the file, and a budget
//! of [`STEPS_PER_OCTET`] steps per octet of the file plus [`STEPS_FIXED`]
//! is never spent (SEC-MED-007). Nothing in FLAC metadata nests, so there
//! is no depth to count: the parser moves along the chain one header at a
//! time and never recurses (SEC-MED-005).

use std::mem;
use std::num::{NonZeroU32, NonZeroU64};
use std::ops::Range;

use crate::parse::{
    Budget, LimitKind, Limits, ParseFault, ReadRequest, SansIo, Step, Window, bounded_vec,
};
use crate::text::{self, Encoding, Lines, Text};
use crate::untrusted::Untrusted;
use crate::values::{BitDepth, Channels, SampleRate, ValueError};

/// The steps per octet of the file that size a budget no parse can spend.
pub const STEPS_PER_OCTET: u64 = 1;

/// The steps beyond [`STEPS_PER_OCTET`] that size a budget no parse can
/// spend.
pub const STEPS_FIXED: u64 = 0;

/// The four octets a FLAC stream starts with.
const MARKER: [u8; 4] = *b"fLaC";

/// Octets in a block header.
const HEADER: u64 = 4;

/// Octets in a STREAMINFO block's body.
const STREAM_INFO: u32 = 34;

/// Octets in one seek point.
const SEEK_POINT: usize = 18;

/// The sample number of a placeholder seek point.
const PLACEHOLDER: u64 = u64::MAX;

/// The media type that says a picture's data is a link, not a picture.
const LINK: &str = "-->";

/// Octets of a picture's type and media type length.
const PICTURE_HEAD: u64 = 8;

/// Octets of a picture's width, height, depth, colours and data length.
const PICTURE_FIELDS: u64 = 20;

/// The read a parser expects first: the start window, which holds no
/// octets at offset 0.
const START: ReadRequest = ReadRequest { offset: 0, len: 0 };

/// What a metadata block is, from the type in its header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    /// STREAMINFO, type 0.
    StreamInfo,
    /// PADDING, type 1.
    Padding,
    /// APPLICATION, type 2.
    Application,
    /// SEEKTABLE, type 3.
    SeekTable,
    /// `VORBIS_COMMENT`, type 4.
    VorbisComment,
    /// CUESHEET, type 5.
    CueSheet,
    /// PICTURE, type 6.
    Picture,
    /// A reserved type, 7 to 126.
    Reserved(u8),
}

impl BlockType {
    /// The type a header's seven-bit code names, or `None` for the
    /// forbidden 127.
    const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::StreamInfo),
            1 => Some(Self::Padding),
            2 => Some(Self::Application),
            3 => Some(Self::SeekTable),
            4 => Some(Self::VorbisComment),
            5 => Some(Self::CueSheet),
            6 => Some(Self::Picture),
            127 => None,
            reserved => Some(Self::Reserved(reserved)),
        }
    }
}

/// The MD5 signature of the decoded audio, as the encoder wrote it.
///
/// Opaque octets: Gunmetal never computes or checks an MD5, and this is
/// never a security value (cryptography record, ADR 9). Identity may use
/// it only when the encoder set it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AudioMd5(pub [u8; 16]);

/// The STREAMINFO block (RFC 9639, section 8.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamInfo {
    /// The smallest block size in samples, except the last block's.
    pub min_block_size: u16,
    /// The largest block size in samples.
    pub max_block_size: u16,
    /// The smallest frame size in octets, when the encoder knew it.
    pub min_frame_size: Option<NonZeroU32>,
    /// The largest frame size in octets, when the encoder knew it.
    pub max_frame_size: Option<NonZeroU32>,
    /// The sample rate.
    pub sample_rate: SampleRate,
    /// The number of channels, 1 to 8.
    pub channels: Channels,
    /// Bits per sample, 1 to 32.
    pub bits_per_sample: BitDepth,
    /// Interchannel samples in the stream, when the encoder knew it.
    pub total_samples: Option<NonZeroU64>,
    /// The MD5 signature of the decoded audio, when the encoder set it.
    pub md5: Option<AudioMd5>,
}

/// One seek point (RFC 9639, section 8.5.1). Placeholder points are not
/// kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeekPoint {
    /// The first sample of the target frame.
    pub sample: u64,
    /// Octets from the first frame header, at
    /// [`FlacMetadata::audio_start`], to the target frame's header. Not
    /// checked against the file.
    pub offset: u64,
    /// Samples in the target frame.
    pub samples: u16,
}

/// A picture block (RFC 9639, section 8.8), with its data as a byte range
/// of the file. Its dimensions are what the block claims, not what the
/// picture holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureRef {
    /// The picture type (RFC 9639, Table 13), as written.
    pub picture_type: u32,
    /// The media type, on one line, capped at [`LimitKind::ShortText`].
    pub media_type: Text,
    /// The description, capped at [`LimitKind::LongText`].
    pub description: Text,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Colour depth in bits per pixel.
    pub depth: u32,
    /// Colours used by an indexed picture, or 0.
    pub colors: u32,
    /// Where the picture data lies in the file.
    pub data: Range<u64>,
}

/// A block kept as it is, as a byte range of the file: padding,
/// application data, a cue sheet or a block of a reserved type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawBlock {
    /// What the block is.
    pub block_type: BlockType,
    /// Where its body lies in the file.
    pub body: Range<u64>,
}

/// An optional block the parser skipped, and why (SEC-MED-017). `block` is
/// the offset of the skipped block's header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockProblem {
    /// A limit, or a length inside the block that runs past its end.
    Fault {
        /// Where the block starts.
        block: u64,
        /// What went wrong.
        fault: ParseFault,
    },
    /// A seek table whose length is not a whole number of seek points.
    SeekTableLength {
        /// Where the block starts.
        block: u64,
        /// Octets its header declares.
        length: u32,
    },
    /// A seek point whose sample number does not follow the one before it,
    /// or a seek point after a placeholder.
    SeekPointOrder {
        /// Where the block starts.
        block: u64,
        /// Where the seek point starts.
        point: u64,
    },
    /// A second block of a kind a stream holds at most once.
    Duplicate {
        /// Where the block starts.
        block: u64,
        /// What the block is.
        block_type: BlockType,
    },
    /// A picture given as a link (media type `-->`) instead of as data.
    /// Links in media are never followed (SEC-MED-016).
    LinkedPicture {
        /// Where the block starts.
        block: u64,
    },
}

/// What the metadata of a FLAC stream says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlacMetadata {
    /// The STREAMINFO block.
    pub stream_info: StreamInfo,
    /// The seek table's points, without placeholders, in order of sample.
    pub seek_table: Vec<SeekPoint>,
    /// Where the Vorbis comment block's body lies, if there is one.
    pub comment: Option<Range<u64>>,
    /// The pictures, in the order of their blocks.
    pub pictures: Vec<PictureRef>,
    /// Padding, application, cue sheet and reserved blocks, in order.
    pub raw: Vec<RawBlock>,
    /// The optional blocks that were skipped, in order.
    pub problems: Vec<BlockProblem>,
    /// Where the first audio frame starts: just past the last block.
    pub audio_start: u64,
}

/// Why the metadata could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlacError {
    /// A failure every parser shares: the file ended, the budget ran out,
    /// or a limit was passed.
    Fault(ParseFault),
    /// The stream does not start with `fLaC`.
    NotFlac {
        /// Where the stream starts.
        offset: u64,
        /// The four octets found there.
        found: [u8; 4],
    },
    /// The first block is not STREAMINFO.
    StreamInfoNotFirst {
        /// Where the block starts.
        offset: u64,
        /// What the block is.
        block_type: BlockType,
    },
    /// A second STREAMINFO block.
    SecondStreamInfo {
        /// Where the block starts.
        offset: u64,
    },
    /// A STREAMINFO block whose length is not 34 octets.
    StreamInfoLength {
        /// Where the block starts.
        offset: u64,
        /// Octets its header declares.
        length: u32,
    },
    /// A sample rate, channel count or bit depth outside its typed range.
    StreamInfoValue {
        /// Where the field that holds the three starts.
        offset: u64,
        /// Which value, and why.
        error: ValueError,
    },
    /// A block of type 127, which frame sync codes look like.
    ForbiddenBlockType {
        /// Where the block starts.
        offset: u64,
    },
    /// The host answered a read with other octets than the ones asked
    /// for.
    WrongWindow {
        /// The read asked for.
        expected: ReadRequest,
        /// Where the window it sent starts.
        offset: u64,
        /// How many octets it held.
        len: u64,
    },
}

impl FlacError {
    /// Where the parser was working when it stopped, in octets from the
    /// start of the file.
    #[must_use]
    pub const fn offset(&self) -> u64 {
        match *self {
            Self::Fault(fault) => fault.offset(),
            Self::NotFlac { offset, .. }
            | Self::StreamInfoNotFirst { offset, .. }
            | Self::SecondStreamInfo { offset }
            | Self::StreamInfoLength { offset, .. }
            | Self::StreamInfoValue { offset, .. }
            | Self::ForbiddenBlockType { offset } => offset,
            Self::WrongWindow { expected, .. } => expected.offset,
        }
    }
}

/// The parser that [`parse_metadata`] returns. Once it has answered with
/// its result, resuming it starts the parse again from the start window.
#[derive(Debug)]
pub struct MetadataParser<'b> {
    /// The limits the parse runs under.
    limits: Limits,
    /// The steps the parse may spend.
    budget: &'b mut Budget,
    /// Where the stream starts in the file.
    start: u64,
    /// How long the file is.
    file_len: u64,
    /// Octets asked for so far.
    read: u64,
    /// The read the next window must answer.
    pending: ReadRequest,
    /// What the next window holds.
    state: State,
}

/// Starts parsing the FLAC stream that begins `stream_start` octets into a
/// file, as a sans-I/O parser, under `limits` and charging `budget`.
///
/// Every offset in the result and in errors counts from the start of the
/// file, so a stream behind an `ID3v2` tag reports where things lie in the
/// file.
#[must_use]
pub fn parse_metadata<'b>(
    stream_start: u64,
    limits: &Limits,
    budget: &'b mut Budget,
) -> MetadataParser<'b> {
    MetadataParser {
        limits: *limits,
        budget,
        start: stream_start,
        file_len: 0,
        read: 0,
        pending: START,
        state: State::Start,
    }
}

/// What the next window holds.
#[derive(Debug)]
enum State {
    /// The start window, which says how long the file is.
    Start,
    /// The marker.
    Marker,
    /// The first block's header.
    FirstHeader,
    /// The STREAMINFO body.
    StreamInfo(Block),
    /// Something after STREAMINFO.
    Chain(Box<Found>, Next),
}

/// What the next window holds once STREAMINFO is read.
#[derive(Debug)]
enum Next {
    /// A block header.
    Header,
    /// A seek table's points.
    SeekPoints(Block, u64),
    /// A picture's type and media type length.
    PictureHead(Block),
    /// A picture's media type and description length.
    MediaType(PictureText),
    /// A picture's description and the fields after it, with the media type
    /// read before them.
    Description(PictureText, Text),
}

/// A picture read up to one of its text fields.
#[derive(Debug, Clone, Copy)]
struct PictureText {
    /// The picture's block.
    block: Block,
    /// The picture type.
    picture_type: u32,
    /// Octets of the text field the next window starts with.
    len: u64,
}

/// Where a block lies.
#[derive(Debug, Clone, Copy)]
struct Block {
    /// Where its header starts.
    header: u64,
    /// Where its body starts.
    body: u64,
    /// Just past its body.
    end: u64,
    /// Octets its header declares.
    length: u32,
    /// Whether it is the last block.
    last: bool,
}

/// What the blocks read so far hold.
#[derive(Debug)]
struct Found {
    /// The STREAMINFO block.
    stream_info: StreamInfo,
    /// The seek points kept.
    seek_table: Vec<SeekPoint>,
    /// The Vorbis comment's body.
    comment: Option<Range<u64>>,
    /// The pictures kept.
    pictures: Vec<PictureRef>,
    /// The blocks kept raw.
    raw: Vec<RawBlock>,
    /// The blocks skipped.
    problems: Vec<BlockProblem>,
    /// Blocks seen, STREAMINFO included.
    blocks: u64,
    /// Picture blocks seen.
    picture_blocks: u64,
    /// Whether a seek table block has been seen.
    seek_table_seen: bool,
}

impl Found {
    /// Nothing found yet but STREAMINFO.
    fn new(stream_info: StreamInfo) -> Box<Self> {
        Box::new(Self {
            stream_info,
            seek_table: Vec::new(),
            comment: None,
            pictures: Vec::new(),
            raw: Vec::new(),
            problems: Vec::new(),
            blocks: 1,
            picture_blocks: 0,
            seek_table_seen: false,
        })
    }

    /// The metadata, with the audio starting at `audio_start`.
    fn finish(self, audio_start: u64) -> FlacMetadata {
        FlacMetadata {
            stream_info: self.stream_info,
            seek_table: self.seek_table,
            comment: self.comment,
            pictures: self.pictures,
            raw: self.raw,
            problems: self.problems,
            audio_start,
        }
    }
}

/// What one turn of the parser returns before the result is wrapped.
type Turn = Result<Step<FlacMetadata>, FlacError>;

impl SansIo for MetadataParser<'_> {
    type Output = Result<FlacMetadata, FlacError>;

    fn resume(&mut self, window: Window<'_>) -> Step<Self::Output> {
        let output = match self.turn(window) {
            Ok(Step::Need(request)) => return Step::Need(request),
            Ok(Step::Done(metadata)) => Ok(metadata),
            Err(error) => Err(error),
        };
        self.pending = START;
        Step::Done(output)
    }
}

impl MetadataParser<'_> {
    /// Takes in the window for the pending read and answers with the next
    /// read or the result. The state is taken first, so a parse that ends
    /// leaves the parser at its start.
    fn turn(&mut self, window: Window<'_>) -> Turn {
        let state = mem::replace(&mut self.state, State::Start);
        let len = u64::try_from(window.bytes.len()).unwrap_or(u64::MAX);
        if window.offset != self.pending.offset || len != u64::from(self.pending.len) {
            return Err(FlacError::WrongWindow {
                expected: self.pending,
                offset: window.offset,
                len,
            });
        }
        match state {
            State::Start => {
                self.file_len = window.file_len;
                self.read = 0;
                self.request(State::Marker, self.start, HEADER)
            }
            State::Marker => {
                let found = octets(window.bytes, 0);
                if found != MARKER {
                    return Err(FlacError::NotFlac {
                        offset: self.start,
                        found,
                    });
                }
                // The marker lies inside the file, so this cannot overflow.
                self.request(State::FirstHeader, self.start.saturating_add(4), HEADER)
            }
            State::FirstHeader => {
                let (block_type, block) = self.header(window, 1)?;
                if block_type != BlockType::StreamInfo {
                    return Err(FlacError::StreamInfoNotFirst {
                        offset: block.header,
                        block_type,
                    });
                }
                if block.length != STREAM_INFO {
                    return Err(FlacError::StreamInfoLength {
                        offset: block.header,
                        length: block.length,
                    });
                }
                self.request(State::StreamInfo(block), block.body, STREAM_INFO.into())
            }
            State::StreamInfo(block) => {
                let stream_info = stream_info(window.bytes, block.body)?;
                self.after(Found::new(stream_info), block)
            }
            State::Chain(found, next) => self.chain(found, next, window),
        }
    }

    /// Takes in a window after STREAMINFO.
    fn chain(&mut self, found: Box<Found>, next: Next, window: Window<'_>) -> Turn {
        match next {
            Next::Header => self.block(found, window),
            Next::SeekPoints(block, count) => self.seek_points(found, block, count, window),
            Next::PictureHead(block) => {
                let picture_type = u32::from_be_bytes(octets(window.bytes, 0));
                let len = u64::from(u32::from_be_bytes(octets(window.bytes, 4)));
                let next = Next::MediaType(PictureText {
                    block,
                    picture_type,
                    len,
                });
                let at = window.offset.saturating_add(PICTURE_HEAD);
                self.part(found, block, next, at, len.saturating_add(4))
            }
            Next::MediaType(picture) => self.media_type(found, picture, window),
            Next::Description(picture, media_type) => {
                self.description(found, picture, media_type, window)
            }
        }
    }

    /// Takes in the header of a block after STREAMINFO.
    fn block(&mut self, mut found: Box<Found>, window: Window<'_>) -> Turn {
        let blocks = found.blocks.saturating_add(1);
        let (block_type, block) = self.header(window, blocks)?;
        found.blocks = blocks;
        match block_type {
            BlockType::StreamInfo => Err(FlacError::SecondStreamInfo {
                offset: block.header,
            }),
            BlockType::SeekTable => self.seek_table(found, block),
            BlockType::VorbisComment => {
                if found.comment.is_some() {
                    let problem = BlockProblem::Duplicate {
                        block: block.header,
                        block_type,
                    };
                    return self.skip(found, block, problem);
                }
                found.comment = Some(block.body..block.end);
                self.after(found, block)
            }
            BlockType::Picture => self.picture(found, block),
            BlockType::Padding
            | BlockType::Application
            | BlockType::CueSheet
            | BlockType::Reserved(_) => {
                found.raw.push(RawBlock {
                    block_type,
                    body: block.body..block.end,
                });
                self.after(found, block)
            }
        }
    }

    /// Takes in a picture's media type and the length of its description.
    fn media_type(&mut self, found: Box<Found>, picture: PictureText, window: Window<'_>) -> Turn {
        let PictureText { block, len, .. } = picture;
        let (bytes, rest) = split(window.bytes, len);
        let cap = self.cap(LimitKind::ShortText);
        let media_type = text::normalise(Untrusted::new(bytes), Lines::Single, cap);
        if media_type.value == LINK {
            let problem = BlockProblem::LinkedPicture {
                block: block.header,
            };
            return self.skip(found, block, problem);
        }
        let description = u64::from(u32::from_be_bytes(octets(rest, 0)));
        let next = Next::Description(
            PictureText {
                len: description,
                ..picture
            },
            media_type,
        );
        let at = window.offset.saturating_add(len).saturating_add(4);
        let len = description.saturating_add(PICTURE_FIELDS);
        self.part(found, block, next, at, len)
    }

    /// Takes in a picture's description and the fields after it, and keeps
    /// the picture if its data lies inside its block and within the limit.
    fn description(
        &mut self,
        mut found: Box<Found>,
        picture: PictureText,
        media_type: Text,
        window: Window<'_>,
    ) -> Turn {
        let PictureText {
            block,
            picture_type,
            len,
        } = picture;
        let (bytes, rest) = split(window.bytes, len);
        let cap = self.cap(LimitKind::LongText);
        let description = text::decode(Untrusted::new(bytes), Encoding::Utf8, cap);
        let field = |at| u32::from_be_bytes(octets(rest, at));
        let data_len = u64::from(field(16));
        let data = window
            .offset
            .saturating_add(len)
            .saturating_add(PICTURE_FIELDS);
        let checked = inside(data, data_len, block.end).and_then(|end| {
            self.limits
                .check(LimitKind::PictureBytes, data_len, data)
                .map(|()| end)
        });
        match checked {
            Ok(end) => {
                found.pictures.push(PictureRef {
                    picture_type,
                    media_type,
                    description,
                    width: field(0),
                    height: field(4),
                    depth: field(8),
                    colors: field(12),
                    data: data..end,
                });
                self.after(found, block)
            }
            Err(fault) => {
                let problem = BlockProblem::Fault {
                    block: block.header,
                    fault,
                };
                self.skip(found, block, problem)
            }
        }
    }

    /// Reads a block header from `window`, charging one step for it and
    /// counting it as block number `blocks`, and checks what every block
    /// must satisfy.
    fn header(&mut self, window: Window<'_>, blocks: u64) -> Result<(BlockType, Block), FlacError> {
        let at = window.offset;
        let [flags, high, middle, low] = octets(window.bytes, 0);
        self.budget.charge(1, at).map_err(FlacError::Fault)?;
        self.limits
            .check(LimitKind::Children, blocks, at)
            .map_err(FlacError::Fault)?;
        let block_type = BlockType::from_code(flags & 0x7F)
            .ok_or(FlacError::ForbiddenBlockType { offset: at })?;
        let length = u32::from_be_bytes([0, high, middle, low]);
        // The header lies inside the file, so this cannot overflow.
        let body = at.saturating_add(HEADER);
        let end = inside(body, length.into(), self.file_len).map_err(FlacError::Fault)?;
        let block = Block {
            header: at,
            body,
            end,
            length,
            last: flags & 0x80 != 0,
        };
        Ok((block_type, block))
    }

    /// Decides what to do with a seek table block.
    fn seek_table(&mut self, mut found: Box<Found>, block: Block) -> Turn {
        if mem::replace(&mut found.seek_table_seen, true) {
            let problem = BlockProblem::Duplicate {
                block: block.header,
                block_type: BlockType::SeekTable,
            };
            return self.skip(found, block, problem);
        }
        let length = u64::from(block.length);
        let point = u64::try_from(SEEK_POINT).unwrap_or(u64::MAX);
        if length.checked_rem(point) != Some(0) {
            let problem = BlockProblem::SeekTableLength {
                block: block.header,
                length: block.length,
            };
            return self.skip(found, block, problem);
        }
        let count = length.checked_div(point).unwrap_or_default();
        if let Err(fault) = self
            .limits
            .check(LimitKind::IndexEntries, count, block.body)
        {
            let problem = BlockProblem::Fault {
                block: block.header,
                fault,
            };
            return self.skip(found, block, problem);
        }
        self.budget
            .charge(count, block.body)
            .map_err(FlacError::Fault)?;
        if count == 0 {
            return self.after(found, block);
        }
        let next = Next::SeekPoints(block, count);
        self.part(found, block, next, block.body, length)
    }

    /// Takes in a seek table's points, which are whole.
    fn seek_points(
        &mut self,
        mut found: Box<Found>,
        block: Block,
        count: u64,
        window: Window<'_>,
    ) -> Turn {
        let point_len = u64::try_from(SEEK_POINT).unwrap_or(u64::MAX);
        let ceiling = self.limits.get(LimitKind::IndexEntries);
        let mut points = bounded_vec(count, point_len, block.length.into(), ceiling);
        let mut previous: Option<u64> = None;
        let mut placeholders = false;
        let mut at = block.body;
        for point in window.bytes.chunks_exact(SEEK_POINT) {
            let sample = u64::from_be_bytes(octets(point, 0));
            if sample == PLACEHOLDER {
                placeholders = true;
            } else if placeholders || previous.is_some_and(|previous| previous >= sample) {
                let problem = BlockProblem::SeekPointOrder {
                    block: block.header,
                    point: at,
                };
                return self.skip(found, block, problem);
            } else {
                previous = Some(sample);
                points.push(SeekPoint {
                    sample,
                    offset: u64::from_be_bytes(octets(point, 8)),
                    samples: u16::from_be_bytes(octets(point, 16)),
                });
            }
            at = at.saturating_add(point_len);
        }
        found.seek_table = points;
        self.after(found, block)
    }

    /// Decides what to do with a picture block.
    fn picture(&mut self, mut found: Box<Found>, block: Block) -> Turn {
        found.picture_blocks = found.picture_blocks.saturating_add(1);
        let counted = self
            .limits
            .check(LimitKind::Pictures, found.picture_blocks, block.header);
        if let Err(fault) = counted {
            let problem = BlockProblem::Fault {
                block: block.header,
                fault,
            };
            return self.skip(found, block, problem);
        }
        let next = Next::PictureHead(block);
        self.part(found, block, next, block.body, PICTURE_HEAD)
    }

    /// Asks for `len` octets at `at` inside an optional block, then expects
    /// `next`. A read that would pass the block's end or a limit skips the
    /// block instead.
    fn part(&mut self, mut found: Box<Found>, block: Block, next: Next, at: u64, len: u64) -> Turn {
        match inside(at, len, block.end).and_then(|_| self.need(at, len)) {
            Ok(request) => {
                self.state = State::Chain(found, next);
                Ok(Step::Need(request))
            }
            Err(fault) => {
                found.problems.push(BlockProblem::Fault {
                    block: block.header,
                    fault,
                });
                self.after(found, block)
            }
        }
    }

    /// Records `problem` for an optional block and moves past it.
    fn skip(&mut self, mut found: Box<Found>, block: Block, problem: BlockProblem) -> Turn {
        found.problems.push(problem);
        self.after(found, block)
    }

    /// Moves past `block`: the result after the last block, otherwise the
    /// next header.
    fn after(&mut self, found: Box<Found>, block: Block) -> Turn {
        if block.last {
            return Ok(Step::Done(found.finish(block.end)));
        }
        self.request(State::Chain(found, Next::Header), block.end, HEADER)
    }

    /// Asks for `len` octets at `at`, which the parse needs, then expects
    /// `state`.
    fn request(&mut self, state: State, at: u64, len: u64) -> Turn {
        let request = self.need(at, len).map_err(FlacError::Fault)?;
        self.state = state;
        Ok(Step::Need(request))
    }

    /// The read of `len` octets at `at`, unless it would pass the end of
    /// the file, the longest read, or the octets one file may have read.
    fn need(&mut self, at: u64, len: u64) -> Result<ReadRequest, ParseFault> {
        inside(at, len, self.file_len)?;
        self.limits.check(LimitKind::ReadBytes, len, at)?;
        let read = self.read.saturating_add(len);
        self.limits.check(LimitKind::FileBytes, read, at)?;
        self.read = read;
        // At most ReadBytes, whose ceiling is 16 MiB, so it fits.
        let len = u32::try_from(len).unwrap_or(u32::MAX);
        self.pending = ReadRequest { offset: at, len };
        Ok(self.pending)
    }

    /// The cap on a text field. Text limits fit 32 bits at their ceilings.
    fn cap(&self, kind: LimitKind) -> u32 {
        u32::try_from(self.limits.get(kind)).unwrap_or(u32::MAX)
    }
}

/// The end of `len` octets at `at`, unless they run past `end`.
fn inside(at: u64, len: u64, end: u64) -> Result<u64, ParseFault> {
    at.checked_add(len)
        .filter(|&stop| stop <= end)
        .ok_or(ParseFault::Truncated {
            offset: at,
            needed: len,
            available: end.saturating_sub(at),
        })
}

/// The STREAMINFO block whose body is `bytes`, starting at `body`.
fn stream_info(bytes: &[u8], body: u64) -> Result<StreamInfo, FlacError> {
    let [min_high, min_middle, min_low, max_high, max_middle, max_low] = octets(bytes, 4);
    // Sample rate (20 bits), channels minus one (3), bits per sample minus
    // one (5) and total samples (36).
    let packed = u64::from_be_bytes(octets(bytes, 10));
    let field = |shift, mask| u32::try_from(packed.wrapping_shr(shift) & mask).unwrap_or(0);
    let md5 = octets(bytes, 18);
    SampleRate::new(field(44, 0xF_FFFF))
        .and_then(|sample_rate| {
            Channels::new(field(41, 0x7).wrapping_add(1)).and_then(|channels| {
                BitDepth::new(field(36, 0x1F).wrapping_add(1)).map(|bits_per_sample| StreamInfo {
                    min_block_size: u16::from_be_bytes(octets(bytes, 0)),
                    max_block_size: u16::from_be_bytes(octets(bytes, 2)),
                    min_frame_size: NonZeroU32::new(u32::from_be_bytes([
                        0, min_high, min_middle, min_low,
                    ])),
                    max_frame_size: NonZeroU32::new(u32::from_be_bytes([
                        0, max_high, max_middle, max_low,
                    ])),
                    sample_rate,
                    channels,
                    bits_per_sample,
                    total_samples: NonZeroU64::new(packed & 0xF_FFFF_FFFF),
                    md5: (md5 != [0; 16]).then_some(AudioMd5(md5)),
                })
            })
        })
        .map_err(|error| FlacError::StreamInfoValue {
            offset: body.saturating_add(10),
            error,
        })
}

/// The `N` octets of `bytes` from `from` on. Every window holds exactly the
/// octets asked for, which the parser checks before reading it, so the
/// zeros that stand in past the end never stand in for data.
fn octets<const N: usize>(bytes: &[u8], from: usize) -> [u8; N] {
    let mut out = [0; N];
    for (slot, &octet) in out.iter_mut().zip(bytes.iter().skip(from)) {
        *slot = octet;
    }
    out
}

/// `bytes` split after its first `len` octets.
fn split(bytes: &[u8], len: u64) -> (&[u8], &[u8]) {
    let at = usize::try_from(len).unwrap_or(usize::MAX);
    bytes.split_at_checked(at).unwrap_or((bytes, &[]))
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use super::*;
    use crate::parse::drive;
    use crate::values::Field;
    use gunmetal_testkit::bytes::Bytes;
    use gunmetal_testkit::flac::{self as build, Block as Built};
    use proptest::collection::{btree_set, vec};
    use proptest::prelude::*;
    use proptest::sample::Index;
    use std::iter::once;

    /// RFC 9639, Appendix D.1, as section D.1.1 prints it: the marker, a
    /// last STREAMINFO block and one frame.
    const EXAMPLE_1: [u8; 57] = [
        0x66, 0x4C, 0x61, 0x43, 0x80, 0x00, 0x00, 0x22, 0x10, 0x00, 0x10, 0x00, //
        0x00, 0x00, 0x0F, 0x00, 0x00, 0x0F, 0x0A, 0xC4, 0x42, 0xF0, 0x00, 0x00, //
        0x00, 0x01, 0x3E, 0x84, 0xB4, 0x18, 0x07, 0xDC, 0x69, 0x03, 0x07, 0x58, //
        0x6A, 0x3D, 0xAD, 0x1A, 0x2E, 0x0F, 0xFF, 0xF8, 0x69, 0x18, 0x00, 0x00, //
        0xBF, 0x03, 0x58, 0xFD, 0x03, 0x12, 0x8B, 0xAA, 0x9A,
    ];

    /// The frame of the first example, which follows every stream built
    /// here, so the audio never starts at the end of the file.
    const FRAME: [u8; 15] = [
        0xFF, 0xF8, 0x69, 0x18, 0x00, 0x00, 0xBF, 0x03, 0x58, 0xFD, 0x03, 0x12, 0x8B, 0xAA, 0x9A,
    ];

    fn length(bytes: &[u8]) -> u64 {
        u64::try_from(bytes.len()).unwrap()
    }

    /// Parses the stream `start` octets into `file` under `limits`, with a
    /// budget sized as the module says no parse can spend.
    fn parse_with(file: &[u8], start: u64, limits: &Limits) -> Result<FlacMetadata, FlacError> {
        let mut budget = Budget::for_input(length(file), STEPS_PER_OCTET, STEPS_FIXED);
        drive(parse_metadata(start, limits, &mut budget), file, limits)
            .expect("the parser asks for no read the host refuses")
    }

    fn parse(file: &[u8]) -> Result<FlacMetadata, FlacError> {
        parse_with(file, 0, &Limits::DEFAULT)
    }

    fn lowered(kind: LimitKind, value: u64) -> Limits {
        Limits::DEFAULT.with_override(kind, value).unwrap()
    }

    fn truncated(offset: u64, needed: u64, available: u64) -> ParseFault {
        ParseFault::Truncated {
            offset,
            needed,
            available,
        }
    }

    fn exceeded(limit: LimitKind, value: u64, max: u64, offset: u64) -> ParseFault {
        ParseFault::LimitExceeded {
            limit,
            value,
            max,
            offset,
        }
    }

    fn text(value: &str) -> Text {
        Text {
            value: value.to_owned(),
            truncated: false,
            replaced: false,
        }
    }

    /// The first example's STREAMINFO, for the builder.
    fn built_info() -> build::StreamInfo {
        build::StreamInfo {
            min_block_size: 4_096,
            max_block_size: 4_096,
            min_frame_size: 15,
            max_frame_size: 15,
            sample_rate: 44_100,
            channels: 2,
            bits_per_sample: 16,
            total_samples: 1,
            md5: [
                0x3E, 0x84, 0xB4, 0x18, 0x07, 0xDC, 0x69, 0x03, 0x07, 0x58, 0x6A, 0x3D, 0xAD, 0x1A,
                0x2E, 0x0F,
            ],
        }
    }

    /// The first example's STREAMINFO, as RFC 9639, Table 28 reads it.
    fn example_info() -> StreamInfo {
        StreamInfo {
            min_block_size: 4_096,
            max_block_size: 4_096,
            min_frame_size: NonZeroU32::new(15),
            max_frame_size: NonZeroU32::new(15),
            sample_rate: SampleRate::new(44_100).unwrap(),
            channels: Channels::new(2).unwrap(),
            bits_per_sample: BitDepth::new(16).unwrap(),
            total_samples: NonZeroU64::new(1),
            md5: Some(AudioMd5([
                0x3E, 0x84, 0xB4, 0x18, 0x07, 0xDC, 0x69, 0x03, 0x07, 0x58, 0x6A, 0x3D, 0xAD, 0x1A,
                0x2E, 0x0F,
            ])),
        }
    }

    /// The marker, the first example's STREAMINFO, `rest` with the final
    /// block flagged as the last, and a frame.
    fn file(rest: Vec<Built>) -> Vec<u8> {
        let blocks: Vec<Built> = once(Built::StreamInfo(built_info())).chain(rest).collect();
        [build::stream(&blocks), FRAME.to_vec()].concat()
    }

    /// What a stream with the first example's STREAMINFO and nothing else
    /// found says, its audio starting at `audio_start`.
    fn metadata(audio_start: u64) -> FlacMetadata {
        FlacMetadata {
            stream_info: example_info(),
            seek_table: vec![],
            comment: None,
            pictures: vec![],
            raw: vec![],
            problems: vec![],
            audio_start,
        }
    }

    fn point(sample: u64, offset: u64, samples: u16) -> build::SeekPoint {
        build::SeekPoint {
            sample,
            offset,
            samples,
        }
    }

    fn placeholder() -> build::SeekPoint {
        point(build::PLACEHOLDER, 0, 0)
    }

    /// A PNG front cover of 1 by 2 pixels, with four octets of data.
    fn cover() -> build::Picture {
        build::Picture {
            picture_type: 3,
            media_type: b"image/png".to_vec(),
            description: b"Cover".to_vec(),
            width: 1,
            height: 2,
            depth: 24,
            colors: 0,
            data: vec![0x89, b'P', b'N', b'G'],
        }
    }

    /// The cover as the parser returns it, for a block whose body starts
    /// at `body`: its data starts 46 octets in.
    fn cover_ref(body: u64) -> PictureRef {
        PictureRef {
            picture_type: 3,
            media_type: text("image/png"),
            description: text("Cover"),
            width: 1,
            height: 2,
            depth: 24,
            colors: 0,
            data: body + 46..body + 50,
        }
    }

    /// Records every read a parser asks for.
    struct Recorder<P> {
        parser: P,
        requests: Vec<(u64, u32)>,
    }

    impl<P: SansIo> SansIo for Recorder<P> {
        type Output = (P::Output, Vec<(u64, u32)>);

        fn resume(&mut self, window: Window<'_>) -> Step<Self::Output> {
            match self.parser.resume(window) {
                Step::Need(request) => {
                    self.requests.push((request.offset, request.len));
                    Step::Need(request)
                }
                Step::Done(output) => Step::Done((output, mem::take(&mut self.requests))),
            }
        }
    }

    /// Parses `file` and returns what it found and every read it asked
    /// for.
    fn recorded(file: &[u8]) -> (Result<FlacMetadata, FlacError>, Vec<(u64, u32)>) {
        let mut budget = Budget::for_input(length(file), STEPS_PER_OCTET, STEPS_FIXED);
        let recorder = Recorder {
            parser: parse_metadata(0, &Limits::DEFAULT, &mut budget),
            requests: vec![],
        };
        drive(recorder, file, &Limits::DEFAULT).expect("the host serves every read")
    }

    /// Serves `parser` the reads it asks for from `file` until it answers,
    /// failing instead of hanging if it asks for more reads than `file`
    /// has octets, since each read is at least one octet.
    fn serve(parser: &mut MetadataParser<'_>, file: &[u8]) -> Result<FlacMetadata, FlacError> {
        let file_len = length(file);
        let mut window = Window::start(file_len);
        let mut reads = 0;
        loop {
            match parser.resume(window) {
                Step::Done(output) => return output,
                Step::Need(ReadRequest { offset, len }) => {
                    reads += 1;
                    assert!(reads <= file.len(), "{reads} reads of {file_len} octets");
                    let from = usize::try_from(offset).unwrap();
                    let to = from + usize::try_from(len).unwrap();
                    window = Window {
                        offset,
                        bytes: &file[from..to],
                        file_len,
                    };
                }
            }
        }
    }

    /// Runs `work` on a thread with the 256 KiB stack SEC-MED-001 names, so
    /// a parse that recursed would fail here.
    fn on_small_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(262_144)
            .spawn(work)
            .expect("the test thread starts")
            .join()
            .expect("the parse returned instead of panicking")
    }

    #[test]
    fn reads_the_first_example_of_rfc_9639() {
        assert_eq!(parse(&EXAMPLE_1), Ok(metadata(42)));
    }

    /// A stream with one block of every kind, the seek table holding a
    /// placeholder. The blocks start at 4, 42, 100, 109, 163, 173, 180 and
    /// 185, and the frame at 191.
    fn every_kind() -> Vec<u8> {
        file(vec![
            Built::SeekTable(vec![
                point(0, 0, 4_096),
                point(4_096, 1_000, 4_096),
                placeholder(),
            ]),
            Built::VorbisComment(vec![1, 2, 3, 4, 5]),
            Built::Picture(cover()),
            Built::Application {
                id: *b"xmcd",
                data: vec![1, 2],
            },
            Built::CueSheet(vec![0xCC, 0xCC, 0xCC]),
            Built::Raw {
                code: 77,
                body: vec![0xEE],
            },
            Built::Padding(2),
        ])
    }

    #[test]
    fn reads_every_kind_of_block_with_its_byte_range() {
        assert_eq!(
            parse(&every_kind()),
            Ok(FlacMetadata {
                seek_table: vec![
                    SeekPoint {
                        sample: 0,
                        offset: 0,
                        samples: 4_096,
                    },
                    SeekPoint {
                        sample: 4_096,
                        offset: 1_000,
                        samples: 4_096,
                    },
                ],
                comment: Some(104..109),
                pictures: vec![cover_ref(113)],
                raw: vec![
                    RawBlock {
                        block_type: BlockType::Application,
                        body: 167..173,
                    },
                    RawBlock {
                        block_type: BlockType::CueSheet,
                        body: 177..180,
                    },
                    RawBlock {
                        block_type: BlockType::Reserved(77),
                        body: 184..185,
                    },
                    RawBlock {
                        block_type: BlockType::Padding,
                        body: 189..191,
                    },
                ],
                ..metadata(191)
            })
        );
    }

    /// The parser reads the marker, the headers, STREAMINFO, the seek
    /// table and a picture's fields, and never a picture's data, the
    /// comment or a raw block, so a scan reads headers only (LIB-019).
    /// Each read lies inside the file and past the one before it.
    ///
    /// Verifies: SEC-MED-008
    #[test]
    fn reads_only_what_it_returns_and_always_moves_on() {
        let (result, requests) = recorded(&every_kind());
        assert_eq!(result.map(|found| found.audio_start), Ok(191));
        assert_eq!(
            requests,
            [
                (0, 4),    // marker
                (4, 4),    // STREAMINFO header
                (8, 34),   // STREAMINFO
                (42, 4),   // seek table header
                (46, 54),  // seek points
                (100, 4),  // comment header
                (109, 4),  // picture header
                (113, 8),  // type and media type length
                (121, 13), // media type and description length
                (134, 25), // description and the fields after it
                (163, 4),  // application header
                (173, 4),  // cue sheet header
                (180, 4),  // reserved block header
                (185, 4),  // padding header
            ]
        );
        // Empty blocks are four header octets each, so even they move the
        // parse on.
        let (result, requests) = recorded(&file(vec![
            Built::Padding(0),
            Built::VorbisComment(vec![]),
            Built::SeekTable(vec![]),
        ]));
        assert_eq!(
            result,
            Ok(FlacMetadata {
                comment: Some(50..50),
                raw: vec![RawBlock {
                    block_type: BlockType::Padding,
                    body: 46..46,
                }],
                ..metadata(54)
            })
        );
        assert_eq!(
            requests,
            [(0, 4), (4, 4), (8, 34), (42, 4), (46, 4), (50, 4)]
        );
    }

    #[test]
    fn reads_a_stream_behind_an_id3v2_tag_at_file_offsets() {
        let mut tagged = Bytes::new();
        tagged
            .bytes(b"ID3\x04\x00\x00\x00\x00\x00\x00")
            .bytes(&EXAMPLE_1);
        let tagged = tagged.into_vec();
        assert_eq!(parse_with(&tagged, 10, &Limits::DEFAULT), Ok(metadata(52)));
        assert_eq!(
            parse(&tagged),
            Err(FlacError::NotFlac {
                offset: 0,
                found: *b"ID3\x04",
            })
        );
    }

    #[test]
    fn reads_every_streaminfo_field_at_both_ends_of_its_range() {
        let narrowest = build::StreamInfo {
            min_block_size: 0,
            max_block_size: 1,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 1,
            channels: 1,
            bits_per_sample: 1,
            total_samples: 0,
            md5: [0; 16],
        };
        let mut stream = build::stream(&[Built::StreamInfo(narrowest)]);
        stream.extend(FRAME);
        assert_eq!(
            parse(&stream),
            Ok(FlacMetadata {
                stream_info: StreamInfo {
                    min_block_size: 0,
                    max_block_size: 1,
                    min_frame_size: None,
                    max_frame_size: None,
                    sample_rate: SampleRate::new(1).unwrap(),
                    channels: Channels::new(1).unwrap(),
                    bits_per_sample: BitDepth::new(1).unwrap(),
                    total_samples: None,
                    md5: None,
                },
                ..metadata(42)
            })
        );
        let widest = build::StreamInfo {
            min_block_size: u16::MAX,
            max_block_size: 0xFFFE,
            min_frame_size: 0xFF_FFFF,
            max_frame_size: 0xFF_FFFE,
            sample_rate: 768_000,
            channels: 8,
            bits_per_sample: 32,
            total_samples: 0xF_FFFF_FFFF,
            md5: [0xFF; 16],
        };
        let mut stream = build::stream(&[Built::StreamInfo(widest)]);
        stream.extend(FRAME);
        assert_eq!(
            parse(&stream),
            Ok(FlacMetadata {
                stream_info: StreamInfo {
                    min_block_size: u16::MAX,
                    max_block_size: 0xFFFE,
                    min_frame_size: NonZeroU32::new(0xFF_FFFF),
                    max_frame_size: NonZeroU32::new(0xFF_FFFE),
                    sample_rate: SampleRate::new(768_000).unwrap(),
                    channels: Channels::new(8).unwrap(),
                    bits_per_sample: BitDepth::new(32).unwrap(),
                    total_samples: NonZeroU64::new(0xF_FFFF_FFFF),
                    md5: Some(AudioMd5([0xFF; 16])),
                },
                ..metadata(42)
            })
        );
    }

    /// An MD5 of all zeros means "not set", and identity must not use it;
    /// zero total samples means "unknown".
    #[test]
    fn keeps_no_md5_or_sample_count_the_encoder_did_not_know() {
        let unknown = build::StreamInfo {
            total_samples: 0,
            md5: [0; 16],
            ..built_info()
        };
        let mut stream = build::stream(&[Built::StreamInfo(unknown)]);
        stream.extend(FRAME);
        assert_eq!(
            parse(&stream),
            Ok(FlacMetadata {
                stream_info: StreamInfo {
                    total_samples: None,
                    md5: None,
                    ..example_info()
                },
                ..metadata(42)
            })
        );
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn refuses_a_sample_rate_outside_its_typed_range() {
        for (rate, accepted) in [
            (0, false),
            (768_000, true),
            (768_001, false),
            (0xF_FFFF, false),
        ] {
            let info = build::StreamInfo {
                sample_rate: rate,
                ..built_info()
            };
            let mut stream = build::stream(&[Built::StreamInfo(info)]);
            stream.extend(FRAME);
            let expected = if accepted {
                Ok(FlacMetadata {
                    stream_info: StreamInfo {
                        sample_rate: SampleRate::new(rate).unwrap(),
                        ..example_info()
                    },
                    ..metadata(42)
                })
            } else {
                Err(FlacError::StreamInfoValue {
                    offset: 18,
                    error: ValueError::OutOfRange {
                        field: Field::SampleRate,
                        value: rate.into(),
                    },
                })
            };
            assert_eq!(parse(&stream), expected, "{rate} Hz");
        }
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn reports_where_a_file_too_short_to_be_flac_ends() {
        let cases: [(&[u8], ParseFault); 5] = [
            (&[], truncated(0, 4, 0)),
            (b"f", truncated(0, 4, 1)),
            (b"fLa", truncated(0, 4, 3)),
            (b"fLaC", truncated(4, 4, 0)),
            (b"fLaC\x80\x00\x00\x22\x10", truncated(8, 34, 1)),
        ];
        for (bytes, fault) in cases {
            assert_eq!(parse(bytes), Err(FlacError::Fault(fault)), "{bytes:02X?}");
        }
    }

    #[test]
    fn reports_a_stream_start_past_the_end_of_the_file() {
        assert_eq!(
            parse_with(&EXAMPLE_1, 54, &Limits::DEFAULT),
            Err(FlacError::Fault(truncated(54, 4, 3)))
        );
        assert_eq!(
            parse_with(&EXAMPLE_1, 58, &Limits::DEFAULT),
            Err(FlacError::Fault(truncated(58, 4, 0)))
        );
        // The marker would end past u64::MAX.
        assert_eq!(
            parse_with(&EXAMPLE_1, u64::MAX - 1, &Limits::DEFAULT),
            Err(FlacError::Fault(truncated(u64::MAX - 1, 4, 0)))
        );
    }

    #[test]
    fn refuses_a_stream_without_the_marker() {
        let mut ogg = EXAMPLE_1;
        ogg[..4].copy_from_slice(b"OggS");
        assert_eq!(
            parse(&ogg),
            Err(FlacError::NotFlac {
                offset: 0,
                found: *b"OggS",
            })
        );
        let mut lower = EXAMPLE_1;
        lower[3] = b'c';
        assert_eq!(
            parse(&lower),
            Err(FlacError::NotFlac {
                offset: 0,
                found: *b"fLac",
            })
        );
    }

    #[test]
    fn refuses_a_first_block_that_is_not_streaminfo() {
        let padding_first = build::stream(&[Built::Padding(0), Built::StreamInfo(built_info())]);
        assert_eq!(
            parse(&padding_first),
            Err(FlacError::StreamInfoNotFirst {
                offset: 4,
                block_type: BlockType::Padding,
            })
        );
        let reserved_first = build::stream(&[Built::Raw {
            code: 9,
            body: vec![],
        }]);
        assert_eq!(
            parse(&reserved_first),
            Err(FlacError::StreamInfoNotFirst {
                offset: 4,
                block_type: BlockType::Reserved(9),
            })
        );
    }

    #[test]
    fn refuses_a_second_streaminfo_block() {
        assert_eq!(
            parse(&file(vec![Built::StreamInfo(built_info())])),
            Err(FlacError::SecondStreamInfo { offset: 42 })
        );
    }

    /// A STREAMINFO block shorter than its fields is an error, as is a
    /// longer one.
    ///
    /// Verifies: SEC-MED-008
    #[test]
    fn refuses_a_streaminfo_block_of_any_length_but_34() {
        for length in [0_u32, 33, 35] {
            let mut body = Bytes::new();
            body.zeros(usize::try_from(length).unwrap());
            let stream = build::stream(&[Built::Raw {
                code: 0,
                body: body.into_vec(),
            }]);
            assert_eq!(
                parse(&stream),
                Err(FlacError::StreamInfoLength { offset: 4, length }),
                "{length} octets"
            );
        }
    }

    /// Type 127 is forbidden so that a frame's sync code cannot pass for a
    /// header.
    #[test]
    fn refuses_a_block_of_the_forbidden_type() {
        let forbidden = file(vec![Built::Raw {
            code: 127,
            body: vec![],
        }]);
        assert_eq!(
            parse(&forbidden),
            Err(FlacError::ForbiddenBlockType { offset: 42 })
        );
    }

    /// Verifies: SEC-TM-032
    #[test]
    fn refuses_a_chain_whose_last_block_flag_is_never_set() {
        let unflagged = [
            build::MARKER.to_vec(),
            Built::StreamInfo(built_info()).encode(false),
        ]
        .concat();
        assert_eq!(
            parse(&unflagged),
            Err(FlacError::Fault(truncated(42, 4, 0)))
        );
        // Followed by audio, the parser reads the frame's sync code as a
        // header of the forbidden type.
        let before_audio = [unflagged, FRAME.to_vec()].concat();
        assert_eq!(
            parse(&before_audio),
            Err(FlacError::ForbiddenBlockType { offset: 42 })
        );
    }

    /// Verifies: SEC-TM-032
    #[test]
    fn refuses_a_block_whose_length_runs_past_the_end_of_the_file() {
        let mut stream = Bytes::new();
        stream
            .bytes(&build::MARKER)
            .bytes(&Built::StreamInfo(built_info()).encode(false))
            .bytes(&build::header(true, 1, 100))
            .zeros(10);
        assert_eq!(
            parse(stream.as_slice()),
            Err(FlacError::Fault(truncated(46, 100, 10)))
        );
        let mut longest = Bytes::new();
        longest
            .bytes(&build::MARKER)
            .bytes(&Built::StreamInfo(built_info()).encode(false))
            .bytes(&build::header(true, 2, 0xFF_FFFF));
        assert_eq!(
            parse(longest.as_slice()),
            Err(FlacError::Fault(truncated(46, 0xFF_FFFF, 0)))
        );
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn allows_65_536_blocks_and_refuses_the_next_one() {
        let blocks =
            |count: u64| -> Vec<u8> { file((1..count).map(|_| Built::Padding(0)).collect()) };
        let most = blocks(65_536);
        let expected = FlacMetadata {
            raw: (0..65_535_u64)
                .map(|index| RawBlock {
                    block_type: BlockType::Padding,
                    body: 46 + 4 * index..46 + 4 * index,
                })
                .collect(),
            ..metadata(42 + 4 * 65_535)
        };
        // Nothing nests, so the parse moves along the chain without
        // recursing, and a 256 KiB stack holds any number of blocks.
        assert_eq!(on_small_stack(move || parse(&most)), Ok(expected));
        assert_eq!(
            parse(&blocks(65_537)),
            Err(FlacError::Fault(exceeded(
                LimitKind::Children,
                65_537,
                65_536,
                42 + 4 * 65_535
            )))
        );
    }

    /// Verifies: SEC-MED-005
    #[test]
    fn walks_a_long_chain_on_a_small_stack() {
        let chain = file((0..10_000).map(|_| Built::Padding(0)).collect());
        let found = on_small_stack(move || parse(&chain).map(|found| found.raw.len()));
        assert_eq!(found, Ok(10_000));
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn counts_every_block_against_a_lowered_limit() {
        assert_eq!(
            parse_with(&EXAMPLE_1, 0, &lowered(LimitKind::Children, 0)),
            Err(FlacError::Fault(exceeded(LimitKind::Children, 1, 0, 4)))
        );
        let three = file(vec![Built::Padding(0), Built::Padding(0)]);
        assert_eq!(
            parse_with(&three, 0, &lowered(LimitKind::Children, 3)),
            Ok(FlacMetadata {
                raw: vec![
                    RawBlock {
                        block_type: BlockType::Padding,
                        body: 46..46,
                    },
                    RawBlock {
                        block_type: BlockType::Padding,
                        body: 50..50,
                    },
                ],
                ..metadata(50)
            })
        );
        assert_eq!(
            parse_with(&three, 0, &lowered(LimitKind::Children, 2)),
            Err(FlacError::Fault(exceeded(LimitKind::Children, 3, 2, 46)))
        );
    }

    /// A stream laid out as RFC 9639's second example charges one step for
    /// each of its four headers and one for its seek point, at offsets 4,
    /// 42, 46, 64 and 126 in that order.
    ///
    /// Verifies: SEC-MED-007
    #[test]
    fn runs_out_of_budget_at_exactly_the_step_it_should() {
        let example_2 = file(vec![
            Built::SeekTable(vec![point(0, 0, 16)]),
            Built::VorbisComment((0..58).collect()),
            Built::Padding(6),
        ]);
        for (steps, offset) in [(0, 4), (1, 42), (2, 46), (3, 64), (4, 126)] {
            let mut budget = Budget::for_input(0, 0, steps);
            let result = drive(
                parse_metadata(0, &Limits::DEFAULT, &mut budget),
                &example_2,
                &Limits::DEFAULT,
            );
            assert_eq!(
                result,
                Ok(Err(FlacError::Fault(ParseFault::BudgetExceeded { offset }))),
                "{steps} steps"
            );
        }
        let mut budget = Budget::for_input(0, 0, 5);
        let result = drive(
            parse_metadata(0, &Limits::DEFAULT, &mut budget),
            &example_2,
            &Limits::DEFAULT,
        );
        assert_eq!(
            result.map(|parsed| parsed.map(|found| found.audio_start)),
            Ok(Ok(136))
        );
        assert_eq!(budget.remaining(), 0);
    }

    /// A seek table is charged whole before it is read.
    ///
    /// Verifies: SEC-MED-007
    #[test]
    fn charges_a_seek_table_for_every_point_before_reading_it() {
        let table = file(vec![Built::SeekTable(vec![
            point(1, 0, 1),
            point(2, 0, 1),
            point(3, 0, 1),
        ])]);
        let mut budget = Budget::for_input(0, 0, 4);
        let recorder = Recorder {
            parser: parse_metadata(0, &Limits::DEFAULT, &mut budget),
            requests: vec![],
        };
        assert_eq!(
            drive(recorder, &table, &Limits::DEFAULT),
            Ok((
                Err(FlacError::Fault(ParseFault::BudgetExceeded { offset: 46 })),
                vec![(0, 4), (4, 4), (8, 34), (42, 4)]
            ))
        );
    }

    #[test]
    fn keeps_the_seek_points_and_drops_the_placeholders() {
        let table = file(vec![Built::SeekTable(vec![
            point(0, 0, 4_096),
            point(u64::MAX - 1, 0x0102_0304_0506_0708, u16::MAX),
            placeholder(),
            point(build::PLACEHOLDER, 1, 1),
        ])]);
        assert_eq!(
            parse(&table),
            Ok(FlacMetadata {
                seek_table: vec![
                    SeekPoint {
                        sample: 0,
                        offset: 0,
                        samples: 4_096,
                    },
                    SeekPoint {
                        sample: u64::MAX - 1,
                        offset: 0x0102_0304_0506_0708,
                        samples: u16::MAX,
                    },
                ],
                ..metadata(118)
            })
        );
    }

    #[test]
    fn skips_a_seek_table_whose_points_are_out_of_order() {
        let cases = [
            ("falling", vec![point(10, 0, 1), point(5, 0, 1)], 64),
            ("repeated", vec![point(10, 0, 1), point(10, 0, 1)], 64),
            (
                "after a placeholder",
                vec![placeholder(), point(10, 0, 1)],
                64,
            ),
            (
                "third",
                vec![point(1, 0, 1), point(2, 0, 1), point(2, 0, 1)],
                82,
            ),
        ];
        for (name, points, at) in cases {
            let padding = 46 + 18 * u64::try_from(points.len()).unwrap();
            let table = file(vec![Built::SeekTable(points), Built::Padding(0)]);
            assert_eq!(
                parse(&table),
                Ok(FlacMetadata {
                    raw: vec![RawBlock {
                        block_type: BlockType::Padding,
                        body: padding + 4..padding + 4,
                    }],
                    problems: vec![BlockProblem::SeekPointOrder {
                        block: 42,
                        point: at,
                    }],
                    ..metadata(padding + 4)
                }),
                "{name}"
            );
        }
    }

    #[test]
    fn skips_a_seek_table_whose_length_is_not_a_multiple_of_18() {
        for length in [1_u32, 17, 19, 35] {
            let mut body = Bytes::new();
            body.zeros(usize::try_from(length).unwrap());
            let table = file(vec![Built::Raw {
                code: 3,
                body: body.into_vec(),
            }]);
            assert_eq!(
                parse(&table),
                Ok(FlacMetadata {
                    problems: vec![BlockProblem::SeekTableLength { block: 42, length }],
                    ..metadata(46 + u64::from(length))
                }),
                "{length} octets"
            );
        }
    }

    #[test]
    fn keeps_the_first_seek_table_and_comment_and_skips_any_other() {
        let twice = file(vec![
            Built::VorbisComment(vec![0xAA]),
            Built::SeekTable(vec![point(7, 8, 9)]),
            Built::SeekTable(vec![point(1, 2, 3)]),
            Built::VorbisComment(vec![0xBB]),
        ]);
        assert_eq!(
            parse(&twice),
            Ok(FlacMetadata {
                seek_table: vec![SeekPoint {
                    sample: 7,
                    offset: 8,
                    samples: 9,
                }],
                comment: Some(46..47),
                problems: vec![
                    BlockProblem::Duplicate {
                        block: 69,
                        block_type: BlockType::SeekTable,
                    },
                    BlockProblem::Duplicate {
                        block: 91,
                        block_type: BlockType::VorbisComment,
                    },
                ],
                ..metadata(96)
            })
        );
        // A seek table skipped for its length still counts as the one.
        let skipped_first = file(vec![
            Built::Raw {
                code: 3,
                body: vec![0],
            },
            Built::SeekTable(vec![point(1, 2, 3)]),
        ]);
        assert_eq!(
            parse(&skipped_first),
            Ok(FlacMetadata {
                problems: vec![
                    BlockProblem::SeekTableLength {
                        block: 42,
                        length: 1,
                    },
                    BlockProblem::Duplicate {
                        block: 47,
                        block_type: BlockType::SeekTable,
                    },
                ],
                ..metadata(69)
            })
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn skips_a_seek_table_with_more_points_than_the_index_may_keep() {
        let table = file(vec![Built::SeekTable(vec![point(1, 0, 1), point(2, 0, 1)])]);
        assert_eq!(
            parse_with(&table, 0, &lowered(LimitKind::IndexEntries, 1)),
            Ok(FlacMetadata {
                problems: vec![BlockProblem::Fault {
                    block: 42,
                    fault: exceeded(LimitKind::IndexEntries, 2, 1, 46),
                }],
                ..metadata(82)
            })
        );
        assert_eq!(
            parse_with(&table, 0, &lowered(LimitKind::IndexEntries, 2))
                .map(|found| found.seek_table.len()),
            Ok(2)
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn allows_16_pictures_and_skips_the_next_one() {
        let small = || {
            Built::Picture(build::Picture {
                picture_type: 0,
                media_type: b"a".to_vec(),
                description: vec![],
                width: 0,
                height: 0,
                depth: 0,
                colors: 0,
                data: vec![],
            })
        };
        // Each picture block is 37 octets, its empty data 33 octets into
        // its body.
        let pictures = file((0..17).map(|_| small()).collect());
        let kept = (0..16_u64)
            .map(|index| PictureRef {
                picture_type: 0,
                media_type: text("a"),
                description: text(""),
                width: 0,
                height: 0,
                depth: 0,
                colors: 0,
                data: 79 + 37 * index..79 + 37 * index,
            })
            .collect();
        assert_eq!(
            parse(&pictures),
            Ok(FlacMetadata {
                pictures: kept,
                problems: vec![BlockProblem::Fault {
                    block: 634,
                    fault: exceeded(LimitKind::Pictures, 17, 16, 634),
                }],
                ..metadata(671)
            })
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn skips_a_picture_larger_than_the_picture_limit() {
        let picture = file(vec![Built::Picture(cover())]);
        assert_eq!(
            parse_with(&picture, 0, &lowered(LimitKind::PictureBytes, 3)),
            Ok(FlacMetadata {
                problems: vec![BlockProblem::Fault {
                    block: 42,
                    fault: exceeded(LimitKind::PictureBytes, 4, 3, 92),
                }],
                ..metadata(96)
            })
        );
        assert_eq!(
            parse_with(&picture, 0, &lowered(LimitKind::PictureBytes, 4)),
            Ok(FlacMetadata {
                pictures: vec![cover_ref(46)],
                ..metadata(96)
            })
        );
    }

    /// Each length inside a picture is checked against the block, and a
    /// picture shorter than its fixed fields is skipped.
    ///
    /// Verifies: SEC-MED-008, SEC-TM-032
    #[test]
    fn skips_a_picture_whose_fields_run_past_its_block() {
        let mut short = Bytes::new();
        short.zeros(7);
        let mut media_type = Bytes::new();
        media_type.u32_be(3).u32_be(1_000).bytes(b"ab");
        let mut description = Bytes::new();
        description
            .u32_be(3)
            .u32_be(1)
            .bytes(b"a")
            .u32_be(500)
            .bytes(b"xyz");
        let mut data = cover().body();
        data.truncate(42);
        data.extend([0x00, 0x00, 0x00, 0x0A, 0x89, b'P', b'N', b'G']);
        let cases = [
            (short.into_vec(), truncated(46, 8, 7)),
            (media_type.into_vec(), truncated(54, 1_004, 2)),
            (description.into_vec(), truncated(59, 520, 3)),
            (data, truncated(92, 10, 4)),
        ];
        for (body, fault) in cases {
            let end = 46 + length(&body);
            let picture = file(vec![Built::Raw { code: 6, body }]);
            assert_eq!(
                parse(&picture),
                Ok(FlacMetadata {
                    problems: vec![BlockProblem::Fault { block: 42, fault }],
                    ..metadata(end)
                }),
                "{fault:?}"
            );
        }
    }

    /// Verifies: SEC-MED-016
    #[test]
    fn skips_a_picture_given_as_a_link() {
        let link = build::Picture {
            media_type: b"-->".to_vec(),
            data: b"https://example.com/cover.png".to_vec(),
            ..cover()
        };
        // The link's block is 4 + 69 octets, the cover's 4 + 50.
        let picture = file(vec![Built::Picture(link), Built::Picture(cover())]);
        assert_eq!(
            parse(&picture),
            Ok(FlacMetadata {
                pictures: vec![cover_ref(119)],
                problems: vec![BlockProblem::LinkedPicture { block: 42 }],
                ..metadata(169)
            })
        );
    }

    /// Verifies: SEC-MED-006, SEC-MED-013
    #[test]
    fn decodes_picture_text_lossily_and_caps_it() {
        let labelled = build::Picture {
            media_type: b"image/\x1Bpng\n".to_vec(),
            description: b"one\ttwo\nthree\x07\xFF".to_vec(),
            ..cover()
        };
        let picture = file(vec![Built::Picture(labelled)]);
        let data = 46 + 4 + 4 + 11 + 4 + 15 + 16 + 4;
        let expected = |media_type: Text, description: Text| {
            Ok(FlacMetadata {
                pictures: vec![PictureRef {
                    media_type,
                    description,
                    data: data..data + 4,
                    ..cover_ref(46)
                }],
                ..metadata(data + 4)
            })
        };
        assert_eq!(
            parse(&picture),
            expected(
                text("image/png"),
                Text {
                    value: "one\ttwo\nthree\u{FFFD}".to_owned(),
                    truncated: false,
                    replaced: true,
                }
            )
        );
        let capped = lowered(LimitKind::ShortText, 5)
            .with_override(LimitKind::LongText, 3)
            .unwrap();
        assert_eq!(
            parse_with(&picture, 0, &capped),
            expected(
                Text {
                    value: "image".to_owned(),
                    truncated: true,
                    replaced: false,
                },
                Text {
                    value: "one".to_owned(),
                    truncated: true,
                    replaced: true,
                }
            )
        );
    }

    /// Verifies: SEC-TM-032
    #[test]
    fn never_asks_for_a_read_longer_than_the_read_limit() {
        assert_eq!(
            parse_with(&EXAMPLE_1, 0, &lowered(LimitKind::ReadBytes, 33)),
            Err(FlacError::Fault(exceeded(LimitKind::ReadBytes, 34, 33, 8)))
        );
        let table = file(vec![Built::SeekTable(vec![point(1, 0, 1), point(2, 0, 1)])]);
        assert_eq!(
            parse_with(&table, 0, &lowered(LimitKind::ReadBytes, 35)),
            Ok(FlacMetadata {
                problems: vec![BlockProblem::Fault {
                    block: 42,
                    fault: exceeded(LimitKind::ReadBytes, 36, 35, 46),
                }],
                ..metadata(82)
            })
        );
        assert_eq!(
            parse_with(&table, 0, &lowered(LimitKind::ReadBytes, 36))
                .map(|found| found.seek_table.len()),
            Ok(2)
        );
    }

    /// Verifies: SEC-TM-032
    #[test]
    fn never_reads_more_of_a_file_than_the_file_limit() {
        // The marker, a header and STREAMINFO are 42 octets.
        let header_after = file(vec![Built::Padding(0)]);
        assert_eq!(
            parse_with(&header_after, 0, &lowered(LimitKind::FileBytes, 45)),
            Err(FlacError::Fault(exceeded(LimitKind::FileBytes, 46, 45, 42)))
        );
        let table = file(vec![Built::SeekTable(vec![
            point(1, 0, 1),
            point(2, 0, 1),
            point(3, 0, 1),
        ])]);
        assert_eq!(
            parse_with(&table, 0, &lowered(LimitKind::FileBytes, 99)),
            Ok(FlacMetadata {
                problems: vec![BlockProblem::Fault {
                    block: 42,
                    fault: exceeded(LimitKind::FileBytes, 100, 99, 46),
                }],
                ..metadata(100)
            })
        );
        assert_eq!(
            parse_with(&table, 0, &lowered(LimitKind::FileBytes, 100))
                .map(|found| found.seek_table.len()),
            Ok(3)
        );
    }

    #[test]
    fn refuses_a_window_other_than_the_one_it_asked_for() {
        let mut budget = Budget::for_input(57, 1, 0);
        let mut parser = parse_metadata(0, &Limits::DEFAULT, &mut budget);
        assert_eq!(
            parser.resume(Window {
                offset: 0,
                bytes: b"f",
                file_len: 57,
            }),
            Step::Done(Err(FlacError::WrongWindow {
                expected: START,
                offset: 0,
                len: 1,
            }))
        );
        let marker = ReadRequest { offset: 0, len: 4 };
        assert_eq!(parser.resume(Window::start(57)), Step::Need(marker));
        assert_eq!(
            parser.resume(Window {
                offset: 1,
                bytes: b"LaC\x80",
                file_len: 57,
            }),
            Step::Done(Err(FlacError::WrongWindow {
                expected: marker,
                offset: 1,
                len: 4,
            }))
        );
        assert_eq!(parser.resume(Window::start(57)), Step::Need(marker));
        assert_eq!(
            parser.resume(Window {
                offset: 0,
                bytes: b"fLa",
                file_len: 57,
            }),
            Step::Done(Err(FlacError::WrongWindow {
                expected: marker,
                offset: 0,
                len: 3,
            }))
        );
    }

    /// A finished parser starts again, with its count of octets read back
    /// at zero, so a file limit that one parse fits allows the next.
    #[test]
    fn starts_again_once_it_has_answered() {
        let mut budget = Budget::for_input(0, 0, 2);
        let limits = lowered(LimitKind::FileBytes, 42);
        let mut parser = parse_metadata(0, &limits, &mut budget);
        assert_eq!(serve(&mut parser, &EXAMPLE_1), Ok(metadata(42)));
        assert_eq!(serve(&mut parser, &EXAMPLE_1), Ok(metadata(42)));
        assert_eq!(
            serve(&mut parser, &EXAMPLE_1),
            Err(FlacError::Fault(ParseFault::BudgetExceeded { offset: 4 }))
        );
    }

    #[test]
    fn reports_where_each_kind_of_error_happened() {
        let marker = ReadRequest { offset: 13, len: 4 };
        let errors = [
            FlacError::Fault(truncated(3, 4, 0)),
            FlacError::NotFlac {
                offset: 5,
                found: *b"OggS",
            },
            FlacError::StreamInfoNotFirst {
                offset: 6,
                block_type: BlockType::Picture,
            },
            FlacError::SecondStreamInfo { offset: 7 },
            FlacError::StreamInfoLength {
                offset: 8,
                length: 33,
            },
            FlacError::StreamInfoValue {
                offset: 9,
                error: ValueError::OutOfRange {
                    field: Field::SampleRate,
                    value: 0,
                },
            },
            FlacError::ForbiddenBlockType { offset: 11 },
            FlacError::WrongWindow {
                expected: marker,
                offset: 12,
                len: 1,
            },
        ];
        assert_eq!(
            errors.map(|error| error.offset()),
            [3, 5, 6, 7, 8, 9, 11, 13]
        );
    }

    /// Any STREAMINFO the builder can write with values in their typed
    /// ranges.
    fn any_info() -> impl Strategy<Value = build::StreamInfo> {
        (
            (any::<u16>(), any::<u16>(), 0_u32..1 << 24, 0_u32..1 << 24),
            (1_u32..=768_000, 1_u8..=8, 1_u8..=32),
            prop_oneof![Just(0), 0_u64..1 << 36],
            prop_oneof![Just([0; 16]), any::<[u8; 16]>()],
        )
            .prop_map(
                |(
                    (min_block_size, max_block_size, min_frame_size, max_frame_size),
                    (sample_rate, channels, bits_per_sample),
                    total_samples,
                    md5,
                )| build::StreamInfo {
                    min_block_size,
                    max_block_size,
                    min_frame_size,
                    max_frame_size,
                    sample_rate,
                    channels,
                    bits_per_sample,
                    total_samples,
                    md5,
                },
            )
    }

    /// Printable ASCII, which both text decoders keep as it is.
    fn printable(max: usize) -> impl Strategy<Value = Vec<u8>> {
        vec(0x20_u8..=0x7E, 0..=max)
    }

    /// A seek table in order: real points, then placeholders.
    fn any_seek_table() -> impl Strategy<Value = Vec<build::SeekPoint>> {
        (
            btree_set(0..build::PLACEHOLDER, 0..4),
            vec((any::<u64>(), any::<u16>()), 4),
            0_usize..3,
        )
            .prop_map(|(samples, fields, placeholders)| {
                samples
                    .into_iter()
                    .zip(fields)
                    .map(|(sample, (offset, samples))| point(sample, offset, samples))
                    .chain((0..placeholders).map(|_| placeholder()))
                    .collect()
            })
    }

    fn any_picture() -> impl Strategy<Value = build::Picture> {
        (
            any::<u32>(),
            prop_oneof![printable(12), Just(b"-->".to_vec())],
            printable(16),
            any::<[u32; 4]>(),
            vec(any::<u8>(), 0..8),
        )
            .prop_map(
                |(picture_type, media_type, description, [width, height, depth, colors], data)| {
                    build::Picture {
                        picture_type,
                        media_type,
                        description,
                        width,
                        height,
                        depth,
                        colors,
                        data,
                    }
                },
            )
    }

    /// Any block with a valid body, now and then a second STREAMINFO.
    fn any_block() -> impl Strategy<Value = Built> {
        prop_oneof![
            4 => (0_usize..8).prop_map(Built::Padding),
            4 => (any::<[u8; 4]>(), vec(any::<u8>(), 0..8))
                .prop_map(|(id, data)| Built::Application { id, data }),
            4 => any_seek_table().prop_map(Built::SeekTable),
            4 => vec(any::<u8>(), 0..16).prop_map(Built::VorbisComment),
            4 => vec(any::<u8>(), 0..16).prop_map(Built::CueSheet),
            4 => any_picture().prop_map(Built::Picture),
            4 => (7_u8..=126, vec(any::<u8>(), 0..8))
                .prop_map(|(code, body)| Built::Raw { code, body }),
            1 => any_info().prop_map(Built::StreamInfo),
        ]
    }

    /// A valid stream: how far into the file it starts, its STREAMINFO and
    /// the blocks after it.
    fn any_stream() -> impl Strategy<Value = (u64, build::StreamInfo, Vec<Built>)> {
        (0_u64..16, any_info(), vec(any_block(), 0..6))
    }

    /// `start` zero octets, the stream and a frame.
    fn assemble(start: u64, info: &build::StreamInfo, rest: &[Built]) -> Vec<u8> {
        let mut prefix = Bytes::new();
        prefix.zeros(usize::try_from(start).unwrap());
        let blocks: Vec<Built> = once(Built::StreamInfo(info.clone()))
            .chain(rest.iter().cloned())
            .collect();
        [prefix.into_vec(), build::stream(&blocks), FRAME.to_vec()].concat()
    }

    /// What the parser should find in a stream [`assemble`] wrote, worked
    /// out from RFC 9639's layout alone: the metadata, or the error at a
    /// second STREAMINFO.
    fn expected(
        start: u64,
        info: &build::StreamInfo,
        rest: &[Built],
    ) -> Result<FlacMetadata, FlacError> {
        let mut found = FlacMetadata {
            stream_info: StreamInfo {
                min_block_size: info.min_block_size,
                max_block_size: info.max_block_size,
                min_frame_size: NonZeroU32::new(info.min_frame_size),
                max_frame_size: NonZeroU32::new(info.max_frame_size),
                sample_rate: SampleRate::new(info.sample_rate).unwrap(),
                channels: Channels::new(info.channels.into()).unwrap(),
                bits_per_sample: BitDepth::new(info.bits_per_sample.into()).unwrap(),
                total_samples: NonZeroU64::new(info.total_samples),
                md5: (info.md5 != [0; 16]).then_some(AudioMd5(info.md5)),
            },
            seek_table: vec![],
            comment: None,
            pictures: vec![],
            raw: vec![],
            problems: vec![],
            audio_start: 0,
        };
        let mut seek_table_seen = false;
        let mut header = start + 42;
        for block in rest {
            let body = header + 4;
            let end = body + length(&block.body());
            let raw = |block_type| RawBlock {
                block_type,
                body: body..end,
            };
            match block {
                Built::Padding(_) => found.raw.push(raw(BlockType::Padding)),
                Built::Application { .. } => found.raw.push(raw(BlockType::Application)),
                Built::CueSheet(_) => found.raw.push(raw(BlockType::CueSheet)),
                Built::Raw { code, .. } => found.raw.push(raw(BlockType::Reserved(*code))),
                Built::SeekTable(points) if !seek_table_seen => {
                    seek_table_seen = true;
                    found.seek_table = points
                        .iter()
                        .filter(|point| point.sample != build::PLACEHOLDER)
                        .map(|point| SeekPoint {
                            sample: point.sample,
                            offset: point.offset,
                            samples: point.samples,
                        })
                        .collect();
                }
                Built::SeekTable(_) => found.problems.push(BlockProblem::Duplicate {
                    block: header,
                    block_type: BlockType::SeekTable,
                }),
                Built::VorbisComment(_) if found.comment.is_none() => {
                    found.comment = Some(body..end);
                }
                Built::VorbisComment(_) => found.problems.push(BlockProblem::Duplicate {
                    block: header,
                    block_type: BlockType::VorbisComment,
                }),
                Built::Picture(picture) if picture.media_type == b"-->" => found
                    .problems
                    .push(BlockProblem::LinkedPicture { block: header }),
                Built::Picture(picture) => {
                    let ascii = |bytes: &[u8]| text(std::str::from_utf8(bytes).unwrap());
                    found.pictures.push(PictureRef {
                        picture_type: picture.picture_type,
                        media_type: ascii(&picture.media_type),
                        description: ascii(&picture.description),
                        width: picture.width,
                        height: picture.height,
                        depth: picture.depth,
                        colors: picture.colors,
                        data: end - length(&picture.data)..end,
                    });
                }
                Built::StreamInfo(_) => {
                    return Err(FlacError::SecondStreamInfo { offset: header });
                }
            }
            header = end;
        }
        found.audio_start = header;
        Ok(found)
    }

    /// Where each structure a parser must find whole lies, in order: the
    /// marker, then each block's header and body, up to a second
    /// STREAMINFO, past which the parser reads nothing.
    fn structures(start: u64, info: &build::StreamInfo, rest: &[Built]) -> Vec<(u64, u64)> {
        let mut found = vec![(start, 4)];
        let mut at = start + 4;
        let streaminfo_ends = rest.iter().take_while(|block| block.code() != 0).count() + 2;
        for block in once(&Built::StreamInfo(info.clone()))
            .chain(rest)
            .take(streaminfo_ends)
        {
            let len = length(&block.body());
            found.extend([(at, 4), (at + 4, len)]);
            at += 4 + len;
        }
        found
    }

    /// Bytes that are anything at all, a marker and anything, or a valid
    /// stream with a few octets changed.
    fn any_input() -> impl Strategy<Value = Vec<u8>> {
        prop_oneof![
            vec(any::<u8>(), 0..128),
            vec(any::<u8>(), 0..128).prop_map(|tail| [build::MARKER.to_vec(), tail].concat()),
            (any_stream(), vec((any::<Index>(), any::<u8>()), 1..4)).prop_map(
                |((_, info, rest), changes)| {
                    let mut file = assemble(0, &info, &rest);
                    for (at, octet) in changes {
                        let at = at.index(file.len());
                        file[at] = octet;
                    }
                    file
                }
            ),
        ]
    }

    proptest! {
        /// Verifies: SEC-MED-001, SEC-MED-007, SEC-MED-008, SEC-HIS-036
        #[test]
        fn returns_for_any_input_within_its_step_bound(bytes in any_input()) {
            let len = length(&bytes);
            let (result, left) = on_small_stack(move || {
                let mut budget = Budget::for_input(len, STEPS_PER_OCTET, STEPS_FIXED);
                let result = drive(
                    parse_metadata(0, &Limits::DEFAULT, &mut budget),
                    &bytes,
                    &Limits::DEFAULT,
                );
                (result, budget.remaining())
            });
            // One step per four octets at most.
            prop_assert!((len - left) * 4 <= len, "{} steps for {} octets", len - left, len);
            match result {
                Ok(Ok(found)) => prop_assert!(found.audio_start <= len),
                Ok(Err(error)) => prop_assert!(error.offset() <= len, "{:?}", error),
                Err(refused) => prop_assert!(false, "the host refused {:?}", refused),
            }
        }

        #[test]
        fn reads_any_stream_the_builder_writes((start, info, rest) in any_stream()) {
            let file = assemble(start, &info, &rest);
            prop_assert_eq!(
                parse_with(&file, start, &Limits::DEFAULT),
                expected(start, &info, &rest)
            );
        }

        /// Verifies: SEC-MED-001, SEC-TM-032
        #[test]
        fn reports_every_cut_at_the_structure_it_cuts(
            (start, info, rest) in any_stream(),
            cut in any::<Index>(),
        ) {
            let file = assemble(start, &info, &rest);
            let cut = cut.index(file.len() + 1);
            let kept = length(&file[..cut]);
            let wanted = structures(start, &info, &rest)
                .into_iter()
                .find(|&(offset, len)| offset + len > kept)
                .map_or_else(
                    || expected(start, &info, &rest),
                    |(offset, needed)| {
                        Err(FlacError::Fault(truncated(offset, needed, kept.saturating_sub(offset))))
                    },
                );
            prop_assert_eq!(parse_with(&file[..cut], start, &Limits::DEFAULT), wanted);
        }
    }
}
