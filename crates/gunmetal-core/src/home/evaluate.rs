//! Evaluating one Home row on the device, from the synced library and the
//! person's own activity (DIS-002).
//!
//! A row is a pure function of its input and `now`, so every device that
//! holds the same library and the same events draws the same Home. Where
//! two cards have nothing else to order them, they keep the order their
//! tracks or places were listed in.
//!
//! - **Continue listening** (MUS-050, DIS-020) reads the places the
//!   person's queue was playing from, because a play records no source
//!   (SEC-PRV-002). Only the latest place on each album or playlist counts.
//!   It is listed while it has entries left to play, was last played from
//!   within [`CONTINUE_WINDOW_MS`] of `now` and, for an album, has a track
//!   in the view; the place played from last comes first.
//! - **Recently played** (DIS-021; API-LOG-04) lists each track of the view
//!   the person has played, by the clock of its latest play, as the user
//!   log's own counts give it.
//! - **Recently added** (DIS-035, DIS-036, MUS-059; API-HOME-03) lists each
//!   album once, at the date its newest track was added, and each track
//!   without an album by itself; the newest comes first. It reads only the
//!   date a track first arrived, which a better copy of the same recording
//!   does not reset (DIS-038), so an upgrade is not an arrival. An album
//!   whose tracks were all added within [`ARRIVAL_SPAN_MS`] of its newest
//!   arrived whole; any other gained the tracks that were.
//! - **Loved songs** (MUS-149, DIS-046) lists the tracks of the view the
//!   person loves now, the latest love first. Which love or removal stands
//!   is the user log's merge rule: the latest clock, then the larger
//!   device, then the larger event ID.

use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::catalog::{AlbumId, TrackId};
use crate::time::Timestamp;
use crate::userdata::event::{Body, ContentId, DeviceId, EventId, ItemRef, Stream};
use crate::userdata::hlc::Hlc;
use crate::userdata::merge::derive_counts;

use super::input::{LibraryView, Listening, ListeningSource, MyEvents};
use super::row::{Card, EmptyState, Reason, Row, RowContent, RowSource, RowSpec};

/// How long after a person last played from an album or a playlist Continue
/// listening still offers it: thirty days, in milliseconds.
///
/// R1 has no way to dismiss a card (DIS-022 is R1.1), so without a bound an
/// abandoned album would stay on Home for good.
pub const CONTINUE_WINDOW_MS: i64 = 2_592_000_000;

/// How far apart, in milliseconds, two tracks of an album may be added and
/// still count as one arrival: a day.
///
/// A scan adds an album's tracks one after another, and a first scan can
/// take hours, so the tracks of one arrival carry dates that differ.
pub const ARRIVAL_SPAN_MS: i64 = 86_400_000;

/// What decides between two events of one stream under a "latest wins"
/// rule: the clock, then the device, then the event ID, as the user log's
/// merge rules order them.
type Precedence = (Hlc, DeviceId, EventId);

/// The row `spec` asks for, as the person `mine` sees the library `lib` at
/// the time `now`.
///
/// The row holds at most `spec.limit` cards. With no track in the view
/// every row is [`EmptyState::EmptyLibrary`], whatever the person has done;
/// a row of a library that has music and nothing for this row is
/// [`EmptyState::NothingYet`].
#[must_use]
pub fn evaluate_row(
    spec: &RowSpec,
    lib: &LibraryView<'_>,
    mine: &MyEvents<'_>,
    now: Timestamp,
) -> Row {
    let (reason, mut cards) = match spec.source {
        RowSource::ContinueListening => (
            Reason::PartWayThrough,
            continue_listening(lib, mine.listening, now),
        ),
        RowSource::RecentlyPlayed => (Reason::PlayedLately, recently_played(lib, mine)),
        RowSource::RecentlyAdded => (Reason::AddedLately, recently_added(lib)),
        RowSource::LovedSongs => (Reason::Loved, loved_songs(lib, mine)),
    };
    cards.truncate(spec.limit);
    let content = if lib.tracks.is_empty() {
        RowContent::Empty(EmptyState::EmptyLibrary)
    } else if cards.is_empty() {
        RowContent::Empty(EmptyState::NothingYet)
    } else {
        RowContent::Cards(cards)
    };
    Row {
        source: spec.source,
        reason,
        content,
    }
}

