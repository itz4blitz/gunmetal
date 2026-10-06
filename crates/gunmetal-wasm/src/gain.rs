//! Playback gain: the core's decision and the linear factor the browser sets
//! (MUS-084, MUS-085, MUS-087 to MUS-089; SEC-MED-015).
//!
//! [`gain_decide`] is one call of [`gain::decide`] with its types converted.
//! The answer carries the applied gain in dB, where it came from, which
//! clamp ran, and the linear factor a Web Audio gain node takes, so the
//! client never computes one. The browser calls it as `gainDecide`.

use gunmetal_core::gain;
use gunmetal_core::values::{GainDb, PeakRatio, ValueError};
use serde::{Deserialize, Serialize};
use tsify::Tsify;

/// How the player chooses between track and album gain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub enum Mode {
    /// Album gain inside a From-lane album run, track gain otherwise.
    Auto,
    /// Track gain whenever it exists.
    Track,
    /// Album gain whenever it exists, even for a single shuffled track.
    Album,
    /// No levelling. An Opus header gain still applies.
    Off,
}

/// Which scheme the file's tags were written against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub enum Scheme {
    /// `REPLAYGAIN_*` tags, relative to −18 LUFS.
    ReplayGain,
    /// `R128_*_GAIN` tags, relative to −23 LUFS.
    R128,
}

/// Where the chosen gain came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub enum Source {
    /// A `ReplayGain` or R128 track tag.
    TaggedTrack,
    /// A `ReplayGain` or R128 album tag.
    TaggedAlbum,
    /// Scan-time integrated loudness.
    Measured,
    /// The fixed fallback.
    Estimated,
    /// Levelling is off; only a codec header gain, if any, applies.
    Off,
}

/// Which safety clamp changed the gain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub enum Clamp {
    /// The computed gain was already inside the safe range and peak limit.
    None,
    /// The gain was held to −30 dB to +12 dB.
    Range,
    /// A boost was refused because no valid peak was known.
    NoPeak,
    /// The gain was reduced so the recorded peak stays at or below full scale.
    Peak,
}

/// Everything the decision needs about one track. Gains are dB; peaks are a
/// ratio of full scale; measured loudness is LUFS. A finite value outside
/// the core's range, or a non-finite one, is [`GainOutcome::Unusable`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Tsify)]
pub struct GainInput {
    /// `ReplayGain` or R128 track gain, in dB.
    pub track_gain: Option<f32>,
    /// `ReplayGain` or R128 album gain, in dB.
    pub album_gain: Option<f32>,
    /// Track peak as a ratio of full scale.
    pub track_peak: Option<f32>,
    /// Album peak as a ratio of full scale.
    pub album_peak: Option<f32>,
    /// Which reference the tags were written against.
    pub scheme: Scheme,
    /// Opus identification-header output gain. Always added.
    pub opus_header: Option<f32>,
    /// Integrated loudness from a measurement pass, when tags are missing.
    pub measured: Option<f32>,
    /// The previous From-lane item is the same album.
    pub prev_same_album: bool,
    /// The next From-lane item is the same album.
    pub next_same_album: bool,
}

/// The gain the player applies, where it came from, which clamp ran, and
/// the linear factor to set on a gain node.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Tsify)]
pub struct GainDecision {
    /// The gain to apply, after every clamp, in dB.
    pub applied_db: f32,
    /// Ten to the power of `applied_db` over twenty: the Web Audio factor.
    pub factor: f32,
    /// Where the pre-clamp gain came from.
    pub source: Source,
    /// The last clamp that changed the value, or `None`.
    pub clamp: Clamp,
}

/// What reading a gain request gave.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Tsify)]
pub enum GainOutcome {
    /// The decision the core made.
    Decision(GainDecision),
    /// A number was not finite, or lay outside the core's range.
    Unusable,
}

/// The TypeScript declarations of this module's mirror types, in the order
/// they are declared above.
pub const DECLARATIONS: [&str; 7] = [
    Mode::DECL,
    Scheme::DECL,
    Source::DECL,
    Clamp::DECL,
    GainInput::DECL,
    GainDecision::DECL,
    GainOutcome::DECL,
];

impl From<gain::Mode> for Mode {
    fn from(value: gain::Mode) -> Self {
        match value {
            gain::Mode::Auto => Self::Auto,
            gain::Mode::Track => Self::Track,
            gain::Mode::Album => Self::Album,
            gain::Mode::Off => Self::Off,
        }
    }
}

impl From<Mode> for gain::Mode {
    fn from(value: Mode) -> Self {
        match value {
            Mode::Auto => Self::Auto,
            Mode::Track => Self::Track,
            Mode::Album => Self::Album,
            Mode::Off => Self::Off,
        }
    }
}

