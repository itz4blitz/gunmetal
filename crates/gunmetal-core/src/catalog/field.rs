//! The field table: a stable numeric code, a name and a value type for
//! every field of the synced-library records.
//!
//! Search names fields by these codes, and from R1.1 the rule format does
//! too (WP-027, register D-85). The rule format defines its own plain
//! `FieldId` and imports nothing from here; its bindings join the two by
//! code. A code is never changed or reused: a field that goes away keeps
//! its code retired. Track fields are numbered from 100, album fields from
//! 200 and artist fields from 300.

use super::kind::RecordKind;

/// The type of a field's value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldType {
    /// A public identifier.
    Id,
    /// A list of public identifiers.
    IdList,
    /// A `MusicBrainz` identifier.
    Mbid,
    /// One text.
    Text,
    /// A list of texts.
    TextList,
    /// A whole number.
    Integer,
    /// A decimal number, such as a gain.
    Decimal,
    /// True or false.
    Boolean,
    /// A date that may lack its month or day.
    Date,
    /// A duration in milliseconds.
    Duration,
    /// An instant in time.
    Timestamp,
    /// The code of a closed vocabulary, such as a codec.
    Code,
    /// A list of codes of a closed vocabulary.
    CodeList,
}

/// Declares the field table. Each entry is a documented variant, its code,
/// its record, its name and its type, so one line holds everything about a
/// field. Mutation testing does not see code a macro generates, so the
/// tests pin every generated value against a literal table.
macro_rules! fields {
    ($(
        $(#[$doc:meta])*
        $variant:ident = ($code:literal, $record:ident, $name:literal, $ty:ident),
    )*) => {
        /// One field of a synced-library record.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum CatalogField {
            $( $(#[$doc])* $variant, )*
        }

        impl CatalogField {
            /// Every field, in order of its code.
            pub const ALL: &'static [Self] = &[$(Self::$variant),*];

            /// The stable code, which never changes and is never reused.
            #[must_use]
            pub const fn code(self) -> u16 {
                match self {
                    $(Self::$variant => $code,)*
                }
            }

            /// The field with code `code`, or `None` for a code this
            /// release does not know.
            #[must_use]
            pub const fn from_code(code: u16) -> Option<Self> {
                match code {
                    $($code => Some(Self::$variant),)*
                    _ => None,
                }
            }

            /// The record the field belongs to.
            #[must_use]
            pub const fn record(self) -> RecordKind {
                match self {
                    $(Self::$variant => RecordKind::$record,)*
                }
            }

            /// The field's name, unique within its record.
            #[must_use]
            pub const fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => $name,)*
                }
            }

            /// The type of the field's value.
            #[must_use]
            pub const fn value_type(self) -> FieldType {
                match self {
                    $(Self::$variant => FieldType::$ty,)*
                }
            }
        }
    };
}

