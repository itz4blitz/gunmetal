//! An MP3 file's facts, from its first frame and its encoder headers.

use crate::catalog::{AudioFormat, Codec, Container, Trim};
use crate::formats::detect::Format;
use crate::formats::mpa::{ChannelMode, MpegStream};
use crate::formats::riff::ByteRange;
use crate::parse::{Limits, ParseFault};
use crate::values::Channels;

use super::draft::{Draft, Gather, Job, id3v2, span};
use super::facts::{PartProblem, SeekIndex};

/// Octets at the end of an MP3 file that say whether it ends with an
/// `ID3v1` tag, 128 octets, and whether an APE tag's 32-octet footer ends
/// the file or lies just before that tag.
const TAIL: u64 = 160;

/// The draft of an MP3 file of `file_len` octets that starts with `lead`
/// octets of `ID3v2` tags.
///
/// The duration is the frames' samples without the encoder's delay and
/// padding, when a LAME header gives them. The audio window starts at the
/// first frame; reading the end of the file then ends it where the first
/// tag there starts. The leading tags are read first, then the end.
pub(super) fn draft(stream: MpegStream, lead: u64, file_len: u64, limits: &Limits) -> Draft {
    let header = stream.header;
    let mut problems: Vec<PartProblem> = [
        fault(stream.xing.as_ref()),
        fault(stream.lame.as_ref()),
        fault(stream.vbri.as_ref()),
    ]
    .into_iter()
    .flatten()
    .collect();
    let trim = stream.lame.and_then(Result::ok).map(|lame| Trim {
        delay: lame.delay.into(),
        padding: lame.padding.into(),
    });
    let added = trim.map_or(0, |trim| {
        u64::from(trim.delay).saturating_add(trim.padding.into())
    });
    let samples = stream
        .frames
        .saturating_mul(header.samples.into())
        .saturating_sub(added);
    let duration = span(Some(samples), Some(header.sample_rate.hz()), &mut problems);
    let channels = if header.mode == ChannelMode::Mono {
        1
    } else {
        2
    };
    let mut jobs = vec![(
        Job::Tail,
        Gather::new(file_len.saturating_sub(TAIL), file_len),
    )];
    if lead > 0 {
        jobs.push(id3v2(
            ByteRange {
                offset: 0,
                len: lead,
            },
            limits,
        ));
    }
    Draft {
        format: Format::Mpeg,
        codec: Some(Codec::Mp3),
        container: Container::Mpeg,
        at: stream.start,
        audio: AudioFormat {
            sample_rate: Some(header.sample_rate),
            bit_depth: None,
            channels: Channels::new(channels).ok(),
            bitrate: None,
            duration,
        },
        trim,
        md5: None,
        window: (stream.start, file_len),
        seek: SeekIndex::Mpeg(stream.seek),
        pictures: Vec::new(),
        tags: Vec::new(),
        problems,
        jobs,
    }
}

/// The problem to record for an encoder header that could not be read.
fn fault<T>(header: Option<&Result<T, ParseFault>>) -> Option<PartProblem> {
    header
        .and_then(|header| header.as_ref().err())
        .map(|&fault| PartProblem::Encoder(fault))
}
