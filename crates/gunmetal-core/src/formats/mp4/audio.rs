//! The audio track: its media header, its first sample entry with the
//! codec configuration inside it, and where its sample tables are.
//!
//! The sample entry is the `AudioSampleEntry` of ISO/IEC 14496-12, section
//! 12.2.3, read as `QuickTime` versions 0 and 1 of the sound description.
//! The configuration boxes are `esds` for `mp4a` (ISO/IEC 14496-14 and
//! 14496-1, with the `AudioSpecificConfig` of ISO/IEC 14496-3), `alac` for
//! `alac` (Apple's ALAC magic cookie), `dfLa` for `fLaC` (FLAC in ISOBMFF)
//! and `dOps` for `Opus` (Opus in ISOBMFF). Values that a later step divides
//! by or decides on become typed values, and one outside its range is
//! dropped with a recorded reason (SEC-MED-014).

use std::num::NonZeroU32;
use std::ops::Range;

use super::boxes::{Children, FourCc, HDLR, Mp4Box, span};
use super::probe::{Mp4Error, Mp4Problem};
use crate::parse::{Budget, Cursor, Limits, ParseFault};
use crate::values::{BitDepth, Channels, Field, GainDb, SampleRate, ValueError};

/// The first audio track of a movie.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioTrack {
    /// Where the `trak` box starts.
    pub offset: u64,
    /// Units of the track's time in one second; never zero.
    pub timescale: NonZeroU32,
    /// How long the track plays, in units of the timescale, unless the
    /// media header says it is unknown.
    pub duration: Option<u64>,
    /// The first sample entry.
    pub entry: AudioEntry,
}

/// An audio sample entry.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioEntry {
    /// The coding name: `mp4a`, `alac`, `fLaC`, `Opus` and so on.
    pub format: FourCc,
    /// The channel count the entry gives.
    pub channels: Option<Channels>,
    /// The sample size the entry gives.
    pub bits: Option<BitDepth>,
    /// The integer part of the sample rate the entry gives. Rates above
    /// 65,535 Hz do not fit; the codec configuration has the real one.
    pub sample_rate: Option<SampleRate>,
    /// The codec configuration.
    pub config: CodecConfig,
}

/// The codec configuration inside a sample entry.
#[derive(Debug, Clone, PartialEq)]
pub enum CodecConfig {
    /// The elementary stream descriptor of an `mp4a` entry.
    Esds(Esds),
    /// The magic cookie of an `alac` entry.
    Alac(Alac),
    /// The STREAMINFO of a `fLaC` entry.
    Flac(Flac),
    /// The Opus specific box of an `Opus` entry.
    Opus(Opus),
    /// No configuration this parser reads: the format is another one, or
    /// its configuration box is missing.
    Missing,
}

/// An elementary stream descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Esds {
    /// The object type indication: 0x40 for MPEG-4 audio, 0x66 to 0x68
    /// for MPEG-2 AAC, 0x69 and 0x6B for MPEG-2 and MPEG-1 audio.
    pub object_type: u8,
    /// The largest bit rate, in bits a second.
    pub max_bitrate: u32,
    /// The average bit rate, in bits a second.
    pub avg_bitrate: u32,
    /// The decoder specific information, when there is any.
    pub specific: Option<Specific>,
}

/// The decoder specific information of an elementary stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Specific {
    /// Where its octets are in the file.
    pub range: Range<u64>,
    /// The `AudioSpecificConfig` they hold, read for the AAC object types.
    pub audio: Option<AudioSpecificConfig>,
}

/// The start of an MPEG-4 `AudioSpecificConfig`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioSpecificConfig {
    /// The audio object type: 2 for AAC LC, 5 for SBR, 29 for PS.
    pub object_type: u8,
    /// The sampling frequency.
    pub sample_rate: Option<SampleRate>,
    /// The channel configuration: 0 when a program config element gives
    /// the channels.
    pub channel_config: u8,
    /// The explicit SBR or PS extension, for object types 5 and 29.
    pub extension: Option<Extension>,
}

/// The explicit extension of an SBR or PS `AudioSpecificConfig`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extension {
    /// The underlying object type, usually 2.
    pub object_type: u8,
    /// The output sampling frequency.
    pub sample_rate: Option<SampleRate>,
}

/// An ALAC magic cookie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alac {
    /// Samples in a frame.
    pub frame_length: u32,
    /// Bits a sample.
    pub bits: Option<BitDepth>,
    /// Channels.
    pub channels: Option<Channels>,
    /// The average bit rate, in bits a second.
    pub avg_bitrate: u32,
    /// The sample rate.
    pub sample_rate: Option<SampleRate>,
    /// Where the cookie's octets are in the file, for a decoder.
    pub cookie: Range<u64>,
}

/// The STREAMINFO block of a FLAC stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flac {
    /// The smallest block, in samples.
    pub min_block: u16,
    /// The largest block, in samples.
    pub max_block: u16,
    /// The sample rate.
    pub sample_rate: Option<SampleRate>,
    /// Channels.
    pub channels: Option<Channels>,
    /// Bits a sample.
    pub bits: Option<BitDepth>,
    /// Samples in the stream, unless STREAMINFO says it is unknown.
    pub total_samples: Option<u64>,
    /// Where the metadata blocks are in the file, for a packager.
    pub blocks: Range<u64>,
}

/// An Opus specific box.
#[derive(Debug, Clone, PartialEq)]
pub struct Opus {
    /// Output channels.
    pub channels: Option<Channels>,
    /// Samples at 48 kHz to drop from the start.
    pub pre_skip: u16,
    /// The sample rate of the original input, which playback ignores; 0
    /// when it was not recorded.
    pub input_sample_rate: u32,
    /// The gain to apply on output.
    pub output_gain: GainDb,
    /// The channel mapping family.
    pub mapping_family: u8,
    /// Where the box's body is in the file, for a packager.
    pub body: Range<u64>,
}

/// Where an audio track's sample tables are, for the sample table reader.
/// Each range is the body of its box, after the box header, so a full box's
/// version and flags come first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SampleTableRanges {
    /// The decoding time to sample box, `stts`.
    pub stts: Option<Range<u64>>,
    /// The sample to chunk box, `stsc`.
    pub stsc: Option<Range<u64>>,
    /// The sample size box.
    pub sizes: Option<SampleSizes>,
    /// The chunk offset box.
    pub offsets: Option<ChunkOffsets>,
}

/// Which sample size box a track has, and where its body is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SampleSizes {
    /// `stsz`, with 32-bit sizes.
    Stsz(Range<u64>),
    /// `stz2`, with compact sizes.
    Stz2(Range<u64>),
}

/// Which chunk offset box a track has, and where its body is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChunkOffsets {
    /// `stco`, with 32-bit offsets.
    Stco(Range<u64>),
    /// `co64`, with 64-bit offsets.
    Co64(Range<u64>),
}

/// What a track holds, gathered while its boxes are walked and read once
/// the walk leaves it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TrackParts<'a> {
    /// Where the `trak` box starts.
    pub(crate) offset: u64,
    /// The handler type, `soun` for audio.
    pub(crate) handler: Option<FourCc>,
    /// The media header box.
    mdhd: Option<Mp4Box<'a>>,
    /// The sample description box.
    stsd: Option<Mp4Box<'a>>,
    /// Where the sample tables are.
    pub(crate) tables: SampleTableRanges,
}

impl<'a> TrackParts<'a> {
    /// The parts of the track whose `trak` box starts at `offset`.
    pub(crate) fn new(offset: u64) -> Self {
        Self {
            offset,
            ..Self::default()
        }
    }

    /// Takes in a box found directly inside the track's `mdia` box: the
    /// media header is kept for later, and the handler type read now.
    ///
    /// # Errors
    ///
    /// [`ParseFault::Truncated`] for a handler box too short to name its
    /// type.
    pub(crate) fn media(&mut self, child: Mp4Box<'a>) -> Result<(), Mp4Error> {
        match child.kind {
            MDHD => self.mdhd = Some(child),
            HDLR => {
                let mut body = child.body;
                body.skip(8)?; // version, flags and pre_defined
                self.handler = Some(FourCc(body.array()?));
            }
            _ => {}
        }
        Ok(())
    }

    /// Takes in a box found directly inside the track's sample table. Of
    /// two boxes of one kind, the last is kept.
    pub(crate) fn table(&mut self, child: Mp4Box<'a>) {
        let range = span(&child.body);
        let tables = &mut self.tables;
        match child.kind {
            STSD => self.stsd = Some(child),
            STTS => tables.stts = Some(range),
            STSC => tables.stsc = Some(range),
            STSZ => tables.sizes = Some(SampleSizes::Stsz(range)),
            STZ2 => tables.sizes = Some(SampleSizes::Stz2(range)),
            STCO => tables.offsets = Some(ChunkOffsets::Stco(range)),
            CO64 => tables.offsets = Some(ChunkOffsets::Co64(range)),
            _ => {}
        }
    }