/// The albums and playlists the person stopped part-way through, the one
/// played from last first.
fn continue_listening(lib: &LibraryView<'_>, listening: &[Listening], now: Timestamp) -> Vec<Card> {
    let albums: HashSet<AlbumId> = lib
        .tracks
        .iter()
        .filter_map(|track| track.record.album)
        .collect();
    let mut places: Vec<&Listening> = listening.iter().collect();
    places.sort_by_key(|place| Reverse(place.last_played));
    let mut seen = HashSet::new();
    places
        .into_iter()
        // The latest place on an album or a playlist is the last word on
        // it: once finished, an earlier, unfinished visit no longer counts.
        .filter(|place| seen.insert(place.source))
        .filter(|place| {
            place.left > 0
                && now.millis().saturating_sub(place.last_played.millis()) <= CONTINUE_WINDOW_MS
        })
        .filter_map(|place| match place.source {
            ListeningSource::Album(album) => albums.contains(&album).then_some(Card::Album(album)),
            ListeningSource::Playlist(playlist) => Some(Card::Playlist(playlist)),
        })
        .collect()
}

/// The tracks of the view the person has played, the one played last first.
fn recently_played(lib: &LibraryView<'_>, mine: &MyEvents<'_>) -> Vec<Card> {
    let counts = derive_counts(mine.events, Stream::Profile(mine.profile));
    let mut played: Vec<(Hlc, TrackId)> = lib
        .tracks
        .iter()
        .filter_map(|track| {
            let last_played = counts.get(&track.identity)?.last_played?;
            Some((last_played, track.record.id))
        })
        .collect();
    played.sort_by_key(|&(last_played, _)| Reverse(last_played));
    played.into_iter().map(|(_, id)| Card::Track(id)).collect()
}

/// The library's arrivals, the newest first: each album once, and each
/// track that is on no album.
fn recently_added(lib: &LibraryView<'_>) -> Vec<Card> {
    let mut albums: HashMap<AlbumId, Vec<Timestamp>> = HashMap::new();
    for track in lib.tracks {
        if let Some(album) = track.record.album {
            albums.entry(album).or_default().push(track.record.added);
        }
    }
    let mut arrivals: Vec<(Timestamp, Card)> = lib
        .tracks
        .iter()
        .filter_map(|track| match track.record.album {
            None => Some((track.record.added, Card::Track(track.record.id))),
            // An album's first track in the view stands for the album, and
            // takes its dates out of the map so that no later track does.
            Some(album) => albums
                .remove(&album)
                .map(|added| album_arrival(album, &added)),
        })
        .collect();
    arrivals.sort_by_key(|&(arrived, _)| Reverse(arrived));
    arrivals.into_iter().map(|(_, card)| card).collect()
}

/// When the album `album` last gained tracks, given the dates its tracks
/// were `added`, and the card that says what arrived then: the whole album,
/// or the tracks that joined it later (DIS-036).
fn album_arrival(album: AlbumId, added: &[Timestamp]) -> (Timestamp, Card) {
    let newest = added.iter().copied().fold(Timestamp::MIN, Timestamp::max);
    let together = added
        .iter()
        .filter(|date| newest.millis().saturating_sub(date.millis()) <= ARRIVAL_SPAN_MS)
        .count();
    let card = if together == added.len() {
        Card::Album(album)
    } else {
        Card::AlbumWithNewTracks {
            album,
            new_tracks: u32::try_from(together).unwrap_or(u32::MAX),
        }
    };
    (newest, card)
}

