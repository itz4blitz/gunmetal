//! Vorbis comments mapped onto [`TrackTags`], for FLAC, Ogg Vorbis and Opus.
//!
//! [`from_vorbis`] reads the raw fields the comment parser returned
//! ([`Comments::fields`]) and fills the canonical tags. Every filled field
//! records the comment it was read from (API-CAT-08), and every value that
//! was dropped or cut is in [`Mapped::problems`] with the reason.
//!
//! # Keys
//!
//! Names are compared without regard to case. Where a field has several
//! names, they are listed in order of precedence.
//!
//! | Field | Keys |
//! |---|---|
//! | Title, its sort form | `TITLE`; `TITLESORT` |
//! | Artists | `ARTISTS`, else `ARTIST`; sort form `ARTISTSORT` |
//! | Album artists | `ALBUMARTIST`, else `ALBUM ARTIST`; sort form `ALBUMARTISTSORT` |
//! | Album, its sort form | `ALBUM`; `ALBUMSORT` |
//! | Track number and total | `TRACKNUMBER` (`3` or `3/12`); `TRACKTOTAL`, `TOTALTRACKS` |
//! | Disc number and total | `DISCNUMBER` (`1` or `1/2`); `DISCTOTAL`, `TOTALDISCS` |
//! | Disc subtitle | `DISCSUBTITLE`, `SETSUBTITLE` |
//! | Dates | `DATE`, `YEAR`; `ORIGINALDATE`, `ORIGINALYEAR` |
//! | Genres, moods, styles | `GENRE`; `MOOD`; `STYLE` |
//! | Labels | `LABEL`, else `ORGANIZATION`, else `PUBLISHER` |
//! | Groupings | `GROUPING`, else `CONTENTGROUP` |
//! | Credits | `COMPOSER`, `CONDUCTOR`, `LYRICIST`, `PRODUCER`, `REMIXER`, `ARRANGER`, `ENGINEER`, `MIXER`, `DJMIXER`, and `PERFORMER` as `Name (role)` |
//! | Compilation | `COMPILATION` (`1` or `0`) |
//! | Release type | `RELEASETYPE`, else `MUSICBRAINZ_ALBUMTYPE` |
//! | Recording codes | `ISRC` |
//! | `MusicBrainz` identifiers | `MUSICBRAINZ_TRACKID` (the recording), `MUSICBRAINZ_RELEASETRACKID`, `MUSICBRAINZ_ALBUMID`, `MUSICBRAINZ_RELEASEGROUPID`, `MUSICBRAINZ_ARTISTID`, `MUSICBRAINZ_ALBUMARTISTID` |
//! | Gains | `R128_TRACK_GAIN`, else `REPLAYGAIN_TRACK_GAIN` with `REPLAYGAIN_TRACK_PEAK`; the same for `ALBUM` |
//! | Lyrics | `LYRICS`, else `UNSYNCEDLYRICS` |
//!
//! # Rules
//!
//! - A value that is empty or only white space is not there.
//! - A field with one value takes the first usable value: of its first key,
//!   then of its next. A value that cannot be read is recorded and the next
//!   one is tried.
//! - A list takes every value of the first of its keys that has any, in
//!   block order. `ARTISTS` holds one artist per comment where `ARTIST` may
//!   hold a display credit, so it comes first; artist strings are not split
//!   here (WP-053). The names of one field written twice for compatibility,
//!   such as `ALBUMARTIST` and `ALBUM ARTIST`, are not added together.
//! - Track and disc numbers and totals are read from every comment that
//!   holds one. The first is kept, and a later one that disagrees is
//!   recorded as [`Problem::Disagrees`]. A number above its total is kept,
//!   as [`TrackPosition`] allows. A total that cannot be read does not cost
//!   the number written before it, after `/` or after `of`.
//! - An R128 gain is a whole number in Q7.8, so every 16-bit value is a
//!   gain from −128 dB to just under +128 dB; it has no peak. A
//!   `ReplayGain` peak belongs to its gain, so it is not mapped without
//!   one or beside an R128 gain.
//! - A release-type comment with words that are not release types is
//!   recorded once, however many such words it holds.
//! - Vorbis comments have no established key for a content advisory or for
//!   encoder trim, so those stay empty.
//!
//! # Limits and typed values
//!
//! Names and titles are cleaned as one line of text and cut at
//! [`LimitKind::ShortText`]; lyrics keep their lines and are cut at
//! [`LimitKind::LongText`] (SEC-MED-013). A cut is recorded as
//! [`Problem::Truncated`]. Every list stops at [`LimitKind::TagFields`]
//! values, and each value past it is recorded as [`Problem::ListFull`]
//! (SEC-MED-006). Dates, numbers, identifiers, gains and peaks are read by
//! the validators in [`values`](crate::values); a value outside its range is
//! dropped and recorded as [`Problem::InvalidValue`] (SEC-MED-014).
//!
//! Each comment is read for one field only, and gives at most two
//! problems (a cut and then a full list, for example), so
//! [`Mapped::problems`] is bounded by the comments the parser kept under
//! [`LimitKind::TagFields`].
//!
//! # Work
//!
//! The mapper looks through the fields once for each key it knows, a fixed
//! number, and reads each value a fixed number of times, so its work is
//! linear in the octets of the fields. Lyrics are read by the lyrics parser
//! under its own step budget (SEC-MED-007) to find out how they are timed.

use std::num::IntErrorKind;

use crate::catalog::{
    Credit, Gain, GainScale, LyricsOrigin, LyricsSource, LyricsTiming, PositionPart, PrimaryType,
    ReleaseType, Role, SecondaryType, TagLyrics, TrackPosition, TrackTags,
};
use crate::formats::vorbis_comment::{Comments, Field};
use crate::lyrics::{self, LRC_FIXED_STEPS, LRC_STEPS_PER_OCTET, Lyrics};
use crate::parse::{Budget, LimitKind, Limits, ParseFault};
use crate::text::{self, Lines};
use crate::untrusted::Untrusted;
use crate::values::{
    Field as ValueField, GainDb, Isrc, Mbid, NumberOf, PartialDate, PeakRatio, ValueError,
};

/// The mapped tags, the source of each field, and what was dropped or cut.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Mapped {
    /// The canonical tags.
    pub tags: TrackTags,
    /// Where each filled field was read.
    pub sources: Sources,
    /// What was dropped or cut, field by field in the order of
    /// [`Sources`], and within a field in block order.
    pub problems: Vec<Problem>,
}

/// The comment a value was read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Source {
    /// The comment's place in [`Comments::fields`], counted from 0.
    pub index: usize,
}

/// Where each catalogue field was read. For a list it is where its first
/// value was read.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Sources {
    /// The title.
    pub title: Option<Source>,
    /// The title to sort by.
    pub title_sort: Option<Source>,
    /// The release's title.
    pub album: Option<Source>,
    /// The release's title to sort by.
    pub album_sort: Option<Source>,
    /// The disc's title.
    pub disc_subtitle: Option<Source>,
    /// The recording's artists.
    pub artist: Option<Source>,
    /// The recording's artists to sort by.
    pub artist_sort: Option<Source>,
    /// The release's artists.
    pub album_artist: Option<Source>,
    /// The release's artists to sort by.
    pub album_artist_sort: Option<Source>,
    /// The genres.
    pub genres: Option<Source>,
    /// The moods.
    pub moods: Option<Source>,
    /// The styles.
    pub styles: Option<Source>,
    /// The record labels.
    pub labels: Option<Source>,
    /// The groupings.
    pub grouping: Option<Source>,
    /// The credited people.
    pub credits: Option<Source>,
    /// The track number.
    pub track: Option<Source>,
    /// The number of tracks.
    pub track_total: Option<Source>,
    /// The disc number.
    pub disc: Option<Source>,
    /// The number of discs.
    pub disc_total: Option<Source>,
    /// The release date.
    pub date: Option<Source>,
    /// The original release date.
    pub original_date: Option<Source>,
    /// Whether the tags mark a compilation.
    pub compilation: Option<Source>,
    /// The release type.
    pub release_type: Option<Source>,
    /// The recording codes.
    pub isrc: Option<Source>,
    /// The recording `MusicBrainz` identifier.
    pub recording_mbid: Option<Source>,
    /// The track `MusicBrainz` identifier.
    pub track_mbid: Option<Source>,
    /// The release `MusicBrainz` identifier.
    pub release_mbid: Option<Source>,
    /// The release-group `MusicBrainz` identifier.
    pub release_group_mbid: Option<Source>,
    /// The recording's artist `MusicBrainz` identifiers.
    pub artist_mbids: Option<Source>,
    /// The release's artist `MusicBrainz` identifiers.
    pub album_artist_mbids: Option<Source>,
    /// The track gain.
    pub track_gain: Option<Source>,
    /// The album gain.
    pub album_gain: Option<Source>,
    /// The lyrics.
    pub lyrics: Option<Source>,
}

/// A value that was dropped or cut while mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    /// A date, number, identifier, gain or peak that could not be read or
    /// is outside its range, and was dropped (SEC-MED-014).
    InvalidValue {
        /// The comment that holds it.
        source: Source,
        /// Why it was dropped.
        error: ValueError,
    },
    /// A compilation flag or a release type that is not one of the values
    /// the field can take, and was dropped.
    Unrecognised {
        /// The comment that holds it.
        source: Source,
    },
    /// A text longer than its limit, cut to fit (SEC-MED-006).
    Truncated {
        /// The comment that holds it.
        source: Source,
        /// The limit it was cut at: [`LimitKind::ShortText`] or
        /// [`LimitKind::LongText`].
        limit: LimitKind,
    },
    /// A value dropped because its list already holds
    /// [`LimitKind::TagFields`] values (SEC-MED-006).
    ListFull {
        /// The comment that holds it.
        source: Source,
    },
    /// A track or disc number or total that differs from the one already
    /// read, and was dropped.
    Disagrees {
        /// The comment that holds it.
        source: Source,
        /// Which part of the position.
        part: PositionPart,
        /// The value read first, which is kept.
        kept: u16,
        /// The value dropped.
        dropped: u16,
    },
    /// Lyrics the lyrics parser refused, and which were dropped.
    Lyrics {
        /// The comment that holds them.
        source: Source,
        /// Why they were refused.
        fault: ParseFault,
    },
}

/// Maps the fields of `comments` onto [`TrackTags`] under `limits`.
#[must_use]
pub fn from_vorbis(comments: &Comments, limits: &Limits) -> Mapped {
    let mut mapper = Mapper {
        fields: &comments.fields,
        limits,
        problems: Vec::new(),
    };
    let mut tags = TrackTags::default();
    let mut sources = Sources::default();
    mapper.names(&mut tags, &mut sources);
    mapper.lists(&mut tags, &mut sources);
    mapper.position(&mut tags, &mut sources);
    mapper.release(&mut tags, &mut sources);
    mapper.identifiers(&mut tags, &mut sources);
    mapper.playback(&mut tags, &mut sources);
    Mapped {
        tags,
        sources,
        problems: mapper.problems,
    }
}