    /// Whether the handler says this is an audio track.
    pub(crate) fn is_audio(&self) -> bool {
        self.handler == Some(SOUN)
    }

    /// Reads the audio track these parts describe.
    ///
    /// # Errors
    ///
    /// [`Mp4Error::Missing`] when the media header or the sample
    /// description is missing, and the errors of reading either.
    pub(crate) fn audio(
        self,
        limits: &Limits,
        budget: &mut Budget,
        problems: &mut Vec<Mp4Problem>,
    ) -> Result<(AudioTrack, SampleTableRanges), Mp4Error> {
        let missing = |kind| Mp4Error::Missing {
            kind,
            offset: self.offset,
        };
        let (timescale, duration) = media_header(self.mdhd.ok_or(missing(MDHD))?)?;
        let entry = sample_entry(self.stsd.ok_or(missing(STSD))?, limits, budget, problems)?;
        let track = AudioTrack {
            offset: self.offset,
            timescale,
            duration,
            entry,
        };
        Ok((track, self.tables))
    }
}

/// The handler type of audio tracks.
const SOUN: FourCc = FourCc(*b"soun");
/// The media header box.
const MDHD: FourCc = FourCc(*b"mdhd");
/// The sample description box.
const STSD: FourCc = FourCc(*b"stsd");
/// The decoding time to sample box.
const STTS: FourCc = FourCc(*b"stts");
/// The sample to chunk box.
const STSC: FourCc = FourCc(*b"stsc");
/// The sample size box.
const STSZ: FourCc = FourCc(*b"stsz");
/// The compact sample size box.
const STZ2: FourCc = FourCc(*b"stz2");
/// The chunk offset box.
const STCO: FourCc = FourCc(*b"stco");
/// The 64-bit chunk offset box.
const CO64: FourCc = FourCc(*b"co64");
/// The MPEG-4 audio sample entry.
const MP4A: FourCc = FourCc(*b"mp4a");
/// The elementary stream descriptor box.
const ESDS: FourCc = FourCc(*b"esds");
/// The ALAC sample entry, and the box holding its magic cookie.
const ALAC: FourCc = FourCc(*b"alac");
/// The FLAC sample entry.
const FLAC: FourCc = FourCc(*b"fLaC");
/// The FLAC specific box.
const DFLA: FourCc = FourCc(*b"dfLa");
/// The Opus sample entry.
const OPUS: FourCc = FourCc(*b"Opus");
/// The Opus specific box.
const DOPS: FourCc = FourCc(*b"dOps");

/// Reads a media header: the timescale and the duration.
fn media_header(mdhd: Mp4Box<'_>) -> Result<(NonZeroU32, Option<u64>), Mp4Error> {
    let mut body = mdhd.body;
    let version_at = body.offset();
    let version = body.u8()?;
    body.skip(3)?; // flags
    let (timescale_at, timescale, duration) = match version {
        0 => {
            body.skip(8)?; // creation and modification times
            let at = body.offset();
            let timescale = body.u32_be()?;
            let duration = body.u32_be()?;
            (
                at,
                timescale,
                (duration != u32::MAX).then_some(u64::from(duration)),
            )
        }
        1 => {
            body.skip(16)?; // creation and modification times
            let at = body.offset();
            let timescale = body.u32_be()?;
            let duration = body.u64_be()?;
            (at, timescale, (duration != u64::MAX).then_some(duration))
        }
        found => {
            return Err(Mp4Error::Unexpected {
                kind: MDHD,
                offset: version_at,
                found: u64::from(found),
            });
        }
    };
    let timescale = NonZeroU32::new(timescale).ok_or(Mp4Error::ZeroTimescale {
        offset: timescale_at,
    })?;
    Ok((timescale, duration))
}

/// A sample rate, or `None` with the reason recorded.
fn sample_rate(hz: u32, offset: u64, problems: &mut Vec<Mp4Problem>) -> Option<SampleRate> {
    SampleRate::new(hz)
        .map_err(|error| problems.push(Mp4Problem::Value { offset, error }))
        .ok()
}

/// A channel count, or `None` with the reason recorded.
fn channels(count: u32, offset: u64, problems: &mut Vec<Mp4Problem>) -> Option<Channels> {
    Channels::new(count)
        .map_err(|error| problems.push(Mp4Problem::Value { offset, error }))
        .ok()
}

/// A bit depth, or `None` with the reason recorded.
fn bit_depth(bits: u32, offset: u64, problems: &mut Vec<Mp4Problem>) -> Option<BitDepth> {
    BitDepth::new(bits)
        .map_err(|error| problems.push(Mp4Problem::Value { offset, error }))
        .ok()
}

/// Reads the first sample entry of a sample description box, with the
/// configuration box its format names. The entry count is not read: only
/// the first entry is, whatever the count says.
///
/// # Errors
///
/// [`Mp4Error::Unexpected`] for a sound description version above 1, and
/// the errors of reading the entry, its children and its configuration.
pub(crate) fn sample_entry(
    stsd: Mp4Box<'_>,
    limits: &Limits,
    budget: &mut Budget,
    problems: &mut Vec<Mp4Problem>,
) -> Result<AudioEntry, Mp4Error> {
    let mut body = stsd.body;
    body.skip(8)?; // version, flags and entry count
    let entry = Children::new(body, stsd.depth).required(limits, budget)?;
    let mut fields = entry.body;
    fields.skip(8)?; // reserved and data reference index
    let version_at = fields.offset();
    let version = fields.u16_be()?;
    fields.skip(6)?; // revision level and vendor
    let count = fields.u16_be()?;
    let bits = fields.u16_be()?;
    fields.skip(4)?; // compression ID and packet size
    let hz = fields.u16_be()?;
    fields.skip(2)?; // the fraction of the rate
    match version {
        0 => {}
        1 => fields.skip(16)?, // samples per packet, bytes per packet, frame and sample
        found => {
            return Err(Mp4Error::Unexpected {
                kind: entry.kind,
                offset: version_at,
                found: u64::from(found),
            });
        }
    }
    let mut config = CodecConfig::Missing;
    let mut children = Children::new(fields, entry.depth);
    while let Some(child) = children.next(limits, budget)? {
        config = match (entry.kind, child.kind) {
            (MP4A, ESDS) => CodecConfig::Esds(esds(child, budget, problems)?),
            (ALAC, ALAC) => CodecConfig::Alac(alac(child, problems)?),
            (FLAC, DFLA) => CodecConfig::Flac(dfla(child, problems)?),
            (OPUS, DOPS) => CodecConfig::Opus(dops(child, problems)?),
            _ => config,
        };
    }
    Ok(AudioEntry {
        format: entry.kind,
        channels: channels(u32::from(count), entry.offset, problems),
        bits: bit_depth(u32::from(bits), entry.offset, problems),
        sample_rate: sample_rate(u32::from(hz), entry.offset, problems),
        config,
    })
}

/// The tag of the elementary stream descriptor.
const ES_DESCRIPTOR: u8 = 0x03;
/// The tag of the decoder configuration descriptor.
const DECODER_CONFIG: u8 = 0x04;
/// The tag of the decoder specific information.
const DECODER_SPECIFIC: u8 = 0x05;

/// Reads the descriptor at the cursor, which must have tag `tag`, and
/// returns a cursor over its body. One step of the budget is charged for
/// it.
fn descriptor<'a>(
    cursor: &mut Cursor<'a>,
    tag: u8,
    budget: &mut Budget,
) -> Result<Cursor<'a>, Mp4Error> {
    let offset = cursor.offset();
    budget.charge(1, offset)?;
    let found = cursor.u8()?;
    if found != tag {
        return Err(Mp4Error::Unexpected {
            kind: ESDS,
            offset,
            found: u64::from(found),
        });
    }
    let len = descriptor_len(cursor)?;
    Ok(cursor.sub(len)?)
}

/// Reads a descriptor length: one to four octets of seven bits each, the
/// top bit set on every octet but the last (ISO/IEC 14496-1, section
/// 8.3.3).
fn descriptor_len(cursor: &mut Cursor<'_>) -> Result<u64, Mp4Error> {
    let mut len = 0_u64;
    let mut last = (cursor.offset(), 0_u8);
    for _ in 0..4 {
        last = (cursor.offset(), cursor.u8()?);
        // At most 28 bits, so nothing saturates.
        len = len
            .saturating_mul(128)
            .saturating_add(u64::from(last.1 & 0x7F));
        if last.1 & 0x80 == 0 {
            return Ok(len);
        }
    }
    // The fourth octet said another follows.
    Err(Mp4Error::Unexpected {
        kind: ESDS,
        offset: last.0,
        found: u64::from(last.1),
    })
}