/// The tracks of the view the person loves now, the latest love first.
fn loved_songs(lib: &LibraryView<'_>, mine: &MyEvents<'_>) -> Vec<Card> {
    let stream = Stream::Profile(mine.profile);
    // The love or removal that stands for each item, and whether it is a
    // love.
    let mut standing: BTreeMap<ContentId, (Precedence, bool)> = BTreeMap::new();
    for event in mine.events.iter().filter(|event| event.stream == stream) {
        let (item, is_love) = match event.body {
            Body::Love(ItemRef::Content(item)) => (item, true),
            Body::Unlove(ItemRef::Content(item)) => (item, false),
            _ => continue,
        };
        let word = ((event.clock, event.device, event.id), is_love);
        standing
            .entry(item)
            .and_modify(|held| *held = word.max(*held))
            .or_insert(word);
    }
    let mut loved: Vec<(Precedence, TrackId)> = lib
        .tracks
        .iter()
        .filter_map(|track| match standing.get(&track.identity) {
            Some(&(when, true)) => Some((when, track.record.id)),
            _ => None,
        })
        .collect();
    loved.sort_by_key(|&(when, _)| Reverse(when));
    loved.into_iter().map(|(_, id)| Card::Track(id)).collect()
}

#[cfg(test)]
mod tests {
    use crate::catalog::{Codec, Container, TrackRecord};
    use crate::userdata::event::{Body, ContentId, DocumentId, Event, ItemRef, Stream};
    use crate::userdata::merge::EventSet;

    use super::super::fixtures::{
        ME, SOMEONE_ELSE, Spec, album_id, at, event, library, love, love_body, on_album,
        on_playlist, play, play_body, playlist, skip, tech, track_id, unlove, unlove_body, view,
    };
    use super::super::input::{LibraryView, Listening, MyEvents};
    use super::super::row::{Card, EmptyState, Reason, Row, RowContent, RowSource, RowSpec};
    use super::evaluate_row;

    /// A row of a library that has music, with nothing in the row yet.
    const NOTHING_YET: RowContent = RowContent::Empty(EmptyState::NothingYet);

    /// The row `source` shows me of `shelf`, at most `limit` cards long,
    /// given my `events`, where I was `listening` and the time `now_ms`.
    fn row_of(
        source: RowSource,
        limit: usize,
        shelf: &[(ContentId, TrackRecord)],
        events: &[Event],
        listening: &[Listening],
        now_ms: i64,
    ) -> Row {
        let tracks = view(shelf);
        let set: EventSet = events.iter().cloned().collect();
        let mine = MyEvents {
            events: &set,
            profile: ME,
            listening,
        };
        evaluate_row(
            &RowSpec { source, limit },
            &LibraryView { tracks: &tracks },
            &mine,
            at(now_ms),
        )
    }

    /// What Continue listening holds at `now_ms`.
    fn continued(specs: &[Spec], listening: &[Listening], now_ms: i64) -> RowContent {
        let shelf = library(specs);
        row_of(
            RowSource::ContinueListening,
            20,
            &shelf,
            &[],
            listening,
            now_ms,
        )
        .content
    }

    /// What Recently played holds.
    fn played(specs: &[Spec], events: &[Event]) -> RowContent {
        row_of(
            RowSource::RecentlyPlayed,
            20,
            &library(specs),
            events,
            &[],
            0,
        )
        .content
    }

    /// What Recently added holds of `shelf`.
    fn added_of(shelf: &[(ContentId, TrackRecord)]) -> RowContent {
        row_of(RowSource::RecentlyAdded, 20, shelf, &[], &[], 0).content
    }

    /// What Recently added holds.
    fn added(specs: &[Spec]) -> RowContent {
        added_of(&library(specs))
    }

    /// What Loved songs holds.
    fn loved(specs: &[Spec], events: &[Event]) -> RowContent {
        row_of(RowSource::LovedSongs, 20, &library(specs), events, &[], 0).content
    }

    /// A row holding `cards`.
    fn shows(cards: &[Card]) -> RowContent {
        RowContent::Cards(cards.to_vec())
    }

