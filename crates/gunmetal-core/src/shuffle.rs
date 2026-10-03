//! Seeded shuffle orders for the From lane (MUS-126).
//!
//! The generator is [`SplitMix64`], so the same seed yields the same order on
//! every device and every platform. Random mode is Durstenfeld's
//! Fisher–Yates. Spread-out mode never bunches the same artist or album when
//! another choice exists, and puts more recently played artists and albums
//! later. Turning shuffle off restores source order of the slice the caller
//! already cut from the current item onwards.
//!
//! This module holds no security control: it is a pure permutation of
//! indices the queue already holds.

use crate::time::Timestamp;

/// The [`SplitMix64`] golden-ratio increment (Steele, Lea, Flood; Vigna's
/// reference `splitmix64.c`).
const GOLDEN_GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;
/// First mix multiplier of [`SplitMix64`].
const MIX_1: u64 = 0xBF58_476D_1CE4_E5B9;
/// Second mix multiplier of [`SplitMix64`].
const MIX_2: u64 = 0x94D0_49BB_1331_11EB;

/// A 64-bit generator with a 64-bit state.
///
/// One output is produced by adding [`GOLDEN_GAMMA`] to the state, then
/// mixing the new state with the two multiplications and xorshifts of
/// Vigna's published C. The order of those steps is part of the algorithm:
/// the increment happens first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// A generator whose first output is the mix of `seed + GOLDEN_GAMMA`.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// The next 64-bit value in the sequence.
    #[must_use]
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(GOLDEN_GAMMA);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(MIX_1);
        z = (z ^ (z >> 27)).wrapping_mul(MIX_2);
        z ^ (z >> 31)
    }
}

/// How the From lane is ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Source order, from the current item onwards.
    Off,
    /// Durstenfeld's Fisher–Yates, driven by [`SplitMix64`].
    Random,
    /// The same artist or album never bunches when another choice exists;
    /// recently played artists and albums come later.
    SpreadOut,
}

/// Identity of an artist for spread-out grouping. The caller maps its own
/// artist IDs into this type; equal values are the same artist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArtistKey(u64);

impl ArtistKey {
    /// A grouping key with this integer.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// The integer the caller supplied.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Identity of an album for spread-out grouping. The caller maps its own
/// album IDs into this type; equal values are the same album.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AlbumKey(u64);

impl AlbumKey {
    /// A grouping key with this integer.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// The integer the caller supplied.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// One From-lane item, named only by the artist and album keys spread-out
/// needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShuffleItem {
    artist: ArtistKey,
    album: AlbumKey,
}

impl ShuffleItem {
    /// An item of this artist on this album.
    #[must_use]
    pub const fn new(artist: ArtistKey, album: AlbumKey) -> Self {
        Self { artist, album }
    }

    /// The artist grouping key.
    #[must_use]
    pub const fn artist(self) -> ArtistKey {
        self.artist
    }

    /// The album grouping key.
    #[must_use]
    pub const fn album(self) -> AlbumKey {
        self.album
    }
}

/// Last-played times of artists and albums, used only by [`Mode::SpreadOut`].
///
/// A later timestamp is more recent and is ordered later in the permutation.
/// Recording the same key twice keeps the later time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentPlays {
    artists: Vec<(ArtistKey, Timestamp)>,
    albums: Vec<(AlbumKey, Timestamp)>,
}

impl RecentPlays {
    /// No recent plays: every item is treated as never played.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            artists: Vec::new(),
            albums: Vec::new(),
        }
    }

    /// Notes that `artist` was last played at `at`. A later time replaces an
    /// earlier one; an earlier time does not.
    pub fn record_artist(&mut self, artist: ArtistKey, at: Timestamp) {
        record_latest(&mut self.artists, artist, at);
    }

    /// Notes that `album` was last played at `at`. A later time replaces an
    /// earlier one; an earlier time does not.
    pub fn record_album(&mut self, album: AlbumKey, at: Timestamp) {
        record_latest(&mut self.albums, album, at);
    }

    fn recency(&self, item: &ShuffleItem) -> Recency {
        self.artist_recency(item.artist())
            .max(self.album_recency(item.album()))
    }

    fn artist_recency(&self, artist: ArtistKey) -> Recency {
        lookup(&self.artists, &artist)
    }

    fn album_recency(&self, album: AlbumKey) -> Recency {
        lookup(&self.albums, &album)
    }
}

