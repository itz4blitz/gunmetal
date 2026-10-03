//! Every canonical tag the mappers fill (WP-049 to WP-051).
//!
//! A mapper reads one tag format into a [`TrackTags`]; the track record
//! derivation (WP-075) merges them by a fixed precedence. Text is decoded,
//! stripped of controls and capped by the mapper before it lands here
//! (SEC-MED-013), and every number, date and identifier is already a typed
//! value (SEC-MED-014). Raw tag fields are kept apart, in the tag field
//! store (LIB-059).

use crate::values::{Isrc, Mbid, PartialDate};

use super::credit::Credit;
use super::lyrics::TagLyrics;
use super::playback::{GainTags, Trim};
use super::position::TrackPosition;
use super::release::{Advisory, ReleaseType};

/// The `MusicBrainz` identifiers a file is tagged with (API-CAT-03).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct MbIds {
    /// The recording.
    pub recording: Option<Mbid>,
    /// The track on the release.
    pub track: Option<Mbid>,
    /// The release.
    pub release: Option<Mbid>,
    /// The release group.
    pub release_group: Option<Mbid>,
    /// The recording's artists, in the order tagged.
    pub artists: Vec<Mbid>,
    /// The release's artists, in the order tagged.
    pub album_artists: Vec<Mbid>,
}

/// The canonical tags of one file. Fields the feature map makes
/// multi-valued, such as genres (MUS-017), are lists in the order tagged.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TrackTags {
    /// The title.
    pub title: Option<String>,
    /// The title to sort by.
    pub title_sort: Option<String>,
    /// The recording's artists as tagged, before any splitting (WP-053).
    pub artist: Vec<String>,
    /// The recording's artists to sort by.
    pub artist_sort: Vec<String>,
    /// The release's artists as tagged.
    pub album_artist: Vec<String>,
    /// The release's artists to sort by.
    pub album_artist_sort: Vec<String>,
    /// The release's title.
    pub album: Option<String>,
    /// The release's title to sort by.
    pub album_sort: Option<String>,
    /// The track and disc numbers and totals.
    pub position: TrackPosition,
    /// The disc's own title.
    pub disc_subtitle: Option<String>,
    /// The release date.
    pub date: Option<PartialDate>,
    /// The original release date.
    pub original_date: Option<PartialDate>,
    /// Genres (MUS-017).
    pub genres: Vec<String>,
    /// Moods (MUS-019, shown from R1.1).
    pub moods: Vec<String>,
    /// Styles (MUS-019, shown from R1.1).
    pub styles: Vec<String>,
    /// Record labels (MUS-019, shown from R1.1).
    pub labels: Vec<String>,
    /// Groupings (MUS-019, shown from R1.1).
    pub grouping: Vec<String>,
    /// Every other credited name with its role: composers, conductors,
    /// lyricists, remixers, producers, performers and the like.
    pub credits: Vec<Credit>,
    /// Whether the tags mark the release a compilation.
    pub compilation: Option<bool>,
    /// The release's type.
    pub release_type: Option<ReleaseType>,
    /// The content advisory.
    pub advisory: Option<Advisory>,
    /// International Standard Recording Codes.
    pub isrc: Vec<Isrc>,
    /// `MusicBrainz` identifiers.
    pub musicbrainz: MbIds,
    /// `ReplayGain` and R128 gains.
    pub gain: GainTags,
    /// Encoder delay and padding from a tag, such as MP4's `iTunSMPB`.
    pub trim: Option<Trim>,
    /// Lyrics the tags hold.
    pub lyrics: Vec<TagLyrics>,
}