    /// A row holding the tracks numbered `numbers`.
    fn songs(numbers: &[u8]) -> RowContent {
        RowContent::Cards(numbers.iter().map(|&n| Card::Track(track_id(n))).collect())
    }

    /// The card of album `n`.
    fn album(n: u8) -> Card {
        Card::Album(album_id(n))
    }

    #[test]
    fn each_row_names_its_source_and_says_why_it_shows_what_it_does() {
        let shelf = library(&[(1, Some(1), 1_000), (2, Some(2), 2_000)]);
        let events = [play(1, 10, 1), love(2, 20, 2)];
        let listening = [on_album(2, 500, 3)];
        let show = |source| row_of(source, 20, &shelf, &events, &listening, 1_000);
        assert_eq!(
            show(RowSource::ContinueListening),
            Row {
                source: RowSource::ContinueListening,
                reason: Reason::PartWayThrough,
                content: shows(&[album(2)]),
            }
        );
        assert_eq!(
            show(RowSource::RecentlyPlayed),
            Row {
                source: RowSource::RecentlyPlayed,
                reason: Reason::PlayedLately,
                content: songs(&[1]),
            }
        );
        assert_eq!(
            show(RowSource::RecentlyAdded),
            Row {
                source: RowSource::RecentlyAdded,
                reason: Reason::AddedLately,
                content: shows(&[album(2), album(1)]),
            }
        );
        assert_eq!(
            show(RowSource::LovedSongs),
            Row {
                source: RowSource::LovedSongs,
                reason: Reason::Loved,
                content: songs(&[2]),
            }
        );
    }

    /// With no track synced there is nothing to make a row of, whatever the
    /// person has done before: Home shows the scan or the "no libraries
    /// yet" note in place of rows (DIS-004).
    #[test]
    fn an_empty_library_gives_every_row_the_empty_library_state() {
        let events = [play(1, 10, 1), love(2, 20, 1)];
        let listening = [on_album(1, 500, 3), on_playlist(1, 600, 2)];
        let rows = [
            (RowSource::ContinueListening, Reason::PartWayThrough),
            (RowSource::RecentlyPlayed, Reason::PlayedLately),
            (RowSource::RecentlyAdded, Reason::AddedLately),
            (RowSource::LovedSongs, Reason::Loved),
        ];
        for (source, reason) in rows {
            assert_eq!(
                row_of(source, 20, &[], &events, &listening, 1_000),
                Row {
                    source,
                    reason,
                    content: RowContent::Empty(EmptyState::EmptyLibrary),
                }
            );
        }
    }

    #[test]
    fn a_row_with_nothing_in_it_yet_says_so() {
        let specs: &[Spec] = &[(1, Some(1), 1_000)];
        assert_eq!(continued(specs, &[], 1_000), NOTHING_YET);
        assert_eq!(played(specs, &[]), NOTHING_YET);
        assert_eq!(loved(specs, &[]), NOTHING_YET);
        assert_eq!(added(specs), shows(&[album(1)]));
    }

    #[test]
    fn a_row_holds_at_most_as_many_cards_as_its_limit() {
        let shelf = library(&[
            (1, Some(1), 1_000),
            (2, Some(2), 3_000),
            (3, Some(3), 2_000),
        ]);
        let limited = |limit| row_of(RowSource::RecentlyAdded, limit, &shelf, &[], &[], 0).content;
        assert_eq!(limited(4), shows(&[album(2), album(3), album(1)]));
        assert_eq!(limited(3), shows(&[album(2), album(3), album(1)]));
        assert_eq!(limited(2), shows(&[album(2), album(3)]));
        assert_eq!(limited(1), shows(&[album(2)]));
        assert_eq!(limited(0), NOTHING_YET);
    }

    #[test]
    fn continue_listening_lists_unfinished_albums_and_playlists_latest_first() {
        let specs: &[Spec] = &[(1, Some(1), 0), (2, Some(2), 0), (3, None, 0)];
        let listening = [
            on_album(1, 1_000, 4),
            on_playlist(7, 3_000, 9),
            on_album(2, 2_000, 1),
        ];
        assert_eq!(
            continued(specs, &listening, 5_000),
            shows(&[Card::Playlist(playlist(7)), album(2), album(1)])
        );
    }

