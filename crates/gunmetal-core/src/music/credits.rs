//! Artist credits: every name a file credits, linked one by one, beside the
//! credit as it is shown (MUS-001 to MUS-004, MUS-006, MUS-035; API-CAT-01).
//!
//! A tag mapper leaves a file's artists and album artists in [`TrackTags`]
//! as it found them. [`credits`] turns each of the two fields into a
//! [`CreditLine`]: the credit as shown, and each name in it with its role,
//! its `MusicBrainz` identifier when one pairs with it, and the text that
//! joins it to the next name. A line always reads back whole: its lead,
//! then each name followed by its join, is the credit as shown, character
//! for character (MUS-002). The album artists stay a credit of their own
//! (MUS-003), so an artist who is credited on a track and not on its album
//! is a guest there (MUS-004).
//!
//! # One value: the credit as tagged
//!
//! A field with one value holds a display credit, such as "A feat. B". It
//! is shown as tagged and split by the library's [`SplitRules`] (MUS-035):
//!
//! - The value is read from left to right. A name ends where a separator
//!   stands. Where several separators stand at one place, the longest is
//!   taken.
//! - Separators and exceptions are matched as written, letter case
//!   included, so " x " leaves "Lil Nas X feat. B" its "X", and " feat. "
//!   does not split at " Feat. ". An empty separator or exception matches
//!   nothing.
//! - A name is the text between two separators without the white space
//!   around it, which belongs to the joins. Text that is only white space
//!   is no name.
//! - An exception is a name that holds a separator and is still one
//!   artist, such as "Simon & Garfunkel". It is tried where a name starts,
//!   and holds when it is that whole name: after it comes the end of the
//!   value or, past any white space, a separator. "Simon & Garfunkel Jr."
//!   and "Paul Simon & Garfunkel" are split like any other value. Where
//!   several exceptions hold, the longest is taken.
//! - A separator that reads "feat", "ft" or "featuring" once it is folded
//!   ([`collate::fold`]) introduces featured artists. Every name after it
//!   has the role [`Role::Featured`]; the others have the role of the
//!   field, [`Role::Artist`] or [`Role::AlbumArtist`].
//!
//! # Several values: one artist each
//!
//! A field with two or more values is a true multi-value tag, and that
//! takes precedence over splitting: each value is one name, whatever
//! separators it holds, without the white space around it. Blank values
//! are passed over. The credit as shown is the names joined with [`JOIN`].
//!
//! # Identifiers and identity
//!
//! `MusicBrainz` artist identifiers pair with the names by position, the
//! first with the first, when the tags hold exactly as many identifiers as
//! names. Otherwise no name of that credit gets one, and the mismatch is
//! recorded as [`Problem::IdsUnpaired`]: three names and two identifiers do
//! not say which name has none.
//!
//! [`ArtistKey`] is what tells one artist from another (MUS-006): the
//! identifier when the name has one, and otherwise the name folded for
//! matching. Two artists of one name stay apart when their identifiers
//! differ. Names are never merged within a credit: "A & A" credits two
//! names with one key. Telling artists of one name apart by the album
//! artist they appear under is left to the scan, which sees more than one
//! file.
//!
//! # Limits
//!
//! Every limit that is reached is recorded as [`Problem::Limit`]
//! (SEC-MED-006):
//!
//! - A display credit longer than [`LimitKind::ShortText`] is cut there,
//!   as the mappers cut it, and what is left is split.
//! - A credit links at most [`LimitKind::TagFields`] names. The rest of a
//!   display credit stays in the join of the last linked name, so the
//!   credit still reads as tagged.
//! - The credit shown for a multi-value field ends before the first name
//!   that would take it past [`LimitKind::ShortText`] or past
//!   [`LimitKind::TagFields`] names. When one name would do both, the text
//!   limit is the one recorded.
//!
//! Identifiers pair with the names the tags hold, not with the names that
//! are kept, so a limit never moves an identifier to another name.
//!
//! # Work
//!
//! A display credit is read once. Each separator is tried at each
//! character and each exception where a name starts, so the work is at
//! most the octets of the credit times the octets of the rules. The credit
//! is capped here. The rules are the library administrator's (API-LIB-07),
//! and capping them belongs where they are stored.
//!
//! Artist merges and aliases from the curation log apply on top of this
//! from R1.3 (WP-107). Roles other than artist, album artist and featured
//! artist are resolved in R1.1 (WP-146).

use crate::catalog::{Credit, Role, TrackTags};
use crate::collate;
use crate::parse::{LimitKind, Limits};
use crate::text::{self, Lines};
use crate::untrusted::Untrusted;
use crate::values::Mbid;

/// What stands between the names of a multi-value field in the credit as
/// shown.
pub const JOIN: &str = ", ";

/// The separators of [`SplitRules::standard`].
const STANDARD: [&str; 7] = [", ", " & ", " feat. ", " ft. ", " x ", " / ", "; "];

/// How one library splits a display credit into names (API-LIB-07).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitRules {
    /// The texts that stand between two names, each matched as written.
    pub separators: Vec<String>,
    /// Names that hold a separator and are still one artist, each matched
    /// as written.
    pub exceptions: Vec<String>,
}

impl SplitRules {
    /// The rules of a library that has set none: the separators ", ",
    /// " & ", " feat. ", " ft. ", " x ", " / " and "; ", and no exception.
    #[must_use]
    pub fn standard() -> Self {
        Self {
            separators: STANDARD.map(String::from).into(),
            exceptions: Vec::new(),
        }
    }
}

/// The credits of one file: its recording's and its release's.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CreditSet {
    /// The recording's artists.
    pub artist: CreditLine,
    /// The release's artists.
    pub album_artist: CreditLine,
    /// What could not be kept or paired: first of the recording's artists,
    /// then of the release's. Of one credit, the limits it reached come
    /// first, the text limit before the name limit, then that it names
    /// nobody, then its unpaired identifiers.
    pub problems: Vec<Problem>,
}

/// One credit as shown and as linked. An untagged field gives an empty
/// one.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CreditLine {
    /// The credit as shown: a display credit exactly as tagged, or the
    /// names of a multi-value field joined with [`JOIN`]. It is the lead
    /// followed by each name and its join.
    pub display: String,
    /// What stands before the first name: separators and white space. A
    /// credit that links no name is all lead.
    pub lead: String,
    /// The credited names, in the order they are shown.
    pub names: Vec<CreditedName>,
}

/// One name of a credit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreditedName {
    /// The name without the white space around it, its role, and its
    /// `MusicBrainz` identifier when one was paired with it. The role is
    /// [`Role::Artist`] or [`Role::AlbumArtist`] after the field, or
    /// [`Role::Featured`].
    pub credit: Credit,
    /// What stands between this name and the next, or after the last
    /// name: separators and white space as tagged, or [`JOIN`]. When the
    /// credit links no more names, the rest of it is here too.
    pub join: String,
}