/// The values found for some keys: each with the comment that holds it.
type Found<'a> = Vec<(Source, &'a str)>;

/// A validator of [`values`](crate::values), or one written like them.
type Parse<T> = fn(Untrusted<&str>) -> Result<T, ValueError>;

/// A track or disc number or total that is kept, and where it was read.
type Kept = Option<(u16, Source)>;

/// Splits a credit into the name and the detail of the role.
type Split = fn(String) -> (String, Option<String>);

/// The keys that credit a name with a role, in the order the credits are
/// listed.
const CREDITS: [(&str, Role, Split); 10] = [
    ("COMPOSER", Role::Composer, whole),
    ("CONDUCTOR", Role::Conductor, whole),
    ("LYRICIST", Role::Lyricist, whole),
    ("PRODUCER", Role::Producer, whole),
    ("REMIXER", Role::Remixer, whole),
    ("ARRANGER", Role::Arranger, whole),
    ("ENGINEER", Role::Engineer, whole),
    ("MIXER", Role::Mixer, whole),
    ("DJMIXER", Role::DjMixer, whole),
    ("PERFORMER", Role::Performer, performer),
];

/// One word of a release type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypeToken {
    /// A primary type.
    Primary(PrimaryType),
    /// A secondary type.
    Secondary(SecondaryType),
}

/// Reads the fields of one block, collecting what it drops.
struct Mapper<'a> {
    /// The block's fields.
    fields: &'a [Field],
    /// The limits to map under.
    limits: &'a Limits,
    /// What was dropped or cut so far.
    problems: Vec<Problem>,
}

impl<'a> Mapper<'a> {
    /// The values of `key` that are not blank, in block order.
    fn values(&self, key: &str) -> Found<'a> {
        self.fields
            .iter()
            .enumerate()
            .filter(|(_, field)| field.key.eq_ignore_ascii_case(key))
            .map(|(index, field)| (Source { index }, field.value.value.as_str()))
            .filter(|(_, value)| !value.trim().is_empty())
            .collect()
    }

    /// The values of every one of `keys`, key by key.
    fn all(&self, keys: &[&str]) -> Found<'a> {
        keys.iter().flat_map(|key| self.values(key)).collect()
    }

    /// The values of the first of `keys` that has any.
    fn first(&self, keys: &[&str]) -> Found<'a> {
        keys.iter()
            .map(|key| self.values(key))
            .find(|found| !found.is_empty())
            .unwrap_or_default()
    }

    /// `raw` as text of `lines`, cut at `limit`, unless nothing is left
    /// of it.
    fn clean(
        &mut self,
        source: Source,
        raw: &str,
        lines: Lines,
        limit: LimitKind,
    ) -> Option<String> {
        // Every text limit's ceiling fits 32 bits.
        let cap = u32::try_from(self.limits.get(limit)).unwrap_or(u32::MAX);
        let text = text::normalise(Untrusted::new(raw.as_bytes()), lines, cap);
        if text.truncated {
            self.problems.push(Problem::Truncated { source, limit });
        }
        Some(text.value).filter(|value| !value.trim().is_empty())
    }

    /// `raw` as a name or title: one line of short text.
    fn name(&mut self, source: Source, raw: &str) -> Option<String> {
        self.clean(source, raw, Lines::Single, LimitKind::ShortText)
    }

    /// `raw` as `parse` reads it, recording why when it does not.
    fn typed<T>(&mut self, source: Source, raw: &str, parse: Parse<T>) -> Option<T> {
        match parse(Untrusted::new(raw.trim())) {
            Ok(value) => Some(value),
            Err(error) => {
                self.problems.push(Problem::InvalidValue { source, error });
                None
            }
        }
    }

    /// Appends `value` to `list` unless the list is full.
    fn add<T>(&mut self, list: &mut Vec<(Source, T)>, source: Source, value: T) {
        let held = u64::try_from(list.len()).unwrap_or(u64::MAX);
        if held < self.limits.get(LimitKind::TagFields) {
            list.push((source, value));
        } else {
            self.problems.push(Problem::ListFull { source });
        }
    }

    /// Appends to `list` what `read` makes of each of `found`.
    fn gather<T>(
        &mut self,
        list: &mut Vec<(Source, T)>,
        found: Found<'a>,
        read: impl Fn(&mut Self, Source, &str) -> Option<T>,
    ) {
        for (source, raw) in found {
            if let Some(value) = read(self, source, raw) {
                self.add(list, source, value);
            }
        }
    }

    /// The first usable short text among the values of `keys`.
    fn text(&mut self, keys: &[&str]) -> (Option<String>, Option<Source>) {
        self.all(keys)
            .into_iter()
            .find_map(|(source, raw)| self.name(source, raw).map(|value| (value, source)))
            .unzip()
    }

    /// Every short text of the first of `keys` that has any.
    fn texts(&mut self, keys: &[&str]) -> (Vec<String>, Option<Source>) {
        let mut list = Vec::new();
        self.gather(&mut list, self.first(keys), Self::name);
        listed(list)
    }

    /// The first value among those of `keys` that `parse` reads.
    fn value<T>(&mut self, keys: &[&str], parse: Parse<T>) -> Option<(T, Source)> {
        self.all(keys)
            .into_iter()
            .find_map(|(source, raw)| self.typed(source, raw, parse).map(|value| (value, source)))
    }

    /// Every value of `key` that `parse` reads.
    fn list<T>(&mut self, key: &str, parse: Parse<T>) -> (Vec<T>, Option<Source>) {
        let mut list = Vec::new();
        self.gather(&mut list, self.values(key), |mapper, source, raw| {
            mapper.typed(source, raw, parse)
        });
        listed(list)
    }

    /// Every credit, role by role in the order of [`CREDITS`].
    fn credits(&mut self) -> (Vec<Credit>, Option<Source>) {
        let mut list = Vec::new();
        for (key, role, split) in CREDITS {
            self.gather(&mut list, self.values(key), |mapper, source, raw| {
                let (name, detail) = split(mapper.name(source, raw)?);
                Credit::new(name, role, detail, None).ok()
            });
        }
        listed(list)
    }

    /// The number and the total a `TRACKNUMBER` or `DISCNUMBER` value
    /// holds.
    fn number_of(&mut self, source: Source, raw: &str) -> (Option<u16>, Option<u16>) {
        match NumberOf::parse(Untrusted::new(raw)) {
            Ok(number) => (Some(number.number()), number.total()),
            // Both were found in range before they were compared.
            Err(ValueError::AboveTotal { number, total }) => (Some(number), Some(total)),
            Err(error) => {
                self.problems.push(Problem::InvalidValue { source, error });
                // Split as `NumberOf::parse` does, so that the number before a
                // total it refused is read on its own.
                let lower = raw.to_ascii_lowercase();
                let number = lower
                    .split_once('/')
                    .or_else(|| lower.split_once(" of "))
                    .and_then(|(number, _)| NumberOf::parse(Untrusted::new(number)).ok())
                    .map(NumberOf::number);
                (number, None)
            }
        }
    }

    /// Keeps `value` as `part` when `kept` is empty, and records it when it
    /// differs from what is kept.
    fn settle(&mut self, part: PositionPart, kept: &mut Kept, source: Source, value: u16) {
        match *kept {
            None => *kept = Some((value, source)),
            Some((first, _)) if first != value => self.problems.push(Problem::Disagrees {
                source,
                part,
                kept: first,
                dropped: value,
            }),
            Some(_) => {}
        }
    }

    /// The number under `key` and its total, which may be written after
    /// the number or under one of `total_keys`.
    fn side(
        &mut self,
        key: &str,
        total_keys: &[&str],
        parts: (PositionPart, PositionPart),
    ) -> (Kept, Kept) {
        let mut number = None;
        let mut total = None;
        for (source, raw) in self.values(key) {
            let (first, second) = self.number_of(source, raw);
            if let Some(value) = first {
                self.settle(parts.0, &mut number, source, value);
            }
            if let Some(value) = second {
                self.settle(parts.1, &mut total, source, value);
            }
        }
        for (source, raw) in self.all(total_keys) {
            if let Some(value) = self.typed(source, raw, total_of).flatten() {
                self.settle(parts.1, &mut total, source, value);
            }
        }
        (number, total)
    }

    /// Whether the release is marked a compilation.
    fn compilation(&mut self) -> (Option<bool>, Option<Source>) {
        self.values("COMPILATION")
            .into_iter()
            .find_map(|(source, raw)| {
                let flag = flag(raw);
                if flag.is_none() {
                    self.problems.push(Problem::Unrecognised { source });
                }
                flag.map(|flag| (flag, source))
            })
            .unzip()
    }

    /// The release type: the first primary type and every secondary type
    /// once.
    fn release_type(&mut self) -> (Option<ReleaseType>, Option<Source>) {
        let mut release = ReleaseType::default();
        let mut first = None;
        for (source, raw) in self.first(&["RELEASETYPE", "MUSICBRAINZ_ALBUMTYPE"]) {
            let mut unknown = false;
            for word in raw.split([';', ',']).filter(|word| !word.trim().is_empty()) {
                let Some(token) = type_token(word) else {
                    unknown = true;
                    continue;
                };
                first.get_or_insert(source);
                match token {
                    TypeToken::Primary(primary) => {
                        release.primary.get_or_insert(primary);
                    }
                    TypeToken::Secondary(secondary) if !release.secondary.contains(&secondary) => {
                        release.secondary.push(secondary);
                    }
                    TypeToken::Secondary(_) => {}
                }
            }
            if unknown {
                self.problems.push(Problem::Unrecognised { source });
            }
        }
        (first.and(Some(release)), first)
    }

    /// The R128 gain under `r128`, else the `ReplayGain` under `gain` with
    /// the peak under `peak`.
    fn gain(&mut self, r128: &str, gain: &str, peak: &str) -> (Option<Gain>, Option<Source>) {
        let r128 = self.value(&[r128], r128_gain);
        let replay = self.value(&[gain], GainDb::parse);
        let peak = self.value(&[peak], PeakRatio::parse).map(|(peak, _)| peak);
        let r128 = r128.map(|(gain, source)| {
            let scale = GainScale::R128;
            let peak = None;
            (Gain { scale, gain, peak }, source)
        });
        let replay = replay.map(|(gain, source)| {
            let scale = GainScale::ReplayGain;
            (Gain { scale, gain, peak }, source)
        });
        r128.or(replay).unzip()
    }

    /// `raw` as lyrics, with how the lyrics parser finds them timed.
    fn lyric(&mut self, source: Source, raw: &str) -> Option<TagLyrics> {
        let text = self.clean(source, raw, Lines::Multi, LimitKind::LongText)?;
        let octets = u64::try_from(text.len()).unwrap_or(u64::MAX);
        let mut budget = Budget::for_input(octets, LRC_STEPS_PER_OCTET, LRC_FIXED_STEPS);
        match lyrics::from_vorbis(Untrusted::new(&text), self.limits, &mut budget) {
            Ok(parsed) => LyricsSource::new(LyricsOrigin::VorbisComment, timing(&parsed.lyrics))
                .ok()
                .map(|source| TagLyrics { source, text }),
            Err(fault) => {
                self.problems.push(Problem::Lyrics { source, fault });
                None
            }
        }
    }

    /// Titles: fields with one short text.
    fn names(&mut self, tags: &mut TrackTags, sources: &mut Sources) {
        (tags.title, sources.title) = self.text(&["TITLE"]);
        (tags.title_sort, sources.title_sort) = self.text(&["TITLESORT"]);
        (tags.album, sources.album) = self.text(&["ALBUM"]);
        (tags.album_sort, sources.album_sort) = self.text(&["ALBUMSORT"]);
        (tags.disc_subtitle, sources.disc_subtitle) = self.text(&["DISCSUBTITLE", "SETSUBTITLE"]);
    }

    /// Lists of short texts, and the credits.
    fn lists(&mut self, tags: &mut TrackTags, sources: &mut Sources) {
        (tags.artist, sources.artist) = self.texts(&["ARTISTS", "ARTIST"]);
        (tags.artist_sort, sources.artist_sort) = self.texts(&["ARTISTSORT"]);
        (tags.album_artist, sources.album_artist) = self.texts(&["ALBUMARTIST", "ALBUM ARTIST"]);
        (tags.album_artist_sort, sources.album_artist_sort) = self.texts(&["ALBUMARTISTSORT"]);
        (tags.genres, sources.genres) = self.texts(&["GENRE"]);
        (tags.moods, sources.moods) = self.texts(&["MOOD"]);
        (tags.styles, sources.styles) = self.texts(&["STYLE"]);
        (tags.labels, sources.labels) = self.texts(&["LABEL", "ORGANIZATION", "PUBLISHER"]);
        (tags.grouping, sources.grouping) = self.texts(&["GROUPING", "CONTENTGROUP"]);
        (tags.credits, sources.credits) = self.credits();
    }

    /// Track and disc numbers and totals.
    fn position(&mut self, tags: &mut TrackTags, sources: &mut Sources) {
        let (track, track_total) = self.side(
            "TRACKNUMBER",
            &["TRACKTOTAL", "TOTALTRACKS"],
            (PositionPart::Track, PositionPart::TrackTotal),
        );
        let (disc, disc_total) = self.side(
            "DISCNUMBER",
            &["DISCTOTAL", "TOTALDISCS"],
            (PositionPart::Disc, PositionPart::DiscTotal),
        );
        let (track, track_total) = (track.unzip(), track_total.unzip());
        let (disc, disc_total) = (disc.unzip(), disc_total.unzip());
        // Every part was read by `NumberOf`, so it is in range.
        tags.position =
            TrackPosition::new(track.0, track_total.0, disc.0, disc_total.0).unwrap_or_default();
        sources.track = track.1;
        sources.track_total = track_total.1;
        sources.disc = disc.1;
        sources.disc_total = disc_total.1;
    }

    /// Dates, the compilation flag, the release type and recording codes.
    fn release(&mut self, tags: &mut TrackTags, sources: &mut Sources) {
        (tags.date, sources.date) = self.value(&["DATE", "YEAR"], PartialDate::parse).unzip();
        (tags.original_date, sources.original_date) = self
            .value(&["ORIGINALDATE", "ORIGINALYEAR"], PartialDate::parse)
            .unzip();
        (tags.compilation, sources.compilation) = self.compilation();
        (tags.release_type, sources.release_type) = self.release_type();
        (tags.isrc, sources.isrc) = self.list("ISRC", Isrc::parse);
    }

    /// `MusicBrainz` identifiers.
    fn identifiers(&mut self, tags: &mut TrackTags, sources: &mut Sources) {
        let ids = &mut tags.musicbrainz;
        (ids.recording, sources.recording_mbid) =
            self.value(&["MUSICBRAINZ_TRACKID"], Mbid::parse).unzip();
        (ids.track, sources.track_mbid) = self
            .value(&["MUSICBRAINZ_RELEASETRACKID"], Mbid::parse)
            .unzip();
        (ids.release, sources.release_mbid) =
            self.value(&["MUSICBRAINZ_ALBUMID"], Mbid::parse).unzip();
        (ids.release_group, sources.release_group_mbid) = self
            .value(&["MUSICBRAINZ_RELEASEGROUPID"], Mbid::parse)
            .unzip();
        (ids.artists, sources.artist_mbids) = self.list("MUSICBRAINZ_ARTISTID", Mbid::parse);
        (ids.album_artists, sources.album_artist_mbids) =
            self.list("MUSICBRAINZ_ALBUMARTISTID", Mbid::parse);
    }

    /// Gains and lyrics.
    fn playback(&mut self, tags: &mut TrackTags, sources: &mut Sources) {
        (tags.gain.track, sources.track_gain) = self.gain(
            "R128_TRACK_GAIN",
            "REPLAYGAIN_TRACK_GAIN",
            "REPLAYGAIN_TRACK_PEAK",
        );
        (tags.gain.album, sources.album_gain) = self.gain(
            "R128_ALBUM_GAIN",
            "REPLAYGAIN_ALBUM_GAIN",
            "REPLAYGAIN_ALBUM_PEAK",
        );
        let mut list = Vec::new();
        let found = self.first(&["LYRICS", "UNSYNCEDLYRICS"]);
        self.gather(&mut list, found, Self::lyric);
        (tags.lyrics, sources.lyrics) = listed(list);
    }
}

