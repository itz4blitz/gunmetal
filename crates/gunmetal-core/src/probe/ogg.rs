//! An Ogg file's facts, from the header packets of its first stream.

use crate::catalog::{AudioFormat, Codec, Container, Trim};
use crate::formats::detect::Format;
use crate::formats::ogg::{self, Packet};
use crate::formats::opus::{self, OpusError};
use crate::formats::vorbis;
use crate::formats::vorbis_comment;
use crate::parse::{Budget, Cursor, Depth, Limits};
use crate::values::SampleRate;

use super::draft::{Draft, Gather, Job};
use super::facts::{PartProblem, ProbeError, SeekIndex, TagBlock};

/// The rate Opus is always decoded at, which its granule positions count
/// in.
const OPUS_RATE: u32 = 48_000;

/// How many octets from the end of an Ogg file are read to find the last
/// page of its stream. The longest page is 65,307 octets.
const TAIL: u64 = 65_536;

/// The draft of the Ogg file of `file_len` octets that starts with
/// `head`, or `None` when `head` holds fewer than the stream's two header
/// packets and `more` of the file is left to read.
///
/// The stream is the one the first sound page belongs to. Its first packet
/// is an Opus or a Vorbis identification header, which playback needs. Its
/// second packet is the comment header, which is optional here: when it is
/// missing or cannot be read, that is recorded and the file kept. The end
/// of the file is read next, for the granule position of the last page.
///
/// # Errors
///
/// [`ProbeError::Ogg`] when the file holds no packet of a stream,
/// [`ProbeError::Opus`] for a damaged Opus header, and
/// [`ProbeError::Vorbis`] for a damaged Vorbis header or a first packet
/// that is neither.
pub(super) fn head(
    head: &[u8],
    more: bool,
    file_len: u64,
    limits: &Limits,
    budget: &mut Budget,
) -> Option<Result<Draft, ProbeError>> {
    let serial = ogg::pages(Cursor::new(head), budget)
        .find_map(Result::ok)
        .map(|page| page.serial);
    let mut error = None;
    let mut packets: Vec<Packet<'_>> = Vec::new();
    if let Some(serial) = serial {
        for item in ogg::packets(Cursor::new(head), serial, limits, budget) {
            match item {
                Ok(packet) => {
                    packets.push(packet);
                    if packets.len() == 2 {
                        break;
                    }
                }
                Err(found) => {
                    error.get_or_insert(found);
                }
            }
        }
    }
    if packets.len() < 2 && more {
        return None;
    }
    let mut packets = packets.into_iter();
    let (Some(serial), Some(ident)) = (serial, packets.next()) else {
        return Some(Err(ProbeError::Ogg(error)));
    };
    let depth = Depth::CONTAINER_ROOT;
    let (codec, rate, channels, skip) = match opus::opus_head(&ident.data, limits, budget, depth) {
        Ok(head) => (
            Codec::Opus,
            SampleRate::new(OPUS_RATE).ok(),
            head.channels,
            Some(head.pre_skip),
        ),
        Err(OpusError::Magic { .. }) => {
            match vorbis::vorbis_ident(&ident.data, limits, budget, depth) {
                Ok(ident) => (Codec::Vorbis, Some(ident.sample_rate), ident.channels, None),
                Err(error) => return Some(Err(ProbeError::Vorbis(error))),
            }
        }
        Err(error) => return Some(Err(ProbeError::Opus(error))),
    };
    let mut tags = Vec::new();
    let mut problems = Vec::new();
    match packets.next() {
        None => problems.extend(error.map(PartProblem::Ogg)),
        Some(packet) => {
            let block = if codec == Codec::Opus {
                opus::opus_tags(&packet.data, limits, budget, depth).map_err(PartProblem::Opus)
            } else {
                vorbis::vorbis_comments(&packet.data, limits, budget, depth)
                    .map_err(PartProblem::Vorbis)
            };
            // The packet was put together from pages, so offsets in the
            // comments count from the start of the block.
            match block.and_then(|block| {
                vorbis_comment::parse(block, 0, limits, budget).map_err(PartProblem::Comment)
            }) {
                Ok(comments) => tags.push(TagBlock::Vorbis(comments)),
                Err(problem) => problems.push(problem),
            }
        }
    }
    Some(Ok(Draft {
        format: Format::Ogg,
        codec: Some(codec),
        container: Container::Ogg,
        at: ident.offset,
        audio: AudioFormat {
            sample_rate: rate,
            bit_depth: None,
            channels: Some(channels),
            bitrate: None,
            duration: None,
        },
        trim: skip.map(|skip| Trim {
            delay: skip.into(),
            padding: 0,
        }),
        md5: None,
        window: (0, file_len),
        seek: SeekIndex::None,
        pictures: Vec::new(),
        tags,
        problems,
        jobs: vec![(
            Job::OggTail {
                serial,
                skip: skip.map_or(0, u64::from),
            },
            Gather::new(file_len.saturating_sub(TAIL), file_len),
        )],
    }))
}
