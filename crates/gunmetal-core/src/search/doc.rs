//! What the index holds: one document per artist, album, track and
//! playlist, with the texts search looks in (MUS-061).

use crate::id::{IdKind, PublicId};

/// The type of thing a document stands for, which is also the group its
/// hits are listed in (DIS-083).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DocKind {
    /// An artist.
    Artist,
    /// An album.
    Album,
    /// A track.
    Track,
    /// A playlist.
    Playlist,
}

impl DocKind {
    /// Every kind, in the order the search screen lists its type chips.
    pub const ALL: [Self; 4] = [Self::Artist, Self::Album, Self::Track, Self::Playlist];

    /// The kind of public identifier a document of this kind carries.
    const fn id_kind(self) -> IdKind {
        match self {
            Self::Artist => IdKind::Artist,
            Self::Album => IdKind::Album,
            Self::Track => IdKind::Track,
            Self::Playlist => IdKind::Playlist,
        }
    }
}

/// Which artist, album, track or playlist a document is: its public
/// identifier, with its kind read from the identifier itself so the two
/// cannot disagree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DocRef {
    kind: DocKind,
    id: PublicId,
}

impl DocRef {
    /// The reference to the thing `id` names, or `None` when `id` names
    /// something search does not index, such as a user or a device.
    #[must_use]
    pub fn new(id: PublicId) -> Option<Self> {
        Self::from_text(&id.to_string())
    }

    /// The reference whose identifier is written `text`, or `None` when
    /// `text` is not the one spelling of an artist's, album's, track's or
    /// playlist's identifier.
    pub(super) fn from_text(text: &str) -> Option<Self> {
        DocKind::ALL.into_iter().find_map(|kind| {
            PublicId::parse(text, kind.id_kind())
                .ok()
                .map(|id| Self { kind, id })
        })
    }

    /// What kind of thing it is.
    #[must_use]
    pub const fn kind(self) -> DocKind {
        self.kind
    }

    /// Its public identifier.
    #[must_use]
    pub const fn id(self) -> PublicId {
        self.id
    }
}

/// One thing to index, with the texts search looks in (MUS-061).
///
/// The caller builds documents only from records the profile may see; the
/// index holds nothing else (SEC-CLI-020).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchDoc {
    /// Which thing this is.
    pub doc: DocRef,
    /// Its title, or an artist's name.
    pub title: String,
    /// Its artist credit as tagged; empty when it has none.
    pub artist: String,
    /// The title of its album; empty when it has none.
    pub album: String,
    /// Every credited name: composers, producers, performers and the rest.
    pub credits: Vec<String>,
    /// Its genres.
    pub genres: Vec<String>,
    /// Its record labels.
    pub labels: Vec<String>,
    /// How often this person has played it.
    pub plays: u32,
}

#[cfg(test)]
mod tests {
    use super::super::testing::id_text;
    use super::*;

    #[test]
    fn the_kinds_are_listed_in_chip_order() {
        assert_eq!(
            DocKind::ALL,
            [
                DocKind::Artist,
                DocKind::Album,
                DocKind::Track,
                DocKind::Playlist
            ]
        );
    }

    #[test]
    fn a_reference_reads_its_kind_from_its_identifier() {
        let cases = [
            (
                "art_00000000000000000000000001",
                IdKind::Artist,
                DocKind::Artist,
            ),
            (
                "alb_00000000000000000000000002",
                IdKind::Album,
                DocKind::Album,
            ),
            (
                "trk_00000000000000000000000003",
                IdKind::Track,
                DocKind::Track,
            ),
            (
                "pls_00000000000000000000000004",
                IdKind::Playlist,
                DocKind::Playlist,
            ),
        ];
        for (text, id_kind, kind) in cases {
            let id = PublicId::parse(text, id_kind).unwrap();
            let doc = DocRef::new(id).unwrap();
            assert_eq!((doc.kind(), doc.id()), (kind, id));
            assert_eq!(DocRef::from_text(text), Some(doc));
        }
    }

    #[test]
    fn nothing_but_an_artist_album_track_or_playlist_is_a_document() {
        let others = [
            ("rgp_00000000000000000000000001", IdKind::ReleaseGroup),
            ("usr_00000000000000000000000001", IdKind::User),
            ("prf_00000000000000000000000001", IdKind::Profile),
            ("dev_00000000000000000000000001", IdKind::Device),
            ("lib_00000000000000000000000001", IdKind::Library),
            ("inv_00000000000000000000000001", IdKind::Invite),
            ("shr_00000000000000000000000001", IdKind::Share),
            ("tok_00000000000000000000000001", IdKind::Token),
        ];
        for (text, id_kind) in others {
            let id = PublicId::parse(text, id_kind).unwrap();
            assert_eq!(DocRef::new(id), None);
            assert_eq!(DocRef::from_text(text), None);
        }
    }

    #[test]
    fn text_that_is_not_one_identifier_is_not_a_document() {
        for text in [
            "",
            "trk_",
            "trk_0000000000000000000000000",
            "trk_000000000000000000000000001",
            "TRK_00000000000000000000000001",
            "trk_0000000000000000000000000L",
            "trk_80000000000000000000000000",
            " trk_00000000000000000000000001",
        ] {
            assert_eq!(DocRef::from_text(text), None);
        }
    }

    #[test]
    fn the_test_builder_writes_each_kind_with_its_own_prefix() {
        assert_eq!(
            DocKind::ALL.map(|kind| id_text(kind, 42)),
            [
                "art_00000000000000000000000042",
                "alb_00000000000000000000000042",
                "trk_00000000000000000000000042",
                "pls_00000000000000000000000042",
            ]
        );
    }
}
