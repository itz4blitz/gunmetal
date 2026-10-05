//! APE tag items mapped onto [`TrackTags`](crate::catalog::TrackTags).
//!
//! [`from_ape`] maps the items the APE parser read
//! ([`crate::formats::ape`]). Keys are matched without regard to case, as
//! the `APEv2` specification compares them. A text item gives one value for
//! each of its zero-separated values; binary items, external locators and
//! items of the reserved type are not mapped. The keys are the ones the
//! specification lists and the ones Picard and foobar2000 write for
//! `MusicBrainz` identifiers, `ReplayGain`, sort names and credits. `Track`
//! and `Disc` hold a number alone or with its total, as `3/12`, and `Year`
//! holds the release date.
//!
//! The values follow the field rules in [`super::mp4`]: text is cleaned and
//! cut to its limit, lists stop at the tag-field limit (SEC-MED-006), and a
//! number, date, identifier or gain outside its range is dropped with the
//! reason (SEC-MED-014).
//!
//! # Values that were cut
//!
//! The APE parser has already cut every text value to the long-text limit
//! and flagged what it cut, and the field rules cut what they are given to
//! the field's own limit. Every value that was cut is recorded once as
//! [`Reason::Truncated`](super::mp4::Reason::Truncated) (SEC-MED-006), by
//! the rule every tag mapper follows ("Values that were cut" in
//! [`super::mp4`]):
//!
//! - A value kept as text is kept, cut, and recorded with the limit it was
//!   last cut at: the short-text limit when the field rules cut it, whether
//!   or not the parser had cut it first, and otherwise the long-text limit.
//!   Lyrics are kept under the long-text limit, so theirs is always that
//!   one.
//! - A value read as a number, a date, a flag, a release type, a recording
//!   code, an identifier, a gain or a peak is not read once it was cut,
//!   because what is left is not what was written: a `Track` of `31` cut to
//!   `3` would read as track 3. It is recorded with the long-text limit
//!   when the parser cut it and with the short-text limit when only the
//!   field rules did, nothing else is recorded for it, and the next value
//!   is tried.
//!
//! # Work
//!
//! [`from_ape`] charges the [`Budget`] one step for each item and one for
//! each of its text values, before it reads them (SEC-MED-007). An item
//! takes at least nine octets in the file and every further value at least
//! one, so a tag of `n` octets costs at most `n` steps. When the budget is
//! spent the mapping stops and says where.

use super::mp4::{Fields, ItemIndex, Mapped, TagField, lookup};
use crate::catalog::{LyricsOrigin, Role};
use crate::formats::ape::{ApeTag, ApeValue};
use crate::parse::{Budget, LimitKind, Limits};

/// The keys that are mapped.
const KEYS: &[(&str, TagField)] = &[
    ("Title", TagField::Title),
    ("Artist", TagField::Artist),
    ("Album", TagField::Album),
    ("Album Artist", TagField::AlbumArtist),
    ("AlbumArtist", TagField::AlbumArtist),
    ("Track", TagField::Track),
    ("Disc", TagField::Disc),
    ("DiscSubtitle", TagField::DiscSubtitle),
    ("Year", TagField::Date),
    ("OriginalDate", TagField::OriginalDate),
    ("OriginalYear", TagField::OriginalDate),
    ("Genre", TagField::Genres),
    ("Mood", TagField::Moods),
    ("Label", TagField::Labels),
    ("Grouping", TagField::Grouping),
    ("Composer", TagField::Credit(Role::Composer)),
    ("Conductor", TagField::Credit(Role::Conductor)),
    ("Lyricist", TagField::Credit(Role::Lyricist)),
    ("MixArtist", TagField::Credit(Role::Remixer)),
    ("Producer", TagField::Credit(Role::Producer)),
    ("Arranger", TagField::Credit(Role::Arranger)),
    ("Engineer", TagField::Credit(Role::Engineer)),
    ("Mixer", TagField::Credit(Role::Mixer)),
    ("DJMixer", TagField::Credit(Role::DjMixer)),
    ("Performer", TagField::Credit(Role::Performer)),
    ("Compilation", TagField::Compilation),
    ("ISRC", TagField::Isrc),
    ("Lyrics", TagField::Lyrics(LyricsOrigin::ApeItem)),
    ("TitleSort", TagField::TitleSort),
    ("ArtistSort", TagField::ArtistSort),
    ("AlbumArtistSort", TagField::AlbumArtistSort),
    ("AlbumSort", TagField::AlbumSort),
    ("MUSICBRAINZ_TRACKID", TagField::RecordingMbid),
    ("MUSICBRAINZ_RELEASETRACKID", TagField::TrackMbid),
    ("MUSICBRAINZ_ALBUMID", TagField::ReleaseMbid),
    ("MUSICBRAINZ_RELEASEGROUPID", TagField::ReleaseGroupMbid),
    ("MUSICBRAINZ_ARTISTID", TagField::ArtistMbids),
    ("MUSICBRAINZ_ALBUMARTISTID", TagField::AlbumArtistMbids),
    ("MUSICBRAINZ_ALBUMTYPE", TagField::ReleaseType),
    ("ReleaseType", TagField::ReleaseType),
    ("REPLAYGAIN_TRACK_GAIN", TagField::TrackGain),
    ("REPLAYGAIN_TRACK_PEAK", TagField::TrackPeak),
    ("REPLAYGAIN_ALBUM_GAIN", TagField::AlbumGain),
    ("REPLAYGAIN_ALBUM_PEAK", TagField::AlbumPeak),
];

