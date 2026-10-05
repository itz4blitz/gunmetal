//! The facts of WAV and AIFF files, from their format chunks.

use std::num::NonZeroU32;

use crate::catalog::{AudioFormat, Codec, Container};
use crate::formats::aiff::AiffFile;
use crate::formats::detect::Format;
use crate::formats::riff::{ByteRange, WavCodec, WavFile};
use crate::parse::{LimitKind, Limits, ParseFault};

use super::draft::{Draft, Gather, Job, chunk_tag, span};
use super::facts::{PartProblem, SeekIndex};

/// The draft of a WAV file.
///
/// Integer and floating-point PCM are read; any other codec is not. The
/// audio window is the samples of the `data` chunk, and the duration is
/// their length at the format's octets a second. The `id3 ` chunk is read
/// first and the `INFO` list after it. A list larger than a tag may be in
/// memory is skipped and recorded.
pub(super) fn wav(file: WavFile, limits: &Limits) -> Draft {
    let WavFile {
        format,
        data,
        info,
        id3,
        stopped,
        form: _,
    } = file;
    let mut pcm = pcm(Format::Wav, Container::Wav, data, id3, stopped, limits);
    if !matches!(format.codec, WavCodec::Pcm | WavCodec::Float) {
        pcm.codec = None;
    }
    pcm.audio = AudioFormat {
        sample_rate: Some(format.sample_rate),
        bit_depth: format.bits_per_sample,
        channels: Some(format.channels),
        bitrate: None,
        duration: span(
            Some(data.len),
            NonZeroU32::new(format.bytes_per_second),
            &mut pcm.problems,
        ),
    };
    if let Some(list) = info {
        match limits.check(LimitKind::Id3v2TagBytes, list.len, list.offset) {
            Ok(()) => pcm.jobs.insert(
                0,
                (
                    Job::Info,
                    Gather::new(list.offset, list.offset.saturating_add(list.len)),
                ),
            ),
            Err(fault) => pcm.problems.push(PartProblem::Fault(fault)),
        }
    }
    pcm
}

/// The draft of an AIFF or AIFF-C file.
///
/// An AIFF-C file is read when its compression type is one of the PCM
/// ones: `NONE`, `sowt`, `fl32` or `fl64`. The audio window is the
/// samples of the `SSND` chunk, and the duration is the sample frames at
/// the sample rate.
pub(super) fn aiff(file: AiffFile, limits: &Limits) -> Draft {
    let AiffFile {
        format,
        sound,
        id3,
        stopped,
        block_size: _,
    } = file;
    let mut pcm = pcm(Format::Aiff, Container::Aiff, sound, id3, stopped, limits);
    if !matches!(
        format.compression.as_ref(),
        None | Some(b"NONE" | b"sowt" | b"fl32" | b"fl64")
    ) {
        pcm.codec = None;
    }
    pcm.audio = AudioFormat {
        sample_rate: Some(format.sample_rate),
        bit_depth: format.sample_size,
        channels: Some(format.channels),
        bitrate: None,
        duration: span(
            Some(format.sample_frames.into()),
            Some(format.sample_rate.hz()),
            &mut pcm.problems,
        ),
    };
    pcm
}

/// What WAV and AIFF files share: PCM samples in one chunk, an `ID3v2` tag
/// in another, and a walk over the chunks that may have stopped early. The
/// chunk holds one tag, so one is read from it.
fn pcm(
    format: Format,
    container: Container,
    samples: ByteRange,
    id3: Option<ByteRange>,
    stopped: Option<ParseFault>,
    limits: &Limits,
) -> Draft {
    Draft {
        format,
        codec: Some(Codec::Pcm),
        container,
        at: samples.offset,
        audio: AudioFormat::default(),
        trim: None,
        md5: None,
        window: (samples.offset, samples.offset.saturating_add(samples.len)),
        seek: SeekIndex::None,
        pictures: Vec::new(),
        tags: Vec::new(),
        problems: stopped.map(PartProblem::Stopped).into_iter().collect(),
        jobs: id3
            .map(|range| chunk_tag(range, limits))
            .into_iter()
            .collect(),
    }
}