/// What tells one artist from another (MUS-006).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ArtistKey {
    /// The artist's `MusicBrainz` identifier.
    Mbid(Mbid),
    /// The name folded for matching ([`collate::fold`]), for a name without
    /// an identifier. A name of which folding leaves nothing, such as
    /// "!!!", is kept as written, without the white space around it.
    Name(String),
}

impl ArtistKey {
    /// The key of the artist `credit` names.
    #[must_use]
    pub fn of(credit: &Credit) -> Self {
        match credit.mbid() {
            Some(mbid) => Self::Mbid(mbid),
            None => Self::Name(matching(credit.name())),
        }
    }
}

/// What [`credits`] could not keep or pair. The role says which credit:
/// [`Role::Artist`] for the recording's, [`Role::AlbumArtist`] for the
/// release's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    /// A credit reached a limit (SEC-MED-006). At
    /// [`LimitKind::ShortText`], a display credit was cut, or the values
    /// of a multi-value field that no longer fit were left out. At
    /// [`LimitKind::TagFields`], the names past the limit were not linked,
    /// and those of a multi-value field were left out.
    Limit {
        /// Which credit.
        role: Role,
        /// The limit it reached.
        limit: LimitKind,
    },
    /// A tagged credit names nobody: it holds only separators and white
    /// space.
    NoName {
        /// Which credit.
        role: Role,
    },
    /// `MusicBrainz` artist identifiers that do not pair with the names by
    /// position, because there are more or fewer of them than names. No
    /// name of the credit was given one.
    IdsUnpaired {
        /// Which credit.
        role: Role,
        /// How many names the tags hold.
        names: usize,
        /// How many identifiers the tags hold.
        ids: usize,
    },
}

/// The credits of the file `tags` were read from, under its library's
/// `rules` and the `limits` its tags were mapped under.
#[must_use]
pub fn credits(tags: &TrackTags, rules: &SplitRules, limits: &Limits) -> CreditSet {
    let mut reader = Reader {
        rules,
        limits,
        problems: Vec::new(),
    };
    let ids = &tags.musicbrainz;
    let artist = reader.line(&tags.artist, &ids.artists, Role::Artist);
    let album_artist = reader.line(&tags.album_artist, &ids.album_artists, Role::AlbumArtist);
    CreditSet {
        artist,
        album_artist,
        problems: reader.problems,
    }
}

/// One name of a credit while it is read.
struct Piece {
    /// The name, without the white space around it. It is never blank.
    name: String,
    /// Whether a featuring separator stands before it.
    featured: bool,
    /// What stands after it, up to the next name.
    join: String,
}

/// A credit as read, before its names are linked.
struct Parts {
    /// What stands before the first name.
    lead: String,
    /// The names that are kept.
    pieces: Vec<Piece>,
    /// How many names the tags hold, kept or not.
    total: usize,
}

/// Reads the credits of one file, collecting what it cannot keep or pair.
struct Reader<'a> {
    /// The library's rules.
    rules: &'a SplitRules,
    /// The limits to read under.
    limits: &'a Limits,
    /// What could not be kept or paired so far.
    problems: Vec<Problem>,
}

impl Reader<'_> {
    /// One limit as a count. A limit wider than a count, which only a
    /// 32-bit target could have, is as good as none.
    fn limit(&self, kind: LimitKind) -> usize {
        usize::try_from(self.limits.get(kind)).unwrap_or(usize::MAX)
    }

    /// The credit of one field: `values` as tagged, with `ids` paired by
    /// position.
    fn line(&mut self, values: &[String], ids: &[Mbid], role: Role) -> CreditLine {
        let parts = match values {
            [only] => self.tagged(only, role),
            several => self.listed(several, role),
        };
        if parts.total == 0 && !values.is_empty() {
            self.problems.push(Problem::NoName { role });
        }
        let ids = self.paired(ids, parts.total, role);
        linked(parts, ids, role)
    }

    /// A display credit: `value` cut at [`LimitKind::ShortText`], split by
    /// the rules, and linked up to [`LimitKind::TagFields`] names.
    fn tagged(&mut self, value: &str, role: Role) -> Parts {
        // Every text limit's ceiling fits 32 bits.
        let cap = u32::try_from(self.limits.get(LimitKind::ShortText)).unwrap_or(u32::MAX);
        let shown = text::normalise(Untrusted::new(value.as_bytes()), Lines::Single, cap);
        if shown.truncated {
            let limit = LimitKind::ShortText;
            self.problems.push(Problem::Limit { role, limit });
        }
        let (mut lead, mut pieces) = split(&shown.value, self.rules);
        let total = pieces.len();
        let max = self.limit(LimitKind::TagFields);
        if total > max {
            let limit = LimitKind::TagFields;
            self.problems.push(Problem::Limit { role, limit });
        }
        let unlinked: String = pieces
            .iter()
            .skip(max)
            .flat_map(|piece| [piece.name.as_str(), piece.join.as_str()])
            .collect();
        pieces.truncate(max);
        glue(&mut lead, &mut pieces).push_str(&unlinked);
        Parts {
            lead,
            pieces,
            total,
        }
    }

    /// A multi-value field: each of `values` that is not blank is one name,
    /// for as long as the credit they make stays within
    /// [`LimitKind::ShortText`] and [`LimitKind::TagFields`] names.
    fn listed(&mut self, values: &[String], role: Role) -> Parts {
        let names: Vec<&str> = values
            .iter()
            .map(String::as_str)
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .collect();
        // Each name is counted with a join after it, so the room is one
        // join more than the limit: the last name has none.
        let room = self.limit(LimitKind::ShortText).saturating_add(JOIN.len());
        let mut octets = 0_usize;
        let fitting = names
            .iter()
            .take_while(|name| {
                octets = octets.saturating_add(name.len()).saturating_add(JOIN.len());
                octets <= room
            })
            .count();
        let max = self.limit(LimitKind::TagFields);
        let kept = fitting.min(max);
        if kept < names.len() {
            let limit = if max < fitting {
                LimitKind::TagFields
            } else {
                LimitKind::ShortText
            };
            self.problems.push(Problem::Limit { role, limit });
        }
        let mut pieces: Vec<Piece> = names
            .iter()
            .take(kept)
            .map(|name| Piece {
                name: (*name).to_owned(),
                featured: false,
                join: JOIN.to_owned(),
            })
            .collect();
        if let Some(last) = pieces.last_mut() {
            last.join.clear();
        }
        Parts {
            lead: String::new(),
            pieces,
            total: names.len(),
        }
    }

    /// `ids` when there is one for each of the `names` the tags hold, and
    /// none otherwise. Identifiers that are there and cannot be paired are
    /// recorded.
    fn paired<'a>(&mut self, ids: &'a [Mbid], names: usize, role: Role) -> &'a [Mbid] {
        if ids.len() == names {
            return ids;
        }
        if !ids.is_empty() {
            let ids = ids.len();
            self.problems
                .push(Problem::IdsUnpaired { role, names, ids });
        }
        &[]
    }
}