/// Maps the items of an APE tag onto
/// [`TrackTags`](crate::catalog::TrackTags).
///
/// The budget is charged one step for each item and one for each of its
/// text values.
#[must_use]
pub fn from_ape(tag: &ApeTag, limits: &Limits, budget: &mut Budget) -> Mapped<ItemIndex> {
    let mut fields = Fields::new(limits);
    for (index, item) in tag.items.iter().enumerate() {
        let source = ItemIndex(index);
        let values = match &item.value {
            ApeValue::Text(values) => values.as_slice(),
            ApeValue::Binary(_) | ApeValue::Locator(_) | ApeValue::Reserved(_) => &[],
        };
        if !fields.charge(budget, values.len(), source) {
            break;
        }
        if let Some(field) = lookup(KEYS, &item.key) {
            for value in values {
                let cut = value.truncated.then_some(LimitKind::LongText);
                fields.set(field, &value.value, cut, source);
            }
        }
    }
    fields.finish()
}

#[cfg(test)]
mod tests {
    use super::super::mp4::{Reason, TagProblem};
    use super::*;
    use crate::catalog::{
        Credit, Gain, GainScale, GainTags, LyricsSource, LyricsTiming, MbIds, PrimaryType,
        ReleaseType, SecondaryType, TagLyrics, TrackPosition, TrackTags,
    };
    use crate::formats::ape::{ApeItem, parse_ape};
    use crate::parse::Window;
    use crate::text::Text;
    use crate::untrusted::Untrusted;
    use crate::values::{GainDb, Isrc, Mbid, PartialDate, PeakRatio};
    use gunmetal_testkit::ape::Ape;
    use proptest::collection::vec;
    use proptest::prelude::*;
    use proptest::sample::select;

    fn text(value: &str) -> Text {
        Text {
            value: value.to_owned(),
            truncated: false,
            replaced: false,
        }
    }

    fn item(key: &str, values: &[&str]) -> ApeItem {
        ApeItem {
            key: key.to_owned(),
            value: ApeValue::Text(values.iter().map(|value| text(value)).collect()),
        }
    }

    fn tag(items: Vec<ApeItem>) -> ApeTag {
        ApeTag {
            range: 0..0,
            items,
            problems: Vec::new(),
        }
    }