impl From<gain::Scheme> for Scheme {
    fn from(value: gain::Scheme) -> Self {
        match value {
            gain::Scheme::ReplayGain => Self::ReplayGain,
            gain::Scheme::R128 => Self::R128,
        }
    }
}

impl From<Scheme> for gain::Scheme {
    fn from(value: Scheme) -> Self {
        match value {
            Scheme::ReplayGain => Self::ReplayGain,
            Scheme::R128 => Self::R128,
        }
    }
}

impl From<gain::Source> for Source {
    fn from(value: gain::Source) -> Self {
        match value {
            gain::Source::TaggedTrack => Self::TaggedTrack,
            gain::Source::TaggedAlbum => Self::TaggedAlbum,
            gain::Source::Measured => Self::Measured,
            gain::Source::Estimated => Self::Estimated,
            gain::Source::Off => Self::Off,
        }
    }
}

impl From<gain::Clamp> for Clamp {
    fn from(value: gain::Clamp) -> Self {
        match value {
            gain::Clamp::None => Self::None,
            gain::Clamp::Range => Self::Range,
            gain::Clamp::NoPeak => Self::NoPeak,
            gain::Clamp::Peak => Self::Peak,
        }
    }
}

impl From<gain::GainDecision> for GainDecision {
    fn from(value: gain::GainDecision) -> Self {
        let gain::GainDecision {
            applied,
            source,
            clamp,
        } = value;
        let applied_db = applied.db();
        Self {
            applied_db,
            factor: factor(applied_db),
            source: source.into(),
            clamp: clamp.into(),
        }
    }
}

fn factor(applied_db: f32) -> f32 {
    10_f32.powf(applied_db / 20.0)
}

fn db(value: Option<f32>) -> Result<Option<GainDb>, ValueError> {
    value.map(GainDb::new).transpose()
}

fn peak(value: Option<f32>) -> Result<Option<PeakRatio>, ValueError> {
    value.map(PeakRatio::new).transpose()
}

fn loudness(value: Option<f32>) -> Result<Option<gain::Lufs>, gain::LufsError> {
    value.map(gain::Lufs::new).transpose()
}

fn core_input(input: GainInput) -> Result<gain::GainInput, GainOutcome> {
    let GainInput {
        track_gain,
        album_gain,
        track_peak,
        album_peak,
        scheme,
        opus_header,
        measured,
        prev_same_album,
        next_same_album,
    } = input;
    let track_gain = db(track_gain).map_err(|_| GainOutcome::Unusable)?;
    let album_gain = db(album_gain).map_err(|_| GainOutcome::Unusable)?;
    let track_peak = peak(track_peak).map_err(|_| GainOutcome::Unusable)?;
    let album_peak = peak(album_peak).map_err(|_| GainOutcome::Unusable)?;
    let opus_header = db(opus_header).map_err(|_| GainOutcome::Unusable)?;
    let measured = loudness(measured).map_err(|_| GainOutcome::Unusable)?;
    Ok(gain::GainInput {
        track_gain,
        album_gain,
        track_peak,
        album_peak,
        scheme: scheme.into(),
        opus_header,
        measured,
        prev_same_album,
        next_same_album,
    })
}

/// The gain to apply for `input` under `mode` at `target` LUFS.
///
/// Verifies: SEC-CLI-021
#[must_use]
pub fn gain_decide(target: f32, input: GainInput, mode: Mode) -> GainOutcome {
    let Ok(target) = gain::Lufs::new(target) else {
        return GainOutcome::Unusable;
    };
    let Ok(input) = core_input(input) else {
        return GainOutcome::Unusable;
    };
    GainOutcome::Decision(gain::decide(&input, mode.into(), target).into())
}

crate::export::export! {
    /// The browser's `gainDecide`: [`gain_decide`], with its types converted.
    ///
    /// `target` is LUFS. A non-finite value, or one outside −70 to 0, comes
    /// back as `Unusable`.
    "gainDecide": fn gain_decide_export = gain_decide(target: f32; input: GainInput, mode: Mode) -> GainOutcome
}

#[cfg(test)]
mod tests {
    use gunmetal_core::gain;
    use gunmetal_core::values::{GainDb, PeakRatio};

    use super::{
        Clamp, DECLARATIONS, GainDecision, GainInput, GainOutcome, Mode, Scheme, Source, factor,
        gain_decide,
    };

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

    fn as_decision(outcome: GainOutcome) -> Option<GainDecision> {
        match outcome {
            GainOutcome::Decision(decision) => Some(decision),
            GainOutcome::Unusable => None,
        }
    }

