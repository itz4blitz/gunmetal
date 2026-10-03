//! What an item is, and which synced record a change touches.

use super::coded::coded;

coded! {
    /// What kind of item a library entry is (LAT-001).
    ///
    /// Every item carries a kind from the first schema, so later media
    /// arrive as new codes rather than as a migration. A code this release
    /// does not know reads as `None`, and the client leaves such an item
    /// out rather than mistaking it for another kind.
    ItemKind: u16 {
        /// A music track.
        Track = 1,
        /// A chapter or part of an audiobook.
        Audiobook = 2,
        /// An episode of a podcast.
        PodcastEpisode = 3,
        /// A film, an episode or another video.
        Video = 4,
        /// A photo.
        Photo = 5,
        /// A book.
        Book = 6,
        /// A comic.
        Comic = 7,
    }
}

coded! {
    /// Which synced-library record a change or a field belongs to. The
    /// release-group record joins in R1.1 (WP-146).
    RecordKind: u8 {
        /// A track record.
        Track = 1,
        /// An album record.
        Album = 2,
        /// An artist record.
        Artist = 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_kinds_have_these_codes() {
        let codes: Vec<(ItemKind, u16)> = ItemKind::ALL.iter().map(|k| (*k, k.code())).collect();
        assert_eq!(
            codes,
            [
                (ItemKind::Track, 1),
                (ItemKind::Audiobook, 2),
                (ItemKind::PodcastEpisode, 3),
                (ItemKind::Video, 4),
                (ItemKind::Photo, 5),
                (ItemKind::Book, 6),
                (ItemKind::Comic, 7),
            ]
        );
    }

    #[test]
    fn item_kind_codes_read_back_and_unknown_codes_read_as_none() {
        let read: Vec<Option<ItemKind>> = (0..=8).map(ItemKind::from_code).collect();
        assert_eq!(
            read,
            [
                None,
                Some(ItemKind::Track),
                Some(ItemKind::Audiobook),
                Some(ItemKind::PodcastEpisode),
                Some(ItemKind::Video),
                Some(ItemKind::Photo),
                Some(ItemKind::Book),
                Some(ItemKind::Comic),
                None,
            ]
        );
        assert_eq!(ItemKind::from_code(u16::MAX), None);
    }

    #[test]
    fn record_kinds_have_these_codes() {
        let codes: Vec<(RecordKind, u8)> = RecordKind::ALL.iter().map(|k| (*k, k.code())).collect();
        assert_eq!(
            codes,
            [
                (RecordKind::Track, 1),
                (RecordKind::Album, 2),
                (RecordKind::Artist, 3),
            ]
        );
        let read: Vec<Option<RecordKind>> = (0..=4).map(RecordKind::from_code).collect();
        assert_eq!(
            read,
            [
                None,
                Some(RecordKind::Track),
                Some(RecordKind::Album),
                Some(RecordKind::Artist),
                None,
            ]
        );
    }
}