    fn map(items: Vec<ApeItem>) -> Mapped<ItemIndex> {
        from_ape(
            &tag(items),
            &Limits::DEFAULT,
            &mut Budget::for_input(0, 0, 1_000),
        )
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn id(text: &str) -> Mbid {
        Mbid::parse(Untrusted::new(text)).expect("the test's identifier is well formed")
    }

    fn day(year: u16, month: Option<u8>, day: Option<u8>) -> PartialDate {
        PartialDate::new(year, month, day).expect("the test's date exists")
    }

    fn credited(name: &str, role: Role, detail: Option<&str>) -> Credit {
        Credit::new(name.to_owned(), role, detail.map(str::to_owned), None)
            .expect("the test's name is not blank")
    }

    fn replay_gain(db: f32, peak: f32) -> Gain {
        Gain {
            scale: GainScale::ReplayGain,
            gain: GainDb::new(db).expect("the test's gain is in range"),
            peak: Some(PeakRatio::new(peak).expect("the test's peak is in range")),
        }
    }

    /// What mapping gives when only `tags` were read, each from the item
    /// beside it.
    fn mapped(tags: TrackTags, sources: Vec<(TagField, usize)>) -> Mapped<ItemIndex> {
        Mapped {
            tags,
            sources: sources
                .into_iter()
                .map(|(field, index)| (field, ItemIndex(index)))
                .collect(),
            problems: Vec::new(),
        }
    }

    const RECORDING: &str = "11111111-1111-4111-8111-111111111111";
    const TRACK: &str = "22222222-2222-4222-8222-222222222222";
    const RELEASE: &str = "33333333-3333-4333-8333-333333333333";
    const GROUP: &str = "44444444-4444-4444-8444-444444444444";
    const ARTIST: &str = "55555555-5555-4555-8555-555555555555";
    const SECOND_ARTIST: &str = "66666666-6666-4666-8666-666666666666";
    const ALBUM_ARTIST: &str = "77777777-7777-4777-8777-777777777777";

    #[test]
    fn maps_the_names_numbers_and_dates() {
        let read = map(vec![
            item("Title", &["Blue in Green"]),
            item("Artist", &["Miles Davis", "Bill Evans"]),
            item("Album", &["Kind of Blue"]),
            item("Album Artist", &["Miles Davis"]),
            item("AlbumArtist", &["Sextet"]),
            item("Track", &["3/5"]),
            item("Disc", &["1"]),
            item("DiscSubtitle", &["Side one"]),
            item("Year", &["1959-08-17"]),
            item("OriginalDate", &["1959-08"]),
            item("OriginalYear", &["1960"]),
            item("Genre", &["Jazz", "Modal"]),
            item("Mood", &["Calm"]),
            item("Label", &["Columbia"]),
            item("Grouping", &["Sessions"]),
            item("TitleSort", &["Blue in Green, 1959"]),
            item("ArtistSort", &["Davis, Miles"]),
            item("AlbumArtistSort", &["Davis"]),
            item("AlbumSort", &["Kind of Blue, 1959"]),
            item("Comment", &["not mapped"]),
        ]);
        assert_eq!(
            read,
            mapped(
                TrackTags {
                    title: Some(String::from("Blue in Green")),
                    title_sort: Some(String::from("Blue in Green, 1959")),
                    artist: strings(&["Miles Davis", "Bill Evans"]),
                    artist_sort: strings(&["Davis, Miles"]),
                    album_artist: strings(&["Miles Davis", "Sextet"]),
                    album_artist_sort: strings(&["Davis"]),
                    album: Some(String::from("Kind of Blue")),
                    album_sort: Some(String::from("Kind of Blue, 1959")),
                    position: TrackPosition::new(Some(3), Some(5), Some(1), None)
                        .expect("the position is in range"),
                    disc_subtitle: Some(String::from("Side one")),
                    date: Some(day(1959, Some(8), Some(17))),
                    original_date: Some(day(1959, Some(8), None)),
                    genres: strings(&["Jazz", "Modal"]),
                    moods: strings(&["Calm"]),
                    labels: strings(&["Columbia"]),
                    grouping: strings(&["Sessions"]),
                    ..TrackTags::default()
                },
                vec![
                    (TagField::Title, 0),
                    (TagField::Artist, 1),
                    (TagField::Album, 2),
                    (TagField::AlbumArtist, 3),
                    (TagField::AlbumArtist, 4),
                    (TagField::Track, 5),
                    (TagField::TrackTotal, 5),
                    (TagField::Disc, 6),
                    (TagField::DiscSubtitle, 7),
                    (TagField::Date, 8),
                    (TagField::OriginalDate, 9),
                    (TagField::Genres, 11),
                    (TagField::Moods, 12),
                    (TagField::Labels, 13),
                    (TagField::Grouping, 14),
                    (TagField::TitleSort, 15),
                    (TagField::ArtistSort, 16),
                    (TagField::AlbumArtistSort, 17),
                    (TagField::AlbumSort, 18),
                ],
            )
        );
    }

    #[test]
    fn maps_the_credits_flags_and_lyrics() {
        let read = map(vec![
            item("Composer", &["A"]),
            item("Conductor", &["B"]),
            item("Lyricist", &["C"]),
            item("MixArtist", &["D"]),
            item("Producer", &["E"]),
            item("Arranger", &["F"]),
            item("Engineer", &["G"]),
            item("Mixer", &["H"]),
            item("DJMixer", &["I"]),
            item("Performer", &["Jaco Pastorius (fretless bass)"]),
            item("Compilation", &["1"]),
            item("ISRC", &["USS1Z9900001"]),
            item("Lyrics", &["first\nsecond"]),
        ]);
        let roles = [
            Role::Composer,
            Role::Conductor,
            Role::Lyricist,
            Role::Remixer,
            Role::Producer,
            Role::Arranger,
            Role::Engineer,
            Role::Mixer,
            Role::DjMixer,
        ];
        let mut credits: Vec<Credit> = roles
            .into_iter()
            .zip(["A", "B", "C", "D", "E", "F", "G", "H", "I"])
            .map(|(role, name)| credited(name, role, None))
            .collect();
        credits.push(credited(
            "Jaco Pastorius",
            Role::Performer,
            Some("fretless bass"),
        ));
        let lyrics = TagField::Lyrics(LyricsOrigin::ApeItem);
        let fields = roles.into_iter().map(TagField::Credit).chain([
            TagField::Credit(Role::Performer),
            TagField::Compilation,
            TagField::Isrc,
            lyrics,
        ]);
        assert_eq!(
            read,
            mapped(
                TrackTags {
                    credits,
                    compilation: Some(true),
                    isrc: vec![
                        Isrc::parse(Untrusted::new("USS1Z9900001")).expect("the code is valid")
                    ],
                    lyrics: vec![TagLyrics {
                        source: LyricsSource::new(LyricsOrigin::ApeItem, LyricsTiming::Plain)
                            .expect("the lyrics are not from a SYLT frame"),
                        text: String::from("first\nsecond"),
                    }],
                    ..TrackTags::default()
                },
                fields.zip(0..13).collect(),
            )
        );
    }

    #[test]
    fn maps_the_identifiers_release_types_and_gains() {
        let read = map(vec![
            item("MUSICBRAINZ_TRACKID", &[RECORDING]),
            item("MUSICBRAINZ_RELEASETRACKID", &[TRACK]),
            item("MUSICBRAINZ_ALBUMID", &[RELEASE]),
            item("MUSICBRAINZ_RELEASEGROUPID", &[GROUP]),
            item("MUSICBRAINZ_ARTISTID", &[ARTIST, SECOND_ARTIST]),
            item("MUSICBRAINZ_ALBUMARTISTID", &[ALBUM_ARTIST]),
            item("MUSICBRAINZ_ALBUMTYPE", &["album"]),
            item("ReleaseType", &["live"]),
            item("REPLAYGAIN_TRACK_GAIN", &["-6.50 dB"]),
            item("REPLAYGAIN_TRACK_PEAK", &["0.5"]),
            item("REPLAYGAIN_ALBUM_GAIN", &["+1.25 dB"]),
            item("REPLAYGAIN_ALBUM_PEAK", &["1.0"]),
        ]);
        let fields = [
            TagField::RecordingMbid,
            TagField::TrackMbid,
            TagField::ReleaseMbid,
            TagField::ReleaseGroupMbid,
            TagField::ArtistMbids,
            TagField::AlbumArtistMbids,
            TagField::ReleaseType,
            TagField::ReleaseType,
            TagField::TrackGain,
            TagField::TrackPeak,
            TagField::AlbumGain,
            TagField::AlbumPeak,
        ];
        assert_eq!(
            read,
            mapped(
                TrackTags {
                    release_type: Some(ReleaseType {
                        primary: Some(PrimaryType::Album),
                        secondary: vec![SecondaryType::Live],
                    }),
                    musicbrainz: MbIds {
                        recording: Some(id(RECORDING)),
                        track: Some(id(TRACK)),
                        release: Some(id(RELEASE)),
                        release_group: Some(id(GROUP)),
                        artists: vec![id(ARTIST), id(SECOND_ARTIST)],
                        album_artists: vec![id(ALBUM_ARTIST)],
                    },
                    gain: GainTags {
                        track: Some(replay_gain(-6.5, 0.5)),
                        album: Some(replay_gain(1.25, 1.0)),
                    },
                    ..TrackTags::default()
                },
                fields.into_iter().zip(0..12).collect(),
            )
        );
    }

    #[test]
    fn a_key_that_differs_only_in_case_is_the_same_key() {
        let read = map(vec![
            item("TITLE", &["Shouted"]),
            item("title", &["whispered"]),
            item("gEnRe", &["Jazz"]),
            item("GENRE", &["Modal"]),
            item("musicbrainz_albumid", &[RELEASE]),
        ]);
        assert_eq!(
            read,
            mapped(
                TrackTags {
                    title: Some(String::from("Shouted")),
                    genres: strings(&["Jazz", "Modal"]),
                    musicbrainz: MbIds {
                        release: Some(id(RELEASE)),
                        ..MbIds::default()
                    },
                    ..TrackTags::default()
                },
                vec![
                    (TagField::Title, 0),
                    (TagField::Genres, 2),
                    (TagField::Genres, 3),
                    (TagField::ReleaseMbid, 4),
                ],
            )
        );
    }

    #[test]
    fn maps_only_text_items() {
        let other = |key: &str, value| ApeItem {
            key: key.to_owned(),
            value,
        };
        let read = map(vec![
            other("Title", ApeValue::Binary(10..20)),
            other("Artist", ApeValue::Locator(vec![text("https://example")])),
            other("Album", ApeValue::Reserved(20..30)),
            item("Genre", &["Jazz"]),
        ]);
        assert_eq!(
            read,
            mapped(
                TrackTags {
                    genres: strings(&["Jazz"]),
                    ..TrackTags::default()
                },
                vec![(TagField::Genres, 3)],
            )
        );
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn drops_values_outside_their_ranges_with_the_reason() {
        use crate::values::{Field, ValueError};
        let read = map(vec![
            item("Track", &["12000/12"]),
            item("MUSICBRAINZ_ALBUMID", &["../../x"]),
            item("REPLAYGAIN_TRACK_GAIN", &["inf"]),
            item("Year", &["0000"]),
        ]);
        let problem = |field, index, error| TagProblem::Value {
            field,
            source: ItemIndex(index),
            reason: Reason::Invalid(error),
        };
        assert_eq!(
            read,
            Mapped {
                tags: TrackTags {
                    position: TrackPosition::new(None, Some(12), None, None)
                        .expect("the position is in range"),
                    ..TrackTags::default()
                },
                sources: vec![(TagField::TrackTotal, ItemIndex(0))],
                problems: vec![
                    problem(
                        TagField::Track,
                        0,
                        ValueError::OutOfRange {
                            field: Field::Number,
                            value: 12_000
                        }
                    ),
                    problem(
                        TagField::ReleaseMbid,
                        1,
                        ValueError::Malformed { field: Field::Mbid }
                    ),
                    problem(
                        TagField::TrackGain,
                        2,
                        ValueError::Malformed { field: Field::Gain }
                    ),
                    problem(
                        TagField::Date,
                        3,
                        ValueError::OutOfRange {
                            field: Field::Year,
                            value: 0
                        }
                    ),
                ],
            }
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn a_list_stops_at_the_tag_field_limit() {
        let limits = Limits::DEFAULT
            .with_override(LimitKind::TagFields, 2)
            .expect("the test lowers a limit");
        let items = vec![
            item("Genre", &["One", "Two"]),
            item("Genre", &["Three"]),
            item("Mood", &["One", "Two", "Three"]),
        ];
        let full = |field, index| TagProblem::Value {
            field,
            source: ItemIndex(index),
            reason: Reason::Limit(LimitKind::TagFields),
        };
        assert_eq!(
            from_ape(&tag(items), &limits, &mut Budget::for_input(0, 0, 100)),
            Mapped {
                tags: TrackTags {
                    genres: strings(&["One", "Two"]),
                    moods: strings(&["One", "Two"]),
                    ..TrackTags::default()
                },
                sources: vec![
                    (TagField::Genres, ItemIndex(0)),
                    (TagField::Moods, ItemIndex(2)),
                ],
                problems: vec![full(TagField::Genres, 1), full(TagField::Moods, 2)],
            }
        );
    }

    #[test]
    fn charges_one_step_for_each_item_and_each_text_value() {
        let items = || {
            vec![
                item("Title", &["Title"]),
                ApeItem {
                    key: String::from("Cover Art (Front)"),
                    value: ApeValue::Binary(0..10),
                },
                item("Genre", &["Jazz", "Modal"]),
                item("Album", &["Album"]),
            ]
        };
        let mut enough = Budget::for_input(0, 0, 8);
        assert_eq!(
            from_ape(&tag(items()), &Limits::DEFAULT, &mut enough),
            mapped(
                TrackTags {
                    title: Some(String::from("Title")),
                    album: Some(String::from("Album")),
                    genres: strings(&["Jazz", "Modal"]),
                    ..TrackTags::default()
                },
                vec![
                    (TagField::Title, 0),
                    (TagField::Genres, 2),
                    (TagField::Album, 3),
                ],
            )
        );
        assert_eq!(enough.remaining(), 0);

        let mut short = Budget::for_input(0, 0, 7);
        assert_eq!(
            from_ape(&tag(items()), &Limits::DEFAULT, &mut short),
            Mapped {
                tags: TrackTags {
                    title: Some(String::from("Title")),
                    genres: strings(&["Jazz", "Modal"]),
                    ..TrackTags::default()
                },
                sources: vec![
                    (TagField::Title, ItemIndex(0)),
                    (TagField::Genres, ItemIndex(2)),
                ],
                problems: vec![TagProblem::BudgetSpent {
                    source: ItemIndex(3)
                }],
            }
        );
        assert_eq!(short.remaining(), 0);
    }

    #[test]
    fn an_empty_tag_maps_to_no_tags() {
        assert_eq!(map(Vec::new()), mapped(TrackTags::default(), Vec::new()));
    }

    /// The default limits with the short-text and long-text limits set.
    fn text_limits(short: u64, long: u64) -> Limits {
        Limits::DEFAULT
            .with_override(LimitKind::ShortText, short)
            .and_then(|limits| limits.with_override(LimitKind::LongText, long))
            .expect("the test lowers both limits")
    }

    /// Maps what the APE parser reads from a file that is `ape` and nothing
    /// else, with the parser and the mapper under the same `limits`.
    fn parsed(ape: &Ape, limits: &Limits) -> Mapped<ItemIndex> {
        let bytes = ape.build();
        let window = Window {
            offset: 0,
            bytes: &bytes,
            file_len: u64::try_from(bytes.len()).expect("the tag is short"),
        };
        let read = parse_ape(window, limits, &mut Budget::for_input(0, 0, 1_000))
            .expect("the tag is sound")
            .expect("the file ends with a tag");
        from_ape(&read, limits, &mut Budget::for_input(0, 0, 1_000))
    }

    /// The problem of a value for `field` in the item at `index` that was
    /// cut to `limit`.
    fn cut(field: TagField, index: usize, limit: LimitKind) -> TagProblem<ItemIndex> {
        TagProblem::Value {
            field,
            source: ItemIndex(index),
            reason: Reason::Truncated(limit),
        }
    }

    /// The parser cuts every text value at the long-text limit before the
    /// mapper sees it. A value of exactly the limit is whole; one octet
    /// more is cut, and the cut is recorded once for that value.
    ///
    /// Verifies: SEC-MED-006
    #[test]
    fn says_so_when_the_parser_cut_a_value_at_the_long_text_limit() {
        let limits = text_limits(8, 6);
        let ape = Ape::new()
            .text("Title", "abcdef")
            .text("Album", "abcdefg")
            .text("Lyrics", "abcdef")
            .text("Lyrics", "abcdefg")
            .item(b"Artist", 0, b"abcdef\0abcdefg");
        let lyrics = TagField::Lyrics(LyricsOrigin::ApeItem);
        let lines = || TagLyrics {
            source: LyricsSource::new(LyricsOrigin::ApeItem, LyricsTiming::Plain)
                .expect("the lyrics are not from a SYLT frame"),
            text: String::from("abcdef"),
        };
        assert_eq!(
            parsed(&ape, &limits),
            Mapped {
                tags: TrackTags {
                    title: Some(String::from("abcdef")),
                    artist: strings(&["abcdef", "abcdef"]),
                    album: Some(String::from("abcdef")),
                    lyrics: vec![lines(), lines()],
                    ..TrackTags::default()
                },
                sources: vec![
                    (TagField::Title, ItemIndex(0)),
                    (TagField::Album, ItemIndex(1)),
                    (lyrics, ItemIndex(2)),
                    (lyrics, ItemIndex(3)),
                    (TagField::Artist, ItemIndex(4)),
                ],
                problems: vec![
                    cut(TagField::Album, 1, LimitKind::LongText),
                    cut(lyrics, 3, LimitKind::LongText),
                    cut(TagField::Artist, 4, LimitKind::LongText),
                ],
            }
        );
    }

    /// The field rules cut what the parser left at the short-text limit,
    /// unless it is lyrics. A value that breaches both limits is one text
    /// cut twice: it is recorded once, with the limit it was last cut at.
    ///
    /// Verifies: SEC-MED-006
    #[test]
    fn says_which_limits_cut_a_value() {
        let limits = text_limits(4, 6);
        let ape = Ape::new()
            .text("Title", "abcd")
            .text("Album", "abcdef")
            .text("Genre", "abcdefg");
        assert_eq!(
            parsed(&ape, &limits),
            Mapped {
                tags: TrackTags {
                    title: Some(String::from("abcd")),
                    album: Some(String::from("abcd")),
                    genres: strings(&["abcd"]),
                    ..TrackTags::default()
                },
                sources: vec![
                    (TagField::Title, ItemIndex(0)),
                    (TagField::Album, ItemIndex(1)),
                    (TagField::Genres, ItemIndex(2)),
                ],
                problems: vec![
                    cut(TagField::Album, 1, LimitKind::ShortText),
                    cut(TagField::Genres, 2, LimitKind::ShortText),
                ],
            }
        );
    }

    /// The parser's cut is recorded whatever becomes of the value, and
    /// once. A date is not read once it was cut, so no second reason
    /// follows the cut. A title cut down to white space is left out.
    ///
    /// Verifies: SEC-MED-006
    #[test]
    fn says_the_parser_cut_a_value_that_is_then_dropped() {
        use crate::values::{Field, ValueError};
        let limits = text_limits(8, 4);
        let ape = Ape::new()
            .text("Year", "soon")
            .text("Year", "sooner")
            .text("Title", "    ")
            .text("Title", "    x");
        let not_a_date = |index| TagProblem::Value {
            field: TagField::Date,
            source: ItemIndex(index),
            reason: Reason::Invalid(ValueError::Malformed { field: Field::Year }),
        };
        assert_eq!(
            parsed(&ape, &limits),
            Mapped {
                tags: TrackTags::default(),
                sources: Vec::new(),
                problems: vec![
                    not_a_date(0),
                    cut(TagField::Date, 1, LimitKind::LongText),
                    cut(TagField::Title, 3, LimitKind::LongText),
                ],
            }
        );
    }

    /// What is left of a number, a date, a flag, a gain or a peak the
    /// parser cut is not what was written. At the long-text limit exactly
    /// each is read. One octet longer, each is cut to the text that was
    /// read before, is recorded as cut and is not read, and the next value
    /// is tried.
    ///
    /// Verifies: SEC-MED-006
    #[test]
    fn does_not_read_a_number_a_date_a_flag_or_a_gain_the_parser_cut() {
        let limits = text_limits(16, 8);
        let whole = Ape::new()
            .text("Track", "    3/12")
            .text("Year", "    1997")
            .text("Compilation", "       1")
            .text("REPLAYGAIN_TRACK_GAIN", "-6.50 dB")
            .text("REPLAYGAIN_TRACK_PEAK", "0.250000");
        assert_eq!(
            parsed(&whole, &limits),
            mapped(
                TrackTags {
                    position: TrackPosition::new(Some(3), Some(12), None, None)
                        .expect("the position is in range"),
                    date: Some(day(1997, None, None)),
                    compilation: Some(true),
                    gain: GainTags {
                        track: Some(replay_gain(-6.5, 0.25)),
                        album: None,
                    },
                    ..TrackTags::default()
                },
                vec![
                    (TagField::Track, 0),
                    (TagField::TrackTotal, 0),
                    (TagField::Date, 1),
                    (TagField::Compilation, 2),
                    (TagField::TrackGain, 3),
                    (TagField::TrackPeak, 4),
                ],
            )
        );
        let longer = Ape::new()
            .text("Track", "    3/120")
            .text("Year", "    19971")
            .text("Compilation", "       10")
            .text("REPLAYGAIN_TRACK_GAIN", "-6.50 dB0")
            .text("REPLAYGAIN_TRACK_PEAK", "0.2500001")
            .text("Track", "7");
        assert_eq!(
            parsed(&longer, &limits),
            Mapped {
                tags: TrackTags {
                    position: TrackPosition::new(Some(7), None, None, None)
                        .expect("the position is in range"),
                    ..TrackTags::default()
                },
                sources: vec![(TagField::Track, ItemIndex(5))],
                problems: vec![
                    cut(TagField::Track, 0, LimitKind::LongText),
                    cut(TagField::Date, 1, LimitKind::LongText),
                    cut(TagField::Compilation, 2, LimitKind::LongText),
                    cut(TagField::TrackGain, 3, LimitKind::LongText),
                    cut(TagField::TrackPeak, 4, LimitKind::LongText),
                ],
            }
        );
    }

    /// A text item with one value, which the parser flagged as cut.
    fn cut_item(key: &str, value: &str) -> ApeItem {
        ApeItem {
            key: key.to_owned(),
            value: ApeValue::Text(vec![Text {
                value: value.to_owned(),
                truncated: true,
                replaced: false,
            }]),
        }
    }

    /// Every key whose value is read as a number, a date, a flag, a
    /// recording code, an identifier, a release type, a gain or a peak,
    /// each with a value that is read when it is whole. Flagged as cut by
    /// the parser, none of them is read, and each is recorded once.
    ///
    /// Verifies: SEC-MED-006
    #[test]
    fn reads_no_cut_value_under_a_key_that_is_not_kept_as_text() {
        let typed = [
            ("Track", "3/12", TagField::Track),
            ("Disc", "1/2", TagField::Disc),
            ("Year", "1959-08-17", TagField::Date),
            ("OriginalDate", "1959-08", TagField::OriginalDate),
            ("OriginalYear", "1960", TagField::OriginalDate),
            ("Compilation", "1", TagField::Compilation),
            ("ISRC", "USS1Z9900001", TagField::Isrc),
            ("MUSICBRAINZ_TRACKID", RECORDING, TagField::RecordingMbid),
            ("MUSICBRAINZ_RELEASETRACKID", TRACK, TagField::TrackMbid),
            ("MUSICBRAINZ_ALBUMID", RELEASE, TagField::ReleaseMbid),
            (
                "MUSICBRAINZ_RELEASEGROUPID",
                GROUP,
                TagField::ReleaseGroupMbid,
            ),
            ("MUSICBRAINZ_ARTISTID", ARTIST, TagField::ArtistMbids),
            (
                "MUSICBRAINZ_ALBUMARTISTID",
                ALBUM_ARTIST,
                TagField::AlbumArtistMbids,
            ),
            ("MUSICBRAINZ_ALBUMTYPE", "album", TagField::ReleaseType),
            ("ReleaseType", "live", TagField::ReleaseType),
            ("REPLAYGAIN_TRACK_GAIN", "-6.50 dB", TagField::TrackGain),
            ("REPLAYGAIN_TRACK_PEAK", "0.5", TagField::TrackPeak),
            ("REPLAYGAIN_ALBUM_GAIN", "+1.25 dB", TagField::AlbumGain),
            ("REPLAYGAIN_ALBUM_PEAK", "1.0", TagField::AlbumPeak),
        ];
        let items = typed
            .iter()
            .map(|(key, value, _)| cut_item(key, value))
            .collect();
        let problems = typed
            .iter()
            .zip(0..)
            .map(|((_, _, field), index)| cut(*field, index, LimitKind::LongText))
            .collect();
        assert_eq!(
            map(items),
            Mapped {
                tags: TrackTags::default(),
                sources: Vec::new(),
                problems,
            }
        );
    }

    /// Every key whose value is kept as text, each flagged as cut by the
    /// parser: what is left is kept, and the cut is recorded once.
    ///
    /// Verifies: SEC-MED-006
    #[test]
    fn keeps_a_cut_value_under_every_key_that_is_kept_as_text() {
        let roles = [
            ("Composer", Role::Composer),
            ("Conductor", Role::Conductor),
            ("Lyricist", Role::Lyricist),
            ("MixArtist", Role::Remixer),
            ("Producer", Role::Producer),
            ("Arranger", Role::Arranger),
            ("Engineer", Role::Engineer),
            ("Mixer", Role::Mixer),
            ("DJMixer", Role::DjMixer),
            ("Performer", Role::Performer),
        ];
        let lyrics = TagField::Lyrics(LyricsOrigin::ApeItem);
        let text = [
            ("Title", TagField::Title),
            ("Artist", TagField::Artist),
            ("Album", TagField::Album),
            ("Album Artist", TagField::AlbumArtist),
            ("AlbumArtist", TagField::AlbumArtist),
            ("DiscSubtitle", TagField::DiscSubtitle),
            ("Genre", TagField::Genres),
            ("Mood", TagField::Moods),
            ("Label", TagField::Labels),
            ("Grouping", TagField::Grouping),
            ("Lyrics", lyrics),
            ("TitleSort", TagField::TitleSort),
            ("ArtistSort", TagField::ArtistSort),
            ("AlbumArtistSort", TagField::AlbumArtistSort),
            ("AlbumSort", TagField::AlbumSort),
        ]
        .into_iter()
        .chain(roles.map(|(key, role)| (key, TagField::Credit(role))))
        .collect::<Vec<_>>();
        let items = text.iter().map(|(key, _)| cut_item(key, "Kept")).collect();
        let kept = || Some(String::from("Kept"));
        let one = || strings(&["Kept"]);
        assert_eq!(
            map(items),
            Mapped {
                tags: TrackTags {
                    title: kept(),
                    title_sort: kept(),
                    artist: one(),
                    artist_sort: one(),
                    album_artist: strings(&["Kept", "Kept"]),
                    album_artist_sort: one(),
                    album: kept(),
                    album_sort: kept(),
                    disc_subtitle: kept(),
                    genres: one(),
                    moods: one(),
                    labels: one(),
                    grouping: one(),
                    credits: roles.map(|(_, role)| credited("Kept", role, None)).to_vec(),
                    lyrics: vec![TagLyrics {
                        source: LyricsSource::new(LyricsOrigin::ApeItem, LyricsTiming::Plain)
                            .expect("the lyrics are not from a SYLT frame"),
                        text: String::from("Kept"),
                    }],
                    ..TrackTags::default()
                },
                sources: text
                    .iter()
                    .zip(0..)
                    .map(|((_, field), index)| (*field, ItemIndex(index)))
                    .collect(),
                problems: text
                    .iter()
                    .zip(0..)
                    .map(|((_, field), index)| cut(*field, index, LimitKind::LongText))
                    .collect(),
            }
        );
    }

    proptest! {
        #[test]
        fn any_values_under_any_key_map_within_the_limits(
            items in vec(
                (
                    prop_oneof![
                        select(KEYS).prop_map(|(key, _)| key.to_owned()),
                        "[ -~]{2,12}",
                    ],
                    vec(prop_oneof![".{0,12}", "[0-9/ .dB-]{0,8}"], 0..4),
                ),
                0..24,
            )
        ) {
            let items = items
                .iter()
                .map(|(key, values)| ApeItem {
                    key: key.clone(),
                    value: ApeValue::Text(values.iter().map(|value| text(value)).collect()),
                })
                .collect();
            let limits = Limits::DEFAULT
                .with_override(LimitKind::TagFields, 3)
                .expect("the test lowers a limit");
            let mut budget = Budget::for_input(0, 0, 200);
            let read = from_ape(&tag(items), &limits, &mut budget);
            let tags = &read.tags;
            let lists = [
                tags.artist.len(),
                tags.artist_sort.len(),
                tags.album_artist.len(),
                tags.album_artist_sort.len(),
                tags.genres.len(),
                tags.moods.len(),
                tags.labels.len(),
                tags.grouping.len(),
                tags.credits.len(),
                tags.isrc.len(),
                tags.musicbrainz.artists.len(),
                tags.musicbrainz.album_artists.len(),
                tags.lyrics.len(),
            ];
            prop_assert!(lists.iter().all(|len| *len <= 3));
            let spent = read
                .problems
                .iter()
                .any(|problem| matches!(problem, TagProblem::BudgetSpent { .. }));
            prop_assert!(!spent);
        }
    }
}
