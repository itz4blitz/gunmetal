//! Playback gain: which adjustment to apply, from where, and which clamp
//! kept it safe (MUS-084, MUS-085, MUS-087, MUS-088, MUS-089; SEC-MED-015).
//!
//! [`decide`] is a pure function of a track's tags, its Opus header gain,
//! the loudness mode, the target and whether the From-lane neighbours are
//! the same album. It never panics: every gain and peak that reaches it is
//! already a [`GainDb`] or [`PeakRatio`], so NaN was refused when those
//! values were parsed (SEC-MED-014).
//!
//! Tag gains are `ReplayGain` 2.0 (relative to [`Lufs::REPLAYGAIN`]) or
//! RFC 7845 R128 (relative to [`Lufs::R128`]). The Opus header output gain
//! always applies, and an R128 tag is added to it (RFC 7845 §5.1 and §5.2).
//! The result is then shifted to the chosen target, clamped to −30 dB to
//! +12 dB, refused when it would boost without a peak, and reduced so the
//! recorded peak cannot go above full scale.

use crate::values::{GainDb, PeakRatio};

/// Lowest gain taken from tags or applied after safety clamps (SEC-MED-015).
pub const TAG_GAIN_MIN_DB: f32 = -30.0;
/// Highest gain taken from tags or applied after safety clamps (SEC-MED-015).
pub const TAG_GAIN_MAX_DB: f32 = 12.0;
/// Attenuation for a track with no tag and no measurement, at the
/// `ReplayGain` 2.0 reference (MUS-089). Roon uses the same figure.
pub const FALLBACK_DB: f32 = -5.0;

/// Integrated loudness in LUFS, finite and from −70 to 0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lufs(f32);

impl Lufs {
    /// Quietest target accepted.
    pub const MIN: f32 = -70.0;
    /// Loudest target accepted, full scale.
    pub const MAX: f32 = 0.0;
    /// `ReplayGain` 2.0 reference, the default target.
    pub const REPLAYGAIN: Self = Self(-18.0);
    /// EBU R128 / RFC 7845 reference.
    pub const R128: Self = Self(-23.0);

    /// A loudness of `lufs` LUFS.
    ///
    /// # Errors
    ///
    /// [`LufsError::Unusable`] when the value is not finite or is outside
    /// [`Self::MIN`] to [`Self::MAX`].
    pub fn new(lufs: f32) -> Result<Self, LufsError> {
        if (Self::MIN..=Self::MAX).contains(&lufs) {
            Ok(Self(lufs))
        } else {
            Err(LufsError::Unusable)
        }
    }

    /// The loudness in LUFS.
    #[must_use]
    pub const fn lufs(self) -> f32 {
        self.0
    }
}

/// Why a [`Lufs`] value was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LufsError {
    /// Not finite, or outside [`Lufs::MIN`] to [`Lufs::MAX`].
    Unusable,
}

/// Which scheme the file's tags were written against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    /// `REPLAYGAIN_*` tags, relative to [`Lufs::REPLAYGAIN`].
    ReplayGain,
    /// `R128_*_GAIN` tags, relative to [`Lufs::R128`].
    R128,
}

impl Scheme {
    /// The LUFS level those tags bring a track to before the user's target.
    #[must_use]
    pub const fn reference(self) -> Lufs {
        match self {
            Self::ReplayGain => Lufs::REPLAYGAIN,
            Self::R128 => Lufs::R128,
        }
    }
}

/// How the player chooses between track and album gain (MUS-087).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Album gain inside a From-lane album run, track gain otherwise.
    #[default]
    Auto,
    /// Track gain whenever it exists.
    Track,
    /// Album gain whenever it exists, even for a single shuffled track.
    Album,
    /// No levelling. An Opus header gain still applies (RFC 7845).
    Off,
}

/// Where the chosen gain came from (MUS-089, MUS-090, MUS-236).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// A `ReplayGain` or R128 track tag.
    TaggedTrack,
    /// A `ReplayGain` or R128 album tag.
    TaggedAlbum,
    /// Scan-time integrated loudness (MUS-086).
    Measured,
    /// The fixed fallback (MUS-089).
    Estimated,
    /// Levelling is off; only a codec header gain, if any, applies.
    Off,
}

