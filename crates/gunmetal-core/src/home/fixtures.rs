//! Literal values for Home's tests.
//!
//! By convention track `n` has the track identifier that spells `n` and is
//! known to events by the content identity made of the byte `n`, so an
//! expected row can be written out by number. Nothing here calls the code
//! under test.

use crate::catalog::{
    AlbumId, AudioFormat, Availability, Codec, Container, GainTags, ItemKind, LibraryId, TechInfo,
    TrackId, TrackPosition, TrackRecord,
};
use crate::id::{IdKind, PublicId};
use crate::time::Timestamp;
use crate::userdata::event::{
    Body, ContentId, DeviceId, Event, EventId, ItemRef, Play, ProfileId, Skip, Stream,
};
use crate::userdata::hlc::Hlc;

use super::input::{LibraryTrack, Listening, ListeningSource};

/// The person whose Home the tests evaluate.
pub const ME: ProfileId = ProfileId::new([1; 16]);

/// Another person on the same server.
pub const SOMEONE_ELSE: ProfileId = ProfileId::new([2; 16]);

/// The identifier of kind `kind` whose 26 symbols spell `n` in decimal.
fn public(prefix: &str, kind: IdKind, n: u8) -> PublicId {
    PublicId::parse(&format!("{prefix}{n:026}"), kind).expect("a canonical identifier")
}

/// The identifier of track `n`.
pub fn track_id(n: u8) -> TrackId {
    TrackId::new(public("trk_", IdKind::Track, n)).expect("a track's identifier")
}

/// The identifier of album `n`.
pub fn album_id(n: u8) -> AlbumId {
    AlbumId::new(public("alb_", IdKind::Album, n)).expect("an album's identifier")
}

/// The identifier of playlist `n`.
pub fn playlist(n: u8) -> PublicId {
    public("pls_", IdKind::Playlist, n)
}

/// The content identity the user log knows track `n` by.
pub fn identity(n: u8) -> ContentId {
    ContentId::new([n; 32])
}

/// The time `millis` milliseconds after the Unix epoch.
pub fn at(millis: i64) -> Timestamp {
    Timestamp::from_millis(millis).expect("a time within range")
}

/// The facts of a file that holds `codec` in `container`.
pub fn tech(codec: Codec, container: Container) -> TechInfo {
    TechInfo::new(codec, container, AudioFormat::default()).expect("a codec the container carries")
}

/// One track of a test library: its number, the number of its album when it
/// is on one, and when it was added, in milliseconds.
pub type Spec = (u8, Option<u8>, i64);

/// Track `n` as the synced library holds it: an MP3 file in library 1.
fn record((n, album, added_ms): Spec) -> TrackRecord {
    TrackRecord {
        id: track_id(n),
        kind: ItemKind::Track,
        library: LibraryId::new(public("lib_", IdKind::Library, 1))
            .expect("a library's identifier"),
        title: format!("Track {n}"),
        title_sort: None,
        artist_credit: String::new(),
        artists: Vec::new(),
        album: album.map(album_id),
        position: TrackPosition::default(),
        disc_subtitle: None,
        date: None,
        original_date: None,
        genres: Vec::new(),
        moods: Vec::new(),
        styles: Vec::new(),
        labels: Vec::new(),
        grouping: Vec::new(),
        advisory: None,
        isrc: Vec::new(),
        recording_mbid: None,
        tech: tech(Codec::Mp3, Container::Mpeg),
        gain: GainTags::default(),
        trim: None,
        lyrics: None,
        availability: Availability::Playable,
        added: at(added_ms),
    }
}

/// The library `specs` describe: each track's record with the identity the
/// user log knows it by.
pub fn library(specs: &[Spec]) -> Vec<(ContentId, TrackRecord)> {
    specs
        .iter()
        .map(|&spec| (identity(spec.0), record(spec)))
        .collect()
}

/// `library` as Home reads it.
pub fn view(library: &[(ContentId, TrackRecord)]) -> Vec<LibraryTrack<'_>> {
    library
        .iter()
        .map(|(known_as, track)| LibraryTrack {
            record: track,
            identity: *known_as,
        })
        .collect()
}

/// A place on album `n`, last played from at `last_ms` with `left` entries
/// still to come.
pub fn on_album(n: u8, last_ms: i64, left: u32) -> Listening {
    Listening {
        source: ListeningSource::Album(album_id(n)),
        last_played: at(last_ms),
        left,
    }
}

/// A place on playlist `n`, last played from at `last_ms` with `left`
/// entries still to come.
pub fn on_playlist(n: u8, last_ms: i64, left: u32) -> Listening {
    Listening {
        source: ListeningSource::Playlist(playlist(n)),
        last_played: at(last_ms),
        left,
    }
}

/// A play of track `n` to its end.
pub fn play_body(n: u8) -> Body {
    Body::Play(Play {
        item: identity(n),
        position_ms: 1_000,
        completed: true,
    })
}

/// A skip of track `n`.
pub fn skip_body(n: u8) -> Body {
    Body::Skip(Skip {
        item: identity(n),
        position_ms: 2_000,
    })
}

/// A love of track `n`.
pub fn love_body(n: u8) -> Body {
    Body::Love(ItemRef::Content(identity(n)))
}

/// A removed love of track `n`.
pub fn unlove_body(n: u8) -> Body {
    Body::Unlove(ItemRef::Content(identity(n)))
}

/// Event `id`, authored by `profile` on device `device` at wall time
/// `wall_ms`.
pub fn event(id: u8, wall_ms: u64, device: u8, profile: ProfileId, body: Body) -> Event {
    Event {
        id: EventId::new([id; 16]),
        clock: Hlc::new(wall_ms, 0),
        device: DeviceId::new([device; 16]),
        stream: Stream::Profile(profile),
        body,
    }
}

/// Event `id`: I played track `n` at `wall_ms`.
pub fn play(id: u8, wall_ms: u64, n: u8) -> Event {
    event(id, wall_ms, 1, ME, play_body(n))
}

/// Event `id`: I skipped track `n` at `wall_ms`.
pub fn skip(id: u8, wall_ms: u64, n: u8) -> Event {
    event(id, wall_ms, 1, ME, skip_body(n))
}

/// Event `id`: I loved track `n` at `wall_ms`.
pub fn love(id: u8, wall_ms: u64, n: u8) -> Event {
    event(id, wall_ms, 1, ME, love_body(n))
}

/// Event `id`: I took back my love of track `n` at `wall_ms`.
pub fn unlove(id: u8, wall_ms: u64, n: u8) -> Event {
    event(id, wall_ms, 1, ME, unlove_body(n))
}