fields! {
    /// A track's identifier.
    TrackId = (100, Track, "id", Id),
    /// A track's item kind.
    TrackKind = (101, Track, "kind", Code),
    /// A track's library.
    TrackLibrary = (102, Track, "library", Id),
    /// A track's title.
    TrackTitle = (103, Track, "title", Text),
    /// A track's title to sort by.
    TrackTitleSort = (104, Track, "title_sort", Text),
    /// A track's artist credit as tagged.
    TrackArtistCredit = (105, Track, "artist_credit", Text),
    /// A track's credited artists.
    TrackArtists = (106, Track, "artists", IdList),
    /// A track's album.
    TrackAlbum = (107, Track, "album", Id),
    /// A track's number.
    TrackNumber = (108, Track, "track_number", Integer),
    /// The number of tracks.
    TrackTotal = (109, Track, "track_total", Integer),
    /// A track's disc number.
    TrackDiscNumber = (110, Track, "disc_number", Integer),
    /// The number of discs.
    TrackDiscTotal = (111, Track, "disc_total", Integer),
    /// A track's disc title.
    TrackDiscSubtitle = (112, Track, "disc_subtitle", Text),
    /// A track's release date.
    TrackDate = (113, Track, "date", Date),
    /// A track's original release date.
    TrackOriginalDate = (114, Track, "original_date", Date),
    /// A track's genres.
    TrackGenres = (115, Track, "genres", TextList),
    /// A track's moods.
    TrackMoods = (116, Track, "moods", TextList),
    /// A track's styles.
    TrackStyles = (117, Track, "styles", TextList),
    /// A track's record labels.
    TrackLabels = (118, Track, "labels", TextList),
    /// A track's groupings.
    TrackGrouping = (119, Track, "grouping", TextList),
    /// A track's content advisory.
    TrackAdvisory = (120, Track, "advisory", Code),
    /// A track's recording codes.
    TrackIsrc = (121, Track, "isrc", TextList),
    /// A track's recording `MusicBrainz` identifier.
    TrackRecordingMbid = (122, Track, "recording_mbid", Mbid),
    /// A track's codec.
    TrackCodec = (123, Track, "codec", Code),
    /// A track's container.
    TrackContainer = (124, Track, "container", Code),
    /// A track's sample rate.
    TrackSampleRate = (125, Track, "sample_rate", Integer),
    /// A track's bit depth.
    TrackBitDepth = (126, Track, "bit_depth", Integer),
    /// A track's channel count.
    TrackChannels = (127, Track, "channels", Integer),
    /// A track's bitrate.
    TrackBitrate = (128, Track, "bitrate", Integer),
    /// A track's duration.
    TrackDuration = (129, Track, "duration", Duration),
    /// The reference of a track's track gain.
    TrackGainScale = (130, Track, "track_gain_scale", Code),
    /// A track's track gain.
    TrackGain = (131, Track, "track_gain", Decimal),
    /// A track's track peak.
    TrackPeak = (132, Track, "track_peak", Decimal),
    /// The reference of a track's album gain.
    TrackAlbumGainScale = (133, Track, "album_gain_scale", Code),
    /// A track's album gain.
    TrackAlbumGain = (134, Track, "album_gain", Decimal),
    /// A track's album peak.
    TrackAlbumPeak = (135, Track, "album_peak", Decimal),
    /// A track's encoder delay.
    TrackTrimDelay = (136, Track, "trim_delay", Integer),
    /// A track's encoder padding.
    TrackTrimPadding = (137, Track, "trim_padding", Integer),
    /// How a track's lyrics are timed.
    TrackLyricsTiming = (138, Track, "lyrics_timing", Code),
    /// Whether a track can be played now.
    TrackAvailability = (139, Track, "availability", Code),
    /// When a track was added.
    TrackAdded = (140, Track, "added", Timestamp),
    /// Where a track's lyrics were found.
    TrackLyricsOrigin = (141, Track, "lyrics_origin", Code),
    /// An album's identifier.
    AlbumId = (200, Album, "id", Id),
    /// An album's library.
    AlbumLibrary = (201, Album, "library", Id),
    /// An album's title.
    AlbumTitle = (202, Album, "title", Text),
    /// An album's title to sort by.
    AlbumTitleSort = (203, Album, "title_sort", Text),
    /// An album's artist credit as tagged.
    AlbumArtistCredit = (204, Album, "artist_credit", Text),
    /// An album's credited artists.
    AlbumArtists = (205, Album, "artists", IdList),
    /// An album's release date.
    AlbumDate = (206, Album, "date", Date),
    /// An album's original release date.
    AlbumOriginalDate = (207, Album, "original_date", Date),
    /// An album's primary type.
    AlbumPrimaryType = (208, Album, "primary_type", Code),
    /// An album's secondary types.
    AlbumSecondaryTypes = (209, Album, "secondary_types", CodeList),
    /// Whether an album is a compilation.
    AlbumCompilation = (210, Album, "compilation", Boolean),
    /// An album's genres.
    AlbumGenres = (211, Album, "genres", TextList),
    /// An album's record labels.
    AlbumLabels = (212, Album, "labels", TextList),
    /// An album's track count.
    AlbumTrackCount = (213, Album, "track_count", Integer),
    /// An album's disc count.
    AlbumDiscCount = (214, Album, "disc_count", Integer),
    /// An album's duration.
    AlbumDuration = (215, Album, "duration", Duration),
    /// Whether an album has artwork.
    AlbumHasArtwork = (216, Album, "has_artwork", Boolean),
    /// An album's release `MusicBrainz` identifier.
    AlbumReleaseMbid = (217, Album, "release_mbid", Mbid),
    /// An album's release group `MusicBrainz` identifier.
    AlbumReleaseGroupMbid = (218, Album, "release_group_mbid", Mbid),
    /// When an album was added.
    AlbumAdded = (219, Album, "added", Timestamp),
    /// An artist's identifier.
    ArtistId = (300, Artist, "id", Id),
    /// An artist's library.
    ArtistLibrary = (301, Artist, "library", Id),
    /// An artist's name.
    ArtistName = (302, Artist, "name", Text),
    /// An artist's name to sort by.
    ArtistNameSort = (303, Artist, "name_sort", Text),
    /// An artist's `MusicBrainz` identifier.
    ArtistMbid = (304, Artist, "mbid", Mbid),
    /// An artist's album count.
    ArtistAlbumCount = (305, Artist, "album_count", Integer),
    /// An artist's track count.
    ArtistTrackCount = (306, Artist, "track_count", Integer),
    /// An artist's genres.
    ArtistGenres = (307, Artist, "genres", TextList),
    /// Whether an artist has artwork.
    ArtistHasArtwork = (308, Artist, "has_artwork", Boolean),
}