    #[test]
    fn an_album_or_a_playlist_played_to_its_end_is_not_continued() {
        let specs: &[Spec] = &[(1, Some(1), 0), (2, Some(2), 0)];
        let listening = [
            on_album(1, 3_000, 0),
            on_album(2, 1_000, 1),
            on_playlist(3, 2_000, 0),
        ];
        assert_eq!(continued(specs, &listening, 5_000), shows(&[album(2)]));
    }

    /// The same album may be listed once for every time the person played
    /// from it. Only the latest counts: an album abandoned and later
    /// finished is done, and one finished and later started again is not.
    #[test]
    fn only_the_latest_time_at_an_album_or_a_playlist_counts() {
        let specs: &[Spec] = &[(1, Some(1), 0), (2, Some(2), 0), (3, Some(3), 0)];
        let listening = [
            on_album(1, 1_000, 5),
            on_album(1, 4_000, 0),
            on_album(2, 3_000, 2),
            on_album(2, 1_500, 0),
            on_album(3, 2_000, 7),
            on_album(3, 5_000, 6),
            on_playlist(4, 6_000, 0),
            on_playlist(4, 2_500, 8),
        ];
        assert_eq!(
            continued(specs, &listening, 7_000),
            shows(&[album(3), album(2)])
        );
    }

    /// R1 has no way to dismiss a card (DIS-022 is R1.1), so a place the
    /// person has not come back to in thirty days leaves the row by itself.
    #[test]
    fn a_place_left_more_than_thirty_days_ago_is_no_longer_continued() {
        let specs: &[Spec] = &[
            (1, Some(1), 0),
            (2, Some(2), 0),
            (3, Some(3), 0),
            (4, Some(4), 0),
        ];
        let listening = [
            on_album(1, 10_000, 1),
            on_album(2, 9_999, 1),
            on_album(3, 2_592_010_000, 1),
            on_album(4, 2_592_020_000, 1),
        ];
        assert_eq!(
            continued(specs, &listening, 2_592_010_000),
            shows(&[album(4), album(3), album(1)])
        );
        assert_eq!(
            continued(specs, &listening, 2_592_010_001),
            shows(&[album(4), album(3)])
        );
    }

    /// The view holds only what the person may see, so an album none of its
    /// tracks is on is one they may not see any more (TM-T15).
    #[test]
    fn an_album_the_library_view_does_not_hold_is_not_continued() {
        let specs: &[Spec] = &[(1, Some(1), 0), (2, None, 0)];
        let listening = [on_album(2, 2_000, 3), on_album(1, 1_000, 3)];
        assert_eq!(continued(specs, &listening, 5_000), shows(&[album(1)]));
    }

    #[test]
    fn places_left_at_the_same_moment_keep_the_order_they_were_given_in() {
        let specs: &[Spec] = &[(1, Some(1), 0), (2, Some(2), 0)];
        let listening = [
            on_album(2, 1_000, 1),
            on_playlist(1, 1_000, 1),
            on_album(1, 1_000, 1),
        ];
        assert_eq!(
            continued(specs, &listening, 5_000),
            shows(&[album(2), Card::Playlist(playlist(1)), album(1)])
        );
    }

    #[test]
    fn recently_played_lists_each_track_once_by_its_latest_play() {
        let specs: &[Spec] = &[(1, Some(1), 0), (2, Some(1), 0), (3, None, 0)];
        let events = [
            play(1, 5, 1),
            play(2, 10, 2),
            play(3, 40, 3),
            play(4, 30, 1),
        ];
        assert_eq!(played(specs, &events), songs(&[3, 1, 2]));
    }

