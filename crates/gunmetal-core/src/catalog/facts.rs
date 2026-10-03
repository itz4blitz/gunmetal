//! What a probe of one file returns (WP-052), apart from its raw tag
//! blocks.
//!
//! The raw tag blocks are the container parsers' own types, which are built
//! in the same wave as this module; the probe adds them to [`FileFacts`]
//! when it is built (WP-052).

use super::artwork::ArtworkRef;
use super::error::CatalogError;
use super::lyrics::LyricsSource;
use super::playback::Trim;
use super::tech::TechInfo;

/// A range of octets in a file: from `start` up to, but not including,
/// `end`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ByteRange {
    start: u64,
    end: u64,
}

impl ByteRange {
    /// The octets from `start` up to `end`. An empty range is allowed.
    ///
    /// # Errors
    ///
    /// [`CatalogError::BackwardsRange`] when `end` is before `start`.
    pub fn new(start: u64, end: u64) -> Result<Self, CatalogError> {
        if end < start {
            return Err(CatalogError::BackwardsRange { start, end });
        }
        Ok(Self { start, end })
    }

    /// The first octet's offset.
    #[must_use]
    pub const fn start(self) -> u64 {
        self.start
    }

    /// The offset just past the last octet.
    #[must_use]
    pub const fn end(self) -> u64 {
        self.end
    }
}

/// What content identity is computed from (WP-077).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdentityInputs {
    /// The MD5 of the decoded audio a FLAC file states, when it is set.
    pub audio_md5: Option<[u8; 16]>,
    /// The octets of audio, without any `ID3v2`, APE or `ID3v1` tag, which
    /// the worker hashes (WP-079).
    pub audio_window: ByteRange,
}

/// The facts a probe reads from one file's headers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileFacts {
    /// Codec, container and format.
    pub tech: TechInfo,
    /// Encoder delay and padding from the stream's headers.
    pub trim: Option<Trim>,
    /// Pictures the file holds or names.
    pub artwork: Vec<ArtworkRef>,
    /// Lyrics the file holds.
    pub lyrics: Vec<LyricsSource>,
    /// What content identity is computed from.
    pub identity: IdentityInputs,
    /// The version of the parser that read the file, so a newer parser can
    /// tell which files to read again.
    pub parser_version: u16,
    /// Octets read from the file (LIB-019 promises header-only reads).
    pub bytes_read: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_a_range() {
        let range = ByteRange::new(10, 4_096).unwrap();
        assert_eq!((range.start(), range.end()), (10, 4_096));
    }

    #[test]
    fn keeps_an_empty_range() {
        let range = ByteRange::new(4_096, 4_096).unwrap();
        assert_eq!((range.start(), range.end()), (4_096, 4_096));
    }

    #[test]
    fn refuses_a_range_that_ends_before_it_starts() {
        assert_eq!(
            ByteRange::new(4_097, 4_096),
            Err(CatalogError::BackwardsRange {
                start: 4_097,
                end: 4_096,
            })
        );
    }
}