/// The credit as shown and as linked: each kept name with its role and,
/// when `ids` pair with the names, its identifier.
fn linked(parts: Parts, ids: &[Mbid], role: Role) -> CreditLine {
    let mut display = parts.lead.clone();
    let names = parts
        .pieces
        .into_iter()
        .enumerate()
        .filter_map(|(at, piece)| {
            display.push_str(&piece.name);
            display.push_str(&piece.join);
            let held = if piece.featured { Role::Featured } else { role };
            // A piece's name is never blank, so every piece is kept.
            Credit::new(piece.name, held, None, ids.get(at).copied())
                .ok()
                .map(|credit| CreditedName {
                    credit,
                    join: piece.join,
                })
        })
        .collect();
    CreditLine {
        display,
        lead: parts.lead,
        names,
    }
}

/// Splits the display credit `shown` by `rules` into what stands before
/// its first name and its names, each with what stands after it.
fn split(shown: &str, rules: &SplitRules) -> (String, Vec<Piece>) {
    let mut lead = String::new();
    let mut pieces: Vec<Piece> = Vec::new();
    let mut name = String::new();
    let mut featured = false;
    let mut resume = 0_usize;
    for (at, c) in shown.char_indices() {
        if at < resume {
            continue;
        }
        let rest = shown.get(at..).unwrap_or_default();
        if name.is_empty()
            && !c.is_whitespace()
            && let Some(whole) = exception_at(rest, rules)
        {
            name.push_str(whole);
            resume = at.saturating_add(whole.len());
        } else if let Some(separator) = separator_at(rest, &rules.separators) {
            close(&mut name, featured, &mut pieces);
            glue(&mut lead, &mut pieces).push_str(separator);
            featured = featured || featuring(separator);
            resume = at.saturating_add(separator.len());
        } else if name.is_empty() && c.is_whitespace() {
            glue(&mut lead, &mut pieces).push(c);
        } else {
            name.push(c);
        }
    }
    close(&mut name, featured, &mut pieces);
    (lead, pieces)
}

/// Ends the name being read, when there is one: the name without the white
/// space after it, which starts its join. A name being read never starts
/// with white space.
fn close(name: &mut String, featured: bool, pieces: &mut Vec<Piece>) {
    let kept = name.trim_end();
    if !kept.is_empty() {
        let join = name.get(kept.len()..).unwrap_or_default().to_owned();
        pieces.push(Piece {
            name: kept.to_owned(),
            featured,
            join,
        });
    }
    name.clear();
}

/// Where the text between names is kept: the join of the last name, or the
/// lead while there is no name yet.
fn glue<'a>(lead: &'a mut String, pieces: &'a mut [Piece]) -> &'a mut String {
    pieces.last_mut().map_or(lead, |last| &mut last.join)
}

/// The longest separator that `rest` starts with.
fn separator_at<'a>(rest: &str, separators: &'a [String]) -> Option<&'a str> {
    separators
        .iter()
        .map(String::as_str)
        .filter(|separator| !separator.is_empty() && rest.starts_with(*separator))
        .max_by_key(|separator| separator.len())
}

/// The longest exception that is the whole name `rest` starts with: after
/// it comes nothing but white space up to a separator or the end.
fn exception_at<'a>(rest: &str, rules: &'a SplitRules) -> Option<&'a str> {
    rules
        .exceptions
        .iter()
        .map(String::as_str)
        .filter(|whole| {
            !whole.is_empty()
                && rest
                    .strip_prefix(*whole)
                    .is_some_and(|after| ends_name(after, &rules.separators))
        })
        .max_by_key(|whole| whole.len())
}

/// Whether `after` holds nothing but white space up to its first separator
/// or its end.
fn ends_name(after: &str, separators: &[String]) -> bool {
    for (at, c) in after.char_indices() {
        let rest = after.get(at..).unwrap_or_default();
        if separator_at(rest, separators).is_some() {
            return true;
        }
        if !c.is_whitespace() {
            return false;
        }
    }
    true
}

/// Whether `separator` introduces featured artists: it reads "feat", "ft"
/// or "featuring" once folded.
fn featuring(separator: &str) -> bool {
    matches!(
        collate::fold(separator).as_str(),
        "feat" | "ft" | "featuring"
    )
}