/// The values of `list`, and where the first was read.
fn listed<T>(list: Vec<(Source, T)>) -> (Vec<T>, Option<Source>) {
    let first = list.first().map(|(source, _)| *source);
    (list.into_iter().map(|(_, value)| value).collect(), first)
}

/// A credit that is all name.
fn whole(credit: String) -> (String, Option<String>) {
    (credit, None)
}

/// A `PERFORMER` credit: `Name (role)` is the name and the role, and
/// anything else is all name.
fn performer(credit: String) -> (String, Option<String>) {
    let parts = credit
        .trim_end()
        .strip_suffix(')')
        .and_then(|rest| rest.rsplit_once('('));
    match parts {
        Some((name, role)) if !name.trim().is_empty() && !role.trim().is_empty() => {
            (name.trim_end().to_owned(), Some(role.trim().to_owned()))
        }
        _ => (credit, None),
    }
}

/// A total written on its own, read as the total of `1/total` so that it
/// is checked, and reported, as a total.
fn total_of(text: Untrusted<&str>) -> Result<Option<u16>, ValueError> {
    let written = format!("1/{}", text.into_inner());
    NumberOf::parse(Untrusted::new(&written)).map(NumberOf::total)
}

/// An R128 gain: a whole number of 1/256 dB that fits 16 bits.
fn r128_gain(text: Untrusted<&str>) -> Result<GainDb, ValueError> {
    let field = ValueField::Gain;
    match text.into_inner().parse::<i16>() {
        Ok(raw) => Ok(GainDb::from_q7_8(raw)),
        Err(error)
            if matches!(
                error.kind(),
                IntErrorKind::PosOverflow | IntErrorKind::NegOverflow
            ) =>
        {
            Err(ValueError::Unusable { field })
        }
        Err(_) => Err(ValueError::Malformed { field }),
    }
}

/// A flag written as `1` or `0`.
fn flag(text: &str) -> Option<bool> {
    match text.trim() {
        "1" => Some(true),
        "0" => Some(false),
        _ => None,
    }
}

/// The release type `word` names, as `MusicBrainz` writes them, in any
/// case.
fn type_token(word: &str) -> Option<TypeToken> {
    use TypeToken::{Primary, Secondary};
    Some(match word.trim().to_ascii_lowercase().as_str() {
        "album" => Primary(PrimaryType::Album),
        "single" => Primary(PrimaryType::Single),
        "ep" => Primary(PrimaryType::Ep),
        "broadcast" => Primary(PrimaryType::Broadcast),
        "other" => Primary(PrimaryType::Other),
        "compilation" => Secondary(SecondaryType::Compilation),
        "soundtrack" => Secondary(SecondaryType::Soundtrack),
        "spokenword" => Secondary(SecondaryType::Spokenword),
        "interview" => Secondary(SecondaryType::Interview),
        "audiobook" => Secondary(SecondaryType::Audiobook),
        "audio drama" => Secondary(SecondaryType::AudioDrama),
        "live" => Secondary(SecondaryType::Live),
        "remix" => Secondary(SecondaryType::Remix),
        "dj-mix" => Secondary(SecondaryType::DjMix),
        "mixtape/street" => Secondary(SecondaryType::Mixtape),
        "demo" => Secondary(SecondaryType::Demo),
        "field recording" => Secondary(SecondaryType::FieldRecording),
        _ => return None,
    })
}