/// Which safety clamp changed the gain (SEC-MED-015).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clamp {
    /// The computed gain was already inside the safe range and peak limit.
    None,
    /// The gain was held to [`TAG_GAIN_MIN_DB`] to [`TAG_GAIN_MAX_DB`].
    Range,
    /// A boost was refused because no valid peak was known.
    NoPeak,
    /// The gain was reduced so the recorded peak stays at or below full scale.
    Peak,
}

/// Everything [`decide`] needs about one track.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GainInput {
    /// `ReplayGain` or R128 track gain.
    pub track_gain: Option<GainDb>,
    /// `ReplayGain` or R128 album gain.
    pub album_gain: Option<GainDb>,
    /// Track peak as a ratio of full scale.
    pub track_peak: Option<PeakRatio>,
    /// Album peak as a ratio of full scale.
    pub album_peak: Option<PeakRatio>,
    /// Which reference the tags were written against.
    pub scheme: Scheme,
    /// Opus identification-header output gain (RFC 7845). Always added.
    pub opus_header: Option<GainDb>,
    /// Integrated loudness from a measurement pass, when tags are missing.
    pub measured: Option<Lufs>,
    /// The previous From-lane item is the same album.
    pub prev_same_album: bool,
    /// The next From-lane item is the same album.
    pub next_same_album: bool,
}

/// The gain the player applies, where it came from, and which clamp ran.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GainDecision {
    /// The gain to apply, after every clamp.
    pub applied: GainDb,
    /// Where the pre-clamp gain came from.
    pub source: Source,
    /// The last clamp that changed the value, or [`Clamp::None`].
    pub clamp: Clamp,
}

/// Chooses the gain to apply for `input` under `mode` at `target`.
#[must_use]
pub fn decide(input: &GainInput, mode: Mode, target: Lufs) -> GainDecision {
    let header = input.opus_header.map_or(0.0, GainDb::db);
    match mode {
        Mode::Off => finish(
            header,
            Source::Off,
            peak_for(input, Source::Off),
            Clamp::None,
        ),
        Mode::Album => level(input, target, header, true),
        Mode::Auto => level(
            input,
            target,
            header,
            input.prev_same_album || input.next_same_album,
        ),
        Mode::Track => level(input, target, header, false),
    }
}

fn level(input: &GainInput, target: Lufs, header: f32, want_album: bool) -> GainDecision {
    let chosen = if want_album {
        pick(
            input.album_gain,
            Source::TaggedAlbum,
            input.track_gain,
            Source::TaggedTrack,
        )
    } else {
        pick(
            input.track_gain,
            Source::TaggedTrack,
            input.album_gain,
            Source::TaggedAlbum,
        )
    };
    match chosen {
        Some((gain, source)) => {
            let offset = target.lufs() - input.scheme.reference().lufs();
            finish(
                gain.db() + header + offset,
                source,
                peak_for(input, source),
                Clamp::None,
            )
        }
        None => {
            if let Some(measured) = input.measured {
                finish(
                    target.lufs() - measured.lufs() + header,
                    Source::Measured,
                    peak_for(input, Source::Measured),
                    Clamp::None,
                )
            } else {
                let offset = target.lufs() - Lufs::REPLAYGAIN.lufs();
                finish(
                    FALLBACK_DB + header + offset,
                    Source::Estimated,
                    peak_for(input, Source::Estimated),
                    Clamp::None,
                )
            }
        }
    }
}

fn pick(
    preferred: Option<GainDb>,
    preferred_source: Source,
    other: Option<GainDb>,
    other_source: Source,
) -> Option<(GainDb, Source)> {
    match preferred {
        Some(gain) => Some((gain, preferred_source)),
        None => other.map(|gain| (gain, other_source)),
    }
}