/// `name` folded for matching, or, when folding leaves nothing of it, as it
/// is written without the white space around it.
fn matching(name: &str) -> String {
    let folded = collate::fold(name);
    if folded.is_empty() {
        name.trim().to_owned()
    } else {
        folded
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::MbIds;
    use crate::untrusted::Untrusted;
    use proptest::collection::vec;
    use proptest::prelude::*;

    /// A duo whose name holds a separator.
    const SIMON: &str = "Simon & Garfunkel";

    /// Three `MusicBrainz` identifiers, told apart by their last digit.
    const IDS: [&str; 3] = [
        "5b11f54e-8a37-11df-8f36-0025905a5711",
        "5b11f54e-8a37-11df-8f36-0025905a5712",
        "5b11f54e-8a37-11df-8f36-0025905a5713",
    ];

    /// The standard separators, written out independently of the code.
    const SEPARATORS: [&str; 7] = [", ", " & ", " feat. ", " ft. ", " x ", " / ", "; "];

    fn mbid(text: &str) -> Mbid {
        Mbid::parse(Untrusted::new(text)).unwrap()
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn split_by(separators: &[&str], exceptions: &[&str]) -> SplitRules {
        SplitRules {
            separators: strings(separators),
            exceptions: strings(exceptions),
        }
    }

    /// The standard separators with these exceptions.
    fn standard_but(exceptions: &[&str]) -> SplitRules {
        split_by(&SEPARATORS, exceptions)
    }

    fn lowered(overrides: &[(LimitKind, u64)]) -> Limits {
        overrides
            .iter()
            .fold(Limits::DEFAULT, |limits, &(kind, value)| {
                limits.with_override(kind, value).unwrap()
            })
    }

    /// Tags that hold these artists and artist identifiers and nothing
    /// else.
    fn artists(names: &[&str], ids: &[&str]) -> TrackTags {
        TrackTags {
            artist: strings(names),
            musicbrainz: MbIds {
                artists: ids.iter().copied().map(mbid).collect(),
                ..MbIds::default()
            },
            ..TrackTags::default()
        }
    }

    fn credited(name: &str, role: Role, id: Option<&str>, join: &str) -> CreditedName {
        CreditedName {
            credit: Credit::new(name.to_owned(), role, None, id.map(mbid)).unwrap(),
            join: join.to_owned(),
        }
    }

    /// A name of the recording's artists without an identifier.
    fn artist(name: &str, join: &str) -> CreditedName {
        credited(name, Role::Artist, None, join)
    }

    /// A featured name without an identifier.
    fn featured(name: &str, join: &str) -> CreditedName {
        credited(name, Role::Featured, None, join)
    }

    fn line(display: &str, lead: &str, names: Vec<CreditedName>) -> CreditLine {
        CreditLine {
            display: display.to_owned(),
            lead: lead.to_owned(),
            names,
        }
    }

    /// Credits with an artist credit, no album artist credit and these
    /// problems.
    fn artist_with(artist: CreditLine, problems: &[Problem]) -> CreditSet {
        CreditSet {
            artist,
            album_artist: CreditLine::default(),
            problems: problems.to_vec(),
        }
    }

    /// Credits with only an artist credit and nothing recorded.
    fn artist_only(artist: CreditLine) -> CreditSet {
        artist_with(artist, &[])
    }

    /// The problem of a credit that reached a limit.
    fn limit(role: Role, limit: LimitKind) -> Problem {
        Problem::Limit { role, limit }
    }

    /// Asserts that `value`, as the only artist under `rules` and the
    /// default limits, is shown as tagged, with `lead` before `names`, and
    /// that nothing is recorded.
    fn assert_split(rules: &SplitRules, value: &str, lead: &str, names: Vec<CreditedName>) {
        assert_eq!(
            credits(&artists(&[value], &[]), rules, &Limits::DEFAULT),
            artist_only(line(value, lead, names)),
            "{value:?}"
        );
    }

    /// The keys of the artists a credit names.
    fn keys(credit: &CreditLine) -> Vec<ArtistKey> {
        credit
            .names
            .iter()
            .map(|name| ArtistKey::of(&name.credit))
            .collect()
    }

    /// The lead, then each name followed by its join.
    fn read_back(credit: &CreditLine) -> String {
        let mut text = credit.lead.clone();
        for name in &credit.names {
            text.push_str(name.credit.name());
            text.push_str(&name.join);
        }
        text
    }

    /// Whether `text` is nothing but standard separators and white space,
    /// by trying every way to read it as that.
    fn is_glue(text: &str) -> bool {
        let mut rest = text.chars();
        let Some(first) = rest.next() else {
            return true;
        };
        (first.is_whitespace() && is_glue(rest.as_str()))
            || SEPARATORS
                .iter()
                .any(|separator| text.strip_prefix(*separator).is_some_and(is_glue))
    }

    /// `count` letters.
    fn letters(count: usize) -> String {
        (0..count).map(|_| 'a').collect()
    }

    #[test]
    fn the_standard_rules_are_seven_separators_and_no_exception() {
        assert_eq!(
            SplitRules::standard(),
            split_by(&[", ", " & ", " feat. ", " ft. ", " x ", " / ", "; "], &[])
        );
    }

    #[test]
    fn splits_simon_and_garfunkel_without_the_exception() {
        assert_split(
            &SplitRules::standard(),
            SIMON,
            "",
            vec![artist("Simon", " & "), artist("Garfunkel", "")],
        );
    }

    #[test]
    fn keeps_simon_and_garfunkel_whole_with_the_exception() {
        assert_split(&standard_but(&[SIMON]), SIMON, "", vec![artist(SIMON, "")]);
    }

    #[test]
    fn links_a_featured_artist_and_shows_the_credit_as_tagged() {
        let rules = SplitRules::standard();
        assert_split(
            &rules,
            "A feat. B",
            "",
            vec![artist("A", " feat. "), featured("B", "")],
        );
        assert_split(
            &rules,
            "Daft Punk feat. Pharrell Williams",
            "",
            vec![
                artist("Daft Punk", " feat. "),
                featured("Pharrell Williams", ""),
            ],
        );
    }

    #[test]
    fn splits_at_every_standard_separator() {
        assert_split(
            &SplitRules::standard(),
            "A, B & C feat. D ft. E x F / G; H",
            "",
            vec![
                artist("A", ", "),
                artist("B", " & "),
                artist("C", " feat. "),
                featured("D", " ft. "),
                featured("E", " x "),
                featured("F", " / "),
                featured("G", "; "),
                featured("H", ""),
            ],
        );
    }

    #[test]
    fn a_separator_that_folds_to_a_featuring_word_marks_the_names_after_it() {
        assert_split(
            &split_by(&[" vs. ", " FT ", " featuring "], &[]),
            "A vs. B FT C featuring D",
            "",
            vec![
                artist("A", " vs. "),
                artist("B", " FT "),
                featured("C", " featuring "),
                featured("D", ""),
            ],
        );
    }

    #[test]
    fn the_album_artists_are_a_credit_of_their_own() {
        let tags = TrackTags {
            artist: strings(&["A feat. B"]),
            album_artist: strings(&["A & C feat. D"]),
            musicbrainz: MbIds {
                album_artists: IDS.iter().copied().map(mbid).collect(),
                ..MbIds::default()
            },
            ..TrackTags::default()
        };
        assert_eq!(
            credits(&tags, &SplitRules::standard(), &Limits::DEFAULT),
            CreditSet {
                artist: line(
                    "A feat. B",
                    "",
                    vec![artist("A", " feat. "), featured("B", "")]
                ),
                album_artist: line(
                    "A & C feat. D",
                    "",
                    vec![
                        credited("A", Role::AlbumArtist, Some(IDS[0]), " & "),
                        credited("C", Role::AlbumArtist, Some(IDS[1]), " feat. "),
                        credited("D", Role::Featured, Some(IDS[2]), ""),
                    ]
                ),
                problems: Vec::new(),
            }
        );
    }

    #[test]
    fn pairs_three_identifiers_with_three_artists_by_position() {
        let rules = SplitRules::standard();
        assert_eq!(
            credits(&artists(&["A", "B", "C"], &IDS), &rules, &Limits::DEFAULT),
            artist_only(line(
                "A, B, C",
                "",
                vec![
                    credited("A", Role::Artist, Some(IDS[0]), ", "),
                    credited("B", Role::Artist, Some(IDS[1]), ", "),
                    credited("C", Role::Artist, Some(IDS[2]), ""),
                ]
            ))
        );
        assert_eq!(
            credits(&artists(&["A, B & C"], &IDS), &rules, &Limits::DEFAULT),
            artist_only(line(
                "A, B & C",
                "",
                vec![
                    credited("A", Role::Artist, Some(IDS[0]), ", "),
                    credited("B", Role::Artist, Some(IDS[1]), " & "),
                    credited("C", Role::Artist, Some(IDS[2]), ""),
                ]
            ))
        );
    }

    #[test]
    fn does_not_pair_two_identifiers_with_three_names_and_records_it() {
        let unpaired = Problem::IdsUnpaired {
            role: Role::Artist,
            names: 3,
            ids: 2,
        };
        assert_eq!(
            credits(
                &artists(&["A", "B", "C"], &IDS[..2]),
                &SplitRules::standard(),
                &Limits::DEFAULT
            ),
            artist_with(
                line(
                    "A, B, C",
                    "",
                    vec![artist("A", ", "), artist("B", ", "), artist("C", "")]
                ),
                &[unpaired]
            )
        );
    }

    #[test]
    fn records_identifiers_that_have_too_few_names_or_none() {
        let rules = SplitRules::standard();
        assert_eq!(
            credits(&artists(&[], &IDS[..1]), &rules, &Limits::DEFAULT),
            artist_with(
                CreditLine::default(),
                &[Problem::IdsUnpaired {
                    role: Role::Artist,
                    names: 0,
                    ids: 1,
                }]
            )
        );
        assert_eq!(
            credits(&artists(&["A"], &IDS[..2]), &rules, &Limits::DEFAULT),
            artist_with(
                line("A", "", vec![artist("A", "")]),
                &[Problem::IdsUnpaired {
                    role: Role::Artist,
                    names: 1,
                    ids: 2,
                }]
            )
        );
        let tags = TrackTags {
            album_artist: strings(&["A"]),
            musicbrainz: MbIds {
                album_artists: vec![mbid(IDS[0]), mbid(IDS[1])],
                ..MbIds::default()
            },
            ..TrackTags::default()
        };
        assert_eq!(
            credits(&tags, &rules, &Limits::DEFAULT),
            CreditSet {
                artist: CreditLine::default(),
                album_artist: line("A", "", vec![credited("A", Role::AlbumArtist, None, "")]),
                problems: vec![Problem::IdsUnpaired {
                    role: Role::AlbumArtist,
                    names: 1,
                    ids: 2,
                }],
            }
        );
    }

    #[test]
    fn keeps_two_artists_of_one_name_apart_when_their_identifiers_differ() {
        let rules = SplitRules::standard();
        let names = ["Nirvana", "Nirvana"];
        let apart = credits(&artists(&names, &IDS[..2]), &rules, &Limits::DEFAULT);
        assert_eq!(
            apart,
            artist_only(line(
                "Nirvana, Nirvana",
                "",
                vec![
                    credited("Nirvana", Role::Artist, Some(IDS[0]), ", "),
                    credited("Nirvana", Role::Artist, Some(IDS[1]), ""),
                ]
            ))
        );
        assert_eq!(
            keys(&apart.artist),
            [ArtistKey::Mbid(mbid(IDS[0])), ArtistKey::Mbid(mbid(IDS[1]))]
        );
        let same = credits(
            &artists(&names, &[IDS[0], IDS[0]]),
            &rules,
            &Limits::DEFAULT,
        );
        assert_eq!(
            keys(&same.artist),
            [ArtistKey::Mbid(mbid(IDS[0])), ArtistKey::Mbid(mbid(IDS[0]))]
        );
    }

    #[test]
    fn names_without_identifiers_share_a_key_and_are_never_merged() {
        let untagged = credits(
            &artists(&["Nirvana & NIRVANA"], &[]),
            &SplitRules::standard(),
            &Limits::DEFAULT,
        );
        assert_eq!(
            untagged,
            artist_only(line(
                "Nirvana & NIRVANA",
                "",
                vec![artist("Nirvana", " & "), artist("NIRVANA", "")]
            ))
        );
        assert_eq!(
            keys(&untagged.artist),
            [
                ArtistKey::Name(String::from("nirvana")),
                ArtistKey::Name(String::from("nirvana"))
            ]
        );
    }

    #[test]
    fn a_key_is_the_identifier_or_else_the_folded_name() {
        let key = |name: &str, id: Option<&str>| {
            ArtistKey::of(
                &Credit::new(name.to_owned(), Role::Composer, None, id.map(mbid)).unwrap(),
            )
        };
        assert_eq!(
            key("Bj\u{f6}rk", Some(IDS[0])),
            ArtistKey::Mbid(mbid(IDS[0]))
        );
        assert_eq!(
            key("Bj\u{f6}rk", None),
            ArtistKey::Name(String::from("bjork"))
        );
        assert_eq!(key("BJORK", None), ArtistKey::Name(String::from("bjork")));
        assert_eq!(key("AC/DC", None), ArtistKey::Name(String::from("acdc")));
        // Folding leaves nothing of these, so they are kept as written.
        assert_eq!(key("!!!", None), ArtistKey::Name(String::from("!!!")));
        assert_eq!(key(" !!! ", None), ArtistKey::Name(String::from("!!!")));
        assert_eq!(key("\u{f7}", None), ArtistKey::Name(String::from("\u{f7}")));
    }

    #[test]
    fn a_credit_of_only_separators_names_nobody() {
        let rules = SplitRules::standard();
        let nobody = [Problem::NoName { role: Role::Artist }];
        for value in [" & ", " /  & ", "; , ", "  "] {
            assert_eq!(
                credits(&artists(&[value], &[]), &rules, &Limits::DEFAULT),
                artist_with(line(value, value, Vec::new()), &nobody),
                "{value:?}"
            );
        }
        assert_eq!(
            credits(&artists(&["", "  "], &[]), &rules, &Limits::DEFAULT),
            artist_with(CreditLine::default(), &nobody)
        );
    }

    #[test]
    fn several_values_are_never_split() {
        let rules = standard_but(&[SIMON]);
        assert_eq!(
            credits(
                &artists(&[SIMON, "Earth, Wind & Fire", "A feat. B"], &[]),
                &rules,
                &Limits::DEFAULT
            ),
            artist_only(line(
                "Simon & Garfunkel, Earth, Wind & Fire, A feat. B",
                "",
                vec![
                    artist(SIMON, ", "),
                    artist("Earth, Wind & Fire", ", "),
                    artist("A feat. B", ""),
                ]
            ))
        );
    }

    #[test]
    fn several_values_lose_their_blanks_and_the_white_space_around_them() {
        assert_eq!(
            credits(
                &artists(&[" A ", "", "B  ", "  "], &[]),
                &SplitRules::standard(),
                &Limits::DEFAULT
            ),
            artist_only(line("A, B", "", vec![artist("A", ", "), artist("B", "")]))
        );
    }

    #[test]
    fn white_space_around_a_name_belongs_to_the_lead_and_the_joins() {
        let rules = SplitRules::standard();
        let tagged = "  A  ,  B  ";
        assert_split(
            &rules,
            tagged,
            "  ",
            vec![artist("A", "  ,  "), artist("B", "  ")],
        );
        let set = credits(&artists(&[tagged], &[]), &rules, &Limits::DEFAULT);
        assert_eq!(read_back(&set.artist), tagged);
    }

    #[test]
    fn separators_at_either_end_and_side_by_side_stay_in_the_lead_and_the_joins() {
        let rules = SplitRules::standard();
        assert_split(&rules, "; A / ", "; ", vec![artist("A", " / ")]);
        assert_split(
            &rules,
            "A, , B",
            "",
            vec![artist("A", ", , "), artist("B", "")],
        );
        assert_split(&rules, "A feat. ", "", vec![artist("A", " feat. ")]);
    }

    #[test]
    fn an_exception_holds_only_as_a_whole_name() {
        let rules = standard_but(&[SIMON]);
        assert_split(
            &rules,
            "Simon & Garfunkel Jr.",
            "",
            vec![artist("Simon", " & "), artist("Garfunkel Jr.", "")],
        );
        assert_split(
            &rules,
            "Paul Simon & Garfunkel",
            "",
            vec![artist("Paul Simon", " & "), artist("Garfunkel", "")],
        );
    }

    #[test]
    fn an_exception_holds_between_separators_and_before_white_space() {
        let rules = standard_but(&[SIMON]);
        assert_split(
            &rules,
            "A, Simon & Garfunkel feat. B",
            "",
            vec![
                artist("A", ", "),
                artist(SIMON, " feat. "),
                featured("B", ""),
            ],
        );
        assert_split(
            &rules,
            "A feat. Simon & Garfunkel",
            "",
            vec![artist("A", " feat. "), featured(SIMON, "")],
        );
        assert_split(&rules, "Simon & Garfunkel  ", "", vec![artist(SIMON, "  ")]);
        assert_split(
            &rules,
            "Simon & Garfunkel  / B",
            "",
            vec![artist(SIMON, "  / "), artist("B", "")],
        );
        assert_split(&rules, " Simon & Garfunkel", " ", vec![artist(SIMON, "")]);
    }

    #[test]
    fn the_longest_exception_that_holds_is_taken() {
        for exceptions in [["A & B", "A & B & C"], ["A & B & C", "A & B"]] {
            let rules = standard_but(&exceptions);
            assert_split(&rules, "A & B & C", "", vec![artist("A & B & C", "")]);
            assert_split(
                &rules,
                "A & B & D",
                "",
                vec![artist("A & B", " & "), artist("D", "")],
            );
        }
    }

    #[test]
    fn an_exception_can_end_at_a_separator_without_white_space() {
        assert_split(
            &split_by(&["/"], &["AC/DC"]),
            "AC/DC/Metallica",
            "",
            vec![artist("AC/DC", "/"), artist("Metallica", "")],
        );
        assert_split(
            &split_by(&["/"], &[]),
            "AC/DC/Metallica",
            "",
            vec![
                artist("AC", "/"),
                artist("DC", "/"),
                artist("Metallica", ""),
            ],
        );
        assert_split(
            &SplitRules::standard(),
            "AC/DC",
            "",
            vec![artist("AC/DC", "")],
        );
    }

    #[test]
    fn an_empty_exception_matches_nothing() {
        assert_split(
            &split_by(&[";"], &[""]),
            "A;;B",
            "",
            vec![artist("A", ";;"), artist("B", "")],
        );
    }

    #[test]
    fn the_longest_separator_at_a_place_is_taken() {
        for separators in [[", ", ", and "], [", and ", ", "]] {
            assert_split(
                &split_by(&separators, &[]),
                "A, and B",
                "",
                vec![artist("A", ", and "), artist("B", "")],
            );
        }
    }

    #[test]
    fn an_empty_separator_matches_nothing() {
        assert_split(&split_by(&[""], &[]), "A B", "", vec![artist("A B", "")]);
        assert_split(
            &split_by(&["", ", "], &[]),
            "A, B",
            "",
            vec![artist("A", ", "), artist("B", "")],
        );
    }

    #[test]
    fn separators_and_exceptions_are_matched_as_written() {
        let rules = standard_but(&[SIMON]);
        assert_split(&rules, "A Feat. B", "", vec![artist("A Feat. B", "")]);
        assert_split(
            &rules,
            "Lil Nas X feat. B",
            "",
            vec![artist("Lil Nas X", " feat. "), featured("B", "")],
        );
        assert_split(
            &rules,
            "SIMON & GARFUNKEL",
            "",
            vec![artist("SIMON", " & "), artist("GARFUNKEL", "")],
        );
    }

    #[test]
    fn without_rules_nothing_is_split() {
        assert_split(
            &split_by(&[], &[]),
            "A, B & C",
            "",
            vec![artist("A, B & C", "")],
        );
    }

    #[test]
    fn untagged_fields_give_empty_credits() {
        assert_eq!(
            credits(
                &TrackTags::default(),
                &SplitRules::standard(),
                &Limits::DEFAULT
            ),
            CreditSet::default()
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn cuts_a_display_credit_at_the_short_text_limit() {
        let rules = SplitRules::standard();
        let limits = lowered(&[(LimitKind::ShortText, 5)]);
        let cut = [limit(Role::Artist, LimitKind::ShortText)];
        let whole = line("A & B", "", vec![artist("A", " & "), artist("B", "")]);
        assert_eq!(
            credits(&artists(&["A & B"], &[]), &rules, &limits),
            artist_only(whole.clone())
        );
        assert_eq!(
            credits(&artists(&["A & Bc"], &[]), &rules, &limits),
            artist_with(whole, &cut)
        );
        // A character that does not fit whole is not kept in part.
        assert_eq!(
            credits(&artists(&["A & \u{e9}"], &[]), &rules, &limits),
            artist_with(line("A & ", "", vec![artist("A", " & ")]), &cut)
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn cuts_a_display_credit_at_the_default_short_text_limit() {
        let rules = SplitRules::standard();
        let at_limit = letters(4_096);
        let whole = line(&at_limit, "", vec![artist(&at_limit, "")]);
        assert_eq!(
            credits(
                &artists(&[at_limit.as_str()], &[]),
                &rules,
                &Limits::DEFAULT
            ),
            artist_only(whole.clone())
        );
        let past = letters(4_097);
        assert_eq!(
            credits(&artists(&[past.as_str()], &[]), &rules, &Limits::DEFAULT),
            artist_with(whole, &[limit(Role::Artist, LimitKind::ShortText)])
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn links_names_up_to_the_tag_field_limit_and_shows_the_rest_unlinked() {
        let rules = SplitRules::standard();
        let full = [limit(Role::Artist, LimitKind::TagFields)];
        let two = lowered(&[(LimitKind::TagFields, 2)]);
        assert_eq!(
            credits(&artists(&["A, B"], &[]), &rules, &two),
            artist_only(line("A, B", "", vec![artist("A", ", "), artist("B", "")]))
        );
        assert_eq!(
            credits(&artists(&["A, B, C; D"], &[]), &rules, &two),
            artist_with(
                line(
                    "A, B, C; D",
                    "",
                    vec![artist("A", ", "), artist("B", ", C; D")]
                ),
                &full
            )
        );
        let none = lowered(&[(LimitKind::TagFields, 0)]);
        assert_eq!(
            credits(&artists(&["A, B"], &[]), &rules, &none),
            artist_with(line("A, B", "A, B", Vec::new()), &full)
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn ends_a_multi_value_credit_at_the_tag_field_limit() {
        let rules = SplitRules::standard();
        let full = [limit(Role::Artist, LimitKind::TagFields)];
        let two = lowered(&[(LimitKind::TagFields, 2)]);
        let both = line("A, B", "", vec![artist("A", ", "), artist("B", "")]);
        assert_eq!(
            credits(&artists(&["A", "B"], &[]), &rules, &two),
            artist_only(both.clone())
        );
        assert_eq!(
            credits(&artists(&["A", "B", "C"], &[]), &rules, &two),
            artist_with(both, &full)
        );
        let none = lowered(&[(LimitKind::TagFields, 0)]);
        assert_eq!(
            credits(&artists(&["A", "B"], &[]), &rules, &none),
            artist_with(CreditLine::default(), &full)
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn ends_a_multi_value_credit_at_the_default_tag_field_limit() {
        let rules = SplitRules::standard();
        // The text limit at its ceiling, so that 4,097 names fit it.
        let limits = lowered(&[(LimitKind::ShortText, 16_384)]);
        let at_limit: Vec<&str> = (0..4_096).map(|_| "a").collect();
        let mut names: Vec<CreditedName> = (0..4_096).map(|_| artist("a", ", ")).collect();
        names.last_mut().unwrap().join.clear();
        let all = line(&at_limit.join(", "), "", names);
        assert_eq!(all.display.len(), 12_286);
        assert_eq!(
            credits(&artists(&at_limit, &[]), &rules, &limits),
            artist_only(all.clone())
        );
        let past: Vec<&str> = (0..4_097).map(|_| "a").collect();
        assert_eq!(
            credits(&artists(&past, &[]), &rules, &limits),
            artist_with(all, &[limit(Role::Artist, LimitKind::TagFields)])
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn ends_a_multi_value_credit_before_the_name_that_passes_the_short_text_limit() {
        let rules = SplitRules::standard();
        let limits = lowered(&[(LimitKind::ShortText, 4)]);
        let cut = [limit(Role::Artist, LimitKind::ShortText)];
        assert_eq!(
            credits(&artists(&["A", "B"], &[]), &rules, &limits),
            artist_only(line("A, B", "", vec![artist("A", ", "), artist("B", "")]))
        );
        assert_eq!(
            credits(&artists(&["A", "Bc", "D"], &[]), &rules, &limits),
            artist_with(line("A", "", vec![artist("A", "")]), &cut)
        );
        assert_eq!(
            credits(&artists(&["Abcde", "B"], &[]), &rules, &limits),
            artist_with(CreditLine::default(), &cut)
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn records_the_text_limit_when_one_name_passes_both_limits() {
        let limits = lowered(&[(LimitKind::ShortText, 1), (LimitKind::TagFields, 1)]);
        assert_eq!(
            credits(&artists(&["A", "B"], &[]), &SplitRules::standard(), &limits),
            artist_with(
                line("A", "", vec![artist("A", "")]),
                &[limit(Role::Artist, LimitKind::ShortText)]
            )
        );
    }

    #[test]
    fn identifiers_pair_with_the_names_tagged_not_the_names_kept() {
        let rules = SplitRules::standard();
        let limits = lowered(&[(LimitKind::TagFields, 2)]);
        let full = limit(Role::Artist, LimitKind::TagFields);
        assert_eq!(
            credits(&artists(&["A, B, C"], &IDS), &rules, &limits),
            artist_with(
                line(
                    "A, B, C",
                    "",
                    vec![
                        credited("A", Role::Artist, Some(IDS[0]), ", "),
                        credited("B", Role::Artist, Some(IDS[1]), ", C"),
                    ]
                ),
                &[full]
            )
        );
        assert_eq!(
            credits(&artists(&["A", "B", "C"], &IDS), &rules, &limits),
            artist_with(
                line(
                    "A, B",
                    "",
                    vec![
                        credited("A", Role::Artist, Some(IDS[0]), ", "),
                        credited("B", Role::Artist, Some(IDS[1]), ""),
                    ]
                ),
                &[full]
            )
        );
        // Two identifiers are not one each for three names, though two
        // names are kept.
        assert_eq!(
            credits(&artists(&["A, B, C"], &IDS[..2]), &rules, &limits),
            artist_with(
                line("A, B, C", "", vec![artist("A", ", "), artist("B", ", C")]),
                &[
                    full,
                    Problem::IdsUnpaired {
                        role: Role::Artist,
                        names: 3,
                        ids: 2,
                    }
                ]
            )
        );
    }

    #[test]
    fn problems_are_listed_credit_by_credit_in_the_order_they_are_found() {
        let tags = TrackTags {
            artist: strings(&["A & Bc"]),
            album_artist: strings(&[" & "]),
            musicbrainz: MbIds {
                artists: vec![mbid(IDS[0])],
                album_artists: vec![mbid(IDS[1])],
                ..MbIds::default()
            },
            ..TrackTags::default()
        };
        let limits = lowered(&[(LimitKind::ShortText, 5), (LimitKind::TagFields, 1)]);
        assert_eq!(
            credits(&tags, &SplitRules::standard(), &limits),
            CreditSet {
                artist: line("A & B", "", vec![artist("A", " & B")]),
                album_artist: line(" & ", " & ", Vec::new()),
                problems: vec![
                    limit(Role::Artist, LimitKind::ShortText),
                    limit(Role::Artist, LimitKind::TagFields),
                    Problem::IdsUnpaired {
                        role: Role::Artist,
                        names: 2,
                        ids: 1,
                    },
                    Problem::NoName {
                        role: Role::AlbumArtist,
                    },
                    Problem::IdsUnpaired {
                        role: Role::AlbumArtist,
                        names: 0,
                        ids: 1,
                    },
                ],
            }
        );
    }

    #[test]
    fn the_glue_oracle_tells_separators_and_white_space_from_names() {
        for glue in ["", " ", " ; ", " feat.  x ", ",  & \u{3000}"] {
            assert!(is_glue(glue), "{glue:?}");
        }
        for text in ["a", " , a", "feat.", " x", ","] {
            assert!(!is_glue(text), "{text:?}");
        }
    }

    /// A piece of a credit: a letter or two, white space, a separator, or
    /// a part of one.
    fn any_token() -> impl Strategy<Value = &'static str> {
        prop::sample::select(vec![
            "A", "b", "Cd", "\u{e9}", " ", "  ", ",", ";", "&", "/", ".", "x", "feat", "ft", ", ",
            " & ", " feat. ", " ft. ", " x ", " / ", "; ",
        ])
    }

    /// A separator or an exception an administrator might set, the empty
    /// one among them.
    fn any_rule() -> impl Strategy<Value = String> {
        vec(any_token(), 0..3).prop_map(|tokens| tokens.concat())
    }

    fn any_separator() -> impl Strategy<Value = &'static str> {
        prop::sample::select(SEPARATORS.to_vec())
    }

    fn any_space() -> impl Strategy<Value = &'static str> {
        prop::sample::select(vec![" ", "  ", "\u{a0}", "\u{3000}"])
    }

    /// A name the standard separators leave whole: one or two words of two
    /// or more letters, so that no word is the "x" of " x ".
    fn any_name() -> impl Strategy<Value = String> {
        "[A-Za-z\u{e9}\u{6771}]{2,4}( [A-Za-z\u{e9}\u{6771}]{2,4})?"
    }

    /// What stands after a name: white space or none, a separator, and then
    /// more of either.
    fn any_gap() -> impl Strategy<Value = Vec<&'static str>> {
        (
            vec(any_space(), 0..2),
            any_separator(),
            vec(prop_oneof![any_space(), any_separator()], 0..3),
        )
            .prop_map(|(before, separator, after)| {
                let mut tokens = before;
                tokens.push(separator);
                tokens.extend(after);
                tokens
            })
    }

    /// Whether a standard separator introduces featured artists, written
    /// out independently of the code.
    fn introduces_guests(separator: &str) -> bool {
        separator == " feat. " || separator == " ft. "
    }

    proptest! {
        /// Joining the credited names never loses a character of the
        /// display credit except separators and the white space around
        /// the names (MUS-002).
        #[test]
        fn a_split_credit_reads_back_whole_and_loses_only_separators(
            tokens in vec(any_token(), 0..12),
        ) {
            let shown = tokens.concat();
            let set = credits(
                &artists(&[shown.as_str()], &[]),
                &SplitRules::standard(),
                &Limits::DEFAULT,
            );
            prop_assert_eq!(&set.artist.display, &shown);
            prop_assert_eq!(read_back(&set.artist), shown);
            prop_assert!(is_glue(&set.artist.lead), "{:?}", set.artist.lead);
            for name in &set.artist.names {
                prop_assert!(is_glue(&name.join), "{:?}", name.join);
                let written = name.credit.name();
                prop_assert_eq!(written.trim(), written);
                let holds_no_separator = SEPARATORS
                    .iter()
                    .all(|separator| !written.contains(*separator));
                prop_assert!(holds_no_separator, "{:?}", written);
            }
        }

        /// Names with separators and white space between them come back as
        /// they went in: each name, its role, and what stands around it.
        #[test]
        fn names_between_separators_are_linked_with_what_stands_around_them(
            lead in vec(prop_oneof![any_space(), any_separator()], 0..3),
            pairs in vec((any_name(), any_gap()), 0..4),
        ) {
            let mut shown = lead.concat();
            let mut guests = lead.iter().copied().any(introduces_guests);
            let mut names = Vec::new();
            for (name, gap) in &pairs {
                let role = if guests { Role::Featured } else { Role::Artist };
                let join = gap.concat();
                shown.push_str(name);
                shown.push_str(&join);
                names.push(credited(name, role, None, &join));
                guests = guests || gap.iter().copied().any(introduces_guests);
            }
            let expected = line(&shown, &lead.concat(), names);
            let set = credits(
                &artists(&[shown.as_str()], &[]),
                &SplitRules::standard(),
                &Limits::DEFAULT,
            );
            prop_assert_eq!(set.artist, expected);
        }

        /// Each value of a multi-value field is one name, whatever it
        /// holds.
        #[test]
        fn several_values_are_one_name_each_whatever_they_hold(
            first in "[A-Z]{1,3}",
            others in vec(
                prop_oneof![3 => "[ABab ,;&/.x]{0,6}", 1 => Just(String::new())],
                1..4,
            ),
        ) {
            let mut values = vec![first.as_str()];
            values.extend(others.iter().map(String::as_str));
            let names: Vec<&str> = values
                .iter()
                .copied()
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .collect();
            let mut expected: Vec<CreditedName> =
                names.iter().copied().map(|name| artist(name, ", ")).collect();
            // The first value is never blank, so there is a last name.
            expected.last_mut().unwrap().join.clear();
            let set = credits(
                &artists(&values, &[]),
                &SplitRules::standard(),
                &Limits::DEFAULT,
            );
            prop_assert_eq!(set, artist_only(line(&names.join(", "), "", expected)));
        }

        /// Whatever the rules and the credit, the credit reads back whole,
        /// no name has white space around it, and no more names are linked
        /// than the limit allows.
        #[test]
        fn any_rules_and_any_credit_read_back_whole_within_the_name_limit(
            tokens in vec(any_token(), 0..12),
            separators in vec(any_rule(), 0..4),
            exceptions in vec(any_rule(), 0..3),
            max in 0_u64..4,
        ) {
            let shown = tokens.concat();
            let rules = SplitRules { separators, exceptions };
            let limits = lowered(&[(LimitKind::TagFields, max)]);
            let set = credits(&artists(&[shown.as_str()], &[]), &rules, &limits);
            prop_assert_eq!(&set.artist.display, &shown);
            prop_assert_eq!(read_back(&set.artist), shown);
            prop_assert!(set.artist.names.len() <= usize::try_from(max).unwrap());
            for name in &set.artist.names {
                let written = name.credit.name();
                prop_assert_eq!(written.trim(), written);
            }
        }
    }
}
