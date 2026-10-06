//! Test support shared by the packager's tests.

use std::num::NonZeroU64;

use gunmetal_testkit::flac_frames::{Header, frame};

use crate::catalog::Trim;
use crate::formats::flac::metadata::StreamInfo;
use crate::values::{BitDepth, Channels, SampleRate};

use super::{FrameIndex, IndexPoint, PackTrack};

/// The stack size SEC-MED-001 names for its property tests, in octets.
const SMALL_STACK: usize = 262_144;

/// Runs `work` on a fresh thread with a 256 KiB stack, so a packager that
/// recurses too deeply fails its test.
pub(super) fn on_small_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(SMALL_STACK)
        .spawn(work)
        .expect("the test thread starts")
        .join()
        .expect("the code under test returned instead of panicking")
}

/// The STREAMINFO block of the test file: CD audio in blocks of 4,096
/// samples, 10,240 samples in all, from an encoder that stated neither its
/// frame sizes nor an MD5.
pub(super) fn cd_info() -> StreamInfo {
    StreamInfo {
        min_block_size: 4_096,
        max_block_size: 4_096,
        min_frame_size: None,
        max_frame_size: None,
        sample_rate: SampleRate::new(44_100).expect("44.1 kHz is a sample rate"),
        channels: Channels::new(2).expect("stereo is a channel count"),
        bits_per_sample: BitDepth::new(16).expect("16 bits is a depth"),
        total_samples: NonZeroU64::new(10_240),
        md5: None,
    }
}

/// The test file's track, with nothing to trim.
pub(super) fn cd_track() -> PackTrack {
    let trim = Trim {
        delay: 0,
        padding: 0,
    };
    PackTrack::flac(&cd_info(), trim).expect("CD audio fits a STREAMINFO block")
}

/// A frame of 16-bit stereo at 44.1 kHz with fixed blocking: frame `number`
/// of the stream, holding the samples block size code `block_size` stands
/// for. With a number below 128 and a code that adds no field, it is 14
/// octets long.
pub(super) fn cd_frame(number: u64, block_size: u8) -> Vec<u8> {
    let header = Header {
        block_size,
        ..Header::cd(false, number)
    };
    frame(&header, &[16, 16])
}

/// The three frames of the test file, 14 octets each: 4,096 samples, 4,096
/// samples and a last frame of 2,048.
pub(super) fn cd_frames() -> [Vec<u8>; 3] {
    [
        cd_frame(0, 0b1100),
        cd_frame(1, 0b1100),
        cd_frame(2, 0b1011),
    ]
}

/// The point at `offset` in the file and at `sample` in the stream.
pub(super) const fn point(offset: u64, sample: u64) -> IndexPoint {
    IndexPoint { offset, sample }
}

/// An index whose segments start at `starts` and whose last ends at `end`,
/// each a file offset and a sample.
pub(super) fn index_of(starts: &[(u64, u64)], end: (u64, u64)) -> FrameIndex {
    FrameIndex {
        starts: starts
            .iter()
            .map(|&(offset, sample)| point(offset, sample))
            .collect(),
        end: point(end.0, end.1),
    }
}