#[cfg(test)]
mod tests {
    use super::*;

    use CatalogField as F;
    use FieldType::{
        Boolean, Code, CodeList, Date, Decimal, Duration, Id, IdList, Integer, Mbid, Text,
        TextList, Timestamp,
    };
    use RecordKind::{Album, Artist, Track};

    /// One row of the literal table: the variant callers name, then its
    /// code, record, name and type.
    type Row = (CatalogField, u16, RecordKind, &'static str, FieldType);

    /// The whole field table, written out independently of the declaration
    /// above, in order. Each row names its variant, so a declaration that
    /// gives a variant another field's code, name or type fails here.
    const TABLE: [Row; 71] = [
        (F::TrackId, 100, Track, "id", Id),
        (F::TrackKind, 101, Track, "kind", Code),
        (F::TrackLibrary, 102, Track, "library", Id),
        (F::TrackTitle, 103, Track, "title", Text),
        (F::TrackTitleSort, 104, Track, "title_sort", Text),
        (F::TrackArtistCredit, 105, Track, "artist_credit", Text),
        (F::TrackArtists, 106, Track, "artists", IdList),
        (F::TrackAlbum, 107, Track, "album", Id),
        (F::TrackNumber, 108, Track, "track_number", Integer),
        (F::TrackTotal, 109, Track, "track_total", Integer),
        (F::TrackDiscNumber, 110, Track, "disc_number", Integer),
        (F::TrackDiscTotal, 111, Track, "disc_total", Integer),
        (F::TrackDiscSubtitle, 112, Track, "disc_subtitle", Text),
        (F::TrackDate, 113, Track, "date", Date),
        (F::TrackOriginalDate, 114, Track, "original_date", Date),
        (F::TrackGenres, 115, Track, "genres", TextList),
        (F::TrackMoods, 116, Track, "moods", TextList),
        (F::TrackStyles, 117, Track, "styles", TextList),
        (F::TrackLabels, 118, Track, "labels", TextList),
        (F::TrackGrouping, 119, Track, "grouping", TextList),
        (F::TrackAdvisory, 120, Track, "advisory", Code),
        (F::TrackIsrc, 121, Track, "isrc", TextList),
        (F::TrackRecordingMbid, 122, Track, "recording_mbid", Mbid),
        (F::TrackCodec, 123, Track, "codec", Code),
        (F::TrackContainer, 124, Track, "container", Code),
        (F::TrackSampleRate, 125, Track, "sample_rate", Integer),
        (F::TrackBitDepth, 126, Track, "bit_depth", Integer),
        (F::TrackChannels, 127, Track, "channels", Integer),
        (F::TrackBitrate, 128, Track, "bitrate", Integer),
        (F::TrackDuration, 129, Track, "duration", Duration),
        (F::TrackGainScale, 130, Track, "track_gain_scale", Code),
        (F::TrackGain, 131, Track, "track_gain", Decimal),
        (F::TrackPeak, 132, Track, "track_peak", Decimal),
        (F::TrackAlbumGainScale, 133, Track, "album_gain_scale", Code),
        (F::TrackAlbumGain, 134, Track, "album_gain", Decimal),
        (F::TrackAlbumPeak, 135, Track, "album_peak", Decimal),
        (F::TrackTrimDelay, 136, Track, "trim_delay", Integer),
        (F::TrackTrimPadding, 137, Track, "trim_padding", Integer),
        (F::TrackLyricsTiming, 138, Track, "lyrics_timing", Code),
        (F::TrackAvailability, 139, Track, "availability", Code),
        (F::TrackAdded, 140, Track, "added", Timestamp),
        (F::TrackLyricsOrigin, 141, Track, "lyrics_origin", Code),
        (F::AlbumId, 200, Album, "id", Id),
        (F::AlbumLibrary, 201, Album, "library", Id),
        (F::AlbumTitle, 202, Album, "title", Text),
        (F::AlbumTitleSort, 203, Album, "title_sort", Text),
        (F::AlbumArtistCredit, 204, Album, "artist_credit", Text),
        (F::AlbumArtists, 205, Album, "artists", IdList),
        (F::AlbumDate, 206, Album, "date", Date),
        (F::AlbumOriginalDate, 207, Album, "original_date", Date),
        (F::AlbumPrimaryType, 208, Album, "primary_type", Code),
        (
            F::AlbumSecondaryTypes,
            209,
            Album,
            "secondary_types",
            CodeList,
        ),
        (F::AlbumCompilation, 210, Album, "compilation", Boolean),
        (F::AlbumGenres, 211, Album, "genres", TextList),
        (F::AlbumLabels, 212, Album, "labels", TextList),
        (F::AlbumTrackCount, 213, Album, "track_count", Integer),
        (F::AlbumDiscCount, 214, Album, "disc_count", Integer),
        (F::AlbumDuration, 215, Album, "duration", Duration),
        (F::AlbumHasArtwork, 216, Album, "has_artwork", Boolean),
        (F::AlbumReleaseMbid, 217, Album, "release_mbid", Mbid),
        (
            F::AlbumReleaseGroupMbid,
            218,
            Album,
            "release_group_mbid",
            Mbid,
        ),
        (F::AlbumAdded, 219, Album, "added", Timestamp),
        (F::ArtistId, 300, Artist, "id", Id),
        (F::ArtistLibrary, 301, Artist, "library", Id),
        (F::ArtistName, 302, Artist, "name", Text),
        (F::ArtistNameSort, 303, Artist, "name_sort", Text),
        (F::ArtistMbid, 304, Artist, "mbid", Mbid),
        (F::ArtistAlbumCount, 305, Artist, "album_count", Integer),
        (F::ArtistTrackCount, 306, Artist, "track_count", Integer),
        (F::ArtistGenres, 307, Artist, "genres", TextList),
        (F::ArtistHasArtwork, 308, Artist, "has_artwork", Boolean),
    ];

    #[test]
    fn every_field_has_the_listed_code_record_name_and_type() {
        let declared: Vec<Row> = CatalogField::ALL
            .iter()
            .map(|f| (*f, f.code(), f.record(), f.name(), f.value_type()))
            .collect();
        assert_eq!(declared, TABLE);
    }

    #[test]
    fn every_code_reads_back_as_its_field() {
        let read: Vec<Option<CatalogField>> = TABLE
            .iter()
            .map(|(_, code, ..)| CatalogField::from_code(*code))
            .collect();
        let expected: Vec<Option<CatalogField>> =
            TABLE.iter().map(|(field, ..)| Some(*field)).collect();
        assert_eq!(read, expected);
    }

    #[test]
    fn codes_outside_the_table_read_as_none() {
        let unknown: Vec<Option<CatalogField>> = [0, 99, 142, 199, 220, 299, 309, u16::MAX]
            .into_iter()
            .map(CatalogField::from_code)
            .collect();
        assert_eq!(unknown, [None; 8]);
    }

    #[test]
    fn codes_are_unique_and_names_unique_within_a_record() {
        let mut codes: Vec<u16> = TABLE.iter().map(|(_, code, ..)| *code).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), TABLE.len());
        let mut names: Vec<(RecordKind, &str)> = TABLE
            .iter()
            .map(|(_, _, record, name, _)| (*record, *name))
            .collect();
        names.sort_unstable_by_key(|(record, name)| (record.code(), *name));
        names.dedup();
        assert_eq!(names.len(), TABLE.len());
    }

    #[test]
    fn each_record_numbers_its_fields_in_its_own_hundred() {
        let outside: Vec<(u16, RecordKind)> = TABLE
            .iter()
            .map(|(_, code, record, ..)| (*code, *record))
            .filter(|(code, record)| code / 100 != u16::from(record.code()))
            .collect();
        assert_eq!(outside, []);
    }
}
