//! MPEG audio: frame headers, the encoder headers in a stream's first
//! frame, and a scan of the frames after it (ISO/IEC 11172-3, ISO/IEC
//! 13818-3 and Fraunhofer's MPEG 2.5 extension).
//!
//! An MP3 file is a run of frames, each a four-octet header and its
//! payload, often after one or more `ID3v2` tags and sometimes after junk.
//! [`frame_header`] decodes one header. [`stream_info`] is a sans-I/O
//! parser that skips the `ID3v2` tags, searches for the first frame and
//! confirms it with the header after it, reads the Xing, Info, LAME and
//! VBRI headers an encoder wrote in that frame, and builds a seek index.
//!
//! Layers I and II are detected and refused: R1 plays MP3, which is Layer
//! III. Free-format streams, whose headers do not give a bitrate, are
//! refused too.

use crate::parse::{
    Budget, Cursor, LimitKind, Limits, ParseFault, ReadRequest, SansIo, Step, Window,
};
use crate::values::{GainDb, PeakRatio, SampleRate, ValueError};

/// The MPEG audio version a frame header declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    /// MPEG-1 (ISO/IEC 11172-3): 32, 44.1 and 48 kHz.
    Mpeg1,
    /// MPEG-2 (ISO/IEC 13818-3): 16, 22.05 and 24 kHz.
    Mpeg2,
    /// MPEG 2.5, Fraunhofer's extension: 8, 11.025 and 12 kHz.
    Mpeg25,
}

/// The layer a frame header declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    /// Layer I.
    I,
    /// Layer II.
    II,
    /// Layer III, the layer of MP3 files.
    III,
}

/// The channel mode a frame header declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelMode {
    /// Two independent channels.
    Stereo,
    /// Two channels coded together.
    JointStereo,
    /// Two unrelated mono channels.
    DualChannel,
    /// One channel.
    Mono,
}

/// One frame header, decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHeader {
    /// The version.
    pub version: Version,
    /// The layer.
    pub layer: Layer,
    /// Whether a CRC follows the header.
    pub crc: bool,
    /// The bitrate in kbit/s.
    pub bitrate: u16,
    /// The sampling frequency.
    pub sample_rate: SampleRate,
    /// Whether the frame carries a padding slot.
    pub padding: bool,
    /// The channel mode.
    pub mode: ChannelMode,
    /// The samples the frame holds per channel.
    pub samples: u16,
    /// The frame's length in octets, header included.
    pub len: u16,
}

/// Why four octets are not a frame header this parser accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderError {
    /// The eleven sync bits were not all set.
    NoSync,
    /// The version bits were `01`, which no standard assigns.
    ReservedVersion,
    /// The layer bits were `00`, which no standard assigns.
    ReservedLayer,
    /// Bitrate index 0: free format, whose bitrate and so frame length the
    /// header does not give. Refused.
    FreeFormat,
    /// Bitrate index 15, which the standards forbid.
    ForbiddenBitrate,
    /// Sampling-frequency index 3, which no standard assigns.
    ReservedSampleRate,
}

/// Decodes the frame header in `octets`.
///
/// # Errors
///
/// Returns the [`HeaderError`] for the first field that is not valid, in
/// the order the header holds them: sync, version, layer, bitrate,
/// sampling frequency.
pub fn frame_header(octets: [u8; 4]) -> Result<FrameHeader, HeaderError> {
    let [first, second, third, fourth] = octets;
    // The sync is the first octet and the top three bits of the second.
    if first != 0xFF || second < 0xE0 {
        return Err(HeaderError::NoSync);
    }
    let version = match (second >> 3) & 0b11 {
        0b11 => Version::Mpeg1,
        0b10 => Version::Mpeg2,
        0b00 => Version::Mpeg25,
        _ => return Err(HeaderError::ReservedVersion),
    };
    let layer = match (second >> 1) & 0b11 {
        0b11 => Layer::I,
        0b10 => Layer::II,
        0b01 => Layer::III,
        _ => return Err(HeaderError::ReservedLayer),
    };
    let bitrate = bitrate(version, layer, third >> 4)?;
    let sample_rate = sample_rate(version, (third >> 2) & 0b11)?;
    let padding = third & 0b10 != 0;
    let mode = match fourth >> 6 {
        0b00 => ChannelMode::Stereo,
        0b01 => ChannelMode::JointStereo,
        0b10 => ChannelMode::DualChannel,
        _ => ChannelMode::Mono,
    };
    let samples = match (layer, version) {
        (Layer::I, _) => 384,
        (Layer::II, _) | (Layer::III, Version::Mpeg1) => 1_152,
        (Layer::III, Version::Mpeg2 | Version::Mpeg25) => 576,
    };
    Ok(FrameHeader {
        version,
        layer,
        crc: second & 1 == 0,
        bitrate,
        sample_rate,
        padding,
        mode,
        samples,
        len: frame_len(layer, version, bitrate, sample_rate, padding),
    })
}

/// Bitrates in kbit/s for indices 1 to 14 (ISO/IEC 11172-3 and ISO/IEC
/// 13818-3, table 2.4.2.3 of each). MPEG 2.5 uses MPEG-2's.
const MPEG1_LAYER1: [u16; 14] = [
    32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448,
];
/// See [`MPEG1_LAYER1`].
const MPEG1_LAYER2: [u16; 14] = [
    32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384,
];
/// See [`MPEG1_LAYER1`].
const MPEG1_LAYER3: [u16; 14] = [
    32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320,
];
/// See [`MPEG1_LAYER1`].
const MPEG2_LAYER1: [u16; 14] = [
    32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256,
];
/// See [`MPEG1_LAYER1`].
const MPEG2_LAYERS2_3: [u16; 14] = [8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160];

/// The bitrate at `index` for `version` and `layer`.
fn bitrate(version: Version, layer: Layer, index: u8) -> Result<u16, HeaderError> {
    let table = match (version, layer) {
        (Version::Mpeg1, Layer::I) => &MPEG1_LAYER1,
        (Version::Mpeg1, Layer::II) => &MPEG1_LAYER2,
        (Version::Mpeg1, Layer::III) => &MPEG1_LAYER3,
        (Version::Mpeg2 | Version::Mpeg25, Layer::I) => &MPEG2_LAYER1,
        (Version::Mpeg2 | Version::Mpeg25, Layer::II | Layer::III) => &MPEG2_LAYERS2_3,
    };
    // Index 0 is free format; index 15 lies past the table's end.
    let slot = index.checked_sub(1).ok_or(HeaderError::FreeFormat)?;
    table
        .get(usize::from(slot))
        .copied()
        .ok_or(HeaderError::ForbiddenBitrate)
}

/// The sampling frequency at `index` for `version`.
fn sample_rate(version: Version, index: u8) -> Result<SampleRate, HeaderError> {
    let table: [u32; 3] = match version {
        Version::Mpeg1 => [44_100, 48_000, 32_000],
        Version::Mpeg2 => [22_050, 24_000, 16_000],
        Version::Mpeg25 => [11_025, 12_000, 8_000],
    };
    // Every frequency in the tables is a valid sample rate; index 3 lies
    // past their end.
    table
        .get(usize::from(index))
        .and_then(|&hz| SampleRate::new(hz).ok())
        .ok_or(HeaderError::ReservedSampleRate)
}

/// A frame's length in octets (ISO/IEC 11172-3 and ISO/IEC 13818-3, section
/// 2.4.3.1 of each): the slots the bitrate fills in the frame's duration,
/// rounded down, plus the padding slot. Layer I counts in four-octet slots.
fn frame_len(
    layer: Layer,
    version: Version,
    kbps: u16,
    sample_rate: SampleRate,
    padding: bool,
) -> u16 {
    // Octets a frame holds per bit/s of bitrate, times the frequency: a
    // twelfth of Layer I's 384 samples, an eighth of 1,152 or 576.
    let (slots_per_bit, slot): (u32, u32) = match (layer, version) {
        (Layer::I, _) => (12, 4),
        (Layer::II, _) | (Layer::III, Version::Mpeg1) => (144, 1),
        (Layer::III, Version::Mpeg2 | Version::Mpeg25) => (72, 1),
    };
    // At most 144 * 448,000 / 8,000 + 1 slots, so nothing saturates and
    // the length fits 16 bits.
    let bits_per_second = u32::from(kbps).saturating_mul(1_000);
    let slots = (slots_per_bit.saturating_mul(bits_per_second) / sample_rate.hz())
        .saturating_add(u32::from(padding));
    u16::try_from(slots.saturating_mul(slot)).unwrap_or(u16::MAX)
}

/// A Xing header, or the Info header LAME writes in its place for a
/// constant bitrate. Each field is present when the header's flags say so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Xing {
    /// Whether the header was named `Info` rather than `Xing`.
    pub info: bool,
    /// The number of audio frames, not counting the frame that holds this
    /// header.
    pub frames: Option<u32>,
    /// The length of the stream in octets, counting the frame that holds
    /// this header.
    pub bytes: Option<u32>,
    /// The table of contents: entry `i` is where `i`% of the duration
    /// starts, as a fraction of [`Xing::bytes`] in 256ths.
    pub toc: Option<[u8; 100]>,
    /// The encoder's quality indicator.
    pub quality: Option<u32>,
}

/// The extension LAME and `FFmpeg` (as `Lavf` and `Lavc`) write after a
/// Xing or Info header.
#[derive(Debug, Clone, PartialEq)]
pub struct LameTag {
    /// The encoder's short version string, such as `LAME3.100`, as raw
    /// octets.
    pub encoder: [u8; 9],
    /// Samples the encoder added before the audio (MUS-069).
    pub delay: u16,
    /// Samples the encoder added after the audio (MUS-069).
    pub padding: u16,
    /// The octets of the stream from the frame that holds the tag to the end
    /// of the last frame, as the encoder counted them.
    pub music_length: u32,
    /// The `ReplayGain` fields, or, when the tag's CRC does not match, the
    /// CRCs that disagreed. The fields are dropped then, because nothing
    /// shows they are what the encoder wrote.
    pub replay_gain: Result<ReplayGain, CrcMismatch>,
}

/// The `ReplayGain` fields of a [`LameTag`] (MUS-084).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReplayGain {
    /// The peak sample, when the encoder measured one, or why it was
    /// dropped.
    pub peak: Option<Result<PeakRatio, ValueError>>,
    /// The track (radio) gain, when the tag holds one.
    pub track: Option<GainDb>,
    /// The album (audiophile) gain, when the tag holds one.
    pub album: Option<GainDb>,
}

/// A LAME tag CRC that does not match the octets it covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrcMismatch {
    /// The CRC the tag holds.
    pub stored: u16,
    /// The CRC of the octets it covers.
    pub computed: u16,
}

/// A VBRI header, which Fraunhofer's encoders write in a stream's first
/// frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vbri {
    /// The header's version.
    pub version: u16,
    /// The encoder's quality indicator.
    pub quality: u16,
    /// The length of the stream in octets.
    pub bytes: u32,
    /// The number of frames.
    pub frames: u32,
}

/// One point of a [`SeekIndex`]: to play from `sample`, start reading at
/// `offset`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeekPoint {
    /// Samples per channel from the start of the audio.
    pub sample: u64,
    /// Octets from the start of the file.
    pub offset: u64,
}

/// Where to start reading to play from a given time (MUS-071).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeekIndex {
    /// The points, in order of sample and of offset.
    pub points: Vec<SeekPoint>,
    /// How many frames, or table of contents entries, each point stands
    /// for. It is 1 unless the index was thinned to the index-entry limit
    /// (SEC-MED-006).
    pub stride: u64,
}

/// An MPEG audio stream, as [`stream_info`] found it.
#[derive(Debug, Clone, PartialEq)]
pub struct MpegStream {
    /// Where the first frame starts.
    pub start: u64,
    /// The first frame's header.
    pub header: FrameHeader,
    /// The Xing or Info header in the first frame, or why it could not be
    /// read.
    pub xing: Option<Result<Xing, ParseFault>>,
    /// The LAME extension after the Xing or Info header, or why it could
    /// not be read.
    pub lame: Option<Result<LameTag, ParseFault>>,
    /// The VBRI header in the first frame, or why it could not be read.
    pub vbri: Option<Result<Vbri, ParseFault>>,
    /// The number of audio frames. A first frame that holds a Xing, Info or
    /// VBRI header holds no audio and does not count. When the seek index
    /// comes from the Xing table of contents, this is the header's count;
    /// otherwise it is the frames the scan found.
    pub frames: u64,
    /// Where the audio ends: after the last whole frame the scan found, or
    /// where the Xing header's byte count ends.
    pub end: u64,
    /// The seek index.
    pub seek: SeekIndex,
}

