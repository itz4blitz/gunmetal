//! Whole synthetic files through the probe, each compared with what the
//! probe must find in it, written out.
//!
//! Every file is built from the testkit's builders, which share no code
//! with the parsers. Offsets and lengths in the expected values are worked
//! out from the layout each test describes.

mod flac;
mod limits;
mod mp4;
mod mpeg;
mod ogg;
mod pcm;
mod properties;
mod refusals;
mod replies;

use crate::catalog::{
    ArtworkRef, ArtworkSource, AudioFormat, Bitrate, ByteRange, Codec, Container, LyricsOrigin,
    LyricsSource, LyricsTiming, PictureType, TechInfo,
};
use crate::parse::{Budget, DriveError, LimitKind, Limits, drive};
use crate::text::Text;
use crate::values::{BitDepth, Channels, Duration, SampleRate};

use super::{FIXED_STEPS, ProbeError, Probed, STEPS_PER_OCTET, probe};

/// What a probe answers with.
type Outcome = Result<Probed, ProbeError>;

/// How many octets `file` holds.
fn len(file: &[u8]) -> u64 {
    u64::try_from(file.len()).unwrap()
}

/// Probes `file` under `limits` with `steps` steps, and returns the
/// answer, or the read a host refused, with the steps left.
fn run_with(
    file: &[u8],
    ext: Option<&str>,
    limits: Limits,
    steps: u64,
) -> (Result<Outcome, DriveError>, u64) {
    let mut budget = Budget::for_input(0, 0, steps);
    let answer = drive(probe(ext, limits, &mut budget), file, &limits);
    (answer, budget.remaining())
}

/// The steps the probe documents as enough for `file`.
fn enough(file: &[u8]) -> u64 {
    len(file) * STEPS_PER_OCTET + FIXED_STEPS
}

/// Probes `file` under the default limits with the documented budget.
fn run(file: &[u8], ext: Option<&str>) -> Outcome {
    run_under(file, ext, Limits::DEFAULT)
}

/// Probes `file` under `limits` with the documented budget.
fn run_under(file: &[u8], ext: Option<&str>, limits: Limits) -> Outcome {
    run_with(file, ext, limits, enough(file))
        .0
        .expect("the probe asks only for reads a host allows")
}

/// `count` zero octets.
fn zeros(count: usize) -> Vec<u8> {
    let mut octets = gunmetal_testkit::bytes::Bytes::new();
    octets.zeros(count);
    octets.into_vec()
}

/// `count` `ID3v2.3` tags with no frames, back to back: ten octets each.
fn empty_tags(count: usize) -> Vec<u8> {
    (0..count).flat_map(|_| *b"ID3\x03\0\0\0\0\0\0").collect()
}

/// The default limits with `kind` lowered to `value`.
fn lowered(kind: LimitKind, value: u64) -> Limits {
    Limits::DEFAULT.with_override(kind, value).unwrap()
}

/// Text that was decoded whole, with nothing replaced.
fn text(value: &str) -> Text {
    Text {
        value: value.to_owned(),
        truncated: false,
        replaced: false,
    }
}

/// The technical facts of a file.
fn tech(
    codec: Codec,
    container: Container,
    (hz, bits, channels): (u32, Option<u32>, u32),
    bitrate: Option<u32>,
    millis: Option<u64>,
) -> TechInfo {
    TechInfo::new(
        codec,
        container,
        AudioFormat {
            sample_rate: Some(SampleRate::new(hz).unwrap()),
            bit_depth: bits.map(|bits| BitDepth::new(bits).unwrap()),
            channels: Some(Channels::new(channels).unwrap()),
            bitrate: bitrate.map(|bps| Bitrate::new(bps).unwrap()),
            duration: millis.map(|millis| Duration::from_millis(millis).unwrap()),
        },
    )
    .unwrap()
}

/// The octets from `start` up to `end`.
fn range(start: u64, end: u64) -> ByteRange {
    ByteRange::new(start, end).unwrap()
}

/// The embedded picture number `index`.
fn artwork(index: u16, picture_type: PictureType, byte_len: u64) -> ArtworkRef {
    ArtworkRef {
        source: ArtworkSource::Embedded { index },
        picture_type,
        byte_len,
    }
}

/// Lyrics found at `origin`, timed as `timing`.
fn lyrics(origin: LyricsOrigin, timing: LyricsTiming) -> LyricsSource {
    LyricsSource::new(origin, timing).unwrap()
}