/// Reads an elementary stream descriptor box.
fn esds(
    child: Mp4Box<'_>,
    budget: &mut Budget,
    problems: &mut Vec<Mp4Problem>,
) -> Result<Esds, Mp4Error> {
    let mut body = child.body;
    body.skip(4)?; // version and flags
    let mut es = descriptor(&mut body, ES_DESCRIPTOR, budget)?;
    es.skip(2)?; // ES_ID
    let flags = es.u8()?;
    if flags & 0x80 != 0 {
        es.skip(2)?; // dependsOn_ES_ID
    }
    if flags & 0x40 != 0 {
        // A URL to the stream, which is never read or followed (SEC-MED-016).
        let len = es.u8()?;
        es.skip(u64::from(len))?;
    }
    if flags & 0x20 != 0 {
        es.skip(2)?; // OCR_ES_Id
    }
    let mut config = descriptor(&mut es, DECODER_CONFIG, budget)?;
    let object_type = config.u8()?;
    config.skip(4)?; // stream type, upstream and reserved bits; buffer size
    let max_bitrate = config.u32_be()?;
    let avg_bitrate = config.u32_be()?;
    let specific = match config.rest().first() {
        Some(&DECODER_SPECIFIC) => {
            let info = descriptor(&mut config, DECODER_SPECIFIC, budget)?;
            let audio = match object_type {
                0x40 | 0x66..=0x68 => Some(audio_specific_config(info, problems)?),
                _ => None,
            };
            Some(Specific {
                range: span(&info),
                audio,
            })
        }
        _ => None,
    };
    Ok(Esds {
        object_type,
        max_bitrate,
        avg_bitrate,
        specific,
    })
}

/// The sampling frequencies of the frequency indexes 0 to 12 (ISO/IEC
/// 14496-3, table 1.18). Indexes 13 and 14 are reserved, and 15 means the
/// frequency follows in 24 bits.
const FREQUENCIES: [u32; 13] = [
    96_000, 88_200, 64_000, 48_000, 44_100, 32_000, 24_000, 22_050, 16_000, 12_000, 11_025, 8_000,
    7_350,
];

/// The first bits of a decoder specific info, read most significant first.
struct BitReader {
    /// Up to the first 16 octets, left-aligned.
    bits: u128,
    /// How many of those bits there are.
    available: u32,
    /// How many have been read.
    used: u32,
    /// Where the octets start in the file.
    offset: u64,
    /// How many octets there are in all.
    len: u64,
}

impl BitReader {
    /// The bits of the octets `info` has left.
    fn new(info: Cursor<'_>) -> Self {
        let mut first = [0_u8; 16];
        let mut count = 0_u32;
        for (slot, &octet) in first.iter_mut().zip(info.rest()) {
            *slot = octet;
            count = count.saturating_add(8);
        }
        Self {
            bits: u128::from_be_bytes(first),
            available: count,
            used: 0,
            offset: info.offset(),
            len: info.remaining(),
        }
    }

    /// The next `width` bits, at most 24, as a number.
    fn take(&mut self, width: u32) -> Result<u32, Mp4Error> {
        let end = self.used.saturating_add(width);
        if end > self.available {
            return Err(Mp4Error::Fault(ParseFault::Truncated {
                offset: self.offset,
                needed: u64::from(end.div_ceil(8)),
                available: self.len,
            }));
        }
        let value = self
            .bits
            .checked_shl(self.used)
            .unwrap_or(0)
            .checked_shr(128_u32.saturating_sub(width))
            .unwrap_or(0);
        self.used = end;
        Ok(u32::try_from(value).unwrap_or(u32::MAX))
    }

    /// An audio object type: five bits, or 32 plus six more after the
    /// escape value 31.
    fn object_type(&mut self) -> Result<u8, Mp4Error> {
        let mut object_type = self.take(5)?;
        if object_type == 31 {
            object_type = self.take(6)?.saturating_add(32);
        }
        Ok(u8::try_from(object_type).unwrap_or(u8::MAX))
    }

    /// A sampling frequency: an index into [`FREQUENCIES`], or 15 and the
    /// frequency in 24 bits.
    fn frequency(
        &mut self,
        problems: &mut Vec<Mp4Problem>,
    ) -> Result<Option<SampleRate>, Mp4Error> {
        let index = self.take(4)?;
        let hz = if index == 15 {
            self.take(24)?
        } else if let Some(&hz) = usize::try_from(index)
            .ok()
            .and_then(|index| FREQUENCIES.get(index))
        {
            hz
        } else {
            problems.push(Mp4Problem::Value {
                offset: self.offset,
                error: ValueError::Malformed {
                    field: Field::SampleRate,
                },
            });
            return Ok(None);
        };
        Ok(sample_rate(hz, self.offset, problems))
    }
}

/// Reads the start of an `AudioSpecificConfig` (ISO/IEC 14496-3, section
/// 1.6.2.1): the object type, the sampling frequency, the channel
/// configuration and, for SBR and PS, the explicit extension. What follows
/// depends on the object type and is not read.
///
/// # Errors
///
/// [`ParseFault::Truncated`], with the octets needed so far, when the bits
/// run out.
pub(crate) fn audio_specific_config(
    info: Cursor<'_>,
    problems: &mut Vec<Mp4Problem>,
) -> Result<AudioSpecificConfig, Mp4Error> {
    let mut bits = BitReader::new(info);
    let object_type = bits.object_type()?;
    let rate = bits.frequency(problems)?;
    let channel_config = u8::try_from(bits.take(4)?).unwrap_or(u8::MAX);
    let extension = match object_type {
        5 | 29 => {
            let rate = bits.frequency(problems)?;
            Some(Extension {
                object_type: bits.object_type()?,
                sample_rate: rate,
            })
        }
        _ => None,
    };
    Ok(AudioSpecificConfig {
        object_type,
        sample_rate: rate,
        channel_config,
        extension,
    })
}

/// Reads the ALAC magic cookie in an `alac` box, after its version and
/// flags.
fn alac(child: Mp4Box<'_>, problems: &mut Vec<Mp4Problem>) -> Result<Alac, Mp4Error> {
    let mut body = child.body;
    body.skip(4)?; // version and flags
    let start = body.offset();
    let frame_length = body.u32_be()?;
    body.skip(1)?; // compatible version
    let bits = body.u8()?;
    body.skip(3)?; // pb, mb and kb, the decoder's tuning
    let count = body.u8()?;
    body.skip(6)?; // max run and max frame bytes
    let avg_bitrate = body.u32_be()?;
    let hz = body.u32_be()?;
    Ok(Alac {
        frame_length,
        bits: bit_depth(u32::from(bits), child.offset, problems),
        channels: channels(u32::from(count), child.offset, problems),
        avg_bitrate,
        sample_rate: sample_rate(hz, child.offset, problems),
        cookie: start..body.offset(),
    })
}

/// Reads the FLAC specific box: its first metadata block must be a
/// STREAMINFO block of 34 octets.
fn dfla(child: Mp4Box<'_>, problems: &mut Vec<Mp4Problem>) -> Result<Flac, Mp4Error> {
    let mut body = child.body;
    body.skip(4)?; // version and flags
    let blocks = span(&body);
    let type_at = body.offset();
    let block_type = body.u8()? & 0x7F;
    if block_type != 0 {
        return Err(Mp4Error::Unexpected {
            kind: DFLA,
            offset: type_at,
            found: u64::from(block_type),
        });
    }
    let len_at = body.offset();
    let len = body.u24_be()?;
    if len != 34 {
        return Err(Mp4Error::Unexpected {
            kind: DFLA,
            offset: len_at,
            found: u64::from(len),
        });
    }
    let min_block = body.u16_be()?;
    let max_block = body.u16_be()?;
    body.skip(6)?; // the smallest and largest frame
    // Sample rate in 20 bits, channels less one in 3, bits less one in 5,
    // and the total in 36.
    let packed = body.u64_be()?;
    body.skip(16)?; // the MD5 of the samples
    let field = |shift: u32, mask: u64| {
        u32::try_from(packed.checked_shr(shift).unwrap_or(0) & mask).unwrap_or(u32::MAX)
    };
    let total_samples = packed & 0x0F_FFFF_FFFF;
    Ok(Flac {
        min_block,
        max_block,
        sample_rate: sample_rate(field(44, 0xF_FFFF), child.offset, problems),
        channels: channels(field(41, 0x7).saturating_add(1), child.offset, problems),
        bits: bit_depth(field(36, 0x1F).saturating_add(1), child.offset, problems),
        total_samples: (total_samples != 0).then_some(total_samples),
        blocks,
    })
}

