//! Properties of Home's rows over generated libraries and activity.
//!
//! Numbers come from small sets, so generated cases collide often: several
//! tracks on one album, a love and its removal at one clock, a play by me
//! and one by someone else of the same track, the same album listened to
//! twice, and tracks, albums and playlists the library view does not hold.

use proptest::collection::vec;
use proptest::prelude::*;

use crate::userdata::event::{Event, ItemRef, Stream};
use crate::userdata::merge::{EventSet, current_love};

use super::evaluate::evaluate_row;
use super::fixtures::{
    ME, SOMEONE_ELSE, Spec, album_id, at, event, identity, library, love_body, play_body, playlist,
    skip_body, track_id, unlove_body, view,
};
use super::input::{LibraryView, Listening, ListeningSource, MyEvents};
use super::row::{Card, Row, RowSource, RowSpec};

/// A generated Home: a library, everyone's events, where I was listening
/// and the time.
#[derive(Debug)]
struct Case {
    specs: Vec<Spec>,
    events: Vec<Event>,
    listening: Vec<Listening>,
    now_ms: i64,
}

impl Case {
    /// The row `source` shows me, at most `limit` cards long, when the
    /// device holds `events`.
    fn row(&self, source: RowSource, limit: usize, events: &[Event]) -> Row {
        let shelf = library(&self.specs);
        let tracks = view(&shelf);
        let set: EventSet = events.iter().cloned().collect();
        let mine = MyEvents {
            events: &set,
            profile: ME,
            listening: &self.listening,
        };
        evaluate_row(
            &RowSpec { source, limit },
            &LibraryView { tracks: &tracks },
            &mine,
            at(self.now_ms),
        )
    }

    /// Every card a row may hold: the view's tracks, their albums, alone or
    /// with any count of new tracks, and the playlists I was listening to.
    fn nameable(&self) -> Vec<Card> {
        let tracks = self.specs.iter().map(|&(n, _, _)| Card::Track(track_id(n)));
        let albums = self
            .specs
            .iter()
            .filter_map(|&(_, on, _)| on)
            .flat_map(|n| {
                (1_u32..8)
                    .map(move |new_tracks| Card::AlbumWithNewTracks {
                        album: album_id(n),
                        new_tracks,
                    })
                    .chain([Card::Album(album_id(n))])
            });
        let playlists = (0_u8..2)
            .map(playlist)
            .filter(|&id| {
                self.listening
                    .iter()
                    .any(|place| place.source == ListeningSource::Playlist(id))
            })
            .map(Card::Playlist);
        tracks.chain(albums).chain(playlists).collect()
    }
}

/// Up to eight tracks, numbered from zero, each on one of three albums or
/// on none, added at one of a few moments on three days.
fn specs() -> impl Strategy<Value = Vec<Spec>> {
    let on = prop_oneof![Just(None), (0_u8..3).prop_map(Some)];
    let added_ms = prop_oneof![
        Just(0_i64),
        Just(1_000_i64),
        Just(86_400_000_i64),
        Just(86_401_001_i64),
        Just(172_800_000_i64),
    ];
    vec((on, added_ms), 0..8).prop_map(|tracks| {
        (0_u8..)
            .zip(tracks)
            .map(|(n, (on, added_ms))| (n, on, added_ms))
            .collect()
    })
}

/// Up to twelve events, each with an ID of its own: plays, skips, loves and
/// removed loves of tracks 0 to 8, by me or by someone else, on one of two
/// devices at one of four clocks. Track 8 is never in the library.
fn events() -> impl Strategy<Value = Vec<Event>> {
    let body = prop_oneof![
        (0_u8..9).prop_map(play_body),
        (0_u8..9).prop_map(skip_body),
        (0_u8..9).prop_map(love_body),
        (0_u8..9).prop_map(unlove_body),
    ];
    vec((body, 0_u64..4, 1_u8..3, any::<bool>()), 0..12).prop_map(|drafts| {
        (0_u8..)
            .zip(drafts)
            .map(|(id, (body, wall_ms, device, by_me))| {
                let profile = [SOMEONE_ELSE, ME][usize::from(by_me)];
                event(id, wall_ms, device, profile, body)
            })
            .collect()
    })
}

