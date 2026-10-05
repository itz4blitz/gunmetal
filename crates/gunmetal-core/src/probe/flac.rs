//! A FLAC file's facts, from its metadata blocks.

use std::num::NonZeroU64;

use crate::catalog::{AudioFormat, Codec, Container};
use crate::formats::detect::Format;
use crate::formats::flac::metadata::{AudioMd5, FlacMetadata};
use crate::formats::riff::ByteRange;
use crate::parse::Limits;

use super::draft::{Draft, Gather, Job, id3v2, span};
use super::facts::{PartProblem, SeekIndex};

/// The draft of a FLAC file of `file_len` octets whose stream starts
/// after `lead` octets of `ID3v2` tags.
///
/// The audio window runs from the first frame to the end of the file. The
/// Vorbis comment is read first and the leading tags after it, so the
/// file's own tag comes first.
pub(super) fn draft(metadata: FlacMetadata, lead: u64, file_len: u64, limits: &Limits) -> Draft {
    let info = metadata.stream_info;
    let mut problems = metadata
        .problems
        .into_iter()
        .map(PartProblem::Flac)
        .collect();
    let duration = span(
        info.total_samples.map(NonZeroU64::get),
        Some(info.sample_rate.hz()),
        &mut problems,
    );
    let mut jobs = Vec::new();
    if lead > 0 {
        jobs.push(id3v2(
            ByteRange {
                offset: 0,
                len: lead,
            },
            limits,
        ));
    }
    jobs.extend(
        metadata
            .comment
            .map(|range| (Job::Comment, Gather::new(range.start, range.end))),
    );
    Draft {
        format: Format::Flac,
        codec: Some(Codec::Flac),
        container: Container::Flac,
        at: lead,
        audio: AudioFormat {
            sample_rate: Some(info.sample_rate),
            bit_depth: Some(info.bits_per_sample),
            channels: Some(info.channels),
            bitrate: None,
            duration,
        },
        trim: None,
        md5: info.md5.map(|AudioMd5(md5)| md5),
        window: (metadata.audio_start, file_len),
        seek: SeekIndex::Flac(metadata.seek_table),
        pictures: metadata
            .pictures
            .iter()
            .map(|picture| {
                (
                    picture.picture_type,
                    picture.data.end.saturating_sub(picture.data.start),
                )
            })
            .collect(),
        tags: Vec::new(),
        covers: Vec::new(),
        problems,
        jobs,
    }
}
