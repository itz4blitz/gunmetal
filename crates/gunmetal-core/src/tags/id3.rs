//! `ID3v2` frames and `ID3v1` fields mapped onto [`TrackTags`].
//!
//! Every filled field records where it was read (API-CAT-08). `ID3v1` fills
//! only fields `ID3v2` left empty. Identifiers, dates, numbers and
//! `ReplayGain` values are parsed into the typed catalogue values; a value
//! outside its range is dropped with a reason (SEC-MED-014). Multi-valued
//! lists stop at the tag-field limit (SEC-MED-006). Artist strings are not
//! split here (WP-053).

use crate::catalog::{
    Advisory, CatalogError, Credit, Gain, GainScale, LyricsOrigin, LyricsSource, LyricsTiming,
    MbIds, PrimaryType, ReleaseType, Role, SecondaryType, TagLyrics, TrackPosition, TrackTags,
};
use crate::formats::id3v1::Id3v1Tag;
use crate::formats::id3v2::{
    Credit as PeopleCredit, Frame, FrameBody, FrameId, Id3v2Tag, LanguageText, SyncedLyrics,
};
use crate::parse::{LimitKind, Limits};
use crate::text::Text;
use crate::untrusted::Untrusted;
use crate::values::{Field, GainDb, Isrc, Mbid, PartialDate, PeakRatio, ValueError};

/// The mapped tags, the source of each field, and the values that were
/// dropped.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Mapped {
    /// The canonical tags.
    pub tags: TrackTags,
    /// Where each filled field was read.
    pub sources: FieldSources,
    /// Values that were dropped, in the order found.
    pub problems: Vec<TagProblem>,
}

/// Where each catalogue field was read.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FieldSources {
    /// The title.
    pub title: Option<FieldSource>,
    /// The title to sort by.
    pub title_sort: Option<FieldSource>,
    /// The recording's artists.
    pub artist: Option<FieldSource>,
    /// The recording's artists to sort by.
    pub artist_sort: Option<FieldSource>,
    /// The release's artists.
    pub album_artist: Option<FieldSource>,
    /// The release's artists to sort by.
    pub album_artist_sort: Option<FieldSource>,
    /// The release's title.
    pub album: Option<FieldSource>,
    /// The release's title to sort by.
    pub album_sort: Option<FieldSource>,
    /// The track number.
    pub track: Option<FieldSource>,
    /// The number of tracks.
    pub track_total: Option<FieldSource>,
    /// The disc number.
    pub disc: Option<FieldSource>,
    /// The number of discs.
    pub disc_total: Option<FieldSource>,
    /// The disc's title.
    pub disc_subtitle: Option<FieldSource>,
    /// The release date.
    pub date: Option<FieldSource>,
    /// The original release date.
    pub original_date: Option<FieldSource>,
    /// The genres.
    pub genres: Option<FieldSource>,
    /// The moods.
    pub moods: Option<FieldSource>,
    /// The record labels.
    pub labels: Option<FieldSource>,
    /// The groupings.
    pub grouping: Option<FieldSource>,
    /// The credited people.
    pub credits: Option<FieldSource>,
    /// Whether the tags mark a compilation.
    pub compilation: Option<FieldSource>,
    /// The release type.
    pub release_type: Option<FieldSource>,
    /// The content advisory.
    pub advisory: Option<FieldSource>,
    /// The recording codes.
    pub isrc: Option<FieldSource>,
    /// The recording `MusicBrainz` identifier.
    pub recording_mbid: Option<FieldSource>,
    /// The track `MusicBrainz` identifier.
    pub track_mbid: Option<FieldSource>,
    /// The release `MusicBrainz` identifier.
    pub release_mbid: Option<FieldSource>,
    /// The release-group `MusicBrainz` identifier.
    pub release_group_mbid: Option<FieldSource>,
    /// The recording's artist `MusicBrainz` identifiers.
    pub artist_mbids: Option<FieldSource>,
    /// The release's artist `MusicBrainz` identifiers.
    pub album_artist_mbids: Option<FieldSource>,
    /// The track gain.
    pub track_gain: Option<FieldSource>,
    /// The album gain.
    pub album_gain: Option<FieldSource>,
    /// The lyrics.
    pub lyrics: Option<FieldSource>,
}

/// One field's origin in an `ID3v2` frame or an `ID3v1` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldSource {
    /// An `ID3v2` frame.
    Id3v2 {
        /// The frame identifier.
        id: FrameId,
        /// Where the frame header starts, in octets from the start of the tag.
        offset: u64,
    },
    /// An `ID3v1` field.
    Id3v1 {
        /// Which `ID3v1` field.
        field: Id3v1Field,
    },
}

/// Which `ID3v1` field a mapped value came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Id3v1Field {
    /// The title.
    Title,
    /// The artist.
    Artist,
    /// The album.
    Album,
    /// The year.
    Year,
    /// The track number.
    Track,
    /// The genre octet.
    Genre,
}

/// A value that was dropped while mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagProblem {
    /// A typed value could not be read (SEC-MED-014).
    InvalidValue {
        /// Where the value was read.
        source: FieldSource,
        /// Why it was dropped.
        error: ValueError,
    },
    /// A catalogue constructor refused the value (SEC-MED-014).
    Catalog {
        /// Where the value was read.
        source: FieldSource,
        /// Why it was dropped.
        error: CatalogError,
    },
    /// A list hit the tag-field limit; later values were dropped
    /// (SEC-MED-006).
    LimitExceeded {
        /// Which limit was hit.
        limit: LimitKind,
        /// The count that exceeded the limit.
        count: u64,
    },
}

/// Maps `v2` and `v1` onto [`TrackTags`]. `v1` fills only fields `v2` left
/// empty.
#[must_use]
pub fn from_id3(v2: Option<&Id3v2Tag>, v1: Option<&Id3v1Tag>, limits: &Limits) -> Mapped {
    Mapper::new(limits).run(v2, v1)
}

/// The Winamp `ID3v1` genre list, as mutagen-specs records it: indices
/// 0 to 191. 255 means unset.
const GENRES: &[&str] = &[
    "Blues",
    "Classic Rock",
    "Country",
    "Dance",
    "Disco",
    "Funk",
    "Grunge",
    "Hip-Hop",
    "Jazz",
    "Metal",
    "New Age",
    "Oldies",
    "Other",
    "Pop",
    "R&B",
    "Rap",
    "Reggae",
    "Rock",
    "Techno",
    "Industrial",
    "Alternative",
    "Ska",
    "Death Metal",
    "Pranks",
    "Soundtrack",
    "Euro-Techno",
    "Ambient",
    "Trip-Hop",
    "Vocal",
    "Jazz+Funk",
    "Fusion",
    "Trance",
    "Classical",
    "Instrumental",
    "Acid",
    "House",
    "Game",
    "Sound Clip",
    "Gospel",
    "Noise",
    "Alt. Rock",
    "Bass",
    "Soul",
    "Punk",
    "Space",
    "Meditative",
    "Instrumental Pop",
    "Instrumental Rock",
    "Ethnic",
    "Gothic",
    "Darkwave",
    "Techno-Industrial",
    "Electronic",
    "Pop-Folk",
    "Eurodance",
    "Dream",
    "Southern Rock",
    "Comedy",
    "Cult",
    "Gangsta Rap",
    "Top 40",
    "Christian Rap",
    "Pop/Funk",
    "Jungle",
    "Native American",
    "Cabaret",
    "New Wave",
    "Psychedelic",
    "Rave",
    "Showtunes",
    "Trailer",
    "Lo-Fi",
    "Tribal",
    "Acid Punk",
    "Acid Jazz",
    "Polka",
    "Retro",
    "Musical",
    "Rock & Roll",
    "Hard Rock",
    "Folk",
    "Folk-Rock",
    "National Folk",
    "Swing",
    "Fast-Fusion",
    "Bebop",
    "Latin",
    "Revival",
    "Celtic",
    "Bluegrass",
    "Avantgarde",
    "Gothic Rock",
    "Progressive Rock",
    "Psychedelic Rock",
    "Symphonic Rock",
    "Slow Rock",
    "Big Band",
    "Chorus",
    "Easy Listening",
    "Acoustic",
    "Humour",
    "Speech",
    "Chanson",
    "Opera",
    "Chamber Music",
    "Sonata",
    "Symphony",
    "Booty Bass",
    "Primus",
    "Porn Groove",
    "Satire",
    "Slow Jam",
    "Club",
    "Tango",
    "Samba",
    "Folklore",
    "Ballad",
    "Power Ballad",
    "Rhythmic Soul",
    "Freestyle",
    "Duet",
    "Punk Rock",
    "Drum Solo",
    "A Cappella",
    "Euro-House",
    "Dance Hall",
    "Goa",
    "Drum & Bass",
    "Club-House",
    "Hardcore",
    "Terror",
    "Indie",
    "BritPop",
    "Afro-Punk",
    "Polsk Punk",
    "Beat",
    "Christian Gangsta Rap",
    "Heavy Metal",
    "Black Metal",
    "Crossover",
    "Contemporary Christian",
    "Christian Rock",
    "Merengue",
    "Salsa",
    "Thrash Metal",
    "Anime",
    "JPop",
    "Synthpop",
    "Abstract",
    "Art Rock",
    "Baroque",
    "Bhangra",
    "Big Beat",
    "Breakbeat",
    "Chillout",
    "Downtempo",
    "Dub",
    "EBM",
    "Eclectic",
    "Electro",
    "Electroclash",
    "Emo",
    "Experimental",
    "Garage",
    "Global",
    "IDM",
    "Illbient",
    "Industro-Goth",
    "Jam Band",
    "Krautrock",
    "Leftfield",
    "Lounge",
    "Math Rock",
    "New Romantic",
    "Nu-Breakz",
    "Post-Punk",
    "Post-Rock",
    "Psytrance",
    "Shoegaze",
    "Space Rock",
    "Trop Rock",
    "World Music",
    "Neoclassical",
    "Audiobook",
    "Audio Theatre",
    "Neue Deutsche Welle",
    "Podcast",
    "Indie Rock",
    "G-Funk",
    "Dubstep",
    "Garage Rock",
    "Psybient",
];

/// Which kind of frame the mapper understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Title,
    TitleSort,
    Artist,
    ArtistSort,
    AlbumArtist,
    AlbumArtistSort,
    Album,
    AlbumSort,
    Track,
    Disc,
    DiscSubtitle,
    Date,
    Year,
    DatePart,
    OriginalDate,
    OriginalYear,
    Genre,
    Mood,
    Label,
    Grouping,
    Credit(Role),
    People { musician: bool },
    Compilation,
    Isrc,
    UserText,
    Ufid,
    Lyrics,
    SyncedLyrics,
    Ignore,
}

/// A primary or secondary release-type token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypeToken {
    Primary(PrimaryType),
    Secondary(SecondaryType),
}

/// Accumulates mapped fields while walking the tags.
struct Mapper<'a> {
    tags: TrackTags,
    sources: FieldSources,
    problems: Vec<TagProblem>,
    limits: &'a Limits,
    year: Option<(PartialDate, FieldSource)>,
    date_part: Option<(u8, u8, FieldSource)>,
    original_year: Option<(PartialDate, FieldSource)>,
    pending_track_peak: Option<PeakRatio>,
    pending_album_peak: Option<PeakRatio>,
}

impl<'a> Mapper<'a> {
    fn new(limits: &'a Limits) -> Self {
        Self {
            tags: TrackTags::default(),
            sources: FieldSources::default(),
            problems: Vec::new(),
            limits,
            year: None,
            date_part: None,
            original_year: None,
            pending_track_peak: None,
            pending_album_peak: None,
        }
    }

    fn run(mut self, v2: Option<&Id3v2Tag>, v1: Option<&Id3v1Tag>) -> Mapped {
        if let Some(tag) = v2 {
            for frame in &tag.frames {
                self.frame(frame);
            }
            self.finish_dates();
        }
        if let Some(tag) = v1 {
            self.id3v1(tag);
        }
        Mapped {
            tags: self.tags,
            sources: self.sources,
            problems: self.problems,
        }
    }

    /// Records `problem`, and returns whether there was room for it.
    fn report(&mut self, problem: TagProblem) -> bool {
        report(&mut self.problems, self.limits, problem)
    }

    fn frame(&mut self, frame: &Frame) {
        let source = FieldSource::Id3v2 {
            id: frame.id,
            offset: frame.offset,
        };
        match &frame.body {
            FrameBody::Text(values) => self.text(kind(frame.id), values, source),
            FrameBody::People(people) => {
                if let Kind::People { musician } = kind(frame.id) {
                    self.people(people, musician, source);
                }
            }
            FrameBody::UserText {
                description,
                values,
            } if kind(frame.id) == Kind::UserText => {
                self.user_text(&description.value, values, source);
            }
            FrameBody::Ufid { owner, id } if kind(frame.id) == Kind::Ufid => {
                self.ufid(&owner.value, id, source);
            }
            FrameBody::Lyrics(body) if kind(frame.id) == Kind::Lyrics => {
                self.uslt(body, source);
            }
            FrameBody::SyncedLyrics(body) if kind(frame.id) == Kind::SyncedLyrics => {
                self.sylt(body, source);
            }
            _ => {}
        }
    }

    fn text(&mut self, kind: Kind, values: &[Text], source: FieldSource) {
        if self.text_single(kind, values, source) {
            return;
        }
        self.text_list(kind, values, source);
    }

    fn text_single(&mut self, kind: Kind, values: &[Text], source: FieldSource) -> bool {
        match kind {
            Kind::Title => set_one(
                &mut self.tags.title,
                &mut self.sources.title,
                values,
                source,
            ),
            Kind::TitleSort => set_one(
                &mut self.tags.title_sort,
                &mut self.sources.title_sort,
                values,
                source,
            ),
            Kind::Album => set_one(
                &mut self.tags.album,
                &mut self.sources.album,
                values,
                source,
            ),
            Kind::AlbumSort => set_one(
                &mut self.tags.album_sort,
                &mut self.sources.album_sort,
                values,
                source,
            ),
            Kind::DiscSubtitle => set_one(
                &mut self.tags.disc_subtitle,
                &mut self.sources.disc_subtitle,
                values,
                source,
            ),
            Kind::Track => self.track(values, source),
            Kind::Disc => self.disc(values, source),
            Kind::Date => self.timestamp(values, source, false),
            Kind::Year => self.year_text(values, source, false),
            Kind::DatePart => self.date_part(values, source),
            Kind::OriginalDate => self.timestamp(values, source, true),
            Kind::OriginalYear => self.year_text(values, source, true),
            Kind::Genre => self.genres(values, source),
            Kind::Credit(role) => self.credits(values, role, source),
            Kind::Compilation => self.compilation(values, source),
            Kind::Isrc => self.isrcs(values, source),
            _ => return false,
        }
        true
    }

    fn text_list(&mut self, kind: Kind, values: &[Text], source: FieldSource) {
        let (list, origin) = match kind {
            Kind::Artist => (&mut self.tags.artist, &mut self.sources.artist),
            Kind::ArtistSort => (&mut self.tags.artist_sort, &mut self.sources.artist_sort),
            Kind::AlbumArtist => (&mut self.tags.album_artist, &mut self.sources.album_artist),
            Kind::AlbumArtistSort => (
                &mut self.tags.album_artist_sort,
                &mut self.sources.album_artist_sort,
            ),
            Kind::Mood => (&mut self.tags.moods, &mut self.sources.moods),
            Kind::Label => (&mut self.tags.labels, &mut self.sources.labels),
            Kind::Grouping => (&mut self.tags.grouping, &mut self.sources.grouping),
            _ => return,
        };
        extend_text(
            list,
            origin,
            values,
            source,
            self.limits,
            &mut self.problems,
        );
    }

    fn track(&mut self, values: &[Text], source: FieldSource) {
        if let Some(value) = first_present(values) {
            self.position(value, source, true);
        }
    }

    fn disc(&mut self, values: &[Text], source: FieldSource) {
        if let Some(value) = first_present(values) {
            self.position(value, source, false);
        }
    }

