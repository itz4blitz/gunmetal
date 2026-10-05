//! MP4 item lists mapped onto [`TrackTags`], and the field rules the MP4,
//! APE ([`super::ape`]) and RIFF ([`super::riff`]) mappers share.
//!
//! # What a mapper returns
//!
//! A [`Mapped`] holds the canonical tags, every source that filled a field
//! or added to a list (API-CAT-08), and every value that was dropped or
//! cut, with the reason. Mapping never fails: a value that cannot be used
//! is left out and the rest of the tag is kept.
//!
//! # The field rules
//!
//! Each mapper turns its own keys into a [`TagField`] and hands the value
//! over as text, so one set of rules serves the three formats:
//!
//! - Text has its control characters removed and is cut to the short-text
//!   limit, or the long-text limit for lyrics, after decoding
//!   (SEC-MED-013, SEC-MED-006). A value that is blank afterwards is left
//!   out.
//! - A field that holds one value keeps the first one tagged. A field that
//!   holds a list keeps every value in the order tagged, up to the
//!   tag-field limit (SEC-MED-006).
//! - Track and disc numbers and totals are whole numbers from 1 to 9,999,
//!   written alone or as `3/12`. A zero means "not given". Dates,
//!   `MusicBrainz` identifiers, recording codes and `ReplayGain` gains and
//!   peaks are read by the typed values of [`crate::values`]; a value
//!   outside its range is dropped with the reason (SEC-MED-014).
//! - Artist strings are kept as tagged. Splitting them is the credit
//!   splitter's job (WP-053).
//!
//! # Text that was cut
//!
//! Every cut is one [`Reason::Truncated`] problem that names the limit the
//! text was cut to, against the field and the source of the value
//! (SEC-MED-006). A value can be cut in two places:
//!
//! - The MP4 and APE parsers cap the text they decode and flag what they
//!   cut ([`Text::truncated`](crate::text::Text::truncated)). The mapper
//!   records that cut first, with the parser's limit: the short-text limit
//!   for an MP4 atom, and the long-text limit for `©lyr`, for a freeform
//!   MP4 item and for every APE value.
//! - The field rules then cut what is left to the field's own limit, and
//!   record that cut.
//!
//! So a value cut once has one problem, whoever cut it. Under one set of
//! limits no cut is recorded twice: text the parser cut to a limit is no
//! longer than that limit, so the field rules cannot cut it to the same
//! limit again. A value cut twice has two problems, the parser's first: a
//! freeform MP4 item or an APE value that is not lyrics, cut to the
//! long-text limit by the parser and, when the short-text limit is the
//! smaller, cut again to it by the field rules.
//!
//! A cut is recorded whatever becomes of what was left. Text that is then
//! blank is left out, and text that is malformed, out of range or past a
//! list's limit is dropped with its reason, as a second problem after the
//! cut. Text the parser cut in an item that holds a number, such as
//! `trkn`, records the cut and then [`Reason::Unreadable`].
//!
//! The `INFO` mapper reads its values from the octets of the list itself,
//! so no value reaches it already cut.
//!
//! # MP4
//!
//! [`from_ilst`] maps the items the MP4 parser read from `ilst`
//! ([`crate::formats::mp4`]). Text atoms (`©nam`, `©ART`, `aART`, `©alb`,
//! `©day`, `©gen`, `©wrt`, `©grp`, `©lyr` and the sort atoms `sonm`,
//! `soar`, `soaa` and `soal`) give one value for each `data` box. `trkn`
//! and `disk` hold the number and the total as 16-bit integers after two
//! reserved octets. `gnre` holds an `ID3v1` genre number plus one. `cpil`
//! is 1 for a compilation, and `rtng` is 0 for no advisory, 1 or 4 for
//! explicit and 2 for clean. Freeform items are matched by name without
//! regard to case, whatever their namespace: the `MusicBrainz` and
//! `ReplayGain` items Picard writes, `iTunSMPB`, and a few names taggers
//! share. `iTunSMPB` is a run of hexadecimal numbers separated by spaces;
//! the second is the encoder delay and the third the padding, in samples
//! (MUS-069).
//!
//! # Work
//!
//! [`from_ilst`] charges the [`Budget`] one step for each item and one for
//! each of its values, before it reads them (SEC-MED-007). Every item and
//! every value is a box of at least eight octets in the file, so a list of
//! `n` octets costs at most `n / 8` steps. When the budget is spent the
//! mapping stops and says where.

use crate::catalog::{
    Advisory, Credit, Gain, GainScale, GainTags, LyricsOrigin, LyricsSource, LyricsTiming,
    PrimaryType, ReleaseType, Role, SecondaryType, TagLyrics, TrackPosition, TrackTags, Trim,
};
use crate::formats::mp4::{FourCc, IlstItem, ItemKey, ItemValue};
use crate::parse::{Budget, Cursor, LimitKind, Limits};
use crate::text::{self, Lines};
use crate::untrusted::Untrusted;
use crate::values::{Field, GainDb, Isrc, Mbid, NumberOf, PartialDate, PeakRatio, ValueError};

/// The tags of one file as one tag format gave them. `S` says where in the
/// tag a value was read.
#[derive(Debug, Clone, PartialEq)]
pub struct Mapped<S> {
    /// The canonical tags.
    pub tags: TrackTags,
    /// Every source that filled a field or added a value to a list, in the
    /// order read. A source that added several values to one list is named
    /// once.
    pub sources: Vec<(TagField, S)>,
    /// The values that were dropped or cut, in the order found.
    pub problems: Vec<TagProblem<S>>,
}

/// A field of [`TrackTags`] that a tag can fill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagField {
    /// The title.
    Title,
    /// The title to sort by.
    TitleSort,
    /// The recording's artists.
    Artist,
    /// The recording's artists to sort by.
    ArtistSort,
    /// The release's artists.
    AlbumArtist,
    /// The release's artists to sort by.
    AlbumArtistSort,
    /// The release's title.
    Album,
    /// The release's title to sort by.
    AlbumSort,
    /// The track number, alone or with the number of tracks as `3/12`.
    Track,
    /// The number of tracks.
    TrackTotal,
    /// The disc number, alone or with the number of discs as `1/2`.
    Disc,
    /// The number of discs.
    DiscTotal,
    /// The disc's own title.
    DiscSubtitle,
    /// The release date.
    Date,
    /// The original release date.
    OriginalDate,
    /// The genres.
    Genres,
    /// The moods.
    Moods,
    /// The record labels.
    Labels,
    /// The groupings.
    Grouping,
    /// A credited name with this role. A performer written as
    /// `Name (instrument)` has the instrument as the credit's detail.
    Credit(Role),
    /// Whether the release is a compilation: `1` or `0`.
    Compilation,
    /// The release's type: one `MusicBrainz` primary or secondary type.
    ReleaseType,
    /// The content advisory: `1` or `4` for explicit, `2` for clean and
    /// `0` for none.
    Advisory,
    /// The recording codes.
    Isrc,
    /// The recording's `MusicBrainz` identifier.
    RecordingMbid,
    /// The `MusicBrainz` identifier of the track on the release.
    TrackMbid,
    /// The release's `MusicBrainz` identifier.
    ReleaseMbid,
    /// The release group's `MusicBrainz` identifier.
    ReleaseGroupMbid,
    /// The `MusicBrainz` identifiers of the recording's artists.
    ArtistMbids,
    /// The `MusicBrainz` identifiers of the release's artists.
    AlbumArtistMbids,
    /// The track's `ReplayGain` gain.
    TrackGain,
    /// The track's `ReplayGain` peak. It is kept only with a track gain.
    TrackPeak,
    /// The album's `ReplayGain` gain.
    AlbumGain,
    /// The album's `ReplayGain` peak. It is kept only with an album gain.
    AlbumPeak,
    /// The encoder delay and padding, written as `iTunSMPB` writes them.
    Trim,
    /// Untimed lyrics found at this origin.
    Lyrics(LyricsOrigin),
}

/// Something a mapper could not keep as the tag gave it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagProblem<S> {
    /// One value was dropped or cut.
    Value {
        /// The field the value was for.
        field: TagField,
        /// Where the value was read.
        source: S,
        /// What was wrong with it.
        reason: Reason,
    },
    /// The step budget was spent: this source and every later one were not
    /// mapped (SEC-MED-007).
    BudgetSpent {
        /// The first source that was not mapped.
        source: S,
    },
}

/// Why a value was dropped or cut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// A typed value refused it, and it was dropped (SEC-MED-014).
    Invalid(ValueError),
    /// It is not written the way its field is written, and it was dropped.
    Unreadable,
    /// A count or size limit was reached, and it was dropped
    /// (SEC-MED-006).
    Limit(LimitKind),
    /// It was longer than this text limit and was cut to it, by the parser
    /// that read the tag or by the field rules (SEC-MED-006). What was left
    /// was kept, unless it was blank or a later problem for the same value
    /// says why it was dropped.
    Truncated(LimitKind),
    /// It means something only beside a value the tag did not give, such
    /// as a `ReplayGain` peak without its gain, and it was dropped.
    Unpaired,
}

impl From<ValueError> for Reason {
    fn from(error: ValueError) -> Self {
        Self::Invalid(error)
    }
}

/// The place of an item in its tag's list of items, counted from 0: an
/// entry of the MP4 parser's item list, or of [`ApeTag::items`].
///
/// [`ApeTag::items`]: crate::formats::ape::ApeTag::items
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemIndex(pub usize);

/// The tags being filled, shared by the mappers of this directory.
pub(super) struct Fields<'a, S> {
    /// The tags filled so far, without the position and the gains.
    tags: TrackTags,
    /// The sources so far.
    sources: Vec<(TagField, S)>,
    /// The problems so far.
    problems: Vec<TagProblem<S>>,
    /// The limits of this parse.
    limits: &'a Limits,
    /// The track number.
    track: Option<u16>,
    /// The number of tracks.
    track_total: Option<u16>,
    /// The disc number.
    disc: Option<u16>,
    /// The number of discs.
    disc_total: Option<u16>,
    /// The track gain.
    track_gain: Option<GainDb>,
    /// The track peak.
    track_peak: Option<PeakRatio>,
    /// The album gain.
    album_gain: Option<GainDb>,
    /// The album peak.
    album_peak: Option<PeakRatio>,
}

impl<'a, S: Copy + PartialEq> Fields<'a, S> {
    /// No tags yet, to be filled under `limits`.
    pub(super) fn new(limits: &'a Limits) -> Self {
        Self {
            tags: TrackTags::default(),
            sources: Vec::new(),
            problems: Vec::new(),
            limits,
            track: None,
            track_total: None,
            disc: None,
            disc_total: None,
            track_gain: None,
            track_peak: None,
            album_gain: None,
            album_peak: None,
        }
    }

    /// Charges `budget` one step for the item at `source` and one for each
    /// of its `values`. Returns whether the item may be mapped; when the
    /// budget is spent, the problem is recorded.
    pub(super) fn charge(&mut self, budget: &mut Budget, values: usize, source: S) -> bool {
        let steps = u64::try_from(values).unwrap_or(u64::MAX).saturating_add(1);
        let spent = budget.charge(steps, 0).is_err();
        if spent {
            self.problems.push(TagProblem::BudgetSpent { source });
        }
        !spent
    }

