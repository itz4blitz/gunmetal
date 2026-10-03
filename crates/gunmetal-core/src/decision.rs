//! Playback decision for music: play the original, play through the
//! packager, or cannot play here, plus the track-details summary
//! (MUS-099, MUS-229, MUS-236).
//!
//! [`decide_audio`] is a pure function of a track's technical facts and
//! this device's capability report. R1 has no transcoder, so a codec the
//! device cannot decode is a structured refusal, never a silent failure.
//! The audio packager (MUS-230) copies only FLAC, MP3 and Opus frames
//! into fragmented MP4; it is chosen when the device can decode the
//! codec but cannot play the original container, or can decode it only
//! through Media Source Extensions.
//!
//! [`track_details`] builds the R1 details summary from the synced track
//! record, this decision and WP-028's gain decision. It holds no file
//! path.

use crate::catalog::{AlbumId, ArtistId, Codec, Container, TechInfo, TrackRecord, Trim};
use crate::gain::{GainDecision, Source};

/// How far a device can play one codec or one container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PlayCap {
    /// The device cannot play it.
    #[default]
    None,
    /// Only through Media Source Extensions.
    MseOnly,
    /// The original file plays in the device's native decoder.
    Native,
}

/// The container the audio packager writes (MUS-230).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PackageFormat {
    /// Fragmented MP4 audio.
    FragmentedMp4,
}

/// Why this device cannot play a track (MUS-229).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reason {
    /// The device has no decoder for this codec.
    CannotDecode {
        /// The codec the device cannot decode.
        codec: Codec,
    },
    /// The device can decode the codec but cannot open this container,
    /// and the packager does not carry the codec.
    UnsupportedContainer {
        /// The codec the device can decode.
        codec: Codec,
        /// The container it cannot open.
        container: Container,
    },
}

impl Reason {
    /// The badge's words for this reason.
    #[must_use]
    pub fn words(self) -> String {
        match self {
            Self::CannotDecode { codec } => format!(
                "Cannot play here: this browser cannot decode {}",
                codec_words(codec)
            ),
            Self::UnsupportedContainer { codec, container } => format!(
                "Cannot play here: this browser cannot open {} for {}",
                container_words(container),
                codec_words(codec)
            ),
        }
    }
}

/// What to do with this track on this device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Play the original file.
    Direct,
    /// Copy frames into this format without re-encoding (MUS-230).
    Packaged(PackageFormat),
    /// Do not play; the reasons are what the badge and the skip notice
    /// show.
    CannotPlay(Vec<Reason>),
}

impl Decision {
    /// The structured reasons, empty when the track can play.
    #[must_use]
    pub fn reasons(&self) -> &[Reason] {
        match self {
            Self::CannotPlay(reasons) => reasons,
            Self::Direct | Self::Packaged(_) => &[],
        }
    }
}

/// What this device reports it can play.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeviceCaps {
    codecs: Vec<(Codec, PlayCap)>,
    containers: Vec<(Container, PlayCap)>,
    mse_fragmented_mp4: bool,
}

impl DeviceCaps {
    /// A report that supports nothing.
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// Records how far this device can play `codec`. A later call for
    /// the same codec replaces the previous one.
    #[must_use]
    pub fn with_codec(mut self, codec: Codec, cap: PlayCap) -> Self {
        if let Some((_, existing)) = self.codecs.iter_mut().find(|(item, _)| *item == codec) {
            *existing = cap;
        } else {
            self.codecs.push((codec, cap));
        }
        self
    }

    /// Records how far this device can play `container`. A later call
    /// for the same container replaces the previous one.
    #[must_use]
    pub fn with_container(mut self, container: Container, cap: PlayCap) -> Self {
        if let Some((_, existing)) = self
            .containers
            .iter_mut()
            .find(|(item, _)| *item == container)
        {
            *existing = cap;
        } else {
            self.containers.push((container, cap));
        }
        self
    }

    /// Records whether Media Source Extensions accept `format`.
    #[must_use]
    pub fn with_package(mut self, format: PackageFormat, accepted: bool) -> Self {
        match format {
            PackageFormat::FragmentedMp4 => self.mse_fragmented_mp4 = accepted,
        }
        self
    }

    /// How far this device can play `codec`.
    #[must_use]
    pub fn codec(&self, codec: Codec) -> PlayCap {
        self.codecs
            .iter()
            .find(|(item, _)| *item == codec)
            .map_or(PlayCap::None, |(_, cap)| *cap)
    }