    fn position(&mut self, value: &str, source: FieldSource, track: bool) {
        let (number, total) = match parse_number_total(value) {
            Ok(parsed) => parsed,
            Err(error) => {
                self.report(TagProblem::InvalidValue { source, error });
                return;
            }
        };
        let parsed = (number, self.checked_total(total, source, track));
        let current = self.tags.position;
        let (track_n, track_total, disc_n, disc_total) = if track {
            (
                current.track().or(parsed.0),
                current.track_total().or(parsed.1),
                current.disc(),
                current.disc_total(),
            )
        } else {
            (
                current.track(),
                current.track_total(),
                current.disc().or(parsed.0),
                current.disc_total().or(parsed.1),
            )
        };
        match TrackPosition::new(track_n, track_total, disc_n, disc_total) {
            Ok(position) => {
                self.tags.position = position;
                if track {
                    remember(&mut self.sources.track, source);
                    if parsed.1.is_some() {
                        remember(&mut self.sources.track_total, source);
                    }
                } else {
                    remember(&mut self.sources.disc, source);
                    if parsed.1.is_some() {
                        remember(&mut self.sources.disc_total, source);
                    }
                }
            }
            Err(error) => {
                self.report(TagProblem::Catalog { source, error });
            }
        }
    }

    /// `total` when the catalogue accepts it as a track or disc total.
    /// Otherwise the reason is recorded, and the number it came with is
    /// kept without a total (SEC-MED-014).
    fn checked_total(
        &mut self,
        total: Option<u16>,
        source: FieldSource,
        track: bool,
    ) -> Option<u16> {
        let total = total?;
        let alone = if track {
            TrackPosition::new(None, Some(total), None, None)
        } else {
            TrackPosition::new(None, None, None, Some(total))
        };
        match alone {
            Ok(_) => Some(total),
            Err(error) => {
                self.report(TagProblem::Catalog { source, error });
                None
            }
        }
    }

    fn timestamp(&mut self, values: &[Text], source: FieldSource, original: bool) {
        let (slot, origin) = if original {
            (
                &mut self.tags.original_date,
                &mut self.sources.original_date,
            )
        } else {
            (&mut self.tags.date, &mut self.sources.date)
        };
        if slot.is_some() {
            return;
        }
        let Some(value) = first_present(values) else {
            return;
        };
        match PartialDate::parse(Untrusted::new(value)) {
            Ok(date) => {
                *slot = Some(date);
                *origin = Some(source);
            }
            Err(error) => {
                self.report(TagProblem::InvalidValue { source, error });
            }
        }
    }

    fn year_text(&mut self, values: &[Text], source: FieldSource, original: bool) {
        let Some(value) = first_present(values) else {
            return;
        };
        match PartialDate::parse(Untrusted::new(value)) {
            Ok(date) => {
                if original {
                    if self.original_year.is_none() {
                        self.original_year = Some((date, source));
                    }
                } else if self.year.is_none() {
                    self.year = Some((date, source));
                }
            }
            Err(error) => {
                self.report(TagProblem::InvalidValue { source, error });
            }
        }
    }

    fn date_part(&mut self, values: &[Text], source: FieldSource) {
        if self.date_part.is_some() {
            return;
        }
        let Some(value) = first_present(values) else {
            return;
        };
        match parse_tdat(value) {
            Ok((month, day)) => self.date_part = Some((month, day, source)),
            Err(error) => {
                self.report(TagProblem::InvalidValue { source, error });
            }
        }
    }

    fn finish_dates(&mut self) {
        if self.tags.date.is_none() {
            if let Some((written, source)) = self.year {
                let combined = self.date_part.map(|(month, day, _)| {
                    PartialDate::new(written.year(), Some(month), Some(day))
                });
                let date = match combined {
                    Some(Ok(date)) => date,
                    Some(Err(error)) => {
                        self.report(TagProblem::InvalidValue { source, error });
                        written
                    }
                    None => written,
                };
                self.tags.date = Some(date);
                self.sources.date = Some(source);
            }
        }
        if self.tags.original_date.is_none() {
            if let Some((date, source)) = self.original_year {
                self.tags.original_date = Some(date);
                self.sources.original_date = Some(source);
            }
        }
    }

    fn genres(&mut self, values: &[Text], source: FieldSource) {
        for value in present(values) {
            for name in expand_genre(value) {
                if !push(&mut self.tags.genres, name, self.limits, &mut self.problems) {
                    return;
                }
                if self.sources.genres.is_none() {
                    self.sources.genres = Some(source);
                }
            }
        }
    }

    fn credits(&mut self, values: &[Text], role: Role, source: FieldSource) {
        for value in values {
            let Some(credit) = Credit::new(value.value.clone(), role, None, None).ok() else {
                continue;
            };
            if !push(
                &mut self.tags.credits,
                credit,
                self.limits,
                &mut self.problems,
            ) {
                break;
            }
            if self.sources.credits.is_none() {
                self.sources.credits = Some(source);
            }
        }
    }

    fn people(&mut self, people: &[PeopleCredit], musician: bool, source: FieldSource) {
        for person in people {
            let (role, detail) = if musician {
                let detail = person.role.value.trim();
                (
                    Role::Performer,
                    (!detail.is_empty()).then(|| person.role.value.clone()),
                )
            } else {
                people_role(&person.role.value)
            };
            let Some(credit) = Credit::new(person.name.value.clone(), role, detail, None).ok()
            else {
                continue;
            };
            if !push(
                &mut self.tags.credits,
                credit,
                self.limits,
                &mut self.problems,
            ) {
                break;
            }
            if self.sources.credits.is_none() {
                self.sources.credits = Some(source);
            }
        }
    }

    fn compilation(&mut self, values: &[Text], source: FieldSource) {
        if self.tags.compilation.is_some() {
            return;
        }
        let Some(value) = first_present(values) else {
            return;
        };
        match value.trim() {
            "1" => {
                self.tags.compilation = Some(true);
                self.sources.compilation = Some(source);
            }
            "0" => {
                self.tags.compilation = Some(false);
                self.sources.compilation = Some(source);
            }
            _ => {}
        }
    }

    /// Reads every code of the frame, until the codes or the problems are
    /// full.
    fn isrcs(&mut self, values: &[Text], source: FieldSource) {
        for value in present(values) {
            let room = match Isrc::parse(Untrusted::new(value)) {
                Ok(isrc) => {
                    let room = push(&mut self.tags.isrc, isrc, self.limits, &mut self.problems);
                    if room {
                        remember(&mut self.sources.isrc, source);
                    }
                    room
                }
                Err(error) => self.report(TagProblem::InvalidValue { source, error }),
            };
            if !room {
                break;
            }
        }
    }

    fn user_text(&mut self, description: &str, values: &[Text], source: FieldSource) {
        match txxx_key(description).as_str() {
            "musicbrainz_album_id" => self.one_mbid(values, source, MbidSlot::Release),
            "musicbrainz_release_group_id" => self.one_mbid(values, source, MbidSlot::ReleaseGroup),
            "musicbrainz_release_track_id" => self.one_mbid(values, source, MbidSlot::Track),
            "musicbrainz_artist_id" => self.many_mbids(values, source, false),
            "musicbrainz_album_artist_id" => self.many_mbids(values, source, true),
            "musicbrainz_album_type" | "releasetype" | "release_type" => {
                self.release_type(values, source);
            }
            "replaygain_track_gain" => self.gain(values, source, true),
            "replaygain_album_gain" => self.gain(values, source, false),
            "replaygain_track_peak" => self.peak(values, source, true),
            "replaygain_album_peak" => self.peak(values, source, false),
            "itunesadvisory" => self.advisory(values, source),
            "mood" => {
                extend_text(
                    &mut self.tags.moods,
                    &mut self.sources.moods,
                    values,
                    source,
                    self.limits,
                    &mut self.problems,
                );
            }
            _ => {}
        }
    }

    fn one_mbid(&mut self, values: &[Text], source: FieldSource, slot: MbidSlot) {
        if mbid_slot(&self.tags.musicbrainz, slot).is_some() {
            return;
        }
        let Some(value) = first_present(values) else {
            return;
        };
        match Mbid::parse(Untrusted::new(value)) {
            Ok(mbid) => {
                *mbid_slot_mut(&mut self.tags.musicbrainz, slot) = Some(mbid);
                *mbid_source_mut(&mut self.sources, slot) = Some(source);
            }
            Err(error) => {
                self.report(TagProblem::InvalidValue { source, error });
            }
        }
    }

    /// Reads every identifier of the frame, until the identifiers or the
    /// problems are full. A 2.3 tag holds one value per frame, and Picard
    /// joins several identifiers in it with `/`; an identifier holds
    /// neither `/` nor `;`, so the values are split on both.
    fn many_mbids(&mut self, values: &[Text], source: FieldSource, album: bool) {
        let ids = present(values)
            .flat_map(|value| value.split(['/', ';']))
            .map(str::trim)
            .filter(|id| !id.is_empty());
        for value in ids {
            let room = match Mbid::parse(Untrusted::new(value)) {
                Ok(mbid) => {
                    let (list, origin) = if album {
                        (
                            &mut self.tags.musicbrainz.album_artists,
                            &mut self.sources.album_artist_mbids,
                        )
                    } else {
                        (
                            &mut self.tags.musicbrainz.artists,
                            &mut self.sources.artist_mbids,
                        )
                    };
                    let room = push(list, mbid, self.limits, &mut self.problems);
                    if room {
                        remember(origin, source);
                    }
                    room
                }
                Err(error) => self.report(TagProblem::InvalidValue { source, error }),
            };
            if !room {
                break;
            }
        }
    }

    fn ufid(&mut self, owner: &str, id: &[u8], source: FieldSource) {
        if self.tags.musicbrainz.recording.is_some() || !is_mb_owner(owner) {
            return;
        }
        let Some(text) = core::str::from_utf8(id).ok() else {
            self.report(TagProblem::InvalidValue {
                source,
                error: ValueError::Malformed { field: Field::Mbid },
            });
            return;
        };
        let text = text.trim();
        match Mbid::parse(Untrusted::new(text)) {
            Ok(mbid) => {
                self.tags.musicbrainz.recording = Some(mbid);
                self.sources.recording_mbid = Some(source);
            }
            Err(error) => {
                self.report(TagProblem::InvalidValue { source, error });
            }
        }
    }

    fn gain(&mut self, values: &[Text], source: FieldSource, track: bool) {
        let slot = if track {
            &mut self.tags.gain.track
        } else {
            &mut self.tags.gain.album
        };
        if slot.is_some() {
            return;
        }
        let Some(value) = first_present(values) else {
            return;
        };
        match GainDb::parse(Untrusted::new(value)) {
            Ok(gain) => {
                let peak = if track {
                    self.pending_track_peak.take()
                } else {
                    self.pending_album_peak.take()
                };
                *slot = Some(Gain {
                    scale: GainScale::ReplayGain,
                    gain,
                    peak,
                });
                if track {
                    self.sources.track_gain = Some(source);
                } else {
                    self.sources.album_gain = Some(source);
                }
            }
            Err(error) => {
                self.report(TagProblem::InvalidValue { source, error });
            }
        }
    }

    fn peak(&mut self, values: &[Text], source: FieldSource, track: bool) {
        let Some(value) = first_present(values) else {
            return;
        };
        match PeakRatio::parse(Untrusted::new(value)) {
            Ok(peak) => {
                let gain = if track {
                    &mut self.tags.gain.track
                } else {
                    &mut self.tags.gain.album
                };
                if let Some(gain) = gain {
                    if gain.peak.is_none() {
                        gain.peak = Some(peak);
                    }
                } else if track {
                    if self.pending_track_peak.is_none() {
                        self.pending_track_peak = Some(peak);
                    }
                } else if self.pending_album_peak.is_none() {
                    self.pending_album_peak = Some(peak);
                }
            }
            Err(error) => {
                self.report(TagProblem::InvalidValue { source, error });
            }
        }
    }

    fn advisory(&mut self, values: &[Text], source: FieldSource) {
        if self.tags.advisory.is_some() {
            return;
        }
        let Some(value) = first_present(values) else {
            return;
        };
        let advisory = match value.trim() {
            "1" => Some(Advisory::Explicit),
            "2" => Some(Advisory::Clean),
            _ => None,
        };
        if let Some(advisory) = advisory {
            self.tags.advisory = Some(advisory);
            self.sources.advisory = Some(source);
        }
    }

    fn release_type(&mut self, values: &[Text], source: FieldSource) {
        if self.tags.release_type.is_some() {
            return;
        }
        let mut release = ReleaseType::default();
        for value in present(values) {
            add_release_tokens(value, &mut release);
        }
        if release.primary.is_some() || !release.secondary.is_empty() {
            self.tags.release_type = Some(release);
            self.sources.release_type = Some(source);
        }
    }

    fn uslt(&mut self, body: &LanguageText, source: FieldSource) {
        if body.text.value.trim().is_empty() {
            return;
        }
        self.embed_lyrics(
            LyricsOrigin::Id3Unsynced,
            LyricsTiming::Plain,
            &body.text.value,
            source,
        );
    }

    /// The lines of a `SYLT` frame joined by newlines, up to the lyrics
    /// limit. Lines the parser dropped at the line limit, and lines past
    /// the lyrics limit, are each reported (SEC-MED-006).
    fn sylt(&mut self, body: &SyncedLyrics, source: FieldSource) {
        // Content type 1 is lyrics and 2 a transcription; writers that
        // leave it unset write 0. Movement names, events, chords, trivia
        // and links are not the words of the recording.
        if body.content_type > 2 {
            return;
        }
        if body.truncated {
            let count = u64::try_from(body.lines.len())
                .unwrap_or(u64::MAX)
                .saturating_add(1);
            self.report(TagProblem::LimitExceeded {
                limit: LimitKind::LyricsLines,
                count,
            });
        }
        let mut text = String::new();
        for (index, line) in body.lines.iter().enumerate() {
            let separator = if index == 0 { "" } else { "\n" };
            let octets = text
                .len()
                .saturating_add(separator.len())
                .saturating_add(line.text.value.len());
            let count = u64::try_from(octets).unwrap_or(u64::MAX);
            if self.limits.check(LimitKind::LyricsBytes, count, 0).is_err() {
                self.report(TagProblem::LimitExceeded {
                    limit: LimitKind::LyricsBytes,
                    count,
                });
                break;
            }
            text.push_str(separator);
            text.push_str(&line.text.value);
        }
        if text.trim().is_empty() {
            return;
        }
        self.embed_lyrics(LyricsOrigin::Id3Synced, LyricsTiming::Line, &text, source);
    }

    fn embed_lyrics(
        &mut self,
        origin: LyricsOrigin,
        timing: LyricsTiming,
        text: &str,
        source: FieldSource,
    ) {
        // `LyricsSource::new` refuses only untimed `SYLT` text, which no
        // caller here asks for.
        LyricsSource::new(origin, timing)
            .into_iter()
            .map(|lyrics_source| TagLyrics {
                source: lyrics_source,
                text: text.to_owned(),
            })
            .for_each(|lyrics| self.add_lyrics(lyrics, source));
    }

    fn add_lyrics(&mut self, lyrics: TagLyrics, source: FieldSource) {
        if push(
            &mut self.tags.lyrics,
            lyrics,
            self.limits,
            &mut self.problems,
        ) {
            remember(&mut self.sources.lyrics, source);
        }
    }

    fn id3v1(&mut self, tag: &Id3v1Tag) {
        set_one(
            &mut self.tags.title,
            &mut self.sources.title,
            std::slice::from_ref(&tag.title),
            FieldSource::Id3v1 {
                field: Id3v1Field::Title,
            },
        );
        if self.tags.artist.is_empty() {
            extend_text(
                &mut self.tags.artist,
                &mut self.sources.artist,
                std::slice::from_ref(&tag.artist),
                FieldSource::Id3v1 {
                    field: Id3v1Field::Artist,
                },
                self.limits,
                &mut self.problems,
            );
        }
        set_one(
            &mut self.tags.album,
            &mut self.sources.album,
            std::slice::from_ref(&tag.album),
            FieldSource::Id3v1 {
                field: Id3v1Field::Album,
            },
        );
        if self.tags.date.is_none() && !tag.year.value.trim().is_empty() {
            let source = FieldSource::Id3v1 {
                field: Id3v1Field::Year,
            };
            match PartialDate::parse(Untrusted::new(&tag.year.value)) {
                Ok(date) => {
                    self.tags.date = Some(date);
                    self.sources.date = Some(source);
                }
                Err(error) => {
                    self.report(TagProblem::InvalidValue { source, error });
                }
            }
        }
        if self.tags.position.track().is_none() {
            if let Some(track) = tag.track {
                let source = FieldSource::Id3v1 {
                    field: Id3v1Field::Track,
                };
                self.position(&track.to_string(), source, true);
            }
        }
        if self.tags.genres.is_empty() {
            if let Some(name) = genre_name(tag.genre) {
                extend_text(
                    &mut self.tags.genres,
                    &mut self.sources.genres,
                    &[Text {
                        value: name.to_owned(),
                        truncated: false,
                        replaced: false,
                    }],
                    FieldSource::Id3v1 {
                        field: Id3v1Field::Genre,
                    },
                    self.limits,
                    &mut self.problems,
                );
            }
        }
    }
}