    /// Records that the value for `field` at `source` was dropped or cut.
    pub(super) fn note(&mut self, field: TagField, source: S, reason: Reason) {
        self.problems.push(TagProblem::Value {
            field,
            source,
            reason,
        });
    }

    /// Gives `field` the value `raw`, read at `source`.
    pub(super) fn set(&mut self, field: TagField, raw: &str, source: S) {
        let (lines, limit) = match field {
            TagField::Lyrics(_) => (Lines::Multi, LimitKind::LongText),
            _ => (Lines::Single, LimitKind::ShortText),
        };
        let cap = u32::try_from(self.limits.get(limit)).unwrap_or(u32::MAX);
        let text = text::normalise(Untrusted::new(raw.as_bytes()), lines, cap);
        if text.truncated {
            self.note(field, source, Reason::Truncated(limit));
        }
        if text.value.trim().is_empty() {
            return;
        }
        match (field, text.value.split_once('/')) {
            (TagField::Track, Some((number, total))) => {
                self.keep(TagField::Track, number, source);
                self.keep(TagField::TrackTotal, total, source);
            }
            (TagField::Disc, Some((number, total))) => {
                self.keep(TagField::Disc, number, source);
                self.keep(TagField::DiscTotal, total, source);
            }
            _ => self.keep(field, &text.value, source),
        }
    }

    /// Reads the cleaned `value` as `field` is written and stores it.
    fn keep(&mut self, field: TagField, value: &str, source: S) {
        let limits = self.limits;
        let t = &mut self.tags;
        let ids = &mut t.musicbrainz;
        let kept = match field {
            TagField::Title => Ok(first(&mut t.title, value.to_owned())),
            TagField::TitleSort => Ok(first(&mut t.title_sort, value.to_owned())),
            TagField::Artist => push(&mut t.artist, value.to_owned(), limits),
            TagField::ArtistSort => push(&mut t.artist_sort, value.to_owned(), limits),
            TagField::AlbumArtist => push(&mut t.album_artist, value.to_owned(), limits),
            TagField::AlbumArtistSort => push(&mut t.album_artist_sort, value.to_owned(), limits),
            TagField::Album => Ok(first(&mut t.album, value.to_owned())),
            TagField::AlbumSort => Ok(first(&mut t.album_sort, value.to_owned())),
            TagField::Track => count(value, Field::Number).map(|n| fill(&mut self.track, n)),
            TagField::TrackTotal => {
                count(value, Field::Total).map(|n| fill(&mut self.track_total, n))
            }
            TagField::Disc => count(value, Field::Number).map(|n| fill(&mut self.disc, n)),
            TagField::DiscTotal => {
                count(value, Field::Total).map(|n| fill(&mut self.disc_total, n))
            }
            TagField::DiscSubtitle => Ok(first(&mut t.disc_subtitle, value.to_owned())),
            TagField::Date => date(value).map(|date| first(&mut t.date, date)),
            TagField::OriginalDate => date(value).map(|date| first(&mut t.original_date, date)),
            TagField::Genres => push(&mut t.genres, value.to_owned(), limits),
            TagField::Moods => push(&mut t.moods, value.to_owned(), limits),
            TagField::Labels => push(&mut t.labels, value.to_owned(), limits),
            TagField::Grouping => push(&mut t.grouping, value.to_owned(), limits),
            TagField::Credit(role) => {
                credit(role, value).and_then(|credit| push(&mut t.credits, credit, limits))
            }
            TagField::Compilation => flag(value).map(|flag| first(&mut t.compilation, flag)),
            TagField::ReleaseType => release_type(&mut t.release_type, value, limits),
            TagField::Advisory => rating(value).map(|rating| fill(&mut t.advisory, rating)),
            TagField::Isrc => isrc(value).and_then(|code| push(&mut t.isrc, code, limits)),
            TagField::RecordingMbid => mbid(value).map(|id| first(&mut ids.recording, id)),
            TagField::TrackMbid => mbid(value).map(|id| first(&mut ids.track, id)),
            TagField::ReleaseMbid => mbid(value).map(|id| first(&mut ids.release, id)),
            TagField::ReleaseGroupMbid => mbid(value).map(|id| first(&mut ids.release_group, id)),
            TagField::ArtistMbids => mbid(value).and_then(|id| push(&mut ids.artists, id, limits)),
            TagField::AlbumArtistMbids => {
                mbid(value).and_then(|id| push(&mut ids.album_artists, id, limits))
            }
            TagField::TrackGain => gain_db(value).map(|db| first(&mut self.track_gain, db)),
            TagField::TrackPeak => peak(value).map(|ratio| first(&mut self.track_peak, ratio)),
            TagField::AlbumGain => gain_db(value).map(|db| first(&mut self.album_gain, db)),
            TagField::AlbumPeak => peak(value).map(|ratio| first(&mut self.album_peak, ratio)),
            TagField::Trim => smpb(value).map(|trim| first(&mut t.trim, trim)),
            TagField::Lyrics(origin) => {
                lyrics(origin, value).and_then(|lyrics| push(&mut t.lyrics, lyrics, limits))
            }
        };
        match kept {
            Ok(true) => {
                // The values of one source are set one after another, and
                // only a split track or disc number fills two fields from
                // one value, both of which hold one value. So a source
                // already named for this field is the last one named.
                if self.sources.last() != Some(&(field, source)) {
                    self.sources.push((field, source));
                }
            }
            Ok(false) => {}
            Err(reason) => self.note(field, source, reason),
        }
    }

    /// The tags as filled.
    pub(super) fn finish(mut self) -> Mapped<S> {
        // Every part is from 1 to 9,999 already, which is all the
        // constructor asks.
        self.tags.position =
            TrackPosition::new(self.track, self.track_total, self.disc, self.disc_total)
                .unwrap_or_default();
        self.tags.gain = GainTags {
            track: gain(self.track_gain, self.track_peak),
            album: gain(self.album_gain, self.album_peak),
        };
        self.unless_paired(TagField::TrackPeak, self.track_gain.is_some());
        self.unless_paired(TagField::AlbumPeak, self.album_gain.is_some());
        Mapped {
            tags: self.tags,
            sources: self.sources,
            problems: self.problems,
        }
    }

    /// Unless `paired`, drops the source of the value `field` holds, which
    /// means nothing alone, and records why.
    fn unless_paired(&mut self, field: TagField, paired: bool) {
        if paired {
            return;
        }
        if let Some(at) = self.sources.iter().position(|(by, _)| *by == field) {
            let (_, source) = self.sources.remove(at);
            self.note(field, source, Reason::Unpaired);
        }
    }
}

/// Fills `slot` with `value` unless it is filled already. Returns whether
/// it was filled now.
fn first<T>(slot: &mut Option<T>, value: T) -> bool {
    let empty = slot.is_none();
    if empty {
        *slot = Some(value);
    }
    empty
}

/// Fills `slot` with `value`, when there is one, unless it is filled
/// already. Returns whether it was filled now.
fn fill<T>(slot: &mut Option<T>, value: Option<T>) -> bool {
    value.is_some_and(|value| first(slot, value))
}

/// Adds `value` to `list`, unless the list already holds as many values as
/// the tag-field limit allows.
fn push<T>(list: &mut Vec<T>, value: T, limits: &Limits) -> Result<bool, Reason> {
    let count = u64::try_from(list.len())
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    if count > limits.get(LimitKind::TagFields) {
        return Err(Reason::Limit(LimitKind::TagFields));
    }
    list.push(value);
    Ok(true)
}

/// The value in `table` whose name is `key`, without regard to ASCII case.
pub(super) fn lookup<T: Copy>(table: &[(&str, T)], key: &str) -> Option<T> {
    table
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(key))
        .map(|(_, value)| *value)
}

/// A track or disc number or total: decimal digits, from 1 to 9,999, or
/// `None` for zero, which taggers write for "not given".
fn count(text: &str, field: Field) -> Result<Option<u16>, Reason> {
    let text = text.trim();
    if text.is_empty() || !text.bytes().all(|octet| octet.is_ascii_digit()) {
        return Err(Reason::Invalid(ValueError::Malformed { field }));
    }
    // Only a number too long for 64 bits fails to parse.
    let value = text.parse::<u64>().unwrap_or(u64::MAX);
    match u16::try_from(value) {
        Ok(0) => Ok(None),
        Ok(number) if number <= NumberOf::MAX => Ok(Some(number)),
        _ => Err(Reason::Invalid(ValueError::OutOfRange { field, value })),
    }
}

/// A date.
fn date(text: &str) -> Result<PartialDate, Reason> {
    Ok(PartialDate::parse(Untrusted::new(text))?)
}

/// A `MusicBrainz` identifier.
fn mbid(text: &str) -> Result<Mbid, Reason> {
    Ok(Mbid::parse(Untrusted::new(text))?)
}

/// A recording code.
fn isrc(text: &str) -> Result<Isrc, Reason> {
    Ok(Isrc::parse(Untrusted::new(text))?)
}

/// A `ReplayGain` gain.
fn gain_db(text: &str) -> Result<GainDb, Reason> {
    Ok(GainDb::parse(Untrusted::new(text))?)
}

/// A `ReplayGain` peak.
fn peak(text: &str) -> Result<PeakRatio, Reason> {
    Ok(PeakRatio::parse(Untrusted::new(text))?)
}

/// A gain with its peak, when the gain was tagged.
fn gain(gain: Option<GainDb>, peak: Option<PeakRatio>) -> Option<Gain> {
    gain.map(|gain| Gain {
        scale: GainScale::ReplayGain,
        gain,
        peak,
    })
}

/// A compilation flag.
fn flag(text: &str) -> Result<bool, Reason> {
    match text.trim() {
        "1" => Ok(true),
        "0" => Ok(false),
        _ => Err(Reason::Unreadable),
    }
}

/// A content advisory, as MP4's `rtng` numbers it.
fn rating(text: &str) -> Result<Option<Advisory>, Reason> {
    match text.trim() {
        "0" => Ok(None),
        "1" | "4" => Ok(Some(Advisory::Explicit)),
        "2" => Ok(Some(Advisory::Clean)),
        _ => Err(Reason::Unreadable),
    }
}

/// The `MusicBrainz` primary release types, as taggers write them.
const PRIMARY_TYPES: &[(&str, PrimaryType)] = &[
    ("album", PrimaryType::Album),
    ("single", PrimaryType::Single),
    ("ep", PrimaryType::Ep),
    ("broadcast", PrimaryType::Broadcast),
    ("other", PrimaryType::Other),
];

