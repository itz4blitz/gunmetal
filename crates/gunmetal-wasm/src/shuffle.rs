//! Seeded shuffle orders for the From lane (MUS-126).
//!
//! [`shuffle_order`] is one call of [`shuffle::order`] with its types
//! converted, so every device given the same seed and the same items gets
//! the same permutation. The browser calls it as `shuffleOrder`.

use gunmetal_core::shuffle;
use gunmetal_core::time::{TimeError, Timestamp};
use serde::{Deserialize, Serialize};
use tsify::Tsify;

/// How the From lane is ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub enum Mode {
    /// Source order, from the current item onwards.
    Off,
    /// Durstenfeld's Fisher–Yates, driven by `SplitMix64`.
    Random,
    /// The same artist or album never bunches when another choice exists;
    /// recently played artists and albums come later.
    SpreadOut,
}

/// One From-lane item, named only by the artist and album keys spread-out
/// needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub struct ShuffleItem {
    /// The caller-chosen artist grouping key.
    pub artist: u64,
    /// The caller-chosen album grouping key.
    pub album: u64,
}

/// A last-played time of one artist or album, as milliseconds since the
/// Unix epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub struct RecentPlay {
    /// The grouping key.
    pub key: u64,
    /// When it was last played, in milliseconds since the Unix epoch.
    pub at: i64,
}

/// Last-played times of artists and albums, used only by `SpreadOut`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub struct RecentPlays {
    /// Artists, each with the later time kept when the same key is listed twice.
    pub artists: Vec<RecentPlay>,
    /// Albums, each with the later time kept when the same key is listed twice.
    pub albums: Vec<RecentPlay>,
}

/// Everything [`shuffle_order`] needs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub struct ShuffleRequest {
    /// The From-lane items, in source order.
    pub items: Vec<ShuffleItem>,
    /// How to order them.
    pub mode: Mode,
    /// The seed; the same seed and items yield the same order on every device.
    pub seed: u64,
    /// Last-played times, used only by `SpreadOut`.
    pub recent: RecentPlays,
}

/// The permutation [`shuffle_order`] returns: indices into the request's
/// items.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Tsify)]
pub struct ShuffleOrder {
    /// Each index is into the request's `items`.
    pub indices: Vec<u32>,
}

/// What reading a shuffle request gave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Tsify)]
pub enum ShuffleOutcome {
    /// The permutation.
    Order(ShuffleOrder),
    /// A last-played time lay outside the years the core accepts.
    UnusableTime,
}

/// The TypeScript declarations of this module's mirror types, in the order
/// they are declared above.
pub const DECLARATIONS: [&str; 7] = [
    Mode::DECL,
    ShuffleItem::DECL,
    RecentPlay::DECL,
    RecentPlays::DECL,
    ShuffleRequest::DECL,
    ShuffleOrder::DECL,
    ShuffleOutcome::DECL,
];

impl From<shuffle::Mode> for Mode {
    fn from(value: shuffle::Mode) -> Self {
        match value {
            shuffle::Mode::Off => Self::Off,
            shuffle::Mode::Random => Self::Random,
            shuffle::Mode::SpreadOut => Self::SpreadOut,
        }
    }
}

impl From<Mode> for shuffle::Mode {
    fn from(value: Mode) -> Self {
        match value {
            Mode::Off => Self::Off,
            Mode::Random => Self::Random,
            Mode::SpreadOut => Self::SpreadOut,
        }
    }
}

impl From<ShuffleItem> for shuffle::ShuffleItem {
    fn from(value: ShuffleItem) -> Self {
        Self::new(
            shuffle::ArtistKey::new(value.artist),
            shuffle::AlbumKey::new(value.album),
        )
    }
}

impl From<&shuffle::ShuffleItem> for ShuffleItem {
    fn from(value: &shuffle::ShuffleItem) -> Self {
        Self {
            artist: value.artist().get(),
            album: value.album().get(),
        }
    }
}