/// Up to six places on albums 0 to 3 and on two playlists, last played from
/// at one of three moments, finished or not. Album 3 is never in the
/// library.
fn listening() -> impl Strategy<Value = Vec<Listening>> {
    let source = prop_oneof![
        (0_u8..4).prop_map(|n| ListeningSource::Album(album_id(n))),
        (0_u8..2).prop_map(|n| ListeningSource::Playlist(playlist(n))),
    ];
    let last_ms = prop_oneof![Just(0_i64), Just(1_000_i64), Just(2_000_i64)];
    vec((source, last_ms, 0_u32..3), 0..6).prop_map(|places| {
        places
            .into_iter()
            .map(|(source, last_ms, left)| Listening {
                source,
                last_played: at(last_ms),
                left,
            })
            .collect()
    })
}

/// A time at which every place is recent, two at which only the later
/// places are, and one at which none is.
fn now_ms() -> impl Strategy<Value = i64> {
    prop_oneof![
        Just(2_000_i64),
        Just(2_592_000_500_i64),
        Just(2_592_001_500_i64),
        Just(9_000_000_000_i64),
    ]
}

/// Any generated Home.
fn cases() -> impl Strategy<Value = Case> {
    (specs(), events(), listening(), now_ms()).prop_map(|(specs, events, listening, now_ms)| Case {
        specs,
        events,
        listening,
        now_ms,
    })
}

/// Any built-in row.
fn sources() -> impl Strategy<Value = RowSource> {
    prop_oneof![
        Just(RowSource::ContinueListening),
        Just(RowSource::RecentlyPlayed),
        Just(RowSource::RecentlyAdded),
        Just(RowSource::LovedSongs),
    ]
}

proptest! {
    /// A row is decided by its input and `now` alone: the same input gives
    /// the same row, whatever order the device received the events in.
    #[test]
    fn a_row_is_decided_by_its_input_and_now(
        case in cases(),
        source in sources(),
        limit in 0_usize..6,
    ) {
        let row = case.row(source, limit, &case.events);
        prop_assert_eq!(&row, &case.row(source, limit, &case.events));
        let backwards: Vec<Event> = case.events.iter().rev().cloned().collect();
        prop_assert_eq!(&row, &case.row(source, limit, &backwards));
    }

    /// A row keeps to its limit, shows no card twice and names only what
    /// the library view holds and the playlists I was listening to,
    /// whatever anyone's events say (TM-T15).
    #[test]
    fn a_row_keeps_to_its_limit_and_names_only_what_the_view_holds(
        case in cases(),
        source in sources(),
        limit in 0_usize..6,
    ) {
        let row = case.row(source, limit, &case.events);
        let cards = row.content.cards();
        prop_assert!(cards.len() <= limit);
        let nameable = case.nameable();
        prop_assert!(cards.iter().all(|card| nameable.contains(card)));
        prop_assert!((0..cards.len()).all(|i| !cards[..i].contains(&cards[i])));
    }

    /// Another person's events never change my rows (TM-T18).
    #[test]
    fn another_persons_events_never_change_my_rows(case in cases(), source in sources()) {
        let mine: Vec<Event> = case
            .events
            .iter()
            .filter(|event| event.stream == Stream::Profile(ME))
            .cloned()
            .collect();
        prop_assert_eq!(case.row(source, 20, &case.events), case.row(source, 20, &mine));
    }

    /// Loved songs holds exactly the tracks of the view that the user log's
    /// own merge rule says I love now.
    #[test]
    fn loved_songs_are_exactly_the_tracks_of_the_view_i_love_now(case in cases()) {
        let set: EventSet = case.events.iter().cloned().collect();
        let loved: Vec<Card> = case
            .specs
            .iter()
            .map(|&(n, _, _)| n)
            .filter(|&n| current_love(&set, Stream::Profile(ME), ItemRef::Content(identity(n))))
            .map(|n| Card::Track(track_id(n)))
            .collect();
        let row = case.row(RowSource::LovedSongs, 20, &case.events);
        let cards = row.content.cards();
        prop_assert_eq!(cards.len(), loved.len());
        prop_assert!(loved.iter().all(|card| cards.contains(card)));
    }
}