/// The `MusicBrainz` secondary release types, as taggers write them.
const SECONDARY_TYPES: &[(&str, SecondaryType)] = &[
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

/// Adds one release type to `slot`: the first primary type tagged, and
/// every secondary type.
fn release_type(
    slot: &mut Option<ReleaseType>,
    text: &str,
    limits: &Limits,
) -> Result<bool, Reason> {
    let token = text.trim();
    if let Some(primary) = lookup(PRIMARY_TYPES, token) {
        Ok(first(&mut slot.get_or_insert_default().primary, primary))
    } else if let Some(secondary) = lookup(SECONDARY_TYPES, token) {
        push(
            &mut slot.get_or_insert_default().secondary,
            secondary,
            limits,
        )
    } else {
        Err(Reason::Unreadable)
    }
}

/// The delay and padding of an `iTunSMPB` value: its second and third
/// hexadecimal numbers.
fn smpb(text: &str) -> Result<Trim, Reason> {
    let mut numbers = text.split_ascii_whitespace().skip(1).map(|number| {
        Some(number)
            .filter(|number| number.bytes().all(|octet| octet.is_ascii_hexdigit()))
            .and_then(|number| u32::from_str_radix(number, 16).ok())
    });
    match (numbers.next(), numbers.next()) {
        (Some(Some(delay)), Some(Some(padding))) => Ok(Trim { delay, padding }),
        _ => Err(Reason::Unreadable),
    }
}

/// A credit of `role`. A performer written as `Name (instrument)` gets the
/// instrument as the detail.
fn credit(role: Role, text: &str) -> Result<Credit, Reason> {
    let bracketed = text
        .strip_suffix(')')
        .and_then(|rest| rest.rsplit_once(" ("));
    let (name, detail) = match (role, bracketed) {
        (Role::Performer, Some((name, detail))) => (name, Some(detail.to_owned())),
        _ => (text, None),
    };
    Credit::new(name.to_owned(), role, detail, None)
        .ok()
        .ok_or(Reason::Unreadable)
}

/// Untimed lyrics found at `origin`.
fn lyrics(origin: LyricsOrigin, text: &str) -> Result<TagLyrics, Reason> {
    LyricsSource::new(origin, LyricsTiming::Plain)
        .ok()
        .map(|source| TagLyrics {
            source,
            text: text.to_owned(),
        })
        .ok_or(Reason::Unreadable)
}

/// How an MP4 item holds its value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Atom {
    /// Text.
    Text(TagField),
    /// A number and its total, as `trkn` and `disk` hold them.
    Pair(TagField),
    /// A whole number.
    Number(TagField),
    /// An `ID3v1` genre number plus one, as `gnre` holds it.
    Genre,
}

/// The box type of the lyrics item, `©lyr`.
const LYRICS: [u8; 4] = *b"\xA9lyr";

/// The items named by their box type.
const ATOMS: &[([u8; 4], Atom)] = &[
    (*b"\xA9nam", Atom::Text(TagField::Title)),
    (*b"\xA9ART", Atom::Text(TagField::Artist)),
    (*b"aART", Atom::Text(TagField::AlbumArtist)),
    (*b"\xA9alb", Atom::Text(TagField::Album)),
    (*b"\xA9day", Atom::Text(TagField::Date)),
    (*b"\xA9gen", Atom::Text(TagField::Genres)),
    (*b"\xA9wrt", Atom::Text(TagField::Credit(Role::Composer))),
    (*b"\xA9grp", Atom::Text(TagField::Grouping)),
    (LYRICS, Atom::Text(TagField::Lyrics(LyricsOrigin::Mp4Item))),
    (*b"sonm", Atom::Text(TagField::TitleSort)),
    (*b"soar", Atom::Text(TagField::ArtistSort)),
    (*b"soaa", Atom::Text(TagField::AlbumArtistSort)),
    (*b"soal", Atom::Text(TagField::AlbumSort)),
    (*b"trkn", Atom::Pair(TagField::Track)),
    (*b"disk", Atom::Pair(TagField::Disc)),
    (*b"gnre", Atom::Genre),
    (*b"cpil", Atom::Number(TagField::Compilation)),
    (*b"rtng", Atom::Number(TagField::Advisory)),
];

/// The freeform items, by name.
const FREEFORM: &[(&str, TagField)] = &[
    ("MusicBrainz Track Id", TagField::RecordingMbid),
    ("MusicBrainz Release Track Id", TagField::TrackMbid),
    ("MusicBrainz Album Id", TagField::ReleaseMbid),
    ("MusicBrainz Release Group Id", TagField::ReleaseGroupMbid),
    ("MusicBrainz Artist Id", TagField::ArtistMbids),
    ("MusicBrainz Album Artist Id", TagField::AlbumArtistMbids),
    ("MusicBrainz Album Type", TagField::ReleaseType),
    ("replaygain_track_gain", TagField::TrackGain),
    ("replaygain_track_peak", TagField::TrackPeak),
    ("replaygain_album_gain", TagField::AlbumGain),
    ("replaygain_album_peak", TagField::AlbumPeak),
    ("iTunSMPB", TagField::Trim),
    ("ISRC", TagField::Isrc),
    ("LABEL", TagField::Labels),
    ("MOOD", TagField::Moods),
    ("DISCSUBTITLE", TagField::DiscSubtitle),
    ("originaldate", TagField::OriginalDate),
    ("ITUNESADVISORY", TagField::Advisory),
    ("CONDUCTOR", TagField::Credit(Role::Conductor)),
    ("LYRICIST", TagField::Credit(Role::Lyricist)),
    ("REMIXER", TagField::Credit(Role::Remixer)),
    ("PRODUCER", TagField::Credit(Role::Producer)),
    ("ARRANGER", TagField::Credit(Role::Arranger)),
    ("ENGINEER", TagField::Credit(Role::Engineer)),
    ("MIXER", TagField::Credit(Role::Mixer)),
    ("DJMIXER", TagField::Credit(Role::DjMixer)),
];

/// The Winamp genre list that `ID3v1` genre numbers index: 0 to 191.
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

/// Maps the items of an MP4 item list onto [`TrackTags`].
///
/// The budget is charged one step for each item and one for each of its
/// values.
#[must_use]
pub fn from_ilst(items: &[IlstItem], limits: &Limits, budget: &mut Budget) -> Mapped<ItemIndex> {
    let mut fields = Fields::new(limits);
    for (index, item) in items.iter().enumerate() {
        let source = ItemIndex(index);
        if !fields.charge(budget, item.values.len(), source) {
            break;
        }
        if let Some(atom) = atom(&item.key) {
            let cut_to = parser_limit(&item.key);
            for value in &item.values {
                let (field, text) = reading(atom, value);
                if cut_by_parser(value) {
                    fields.note(field, source, Reason::Truncated(cut_to));
                }
                match text {
                    Some(text) => fields.set(field, &text, source),
                    None => fields.note(field, source, Reason::Unreadable),
                }
            }
        }
    }
    fields.finish()
}

/// How the item named `key` holds its value, or `None` for an item that is
/// not mapped.
fn atom(key: &ItemKey) -> Option<Atom> {
    match key {
        ItemKey::Atom(kind) => ATOMS
            .iter()
            .find(|(name, _)| *name == kind.0)
            .map(|(_, atom)| *atom),
        ItemKey::Freeform { name, .. } => lookup(FREEFORM, &name.value).map(Atom::Text),
    }
}

/// The limit the MP4 parser cut the text of the item named `key` to: the
/// long-text limit for lyrics and for a freeform item, and the short-text
/// limit for every other item that is mapped.
fn parser_limit(key: &ItemKey) -> LimitKind {
    match key {
        ItemKey::Atom(FourCc(LYRICS)) | ItemKey::Freeform { .. } => LimitKind::LongText,
        ItemKey::Atom(_) => LimitKind::ShortText,
    }
}

/// Whether `value` is text the parser cut to its limit.
fn cut_by_parser(value: &ItemValue) -> bool {
    matches!(value, ItemValue::Text(text) if text.truncated)
}

/// The field `atom` fills, and `value` as that field's text, or `None`
/// when the value is not of the kind the item holds.
fn reading(atom: Atom, value: &ItemValue) -> (TagField, Option<String>) {
    match atom {
        Atom::Text(field) => (field, words(value)),
        Atom::Pair(field) => (field, pair(value)),
        Atom::Number(field) => (field, integer(value).map(|number| number.to_string())),
        Atom::Genre => (TagField::Genres, integer(value).and_then(genre)),
    }
}

/// The text of a text value.
fn words(value: &ItemValue) -> Option<String> {
    match value {
        ItemValue::Text(text) => Some(text.value.clone()),
        ItemValue::Signed(_)
        | ItemValue::Unsigned(_)
        | ItemValue::Picture(_)
        | ItemValue::Binary { .. } => None,
    }
}

/// The number and total of a `trkn` or `disk` value, as `3/12`: two
/// reserved octets, then two big-endian 16-bit integers.
fn pair(value: &ItemValue) -> Option<String> {
    let ItemValue::Binary { bytes, .. } = value else {
        return None;
    };
    let mut cursor = Cursor::new(bytes);
    cursor.skip(2).ok()?;
    let number = cursor.u16_be().ok()?;
    let total = cursor.u16_be().ok()?;
    Some(format!("{number}/{total}"))
}

/// A whole number that is not negative, however the writer typed it: a
/// binary value is read as a big-endian integer that saturates.
fn integer(value: &ItemValue) -> Option<u64> {
    match value {
        ItemValue::Signed(number) => u64::try_from(*number).ok(),
        ItemValue::Unsigned(number) => Some(*number),
        ItemValue::Binary { bytes, .. } => Some(bytes.iter().fold(0_u64, |number, octet| {
            number.saturating_mul(256).saturating_add(u64::from(*octet))
        })),
        ItemValue::Text(_) | ItemValue::Picture(_) => None,
    }
}