    /// How far this device can play `container`.
    #[must_use]
    pub fn container(&self, container: Container) -> PlayCap {
        self.containers
            .iter()
            .find(|(item, _)| *item == container)
            .map_or(PlayCap::None, |(_, cap)| *cap)
    }

    /// Whether Media Source Extensions accept `format`.
    #[must_use]
    pub fn accepts_package(&self, format: PackageFormat) -> bool {
        match format {
            PackageFormat::FragmentedMp4 => self.mse_fragmented_mp4,
        }
    }
}

/// The container the packager would write for `codec`, when it carries
/// that codec.
#[must_use]
pub const fn package_format_for(codec: Codec) -> Option<PackageFormat> {
    match codec {
        Codec::Flac | Codec::Mp3 | Codec::Opus => Some(PackageFormat::FragmentedMp4),
        Codec::Aac | Codec::Alac | Codec::Pcm | Codec::Vorbis => None,
    }
}

/// The playback decision for `tech` on a device that reported `device`.
#[must_use]
pub fn decide_audio(tech: &TechInfo, device: &DeviceCaps) -> Decision {
    let codec = tech.codec();
    let container = tech.container();
    let codec_cap = device.codec(codec);
    let container_cap = device.container(container);

    if codec_cap == PlayCap::None {
        return Decision::CannotPlay(vec![Reason::CannotDecode { codec }]);
    }
    if codec_cap == PlayCap::Native && container_cap == PlayCap::Native {
        return Decision::Direct;
    }
    if package_format_for(codec).is_some() && device.accepts_package(PackageFormat::FragmentedMp4) {
        return Decision::Packaged(PackageFormat::FragmentedMp4);
    }
    if container_cap == PlayCap::None {
        return Decision::CannotPlay(vec![Reason::UnsupportedContainer { codec, container }]);
    }
    Decision::Direct
}

/// The R1 track-details summary (MUS-236).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackDetails {
    /// The title as tagged.
    pub title: String,
    /// The artist credit as tagged.
    pub credit: String,
    /// The credited artists, in credit order.
    pub artists: Vec<ArtistId>,
    /// The album, when the track is grouped into one.
    pub album: Option<AlbumId>,
    /// Codec, container and technical facts.
    pub format: TechInfo,
    /// The playback decision for this device.
    pub decision: Decision,
    /// That decision in the quality badge's words (MUS-099, MUS-229).
    pub badge: String,
    /// Whether the track joins the next one without a gap: the trim is
    /// known and the chosen path keeps it (MUS-067, MUS-069).
    pub gapless: bool,
    /// Where the gain came from (MUS-084, MUS-089).
    pub gain_source: Source,
}

impl TrackDetails {
    /// Every field this summary holds, in the order the view shows them.
    /// There is no path.
    pub const FIELDS: [&'static str; 9] = [
        "title",
        "credit",
        "artists",
        "album",
        "format",
        "decision",
        "badge",
        "gapless",
        "gain_source",
    ];
}

/// The details summary for synced track `track` under `decision` and
/// `gain`.
#[must_use]
pub fn track_details(
    track: &TrackRecord,
    decision: &Decision,
    gain: &GainDecision,
) -> TrackDetails {
    TrackDetails {
        title: track.title.clone(),
        credit: track.artist_credit.clone(),
        artists: track.artists.clone(),
        album: track.album,
        format: track.tech,
        decision: decision.clone(),
        badge: badge_words(&track.tech, decision),
        gapless: joins_without_a_gap(track.trim, decision),
        gain_source: gain.source,
    }
}

fn joins_without_a_gap(trim: Option<Trim>, decision: &Decision) -> bool {
    trim.is_some() && matches!(decision, Decision::Packaged(_))
}

fn badge_words(tech: &TechInfo, decision: &Decision) -> String {
    match decision {
        Decision::CannotPlay(reasons) => reasons
            .iter()
            .next()
            .map_or_else(|| "Cannot play here".to_owned(), |reason| reason.words()),
        Decision::Direct => original_badge(tech, "played directly"),
        Decision::Packaged(_) => original_badge(tech, "played through the packager"),
    }
}

fn original_badge(tech: &TechInfo, path: &str) -> String {
    let mut parts = Vec::new();
    parts.push(format!("Original {}", codec_words(tech.codec())));
    if let Some(depth) = tech.format().bit_depth {
        parts.push(format!("{}-bit", depth.get()));
    }
    if let Some(rate) = tech.format().sample_rate {
        parts.push(sample_rate_words(rate));
    }
    parts.push(path.to_owned());
    parts.join(", ")
}