/// Reads the Opus specific box, version 0, every field big-endian.
fn dops(child: Mp4Box<'_>, problems: &mut Vec<Mp4Problem>) -> Result<Opus, Mp4Error> {
    let mut body = child.body;
    let range = span(&body);
    let version_at = body.offset();
    let version = body.u8()?;
    if version != 0 {
        return Err(Mp4Error::Unexpected {
            kind: DOPS,
            offset: version_at,
            found: u64::from(version),
        });
    }
    let count = body.u8()?;
    let pre_skip = body.u16_be()?;
    let input_sample_rate = body.u32_be()?;
    let output_gain = GainDb::from_q7_8(i16::from_be_bytes(body.array()?));
    let mapping_family = body.u8()?;
    if mapping_family != 0 {
        // The stream count, the coupled count and one octet a channel.
        body.skip(u64::from(count).saturating_add(2))?;
    }
    Ok(Opus {
        channels: channels(u32::from(count), child.offset, problems),
        pre_skip,
        input_sample_rate,
        output_gain,
        mapping_family,
        body: range,
    })
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use super::*;
    use crate::parse::{Cursor, Depth, LimitKind, ParseFault};
    use crate::values::{Field, ValueError};
    use gunmetal_testkit::bytes::Bits;
    use gunmetal_testkit::mp4::{
        self as kit, SampleEntry, audio_specific_config as asc, descriptor, full_box, mp4_box,
        sample_entry as entry_box, stsd,
    };

    /// The depth `level` levels below the root of a container.
    fn depth(level: u64) -> Depth {
        (0..level).fold(Depth::CONTAINER_ROOT, |depth, _| {
            depth
                .descend(&Limits::DEFAULT, 0)
                .expect("the test stays within the depth limit")
        })
    }

    /// The box at the start of `file`, `level` levels deep.
    fn first_box(file: &[u8], level: u64) -> Mp4Box<'_> {
        crate::formats::mp4::boxes::Children::new(Cursor::at(file, 0), depth(level - 1))
            .required(&Limits::DEFAULT, &mut Budget::for_input(0, 0, 1))
            .expect("the test's box is well formed")
    }

    /// Reads the sample entry of the `stsd` box `file` holds, six levels
    /// deep as in a real file.
    fn read_entry(file: &[u8]) -> (Result<AudioEntry, Mp4Error>, Vec<Mp4Problem>) {
        let mut problems = Vec::new();
        let entry = sample_entry(
            first_box(file, 6),
            &Limits::DEFAULT,
            &mut Budget::for_input(0, 0, 99),
            &mut problems,
        );
        (entry, problems)
    }

    /// An `stsd` box with one entry of `format`, two channels of 16 bits at
    /// 44.1 kHz, holding `children`. The children start at offset 52.
    fn description(format: [u8; 4], children: &[u8]) -> Vec<u8> {
        stsd(&[&entry_box(&SampleEntry {
            format,
            version: 0,
            channels: 2,
            bits: 16,
            rate: 44_100,
            children,
        })])
    }

    fn rate(hz: u32) -> Option<SampleRate> {
        SampleRate::new(hz).ok()
    }

    fn channels(count: u32) -> Option<Channels> {
        Channels::new(count).ok()
    }

    fn bits(depth: u32) -> Option<BitDepth> {
        BitDepth::new(depth).ok()
    }

    /// The entry fields of [`description`] with `config`.
    fn stereo(format: [u8; 4], config: CodecConfig) -> AudioEntry {
        AudioEntry {
            format: FourCc(format),
            channels: channels(2),
            bits: bits(16),
            sample_rate: rate(44_100),
            config,
        }
    }

    fn truncated(offset: u64, needed: u64, available: u64) -> Mp4Error {
        Mp4Error::Fault(ParseFault::Truncated {
            offset,
            needed,
            available,
        })
    }

    fn aac_lc() -> AudioSpecificConfig {
        AudioSpecificConfig {
            object_type: 2,
            sample_rate: rate(44_100),
            channel_config: 2,
            extension: None,
        }
    }

    /// An esds box for AAC LC at 44.1 kHz in stereo, with every length in
    /// `width` octets.
    fn aac_esds(width: usize) -> Vec<u8> {
        kit::esds(&kit::Esds {
            object_type: 0x40,
            max_bitrate: 128_000,
            avg_bitrate: 96_000,
            specific: Some(&asc(2, 4, 2)),
            width,
        })
    }

    #[test]
    fn reads_an_aac_entry_and_its_audio_specific_config() {
        // A bit rate box before the esds, which is not read. The esds box
        // starts at 72; with four-octet lengths the specific info is at
        // 115 and 116.
        let children = [mp4_box(*b"btrt", &[0; 12]), aac_esds(4)].concat();
        assert_eq!(
            read_entry(&description(*b"mp4a", &children)),
            (
                Ok(stereo(
                    *b"mp4a",
                    CodecConfig::Esds(Esds {
                        object_type: 0x40,
                        max_bitrate: 128_000,
                        avg_bitrate: 96_000,
                        specific: Some(Specific {
                            range: 115..117,
                            audio: Some(aac_lc()),
                        }),
                    })
                )),
                vec![]
            )
        );
    }

    #[test]
    fn reads_descriptor_lengths_of_every_width() {
        for width in 1..=4 {
            // The esds starts at 52: its version and flags, then three
            // descriptor headers of 1 + width octets, ES_ID and flags, and
            // the 13 octets of the decoder config.
            let start = 52 + 8 + 4 + 3 * (1 + width as u64) + 3 + 13;
            assert_eq!(
                read_entry(&description(*b"mp4a", &aac_esds(width))).0,
                Ok(stereo(
                    *b"mp4a",
                    CodecConfig::Esds(Esds {
                        object_type: 0x40,
                        max_bitrate: 128_000,
                        avg_bitrate: 96_000,
                        specific: Some(Specific {
                            range: start..start + 2,
                            audio: Some(aac_lc()),
                        }),
                    })
                )),
                "width {width}"
            );
        }
    }

    /// An esds box whose ES descriptor body is `es`, with one-octet
    /// lengths.
    fn esds_with(es: &[u8]) -> Vec<u8> {
        full_box(*b"esds", 0, 0, &descriptor(0x03, 1, es))
    }

    /// The decoder config descriptor of MPEG-1 audio, with no specific
    /// info.
    fn mp3_config() -> Vec<u8> {
        descriptor(
            0x04,
            1,
            &[0x6B, 0x15, 0, 0, 0, 0, 0, 0x7D, 0, 0, 0, 0x7D, 0],
        )
    }

    fn mp3() -> CodecConfig {
        CodecConfig::Esds(Esds {
            object_type: 0x6B,
            max_bitrate: 32_000,
            avg_bitrate: 32_000,
            specific: None,
        })
    }

    #[test]
    fn reads_mp3_in_mp4_without_specific_info() {
        let es = [&[0x00, 0x01, 0x00][..], &mp3_config()].concat();
        assert_eq!(
            read_entry(&description(*b"mp4a", &esds_with(&es))),
            (Ok(stereo(*b"mp4a", mp3())), vec![])
        );
    }

    /// Verifies: SEC-MED-016
    #[test]
    fn skips_the_optional_es_fields_and_never_reads_the_url() {
        // dependsOn_ES_ID, a URL of six octets, and OCR_ES_Id, then the
        // decoder config as usual.
        let es = [
            &[0x00, 0x01, 0xE0, 0x00, 0x02][..],
            &[0x06],
            b"file:/",
            &[0x00, 0x03],
            &mp3_config(),
        ]
        .concat();
        assert_eq!(
            read_entry(&description(*b"mp4a", &esds_with(&es))),
            (Ok(stereo(*b"mp4a", mp3())), vec![])
        );
        // Each flag alone moves the decoder config by its own field.
        for (flag, field) in [(0x80, &[0, 2][..]), (0x40, &[1, b'x']), (0x20, &[0, 3])] {
            let es = [&[0x00, 0x01, flag][..], field, &mp3_config()].concat();
            assert_eq!(
                read_entry(&description(*b"mp4a", &esds_with(&es))).0,
                Ok(stereo(*b"mp4a", mp3())),
                "flag {flag:02X}"
            );
        }
    }

    #[test]
    fn reads_specific_info_only_when_the_next_descriptor_is_one() {
        // A profile level indication descriptor where the specific info
        // would be is not read.
        let config = descriptor(
            0x04,
            1,
            &[
                &[0x40, 0x15, 0, 0, 0, 0, 0, 0x7D, 0, 0, 0, 0x7D, 0][..],
                &descriptor(0x14, 1, &[1]),
            ]
            .concat(),
        );
        let es = [&[0x00, 0x01, 0x00][..], &config].concat();
        assert_eq!(
            read_entry(&description(*b"mp4a", &esds_with(&es))).0,
            Ok(stereo(
                *b"mp4a",
                CodecConfig::Esds(Esds {
                    object_type: 0x40,
                    max_bitrate: 32_000,
                    avg_bitrate: 32_000,
                    specific: None,
                })
            ))
        );
    }

    #[test]
    fn reads_the_specific_info_of_other_object_types_without_decoding_it() {
        for (object_type, decoded) in [
            (0x66, true),
            (0x67, true),
            (0x68, true),
            (0x69, false),
            (0x6B, false),
            (0x41, false),
        ] {
            let file = description(
                *b"mp4a",
                &kit::esds(&kit::Esds {
                    object_type,
                    max_bitrate: 1,
                    avg_bitrate: 2,
                    specific: Some(&asc(2, 4, 2)),
                    width: 1,
                }),
            );
            assert_eq!(
                read_entry(&file).0,
                Ok(stereo(
                    *b"mp4a",
                    CodecConfig::Esds(Esds {
                        object_type,
                        max_bitrate: 1,
                        avg_bitrate: 2,
                        specific: Some(Specific {
                            range: 86..88,
                            audio: decoded.then(aac_lc),
                        }),
                    })
                )),
                "object type {object_type:02X}"
            );
        }
    }

    #[test]
    fn refuses_a_descriptor_with_the_wrong_tag() {
        // The ES descriptor's tag at 64; then the decoder config's at 69.
        let file = description(
            *b"mp4a",
            &full_box(*b"esds", 0, 0, &descriptor(0x04, 1, &[0; 3])),
        );
        assert_eq!(
            read_entry(&file).0,
            Err(Mp4Error::Unexpected {
                kind: FourCc(*b"esds"),
                offset: 64,
                found: 4,
            })
        );
        let es = [&[0x00, 0x01, 0x00][..], &descriptor(0x05, 1, &[0; 13])].concat();
        assert_eq!(
            read_entry(&description(*b"mp4a", &esds_with(&es))).0,
            Err(Mp4Error::Unexpected {
                kind: FourCc(*b"esds"),
                offset: 69,
                found: 5,
            })
        );
    }

    /// Verifies: SEC-MED-008
    #[test]
    fn refuses_a_descriptor_length_longer_than_four_octets() {
        // Its fourth length octet, at 68, still says another follows.
        let file = description(
            *b"mp4a",
            &full_box(*b"esds", 0, 0, &[0x03, 0x80, 0x80, 0x80, 0x81, 0x01, 0x00]),
        );
        assert_eq!(
            read_entry(&file).0,
            Err(Mp4Error::Unexpected {
                kind: FourCc(*b"esds"),
                offset: 68,
                found: 0x81,
            })
        );
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn reports_a_descriptor_longer_than_its_box() {
        // The ES descriptor claims 9 octets from 66, and the box has 3.
        let file = description(*b"mp4a", &full_box(*b"esds", 0, 0, &[0x03, 0x09, 0, 1, 0]));
        assert_eq!(read_entry(&file).0, Err(truncated(66, 9, 3)));
        // A length cut off inside its octets.
        let file = description(*b"mp4a", &full_box(*b"esds", 0, 0, &[0x03, 0x80]));
        assert_eq!(read_entry(&file).0, Err(truncated(66, 1, 0)));
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn charges_a_step_for_every_descriptor() {
        // Two steps reach the entry and the esds box; the ES descriptor,
        // the decoder config and the specific info cost three more.
        let file = description(*b"mp4a", &aac_esds(1));
        for (steps, wanted) in [(3, Some(69)), (4, Some(84)), (5, None)] {
            let mut problems = Vec::new();
            let entry = sample_entry(
                first_box(&file, 6),
                &Limits::DEFAULT,
                &mut Budget::for_input(0, 0, steps),
                &mut problems,
            );
            match wanted {
                Some(offset) => assert_eq!(
                    entry,
                    Err(Mp4Error::Fault(ParseFault::BudgetExceeded { offset })),
                    "{steps} steps"
                ),
                None => assert!(entry.is_ok(), "{steps} steps: {entry:?}"),
            }
        }
    }

    /// Reads an `AudioSpecificConfig` from `bytes` at offset 500.
    fn read_asc(bytes: &[u8]) -> (Result<AudioSpecificConfig, Mp4Error>, Vec<Mp4Problem>) {
        let mut problems = Vec::new();
        let config = audio_specific_config(Cursor::at(bytes, 500), &mut problems);
        (config, problems)
    }

    /// `fields`, each a width and a value, packed and padded with zeros.
    fn packed(fields: &[(u32, u64)]) -> Vec<u8> {
        let mut bits = Bits::new();
        for &(width, value) in fields {
            bits.put(width, value);
        }
        bits.pad_to_byte();
        bits.into_vec()
    }

    #[test]
    fn reads_every_sampling_frequency_index() {
        // ISO/IEC 14496-3 table 1.18.
        let rates = [
            96_000, 88_200, 64_000, 48_000, 44_100, 32_000, 24_000, 22_050, 16_000, 12_000, 11_025,
            8_000, 7_350,
        ];
        for (index, hz) in rates.into_iter().enumerate() {
            assert_eq!(
                read_asc(&asc(2, u8::try_from(index).expect("an index fits"), 1)),
                (
                    Ok(AudioSpecificConfig {
                        object_type: 2,
                        sample_rate: rate(hz),
                        channel_config: 1,
                        extension: None,
                    }),
                    vec![]
                ),
                "index {index}"
            );
        }
        for index in [13, 14] {
            assert_eq!(
                read_asc(&asc(1, index, 7)),
                (
                    Ok(AudioSpecificConfig {
                        object_type: 1,
                        sample_rate: None,
                        channel_config: 7,
                        extension: None,
                    }),
                    vec![Mp4Problem::Value {
                        offset: 500,
                        error: ValueError::Malformed {
                            field: Field::SampleRate,
                        },
                    }]
                ),
                "index {index}"
            );
        }
    }

    #[test]
    fn reads_an_explicit_frequency_and_an_escaped_object_type() {
        // Object type 31 escapes to 32 plus six bits; index 15 is followed
        // by the frequency itself in 24 bits.
        let bytes = packed(&[(5, 31), (6, 10), (4, 15), (24, 44_100), (4, 1)]);
        assert_eq!(
            read_asc(&bytes),
            (
                Ok(AudioSpecificConfig {
                    object_type: 42,
                    sample_rate: rate(44_100),
                    channel_config: 1,
                    extension: None,
                }),
                vec![]
            )
        );
        for (hz, value) in [(0, 0), (0xFF_FFFF, 0xFF_FFFF)] {
            let bytes = packed(&[(5, 2), (4, 15), (24, hz), (4, 2)]);
            assert_eq!(
                read_asc(&bytes),
                (
                    Ok(AudioSpecificConfig {
                        object_type: 2,
                        sample_rate: None,
                        channel_config: 2,
                        extension: None,
                    }),
                    vec![Mp4Problem::Value {
                        offset: 500,
                        error: ValueError::OutOfRange {
                            field: Field::SampleRate,
                            value,
                        },
                    }]
                )
            );
        }
    }

    #[test]
    fn reads_the_explicit_extension_of_sbr_and_ps() {
        // HE-AAC: SBR (5) at 24 kHz in stereo, extended to 48 kHz over
        // AAC LC.
        let sbr = packed(&[(5, 5), (4, 6), (4, 2), (4, 3), (5, 2)]);
        assert_eq!(
            read_asc(&sbr).0,
            Ok(AudioSpecificConfig {
                object_type: 5,
                sample_rate: rate(24_000),
                channel_config: 2,
                extension: Some(Extension {
                    object_type: 2,
                    sample_rate: rate(48_000),
                }),
            })
        );
        // HE-AAC v2: PS (29) in mono, with an explicit extension frequency
        // and an escaped underlying type.
        let ps = packed(&[
            (5, 29),
            (4, 8),
            (4, 1),
            (4, 15),
            (24, 32_000),
            (5, 31),
            (6, 0),
        ]);
        assert_eq!(
            read_asc(&ps).0,
            Ok(AudioSpecificConfig {
                object_type: 29,
                sample_rate: rate(16_000),
                channel_config: 1,
                extension: Some(Extension {
                    object_type: 32,
                    sample_rate: rate(32_000),
                }),
            })
        );
        // Any other object type has no extension, whatever follows.
        let lc = packed(&[(5, 2), (4, 6), (4, 2), (4, 3), (5, 2)]);
        assert_eq!(read_asc(&lc).0.map(|config| config.extension), Ok(None));
    }

    #[test]
    fn reads_only_the_start_of_a_long_specific_info() {
        let mut bytes = asc(2, 4, 2);
        bytes.extend([0xFF; 30]);
        assert_eq!(read_asc(&bytes), (Ok(aac_lc()), vec![]));
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn reports_a_truncated_audio_specific_config_with_the_octets_it_needs() {
        assert_eq!(read_asc(&[]).0, Err(truncated(500, 1, 0)));
        // Five bits of object type fit; the frequency index needs a second
        // octet.
        assert_eq!(read_asc(&[0x12]).0, Err(truncated(500, 2, 1)));
        // An explicit frequency needs 5 + 4 + 24 bits.
        assert_eq!(
            read_asc(&packed(&[(5, 2), (4, 15), (7, 0)])).0,
            Err(truncated(500, 5, 2))
        );
        // An escaped object type needs 11 bits.
        assert_eq!(read_asc(&[0xF8]).0, Err(truncated(500, 2, 1)));
        // SBR needs its extension index and underlying type.
        assert_eq!(
            read_asc(&packed(&[(5, 5), (4, 6), (4, 2)])).0,
            Err(truncated(500, 3, 2))
        );
        assert_eq!(
            read_asc(&packed(&[(5, 5), (4, 6), (4, 2), (4, 15), (3, 0)])).0,
            Err(truncated(500, 6, 3))
        );
        // An escaped object type leaves too few bits for the channels.
        assert_eq!(
            read_asc(&packed(&[(5, 31), (6, 0), (4, 4)])).0,
            Err(truncated(500, 3, 2))
        );
        // An escaped extension object type runs out of bits.
        let short = packed(&[
            (5, 5),
            (4, 15),
            (24, 24_000),
            (4, 2),
            (4, 3),
            (5, 31),
            (2, 0),
        ]);
        assert_eq!(read_asc(&short).0, Err(truncated(500, 7, 6)));
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn takes_every_bit_the_decoder_specific_info_holds() {
        // Two octets hold 16 bits. Taking them all must succeed: a bound
        // of `end >= available` would refuse the last field even though
        // the bits are there.
        let mut bits = BitReader::new(Cursor::at(&[0xA5, 0x5A], 40));
        assert_eq!(bits.take(16).expect("both octets"), 0xA55A);
        assert_eq!(bits.take(1), Err(truncated(40, 3, 2)));
    }

    /// Checks every cut of `body`, a structure whose fields have `widths`
    /// and whose first octet is at `base` in the file: reading the input
    /// built from the first `cut` octets must fail with `Truncated` at the
    /// first field the cut leaves short.
    fn every_cut<T>(
        body: &[u8],
        widths: &[u64],
        base: u64,
        read: impl Fn(&[u8]) -> Result<T, Mp4Error>,
    ) {
        assert_eq!(
            widths.iter().sum::<u64>(),
            body.len() as u64,
            "the fields cover the structure"
        );
        for cut in 0..body.len() {
            let octets = cut as u64;
            let mut start = 0;
            let mut short = None;
            for &width in widths {
                if short.is_none() && start + width > octets {
                    short = Some(truncated(base + start, width, octets - start));
                }
                start += width;
            }
            assert_eq!(read(&body[..cut]).err(), short, "cut at {cut}");
        }
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn reports_every_cut_of_a_media_header_and_a_handler() {
        let read = |header: &[u8]| read_track(&[header, &description(*b"Opus", &[])].concat()).0;
        // The version, the flags, the two times, the timescale and the
        // duration, from 8.
        every_cut(&kit::mdhd(1, 2)[8..28], &[1, 3, 8, 4, 4], 8, |cut| {
            read(&mp4_box(*b"mdhd", cut))
        });
        every_cut(&kit::mdhd_v1(1, 2)[8..40], &[1, 3, 16, 4, 8], 8, |cut| {
            read(&mp4_box(*b"mdhd", cut))
        });
        // The version, flags and pre_defined, then the handler type.
        every_cut(&kit::hdlr(*b"soun")[8..20], &[8, 4], 8, |cut| {
            let file = mp4_box(*b"hdlr", cut);
            parts(&file).err().map_or(Ok(()), Err)
        });
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn reports_every_cut_of_a_sample_description_and_its_entry() {
        // The version, flags and entry count of stsd, from 8.
        every_cut(&[0; 8], &[8], 8, |cut| {
            read_entry(&mp4_box(*b"stsd", cut)).0
        });
        // The fields of a version 1 entry, from 24.
        let entry = entry_box(&SampleEntry {
            format: *b"mp4a",
            version: 1,
            channels: 2,
            bits: 16,
            rate: 8_000,
            children: &[],
        });
        every_cut(&entry[8..], &[8, 2, 6, 2, 2, 4, 2, 2, 16], 24, |cut| {
            read_entry(&stsd(&[&mp4_box(*b"mp4a", cut)])).0
        });
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn reports_every_cut_of_the_configuration_boxes() {
        // Each configuration box starts at 52 and its body at 60.
        let cookie = kit::alac(&kit::Alac {
            frame_length: 1,
            bits: 16,
            channels: 2,
            max_frame_bytes: 0,
            avg_bitrate: 0,
            sample_rate: 8_000,
        });
        every_cut(&cookie[8..], &[4, 4, 1, 1, 3, 1, 6, 4, 4], 60, |cut| {
            read_entry(&description(*b"alac", &mp4_box(*b"alac", cut))).0
        });
        let streaminfo = kit::dfla(&kit::StreamInfo {
            min_block: 16,
            max_block: 16,
            sample_rate: 8_000,
            channels: 1,
            bits: 8,
            total_samples: 0,
        });
        every_cut(&streaminfo[8..], &[4, 1, 3, 2, 2, 6, 8, 16], 60, |cut| {
            read_entry(&description(*b"fLaC", &mp4_box(*b"dfLa", cut))).0
        });
        let opus = kit::dops(&kit::Opus {
            channels: 2,
            pre_skip: 0,
            input_rate: 0,
            gain: 0,
            family: 1,
            mapping: &[1, 1, 0, 1],
        });
        every_cut(&opus[8..], &[1, 1, 2, 4, 2, 1, 4], 60, |cut| {
            read_entry(&description(*b"Opus", &mp4_box(*b"dOps", cut))).0
        });
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn reports_every_cut_of_an_esds_box() {
        let read = |body: &[u8]| read_entry(&description(*b"mp4a", &mp4_box(*b"esds", body))).0;
        // The version and flags, then the ES descriptor's tag and length,
        // from 60.
        every_cut(&[0, 0, 0, 0, 0x03, 0x00], &[4, 1, 1], 60, read);
        // The ES descriptor's body from 66, with every optional field:
        // ES_ID, flags, dependsOn_ES_ID, a URL of two octets after its
        // length, OCR_ES_Id, then the decoder config's tag and length.
        let es = [0, 1, 0xE0, 0, 2, 2, b'a', b'b', 0, 3, 0x04, 0x00];
        every_cut(&es, &[2, 1, 2, 1, 2, 2, 1, 1], 66, |cut| {
            read(&[&[0, 0, 0, 0][..], &descriptor(0x03, 1, cut)].concat())
        });
        // The decoder config's body from 71.
        let config = [0x40, 0x15, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 2];
        every_cut(&config, &[1, 4, 4, 4], 71, |cut| {
            let es = [&[0, 1, 0][..], &descriptor(0x04, 1, cut)].concat();
            read(&[&[0, 0, 0, 0][..], &descriptor(0x03, 1, &es)].concat())
        });
        // MPEG-4 audio whose specific info, at 86, is empty.
        let config = [&config[..], &descriptor(0x05, 1, &[])].concat();
        let es = [&[0, 1, 0][..], &descriptor(0x04, 1, &config)].concat();
        assert_eq!(
            read(&[&[0, 0, 0, 0][..], &descriptor(0x03, 1, &es)].concat()),
            Err(truncated(86, 1, 0))
        );
    }

    fn alac_entry(cookie: &kit::Alac) -> Vec<u8> {
        stsd(&[&entry_box(&SampleEntry {
            format: *b"alac",
            version: 0,
            channels: 2,
            bits: 16,
            rate: 44_100,
            children: &kit::alac(cookie),
        })])
    }

    #[test]
    fn reads_an_alac_magic_cookie() {
        let file = alac_entry(&kit::Alac {
            frame_length: 4_096,
            bits: 24,
            channels: 6,
            max_frame_bytes: 9,
            avg_bitrate: 2_000_000,
            sample_rate: 96_000,
        });
        // The alac box starts at 52; its cookie is octets 64 to 88.
        assert_eq!(
            read_entry(&file),
            (
                Ok(stereo(
                    *b"alac",
                    CodecConfig::Alac(Alac {
                        frame_length: 4_096,
                        bits: bits(24),
                        channels: channels(6),
                        avg_bitrate: 2_000_000,
                        sample_rate: rate(96_000),
                        cookie: 64..88,
                    })
                )),
                vec![]
            )
        );
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn drops_zero_values_from_a_cookie_with_their_reasons() {
        let file = alac_entry(&kit::Alac {
            frame_length: 1,
            bits: 0,
            channels: 0,
            max_frame_bytes: 0,
            avg_bitrate: 0,
            sample_rate: 0,
        });
        let dropped = |field, value| Mp4Problem::Value {
            offset: 52,
            error: ValueError::OutOfRange { field, value },
        };
        assert_eq!(
            read_entry(&file),
            (
                Ok(stereo(
                    *b"alac",
                    CodecConfig::Alac(Alac {
                        frame_length: 1,
                        bits: None,
                        channels: None,
                        avg_bitrate: 0,
                        sample_rate: None,
                        cookie: 64..88,
                    })
                )),
                vec![
                    dropped(Field::BitDepth, 0),
                    dropped(Field::Channels, 0),
                    dropped(Field::SampleRate, 0),
                ]
            )
        );
    }

    fn flac_entry(dfla: &[u8]) -> Vec<u8> {
        stsd(&[&entry_box(&SampleEntry {
            format: *b"fLaC",
            version: 0,
            channels: 2,
            bits: 16,
            rate: 44_100,
            children: dfla,
        })])
    }

    #[test]
    fn reads_the_streaminfo_of_a_flac_entry() {
        let file = flac_entry(&kit::dfla(&kit::StreamInfo {
            min_block: 1_152,
            max_block: 4_608,
            sample_rate: 192_000,
            channels: 8,
            bits: 24,
            total_samples: 0x0F_FFFF_FFFF,
        }));
        // The dfLa box starts at 52; its blocks at 64, 38 octets.
        assert_eq!(
            read_entry(&file),
            (
                Ok(stereo(
                    *b"fLaC",
                    CodecConfig::Flac(Flac {
                        min_block: 1_152,
                        max_block: 4_608,
                        sample_rate: rate(192_000),
                        channels: channels(8),
                        bits: bits(24),
                        total_samples: Some(0x0F_FFFF_FFFF),
                        blocks: 64..102,
                    })
                )),
                vec![]
            )
        );
        let file = flac_entry(&kit::dfla(&kit::StreamInfo {
            min_block: 16,
            max_block: 16,
            sample_rate: 0,
            channels: 1,
            bits: 1,
            total_samples: 0,
        }));
        assert_eq!(
            read_entry(&file),
            (
                Ok(stereo(
                    *b"fLaC",
                    CodecConfig::Flac(Flac {
                        min_block: 16,
                        max_block: 16,
                        sample_rate: None,
                        channels: channels(1),
                        bits: bits(1),
                        total_samples: None,
                        blocks: 64..102,
                    })
                )),
                vec![Mp4Problem::Value {
                    offset: 52,
                    error: ValueError::OutOfRange {
                        field: Field::SampleRate,
                        value: 0,
                    },
                }]
            )
        );
    }

    #[test]
    fn reads_a_flac_sample_rate_above_what_typed_values_allow_as_dropped() {
        let file = flac_entry(&kit::dfla(&kit::StreamInfo {
            min_block: 16,
            max_block: 16,
            sample_rate: 0xF_FFFF,
            channels: 2,
            bits: 16,
            total_samples: 1,
        }));
        assert_eq!(
            read_entry(&file).1,
            vec![Mp4Problem::Value {
                offset: 52,
                error: ValueError::OutOfRange {
                    field: Field::SampleRate,
                    value: 0xF_FFFF,
                },
            }]
        );
    }

    #[test]
    fn refuses_a_flac_box_that_does_not_start_with_streaminfo() {
        // A PADDING block (type 1), marked last, at 64.
        let file = flac_entry(&full_box(*b"dfLa", 0, 0, &[0x81, 0, 0, 34]));
        assert_eq!(
            read_entry(&file).0,
            Err(Mp4Error::Unexpected {
                kind: FourCc(*b"dfLa"),
                offset: 64,
                found: 1,
            })
        );
        // A STREAMINFO not marked last is fine; one of 33 octets is not.
        let file = flac_entry(&full_box(*b"dfLa", 0, 0, &[0x00, 0, 0, 33]));
        assert_eq!(
            read_entry(&file).0,
            Err(Mp4Error::Unexpected {
                kind: FourCc(*b"dfLa"),
                offset: 65,
                found: 33,
            })
        );
        // One of 34 octets whose octets are missing.
        let file = flac_entry(&full_box(*b"dfLa", 0, 0, &[0x00, 0, 0, 34, 0, 16, 0, 16]));
        assert_eq!(read_entry(&file).0, Err(truncated(72, 6, 0)));
    }

    fn opus_entry(dops: &[u8]) -> Vec<u8> {
        stsd(&[&entry_box(&SampleEntry {
            format: *b"Opus",
            version: 0,
            channels: 2,
            bits: 16,
            rate: 48_000,
            children: dops,
        })])
    }

    #[test]
    fn reads_an_opus_specific_box_with_and_without_a_mapping() {
        let file = opus_entry(&kit::dops(&kit::Opus {
            channels: 2,
            pre_skip: 3_840,
            input_rate: 44_100,
            gain: -1_280,
            family: 0,
            mapping: &[],
        }));
        let entry = |config| AudioEntry {
            sample_rate: rate(48_000),
            ..stereo(*b"Opus", config)
        };
        // dOps starts at 52; its body at 60.
        assert_eq!(
            read_entry(&file),
            (
                Ok(entry(CodecConfig::Opus(Opus {
                    channels: channels(2),
                    pre_skip: 3_840,
                    input_sample_rate: 44_100,
                    output_gain: GainDb::from_q7_8(-1_280),
                    mapping_family: 0,
                    body: 60..71,
                }))),
                vec![]
            )
        );
        let file = opus_entry(&kit::dops(&kit::Opus {
            channels: 3,
            pre_skip: 0,
            input_rate: 0,
            gain: 256,
            family: 1,
            mapping: &[2, 1, 0, 2, 1],
        }));
        assert_eq!(
            read_entry(&file).0,
            Ok(entry(CodecConfig::Opus(Opus {
                channels: channels(3),
                pre_skip: 0,
                input_sample_rate: 0,
                output_gain: GainDb::from_q7_8(256),
                mapping_family: 1,
                body: 60..76,
            })))
        );
    }

    #[test]
    fn refuses_an_opus_box_of_another_version_or_a_short_mapping() {
        let file = opus_entry(&mp4_box(*b"dOps", &[1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0]));
        assert_eq!(
            read_entry(&file).0,
            Err(Mp4Error::Unexpected {
                kind: FourCc(*b"dOps"),
                offset: 60,
                found: 1,
            })
        );
        // Family 1 with three channels needs five octets of mapping.
        let file = opus_entry(&kit::dops(&kit::Opus {
            channels: 3,
            pre_skip: 0,
            input_rate: 0,
            gain: 0,
            family: 1,
            mapping: &[2, 1, 0, 2],
        }));
        assert_eq!(read_entry(&file).0, Err(truncated(71, 5, 4)));
        // Channels of zero is dropped with its reason.
        let file = opus_entry(&kit::dops(&kit::Opus {
            channels: 0,
            pre_skip: 0,
            input_rate: 0,
            gain: 0,
            family: 0,
            mapping: &[],
        }));
        assert_eq!(
            read_entry(&file).1,
            vec![Mp4Problem::Value {
                offset: 52,
                error: ValueError::OutOfRange {
                    field: Field::Channels,
                    value: 0,
                },
            }]
        );
    }

    #[test]
    fn reads_a_version_1_entry_past_its_extra_fields() {
        let file = stsd(&[&entry_box(&SampleEntry {
            format: *b"mp4a",
            version: 1,
            channels: 1,
            bits: 16,
            rate: 22_050,
            children: &aac_esds(1),
        })]);
        // The esds starts 16 octets later, at 68.
        assert_eq!(
            read_entry(&file).0,
            Ok(AudioEntry {
                format: FourCc(*b"mp4a"),
                channels: channels(1),
                bits: bits(16),
                sample_rate: rate(22_050),
                config: CodecConfig::Esds(Esds {
                    object_type: 0x40,
                    max_bitrate: 128_000,
                    avg_bitrate: 96_000,
                    specific: Some(Specific {
                        range: 102..104,
                        audio: Some(aac_lc()),
                    }),
                }),
            })
        );
    }

    #[test]
    fn refuses_a_version_2_entry() {
        let file = stsd(&[&entry_box(&SampleEntry {
            format: *b"lpcm",
            version: 2,
            channels: 3,
            bits: 16,
            rate: 1,
            children: &[],
        })]);
        assert_eq!(
            read_entry(&file).0,
            Err(Mp4Error::Unexpected {
                kind: FourCc(*b"lpcm"),
                offset: 32,
                found: 2,
            })
        );
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn drops_zero_entry_fields_with_their_reasons() {
        let file = stsd(&[&entry_box(&SampleEntry {
            format: *b"ac-3",
            version: 0,
            channels: 0,
            bits: 0,
            rate: 0,
            children: &mp4_box(*b"dac3", &[0; 3]),
        })]);
        let dropped = |field| Mp4Problem::Value {
            offset: 16,
            error: ValueError::OutOfRange { field, value: 0 },
        };
        assert_eq!(
            read_entry(&file),
            (
                Ok(AudioEntry {
                    format: FourCc(*b"ac-3"),
                    channels: None,
                    bits: None,
                    sample_rate: None,
                    config: CodecConfig::Missing,
                }),
                vec![
                    dropped(Field::Channels),
                    dropped(Field::BitDepth),
                    dropped(Field::SampleRate),
                ]
            )
        );
    }

    #[test]
    fn reads_only_the_configuration_box_of_the_entry_format() {
        // An mp4a entry with no esds, and with a dfLa it does not read.
        let dfla = kit::dfla(&kit::StreamInfo {
            min_block: 16,
            max_block: 16,
            sample_rate: 8_000,
            channels: 1,
            bits: 8,
            total_samples: 0,
        });
        assert_eq!(
            read_entry(&description(*b"mp4a", &dfla)).0,
            Ok(stereo(*b"mp4a", CodecConfig::Missing))
        );
        assert_eq!(
            read_entry(&description(*b"fLaC", &aac_esds(1))).0,
            Ok(stereo(*b"fLaC", CodecConfig::Missing))
        );
        let opus = kit::dops(&kit::Opus {
            channels: 1,
            pre_skip: 0,
            input_rate: 0,
            gain: 0,
            family: 0,
            mapping: &[],
        });
        assert_eq!(
            read_entry(&description(*b"alac", &opus)).0,
            Ok(stereo(*b"alac", CodecConfig::Missing))
        );
        assert_eq!(
            read_entry(&description(*b"Opus", &mp4_box(*b"alac", &[0; 28]))).0,
            Ok(stereo(*b"Opus", CodecConfig::Missing))
        );
    }

    #[test]
    fn keeps_the_last_configuration_box() {
        let children = [aac_esds(1), {
            let es = [&[0x00, 0x01, 0x00][..], &mp3_config()].concat();
            esds_with(&es)
        }]
        .concat();
        assert_eq!(
            read_entry(&description(*b"mp4a", &children)).0,
            Ok(stereo(*b"mp4a", mp3()))
        );
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn reports_a_description_without_an_entry_or_with_a_short_one() {
        assert_eq!(read_entry(&stsd(&[])).0, Err(truncated(16, 4, 0)));
        // An entry whose fields stop after the channel count.
        let short = mp4_box(*b"mp4a", &[0; 18]);
        assert_eq!(read_entry(&stsd(&[&short])).0, Err(truncated(42, 2, 0)));
    }

    /// Verifies: SEC-MED-005
    #[test]
    fn counts_the_depth_of_the_entry_and_its_configuration() {
        let file = description(*b"mp4a", &aac_esds(1));
        let read = |level| {
            sample_entry(
                first_box(&file, level),
                &Limits::DEFAULT,
                &mut Budget::for_input(0, 0, 99),
                &mut Vec::new(),
            )
        };
        // The configuration box is two levels below stsd.
        assert!(read(30).is_ok());
        assert_eq!(
            read(31),
            Err(Mp4Error::Fault(ParseFault::TooDeep {
                limit: LimitKind::ContainerDepth,
                depth: 33,
                max: 32,
                offset: 52,
            }))
        );
        assert_eq!(
            read(32),
            Err(Mp4Error::Fault(ParseFault::TooDeep {
                limit: LimitKind::ContainerDepth,
                depth: 33,
                max: 32,
                offset: 16,
            }))
        );
    }

    /// The parts of a track gathered from `boxes`, read as the `mdia` and
    /// `stbl` children they are named after.
    fn parts(file: &[u8]) -> Result<TrackParts<'_>, Mp4Error> {
        let mut parts = TrackParts::new(1_000);
        let mut children = crate::formats::mp4::boxes::Children::new(Cursor::at(file, 0), depth(4));
        while let Some(child) = children
            .next(&Limits::DEFAULT, &mut Budget::for_input(0, 0, 9))
            .expect("the test's boxes are well formed")
        {
            match child.kind.0 {
                [b's', b't', ..] | [b'c', b'o', b'6', b'4'] => parts.table(child),
                _ => parts.media(child)?,
            }
        }
        Ok(parts)
    }

    fn read_track(
        file: &[u8],
    ) -> (
        Result<(AudioTrack, SampleTableRanges), Mp4Error>,
        Vec<Mp4Problem>,
    ) {
        let mut problems = Vec::new();
        let track = parts(file).and_then(|parts| {
            parts.audio(
                &Limits::DEFAULT,
                &mut Budget::for_input(0, 0, 99),
                &mut problems,
            )
        });
        (track, problems)
    }

    #[test]
    fn gathers_the_parts_of_a_track_and_reads_it() {
        let header = kit::mdhd(44_100, 441_000);
        let description = description(*b"mp4a", &aac_esds(1));
        let file = [
            header.clone(),
            kit::hdlr(*b"soun"),
            mp4_box(*b"minf", &[]),
            mp4_box(*b"stts", &[1]),
            mp4_box(*b"stsc", &[2, 2]),
            mp4_box(*b"stz2", &[3]),
            mp4_box(*b"co64", &[4]),
            description.clone(),
        ]
        .concat();
        let gathered = parts(&file).expect("the parts are read");
        assert_eq!(gathered.handler, Some(FourCc(*b"soun")));
        assert!(gathered.is_audio());
        // mdhd 0, hdlr 32, minf 65, stts 73, stsc 82, stz2 92, co64 101,
        // stsd 110.
        let tables = SampleTableRanges {
            stts: Some(81..82),
            stsc: Some(90..92),
            sizes: Some(SampleSizes::Stz2(100..101)),
            offsets: Some(ChunkOffsets::Co64(109..110)),
        };
        assert_eq!(gathered.tables, tables);
        // The description is the one of the first test, 110 octets on.
        let entry = stereo(
            *b"mp4a",
            CodecConfig::Esds(Esds {
                object_type: 0x40,
                max_bitrate: 128_000,
                avg_bitrate: 96_000,
                specific: Some(Specific {
                    range: 110 + 86..110 + 88,
                    audio: Some(aac_lc()),
                }),
            }),
        );
        assert_eq!(
            read_track(&file),
            (
                Ok((
                    AudioTrack {
                        offset: 1_000,
                        timescale: NonZeroU32::new(44_100).expect("non-zero"),
                        duration: Some(441_000),
                        entry,
                    },
                    tables
                )),
                vec![]
            )
        );
    }

    #[test]
    fn keeps_the_other_sample_table_boxes_and_the_last_of_each() {
        let file = [
            mp4_box(*b"stsz", &[1]),
            mp4_box(*b"stco", &[2]),
            mp4_box(*b"stts", &[3]),
            mp4_box(*b"stts", &[4, 4]),
            mp4_box(*b"stss", &[5]),
        ]
        .concat();
        assert_eq!(
            parts(&file).map(|parts| parts.tables),
            Ok(SampleTableRanges {
                stts: Some(35..37),
                stsc: None,
                sizes: Some(SampleSizes::Stsz(8..9)),
                offsets: Some(ChunkOffsets::Stco(17..18)),
            })
        );
    }

    #[test]
    fn says_whether_a_handler_is_audio() {
        let handler = kit::hdlr(*b"vide");
        let video = parts(&handler).expect("the handler is read");
        assert_eq!(video.handler, Some(FourCc(*b"vide")));
        assert!(!video.is_audio());
        let none = parts(&[]).expect("nothing to read");
        assert_eq!(none.handler, None);
        assert!(!none.is_audio());
        assert_eq!(
            parts(&full_box(*b"hdlr", 0, 0, &[0, 0, 0, 0, b's', b'o'])),
            Err(truncated(16, 4, 2))
        );
    }

    #[test]
    fn reads_media_headers_of_both_versions() {
        let description = description(*b"Opus", &[]);
        let with = |header: Vec<u8>| [header, description.clone()].concat();
        let duration = |file: &[u8]| {
            read_track(file)
                .0
                .map(|(track, _)| (track.timescale.get(), track.duration))
        };
        assert_eq!(duration(&with(kit::mdhd(1_000, 7))), Ok((1_000, Some(7))));
        assert_eq!(
            duration(&with(kit::mdhd(1_000, u32::MAX))),
            Ok((1_000, None))
        );
        assert_eq!(
            duration(&with(kit::mdhd_v1(48_000, 1 << 40))),
            Ok((48_000, Some(1 << 40)))
        );
        assert_eq!(
            duration(&with(kit::mdhd_v1(48_000, u64::MAX - 1))),
            Ok((48_000, Some(u64::MAX - 1)))
        );
        assert_eq!(
            duration(&with(kit::mdhd_v1(48_000, u64::MAX))),
            Ok((48_000, None))
        );
        assert_eq!(
            duration(&with(full_box(*b"mdhd", 2, 0, &[0; 20]))),
            Err(Mp4Error::Unexpected {
                kind: FourCc(*b"mdhd"),
                offset: 8,
                found: 2,
            })
        );
        assert_eq!(
            duration(&with(full_box(*b"mdhd", 1, 0, &[0; 20]))),
            Err(truncated(32, 8, 0))
        );
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn refuses_a_timescale_of_zero() {
        let file = [kit::mdhd(0, 100), description(*b"Opus", &[])].concat();
        assert_eq!(
            read_track(&file).0,
            Err(Mp4Error::ZeroTimescale { offset: 20 })
        );
        let file = [kit::mdhd_v1(0, 100), description(*b"Opus", &[])].concat();
        assert_eq!(
            read_track(&file).0,
            Err(Mp4Error::ZeroTimescale { offset: 28 })
        );
    }

    #[test]
    fn refuses_a_track_without_its_media_header_or_sample_description() {
        assert_eq!(
            read_track(&description(*b"Opus", &[])).0,
            Err(Mp4Error::Missing {
                kind: FourCc(*b"mdhd"),
                offset: 1_000,
            })
        );
        assert_eq!(
            read_track(&kit::mdhd(1, 1)).0,
            Err(Mp4Error::Missing {
                kind: FourCc(*b"stsd"),
                offset: 1_000,
            })
        );
        // A sample description at 32 with no entry, which would be at 48.
        assert_eq!(
            read_track(&[kit::mdhd(1, 1), stsd(&[])].concat()).0,
            Err(truncated(48, 4, 0))
        );
    }
}
