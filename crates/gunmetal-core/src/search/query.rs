//! Answering a query (DIS-083, DIS-085).
//!
//! Only the first [`MAX_QUERY_CHARS`] characters of a query are read, and
//! only the first [`MAX_TERMS`] tokens of those (SEC-API-063). A document
//! is a hit when every term matches one of its tokens: as the whole token,
//! as its beginning, or, for a term of [`MIN_TYPO_CHARS`] characters or
//! more, within one edit (a letter missing, added or wrong, or two
//! neighbours swapped). The comparison is of folded text only; no
//! character of a query has a special meaning (SEC-STD-011).
//!
//! How well a document matched is its [`Match`], which depends on that
//! document alone. Hits are ranked by their match, then by the person's
//! own plays, then by the order the documents were indexed in. They come
//! back grouped by type, each group in rank order and holding at most
//! `limit` hits, with the group that holds the best hit first, so the best
//! hit of all is always the first.
//!
//! One query costs one pass over the vocabulary for each term. Whether
//! that meets the DIS-019 budget at 100,000 tracks is measured by the
//! benchmark runner (WP-115), not here.

use std::cmp::Reverse;
use std::collections::BTreeMap;

use super::doc::{DocKind, DocRef};
use super::index::{Index, tokens};

/// The most characters of a query that are read (SEC-API-063).
pub const MAX_QUERY_CHARS: usize = 256;

/// The most terms of a query that are used (SEC-API-063).
pub const MAX_TERMS: u8 = 16;

/// The fewest characters a term needs before a token one edit away
/// matches it.
pub const MIN_TYPO_CHARS: usize = 4;

/// Which types of document a query returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KindFilter {
    /// Every type.
    All,
    /// One type only.
    Only(DocKind),
}

impl KindFilter {
    /// Whether a document of kind `kind` passes.
    fn admits(self, kind: DocKind) -> bool {
        match self {
            Self::All => true,
            Self::Only(only) => only == kind,
        }
    }
}

/// How well a document matched a query, best first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Match {
    /// The query is the whole title.
    WholeTitle,
    /// The title begins with the query.
    TitleStart,
    /// Every term is a whole word of the document.
    Words,
    /// Every term is a whole word or the beginning of one.
    WordStarts,
    /// At least one term matched only within one edit.
    NearMiss,
}

/// One document a query found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hit {
    /// Which thing it is.
    pub doc: DocRef,
    /// How well it matched.
    pub matched: Match,
}

/// How one term fits one token, or a title position; worst first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Fit {
    /// Not at all.
    None,
    /// Within one edit.
    Near,
    /// The token begins with the term.
    Start,
    /// The token is the term.
    Whole,
}

/// How one term fits one document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TermHit {
    /// Its best fit to any token of the document.
    best: Fit,
    /// Its best fit to the title token at the term's own position.
    placed: Fit,
}

/// How the terms read so far fit one document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Tally {
    /// The worst of the terms' best fits.
    worst: Fit,
    /// [`Fit::Whole`] while every term so far is the title token at its
    /// own position; [`Fit::Start`] when the last one only begins it;
    /// anything else once the title and the query have parted.
    lead: Fit,
}

impl Tally {
    /// Before any term.
    const START: Self = Self {
        worst: Fit::Whole,
        lead: Fit::Whole,
    };

    /// This tally with one more term.
    fn with(self, hit: TermHit) -> Self {
        Self {
            worst: self.worst.min(hit.best),
            lead: if self.lead == Fit::Whole {
                hit.placed
            } else {
                Fit::None
            },
        }
    }

    /// The match of a document whose title has `title_len` tokens, to a
    /// query of `terms` terms.
    fn verdict(self, title_len: u8, terms: usize) -> Match {
        match (self.lead, self.worst) {
            (Fit::Whole, _) if usize::from(title_len) == terms => Match::WholeTitle,
            (Fit::Whole | Fit::Start, _) => Match::TitleStart,
            (_, Fit::Whole) => Match::Words,
            (_, Fit::Start) => Match::WordStarts,
            (_, Fit::Near | Fit::None) => Match::NearMiss,
        }
    }
}