fn sample_rate_words(rate: crate::values::SampleRate) -> String {
    let hz = rate.hz().get();
    match (
        hz.checked_div(1_000),
        hz.checked_rem(1_000),
        hz.checked_rem(100),
    ) {
        (Some(khz), Some(0), _) => format!("{khz} kHz"),
        (Some(khz), Some(remainder), Some(0)) => {
            format!("{khz}.{} kHz", remainder.saturating_div(100))
        }
        _ => format!("{hz} Hz"),
    }
}

fn codec_words(codec: Codec) -> &'static str {
    match codec {
        Codec::Aac => "AAC",
        Codec::Alac => "ALAC",
        Codec::Flac => "FLAC",
        Codec::Mp3 => "MP3",
        Codec::Opus => "Opus",
        Codec::Pcm => "PCM",
        Codec::Vorbis => "Vorbis",
    }
}

fn container_words(container: Container) -> &'static str {
    match container {
        Container::Aiff => "AIFF",
        Container::Flac => "FLAC",
        Container::Mp4 => "MP4",
        Container::Mpeg => "MPEG",
        Container::Ogg => "Ogg",
        Container::Wav => "WAV",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::catalog::{
        AudioFormat, Availability, GainTags, ItemKind, LibraryId, TrackId, TrackPosition,
    };
    use crate::gain::Clamp;
    use crate::id::{IdKind, PublicId};
    use crate::time::Timestamp;
    use crate::values::{BitDepth, Channels, Duration, GainDb, SampleRate};

    /// Every R1 core format with the container files of that kind use.
    const CORES: [(Codec, Container); 7] = [
        (Codec::Aac, Container::Mp4),
        (Codec::Alac, Container::Mp4),
        (Codec::Flac, Container::Flac),
        (Codec::Mp3, Container::Mpeg),
        (Codec::Opus, Container::Ogg),
        (Codec::Pcm, Container::Wav),
        (Codec::Vorbis, Container::Ogg),
    ];

    fn tech(codec: Codec, container: Container) -> TechInfo {
        TechInfo::new(codec, container, AudioFormat::default()).unwrap()
    }

    fn flac_24_96() -> TechInfo {
        TechInfo::new(
            Codec::Flac,
            Container::Flac,
            AudioFormat {
                sample_rate: Some(SampleRate::new(96_000).unwrap()),
                bit_depth: Some(BitDepth::new(24).unwrap()),
                channels: Some(Channels::new(2).unwrap()),
                bitrate: None,
                duration: Some(Duration::from_millis(215_000).unwrap()),
            },
        )
        .unwrap()
    }

    fn mp3_44k() -> TechInfo {
        TechInfo::new(
            Codec::Mp3,
            Container::Mpeg,
            AudioFormat {
                sample_rate: Some(SampleRate::new(44_100).unwrap()),
                bit_depth: None,
                channels: Some(Channels::new(2).unwrap()),
                bitrate: None,
                duration: None,
            },
        )
        .unwrap()
    }

    fn opus_48k() -> TechInfo {
        TechInfo::new(
            Codec::Opus,
            Container::Ogg,
            AudioFormat {
                sample_rate: Some(SampleRate::new(48_000).unwrap()),
                bit_depth: None,
                channels: Some(Channels::new(2).unwrap()),
                bitrate: None,
                duration: None,
            },
        )
        .unwrap()
    }

    fn alac_16_44() -> TechInfo {
        TechInfo::new(
            Codec::Alac,
            Container::Mp4,
            AudioFormat {
                sample_rate: Some(SampleRate::new(44_100).unwrap()),
                bit_depth: Some(BitDepth::new(16).unwrap()),
                channels: Some(Channels::new(2).unwrap()),
                bitrate: None,
                duration: None,
            },
        )
        .unwrap()
    }

    fn supports(codec: Codec, container: Container) -> DeviceCaps {
        DeviceCaps::none()
            .with_codec(codec, PlayCap::Native)
            .with_container(container, PlayCap::Native)
            .with_package(PackageFormat::FragmentedMp4, true)
    }

    fn codec_only(codec: Codec) -> DeviceCaps {
        DeviceCaps::none()
            .with_codec(codec, PlayCap::Native)
            .with_package(PackageFormat::FragmentedMp4, true)
    }

    fn mse_only(codec: Codec, container: Container) -> DeviceCaps {
        DeviceCaps::none()
            .with_codec(codec, PlayCap::MseOnly)
            .with_container(container, PlayCap::MseOnly)
            .with_package(PackageFormat::FragmentedMp4, true)
    }

    fn public(kind: IdKind, prefix: &str, symbols: &str) -> PublicId {
        PublicId::parse(&format!("{prefix}{symbols}"), kind).unwrap()
    }

    const SYMBOLS: &str = "0123456789abcdefghjkmnpqrs";
    const OTHER_SYMBOLS: &str = "7zzzzzzzzzzzzzzzzzzzzzzzzz";

    fn track_id() -> TrackId {
        TrackId::new(public(IdKind::Track, "trk_", SYMBOLS)).unwrap()
    }

    fn album_id() -> AlbumId {
        AlbumId::new(public(IdKind::Album, "alb_", SYMBOLS)).unwrap()
    }

    fn artist_id(symbols: &str) -> ArtistId {
        ArtistId::new(public(IdKind::Artist, "art_", symbols)).unwrap()
    }

    fn library_id() -> LibraryId {
        LibraryId::new(public(IdKind::Library, "lib_", SYMBOLS)).unwrap()
    }

    fn record(title: &str, credit: &str, tech_info: TechInfo, trim: Option<Trim>) -> TrackRecord {
        TrackRecord {
            id: track_id(),
            kind: ItemKind::Track,
            library: library_id(),
            title: title.to_owned(),
            title_sort: Some("sorted-title-not-the-title".to_owned()),
            artist_credit: credit.to_owned(),
            artists: vec![artist_id(SYMBOLS)],
            album: Some(album_id()),
            position: TrackPosition::default(),
            disc_subtitle: None,
            date: None,
            original_date: None,
            genres: vec!["not-a-details-field".to_owned()],
            moods: Vec::new(),
            styles: Vec::new(),
            labels: Vec::new(),
            grouping: Vec::new(),
            advisory: None,
            isrc: Vec::new(),
            recording_mbid: None,
            tech: tech_info,
            gain: GainTags::default(),
            trim,
            lyrics: None,
            availability: Availability::Playable,
            added: Timestamp::from_millis(0).unwrap(),
        }
    }

    fn gain(source: Source) -> GainDecision {
        GainDecision {
            applied: GainDb::new(-5.0).unwrap(),
            source,
            clamp: Clamp::None,
        }
    }

    #[test]
    fn each_core_format_plays_directly_when_the_device_supports_it() {
        let got: Vec<(Codec, Decision)> = CORES
            .iter()
            .map(|(codec, container)| {
                (
                    *codec,
                    decide_audio(&tech(*codec, *container), &supports(*codec, *container)),
                )
            })
            .collect();
        assert_eq!(
            got,
            [
                (Codec::Aac, Decision::Direct),
                (Codec::Alac, Decision::Direct),
                (Codec::Flac, Decision::Direct),
                (Codec::Mp3, Decision::Direct),
                (Codec::Opus, Decision::Direct),
                (Codec::Pcm, Decision::Direct),
                (Codec::Vorbis, Decision::Direct),
            ]
        );
    }

    #[test]
    fn each_core_format_is_refused_when_the_device_lacks_the_codec() {
        let got: Vec<(Codec, Decision)> = CORES
            .iter()
            .map(|(codec, container)| {
                (
                    *codec,
                    decide_audio(&tech(*codec, *container), &DeviceCaps::none()),
                )
            })
            .collect();
        assert_eq!(
            got,
            [
                (
                    Codec::Aac,
                    Decision::CannotPlay(vec![Reason::CannotDecode { codec: Codec::Aac }])
                ),
                (
                    Codec::Alac,
                    Decision::CannotPlay(vec![Reason::CannotDecode { codec: Codec::Alac }])
                ),
                (
                    Codec::Flac,
                    Decision::CannotPlay(vec![Reason::CannotDecode { codec: Codec::Flac }])
                ),
                (
                    Codec::Mp3,
                    Decision::CannotPlay(vec![Reason::CannotDecode { codec: Codec::Mp3 }])
                ),
                (
                    Codec::Opus,
                    Decision::CannotPlay(vec![Reason::CannotDecode { codec: Codec::Opus }])
                ),
                (
                    Codec::Pcm,
                    Decision::CannotPlay(vec![Reason::CannotDecode { codec: Codec::Pcm }])
                ),
                (
                    Codec::Vorbis,
                    Decision::CannotPlay(vec![Reason::CannotDecode {
                        codec: Codec::Vorbis
                    }])
                ),
            ]
        );
    }

    #[test]
    fn each_core_format_packages_or_refuses_when_the_container_is_missing() {
        let got: Vec<(Codec, Decision)> = CORES
            .iter()
            .map(|(codec, container)| {
                (
                    *codec,
                    decide_audio(&tech(*codec, *container), &codec_only(*codec)),
                )
            })
            .collect();
        assert_eq!(
            got,
            [
                (
                    Codec::Aac,
                    Decision::CannotPlay(vec![Reason::UnsupportedContainer {
                        codec: Codec::Aac,
                        container: Container::Mp4,
                    }])
                ),
                (
                    Codec::Alac,
                    Decision::CannotPlay(vec![Reason::UnsupportedContainer {
                        codec: Codec::Alac,
                        container: Container::Mp4,
                    }])
                ),
                (
                    Codec::Flac,
                    Decision::Packaged(PackageFormat::FragmentedMp4)
                ),
                (Codec::Mp3, Decision::Packaged(PackageFormat::FragmentedMp4)),
                (
                    Codec::Opus,
                    Decision::Packaged(PackageFormat::FragmentedMp4)
                ),
                (
                    Codec::Pcm,
                    Decision::CannotPlay(vec![Reason::UnsupportedContainer {
                        codec: Codec::Pcm,
                        container: Container::Wav,
                    }])
                ),
                (
                    Codec::Vorbis,
                    Decision::CannotPlay(vec![Reason::UnsupportedContainer {
                        codec: Codec::Vorbis,
                        container: Container::Ogg,
                    }])
                ),
            ]
        );
    }

    #[test]
    fn each_core_format_packages_or_plays_the_original_when_only_mse_can_decode_it() {
        let got: Vec<(Codec, Decision)> = CORES
            .iter()
            .map(|(codec, container)| {
                (
                    *codec,
                    decide_audio(&tech(*codec, *container), &mse_only(*codec, *container)),
                )
            })
            .collect();
        assert_eq!(
            got,
            [
                (Codec::Aac, Decision::Direct),
                (Codec::Alac, Decision::Direct),
                (
                    Codec::Flac,
                    Decision::Packaged(PackageFormat::FragmentedMp4)
                ),
                (Codec::Mp3, Decision::Packaged(PackageFormat::FragmentedMp4)),
                (
                    Codec::Opus,
                    Decision::Packaged(PackageFormat::FragmentedMp4)
                ),
                (Codec::Pcm, Decision::Direct),
                (Codec::Vorbis, Decision::Direct),
            ]
        );
    }

    #[test]
    fn alac_that_this_browser_cannot_decode_uses_these_words() {
        let decision = decide_audio(&alac_16_44(), &DeviceCaps::none());
        assert_eq!(
            decision,
            Decision::CannotPlay(vec![Reason::CannotDecode { codec: Codec::Alac }])
        );
        assert_eq!(
            decision.reasons(),
            [Reason::CannotDecode { codec: Codec::Alac }]
        );
        assert_eq!(
            Reason::CannotDecode { codec: Codec::Alac }.words(),
            "Cannot play here: this browser cannot decode ALAC"
        );
    }

    #[test]
    fn native_support_stays_direct_even_when_the_packager_is_available() {
        let device = supports(Codec::Flac, Container::Flac);
        assert!(device.accepts_package(PackageFormat::FragmentedMp4));
        assert_eq!(decide_audio(&flac_24_96(), &device), Decision::Direct);
    }

    #[test]
    fn a_packagable_codec_without_mse_for_the_package_cannot_open_the_container() {
        let device = DeviceCaps::none()
            .with_codec(Codec::Flac, PlayCap::Native)
            .with_package(PackageFormat::FragmentedMp4, false);
        assert_eq!(
            decide_audio(&tech(Codec::Flac, Container::Flac), &device),
            Decision::CannotPlay(vec![Reason::UnsupportedContainer {
                codec: Codec::Flac,
                container: Container::Flac,
            }])
        );
    }

    #[test]
    fn flac_in_ogg_packages_when_the_device_plays_flac_but_not_ogg() {
        let device = codec_only(Codec::Flac).with_container(Container::Flac, PlayCap::Native);
        assert_eq!(
            decide_audio(&tech(Codec::Flac, Container::Ogg), &device),
            Decision::Packaged(PackageFormat::FragmentedMp4)
        );
        assert_eq!(
            decide_audio(
                &tech(Codec::Flac, Container::Ogg),
                &supports(Codec::Flac, Container::Ogg)
            ),
            Decision::Direct
        );
    }

    #[test]
    fn mse_only_aac_in_mp4_plays_the_original() {
        assert_eq!(
            decide_audio(
                &tech(Codec::Aac, Container::Mp4),
                &mse_only(Codec::Aac, Container::Mp4)
            ),
            Decision::Direct
        );
    }

    #[test]
    fn the_packager_carries_only_flac_mp3_and_opus() {
        let got: Vec<(Codec, Option<PackageFormat>)> = Codec::ALL
            .iter()
            .map(|codec| (*codec, package_format_for(*codec)))
            .collect();
        assert_eq!(
            got,
            [
                (Codec::Aac, None),
                (Codec::Alac, None),
                (Codec::Flac, Some(PackageFormat::FragmentedMp4)),
                (Codec::Mp3, Some(PackageFormat::FragmentedMp4)),
                (Codec::Opus, Some(PackageFormat::FragmentedMp4)),
                (Codec::Pcm, None),
                (Codec::Vorbis, None),
            ]
        );
    }

    #[test]
    fn every_codec_and_container_has_these_badge_words() {
        let codecs: Vec<(Codec, String)> = Codec::ALL
            .iter()
            .map(|codec| (*codec, Reason::CannotDecode { codec: *codec }.words()))
            .collect();
        assert_eq!(
            codecs,
            [
                (
                    Codec::Aac,
                    "Cannot play here: this browser cannot decode AAC".to_owned()
                ),
                (
                    Codec::Alac,
                    "Cannot play here: this browser cannot decode ALAC".to_owned()
                ),
                (
                    Codec::Flac,
                    "Cannot play here: this browser cannot decode FLAC".to_owned()
                ),
                (
                    Codec::Mp3,
                    "Cannot play here: this browser cannot decode MP3".to_owned()
                ),
                (
                    Codec::Opus,
                    "Cannot play here: this browser cannot decode Opus".to_owned()
                ),
                (
                    Codec::Pcm,
                    "Cannot play here: this browser cannot decode PCM".to_owned()
                ),
                (
                    Codec::Vorbis,
                    "Cannot play here: this browser cannot decode Vorbis".to_owned()
                ),
            ]
        );
        let containers: Vec<(Container, String)> = Container::ALL
            .iter()
            .map(|container| {
                (
                    *container,
                    Reason::UnsupportedContainer {
                        codec: Codec::Aac,
                        container: *container,
                    }
                    .words(),
                )
            })
            .collect();
        assert_eq!(
            containers,
            [
                (
                    Container::Aiff,
                    "Cannot play here: this browser cannot open AIFF for AAC".to_owned()
                ),
                (
                    Container::Flac,
                    "Cannot play here: this browser cannot open FLAC for AAC".to_owned()
                ),
                (
                    Container::Mp4,
                    "Cannot play here: this browser cannot open MP4 for AAC".to_owned()
                ),
                (
                    Container::Mpeg,
                    "Cannot play here: this browser cannot open MPEG for AAC".to_owned()
                ),
                (
                    Container::Ogg,
                    "Cannot play here: this browser cannot open Ogg for AAC".to_owned()
                ),
                (
                    Container::Wav,
                    "Cannot play here: this browser cannot open WAV for AAC".to_owned()
                ),
            ]
        );
    }

    #[test]
    fn a_playable_decision_has_no_reasons() {
        assert_eq!(Decision::Direct.reasons(), []);
        assert_eq!(
            Decision::Packaged(PackageFormat::FragmentedMp4).reasons(),
            []
        );
    }

    #[test]
    fn device_caps_default_to_nothing_and_a_later_entry_replaces_the_earlier_one() {
        let empty = DeviceCaps::default();
        assert_eq!(empty, DeviceCaps::none());
        assert_eq!(empty.codec(Codec::Flac), PlayCap::None);
        assert_eq!(empty.container(Container::Flac), PlayCap::None);
        assert!(!empty.accepts_package(PackageFormat::FragmentedMp4));
        assert_eq!(PlayCap::default(), PlayCap::None);

        let replaced = DeviceCaps::none()
            .with_codec(Codec::Flac, PlayCap::Native)
            .with_codec(Codec::Flac, PlayCap::MseOnly)
            .with_container(Container::Flac, PlayCap::Native)
            .with_container(Container::Flac, PlayCap::None)
            .with_package(PackageFormat::FragmentedMp4, true)
            .with_package(PackageFormat::FragmentedMp4, false);
        assert_eq!(replaced.codec(Codec::Flac), PlayCap::MseOnly);
        assert_eq!(replaced.container(Container::Flac), PlayCap::None);
        assert!(!replaced.accepts_package(PackageFormat::FragmentedMp4));
        assert_eq!(replaced.codec(Codec::Mp3), PlayCap::None);
        assert_eq!(replaced.container(Container::Mpeg), PlayCap::None);
    }

    #[test]
    fn the_details_summary_for_flac_played_directly() {
        let track = record("Helplessness Blues", "Fleet Foxes", flac_24_96(), None);
        let decision = Decision::Direct;
        assert_eq!(
            track_details(&track, &decision, &gain(Source::TaggedTrack)),
            TrackDetails {
                title: "Helplessness Blues".to_owned(),
                credit: "Fleet Foxes".to_owned(),
                artists: vec![artist_id(SYMBOLS)],
                album: Some(album_id()),
                format: flac_24_96(),
                decision: Decision::Direct,
                badge: "Original FLAC, 24-bit, 96 kHz, played directly".to_owned(),
                gapless: false,
                gain_source: Source::TaggedTrack,
            }
        );
    }

    #[test]
    fn the_details_summary_for_mp3_with_lame_trim_through_the_packager() {
        let trim = Trim {
            delay: 576,
            padding: 1_584,
        };
        let track = record("Only in Dreams", "Weezer", mp3_44k(), Some(trim));
        let decision = Decision::Packaged(PackageFormat::FragmentedMp4);
        assert_eq!(
            track_details(&track, &decision, &gain(Source::TaggedAlbum)),
            TrackDetails {
                title: "Only in Dreams".to_owned(),
                credit: "Weezer".to_owned(),
                artists: vec![artist_id(SYMBOLS)],
                album: Some(album_id()),
                format: mp3_44k(),
                decision: Decision::Packaged(PackageFormat::FragmentedMp4),
                badge: "Original MP3, 44.1 kHz, played through the packager".to_owned(),
                gapless: true,
                gain_source: Source::TaggedAlbum,
            }
        );
    }

    #[test]
    fn the_details_summary_for_opus_with_pre_skip() {
        let trim = Trim {
            delay: 312,
            padding: 0,
        };
        let track = record("Weight of Love", "The Black Keys", opus_48k(), Some(trim));
        let decision = Decision::Packaged(PackageFormat::FragmentedMp4);
        assert_eq!(
            track_details(&track, &decision, &gain(Source::TaggedTrack)),
            TrackDetails {
                title: "Weight of Love".to_owned(),
                credit: "The Black Keys".to_owned(),
                artists: vec![artist_id(SYMBOLS)],
                album: Some(album_id()),
                format: opus_48k(),
                decision: Decision::Packaged(PackageFormat::FragmentedMp4),
                badge: "Original Opus, 48 kHz, played through the packager".to_owned(),
                gapless: true,
                gain_source: Source::TaggedTrack,
            }
        );
    }

    #[test]
    fn the_details_summary_for_a_track_with_no_gain_tags_is_estimated() {
        let track = record("Untitled", "Unknown", flac_24_96(), None);
        assert_eq!(
            track_details(&track, &Decision::Direct, &gain(Source::Estimated)).gain_source,
            Source::Estimated
        );
        assert_eq!(
            track_details(&track, &Decision::Direct, &gain(Source::Measured)).gain_source,
            Source::Measured
        );
        assert_eq!(
            track_details(&track, &Decision::Direct, &gain(Source::Off)).gain_source,
            Source::Off
        );
    }

    #[test]
    fn the_details_summary_for_alac_this_browser_cannot_decode() {
        let track = record(
            "Kimigayo",
            "Unknown Artist",
            alac_16_44(),
            Some(Trim::default()),
        );
        let decision = Decision::CannotPlay(vec![Reason::CannotDecode { codec: Codec::Alac }]);
        assert_eq!(
            track_details(&track, &decision, &gain(Source::Estimated)),
            TrackDetails {
                title: "Kimigayo".to_owned(),
                credit: "Unknown Artist".to_owned(),
                artists: vec![artist_id(SYMBOLS)],
                album: Some(album_id()),
                format: alac_16_44(),
                decision: Decision::CannotPlay(vec![Reason::CannotDecode { codec: Codec::Alac }]),
                badge: "Cannot play here: this browser cannot decode ALAC".to_owned(),
                gapless: false,
                gain_source: Source::Estimated,
            }
        );
    }

    #[test]
    fn gapless_needs_a_known_trim_and_the_packager_path() {
        let trim = Some(Trim {
            delay: 576,
            padding: 1_584,
        });
        let packaged = Decision::Packaged(PackageFormat::FragmentedMp4);
        let cannot = Decision::CannotPlay(vec![Reason::CannotDecode { codec: Codec::Mp3 }]);
        let estimated = gain(Source::Estimated);
        assert_eq!(
            [
                track_details(&record("A", "B", mp3_44k(), trim), &packaged, &estimated).gapless,
                track_details(
                    &record("A", "B", mp3_44k(), Some(Trim::default())),
                    &packaged,
                    &estimated
                )
                .gapless,
                track_details(&record("A", "B", mp3_44k(), None), &packaged, &estimated).gapless,
                track_details(
                    &record("A", "B", mp3_44k(), trim),
                    &Decision::Direct,
                    &estimated
                )
                .gapless,
                track_details(&record("A", "B", mp3_44k(), trim), &cannot, &estimated).gapless,
            ],
            [true, true, false, false, false]
        );
    }

    #[test]
    fn details_keep_every_credited_artist_and_a_missing_album() {
        let mut track = record("Duets", "A feat. B", flac_24_96(), None);
        track.artists = vec![artist_id(SYMBOLS), artist_id(OTHER_SYMBOLS)];
        track.album = None;
        let details = track_details(&track, &Decision::Direct, &gain(Source::TaggedTrack));
        assert_eq!(
            (
                details.artists.clone(),
                details.album,
                details.credit.clone(),
                details.title.clone()
            ),
            (
                vec![artist_id(SYMBOLS), artist_id(OTHER_SYMBOLS)],
                None,
                "A feat. B".to_owned(),
                "Duets".to_owned()
            )
        );
        assert_ne!(details.title, track.title_sort.clone().unwrap());
    }

    #[test]
    fn a_badge_without_rate_or_depth_is_only_the_codec_and_the_path() {
        let bare = tech(Codec::Mp3, Container::Mpeg);
        let mut track = record("Bare", "Solo", bare, None);
        track.tech = bare;
        assert_eq!(
            track_details(&track, &Decision::Direct, &gain(Source::Estimated)).badge,
            "Original MP3, played directly"
        );
        assert_eq!(
            track_details(
                &track,
                &Decision::Packaged(PackageFormat::FragmentedMp4),
                &gain(Source::Estimated)
            )
            .badge,
            "Original MP3, played through the packager"
        );
        assert_eq!(
            track_details(
                &track,
                &Decision::CannotPlay(Vec::new()),
                &gain(Source::Estimated)
            )
            .badge,
            "Cannot play here"
        );
    }

    #[test]
    fn sample_rate_words_use_these_literals() {
        let words = |hz: u32, path: &str| {
            let format = AudioFormat {
                sample_rate: Some(SampleRate::new(hz).unwrap()),
                bit_depth: None,
                channels: None,
                bitrate: None,
                duration: None,
            };
            let info = TechInfo::new(Codec::Pcm, Container::Wav, format).unwrap();
            let mut track = record("Rate", "Test", info, None);
            track.tech = info;
            let badge = track_details(&track, &Decision::Direct, &gain(Source::Estimated)).badge;
            assert_eq!(badge, format!("Original PCM, {path}, played directly"));
        };
        words(96_000, "96 kHz");
        words(48_000, "48 kHz");
        words(44_100, "44.1 kHz");
        words(88_200, "88.2 kHz");
        words(22_050, "22050 Hz");
        words(8_000, "8 kHz");
        words(100, "0.1 kHz");
        words(50, "50 Hz");
        words(1, "1 Hz");
    }

    #[test]
    fn the_summary_field_list_is_this_literal_and_has_no_path() {
        assert_eq!(
            TrackDetails::FIELDS,
            [
                "title",
                "credit",
                "artists",
                "album",
                "format",
                "decision",
                "badge",
                "gapless",
                "gain_source",
            ]
        );
        assert_eq!(
            TrackDetails::FIELDS
                .iter()
                .copied()
                .filter(|field| field.contains("path"))
                .collect::<Vec<_>>(),
            Vec::<&str>::new()
        );
    }
}