    #[test]
    fn a_track_never_played_or_only_skipped_is_not_recently_played() {
        let specs: &[Spec] = &[(1, None, 0), (2, None, 0), (3, None, 0)];
        let events = [skip(1, 50, 2), play(2, 10, 3)];
        assert_eq!(played(specs, &events), songs(&[3]));
    }

    /// Only my own stream is read (TM-T18), and a play of something the
    /// view does not hold has no card to show (TM-T15).
    #[test]
    fn another_persons_plays_and_plays_of_tracks_outside_the_view_never_show() {
        let specs: &[Spec] = &[(1, None, 0), (2, None, 0)];
        let events = [
            play(1, 10, 1),
            event(2, 99, 1, SOMEONE_ELSE, play_body(2)),
            play(3, 50, 9),
            Event {
                stream: Stream::Household,
                ..play(4, 70, 2)
            },
        ];
        assert_eq!(played(specs, &events), songs(&[1]));
    }

    #[test]
    fn tracks_last_played_at_the_same_clock_keep_the_order_of_the_view() {
        let events = [play(1, 10, 2), play(2, 10, 1), play(3, 5, 3)];
        assert_eq!(
            played(&[(1, None, 0), (2, None, 0), (3, None, 0)], &events),
            songs(&[1, 2, 3])
        );
        assert_eq!(
            played(&[(3, None, 0), (2, None, 0), (1, None, 0)], &events),
            songs(&[2, 1, 3])
        );
    }

    #[test]
    fn recently_added_shows_each_album_once_and_loose_tracks_newest_first() {
        let specs: &[Spec] = &[
            (1, Some(1), 1_000),
            (2, Some(1), 1_500),
            (3, None, 3_000),
            (4, Some(2), 2_000),
            (5, Some(2), 2_000),
            (6, None, 500),
        ];
        assert_eq!(
            added(specs),
            shows(&[
                Card::Track(track_id(3)),
                album(2),
                album(1),
                Card::Track(track_id(6)),
            ])
        );
    }

    /// Album 1 arrived with three tracks and gained two more ten days on;
    /// album 2 arrived whole in between (DIS-036).
    #[test]
    fn tracks_that_join_an_album_already_held_show_as_that_album_with_their_count() {
        let specs: &[Spec] = &[
            (1, Some(1), 1_000),
            (2, Some(1), 1_000),
            (3, Some(1), 2_000),
            (4, Some(2), 432_000_000),
            (5, Some(1), 864_000_000),
            (6, Some(1), 864_060_000),
        ];
        assert_eq!(
            added(specs),
            shows(&[
                Card::AlbumWithNewTracks {
                    album: album_id(1),
                    new_tracks: 2,
                },
                album(2),
            ])
        );
    }

    /// A scan stamps an album's tracks one after another, so tracks added
    /// within a day of the album's last arrival came with it.
    #[test]
    fn tracks_added_within_a_day_of_an_albums_last_arrival_arrived_with_it() {
        assert_eq!(
            added(&[(1, Some(1), 5_000), (2, Some(1), 86_405_000)]),
            shows(&[album(1)])
        );
        assert_eq!(
            added(&[(2, Some(1), 86_405_000), (1, Some(1), 5_000)]),
            shows(&[album(1)])
        );
        let grew = shows(&[Card::AlbumWithNewTracks {
            album: album_id(1),
            new_tracks: 1,
        }]);
        assert_eq!(
            added(&[(1, Some(1), 5_000), (2, Some(1), 86_405_001)]),
            grew
        );
        assert_eq!(
            added(&[(2, Some(1), 86_405_001), (1, Some(1), 5_000)]),
            grew
        );
    }

    #[test]
    fn arrivals_at_the_same_moment_keep_the_order_of_the_view() {
        assert_eq!(
            added(&[(1, Some(2), 1_000), (2, None, 1_000), (3, Some(1), 1_000)]),
            shows(&[album(2), Card::Track(track_id(2)), album(1)])
        );
    }