impl Index {
    /// The documents that match `q`, of the kinds `kind` lets through,
    /// with at most `limit` of each kind.
    ///
    /// A query with no letter or digit in its first [`MAX_QUERY_CHARS`]
    /// characters finds nothing.
    #[must_use]
    pub fn query(&self, q: &str, kind: KindFilter, limit: u16) -> Vec<Hit> {
        let read: String = q.chars().take(MAX_QUERY_CHARS).collect();
        let terms = tokens(&read);
        let mut tallies: BTreeMap<u32, Tally> = BTreeMap::new();
        let used = terms.len().min(usize::from(MAX_TERMS));
        for (place, term) in (0..MAX_TERMS).zip(&terms) {
            tallies = self
                .term_hits(term, place)
                .into_iter()
                .filter_map(|(doc, hit)| {
                    let before = if place == 0 {
                        Tally::START
                    } else {
                        *tallies.get(&doc)?
                    };
                    Some((doc, before.with(hit)))
                })
                .collect();
        }
        let mut ranked: Vec<(Match, Reverse<u32>, u32, DocRef)> = tallies
            .into_iter()
            .filter_map(|(ordinal, tally)| {
                usize::try_from(ordinal)
                    .ok()
                    .and_then(|at| self.docs.get(at))
                    .filter(|entry| kind.admits(entry.doc.kind()))
                    .map(|entry| {
                        (
                            tally.verdict(entry.title_len, used),
                            Reverse(entry.plays),
                            ordinal,
                            entry.doc,
                        )
                    })
            })
            .collect();
        ranked.sort_unstable_by_key(|&(matched, plays, ordinal, _)| (matched, plays, ordinal));
        let mut kinds: Vec<DocKind> = Vec::new();
        for (_, _, _, doc) in &ranked {
            if !kinds.contains(&doc.kind()) {
                kinds.push(doc.kind());
            }
        }
        kinds
            .iter()
            .flat_map(|kind| {
                ranked
                    .iter()
                    .filter(move |(_, _, _, doc)| doc.kind() == *kind)
                    .take(usize::from(limit))
            })
            .map(|&(matched, _, _, doc)| Hit { doc, matched })
            .collect()
    }

    /// How `term`, the query's term number `place`, fits every document
    /// it fits at all.
    fn term_hits(&self, term: &str, place: u8) -> BTreeMap<u32, TermHit> {
        let fuzzy = term.chars().count() >= MIN_TYPO_CHARS;
        let mut hits: BTreeMap<u32, TermHit> = BTreeMap::new();
        for (token, postings) in &self.terms {
            let fit = fit(term, token, fuzzy);
            if fit == Fit::None {
                continue;
            }
            for posting in postings {
                let hit = hits.entry(posting.doc).or_insert(TermHit {
                    best: Fit::None,
                    placed: Fit::None,
                });
                hit.best = hit.best.max(fit);
                if posting.place == place {
                    hit.placed = hit.placed.max(fit);
                }
            }
        }
        hits
    }
}

/// How `term` fits `token`. A near miss counts only when `fuzzy`.
fn fit(term: &str, token: &str, fuzzy: bool) -> Fit {
    if token == term {
        Fit::Whole
    } else if token.starts_with(term) {
        Fit::Start
    } else if fuzzy && one_edit_apart(term, token) {
        Fit::Near
    } else {
        Fit::None
    }
}

/// `text` without its first `skip` characters.
fn after(text: &str, skip: usize) -> &str {
    let mut chars = text.chars();
    for _ in chars.by_ref().take(skip) {}
    chars.as_str()
}

