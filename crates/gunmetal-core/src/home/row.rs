//! A Home row: which one it is, what it holds and why (DIS-001, DIS-004;
//! API-HOME-04).
//!
//! A row is asked for with a [`RowSpec`] and comes back as a [`Row`]: its
//! source again, the reason it shows what it shows, and either its cards or
//! the empty state the client draws in their place. A card names an item
//! only by its public identifier, so a row holds no title, no path and no
//! person's name.

use crate::catalog::{AlbumId, TrackId};
use crate::id::PublicId;

/// A built-in row source: one of the rows R1 ships.
///
/// The value is the row's stable identity. R1 stores no layout, so it needs
/// no number yet; the arrangeable Home gives each a code when it stores
/// layouts (WP-154, DIS-015).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RowSource {
    /// The albums and playlists the person stopped part-way through
    /// (MUS-050, DIS-020).
    ContinueListening,
    /// The tracks the person played last (DIS-021).
    RecentlyPlayed,
    /// The library's newest arrivals, grouped by album (DIS-035, DIS-036,
    /// MUS-059).
    RecentlyAdded,
    /// The tracks the person loves, the latest love first (MUS-149,
    /// DIS-046).
    LovedSongs,
}

/// One row of a layout: where its cards come from and how many it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RowSpec {
    /// Where the row's cards come from.
    pub source: RowSource,
    /// The most cards the row holds. Rows are capped until "See all"
    /// arrives with the arrangeable Home (DIS-009, R1.2).
    pub limit: usize,
}

/// How many cards each row of the default layout holds.
pub const DEFAULT_ROW_LIMIT: usize = 20;

/// The default Home: the four built-in rows, in the order the Home surface
/// lists them (SUR-020; DIS-004). Nobody arranges it in R1 (DIS-003 is
/// R1.2), so every profile has this layout.
pub const DEFAULT_LAYOUT: &[RowSpec] = &[
    RowSpec {
        source: RowSource::ContinueListening,
        limit: DEFAULT_ROW_LIMIT,
    },
    RowSpec {
        source: RowSource::RecentlyPlayed,
        limit: DEFAULT_ROW_LIMIT,
    },
    RowSpec {
        source: RowSource::RecentlyAdded,
        limit: DEFAULT_ROW_LIMIT,
    },
    RowSpec {
        source: RowSource::LovedSongs,
        limit: DEFAULT_ROW_LIMIT,
    },
];

/// Why a row shows what it shows, as a code the client turns into words
/// (API-HOME-04).
///
/// A reason is one of a closed set with no fields, so it cannot name a
/// person, an item or a search, whoever's Home it is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reason {
    /// The person stopped part-way through these.
    PartWayThrough,
    /// The person played these most recently.
    PlayedLately,
    /// These are the library's newest arrivals.
    AddedLately,
    /// The person loves these.
    Loved,
}

/// One card of a row: the item it opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Card {
    /// A track.
    Track(TrackId),
    /// An album.
    Album(AlbumId),
    /// An album the library already held that has gained tracks (DIS-036).
    AlbumWithNewTracks {
        /// The album.
        album: AlbumId,
        /// How many of its tracks arrived last.
        new_tracks: u32,
    },
    /// A playlist, by its public identifier.
    Playlist(PublicId),
}

/// Why a row has no cards: the designed empty states of Home (DIS-004;
/// design language, "The states").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EmptyState {
    /// The synced library holds no track. Home shows what is happening in
    /// place of rows: the first scan's progress, or that no library has
    /// been shared with this person yet. The client knows which.
    EmptyLibrary,
    /// The library has music, but nothing belongs in this row yet. The row
    /// stays hidden.
    NothingYet,
}

/// What a row holds: cards, or the reason it has none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowContent {
    /// The cards, in order. Never empty.
    Cards(Vec<Card>),
    /// No cards, and why.
    Empty(EmptyState),
}

impl RowContent {
    /// The cards, of which an empty row has none.
    #[must_use]
    pub fn cards(&self) -> &[Card] {
        match self {
            Self::Cards(cards) => cards,
            Self::Empty(_) => &[],
        }
    }
}

/// One evaluated row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// Which row it is.
    pub source: RowSource,
    /// Why it shows what it shows.
    pub reason: Reason,
    /// Its cards, or the empty state to draw.
    pub content: RowContent,
}

#[cfg(test)]
mod tests {
    use super::super::fixtures::{album_id, playlist, track_id};
    use super::*;

    #[test]
    fn the_default_home_is_the_four_built_in_rows_with_continue_listening_first() {
        assert_eq!(
            DEFAULT_LAYOUT,
            [
                RowSpec {
                    source: RowSource::ContinueListening,
                    limit: 20,
                },
                RowSpec {
                    source: RowSource::RecentlyPlayed,
                    limit: 20,
                },
                RowSpec {
                    source: RowSource::RecentlyAdded,
                    limit: 20,
                },
                RowSpec {
                    source: RowSource::LovedSongs,
                    limit: 20,
                },
            ]
        );
        assert_eq!(DEFAULT_ROW_LIMIT, 20);
    }

    #[test]
    fn a_rows_cards_are_its_content_and_an_empty_row_has_none() {
        let cards = vec![
            Card::Track(track_id(1)),
            Card::Album(album_id(2)),
            Card::AlbumWithNewTracks {
                album: album_id(3),
                new_tracks: 4,
            },
            Card::Playlist(playlist(5)),
        ];
        assert_eq!(RowContent::Cards(cards.clone()).cards(), cards);
        let none: &[Card] = &[];
        assert_eq!(RowContent::Empty(EmptyState::NothingYet).cards(), none);
        assert_eq!(RowContent::Empty(EmptyState::EmptyLibrary).cards(), none);
    }
}