fn record_latest<K: PartialEq>(slots: &mut Vec<(K, Timestamp)>, key: K, at: Timestamp) {
    for (held, time) in slots.iter_mut() {
        if *held == key {
            if at > *time {
                *time = at;
            }
            return;
        }
    }
    slots.push((key, at));
}

fn lookup<K: PartialEq>(slots: &[(K, Timestamp)], key: &K) -> Recency {
    slots
        .iter()
        .find_map(|(held, time)| (held == key).then_some(Recency::At(*time)))
        .unwrap_or(Recency::Never)
}

/// How recently an item was played. Never-played sorts before any timestamp,
/// and an earlier timestamp before a later one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Recency {
    Never,
    At(Timestamp),
}

/// How well an item spreads from the previous one. Higher is a better fit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Fit {
    SameBoth,
    DifferentAlbum,
    DifferentArtist,
    DifferentBoth,
}

/// Permutation of `0..items.len()`: the play order of the From lane from the
/// current item onwards.
///
/// [`Mode::Off`] ignores `seed` and `recent` and returns source order.
/// [`Mode::Random`] is Fisher–Yates and ignores `recent`. [`Mode::SpreadOut`]
/// uses both.
///
/// ```
/// use gunmetal_core::shuffle::{
///     order, AlbumKey, ArtistKey, Mode, RecentPlays, ShuffleItem,
/// };
/// let items = [
///     ShuffleItem::new(ArtistKey::new(1), AlbumKey::new(10)),
///     ShuffleItem::new(ArtistKey::new(2), AlbumKey::new(20)),
/// ];
/// assert_eq!(
///     order(&items, Mode::Off, 0, &RecentPlays::empty()),
///     vec![0, 1]
/// );
/// ```
#[must_use]
pub fn order(items: &[ShuffleItem], mode: Mode, seed: u64, recent: &RecentPlays) -> Vec<usize> {
    match mode {
        Mode::Off => source_order(items.len()),
        Mode::Random => fisher_yates(items.len(), seed),
        Mode::SpreadOut => spread_out(items, seed, recent),
    }
}

fn source_order(len: usize) -> Vec<usize> {
    let mut order = Vec::new();
    for index in 0..len {
        order.push(index);
    }
    order
}

/// Durstenfeld: for `i` from `n-1` down to 1, swap `i` with a uniform index
/// in `0..=i`, each index taken as one [`SplitMix64`] output modulo `i + 1`.
fn fisher_yates(len: usize, seed: u64) -> Vec<usize> {
    let mut order = source_order(len);
    let mut rng = SplitMix64::new(seed);
    for i in (1..len).rev() {
        let j = index_below(&mut rng, i.saturating_add(1));
        order.swap(i, j);
    }
    order
}

fn spread_out(items: &[ShuffleItem], seed: u64, recent: &RecentPlays) -> Vec<usize> {
    let mut remaining = source_order(items.len());
    let mut rng = SplitMix64::new(seed);
    let mut result = Vec::new();
    while !remaining.is_empty() {
        let prev = result.last().copied().and_then(|index| items.get(index));
        let slot = pick_slot(items, &remaining, prev, recent, &mut rng);
        result.push(remaining.remove(slot));
    }
    result
}

