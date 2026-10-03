//! Technical facts about a file's audio: codec, container and format
//! (API-CAT-04, MUS-021, LIB-146).
//!
//! Clients draw quality badges from these fields (LIB-146), and the
//! playback decision engine reads them (WP-055), so a codec a container
//! cannot carry, or a bit depth on a lossy codec, is refused here rather
//! than reaching either.

use std::num::NonZeroU32;

use crate::values::{BitDepth, Channels, Duration, SampleRate};

use super::coded::coded;
use super::error::CatalogError;

coded! {
    /// An audio codec R1 reads.
    Codec: u8 {
        /// Advanced Audio Coding.
        Aac = 1,
        /// Apple Lossless.
        Alac = 2,
        /// Free Lossless Audio Codec.
        Flac = 3,
        /// MPEG-1 or MPEG-2 Audio Layer III.
        Mp3 = 4,
        /// Opus.
        Opus = 5,
        /// Uncompressed pulse-code modulation, integer or floating point.
        Pcm = 6,
        /// Vorbis.
        Vorbis = 7,
    }
}

impl Codec {
    /// Whether the codec keeps every sample exactly.
    #[must_use]
    pub const fn lossless(self) -> bool {
        matches!(self, Self::Alac | Self::Flac | Self::Pcm)
    }
}

coded! {
    /// A file format that holds audio, as R1 reads it.
    Container: u8 {
        /// An AIFF or AIFF-C file.
        Aiff = 1,
        /// A native FLAC file.
        Flac = 2,
        /// An ISO base media file: MP4, M4A.
        Mp4 = 3,
        /// A stream of MPEG audio frames: an MP3 file.
        Mpeg = 4,
        /// An Ogg file.
        Ogg = 5,
        /// A RIFF WAVE file.
        Wav = 6,
    }
}

impl Container {
    /// Whether this container can carry `codec`.
    #[must_use]
    pub const fn carries(self, codec: Codec) -> bool {
        matches!(
            (self, codec),
            (
                Self::Mp4,
                Codec::Aac | Codec::Alac | Codec::Flac | Codec::Mp3 | Codec::Opus | Codec::Pcm
            ) | (Self::Flac, Codec::Flac)
                | (Self::Ogg, Codec::Flac | Codec::Opus | Codec::Vorbis)
                | (Self::Mpeg, Codec::Mp3)
                | (Self::Wav, Codec::Mp3 | Codec::Pcm)
                | (Self::Aiff, Codec::Pcm)
        )
    }
}

/// A bitrate in bits a second, never zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bitrate(NonZeroU32);

impl Bitrate {
    /// A bitrate of `bps` bits a second.
    ///
    /// # Errors
    ///
    /// [`CatalogError::ZeroBitrate`] for zero.
    pub fn new(bps: u32) -> Result<Self, CatalogError> {
        NonZeroU32::new(bps)
            .map(Self)
            .ok_or(CatalogError::ZeroBitrate)
    }

    /// The bitrate in bits a second.
    #[must_use]
    pub const fn bps(self) -> NonZeroU32 {
        self.0
    }
}

/// The audio format, as far as the file states it. Each part is a typed
/// value with its own range (SEC-MED-014).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct AudioFormat {
    /// Samples a second.
    pub sample_rate: Option<SampleRate>,
    /// Bits a sample, for lossless codecs only.
    pub bit_depth: Option<BitDepth>,
    /// Channels.
    pub channels: Option<Channels>,
    /// The average bitrate.
    pub bitrate: Option<Bitrate>,
    /// How long the audio plays.
    pub duration: Option<Duration>,
}

/// A file's codec, container and audio format, in a combination that
/// exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TechInfo {
    codec: Codec,
    container: Container,
    format: AudioFormat,
}

impl TechInfo {
    /// The facts of a file whose `container` holds `codec` in `format`.
    ///
    /// # Errors
    ///
    /// [`CatalogError::CodecNotInContainer`] when the container cannot
    /// carry the codec; [`CatalogError::BitDepthOnLossy`] for a bit depth
    /// on a lossy codec.
    pub fn new(
        codec: Codec,
        container: Container,
        format: AudioFormat,
    ) -> Result<Self, CatalogError> {
        if !container.carries(codec) {
            return Err(CatalogError::CodecNotInContainer { codec, container });
        }
        if format.bit_depth.is_some() && !codec.lossless() {
            return Err(CatalogError::BitDepthOnLossy { codec });
        }
        Ok(Self {
            codec,
            container,
            format,
        })
    }

    /// The codec.
    #[must_use]
    pub const fn codec(self) -> Codec {
        self.codec
    }

    /// The container.
    #[must_use]
    pub const fn container(self) -> Container {
        self.container
    }

