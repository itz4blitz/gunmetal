//! The track a packager writes: its codec configuration and the trim that
//! makes it play without a gap.

use std::num::NonZeroU32;

use crate::catalog::Trim;
use crate::formats::flac::metadata::StreamInfo;

use super::error::PackError;

/// Which value of a track does not fit the box field that holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackField {
    /// The channel count.
    Channels,
    /// The bits per sample.
    BitDepth,
    /// The smallest or the largest frame size.
    FrameSize,
    /// The number of samples in the stream.
    TotalSamples,
}

/// A track the packager can write as fragmented MP4.
///
/// It is made only from values that fit the boxes they go in, so writing
/// its initialisation segment cannot fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackTrack {
    /// The STREAMINFO block of the FLAC stream.
    pub(super) info: StreamInfo,
    /// The samples to cut from the ends of the decoded audio.
    pub(super) trim: Trim,
}

impl PackTrack {
    /// A FLAC track, written as a `fLaC` sample entry whose `dfLa` box
    /// holds the stream's STREAMINFO block, `info`. Each sample is one FLAC
    /// frame, and the timescale is the sample rate.
    ///
    /// `trim` is what the scan found: the samples to drop at the start and
    /// at the end of the decoded audio, at the stream's sample rate. The
    /// packager writes it as it is given.
    ///
    /// # Errors
    ///
    /// Returns [`PackError::Field`] for a value that does not fit a
    /// STREAMINFO block: more than 8 channels, more than 32 bits per
    /// sample, a frame size of more than 24 bits or a sample count of more
    /// than 36 bits.
    pub fn flac(info: &StreamInfo, trim: Trim) -> Result<Self, PackError> {
        Ok(Self { info: *info, trim })
    }

    /// Units of the track's time in one second: the sample rate.
    #[must_use]
    pub fn timescale(&self) -> NonZeroU32 {
        self.info.sample_rate.hz()
    }
}

#[cfg(test)]
mod tests {
    use std::num::{NonZeroU32, NonZeroU64};

    use super::super::testing::cd_info;
    use super::{PackTrack, TrackField};
    use crate::catalog::Trim;
    use crate::formats::flac::metadata::StreamInfo;
    use crate::package::PackError;
    use crate::values::{BitDepth, Channels, SampleRate};

    const NO_TRIM: Trim = Trim {
        delay: 0,
        padding: 0,
    };

    fn refused(field: TrackField, value: u64, max: u64) -> Result<PackTrack, PackError> {
        Err(PackError::Field { field, value, max })
    }

    #[test]
    fn takes_a_flac_track_and_counts_its_time_in_samples() {
        let trim = Trim {
            delay: 2_112,
            padding: 1_000,
        };
        let track = PackTrack::flac(&cd_info(), trim);
        assert_eq!(
            track,
            Ok(PackTrack {
                info: cd_info(),
                trim,
            })
        );
        assert_eq!(track.map(|track| track.timescale().get()), Ok(44_100));
        let hi_res = StreamInfo {
            sample_rate: SampleRate::new(192_000).unwrap(),
            ..cd_info()
        };
        assert_eq!(
            PackTrack::flac(&hi_res, NO_TRIM).map(|track| track.timescale().get()),
            Ok(192_000)
        );
    }

    #[test]
    fn takes_the_largest_values_a_stream_info_block_holds() {
        let largest = StreamInfo {
            min_block_size: u16::MAX,
            max_block_size: u16::MAX,
            min_frame_size: NonZeroU32::new(0xFF_FFFF),
            max_frame_size: NonZeroU32::new(0xFF_FFFF),
            sample_rate: SampleRate::new(768_000).unwrap(),
            channels: Channels::new(8).unwrap(),
            bits_per_sample: BitDepth::new(32).unwrap(),
            total_samples: NonZeroU64::new(0xF_FFFF_FFFF),
            md5: None,
        };
        assert_eq!(
            PackTrack::flac(&largest, NO_TRIM),
            Ok(PackTrack {
                info: largest,
                trim: NO_TRIM,
            })
        );
    }

    #[test]
    fn refuses_each_value_a_stream_info_block_cannot_hold() {
        let channels = StreamInfo {
            channels: Channels::new(9).unwrap(),
            ..cd_info()
        };
        assert_eq!(
            PackTrack::flac(&channels, NO_TRIM),
            refused(TrackField::Channels, 9, 8)
        );
        let bits = StreamInfo {
            bits_per_sample: BitDepth::new(33).unwrap(),
            ..cd_info()
        };
        assert_eq!(
            PackTrack::flac(&bits, NO_TRIM),
            refused(TrackField::BitDepth, 33, 32)
        );
        let smallest_frame = StreamInfo {
            min_frame_size: NonZeroU32::new(0x100_0000),
            ..cd_info()
        };
        assert_eq!(
            PackTrack::flac(&smallest_frame, NO_TRIM),
            refused(TrackField::FrameSize, 16_777_216, 16_777_215)
        );
        let largest_frame = StreamInfo {
            max_frame_size: NonZeroU32::new(0x100_0001),
            ..cd_info()
        };
        assert_eq!(
            PackTrack::flac(&largest_frame, NO_TRIM),
            refused(TrackField::FrameSize, 16_777_217, 16_777_215)
        );
        let samples = StreamInfo {
            total_samples: NonZeroU64::new(0x10_0000_0000),
            ..cd_info()
        };
        assert_eq!(
            PackTrack::flac(&samples, NO_TRIM),
            refused(TrackField::TotalSamples, 68_719_476_736, 68_719_476_735)
        );
    }
}