/// Why [`stream_info`] found no stream it accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MpaError {
    /// A failure every parser shares: an `ID3v2` tag or the only frame
    /// candidate running past the end of the file, or the step budget
    /// running out.
    Fault(ParseFault),
    /// No MPEG audio frame was found at or after `offset`.
    NoFrames {
        /// Where the search started, after any `ID3v2` tags.
        offset: u64,
    },
    /// No stream was found, and the first header the search met declared
    /// free format.
    FreeFormat {
        /// Where that header starts.
        offset: u64,
    },
    /// The stream is Layer I or Layer II.
    UnsupportedLayer {
        /// Where its first frame starts.
        offset: u64,
        /// Its layer.
        layer: Layer,
    },
}

impl From<ParseFault> for MpaError {
    fn from(fault: ParseFault) -> Self {
        Self::Fault(fault)
    }
}

/// The steps [`stream_info`] may charge per octet of the file. With
/// [`FIXED_STEPS`], `Budget::for_input(file_len, STEPS_PER_OCTET,
/// FIXED_STEPS)` is always enough (SEC-MED-007).
///
/// The parser charges one step for every read it is resumed with, one for
/// every octet it examines while it searches for the first frame, and one
/// for every table of contents entry. It examines each octet at most once,
/// each read examines at least one octet or skips a tag of at least ten,
/// each confirming read follows an examined octet, and each frame it scans
/// is at least 24 octets long, so it charges at most about 3.2 steps an
/// octet, plus the table's 100 and the first frame's read.
pub const STEPS_PER_OCTET: u64 = 4;

/// The steps [`stream_info`] may charge beyond [`STEPS_PER_OCTET`].
pub const FIXED_STEPS: u64 = 128;

/// How many octets one search read asks for, at most.
const SEARCH_WINDOW: u64 = 4_096;

/// The octets of a frame header.
const HEADER: u64 = 4;

/// The octets of the smallest frame any header describes: MPEG-2 Layer III
/// at 8 kbit/s and 24 kHz.
const SMALLEST_FRAME: u64 = 24;

/// A parser of one MPEG audio file, resumed through [`SansIo`]. Made by
/// [`stream_info`].
///
/// Resumed again after it has returned its result, it starts over on the
/// file the window describes, with what is left of its budget.
#[derive(Debug)]
pub struct StreamInfo {
    /// The limits of the parse.
    limits: Limits,
    /// The steps left.
    budget: Budget,
    /// The length of the file.
    file_len: u64,
    /// What the parser asked for last.
    state: State,
    /// What came closest to a stream while the search found none: the
    /// first free-format header, or the first frame that ran past the end
    /// of the file.
    near_miss: Option<MpaError>,
}

/// What a [`StreamInfo`] asked for last, and why.
#[derive(Debug)]
enum State {
    /// Nothing yet: the next window is the first of the parse.
    Start,
    /// Octets from `at`, to look for an `ID3v2` tag (when `at` is `from`)
    /// and frame headers. `from` is where the search started.
    Search {
        /// Where the window starts.
        at: u64,
        /// Where the search started, after any tags.
        from: u64,
    },
    /// The header after the frame at `candidate`, to confirm it.
    Confirm {
        /// Where the candidate frame starts.
        candidate: u64,
        /// Its header.
        header: FrameHeader,
        /// Where the search started.
        from: u64,
    },
    /// The whole first frame, to read the encoder headers in it.
    First {
        /// Where it starts.
        start: u64,
        /// Its header.
        header: FrameHeader,
    },
    /// The header of the frame at `at`, to scan it.
    Scan {
        /// Where the frame starts.
        at: u64,
        /// The stream so far.
        stream: Box<Scanning>,
    },
}

/// What a resume leads to.
enum Next {
    /// Another read, and the state to resume in.
    Need(State, ReadRequest),
    /// The stream.
    Done(Box<MpegStream>),
}

/// A stream whose frames are being scanned.
#[derive(Debug)]
struct Scanning {
    /// Where the first frame starts.
    start: u64,
    /// The first frame's header.
    header: FrameHeader,
    /// The encoder headers in the first frame.
    encoder: Encoder,
    /// Audio frames found so far.
    frames: u64,
    /// The seek index so far.
    seek: Thinner,
}

impl Scanning {
    /// Counts the audio frame at `offset`.
    fn add(&mut self, offset: u64) {
        let sample = self.frames.saturating_mul(u64::from(self.header.samples));
        self.seek.offer(self.frames, SeekPoint { sample, offset });
        self.frames = self.frames.saturating_add(1);
    }

    /// The stream, whose audio ends at `end`.
    fn finish(self, end: u64) -> Box<MpegStream> {
        let Encoder { xing, lame, vbri } = self.encoder;
        Box::new(MpegStream {
            start: self.start,
            header: self.header,
            xing,
            lame,
            vbri,
            frames: self.frames,
            end,
            seek: self.seek.finish(),
        })
    }
}

/// Starts parsing an MPEG audio file under `limits`, spending at most
/// `budget`.
#[must_use]
pub fn stream_info(limits: &Limits, budget: Budget) -> StreamInfo {
    StreamInfo {
        limits: *limits,
        budget,
        file_len: 0,
        state: State::Start,
        near_miss: None,
    }
}

impl SansIo for StreamInfo {
    type Output = Result<MpegStream, MpaError>;

    fn resume(&mut self, window: Window<'_>) -> Step<Self::Output> {
        let next = match std::mem::replace(&mut self.state, State::Start) {
            State::Start => self.start(window.file_len),
            State::Search { at, from } => self.searched(at, from, window.bytes),
            State::Confirm {
                candidate,
                header,
                from,
            } => self.confirmed(candidate, header, from, window.bytes),
            State::First { start, header } => self.first(start, header, window.bytes),
            State::Scan { at, stream } => self.scanned(at, *stream, window.bytes),
        };
        match next {
            Ok(Next::Need(state, request)) => {
                self.state = state;
                Step::Need(request)
            }
            Ok(Next::Done(stream)) => Step::Done(Ok(*stream)),
            Err(error) => Step::Done(Err(error)),
        }
    }
}

impl StreamInfo {
    /// Begins a parse of a file of `file_len` octets.
    fn start(&mut self, file_len: u64) -> Result<Next, MpaError> {
        self.file_len = file_len;
        self.near_miss = None;
        self.search(0, 0)
    }

    /// Asks for the octets from `at`, to search them; or, when fewer than a
    /// header's are left, ends the search that started at `from`.
    fn search(&mut self, at: u64, from: u64) -> Result<Next, MpaError> {
        let left = self.file_len.saturating_sub(at);
        if left < HEADER {
            return Err(self
                .near_miss
                .take()
                .unwrap_or(MpaError::NoFrames { offset: from }));
        }
        let len = left
            .min(SEARCH_WINDOW)
            .min(self.limits.get(LimitKind::ReadBytes));
        Ok(Next::Need(State::Search { at, from }, read(at, len)))
    }

    /// Skips the `ID3v2` tag that opens `octets`, from `at`, if the search
    /// has just started; otherwise examines every octet for a frame header.
    fn searched(&mut self, at: u64, from: u64, octets: &[u8]) -> Result<Next, MpaError> {
        self.budget.charge(1, at)?;
        if at == from
            && let Some(tag) = id3v2_len(octets)
        {
            let available = self.file_len.saturating_sub(at);
            if tag > available {
                return Err(ParseFault::Truncated {
                    offset: at,
                    needed: tag,
                    available,
                }
                .into());
            }
            let end = at.saturating_add(tag);
            return self.search(end, end);
        }
        let mut offset = at;
        let mut rest = octets;
        while let Some((&header, _)) = rest.split_first_chunk::<4>() {
            self.budget.charge(1, offset)?;
            match frame_header(header) {
                Ok(header) => {
                    if let Some(next) = self.candidate(offset, header, from)? {
                        return Ok(next);
                    }
                }
                Err(HeaderError::FreeFormat) => self.miss(MpaError::FreeFormat { offset }),
                Err(_) => {}
            }
            rest = rest.get(1..).unwrap_or_default();
            offset = offset.saturating_add(1);
        }
        // The last three octets may start a header that the next window
        // holds whole. Every window moves the search on by at least one.
        let end = at.saturating_add(len(octets));
        self.search(
            end.saturating_sub(HEADER - 1).max(at.saturating_add(1)),
            from,
        )
    }

    /// Weighs the frame header found at `offset`: refuses a frame that runs
    /// past the end of the file, accepts one with no room for a header
    /// after it, and asks for the header after any other.
    fn candidate(
        &mut self,
        offset: u64,
        header: FrameHeader,
        from: u64,
    ) -> Result<Option<Next>, MpaError> {
        let frame = u64::from(header.len);
        let available = self.file_len.saturating_sub(offset);
        if frame > available {
            self.miss(
                ParseFault::Truncated {
                    offset,
                    needed: frame,
                    available,
                }
                .into(),
            );
            return Ok(None);
        }
        if available.saturating_sub(frame) < HEADER {
            return accept(offset, header).map(Some);
        }
        Ok(Some(Next::Need(
            State::Confirm {
                candidate: offset,
                header,
                from,
            },
            read(offset.saturating_add(frame), HEADER),
        )))
    }

    /// Notes `error` as what came closest to a stream, unless something
    /// came first.
    fn miss(&mut self, error: MpaError) {
        self.near_miss.get_or_insert(error);
    }

    /// Accepts the frame at `candidate` when the header in `octets`, which
    /// follows it, belongs to the same stream; otherwise searches on from
    /// the octet after it.
    fn confirmed(
        &mut self,
        candidate: u64,
        header: FrameHeader,
        from: u64,
        octets: &[u8],
    ) -> Result<Next, MpaError> {
        self.budget
            .charge(1, candidate.saturating_add(u64::from(header.len)))?;
        let next = octets
            .first_chunk::<4>()
            .and_then(|&next| frame_header(next).ok());
        if next.is_some_and(|next| same_stream(&header, &next)) {
            accept(candidate, header)
        } else {
            self.search(candidate.saturating_add(1), from)
        }
    }

    /// Reads the encoder headers in the first frame, `octets`, then builds
    /// the seek index from the Xing table of contents, or else scans.
    fn first(&mut self, start: u64, header: FrameHeader, octets: &[u8]) -> Result<Next, MpaError> {
        self.budget.charge(1, start)?;
        let encoder = Encoder::read(octets, start, &header);
        let mut seek = Thinner::new(self.limits.get(LimitKind::IndexEntries));
        if let Some(Ok(xing)) = &encoder.xing
            && let Some((frames, bytes, toc)) =
                table_of_contents(xing, &header, start, self.file_len)
        {
            let duration = u64::from(frames).saturating_mul(u64::from(header.samples));
            let mut floor = start;
            for (entry, &fraction) in (0_u64..).zip(toc) {
                self.budget.charge(1, start)?;
                // Entry i of the table is where i% of the duration starts,
                // in 256ths of the byte count. An entry below the one before
                // it is raised to it, so the index stays in order.
                let offset = start
                    .saturating_add(u64::from(fraction).saturating_mul(u64::from(bytes)) / 256)
                    .max(floor);
                floor = offset;
                let sample = duration.saturating_mul(entry) / 100;
                seek.offer(entry, SeekPoint { sample, offset });
            }
            let Encoder { xing, lame, vbri } = encoder;
            return Ok(Next::Done(Box::new(MpegStream {
                start,
                header,
                xing,
                lame,
                vbri,
                frames: u64::from(frames),
                end: start.saturating_add(u64::from(bytes)),
                seek: seek.finish(),
            })));
        }
        // A frame that holds an encoder header holds no audio.
        let audio = encoder.xing.is_none() && encoder.vbri.is_none();
        let mut stream = Scanning {
            start,
            header,
            encoder,
            frames: 0,
            seek,
        };
        if audio {
            stream.add(start);
        }
        Ok(self.scan(start.saturating_add(u64::from(header.len)), stream))
    }

    /// Asks for the header at `at`, or ends the stream there when no frame
    /// fits in what is left.
    fn scan(&self, at: u64, stream: Scanning) -> Next {
        if self.file_len.saturating_sub(at) < SMALLEST_FRAME {
            return Next::Done(stream.finish(at));
        }
        Next::Need(
            State::Scan {
                at,
                stream: Box::new(stream),
            },
            read(at, HEADER),
        )
    }

    /// Counts the frame at `at` and moves past it when `octets` hold a
    /// header of the same stream whose frame ends inside the file;
    /// otherwise the stream ends at `at`.
    fn scanned(&mut self, at: u64, mut stream: Scanning, octets: &[u8]) -> Result<Next, MpaError> {
        self.budget.charge(1, at)?;
        let frame = octets
            .first_chunk::<4>()
            .and_then(|&next| frame_header(next).ok())
            .filter(|next| same_stream(&stream.header, next))
            .map(|next| u64::from(next.len))
            .filter(|&len| len <= self.file_len.saturating_sub(at));
        match frame {
            Some(len) => {
                stream.add(at);
                Ok(self.scan(at.saturating_add(len), stream))
            }
            None => Ok(Next::Done(stream.finish(at))),
        }
    }
}