/// Records `source` when that field has not been filled yet.
fn remember(slot: &mut Option<FieldSource>, source: FieldSource) {
    if slot.is_none() {
        *slot = Some(source);
    }
}

/// Sets `slot` from the first present value when it is still empty.
fn set_one(
    slot: &mut Option<String>,
    origin: &mut Option<FieldSource>,
    values: &[Text],
    source: FieldSource,
) {
    if slot.is_some() {
        return;
    }
    if let Some(value) = first_present(values) {
        *slot = Some(value.to_owned());
        *origin = Some(source);
    }
}

/// Appends each present value to `list`, stopping at the tag-field limit.
fn extend_text(
    list: &mut Vec<String>,
    origin: &mut Option<FieldSource>,
    values: &[Text],
    source: FieldSource,
    limits: &Limits,
    problems: &mut Vec<TagProblem>,
) {
    for value in present(values) {
        if !push(list, value.to_owned(), limits, problems) {
            break;
        }
        if origin.is_none() {
            *origin = Some(source);
        }
    }
}

/// Pushes `item` when `list` is still under the tag-field limit.
fn push<T>(list: &mut Vec<T>, item: T, limits: &Limits, problems: &mut Vec<TagProblem>) -> bool {
    let count = u64::try_from(list.len())
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    if limits.check(LimitKind::TagFields, count, 0).is_err() {
        report(
            problems,
            limits,
            TagProblem::LimitExceeded {
                limit: LimitKind::TagFields,
                count,
            },
        );
        return false;
    }
    list.push(item);
    true
}

/// Records `problem` while `problems` is under the tag-field limit, and
/// returns whether it was. A full list keeps its length; its last entry
/// becomes a [`TagProblem::LimitExceeded`] that says the list was cut
/// (SEC-MED-006).
fn report(problems: &mut Vec<TagProblem>, limits: &Limits, problem: TagProblem) -> bool {
    let count = u64::try_from(problems.len())
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    if limits.check(LimitKind::TagFields, count, 0).is_ok() {
        problems.push(problem);
        return true;
    }
    if let Some(last) = problems.last_mut() {
        *last = TagProblem::LimitExceeded {
            limit: LimitKind::TagFields,
            count,
        };
    }
    false
}

/// Which single `MusicBrainz` identifier a `TXXX` frame fills.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MbidSlot {
    Release,
    ReleaseGroup,
    Track,
}

/// The identifier in `ids` named by `slot`.
fn mbid_slot(ids: &MbIds, slot: MbidSlot) -> Option<Mbid> {
    match slot {
        MbidSlot::Release => ids.release,
        MbidSlot::ReleaseGroup => ids.release_group,
        MbidSlot::Track => ids.track,
    }
}

/// The identifier in `ids` named by `slot`.
fn mbid_slot_mut(ids: &mut MbIds, slot: MbidSlot) -> &mut Option<Mbid> {
    match slot {
        MbidSlot::Release => &mut ids.release,
        MbidSlot::ReleaseGroup => &mut ids.release_group,
        MbidSlot::Track => &mut ids.track,
    }
}

/// The source slot for `slot`.
fn mbid_source_mut(sources: &mut FieldSources, slot: MbidSlot) -> &mut Option<FieldSource> {
    match slot {
        MbidSlot::Release => &mut sources.release_mbid,
        MbidSlot::ReleaseGroup => &mut sources.release_group_mbid,
        MbidSlot::Track => &mut sources.track_mbid,
    }
}

/// Which kind of body the mapper reads from `id`.
fn kind(id: FrameId) -> Kind {
    match id {
        FrameId::Four(id) => kind4(id),
        FrameId::Three(id) => kind3(id),
    }
}

/// A 2.3 or 2.4 frame identifier.
fn kind4(id: [u8; 4]) -> Kind {
    match &id {
        b"TIT2" => Kind::Title,
        b"TSOT" | b"XSOT" => Kind::TitleSort,
        b"TPE1" => Kind::Artist,
        b"TSOP" | b"XSOP" => Kind::ArtistSort,
        b"TPE2" => Kind::AlbumArtist,
        b"TSO2" => Kind::AlbumArtistSort,
        b"TALB" => Kind::Album,
        b"TSOA" | b"XSOA" => Kind::AlbumSort,
        b"TRCK" => Kind::Track,
        b"TPOS" => Kind::Disc,
        b"TSST" => Kind::DiscSubtitle,
        b"TDRC" => Kind::Date,
        b"TYER" => Kind::Year,
        b"TDAT" => Kind::DatePart,
        b"TDOR" => Kind::OriginalDate,
        b"TORY" => Kind::OriginalYear,
        b"TCON" => Kind::Genre,
        b"TMOO" => Kind::Mood,
        b"TPUB" => Kind::Label,
        b"TIT1" | b"GRP1" => Kind::Grouping,
        b"TCOM" => Kind::Credit(Role::Composer),
        b"TPE3" => Kind::Credit(Role::Conductor),
        b"TEXT" => Kind::Credit(Role::Lyricist),
        b"TPE4" => Kind::Credit(Role::Remixer),
        b"TIPL" | b"IPLS" => Kind::People { musician: false },
        b"TMCL" => Kind::People { musician: true },
        b"TCMP" => Kind::Compilation,
        b"TSRC" => Kind::Isrc,
        b"TXXX" => Kind::UserText,
        b"UFID" => Kind::Ufid,
        b"USLT" => Kind::Lyrics,
        b"SYLT" => Kind::SyncedLyrics,
        _ => Kind::Ignore,
    }
}

/// A 2.2 frame identifier.
fn kind3(id: [u8; 3]) -> Kind {
    match &id {
        b"TT2" => Kind::Title,
        b"TST" => Kind::TitleSort,
        b"TP1" => Kind::Artist,
        b"TSP" => Kind::ArtistSort,
        b"TP2" => Kind::AlbumArtist,
        b"TS2" => Kind::AlbumArtistSort,
        b"TAL" => Kind::Album,
        b"TSA" => Kind::AlbumSort,
        b"TRK" => Kind::Track,
        b"TPA" => Kind::Disc,
        b"TYE" => Kind::Year,
        b"TDA" => Kind::DatePart,
        b"TOR" => Kind::OriginalYear,
        b"TCO" => Kind::Genre,
        b"TPB" => Kind::Label,
        b"TT1" => Kind::Grouping,
        b"TCM" => Kind::Credit(Role::Composer),
        b"TP3" => Kind::Credit(Role::Conductor),
        b"TXT" => Kind::Credit(Role::Lyricist),
        b"TP4" => Kind::Credit(Role::Remixer),
        b"IPL" => Kind::People { musician: false },
        b"TCP" => Kind::Compilation,
        b"TRC" => Kind::Isrc,
        b"TXX" => Kind::UserText,
        b"UFI" => Kind::Ufid,
        b"ULT" => Kind::Lyrics,
        b"SLT" => Kind::SyncedLyrics,
        _ => Kind::Ignore,
    }
}

/// The first value that is not empty or only white space.
fn first_present(values: &[Text]) -> Option<&str> {
    present(values).next()
}

/// The values that are not empty or only white space.
fn present(values: &[Text]) -> impl Iterator<Item = &str> {
    values
        .iter()
        .map(|text| text.value.as_str())
        .filter(|value| !value.trim().is_empty())
}

/// A track or disc number and optional total, allowing a number above its
/// total so [`TrackPosition`] can keep and flag it.
fn parse_number_total(text: &str) -> Result<(Option<u16>, Option<u16>), ValueError> {
    let lower = text.trim().to_ascii_lowercase();
    let (number, total) = match lower.split_once('/').or_else(|| lower.split_once(" of ")) {
        Some((number, total)) => (number.trim_end(), Some(total.trim_start())),
        None => (lower.as_str(), None),
    };
    Ok((
        Some(parse_count(number, Field::Number)?),
        total
            .map(|total| parse_count(total, Field::Total))
            .transpose()?,
    ))
}

/// A whole number written in ASCII digits.
fn parse_count(text: &str, field: Field) -> Result<u16, ValueError> {
    if text.is_empty() {
        return Err(ValueError::Malformed { field });
    }
    let mut value = 0_u64;
    for octet in text.bytes() {
        let digit = char::from(octet)
            .to_digit(10)
            .ok_or(ValueError::Malformed { field })?;
        value = value.saturating_mul(10).saturating_add(u64::from(digit));
    }
    u16::try_from(value).map_err(|_| ValueError::OutOfRange { field, value })
}

/// `ID3v2.3` `TDAT`: day then month, four ASCII digits.
fn parse_tdat(text: &str) -> Result<(u8, u8), ValueError> {
    let &[day_tens, day_ones, month_tens, month_ones] = text.trim().as_bytes() else {
        return Err(ValueError::Malformed { field: Field::Day });
    };
    let day = two_digits(day_tens, day_ones, Field::Day)?;
    let month = two_digits(month_tens, month_ones, Field::Month)?;
    Ok((month, day))
}

/// Two ASCII digits as a number from 0 to 99.
fn two_digits(tens: u8, ones: u8, field: Field) -> Result<u8, ValueError> {
    let digit = |octet: u8| {
        octet
            .is_ascii_digit()
            .then_some(octet.saturating_sub(b'0'))
            .ok_or(ValueError::Malformed { field })
    };
    Ok(digit(tens)?.saturating_mul(10).saturating_add(digit(ones)?))
}

/// The Winamp name for genre octet `id`.
fn genre_name(id: u8) -> Option<&'static str> {
    GENRES.get(usize::from(id)).copied()
}

/// A decimal genre index written as text.
fn parse_decimal_genre(text: &str) -> Option<&'static str> {
    if text.is_empty() || !text.bytes().all(|octet| octet.is_ascii_digit()) {
        return None;
    }
    let mut value = 0_u16;
    for octet in text.bytes() {
        let digit = u16::from(octet.saturating_sub(b'0'));
        value = value.saturating_mul(10).saturating_add(digit);
    }
    u8::try_from(value).ok().and_then(genre_name)
}

/// Expands `(nn)` references, `RX`/`CR`, and leftover genre text from a
/// value that is not blank.
fn expand_genre(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    if trimmed.bytes().all(|octet| octet.is_ascii_digit()) {
        return parse_decimal_genre(trimmed)
            .map(|name| vec![name.to_owned()])
            .unwrap_or_default();
    }
    if trimmed.eq_ignore_ascii_case("CR") {
        return vec![String::from("Cover")];
    }
    if trimmed.eq_ignore_ascii_case("RX") {
        return vec![String::from("Remix")];
    }
    let mut rest = trimmed;
    let mut names = Vec::new();
    while let Some(inner_and_after) = rest.strip_prefix('(') {
        let Some((inner, after)) = inner_and_after.split_once(')') else {
            break;
        };
        let mapped = if inner.eq_ignore_ascii_case("CR") {
            Some("Cover")
        } else if inner.eq_ignore_ascii_case("RX") {
            Some("Remix")
        } else {
            parse_decimal_genre(inner)
        };
        if let Some(name) = mapped {
            if !names.iter().any(|have| have == name) {
                names.push(name.to_owned());
            }
            rest = after;
        } else if inner.bytes().all(|octet| octet.is_ascii_digit()) {
            rest = after;
        } else {
            break;
        }
    }
    let remainder = rest
        .strip_prefix('(')
        .and_then(|tail| tail.starts_with('(').then_some(tail))
        .unwrap_or(rest);
    if !remainder.is_empty() && !names.iter().any(|have| have == remainder) {
        names.push(remainder.to_owned());
    }
    names
}

/// A `TXXX` description folded for matching.
fn txxx_key(text: &str) -> String {
    text.trim().to_ascii_lowercase().replace(' ', "_")
}

/// Whether `owner` is the `MusicBrainz` UFID owner.
fn is_mb_owner(owner: &str) -> bool {
    let owner = owner.trim().trim_end_matches('/').to_ascii_lowercase();
    owner == "http://musicbrainz.org" || owner == "https://musicbrainz.org"
}

