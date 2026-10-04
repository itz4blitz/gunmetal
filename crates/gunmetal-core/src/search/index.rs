//! The index itself: every document, and for every folded token the
//! documents that hold it.
//!
//! A token is one whitespace-separated word of a text after
//! [`fold`], cut to [`MAX_TOKEN_CHARS`] characters, so "Amélie" and
//! "amelie", and "d’un" and "d'un", are the same token (DIS-085). Each
//! token keeps one posting per document and place: the token's position in
//! the title, or [`ELSEWHERE`] for the artist, album, credits, genres and
//! labels (MUS-061). The title positions are what lets a query tell a whole
//! title from a title that merely holds its words.

use std::collections::BTreeMap;

use crate::collate::fold;

use super::doc::{DocRef, SearchDoc};

/// The most characters of one token that are kept, in the index and in a
/// query alike. Longer words match on their first 32 characters.
pub const MAX_TOKEN_CHARS: usize = 32;

/// The place of a token that is not in the title, or that sits so far into
/// a title that no query term can line up with it.
pub(super) const ELSEWHERE: u8 = u8::MAX;

/// One document as the index keeps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Entry {
    /// Which thing it is.
    pub(super) doc: DocRef,
    /// How often this person has played it.
    pub(super) plays: u32,
    /// How many tokens its title has, counted up to 255.
    pub(super) title_len: u8,
}

/// One place a token occurs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Posting {
    /// The document, as its position in [`Index::docs`].
    pub(super) doc: u32,
    /// The token's position in the title, or [`ELSEWHERE`].
    pub(super) place: u8,
}

/// A search index over one profile's synced library.
///
/// It holds no text but the folded tokens, so it is small, and nothing in
/// it says which field a token came from beyond the title's positions.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Index {
    /// Every document, in the order it was given.
    pub(super) docs: Vec<Entry>,
    /// Every token, with its postings in ascending order of document.
    pub(super) terms: BTreeMap<String, Vec<Posting>>,
}

impl Index {
    /// Builds the index of `items`.
    ///
    /// Hits that rank alike come back in the order the documents are given
    /// here, so the caller chooses that order, for example by sort key. At
    /// most `u32::MAX` documents are taken.
    #[must_use]
    pub fn build(items: impl Iterator<Item = SearchDoc>) -> Self {
        let mut index = Self::default();
        for (ordinal, item) in (0..u32::MAX).zip(items) {
            let mut place = 0_u8;
            for token in tokens(&item.title) {
                index.post(
                    token,
                    Posting {
                        doc: ordinal,
                        place,
                    },
                );
                place = place.saturating_add(1);
            }
            let elsewhere = [&item.artist, &item.album]
                .into_iter()
                .chain(&item.credits)
                .chain(&item.genres)
                .chain(&item.labels);
            for text in elsewhere {
                for token in tokens(text) {
                    index.post(
                        token,
                        Posting {
                            doc: ordinal,
                            place: ELSEWHERE,
                        },
                    );
                }
            }
            index.docs.push(Entry {
                doc: item.doc,
                plays: item.plays,
                title_len: place,
            });
        }
        index
    }

    /// Records that `token` occurs at `posting`, once.
    fn post(&mut self, token: String, posting: Posting) {
        let postings = self.terms.entry(token).or_default();
        if postings.last() != Some(&posting) {
            postings.push(posting);
        }
    }
}

