//! The synced-library records: what each device's copy of the library holds
//! for a track, an album and an artist (API-CAT-01 to API-CAT-09).
//!
//! The client reads its artist and album pages, sort orders, genre browse
//! and quality badges from these records (MUS-051, MUS-054, MUS-060,
//! LIB-146). Every record names its library, so the visibility predicate
//! can filter it (TB4, TM-T15). [`CatalogField`](super::field::CatalogField)
//! gives each field a stable code for search and, from R1.1, the rule
//! format. Paths relative to a library root (API-CAT-10) join the track
//! record with the path rules' type (WP-024). The release-group record is
//! R1.1 (WP-146).

use crate::time::Timestamp;
use crate::values::{Duration, Isrc, Mbid, PartialDate};

use super::coded::coded;
use super::ids::{AlbumId, ArtistId, LibraryId, TrackId};
use super::kind::ItemKind;
use super::lyrics::LyricsSource;
use super::playback::{GainTags, Trim};
use super::position::TrackPosition;
use super::release::{Advisory, ReleaseType};
use super::tech::TechInfo;

coded! {
    /// Whether an item can be played now (API-CAT-09). The client adds
    /// "cannot decode here" from its own capability probe.
    Availability: u8 {
        /// The file is there and plays.
        Playable = 1,
        /// The drive holding the file is offline.
        Offline = 2,
        /// The file is there but damaged.
        Damaged = 3,
        /// The file is gone.
        Missing = 4,
    }
}

/// A track as every device's copy of the library holds it.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackRecord {
    /// The track's identifier.
    pub id: TrackId,
    /// What kind of item it is (LAT-001).
    pub kind: ItemKind,
    /// The library it is in.
    pub library: LibraryId,
    /// The title.
    pub title: String,
    /// The title to sort by.
    pub title_sort: Option<String>,
    /// The artist credit as tagged.
    pub artist_credit: String,
    /// The credited artists, in credit order.
    pub artists: Vec<ArtistId>,
    /// The album, when grouped into one.
    pub album: Option<AlbumId>,
    /// Track and disc numbers and totals.
    pub position: TrackPosition,
    /// The disc's own title.
    pub disc_subtitle: Option<String>,
    /// The release date.
    pub date: Option<PartialDate>,
    /// The original release date.
    pub original_date: Option<PartialDate>,
    /// Genres (MUS-017, MUS-060).
    pub genres: Vec<String>,
    /// Moods (MUS-019, shown from R1.1).
    pub moods: Vec<String>,
    /// Styles (MUS-019, shown from R1.1).
    pub styles: Vec<String>,
    /// Record labels (MUS-019, shown from R1.1).
    pub labels: Vec<String>,
    /// Groupings (MUS-019, shown from R1.1).
    pub grouping: Vec<String>,
    /// The content advisory.
    pub advisory: Option<Advisory>,
    /// International Standard Recording Codes.
    pub isrc: Vec<Isrc>,
    /// The recording's `MusicBrainz` identifier.
    pub recording_mbid: Option<Mbid>,
    /// Codec, container and format (MUS-021, LIB-146).
    pub tech: TechInfo,
    /// Track and album gains (API-CAT-05).
    pub gain: GainTags,
    /// Encoder delay and padding (API-CAT-05).
    pub trim: Option<Trim>,
    /// Where the lyrics came from, when there are any (API-CAT-06).
    pub lyrics: Option<LyricsSource>,
    /// Whether it can be played now.
    pub availability: Availability,
    /// When it was added to the library.
    pub added: Timestamp,
}

/// An album as every device's copy of the library holds it (MUS-054).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AlbumRecord {
    /// The album's identifier.
    pub id: AlbumId,
    /// The library it is in.
    pub library: LibraryId,
    /// The title.
    pub title: String,
    /// The title to sort by.
    pub title_sort: Option<String>,
    /// The album artist credit as tagged.
    pub artist_credit: String,
    /// The credited album artists, in credit order.
    pub artists: Vec<ArtistId>,
    /// The release date.
    pub date: Option<PartialDate>,
    /// The original release date.
    pub original_date: Option<PartialDate>,
    /// The release's type.
    pub release_type: ReleaseType,
    /// Whether it is a compilation.
    pub compilation: bool,
    /// Genres rolled up from its tracks (MUS-017).
    pub genres: Vec<String>,
    /// Record labels (MUS-019, shown from R1.1).
    pub labels: Vec<String>,
    /// How many tracks it has in the library.
    pub track_count: u32,
    /// How many discs it has in the library.
    pub disc_count: u16,
    /// How long its tracks play together.
    pub duration: Duration,
    /// Whether it has artwork.
    pub has_artwork: bool,
    /// The release's `MusicBrainz` identifier.
    pub release_mbid: Option<Mbid>,
    /// The release group's `MusicBrainz` identifier.
    pub release_group_mbid: Option<Mbid>,
    /// When it was added to the library.
    pub added: Timestamp,
}

/// An artist as every device's copy of the library holds it (MUS-051).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArtistRecord {
    /// The artist's identifier.
    pub id: ArtistId,
    /// The library it is in.
    pub library: LibraryId,
    /// The name as credited.
    pub name: String,
    /// The name to sort by.
    pub name_sort: Option<String>,
    /// The artist's `MusicBrainz` identifier.
    pub mbid: Option<Mbid>,
    /// How many albums credit the artist.
    pub album_count: u32,
    /// How many tracks credit the artist.
    pub track_count: u32,
    /// Genres rolled up from the artist's tracks.
    pub genres: Vec<String>,
    /// Whether it has artwork.
    pub has_artwork: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn availabilities_have_these_codes() {
        let read: Vec<(Availability, u8)> =
            Availability::ALL.iter().map(|a| (*a, a.code())).collect();
        assert_eq!(
            read,
            [
                (Availability::Playable, 1),
                (Availability::Offline, 2),
                (Availability::Damaged, 3),
                (Availability::Missing, 4),
            ]
        );
        let read: Vec<Option<Availability>> = (0..=5).map(Availability::from_code).collect();
        let mut expected = vec![None];
        expected.extend(Availability::ALL.iter().copied().map(Some));
        expected.push(None);
        assert_eq!(read, expected);
    }
}