    /// The audio format.
    #[must_use]
    pub const fn format(self) -> AudioFormat {
        self.format
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codecs_have_these_codes_and_losslessness() {
        let codecs: Vec<(Codec, u8, bool)> = Codec::ALL
            .iter()
            .map(|c| (*c, c.code(), c.lossless()))
            .collect();
        assert_eq!(
            codecs,
            [
                (Codec::Aac, 1, false),
                (Codec::Alac, 2, true),
                (Codec::Flac, 3, true),
                (Codec::Mp3, 4, false),
                (Codec::Opus, 5, false),
                (Codec::Pcm, 6, true),
                (Codec::Vorbis, 7, false),
            ]
        );
        let read: Vec<Option<Codec>> = (0..=8).map(Codec::from_code).collect();
        let mut expected = vec![None];
        expected.extend(Codec::ALL.iter().copied().map(Some));
        expected.push(None);
        assert_eq!(read, expected);
    }

    #[test]
    fn containers_have_these_codes() {
        let containers: Vec<(Container, u8)> =
            Container::ALL.iter().map(|c| (*c, c.code())).collect();
        assert_eq!(
            containers,
            [
                (Container::Aiff, 1),
                (Container::Flac, 2),
                (Container::Mp4, 3),
                (Container::Mpeg, 4),
                (Container::Ogg, 5),
                (Container::Wav, 6),
            ]
        );
        let read: Vec<Option<Container>> = (0..=7).map(Container::from_code).collect();
        let mut expected = vec![None];
        expected.extend(Container::ALL.iter().copied().map(Some));
        expected.push(None);
        assert_eq!(read, expected);
    }

    /// Every pair a container can carry, written out by container.
    const CARRIED: [(Container, Codec); 14] = [
        (Container::Aiff, Codec::Pcm),
        (Container::Flac, Codec::Flac),
        (Container::Mp4, Codec::Aac),
        (Container::Mp4, Codec::Alac),
        (Container::Mp4, Codec::Flac),
        (Container::Mp4, Codec::Mp3),
        (Container::Mp4, Codec::Opus),
        (Container::Mp4, Codec::Pcm),
        (Container::Mpeg, Codec::Mp3),
        (Container::Ogg, Codec::Flac),
        (Container::Ogg, Codec::Opus),
        (Container::Ogg, Codec::Vorbis),
        (Container::Wav, Codec::Mp3),
        (Container::Wav, Codec::Pcm),
    ];

    #[test]
    fn containers_carry_exactly_the_listed_codecs() {
        let carried: Vec<(Container, Codec)> = Container::ALL
            .iter()
            .flat_map(|container| Codec::ALL.iter().map(move |codec| (*container, *codec)))
            .filter(|(container, codec)| container.carries(*codec))
            .collect();
        assert_eq!(carried, CARRIED);
    }

    #[test]
    fn keeps_a_whole_lossless_format() {
        let format = AudioFormat {
            sample_rate: Some(SampleRate::new(96_000).unwrap()),
            bit_depth: Some(BitDepth::new(24).unwrap()),
            channels: Some(Channels::new(2).unwrap()),
            bitrate: Some(Bitrate::new(2_304_000).unwrap()),
            duration: Some(Duration::from_millis(215_000).unwrap()),
        };
        let tech = TechInfo::new(Codec::Flac, Container::Flac, format).unwrap();
        assert_eq!(
            (tech.codec(), tech.container(), tech.format()),
            (Codec::Flac, Container::Flac, format)
        );
    }

    #[test]
    fn keeps_a_lossy_format_without_a_bit_depth() {
        let format = AudioFormat {
            sample_rate: Some(SampleRate::new(48_000).unwrap()),
            bit_depth: None,
            channels: Some(Channels::new(2).unwrap()),
            bitrate: Some(Bitrate::new(160_000).unwrap()),
            duration: None,
        };
        let tech = TechInfo::new(Codec::Opus, Container::Ogg, format).unwrap();
        assert_eq!(
            (tech.codec(), tech.container(), tech.format()),
            (Codec::Opus, Container::Ogg, format)
        );
    }

    #[test]
    fn refuses_a_codec_the_container_cannot_carry() {
        assert_eq!(
            TechInfo::new(Codec::Vorbis, Container::Mp4, AudioFormat::default()),
            Err(CatalogError::CodecNotInContainer {
                codec: Codec::Vorbis,
                container: Container::Mp4,
            })
        );
    }

    #[test]
    fn refuses_a_bit_depth_on_a_lossy_codec() {
        let format = AudioFormat {
            bit_depth: Some(BitDepth::new(16).unwrap()),
            ..AudioFormat::default()
        };
        assert_eq!(
            TechInfo::new(Codec::Mp3, Container::Mpeg, format),
            Err(CatalogError::BitDepthOnLossy { codec: Codec::Mp3 })
        );
    }

    #[test]
    fn refuses_the_container_first() {
        let format = AudioFormat {
            bit_depth: Some(BitDepth::new(16).unwrap()),
            ..AudioFormat::default()
        };
        assert_eq!(
            TechInfo::new(Codec::Aac, Container::Ogg, format),
            Err(CatalogError::CodecNotInContainer {
                codec: Codec::Aac,
                container: Container::Ogg,
            })
        );
    }

    #[test]
    fn a_bitrate_is_never_zero() {
        assert_eq!(Bitrate::new(0), Err(CatalogError::ZeroBitrate));
        assert_eq!(
            [
                Bitrate::new(1).unwrap().bps().get(),
                Bitrate::new(u32::MAX).unwrap().bps().get()
            ],
            [1, u32::MAX]
        );
    }
}
