//! The limits every parse runs under.
//!
//! [`Limits::DEFAULT`] holds the defaults of the limits table in
//! `docs/security/media-and-parser-safety.md`, section 3, for every row a
//! core parser enforces in R1. The server may lower any limit, and may raise
//! one only up to its compiled-in ceiling ([`LimitKind::ceiling`]), so no
//! configuration can switch a limit off.
//!
//! Ceilings follow owner decision D-03 (the stricter value wins wherever two
//! documents disagree). A limit whose value a requirement row states outright
//! cannot be raised at all: the nesting depths (SEC-MED-005); the size of one
//! embedded picture and what a compressed picture may inflate to, because
//! SEC-MED-045 allows every artwork image at most 32 MiB encoded and
//! SEC-MED-009 stops inflating at the lower of that cap and the declared
//! size; the read and per-file caps (SEC-MED-010); and the lyrics caps,
//! where SEC-API-090 is stricter than SEC-MED-049 and the table. Every other
//! limit may be raised to four times its default, as the table allows.
//!
//! Rows the core does not enforce live with the code that does: the worker's
//! memory, deadlines and quarantine (the worker host), artwork dimensions
//! and loudness analysis (the worker's jobs), the IPC frame cap (the wire
//! codec, fixed by SEC-MED-023), and the rows for CUE sheets, subtitles,
//! archives and XML (the R2 parsers that need them).

use super::fault::ParseFault;

/// One limit in [`Limits`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitKind {
    /// Nesting depth of binary containers: EBML master elements, including
    /// the recursive `SimpleTag` and `ChapterAtom`, and ISOBMFF boxes.
    ContainerDepth,
    /// Nesting depth of frames embedded in `ID3v2` `CHAP` and `CTOC` frames.
    EmbeddedFrameDepth,
    /// Children of one parent in any container, except the index tables.
    Children,
    /// Index entries kept per file: cue points, keyframes, frame offsets.
    IndexEntries,
    /// Tag fields per file: `ID3v2` frames, Vorbis comments, MP4 `ilst`
    /// items, Matroska tags.
    TagFields,
    /// Octets of a short text field, such as a title, after decoding.
    ShortText,
    /// Octets of a long text field, such as a comment, after decoding.
    LongText,
    /// Octets of one embedded picture. A base64 picture in a Vorbis comment
    /// may take 4/3 of this.
    PictureBytes,
    /// Embedded pictures per file.
    Pictures,
    /// Octets of an `ID3v2` tag held in memory.
    Id3v2TagBytes,
    /// Octets a compressed picture may inflate to.
    InflatedPicture,
    /// Octets compressed codec private data or a subtitle frame may inflate
    /// to.
    InflatedCodecPrivate,
    /// Octets a compressed header may inflate to.
    InflatedHeader,
    /// Octets in one read request from a parser to its host (SEC-MED-010).
    ReadBytes,
    /// Octets a parser may read from one file for its metadata
    /// (SEC-MED-010).
    FileBytes,
    /// Octets of one lyrics text.
    LyricsBytes,
    /// Lines in one lyrics text.
    LyricsLines,
    /// Octets in one lyrics line.
    LyricsLineBytes,
    /// Milliseconds from the start of a track to a lyrics timestamp.
    LyricsTimestampMs,
    /// Octets of one playlist file.
    PlaylistBytes,
    /// Entries in one playlist file.
    PlaylistEntries,
    /// Octets in one playlist line.
    PlaylistLineBytes,
}

impl LimitKind {
    /// Every limit, in declaration order.
    pub const ALL: [Self; 22] = [
        Self::ContainerDepth,
        Self::EmbeddedFrameDepth,
        Self::Children,
        Self::IndexEntries,
        Self::TagFields,
        Self::ShortText,
        Self::LongText,
        Self::PictureBytes,
        Self::Pictures,
        Self::Id3v2TagBytes,
        Self::InflatedPicture,
        Self::InflatedCodecPrivate,
        Self::InflatedHeader,
        Self::ReadBytes,
        Self::FileBytes,
        Self::LyricsBytes,
        Self::LyricsLines,
        Self::LyricsLineBytes,
        Self::LyricsTimestampMs,
        Self::PlaylistBytes,
        Self::PlaylistEntries,
        Self::PlaylistLineBytes,
    ];

