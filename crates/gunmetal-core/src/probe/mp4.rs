//! An MP4 file's facts, from its first audio track and its item lists.

use crate::catalog::{AudioFormat, Bitrate, Codec, Container, Trim};
use crate::formats::detect::Format;
use crate::formats::mp4::{AudioEntry, CodecConfig, Mp4Audio};
use crate::values::{BitDepth, Channels, SampleRate};

use super::draft::{Draft, span};
use super::facts::{PartProblem, SeekIndex, TagBlock};

/// The rate Opus is always decoded at, whatever its input was.
const OPUS_RATE: u32 = 48_000;

/// What a sample entry's codec configuration says.
#[derive(Debug, Default, PartialEq, Eq)]
struct Coded {
    /// The codec, or `None` for one that is not read.
    codec: Option<Codec>,
    /// The sample rate, when the configuration gives one.
    rate: Option<SampleRate>,
    /// The channels, when the configuration gives them.
    channels: Option<Channels>,
    /// The bit depth of a lossless codec.
    bits: Option<BitDepth>,
    /// The average bitrate, or 0 when the configuration gives none.
    bitrate: u32,
    /// Samples to skip at the start, for Opus.
    skip: Option<u16>,
}

/// Reads the codec configuration of `entry`.
///
/// An `mp4a` entry is AAC for the MPEG-4 and MPEG-2 AAC object type
/// indications and MP3 for the MPEG-2 and MPEG-1 audio ones. Its sample
/// rate is the one its `AudioSpecificConfig` gives, and the output rate of
/// an explicit SBR extension before that, since the entry's own field
/// cannot hold a rate above 65,535 Hz.
fn coded(entry: &AudioEntry) -> Coded {
    match &entry.config {
        CodecConfig::Esds(esds) => {
            let audio = esds
                .specific
                .as_ref()
                .and_then(|specific| specific.audio.as_ref());
            Coded {
                codec: match esds.object_type {
                    0x40 | 0x66..=0x68 => Some(Codec::Aac),
                    0x69 | 0x6B => Some(Codec::Mp3),
                    _ => None,
                },
                rate: audio.and_then(|audio| {
                    audio
                        .extension
                        .as_ref()
                        .and_then(|extension| extension.sample_rate)
                        .or(audio.sample_rate)
                }),
                bitrate: esds.avg_bitrate,
                ..Coded::default()
            }
        }
        CodecConfig::Alac(alac) => Coded {
            codec: Some(Codec::Alac),
            rate: alac.sample_rate,
            channels: alac.channels,
            bits: alac.bits,
            bitrate: alac.avg_bitrate,
            skip: None,
        },
        CodecConfig::Flac(flac) => Coded {
            codec: Some(Codec::Flac),
            rate: flac.sample_rate,
            channels: flac.channels,
            bits: flac.bits,
            ..Coded::default()
        },
        CodecConfig::Opus(opus) => Coded {
            codec: Some(Codec::Opus),
            rate: SampleRate::new(OPUS_RATE).ok(),
            channels: opus.channels,
            skip: Some(opus.pre_skip),
            ..Coded::default()
        },
        CodecConfig::Missing => Coded::default(),
    }
}

