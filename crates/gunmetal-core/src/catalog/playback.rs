//! Playback data per file: gain tags and the encoder's trim (API-CAT-05).
//!
//! These hold what the file says. Deciding what gain to apply, and
//! clamping gain from tags (SEC-MED-015), is the gain decision's job
//! (WP-028).

use crate::values::{GainDb, PeakRatio};

use super::coded::coded;

coded! {
    /// Which loudness reference a gain is relative to.
    GainScale: u8 {
        /// `ReplayGain`: a reference of −18 LUFS.
        ReplayGain = 1,
        /// EBU R128, as Opus's `R128_*_GAIN` tags: a reference of −23 LUFS.
        R128 = 2,
    }
}

/// One gain from a tag, with its peak when tagged.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gain {
    /// The reference the gain is relative to.
    pub scale: GainScale,
    /// The gain.
    pub gain: GainDb,
    /// The largest sample, when tagged.
    pub peak: Option<PeakRatio>,
}

/// The track and album gains a file's tags hold.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GainTags {
    /// The track's gain, when tagged.
    pub track: Option<Gain>,
    /// The album's gain, when tagged.
    pub album: Option<Gain>,
}

/// Samples the encoder added before and after the audio, which gapless
/// playback removes (MUS-069): from a LAME header, MP4's `iTunSMPB` or an
/// Opus header's pre-skip. Counted at the stream's sample rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Trim {
    /// Samples to skip at the start.
    pub delay: u32,
    /// Samples to drop at the end.
    pub padding: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gain_scales_have_these_codes() {
        let scales: Vec<(GainScale, u8)> = GainScale::ALL.iter().map(|s| (*s, s.code())).collect();
        assert_eq!(scales, [(GainScale::ReplayGain, 1), (GainScale::R128, 2)]);
        let read: Vec<Option<GainScale>> = (0..=3).map(GainScale::from_code).collect();
        assert_eq!(
            read,
            [
                None,
                Some(GainScale::ReplayGain),
                Some(GainScale::R128),
                None
            ]
        );
    }
}