    /// The largest value this limit may be raised to.
    #[must_use]
    pub const fn ceiling(self) -> u64 {
        let default = Limits::DEFAULT.get(self);
        match self {
            // A requirement row states these values outright.
            Self::ContainerDepth
            | Self::EmbeddedFrameDepth
            | Self::PictureBytes
            | Self::InflatedPicture
            | Self::ReadBytes
            | Self::FileBytes
            | Self::LyricsBytes
            | Self::LyricsLines
            | Self::LyricsLineBytes
            | Self::LyricsTimestampMs => default,
            Self::Children
            | Self::IndexEntries
            | Self::TagFields
            | Self::ShortText
            | Self::LongText
            | Self::Pictures
            | Self::Id3v2TagBytes
            | Self::InflatedCodecPrivate
            | Self::InflatedHeader
            | Self::PlaylistBytes
            | Self::PlaylistEntries
            | Self::PlaylistLineBytes => default.saturating_mul(4),
        }
    }
}

/// Why a limit could not be changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitError {
    /// The new value was above the limit's compiled-in ceiling.
    AboveCeiling {
        /// Which limit it was.
        limit: LimitKind,
        /// The value asked for.
        value: u64,
        /// The largest value the limit may take.
        ceiling: u64,
    },
}

/// The limits one parse runs under.
///
/// The only values are [`Limits::DEFAULT`] and what
/// [`Limits::with_override`] derives from it, so every limit stays at or
/// below its ceiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    container_depth: u64,
    embedded_frame_depth: u64,
    children: u64,
    index_entries: u64,
    tag_fields: u64,
    short_text: u64,
    long_text: u64,
    picture_bytes: u64,
    pictures: u64,
    id3v2_tag_bytes: u64,
    inflated_picture: u64,
    inflated_codec_private: u64,
    inflated_header: u64,
    read_bytes: u64,
    file_bytes: u64,
    lyrics_bytes: u64,
    lyrics_lines: u64,
    lyrics_line_bytes: u64,
    lyrics_timestamp_ms: u64,
    playlist_bytes: u64,
    playlist_entries: u64,
    playlist_line_bytes: u64,
}

impl Limits {
    /// The defaults of the limits table.
    pub const DEFAULT: Self = Self {
        container_depth: 32,
        embedded_frame_depth: 4,
        children: 65_536,
        index_entries: 1_000_000,
        tag_fields: 4_096,
        short_text: 4_096,
        long_text: 65_536,
        picture_bytes: 33_554_432,
        pictures: 16,
        id3v2_tag_bytes: 67_108_864,
        inflated_picture: 33_554_432,
        inflated_codec_private: 1_048_576,
        inflated_header: 4_194_304,
        read_bytes: 16_777_216,
        file_bytes: 268_435_456,
        lyrics_bytes: 262_144,
        lyrics_lines: 10_000,
        lyrics_line_bytes: 4_096,
        lyrics_timestamp_ms: 86_400_000,
        playlist_bytes: 16_777_216,
        playlist_entries: 100_000,
        playlist_line_bytes: 8_192,
    };

    /// The current value of one limit.
    #[must_use]
    pub const fn get(&self, kind: LimitKind) -> u64 {
        match kind {
            LimitKind::ContainerDepth => self.container_depth,
            LimitKind::EmbeddedFrameDepth => self.embedded_frame_depth,
            LimitKind::Children => self.children,
            LimitKind::IndexEntries => self.index_entries,
            LimitKind::TagFields => self.tag_fields,
            LimitKind::ShortText => self.short_text,
            LimitKind::LongText => self.long_text,
            LimitKind::PictureBytes => self.picture_bytes,
            LimitKind::Pictures => self.pictures,
            LimitKind::Id3v2TagBytes => self.id3v2_tag_bytes,
            LimitKind::InflatedPicture => self.inflated_picture,
            LimitKind::InflatedCodecPrivate => self.inflated_codec_private,
            LimitKind::InflatedHeader => self.inflated_header,
            LimitKind::ReadBytes => self.read_bytes,
            LimitKind::FileBytes => self.file_bytes,
            LimitKind::LyricsBytes => self.lyrics_bytes,
            LimitKind::LyricsLines => self.lyrics_lines,
            LimitKind::LyricsLineBytes => self.lyrics_line_bytes,
            LimitKind::LyricsTimestampMs => self.lyrics_timestamp_ms,
            LimitKind::PlaylistBytes => self.playlist_bytes,
            LimitKind::PlaylistEntries => self.playlist_entries,
            LimitKind::PlaylistLineBytes => self.playlist_line_bytes,
        }
    }