fn peak_for(input: &GainInput, source: Source) -> Option<PeakRatio> {
    match source {
        Source::TaggedAlbum => input.album_peak.or(input.track_peak),
        Source::TaggedTrack | Source::Measured | Source::Estimated | Source::Off => {
            input.track_peak.or(input.album_peak)
        }
    }
}

fn finish(raw: f32, source: Source, peak: Option<PeakRatio>, mut clamp: Clamp) -> GainDecision {
    let ranged = raw.clamp(TAG_GAIN_MIN_DB, TAG_GAIN_MAX_DB);
    if ranged.to_bits() != raw.to_bits() {
        clamp = Clamp::Range;
    }
    let applied = match peak {
        None => {
            if ranged > 0.0 {
                clamp = Clamp::NoPeak;
                0.0
            } else {
                ranged
            }
        }
        Some(peak) => {
            let ratio = peak.ratio();
            if ratio > 0.0 {
                let headroom = peak_headroom_db(ratio);
                if ranged > headroom {
                    clamp = Clamp::Peak;
                    headroom
                } else {
                    ranged
                }
            } else if ranged > 0.0 {
                // A zero peak cannot bound a boost, and must not be a
                // divisor (SEC-MED-014), so it is treated as missing.
                clamp = Clamp::NoPeak;
                0.0
            } else {
                ranged
            }
        }
    };
    GainDecision {
        applied: gain_in_tag_range(applied),
        source,
        clamp,
    }
}

/// Headroom to full scale: 20 · log10(1 / peak), correctly rounded to f32.
fn peak_headroom_db(ratio: f32) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "20·log10(1/peak) for a peak in (0, 16] is a finite f32 before the −30…+12 clamp"
    )]
    {
        (20.0 * (1.0 / f64::from(ratio)).log10()) as f32
    }
}

fn gain_in_tag_range(db: f32) -> GainDb {
    #[expect(
        clippy::unwrap_used,
        reason = "finish clamps to −30 dB to +12 dB, inside GainDb::MIN to GainDb::MAX"
    )]
    GainDb::new(db).unwrap()
}

#[cfg(test)]
mod tests {
    use super::{
        Clamp, FALLBACK_DB, GainDecision, GainInput, Lufs, LufsError, Mode, Scheme, Source,
        TAG_GAIN_MAX_DB, TAG_GAIN_MIN_DB, decide,
    };
    use crate::values::{GainDb, PeakRatio};
    use proptest::prelude::*;

    fn db(value: f32) -> GainDb {
        GainDb::new(value).unwrap()
    }

    fn peak(ratio: f32) -> PeakRatio {
        PeakRatio::new(ratio).unwrap()
    }

    fn blank() -> GainInput {
        GainInput {
            track_gain: None,
            album_gain: None,
            track_peak: None,
            album_peak: None,
            scheme: Scheme::ReplayGain,
            opus_header: None,
            measured: None,
            prev_same_album: false,
            next_same_album: false,
        }
    }

    fn decision(applied: f32, source: Source, clamp: Clamp) -> GainDecision {
        GainDecision {
            applied: db(applied),
            source,
            clamp,
        }
    }