/// How `lyrics` are timed.
const fn timing(lyrics: &Lyrics) -> LyricsTiming {
    match lyrics {
        Lyrics::Plain(_) => LyricsTiming::Plain,
        Lyrics::Lines(_) => LyricsTiming::Line,
        Lyrics::Words(_) => LyricsTiming::Word,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::MbIds;
    use crate::formats::vorbis_comment;
    use crate::text::Text;
    use gunmetal_testkit::vorbis_comment::CommentBlock;
    use proptest::collection::vec;
    use proptest::prelude::*;

    /// Text that was neither cut nor repaired.
    fn plain(value: &str) -> Text {
        Text {
            value: value.to_owned(),
            truncated: false,
            replaced: false,
        }
    }

    /// A parsed block that holds `fields` and nothing else.
    fn block(fields: &[(&str, &str)]) -> Comments {
        Comments {
            vendor: plain("kit"),
            fields: fields
                .iter()
                .map(|(key, value)| Field {
                    key: (*key).to_owned(),
                    value: plain(value),
                })
                .collect(),
            pictures: vec![],
            problems: vec![],
            end: 0,
        }
    }

    /// `fields` mapped under the default limits.
    fn map(fields: &[(&str, &str)]) -> Mapped {
        from_vorbis(&block(fields), &Limits::DEFAULT)
    }

    /// The default limits with each of `overrides` applied.
    fn limits(overrides: &[(LimitKind, u64)]) -> Limits {
        overrides
            .iter()
            .fold(Limits::DEFAULT, |limits, &(kind, value)| {
                limits.with_override(kind, value).unwrap()
            })
    }

    fn src(index: usize) -> Source {
        Source { index }
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    /// A mapping without problems.
    fn clean(tags: TrackTags, sources: Sources) -> Mapped {
        Mapped {
            tags,
            sources,
            problems: vec![],
        }
    }

    /// A mapping that filled nothing and recorded `problems`.
    fn only_problems(problems: Vec<Problem>) -> Mapped {
        Mapped {
            problems,
            ..Mapped::default()
        }
    }

    fn invalid(index: usize, error: ValueError) -> Problem {
        Problem::InvalidValue {
            source: src(index),
            error,
        }
    }

    fn malformed(field: ValueField) -> ValueError {
        ValueError::Malformed { field }
    }

    fn out_of_range(field: ValueField, value: u64) -> ValueError {
        ValueError::OutOfRange { field, value }
    }

    fn position(
        track: Option<u16>,
        track_total: Option<u16>,
        disc: Option<u16>,
        disc_total: Option<u16>,
    ) -> TrackPosition {
        TrackPosition::new(track, track_total, disc, disc_total).unwrap()
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

    fn credit(name: &str, role: Role, detail: Option<&str>) -> Credit {
        Credit::new(name.to_owned(), role, detail.map(str::to_owned), None).unwrap()
    }

    fn gain(scale: GainScale, db: f32, peak: Option<f32>) -> Gain {
        Gain {
            scale,
            gain: GainDb::new(db).unwrap(),
            peak: peak.map(|peak| PeakRatio::new(peak).unwrap()),
        }
    }

    fn lyrics(timing: LyricsTiming, text: &str) -> TagLyrics {
        TagLyrics {
            source: LyricsSource::new(LyricsOrigin::VorbisComment, timing).unwrap(),
            text: text.to_owned(),
        }
    }

    const RECORDING: &str = "f81d4fae-7dec-11d0-a765-00a0c91e6bf6";
    const TRACK: &str = "0e3a3e5c-1111-4222-8333-444455556666";
    const RELEASE: &str = "1b2c3d4e-aaaa-4bbb-8ccc-ddddeeeeffff";
    const GROUP: &str = "22222222-3333-4444-5555-666666666666";
    const ARTIST_A: &str = "aaaaaaaa-0000-4000-8000-000000000001";
    const ARTIST_B: &str = "bbbbbbbb-0000-4000-8000-000000000002";

    #[test]
    fn a_block_without_fields_maps_to_nothing() {
        assert_eq!(map(&[]), Mapped::default());
    }

    #[test]
    fn keys_the_mapper_does_not_know_map_to_nothing() {
        assert_eq!(
            map(&[("COMMENT", "ripped"), ("ENCODER", "kit"), ("TITLES", "x")]),
            Mapped::default()
        );
    }

    #[test]
    fn maps_each_field_of_one_short_text() {
        assert_eq!(
            map(&[
                ("TITLE", "Blue in Green"),
                ("TITLESORT", "Blue in Green, sorted"),
                ("ALBUM", "Kind of Blue"),
                ("ALBUMSORT", "Kind of Blue, sorted"),
                ("DISCSUBTITLE", "Side one"),
            ]),
            clean(
                TrackTags {
                    title: Some(String::from("Blue in Green")),
                    title_sort: Some(String::from("Blue in Green, sorted")),
                    album: Some(String::from("Kind of Blue")),
                    album_sort: Some(String::from("Kind of Blue, sorted")),
                    disc_subtitle: Some(String::from("Side one")),
                    ..TrackTags::default()
                },
                Sources {
                    title: Some(src(0)),
                    title_sort: Some(src(1)),
                    album: Some(src(2)),
                    album_sort: Some(src(3)),
                    disc_subtitle: Some(src(4)),
                    ..Sources::default()
                },
            )
        );
    }

    #[test]
    fn the_first_value_that_is_not_blank_is_the_title() {
        assert_eq!(
            map(&[("TITLE", " \t"), ("TITLE", "So What"), ("TITLE", "Other")]),
            clean(
                TrackTags {
                    title: Some(String::from("So What")),
                    ..TrackTags::default()
                },
                Sources {
                    title: Some(src(1)),
                    ..Sources::default()
                },
            )
        );
    }

    #[test]
    fn a_disc_subtitle_is_read_from_setsubtitle_only_without_discsubtitle() {
        let expected = |text: &str, index: usize| {
            clean(
                TrackTags {
                    disc_subtitle: Some(String::from(text)),
                    ..TrackTags::default()
                },
                Sources {
                    disc_subtitle: Some(src(index)),
                    ..Sources::default()
                },
            )
        };
        assert_eq!(map(&[("SETSUBTITLE", "Live")]), expected("Live", 0));
        assert_eq!(
            map(&[("SETSUBTITLE", "Live"), ("DISCSUBTITLE", "Studio")]),
            expected("Studio", 1)
        );
    }

    #[test]
    fn maps_each_list_of_short_texts_in_block_order() {
        assert_eq!(
            map(&[
                ("ARTIST", "Miles Davis"),
                ("GENRE", "Jazz"),
                ("ARTIST", "Bill Evans"),
                ("ARTISTSORT", "Davis, Miles"),
                ("ARTISTSORT", "Evans, Bill"),
                ("ALBUMARTIST", "Miles Davis Sextet"),
                ("ALBUMARTISTSORT", "Davis, Miles, Sextet"),
                ("GENRE", "Modal"),
                ("MOOD", "Calm"),
                ("MOOD", "Late"),
                ("STYLE", "Cool"),
                ("LABEL", "Columbia"),
                ("LABEL", "Legacy"),
                ("GROUPING", "Sessions"),
            ]),
            clean(
                TrackTags {
                    artist: strings(&["Miles Davis", "Bill Evans"]),
                    artist_sort: strings(&["Davis, Miles", "Evans, Bill"]),
                    album_artist: strings(&["Miles Davis Sextet"]),
                    album_artist_sort: strings(&["Davis, Miles, Sextet"]),
                    genres: strings(&["Jazz", "Modal"]),
                    moods: strings(&["Calm", "Late"]),
                    styles: strings(&["Cool"]),
                    labels: strings(&["Columbia", "Legacy"]),
                    grouping: strings(&["Sessions"]),
                    ..TrackTags::default()
                },
                Sources {
                    artist: Some(src(0)),
                    artist_sort: Some(src(3)),
                    album_artist: Some(src(5)),
                    album_artist_sort: Some(src(6)),
                    genres: Some(src(1)),
                    moods: Some(src(8)),
                    styles: Some(src(10)),
                    labels: Some(src(11)),
                    grouping: Some(src(13)),
                    ..Sources::default()
                },
            )
        );
    }

    #[test]
    fn a_blank_value_is_left_out_of_a_list() {
        assert_eq!(
            map(&[("GENRE", ""), ("GENRE", "Jazz"), ("GENRE", "  ")]),
            clean(
                TrackTags {
                    genres: strings(&["Jazz"]),
                    ..TrackTags::default()
                },
                Sources {
                    genres: Some(src(1)),
                    ..Sources::default()
                },
            )
        );
    }

    #[test]
    fn artists_come_before_artist_wherever_they_are_written() {
        let expected = |index: usize| {
            clean(
                TrackTags {
                    artist: strings(&["Miles Davis", "Bill Evans"]),
                    ..TrackTags::default()
                },
                Sources {
                    artist: Some(src(index)),
                    ..Sources::default()
                },
            )
        };
        assert_eq!(
            map(&[
                ("ARTIST", "Miles Davis feat. Bill Evans"),
                ("ARTISTS", "Miles Davis"),
                ("ARTISTS", "Bill Evans"),
            ]),
            expected(1)
        );
        assert_eq!(
            map(&[
                ("ARTISTS", "Miles Davis"),
                ("ARTISTS", "Bill Evans"),
                ("ARTIST", "Miles Davis feat. Bill Evans"),
            ]),
            expected(0)
        );
    }

    #[test]
    fn blank_artists_leave_the_artist_to_artist() {
        assert_eq!(
            map(&[("ARTISTS", " "), ("ARTIST", "Miles Davis")]),
            clean(
                TrackTags {
                    artist: strings(&["Miles Davis"]),
                    ..TrackTags::default()
                },
                Sources {
                    artist: Some(src(1)),
                    ..Sources::default()
                },
            )
        );
    }

    #[test]
    fn an_album_artist_is_read_from_either_spelling_and_not_from_both() {
        let expected = |name: &str, index: usize| {
            clean(
                TrackTags {
                    album_artist: strings(&[name]),
                    ..TrackTags::default()
                },
                Sources {
                    album_artist: Some(src(index)),
                    ..Sources::default()
                },
            )
        };
        assert_eq!(map(&[("ALBUM ARTIST", "Various")]), expected("Various", 0));
        assert_eq!(
            map(&[("ALBUM ARTIST", "Spaced"), ("ALBUMARTIST", "Joined")]),
            expected("Joined", 1)
        );
    }

    #[test]
    fn labels_and_groupings_are_read_from_their_other_keys_in_order() {
        let labels = |name: &str, index: usize| {
            clean(
                TrackTags {
                    labels: strings(&[name]),
                    ..TrackTags::default()
                },
                Sources {
                    labels: Some(src(index)),
                    ..Sources::default()
                },
            )
        };
        assert_eq!(map(&[("PUBLISHER", "Third")]), labels("Third", 0));
        assert_eq!(
            map(&[("PUBLISHER", "Third"), ("ORGANIZATION", "Second")]),
            labels("Second", 1)
        );
        assert_eq!(
            map(&[
                ("PUBLISHER", "Third"),
                ("ORGANIZATION", "Second"),
                ("LABEL", "First"),
            ]),
            labels("First", 2)
        );
        let grouping = |name: &str, index: usize| {
            clean(
                TrackTags {
                    grouping: strings(&[name]),
                    ..TrackTags::default()
                },
                Sources {
                    grouping: Some(src(index)),
                    ..Sources::default()
                },
            )
        };
        assert_eq!(map(&[("CONTENTGROUP", "Other")]), grouping("Other", 0));
        assert_eq!(
            map(&[("CONTENTGROUP", "Other"), ("GROUPING", "Main")]),
            grouping("Main", 1)
        );
    }

    #[test]
    fn a_key_in_two_cases_is_one_key() {
        assert_eq!(
            map(&[
                ("Artist", "Miles Davis"),
                ("ARTIST", "Bill Evans"),
                ("artist", "Paul Chambers"),
                ("title", "So What"),
            ]),
            clean(
                TrackTags {
                    title: Some(String::from("So What")),
                    artist: strings(&["Miles Davis", "Bill Evans", "Paul Chambers"]),
                    ..TrackTags::default()
                },
                Sources {
                    title: Some(src(3)),
                    artist: Some(src(0)),
                    ..Sources::default()
                },
            )
        );
    }

    #[test]
    fn maps_a_block_the_comment_parser_read() {
        let bytes = CommentBlock::new(b"kit")
            .field("artist", "Miles Davis")
            .field("ARTIST", "Bill Evans")
            .field("TrackNumber", "3/12")
            .field("tracktotal", "14")
            .build();
        let mut budget = Budget::for_input(0, 0, 1 << 20);
        let comments = vorbis_comment::parse(&bytes, 0, &Limits::DEFAULT, &mut budget).unwrap();
        assert_eq!(
            from_vorbis(&comments, &Limits::DEFAULT),
            Mapped {
                tags: TrackTags {
                    artist: strings(&["Miles Davis", "Bill Evans"]),
                    position: position(Some(3), Some(12), None, None),
                    ..TrackTags::default()
                },
                sources: Sources {
                    artist: Some(src(0)),
                    track: Some(src(2)),
                    track_total: Some(src(2)),
                    ..Sources::default()
                },
                problems: vec![Problem::Disagrees {
                    source: src(3),
                    part: PositionPart::TrackTotal,
                    kept: 12,
                    dropped: 14,
                }],
            }
        );
    }

    #[test]
    fn a_name_is_one_line_of_text_without_controls() {
        assert_eq!(
            map(&[
                ("TITLE", "So\nWhat\u{202E}\t!"),
                ("ARTIST", "Miles\u{7}Davis"),
                ("ALBUM", "\u{202E}\n"),
                ("COMPOSER", "\u{2066}"),
            ]),
            clean(
                TrackTags {
                    title: Some(String::from("SoWhat!")),
                    artist: strings(&["MilesDavis"]),
                    ..TrackTags::default()
                },
                Sources {
                    title: Some(src(0)),
                    artist: Some(src(1)),
                    ..Sources::default()
                },
            )
        );
    }

    #[test]
    fn maps_a_credit_for_each_role_key_in_role_order() {
        assert_eq!(
            map(&[
                ("PERFORMER", "Jaco Pastorius (fretless bass)"),
                ("DJMIXER", "Nine"),
                ("MIXER", "Eight"),
                ("ENGINEER", "Seven"),
                ("ARRANGER", "Six"),
                ("REMIXER", "Five"),
                ("PRODUCER", "Four"),
                ("LYRICIST", "Three"),
                ("CONDUCTOR", "Two"),
                ("COMPOSER", "One"),
                ("COMPOSER", "One more"),
            ]),
            clean(
                TrackTags {
                    credits: vec![
                        credit("One", Role::Composer, None),
                        credit("One more", Role::Composer, None),
                        credit("Two", Role::Conductor, None),
                        credit("Three", Role::Lyricist, None),
                        credit("Four", Role::Producer, None),
                        credit("Five", Role::Remixer, None),
                        credit("Six", Role::Arranger, None),
                        credit("Seven", Role::Engineer, None),
                        credit("Eight", Role::Mixer, None),
                        credit("Nine", Role::DjMixer, None),
                        credit("Jaco Pastorius", Role::Performer, Some("fretless bass")),
                    ],
                    ..TrackTags::default()
                },
                Sources {
                    credits: Some(src(9)),
                    ..Sources::default()
                },
            )
        );
    }

    #[test]
    fn a_name_in_brackets_is_part_of_the_name_except_for_a_performer() {
        assert_eq!(
            map(&[("COMPOSER", "Prince (the artist)")]),
            clean(
                TrackTags {
                    credits: vec![credit("Prince (the artist)", Role::Composer, None)],
                    ..TrackTags::default()
                },
                Sources {
                    credits: Some(src(0)),
                    ..Sources::default()
                },
            )
        );
    }

    #[test]
    fn a_performer_is_a_name_with_or_without_a_role_in_brackets() {
        let performers = |values: &[&str]| {
            let fields: Vec<(&str, &str)> =
                values.iter().map(|value| ("PERFORMER", *value)).collect();
            map(&fields)
        };
        assert_eq!(
            performers(&[
                "Herbie Hancock",
                "Ron Carter  ( double bass )  ",
                "Earth (band) (vocals)",
                "Tony Williams ()",
                "(drums)",
                "Wayne (sax",
                "Freddie) trumpet",
            ]),
            clean(
                TrackTags {
                    credits: vec![
                        credit("Herbie Hancock", Role::Performer, None),
                        credit("Ron Carter", Role::Performer, Some("double bass")),
                        credit("Earth (band)", Role::Performer, Some("vocals")),
                        credit("Tony Williams ()", Role::Performer, None),
                        credit("(drums)", Role::Performer, None),
                        credit("Wayne (sax", Role::Performer, None),
                        credit("Freddie) trumpet", Role::Performer, None),
                    ],
                    ..TrackTags::default()
                },
                Sources {
                    credits: Some(src(0)),
                    ..Sources::default()
                },
            )
        );
    }

    #[test]
    fn maps_a_track_and_a_disc_number_without_totals() {
        assert_eq!(
            map(&[("TRACKNUMBER", "03"), ("DISCNUMBER", "2")]),
            clean(
                TrackTags {
                    position: position(Some(3), None, Some(2), None),
                    ..TrackTags::default()
                },
                Sources {
                    track: Some(src(0)),
                    disc: Some(src(1)),
                    ..Sources::default()
                },
            )
        );
    }

    #[test]
    fn maps_totals_written_after_the_numbers() {
        assert_eq!(
            map(&[("DISCNUMBER", "2/4"), ("TRACKNUMBER", "3/12")]),
            clean(
                TrackTags {
                    position: position(Some(3), Some(12), Some(2), Some(4)),
                    ..TrackTags::default()
                },
                Sources {
                    track: Some(src(1)),
                    track_total: Some(src(1)),
                    disc: Some(src(0)),
                    disc_total: Some(src(0)),
                    ..Sources::default()
                },
            )
        );
    }

    #[test]
    fn maps_totals_written_under_each_of_their_own_keys() {
        let expected = |track_total: usize, disc_total: usize| {
            clean(
                TrackTags {
                    position: position(Some(3), Some(12), Some(2), Some(4)),
                    ..TrackTags::default()
                },
                Sources {
                    track: Some(src(0)),
                    track_total: Some(src(track_total)),
                    disc: Some(src(1)),
                    disc_total: Some(src(disc_total)),
                    ..Sources::default()
                },
            )
        };
        assert_eq!(
            map(&[
                ("TRACKNUMBER", "3"),
                ("DISCNUMBER", "2"),
                ("TRACKTOTAL", " 12 "),
                ("DISCTOTAL", "4"),
            ]),
            expected(2, 3)
        );
        assert_eq!(
            map(&[
                ("TRACKNUMBER", "3"),
                ("DISCNUMBER", "2"),
                ("TOTALDISCS", "4"),
                ("TOTALTRACKS", "12"),
            ]),
            expected(3, 2)
        );
    }

    #[test]
    fn a_total_alone_is_kept() {
        assert_eq!(
            map(&[("TRACKTOTAL", "12")]),
            clean(
                TrackTags {
                    position: position(None, Some(12), None, None),
                    ..TrackTags::default()
                },
                Sources {
                    track_total: Some(src(0)),
                    ..Sources::default()
                },
            )
        );
    }

    #[test]
    fn totals_that_agree_are_not_a_problem() {
        assert_eq!(
            map(&[
                ("TRACKNUMBER", "3/12"),
                ("TRACKTOTAL", "12"),
                ("TOTALTRACKS", "012"),
                ("TRACKNUMBER", "3"),
            ]),
            clean(
                TrackTags {
                    position: position(Some(3), Some(12), None, None),
                    ..TrackTags::default()
                },
                Sources {
                    track: Some(src(0)),
                    track_total: Some(src(0)),
                    ..Sources::default()
                },
            )
        );
    }

    #[test]
    fn a_total_that_disagrees_with_the_one_after_the_number_is_recorded() {
        assert_eq!(
            map(&[
                ("TRACKTOTAL", "14"),
                ("TRACKNUMBER", "3/12"),
                ("DISCNUMBER", "1/2"),
                ("TOTALDISCS", "3"),
            ]),
            Mapped {
                tags: TrackTags {
                    position: position(Some(3), Some(12), Some(1), Some(2)),
                    ..TrackTags::default()
                },
                sources: Sources {
                    track: Some(src(1)),
                    track_total: Some(src(1)),
                    disc: Some(src(2)),
                    disc_total: Some(src(2)),
                    ..Sources::default()
                },
                problems: vec![
                    Problem::Disagrees {
                        source: src(0),
                        part: PositionPart::TrackTotal,
                        kept: 12,
                        dropped: 14,
                    },
                    Problem::Disagrees {
                        source: src(3),
                        part: PositionPart::DiscTotal,
                        kept: 2,
                        dropped: 3,
                    },
                ],
            }
        );
    }

    #[test]
    fn numbers_and_separate_totals_that_disagree_are_recorded() {
        assert_eq!(
            map(&[
                ("TRACKNUMBER", "3"),
                ("TRACKNUMBER", "4"),
                ("TOTALTRACKS", "14"),
                ("TRACKTOTAL", "12"),
                ("DISCNUMBER", "1"),
                ("DISCNUMBER", "2"),
            ]),
            Mapped {
                tags: TrackTags {
                    position: position(Some(3), Some(12), Some(1), None),
                    ..TrackTags::default()
                },
                sources: Sources {
                    track: Some(src(0)),
                    track_total: Some(src(3)),
                    disc: Some(src(4)),
                    ..Sources::default()
                },
                problems: vec![
                    Problem::Disagrees {
                        source: src(1),
                        part: PositionPart::Track,
                        kept: 3,
                        dropped: 4,
                    },
                    Problem::Disagrees {
                        source: src(2),
                        part: PositionPart::TrackTotal,
                        kept: 12,
                        dropped: 14,
                    },
                    Problem::Disagrees {
                        source: src(5),
                        part: PositionPart::Disc,
                        kept: 1,
                        dropped: 2,
                    },
                ],
            }
        );
    }

    #[test]
    fn a_number_above_its_total_is_kept() {
        let mapped = map(&[("TRACKNUMBER", "13/12")]);
        assert_eq!(
            mapped,
            clean(
                TrackTags {
                    position: position(Some(13), Some(12), None, None),
                    ..TrackTags::default()
                },
                Sources {
                    track: Some(src(0)),
                    track_total: Some(src(0)),
                    ..Sources::default()
                },
            )
        );
        assert!(mapped.tags.position.track_above_total());
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn a_number_or_total_outside_its_range_is_dropped_with_the_reason() {
        use ValueField::{Number, Total};
        assert_eq!(
            map(&[
                ("TRACKNUMBER", "0"),
                ("TRACKNUMBER", "10000"),
                ("TRACKNUMBER", "three"),
                ("TRACKNUMBER", "-1"),
                ("TRACKNUMBER", "x/12"),
                ("TRACKNUMBER", "0/12"),
                ("TRACKTOTAL", "0"),
                ("TRACKTOTAL", "10000"),
                ("TOTALTRACKS", "many"),
                ("TOTALTRACKS", "5/7"),
                ("DISCNUMBER", "99999999999999999999999"),
                ("DISCTOTAL", "1 of 2"),
            ]),
            only_problems(vec![
                invalid(0, out_of_range(Number, 0)),
                invalid(1, out_of_range(Number, 10_000)),
                invalid(2, malformed(Number)),
                invalid(3, malformed(Number)),
                invalid(4, malformed(Number)),
                invalid(5, out_of_range(Number, 0)),
                invalid(6, out_of_range(Total, 0)),
                invalid(7, out_of_range(Total, 10_000)),
                invalid(8, malformed(Total)),
                invalid(9, malformed(Total)),
                invalid(10, out_of_range(Number, u64::MAX)),
                invalid(11, malformed(Total)),
            ])
        );
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn a_total_that_cannot_be_read_does_not_cost_the_number() {
        use ValueField::Total;
        assert_eq!(
            map(&[
                ("TRACKNUMBER", "3/0"),
                ("DISCNUMBER", "2/many"),
                ("DISCNUMBER", "2/3/4"),
            ]),
            Mapped {
                tags: TrackTags {
                    position: position(Some(3), None, Some(2), None),
                    ..TrackTags::default()
                },
                sources: Sources {
                    track: Some(src(0)),
                    disc: Some(src(1)),
                    ..Sources::default()
                },
                problems: vec![
                    invalid(0, out_of_range(Total, 0)),
                    invalid(1, malformed(Total)),
                    invalid(2, malformed(Total)),
                ],
            }
        );
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn a_total_written_after_of_that_cannot_be_read_does_not_cost_the_number() {
        use ValueField::Total;
        assert_eq!(
            map(&[("TRACKNUMBER", "3 Of many"), ("DISCNUMBER", "2 of 0")]),
            Mapped {
                tags: TrackTags {
                    position: position(Some(3), None, Some(2), None),
                    ..TrackTags::default()
                },
                sources: Sources {
                    track: Some(src(0)),
                    disc: Some(src(1)),
                    ..Sources::default()
                },
                problems: vec![
                    invalid(0, malformed(Total)),
                    invalid(1, out_of_range(Total, 0)),
                ],
            }
        );
    }

    #[test]
    fn the_limits_of_a_number_and_a_total_are_kept() {
        assert_eq!(
            map(&[("TRACKNUMBER", "1/1"), ("DISCNUMBER", "9999/9999")]),
            clean(
                TrackTags {
                    position: position(Some(1), Some(1), Some(9_999), Some(9_999)),
                    ..TrackTags::default()
                },
                Sources {
                    track: Some(src(0)),
                    track_total: Some(src(0)),
                    disc: Some(src(1)),
                    disc_total: Some(src(1)),
                    ..Sources::default()
                },
            )
        );
    }

    #[test]
    fn maps_dates_from_each_of_their_keys() {
        assert_eq!(
            map(&[("ORIGINALDATE", "1959-08"), ("DATE", "1997-03-04")]),
            clean(
                TrackTags {
                    date: Some(date(1997, Some(3), Some(4))),
                    original_date: Some(date(1959, Some(8), None)),
                    ..TrackTags::default()
                },
                Sources {
                    date: Some(src(1)),
                    original_date: Some(src(0)),
                    ..Sources::default()
                },
            )
        );
        assert_eq!(
            map(&[("YEAR", "1997"), ("ORIGINALYEAR", "1959")]),
            clean(
                TrackTags {
                    date: Some(date(1997, None, None)),
                    original_date: Some(date(1959, None, None)),
                    ..TrackTags::default()
                },
                Sources {
                    date: Some(src(0)),
                    original_date: Some(src(1)),
                    ..Sources::default()
                },
            )
        );
    }

    #[test]
    fn date_comes_before_year_wherever_they_are_written() {
        assert_eq!(
            map(&[
                ("YEAR", "1998"),
                ("ORIGINALYEAR", "1960"),
                ("DATE", "1997"),
                ("ORIGINALDATE", "1959"),
                ("DATE", "1996"),
            ]),
            clean(
                TrackTags {
                    date: Some(date(1997, None, None)),
                    original_date: Some(date(1959, None, None)),
                    ..TrackTags::default()
                },
                Sources {
                    date: Some(src(2)),
                    original_date: Some(src(3)),
                    ..Sources::default()
                },
            )
        );
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn a_date_that_cannot_be_read_is_recorded_and_the_next_one_is_tried() {
        assert_eq!(
            map(&[
                ("YEAR", "1997"),
                ("DATE", "last spring"),
                ("DATE", "1997-13-01"),
                ("ORIGINALDATE", "0000"),
                ("DATE", "  "),
            ]),
            Mapped {
                tags: TrackTags {
                    date: Some(date(1997, None, None)),
                    ..TrackTags::default()
                },
                sources: Sources {
                    date: Some(src(0)),
                    ..Sources::default()
                },
                problems: vec![
                    invalid(1, malformed(ValueField::Year)),
                    invalid(2, out_of_range(ValueField::Month, 13)),
                    invalid(3, out_of_range(ValueField::Year, 0)),
                ],
            }
        );
    }

    #[test]
    fn maps_the_compilation_flag() {
        let expected = |flag: bool| {
            clean(
                TrackTags {
                    compilation: Some(flag),
                    ..TrackTags::default()
                },
                Sources {
                    compilation: Some(src(0)),
                    ..Sources::default()
                },
            )
        };
        assert_eq!(map(&[("COMPILATION", "1")]), expected(true));
        assert_eq!(map(&[("COMPILATION", " 0 ")]), expected(false));
        assert_eq!(
            map(&[("COMPILATION", "1"), ("COMPILATION", "0")]),
            expected(true)
        );
    }

    #[test]
    fn a_compilation_flag_that_is_not_one_or_zero_is_recorded() {
        assert_eq!(
            map(&[("COMPILATION", "yes"), ("COMPILATION", "0")]),
            Mapped {
                tags: TrackTags {
                    compilation: Some(false),
                    ..TrackTags::default()
                },
                sources: Sources {
                    compilation: Some(src(1)),
                    ..Sources::default()
                },
                problems: vec![Problem::Unrecognised { source: src(0) }],
            }
        );
        assert_eq!(
            map(&[("COMPILATION", "10")]),
            only_problems(vec![Problem::Unrecognised { source: src(0) }])
        );
    }

    /// The mapping of fields that give the release type `primary` and
    /// `secondary`, first named in comment `index`.
    fn released(primary: Option<PrimaryType>, secondary: &[SecondaryType], index: usize) -> Mapped {
        clean(
            TrackTags {
                release_type: Some(ReleaseType {
                    primary,
                    secondary: secondary.to_vec(),
                }),
                ..TrackTags::default()
            },
            Sources {
                release_type: Some(src(index)),
                ..Sources::default()
            },
        )
    }

    #[test]
    fn maps_every_primary_release_type() {
        let cases = [
            ("album", PrimaryType::Album),
            ("single", PrimaryType::Single),
            ("ep", PrimaryType::Ep),
            ("broadcast", PrimaryType::Broadcast),
            ("other", PrimaryType::Other),
        ];
        for (word, primary) in cases {
            assert_eq!(
                map(&[("RELEASETYPE", word)]),
                released(Some(primary), &[], 0)
            );
        }
    }

    #[test]
    fn maps_every_secondary_release_type() {
        let cases = [
            ("compilation", SecondaryType::Compilation),
            ("soundtrack", SecondaryType::Soundtrack),
            ("spokenword", SecondaryType::Spokenword),
            ("interview", SecondaryType::Interview),
            ("audiobook", SecondaryType::Audiobook),
            ("audio drama", SecondaryType::AudioDrama),
            ("live", SecondaryType::Live),
            ("remix", SecondaryType::Remix),
            ("dj-mix", SecondaryType::DjMix),
            ("mixtape/street", SecondaryType::Mixtape),
            ("demo", SecondaryType::Demo),
            ("field recording", SecondaryType::FieldRecording),
        ];
        for (word, secondary) in cases {
            assert_eq!(
                map(&[("RELEASETYPE", word)]),
                released(None, &[secondary], 0)
            );
        }
    }

    #[test]
    fn a_release_type_is_read_from_several_comments_and_from_one() {
        let expected = |index: usize| {
            released(
                Some(PrimaryType::Album),
                &[SecondaryType::Live, SecondaryType::Compilation],
                index,
            )
        };
        assert_eq!(
            map(&[
                ("RELEASETYPE", "Album"),
                ("RELEASETYPE", " LIVE "),
                ("RELEASETYPE", "compilation"),
                ("RELEASETYPE", "live"),
                ("RELEASETYPE", "single"),
            ]),
            expected(0)
        );
        assert_eq!(
            map(&[
                ("TITLE", " "),
                ("RELEASETYPE", "album; live, compilation;; live ,single")
            ]),
            expected(1)
        );
    }

    #[test]
    fn a_release_type_is_read_from_the_older_key_only_without_releasetype() {
        assert_eq!(
            map(&[("MUSICBRAINZ_ALBUMTYPE", "ep")]),
            released(Some(PrimaryType::Ep), &[], 0)
        );
        assert_eq!(
            map(&[("MUSICBRAINZ_ALBUMTYPE", "ep"), ("RELEASETYPE", "single")]),
            released(Some(PrimaryType::Single), &[], 1)
        );
    }

    #[test]
    fn a_release_type_that_is_not_known_is_recorded() {
        assert_eq!(
            map(&[("RELEASETYPE", "bootleg; live"), ("RELEASETYPE", "albums")]),
            Mapped {
                problems: vec![
                    Problem::Unrecognised { source: src(0) },
                    Problem::Unrecognised { source: src(1) },
                ],
                ..released(None, &[SecondaryType::Live], 0)
            }
        );
        assert_eq!(
            map(&[("RELEASETYPE", "bootleg")]),
            only_problems(vec![Problem::Unrecognised { source: src(0) }])
        );
    }

    #[test]
    fn a_release_type_comment_is_recorded_once_however_many_words_are_unknown() {
        assert_eq!(
            map(&[
                ("RELEASETYPE", "bootleg; promo, live;x"),
                ("RELEASETYPE", "y,y,y"),
            ]),
            Mapped {
                problems: vec![
                    Problem::Unrecognised { source: src(0) },
                    Problem::Unrecognised { source: src(1) },
                ],
                ..released(None, &[SecondaryType::Live], 0)
            }
        );
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn maps_recording_codes_and_drops_what_is_not_one() {
        assert_eq!(
            map(&[
                ("ISRC", "USS1Z9900001"),
                ("ISRC", "not a code"),
                ("ISRC", " gb-ayE-69-00531 "),
            ]),
            Mapped {
                tags: TrackTags {
                    isrc: vec![isrc("USS1Z9900001"), isrc("GBAYE6900531")],
                    ..TrackTags::default()
                },
                sources: Sources {
                    isrc: Some(src(0)),
                    ..Sources::default()
                },
                problems: vec![invalid(1, malformed(ValueField::Isrc))],
            }
        );
    }

    #[test]
    fn maps_every_musicbrainz_identifier() {
        assert_eq!(
            map(&[
                ("MUSICBRAINZ_ALBUMARTISTID", ARTIST_B),
                ("MUSICBRAINZ_ARTISTID", ARTIST_A),
                ("MUSICBRAINZ_ARTISTID", ARTIST_B),
                ("MUSICBRAINZ_RELEASEGROUPID", GROUP),
                ("MUSICBRAINZ_ALBUMID", RELEASE),
                ("MUSICBRAINZ_RELEASETRACKID", TRACK),
                ("MUSICBRAINZ_TRACKID", RECORDING),
                ("MUSICBRAINZ_ALBUMARTISTID", ARTIST_A),
            ]),
            clean(
                TrackTags {
                    musicbrainz: MbIds {
                        recording: Some(mbid(RECORDING)),
                        track: Some(mbid(TRACK)),
                        release: Some(mbid(RELEASE)),
                        release_group: Some(mbid(GROUP)),
                        artists: vec![mbid(ARTIST_A), mbid(ARTIST_B)],
                        album_artists: vec![mbid(ARTIST_B), mbid(ARTIST_A)],
                    },
                    ..TrackTags::default()
                },
                Sources {
                    recording_mbid: Some(src(6)),
                    track_mbid: Some(src(5)),
                    release_mbid: Some(src(4)),
                    release_group_mbid: Some(src(3)),
                    artist_mbids: Some(src(1)),
                    album_artist_mbids: Some(src(0)),
                    ..Sources::default()
                },
            )
        );
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn an_identifier_that_is_not_one_is_dropped_with_the_reason() {
        let padded = format!("  {RELEASE}\t");
        assert_eq!(
            map(&[
                ("MUSICBRAINZ_TRACKID", "../../x"),
                ("MUSICBRAINZ_TRACKID", RECORDING),
                ("MUSICBRAINZ_TRACKID", "never read"),
                ("MUSICBRAINZ_ALBUMID", padded.as_str()),
                (
                    "MUSICBRAINZ_ARTISTID",
                    "{aaaaaaaa-0000-4000-8000-000000000001}"
                ),
                ("MUSICBRAINZ_ARTISTID", ARTIST_B),
                (
                    "MUSICBRAINZ_RELEASEGROUPID",
                    "22222222333344445555666666666666"
                ),
            ]),
            Mapped {
                tags: TrackTags {
                    musicbrainz: MbIds {
                        recording: Some(mbid(RECORDING)),
                        release: Some(mbid(RELEASE)),
                        artists: vec![mbid(ARTIST_B)],
                        ..MbIds::default()
                    },
                    ..TrackTags::default()
                },
                sources: Sources {
                    recording_mbid: Some(src(1)),
                    release_mbid: Some(src(3)),
                    artist_mbids: Some(src(5)),
                    ..Sources::default()
                },
                problems: vec![
                    invalid(0, malformed(ValueField::Mbid)),
                    invalid(6, malformed(ValueField::Mbid)),
                    invalid(4, malformed(ValueField::Mbid)),
                ],
            }
        );
    }

    #[test]
    fn maps_replaygain_with_its_peaks() {
        assert_eq!(
            map(&[
                ("REPLAYGAIN_ALBUM_PEAK", "1.5"),
                ("REPLAYGAIN_TRACK_GAIN", "-6.5 dB"),
                ("REPLAYGAIN_TRACK_PEAK", "0.988525"),
                ("REPLAYGAIN_ALBUM_GAIN", "+3.25 dB"),
            ]),
            clean(
                TrackTags {
                    gain: crate::catalog::GainTags {
                        track: Some(gain(GainScale::ReplayGain, -6.5, Some(0.988_525))),
                        album: Some(gain(GainScale::ReplayGain, 3.25, Some(1.5))),
                    },
                    ..TrackTags::default()
                },
                Sources {
                    track_gain: Some(src(1)),
                    album_gain: Some(src(3)),
                    ..Sources::default()
                },
            )
        );
    }

    #[test]
    fn a_gain_without_a_peak_is_kept_and_a_peak_without_a_gain_is_not() {
        assert_eq!(
            map(&[
                ("REPLAYGAIN_TRACK_GAIN", "-6.5 dB"),
                ("REPLAYGAIN_ALBUM_PEAK", "1.5"),
            ]),
            clean(
                TrackTags {
                    gain: crate::catalog::GainTags {
                        track: Some(gain(GainScale::ReplayGain, -6.5, None)),
                        album: None,
                    },
                    ..TrackTags::default()
                },
                Sources {
                    track_gain: Some(src(0)),
                    ..Sources::default()
                },
            )
        );
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn a_gain_or_peak_that_is_not_a_usable_number_is_dropped_with_the_reason() {
        use ValueField::{Gain, Peak};
        let unusable = |field| ValueError::Unusable { field };
        assert_eq!(
            map(&[
                ("REPLAYGAIN_TRACK_GAIN", "NaN dB"),
                ("REPLAYGAIN_TRACK_GAIN", "inf"),
                ("REPLAYGAIN_TRACK_GAIN", "+1e308 dB"),
                ("REPLAYGAIN_TRACK_GAIN", "200 dB"),
                ("REPLAYGAIN_TRACK_PEAK", "NaN"),
                ("REPLAYGAIN_TRACK_PEAK", "16.5"),
                ("REPLAYGAIN_ALBUM_GAIN", "-128.5 dB"),
                ("REPLAYGAIN_ALBUM_PEAK", "-0.5"),
            ]),
            only_problems(vec![
                invalid(0, malformed(Gain)),
                invalid(1, malformed(Gain)),
                invalid(2, malformed(Gain)),
                invalid(3, unusable(Gain)),
                invalid(4, malformed(Peak)),
                invalid(5, unusable(Peak)),
                invalid(6, unusable(Gain)),
                invalid(7, unusable(Peak)),
            ])
        );
    }

    /// The mapping of one R128 track gain of `db` decibels.
    fn r128_track(db: f32) -> Mapped {
        clean(
            TrackTags {
                gain: crate::catalog::GainTags {
                    track: Some(gain(GainScale::R128, db, None)),
                    album: None,
                },
                ..TrackTags::default()
            },
            Sources {
                track_gain: Some(src(0)),
                ..Sources::default()
            },
        )
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn maps_r128_gains_up_to_the_limits_of_q7_8() {
        let cases = [
            ("0", 0.0),
            ("256", 1.0),
            ("+256", 1.0),
            ("-1", -1.0 / 256.0),
            (" -573 ", -573.0 / 256.0),
            ("32767", 32_767.0 / 256.0),
            ("-32768", -128.0),
        ];
        for (text, db) in cases {
            assert_eq!(map(&[("R128_TRACK_GAIN", text)]), r128_track(db));
        }
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn an_r128_gain_past_the_limits_of_q7_8_is_dropped_with_the_reason() {
        let unusable = ValueError::Unusable {
            field: ValueField::Gain,
        };
        assert_eq!(
            map(&[
                ("R128_TRACK_GAIN", "32768"),
                ("R128_TRACK_GAIN", "-32769"),
                ("R128_TRACK_GAIN", "1.5"),
                ("R128_TRACK_GAIN", "-6.5 dB"),
                ("R128_ALBUM_GAIN", "99999999999999999999"),
                ("R128_ALBUM_GAIN", "loud"),
            ]),
            only_problems(vec![
                invalid(0, unusable),
                invalid(1, unusable),
                invalid(2, malformed(ValueField::Gain)),
                invalid(3, malformed(ValueField::Gain)),
                invalid(4, unusable),
                invalid(5, malformed(ValueField::Gain)),
            ])
        );
    }

    #[test]
    fn an_r128_gain_comes_before_replaygain_and_takes_no_peak() {
        assert_eq!(
            map(&[
                ("REPLAYGAIN_TRACK_GAIN", "-6.5 dB"),
                ("REPLAYGAIN_TRACK_PEAK", "0.5"),
                ("R128_TRACK_GAIN", "512"),
                ("REPLAYGAIN_ALBUM_GAIN", "-7.5 dB"),
                ("REPLAYGAIN_ALBUM_PEAK", "0.25"),
                ("R128_ALBUM_GAIN", "-512"),
            ]),
            clean(
                TrackTags {
                    gain: crate::catalog::GainTags {
                        track: Some(gain(GainScale::R128, 2.0, None)),
                        album: Some(gain(GainScale::R128, -2.0, None)),
                    },
                    ..TrackTags::default()
                },
                Sources {
                    track_gain: Some(src(2)),
                    album_gain: Some(src(5)),
                    ..Sources::default()
                },
            )
        );
    }

    #[test]
    fn an_r128_gain_that_cannot_be_read_leaves_the_gain_to_replaygain() {
        assert_eq!(
            map(&[
                ("R128_TRACK_GAIN", "loud"),
                ("REPLAYGAIN_TRACK_GAIN", "-6.5 dB"),
                ("REPLAYGAIN_TRACK_PEAK", "0.5"),
            ]),
            Mapped {
                tags: TrackTags {
                    gain: crate::catalog::GainTags {
                        track: Some(gain(GainScale::ReplayGain, -6.5, Some(0.5))),
                        album: None,
                    },
                    ..TrackTags::default()
                },
                sources: Sources {
                    track_gain: Some(src(1)),
                    ..Sources::default()
                },
                problems: vec![invalid(0, malformed(ValueField::Gain))],
            }
        );
    }

    /// The mapping of lyrics `text`, timed as `timing`, from comment
    /// `index`.
    fn sung(timing: LyricsTiming, text: &str, index: usize) -> Mapped {
        clean(
            TrackTags {
                lyrics: vec![lyrics(timing, text)],
                ..TrackTags::default()
            },
            Sources {
                lyrics: Some(src(index)),
                ..Sources::default()
            },
        )
    }

    #[test]
    fn maps_lyrics_with_how_they_are_timed() {
        let plain = "So what\n\tso what\u{7}";
        assert_eq!(
            map(&[("LYRICS", plain)]),
            sung(LyricsTiming::Plain, "So what\n\tso what", 0)
        );
        let lines = "[00:01.00]So what\n[00:02.50]So what";
        assert_eq!(
            map(&[("LYRICS", lines)]),
            sung(LyricsTiming::Line, lines, 0)
        );
        let words = "[00:01.00]<00:01.00>So <00:01.40>what";
        assert_eq!(
            map(&[("LYRICS", words)]),
            sung(LyricsTiming::Word, words, 0)
        );
    }

    #[test]
    fn lyrics_are_read_from_unsyncedlyrics_only_without_lyrics() {
        assert_eq!(
            map(&[("UNSYNCEDLYRICS", "So what")]),
            sung(LyricsTiming::Plain, "So what", 0)
        );
        assert_eq!(
            map(&[("UNSYNCEDLYRICS", "Other"), ("LYRICS", "So what")]),
            sung(LyricsTiming::Plain, "So what", 1)
        );
    }

    #[test]
    fn every_lyrics_comment_is_kept_and_one_of_only_controls_is_not() {
        assert_eq!(
            map(&[
                ("LYRICS", "\u{1}\u{2}"),
                ("LYRICS", "English"),
                ("LYRICS", "[00:01.00]Deutsch"),
            ]),
            clean(
                TrackTags {
                    lyrics: vec![
                        lyrics(LyricsTiming::Plain, "English"),
                        lyrics(LyricsTiming::Line, "[00:01.00]Deutsch"),
                    ],
                    ..TrackTags::default()
                },
                Sources {
                    lyrics: Some(src(1)),
                    ..Sources::default()
                },
            )
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn lyrics_longer_than_the_lyrics_limit_are_dropped_with_the_fault() {
        let limits = limits(&[(LimitKind::LyricsBytes, 7)]);
        assert_eq!(
            from_vorbis(&block(&[("LYRICS", "So what")]), &limits),
            sung(LyricsTiming::Plain, "So what", 0)
        );
        assert_eq!(
            from_vorbis(&block(&[("LYRICS", "So what?")]), &limits),
            only_problems(vec![Problem::Lyrics {
                source: src(0),
                fault: ParseFault::LimitExceeded {
                    limit: LimitKind::LyricsBytes,
                    value: 8,
                    max: 7,
                    offset: 0,
                },
            }])
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn a_short_text_is_kept_whole_at_the_limit_and_cut_one_past_it() {
        let limits = limits(&[(LimitKind::ShortText, 4)]);
        let fields = [
            ("TITLE", "abcd"),
            ("ALBUM", "abcde"),
            ("ARTIST", "abcd"),
            ("ARTIST", "abcdé"),
            ("COMPOSER", "abcde"),
            ("PERFORMER", "ab (cd)"),
        ];
        let short = LimitKind::ShortText;
        assert_eq!(
            from_vorbis(&block(&fields), &limits),
            Mapped {
                tags: TrackTags {
                    title: Some(String::from("abcd")),
                    album: Some(String::from("abcd")),
                    artist: strings(&["abcd", "abcd"]),
                    credits: vec![
                        credit("abcd", Role::Composer, None),
                        credit("ab (", Role::Performer, None),
                    ],
                    ..TrackTags::default()
                },
                sources: Sources {
                    title: Some(src(0)),
                    album: Some(src(1)),
                    artist: Some(src(2)),
                    credits: Some(src(4)),
                    ..Sources::default()
                },
                problems: vec![
                    Problem::Truncated {
                        source: src(1),
                        limit: short,
                    },
                    Problem::Truncated {
                        source: src(3),
                        limit: short,
                    },
                    Problem::Truncated {
                        source: src(4),
                        limit: short,
                    },
                    Problem::Truncated {
                        source: src(5),
                        limit: short,
                    },
                ],
            }
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn lyrics_are_kept_whole_at_the_long_text_limit_and_cut_one_past_it() {
        let limits = limits(&[(LimitKind::LongText, 7)]);
        assert_eq!(
            from_vorbis(&block(&[("LYRICS", "So what")]), &limits),
            sung(LyricsTiming::Plain, "So what", 0)
        );
        assert_eq!(
            from_vorbis(&block(&[("LYRICS", "So what?")]), &limits),
            Mapped {
                problems: vec![Problem::Truncated {
                    source: src(0),
                    limit: LimitKind::LongText,
                }],
                ..sung(LyricsTiming::Plain, "So what", 0)
            }
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn a_text_cut_to_nothing_is_dropped_and_recorded() {
        let limits = limits(&[(LimitKind::ShortText, 0)]);
        assert_eq!(
            from_vorbis(&block(&[("TITLE", "So What")]), &limits),
            only_problems(vec![Problem::Truncated {
                source: src(0),
                limit: LimitKind::ShortText,
            }])
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn a_list_holds_the_tag_field_limit_and_drops_one_past_it() {
        let limits = limits(&[(LimitKind::TagFields, 2)]);
        let at_limit = [
            ("GENRE", "Jazz"),
            ("GENRE", "Modal"),
            ("ISRC", "USS1Z9900001"),
            ("ISRC", "USS1Z9900002"),
            ("COMPOSER", "One"),
            ("PERFORMER", "Two"),
            ("LYRICS", "English"),
            ("LYRICS", "Deutsch"),
            ("MUSICBRAINZ_ARTISTID", ARTIST_A),
            ("MUSICBRAINZ_ARTISTID", ARTIST_B),
        ];
        let held = clean(
            TrackTags {
                genres: strings(&["Jazz", "Modal"]),
                isrc: vec![isrc("USS1Z9900001"), isrc("USS1Z9900002")],
                credits: vec![
                    credit("One", Role::Composer, None),
                    credit("Two", Role::Performer, None),
                ],
                lyrics: vec![
                    lyrics(LyricsTiming::Plain, "English"),
                    lyrics(LyricsTiming::Plain, "Deutsch"),
                ],
                musicbrainz: MbIds {
                    artists: vec![mbid(ARTIST_A), mbid(ARTIST_B)],
                    ..MbIds::default()
                },
                ..TrackTags::default()
            },
            Sources {
                genres: Some(src(0)),
                isrc: Some(src(2)),
                credits: Some(src(4)),
                lyrics: Some(src(6)),
                artist_mbids: Some(src(8)),
                ..Sources::default()
            },
        );
        assert_eq!(from_vorbis(&block(&at_limit), &limits), held);
        let mut past = at_limit.to_vec();
        past.extend([
            ("GENRE", "Cool"),
            ("GENRE", "Bop"),
            ("ISRC", "USS1Z9900003"),
            ("CONDUCTOR", "Three"),
            ("LYRICS", "Italiano"),
            ("MUSICBRAINZ_ARTISTID", RECORDING),
        ]);
        assert_eq!(
            from_vorbis(&block(&past), &limits),
            Mapped {
                tags: TrackTags {
                    credits: vec![
                        credit("One", Role::Composer, None),
                        credit("Three", Role::Conductor, None),
                    ],
                    ..held.tags
                },
                sources: held.sources,
                problems: vec![
                    Problem::ListFull { source: src(10) },
                    Problem::ListFull { source: src(11) },
                    Problem::ListFull { source: src(5) },
                    Problem::ListFull { source: src(12) },
                    Problem::ListFull { source: src(15) },
                    Problem::ListFull { source: src(14) },
                ],
            }
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn a_list_holds_the_default_tag_field_limit_and_drops_one_past_it() {
        let fields = (0..4_097).map(|_| ("GENRE", "Jazz")).collect::<Vec<_>>();
        let mapped = map(&fields);
        assert_eq!(
            mapped,
            Mapped {
                tags: TrackTags {
                    genres: (0..4_096).map(|_| String::from("Jazz")).collect(),
                    ..TrackTags::default()
                },
                sources: Sources {
                    genres: Some(src(0)),
                    ..Sources::default()
                },
                problems: vec![Problem::ListFull { source: src(4_096) }],
            }
        );
    }

    /// The comment `problem` names.
    fn problem_source(problem: &Problem) -> Source {
        match *problem {
            Problem::InvalidValue { source, .. }
            | Problem::Unrecognised { source }
            | Problem::Truncated { source, .. }
            | Problem::ListFull { source }
            | Problem::Disagrees { source, .. }
            | Problem::Lyrics { source, .. } => source,
        }
    }

    /// A key the mapper knows or any other, and any text as its value.
    fn any_field() -> impl Strategy<Value = (String, String)> {
        let known = prop::sample::select(vec![
            "TITLE",
            "ARTIST",
            "ARTISTS",
            "GENRE",
            "PERFORMER",
            "COMPOSER",
            "TRACKNUMBER",
            "TRACKTOTAL",
            "DISCNUMBER",
            "DATE",
            "COMPILATION",
            "RELEASETYPE",
            "ISRC",
            "MUSICBRAINZ_ARTISTID",
            "REPLAYGAIN_TRACK_GAIN",
            "REPLAYGAIN_TRACK_PEAK",
            "R128_TRACK_GAIN",
            "LYRICS",
        ])
        .prop_map(str::to_owned);
        let value = prop_oneof![
            any::<String>(),
            "[0-9/ ]{0,6}",
            "\\[0[0-9]:0[0-9]\\][a-z\n]{0,6}",
            "[a-z ;,]{0,16}",
            Just(String::from(ARTIST_A)),
            Just(String::from("USS1Z9900001")),
        ];
        (prop_oneof![known, "[ -~]{0,8}"], value)
    }

    proptest! {
        /// Whatever the fields hold, no list is longer than the tag-field
        /// limit, no text is longer than its limit, every source names a
        /// comment of the block, and no comment gives more than two
        /// problems.
        #[test]
        fn any_fields_map_within_the_limits(fields in vec(any_field(), 0..12)) {
            let limits = limits(&[
                (LimitKind::TagFields, 2),
                (LimitKind::ShortText, 6),
                (LimitKind::LongText, 12),
            ]);
            let borrowed: Vec<(&str, &str)> = fields
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str()))
                .collect();
            let mapped = from_vorbis(&block(&borrowed), &limits);
            let tags = &mapped.tags;
            let lists = [
                tags.artist.len(),
                tags.genres.len(),
                tags.credits.len(),
                tags.isrc.len(),
                tags.musicbrainz.artists.len(),
                tags.lyrics.len(),
            ];
            prop_assert!(lists.iter().all(|len| *len <= 2));
            let short = tags
                .title
                .iter()
                .chain(&tags.artist)
                .chain(&tags.genres)
                .map(String::len)
                .chain(tags.credits.iter().map(|credit| credit.name().len()));
            prop_assert!(short.into_iter().all(|len| len <= 6));
            prop_assert!(tags.lyrics.iter().all(|lyrics| lyrics.text.len() <= 12));
            let sources = &mapped.sources;
            let named = [
                sources.title,
                sources.artist,
                sources.genres,
                sources.credits,
                sources.track,
                sources.track_total,
                sources.disc,
                sources.date,
                sources.compilation,
                sources.release_type,
                sources.isrc,
                sources.artist_mbids,
                sources.track_gain,
                sources.lyrics,
            ];
            prop_assert!(named.iter().flatten().all(|source| source.index < fields.len()));
            let problems = mapped.problems.iter().map(problem_source);
            prop_assert!(problems.clone().all(|source| source.index < fields.len()));
            for index in 0..fields.len() {
                let given = problems.clone().filter(|source| *source == src(index)).count();
                prop_assert!(given <= 2, "comment {} gave {} problems", index, given);
            }
        }
    }
}