    /// The field that holds one limit.
    const fn slot(&mut self, kind: LimitKind) -> &mut u64 {
        match kind {
            LimitKind::ContainerDepth => &mut self.container_depth,
            LimitKind::EmbeddedFrameDepth => &mut self.embedded_frame_depth,
            LimitKind::Children => &mut self.children,
            LimitKind::IndexEntries => &mut self.index_entries,
            LimitKind::TagFields => &mut self.tag_fields,
            LimitKind::ShortText => &mut self.short_text,
            LimitKind::LongText => &mut self.long_text,
            LimitKind::PictureBytes => &mut self.picture_bytes,
            LimitKind::Pictures => &mut self.pictures,
            LimitKind::Id3v2TagBytes => &mut self.id3v2_tag_bytes,
            LimitKind::InflatedPicture => &mut self.inflated_picture,
            LimitKind::InflatedCodecPrivate => &mut self.inflated_codec_private,
            LimitKind::InflatedHeader => &mut self.inflated_header,
            LimitKind::ReadBytes => &mut self.read_bytes,
            LimitKind::FileBytes => &mut self.file_bytes,
            LimitKind::LyricsBytes => &mut self.lyrics_bytes,
            LimitKind::LyricsLines => &mut self.lyrics_lines,
            LimitKind::LyricsLineBytes => &mut self.lyrics_line_bytes,
            LimitKind::LyricsTimestampMs => &mut self.lyrics_timestamp_ms,
            LimitKind::PlaylistBytes => &mut self.playlist_bytes,
            LimitKind::PlaylistEntries => &mut self.playlist_entries,
            LimitKind::PlaylistLineBytes => &mut self.playlist_line_bytes,
        }
    }

    /// These limits with `kind` set to `value`.
    ///
    /// # Errors
    ///
    /// Returns [`LimitError::AboveCeiling`] when `value` is above the
    /// limit's ceiling. Any value at or below it is accepted, so a limit may
    /// always be lowered.
    pub const fn with_override(mut self, kind: LimitKind, value: u64) -> Result<Self, LimitError> {
        let ceiling = kind.ceiling();
        if value > ceiling {
            return Err(LimitError::AboveCeiling {
                limit: kind,
                value,
                ceiling,
            });
        }
        *self.slot(kind) = value;
        Ok(self)
    }