/// The tokens of `text`: its folded words, each cut to
/// [`MAX_TOKEN_CHARS`] characters.
pub(super) fn tokens(text: &str) -> Vec<String> {
    fold(text)
        .split(' ')
        .filter(|word| !word.is_empty())
        .map(|word| word.chars().take(MAX_TOKEN_CHARS).collect())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::doc::DocKind;
    use super::super::testing::{doc_ref, owned, titled};
    use super::*;

    fn posting(doc: u32, place: u8) -> Posting {
        Posting { doc, place }
    }

    #[test]
    fn tokens_are_folded_words() {
        assert_eq!(tokens("Amélie"), ["amelie"]);
        assert_eq!(
            tokens("  Comptine d’un  autre\tété "),
            ["comptine", "dun", "autre", "ete"]
        );
        assert_eq!(tokens("AC/DC"), ["acdc"]);
        assert_eq!(tokens("Guns N' Roses"), ["guns", "n", "roses"]);
    }

    #[test]
    fn text_without_a_letter_or_digit_has_no_tokens() {
        assert_eq!(tokens(""), Vec::<String>::new());
        assert_eq!(tokens("   "), Vec::<String>::new());
        assert_eq!(tokens("?! ... (*)"), Vec::<String>::new());
    }

    #[test]
    fn a_token_is_cut_to_32_characters() {
        // 33 letters: the first 32 are kept.
        assert_eq!(
            tokens("abcdefghijklmnopqrstuvwxyzabcdefg hi"),
            ["abcdefghijklmnopqrstuvwxyzabcdef", "hi"]
        );
        // 32 letters are kept whole.
        assert_eq!(
            tokens("abcdefghijklmnopqrstuvwxyzabcdef"),
            ["abcdefghijklmnopqrstuvwxyzabcdef"]
        );
        // The cut counts folded characters, not the characters typed: one
        // U+FB03 ligature folds to "ffi", so eleven of them are 33.
        assert_eq!(tokens("ﬃﬃﬃﬃﬃﬃﬃﬃﬃﬃﬃ"), ["ffiffiffiffiffiffiffiffiffiffiff"]);
    }

    #[test]
    fn builds_nothing_from_nothing() {
        let index = Index::build(std::iter::empty());
        assert_eq!(index, Index::default());
        assert_eq!(index.docs, []);
        assert_eq!(index.terms, BTreeMap::new());
    }

    #[test]
    fn keeps_every_document_in_the_order_given() {
        let mut played = titled(DocKind::Track, 7, "Go");
        played.plays = 300;
        let index = Index::build(
            [
                played,
                titled(DocKind::Artist, 2, "Go Go"),
                titled(DocKind::Playlist, 9, "!!!"),
            ]
            .into_iter(),
        );
        assert_eq!(
            index.docs,
            [
                Entry {
                    doc: doc_ref(DocKind::Track, 7),
                    plays: 300,
                    title_len: 1
                },
                Entry {
                    doc: doc_ref(DocKind::Artist, 2),
                    plays: 0,
                    title_len: 2
                },
                Entry {
                    doc: doc_ref(DocKind::Playlist, 9),
                    plays: 0,
                    title_len: 0
                },
            ]
        );
    }

    #[test]
    fn posts_title_tokens_with_their_positions() {
        let index = Index::build(
            [
                titled(DocKind::Track, 1, "Go"),
                titled(DocKind::Artist, 2, "Go West Go"),
            ]
            .into_iter(),
        );
        assert_eq!(
            index.terms,
            BTreeMap::from([
                (
                    String::from("go"),
                    vec![posting(0, 0), posting(1, 0), posting(1, 2)]
                ),
                (String::from("west"), vec![posting(1, 1)]),
            ])
        );
    }

    #[test]
    fn posts_every_other_field_as_elsewhere_once() {
        let mut track = titled(DocKind::Track, 1, "Opening");
        track.artist = String::from("Michael Riesman");
        track.album = String::from("Glassworks");
        track.credits = owned(&["Philip Glass", "Michael Riesman"]);
        track.genres = owned(&["Minimalism", "Glass"]);
        track.labels = owned(&["CBS", "Opening"]);
        let index = Index::build([track].into_iter());
        assert_eq!(
            index.terms,
            BTreeMap::from([
                (String::from("cbs"), vec![posting(0, ELSEWHERE)]),
                (String::from("glass"), vec![posting(0, ELSEWHERE)]),
                (String::from("glassworks"), vec![posting(0, ELSEWHERE)]),
                (String::from("michael"), vec![posting(0, ELSEWHERE)]),
                (String::from("minimalism"), vec![posting(0, ELSEWHERE)]),
                (
                    String::from("opening"),
                    vec![posting(0, 0), posting(0, ELSEWHERE)]
                ),
                (String::from("philip"), vec![posting(0, ELSEWHERE)]),
                (String::from("riesman"), vec![posting(0, ELSEWHERE)]),
            ])
        );
    }

    #[test]
    fn an_empty_field_posts_nothing() {
        let index = Index::build([titled(DocKind::Album, 1, "")].into_iter());
        assert_eq!(index.terms, BTreeMap::new());
        assert_eq!(
            index.docs,
            [Entry {
                doc: doc_ref(DocKind::Album, 1),
                plays: 0,
                title_len: 0
            }]
        );
    }
}