fn index_u32(index: usize) -> u32 {
    u32::try_from(index).unwrap_or(u32::MAX)
}

fn recent_plays(recent: &RecentPlays) -> Result<shuffle::RecentPlays, TimeError> {
    let mut plays = shuffle::RecentPlays::empty();
    for play in &recent.artists {
        plays.record_artist(
            shuffle::ArtistKey::new(play.key),
            Timestamp::from_millis(play.at)?,
        );
    }
    for play in &recent.albums {
        plays.record_album(
            shuffle::AlbumKey::new(play.key),
            Timestamp::from_millis(play.at)?,
        );
    }
    Ok(plays)
}

/// The permutation of `request.items` under `request.mode`.
///
/// Verifies: SEC-CLI-021
#[must_use]
pub fn shuffle_order(request: ShuffleRequest) -> ShuffleOutcome {
    let ShuffleRequest {
        items,
        mode,
        seed,
        recent,
    } = request;
    let Ok(recent) = recent_plays(&recent) else {
        return ShuffleOutcome::UnusableTime;
    };
    let core_items: Vec<shuffle::ShuffleItem> = items.into_iter().map(Into::into).collect();
    let indices = shuffle::order(&core_items, mode.into(), seed, &recent)
        .into_iter()
        .map(index_u32)
        .collect();
    ShuffleOutcome::Order(ShuffleOrder { indices })
}

crate::export::export! {
    /// The browser's `shuffleOrder`: [`shuffle_order`], with its types converted.
    "shuffleOrder": fn shuffle_order_export = shuffle_order(; request: ShuffleRequest) -> ShuffleOutcome
}

#[cfg(test)]
mod tests {
    use gunmetal_core::shuffle;
    use gunmetal_core::time::Timestamp;

    use super::{
        DECLARATIONS, Mode, RecentPlay, RecentPlays, ShuffleItem, ShuffleOrder, ShuffleOutcome,
        ShuffleRequest, shuffle_order,
    };

    fn order(outcome: ShuffleOutcome) -> Option<Vec<u32>> {
        match outcome {
            ShuffleOutcome::Order(ShuffleOrder { indices }) => Some(indices),
            ShuffleOutcome::UnusableTime => None,
        }
    }

    fn item(artist: u64, album: u64) -> ShuffleItem {
        ShuffleItem { artist, album }
    }

    fn empty_recent() -> RecentPlays {
        RecentPlays {
            artists: Vec::new(),
            albums: Vec::new(),
        }
    }

    #[test]
    fn each_mode_converts_to_its_mirror_and_back() {
        assert_eq!(Mode::from(shuffle::Mode::Off), Mode::Off);
        assert_eq!(shuffle::Mode::from(Mode::Off), shuffle::Mode::Off);
        assert_eq!(Mode::from(shuffle::Mode::Random), Mode::Random);
        assert_eq!(shuffle::Mode::from(Mode::Random), shuffle::Mode::Random);
        assert_eq!(Mode::from(shuffle::Mode::SpreadOut), Mode::SpreadOut);
        assert_eq!(
            shuffle::Mode::from(Mode::SpreadOut),
            shuffle::Mode::SpreadOut
        );
    }

    #[test]
    fn an_item_converts_to_the_core_through_both_keys_and_back() {
        let mirrored = item(7, 9);
        let core = shuffle::ShuffleItem::from(mirrored);
        assert_eq!(core.artist().get(), 7);
        assert_eq!(core.album().get(), 9);
        assert_eq!(ShuffleItem::from(&core), mirrored);
    }