    /// Checks a count, length or size the input declared or reached against
    /// one limit.
    ///
    /// # Errors
    ///
    /// Returns [`ParseFault::LimitExceeded`] when `value` is above the
    /// limit; a value equal to the limit is allowed.
    pub const fn check(&self, kind: LimitKind, value: u64, offset: u64) -> Result<(), ParseFault> {
        let max = self.get(kind);
        if value > max {
            return Err(ParseFault::LimitExceeded {
                limit: kind,
                value,
                max,
                offset,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    // Qodana does not expand `proptest!` or resolve `prop_oneof!` through `prelude::*`.
    use proptest::prop_oneof;
    use proptest::test_runner::{Config, TestRunner};

    /// The defaults and ceilings this package must ship, written out from
    /// the limits table and the requirement rows, independently of the code.
    /// The match is exhaustive, so a new limit cannot be added without
    /// deciding both values here.
    #[expect(
        clippy::match_same_arms,
        reason = "one row per limit, as the limits table lists them"
    )]
    const fn expected(kind: LimitKind) -> (u64, u64) {
        match kind {
            // SEC-MED-005 states both depths outright.
            LimitKind::ContainerDepth => (32, 32),
            LimitKind::EmbeddedFrameDepth => (4, 4),
            LimitKind::Children => (65_536, 262_144),
            LimitKind::IndexEntries => (1_000_000, 4_000_000),
            LimitKind::TagFields => (4_096, 16_384),
            LimitKind::ShortText => (4_096, 16_384),
            LimitKind::LongText => (65_536, 262_144),
            // SEC-MED-045 states 32 MiB encoded for every artwork image,
            // embedded pictures included, so neither a picture nor what a
            // compressed one inflates to (SEC-MED-009) may be raised.
            LimitKind::PictureBytes => (33_554_432, 33_554_432),
            LimitKind::Pictures => (16, 64),
            // 64 MiB, raisable to 256 MiB.
            LimitKind::Id3v2TagBytes => (67_108_864, 268_435_456),
            LimitKind::InflatedPicture => (33_554_432, 33_554_432),
            // 1 MiB, raisable to 4 MiB.
            LimitKind::InflatedCodecPrivate => (1_048_576, 4_194_304),
            // 4 MiB, raisable to 16 MiB.
            LimitKind::InflatedHeader => (4_194_304, 16_777_216),
            // SEC-MED-010 states 16 MiB and 256 MiB outright.
            LimitKind::ReadBytes => (16_777_216, 16_777_216),
            LimitKind::FileBytes => (268_435_456, 268_435_456),
            // SEC-API-090 (256 KiB, 10,000 lines) is stricter than
            // SEC-MED-049 (1 MiB, 20,000 lines); SEC-MED-049 states 4 KiB
            // per line and 24 hours.
            LimitKind::LyricsBytes => (262_144, 262_144),
            LimitKind::LyricsLines => (10_000, 10_000),
            LimitKind::LyricsLineBytes => (4_096, 4_096),
            LimitKind::LyricsTimestampMs => (86_400_000, 86_400_000),
            // 16 MiB, raisable to 64 MiB.
            LimitKind::PlaylistBytes => (16_777_216, 67_108_864),
            LimitKind::PlaylistEntries => (100_000, 400_000),
            LimitKind::PlaylistLineBytes => (8_192, 32_768),
        }
    }

    /// Every limit as `(kind, value)` pairs, read through [`Limits::get`].
    fn values(limits: &Limits) -> Vec<(LimitKind, u64)> {
        LimitKind::ALL
            .iter()
            .map(|&kind| (kind, limits.get(kind)))
            .collect()
    }

    /// The default value of every limit, from the independent table.
    fn expected_defaults() -> Vec<(LimitKind, u64)> {
        LimitKind::ALL
            .iter()
            .map(|&kind| (kind, expected(kind).0))
            .collect()
    }

    #[test]
    fn lists_every_limit_once_in_declaration_order() {
        assert_eq!(
            LimitKind::ALL,
            [
                LimitKind::ContainerDepth,
                LimitKind::EmbeddedFrameDepth,
                LimitKind::Children,
                LimitKind::IndexEntries,
                LimitKind::TagFields,
                LimitKind::ShortText,
                LimitKind::LongText,
                LimitKind::PictureBytes,
                LimitKind::Pictures,
                LimitKind::Id3v2TagBytes,
                LimitKind::InflatedPicture,
                LimitKind::InflatedCodecPrivate,
                LimitKind::InflatedHeader,
                LimitKind::ReadBytes,
                LimitKind::FileBytes,
                LimitKind::LyricsBytes,
                LimitKind::LyricsLines,
                LimitKind::LyricsLineBytes,
                LimitKind::LyricsTimestampMs,
                LimitKind::PlaylistBytes,
                LimitKind::PlaylistEntries,
                LimitKind::PlaylistLineBytes,
            ]
        );
    }

    /// Verifies: SEC-MED-005, SEC-MED-006, SEC-MED-010
    #[test]
    fn defaults_are_the_values_of_the_limits_table() {
        assert_eq!(values(&Limits::DEFAULT), expected_defaults());
    }

    /// Verifies: SEC-MED-005, SEC-MED-010
    #[test]
    fn ceilings_fix_the_stated_limits_and_allow_four_times_the_rest() {
        let ceilings: Vec<_> = LimitKind::ALL
            .iter()
            .map(|&kind| (kind, kind.ceiling()))
            .collect();
        let expected: Vec<_> = LimitKind::ALL
            .iter()
            .map(|&kind| (kind, expected(kind).1))
            .collect();
        assert_eq!(ceilings, expected);
    }

    /// Verifies: SEC-MED-005, SEC-MED-010
    #[test]
    fn refuses_an_override_above_the_ceiling() {
        for kind in LimitKind::ALL {
            let ceiling = expected(kind).1;
            // The label is built first. `kind` is `Copy`, and the message is
            // a separate value, so the assertion does not use `kind` twice.
            let rendered = format!("{kind:?}");
            assert_eq!(
                Limits::DEFAULT.with_override(kind, ceiling + 1),
                Err(LimitError::AboveCeiling {
                    limit: kind,
                    value: ceiling + 1,
                    ceiling,
                }),
                "{rendered}"
            );
            assert_eq!(
                Limits::DEFAULT.with_override(kind, u64::MAX),
                Err(LimitError::AboveCeiling {
                    limit: kind,
                    value: u64::MAX,
                    ceiling,
                }),
                "{rendered}"
            );
        }
    }

    #[test]
    fn accepts_an_override_at_the_ceiling_and_changes_only_that_limit() {
        for kind in LimitKind::ALL {
            let ceiling = expected(kind).1;
            let rendered = format!("{kind:?}");
            let raised = Limits::DEFAULT
                .with_override(kind, ceiling)
                .map(|limits| values(&limits));
            let mut wanted = expected_defaults();
            for entry in &mut wanted {
                if entry.0 == kind {
                    entry.1 = ceiling;
                }
            }
            assert_eq!(raised, Ok(wanted), "{rendered}");
        }
    }

    #[test]
    fn accepts_an_override_below_the_default() {
        for kind in LimitKind::ALL {
            let lowered = expected(kind).0 - 1;
            let rendered = format!("{kind:?}");
            let limits = Limits::DEFAULT.with_override(kind, lowered);
            assert_eq!(
                limits.map(|limits| limits.get(kind)),
                Ok(lowered),
                "{rendered}"
            );
        }
    }

    #[test]
    fn keeps_earlier_overrides_when_another_is_applied() {
        let limits = Limits::DEFAULT
            .with_override(LimitKind::ReadBytes, 4_096)
            .and_then(|limits| limits.with_override(LimitKind::Children, 0));
        let mut wanted = expected_defaults();
        for entry in &mut wanted {
            match entry.0 {
                LimitKind::ReadBytes => entry.1 = 4_096,
                LimitKind::Children => entry.1 = 0,
                _ => {}
            }
        }
        assert_eq!(limits.map(|limits| values(&limits)), Ok(wanted));
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn allows_a_value_at_each_limit_and_refuses_one_past_it() {
        for kind in LimitKind::ALL {
            let max = expected(kind).0;
            let rendered = format!("{kind:?}");
            assert_eq!(Limits::DEFAULT.check(kind, max, 9), Ok(()), "{rendered}");
            assert_eq!(
                Limits::DEFAULT.check(kind, max + 1, 9),
                Err(ParseFault::LimitExceeded {
                    limit: kind,
                    value: max + 1,
                    max,
                    offset: 9,
                }),
                "{rendered}"
            );
        }
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn checks_against_a_lowered_limit() {
        let limits = Limits::DEFAULT.with_override(LimitKind::TagFields, 0);
        assert_eq!(
            limits.map(|limits| limits.check(LimitKind::TagFields, 0, 4)),
            Ok(Ok(()))
        );
        assert_eq!(
            limits.map(|limits| limits.check(LimitKind::TagFields, 1, 4)),
            Ok(Err(ParseFault::LimitExceeded {
                limit: LimitKind::TagFields,
                value: 1,
                max: 0,
                offset: 4,
            }))
        );
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn refuses_exactly_the_values_above_the_limit() {
        TestRunner::new(Config::default())
            .run(
                &(
                    0..LimitKind::ALL.len(),
                    prop_oneof![any::<u64>(), 0_u64..300_000_000],
                    any::<u64>(),
                ),
                |(index, value, offset)| {
                    let kind = LimitKind::ALL[index];
                    let max = expected(kind).0;
                    let wanted = if value > max {
                        Err(ParseFault::LimitExceeded {
                            limit: kind,
                            value,
                            max,
                            offset,
                        })
                    } else {
                        Ok(())
                    };
                    prop_assert_eq!(Limits::DEFAULT.check(kind, value, offset), wanted);
                    Ok(())
                },
            )
            .unwrap();
    }

    /// Verifies: SEC-MED-010
    #[test]
    fn never_holds_a_limit_above_its_ceiling() {
        TestRunner::new(Config::default())
            .run(
                &proptest::collection::vec((0..LimitKind::ALL.len(), any::<u64>()), 0..8),
                |overrides| {
                    let mut limits = Limits::DEFAULT;
                    for (index, value) in overrides {
                        let kind = LimitKind::ALL[index];
                        if let Ok(changed) = limits.with_override(kind, value) {
                            limits = changed;
                        }
                    }
                    for kind in LimitKind::ALL {
                        prop_assert!(limits.get(kind) <= expected(kind).1, "{:?}", kind);
                    }
                    Ok(())
                },
            )
            .unwrap();
    }
}