/// The name of the genre a `gnre` value names: an `ID3v1` genre number
/// plus one.
fn genre(number: u64) -> Option<String> {
    number
        .checked_sub(1)
        .and_then(|index| usize::try_from(index).ok())
        .and_then(|index| GENRES.get(index))
        .map(|name| (*name).to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::MbIds;
    use crate::formats::mp4::{PictureRef, Probe};
    use crate::parse::drive;
    use crate::text::Text;
    use gunmetal_testkit::mp4 as kit;
    use proptest::collection::vec;
    use proptest::prelude::*;
    use proptest::sample::select;

    fn text(value: &str) -> ItemValue {
        ItemValue::Text(Text {
            value: value.to_owned(),
            truncated: false,
            replaced: false,
        })
    }

    fn binary(bytes: &[u8]) -> ItemValue {
        ItemValue::Binary {
            type_code: 0,
            bytes: bytes.to_vec(),
        }
    }

    fn item(kind: [u8; 4], values: Vec<ItemValue>) -> IlstItem {
        IlstItem {
            key: ItemKey::Atom(FourCc(kind)),
            values,
        }
    }

    fn words(kind: [u8; 4], values: &[&str]) -> IlstItem {
        item(kind, values.iter().map(|value| text(value)).collect())
    }

    fn name(value: &str) -> Text {
        Text {
            value: value.to_owned(),
            truncated: false,
            replaced: false,
        }
    }

    fn free(key: &str, values: &[&str]) -> IlstItem {
        IlstItem {
            key: ItemKey::Freeform {
                mean: Some(name("com.apple.iTunes")),
                name: name(key),
            },
            values: values.iter().map(|value| text(value)).collect(),
        }
    }

    fn map(items: &[IlstItem]) -> Mapped<ItemIndex> {
        from_ilst(items, &Limits::DEFAULT, &mut Budget::for_input(0, 0, 1_000))
    }

    fn lowered(lowered: &[(LimitKind, u64)]) -> Limits {
        lowered
            .iter()
            .fold(Limits::DEFAULT, |limits, &(kind, value)| {
                limits
                    .with_override(kind, value)
                    .expect("the test lowers a limit")
            })
    }

    /// Sets each value in turn; the source of a value is its place in
    /// `values`.
    fn set_under(limits: &Limits, values: &[(TagField, &str)]) -> Mapped<u8> {
        let mut fields = Fields::new(limits);
        for (source, (field, value)) in (0_u8..).zip(values) {
            fields.set(*field, value, source);
        }
        fields.finish()
    }

    fn set(values: &[(TagField, &str)]) -> Mapped<u8> {
        set_under(&Limits::DEFAULT, values)
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

    fn position(
        track: Option<u16>,
        track_total: Option<u16>,
        disc: Option<u16>,
        disc_total: Option<u16>,
    ) -> TrackPosition {
        TrackPosition::new(track, track_total, disc, disc_total)
            .expect("the test's position is in range")
    }

    fn replay_gain(db: f32, peak: Option<f32>) -> Gain {
        Gain {
            scale: GainScale::ReplayGain,
            gain: GainDb::new(db).expect("the test's gain is in range"),
            peak: peak.map(|peak| PeakRatio::new(peak).expect("the test's peak is in range")),
        }
    }

    fn plain_lyrics(origin: LyricsOrigin, text: &str) -> TagLyrics {
        TagLyrics {
            source: LyricsSource::new(origin, LyricsTiming::Plain)
                .expect("the test's lyrics are not from a SYLT frame"),
            text: text.to_owned(),
        }
    }

    fn problem<S>(field: TagField, source: S, reason: Reason) -> TagProblem<S> {
        TagProblem::Value {
            field,
            source,
            reason,
        }
    }

    fn invalid(error: ValueError) -> Reason {
        Reason::Invalid(error)
    }

    /// What mapping gives when only `tags` were read, each from the source
    /// beside it.
    fn mapped<S>(tags: TrackTags, sources: Vec<(TagField, S)>) -> Mapped<S> {
        Mapped {
            tags,
            sources,
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
    fn keeps_the_first_value_of_a_single_field() {
        let single = [
            TagField::Title,
            TagField::TitleSort,
            TagField::Album,
            TagField::AlbumSort,
            TagField::DiscSubtitle,
        ];
        let read: Vec<Mapped<u8>> = single
            .iter()
            .map(|field| set(&[(*field, "One"), (*field, "Two")]))
            .collect();
        let tags = [
            TrackTags {
                title: Some(String::from("One")),
                ..TrackTags::default()
            },
            TrackTags {
                title_sort: Some(String::from("One")),
                ..TrackTags::default()
            },
            TrackTags {
                album: Some(String::from("One")),
                ..TrackTags::default()
            },
            TrackTags {
                album_sort: Some(String::from("One")),
                ..TrackTags::default()
            },
            TrackTags {
                disc_subtitle: Some(String::from("One")),
                ..TrackTags::default()
            },
        ];
        let expected: Vec<Mapped<u8>> = tags
            .into_iter()
            .zip(single)
            .map(|(tags, field)| mapped(tags, vec![(field, 0)]))
            .collect();
        assert_eq!(read, expected);
    }

    #[test]
    fn keeps_every_value_of_a_list_in_order_with_each_source() {
        let lists = [
            TagField::Artist,
            TagField::ArtistSort,
            TagField::AlbumArtist,
            TagField::AlbumArtistSort,
            TagField::Genres,
            TagField::Moods,
            TagField::Labels,
            TagField::Grouping,
        ];
        let read: Vec<Mapped<u8>> = lists
            .iter()
            .map(|field| set(&[(*field, "One"), (*field, "Two")]))
            .collect();
        let both = || strings(&["One", "Two"]);
        let tags = [
            TrackTags {
                artist: both(),
                ..TrackTags::default()
            },
            TrackTags {
                artist_sort: both(),
                ..TrackTags::default()
            },
            TrackTags {
                album_artist: both(),
                ..TrackTags::default()
            },
            TrackTags {
                album_artist_sort: both(),
                ..TrackTags::default()
            },
            TrackTags {
                genres: both(),
                ..TrackTags::default()
            },
            TrackTags {
                moods: both(),
                ..TrackTags::default()
            },
            TrackTags {
                labels: both(),
                ..TrackTags::default()
            },
            TrackTags {
                grouping: both(),
                ..TrackTags::default()
            },
        ];
        let expected: Vec<Mapped<u8>> = tags
            .into_iter()
            .zip(lists)
            .map(|(tags, field)| mapped(tags, vec![(field, 0), (field, 1)]))
            .collect();
        assert_eq!(read, expected);
    }

    #[test]
    fn removes_controls_and_leaves_blank_values_out() {
        assert_eq!(
            set(&[
                (TagField::Title, "A\u{202E}B\nC\u{0}"),
                (TagField::Album, " \t "),
                (TagField::Genres, ""),
                (TagField::Genres, " Jazz "),
            ]),
            mapped(
                TrackTags {
                    title: Some(String::from("ABC")),
                    genres: strings(&[" Jazz "]),
                    ..TrackTags::default()
                },
                vec![(TagField::Title, 0), (TagField::Genres, 3)],
            )
        );
    }

    #[test]
    fn reads_numbers_alone_and_with_their_totals() {
        let expected_position = position(Some(3), Some(12), Some(1), Some(2));
        assert_eq!(
            set(&[(TagField::Track, " 03 / 12 "), (TagField::Disc, "1/2")]),
            mapped(
                TrackTags {
                    position: expected_position,
                    ..TrackTags::default()
                },
                vec![
                    (TagField::Track, 0),
                    (TagField::TrackTotal, 0),
                    (TagField::Disc, 1),
                    (TagField::DiscTotal, 1),
                ],
            )
        );
        assert_eq!(
            set(&[
                (TagField::Track, "3"),
                (TagField::TrackTotal, "12"),
                (TagField::Disc, "1"),
                (TagField::DiscTotal, "2"),
                (TagField::Track, "4/13"),
                (TagField::Disc, "2/3"),
            ]),
            mapped(
                TrackTags {
                    position: expected_position,
                    ..TrackTags::default()
                },
                vec![
                    (TagField::Track, 0),
                    (TagField::TrackTotal, 1),
                    (TagField::Disc, 2),
                    (TagField::DiscTotal, 3),
                ],
            )
        );
    }

    #[test]
    fn a_zero_number_or_total_means_not_given() {
        assert_eq!(
            set(&[(TagField::Track, "3/0"), (TagField::Disc, "0/2")]),
            mapped(
                TrackTags {
                    position: position(Some(3), None, None, Some(2)),
                    ..TrackTags::default()
                },
                vec![(TagField::Track, 0), (TagField::DiscTotal, 1)],
            )
        );
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn keeps_numbers_up_to_9999_and_drops_the_rest_with_the_reason() {
        let read = set(&[
            (TagField::Track, "9999/9999"),
            (TagField::Disc, "10000/10000"),
            (TagField::Disc, "x/2"),
            (TagField::Disc, "3/"),
            (TagField::Disc, "70000"),
            (TagField::Disc, "99999999999999999999999"),
            (TagField::Disc, "-1"),
            (TagField::Disc, "+4"),
        ]);
        let malformed = |field| invalid(ValueError::Malformed { field });
        let out_of_range = |field, value| invalid(ValueError::OutOfRange { field, value });
        assert_eq!(
            read,
            Mapped {
                tags: TrackTags {
                    position: position(Some(9_999), Some(9_999), Some(3), Some(2)),
                    ..TrackTags::default()
                },
                sources: vec![
                    (TagField::Track, 0),
                    (TagField::TrackTotal, 0),
                    (TagField::DiscTotal, 2),
                    (TagField::Disc, 3),
                ],
                problems: vec![
                    problem(TagField::Disc, 1, out_of_range(Field::Number, 10_000)),
                    problem(TagField::DiscTotal, 1, out_of_range(Field::Total, 10_000)),
                    problem(TagField::Disc, 2, malformed(Field::Number)),
                    problem(TagField::DiscTotal, 3, malformed(Field::Total)),
                    problem(TagField::Disc, 4, out_of_range(Field::Number, 70_000)),
                    problem(TagField::Disc, 5, out_of_range(Field::Number, u64::MAX)),
                    problem(TagField::Disc, 6, malformed(Field::Number)),
                    problem(TagField::Disc, 7, malformed(Field::Number)),
                ],
            }
        );
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn reads_dates_and_drops_one_that_does_not_exist() {
        assert_eq!(
            set(&[
                (TagField::Date, "soon"),
                (TagField::Date, "1959-13-01"),
                (TagField::Date, "1959-08-17T07:00:00Z"),
                (TagField::Date, "1960"),
                (TagField::OriginalDate, "1959-08"),
                (TagField::OriginalDate, "1961"),
            ]),
            Mapped {
                tags: TrackTags {
                    date: Some(day(1959, Some(8), Some(17))),
                    original_date: Some(day(1959, Some(8), None)),
                    ..TrackTags::default()
                },
                sources: vec![(TagField::Date, 2), (TagField::OriginalDate, 4)],
                problems: vec![
                    problem(
                        TagField::Date,
                        0,
                        invalid(ValueError::Malformed { field: Field::Year })
                    ),
                    problem(
                        TagField::Date,
                        1,
                        invalid(ValueError::OutOfRange {
                            field: Field::Month,
                            value: 13
                        })
                    ),
                ],
            }
        );
    }

    #[test]
    fn reads_a_compilation_flag_of_one_or_zero() {
        let read: Vec<Mapped<u8>> = ["1", " 0 ", "yes"]
            .into_iter()
            .map(|value| set(&[(TagField::Compilation, value), (TagField::Compilation, "1")]))
            .collect();
        let flagged = |compilation| TrackTags {
            compilation: Some(compilation),
            ..TrackTags::default()
        };
        assert_eq!(
            read,
            [
                mapped(flagged(true), vec![(TagField::Compilation, 0)]),
                mapped(flagged(false), vec![(TagField::Compilation, 0)]),
                Mapped {
                    tags: flagged(true),
                    sources: vec![(TagField::Compilation, 1)],
                    problems: vec![problem(TagField::Compilation, 0, Reason::Unreadable)],
                },
            ]
        );
    }

    #[test]
    fn reads_an_advisory_as_rtng_numbers_it() {
        let read: Vec<Mapped<u8>> = ["0", "1", " 2 ", "3", "4"]
            .into_iter()
            .map(|value| set(&[(TagField::Advisory, value)]))
            .collect();
        let advised = |advisory| {
            mapped(
                TrackTags {
                    advisory: Some(advisory),
                    ..TrackTags::default()
                },
                vec![(TagField::Advisory, 0)],
            )
        };
        assert_eq!(
            read,
            [
                mapped(TrackTags::default(), Vec::new()),
                advised(Advisory::Explicit),
                advised(Advisory::Clean),
                Mapped {
                    tags: TrackTags::default(),
                    sources: Vec::new(),
                    problems: vec![problem(TagField::Advisory, 0, Reason::Unreadable)],
                },
                advised(Advisory::Explicit),
            ]
        );
        assert_eq!(
            set(&[(TagField::Advisory, "2"), (TagField::Advisory, "1")]),
            advised(Advisory::Clean)
        );
    }

    #[test]
    fn reads_every_primary_release_type() {
        let read: Vec<Mapped<u8>> = ["album", "Single", "EP", "broadcast", " other "]
            .into_iter()
            .map(|value| set(&[(TagField::ReleaseType, value)]))
            .collect();
        let expected: Vec<Mapped<u8>> = [
            PrimaryType::Album,
            PrimaryType::Single,
            PrimaryType::Ep,
            PrimaryType::Broadcast,
            PrimaryType::Other,
        ]
        .into_iter()
        .map(|primary| {
            mapped(
                TrackTags {
                    release_type: Some(ReleaseType {
                        primary: Some(primary),
                        secondary: Vec::new(),
                    }),
                    ..TrackTags::default()
                },
                vec![(TagField::ReleaseType, 0)],
            )
        })
        .collect();
        assert_eq!(read, expected);
    }

    #[test]
    fn reads_every_secondary_release_type_and_the_first_primary() {
        let read = set(&[
            (TagField::ReleaseType, "Compilation"),
            (TagField::ReleaseType, "soundtrack"),
            (TagField::ReleaseType, "spokenword"),
            (TagField::ReleaseType, "interview"),
            (TagField::ReleaseType, "audiobook"),
            (TagField::ReleaseType, "audio drama"),
            (TagField::ReleaseType, "ep"),
            (TagField::ReleaseType, "live"),
            (TagField::ReleaseType, "remix"),
            (TagField::ReleaseType, "DJ-mix"),
            (TagField::ReleaseType, "mixtape/street"),
            (TagField::ReleaseType, "album"),
            (TagField::ReleaseType, "demo"),
            (TagField::ReleaseType, "field recording"),
            (TagField::ReleaseType, "bootleg"),
        ]);
        let sources = [0_u8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13]
            .into_iter()
            .map(|source| (TagField::ReleaseType, source))
            .collect();
        assert_eq!(
            read,
            Mapped {
                tags: TrackTags {
                    release_type: Some(ReleaseType {
                        primary: Some(PrimaryType::Ep),
                        secondary: vec![
                            SecondaryType::Compilation,
                            SecondaryType::Soundtrack,
                            SecondaryType::Spokenword,
                            SecondaryType::Interview,
                            SecondaryType::Audiobook,
                            SecondaryType::AudioDrama,
                            SecondaryType::Live,
                            SecondaryType::Remix,
                            SecondaryType::DjMix,
                            SecondaryType::Mixtape,
                            SecondaryType::Demo,
                            SecondaryType::FieldRecording,
                        ],
                    }),
                    ..TrackTags::default()
                },
                sources,
                problems: vec![problem(TagField::ReleaseType, 14, Reason::Unreadable)],
            }
        );
    }

    #[test]
    fn an_unknown_release_type_alone_leaves_the_type_unset() {
        assert_eq!(
            set(&[(TagField::ReleaseType, "bootleg")]),
            Mapped {
                tags: TrackTags::default(),
                sources: Vec::new(),
                problems: vec![problem(TagField::ReleaseType, 0, Reason::Unreadable)],
            }
        );
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn reads_identifiers_and_drops_malformed_ones_with_the_reason() {
        let read = set(&[
            (TagField::RecordingMbid, "../../x"),
            (TagField::RecordingMbid, RECORDING),
            (TagField::RecordingMbid, TRACK),
            (TagField::TrackMbid, TRACK),
            (TagField::ReleaseMbid, RELEASE),
            (TagField::ReleaseGroupMbid, GROUP),
            (TagField::ArtistMbids, ARTIST),
            (
                TagField::ArtistMbids,
                "{55555555-5555-4555-8555-555555555555}",
            ),
            (TagField::ArtistMbids, SECOND_ARTIST),
            (TagField::AlbumArtistMbids, ALBUM_ARTIST),
            (TagField::Isrc, "uss1z9900001"),
            (TagField::Isrc, "USS1Z99"),
            (TagField::Isrc, "GB-AAA-99-00002"),
        ]);
        let malformed = |field| invalid(ValueError::Malformed { field });
        let code = |text| Isrc::parse(Untrusted::new(text)).expect("the test's code is valid");
        assert_eq!(
            read,
            Mapped {
                tags: TrackTags {
                    isrc: vec![code("USS1Z9900001"), code("GBAAA9900002")],
                    musicbrainz: MbIds {
                        recording: Some(id(RECORDING)),
                        track: Some(id(TRACK)),
                        release: Some(id(RELEASE)),
                        release_group: Some(id(GROUP)),
                        artists: vec![id(ARTIST), id(SECOND_ARTIST)],
                        album_artists: vec![id(ALBUM_ARTIST)],
                    },
                    ..TrackTags::default()
                },
                sources: vec![
                    (TagField::RecordingMbid, 1),
                    (TagField::TrackMbid, 3),
                    (TagField::ReleaseMbid, 4),
                    (TagField::ReleaseGroupMbid, 5),
                    (TagField::ArtistMbids, 6),
                    (TagField::ArtistMbids, 8),
                    (TagField::AlbumArtistMbids, 9),
                    (TagField::Isrc, 10),
                    (TagField::Isrc, 12),
                ],
                problems: vec![
                    problem(TagField::RecordingMbid, 0, malformed(Field::Mbid)),
                    problem(TagField::ArtistMbids, 7, malformed(Field::Mbid)),
                    problem(TagField::Isrc, 11, malformed(Field::Isrc)),
                ],
            }
        );
    }

    #[test]
    fn pairs_each_gain_with_its_peak_in_either_order() {
        assert_eq!(
            set(&[
                (TagField::TrackPeak, "0.5"),
                (TagField::TrackGain, "-6.50 dB"),
                (TagField::AlbumGain, "+1.25 dB"),
                (TagField::AlbumPeak, "1.0"),
                (TagField::TrackGain, "-1.00 dB"),
                (TagField::AlbumPeak, "0.25"),
            ]),
            mapped(
                TrackTags {
                    gain: GainTags {
                        track: Some(replay_gain(-6.5, Some(0.5))),
                        album: Some(replay_gain(1.25, Some(1.0))),
                    },
                    ..TrackTags::default()
                },
                vec![
                    (TagField::TrackPeak, 0),
                    (TagField::TrackGain, 1),
                    (TagField::AlbumGain, 2),
                    (TagField::AlbumPeak, 3),
                ],
            )
        );
    }

    /// A peak with no gain to go with it is dropped, with the reason, and
    /// is not named as a source.
    #[test]
    fn keeps_a_gain_without_a_peak_and_drops_a_peak_without_a_gain() {
        assert_eq!(
            set(&[
                (TagField::TrackGain, "-6.5 dB"),
                (TagField::AlbumPeak, "1.0")
            ]),
            Mapped {
                tags: TrackTags {
                    gain: GainTags {
                        track: Some(replay_gain(-6.5, None)),
                        album: None,
                    },
                    ..TrackTags::default()
                },
                sources: vec![(TagField::TrackGain, 0)],
                problems: vec![problem(TagField::AlbumPeak, 1, Reason::Unpaired)],
            }
        );
        assert_eq!(
            set(&[(TagField::AlbumGain, "2 dB"), (TagField::TrackPeak, "1.0")]),
            Mapped {
                tags: TrackTags {
                    gain: GainTags {
                        track: None,
                        album: Some(replay_gain(2.0, None)),
                    },
                    ..TrackTags::default()
                },
                sources: vec![(TagField::AlbumGain, 0)],
                problems: vec![problem(TagField::TrackPeak, 1, Reason::Unpaired)],
            }
        );
    }

    /// A peak whose gain was dropped is dropped too, after the gain.
    #[test]
    fn drops_a_peak_whose_gain_was_dropped() {
        assert_eq!(
            set(&[
                (TagField::AlbumPeak, "0.5"),
                (TagField::Title, "Kept"),
                (TagField::TrackPeak, "0.25"),
                (TagField::AlbumGain, "NaN"),
                (TagField::TrackGain, "200 dB"),
            ]),
            Mapped {
                tags: TrackTags {
                    title: Some(String::from("Kept")),
                    ..TrackTags::default()
                },
                sources: vec![(TagField::Title, 1)],
                problems: vec![
                    problem(
                        TagField::AlbumGain,
                        3,
                        invalid(ValueError::Malformed { field: Field::Gain })
                    ),
                    problem(
                        TagField::TrackGain,
                        4,
                        invalid(ValueError::Unusable { field: Field::Gain })
                    ),
                    problem(TagField::TrackPeak, 2, Reason::Unpaired),
                    problem(TagField::AlbumPeak, 0, Reason::Unpaired),
                ],
            }
        );
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn drops_gains_and_peaks_that_are_not_usable_numbers() {
        let read = set(&[
            (TagField::TrackGain, "NaN"),
            (TagField::TrackGain, "inf"),
            (TagField::TrackGain, "+1e308 dB"),
            (TagField::AlbumGain, "200 dB"),
            (TagField::TrackPeak, "nan"),
            (TagField::AlbumPeak, "17"),
        ]);
        let malformed = |field| invalid(ValueError::Malformed { field });
        let unusable = |field| invalid(ValueError::Unusable { field });
        assert_eq!(
            read,
            Mapped {
                tags: TrackTags::default(),
                sources: Vec::new(),
                problems: vec![
                    problem(TagField::TrackGain, 0, malformed(Field::Gain)),
                    problem(TagField::TrackGain, 1, malformed(Field::Gain)),
                    problem(TagField::TrackGain, 2, malformed(Field::Gain)),
                    problem(TagField::AlbumGain, 3, unusable(Field::Gain)),
                    problem(TagField::TrackPeak, 4, malformed(Field::Peak)),
                    problem(TagField::AlbumPeak, 5, unusable(Field::Peak)),
                ],
            }
        );
    }

    /// The value written for a file with 2,112 samples of delay and 458 of
    /// padding.
    const SMPB: &str = " 00000000 00000840 000001CA 00000000003F31F6 00000000 00000000 00000000 \
                        00000000 00000000 00000000 00000000 00000000";

    #[test]
    fn reads_the_delay_and_padding_of_itunsmpb() {
        let trimmed = |delay, padding| {
            mapped(
                TrackTags {
                    trim: Some(Trim { delay, padding }),
                    ..TrackTags::default()
                },
                vec![(TagField::Trim, 0)],
            )
        };
        assert_eq!(
            set(&[(TagField::Trim, SMPB), (TagField::Trim, "0 1 2")]),
            trimmed(2_112, 458)
        );
        // Without the leading space, with two spaces, in lower case and
        // with only the three numbers that matter.
        assert_eq!(
            set(&[(TagField::Trim, "00000000 00000840  000001ca")]),
            trimmed(2_112, 458)
        );
        assert_eq!(
            set(&[(TagField::Trim, "0 FFFFFFFF 0")]),
            trimmed(u32::MAX, 0)
        );
    }

    #[test]
    fn drops_an_itunsmpb_value_that_is_not_hexadecimal_numbers() {
        let cases = [
            " 00000000 00000840",
            " 00000000",
            " 00000000 0000084G 000001CA",
            " 00000000 00000840 000001CG",
            " 00000000 +0000840 000001CA",
            " 00000000 100000000 000001CA",
        ];
        for case in cases {
            assert_eq!(
                set(&[(TagField::Trim, case)]),
                Mapped {
                    tags: TrackTags::default(),
                    sources: Vec::new(),
                    problems: vec![problem(TagField::Trim, 0, Reason::Unreadable)],
                }
            );
        }
    }

    #[test]
    fn credits_each_role_and_reads_a_performers_instrument() {
        let read = set(&[
            (TagField::Credit(Role::Composer), "Miles Davis (trumpet)"),
            (
                TagField::Credit(Role::Performer),
                "Jaco Pastorius (fretless bass)",
            ),
            (TagField::Credit(Role::Performer), "Bill Evans"),
            (TagField::Credit(Role::Performer), " (violin)"),
            (TagField::Credit(Role::Performer), "A (b) (c)"),
            (TagField::Credit(Role::Performer), "Name(tight)"),
        ]);
        assert_eq!(
            read,
            Mapped {
                tags: TrackTags {
                    credits: vec![
                        credited("Miles Davis (trumpet)", Role::Composer, None),
                        credited("Jaco Pastorius", Role::Performer, Some("fretless bass")),
                        credited("Bill Evans", Role::Performer, None),
                        credited("A (b)", Role::Performer, Some("c")),
                        credited("Name(tight)", Role::Performer, None),
                    ],
                    ..TrackTags::default()
                },
                sources: vec![
                    (TagField::Credit(Role::Composer), 0),
                    (TagField::Credit(Role::Performer), 1),
                    (TagField::Credit(Role::Performer), 2),
                    (TagField::Credit(Role::Performer), 4),
                    (TagField::Credit(Role::Performer), 5),
                ],
                problems: vec![problem(
                    TagField::Credit(Role::Performer),
                    3,
                    Reason::Unreadable
                )],
            }
        );
    }

    #[test]
    fn keeps_the_lines_of_lyrics_and_refuses_untimed_synced_lyrics() {
        let origin = LyricsOrigin::ApeItem;
        assert_eq!(
            set(&[
                (TagField::Lyrics(origin), "one\r\ntwo\u{7}"),
                (TagField::Lyrics(LyricsOrigin::Id3Synced), "three"),
            ]),
            Mapped {
                tags: TrackTags {
                    lyrics: vec![plain_lyrics(origin, "one\ntwo")],
                    ..TrackTags::default()
                },
                sources: vec![(TagField::Lyrics(origin), 0)],
                problems: vec![problem(
                    TagField::Lyrics(LyricsOrigin::Id3Synced),
                    1,
                    Reason::Unreadable
                )],
            }
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn cuts_text_at_its_limit_and_says_so() {
        let limits = lowered(&[(LimitKind::ShortText, 4), (LimitKind::LongText, 6)]);
        let origin = LyricsOrigin::Mp4Item;
        assert_eq!(
            set_under(
                &limits,
                &[
                    (TagField::Title, "abcd"),
                    (TagField::Album, "abcde"),
                    (TagField::Lyrics(origin), "abcdef"),
                    (TagField::Lyrics(origin), "abcdefg"),
                ]
            ),
            Mapped {
                tags: TrackTags {
                    title: Some(String::from("abcd")),
                    album: Some(String::from("abcd")),
                    lyrics: vec![
                        plain_lyrics(origin, "abcdef"),
                        plain_lyrics(origin, "abcdef")
                    ],
                    ..TrackTags::default()
                },
                sources: vec![
                    (TagField::Title, 0),
                    (TagField::Album, 1),
                    (TagField::Lyrics(origin), 2),
                    (TagField::Lyrics(origin), 3),
                ],
                problems: vec![
                    problem(TagField::Album, 1, Reason::Truncated(LimitKind::ShortText)),
                    problem(
                        TagField::Lyrics(origin),
                        3,
                        Reason::Truncated(LimitKind::LongText)
                    ),
                ],
            }
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn reports_text_cut_to_nothing() {
        let limits = lowered(&[(LimitKind::ShortText, 0)]);
        assert_eq!(
            set_under(&limits, &[(TagField::Title, "a")]),
            Mapped {
                tags: TrackTags::default(),
                sources: Vec::new(),
                problems: vec![problem(
                    TagField::Title,
                    0,
                    Reason::Truncated(LimitKind::ShortText)
                )],
            }
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn a_list_holds_as_many_values_as_the_tag_field_limit_and_no_more() {
        let limits = lowered(&[(LimitKind::TagFields, 2)]);
        let full = limit(LimitKind::TagFields);
        assert_eq!(
            set_under(
                &limits,
                &[
                    (TagField::Genres, "One"),
                    (TagField::Genres, "Two"),
                    (TagField::Genres, "Three"),
                    (TagField::Artist, "One"),
                    (TagField::Artist, "Two"),
                ]
            ),
            Mapped {
                tags: TrackTags {
                    artist: strings(&["One", "Two"]),
                    genres: strings(&["One", "Two"]),
                    ..TrackTags::default()
                },
                sources: vec![
                    (TagField::Genres, 0),
                    (TagField::Genres, 1),
                    (TagField::Artist, 3),
                    (TagField::Artist, 4),
                ],
                problems: vec![problem(TagField::Genres, 2, full)],
            }
        );
    }

    fn limit(kind: LimitKind) -> Reason {
        Reason::Limit(kind)
    }

    #[test]
    fn maps_every_text_atom() {
        let read = map(&[
            words(*b"\xA9nam", &["Blue in Green"]),
            words(*b"\xA9ART", &["Miles Davis", "Bill Evans"]),
            words(*b"aART", &["Miles Davis"]),
            words(*b"\xA9alb", &["Kind of Blue"]),
            words(*b"\xA9day", &["1959-08-17T07:00:00Z"]),
            words(*b"\xA9gen", &["Jazz", "Modal"]),
            words(*b"\xA9wrt", &["Miles Davis"]),
            words(*b"\xA9grp", &["Sextet"]),
            words(*b"\xA9lyr", &["first\nsecond"]),
            words(*b"sonm", &["Blue in Green, 1959"]),
            words(*b"soar", &["Davis, Miles"]),
            words(*b"soaa", &["Davis"]),
            words(*b"soal", &["Kind of Blue, 1959"]),
            words(*b"\xA9too", &["an encoder"]),
        ]);
        let lyrics = TagField::Lyrics(LyricsOrigin::Mp4Item);
        let composer = TagField::Credit(Role::Composer);
        let fields = [
            TagField::Title,
            TagField::Artist,
            TagField::AlbumArtist,
            TagField::Album,
            TagField::Date,
            TagField::Genres,
            composer,
            TagField::Grouping,
            lyrics,
            TagField::TitleSort,
            TagField::ArtistSort,
            TagField::AlbumArtistSort,
            TagField::AlbumSort,
        ];
        let sources = fields
            .into_iter()
            .zip((0..13).map(ItemIndex))
            .collect::<Vec<_>>();
        assert_eq!(
            read,
            mapped(
                TrackTags {
                    title: Some(String::from("Blue in Green")),
                    title_sort: Some(String::from("Blue in Green, 1959")),
                    artist: strings(&["Miles Davis", "Bill Evans"]),
                    artist_sort: strings(&["Davis, Miles"]),
                    album_artist: strings(&["Miles Davis"]),
                    album_artist_sort: strings(&["Davis"]),
                    album: Some(String::from("Kind of Blue")),
                    album_sort: Some(String::from("Kind of Blue, 1959")),
                    date: Some(day(1959, Some(8), Some(17))),
                    genres: strings(&["Jazz", "Modal"]),
                    grouping: strings(&["Sextet"]),
                    credits: vec![credited("Miles Davis", Role::Composer, None)],
                    lyrics: vec![plain_lyrics(LyricsOrigin::Mp4Item, "first\nsecond")],
                    ..TrackTags::default()
                },
                sources,
            )
        );
    }

    #[test]
    fn maps_the_identifier_and_gain_freeform_items() {
        let read = map(&[
            free("MusicBrainz Track Id", &[RECORDING]),
            free("MusicBrainz Release Track Id", &[TRACK]),
            free("MusicBrainz Album Id", &[RELEASE]),
            free("MusicBrainz Release Group Id", &[GROUP]),
            free("MusicBrainz Artist Id", &[ARTIST, SECOND_ARTIST]),
            free("MUSICBRAINZ ALBUM ARTIST ID", &[ALBUM_ARTIST]),
            free("MusicBrainz Album Type", &["album", "live"]),
            free("replaygain_track_gain", &["-6.50 dB"]),
            free("replaygain_track_peak", &["0.5"]),
            free("REPLAYGAIN_ALBUM_GAIN", &["+1.25 dB"]),
            free("replaygain_album_peak", &["1.0"]),
            free("iTunSMPB", &[SMPB]),
            free("ISRC", &["USS1Z9900001"]),
            free("Encoding Params", &["vers"]),
        ]);
        let fields = [
            TagField::RecordingMbid,
            TagField::TrackMbid,
            TagField::ReleaseMbid,
            TagField::ReleaseGroupMbid,
            TagField::ArtistMbids,
            TagField::AlbumArtistMbids,
            TagField::ReleaseType,
            TagField::TrackGain,
            TagField::TrackPeak,
            TagField::AlbumGain,
            TagField::AlbumPeak,
            TagField::Trim,
            TagField::Isrc,
        ];
        let sources = fields
            .into_iter()
            .zip((0..13).map(ItemIndex))
            .collect::<Vec<_>>();
        assert_eq!(
            read,
            mapped(
                TrackTags {
                    release_type: Some(ReleaseType {
                        primary: Some(PrimaryType::Album),
                        secondary: vec![SecondaryType::Live],
                    }),
                    isrc: vec![
                        Isrc::parse(Untrusted::new("USS1Z9900001")).expect("the code is valid")
                    ],
                    musicbrainz: MbIds {
                        recording: Some(id(RECORDING)),
                        track: Some(id(TRACK)),
                        release: Some(id(RELEASE)),
                        release_group: Some(id(GROUP)),
                        artists: vec![id(ARTIST), id(SECOND_ARTIST)],
                        album_artists: vec![id(ALBUM_ARTIST)],
                    },
                    gain: GainTags {
                        track: Some(replay_gain(-6.5, Some(0.5))),
                        album: Some(replay_gain(1.25, Some(1.0))),
                    },
                    trim: Some(Trim {
                        delay: 2_112,
                        padding: 458,
                    }),
                    ..TrackTags::default()
                },
                sources,
            )
        );
    }

    #[test]
    fn maps_the_other_freeform_items() {
        let read = map(&[
            free("LABEL", &["Columbia"]),
            free("MOOD", &["Calm"]),
            free("DISCSUBTITLE", &["Side one"]),
            free("originaldate", &["1959"]),
            free("ITUNESADVISORY", &["1"]),
            free("CONDUCTOR", &["A"]),
            free("LYRICIST", &["B"]),
            free("REMIXER", &["C"]),
            free("PRODUCER", &["D"]),
            free("ARRANGER", &["E"]),
            free("ENGINEER", &["F"]),
            free("MIXER", &["G"]),
            free("DJMIXER", &["H"]),
        ]);
        let roles = [
            Role::Conductor,
            Role::Lyricist,
            Role::Remixer,
            Role::Producer,
            Role::Arranger,
            Role::Engineer,
            Role::Mixer,
            Role::DjMixer,
        ];
        let fields = [
            TagField::Labels,
            TagField::Moods,
            TagField::DiscSubtitle,
            TagField::OriginalDate,
            TagField::Advisory,
        ]
        .into_iter()
        .chain(roles.into_iter().map(TagField::Credit));
        let sources = fields.zip((0..13).map(ItemIndex)).collect::<Vec<_>>();
        let credits = roles
            .into_iter()
            .zip(["A", "B", "C", "D", "E", "F", "G", "H"])
            .map(|(role, name)| credited(name, role, None))
            .collect();
        assert_eq!(
            read,
            mapped(
                TrackTags {
                    disc_subtitle: Some(String::from("Side one")),
                    original_date: Some(day(1959, None, None)),
                    moods: strings(&["Calm"]),
                    labels: strings(&["Columbia"]),
                    credits,
                    advisory: Some(Advisory::Explicit),
                    ..TrackTags::default()
                },
                sources,
            )
        );
    }

    #[test]
    fn a_freeform_item_needs_no_namespace() {
        let bare = IlstItem {
            key: ItemKey::Freeform {
                mean: None,
                name: name("label"),
            },
            values: vec![text("Blue Note")],
        };
        assert_eq!(
            map(&[bare]),
            mapped(
                TrackTags {
                    labels: strings(&["Blue Note"]),
                    ..TrackTags::default()
                },
                vec![(TagField::Labels, ItemIndex(0))],
            )
        );
    }

    #[test]
    fn reads_trkn_and_disk() {
        assert_eq!(
            map(&[
                item(*b"trkn", vec![binary(&[0, 0, 0, 3, 0, 12, 0, 0])]),
                item(*b"disk", vec![binary(&[0, 0, 1, 2, 0, 2])]),
            ]),
            mapped(
                TrackTags {
                    position: position(Some(3), Some(12), Some(258), Some(2)),
                    ..TrackTags::default()
                },
                vec![
                    (TagField::Track, ItemIndex(0)),
                    (TagField::TrackTotal, ItemIndex(0)),
                    (TagField::Disc, ItemIndex(1)),
                    (TagField::DiscTotal, ItemIndex(1)),
                ],
            )
        );
    }

    #[test]
    fn a_trkn_with_a_zero_total_keeps_the_number() {
        assert_eq!(
            map(&[item(*b"trkn", vec![binary(&[0, 0, 0, 3, 0, 0, 0, 0])])]),
            mapped(
                TrackTags {
                    position: position(Some(3), None, None, None),
                    ..TrackTags::default()
                },
                vec![(TagField::Track, ItemIndex(0))],
            )
        );
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn drops_a_trkn_that_is_short_out_of_range_or_not_binary() {
        let read = map(&[
            item(*b"trkn", vec![binary(&[0, 0, 0, 3, 0])]),
            item(*b"trkn", vec![binary(&[0, 0, 0x27, 0x10, 0x27, 0x0F])]),
            item(*b"disk", vec![text("1/2")]),
            item(*b"disk", vec![binary(&[0, 0, 0])]),
            item(*b"disk", vec![binary(&[0])]),
        ]);
        assert_eq!(
            read,
            Mapped {
                tags: TrackTags {
                    position: position(None, Some(9_999), None, None),
                    ..TrackTags::default()
                },
                sources: vec![(TagField::TrackTotal, ItemIndex(1))],
                problems: vec![
                    problem(TagField::Track, ItemIndex(0), Reason::Unreadable),
                    problem(
                        TagField::Track,
                        ItemIndex(1),
                        invalid(ValueError::OutOfRange {
                            field: Field::Number,
                            value: 10_000
                        })
                    ),
                    problem(TagField::Disc, ItemIndex(2), Reason::Unreadable),
                    problem(TagField::Disc, ItemIndex(3), Reason::Unreadable),
                    problem(TagField::Disc, ItemIndex(4), Reason::Unreadable),
                ],
            }
        );
    }

    #[test]
    fn reads_rtng_values_0_1_2_and_4() {
        let read: Vec<Mapped<ItemIndex>> = [0, 1, 2, 4]
            .into_iter()
            .map(|rating| map(&[item(*b"rtng", vec![ItemValue::Signed(rating)])]))
            .collect();
        let advised = |advisory| {
            mapped(
                TrackTags {
                    advisory: Some(advisory),
                    ..TrackTags::default()
                },
                vec![(TagField::Advisory, ItemIndex(0))],
            )
        };
        assert_eq!(
            read,
            [
                mapped(TrackTags::default(), Vec::new()),
                advised(Advisory::Explicit),
                advised(Advisory::Clean),
                advised(Advisory::Explicit),
            ]
        );
    }

    #[test]
    fn reads_cpil_however_the_number_is_typed() {
        let read = map(&[
            item(*b"cpil", vec![ItemValue::Signed(-1)]),
            item(*b"cpil", vec![text("1")]),
            item(
                *b"cpil",
                vec![ItemValue::Picture(PictureRef {
                    type_code: 13,
                    offset: 0,
                    len: 1,
                })],
            ),
            item(*b"cpil", vec![ItemValue::Unsigned(2)]),
            item(*b"cpil", vec![binary(&[0, 1])]),
            item(*b"cpil", vec![ItemValue::Unsigned(0)]),
        ]);
        let unreadable =
            |index| problem(TagField::Compilation, ItemIndex(index), Reason::Unreadable);
        assert_eq!(
            read,
            Mapped {
                tags: TrackTags {
                    compilation: Some(true),
                    ..TrackTags::default()
                },
                sources: vec![(TagField::Compilation, ItemIndex(4))],
                problems: vec![unreadable(0), unreadable(1), unreadable(2), unreadable(3)],
            }
        );
        assert_eq!(
            map(&[item(*b"cpil", vec![ItemValue::Signed(0)])]),
            mapped(
                TrackTags {
                    compilation: Some(false),
                    ..TrackTags::default()
                },
                vec![(TagField::Compilation, ItemIndex(0))],
            )
        );
    }

    #[test]
    fn a_binary_number_is_big_endian_and_saturates() {
        assert_eq!(
            map(&[
                item(*b"trkn", vec![binary(&[0, 0, 0, 1, 0, 0])]),
                item(*b"cpil", vec![binary(&[1, 0])]),
                item(*b"cpil", vec![binary(&[0xFF; 9])]),
                item(*b"cpil", vec![binary(&[])]),
            ]),
            Mapped {
                tags: TrackTags {
                    position: position(Some(1), None, None, None),
                    compilation: Some(false),
                    ..TrackTags::default()
                },
                sources: vec![
                    (TagField::Track, ItemIndex(0)),
                    (TagField::Compilation, ItemIndex(3)),
                ],
                problems: vec![
                    problem(TagField::Compilation, ItemIndex(1), Reason::Unreadable),
                    problem(TagField::Compilation, ItemIndex(2), Reason::Unreadable),
                ],
            }
        );
    }

    #[test]
    fn names_the_genre_of_a_gnre_number() {
        let read = map(&[
            item(*b"gnre", vec![binary(&[0, 1])]),
            item(*b"gnre", vec![binary(&[0, 18])]),
            item(*b"gnre", vec![ItemValue::Signed(80)]),
            item(*b"gnre", vec![ItemValue::Unsigned(148)]),
            item(*b"gnre", vec![binary(&[0, 192])]),
            item(*b"gnre", vec![binary(&[0, 0])]),
            item(*b"gnre", vec![binary(&[0, 193])]),
            item(*b"gnre", vec![text("Rock")]),
        ]);
        let unreadable = |index| problem(TagField::Genres, ItemIndex(index), Reason::Unreadable);
        assert_eq!(
            read,
            Mapped {
                tags: TrackTags {
                    genres: strings(&["Blues", "Rock", "Hard Rock", "Synthpop", "Psybient"]),
                    ..TrackTags::default()
                },
                sources: (0..5)
                    .map(|index| (TagField::Genres, ItemIndex(index)))
                    .collect(),
                problems: vec![unreadable(5), unreadable(6), unreadable(7)],
            }
        );
        assert_eq!(GENRES.len(), 192);
    }

    #[test]
    fn a_text_atom_with_a_value_that_is_not_text_is_dropped() {
        let unreadable = problem(TagField::Title, ItemIndex(0), Reason::Unreadable);
        assert_eq!(
            map(&[item(
                *b"\xA9nam",
                vec![
                    ItemValue::Signed(1),
                    ItemValue::Unsigned(1),
                    binary(b"Title"),
                    ItemValue::Picture(PictureRef {
                        type_code: 13,
                        offset: 0,
                        len: 1,
                    }),
                    text("Title"),
                ]
            )]),
            Mapped {
                tags: TrackTags {
                    title: Some(String::from("Title")),
                    ..TrackTags::default()
                },
                sources: vec![(TagField::Title, ItemIndex(0))],
                problems: vec![unreadable, unreadable, unreadable, unreadable],
            }
        );
    }

    #[test]
    fn charges_one_step_for_each_item_and_each_value() {
        let items = [
            words(*b"\xA9nam", &["Title"]),
            words(*b"\xA9too", &["an encoder", "another"]),
            words(*b"\xA9alb", &["Album"]),
        ];
        let mut budget = Budget::for_input(0, 0, 7);
        assert_eq!(
            from_ilst(&items, &Limits::DEFAULT, &mut budget),
            mapped(
                TrackTags {
                    title: Some(String::from("Title")),
                    album: Some(String::from("Album")),
                    ..TrackTags::default()
                },
                vec![
                    (TagField::Title, ItemIndex(0)),
                    (TagField::Album, ItemIndex(2)),
                ],
            )
        );
        assert_eq!(budget.remaining(), 0);
    }

    #[test]
    fn stops_at_the_item_the_budget_cannot_pay_for() {
        let items = [
            words(*b"\xA9nam", &["Title"]),
            words(*b"\xA9gen", &["Jazz", "Modal"]),
            words(*b"\xA9alb", &["Album"]),
            words(*b"\xA9ART", &["Artist"]),
        ];
        let mut budget = Budget::for_input(0, 0, 6);
        assert_eq!(
            from_ilst(&items, &Limits::DEFAULT, &mut budget),
            Mapped {
                tags: TrackTags {
                    title: Some(String::from("Title")),
                    genres: strings(&["Jazz", "Modal"]),
                    ..TrackTags::default()
                },
                sources: vec![
                    (TagField::Title, ItemIndex(0)),
                    (TagField::Genres, ItemIndex(1)),
                ],
                problems: vec![TagProblem::BudgetSpent {
                    source: ItemIndex(2)
                }],
            }
        );
        assert_eq!(budget.remaining(), 0);
    }

    #[test]
    fn an_empty_list_maps_to_no_tags() {
        assert_eq!(map(&[]), mapped(TrackTags::default(), Vec::new()));
    }

    /// A whole MP4 file whose item list is `items`: a file type box, then a
    /// movie that holds one AAC track, because the probe returns nothing
    /// for a file without an audio track, and the list in `udta`.
    fn file(items: &[u8]) -> Vec<u8> {
        let esds = kit::esds(&kit::Esds {
            object_type: 0x40,
            max_bitrate: 128_000,
            avg_bitrate: 96_000,
            specific: Some(&kit::audio_specific_config(2, 4, 2)),
            width: 4,
        });
        let entry = kit::sample_entry(&kit::SampleEntry {
            format: *b"mp4a",
            version: 0,
            channels: 2,
            bits: 16,
            rate: 44_100,
            children: &esds,
        });
        let tables = [
            kit::stsd(&[&entry]),
            kit::full_box(*b"stts", 0, 0, &[0, 0, 0, 1, 0, 0, 0, 10, 0, 0, 4, 0]),
            kit::full_box(
                *b"stsc",
                0,
                0,
                &[0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 10, 0, 0, 0, 1],
            ),
            kit::full_box(*b"stsz", 0, 0, &[0, 0, 0, 16, 0, 0, 0, 10]),
            kit::full_box(*b"stco", 0, 0, &[0, 0, 0, 1, 0, 0, 0, 0]),
        ]
        .concat();
        let track = kit::trak(*b"soun", &kit::mdhd(44_100, 441_000), &tables);
        let movie = kit::mp4_box(*b"moov", &[track, kit::udta(false, items)].concat());
        [
            kit::ftyp(*b"M4A ", 0x200, &[*b"M4A ", *b"mp42", *b"isom"]),
            movie,
        ]
        .concat()
    }

    /// Maps the items the MP4 probe reads from a file whose item list is
    /// `items`, with the probe and the mapper under the same `limits`.
    fn probed(items: &[u8], limits: &Limits) -> Mapped<ItemIndex> {
        let octets = file(items);
        let probe = Probe::new(*limits, Budget::for_input(0, 0, 100_000));
        let audio = drive(probe, &octets, limits)
            .expect("the probe asks only for what the host allows")
            .expect("the file is a sound MP4 file");
        from_ilst(&audio.items, limits, &mut Budget::for_input(0, 0, 1_000))
    }

    /// An item of type `kind` as a file holds it, with one UTF-8 value.
    fn utf8_item(kind: [u8; 4], value: &[u8]) -> Vec<u8> {
        kit::mp4_box(kind, &kit::data(1, value))
    }

    /// A freeform `MOOD` item as a file holds it, with one UTF-8 value.
    fn mood_item(value: &[u8]) -> Vec<u8> {
        kit::freeform(None, Some("MOOD"), &[kit::data(1, value)])
    }

    /// The parser cuts an atom's text before the mapper sees it: at the
    /// short-text limit, or at the long-text limit for lyrics. A value of
    /// exactly the limit is whole; one octet more is cut, and the cut is
    /// recorded once for that value.
    ///
    /// Verifies: SEC-MED-006
    #[test]
    fn says_so_when_the_parser_cut_an_atom_at_its_text_limit() {
        let limits = lowered(&[(LimitKind::ShortText, 4), (LimitKind::LongText, 6)]);
        let artists = [kit::data(1, b"abcd"), kit::data(1, b"abcde")].concat();
        let items = [
            utf8_item(*b"\xA9nam", b"abcd"),
            utf8_item(*b"\xA9alb", b"abcde"),
            utf8_item(*b"\xA9lyr", b"abcdef"),
            utf8_item(*b"\xA9lyr", b"abcdefg"),
            kit::mp4_box(*b"\xA9ART", &artists),
        ]
        .concat();
        let lyrics = TagField::Lyrics(LyricsOrigin::Mp4Item);
        assert_eq!(
            probed(&items, &limits),
            Mapped {
                tags: TrackTags {
                    title: Some(String::from("abcd")),
                    artist: strings(&["abcd", "abcd"]),
                    album: Some(String::from("abcd")),
                    lyrics: vec![
                        plain_lyrics(LyricsOrigin::Mp4Item, "abcdef"),
                        plain_lyrics(LyricsOrigin::Mp4Item, "abcdef"),
                    ],
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
                    problem(
                        TagField::Album,
                        ItemIndex(1),
                        Reason::Truncated(LimitKind::ShortText)
                    ),
                    problem(lyrics, ItemIndex(3), Reason::Truncated(LimitKind::LongText)),
                    problem(
                        TagField::Artist,
                        ItemIndex(4),
                        Reason::Truncated(LimitKind::ShortText)
                    ),
                ],
            }
        );
    }

    /// The parser cuts a freeform value at the long-text limit, and the
    /// field rules then cut a mood at the short-text limit. A value that
    /// breaches both records both, the parser's cut first.
    ///
    /// Verifies: SEC-MED-006
    #[test]
    fn says_which_limits_cut_a_freeform_value() {
        let limits = lowered(&[(LimitKind::ShortText, 4), (LimitKind::LongText, 6)]);
        let items = [
            mood_item(b"abcd"),
            mood_item(b"abcdef"),
            mood_item(b"abcdefg"),
        ]
        .concat();
        assert_eq!(
            probed(&items, &limits),
            Mapped {
                tags: TrackTags {
                    moods: strings(&["abcd", "abcd", "abcd"]),
                    ..TrackTags::default()
                },
                sources: vec![
                    (TagField::Moods, ItemIndex(0)),
                    (TagField::Moods, ItemIndex(1)),
                    (TagField::Moods, ItemIndex(2)),
                ],
                problems: vec![
                    problem(
                        TagField::Moods,
                        ItemIndex(1),
                        Reason::Truncated(LimitKind::ShortText)
                    ),
                    problem(
                        TagField::Moods,
                        ItemIndex(2),
                        Reason::Truncated(LimitKind::LongText)
                    ),
                    problem(
                        TagField::Moods,
                        ItemIndex(2),
                        Reason::Truncated(LimitKind::ShortText)
                    ),
                ],
            }
        );
    }

    /// With the long-text limit below the short-text one, only the parser
    /// cuts a freeform value, and the problem names the parser's limit.
    ///
    /// Verifies: SEC-MED-006
    #[test]
    fn names_the_parsers_limit_when_only_the_parser_cut_a_freeform_value() {
        let limits = lowered(&[(LimitKind::ShortText, 8), (LimitKind::LongText, 6)]);
        let items = [mood_item(b"abcdef"), mood_item(b"abcdefg")].concat();
        assert_eq!(
            probed(&items, &limits),
            Mapped {
                tags: TrackTags {
                    moods: strings(&["abcdef", "abcdef"]),
                    ..TrackTags::default()
                },
                sources: vec![
                    (TagField::Moods, ItemIndex(0)),
                    (TagField::Moods, ItemIndex(1)),
                ],
                problems: vec![problem(
                    TagField::Moods,
                    ItemIndex(1),
                    Reason::Truncated(LimitKind::LongText)
                )],
            }
        );
    }

    /// The parser's cut is recorded whatever becomes of what was left: a
    /// date that is no date, text where `trkn` holds a number, and a title
    /// that is blank. The reason the rest was dropped follows the cut.
    ///
    /// Verifies: SEC-MED-006
    #[test]
    fn says_the_parser_cut_a_value_that_is_then_dropped() {
        let limits = lowered(&[(LimitKind::ShortText, 4)]);
        let items = [
            utf8_item(*b"\xA9day", b"soon"),
            utf8_item(*b"\xA9day", b"sooner"),
            utf8_item(*b"trkn", b"abcd"),
            utf8_item(*b"trkn", b"abcde"),
            utf8_item(*b"\xA9nam", b"    "),
            utf8_item(*b"\xA9nam", b"    x"),
        ]
        .concat();
        let cut = Reason::Truncated(LimitKind::ShortText);
        let not_a_date = invalid(ValueError::Malformed { field: Field::Year });
        assert_eq!(
            probed(&items, &limits),
            Mapped {
                tags: TrackTags::default(),
                sources: Vec::new(),
                problems: vec![
                    problem(TagField::Date, ItemIndex(0), not_a_date),
                    problem(TagField::Date, ItemIndex(1), cut),
                    problem(TagField::Date, ItemIndex(1), not_a_date),
                    problem(TagField::Track, ItemIndex(2), Reason::Unreadable),
                    problem(TagField::Track, ItemIndex(3), cut),
                    problem(TagField::Track, ItemIndex(3), Reason::Unreadable),
                    problem(TagField::Title, ItemIndex(5), cut),
                ],
            }
        );
    }

    /// Any one item, with keys the mapper knows and keys it does not.
    fn any_item() -> impl Strategy<Value = IlstItem> {
        let key = prop_oneof![
            select(ATOMS).prop_map(|(kind, _)| ItemKey::Atom(FourCc(kind))),
            any::<[u8; 4]>().prop_map(|kind| ItemKey::Atom(FourCc(kind))),
            select(FREEFORM).prop_map(|(key, _)| ItemKey::Freeform {
                mean: None,
                name: name(key)
            }),
        ];
        let value = prop_oneof![
            ".{0,12}".prop_map(|value| text(&value)),
            "[0-9/ ]{0,6}".prop_map(|value| text(&value)),
            any::<i64>().prop_map(ItemValue::Signed),
            any::<u64>().prop_map(ItemValue::Unsigned),
            vec(any::<u8>(), 0..10).prop_map(|bytes| binary(&bytes)),
        ];
        (key, vec(value, 0..4)).prop_map(|(key, values)| IlstItem { key, values })
    }

    proptest! {
        #[test]
        fn every_list_stays_within_the_limits(items in vec(any_item(), 0..24)) {
            let limits = lowered(&[(LimitKind::TagFields, 3), (LimitKind::ShortText, 8)]);
            let mut budget = Budget::for_input(0, 0, 200);
            let read = from_ilst(&items, &limits, &mut budget);
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
            let texts = tags.title.iter().chain(&tags.album).chain(&tags.genres).chain(&tags.artist);
            prop_assert!(texts.into_iter().all(|text| text.len() <= 8));
            let spent = read
                .problems
                .iter()
                .any(|problem| matches!(problem, TagProblem::BudgetSpent { .. }));
            prop_assert!(!spent);
        }
    }
}