/// The draft of an MP4 file of `file_len` octets.
///
/// What the codec configuration says comes before what the sample entry
/// says. Tags lie inside the movie, so the audio window is the whole file.
pub(super) fn draft(audio: Mp4Audio, file_len: u64) -> Draft {
    let Mp4Audio {
        track,
        items,
        sample_tables,
        problems,
        file_type: _,
    } = audio;
    let coded = coded(&track.entry);
    let mut problems = problems.into_iter().map(PartProblem::Mp4).collect();
    let duration = span(track.duration, Some(track.timescale), &mut problems);
    Draft {
        format: Format::Mp4,
        codec: coded.codec,
        container: Container::Mp4,
        at: track.offset,
        audio: AudioFormat {
            sample_rate: coded.rate.or(track.entry.sample_rate),
            bit_depth: coded.bits,
            channels: coded.channels.or(track.entry.channels),
            bitrate: Bitrate::new(coded.bitrate).ok(),
            duration,
        },
        trim: coded.skip.map(|skip| Trim {
            delay: skip.into(),
            padding: 0,
        }),
        md5: None,
        window: (0, file_len),
        seek: SeekIndex::Mp4(sample_tables),
        pictures: Vec::new(),
        tags: vec![TagBlock::Mp4(items)],
        problems,
        jobs: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use super::*;
    use crate::catalog::{ByteRange, FileFacts, IdentityInputs, TechInfo};
    use crate::catalog::{LyricsOrigin, LyricsSource, LyricsTiming};
    use crate::formats::mp4::{
        Alac, AudioSpecificConfig, AudioTrack, Esds, Extension, Flac, FourCc, IlstItem, ItemKey,
        ItemValue, Mp4Problem, Opus, SampleTableRanges, Specific,
    };
    use crate::parse::{Budget, Limits};
    use crate::probe::facts::Probed;
    use crate::text::Text;
    use crate::values::{Duration, GainDb};

    fn rate(hz: u32) -> Option<SampleRate> {
        SampleRate::new(hz).ok()
    }

    fn channels(count: u32) -> Option<Channels> {
        Channels::new(count).ok()
    }

    fn bits(count: u32) -> Option<BitDepth> {
        BitDepth::new(count).ok()
    }

    /// A sample entry that says 44.1 kHz, 16 bits and stereo, with
    /// `config`.
    fn entry(config: CodecConfig) -> AudioEntry {
        AudioEntry {
            format: FourCc(*b"test"),
            channels: channels(2),
            bits: bits(16),
            sample_rate: rate(44_100),
            config,
        }
    }

    /// A descriptor with `object_type`, 96,000 bits a second, and `audio`
    /// as its specific information.
    fn esds(object_type: u8, audio: Option<AudioSpecificConfig>) -> CodecConfig {
        CodecConfig::Esds(Esds {
            object_type,
            max_bitrate: 0,
            avg_bitrate: 96_000,
            specific: Some(Specific { range: 0..2, audio }),
        })
    }

    fn alac() -> CodecConfig {
        CodecConfig::Alac(Alac {
            frame_length: 4_096,
            bits: bits(24),
            channels: channels(6),
            avg_bitrate: 700_000,
            sample_rate: rate(96_000),
            cookie: 0..24,
        })
    }

    fn opus() -> CodecConfig {
        CodecConfig::Opus(Opus {
            channels: channels(1),
            pre_skip: 312,
            input_sample_rate: 44_100,
            output_gain: GainDb::from_q7_8(0),
            mapping_family: 0,
            body: 0..11,
        })
    }

    #[test]
    fn names_the_codec_of_an_mp4a_entry_by_its_object_type() {
        let cases = [
            (0x3F, None),
            (0x40, Some(Codec::Aac)),
            (0x41, None),
            (0x65, None),
            (0x66, Some(Codec::Aac)),
            (0x67, Some(Codec::Aac)),
            (0x68, Some(Codec::Aac)),
            (0x69, Some(Codec::Mp3)),
            (0x6A, None),
            (0x6B, Some(Codec::Mp3)),
            (0x6C, None),
        ];
        for (object_type, codec) in cases {
            assert_eq!(
                coded(&entry(esds(object_type, None))),
                Coded {
                    codec,
                    bitrate: 96_000,
                    ..Coded::default()
                },
                "object type {object_type:#04X}"
            );
        }
    }

    #[test]
    fn takes_the_rate_of_an_mp4a_entry_from_its_audio_specific_config() {
        let plain = AudioSpecificConfig {
            object_type: 2,
            sample_rate: rate(22_050),
            channel_config: 2,
            extension: None,
        };
        let extended = |sample_rate| AudioSpecificConfig {
            object_type: 5,
            extension: Some(Extension {
                object_type: 2,
                sample_rate,
            }),
            ..plain.clone()
        };
        let rate_of = |audio| coded(&entry(esds(0x40, Some(audio)))).rate;
        assert_eq!(rate_of(plain.clone()), rate(22_050));
        // An SBR extension gives the rate the stream is played at.
        assert_eq!(rate_of(extended(rate(44_100))), rate(44_100));
        assert_eq!(rate_of(extended(None)), rate(22_050));
    }

    #[test]
    fn gives_no_rate_for_a_descriptor_without_specific_information() {
        let config = CodecConfig::Esds(Esds {
            object_type: 0x40,
            max_bitrate: 0,
            avg_bitrate: 0,
            specific: None,
        });
        assert_eq!(
            coded(&entry(config)),
            Coded {
                codec: Some(Codec::Aac),
                ..Coded::default()
            }
        );
    }

    #[test]
    fn reads_the_lossless_and_opus_configurations() {
        assert_eq!(
            coded(&entry(alac())),
            Coded {
                codec: Some(Codec::Alac),
                rate: rate(96_000),
                channels: channels(6),
                bits: bits(24),
                bitrate: 700_000,
                skip: None,
            }
        );
        let flac = CodecConfig::Flac(Flac {
            min_block: 4_096,
            max_block: 4_096,
            sample_rate: rate(88_200),
            channels: channels(1),
            bits: bits(20),
            total_samples: None,
            blocks: 0..38,
        });
        assert_eq!(
            coded(&entry(flac)),
            Coded {
                codec: Some(Codec::Flac),
                rate: rate(88_200),
                channels: channels(1),
                bits: bits(20),
                bitrate: 0,
                skip: None,
            }
        );
        // Opus plays at 48 kHz, whatever its input was.
        assert_eq!(
            coded(&entry(opus())),
            Coded {
                codec: Some(Codec::Opus),
                rate: rate(48_000),
                channels: channels(1),
                bits: None,
                bitrate: 0,
                skip: Some(312),
            }
        );
        assert_eq!(coded(&entry(CodecConfig::Missing)), Coded::default());
    }

    /// The result of a probe whose MP4 parser found `config` in a track
    /// that starts at 40, in a file of 500 octets.
    fn finished(config: CodecConfig, duration: Option<u64>) -> Probed {
        finished_with(config, duration, vec![])
    }

    /// As [`finished`], for a movie whose item lists hold `items`.
    fn finished_with(config: CodecConfig, duration: Option<u64>, items: Vec<IlstItem>) -> Probed {
        let audio = Mp4Audio {
            file_type: None,
            track: AudioTrack {
                offset: 40,
                timescale: NonZeroU32::new(1_000).unwrap(),
                duration,
                entry: entry(config),
            },
            items,
            sample_tables: SampleTableRanges::default(),
            problems: vec![Mp4Problem::ExtraAudioTrack { offset: 300 }],
        };
        draft(audio, 500)
            .finish(&Limits::DEFAULT, &mut Budget::for_input(0, 0, 100), 77)
            .unwrap()
    }

    fn probed(tech: TechInfo, trim: Option<Trim>) -> Probed {
        Probed {
            format: Format::Mp4,
            facts: FileFacts {
                tech,
                trim,
                artwork: vec![],
                lyrics: vec![],
                identity: IdentityInputs {
                    audio_md5: None,
                    audio_window: ByteRange::new(0, 500).unwrap(),
                },
                parser_version: 1,
                bytes_read: 77,
            },
            tags: vec![TagBlock::Mp4(vec![])],
            seek: SeekIndex::Mp4(SampleTableRanges::default()),
            problems: vec![PartProblem::Mp4(Mp4Problem::ExtraAudioTrack {
                offset: 300,
            })],
        }
    }

    fn tech(
        codec: Codec,
        format: (Option<SampleRate>, Option<BitDepth>, Option<Channels>),
        bitrate: Option<u32>,
        millis: Option<u64>,
    ) -> TechInfo {
        let (sample_rate, bit_depth, channels) = format;
        TechInfo::new(
            codec,
            Container::Mp4,
            AudioFormat {
                sample_rate,
                bit_depth,
                channels,
                bitrate: bitrate.and_then(|bps| Bitrate::new(bps).ok()),
                duration: millis.and_then(|millis| Duration::from_millis(millis).ok()),
            },
        )
        .unwrap()
    }

    /// The configuration's rate, channels, depth and bitrate come before
    /// the sample entry's 44.1 kHz stereo, and the parser's problems are
    /// passed on.
    #[test]
    fn drafts_a_lossless_track_from_its_configuration() {
        assert_eq!(
            finished(alac(), Some(3_000)),
            probed(
                tech(
                    Codec::Alac,
                    (rate(96_000), bits(24), channels(6)),
                    Some(700_000),
                    Some(3_000)
                ),
                None
            )
        );
    }

    /// With no bitrate in the configuration, the 500 octets of the file in
    /// 4 seconds give 1,000 bits a second. The pre-skip is the trim.
    #[test]
    fn drafts_an_opus_track_with_its_pre_skip() {
        assert_eq!(
            finished(opus(), Some(4_000)),
            probed(
                tech(
                    Codec::Opus,
                    (rate(48_000), None, channels(1)),
                    Some(1_000),
                    Some(4_000)
                ),
                Some(Trim {
                    delay: 312,
                    padding: 0,
                })
            )
        );
    }

    /// A descriptor gives no rate or channels of its own here, so the
    /// sample entry's are used; a lossy codec has no bit depth.
    #[test]
    fn drafts_a_track_of_unknown_length_from_its_sample_entry() {
        assert_eq!(
            finished(esds(0x6B, None), None),
            probed(
                tech(
                    Codec::Mp3,
                    (rate(44_100), None, channels(2)),
                    Some(96_000),
                    None
                ),
                None
            )
        );
    }

    /// Only the text values of a `©lyr` item are lyrics: not a value of
    /// another kind in it, and not the text of another item.
    #[test]
    fn takes_lyrics_from_the_text_of_the_lyrics_item() {
        let text = |value: &str| {
            ItemValue::Text(Text {
                value: value.to_owned(),
                truncated: false,
                replaced: false,
            })
        };
        let items = vec![
            IlstItem {
                key: ItemKey::Atom(FourCc(*b"\xA9nam")),
                values: vec![text("[00:01.00]not lyrics")],
            },
            IlstItem {
                key: ItemKey::Atom(FourCc(*b"\xA9lyr")),
                values: vec![
                    ItemValue::Binary {
                        type_code: 0,
                        bytes: b"la".to_vec(),
                    },
                    text("la la"),
                    ItemValue::Unsigned(7),
                ],
            },
        ];
        let found = finished_with(alac(), None, items.clone());
        assert_eq!(found.tags, [TagBlock::Mp4(items)]);
        assert_eq!(
            found.facts.lyrics,
            Vec::from_iter(LyricsSource::new(
                LyricsOrigin::Mp4Item,
                LyricsTiming::Plain
            ))
        );
        assert_eq!(found.facts.artwork, []);
    }
}