    /// Shuffle off is source order: 0, 1, 2, matching the core.
    #[test]
    fn shuffle_off_is_source_order() {
        let request = ShuffleRequest {
            items: vec![item(1, 1), item(2, 2), item(3, 3)],
            mode: Mode::Off,
            seed: 0,
            recent: empty_recent(),
        };
        assert_eq!(
            shuffle_order(request),
            ShuffleOutcome::Order(ShuffleOrder {
                indices: vec![0, 1, 2],
            })
        );
        let core_items = [
            shuffle::ShuffleItem::new(shuffle::ArtistKey::new(1), shuffle::AlbumKey::new(1)),
            shuffle::ShuffleItem::new(shuffle::ArtistKey::new(2), shuffle::AlbumKey::new(2)),
            shuffle::ShuffleItem::new(shuffle::ArtistKey::new(3), shuffle::AlbumKey::new(3)),
        ];
        assert_eq!(
            shuffle::order(
                &core_items,
                shuffle::Mode::Off,
                0,
                &shuffle::RecentPlays::empty()
            ),
            vec![0, 1, 2]
        );
    }

    /// The same seed and items yield the same random permutation as the core.
    #[test]
    fn the_same_seed_matches_the_core_s_random_order() {
        let items = vec![item(1, 1), item(2, 2), item(3, 3), item(4, 4)];
        let request = ShuffleRequest {
            items: items.clone(),
            mode: Mode::Random,
            seed: 0x0123_4567_89AB_CDEF,
            recent: empty_recent(),
        };
        let indices = order(shuffle_order(request)).unwrap();
        let core_items: Vec<_> = items
            .iter()
            .map(|item| shuffle::ShuffleItem::from(*item))
            .collect();
        let expected = shuffle::order(
            &core_items,
            shuffle::Mode::Random,
            0x0123_4567_89AB_CDEF,
            &shuffle::RecentPlays::empty(),
        );
        assert_eq!(
            indices,
            expected
                .into_iter()
                .map(|index| u32::try_from(index).expect("four items"))
                .collect::<Vec<_>>()
        );
        assert_ne!(indices, vec![0, 1, 2, 3]);
    }

    #[test]
    fn spread_out_accepts_artist_and_album_recency() {
        let request = ShuffleRequest {
            items: vec![item(1, 1), item(2, 2)],
            mode: Mode::SpreadOut,
            seed: 7,
            recent: RecentPlays {
                artists: vec![RecentPlay { key: 1, at: 0 }],
                albums: vec![RecentPlay { key: 2, at: 1 }],
            },
        };
        let indices = order(shuffle_order(request)).unwrap();
        assert_eq!(indices.len(), 2);
    }

    /// A last-played time the core refuses is a typed conversion error.
    ///
    /// Verifies: SEC-CLI-021
    #[test]
    fn a_time_outside_the_core_s_years_is_unusable() {
        let request = ShuffleRequest {
            items: vec![item(1, 1)],
            mode: Mode::SpreadOut,
            seed: 1,
            recent: RecentPlays {
                artists: vec![RecentPlay {
                    key: 1,
                    at: Timestamp::MIN.millis() - 1,
                }],
                albums: Vec::new(),
            },
        };
        assert_eq!(order(shuffle_order(request)), None);
        let albums = ShuffleRequest {
            items: vec![item(1, 1)],
            mode: Mode::SpreadOut,
            seed: 1,
            recent: RecentPlays {
                artists: Vec::new(),
                albums: vec![RecentPlay {
                    key: 1,
                    at: Timestamp::MAX.millis() + 1,
                }],
            },
        };
        assert_eq!(order(shuffle_order(albums)), None);
    }

    #[test]
    fn an_index_that_does_not_fit_in_u32_saturates() {
        assert_eq!(super::index_u32(0), 0);
        assert_eq!(super::index_u32(3), 3);
        assert_eq!(super::index_u32(u32::MAX as usize), u32::MAX);
        assert_eq!(super::index_u32(usize::MAX), u32::MAX);
    }

    #[test]
    fn the_declarations_name_every_mirror() {
        assert_eq!(DECLARATIONS.len(), 7);
        assert!(DECLARATIONS[0].contains("export type Mode"));
        assert!(DECLARATIONS[5].contains("export interface ShuffleOrder"));
    }
}