    /// Independent oracle: 20 · log10(1 / peak), correctly rounded to f32.
    /// Used only by the peak-headroom property, never as a unit-test expected.
    fn headroom_db(ratio: f32) -> f32 {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the property oracle rounds 20·log10(1/peak) to the f32 the player applies"
        )]
        {
            (20.0 * (1.0 / f64::from(ratio)).log10()) as f32
        }
    }

    /// Album gain is used only in a From-lane album run (MUS-087).
    #[test]
    fn auto_uses_album_gain_only_inside_an_album_run() {
        let mut input = blank();
        input.track_gain = Some(db(-4.0));
        input.album_gain = Some(db(-8.0));
        input.track_peak = Some(peak(0.5));
        input.album_peak = Some(peak(0.5));
        let target = Lufs::REPLAYGAIN;

        input.prev_same_album = false;
        input.next_same_album = false;
        assert_eq!(
            decide(&input, Mode::Auto, target),
            decision(-4.0, Source::TaggedTrack, Clamp::None)
        );

        input.prev_same_album = true;
        input.next_same_album = false;
        assert_eq!(
            decide(&input, Mode::Auto, target),
            decision(-8.0, Source::TaggedAlbum, Clamp::None)
        );

        input.prev_same_album = false;
        input.next_same_album = true;
        assert_eq!(
            decide(&input, Mode::Auto, target),
            decision(-8.0, Source::TaggedAlbum, Clamp::None)
        );

        input.prev_same_album = true;
        input.next_same_album = true;
        assert_eq!(
            decide(&input, Mode::Auto, target),
            decision(-8.0, Source::TaggedAlbum, Clamp::None)
        );

        input.track_gain = None;
        input.prev_same_album = false;
        input.next_same_album = false;
        assert_eq!(
            decide(&input, Mode::Auto, target),
            decision(-8.0, Source::TaggedAlbum, Clamp::None)
        );
    }

    /// Track and album modes ignore the From-lane neighbours (MUS-087).
    #[test]
    fn track_and_album_modes_ignore_the_album_run() {
        let mut input = blank();
        input.track_gain = Some(db(-4.0));
        input.album_gain = Some(db(-8.0));
        input.track_peak = Some(peak(0.5));
        input.album_peak = Some(peak(0.5));
        input.prev_same_album = true;
        input.next_same_album = true;
        let target = Lufs::REPLAYGAIN;

        assert_eq!(
            decide(&input, Mode::Track, target),
            decision(-4.0, Source::TaggedTrack, Clamp::None)
        );
        assert_eq!(
            decide(&input, Mode::Album, target),
            decision(-8.0, Source::TaggedAlbum, Clamp::None)
        );

        input.prev_same_album = false;
        input.next_same_album = false;
        assert_eq!(
            decide(&input, Mode::Album, target),
            decision(-8.0, Source::TaggedAlbum, Clamp::None)
        );
        assert_eq!(
            decide(&input, Mode::Track, target),
            decision(-4.0, Source::TaggedTrack, Clamp::None)
        );
    }

    /// Missing preferred tag falls back to the other, then measured, then estimated (MUS-089).
    #[test]
    fn missing_preferred_tag_falls_back_to_the_other_then_measured_then_estimated() {
        let target = Lufs::REPLAYGAIN;
        let mut input = blank();
        input.album_gain = Some(db(-8.0));
        input.album_peak = Some(peak(0.5));
        assert_eq!(
            decide(&input, Mode::Track, target),
            decision(-8.0, Source::TaggedAlbum, Clamp::None)
        );

        input = blank();
        input.track_gain = Some(db(-4.0));
        input.track_peak = Some(peak(0.5));
        assert_eq!(
            decide(&input, Mode::Album, target),
            decision(-4.0, Source::TaggedTrack, Clamp::None)
        );

        input = blank();
        input.measured = Some(Lufs::new(-16.0).unwrap());
        input.track_peak = Some(peak(0.5));
        assert_eq!(
            decide(&input, Mode::Track, target),
            decision(-2.0, Source::Measured, Clamp::None)
        );

        input = blank();
        assert_eq!(
            decide(&input, Mode::Track, target),
            decision(FALLBACK_DB, Source::Estimated, Clamp::None)
        );
    }

    /// A tag is used in preference to a scan-time measurement (MUS-084).
    #[test]
    fn tags_win_over_a_measurement() {
        let mut input = blank();
        input.track_gain = Some(db(-6.5));
        input.track_peak = Some(peak(0.5));
        input.measured = Some(Lufs::new(-10.0).unwrap());
        assert_eq!(
            decide(&input, Mode::Track, Lufs::REPLAYGAIN),
            decision(-6.5, Source::TaggedTrack, Clamp::None)
        );
    }

    /// Verifies: SEC-MED-015
    #[test]
    fn grid_of_tag_gains_and_peaks() {
        let target = Lufs::REPLAYGAIN;
        let cases = [
            (-6.5, Some(0.5), -6.5, Clamp::None),
            (0.0, Some(1.0), 0.0, Clamp::None),
            (0.0, None, 0.0, Clamp::None),
            (0.0, Some(0.5), 0.0, Clamp::None),
            (6.0, Some(0.5), 6.0, Clamp::None),
            // 20·log10(1/0.5) = 20·log10(2) ≈ 6.020599913279624
            (6.020_6, Some(0.5), 6.020_6, Clamp::None),
            (12.0, Some(0.1), 12.0, Clamp::None),
            (60.0, Some(0.1), 12.0, Clamp::Range),
            (60.0, None, 0.0, Clamp::NoPeak),
            (60.0, Some(1.0), 0.0, Clamp::Peak),
            (3.0, Some(1.0), 0.0, Clamp::Peak),
            // 20·log10(1/1.5) ≈ −3.521825181113625
            (3.0, Some(1.5), -3.521_825, Clamp::Peak),
            (-2.0, Some(1.5), -3.521_825, Clamp::Peak),
            (-6.5, Some(1.5), -6.5, Clamp::None),
            (-60.0, Some(1.0), -30.0, Clamp::Range),
            (-30.0, Some(1.0), -30.0, Clamp::None),
            (12.0, None, 0.0, Clamp::NoPeak),
            (-6.5, None, -6.5, Clamp::None),
            (6.0, Some(0.0), 0.0, Clamp::NoPeak),
            (0.0, Some(0.0), 0.0, Clamp::None),
            (-6.5, Some(0.0), -6.5, Clamp::None),
        ];
        for (gain, peak_ratio, applied, clamp) in cases {
            let mut input = blank();
            input.track_gain = Some(db(gain));
            input.track_peak = peak_ratio.map(peak);
            assert_eq!(
                decide(&input, Mode::Track, target),
                decision(applied, Source::TaggedTrack, clamp),
                "gain {gain} peak {peak_ratio:?}"
            );
        }
    }

    /// Opus header output gain always applies; an R128 tag is added to it (MUS-085).
    #[test]
    fn opus_header_gain_plus_r128_track_gain() {
        let mut input = blank();
        input.scheme = Scheme::R128;
        input.opus_header = Some(db(-5.0));
        input.track_gain = Some(db(-3.0));
        input.track_peak = Some(peak(0.5));

        assert_eq!(
            decide(&input, Mode::Track, Lufs::R128),
            decision(-8.0, Source::TaggedTrack, Clamp::None)
        );
        assert_eq!(
            decide(&input, Mode::Track, Lufs::REPLAYGAIN),
            decision(-3.0, Source::TaggedTrack, Clamp::None)
        );
    }

    /// Opus header output gain always applies; a `ReplayGain` tag is added to it (MUS-085).
    #[test]
    fn opus_header_gain_plus_replaygain_track_gain() {
        let mut input = blank();
        input.scheme = Scheme::ReplayGain;
        input.opus_header = Some(db(-3.0));
        input.track_gain = Some(db(-4.0));
        input.track_peak = Some(peak(0.5));

        assert_eq!(
            decide(&input, Mode::Track, Lufs::REPLAYGAIN),
            decision(-7.0, Source::TaggedTrack, Clamp::None)
        );

        // Tight peak: the header is added before the peak clamp, so −4−3 stays −7.
        input.track_peak = Some(peak(1.0));
        assert_eq!(
            decide(&input, Mode::Track, Lufs::REPLAYGAIN),
            decision(-7.0, Source::TaggedTrack, Clamp::None)
        );
    }

    /// A zero `ReplayGain` tag stays at the `ReplayGain` reference; R128 adds 5 dB at −18 LUFS (MUS-084).
    #[test]
    fn replaygain_zero_tag_stays_zero_at_its_reference_and_r128_adds_five() {
        let mut rg = blank();
        rg.track_gain = Some(db(0.0));
        rg.track_peak = Some(peak(0.5));
        assert_eq!(
            decide(&rg, Mode::Track, Lufs::REPLAYGAIN),
            decision(0.0, Source::TaggedTrack, Clamp::None)
        );

        let mut r128 = rg;
        r128.scheme = Scheme::R128;
        assert_eq!(
            decide(&r128, Mode::Track, Lufs::REPLAYGAIN),
            decision(5.0, Source::TaggedTrack, Clamp::None)
        );
        assert_eq!(
            decide(&r128, Mode::Track, Lufs::R128),
            decision(0.0, Source::TaggedTrack, Clamp::None)
        );
    }

    /// Verifies: SEC-MED-015
    #[test]
    fn target_shift_is_capped_by_true_peak_headroom() {
        let mut input = blank();
        input.track_gain = Some(db(0.0));
        input.track_peak = Some(peak(1.0));
        let loud = Lufs::new(-14.0).unwrap();
        assert_eq!(
            decide(&input, Mode::Track, loud),
            decision(0.0, Source::TaggedTrack, Clamp::Peak)
        );

        input.track_peak = Some(peak(0.5));
        assert_eq!(
            decide(&input, Mode::Track, loud),
            decision(4.0, Source::TaggedTrack, Clamp::None)
        );
    }

    /// Verifies: SEC-MED-015
    #[test]
    fn off_applies_only_the_opus_header() {
        let mut input = blank();
        input.track_gain = Some(db(-6.5));
        input.album_gain = Some(db(-8.0));
        input.opus_header = Some(db(-3.0));
        input.measured = Some(Lufs::new(-10.0).unwrap());
        assert_eq!(
            decide(&input, Mode::Off, Lufs::REPLAYGAIN),
            decision(-3.0, Source::Off, Clamp::None)
        );

        input.opus_header = None;
        assert_eq!(
            decide(&input, Mode::Off, Lufs::REPLAYGAIN),
            decision(0.0, Source::Off, Clamp::None)
        );

        input.opus_header = Some(db(60.0));
        assert_eq!(
            decide(&input, Mode::Off, Lufs::REPLAYGAIN),
            decision(0.0, Source::Off, Clamp::NoPeak)
        );

        input.track_peak = Some(peak(0.1));
        assert_eq!(
            decide(&input, Mode::Off, Lufs::REPLAYGAIN),
            decision(12.0, Source::Off, Clamp::Range)
        );

        // A non-reference target must not add an offset: Off is header only (MUS-085).
        input.opus_header = Some(db(-3.0));
        input.track_peak = Some(peak(0.5));
        assert_eq!(
            decide(&input, Mode::Off, Lufs::new(-14.0).unwrap()),
            decision(-3.0, Source::Off, Clamp::None)
        );
        assert_eq!(
            decide(&input, Mode::Off, Lufs::R128),
            decision(-3.0, Source::Off, Clamp::None)
        );
    }

    /// Verifies: SEC-MED-015
    #[test]
    fn estimated_fallback_shifts_with_the_target() {
        let input = blank();
        assert_eq!(
            decide(&input, Mode::Track, Lufs::REPLAYGAIN),
            decision(-5.0, Source::Estimated, Clamp::None)
        );
        assert_eq!(
            decide(&input, Mode::Auto, Lufs::REPLAYGAIN),
            decision(-5.0, Source::Estimated, Clamp::None)
        );
        assert_eq!(
            decide(&input, Mode::Track, Lufs::new(-14.0).unwrap()),
            decision(-1.0, Source::Estimated, Clamp::None)
        );
        assert_eq!(
            decide(&input, Mode::Track, Lufs::R128),
            decision(-10.0, Source::Estimated, Clamp::None)
        );

        let loud = Lufs::new(0.0).unwrap();
        assert_eq!(
            decide(&input, Mode::Track, loud),
            decision(0.0, Source::Estimated, Clamp::NoPeak)
        );
        let mut with_peak = input;
        with_peak.album_peak = Some(peak(1.0));
        assert_eq!(
            decide(&with_peak, Mode::Track, loud),
            decision(0.0, Source::Estimated, Clamp::Peak)
        );
        with_peak.opus_header = Some(db(-2.0));
        assert_eq!(
            decide(&with_peak, Mode::Track, Lufs::REPLAYGAIN),
            decision(-7.0, Source::Estimated, Clamp::None)
        );
    }

    /// Measured loudness is the difference to the target, plus the Opus header (MUS-089).
    #[test]
    fn measured_loudness_is_the_difference_to_the_target() {
        let mut input = blank();
        input.measured = Some(Lufs::new(-16.0).unwrap());
        input.track_peak = Some(peak(0.5));
        input.opus_header = Some(db(-1.0));
        assert_eq!(
            decide(&input, Mode::Track, Lufs::REPLAYGAIN),
            decision(-3.0, Source::Measured, Clamp::None)
        );
    }

    /// Album source clamps against the album peak, falling back to the track peak (MUS-088).
    #[test]
    fn album_source_prefers_the_album_peak() {
        let mut input = blank();
        input.track_gain = Some(db(6.0));
        input.album_gain = Some(db(6.0));
        input.album_peak = Some(peak(1.0));
        input.track_peak = Some(peak(0.25));
        assert_eq!(
            decide(&input, Mode::Album, Lufs::REPLAYGAIN),
            decision(0.0, Source::TaggedAlbum, Clamp::Peak)
        );

        input.prev_same_album = true;
        input.next_same_album = false;
        assert_eq!(
            decide(&input, Mode::Auto, Lufs::REPLAYGAIN),
            decision(0.0, Source::TaggedAlbum, Clamp::Peak)
        );

        input.album_peak = None;
        assert_eq!(
            decide(&input, Mode::Album, Lufs::REPLAYGAIN),
            decision(6.0, Source::TaggedAlbum, Clamp::None)
        );
    }

    /// Track source clamps against the track peak, falling back to the album peak (MUS-088).
    #[test]
    fn track_source_prefers_the_track_peak_and_falls_back_to_the_album_peak() {
        let mut input = blank();
        input.track_gain = Some(db(6.0));
        input.track_peak = Some(peak(1.0));
        input.album_peak = Some(peak(0.25));
        assert_eq!(
            decide(&input, Mode::Track, Lufs::REPLAYGAIN),
            decision(0.0, Source::TaggedTrack, Clamp::Peak)
        );

        input.track_peak = None;
        assert_eq!(
            decide(&input, Mode::Track, Lufs::REPLAYGAIN),
            decision(6.0, Source::TaggedTrack, Clamp::None)
        );
    }

    /// Auto is the default loudness mode (MUS-087).
    #[test]
    fn default_mode_is_auto() {
        assert_eq!(Mode::default(), Mode::Auto);
    }

    /// LUFS targets are finite and from −70 to 0.
    #[test]
    fn lufs_accepts_its_range_and_refuses_the_rest() {
        assert_eq!(Lufs::new(Lufs::MIN), Ok(Lufs::new(-70.0).unwrap()));
        assert_eq!(Lufs::new(Lufs::MAX), Ok(Lufs::new(0.0).unwrap()));
        assert_eq!(Lufs::new(-18.0), Ok(Lufs::REPLAYGAIN));
        assert_eq!(Lufs::new(-23.0), Ok(Lufs::R128));
        assert_eq!(Scheme::ReplayGain.reference(), Lufs::REPLAYGAIN);
        assert_eq!(Scheme::R128.reference(), Lufs::R128);

        let above = f32::from_bits(0.0_f32.to_bits() + 1);
        let below = f32::from_bits((-70.0_f32).to_bits() + 1);
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, above, below] {
            assert_eq!(Lufs::new(value), Err(LufsError::Unusable), "{value}");
        }
    }

    /// Tag-gain clamps sit inside the [`GainDb`] range.
    #[test]
    fn tag_gain_bounds_are_inside_gain_db() {
        assert_eq!(GainDb::new(TAG_GAIN_MIN_DB), Ok(db(-30.0)));
        assert_eq!(GainDb::new(TAG_GAIN_MAX_DB), Ok(db(12.0)));
        assert_eq!(db(FALLBACK_DB), db(-5.0));
        // 20·log10(1/1.5) and 20·log10(1/16), the grid's peak-clamp literals.
        assert_eq!(GainDb::new(-3.521_825), Ok(db(-3.521_825)));
        assert_eq!(GainDb::new(-24.082_4), Ok(db(-24.082_4)));
    }

    /// Verifies: SEC-MED-015
    #[test]
    fn peak_of_sixteen_forces_enough_attenuation() {
        let mut input = blank();
        input.track_gain = Some(db(0.0));
        input.track_peak = Some(peak(16.0));
        assert_eq!(
            decide(&input, Mode::Track, Lufs::REPLAYGAIN),
            // 20·log10(1/16) ≈ −24.082399653118497
            decision(-24.082_4, Source::TaggedTrack, Clamp::Peak)
        );
    }

    fn optional_gain() -> impl Strategy<Value = Option<GainDb>> {
        prop_oneof![
            Just(None),
            (-128.0f32..=128.0).prop_map(|value| Some(db(value))),
        ]
    }

    fn optional_peak() -> impl Strategy<Value = Option<PeakRatio>> {
        prop_oneof![
            Just(None),
            (0.0f32..=16.0).prop_map(|ratio| Some(peak(ratio))),
        ]
    }

    fn any_mode() -> impl Strategy<Value = Mode> {
        prop_oneof![
            Just(Mode::Auto),
            Just(Mode::Track),
            Just(Mode::Album),
            Just(Mode::Off),
        ]
    }

    fn any_scheme() -> impl Strategy<Value = Scheme> {
        prop_oneof![Just(Scheme::ReplayGain), Just(Scheme::R128)]
    }

    fn any_target() -> impl Strategy<Value = Lufs> {
        (-70.0f32..=0.0).prop_map(|value| Lufs::new(value).unwrap())
    }

    fn any_measured() -> impl Strategy<Value = Option<Lufs>> {
        prop_oneof![
            Just(None),
            (-70.0f32..=0.0).prop_map(|value| Some(Lufs::new(value).unwrap())),
        ]
    }

    proptest! {
        /// Verifies: SEC-MED-015
        #[test]
        fn applied_gain_never_takes_the_recorded_peak_above_full_scale(
            track_gain in optional_gain(),
            album_gain in optional_gain(),
            track_peak in optional_peak(),
            album_peak in optional_peak(),
            scheme in any_scheme(),
            opus_header in optional_gain(),
            measured in any_measured(),
            prev_same_album in any::<bool>(),
            next_same_album in any::<bool>(),
            mode in any_mode(),
            target in any_target(),
        ) {
            let input = GainInput {
                track_gain,
                album_gain,
                track_peak,
                album_peak,
                scheme,
                opus_header,
                measured,
                prev_same_album,
                next_same_album,
            };
            let decided = decide(&input, mode, target);
            let applied = decided.applied.db();
            prop_assert!((TAG_GAIN_MIN_DB..=TAG_GAIN_MAX_DB).contains(&applied));
            prop_assert!(applied.is_finite());
            let recorded = match decided.source {
                Source::TaggedAlbum => album_peak.or(track_peak),
                Source::TaggedTrack | Source::Measured | Source::Estimated | Source::Off => {
                    track_peak.or(album_peak)
                }
            };
            if recorded.is_none_or(|peak| peak.ratio() == 0.0) {
                prop_assert!(applied <= 0.0);
            }
            if let Some(peak) = recorded.filter(|peak| peak.ratio() > 0.0) {
                let ratio = peak.ratio();
                let headroom = headroom_db(ratio);
                // One ulp of the dB headroom: the 20·log10 round-trip can
                // land 1 ulp above the f32 value, but 1e-4 dB of slack would
                // still accept a peak slightly over full scale.
                prop_assert!(
                    applied <= headroom.next_up(),
                    "applied {applied} exceeds headroom {headroom} of peak {ratio}"
                );
            }
        }
    }
}