    #[test]
    fn each_mode_converts_to_its_mirror_and_back() {
        let pairs = [
            (gain::Mode::Auto, Mode::Auto),
            (gain::Mode::Track, Mode::Track),
            (gain::Mode::Album, Mode::Album),
            (gain::Mode::Off, Mode::Off),
        ];
        for (core, mirrored) in pairs {
            assert_eq!(Mode::from(core), mirrored);
            assert_eq!(gain::Mode::from(mirrored), core);
        }
    }

    #[test]
    fn each_scheme_converts_to_its_mirror_and_back() {
        assert_eq!(Scheme::from(gain::Scheme::ReplayGain), Scheme::ReplayGain);
        assert_eq!(
            gain::Scheme::from(Scheme::ReplayGain),
            gain::Scheme::ReplayGain
        );
        assert_eq!(Scheme::from(gain::Scheme::R128), Scheme::R128);
        assert_eq!(gain::Scheme::from(Scheme::R128), gain::Scheme::R128);
    }

    /// Levelling off with no header is 0 dB and factor 1, matching the core.
    ///
    /// Verifies: SEC-MED-015
    #[test]
    fn levelling_off_with_no_header_is_unity() {
        let decision = as_decision(gain_decide(-18.0, blank(), Mode::Off)).unwrap();
        assert_eq!(decision.applied_db.to_bits(), 0.0_f32.to_bits());
        assert_eq!(decision.factor.to_bits(), 1.0_f32.to_bits());
        assert_eq!(decision.source, Source::Off);
        assert_eq!(decision.clamp, Clamp::None);
        let core = gain::decide(
            &gain::GainInput {
                track_gain: None,
                album_gain: None,
                track_peak: None,
                album_peak: None,
                scheme: gain::Scheme::ReplayGain,
                opus_header: None,
                measured: None,
                prev_same_album: false,
                next_same_album: false,
            },
            gain::Mode::Off,
            gain::Lufs::REPLAYGAIN,
        );
        assert_eq!(core.applied.db().to_bits(), 0.0_f32.to_bits());
        assert_eq!(core.source, gain::Source::Off);
        assert_eq!(core.clamp, gain::Clamp::None);
    }

    /// A tagged track gain of −6.5 dB at the `ReplayGain` target, with a peak
    /// that allows it, is applied as-is, and the factor is ten to that over
    /// twenty.
    ///
    /// Verifies: SEC-MED-015
    #[test]
    fn a_tagged_track_gain_becomes_the_applied_db_and_its_factor() {
        let mut input = blank();
        input.track_gain = Some(-6.5);
        input.track_peak = Some(0.5);
        let decision = as_decision(gain_decide(-18.0, input, Mode::Track)).unwrap();
        assert_eq!(decision.applied_db.to_bits(), (-6.5_f32).to_bits());
        assert_eq!(decision.factor.to_bits(), factor(-6.5).to_bits());
        assert_eq!(decision.source, Source::TaggedTrack);
        assert_eq!(decision.clamp, Clamp::None);
        let core = gain::decide(
            &gain::GainInput {
                track_gain: Some(GainDb::new(-6.5).expect("in range")),
                album_gain: None,
                track_peak: Some(PeakRatio::new(0.5).expect("in range")),
                album_peak: None,
                scheme: gain::Scheme::ReplayGain,
                opus_header: None,
                measured: None,
                prev_same_album: false,
                next_same_album: false,
            },
            gain::Mode::Track,
            gain::Lufs::REPLAYGAIN,
        );
        assert_eq!(core.applied.db().to_bits(), (-6.5_f32).to_bits());
        assert_eq!(core.source, gain::Source::TaggedTrack);
        assert_eq!(core.clamp, gain::Clamp::None);
    }

    /// A number the core refuses is a typed conversion error, not a guess.
    ///
    /// Verifies: SEC-CLI-021
    #[test]
    fn a_non_finite_or_out_of_range_number_is_unusable() {
        let mut input = blank();
        input.track_gain = Some(f32::NAN);
        assert_eq!(
            gain_decide(-18.0, input, Mode::Track),
            GainOutcome::Unusable
        );
        input = blank();
        input.track_peak = Some(-0.1);
        assert_eq!(
            gain_decide(-18.0, input, Mode::Track),
            GainOutcome::Unusable
        );
        input = blank();
        input.measured = Some(1.0);
        assert_eq!(
            gain_decide(-18.0, input, Mode::Track),
            GainOutcome::Unusable
        );
        assert_eq!(
            gain_decide(f32::INFINITY, blank(), Mode::Track),
            GainOutcome::Unusable
        );
        assert_eq!(as_decision(gain_decide(1.0, blank(), Mode::Track)), None);
    }

