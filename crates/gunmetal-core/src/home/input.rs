//! What a Home row is evaluated from: the synced library and one person's
//! own activity (DIS-002).
//!
//! The synced library holds only what the profile may see, because the
//! server filters it by grants as it builds it (SEC-CLI-020), so a row can
//! name nothing else (TB4, TM-T15). The activity is one profile's: its
//! stream of the user log, and the albums and playlists its queue was
//! playing from. The row sources read no other stream, so one person's
//! plays and loves never reach another person's Home (TM-T18).
//!
//! Everything here is plain data with public fields, and none of it is
//! trusted: a place on an album the view does not hold, and an album or a
//! playlist listed twice, evaluate to a row like any other input.

use crate::catalog::{AlbumId, TrackRecord};
use crate::id::PublicId;
use crate::time::Timestamp;
use crate::userdata::event::{ContentId, ProfileId};
use crate::userdata::merge::EventSet;

/// One track of the synced library, with the name the user log knows it by.
///
/// The log names a library item only by its content identity (ADR 3,
/// section 14) and a record only by its public identifier, so whoever
/// builds the view pairs the two.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LibraryTrack<'a> {
    /// The track's record.
    pub record: &'a TrackRecord,
    /// The content identity the person's events name the track by.
    pub identity: ContentId,
}

/// The synced library as Home reads it: every track the profile may see.
///
/// Albums are read from the tracks' own records, so an album is in the
/// view exactly when one of its tracks is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LibraryView<'a> {
    /// The tracks, in the order the device holds them. Where a row has no
    /// other way to order two cards, it keeps this order.
    pub tracks: &'a [LibraryTrack<'a>],
}

/// What a person was playing from: one of the context identifiers the
/// queue stores (MUS-050, MUS-122).
///
/// A play event records no source (SEC-PRV-002), so Continue listening
/// reads the queue's contexts instead of the history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ListeningSource {
    /// An album.
    Album(AlbumId),
    /// A playlist, by its public identifier.
    Playlist(PublicId),
}

/// One place a person stopped listening: an album or a playlist, when they
/// last played from it, and how much of it was still to come.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Listening {
    /// What they were playing from.
    pub source: ListeningSource,
    /// When they last played from it.
    pub last_played: Timestamp,
    /// How many of its entries were still to play when they stopped; zero
    /// when they reached its end.
    pub left: u32,
}

/// One person's own activity, as Home reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MyEvents<'a> {
    /// The events the device holds. Only the stream of
    /// [`MyEvents::profile`] is read.
    pub events: &'a EventSet,
    /// The profile whose Home this is.
    pub profile: ProfileId,
    /// The places the person's queue has been playing from, in any order.
    /// The same album or playlist may be listed more than once; the latest
    /// entry is the one that counts.
    pub listening: &'a [Listening],
}