/// Index into `remaining` of the next spread-out pick. `remaining` holds
/// source indices still unused. Out-of-range source indices are skipped.
/// No remaining candidate yields 0.
fn pick_slot(
    items: &[ShuffleItem],
    remaining: &[usize],
    prev: Option<&ShuffleItem>,
    recent: &RecentPlays,
    rng: &mut SplitMix64,
) -> usize {
    let mut best_fit = Fit::SameBoth;
    let mut best_recency = Recency::At(Timestamp::MAX);
    let mut candidates = Vec::new();
    for (slot, &index) in remaining.iter().enumerate() {
        let Some(item) = items.get(index) else {
            continue;
        };
        let this_fit = fit(prev, item);
        let this_recency = recent.recency(item);
        if this_fit > best_fit || (this_fit == best_fit && this_recency < best_recency) {
            best_fit = this_fit;
            best_recency = this_recency;
            candidates.clear();
            candidates.push(slot);
        } else if this_fit == best_fit && this_recency == best_recency {
            candidates.push(slot);
        }
    }
    candidates
        .get(index_below(rng, candidates.len()))
        .copied()
        .unwrap_or(0)
}

fn fit(prev: Option<&ShuffleItem>, item: &ShuffleItem) -> Fit {
    let Some(prev) = prev else {
        return Fit::DifferentBoth;
    };
    match (prev.artist() == item.artist(), prev.album() == item.album()) {
        (false, false) => Fit::DifferentBoth,
        (false, true) => Fit::DifferentArtist,
        (true, false) => Fit::DifferentAlbum,
        (true, true) => Fit::SameBoth,
    }
}