/// Takes the frame at `start` as the stream's first, refusing Layers I and
/// II, and asks for all of it.
fn accept(start: u64, header: FrameHeader) -> Result<Next, MpaError> {
    if header.layer != Layer::III {
        return Err(MpaError::UnsupportedLayer {
            offset: start,
            layer: header.layer,
        });
    }
    Ok(Next::Need(
        State::First { start, header },
        read(start, u64::from(header.len)),
    ))
}

/// A request for `len` octets at `offset`. Every request is at most
/// [`SEARCH_WINDOW`] octets or one frame, so the length fits.
fn read(offset: u64, len: u64) -> ReadRequest {
    ReadRequest {
        offset,
        len: u32::try_from(len).unwrap_or(u32::MAX),
    }
}

/// How many octets `octets` holds.
fn len(octets: &[u8]) -> u64 {
    // A slice never holds more than u64::MAX octets.
    u64::try_from(octets.len()).unwrap_or(u64::MAX)
}

/// Whether two frame headers belong to one stream: the same version, layer
/// and sampling frequency, as decoders require. The bitrate may change
/// from frame to frame.
fn same_stream(first: &FrameHeader, next: &FrameHeader) -> bool {
    first.version == next.version
        && first.layer == next.layer
        && first.sample_rate == next.sample_rate
}

/// The length of the `ID3v2` tag whose header opens `octets`, footer
/// included, or `None` when they do not open with one (`ID3v2.4.0`
/// structure, section 3.1: `ID3`, two version octets below 0xFF, flags and
/// a syncsafe size).
fn id3v2_len(octets: &[u8]) -> Option<u64> {
    let [i, d, three, major, revision, flags, size @ ..] = *octets.first_chunk::<10>()?;
    let size = Cursor::new(&size).syncsafe_u32().ok()?;
    let tag = [i, d, three] == *b"ID3" && major != 0xFF && revision != 0xFF;
    // A footer repeats the ten-octet header after the tag.
    let footer = if flags & 0x10 == 0 { 0 } else { 10 };
    tag.then(|| u64::from(size).saturating_add(10).saturating_add(footer))
}

/// The Xing table of contents, with the frame and byte counts that give it
/// meaning, when it can be used: there is at least one frame, and the
/// byte count covers the first frame and ends inside the file
/// (SEC-MED-008).
fn table_of_contents<'a>(
    xing: &'a Xing,
    header: &FrameHeader,
    start: u64,
    file_len: u64,
) -> Option<(u32, u32, &'a [u8; 100])> {
    let (Some(frames), Some(bytes), Some(toc)) = (xing.frames, xing.bytes, xing.toc.as_ref())
    else {
        return None;
    };
    let inside = start
        .checked_add(u64::from(bytes))
        .is_some_and(|end| end <= file_len);
    (frames > 0 && bytes >= u32::from(header.len) && inside).then_some((frames, bytes, toc))
}

/// The encoder headers of a stream's first frame.
#[derive(Debug)]
struct Encoder {
    /// See [`MpegStream::xing`].
    xing: Option<Result<Xing, ParseFault>>,
    /// See [`MpegStream::lame`].
    lame: Option<Result<LameTag, ParseFault>>,
    /// See [`MpegStream::vbri`].
    vbri: Option<Result<Vbri, ParseFault>>,
}

impl Encoder {
    /// Reads the encoder headers in `frame`, the octets of the first
    /// frame, which starts at `start` and has `header`. A Xing or Info
    /// header starts where the side information ends, CRC or not, as LAME
    /// writes it and decoders read it; a VBRI header starts 32 octets after
    /// the frame header.
    fn read(frame: &[u8], start: u64, header: &FrameHeader) -> Self {
        let mut cursor = Cursor::at(frame, start);
        let name = cursor
            .skip(HEADER.saturating_add(side_info_len(header)))
            .and_then(|()| cursor.array::<4>());
        match name {
            Ok(name @ (XING | INFO)) => {
                let xing = read_xing(&mut cursor, name == INFO);
                let lame = xing
                    .is_ok()
                    .then(|| read_lame(&mut cursor, frame))
                    .flatten();
                Self {
                    xing: Some(xing),
                    lame,
                    vbri: None,
                }
            }
            _ => Self {
                xing: None,
                lame: None,
                vbri: read_vbri(frame, start),
            },
        }
    }
}

/// The name of a Xing header.
const XING: [u8; 4] = *b"Xing";
/// The name LAME gives a Xing header for a constant bitrate.
const INFO: [u8; 4] = *b"Info";

/// The octets of Layer III side information, after which an encoder
/// writes a Xing or Info header.
const fn side_info_len(header: &FrameHeader) -> u64 {
    match (header.version, header.mode) {
        (Version::Mpeg1, ChannelMode::Mono) => 17,
        (Version::Mpeg1, _) => 32,
        (Version::Mpeg2 | Version::Mpeg25, ChannelMode::Mono) => 9,
        (Version::Mpeg2 | Version::Mpeg25, _) => 17,
    }
}

/// Reads the flags and fields of a Xing header from `cursor`, just after
/// its name.
fn read_xing(cursor: &mut Cursor<'_>, info: bool) -> Result<Xing, ParseFault> {
    let flags = cursor.u32_be()?;
    let has = |flag: u32| flags & flag != 0;
    // Frames, bytes, table of contents and quality, as the flags announce.
    let fields = [(1, 4), (2, 4), (4, 100), (8, 4)]
        .into_iter()
        .filter(|&(flag, _)| has(flag))
        .fold(0, |len: u64, (_, field)| len.saturating_add(field));
    // The fields are in hand, so each read below succeeds.
    let mut fields = cursor.sub(fields)?;
    let frames = has(1).then(|| fields.u32_be().unwrap_or_default());
    let bytes = has(2).then(|| fields.u32_be().unwrap_or_default());
    let toc = has(4).then(|| fields.array::<100>().unwrap_or([0; 100]));
    let quality = has(8).then(|| fields.u32_be().unwrap_or_default());
    Ok(Xing {
        info,
        frames,
        bytes,
        toc,
        quality,
    })
}

/// Reads the LAME extension from `cursor`, just after a Xing header in
/// `frame`, when the encoder that wrote it is LAME or `FFmpeg`.
fn read_lame(cursor: &mut Cursor<'_>, frame: &[u8]) -> Option<Result<LameTag, ParseFault>> {
    if !matches!(
        cursor.rest().first_chunk::<4>()?,
        b"LAME" | b"Lavf" | b"Lavc"
    ) {
        return None;
    }
    // The tag's CRC covers the frame up to the CRC itself, which is the
    // extension's last two octets.
    let covered = frame
        .len()
        .saturating_sub(cursor.rest().len())
        .saturating_add(34);
    Some(cursor.array::<36>().map(|tag| {
        let crc = frame.get(..covered).map_or(0, crc16_arc);
        lame_tag(&tag, crc)
    }))
}

/// The LAME extension in `tag`, whose CRC should be `crc` (LAME's
/// `VbrTag.c`; `FFmpeg` reads it the same way).
fn lame_tag(tag: &[u8; 36], crc: u16) -> LameTag {
    // Thirty-six octets are in hand, so every read below succeeds.
    let mut fields = Cursor::new(tag);
    let encoder = fields.array::<9>().unwrap_or_default();
    // The tag revision and VBR method, then the lowpass frequency.
    let _ = fields.skip(2);
    let peak = fields.u32_be().unwrap_or_default();
    let track = fields.u16_be().unwrap_or_default();
    let album = fields.u16_be().unwrap_or_default();
    // The encoding flags and ATH type, then the bitrate.
    let _ = fields.skip(2);
    let delay_and_padding = fields.u24_be().unwrap_or_default();
    // Misc, the MP3 gain, surround and preset.
    let _ = fields.skip(4);
    let music_length = fields.u32_be().unwrap_or_default();
    // The music CRC.
    let _ = fields.skip(2);
    let stored = fields.u16_be().unwrap_or_default();
    let replay_gain = if stored == crc {
        Ok(ReplayGain {
            peak: (peak != 0).then(|| peak_ratio(peak)),
            track: gain(track, 1),
            album: gain(album, 2),
        })
    } else {
        Err(CrcMismatch {
            stored,
            computed: crc,
        })
    };
    LameTag {
        encoder,
        delay: narrow(delay_and_padding >> 12),
        padding: narrow(delay_and_padding & 0x0FFF),
        music_length,
        replay_gain,
    }
}

/// A value of at most 16 bits, which always fits.
fn narrow(value: u32) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

/// The peak sample LAME stores as a fraction of full scale times 2^23.
fn peak_ratio(raw: u32) -> Result<PeakRatio, ValueError> {
    let [a, b, c, d] = raw.to_be_bytes();
    // Built from two exact halves, since a u32 does not convert to f32
    // without rounding.
    let value =
        f32::from(u16::from_be_bytes([a, b])) * 65_536.0 + f32::from(u16::from_be_bytes([c, d]));
    PeakRatio::new(value / 8_388_608.0)
}

/// The gain in a `ReplayGain` field of the LAME extension, when its name
/// code is `name` (1 for the track, 2 for the album): a sign bit and nine
/// bits of tenths of a decibel, at most 51.1 dB, always in range.
fn gain(field: u16, name: u16) -> Option<GainDb> {
    if field >> 13 != name {
        return None;
    }
    let tenths = f32::from(field & 0x01FF);
    let tenths = if field & 0x0200 == 0 { tenths } else { -tenths };
    GainDb::new(tenths / 10.0).ok()
}

/// CRC-16/ARC (polynomial 0x8005 reflected, initial value 0), the CRC of
/// the LAME extension.
fn crc16_arc(octets: &[u8]) -> u16 {
    octets.iter().fold(0, |crc, &octet| {
        (0..8).fold(crc ^ u16::from(octet), |crc, _| {
            if crc & 1 == 0 {
                crc >> 1
            } else {
                (crc >> 1) ^ 0xA001
            }
        })
    })
}

/// Reads the VBRI header in `frame`, which starts at `start`, when it
/// holds one.
fn read_vbri(frame: &[u8], start: u64) -> Option<Result<Vbri, ParseFault>> {
    if frame.get(36..40) != Some(b"VBRI".as_slice()) {
        return None;
    }
    let mut cursor = Cursor::at(frame, start);
    let fields = cursor.skip(40).and_then(|()| cursor.array::<18>());
    Some(fields.map(|fields| {
        // Eighteen octets are in hand, so every read below succeeds.
        let mut fields = Cursor::new(&fields);
        let version = fields.u16_be().unwrap_or_default();
        // The delay.
        let _ = fields.skip(2);
        Vbri {
            version,
            quality: fields.u16_be().unwrap_or_default(),
            bytes: fields.u32_be().unwrap_or_default(),
            frames: fields.u32_be().unwrap_or_default(),
        }
    }))
}

/// Builds a seek index that never holds more points than the index-entry
/// limit (SEC-MED-006): it keeps every `stride`-th frame, starting at the
/// first, and doubles the stride, dropping every other point, each time it
/// is full.
#[derive(Debug, PartialEq, Eq)]
struct Thinner {
    /// The most points to keep.
    max: u64,
    /// Every how many frames a point is kept; always a power of two.
    stride: u64,
    /// The points kept: those of frames 0, `stride`, 2 × `stride`, and so
    /// on.
    points: Vec<SeekPoint>,
}

impl Thinner {
    /// An empty index that keeps at most `max` points.
    const fn new(max: u64) -> Self {
        Self {
            max,
            stride: 1,
            points: Vec::new(),
        }
    }

    /// Offers the point of frame `rank`. Frames are offered in order,
    /// starting at 0.
    fn offer(&mut self, rank: u64, point: SeekPoint) {
        if self.max == 0 || rank.checked_rem(self.stride) != Some(0) {
            return;
        }
        if len_of(&self.points) >= self.max {
            self.stride = self.stride.saturating_mul(2);
            let mut keep = false;
            self.points.retain(|_| {
                keep = !keep;
                keep
            });
            if rank.checked_rem(self.stride) != Some(0) {
                return;
            }
        }
        self.points.push(point);
    }

    /// The index.
    fn finish(self) -> SeekIndex {
        SeekIndex {
            points: self.points,
            stride: self.stride,
        }
    }
}