/// White space folded and lower-cased.
fn folded(text: &str) -> String {
    text.trim()
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The role a `TIPL`/`IPLS` string names, or a performer with that detail.
fn people_role(text: &str) -> (Role, Option<String>) {
    match folded(text).as_str() {
        "composer" => (Role::Composer, None),
        "conductor" => (Role::Conductor, None),
        "lyricist" | "lyrics" => (Role::Lyricist, None),
        "producer" => (Role::Producer, None),
        "remixer" | "remix" => (Role::Remixer, None),
        "performer" => (Role::Performer, None),
        "arranger" => (Role::Arranger, None),
        "engineer" | "recording engineer" => (Role::Engineer, None),
        "mix" | "mixer" | "mixing" | "mix engineer" => (Role::Mixer, None),
        "dj-mix" | "dj mix" | "dj mixer" | "djmix" => (Role::DjMixer, None),
        "artist" => (Role::Artist, None),
        "featured" | "featuring" | "feat" | "featured artist" => (Role::Featured, None),
        "album artist" | "albumartist" => (Role::AlbumArtist, None),
        _ => (
            Role::Performer,
            Some(text.trim())
                .filter(|value| !value.is_empty())
                .map(str::to_owned),
        ),
    }
}

/// Classifies one release-type token.
fn classify_type(token: &str) -> Option<TypeToken> {
    let key = folded(token).replace(['_', '-'], " ");
    match key.as_str() {
        "album" => Some(TypeToken::Primary(PrimaryType::Album)),
        "single" => Some(TypeToken::Primary(PrimaryType::Single)),
        "ep" => Some(TypeToken::Primary(PrimaryType::Ep)),
        "broadcast" => Some(TypeToken::Primary(PrimaryType::Broadcast)),
        "other" => Some(TypeToken::Primary(PrimaryType::Other)),
        "compilation" => Some(TypeToken::Secondary(SecondaryType::Compilation)),
        "soundtrack" => Some(TypeToken::Secondary(SecondaryType::Soundtrack)),
        "spokenword" | "spoken word" => Some(TypeToken::Secondary(SecondaryType::Spokenword)),
        "interview" => Some(TypeToken::Secondary(SecondaryType::Interview)),
        "audiobook" => Some(TypeToken::Secondary(SecondaryType::Audiobook)),
        "audio drama" | "audiodrama" => Some(TypeToken::Secondary(SecondaryType::AudioDrama)),
        "live" => Some(TypeToken::Secondary(SecondaryType::Live)),
        "remix" => Some(TypeToken::Secondary(SecondaryType::Remix)),
        "dj mix" | "djmix" => Some(TypeToken::Secondary(SecondaryType::DjMix)),
        "mixtape/street" | "mixtape" => Some(TypeToken::Secondary(SecondaryType::Mixtape)),
        "demo" => Some(TypeToken::Secondary(SecondaryType::Demo)),
        "field recording" | "fieldrecording" => {
            Some(TypeToken::Secondary(SecondaryType::FieldRecording))
        }
        _ => None,
    }
}

/// Applies `token` to `release`, first primary winning and secondaries unique.
fn apply_type(token: TypeToken, release: &mut ReleaseType) {
    match token {
        TypeToken::Primary(primary) if release.primary.is_none() => {
            release.primary = Some(primary);
        }
        TypeToken::Secondary(secondary) if !release.secondary.contains(&secondary) => {
            release.secondary.push(secondary);
        }
        _ => {}
    }
}

/// Parses `raw` into primary and secondary types, splitting on `/` only
/// when the whole string is not itself a token.
fn add_release_tokens(raw: &str, release: &mut ReleaseType) {
    let raw = raw.trim();
    if raw.is_empty() {
        return;
    }
    if let Some(token) = classify_type(raw) {
        apply_type(token, release);
        return;
    }
    if raw.contains('/') {
        for part in raw.split('/') {
            add_release_tokens(part, release);
        }
        return;
    }
    for part in raw.split([';', ',']) {
        if part != raw {
            add_release_tokens(part.trim(), release);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroU8;

    use crate::catalog::{GainTags, LyricsOrigin, LyricsTiming};
    use crate::formats::id3v2::Header;

    use super::{Kind, add_release_tokens, kind, parse_decimal_genre};

    fn text(value: &str) -> Text {
        Text {
            value: value.to_owned(),
            truncated: false,
            replaced: false,
        }
    }

    fn texts(values: &[&str]) -> Vec<Text> {
        values.iter().copied().map(text).collect()
    }

    fn frame_id(id: &[u8]) -> FrameId {
        match *id {
            [a, b, c] => FrameId::Three([a, b, c]),
            [a, b, c, d] => FrameId::Four([a, b, c, d]),
            _ => panic!("a frame id is three or four octets"),
        }
    }

    fn frame(id: &[u8], body: FrameBody) -> Frame {
        Frame {
            id: frame_id(id),
            offset: 10,
            flags: 0,
            body,
        }
    }

    fn text_frame(id: &[u8], values: &[&str]) -> Frame {
        frame(id, FrameBody::Text(texts(values)))
    }

    fn user_text(id: &[u8], description: &str, values: &[&str]) -> Frame {
        frame(
            id,
            FrameBody::UserText {
                description: text(description),
                values: texts(values),
            },
        )
    }

    fn people_frame(id: &[u8], pairs: &[(&str, &str)]) -> Frame {
        frame(
            id,
            FrameBody::People(
                pairs
                    .iter()
                    .map(|(role, name)| PeopleCredit {
                        role: text(role),
                        name: text(name),
                    })
                    .collect(),
            ),
        )
    }

    /// A `SYLT` frame of `lines`, a tenth of a second apart.
    fn synced_frame(content_type: u8, lines: &[&str], truncated: bool) -> Frame {
        frame(
            b"SYLT",
            FrameBody::SyncedLyrics(SyncedLyrics {
                language: *b"eng",
                timestamp_format: 2,
                content_type,
                description: text(""),
                lines: lines
                    .iter()
                    .zip((0_u32..).step_by(100))
                    .map(|(line, time)| crate::formats::id3v2::SyncedText {
                        text: text(line),
                        time,
                    })
                    .collect(),
                truncated,
            }),
        )
    }

    fn tag(major: u8, frames: Vec<Frame>) -> Id3v2Tag {
        Id3v2Tag {
            header: Header {
                major,
                revision: 0,
                flags: 0,
                size: 0,
                len: 10,
            },
            extended: None,
            frames,
            problems: Vec::new(),
        }
    }

    fn map(v2: Option<&Id3v2Tag>, v1: Option<&Id3v1Tag>) -> Mapped {
        from_id3(v2, v1, &Limits::DEFAULT)
    }

    fn map_frames(major: u8, frames: Vec<Frame>) -> Mapped {
        let tag = tag(major, frames);
        map(Some(&tag), None)
    }

    fn map_text(major: u8, id: &[u8], value: &str) -> Mapped {
        map_frames(major, vec![text_frame(id, &[value])])
    }

    /// `frames` with frame `i` at offset `10 + 20 * i`, so that two frames
    /// with the same identifier are different sources.
    fn spaced(frames: Vec<Frame>) -> Vec<Frame> {
        frames
            .into_iter()
            .zip((10..).step_by(20))
            .map(|(frame, offset)| Frame { offset, ..frame })
            .collect()
    }

    fn v2_at(id: &[u8], offset: u64) -> FieldSource {
        FieldSource::Id3v2 {
            id: frame_id(id),
            offset,
        }
    }

    fn v2_source(id: &[u8]) -> FieldSource {
        v2_at(id, 10)
    }

    fn v1_source(field: Id3v1Field) -> FieldSource {
        FieldSource::Id3v1 { field }
    }

    fn credit(name: &str, role: Role) -> Credit {
        Credit::new(name.to_owned(), role, None, None).unwrap()
    }

    fn performer(name: &str, detail: &str) -> Credit {
        Credit::new(
            name.to_owned(),
            Role::Performer,
            Some(detail.to_owned()),
            None,
        )
        .unwrap()
    }

    fn date(year: u16, month: Option<u8>, day: Option<u8>) -> PartialDate {
        PartialDate::new(year, month, day).unwrap()
    }

    fn mbid(text: &str) -> Mbid {
        Mbid::parse(Untrusted::new(text)).unwrap()
    }

    fn isrc(text: &str) -> Isrc {
        Isrc::parse(Untrusted::new(text)).unwrap()
    }

    fn gain(db: &str) -> Gain {
        Gain {
            scale: GainScale::ReplayGain,
            gain: GainDb::parse(Untrusted::new(db)).unwrap(),
            peak: None,
        }
    }

    fn lyrics(origin: LyricsOrigin, timing: LyricsTiming, text: &str) -> TagLyrics {
        TagLyrics {
            source: LyricsSource::new(origin, timing).unwrap(),
            text: text.to_owned(),
        }
    }

    fn v1_full() -> Id3v1Tag {
        Id3v1Tag {
            range: 1_000..1_128,
            title: text("V1 Title"),
            artist: text("V1 Artist"),
            album: text("V1 Album"),
            year: text("1991"),
            comment: text("ignored"),
            track: NonZeroU8::new(7),
            genre: 17,
        }
    }

    /// The Winamp `ID3v1` genre list, written from the mutagen-specs table,
    /// independently of the mapper's table.
    const GENRE_TABLE: [&str; 192] = [
        "Blues",
        "Classic Rock",
        "Country",
        "Dance",
        "Disco",
        "Funk",
        "Grunge",
        "Hip-Hop",
        "Jazz",
        "Metal",
        "New Age",
        "Oldies",
        "Other",
        "Pop",
        "R&B",
        "Rap",
        "Reggae",
        "Rock",
        "Techno",
        "Industrial",
        "Alternative",
        "Ska",
        "Death Metal",
        "Pranks",
        "Soundtrack",
        "Euro-Techno",
        "Ambient",
        "Trip-Hop",
        "Vocal",
        "Jazz+Funk",
        "Fusion",
        "Trance",
        "Classical",
        "Instrumental",
        "Acid",
        "House",
        "Game",
        "Sound Clip",
        "Gospel",
        "Noise",
        "Alt. Rock",
        "Bass",
        "Soul",
        "Punk",
        "Space",
        "Meditative",
        "Instrumental Pop",
        "Instrumental Rock",
        "Ethnic",
        "Gothic",
        "Darkwave",
        "Techno-Industrial",
        "Electronic",
        "Pop-Folk",
        "Eurodance",
        "Dream",
        "Southern Rock",
        "Comedy",
        "Cult",
        "Gangsta Rap",
        "Top 40",
        "Christian Rap",
        "Pop/Funk",
        "Jungle",
        "Native American",
        "Cabaret",
        "New Wave",
        "Psychedelic",
        "Rave",
        "Showtunes",
        "Trailer",
        "Lo-Fi",
        "Tribal",
        "Acid Punk",
        "Acid Jazz",
        "Polka",
        "Retro",
        "Musical",
        "Rock & Roll",
        "Hard Rock",
        "Folk",
        "Folk-Rock",
        "National Folk",
        "Swing",
        "Fast-Fusion",
        "Bebop",
        "Latin",
        "Revival",
        "Celtic",
        "Bluegrass",
        "Avantgarde",
        "Gothic Rock",
        "Progressive Rock",
        "Psychedelic Rock",
        "Symphonic Rock",
        "Slow Rock",
        "Big Band",
        "Chorus",
        "Easy Listening",
        "Acoustic",
        "Humour",
        "Speech",
        "Chanson",
        "Opera",
        "Chamber Music",
        "Sonata",
        "Symphony",
        "Booty Bass",
        "Primus",
        "Porn Groove",
        "Satire",
        "Slow Jam",
        "Club",
        "Tango",
        "Samba",
        "Folklore",
        "Ballad",
        "Power Ballad",
        "Rhythmic Soul",
        "Freestyle",
        "Duet",
        "Punk Rock",
        "Drum Solo",
        "A Cappella",
        "Euro-House",
        "Dance Hall",
        "Goa",
        "Drum & Bass",
        "Club-House",
        "Hardcore",
        "Terror",
        "Indie",
        "BritPop",
        "Afro-Punk",
        "Polsk Punk",
        "Beat",
        "Christian Gangsta Rap",
        "Heavy Metal",
        "Black Metal",
        "Crossover",
        "Contemporary Christian",
        "Christian Rock",
        "Merengue",
        "Salsa",
        "Thrash Metal",
        "Anime",
        "JPop",
        "Synthpop",
        "Abstract",
        "Art Rock",
        "Baroque",
        "Bhangra",
        "Big Beat",
        "Breakbeat",
        "Chillout",
        "Downtempo",
        "Dub",
        "EBM",
        "Eclectic",
        "Electro",
        "Electroclash",
        "Emo",
        "Experimental",
        "Garage",
        "Global",
        "IDM",
        "Illbient",
        "Industro-Goth",
        "Jam Band",
        "Krautrock",
        "Leftfield",
        "Lounge",
        "Math Rock",
        "New Romantic",
        "Nu-Breakz",
        "Post-Punk",
        "Post-Rock",
        "Psytrance",
        "Shoegaze",
        "Space Rock",
        "Trop Rock",
        "World Music",
        "Neoclassical",
        "Audiobook",
        "Audio Theatre",
        "Neue Deutsche Welle",
        "Podcast",
        "Indie Rock",
        "G-Funk",
        "Dubstep",
        "Garage Rock",
        "Psybient",
    ];

    #[test]
    fn maps_nothing_when_both_tags_are_missing() {
        assert_eq!(map(None, None), Mapped::default());
    }

    #[test]
    fn maps_title_from_each_id3_version() {
        for (major, id) in [(4, &b"TIT2"[..]), (2, &b"TT2"[..])] {
            let mapped = map_text(major, id, "Blackstar");
            assert_eq!(
                (mapped.tags.title.as_deref(), mapped.sources.title),
                (Some("Blackstar"), Some(v2_source(id)))
            );
        }
    }

    #[test]
    fn maps_title_sort_from_each_sort_frame() {
        for id in [&b"TSOT"[..], &b"XSOT"[..], &b"TST"[..]] {
            let mapped = map_text(4, id, "Blackstar");
            assert_eq!(mapped.tags.title_sort.as_deref(), Some("Blackstar"));
            assert_eq!(mapped.sources.title_sort, Some(v2_source(id)));
        }
    }

    #[test]
    fn keeps_v2_3_slash_separated_artists_as_one_string() {
        let mapped = map_text(3, b"TPE1", "Bowie / Alomar");
        assert_eq!(mapped.tags.artist, ["Bowie / Alomar"]);
        assert_eq!(mapped.sources.artist, Some(v2_source(b"TPE1")));
    }

    #[test]
    fn maps_v2_4_multi_value_artists() {
        let mapped = map_frames(4, vec![text_frame(b"TPE1", &["Bowie", "Alomar"])]);
        assert_eq!(mapped.tags.artist, ["Bowie", "Alomar"]);
    }

    #[test]
    fn maps_album_artist_album_and_sorts() {
        let mapped = map_frames(
            4,
            spaced(vec![
                text_frame(b"TPE2", &["David Bowie"]),
                text_frame(b"TSO2", &["Bowie, David"]),
                text_frame(b"TALB", &["Blackstar"]),
                text_frame(b"TSOA", &["Blackstar"]),
                text_frame(b"TSOP", &["Bowie, David"]),
            ]),
        );
        assert_eq!(mapped.tags.album_artist, ["David Bowie"]);
        assert_eq!(mapped.tags.album_artist_sort, ["Bowie, David"]);
        assert_eq!(mapped.tags.album.as_deref(), Some("Blackstar"));
        assert_eq!(mapped.tags.album_sort.as_deref(), Some("Blackstar"));
        assert_eq!(mapped.tags.artist_sort, ["Bowie, David"]);
        assert_eq!(
            mapped.sources,
            FieldSources {
                album_artist: Some(v2_at(b"TPE2", 10)),
                album_artist_sort: Some(v2_at(b"TSO2", 30)),
                album: Some(v2_at(b"TALB", 50)),
                album_sort: Some(v2_at(b"TSOA", 70)),
                artist_sort: Some(v2_at(b"TSOP", 90)),
                ..FieldSources::default()
            }
        );
    }

    #[test]
    fn maps_v2_2_album_artist_and_album() {
        let mapped = map_frames(
            2,
            vec![
                text_frame(b"TP1", &["Bowie"]),
                text_frame(b"TP2", &["Bowie"]),
                text_frame(b"TAL", &["Low"]),
            ],
        );
        assert_eq!(mapped.tags.artist, ["Bowie"]);
        assert_eq!(mapped.tags.album_artist, ["Bowie"]);
        assert_eq!(mapped.tags.album.as_deref(), Some("Low"));
        assert_eq!(mapped.sources.artist, Some(v2_source(b"TP1")));
    }

    #[test]
    fn maps_v2_2_compilation_and_album_artist_sort() {
        let mapped = map_frames(
            2,
            vec![
                text_frame(b"TCP", &["1"]),
                text_frame(b"TS2", &["Bowie, David"]),
            ],
        );
        assert_eq!(
            mapped.tags,
            TrackTags {
                compilation: Some(true),
                album_artist_sort: vec![String::from("Bowie, David")],
                ..TrackTags::default()
            }
        );
        assert_eq!(
            mapped.sources,
            FieldSources {
                compilation: Some(v2_source(b"TCP")),
                album_artist_sort: Some(v2_source(b"TS2")),
                ..FieldSources::default()
            }
        );
        assert_eq!(mapped.problems, []);
    }

    #[test]
    fn maps_track_and_disc_with_totals() {
        let mapped = map_frames(
            4,
            vec![
                text_frame(b"TRCK", &["3/12"]),
                text_frame(b"TPOS", &["2/3"]),
            ],
        );
        let position = TrackPosition::new(Some(3), Some(12), Some(2), Some(3)).unwrap();
        assert_eq!(mapped.tags.position, position);
        assert_eq!(mapped.sources.track, Some(v2_source(b"TRCK")));
        assert_eq!(mapped.sources.track_total, Some(v2_source(b"TRCK")));
        assert_eq!(mapped.sources.disc, Some(v2_source(b"TPOS")));
        assert_eq!(mapped.sources.disc_total, Some(v2_source(b"TPOS")));
    }

    #[test]
    fn maps_v2_2_track_and_disc() {
        let mapped = map_frames(
            2,
            vec![text_frame(b"TRK", &["1"]), text_frame(b"TPA", &["2"])],
        );
        assert_eq!(
            mapped.tags.position,
            TrackPosition::new(Some(1), None, Some(2), None).unwrap()
        );
        assert_eq!(mapped.sources.track, Some(v2_source(b"TRK")));
        assert_eq!(mapped.sources.disc, Some(v2_source(b"TPA")));
        assert_eq!(mapped.sources.track_total, None);
        assert_eq!(mapped.sources.disc_total, None);
    }

    #[test]
    fn keeps_a_track_number_above_its_total() {
        let mapped = map_text(4, b"TRCK", "13/12");
        let position = TrackPosition::new(Some(13), Some(12), None, None).unwrap();
        assert_eq!(mapped.tags.position, position);
        assert!(mapped.tags.position.track_above_total());
        assert_eq!(mapped.problems, []);
    }

    #[test]
    fn maps_set_subtitle() {
        let mapped = map_text(4, b"TSST", "Live disc");
        assert_eq!(mapped.tags.disc_subtitle.as_deref(), Some("Live disc"));
        assert_eq!(mapped.sources.disc_subtitle, Some(v2_source(b"TSST")));
    }

    #[test]
    fn maps_v2_4_dates() {
        let mapped = map_frames(
            4,
            vec![
                text_frame(b"TDRC", &["2016-01-08T00:00:00"]),
                text_frame(b"TDOR", &["2015-12"]),
            ],
        );
        assert_eq!(mapped.tags.date, Some(date(2016, Some(1), Some(8))));
        assert_eq!(mapped.tags.original_date, Some(date(2015, Some(12), None)));
        assert_eq!(mapped.sources.date, Some(v2_source(b"TDRC")));
        assert_eq!(mapped.sources.original_date, Some(v2_source(b"TDOR")));
    }

    #[test]
    fn combines_v2_3_year_and_date() {
        let mapped = map_frames(
            3,
            vec![
                text_frame(b"TYER", &["1971"]),
                text_frame(b"TDAT", &["1712"]),
                text_frame(b"TORY", &["1969"]),
            ],
        );
        assert_eq!(mapped.tags.date, Some(date(1971, Some(12), Some(17))));
        assert_eq!(mapped.tags.original_date, Some(date(1969, None, None)));
        assert_eq!(mapped.sources.date, Some(v2_source(b"TYER")));
    }

    #[test]
    fn keeps_a_whole_date_written_in_a_year_frame() {
        let mapped = map_frames(
            3,
            vec![
                text_frame(b"TYER", &["1971-12-17"]),
                text_frame(b"TORY", &["1969-07"]),
            ],
        );
        assert_eq!(mapped.tags.date, Some(date(1971, Some(12), Some(17))));
        assert_eq!(mapped.tags.original_date, Some(date(1969, Some(7), None)));
        assert_eq!(mapped.sources.date, Some(v2_source(b"TYER")));
        assert_eq!(mapped.sources.original_date, Some(v2_source(b"TORY")));
        assert_eq!(mapped.problems, []);
    }

    #[test]
    fn combines_v2_2_year_and_date() {
        let mapped = map_frames(
            2,
            vec![text_frame(b"TYE", &["1977"]), text_frame(b"TDA", &["1401"])],
        );
        assert_eq!(mapped.tags.date, Some(date(1977, Some(1), Some(14))));
    }

    #[test]
    fn prefers_tdrc_over_tyer() {
        let mapped = map_frames(
            4,
            vec![
                text_frame(b"TYER", &["1971"]),
                text_frame(b"TDRC", &["2016"]),
            ],
        );
        assert_eq!(mapped.tags.date, Some(date(2016, None, None)));
        assert_eq!(mapped.sources.date, Some(v2_source(b"TDRC")));
    }

    #[test]
    fn maps_named_and_numeric_genres() {
        let mapped = map_frames(4, vec![text_frame(b"TCON", &["Jazz", "17", "(32)"])]);
        assert_eq!(mapped.tags.genres, ["Jazz", "Rock", "Classical"]);
        assert_eq!(mapped.sources.genres, Some(v2_source(b"TCON")));
    }

    #[test]
    fn expands_parenthesised_genre_references() {
        let mapped = map_text(3, b"TCON", "(17)(32)Soul");
        assert_eq!(mapped.tags.genres, ["Rock", "Classical", "Soul"]);
    }

    #[test]
    fn does_not_duplicate_a_genre_refinement_that_matches_the_reference() {
        let mapped = map_text(3, b"TCON", "(17)Rock");
        assert_eq!(mapped.tags.genres, ["Rock"]);
    }

    #[test]
    fn maps_remix_and_cover_genre_references() {
        let mapped = map_frames(4, vec![text_frame(b"TCON", &["(RX)", "(CR)", "RX", "CR"])]);
        assert_eq!(mapped.tags.genres, ["Remix", "Cover", "Remix", "Cover"]);
    }

    #[test]
    fn unescapes_a_leading_double_parenthesis_in_a_genre_name() {
        let mapped = map_text(3, b"TCON", "((17)");
        assert_eq!(mapped.tags.genres, ["(17)"]);
    }

    #[test]
    fn maps_every_winamp_genre_index() {
        let mapped = map_frames(
            4,
            vec![text_frame(
                b"TCON",
                &GENRE_TABLE
                    .iter()
                    .enumerate()
                    .map(|(index, _)| format!("{index}"))
                    .collect::<Vec<_>>()
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
            )],
        );
        assert_eq!(mapped.tags.genres, GENRE_TABLE);
    }

    #[test]
    fn maps_v2_2_genre() {
        let mapped = map_text(2, b"TCO", "Jazz");
        assert_eq!(mapped.tags.genres, ["Jazz"]);
    }

    #[test]
    fn maps_composer_conductor_lyricist_and_remixer() {
        let mapped = map_frames(
            4,
            vec![
                text_frame(b"TCOM", &["Eno"]),
                text_frame(b"TPE3", &["Visconti"]),
                text_frame(b"TEXT", &["Bowie"]),
                text_frame(b"TPE4", &["Moby"]),
            ],
        );
        assert_eq!(
            mapped.tags.credits,
            [
                credit("Eno", Role::Composer),
                credit("Visconti", Role::Conductor),
                credit("Bowie", Role::Lyricist),
                credit("Moby", Role::Remixer),
            ]
        );
        assert_eq!(mapped.sources.credits, Some(v2_source(b"TCOM")));
    }

    #[test]
    fn maps_v2_2_credit_frames() {
        let mapped = map_frames(
            2,
            vec![
                text_frame(b"TCM", &["Eno"]),
                text_frame(b"TP3", &["Visconti"]),
                text_frame(b"TXT", &["Bowie"]),
                text_frame(b"TP4", &["Moby"]),
            ],
        );
        assert_eq!(
            mapped.tags.credits,
            [
                credit("Eno", Role::Composer),
                credit("Visconti", Role::Conductor),
                credit("Bowie", Role::Lyricist),
                credit("Moby", Role::Remixer),
            ]
        );
    }

    #[test]
    fn maps_involved_people_and_musician_credits() {
        let mapped = map_frames(
            4,
            vec![
                people_frame(b"TIPL", &[("producer", "Visconti"), ("mix", "Eno")]),
                people_frame(b"TMCL", &[("guitar", "Alomar")]),
            ],
        );
        assert_eq!(
            mapped.tags.credits,
            [
                credit("Visconti", Role::Producer),
                credit("Eno", Role::Mixer),
                performer("Alomar", "guitar"),
            ]
        );
    }

    #[test]
    fn maps_v2_3_and_v2_2_involved_people() {
        let v23 = map_frames(3, vec![people_frame(b"IPLS", &[("arranger", "Eno")])]);
        let v22 = map_frames(2, vec![people_frame(b"IPL", &[("engineer", "Scott")])]);
        assert_eq!(v23.tags.credits, [credit("Eno", Role::Arranger)]);
        assert_eq!(v22.tags.credits, [credit("Scott", Role::Engineer)]);
    }

    #[test]
    fn maps_every_known_involved_people_role() {
        let pairs = [
            ("composer", Role::Composer),
            ("conductor", Role::Conductor),
            ("lyricist", Role::Lyricist),
            ("lyrics", Role::Lyricist),
            ("producer", Role::Producer),
            ("remixer", Role::Remixer),
            ("remix", Role::Remixer),
            ("performer", Role::Performer),
            ("arranger", Role::Arranger),
            ("engineer", Role::Engineer),
            ("recording engineer", Role::Engineer),
            ("mix", Role::Mixer),
            ("mixer", Role::Mixer),
            ("mixing", Role::Mixer),
            ("mix engineer", Role::Mixer),
            ("dj-mix", Role::DjMixer),
            ("dj mix", Role::DjMixer),
            ("dj mixer", Role::DjMixer),
            ("djmix", Role::DjMixer),
            ("artist", Role::Artist),
            ("featured", Role::Featured),
            ("featuring", Role::Featured),
            ("feat", Role::Featured),
            ("featured artist", Role::Featured),
            ("album artist", Role::AlbumArtist),
            ("albumartist", Role::AlbumArtist),
        ];
        let mapped = map_frames(
            4,
            vec![people_frame(
                b"TIPL",
                &pairs
                    .iter()
                    .map(|(role, _)| (*role, "Name"))
                    .collect::<Vec<_>>(),
            )],
        );
        let expected: Vec<Credit> = pairs
            .iter()
            .map(|(_, role)| credit("Name", *role))
            .collect();
        assert_eq!(mapped.tags.credits, expected);
    }

    #[test]
    fn keeps_an_unknown_involved_role_as_a_performer_detail() {
        let mapped = map_frames(4, vec![people_frame(b"TIPL", &[("theremin", "Clara")])]);
        assert_eq!(mapped.tags.credits, [performer("Clara", "theremin")]);
    }

    #[test]
    fn maps_compilation_grouping_mood_and_label() {
        let mapped = map_frames(
            4,
            spaced(vec![
                text_frame(b"TCMP", &["1"]),
                text_frame(b"GRP1", &["Work"]),
                text_frame(b"TIT1", &["Movement"]),
                text_frame(b"TMOO", &["Nocturnal"]),
                text_frame(b"TPUB", &["ISO"]),
            ]),
        );
        assert_eq!(mapped.tags.compilation, Some(true));
        assert_eq!(mapped.tags.grouping, ["Work", "Movement"]);
        assert_eq!(mapped.tags.moods, ["Nocturnal"]);
        assert_eq!(mapped.tags.labels, ["ISO"]);
        assert_eq!(
            mapped.sources,
            FieldSources {
                compilation: Some(v2_at(b"TCMP", 10)),
                grouping: Some(v2_at(b"GRP1", 30)),
                moods: Some(v2_at(b"TMOO", 70)),
                labels: Some(v2_at(b"TPUB", 90)),
                ..FieldSources::default()
            }
        );
    }

    #[test]
    fn maps_a_false_compilation_and_v2_2_label_and_group() {
        let mapped = map_frames(
            2,
            vec![
                text_frame(b"TCMP", &["0"]),
                text_frame(b"TT1", &["Suite"]),
                text_frame(b"TPB", &["RCA"]),
            ],
        );
        assert_eq!(mapped.tags.compilation, Some(false));
        assert_eq!(mapped.tags.grouping, ["Suite"]);
        assert_eq!(mapped.tags.labels, ["RCA"]);
    }

    #[test]
    fn maps_mood_from_txxx() {
        let mapped = map_frames(4, vec![user_text(b"TXXX", "MOOD", &["Tense"])]);
        assert_eq!(mapped.tags.moods, ["Tense"]);
        assert_eq!(
            mapped.sources,
            FieldSources {
                moods: Some(v2_source(b"TXXX")),
                ..FieldSources::default()
            }
        );
    }

    #[test]
    fn maps_isrc_from_each_version() {
        for (major, id) in [(4, &b"TSRC"[..]), (2, &b"TRC"[..])] {
            let mapped = map_text(major, id, "uss1z9900001");
            assert_eq!(mapped.tags.isrc, [isrc("USS1Z9900001")]);
            assert_eq!(mapped.sources.isrc, Some(v2_source(id)));
        }
    }

    #[test]
    fn maps_musicbrainz_ids_from_ufid_and_txxx() {
        let recording = "f81d4fae-7dec-11d0-a765-00a0c91e6bf6";
        let release = "5b11f54e-8a37-11df-8f36-0025905a5714";
        let group = "6b11f54e-8a37-11df-8f36-0025905a5714";
        let track = "7b11f54e-8a37-11df-8f36-0025905a5714";
        let artist = "8b11f54e-8a37-11df-8f36-0025905a5714";
        let album_artist = "9b11f54e-8a37-11df-8f36-0025905a5714";
        let mapped = map_frames(
            4,
            spaced(vec![
                frame(
                    b"UFID",
                    FrameBody::Ufid {
                        owner: text("http://musicbrainz.org"),
                        id: recording.as_bytes().to_vec(),
                    },
                ),
                user_text(b"TXXX", "mUsIcBrAiNz AlBuM iD", &[release]),
                user_text(b"TXXX", "MusicBrainz Release Group Id", &[group]),
                user_text(b"TXXX", "MusicBrainz Release Track Id", &[track]),
                user_text(b"TXXX", "MusicBrainz Artist Id", &[artist]),
                user_text(b"TXXX", "MusicBrainz Album Artist Id", &[album_artist]),
            ]),
        );
        assert_eq!(
            mapped.tags.musicbrainz,
            MbIds {
                recording: Some(mbid(recording)),
                track: Some(mbid(track)),
                release: Some(mbid(release)),
                release_group: Some(mbid(group)),
                artists: vec![mbid(artist)],
                album_artists: vec![mbid(album_artist)],
            }
        );
        assert_eq!(
            mapped.sources,
            FieldSources {
                recording_mbid: Some(v2_at(b"UFID", 10)),
                release_mbid: Some(v2_at(b"TXXX", 30)),
                release_group_mbid: Some(v2_at(b"TXXX", 50)),
                track_mbid: Some(v2_at(b"TXXX", 70)),
                artist_mbids: Some(v2_at(b"TXXX", 90)),
                album_artist_mbids: Some(v2_at(b"TXXX", 110)),
                ..FieldSources::default()
            }
        );
    }

    #[test]
    fn maps_ufid_from_v2_2_and_https_owner() {
        let recording = "f81d4fae-7dec-11d0-a765-00a0c91e6bf6";
        let mapped = map_frames(
            2,
            vec![frame(
                b"UFI",
                FrameBody::Ufid {
                    owner: text("https://musicbrainz.org/"),
                    id: recording.as_bytes().to_vec(),
                },
            )],
        );
        assert_eq!(mapped.tags.musicbrainz.recording, Some(mbid(recording)));
    }

    #[test]
    fn maps_replaygain_from_txxx() {
        let mapped = map_frames(
            4,
            vec![
                user_text(b"TXXX", "REPLAYGAIN_TRACK_GAIN", &["-6.5 dB"]),
                user_text(b"TXXX", "REPLAYGAIN_TRACK_PEAK", &["0.5"]),
                user_text(b"TXXX", "replaygain album gain", &["-7.0 dB"]),
                user_text(b"TXXX", "REPLAYGAIN_ALBUM_PEAK", &["0.25"]),
            ],
        );
        assert_eq!(
            mapped.tags.gain,
            GainTags {
                track: Some(Gain {
                    scale: GainScale::ReplayGain,
                    gain: GainDb::parse(Untrusted::new("-6.5 dB")).unwrap(),
                    peak: Some(PeakRatio::parse(Untrusted::new("0.5")).unwrap()),
                }),
                album: Some(Gain {
                    scale: GainScale::ReplayGain,
                    gain: GainDb::parse(Untrusted::new("-7.0 dB")).unwrap(),
                    peak: Some(PeakRatio::parse(Untrusted::new("0.25")).unwrap()),
                }),
            }
        );
        assert_eq!(mapped.sources.track_gain, Some(v2_source(b"TXXX")));
        assert_eq!(mapped.sources.album_gain, Some(v2_source(b"TXXX")));
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn drops_a_replaygain_value_outside_the_range() {
        let mapped = map_frames(
            4,
            vec![user_text(b"TXXX", "REPLAYGAIN_TRACK_GAIN", &["129 dB"])],
        );
        assert_eq!(mapped.tags.gain, GainTags::default());
        assert_eq!(
            mapped.problems,
            [TagProblem::InvalidValue {
                source: v2_source(b"TXXX"),
                error: ValueError::Unusable { field: Field::Gain },
            }]
        );
    }

    #[test]
    fn maps_lyrics_from_uslt_and_sylt() {
        let mapped = map_frames(
            4,
            vec![
                frame(
                    b"USLT",
                    FrameBody::Lyrics(LanguageText {
                        language: *b"eng",
                        description: text(""),
                        text: text("plain words"),
                    }),
                ),
                frame(
                    b"SYLT",
                    FrameBody::SyncedLyrics(SyncedLyrics {
                        language: *b"eng",
                        timestamp_format: 2,
                        content_type: 1,
                        description: text(""),
                        lines: vec![
                            crate::formats::id3v2::SyncedText {
                                text: text("Hel"),
                                time: 0,
                            },
                            crate::formats::id3v2::SyncedText {
                                text: text("lo"),
                                time: 10,
                            },
                        ],
                        truncated: false,
                    }),
                ),
            ],
        );
        assert_eq!(
            mapped.tags.lyrics,
            [
                lyrics(
                    LyricsOrigin::Id3Unsynced,
                    LyricsTiming::Plain,
                    "plain words"
                ),
                lyrics(LyricsOrigin::Id3Synced, LyricsTiming::Line, "Hel\nlo"),
            ]
        );
        assert_eq!(mapped.sources.lyrics, Some(v2_source(b"USLT")));
    }

    #[test]
    fn maps_v2_2_lyrics() {
        let mapped = map_frames(
            2,
            vec![frame(
                b"ULT",
                FrameBody::Lyrics(LanguageText {
                    language: *b"eng",
                    description: text(""),
                    text: text("old"),
                }),
            )],
        );
        assert_eq!(
            mapped.tags.lyrics,
            [lyrics(
                LyricsOrigin::Id3Unsynced,
                LyricsTiming::Plain,
                "old"
            )]
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn keeps_synced_lyrics_at_the_lyrics_limit() {
        let limits = Limits::DEFAULT
            .with_override(LimitKind::LyricsBytes, 9)
            .unwrap();
        let v2 = tag(4, vec![synced_frame(1, &["abc", "de", "fg"], false)]);
        let mapped = from_id3(Some(&v2), None, &limits);
        assert_eq!(
            mapped.tags.lyrics,
            [lyrics(
                LyricsOrigin::Id3Synced,
                LyricsTiming::Line,
                "abc\nde\nfg"
            )]
        );
        assert_eq!(mapped.problems, []);
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn cuts_synced_lyrics_past_the_lyrics_limit() {
        let limits = Limits::DEFAULT
            .with_override(LimitKind::LyricsBytes, 8)
            .unwrap();
        let v2 = tag(4, vec![synced_frame(1, &["abc", "de", "fg"], false)]);
        let mapped = from_id3(Some(&v2), None, &limits);
        assert_eq!(
            mapped.tags.lyrics,
            [lyrics(
                LyricsOrigin::Id3Synced,
                LyricsTiming::Line,
                "abc\nde"
            )]
        );
        assert_eq!(mapped.sources.lyrics, Some(v2_source(b"SYLT")));
        assert_eq!(
            mapped.problems,
            [TagProblem::LimitExceeded {
                limit: LimitKind::LyricsBytes,
                count: 9,
            }]
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn drops_a_first_synced_line_past_the_lyrics_limit() {
        let limits = Limits::DEFAULT
            .with_override(LimitKind::LyricsBytes, 2)
            .unwrap();
        let v2 = tag(4, vec![synced_frame(1, &["abc", "d"], false)]);
        let mapped = from_id3(Some(&v2), None, &limits);
        assert_eq!(mapped.tags.lyrics, [] as [TagLyrics; 0]);
        assert_eq!(mapped.sources.lyrics, None);
        assert_eq!(
            mapped.problems,
            [TagProblem::LimitExceeded {
                limit: LimitKind::LyricsBytes,
                count: 3,
            }]
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn reports_synced_lyrics_the_parser_cut_at_the_line_limit() {
        let lines: Vec<&str> = (0..10_000).map(|_| "x").collect();
        let v2 = tag(4, vec![synced_frame(1, &lines, true)]);
        let mapped = map(Some(&v2), None);
        assert_eq!(
            mapped.tags.lyrics,
            [lyrics(
                LyricsOrigin::Id3Synced,
                LyricsTiming::Line,
                &lines.join("\n")
            )]
        );
        assert_eq!(
            mapped.problems,
            [TagProblem::LimitExceeded {
                limit: LimitKind::LyricsLines,
                count: 10_001,
            }]
        );
    }

    #[test]
    fn keeps_a_blank_first_synced_line() {
        let mapped = map_frames(4, vec![synced_frame(1, &["", "la"], false)]);
        assert_eq!(
            mapped.tags.lyrics,
            [lyrics(LyricsOrigin::Id3Synced, LyricsTiming::Line, "\nla")]
        );
    }

    #[test]
    fn maps_synced_text_only_when_it_holds_words() {
        let words = [lyrics(LyricsOrigin::Id3Synced, LyricsTiming::Line, "la")];
        for content_type in 0..=2 {
            let mapped = map_frames(4, vec![synced_frame(content_type, &["la"], false)]);
            assert_eq!(mapped.tags.lyrics, words, "content type {content_type}");
        }
        for content_type in [3, 4, 5, 6, 7, 8, 255] {
            let mapped = map_frames(4, vec![synced_frame(content_type, &["la"], false)]);
            assert_eq!(mapped, Mapped::default(), "content type {content_type}");
        }
    }

    #[test]
    fn maps_release_type_and_the_explicit_flag() {
        let mapped = map_frames(
            4,
            vec![
                user_text(b"TXXX", "MusicBrainz Album Type", &["album/soundtrack"]),
                user_text(b"TXXX", "ITUNESADVISORY", &["1"]),
            ],
        );
        assert_eq!(
            mapped.tags.release_type,
            Some(ReleaseType {
                primary: Some(PrimaryType::Album),
                secondary: vec![SecondaryType::Soundtrack],
            })
        );
        assert_eq!(mapped.tags.advisory, Some(Advisory::Explicit));
        assert_eq!(mapped.sources.release_type, Some(v2_source(b"TXXX")));
        assert_eq!(mapped.sources.advisory, Some(v2_source(b"TXXX")));
    }

    #[test]
    fn maps_every_release_type_token() {
        let tokens = [
            ("album", Some(PrimaryType::Album), None),
            ("single", Some(PrimaryType::Single), None),
            ("ep", Some(PrimaryType::Ep), None),
            ("broadcast", Some(PrimaryType::Broadcast), None),
            ("other", Some(PrimaryType::Other), None),
            ("compilation", None, Some(SecondaryType::Compilation)),
            ("soundtrack", None, Some(SecondaryType::Soundtrack)),
            ("spokenword", None, Some(SecondaryType::Spokenword)),
            ("spoken word", None, Some(SecondaryType::Spokenword)),
            ("interview", None, Some(SecondaryType::Interview)),
            ("audiobook", None, Some(SecondaryType::Audiobook)),
            ("audio drama", None, Some(SecondaryType::AudioDrama)),
            ("audiodrama", None, Some(SecondaryType::AudioDrama)),
            ("live", None, Some(SecondaryType::Live)),
            ("remix", None, Some(SecondaryType::Remix)),
            ("dj-mix", None, Some(SecondaryType::DjMix)),
            ("dj mix", None, Some(SecondaryType::DjMix)),
            ("djmix", None, Some(SecondaryType::DjMix)),
            ("mixtape/street", None, Some(SecondaryType::Mixtape)),
            ("mixtape", None, Some(SecondaryType::Mixtape)),
            ("demo", None, Some(SecondaryType::Demo)),
            ("field recording", None, Some(SecondaryType::FieldRecording)),
            ("fieldrecording", None, Some(SecondaryType::FieldRecording)),
        ];
        for (token, primary, secondary) in tokens {
            let mapped = map_frames(4, vec![user_text(b"TXXX", "RELEASETYPE", &[token])]);
            assert_eq!(
                mapped.tags.release_type,
                Some(ReleaseType {
                    primary,
                    secondary: secondary.into_iter().collect(),
                })
            );
        }
    }

    #[test]
    fn maps_a_clean_advisory() {
        let mapped = map_frames(4, vec![user_text(b"TXXX", "itunesadvisory", &["2"])]);
        assert_eq!(mapped.tags.advisory, Some(Advisory::Clean));
    }

    #[test]
    fn ignores_a_zero_advisory() {
        let mapped = map_frames(4, vec![user_text(b"TXXX", "ITUNESADVISORY", &["0"])]);
        assert_eq!(mapped.tags.advisory, None);
        assert_eq!(mapped.problems, []);
    }

    #[test]
    fn v1_fills_only_fields_v2_left_empty() {
        let v2 = tag(4, vec![text_frame(b"TIT2", &["V2 Title"])]);
        let v1 = v1_full();
        let mapped = map(Some(&v2), Some(&v1));
        assert_eq!(mapped.tags.title.as_deref(), Some("V2 Title"));
        assert_eq!(mapped.sources.title, Some(v2_source(b"TIT2")));
        assert_eq!(mapped.tags.artist, ["V1 Artist"]);
        assert_eq!(mapped.sources.artist, Some(v1_source(Id3v1Field::Artist)));
        assert_eq!(mapped.tags.album.as_deref(), Some("V1 Album"));
        assert_eq!(mapped.tags.date, Some(date(1991, None, None)));
        assert_eq!(
            mapped.tags.position,
            TrackPosition::new(Some(7), None, None, None).unwrap()
        );
        assert_eq!(mapped.tags.genres, ["Rock"]);
        assert_eq!(
            mapped.sources,
            FieldSources {
                title: Some(v2_source(b"TIT2")),
                artist: Some(v1_source(Id3v1Field::Artist)),
                album: Some(v1_source(Id3v1Field::Album)),
                date: Some(v1_source(Id3v1Field::Year)),
                track: Some(v1_source(Id3v1Field::Track)),
                genres: Some(v1_source(Id3v1Field::Genre)),
                ..FieldSources::default()
            }
        );
    }

    #[test]
    fn v2_wins_when_v1_disagrees() {
        let v2 = tag(
            4,
            vec![
                text_frame(b"TIT2", &["V2 Title"]),
                text_frame(b"TPE1", &["V2 Artist"]),
                text_frame(b"TALB", &["V2 Album"]),
                text_frame(b"TDRC", &["2016"]),
                text_frame(b"TRCK", &["3"]),
                text_frame(b"TCON", &["Jazz"]),
            ],
        );
        let v1 = v1_full();
        let mapped = map(Some(&v2), Some(&v1));
        assert_eq!(mapped.tags.title.as_deref(), Some("V2 Title"));
        assert_eq!(mapped.tags.artist, ["V2 Artist"]);
        assert_eq!(mapped.tags.album.as_deref(), Some("V2 Album"));
        assert_eq!(mapped.tags.date, Some(date(2016, None, None)));
        assert_eq!(
            mapped.tags.position,
            TrackPosition::new(Some(3), None, None, None).unwrap()
        );
        assert_eq!(mapped.tags.genres, ["Jazz"]);
        assert_eq!(mapped.problems, []);
    }

    #[test]
    fn maps_v1_alone() {
        let v1 = v1_full();
        let mapped = map(None, Some(&v1));
        assert_eq!(mapped.tags.title.as_deref(), Some("V1 Title"));
        assert_eq!(mapped.sources.title, Some(v1_source(Id3v1Field::Title)));
        assert_eq!(mapped.tags.artist, ["V1 Artist"]);
        assert_eq!(mapped.tags.album.as_deref(), Some("V1 Album"));
        assert_eq!(mapped.tags.date, Some(date(1991, None, None)));
        assert_eq!(
            mapped.tags.position,
            TrackPosition::new(Some(7), None, None, None).unwrap()
        );
        assert_eq!(mapped.tags.genres, ["Rock"]);
        assert_eq!(
            mapped.sources,
            FieldSources {
                title: Some(v1_source(Id3v1Field::Title)),
                artist: Some(v1_source(Id3v1Field::Artist)),
                album: Some(v1_source(Id3v1Field::Album)),
                date: Some(v1_source(Id3v1Field::Year)),
                track: Some(v1_source(Id3v1Field::Track)),
                genres: Some(v1_source(Id3v1Field::Genre)),
                ..FieldSources::default()
            }
        );
    }

    #[test]
    fn skips_an_unset_v1_genre() {
        let mut v1 = v1_full();
        v1.genre = 255;
        v1.title = text("");
        v1.artist = text("");
        v1.album = text("");
        v1.year = text("");
        v1.track = None;
        let mapped = map(None, Some(&v1));
        assert_eq!(mapped.tags, TrackTags::default());
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn drops_the_4097th_genre() {
        let values: Vec<String> = (0..4_097).map(|index| format!("g{index}")).collect();
        let refs: Vec<&str> = values.iter().map(String::as_str).collect();
        let mapped = map_frames(4, vec![text_frame(b"TCON", &refs)]);
        assert_eq!(mapped.tags.genres.len(), 4_096);
        assert_eq!(mapped.tags.genres.first().map(String::as_str), Some("g0"));
        assert_eq!(mapped.tags.genres.last().map(String::as_str), Some("g4095"));
        assert_eq!(
            mapped.problems,
            [TagProblem::LimitExceeded {
                limit: LimitKind::TagFields,
                count: 4_097,
            }]
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn keeps_4096_genres_without_a_problem() {
        let values: Vec<String> = (0..4_096).map(|index| format!("g{index}")).collect();
        let refs: Vec<&str> = values.iter().map(String::as_str).collect();
        let mapped = map_frames(4, vec![text_frame(b"TCON", &refs)]);
        assert_eq!(mapped.tags.genres, values);
        assert_eq!(mapped.problems, []);
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn caps_every_list_at_the_tag_field_limit() {
        let limits = Limits::DEFAULT
            .with_override(LimitKind::TagFields, 3)
            .unwrap();
        let v2 = tag(
            4,
            vec![
                text_frame(b"TPE1", &["A", "B", "C", "D"]),
                text_frame(b"TCON", &["J", "K", "L", "M"]),
                text_frame(b"TCOM", &["P", "Q", "R", "S"]),
            ],
        );
        let mapped = from_id3(Some(&v2), None, &limits);
        assert_eq!(mapped.tags.artist, ["A", "B", "C"]);
        assert_eq!(mapped.tags.genres, ["J", "K", "L"]);
        assert_eq!(
            mapped.tags.credits,
            [
                credit("P", Role::Composer),
                credit("Q", Role::Composer),
                credit("R", Role::Composer)
            ]
        );
        let past_the_limit = TagProblem::LimitExceeded {
            limit: LimitKind::TagFields,
            count: 4,
        };
        assert_eq!(
            mapped.problems,
            [
                past_the_limit.clone(),
                past_the_limit.clone(),
                past_the_limit
            ]
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn keeps_every_problem_up_to_the_tag_field_limit() {
        let limits = Limits::DEFAULT
            .with_override(LimitKind::TagFields, 2)
            .unwrap();
        let artist = "f81d4fae-7dec-11d0-a765-00a0c91e6bf6";
        let v2 = tag(
            4,
            spaced(vec![
                text_frame(b"TSRC", &["a", "USS1Z9900001"]),
                user_text(b"TXXX", "MusicBrainz Artist Id", &["x", artist]),
            ]),
        );
        let mapped = from_id3(Some(&v2), None, &limits);
        assert_eq!(mapped.tags.isrc, [isrc("USS1Z9900001")]);
        assert_eq!(mapped.tags.musicbrainz.artists, [mbid(artist)]);
        assert_eq!(
            mapped.problems,
            [
                TagProblem::InvalidValue {
                    source: v2_at(b"TSRC", 10),
                    error: ValueError::Malformed { field: Field::Isrc },
                },
                TagProblem::InvalidValue {
                    source: v2_at(b"TXXX", 30),
                    error: ValueError::Malformed { field: Field::Mbid },
                },
            ]
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn caps_the_problems_at_the_tag_field_limit() {
        let limits = Limits::DEFAULT
            .with_override(LimitKind::TagFields, 2)
            .unwrap();
        let artist = "f81d4fae-7dec-11d0-a765-00a0c91e6bf6";
        let v2 = tag(
            4,
            spaced(vec![
                text_frame(b"TSRC", &["a", "b", "c", "USS1Z9900001"]),
                user_text(b"TXXX", "MusicBrainz Artist Id", &["x", artist]),
                text_frame(b"TRCK", &["0"]),
                text_frame(b"TIT2", &["Still mapped"]),
            ]),
        );
        let mapped = from_id3(Some(&v2), None, &limits);
        assert_eq!(
            mapped.tags,
            TrackTags {
                title: Some(String::from("Still mapped")),
                ..TrackTags::default()
            }
        );
        assert_eq!(
            mapped.problems,
            [
                TagProblem::InvalidValue {
                    source: v2_at(b"TSRC", 10),
                    error: ValueError::Malformed { field: Field::Isrc },
                },
                TagProblem::LimitExceeded {
                    limit: LimitKind::TagFields,
                    count: 3,
                },
            ]
        );
    }

    /// A zero field budget keeps both lists and diagnostics empty while
    /// independent scalar fields still map.
    ///
    /// Verifies: SEC-MED-006
    #[test]
    fn a_zero_field_limit_keeps_lists_and_problems_empty() {
        let limits = Limits::DEFAULT
            .with_override(LimitKind::TagFields, 0)
            .unwrap();
        let v2 = tag(
            4,
            spaced(vec![
                text_frame(b"TRCK", &["0"]),
                text_frame(b"TPE1", &["Artist"]),
                text_frame(b"TIT2", &["Still mapped"]),
            ]),
        );
        assert_eq!(
            from_id3(Some(&v2), None, &limits),
            Mapped {
                tags: TrackTags {
                    title: Some(String::from("Still mapped")),
                    ..TrackTags::default()
                },
                sources: FieldSources {
                    title: Some(v2_at(b"TIT2", 50)),
                    ..FieldSources::default()
                },
                problems: vec![],
            }
        );
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn drops_a_malformed_mbid_and_isrc() {
        let mapped = map_frames(
            4,
            vec![
                user_text(b"TXXX", "MusicBrainz Album Id", &["../../x"]),
                text_frame(b"TSRC", &["not-an-isrc"]),
            ],
        );
        assert_eq!(mapped.tags.musicbrainz, MbIds::default());
        assert_eq!(mapped.tags.isrc, [] as [Isrc; 0]);
        assert_eq!(
            mapped.problems,
            [
                TagProblem::InvalidValue {
                    source: v2_source(b"TXXX"),
                    error: ValueError::Malformed { field: Field::Mbid },
                },
                TagProblem::InvalidValue {
                    source: v2_source(b"TSRC"),
                    error: ValueError::Malformed { field: Field::Isrc },
                },
            ]
        );
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn drops_a_track_number_of_zero() {
        let mapped = map_text(4, b"TRCK", "0");
        assert_eq!(mapped.tags.position, TrackPosition::default());
        assert_eq!(
            mapped.problems,
            [TagProblem::Catalog {
                source: v2_source(b"TRCK"),
                error: CatalogError::OutOfRange {
                    part: crate::catalog::PositionPart::Track,
                    value: 0,
                },
            }]
        );
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn keeps_a_number_whose_total_is_out_of_range() {
        let mapped = map_frames(
            4,
            spaced(vec![
                text_frame(b"TRCK", &["3/0"]),
                text_frame(b"TPOS", &["2/10000"]),
            ]),
        );
        assert_eq!(
            mapped.tags.position,
            TrackPosition::new(Some(3), None, Some(2), None).unwrap()
        );
        assert_eq!(
            mapped.sources,
            FieldSources {
                track: Some(v2_at(b"TRCK", 10)),
                disc: Some(v2_at(b"TPOS", 30)),
                ..FieldSources::default()
            }
        );
        assert_eq!(
            mapped.problems,
            [
                TagProblem::Catalog {
                    source: v2_at(b"TRCK", 10),
                    error: CatalogError::OutOfRange {
                        part: crate::catalog::PositionPart::TrackTotal,
                        value: 0,
                    },
                },
                TagProblem::Catalog {
                    source: v2_at(b"TPOS", 30),
                    error: CatalogError::OutOfRange {
                        part: crate::catalog::PositionPart::DiscTotal,
                        value: 10_000,
                    },
                },
            ]
        );
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn reports_a_number_and_a_total_that_are_both_zero() {
        let mapped = map_text(4, b"TRCK", "0/0");
        assert_eq!(mapped.tags.position, TrackPosition::default());
        assert_eq!(mapped.sources, FieldSources::default());
        assert_eq!(
            mapped.problems,
            [
                TagProblem::Catalog {
                    source: v2_source(b"TRCK"),
                    error: CatalogError::OutOfRange {
                        part: crate::catalog::PositionPart::TrackTotal,
                        value: 0,
                    },
                },
                TagProblem::Catalog {
                    source: v2_source(b"TRCK"),
                    error: CatalogError::OutOfRange {
                        part: crate::catalog::PositionPart::Track,
                        value: 0,
                    },
                },
            ]
        );
    }

    #[test]
    fn splits_artist_mbids_joined_in_one_v2_3_value() {
        let first = "f81d4fae-7dec-11d0-a765-00a0c91e6bf6";
        let second = "5b11f54e-8a37-11df-8f36-0025905a5714";
        let mapped = map_frames(
            3,
            spaced(vec![
                user_text(
                    b"TXXX",
                    "MusicBrainz Artist Id",
                    &[&format!("{first}/{second}; ")],
                ),
                user_text(
                    b"TXXX",
                    "MusicBrainz Album Artist Id",
                    &[&format!("{second};{first}")],
                ),
            ]),
        );
        assert_eq!(
            mapped.tags.musicbrainz,
            MbIds {
                artists: vec![mbid(first), mbid(second)],
                album_artists: vec![mbid(second), mbid(first)],
                ..MbIds::default()
            }
        );
        assert_eq!(
            mapped.sources,
            FieldSources {
                artist_mbids: Some(v2_at(b"TXXX", 10)),
                album_artist_mbids: Some(v2_at(b"TXXX", 30)),
                ..FieldSources::default()
            }
        );
        assert_eq!(mapped.problems, []);
    }

    #[test]
    fn first_text_frame_wins_for_a_single_value() {
        let mapped = map_frames(
            4,
            vec![
                text_frame(b"TIT2", &["First"]),
                text_frame(b"TIT2", &["Second"]),
            ],
        );
        assert_eq!(mapped.tags.title.as_deref(), Some("First"));
    }

    #[test]
    fn skips_blank_text_and_unknown_frames() {
        let mapped = map_frames(
            4,
            vec![
                text_frame(b"TIT2", &["", "  "]),
                frame(
                    b"COMM",
                    FrameBody::Raw(crate::formats::id3v2::Span {
                        start: 10,
                        end: 20,
                        unsynchronised: false,
                    }),
                ),
                text_frame(b"TIT2", &["Kept"]),
            ],
        );
        assert_eq!(mapped.tags.title.as_deref(), Some("Kept"));
        assert_eq!(mapped.problems, []);
    }

    #[test]
    fn skips_a_people_credit_with_a_blank_name() {
        let mapped = map_frames(4, vec![people_frame(b"TIPL", &[("producer", "")])]);
        assert_eq!(mapped.tags.credits, [] as [Credit; 0]);
    }

    #[test]
    fn year_alone_still_maps_when_the_date_part_is_bad() {
        let mapped = map_frames(
            3,
            vec![
                text_frame(b"TYER", &["1971"]),
                text_frame(b"TDAT", &["xxxx"]),
            ],
        );
        assert_eq!(mapped.tags.date, Some(date(1971, None, None)));
        assert_eq!(
            mapped.problems,
            [TagProblem::InvalidValue {
                source: v2_source(b"TDAT"),
                error: ValueError::Malformed { field: Field::Day },
            }]
        );
    }

    #[test]
    fn drops_an_unusable_peak() {
        let mapped = map_frames(
            4,
            vec![
                user_text(b"TXXX", "REPLAYGAIN_TRACK_GAIN", &["-1.0 dB"]),
                user_text(b"TXXX", "REPLAYGAIN_TRACK_PEAK", &["20"]),
            ],
        );
        assert_eq!(mapped.tags.gain.track, Some(gain("-1.0 dB")));
        assert_eq!(
            mapped.problems,
            [TagProblem::InvalidValue {
                source: v2_source(b"TXXX"),
                error: ValueError::Unusable { field: Field::Peak },
            }]
        );
    }

    #[test]
    fn maps_v2_3_sort_frames() {
        let mapped = map_frames(
            3,
            vec![
                text_frame(b"XSOP", &["Bowie, David"]),
                text_frame(b"XSOA", &["Low"]),
            ],
        );
        assert_eq!(mapped.tags.artist_sort, ["Bowie, David"]);
        assert_eq!(mapped.tags.album_sort.as_deref(), Some("Low"));
    }

    #[test]
    fn ignores_an_unknown_ufid_owner() {
        let mapped = map_frames(
            4,
            vec![frame(
                b"UFID",
                FrameBody::Ufid {
                    owner: text("http://example.test"),
                    id: b"f81d4fae-7dec-11d0-a765-00a0c91e6bf6".to_vec(),
                },
            )],
        );
        assert_eq!(mapped.tags.musicbrainz.recording, None);
    }

    #[test]
    fn fills_a_missing_total_from_a_later_frame() {
        let mapped = map_frames(
            4,
            vec![text_frame(b"TRCK", &["3"]), text_frame(b"TRCK", &["3/12"])],
        );
        assert_eq!(
            mapped.tags.position,
            TrackPosition::new(Some(3), Some(12), None, None).unwrap()
        );
        assert_eq!(mapped.sources.track_total, Some(v2_source(b"TRCK")));
    }

    #[test]
    fn attaches_a_peak_that_arrives_before_its_gain() {
        let mapped = map_frames(
            4,
            vec![
                user_text(b"TXXX", "REPLAYGAIN_TRACK_PEAK", &["0.5"]),
                user_text(b"TXXX", "REPLAYGAIN_TRACK_GAIN", &["-1.0 dB"]),
                user_text(b"TXXX", "REPLAYGAIN_ALBUM_PEAK", &["0.25"]),
                user_text(b"TXXX", "REPLAYGAIN_ALBUM_GAIN", &["-2.0 dB"]),
            ],
        );
        assert_eq!(
            mapped.tags.gain,
            GainTags {
                track: Some(Gain {
                    scale: GainScale::ReplayGain,
                    gain: GainDb::parse(Untrusted::new("-1.0 dB")).unwrap(),
                    peak: Some(PeakRatio::parse(Untrusted::new("0.5")).unwrap()),
                }),
                album: Some(Gain {
                    scale: GainScale::ReplayGain,
                    gain: GainDb::parse(Untrusted::new("-2.0 dB")).unwrap(),
                    peak: Some(PeakRatio::parse(Untrusted::new("0.25")).unwrap()),
                }),
            }
        );
    }

    #[test]
    fn first_gain_and_peak_win() {
        let mapped = map_frames(
            4,
            vec![
                user_text(b"TXXX", "REPLAYGAIN_TRACK_GAIN", &["-1.0 dB"]),
                user_text(b"TXXX", "REPLAYGAIN_TRACK_GAIN", &["-9.0 dB"]),
                user_text(b"TXXX", "REPLAYGAIN_TRACK_PEAK", &["0.5"]),
                user_text(b"TXXX", "REPLAYGAIN_TRACK_PEAK", &["0.1"]),
            ],
        );
        assert_eq!(
            mapped.tags.gain.track,
            Some(Gain {
                scale: GainScale::ReplayGain,
                gain: GainDb::parse(Untrusted::new("-1.0 dB")).unwrap(),
                peak: Some(PeakRatio::parse(Untrusted::new("0.5")).unwrap()),
            })
        );
    }

    #[test]
    fn drops_a_ufid_that_is_not_utf8_or_not_an_mbid() {
        let bad_utf8 = map_frames(
            4,
            vec![frame(
                b"UFID",
                FrameBody::Ufid {
                    owner: text("http://musicbrainz.org"),
                    id: vec![0xFF, 0xFE],
                },
            )],
        );
        let bad_id = map_frames(
            4,
            vec![frame(
                b"UFID",
                FrameBody::Ufid {
                    owner: text("http://musicbrainz.org"),
                    id: b"not-a-uuid".to_vec(),
                },
            )],
        );
        assert_eq!(
            bad_utf8.problems,
            [TagProblem::InvalidValue {
                source: v2_source(b"UFID"),
                error: ValueError::Malformed { field: Field::Mbid },
            }]
        );
        assert_eq!(
            bad_id.problems,
            [TagProblem::InvalidValue {
                source: v2_source(b"UFID"),
                error: ValueError::Malformed { field: Field::Mbid },
            }]
        );
    }

    #[test]
    fn first_recording_mbid_wins() {
        let first = "f81d4fae-7dec-11d0-a765-00a0c91e6bf6";
        let second = "5b11f54e-8a37-11df-8f36-0025905a5714";
        let mapped = map_frames(
            4,
            vec![
                frame(
                    b"UFID",
                    FrameBody::Ufid {
                        owner: text("http://musicbrainz.org"),
                        id: first.as_bytes().to_vec(),
                    },
                ),
                frame(
                    b"UFID",
                    FrameBody::Ufid {
                        owner: text("http://musicbrainz.org"),
                        id: second.as_bytes().to_vec(),
                    },
                ),
            ],
        );
        assert_eq!(mapped.tags.musicbrainz.recording, Some(mbid(first)));
    }

    #[test]
    fn parses_a_track_written_as_of() {
        let mapped = map_text(4, b"TRCK", "3 of 12");
        assert_eq!(
            mapped.tags.position,
            TrackPosition::new(Some(3), Some(12), None, None).unwrap()
        );
    }

    #[test]
    fn drops_a_malformed_track_and_a_short_date_part() {
        let track = map_text(4, b"TRCK", "x");
        let date = map_frames(3, vec![text_frame(b"TDAT", &["12"])]);
        assert_eq!(
            track.problems,
            [TagProblem::InvalidValue {
                source: v2_source(b"TRCK"),
                error: ValueError::Malformed {
                    field: Field::Number
                },
            }]
        );
        assert_eq!(
            date.problems,
            [TagProblem::InvalidValue {
                source: v2_source(b"TDAT"),
                error: ValueError::Malformed { field: Field::Day },
            }]
        );
    }

    #[test]
    fn drops_an_impossible_combined_date_and_keeps_the_year() {
        let mapped = map_frames(
            3,
            vec![
                text_frame(b"TYER", &["1971"]),
                text_frame(b"TDAT", &["3102"]),
            ],
        );
        assert_eq!(mapped.tags.date, Some(date(1971, None, None)));
        assert_eq!(
            mapped.problems,
            [TagProblem::InvalidValue {
                source: v2_source(b"TYER"),
                error: ValueError::OutOfRange {
                    field: Field::Day,
                    value: 31,
                },
            }]
        );
    }

    #[test]
    fn drops_a_malformed_timestamp_and_year() {
        let date = map_text(4, b"TDRC", "not-a-date");
        let year = map_text(3, b"TYER", "abcd");
        assert_eq!(
            date.problems,
            [TagProblem::InvalidValue {
                source: v2_source(b"TDRC"),
                error: ValueError::Malformed { field: Field::Year },
            }]
        );
        assert_eq!(
            year.problems,
            [TagProblem::InvalidValue {
                source: v2_source(b"TYER"),
                error: ValueError::Malformed { field: Field::Year },
            }]
        );
    }

    #[test]
    fn ignores_unknown_compilation_values_and_unknown_txxx() {
        let mapped = map_frames(
            4,
            vec![
                text_frame(b"TCMP", &["yes"]),
                user_text(b"TXXX", "comment", &["nope"]),
                text_frame(b"TIT3", &["subtitle"]),
            ],
        );
        assert_eq!(mapped.tags.compilation, None);
        assert_eq!(mapped.tags.title, None);
        assert_eq!(mapped.problems, []);
    }

    #[test]
    fn skips_empty_lyrics() {
        let mapped = map_frames(
            4,
            vec![
                frame(
                    b"USLT",
                    FrameBody::Lyrics(LanguageText {
                        language: *b"eng",
                        description: text(""),
                        text: text("  "),
                    }),
                ),
                frame(
                    b"SYLT",
                    FrameBody::SyncedLyrics(SyncedLyrics {
                        language: *b"eng",
                        timestamp_format: 2,
                        content_type: 1,
                        description: text(""),
                        lines: Vec::new(),
                        truncated: false,
                    }),
                ),
            ],
        );
        assert_eq!(mapped.tags.lyrics, [] as [TagLyrics; 0]);
    }

    #[test]
    fn maps_v2_2_user_text_and_synced_lyrics() {
        let release = "5b11f54e-8a37-11df-8f36-0025905a5714";
        let mapped = map_frames(
            2,
            vec![
                user_text(b"TXX", "MusicBrainz Album Id", &[release]),
                frame(
                    b"SLT",
                    FrameBody::SyncedLyrics(SyncedLyrics {
                        language: *b"eng",
                        timestamp_format: 2,
                        content_type: 1,
                        description: text(""),
                        lines: vec![crate::formats::id3v2::SyncedText {
                            text: text("word"),
                            time: 0,
                        }],
                        truncated: false,
                    }),
                ),
            ],
        );
        assert_eq!(mapped.tags.musicbrainz.release, Some(mbid(release)));
        assert_eq!(
            mapped.tags.lyrics,
            [lyrics(LyricsOrigin::Id3Synced, LyricsTiming::Line, "word")]
        );
    }

    #[test]
    fn first_primary_type_wins_and_secondaries_stay_unique() {
        let mapped = map_frames(
            4,
            vec![user_text(
                b"TXXX",
                "MusicBrainz Album Type",
                &["single;album;compilation;compilation"],
            )],
        );
        assert_eq!(
            mapped.tags.release_type,
            Some(ReleaseType {
                primary: Some(PrimaryType::Single),
                secondary: vec![SecondaryType::Compilation],
            })
        );
    }

    #[test]
    fn skips_an_unknown_numeric_genre_and_a_v1_genre_past_the_list() {
        let v2 = map_text(4, b"TCON", "200");
        let mut v1 = v1_full();
        v1.genre = 200;
        v1.title = text("");
        v1.artist = text("");
        v1.album = text("");
        v1.year = text("");
        v1.track = None;
        let mapped_v1 = map(None, Some(&v1));
        assert_eq!(v2.tags.genres, [] as [String; 0]);
        assert_eq!(mapped_v1.tags.genres, [] as [String; 0]);
    }

    #[test]
    fn drops_a_bad_v1_year() {
        let mut v1 = v1_full();
        v1.year = text("abcd");
        v1.title = text("");
        v1.artist = text("");
        v1.album = text("");
        v1.track = None;
        v1.genre = 255;
        let mapped = map(None, Some(&v1));
        assert_eq!(mapped.tags.date, None);
        assert_eq!(
            mapped.problems,
            [TagProblem::InvalidValue {
                source: v1_source(Id3v1Field::Year),
                error: ValueError::Malformed { field: Field::Year },
            }]
        );
    }

    #[test]
    fn keeps_the_first_advisory_and_release_type() {
        let mapped = map_frames(
            4,
            vec![
                user_text(b"TXXX", "ITUNESADVISORY", &["1"]),
                user_text(b"TXXX", "ITUNESADVISORY", &["2"]),
                user_text(b"TXXX", "RELEASETYPE", &["ep"]),
                user_text(b"TXXX", "RELEASETYPE", &["album"]),
            ],
        );
        assert_eq!(mapped.tags.advisory, Some(Advisory::Explicit));
        assert_eq!(
            mapped.tags.release_type,
            Some(ReleaseType {
                primary: Some(PrimaryType::Ep),
                secondary: Vec::new(),
            })
        );
    }

    #[test]
    fn maps_v2_2_sort_names() {
        let mapped = map_frames(
            2,
            vec![
                text_frame(b"TSP", &["Bowie, David"]),
                text_frame(b"TSA", &["Low"]),
                text_frame(b"TST", &["Sound and Vision"]),
            ],
        );
        assert_eq!(mapped.tags.artist_sort, ["Bowie, David"]);
        assert_eq!(mapped.tags.album_sort.as_deref(), Some("Low"));
        assert_eq!(mapped.tags.title_sort.as_deref(), Some("Sound and Vision"));
    }

    #[test]
    fn prefers_tdor_over_tory() {
        let mapped = map_frames(
            4,
            vec![
                text_frame(b"TORY", &["1969"]),
                text_frame(b"TDOR", &["1971-11"]),
            ],
        );
        assert_eq!(mapped.tags.original_date, Some(date(1971, Some(11), None)));
        assert_eq!(mapped.sources.original_date, Some(v2_source(b"TDOR")));
    }

    #[test]
    fn maps_original_year_from_v2_2() {
        let mapped = map_text(2, b"TOR", "1969");
        assert_eq!(mapped.tags.original_date, Some(date(1969, None, None)));
    }

    #[test]
    fn ignores_a_people_body_on_a_text_frame_id() {
        let mapped = map_frames(4, vec![people_frame(b"TIT2", &[("producer", "Visconti")])]);
        assert_eq!(mapped.tags.title, None);
        assert_eq!(mapped.tags.credits, [] as [Credit; 0]);
    }

    #[test]
    fn skips_blank_composer_values() {
        let mapped = map_frames(4, vec![text_frame(b"TCOM", &["", "Eno"])]);
        assert_eq!(mapped.tags.credits, [credit("Eno", Role::Composer)]);
    }

    #[test]
    fn first_date_year_and_date_part_win() {
        let mapped = map_frames(
            4,
            vec![
                text_frame(b"TDRC", &["2016"]),
                text_frame(b"TDRC", &["1971"]),
                text_frame(b"TYER", &["1969"]),
                text_frame(b"TYER", &["1968"]),
                text_frame(b"TDAT", &["0101"]),
                text_frame(b"TDAT", &["0202"]),
                text_frame(b"TDOR", &["2015"]),
                text_frame(b"TORY", &["1969"]),
            ],
        );
        assert_eq!(mapped.tags.date, Some(date(2016, None, None)));
        assert_eq!(mapped.tags.original_date, Some(date(2015, None, None)));
    }

    #[test]
    fn skips_empty_date_year_date_part_gain_peak_advisory_and_mbid() {
        let mapped = map_frames(
            4,
            vec![
                text_frame(b"TDRC", &[""]),
                text_frame(b"TYER", &[""]),
                text_frame(b"TDAT", &[""]),
                text_frame(b"TCMP", &[""]),
                user_text(b"TXXX", "REPLAYGAIN_TRACK_GAIN", &[""]),
                user_text(b"TXXX", "REPLAYGAIN_TRACK_PEAK", &[""]),
                user_text(b"TXXX", "ITUNESADVISORY", &[""]),
                user_text(b"TXXX", "MusicBrainz Album Id", &[""]),
            ],
        );
        assert_eq!(mapped.tags, TrackTags::default());
        assert_eq!(mapped.problems, []);
    }

    #[test]
    fn first_compilation_and_mbid_win() {
        let first = "f81d4fae-7dec-11d0-a765-00a0c91e6bf6";
        let second = "5b11f54e-8a37-11df-8f36-0025905a5714";
        let mapped = map_frames(
            4,
            vec![
                text_frame(b"TCMP", &["1"]),
                text_frame(b"TCMP", &["0"]),
                user_text(b"TXXX", "MusicBrainz Album Id", &[first]),
                user_text(b"TXXX", "MusicBrainz Album Id", &[second]),
            ],
        );
        assert_eq!(mapped.tags.compilation, Some(true));
        assert_eq!(mapped.tags.musicbrainz.release, Some(mbid(first)));
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn caps_isrcs_mbids_lyrics_and_people() {
        let limits = Limits::DEFAULT
            .with_override(LimitKind::TagFields, 5)
            .unwrap();
        let isrcs: Vec<String> = (1..=6).map(|n| format!("USS1Z990000{n}")).collect();
        let isrc_refs: Vec<&str> = isrcs.iter().map(String::as_str).collect();
        let ids: Vec<String> = (1..=6)
            .map(|n| format!("{n}b11f54e-8a37-11df-8f36-0025905a5714"))
            .collect();
        let id_refs: Vec<&str> = ids.iter().map(String::as_str).collect();
        let names = ["A", "B", "C", "D", "E", "F"];
        let people: Vec<(&str, &str)> = names.iter().map(|name| ("producer", *name)).collect();
        let verses: Vec<String> = (1..=6).map(|n| format!("verse {n}")).collect();
        let mut frames = vec![
            text_frame(b"TSRC", &isrc_refs),
            user_text(b"TXXX", "MusicBrainz Artist Id", &id_refs),
            user_text(b"TXXX", "MusicBrainz Album Artist Id", &id_refs),
            people_frame(b"TIPL", &people),
        ];
        frames.extend(verses.iter().map(|verse| {
            frame(
                b"USLT",
                FrameBody::Lyrics(LanguageText {
                    language: *b"eng",
                    description: text(""),
                    text: text(verse),
                }),
            )
        }));
        let mapped = from_id3(Some(&tag(4, frames)), None, &limits);
        let first_five = |values: &[String]| values.get(..5).unwrap().to_vec();
        assert_eq!(
            mapped.tags.isrc,
            first_five(&isrcs)
                .iter()
                .map(|code| isrc(code))
                .collect::<Vec<_>>()
        );
        let mbids: Vec<Mbid> = first_five(&ids).iter().map(|id| mbid(id)).collect();
        assert_eq!(mapped.tags.musicbrainz.artists, mbids);
        assert_eq!(mapped.tags.musicbrainz.album_artists, mbids);
        assert_eq!(
            mapped.tags.credits,
            names
                .get(..5)
                .unwrap()
                .iter()
                .map(|name| credit(name, Role::Producer))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            mapped.tags.lyrics,
            first_five(&verses)
                .iter()
                .map(|verse| lyrics(LyricsOrigin::Id3Unsynced, LyricsTiming::Plain, verse))
                .collect::<Vec<_>>()
        );
        let past_the_limit = TagProblem::LimitExceeded {
            limit: LimitKind::TagFields,
            count: 6,
        };
        assert_eq!(
            mapped.problems,
            [
                past_the_limit.clone(),
                past_the_limit.clone(),
                past_the_limit.clone(),
                past_the_limit.clone(),
                past_the_limit,
            ]
        );
    }

    #[test]
    fn drops_a_malformed_artist_mbid_and_a_truncated_track_total() {
        let mapped = map_frames(
            4,
            vec![
                user_text(b"TXXX", "MusicBrainz Artist Id", &["not-an-id"]),
                text_frame(b"TRCK", &["3/"]),
            ],
        );
        assert_eq!(mapped.tags.musicbrainz.artists, [] as [Mbid; 0]);
        assert_eq!(mapped.tags.position, TrackPosition::default());
        assert_eq!(
            mapped.problems,
            [
                TagProblem::InvalidValue {
                    source: v2_source(b"TXXX"),
                    error: ValueError::Malformed { field: Field::Mbid },
                },
                TagProblem::InvalidValue {
                    source: v2_source(b"TRCK"),
                    error: ValueError::Malformed {
                        field: Field::Total
                    },
                },
            ]
        );
    }

    #[test]
    fn ignores_a_three_character_unknown_frame() {
        let mapped = map_text(2, b"XXX", "nope");
        assert_eq!(mapped.tags, TrackTags::default());
    }

    #[test]
    fn ignores_typed_bodies_on_the_wrong_frame_id() {
        let mapped = map_frames(
            4,
            vec![
                frame(
                    b"TIT2",
                    FrameBody::UserText {
                        description: text("MOOD"),
                        values: texts(&["Nocturnal"]),
                    },
                ),
                frame(
                    b"TIT2",
                    FrameBody::Ufid {
                        owner: text("http://musicbrainz.org"),
                        id: b"f81d4fae-7dec-11d0-a765-00a0c91e6bf6".to_vec(),
                    },
                ),
                frame(
                    b"TIT2",
                    FrameBody::Lyrics(LanguageText {
                        language: *b"eng",
                        description: text(""),
                        text: text("words"),
                    }),
                ),
                frame(
                    b"TIT2",
                    FrameBody::SyncedLyrics(SyncedLyrics {
                        language: *b"eng",
                        timestamp_format: 2,
                        content_type: 1,
                        description: text(""),
                        lines: vec![crate::formats::id3v2::SyncedText {
                            text: text("words"),
                            time: 0,
                        }],
                        truncated: false,
                    }),
                ),
            ],
        );
        assert_eq!(mapped.tags, TrackTags::default());
        assert_eq!(mapped.tags.moods, [] as [String; 0]);
        assert_eq!(mapped.tags.lyrics, [] as [crate::catalog::TagLyrics; 0]);
    }

    #[test]
    fn expands_an_out_of_range_reference_and_an_unclosed_paren() {
        let mapped = map_frames(4, vec![text_frame(b"TCON", &["(200)Jazz", "("])]);
        assert_eq!(mapped.tags.genres, ["Jazz", "("]);
    }

    #[test]
    fn splits_a_trailing_empty_release_type_token() {
        let mapped = map_frames(4, vec![user_text(b"TXXX", "RELEASETYPE", &["album;"])]);
        assert_eq!(
            mapped.tags.release_type,
            Some(ReleaseType {
                primary: Some(PrimaryType::Album),
                secondary: Vec::new(),
            })
        );
    }

    #[test]
    fn maps_a_musician_without_an_instrument() {
        let mapped = map_frames(4, vec![people_frame(b"TMCL", &[("", "Alomar")])]);
        assert_eq!(mapped.tags.credits, [credit("Alomar", Role::Performer)]);
    }

    #[test]
    fn drops_a_track_number_that_does_not_fit() {
        let mapped = map_text(4, b"TRCK", "70000");
        assert_eq!(
            mapped.problems,
            [TagProblem::InvalidValue {
                source: v2_source(b"TRCK"),
                error: ValueError::OutOfRange {
                    field: Field::Number,
                    value: 70_000,
                },
            }]
        );
    }

    #[test]
    fn empty_track_and_disc_leave_position_unset() {
        let mapped = map_frames(
            4,
            vec![text_frame(b"TRCK", &[""]), text_frame(b"TPOS", &[""])],
        );
        assert_eq!(mapped.tags.position, TrackPosition::default());
    }

    #[test]
    fn first_original_year_isrc_mbid_and_disc_total_win() {
        let first = "f81d4fae-7dec-11d0-a765-00a0c91e6bf6";
        let second = "5b11f54e-8a37-11df-8f36-0025905a5714";
        let mapped = map_frames(
            4,
            vec![
                text_frame(b"TORY", &["1969"]),
                text_frame(b"TORY", &["1971"]),
                text_frame(b"TSRC", &["USS1Z9900001", "GBUM71029604"]),
                user_text(b"TXXX", "MusicBrainz Artist Id", &[first, second]),
                text_frame(b"TPOS", &["1/2"]),
                text_frame(b"TPOS", &["1/3"]),
            ],
        );
        assert_eq!(mapped.tags.original_date, Some(date(1969, None, None)));
        assert_eq!(
            mapped.tags.isrc,
            [isrc("USS1Z9900001"), isrc("GBUM71029604")]
        );
        assert_eq!(mapped.tags.musicbrainz.artists, [mbid(first), mbid(second)]);
        assert_eq!(
            mapped.tags.position,
            TrackPosition::new(None, None, Some(1), Some(2)).unwrap()
        );
        assert_eq!(mapped.sources.disc_total, Some(v2_source(b"TPOS")));
    }

    #[test]
    fn first_pending_peaks_win_and_unknown_release_type_is_dropped() {
        let mapped = map_frames(
            4,
            vec![
                user_text(b"TXXX", "REPLAYGAIN_TRACK_PEAK", &["0.5"]),
                user_text(b"TXXX", "REPLAYGAIN_TRACK_PEAK", &["0.1"]),
                user_text(b"TXXX", "REPLAYGAIN_TRACK_GAIN", &["-1.0 dB"]),
                user_text(b"TXXX", "REPLAYGAIN_ALBUM_PEAK", &["0.25"]),
                user_text(b"TXXX", "REPLAYGAIN_ALBUM_PEAK", &["0.05"]),
                user_text(b"TXXX", "REPLAYGAIN_ALBUM_GAIN", &["-2.0 dB"]),
                user_text(b"TXXX", "RELEASETYPE", &["unknown"]),
            ],
        );
        assert_eq!(
            mapped.tags.gain.track.and_then(|gain| gain.peak),
            Some(PeakRatio::parse(Untrusted::new("0.5")).unwrap())
        );
        assert_eq!(
            mapped.tags.gain.album.and_then(|gain| gain.peak),
            Some(PeakRatio::parse(Untrusted::new("0.25")).unwrap())
        );
        assert_eq!(mapped.tags.release_type, None);
    }

    #[test]
    fn drops_a_date_part_whose_month_is_not_digits() {
        let mapped = map_text(3, b"TDAT", "01AB");
        assert_eq!(
            mapped.problems,
            [TagProblem::InvalidValue {
                source: v2_source(b"TDAT"),
                error: ValueError::Malformed {
                    field: Field::Month
                },
            }]
        );
    }

    #[test]
    fn does_not_duplicate_a_repeated_numeric_genre_reference() {
        let mapped = map_text(4, b"TCON", "(17)(17)");
        assert_eq!(mapped.tags.genres, ["Rock"]);
    }

    #[test]
    fn drops_a_date_part_whose_day_is_not_digits() {
        // Four octets, the second and third of them one character.
        let mapped = map_text(3, b"TDAT", "1\u{e9}2");
        assert_eq!(
            mapped.problems,
            [TagProblem::InvalidValue {
                source: v2_source(b"TDAT"),
                error: ValueError::Malformed { field: Field::Day },
            }]
        );
    }

    #[test]
    fn helper_edges_are_pinned() {
        assert_eq!(parse_decimal_genre(""), None);
        assert_eq!(parse_decimal_genre("17a"), None);
        assert_eq!(kind(FrameId::Three(*b"XXX")), Kind::Ignore);
        assert_eq!(kind(FrameId::Four(*b"TIT3")), Kind::Ignore);
        assert_eq!(add_release_tokens_empty(), ReleaseType::default());
    }

    #[test]
    #[should_panic(expected = "a frame id is three or four octets")]
    fn frame_id_rejects_other_lengths() {
        let _ = frame_id(b"AB");
    }

    fn add_release_tokens_empty() -> ReleaseType {
        let mut release = ReleaseType::default();
        add_release_tokens("   ", &mut release);
        add_release_tokens("unknown", &mut release);
        release
    }
}