/// Uniform index in `0..bound`. `bound == 0` or `1` returns 0. One generator
/// output is consumed even when the bound is 1, so a later pick sees the
/// next value.
fn index_below(rng: &mut SplitMix64, bound: usize) -> usize {
    let raw = rng.next_u64();
    let n = u64::try_from(bound).unwrap_or(0);
    raw.checked_rem(n)
        .and_then(|remainder| usize::try_from(remainder).ok())
        .unwrap_or(0)
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use super::*;
    use proptest::collection::vec;
    use proptest::prelude::*;

    /// Rosetta Code, "Pseudo-random numbers/Splitmix64": seed 1234567,
    /// first five 64-bit outputs. Independent of this crate.
    const SPLITMIX64_SEED_1234567: [u64; 5] = [
        6_457_827_717_110_365_317,
        3_203_168_211_198_807_973,
        9_817_491_932_198_370_423,
        4_593_380_528_125_082_431,
        16_408_922_859_458_223_821,
    ];

    fn item(artist: u64, album: u64) -> ShuffleItem {
        ShuffleItem::new(ArtistKey::new(artist), AlbumKey::new(album))
    }

    fn ts(millis: i64) -> Timestamp {
        Timestamp::from_millis(millis).expect("a millisecond in year 0 to 9999")
    }

    fn empty() -> RecentPlays {
        RecentPlays::empty()
    }

    /// Durstenfeld's Fisher–Yates, written for the test from the algorithm
    /// and fed this module's [`SplitMix64`] outputs. It shares no code with
    /// [`fisher_yates`].
    fn reference_fisher_yates(len: usize, seed: u64) -> Vec<usize> {
        let mut rng = SplitMix64::new(seed);
        let mut order: Vec<usize> = (0..len).collect();
        let mut i = len;
        while i > 1 {
            i -= 1;
            let bound = u64::try_from(i + 1).expect("i + 1 fits in u64");
            let j = usize::try_from(rng.next_u64() % bound).expect("remainder is below bound");
            order.swap(i, j);
        }
        order
    }

    fn is_permutation(order: &[usize], len: usize) -> bool {
        let mut sorted = order.to_vec();
        sorted.sort_unstable();
        sorted == (0..len).collect::<Vec<_>>()
    }

    #[test]
    fn index_below_takes_the_remainder_of_one_output() {
        let mut rng = SplitMix64::new(1_234_567);
        assert_eq!(index_below(&mut rng, 3), 0);
        let mut rng = SplitMix64::new(1_234_567);
        assert_eq!(index_below(&mut rng, 2), 1);
        let mut rng = SplitMix64::new(1_234_567);
        assert_eq!(index_below(&mut rng, 1), 0);
        let mut rng = SplitMix64::new(1_234_567);
        assert_eq!(index_below(&mut rng, 0), 0);
    }

    #[test]
    fn splitmix64_matches_the_published_rosetta_code_vectors() {
        let mut rng = SplitMix64::new(1_234_567);
        let got: Vec<u64> = (0..5).map(|_| rng.next_u64()).collect();
        assert_eq!(got, SPLITMIX64_SEED_1234567);
    }

    #[test]
    fn splitmix64_mixes_the_incremented_state_not_the_seed() {
        // Vigna's splitmix64.c with seed 0: first output is the mix of
        // GOLDEN_GAMMA, not the mix of 0 (which is 0).
        let mut rng = SplitMix64::new(0);
        assert_eq!(rng.next_u64(), 16_294_208_416_658_607_535);
        assert_ne!(SplitMix64::new(0).next_u64(), 0);
    }

    #[test]
    fn splitmix64_the_same_seed_repeats_the_sequence() {
        let mut a = SplitMix64::new(42);
        let mut b = SplitMix64::new(42);
        let first: Vec<u64> = (0..8).map(|_| a.next_u64()).collect();
        let second: Vec<u64> = (0..8).map(|_| b.next_u64()).collect();
        assert_eq!(first, second);
        assert_ne!(SplitMix64::new(42), a);
    }

    #[test]
    fn keys_and_items_round_trip_the_integers() {
        let artist = ArtistKey::new(7);
        let album = AlbumKey::new(9);
        assert_eq!(artist.get(), 7);
        assert_eq!(album.get(), 9);
        let track = ShuffleItem::new(artist, album);
        assert_eq!(track.artist(), artist);
        assert_eq!(track.album(), album);
        assert_ne!(artist, ArtistKey::new(8));
        assert_ne!(album, AlbumKey::new(8));
    }

    #[test]
    fn modes_are_distinct() {
        assert_ne!(Mode::Off, Mode::Random);
        assert_ne!(Mode::Off, Mode::SpreadOut);
        assert_ne!(Mode::Random, Mode::SpreadOut);
    }

    #[test]
    fn off_is_source_order_and_ignores_seed_and_recent() {
        let items = [item(1, 10), item(2, 20), item(1, 10)];
        let mut recent = RecentPlays::empty();
        recent.record_artist(ArtistKey::new(1), ts(100));
        assert_eq!(order(&items, Mode::Off, 0, &empty()), vec![0, 1, 2]);
        assert_eq!(order(&items, Mode::Off, 1_234_567, &recent), vec![0, 1, 2]);
        assert_eq!(order(&[] as &[ShuffleItem], Mode::Off, 7, &empty()), vec![]);
        assert_eq!(order(&[item(1, 1)], Mode::Off, 7, &empty()), vec![0]);
    }

    #[test]
    fn random_matches_independent_fisher_yates_on_the_published_seed() {
        // Worked from the published outputs: first % 3 == 0, second % 2 == 1,
        // so n = 3 starts as [0, 1, 2], swaps 2 with 0, then 1 with 1.
        let items = [item(1, 10), item(2, 20), item(3, 30)];
        assert_eq!(SPLITMIX64_SEED_1234567[0] % 3, 0);
        assert_eq!(SPLITMIX64_SEED_1234567[1] % 2, 1);
        assert_eq!(
            order(&items, Mode::Random, 1_234_567, &empty()),
            vec![2, 1, 0]
        );
        assert_eq!(reference_fisher_yates(3, 1_234_567), vec![2, 1, 0]);
        assert_eq!(
            order(&items, Mode::Random, 1_234_567, &empty()),
            reference_fisher_yates(3, 1_234_567)
        );
    }

    #[test]
    fn random_ignores_recent_plays() {
        let items = [item(1, 10), item(2, 20), item(3, 30)];
        let mut recent = RecentPlays::empty();
        recent.record_artist(ArtistKey::new(3), ts(1));
        recent.record_album(AlbumKey::new(10), ts(1));
        assert_eq!(
            order(&items, Mode::Random, 1_234_567, &recent),
            reference_fisher_yates(3, 1_234_567)
        );
    }

    #[test]
    fn random_empty_and_singleton_are_identity() {
        assert_eq!(
            order(&[] as &[ShuffleItem], Mode::Random, 1_234_567, &empty()),
            vec![]
        );
        assert_eq!(
            order(&[item(1, 1)], Mode::Random, 1_234_567, &empty()),
            vec![0]
        );
        assert_eq!(reference_fisher_yates(0, 1_234_567), vec![]);
        assert_eq!(reference_fisher_yates(1, 1_234_567), vec![0]);
    }

    #[test]
    fn spread_out_with_no_recency_picks_from_the_published_stream() {
        // All unique artists and albums, never played: sequential uniform
        // picks among remaining. First output % 3 == 0 → index 0; second
        // % 2 == 1 → the second of {1, 2}, which is 2; last is 1.
        let items = [item(1, 10), item(2, 20), item(3, 30)];
        assert_eq!(
            order(&items, Mode::SpreadOut, 1_234_567, &empty()),
            vec![0, 2, 1]
        );
        assert_ne!(
            order(&items, Mode::SpreadOut, 1_234_567, &empty()),
            order(&items, Mode::Random, 1_234_567, &empty())
        );
        assert_ne!(
            order(&items, Mode::SpreadOut, 1_234_567, &empty()),
            order(&items, Mode::Off, 1_234_567, &empty())
        );
    }

    #[test]
    fn spread_out_puts_a_recent_artist_later() {
        let items = [item(1, 10), item(2, 20)];
        let mut recent = RecentPlays::empty();
        recent.record_artist(ArtistKey::new(1), ts(100));
        assert_eq!(order(&items, Mode::SpreadOut, 0, &recent), vec![1, 0]);
        assert_eq!(
            order(&items, Mode::SpreadOut, 1_234_567, &recent),
            vec![1, 0]
        );
    }

    #[test]
    fn spread_out_puts_a_recent_album_later() {
        let items = [item(1, 10), item(2, 20)];
        let mut recent = RecentPlays::empty();
        recent.record_album(AlbumKey::new(10), ts(100));
        assert_eq!(order(&items, Mode::SpreadOut, 0, &recent), vec![1, 0]);
    }

    #[test]
    fn spread_out_uses_the_later_of_artist_and_album_recency() {
        let items = [item(1, 10), item(2, 20)];
        let mut recent = RecentPlays::empty();
        recent.record_artist(ArtistKey::new(1), ts(100));
        recent.record_album(AlbumKey::new(10), ts(10));
        recent.record_artist(ArtistKey::new(2), ts(50));
        recent.record_album(AlbumKey::new(20), ts(50));
        // Item 0 is max(100, 10) = 100; item 1 is max(50, 50) = 50.
        assert_eq!(order(&items, Mode::SpreadOut, 0, &recent), vec![1, 0]);
    }

    #[test]
    fn recording_a_later_play_replaces_an_earlier_one() {
        let items = [item(1, 10), item(2, 20), item(3, 30)];
        let mut recent = RecentPlays::empty();
        recent.record_artist(ArtistKey::new(1), ts(50));
        recent.record_artist(ArtistKey::new(1), ts(100));
        recent.record_artist(ArtistKey::new(2), ts(80));
        // C never, B at 80, A at 100 → [2, 1, 0]. If 50 had stuck, A at 50
        // would come before B: [2, 0, 1].
        assert_eq!(order(&items, Mode::SpreadOut, 0, &recent), vec![2, 1, 0]);
    }

    #[test]
    fn recording_an_earlier_play_does_not_replace_a_later_one() {
        let items = [item(1, 10), item(2, 20), item(3, 30)];
        let mut recent = RecentPlays::empty();
        recent.record_artist(ArtistKey::new(1), ts(100));
        recent.record_artist(ArtistKey::new(1), ts(50));
        recent.record_artist(ArtistKey::new(2), ts(80));
        assert_eq!(order(&items, Mode::SpreadOut, 0, &recent), vec![2, 1, 0]);
    }

    #[test]
    fn recording_a_later_album_play_replaces_an_earlier_one() {
        let items = [item(1, 10), item(2, 20), item(3, 30)];
        let mut recent = RecentPlays::empty();
        recent.record_album(AlbumKey::new(10), ts(50));
        recent.record_album(AlbumKey::new(10), ts(100));
        recent.record_album(AlbumKey::new(20), ts(80));
        assert_eq!(order(&items, Mode::SpreadOut, 0, &recent), vec![2, 1, 0]);
    }

    #[test]
    fn spread_out_prefers_a_different_artist_and_album_over_recency() {
        let items = [item(1, 10), item(1, 20), item(2, 30)];
        let mut recent = RecentPlays::empty();
        recent.record_album(AlbumKey::new(20), ts(1));
        recent.record_artist(ArtistKey::new(2), ts(100));
        // Item 0 is never played, so it is first. Remaining: same-artist
        // different-album (1) vs different both (2). Fit beats recency.
        assert_eq!(order(&items, Mode::SpreadOut, 0, &recent), vec![0, 2, 1]);
    }

    #[test]
    fn spread_out_prefers_a_different_artist_over_a_different_album() {
        // First output % 3 == 0, so index 0 leads. After A/10, B/10 is a
        // different artist and A/20 is only a different album.
        let items = [item(1, 10), item(2, 10), item(1, 20)];
        assert_eq!(
            order(&items, Mode::SpreadOut, 1_234_567, &empty()),
            vec![0, 1, 2]
        );
    }

    #[test]
    fn spread_out_prefers_a_different_album_when_no_other_artist_remains() {
        // First output % 3 == 0, so index 0 leads. After A/10 the same
        // album is a worse fit than A/20.
        let items = [item(1, 10), item(1, 10), item(1, 20)];
        assert_eq!(
            order(&items, Mode::SpreadOut, 1_234_567, &empty()),
            vec![0, 2, 1]
        );
    }

    #[test]
    fn spread_out_allows_the_same_artist_when_no_other_artist_remains() {
        let items = [item(1, 10), item(1, 10), item(1, 10)];
        let got = order(&items, Mode::SpreadOut, 1_234_567, &empty());
        assert!(is_permutation(&got, 3), "{got:?}");
        let adjacent_same = got
            .windows(2)
            .any(|pair| items[pair[0]].artist() == items[pair[1]].artist());
        assert!(adjacent_same);
    }

    #[test]
    fn spread_out_empty_and_singleton() {
        assert_eq!(
            order(&[] as &[ShuffleItem], Mode::SpreadOut, 1, &empty()),
            vec![]
        );
        assert_eq!(order(&[item(1, 1)], Mode::SpreadOut, 1, &empty()), vec![0]);
    }

    #[test]
    fn pick_slot_returns_zero_when_nothing_remains() {
        let items = [item(1, 10)];
        let mut rng = SplitMix64::new(1_234_567);
        assert_eq!(pick_slot(&items, &[], None, &empty(), &mut rng), 0);
    }

    #[test]
    fn pick_slot_skips_indices_past_the_item_slice() {
        let items = [item(1, 10), item(2, 20)];
        let remaining = [99_usize, 1];
        let mut rng = SplitMix64::new(2);
        assert_eq!(pick_slot(&items, &remaining, None, &empty(), &mut rng), 1);
    }

    #[test]
    fn pick_slot_keeps_only_the_better_fit_once_it_appears() {
        // Seed 2's first output is even, so a leftover worse candidate at
        // slot 0 would be chosen if it were still in the list (first % 2
        // == 0). After A/10, A/10 is SameBoth and B/20 is DifferentBoth.
        let items = [item(1, 10), item(2, 20)];
        let remaining = [0_usize, 1];
        let prev = item(1, 10);
        assert_eq!(SplitMix64::new(2).next_u64() % 2, 0);
        let mut rng = SplitMix64::new(2);
        assert_eq!(
            pick_slot(&items, &remaining, Some(&prev), &empty(), &mut rng),
            1
        );
    }

    #[test]
    fn pick_slot_skips_equal_fit_that_is_more_recent() {
        let items = [item(1, 10), item(2, 20), item(3, 30)];
        let remaining = [0_usize, 1, 2];
        let mut recent = RecentPlays::empty();
        recent.record_artist(ArtistKey::new(2), ts(100));
        // Fit ties (no previous item). Item 1 is more recent, so the
        // candidates are slots 0 and 2. Seed 1234567 first % 2 == 1 picks
        // slot 2. A leftover slot 1 would make three candidates and first
        // % 3 == 0 would pick slot 0.
        assert_eq!(SPLITMIX64_SEED_1234567[0] % 2, 1);
        assert_eq!(SPLITMIX64_SEED_1234567[0] % 3, 0);
        let mut rng = SplitMix64::new(1_234_567);
        assert_eq!(pick_slot(&items, &remaining, None, &recent, &mut rng), 2);
    }

    fn any_mode() -> impl Strategy<Value = Mode> {
        prop_oneof![Just(Mode::Off), Just(Mode::Random), Just(Mode::SpreadOut)]
    }

    fn any_item() -> impl Strategy<Value = ShuffleItem> {
        (0_u64..4, 0_u64..5).prop_map(|(artist, album)| item(artist, album))
    }

    proptest! {
        #[test]
        fn order_is_a_permutation(
            items in vec(any_item(), 0..12),
            seed in any::<u64>(),
            mode in any_mode(),
        ) {
            let got = order(&items, mode, seed, &empty());
            prop_assert!(is_permutation(&got, items.len()), "{got:?}");
        }

        #[test]
        fn the_same_seed_and_input_give_the_same_order(
            items in vec(any_item(), 0..12),
            seed in any::<u64>(),
            mode in any_mode(),
        ) {
            let recent = empty();
            let first = order(&items, mode, seed, &recent);
            let second = order(&items, mode, seed, &recent);
            prop_assert_eq!(first, second);
        }

        #[test]
        fn random_matches_an_independent_fisher_yates(
            len in 0_usize..16,
            seed in any::<u64>(),
        ) {
            let items: Vec<ShuffleItem> = (0..len)
                .map(|index| item(u64::try_from(index).unwrap(), 0))
                .collect();
            let got = order(&items, Mode::Random, seed, &empty());
            prop_assert_eq!(got, reference_fisher_yates(len, seed));
        }

        #[test]
        fn spread_out_never_puts_the_same_artist_adjacent_when_another_exists(
            items in vec(any_item(), 0..12),
            seed in any::<u64>(),
        ) {
            let got = order(&items, Mode::SpreadOut, seed, &empty());
            for position in 1..got.len() {
                let prev = items[got[position - 1]];
                let cur = items[got[position]];
                let remaining = &got[position..];
                let other_artist = remaining.iter().any(|&index| {
                    items[index].artist() != prev.artist()
                });
                if other_artist {
                    prop_assert_ne!(
                        cur.artist(),
                        prev.artist(),
                        "adjacent same artist at {} in {:?}",
                        position,
                        got
                    );
                }
                let other_both = remaining.iter().any(|&index| {
                    items[index].artist() != prev.artist()
                        && items[index].album() != prev.album()
                });
                if other_both {
                    prop_assert_ne!(cur.album(), prev.album());
                } else {
                    let other_album = remaining.iter().any(|&index| {
                        items[index].album() != prev.album()
                    });
                    if !other_artist && other_album {
                        prop_assert_ne!(cur.album(), prev.album());
                    }
                }
            }
        }
    }
}