    #[test]
    fn every_source_and_clamp_converts() {
        let sources = [
            (gain::Source::TaggedTrack, Source::TaggedTrack),
            (gain::Source::TaggedAlbum, Source::TaggedAlbum),
            (gain::Source::Measured, Source::Measured),
            (gain::Source::Estimated, Source::Estimated),
            (gain::Source::Off, Source::Off),
        ];
        for (core, mirrored) in sources {
            assert_eq!(Source::from(core), mirrored);
        }
        let clamps = [
            (gain::Clamp::None, Clamp::None),
            (gain::Clamp::Range, Clamp::Range),
            (gain::Clamp::NoPeak, Clamp::NoPeak),
            (gain::Clamp::Peak, Clamp::Peak),
        ];
        for (core, mirrored) in clamps {
            assert_eq!(Clamp::from(core), mirrored);
        }
    }

    #[test]
    fn an_unusable_album_gain_header_or_peak_is_unusable() {
        let mut input = blank();
        input.album_gain = Some(f32::INFINITY);
        assert_eq!(
            gain_decide(-18.0, input, Mode::Album),
            GainOutcome::Unusable
        );
        input = blank();
        input.album_peak = Some(-1.0);
        assert_eq!(
            gain_decide(-18.0, input, Mode::Album),
            GainOutcome::Unusable
        );
        input = blank();
        input.opus_header = Some(f32::NAN);
        assert_eq!(gain_decide(-18.0, input, Mode::Off), GainOutcome::Unusable);
        input = blank();
        input.track_gain = Some(129.0);
        assert_eq!(
            gain_decide(-18.0, input, Mode::Track),
            GainOutcome::Unusable
        );
    }

    /// A tagged album gain, a measured loudness, an Opus header and R128
    /// convert through the same path the core uses, including every source
    /// and clamp the decision can name.
    ///
    /// Verifies: SEC-MED-015
    #[test]
    fn album_measured_header_and_clamps_match_the_core() {
        let mut album = blank();
        album.album_gain = Some(-4.0);
        album.album_peak = Some(0.8);
        album.prev_same_album = true;
        album.next_same_album = true;
        let decision = as_decision(gain_decide(-18.0, album, Mode::Album)).unwrap();
        assert_eq!(decision.applied_db.to_bits(), (-4.0_f32).to_bits());
        assert_eq!(decision.source, Source::TaggedAlbum);
        assert_eq!(decision.clamp, Clamp::None);

        let mut measured = blank();
        measured.measured = Some(-23.0);
        let decision = as_decision(gain_decide(-18.0, measured, Mode::Track)).unwrap();
        assert_eq!(decision.source, Source::Measured);

        let mut header = blank();
        header.opus_header = Some(-3.0);
        let decision = as_decision(gain_decide(-18.0, header, Mode::Off)).unwrap();
        assert_eq!(decision.applied_db.to_bits(), (-3.0_f32).to_bits());
        assert_eq!(decision.source, Source::Off);

        let mut r128 = blank();
        r128.scheme = Scheme::R128;
        r128.track_gain = Some(-1.0);
        r128.track_peak = Some(0.5);
        let decision = as_decision(gain_decide(-23.0, r128, Mode::Track)).unwrap();
        assert_eq!(decision.source, Source::TaggedTrack);

        let mut range = blank();
        range.track_gain = Some(-40.0);
        range.track_peak = Some(0.5);
        let decision = as_decision(gain_decide(-18.0, range, Mode::Track)).unwrap();
        assert_eq!(decision.clamp, Clamp::Range);
        assert_eq!(decision.applied_db.to_bits(), (-30.0_f32).to_bits());

        let mut no_peak = blank();
        no_peak.track_gain = Some(6.0);
        let decision = as_decision(gain_decide(-18.0, no_peak, Mode::Track)).unwrap();
        assert_eq!(decision.clamp, Clamp::NoPeak);

        let mut peak = blank();
        peak.track_gain = Some(6.0);
        peak.track_peak = Some(1.0);
        let decision = as_decision(gain_decide(-18.0, peak, Mode::Track)).unwrap();
        assert_eq!(decision.clamp, Clamp::Peak);

        let decision = as_decision(gain_decide(-18.0, blank(), Mode::Auto)).unwrap();
        assert_eq!(decision.source, Source::Estimated);
    }

    #[test]
    fn the_declarations_name_every_mirror() {
        assert_eq!(DECLARATIONS.len(), 7);
        assert!(DECLARATIONS[0].contains("export type Mode"));
        assert!(DECLARATIONS[4].contains("export interface GainInput"));
        assert!(DECLARATIONS[5].contains("factor"));
        assert!(DECLARATIONS[6].contains("export type GainOutcome"));
    }
}