/// Whether one edit turns `a` into `b`: one character removed, added or
/// replaced, or two neighbouring characters swapped. Equal texts are not
/// one edit apart.
fn one_edit_apart(a: &str, b: &str) -> bool {
    let same = a
        .chars()
        .zip(b.chars())
        .take_while(|(in_a, in_b)| in_a == in_b)
        .count();
    // What is left once the common beginning is gone. The two differ in
    // their first character, unless one of them is empty.
    let (a, b) = (after(a, same), after(b, same));
    let (rest_a, rest_b) = (after(a, 1), after(b, 1));
    let mut chars_a = a.chars();
    let mut chars_b = b.chars();
    let (first_a, second_a) = (chars_a.next(), chars_a.next());
    let (first_b, second_b) = (chars_b.next(), chars_b.next());
    let swapped =
        first_a == second_b && second_a == first_b && chars_a.as_str() == chars_b.as_str();
    a != b && (rest_a == rest_b || rest_a == b || a == rest_b || swapped)
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use super::super::doc::SearchDoc;
    use super::super::testing::{doc_ref, owned, titled};
    use super::*;
    use proptest::collection::{btree_set, vec};
    use proptest::prelude::*;

    use DocKind::{Album, Artist, Playlist, Track};

    /// One document of the hand-built library.
    #[expect(
        clippy::too_many_arguments,
        reason = "one argument per field keeps the library below readable as a table"
    )]
    fn entry(
        kind: DocKind,
        number: u32,
        title: &str,
        artist: &str,
        album: &str,
        credits: &[&str],
        genres: &[&str],
        labels: &[&str],
        plays: u32,
    ) -> SearchDoc {
        SearchDoc {
            doc: doc_ref(kind, number),
            title: title.to_owned(),
            artist: artist.to_owned(),
            album: album.to_owned(),
            credits: owned(credits),
            genres: owned(genres),
            labels: owned(labels),
            plays,
        }
    }

    /// A small library. Each document's number is its position, from 1.
    fn library() -> Index {
        let story = "(What's the Story) Morning Glory?";
        Index::build(
            [
                entry(Artist, 1, "Björk", "", "", &[], &["Electronic"], &[], 5),
                entry(
                    Album,
                    2,
                    "Amélie",
                    "Yann Tiersen",
                    "",
                    &[],
                    &["Soundtrack"],
                    &["Virgin"],
                    0,
                ),
                entry(
                    Track,
                    3,
                    "Comptine d’un autre été",
                    "Yann Tiersen",
                    "Amélie",
                    &["Yann Tiersen"],
                    &["Soundtrack"],
                    &["Virgin"],
                    3,
                ),
                entry(Artist, 4, "Radiohead", "", "", &[], &["Rock"], &[], 9),
                entry(
                    Album,
                    5,
                    "OK Computer",
                    "Radiohead",
                    "",
                    &[],
                    &["Rock"],
                    &["Parlophone"],
                    2,
                ),
                entry(
                    Track,
                    6,
                    "Paranoid Android",
                    "Radiohead",
                    "OK Computer",
                    &[],
                    &["Rock"],
                    &["Parlophone"],
                    12,
                ),
                entry(
                    Track,
                    7,
                    "Karma Police",
                    "Radiohead",
                    "OK Computer",
                    &[],
                    &["Rock"],
                    &["Parlophone"],
                    40,
                ),
                entry(
                    Track,
                    8,
                    "Opening",
                    "Michael Riesman",
                    "Glassworks",
                    &["Philip Glass"],
                    &["Minimalism"],
                    &[],
                    1,
                ),
                entry(Playlist, 9, "Road Trip", "", "", &[], &[], &[], 0),
                entry(
                    Track,
                    10,
                    story,
                    "Oasis",
                    story,
                    &[],
                    &["Rock"],
                    &["Creation"],
                    0,
                ),
            ]
            .into_iter(),
        )
    }

    fn hit(kind: DocKind, number: u32, matched: Match) -> Hit {
        Hit {
            doc: doc_ref(kind, number),
            matched,
        }
    }

    /// Every hit for `q` in the library, with no filter.
    fn all(q: &str) -> Vec<Hit> {
        library().query(q, KindFilter::All, u16::MAX)
    }

    #[test]
    fn finds_accented_titles_from_plain_letters() {
        assert_eq!(
            all("Amelie"),
            [
                hit(Album, 2, Match::WholeTitle),
                hit(Track, 3, Match::Words)
            ]
        );
        assert_eq!(all("bjork"), [hit(Artist, 1, Match::WholeTitle)]);
        assert_eq!(all("ete"), [hit(Track, 3, Match::Words)]);
    }

    #[test]
    fn finds_plain_titles_from_accented_letters() {
        assert_eq!(all("Rádiöhead"), all("radiohead"));
        assert_eq!(all("Rádiöhead").len(), 4);
    }

    #[test]
    fn ignores_case() {
        assert_eq!(
            all("RADIOHEAD"),
            [
                hit(Artist, 4, Match::WholeTitle),
                hit(Track, 7, Match::Words),
                hit(Track, 6, Match::Words),
                hit(Album, 5, Match::Words),
            ]
        );
        assert_eq!(all("bJÖRK"), [hit(Artist, 1, Match::WholeTitle)]);
    }

    #[test]
    fn treats_curly_and_straight_apostrophes_and_punctuation_alike() {
        let expected = [hit(Track, 3, Match::TitleStart)];
        assert_eq!(all("comptine d'un"), expected);
        assert_eq!(all("comptine d’un"), expected);
        assert_eq!(all("comptine dun"), expected);
        assert_eq!(all("Comptine, d'un..."), expected);
    }

    #[test]
    fn forgives_a_missing_letter() {
        assert_eq!(
            all("radiohed"),
            [
                hit(Track, 7, Match::NearMiss),
                hit(Track, 6, Match::NearMiss),
                hit(Artist, 4, Match::NearMiss),
                hit(Album, 5, Match::NearMiss),
            ]
        );
    }

    #[test]
    fn forgives_a_swapped_pair() {
        assert_eq!(
            all("raidohead"),
            [
                hit(Track, 7, Match::NearMiss),
                hit(Track, 6, Match::NearMiss),
                hit(Artist, 4, Match::NearMiss),
                hit(Album, 5, Match::NearMiss),
            ]
        );
    }

    #[test]
    fn forgives_an_added_letter_and_a_wrong_letter() {
        assert_eq!(all("bjorkk"), [hit(Artist, 1, Match::NearMiss)]);
        assert_eq!(all("bjurk"), [hit(Artist, 1, Match::NearMiss)]);
    }

    #[test]
    fn does_not_forgive_two_edits() {
        assert_eq!(all("radiohd"), []);
        assert_eq!(all("bjurkk"), []);
    }

    #[test]
    fn forgives_a_typo_only_in_a_term_of_four_characters_or_more() {
        // Four characters: "rokc" is "rock" with a swapped pair.
        assert_eq!(
            all("rokc"),
            [
                hit(Track, 7, Match::NearMiss),
                hit(Track, 6, Match::NearMiss),
                hit(Track, 10, Match::NearMiss),
                hit(Artist, 4, Match::NearMiss),
                hit(Album, 5, Match::NearMiss),
            ]
        );
        // Three characters: "rok" is "rock" with a letter missing, and
        // "teh" is "the" with a swapped pair.
        assert_eq!(all("rok"), []);
        assert_eq!(all("teh"), []);
    }

    #[test]
    fn a_typo_in_one_term_makes_the_whole_hit_a_near_miss() {
        assert_eq!(all("karma polise"), [hit(Track, 7, Match::NearMiss)]);
        assert_eq!(all("karma police"), [hit(Track, 7, Match::WholeTitle)]);
    }

    #[test]
    fn a_query_of_one_character_finds_every_word_that_begins_with_it() {
        assert_eq!(
            all("r"),
            [
                hit(Artist, 4, Match::TitleStart),
                hit(Playlist, 9, Match::TitleStart),
                hit(Track, 7, Match::WordStarts),
                hit(Track, 6, Match::WordStarts),
                hit(Track, 8, Match::WordStarts),
                hit(Track, 10, Match::WordStarts),
                hit(Album, 5, Match::WordStarts),
            ]
        );
    }

    /// An index for the caps: a title of 16 words, a title of 17, a title
    /// of one 33-letter word, and "Karma Police".
    fn long_titles() -> Index {
        Index::build(
            [
                titled(Track, 1, "a b c d e f g h i j k l m n o p"),
                titled(Track, 2, "a b c d e f g h i j k l m n o p q"),
                titled(Album, 3, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
                titled(Track, 4, "Karma Police"),
            ]
            .into_iter(),
        )
    }

    /// `head`, then spaces, then `tail`, `chars` characters in all.
    fn padded(head: &str, tail: &str, chars: usize) -> String {
        let spaces = chars - head.chars().count() - tail.chars().count();
        let mut text = String::from(head);
        text.extend((0..spaces).map(|_| ' '));
        text.push_str(tail);
        assert_eq!(text.chars().count(), chars);
        text
    }

    /// Verifies: SEC-API-063
    #[test]
    fn uses_only_the_first_16_terms_of_a_query() {
        let index = long_titles();
        let sixteen = "a b c d e f g h i j k l m n o p";
        // Sixteen terms: the 16-word title whole, and the start of the
        // 17-word one.
        let expected = [
            hit(Track, 1, Match::WholeTitle),
            hit(Track, 2, Match::TitleStart),
        ];
        assert_eq!(index.query(sixteen, KindFilter::All, u16::MAX), expected);
        // 256 characters and 17 terms: the 17th term, which is in no
        // document, is not used.
        let seventeen = padded(sixteen, "zzz", 256);
        assert_eq!(seventeen.split_whitespace().count(), 17);
        assert_eq!(index.query(&seventeen, KindFilter::All, u16::MAX), expected);
        // The 17-word title in full is cut to its first 16 words too.
        assert_eq!(
            index.query(
                "a b c d e f g h i j k l m n o p q",
                KindFilter::All,
                u16::MAX
            ),
            expected
        );
        // The 16th term is used: one that is in no document finds nothing.
        assert_eq!(
            index.query(
                "a b c d e f g h i j k l m n o zzz",
                KindFilter::All,
                u16::MAX
            ),
            []
        );
    }

    /// Verifies: SEC-API-063
    #[test]
    fn reads_only_the_first_256_characters_of_a_query() {
        let index = long_titles();
        let karma = [hit(Track, 4, Match::TitleStart)];
        // The 256th character is read: "z" is in no document.
        assert_eq!(
            index.query(&padded("karma", "z", 256), KindFilter::All, u16::MAX),
            []
        );
        // The 257th is not.
        assert_eq!(
            index.query(&padded("karma", "z", 257), KindFilter::All, u16::MAX),
            karma
        );
        // The cut counts characters, not octets: 251 two-octet spaces.
        let mut wide = String::from("karma");
        wide.extend((0..251).map(|_| '\u{00A0}'));
        wide.push('z');
        assert_eq!(wide.chars().count(), 257);
        assert_eq!(index.query(&wide, KindFilter::All, u16::MAX), karma);
    }

    #[test]
    fn matches_a_long_word_on_its_first_32_characters() {
        let index = long_titles();
        let expected = [hit(Album, 3, Match::WholeTitle)];
        // 32 letters, 33, and 40 with another ending.
        for q in [
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabbbbbbbb",
        ] {
            assert_eq!(index.query(q, KindFilter::All, u16::MAX), expected);
        }
    }

    #[test]
    fn a_type_filter_returns_one_type() {
        let index = library();
        assert_eq!(
            index.query("radiohead", KindFilter::Only(Track), u16::MAX),
            [hit(Track, 7, Match::Words), hit(Track, 6, Match::Words)]
        );
        assert_eq!(
            index.query("radiohead", KindFilter::Only(Artist), u16::MAX),
            [hit(Artist, 4, Match::WholeTitle)]
        );
        assert_eq!(
            index.query("radiohead", KindFilter::Only(Album), u16::MAX),
            [hit(Album, 5, Match::Words)]
        );
        assert_eq!(
            index.query("radiohead", KindFilter::Only(Playlist), u16::MAX),
            []
        );
        assert_eq!(
            index.query("road", KindFilter::Only(Playlist), u16::MAX),
            [hit(Playlist, 9, Match::TitleStart)]
        );
    }

    #[test]
    fn the_limit_applies_to_each_type() {
        let index = library();
        assert_eq!(
            index.query("radiohead", KindFilter::All, 1),
            [
                hit(Artist, 4, Match::WholeTitle),
                hit(Track, 7, Match::Words),
                hit(Album, 5, Match::Words),
            ]
        );
        assert_eq!(
            index.query("r", KindFilter::All, 2),
            [
                hit(Artist, 4, Match::TitleStart),
                hit(Playlist, 9, Match::TitleStart),
                hit(Track, 7, Match::WordStarts),
                hit(Track, 6, Match::WordStarts),
                hit(Album, 5, Match::WordStarts),
            ]
        );
        assert_eq!(
            index.query("r", KindFilter::Only(Track), 3),
            [
                hit(Track, 7, Match::WordStarts),
                hit(Track, 6, Match::WordStarts),
                hit(Track, 8, Match::WordStarts),
            ]
        );
        assert_eq!(index.query("radiohead", KindFilter::All, 0), []);
    }

    #[test]
    fn finds_a_composer_through_the_credits() {
        assert_eq!(all("philip glass"), [hit(Track, 8, Match::Words)]);
        assert_eq!(all("Glass"), [hit(Track, 8, Match::Words)]);
        assert_eq!(all("phil gla"), [hit(Track, 8, Match::WordStarts)]);
    }

    #[test]
    fn finds_by_artist_album_genre_and_label() {
        assert_eq!(all("oasis"), [hit(Track, 10, Match::Words)]);
        assert_eq!(all("glassworks"), [hit(Track, 8, Match::Words)]);
        assert_eq!(all("minimalism"), [hit(Track, 8, Match::Words)]);
        assert_eq!(
            all("parlophone"),
            [
                hit(Track, 7, Match::Words),
                hit(Track, 6, Match::Words),
                hit(Album, 5, Match::Words),
            ]
        );
    }

    /// Verifies: SEC-STD-011
    #[test]
    fn matches_regular_expression_metacharacters_literally() {
        // A pattern that would match everything matches nothing.
        assert_eq!(all(".*"), []);
        assert_eq!(all("^.+$"), []);
        assert_eq!(all("[a-z]+"), []);
        assert_eq!(all("\\w+|\\d*"), []);
        // An alternation is one word, which no document holds.
        assert_eq!(all("ok|karma"), []);
        assert_eq!(all("(ok)?(karma)?"), []);
        // The characters are dropped like any other punctuation.
        assert_eq!(
            all("radio.*"),
            [
                hit(Artist, 4, Match::TitleStart),
                hit(Track, 7, Match::WordStarts),
                hit(Track, 6, Match::WordStarts),
                hit(Album, 5, Match::WordStarts),
            ]
        );
        assert_eq!(
            all("(What's the Story) Morning Glory?"),
            [hit(Track, 10, Match::WholeTitle)]
        );
        assert_eq!(
            all("{what's} [the] ^story$ morning+ glory*"),
            [hit(Track, 10, Match::WholeTitle)]
        );
    }

    /// Verifies: SEC-STD-011, SEC-API-063
    #[test]
    fn answers_a_megabyte_of_a_catastrophic_pattern_from_its_first_256_characters() {
        // "(a+)+$" backtracks exponentially in a backtracking engine. Here
        // its first 256 characters fold to 43 letters "a", which are cut to
        // one 32-letter term.
        let pattern: String = (0..174_763).flat_map(|_| "(a+)+$".chars()).collect();
        assert_eq!(pattern.len(), 1_048_578);
        assert_eq!(
            long_titles().query(&pattern, KindFilter::All, u16::MAX),
            [hit(Album, 3, Match::WholeTitle)]
        );
        // A megabyte of punctuation holds no term at all.
        let nested: String = (0..1_048_576).map(|_| '(').collect();
        assert_eq!(library().query(&nested, KindFilter::All, u16::MAX), []);
    }

    #[test]
    fn a_query_without_a_letter_or_digit_finds_nothing() {
        assert_eq!(all(""), []);
        assert_eq!(all("   "), []);
        assert_eq!(all("?!"), []);
        assert_eq!(Index::default().query("radiohead", KindFilter::All, 9), []);
    }

    #[test]
    fn every_term_has_to_match() {
        assert_eq!(all("karma radiohead"), [hit(Track, 7, Match::Words)]);
        assert_eq!(all("karma bjork"), []);
        assert_eq!(all("bjork karma"), []);
        assert_eq!(all("radiohead rock zzz"), []);
    }

    #[test]
    fn ranks_the_whole_title_then_its_start_then_words() {
        assert_eq!(
            all("ok computer"),
            [
                hit(Album, 5, Match::WholeTitle),
                hit(Track, 7, Match::Words),
                hit(Track, 6, Match::Words),
            ]
        );
        assert_eq!(
            all("ok comp"),
            [
                hit(Album, 5, Match::TitleStart),
                hit(Track, 7, Match::WordStarts),
                hit(Track, 6, Match::WordStarts),
            ]
        );
        assert_eq!(
            all("ok"),
            [
                hit(Album, 5, Match::TitleStart),
                hit(Track, 7, Match::Words),
                hit(Track, 6, Match::Words),
            ]
        );
    }

    #[test]
    fn a_title_matches_as_a_title_only_in_its_own_word_order() {
        // The words of "OK Computer" the other way round are just words.
        assert_eq!(
            all("computer ok"),
            [
                hit(Track, 7, Match::Words),
                hit(Track, 6, Match::Words),
                hit(Album, 5, Match::Words),
            ]
        );
        // "computer" is the second word of the title, but the first term
        // is not its first word.
        assert_eq!(
            all("radiohead computer"),
            [
                hit(Track, 7, Match::Words),
                hit(Track, 6, Match::Words),
                hit(Album, 5, Match::Words),
            ]
        );
        // The title's second word alone is a word, not the title's start.
        assert_eq!(all("police"), [hit(Track, 7, Match::Words)]);
        // Only the last term may be the beginning of its word.
        assert_eq!(all("kar police"), [hit(Track, 7, Match::WordStarts)]);
        assert_eq!(all("karma pol"), [hit(Track, 7, Match::TitleStart)]);
    }

    #[test]
    fn ranks_by_plays_within_a_match_and_keeps_the_given_order_on_a_tie() {
        let mut often = titled(Track, 20, "Echo");
        often.plays = 2;
        let index = Index::build(
            [
                titled(Track, 30, "Echo"),
                titled(Track, 10, "Echo"),
                often,
                titled(Track, 40, "Echoes"),
            ]
            .into_iter(),
        );
        assert_eq!(
            index.query("echo", KindFilter::All, u16::MAX),
            [
                hit(Track, 20, Match::WholeTitle),
                hit(Track, 30, Match::WholeTitle),
                hit(Track, 10, Match::WholeTitle),
                hit(Track, 40, Match::TitleStart),
            ]
        );
    }

    #[test]
    fn plays_never_lift_a_weaker_match_over_a_better_one() {
        let mut often = titled(Track, 1, "Echoes");
        often.plays = u32::MAX;
        let index = Index::build([often, titled(Track, 2, "Echo")].into_iter());
        assert_eq!(
            index.query("echo", KindFilter::All, u16::MAX),
            [
                hit(Track, 2, Match::WholeTitle),
                hit(Track, 1, Match::TitleStart),
            ]
        );
    }

    #[test]
    fn the_group_with_the_best_hit_comes_first() {
        let mut album = titled(Album, 2, "Blue");
        album.plays = 1;
        let index = Index::build(
            [
                titled(Artist, 1, "Blue Note"),
                album,
                titled(Track, 3, "Blue"),
                titled(Playlist, 4, "Kind of Blue"),
                titled(Artist, 5, "Blue"),
            ]
            .into_iter(),
        );
        assert_eq!(
            index.query("blue", KindFilter::All, u16::MAX),
            [
                hit(Album, 2, Match::WholeTitle),
                hit(Track, 3, Match::WholeTitle),
                hit(Artist, 5, Match::WholeTitle),
                hit(Artist, 1, Match::TitleStart),
                hit(Playlist, 4, Match::Words),
            ]
        );
    }

    #[test]
    fn one_edit_is_a_removal_an_addition_a_replacement_or_a_swap() {
        let apart = [
            // A character replaced: first, middle, last.
            ("cat", "bat"),
            ("cat", "cot"),
            ("cat", "cab"),
            // A character removed or added: first, middle, last.
            ("cat", "at"),
            ("cat", "ct"),
            ("cat", "ca"),
            ("a", ""),
            // Neighbours swapped: first, middle, last.
            ("abcd", "bacd"),
            ("abcd", "acbd"),
            ("abcd", "abdc"),
            ("ab", "ba"),
            // Beyond ASCII.
            ("ete", "éte"),
            ("東京", "京東"),
            ("東京", "東"),
        ];
        for (a, b) in apart {
            assert_eq!((a, b, one_edit_apart(a, b)), (a, b, true));
            assert_eq!((b, a, one_edit_apart(b, a)), (b, a, true));
        }
    }

    #[test]
    fn equal_texts_and_texts_two_edits_apart_are_not_one_edit_apart() {
        let not_apart = [
            ("", ""),
            ("a", "a"),
            ("cat", "cat"),
            // Two replaced.
            ("cat", "dog"),
            ("cat", "bot"),
            ("cat", "cub"),
            // Two removed.
            ("cat", "c"),
            ("cat", "t"),
            ("ab", ""),
            // One removed and one replaced.
            ("cart", "cab"),
            ("cart", "bat"),
            // A swap and another edit.
            ("abcd", "bacx"),
            ("abcd", "bac"),
            ("abcd", "badc"),
            // Swapped, but not neighbours.
            ("abc", "cba"),
            // A swap with something between.
            ("abxcd", "baycd"),
            // A rotation takes two edits.
            ("abc", "bca"),
            ("abc", "cab"),
        ];
        for (a, b) in not_apart {
            assert_eq!((a, b, one_edit_apart(a, b)), (a, b, false));
            assert_eq!((b, a, one_edit_apart(b, a)), (b, a, false));
        }
    }

    /// The restricted edit distance (optimal string alignment) of `a` and
    /// `b`, by the textbook table.
    fn distance(a: &[char], b: &[char]) -> usize {
        // Row 0 and column 0 are the cost of adding or removing everything.
        let mut table: Vec<Vec<usize>> = (0..=a.len())
            .map(|i| (0..=b.len()).map(|j| i.max(j)).collect())
            .collect();
        for i in 1..=a.len() {
            for j in 1..=b.len() {
                let cost = usize::from(a[i - 1] != b[j - 1]);
                let mut best = (table[i - 1][j] + 1)
                    .min(table[i][j - 1] + 1)
                    .min(table[i - 1][j - 1] + cost);
                if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                    best = best.min(table[i - 2][j - 2] + 1);
                }
                table[i][j] = best;
            }
        }
        table[a.len()][b.len()]
    }

    #[test]
    fn the_reference_distance_agrees_with_known_values() {
        let cases = [
            ("", "", 0),
            ("abc", "abc", 0),
            ("kitten", "sitting", 3),
            ("ab", "ba", 1),
            ("ca", "abc", 3),
            ("radiohead", "raidohead", 1),
            ("radiohead", "radiohed", 1),
        ];
        for (a, b, expected) in cases {
            let a: Vec<char> = a.chars().collect();
            let b: Vec<char> = b.chars().collect();
            assert_eq!(distance(&a, &b), expected);
        }
    }

    /// A word of a few letters from a small alphabet, so words collide,
    /// begin one another and sit one edit apart often.
    fn word() -> impl Strategy<Value = String> {
        "[abc]{1,5}"
    }

    /// One to three words.
    fn phrase() -> impl Strategy<Value = String> {
        vec(word(), 1..=3).prop_map(|words| words.join(" "))
    }

    fn kind() -> impl Strategy<Value = DocKind> {
        prop_oneof![Just(Artist), Just(Album), Just(Track), Just(Playlist)]
    }

    /// A document's kind, artist, a credit and plays; its number and title
    /// are given when the library is put together.
    fn rest() -> impl Strategy<Value = (DocKind, String, String, u32)> {
        (kind(), phrase(), phrase(), 0_u32..4)
    }

    /// Documents numbered from 0 with the given titles.
    fn docs(titles: Vec<String>, rest: Vec<(DocKind, String, String, u32)>) -> Vec<SearchDoc> {
        titles
            .into_iter()
            .zip(rest)
            .zip(0_u32..)
            .map(
                |((title, (kind, artist, credit, plays)), number)| SearchDoc {
                    doc: doc_ref(kind, number),
                    title,
                    artist,
                    album: String::new(),
                    credits: vec![credit],
                    genres: Vec::new(),
                    labels: Vec::new(),
                    plays,
                },
            )
            .collect()
    }

    proptest! {
        #[test]
        fn one_edit_apart_means_a_reference_distance_of_one(
            a in "[abc]{0,6}",
            b in "[abc]{0,6}",
        ) {
            let left: Vec<char> = a.chars().collect();
            let right: Vec<char> = b.chars().collect();
            prop_assert_eq!(one_edit_apart(&a, &b), distance(&left, &right) == 1);
        }

        #[test]
        fn a_title_no_other_document_has_is_the_first_hit_for_itself(
            titles in btree_set(phrase(), 1..12),
            rest in vec(rest(), 12),
            pick in any::<prop::sample::Index>(),
        ) {
            let titles: Vec<String> = titles.into_iter().collect();
            let library = docs(titles.clone(), rest);
            let target = pick.get(&library).clone();
            let index = Index::build(library.into_iter());
            prop_assert_eq!(
                index.query(&target.title, KindFilter::All, 1).first().copied(),
                Some(Hit { doc: target.doc, matched: Match::WholeTitle })
            );
        }

        #[test]
        fn adding_documents_keeps_every_hit_in_its_order(
            titles in vec(phrase(), 0..10),
            rest in vec(rest(), 10),
            keep in 0_usize..10,
            q in phrase(),
        ) {
            let library = docs(titles, rest);
            let keep = keep.min(library.len());
            let before = Index::build(library.iter().take(keep).cloned());
            let after = Index::build(library.iter().cloned());
            let kept: Vec<DocRef> = library.iter().take(keep).map(|doc| doc.doc).collect();
            for filter in [
                KindFilter::Only(Artist),
                KindFilter::Only(Album),
                KindFilter::Only(Track),
                KindFilter::Only(Playlist),
            ] {
                let earlier = before.query(&q, filter, u16::MAX);
                let mut later = after.query(&q, filter, u16::MAX);
                later.retain(|hit| kept.contains(&hit.doc));
                prop_assert_eq!(later, earlier);
            }
        }

        #[test]
        fn a_query_of_any_text_returns_no_more_than_the_limit_of_each_type(
            titles in vec(".{0,12}", 0..8),
            rest in vec(rest(), 8),
            q in ".{0,300}",
            limit in 0_u16..4,
        ) {
            let index = Index::build(docs(titles, rest).into_iter());
            let hits = index.query(&q, KindFilter::All, limit);
            for kind in DocKind::ALL {
                let of_kind = hits.iter().filter(|hit| hit.doc.kind() == kind).count();
                prop_assert!(of_kind <= usize::from(limit));
                prop_assert_eq!(
                    index.query(&q, KindFilter::Only(kind), limit),
                    hits.iter().filter(|hit| hit.doc.kind() == kind).copied().collect::<Vec<Hit>>()
                );
            }
        }
    }
}