    /// An upgrade is the same recording in a new file. The scan keeps its
    /// identity, its identifiers and the date it first arrived (MUS-059,
    /// DIS-038), so nothing about it is an arrival.
    #[test]
    fn an_upgrade_does_not_appear_in_recently_added() {
        let specs: &[Spec] = &[(1, Some(1), 1_000), (2, Some(2), 5_000)];
        let before = library(specs);
        let mut after = library(specs);
        after[0].1.tech = tech(Codec::Flac, Container::Flac);
        assert_ne!(before, after);
        let newest_first = shows(&[album(2), album(1)]);
        assert_eq!(added_of(&before), newest_first);
        assert_eq!(added_of(&after), newest_first);
        // Had the new file been taken for a new arrival, its album would
        // lead the row.
        after[0].1.added = at(9_000);
        assert_eq!(added_of(&after), shows(&[album(1), album(2)]));
    }

    /// Track 3's love was taken back, track 4 was loved again after its
    /// love was taken back, and track 2's love is later than a removal that
    /// reached the device after it.
    #[test]
    fn loved_songs_are_exactly_the_loved_tracks_latest_love_first_with_a_removed_love_gone() {
        let specs: &[Spec] = &[
            (1, None, 0),
            (2, None, 0),
            (3, None, 0),
            (4, None, 0),
            (5, None, 0),
        ];
        let events = [
            love(1, 10, 1),
            love(2, 30, 2),
            love(3, 20, 3),
            unlove(4, 40, 3),
            love(5, 5, 4),
            unlove(6, 6, 4),
            love(7, 50, 4),
            unlove(8, 1, 2),
            play(9, 60, 5),
        ];
        assert_eq!(loved(specs, &events), songs(&[4, 2, 1]));
    }

    /// The merge rule of the user log: the latest clock wins, then the
    /// larger device, then the larger event ID.
    #[test]
    fn a_love_and_its_removal_at_one_clock_go_to_the_larger_device_then_the_larger_event() {
        let specs: &[Spec] = &[(1, None, 0)];
        let decided = |love_id, love_device, unlove_id, unlove_device| {
            loved(
                specs,
                &[
                    event(love_id, 10, love_device, ME, love_body(1)),
                    event(unlove_id, 10, unlove_device, ME, unlove_body(1)),
                ],
            )
        };
        assert_eq!(decided(1, 5, 2, 4), songs(&[1]));
        assert_eq!(decided(2, 4, 1, 5), NOTHING_YET);
        assert_eq!(decided(2, 4, 1, 4), songs(&[1]));
        assert_eq!(decided(1, 4, 2, 4), NOTHING_YET);
    }

    #[test]
    fn songs_loved_at_one_clock_are_ordered_by_device_then_by_event() {
        let specs: &[Spec] = &[(1, None, 0), (2, None, 0), (3, None, 0)];
        let events = [
            event(3, 10, 1, ME, love_body(1)),
            event(1, 10, 2, ME, love_body(2)),
            event(2, 10, 2, ME, love_body(3)),
        ];
        assert_eq!(loved(specs, &events), songs(&[3, 2, 1]));
    }

    /// Only my own stream is read (TM-T18): neither another person's love
    /// nor their removal of one, nor the household's. A loved playlist is
    /// not a song, and a love of something the view does not hold has no
    /// card to show (TM-T15).
    #[test]
    fn only_my_own_loves_of_tracks_in_the_view_are_listed() {
        let specs: &[Spec] = &[(1, None, 0), (2, None, 0), (3, None, 0)];
        let events = [
            love(1, 10, 1),
            event(2, 20, 1, SOMEONE_ELSE, love_body(2)),
            Event {
                stream: Stream::Household,
                ..love(3, 30, 3)
            },
            love(4, 40, 9),
            event(
                5,
                50,
                1,
                ME,
                Body::Love(ItemRef::Document(DocumentId::new([1; 16]))),
            ),
            event(6, 60, 1, SOMEONE_ELSE, unlove_body(1)),
        ];
        assert_eq!(loved(specs, &events), songs(&[1]));
    }
}
