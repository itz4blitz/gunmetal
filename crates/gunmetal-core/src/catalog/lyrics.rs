//! Where lyrics came from (API-CAT-06).
//!
//! Parsing lyrics text into timed lines under the lyrics caps is the
//! lyrics parser's job (WP-021); this module records where they were found
//! and how they are timed, which the details view shows.

use super::coded::coded;
use super::error::CatalogError;

coded! {
    /// Where a file's lyrics were found.
    LyricsOrigin: u8 {
        /// An `ID3v2` `USLT` frame.
        Id3Unsynced = 1,
        /// An `ID3v2` `SYLT` frame, which is always timed.
        Id3Synced = 2,
        /// A Vorbis comment, such as `LYRICS` or `UNSYNCEDLYRICS`.
        VorbisComment = 3,
        /// An MP4 `©lyr` item.
        Mp4Item = 4,
        /// An `APEv2` `Lyrics` item.
        ApeItem = 5,
        /// An `.lrc` file beside the audio file.
        LrcSidecar = 6,
    }
}

coded! {
    /// How lyrics are timed.
    LyricsTiming: u8 {
        /// Untimed text.
        Plain = 1,
        /// A time for each line.
        Line = 2,
        /// A time for each word (shown from R1.1, MUS-156).
        Word = 3,
    }
}

/// Where a file's lyrics came from and how they are timed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LyricsSource {
    origin: LyricsOrigin,
    timing: LyricsTiming,
}

impl LyricsSource {
    /// Lyrics found at `origin`, timed as `timing`.
    ///
    /// # Errors
    ///
    /// [`CatalogError::UntimedSyncedLyrics`] for plain lyrics from a
    /// `SYLT` frame, which holds only timed text.
    pub fn new(origin: LyricsOrigin, timing: LyricsTiming) -> Result<Self, CatalogError> {
        if matches!(
            (origin, timing),
            (LyricsOrigin::Id3Synced, LyricsTiming::Plain)
        ) {
            return Err(CatalogError::UntimedSyncedLyrics);
        }
        Ok(Self { origin, timing })
    }

    /// Where the lyrics were found.
    #[must_use]
    pub const fn origin(self) -> LyricsOrigin {
        self.origin
    }

    /// How they are timed.
    #[must_use]
    pub const fn timing(self) -> LyricsTiming {
        self.timing
    }
}

/// Lyrics a tag holds, as decoded text, with where they came from.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TagLyrics {
    /// Where they came from.
    pub source: LyricsSource,
    /// The text, decoded and capped by the mapper.
    pub text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origins_have_these_codes() {
        let origins: Vec<(LyricsOrigin, u8)> =
            LyricsOrigin::ALL.iter().map(|o| (*o, o.code())).collect();
        assert_eq!(
            origins,
            [
                (LyricsOrigin::Id3Unsynced, 1),
                (LyricsOrigin::Id3Synced, 2),
                (LyricsOrigin::VorbisComment, 3),
                (LyricsOrigin::Mp4Item, 4),
                (LyricsOrigin::ApeItem, 5),
                (LyricsOrigin::LrcSidecar, 6),
            ]
        );
        let read: Vec<Option<LyricsOrigin>> = (0..=7).map(LyricsOrigin::from_code).collect();
        let mut expected = vec![None];
        expected.extend(LyricsOrigin::ALL.iter().copied().map(Some));
        expected.push(None);
        assert_eq!(read, expected);
    }

    #[test]
    fn timings_have_these_codes() {
        let timings: Vec<(LyricsTiming, u8)> =
            LyricsTiming::ALL.iter().map(|t| (*t, t.code())).collect();
        assert_eq!(
            timings,
            [
                (LyricsTiming::Plain, 1),
                (LyricsTiming::Line, 2),
                (LyricsTiming::Word, 3),
            ]
        );
        let read: Vec<Option<LyricsTiming>> = (0..=4).map(LyricsTiming::from_code).collect();
        let mut expected = vec![None];
        expected.extend(LyricsTiming::ALL.iter().copied().map(Some));
        expected.push(None);
        assert_eq!(read, expected);
    }

    #[test]
    fn accepts_every_combination_but_plain_synced_lyrics() {
        let accepted: Vec<(LyricsOrigin, LyricsTiming)> = LyricsOrigin::ALL
            .iter()
            .flat_map(|origin| {
                LyricsTiming::ALL
                    .iter()
                    .map(move |timing| (*origin, *timing))
            })
            .filter_map(|(origin, timing)| LyricsSource::new(origin, timing).ok())
            .map(|source| (source.origin(), source.timing()))
            .collect();
        let mut expected = Vec::new();
        for origin in LyricsOrigin::ALL {
            for timing in LyricsTiming::ALL {
                if (*origin, *timing) != (LyricsOrigin::Id3Synced, LyricsTiming::Plain) {
                    expected.push((*origin, *timing));
                }
            }
        }
        assert_eq!(accepted.len(), 17);
        assert_eq!(accepted, expected);
    }

    #[test]
    fn refuses_plain_lyrics_from_a_sylt_frame() {
        assert_eq!(
            LyricsSource::new(LyricsOrigin::Id3Synced, LyricsTiming::Plain),
            Err(CatalogError::UntimedSyncedLyrics)
        );
    }
}
