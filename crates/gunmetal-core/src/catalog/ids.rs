//! Public identifiers typed by what they name, so a record cannot hold an
//! album's identifier where a track's belongs (SEC-API-024's kinds, carried
//! into the types).

use crate::id::{IdKind, PublicId};

use super::error::CatalogError;
use super::kind::RecordKind;

/// `id` when it is of kind `kind`.
///
/// [`PublicId`] keeps its kind to itself. Its one text form starts with the
/// kind's prefix, so reading that text back as `kind` succeeds exactly when
/// the kinds match.
fn of_kind(id: PublicId, kind: IdKind) -> Result<PublicId, CatalogError> {
    PublicId::parse(&id.to_string(), kind).map_err(|_| CatalogError::WrongIdKind { expected: kind })
}

/// Declares a public identifier that only holds one kind.
macro_rules! typed_id {
    ($(#[$doc:meta])* $name:ident = $kind:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(PublicId);

        impl $name {
            #[doc = concat!("`id`, which must be of kind [`IdKind::", stringify!($kind), "`].")]
            ///
            /// # Errors
            ///
            /// [`CatalogError::WrongIdKind`] for an identifier of any other
            /// kind.
            pub fn new(id: PublicId) -> Result<Self, CatalogError> {
                of_kind(id, IdKind::$kind).map(Self)
            }

            /// The public identifier.
            #[must_use]
            pub const fn get(self) -> PublicId {
                self.0
            }
        }
    };
}

typed_id! {
    /// A track's public identifier.
    TrackId = Track
}

typed_id! {
    /// An album's public identifier.
    AlbumId = Album
}

typed_id! {
    /// An artist's public identifier.
    ArtistId = Artist
}

typed_id! {
    /// A library's public identifier.
    LibraryId = Library
}

/// The identifier of one synced-library record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecordId {
    /// A track.
    Track(TrackId),
    /// An album.
    Album(AlbumId),
    /// An artist.
    Artist(ArtistId),
}

impl RecordId {
    /// Which kind of record it identifies.
    #[must_use]
    pub const fn kind(self) -> RecordKind {
        match self {
            Self::Track(_) => RecordKind::Track,
            Self::Album(_) => RecordKind::Album,
            Self::Artist(_) => RecordKind::Artist,
        }
    }

    /// The public identifier.
    #[must_use]
    pub const fn public_id(self) -> PublicId {
        match self {
            Self::Track(id) => id.get(),
            Self::Album(id) => id.get(),
            Self::Artist(id) => id.get(),
        }
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// The identifier of kind `kind` whose symbols are `symbols`.
    pub(in crate::catalog) fn public(kind: IdKind, prefix: &str, symbols: &str) -> PublicId {
        PublicId::parse(&format!("{prefix}_{symbols}"), kind).unwrap()
    }

    const SYMBOLS: &str = "0123456789abcdefghjkmnpqrs";

    #[test]
    fn each_typed_identifier_keeps_its_own_kind() {
        let track = public(IdKind::Track, "trk", SYMBOLS);
        let album = public(IdKind::Album, "alb", SYMBOLS);
        let artist = public(IdKind::Artist, "art", SYMBOLS);
        let library = public(IdKind::Library, "lib", SYMBOLS);
        assert_eq!(
            (
                TrackId::new(track).map(TrackId::get),
                AlbumId::new(album).map(AlbumId::get),
                ArtistId::new(artist).map(ArtistId::get),
                LibraryId::new(library).map(LibraryId::get),
            ),
            (Ok(track), Ok(album), Ok(artist), Ok(library))
        );
    }

    #[test]
    fn each_typed_identifier_refuses_another_kind() {
        let track = public(IdKind::Track, "trk", SYMBOLS);
        let album = public(IdKind::Album, "alb", SYMBOLS);
        let wrong = |expected| CatalogError::WrongIdKind { expected };
        assert_eq!(
            (
                TrackId::new(album),
                AlbumId::new(track),
                ArtistId::new(track),
                LibraryId::new(track),
            ),
            (
                Err(wrong(IdKind::Track)),
                Err(wrong(IdKind::Album)),
                Err(wrong(IdKind::Artist)),
                Err(wrong(IdKind::Library)),
            )
        );
    }

    #[test]
    fn a_record_identifier_knows_its_kind_and_its_public_identifier() {
        let track = public(IdKind::Track, "trk", SYMBOLS);
        let album = public(IdKind::Album, "alb", "7zzzzzzzzzzzzzzzzzzzzzzzzz");
        let artist = public(IdKind::Artist, "art", "00000000000000000000000000");
        let ids = [
            RecordId::Track(TrackId::new(track).unwrap()),
            RecordId::Album(AlbumId::new(album).unwrap()),
            RecordId::Artist(ArtistId::new(artist).unwrap()),
        ];
        let read: Vec<(RecordKind, PublicId)> =
            ids.iter().map(|id| (id.kind(), id.public_id())).collect();
        assert_eq!(
            read,
            [
                (RecordKind::Track, track),
                (RecordKind::Album, album),
                (RecordKind::Artist, artist),
            ]
        );
    }
}