/// How many points `points` holds.
fn len_of(points: &[SeekPoint]) -> u64 {
    u64::try_from(points.len()).unwrap_or(u64::MAX)
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use super::*;
    use gunmetal_testkit::mpa as build;
    use proptest::prelude::*;

    fn hz(value: u32) -> SampleRate {
        SampleRate::new(value).unwrap()
    }

    /// The header LAME writes for its default stream.
    #[test]
    fn decodes_the_header_lame_writes_for_128_kbps_joint_stereo() {
        assert_eq!(
            frame_header([0xFF, 0xFB, 0x90, 0x64]),
            Ok(FrameHeader {
                version: Version::Mpeg1,
                layer: Layer::III,
                crc: false,
                bitrate: 128,
                sample_rate: hz(44_100),
                padding: false,
                mode: ChannelMode::JointStereo,
                samples: 1_152,
                len: 417,
            })
        );
    }

    /// The bitrate tables of ISO/IEC 11172-3 and 13818-3 (table 2.4.2.3 of
    /// each) for indices 1 to 14, written out for this test.
    const BITRATES: [(Version, Layer, [u16; 14]); 9] = [
        (
            Version::Mpeg1,
            Layer::I,
            [
                32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448,
            ],
        ),
        (
            Version::Mpeg1,
            Layer::II,
            [
                32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384,
            ],
        ),
        (
            Version::Mpeg1,
            Layer::III,
            [
                32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320,
            ],
        ),
        (
            Version::Mpeg2,
            Layer::I,
            [
                32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256,
            ],
        ),
        (
            Version::Mpeg2,
            Layer::II,
            [8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160],
        ),
        (
            Version::Mpeg2,
            Layer::III,
            [8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160],
        ),
        (
            Version::Mpeg25,
            Layer::I,
            [
                32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256,
            ],
        ),
        (
            Version::Mpeg25,
            Layer::II,
            [8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160],
        ),
        (
            Version::Mpeg25,
            Layer::III,
            [8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160],
        ),
    ];

    fn testkit_version(version: Version) -> build::Version {
        match version {
            Version::Mpeg1 => build::Version::Mpeg1,
            Version::Mpeg2 => build::Version::Mpeg2,
            Version::Mpeg25 => build::Version::Mpeg25,
        }
    }

    fn testkit_layer(layer: Layer) -> build::Layer {
        match layer {
            Layer::I => build::Layer::I,
            Layer::II => build::Layer::II,
            Layer::III => build::Layer::III,
        }
    }

    /// A stereo header with the given fields and every flag clear.
    fn header(version: Version, layer: Layer, bitrate_index: u8, rate_index: u8) -> [u8; 4] {
        build::Frame {
            layer: testkit_layer(layer),
            ..build::Frame::layer3(
                testkit_version(version),
                bitrate_index,
                rate_index,
                build::Mode::Stereo,
            )
        }
        .header()
    }

    /// The highest sampling frequency of each version: 48, 24 and 12 kHz,
    /// where every bitrate gives a whole number of octets.
    const FREQUENCY_INDEX: u8 = 1;

    /// The samples a frame of each layer and version holds, and the
    /// octets each kbit/s adds to a frame at [`FREQUENCY_INDEX`].
    fn shape(version: Version, layer: Layer) -> (u16, u16, u32) {
        match (version, layer) {
            (Version::Mpeg1, Layer::I) => (384, 1, 48_000),
            (Version::Mpeg1, _) => (1_152, 3, 48_000),
            (Version::Mpeg2, Layer::I) => (384, 2, 24_000),
            (Version::Mpeg2, Layer::II) => (1_152, 6, 24_000),
            (Version::Mpeg2, Layer::III) => (576, 3, 24_000),
            (Version::Mpeg25, Layer::I) => (384, 4, 12_000),
            (Version::Mpeg25, Layer::II) => (1_152, 12, 12_000),
            (Version::Mpeg25, Layer::III) => (576, 6, 12_000),
        }
    }

    #[test]
    fn decodes_every_bitrate_of_every_version_and_layer() {
        for (version, layer, rates) in BITRATES {
            let (samples, octets_per_kbps, frequency) = shape(version, layer);
            for (index, bitrate) in (1..=14).zip(rates) {
                assert_eq!(
                    frame_header(header(version, layer, index, FREQUENCY_INDEX)),
                    Ok(FrameHeader {
                        version,
                        layer,
                        crc: false,
                        bitrate,
                        sample_rate: hz(frequency),
                        padding: false,
                        mode: ChannelMode::Stereo,
                        samples,
                        len: bitrate * octets_per_kbps,
                    }),
                    "{version:?} {layer:?} index {index}"
                );
            }
        }
    }

    #[test]
    fn refuses_free_format_and_the_forbidden_bitrate_index_in_every_table() {
        for (version, layer, _) in BITRATES {
            assert_eq!(
                frame_header(header(version, layer, 0, 0)),
                Err(HeaderError::FreeFormat),
                "{version:?} {layer:?}"
            );
            assert_eq!(
                frame_header(header(version, layer, 15, 0)),
                Err(HeaderError::ForbiddenBitrate),
                "{version:?} {layer:?}"
            );
        }
    }

    #[test]
    fn decodes_every_sampling_frequency_and_refuses_index_3() {
        let cases = [
            (Version::Mpeg1, [44_100, 48_000, 32_000]),
            (Version::Mpeg2, [22_050, 24_000, 16_000]),
            (Version::Mpeg25, [11_025, 12_000, 8_000]),
        ];
        for (version, frequencies) in cases {
            for (index, frequency) in (0..3).zip(frequencies) {
                let decoded = frame_header(header(version, Layer::III, 1, index));
                assert_eq!(
                    decoded.map(|header| header.sample_rate),
                    Ok(hz(frequency)),
                    "{version:?} index {index}"
                );
            }
            assert_eq!(
                frame_header(header(version, Layer::III, 1, 3)),
                Err(HeaderError::ReservedSampleRate),
                "{version:?}"
            );
        }
    }

    #[test]
    fn sizes_padded_frames_by_one_slot_of_their_layer() {
        // 44.1 kHz rounds down: 417.96 octets at 128 kbit/s, 121.9 slots
        // of four octets at 448 kbit/s in Layer I.
        let padded = |version, layer, index| {
            let frame = build::Frame {
                layer: testkit_layer(layer),
                padding: true,
                ..build::Frame::layer3(testkit_version(version), index, 0, build::Mode::Mono)
            };
            frame_header(frame.header()).map(|header| (header.padding, header.len))
        };
        assert_eq!(padded(Version::Mpeg1, Layer::III, 9), Ok((true, 418)));
        assert_eq!(padded(Version::Mpeg1, Layer::II, 14), Ok((true, 1_254)));
        assert_eq!(padded(Version::Mpeg1, Layer::I, 14), Ok((true, 488)));
        assert_eq!(padded(Version::Mpeg2, Layer::III, 8), Ok((true, 209)));
        // The largest frame there is: MPEG 2.5 Layer II at 160 kbit/s and
        // 8 kHz.
        let largest = build::Frame {
            layer: build::Layer::II,
            padding: true,
            ..build::Frame::layer3(build::Version::Mpeg25, 14, 2, build::Mode::Mono)
        };
        assert_eq!(
            frame_header(largest.header()).map(|header| header.len),
            Ok(2_881)
        );
    }

    #[test]
    fn decodes_the_crc_flag_and_every_channel_mode() {
        let modes = [
            (build::Mode::Stereo, ChannelMode::Stereo),
            (build::Mode::JointStereo, ChannelMode::JointStereo),
            (build::Mode::DualChannel, ChannelMode::DualChannel),
            (build::Mode::Mono, ChannelMode::Mono),
        ];
        for (written, mode) in modes {
            let frame = build::Frame {
                crc: true,
                ..build::Frame::layer3(build::Version::Mpeg1, 9, 0, written)
            };
            assert_eq!(
                frame_header(frame.header()).map(|header| (header.crc, header.mode)),
                Ok((true, mode)),
                "{mode:?}"
            );
        }
    }

    #[test]
    fn refuses_the_reserved_version_and_layer() {
        // Version bits 01 and layer bits 00, each with every other field
        // valid.
        assert_eq!(
            frame_header([0xFF, 0xEB, 0x90, 0x00]),
            Err(HeaderError::ReservedVersion)
        );
        assert_eq!(
            frame_header([0xFF, 0xF9, 0x90, 0x00]),
            Err(HeaderError::ReservedLayer)
        );
        // MPEG 2.5 with the reserved layer: the smallest second octet that
        // has every sync bit.
        assert_eq!(
            frame_header([0xFF, 0xE0, 0x10, 0x00]),
            Err(HeaderError::ReservedLayer)
        );
    }

    #[test]
    fn refuses_a_header_missing_any_sync_bit() {
        for octets in [
            [0x7F, 0xFB, 0x90, 0x64],
            [0xFE, 0xFB, 0x90, 0x64],
            [0xFF, 0xDB, 0x90, 0x64],
            [0xFF, 0xBB, 0x90, 0x64],
            [0xFF, 0x7B, 0x90, 0x64],
            [0x00, 0x00, 0x00, 0x00],
        ] {
            assert_eq!(
                frame_header(octets),
                Err(HeaderError::NoSync),
                "{octets:02X?}"
            );
        }
    }

    #[test]
    fn reports_the_first_invalid_field_in_header_order() {
        // Reserved version, reserved layer, forbidden bitrate and reserved
        // frequency together; each fix uncovers the next.
        assert_eq!(
            frame_header([0xFF, 0xE8, 0xFC, 0x00]),
            Err(HeaderError::ReservedVersion)
        );
        assert_eq!(
            frame_header([0xFF, 0xF8, 0xFC, 0x00]),
            Err(HeaderError::ReservedLayer)
        );
        assert_eq!(
            frame_header([0xFF, 0xFA, 0xFC, 0x00]),
            Err(HeaderError::ForbiddenBitrate)
        );
        assert_eq!(
            frame_header([0xFF, 0xFA, 0x0C, 0x00]),
            Err(HeaderError::FreeFormat)
        );
        assert_eq!(
            frame_header([0xFF, 0xFA, 0x1C, 0x00]),
            Err(HeaderError::ReservedSampleRate)
        );
    }

    /// The standards' header rules as one function of the 32-bit header,
    /// written for this test with its own tables, in which index 0 is free
    /// format and index 15 forbidden.
    fn model(word: u32) -> Result<FrameHeader, HeaderError> {
        const KBPS: [[u16; 16]; 5] = [
            [
                0, 32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448, 0,
            ],
            [
                0, 32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384, 0,
            ],
            [
                0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0,
            ],
            [
                0, 32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256, 0,
            ],
            [
                0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, 0,
            ],
        ];
        // Indexed by the version bits: 2.5, reserved, 2, 1.
        const HZ: [[u32; 4]; 4] = [
            [11_025, 12_000, 8_000, 0],
            [0; 4],
            [22_050, 24_000, 16_000, 0],
            [44_100, 48_000, 32_000, 0],
        ];
        let field = |shift: u32, mask: u32| ((word >> shift) & mask) as usize;
        if word >> 21 != 0x7FF {
            return Err(HeaderError::NoSync);
        }
        let version_bits = field(19, 3);
        if version_bits == 1 {
            return Err(HeaderError::ReservedVersion);
        }
        let layer_bits = field(17, 3);
        if layer_bits == 0 {
            return Err(HeaderError::ReservedLayer);
        }
        let index = field(12, 15);
        match index {
            0 => return Err(HeaderError::FreeFormat),
            15 => return Err(HeaderError::ForbiddenBitrate),
            _ => {}
        }
        let frequency = HZ[version_bits][field(10, 3)];
        if frequency == 0 {
            return Err(HeaderError::ReservedSampleRate);
        }
        let mpeg1 = version_bits == 3;
        let table = match (mpeg1, layer_bits) {
            (true, 3) => 0,
            (true, 2) => 1,
            (true, _) => 2,
            (false, 3) => 3,
            (false, _) => 4,
        };
        let bitrate = KBPS[table][index];
        let padding = field(9, 1) == 1;
        let samples: u16 = match (layer_bits, mpeg1) {
            (3, _) => 384,
            (2, _) | (1, true) => 1_152,
            _ => 576,
        };
        let pad = u32::from(padding);
        let bits_per_second = u32::from(bitrate) * 1_000;
        let len = if layer_bits == 3 {
            (12 * bits_per_second / frequency + pad) * 4
        } else {
            u32::from(samples) / 8 * bits_per_second / frequency + pad
        };
        Ok(FrameHeader {
            version: [
                Version::Mpeg25,
                Version::Mpeg25,
                Version::Mpeg2,
                Version::Mpeg1,
            ][version_bits],
            layer: [Layer::III, Layer::III, Layer::II, Layer::I][layer_bits],
            crc: field(16, 1) == 0,
            bitrate,
            sample_rate: hz(frequency),
            padding,
            mode: [
                ChannelMode::Stereo,
                ChannelMode::JointStereo,
                ChannelMode::DualChannel,
                ChannelMode::Mono,
            ][field(6, 3)],
            samples,
            len: u16::try_from(len).unwrap(),
        })
    }

    /// Checks the model against literal headers of each outcome, so the
    /// property below rests on a model known to agree with the standards.
    #[test]
    fn the_model_agrees_with_literal_headers() {
        assert_eq!(
            model(0xFFFB_9064),
            Ok(FrameHeader {
                version: Version::Mpeg1,
                layer: Layer::III,
                crc: false,
                bitrate: 128,
                sample_rate: hz(44_100),
                padding: false,
                mode: ChannelMode::JointStereo,
                samples: 1_152,
                len: 417,
            })
        );
        assert_eq!(
            model(0xFFE3_48C4),
            Ok(FrameHeader {
                version: Version::Mpeg25,
                layer: Layer::III,
                crc: false,
                bitrate: 32,
                sample_rate: hz(8_000),
                padding: false,
                mode: ChannelMode::Mono,
                samples: 576,
                len: 288,
            })
        );
        assert_eq!(
            model(0xFFFE_EBBF),
            Ok(FrameHeader {
                version: Version::Mpeg1,
                layer: Layer::I,
                crc: true,
                bitrate: 448,
                sample_rate: hz(32_000),
                padding: true,
                mode: ChannelMode::DualChannel,
                samples: 384,
                len: 676,
            })
        );
        // MPEG-2 Layer II at 8 kbit/s and 24 kHz.
        assert_eq!(model(0xFFF5_1400).map(|h| h.len), Ok(48));
        let errors = [
            (0x7FFB_9064, HeaderError::NoSync),
            (0xFFEB_9000, HeaderError::ReservedVersion),
            (0xFFF9_9000, HeaderError::ReservedLayer),
            (0xFFFB_0000, HeaderError::FreeFormat),
            (0xFFFB_F000, HeaderError::ForbiddenBitrate),
            (0xFFFB_9C00, HeaderError::ReservedSampleRate),
        ];
        for (word, error) in errors {
            assert_eq!(model(word), Err(error), "{word:08X}");
        }
    }

    /// Headers with every sync bit set most of the time, so that random
    /// input reaches the fields behind the sync check.
    fn any_header() -> impl Strategy<Value = u32> {
        prop_oneof![
            4 => any::<u32>().prop_map(|word| word | 0xFFE0_0000),
            1 => any::<u32>(),
        ]
    }

    proptest! {
        /// Verifies: SEC-MED-001
        #[test]
        fn decodes_every_header_exactly_as_the_standards_say(word in any_header()) {
            prop_assert_eq!(frame_header(word.to_be_bytes()), model(word));
        }
    }

    // ---- stream_info ----

    use crate::parse::{DriveError, LimitKind, drive};

    /// What driving [`stream_info`] over a file in memory gives.
    type Parsed = Result<Result<MpegStream, MpaError>, DriveError>;

    /// `count` copies of `item`. The core's lints refuse `vec![item;
    /// count]`, tests included.
    fn repeated<T: Clone>(item: T, count: usize) -> Vec<T> {
        (0..count).map(|_| item.clone()).collect()
    }

    /// The budget the parser documents as always enough for `file`.
    fn budget(file: &[u8]) -> Budget {
        Budget::for_input(
            u64::try_from(file.len()).unwrap(),
            STEPS_PER_OCTET,
            FIXED_STEPS,
        )
    }

    fn parse(file: &[u8]) -> Parsed {
        parse_with(file, &Limits::DEFAULT)
    }

    fn parse_with(file: &[u8], limits: &Limits) -> Parsed {
        drive(stream_info(limits, budget(file)), file, limits)
    }

    /// Parses `file` with a budget of exactly `steps`.
    fn parse_spending(file: &[u8], steps: u64) -> Parsed {
        drive(
            stream_info(&Limits::DEFAULT, Budget::for_input(0, 0, steps)),
            file,
            &Limits::DEFAULT,
        )
    }

    fn limited(kind: LimitKind, value: u64) -> Limits {
        Limits::DEFAULT.with_override(kind, value).unwrap()
    }

    /// MPEG-2 Layer III at 8 kbit/s and 24 kHz, mono: 24 octets, the
    /// smallest frame there is.
    fn small() -> build::Frame {
        build::Frame::layer3(build::Version::Mpeg2, 1, 1, build::Mode::Mono)
    }

    fn small_header() -> FrameHeader {
        FrameHeader {
            version: Version::Mpeg2,
            layer: Layer::III,
            crc: false,
            bitrate: 8,
            sample_rate: hz(24_000),
            padding: false,
            mode: ChannelMode::Mono,
            samples: 576,
            len: 24,
        }
    }

    /// The same stream at 16 kbit/s: 48 octets.
    fn middling() -> build::Frame {
        build::Frame::layer3(build::Version::Mpeg2, 2, 1, build::Mode::Mono)
    }

    /// The same stream at 64 kbit/s: 192 octets, room for an Info header
    /// with every field and the LAME extension.
    fn roomy() -> build::Frame {
        build::Frame::layer3(build::Version::Mpeg2, 8, 1, build::Mode::Mono)
    }

    fn roomy_header() -> FrameHeader {
        FrameHeader {
            bitrate: 64,
            len: 192,
            ..small_header()
        }
    }

    const fn point(sample: u64, offset: u64) -> SeekPoint {
        SeekPoint { sample, offset }
    }

    /// The seek index of `count` frames of 576 samples, every frame kept,
    /// starting at `start`, `len` octets apart.
    fn every_frame(count: u64, start: u64, len: u64) -> SeekIndex {
        SeekIndex {
            points: (0..count)
                .map(|frame| point(576 * frame, start + len * frame))
                .collect(),
            stride: 1,
        }
    }

    /// A stream with no encoder header.
    fn plain(
        start: u64,
        header: FrameHeader,
        frames: u64,
        end: u64,
        seek: SeekIndex,
    ) -> MpegStream {
        MpegStream {
            start,
            header,
            xing: None,
            lame: None,
            vbri: None,
            frames,
            end,
            seek,
        }
    }

    /// Verifies: SEC-MED-001, SEC-MED-008
    #[test]
    fn scans_every_frame_of_a_stream_without_an_encoder_header() {
        let file = build::stream(&[small(); 3]);
        assert_eq!(
            parse(&file),
            Ok(Ok(plain(0, small_header(), 3, 72, every_frame(3, 0, 24))))
        );
    }

    #[test]
    fn scans_frames_whose_bitrate_and_padding_vary() {
        let padded = build::Frame {
            padding: true,
            ..small()
        };
        let file = build::stream(&[small(), padded, middling(), small()]);
        assert_eq!(
            parse(&file),
            Ok(Ok(plain(
                0,
                small_header(),
                4,
                121,
                SeekIndex {
                    points: vec![
                        point(0, 0),
                        point(576, 24),
                        point(1_152, 49),
                        point(1_728, 97)
                    ],
                    stride: 1,
                },
            )))
        );
    }

    #[test]
    fn finds_a_stream_after_8_kb_of_zeros() {
        let mut file = repeated(0, 8_192);
        file.extend(build::stream(&[small(); 3]));
        assert_eq!(
            parse(&file),
            Ok(Ok(plain(
                8_192,
                small_header(),
                3,
                8_264,
                every_frame(3, 8_192, 24)
            )))
        );
    }

    #[test]
    fn finds_a_header_that_straddles_two_search_windows() {
        // The first window holds 4,096 octets; a header that starts in its
        // last three is found in the next.
        for zeros in [4_092, 4_093, 4_094, 4_095, 4_096] {
            let mut file = repeated(0, zeros);
            file.extend(build::stream(&[small(); 2]));
            let start = u64::try_from(zeros).unwrap();
            assert_eq!(
                parse(&file),
                Ok(Ok(plain(
                    start,
                    small_header(),
                    2,
                    start + 48,
                    every_frame(2, start, 24)
                ))),
                "{zeros} zeros"
            );
        }
    }

    #[test]
    fn skips_an_id3v2_tag_that_holds_a_false_sync() {
        // The tag holds two frames of a valid stream. Read as audio, they
        // would start the stream at offset 10.
        let mut file = build::id3v2_tag(&build::stream(&[small(); 2]), false);
        file.extend(build::stream(&[middling(); 3]));
        let header = FrameHeader {
            bitrate: 16,
            len: 48,
            ..small_header()
        };
        assert_eq!(
            parse(&file),
            Ok(Ok(plain(58, header, 3, 202, every_frame(3, 58, 48))))
        );
    }

    #[test]
    fn skips_stacked_id3v2_tags_and_a_footer() {
        let mut file = build::id3v2_tag(&[], false);
        file.extend(build::id3v2_tag(&[0xFF; 5], true));
        file.extend(build::stream(&[small(); 2]));
        assert_eq!(
            parse(&file),
            Ok(Ok(plain(35, small_header(), 2, 83, every_frame(2, 35, 24))))
        );
    }

    /// A tag whose ten-octet header is changed by `edit`, holding one frame
    /// of the stream that follows it. Read as a tag, it is skipped and the
    /// stream starts at 34; read as audio, the frame inside it starts the
    /// stream at 10.
    fn tag_holding_a_frame(edit: impl FnOnce(&mut [u8])) -> Vec<u8> {
        let mut file = build::id3v2_tag(&small().write(&[]), false);
        edit(&mut file[..10]);
        file.extend(build::stream(&[small(); 2]));
        file
    }

    #[test]
    fn reads_an_id3v2_header_only_when_every_field_is_valid() {
        let as_tag = Ok(Ok(plain(34, small_header(), 2, 82, every_frame(2, 34, 24))));
        let as_audio = Ok(Ok(plain(10, small_header(), 3, 82, every_frame(3, 10, 24))));
        assert_eq!(parse(&tag_holding_a_frame(|_| {})), as_tag);
        // Any version and revision but 0xFF, and any flags.
        assert_eq!(
            parse(&tag_holding_a_frame(|header| {
                header[3] = 0xFE;
                header[4] = 0xFE;
                header[5] = 0xEF;
            })),
            as_tag
        );
        let not_tags: [fn(&mut [u8]); 6] = [
            |header| header[0] = b'J',
            |header| header[1] = b'C',
            |header| header[2] = b'4',
            |header| header[3] = 0xFF,
            |header| header[4] = 0xFF,
            |header| header[9] |= 0x80,
        ];
        for (case, edit) in not_tags.into_iter().enumerate() {
            assert_eq!(parse(&tag_holding_a_frame(edit)), as_audio, "case {case}");
        }
    }

    /// Verifies: SEC-TM-032, SEC-MED-008
    #[test]
    fn refuses_an_id3v2_tag_that_runs_past_the_file() {
        let mut file = build::id3v2_tag(&[0; 100], false);
        file.truncate(30);
        assert_eq!(
            parse(&file),
            Ok(Err(MpaError::Fault(ParseFault::Truncated {
                offset: 0,
                needed: 110,
                available: 30,
            })))
        );
        // A footer counts too.
        let mut file = build::id3v2_tag(&[], false);
        file.extend(build::id3v2_tag(&[0; 4], true));
        file.truncate(30);
        assert_eq!(
            parse(&file),
            Ok(Err(MpaError::Fault(ParseFault::Truncated {
                offset: 10,
                needed: 24,
                available: 20,
            })))
        );
    }

    #[test]
    fn reports_where_the_search_started_when_it_finds_no_frame() {
        assert_eq!(parse(&[]), Ok(Err(MpaError::NoFrames { offset: 0 })));
        assert_eq!(parse(&[0xFF]), Ok(Err(MpaError::NoFrames { offset: 0 })));
        assert_eq!(
            parse(&[0x00; 5_000]),
            Ok(Err(MpaError::NoFrames { offset: 0 }))
        );
        let mut file = build::id3v2_tag(&[], false);
        file.extend([0x00; 20]);
        assert_eq!(parse(&file), Ok(Err(MpaError::NoFrames { offset: 10 })));
        // A tag that ends the file leaves nothing to search.
        assert_eq!(
            parse(&build::id3v2_tag(&[0; 3], false)),
            Ok(Err(MpaError::NoFrames { offset: 13 }))
        );
    }

    /// A free-format header for [`small`]'s stream: bitrate index 0.
    fn free_format() -> [u8; 4] {
        build::Frame {
            bitrate_index: 0,
            ..small()
        }
        .header()
    }

    #[test]
    fn refuses_a_stream_of_free_format_frames() {
        let mut file = Vec::new();
        for _ in 0..3 {
            file.extend(free_format());
            file.extend([0; 60]);
        }
        assert_eq!(parse(&file), Ok(Err(MpaError::FreeFormat { offset: 0 })));
    }

    #[test]
    fn explains_a_failed_search_by_the_first_near_miss() {
        // A free-format header, then a header whose frame runs past the
        // end of the file.
        let mut file = free_format().to_vec();
        file.extend([0; 4]);
        file.extend(small().header());
        file.extend([0; 5]);
        assert_eq!(parse(&file), Ok(Err(MpaError::FreeFormat { offset: 0 })));
        // The same two the other way round: the free-format header lies
        // inside the frame that runs past the end.
        let mut file = small().header().to_vec();
        file.extend([0; 4]);
        file.extend(free_format());
        file.extend([0; 4]);
        assert_eq!(
            parse(&file),
            Ok(Err(MpaError::Fault(ParseFault::Truncated {
                offset: 0,
                needed: 24,
                available: 16,
            })))
        );
    }

    #[test]
    fn detects_and_refuses_layer_i_and_layer_ii_streams() {
        let layer2 = build::Frame {
            layer: build::Layer::II,
            ..build::Frame::layer3(build::Version::Mpeg1, 9, 0, build::Mode::Stereo)
        };
        let layer1 = build::Frame {
            layer: build::Layer::I,
            ..build::Frame::layer3(build::Version::Mpeg1, 4, 1, build::Mode::Stereo)
        };
        assert_eq!(
            parse(&build::stream(&[layer2; 2])),
            Ok(Err(MpaError::UnsupportedLayer {
                offset: 0,
                layer: Layer::II,
            }))
        );
        assert_eq!(
            parse(&build::stream(&[layer1; 2])),
            Ok(Err(MpaError::UnsupportedLayer {
                offset: 0,
                layer: Layer::I,
            }))
        );
        // A single frame with nothing after it is detected too.
        let mut file = repeated(0, 7);
        file.extend(build::stream(&[layer2]));
        assert_eq!(
            parse(&file),
            Ok(Err(MpaError::UnsupportedLayer {
                offset: 7,
                layer: Layer::II,
            }))
        );
    }

    #[test]
    fn accepts_a_lone_frame_only_where_no_header_can_follow_it() {
        let lone = Ok(Ok(plain(0, small_header(), 1, 24, every_frame(1, 0, 24))));
        let mut file = build::stream(&[small()]);
        assert_eq!(parse(&file), lone);
        file.extend([0xAA; 3]);
        assert_eq!(parse(&file), lone);
        // Four octets after it could have held a header, and do not.
        file.extend([0xAA]);
        assert_eq!(parse(&file), Ok(Err(MpaError::NoFrames { offset: 0 })));
    }

    /// Frames of [`small`]'s stream that differ from it in one field each.
    fn strangers() -> [(build::Frame, &'static str); 3] {
        [
            (
                build::Frame::layer3(build::Version::Mpeg1, 1, 1, build::Mode::Mono),
                "version",
            ),
            (
                build::Frame {
                    layer: build::Layer::II,
                    ..small()
                },
                "layer",
            ),
            (
                build::Frame::layer3(build::Version::Mpeg2, 1, 0, build::Mode::Mono),
                "frequency",
            ),
        ]
    }

    #[test]
    fn needs_the_next_header_to_match_in_version_layer_and_frequency() {
        // A small frame, then two of the stranger's: the small one is not
        // confirmed, and the stream starts with the stranger, which is
        // refused when it is Layer II.
        let file = |stranger: build::Frame| {
            let mut file = build::stream(&[small()]);
            file.extend(build::stream(&[stranger; 2]));
            file
        };
        assert_eq!(
            parse(&file(build::Frame::layer3(
                build::Version::Mpeg1,
                1,
                1,
                build::Mode::Mono,
            ))),
            Ok(Ok(plain(
                24,
                FrameHeader {
                    version: Version::Mpeg1,
                    layer: Layer::III,
                    crc: false,
                    bitrate: 32,
                    sample_rate: hz(48_000),
                    padding: false,
                    mode: ChannelMode::Mono,
                    samples: 1_152,
                    len: 96,
                },
                2,
                216,
                SeekIndex {
                    points: vec![point(0, 24), point(1_152, 120)],
                    stride: 1,
                },
            )))
        );
        assert_eq!(
            parse(&file(build::Frame {
                layer: build::Layer::II,
                ..small()
            })),
            Ok(Err(MpaError::UnsupportedLayer {
                offset: 24,
                layer: Layer::II,
            }))
        );
        assert_eq!(
            parse(&file(build::Frame::layer3(
                build::Version::Mpeg2,
                1,
                0,
                build::Mode::Mono,
            ))),
            Ok(Ok(plain(
                24,
                FrameHeader {
                    version: Version::Mpeg2,
                    layer: Layer::III,
                    crc: false,
                    bitrate: 8,
                    sample_rate: hz(22_050),
                    padding: false,
                    mode: ChannelMode::Mono,
                    samples: 576,
                    len: 26,
                },
                2,
                76,
                SeekIndex {
                    points: vec![point(0, 24), point(576, 50)],
                    stride: 1,
                },
            )))
        );
    }

    #[test]
    fn ends_the_scan_at_a_frame_of_another_stream() {
        for (stranger, field) in strangers() {
            let mut file = build::stream(&[small(); 2]);
            file.extend(build::stream(&[stranger, small()]));
            assert_eq!(
                parse(&file),
                Ok(Ok(plain(0, small_header(), 2, 48, every_frame(2, 0, 24)))),
                "{field}"
            );
        }
    }

    #[test]
    fn ends_the_scan_before_a_partial_frame_or_trailing_tag() {
        let whole = Ok(Ok(plain(0, small_header(), 2, 48, every_frame(2, 0, 24))));
        let mut file = build::stream(&[small(); 3]);
        file.truncate(70);
        assert_eq!(parse(&file), whole);
        file.truncate(51);
        assert_eq!(parse(&file), whole);
        let mut file = build::stream(&[small(); 2]);
        file.extend(b"TAG");
        file.extend([0x20; 125]);
        assert_eq!(parse(&file), whole);
    }

    /// The table of contents `[0, 1, 2, ..., 99]`.
    fn counting_toc() -> [u8; 100] {
        core::array::from_fn(|entry| u8::try_from(entry).unwrap())
    }

    /// The LAME extension of a VBR encode with `ReplayGain`: +6.7 dB for the
    /// track, -6.5 dB for the album and a peak of 253/256.
    fn lame() -> build::Lame {
        build::Lame {
            encoder: *b"LAME3.100",
            revision_method: 0x04,
            lowpass: 0xB9,
            peak: 0x007E_8000,
            track_gain: build::replay_gain(1, 3, 67),
            album_gain: build::replay_gain(2, 2, -65),
            flags_ath: 0x15,
            bitrate: 64,
            delay: 576,
            padding: 1_152,
            misc: 0x4D,
            mp3_gain: 0,
            preset: 0x01E0,
            music_length: 2_592,
            music_crc: 0x1A7E,
        }
    }

    fn info() -> build::Xing {
        build::Xing {
            info: true,
            frames: Some(100),
            bytes: Some(2_560),
            toc: Some(counting_toc()),
            quality: Some(57),
        }
    }

    /// An Info frame with `xing` and `lame`, then 100 small frames.
    fn encoded(xing: &build::Xing, lame: Option<&build::Lame>) -> Vec<u8> {
        let mut file = build::xing_frame(&roomy(), xing, lame);
        file.extend(build::stream(&[small(); 100]));
        file
    }

    fn info_read() -> Xing {
        Xing {
            info: true,
            frames: Some(100),
            bytes: Some(2_560),
            toc: Some(counting_toc()),
            quality: Some(57),
        }
    }

    fn lame_read() -> LameTag {
        LameTag {
            encoder: *b"LAME3.100",
            delay: 576,
            padding: 1_152,
            music_length: 2_592,
            replay_gain: Ok(ReplayGain {
                peak: Some(PeakRatio::new(0.988_281_25)),
                track: Some(GainDb::new(6.7).unwrap()),
                album: Some(GainDb::new(-6.5).unwrap()),
            }),
        }
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn reads_the_info_header_and_lame_extension_and_seeks_by_the_table_of_contents() {
        // 100 frames of 576 samples in 2,560 octets: entry i of the table
        // is i, so i% of the duration starts at 2,560 * i / 256 octets.
        assert_eq!(
            parse(&encoded(&info(), Some(&lame()))),
            Ok(Ok(MpegStream {
                start: 0,
                header: roomy_header(),
                xing: Some(Ok(info_read())),
                lame: Some(Ok(lame_read())),
                vbri: None,
                frames: 100,
                end: 2_560,
                seek: SeekIndex {
                    points: (0..100)
                        .map(|entry| point(576 * entry, 10 * entry))
                        .collect(),
                    stride: 1,
                },
            }))
        );
    }

    /// The stream an [`encoded`] file gives when its seek index comes from
    /// scanning the 100 small frames after the Info frame.
    fn scanned(xing: Xing, lame: Option<Result<LameTag, ParseFault>>) -> MpegStream {
        MpegStream {
            start: 0,
            header: roomy_header(),
            xing: Some(Ok(xing)),
            lame,
            vbri: None,
            frames: 100,
            end: 2_592,
            seek: every_frame(100, 192, 24),
        }
    }

    /// Verifies: SEC-MED-008
    #[test]
    fn scans_instead_when_the_table_of_contents_cannot_be_used() {
        // The table needs a frame count above zero, and a byte count that
        // covers the first frame and stays inside the file.
        let cases = [
            Xing {
                frames: None,
                ..info_read()
            },
            Xing {
                frames: Some(0),
                ..info_read()
            },
            Xing {
                bytes: None,
                ..info_read()
            },
            Xing {
                bytes: Some(191),
                ..info_read()
            },
            Xing {
                bytes: Some(2_593),
                ..info_read()
            },
            Xing {
                bytes: Some(u32::MAX),
                ..info_read()
            },
            Xing {
                toc: None,
                ..info_read()
            },
        ];
        for xing in cases {
            let written = build::Xing {
                info: xing.info,
                frames: xing.frames,
                bytes: xing.bytes,
                toc: xing.toc,
                quality: xing.quality,
            };
            assert_eq!(
                parse(&encoded(&written, None)),
                Ok(Ok(scanned(xing.clone(), None))),
                "{xing:?}"
            );
        }
    }

    #[test]
    fn seeks_by_a_table_of_contents_that_exactly_fits() {
        // One frame and a byte count of exactly the first frame; then a
        // byte count that ends exactly at the end of the file.
        let xing = |frames, bytes| build::Xing {
            info: true,
            frames: Some(frames),
            bytes: Some(bytes),
            toc: Some([0; 100]),
            quality: None,
        };
        let points = |offset| SeekIndex {
            points: (0..100)
                .map(|entry| point(576 * entry / 100, offset))
                .collect(),
            stride: 1,
        };
        let read = |frames, bytes| Xing {
            info: true,
            frames: Some(frames),
            bytes: Some(bytes),
            toc: Some([0; 100]),
            quality: None,
        };
        assert_eq!(
            parse(&encoded(&xing(1, 192), None)),
            Ok(Ok(MpegStream {
                frames: 1,
                end: 192,
                seek: points(0),
                ..scanned(read(1, 192), None)
            }))
        );
        let mut file = repeated(0, 3);
        file.extend(encoded(&xing(1, 2_592), None));
        assert_eq!(
            parse(&file),
            Ok(Ok(MpegStream {
                start: 3,
                frames: 1,
                end: 2_595,
                seek: points(3),
                ..scanned(read(1, 2_592), None)
            }))
        );
    }

    #[test]
    fn keeps_a_table_of_contents_seek_index_in_order() {
        // Entries that fall back are raised to the entry before them.
        let mut toc = [0; 100];
        toc[0] = 10;
        toc[1] = 5;
        toc[2] = 128;
        toc[3] = 64;
        toc[4] = 255;
        let xing = build::Xing {
            info: false,
            frames: Some(100),
            bytes: Some(2_560),
            toc: Some(toc),
            quality: None,
        };
        let parsed = parse(&encoded(&xing, None)).unwrap().unwrap();
        let offsets: Vec<u64> = parsed
            .seek
            .points
            .iter()
            .map(|point| point.offset)
            .collect();
        let mut expected = vec![100, 100, 1_280, 1_280];
        expected.extend([2_550; 96]);
        assert_eq!(offsets, expected);
    }

    #[test]
    fn finds_the_xing_header_where_each_version_and_mode_ends_its_side_information() {
        // MPEG-1 stereo and mono, MPEG-2 stereo and MPEG 2.5 mono: the
        // header starts 36, 21, 21 and 13 octets into the frame.
        let frames = [
            (
                build::Frame::layer3(build::Version::Mpeg1, 9, 0, build::Mode::Stereo),
                417_u64,
            ),
            (
                build::Frame::layer3(build::Version::Mpeg1, 9, 0, build::Mode::Mono),
                417,
            ),
            (
                build::Frame::layer3(build::Version::Mpeg2, 8, 1, build::Mode::JointStereo),
                192,
            ),
            (
                build::Frame::layer3(build::Version::Mpeg25, 4, 2, build::Mode::Mono),
                288,
            ),
        ];
        let xing = build::Xing {
            info: false,
            frames: Some(7),
            bytes: None,
            toc: None,
            quality: None,
        };
        for (frame, len) in frames {
            let mut file = build::xing_frame(&frame, &xing, None);
            file.extend(build::stream(&[frame; 2]));
            let parsed = parse(&file).unwrap().unwrap();
            let samples = u64::from(parsed.header.samples);
            assert_eq!(
                (parsed.xing, parsed.frames, parsed.end, parsed.seek),
                (
                    Some(Ok(Xing {
                        info: false,
                        frames: Some(7),
                        bytes: None,
                        toc: None,
                        quality: None,
                    })),
                    2,
                    3 * len,
                    SeekIndex {
                        points: vec![point(0, len), point(samples, 2 * len)],
                        stride: 1,
                    },
                ),
                "{frame:?}"
            );
        }
    }

    #[test]
    fn finds_xing_at_the_lame_offset_when_the_frame_declares_a_crc() {
        // LAME overwrites the CRC slot: Xing starts at 4 + side_info_len
        // even when the protection bit is clear (`lame --protect`).
        let frame = build::Frame {
            crc: true,
            ..roomy()
        };
        let header = FrameHeader {
            crc: true,
            ..roomy_header()
        };
        let mut file = build::xing_frame(&frame, &info(), Some(&lame()));
        file.extend(build::stream(&[small(); 100]));
        assert_eq!(
            parse(&file),
            Ok(Ok(MpegStream {
                start: 0,
                header,
                xing: Some(Ok(info_read())),
                lame: Some(Ok(lame_read())),
                vbri: None,
                frames: 100,
                end: 2_560,
                seek: SeekIndex {
                    points: (0..100)
                        .map(|entry| point(576 * entry, 10 * entry))
                        .collect(),
                    stride: 1,
                },
            }))
        );
        // The ISO offset for a protected MPEG-2 mono frame is two CRC
        // octets plus nine of side information. A Xing written there is
        // not the encoder header LAME writes.
        let mut body = repeated(0, 11);
        body.extend(info().write());
        let mut iso = frame.write(&body);
        iso.extend(build::stream(&[small(); 2]));
        assert_eq!(
            parse(&iso),
            Ok(Ok(plain(
                0,
                header,
                3,
                240,
                SeekIndex {
                    points: vec![point(0, 0), point(576, 192), point(1_152, 216)],
                    stride: 1,
                },
            )))
        );
    }

    #[test]
    fn reads_an_info_header_with_zero_frames() {
        let xing = build::Xing {
            info: true,
            frames: Some(0),
            bytes: Some(192),
            toc: Some(counting_toc()),
            quality: None,
        };
        assert_eq!(
            parse(&build::xing_frame(&roomy(), &xing, None)),
            Ok(Ok(MpegStream {
                start: 0,
                header: roomy_header(),
                xing: Some(Ok(Xing {
                    info: true,
                    frames: Some(0),
                    bytes: Some(192),
                    toc: Some(counting_toc()),
                    quality: None,
                })),
                lame: None,
                vbri: None,
                frames: 0,
                end: 192,
                seek: SeekIndex {
                    points: vec![],
                    stride: 1,
                },
            }))
        );
    }

    /// A frame of `frame`'s length that holds `body` after its header.
    fn frame_holding(frame: build::Frame, body: &[u8]) -> Vec<u8> {
        frame.write(body)
    }

    #[test]
    fn reports_a_xing_header_cut_short_by_its_frame() {
        // MPEG-2 stereo at 8 kbit/s and 22.05 kHz: 26 octets, of which
        // the flags would take 25 to 28.
        let narrow = build::Frame::layer3(build::Version::Mpeg2, 1, 0, build::Mode::Stereo);
        let mut body = repeated(0, 17);
        body.extend(b"Xing\x00");
        let mut file = frame_holding(narrow, &body);
        file.extend(build::stream(&[narrow; 2]));
        let parsed = parse(&file).unwrap().unwrap();
        assert_eq!(
            (parsed.xing, parsed.lame, parsed.frames),
            (
                Some(Err(ParseFault::Truncated {
                    offset: 25,
                    needed: 4,
                    available: 1,
                })),
                None,
                2
            )
        );
        // The frame count the flags announce runs from 21 to 25 in a
        // 24-octet frame.
        let mut body = repeated(0, 9);
        body.extend(b"Xing\x00\x00\x00\x01\x00\x00\x00");
        let mut file = frame_holding(small(), &body);
        file.extend(build::stream(&[small(); 2]));
        let parsed = parse(&file).unwrap().unwrap();
        assert_eq!(
            (parsed.xing, parsed.frames, parsed.seek),
            (
                Some(Err(ParseFault::Truncated {
                    offset: 21,
                    needed: 4,
                    available: 3,
                })),
                2,
                every_frame(2, 24, 24)
            )
        );
    }

    #[test]
    fn reports_a_lame_extension_cut_short_by_its_frame() {
        // MPEG-2 mono at 56 kbit/s: 168 octets. The Info header ends at
        // 133, and the extension would need 36 octets from there.
        let frame = build::Frame::layer3(build::Version::Mpeg2, 7, 1, build::Mode::Mono);
        let mut file = build::xing_frame(&frame, &info(), None);
        file[133..137].copy_from_slice(b"LAME");
        file.extend(build::stream(&[small(); 2]));
        let parsed = parse(&file).unwrap().unwrap();
        assert_eq!(
            parsed.lame,
            Some(Err(ParseFault::Truncated {
                offset: 133,
                needed: 36,
                available: 35,
            }))
        );
    }

    #[test]
    fn reads_no_lame_extension_where_fewer_than_four_octets_follow_the_xing_header() {
        // MPEG-2 stereo at 8 kbit/s and 16 kHz: 36 octets. A Xing header
        // with a frame count ends at 33.
        let frame = build::Frame::layer3(build::Version::Mpeg2, 1, 2, build::Mode::Stereo);
        let xing = build::Xing {
            info: false,
            frames: Some(2),
            bytes: None,
            toc: None,
            quality: None,
        };
        let mut file = build::xing_frame(&frame, &xing, None);
        file.extend(build::stream(&[frame; 2]));
        let parsed = parse(&file).unwrap().unwrap();
        assert_eq!(
            (
                parsed.xing.map(|xing| xing.map(|xing| xing.frames)),
                parsed.lame
            ),
            (Some(Ok(Some(2))), None)
        );
    }

    #[test]
    fn recognises_the_extension_lame_and_ffmpeg_write_and_nothing_else() {
        for (encoder, recognised) in [
            (*b"Lavf61.7\x00", true),
            (*b"Lavc61.19", true),
            (*b"GOGO3.13a", false),
            (*b"LAMB3.100", false),
        ] {
            let tag = build::Lame { encoder, ..lame() };
            let parsed = parse(&encoded(&info(), Some(&tag))).unwrap().unwrap();
            let expected = recognised.then(|| {
                Ok(LameTag {
                    encoder,
                    ..lame_read()
                })
            });
            assert_eq!(parsed.lame, expected, "{encoder:?}");
        }
    }

    /// The LAME extension `parse` reads from an [`encoded`] file whose tag
    /// is `tag`.
    fn lame_of(tag: &build::Lame) -> Option<Result<LameTag, ParseFault>> {
        parse(&encoded(&info(), Some(tag))).unwrap().unwrap().lame
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn drops_the_replay_gain_of_a_lame_extension_whose_crc_does_not_match() {
        let mut file = encoded(&info(), Some(&lame()));
        let stored = build::crc16_arc(&file[..167]);
        // Change the music CRC, which the tag CRC covers.
        file[165] ^= 0x01;
        let computed = build::crc16_arc(&file[..167]);
        assert_ne!(stored, computed);
        let parsed = parse(&file).unwrap().unwrap();
        assert_eq!(
            parsed.lame,
            Some(Ok(LameTag {
                replay_gain: Err(CrcMismatch { stored, computed }),
                ..lame_read()
            }))
        );
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn reads_a_peak_up_to_16_and_drops_one_beyond() {
        let peak_of = |raw| {
            lame_of(&build::Lame {
                peak: raw,
                ..lame()
            })
            .unwrap()
            .unwrap()
            .replay_gain
            .unwrap()
            .peak
        };
        // Zero means the encoder did not measure one.
        assert_eq!(peak_of(0), None);
        assert_eq!(peak_of(1 << 23), Some(PeakRatio::new(1.0)));
        assert_eq!(peak_of(1 << 27), Some(PeakRatio::new(16.0)));
        assert_eq!(
            peak_of((1 << 27) + 16),
            Some(Err(ValueError::Unusable {
                field: crate::values::Field::Peak,
            }))
        );
        assert_eq!(
            peak_of(u32::MAX),
            Some(Err(ValueError::Unusable {
                field: crate::values::Field::Peak,
            }))
        );
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn reads_each_gain_only_from_its_own_slot() {
        let gains_of = |track, album| {
            let gain = lame_of(&build::Lame {
                track_gain: track,
                album_gain: album,
                ..lame()
            })
            .unwrap()
            .unwrap()
            .replay_gain
            .unwrap();
            (gain.track, gain.album)
        };
        let db = |value| Some(GainDb::new(value).unwrap());
        // The name code says which gain a slot holds; the originator does
        // not matter.
        assert_eq!(
            gains_of(
                build::replay_gain(1, 0, -511),
                build::replay_gain(2, 7, 511)
            ),
            (db(-51.1), db(51.1))
        );
        assert_eq!(gains_of(build::replay_gain(1, 1, 0), 0), (db(0.0), None));
        assert_eq!(
            gains_of(build::replay_gain(2, 3, 30), build::replay_gain(1, 3, 30)),
            (None, None)
        );
        assert_eq!(
            gains_of(build::replay_gain(0, 3, 30), build::replay_gain(3, 3, 30)),
            (None, None)
        );
    }

    #[test]
    fn reads_a_vbri_header_and_scans_the_frames_after_it() {
        let lame_default =
            build::Frame::layer3(build::Version::Mpeg1, 9, 0, build::Mode::JointStereo);
        let vbri = build::Vbri {
            version: 1,
            delay: 0x0123,
            quality: 75,
            bytes: 1_251,
            frames: 3,
            scale: 1,
            frames_per_entry: 1,
            table: vec![417, 417, 417],
        };
        let mut file = build::vbri_frame(&lame_default, &vbri);
        file.extend(build::stream(&[lame_default; 2]));
        let header = FrameHeader {
            version: Version::Mpeg1,
            layer: Layer::III,
            crc: false,
            bitrate: 128,
            sample_rate: hz(44_100),
            padding: false,
            mode: ChannelMode::JointStereo,
            samples: 1_152,
            len: 417,
        };
        assert_eq!(
            parse(&file),
            Ok(Ok(MpegStream {
                start: 0,
                header,
                xing: None,
                lame: None,
                vbri: Some(Ok(Vbri {
                    version: 1,
                    quality: 75,
                    bytes: 1_251,
                    frames: 3,
                })),
                frames: 2,
                end: 1_251,
                seek: SeekIndex {
                    points: vec![point(0, 417), point(1_152, 834)],
                    stride: 1,
                },
            }))
        );
    }

    #[test]
    fn reports_a_vbri_header_cut_short_by_its_frame() {
        // 48 octets: the fields after the name need 40 to 58.
        let mut body = repeated(0, 32);
        body.extend(b"VBRI");
        let mut file = frame_holding(middling(), &body);
        file.extend(build::stream(&[middling()]));
        let parsed = parse(&file).unwrap().unwrap();
        assert_eq!(
            (parsed.vbri, parsed.frames),
            (
                Some(Err(ParseFault::Truncated {
                    offset: 40,
                    needed: 18,
                    available: 8,
                })),
                1
            )
        );
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn thins_a_scanned_seek_index_to_the_index_entry_limit() {
        let four = limited(LimitKind::IndexEntries, 4);
        assert_eq!(
            parse_with(&build::stream(&[small(); 4]), &four),
            Ok(Ok(plain(0, small_header(), 4, 96, every_frame(4, 0, 24))))
        );
        assert_eq!(
            parse_with(&build::stream(&[small(); 5]), &four),
            Ok(Ok(plain(
                0,
                small_header(),
                5,
                120,
                SeekIndex {
                    points: vec![point(0, 0), point(1_152, 48), point(2_304, 96)],
                    stride: 2,
                }
            )))
        );
        assert_eq!(
            parse_with(
                &build::stream(&[small(); 5]),
                &limited(LimitKind::IndexEntries, 0)
            ),
            Ok(Ok(plain(
                0,
                small_header(),
                5,
                120,
                SeekIndex {
                    points: vec![],
                    stride: 1,
                }
            )))
        );
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn thins_a_table_of_contents_seek_index_to_the_index_entry_limit() {
        let parsed = parse_with(
            &encoded(&info(), Some(&lame())),
            &limited(LimitKind::IndexEntries, 4),
        );
        assert_eq!(
            parsed.map(|stream| stream.map(|stream| stream.seek)),
            Ok(Ok(SeekIndex {
                points: vec![
                    point(0, 0),
                    point(18_432, 320),
                    point(36_864, 640),
                    point(55_296, 960)
                ],
                stride: 32,
            }))
        );
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn runs_out_of_budget_at_exactly_the_step_it_should() {
        // Three small frames take six steps: the search window, the octet
        // at 0, the confirming header at 24, the first frame, and the
        // headers at 24 and 48.
        let file = build::stream(&[small(); 3]);
        for (steps, offset) in [0, 0, 24, 0, 24, 48].into_iter().enumerate() {
            assert_eq!(
                parse_spending(&file, u64::try_from(steps).unwrap()),
                Ok(Err(MpaError::Fault(ParseFault::BudgetExceeded { offset }))),
                "{steps} steps"
            );
        }
        assert_eq!(
            parse_spending(&file, 6),
            Ok(Ok(plain(0, small_header(), 3, 72, every_frame(3, 0, 24))))
        );
        // A tag, two octets of junk and a lone frame: the first window, the
        // window after the tag, three octets, then the frame.
        let mut file = build::id3v2_tag(&[], false);
        file.extend([0, 0]);
        file.extend(build::stream(&[small()]));
        for (steps, offset) in [0, 10, 10, 11, 12, 12].into_iter().enumerate() {
            assert_eq!(
                parse_spending(&file, u64::try_from(steps).unwrap()),
                Ok(Err(MpaError::Fault(ParseFault::BudgetExceeded { offset }))),
                "{steps} steps"
            );
        }
        assert_eq!(
            parse_spending(&file, 6),
            Ok(Ok(plain(12, small_header(), 1, 36, every_frame(1, 12, 24))))
        );
        // The table of contents takes one step an entry: 104 in all.
        let file = encoded(&info(), Some(&lame()));
        assert_eq!(
            parse_spending(&file, 103),
            Ok(Err(MpaError::Fault(ParseFault::BudgetExceeded {
                offset: 0
            })))
        );
        assert_eq!(
            parse_spending(&file, 104).map(|stream| stream.map(|stream| stream.frames)),
            Ok(Ok(100))
        );
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn examines_each_octet_once_across_search_windows() {
        // 4,100 zeros, then two small frames. The first window examines
        // octets 0 to 4,092; the second starts at 4,093, three octets
        // before the first window's end, and examines 4,093 to the header
        // at 4,100. With the reads of each window, the confirming header,
        // the first frame and the second frame's header: 4,106 steps.
        let mut file = repeated(0, 4_100);
        file.extend(build::stream(&[small(); 2]));
        assert_eq!(
            parse_spending(&file, 4_105),
            Ok(Err(MpaError::Fault(ParseFault::BudgetExceeded {
                offset: 4_124,
            })))
        );
        assert_eq!(
            parse_spending(&file, 4_106),
            Ok(Ok(plain(
                4_100,
                small_header(),
                2,
                4_148,
                every_frame(2, 4_100, 24)
            )))
        );
    }

    /// Verifies: SEC-MED-008, SEC-MED-010
    #[test]
    fn keeps_moving_when_a_read_may_hold_fewer_than_four_octets() {
        assert_eq!(
            parse_with(
                &build::stream(&[small(); 3]),
                &limited(LimitKind::ReadBytes, 3)
            ),
            Ok(Err(MpaError::NoFrames { offset: 0 }))
        );
        // Four octets a read find the frame, but cannot read it whole.
        assert_eq!(
            parse_with(
                &build::stream(&[small(); 3]),
                &limited(LimitKind::ReadBytes, 4)
            ),
            Err(DriveError::TooLong {
                offset: 0,
                len: 24,
                max: 4,
            })
        );
    }

    /// The file every cut is taken from: a 16-octet tag, an Info frame
    /// whose byte count covers the rest of the file, and three small
    /// frames.
    fn cut_source() -> Vec<u8> {
        let toc: [u8; 100] = core::array::from_fn(|entry| u8::try_from(2 * entry).unwrap());
        let xing = build::Xing {
            info: true,
            frames: Some(3),
            bytes: Some(264),
            toc: Some(toc),
            quality: None,
        };
        let mut file = build::id3v2_tag(&[0; 6], false);
        file.extend(build::xing_frame(&roomy(), &xing, None));
        file.extend(build::stream(&[small(); 3]));
        file
    }

    /// What a parse of the first `cut` octets of [`cut_source`] must give,
    /// worked out from the file's layout alone.
    fn cut_model(cut: u64) -> Result<MpegStream, MpaError> {
        let toc: [u8; 100] = core::array::from_fn(|entry| u8::try_from(2 * entry).unwrap());
        let xing = Xing {
            info: true,
            frames: Some(3),
            bytes: Some(264),
            toc: Some(toc),
            quality: None,
        };
        let stream = |frames, end, seek| MpegStream {
            start: 16,
            header: roomy_header(),
            xing: Some(Ok(xing.clone())),
            lame: None,
            vbri: None,
            frames,
            end,
            seek,
        };
        match cut {
            // Too short for an ID3v2 header, and holding no frame.
            0..10 => Err(MpaError::NoFrames { offset: 0 }),
            10..16 => Err(MpaError::Fault(ParseFault::Truncated {
                offset: 0,
                needed: 16,
                available: cut,
            })),
            16..20 => Err(MpaError::NoFrames { offset: 16 }),
            20..208 => Err(MpaError::Fault(ParseFault::Truncated {
                offset: 16,
                needed: 192,
                available: cut - 16,
            })),
            // Only the whole file holds the 264 octets the Info header
            // counts, so only it seeks by the table of contents.
            280 => Ok(stream(
                3,
                280,
                SeekIndex {
                    points: (0..100)
                        .map(|entry| point(3 * 576 * entry / 100, 16 + 2 * entry * 264 / 256))
                        .collect(),
                    stride: 1,
                },
            )),
            _ => {
                let frames = (cut - 208) / 24;
                Ok(stream(
                    frames,
                    208 + 24 * frames,
                    every_frame(frames, 208, 24),
                ))
            }
        }
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn every_cut_of_a_valid_file_parses_as_its_layout_says() {
        let file = cut_source();
        assert_eq!(file.len(), 280);
        for cut in 0..=280 {
            assert_eq!(
                parse(&file[..cut]),
                Ok(cut_model(u64::try_from(cut).unwrap())),
                "cut at {cut}"
            );
        }
    }

    /// Runs `work` on a thread with the 256 KiB stack SEC-MED-001 names.
    fn on_small_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(262_144)
            .spawn(work)
            .unwrap()
            .join()
            .unwrap()
    }

    /// Verifies: SEC-MED-001, SEC-MED-008
    #[test]
    fn walks_long_runs_of_tags_and_frames_without_recursing() {
        // MPEG audio does not nest; a thousand tags and four thousand
        // frames are walked in a loop, on a small stack.
        let mut file = Vec::new();
        for _ in 0..1_000 {
            file.extend(build::id3v2_tag(&[], false));
        }
        file.extend(build::stream(&repeated(small(), 4_000)));
        let parsed = on_small_stack(move || parse(&file));
        assert_eq!(
            parsed,
            Ok(Ok(plain(
                10_000,
                small_header(),
                4_000,
                106_000,
                every_frame(4_000, 10_000, 24)
            )))
        );
    }

    /// One piece of an adversarial file.
    fn piece() -> impl Strategy<Value = Vec<u8>> {
        prop_oneof![
            // Headers whose frames are never confirmed, back to back.
            (1_usize..64).prop_map(|count| repeated(small().header(), count).concat()),
            // Empty ID3v2 tags, back to back.
            (1_usize..64).prop_map(|count| repeated(build::id3v2_tag(&[], false), count).concat()),
            // Valid frames.
            (1_usize..8).prop_map(|count| build::stream(&repeated(small(), count))),
            Just(free_format().to_vec()),
            proptest::collection::vec(any::<u8>(), 0..64),
            (1_usize..300).prop_map(|count| repeated(0xFF, count)),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        /// Verifies: SEC-MED-001, SEC-MED-007, SEC-MED-010, SEC-TM-032, SEC-HIS-036
        #[test]
        fn returns_within_its_budget_for_any_file(
            pieces in proptest::collection::vec(piece(), 0..12),
        ) {
            let file: Vec<u8> = pieces.concat();
            let parsed = on_small_stack(move || parse(&file));
            // The driver refused no read, and the documented budget was
            // enough.
            prop_assert!(parsed.is_ok(), "{:?}", parsed);
            let over_budget = matches!(
                parsed,
                Ok(Err(MpaError::Fault(ParseFault::BudgetExceeded { .. })))
            );
            prop_assert!(!over_budget);
        }

        /// Verifies: SEC-MED-001, SEC-HIS-036
        #[test]
        fn returns_for_any_bytes(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
            let parsed = on_small_stack(move || parse(&bytes));
            prop_assert!(parsed.is_ok(), "{:?}", parsed);
        }
    }

    /// The seek index a scan keeps of `count` frames under a limit of
    /// `max` points: every frame at the smallest power-of-two stride that
    /// keeps the count within the limit, and nothing when the limit is
    /// zero. `points(frame)` is the point of frame `frame`.
    fn thinned(count: u64, max: u64, points: impl Fn(u64) -> SeekPoint) -> SeekIndex {
        if max == 0 || count == 0 {
            return SeekIndex {
                points: vec![],
                stride: 1,
            };
        }
        let mut stride = 1;
        while count.div_ceil(stride) > max {
            stride *= 2;
        }
        SeekIndex {
            points: (0..count)
                .step_by(usize::try_from(stride).unwrap())
                .map(points)
                .collect(),
            stride,
        }
    }

    #[test]
    fn the_thinning_model_keeps_the_points_worked_out_by_hand() {
        let at = |rank: u64| point(rank, rank);
        assert_eq!(thinned(10, 1, at).points, vec![at(0)]);
        assert_eq!(thinned(10, 1, at).stride, 16);
        assert_eq!(thinned(5, 4, at).points, vec![at(0), at(2), at(4)]);
        assert_eq!(thinned(7, 3, at).points, vec![at(0), at(4)]);
        assert_eq!(thinned(4, 4, at).stride, 1);
        assert_eq!(thinned(0, 4, at).stride, 1);
        assert_eq!(thinned(9, 0, at).points, vec![]);
    }

    /// One valid MPEG Layer III stream, as a generator lays it out.
    #[derive(Debug, Clone)]
    struct Layout {
        zeros: usize,
        tags: Vec<usize>,
        frames: Vec<build::Frame>,
        trailer: bool,
        max: u64,
    }

    fn layout() -> impl Strategy<Value = Layout> {
        let stream = (
            prop_oneof![
                Just(build::Version::Mpeg1),
                Just(build::Version::Mpeg2),
                Just(build::Version::Mpeg25),
            ],
            0_u8..3,
            prop_oneof![
                Just(build::Mode::Stereo),
                Just(build::Mode::JointStereo),
                Just(build::Mode::DualChannel),
                Just(build::Mode::Mono),
            ],
        );
        (
            stream,
            proptest::collection::vec((1_u8..15, any::<bool>()), 2..24),
            0_usize..600,
            proptest::collection::vec(0_usize..40, 0..3),
            any::<bool>(),
            0_u64..12,
        )
            .prop_map(
                |((version, rate, mode), shapes, zeros, tags, trailer, max)| Layout {
                    zeros,
                    tags,
                    frames: shapes
                        .into_iter()
                        .map(|(bitrate, padding)| build::Frame {
                            padding,
                            ..build::Frame::layer3(version, bitrate, rate, mode)
                        })
                        .collect(),
                    trailer,
                    max,
                },
            )
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        /// Verifies: SEC-MED-001, SEC-MED-006
        #[test]
        fn reads_every_stream_the_testkit_writes(layout in layout()) {
            // Tags open a file and junk follows them; a tag after junk is
            // junk too.
            let mut file = Vec::new();
            for &len in &layout.tags {
                file.extend(build::id3v2_tag(&repeated(0xFF, len), false));
            }
            file.extend(repeated(0, layout.zeros));
            let start = u64::try_from(file.len()).unwrap();
            let mut offsets = Vec::new();
            for frame in &layout.frames {
                offsets.push(u64::try_from(file.len()).unwrap());
                file.extend(frame.write(&[]));
            }
            let end = u64::try_from(file.len()).unwrap();
            if layout.trailer {
                file.extend(b"TAG");
                file.extend([0x20; 125]);
            }
            let count = u64::try_from(layout.frames.len()).unwrap();
            let samples = layout.frames[0].samples();
            let seek = thinned(count, layout.max, |frame| {
                point(samples * frame, offsets[usize::try_from(frame).unwrap()])
            });
            let parsed = parse_with(&file, &limited(LimitKind::IndexEntries, layout.max));
            let parsed = parsed.unwrap().unwrap();
            prop_assert_eq!(
                (parsed.start, parsed.frames, parsed.end, parsed.seek),
                (start, count, end, seek)
            );
            prop_assert_eq!(
                (parsed.xing, parsed.lame, parsed.vbri),
                (None, None, None)
            );
            prop_assert_eq!(
                frame_header(layout.frames[0].header()),
                Ok(parsed.header)
            );
        }
    }
}
