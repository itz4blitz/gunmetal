//! Lyrics: the core's position lookup, for the browser (MUS-154, MUS-155).
//!
//! [`lyric_position`] is one call of [`lyrics::position`] with its types
//! converted, so the highlighted line is the same one the server would pick.
//! The browser calls it as `lyricPosition`.

use gunmetal_core::lyrics;
use serde::{Deserialize, Serialize};
use tsify::Tsify;

/// A line with a time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub struct Line {
    /// When the line starts, in milliseconds.
    pub at: u32,
    /// The line.
    pub text: String,
}

/// A word with a time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub struct Word {
    /// When the word starts, in milliseconds.
    pub at: u32,
    /// The word.
    pub text: String,
}

/// A line whose words have a time each.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub struct WordLine {
    /// When the line starts, in milliseconds.
    pub at: u32,
    /// The words in order.
    pub words: Vec<Word>,
}

/// Lyrics, in the order they are sung.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub enum Lyrics {
    /// Lines without times. An empty line separates stanzas.
    Plain(Vec<String>),
    /// Lines with a time each, sorted by time.
    Lines(Vec<Line>),
    /// Lines whose words have a time each, sorted by time.
    Words(Vec<WordLine>),
}

/// The line, and the word in it, that a player is at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Tsify)]
pub struct Cursor {
    /// The index of the line in the lyrics.
    pub line: u32,
    /// The index of the word in that line, or omitted before its first word
    /// and in lyrics without word times.
    pub word: Option<u32>,
}

/// What [`lyric_position`] found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Tsify)]
pub enum LyricPosition {
    /// The player is at this line (and word, when the lyrics have word times).
    At(Cursor),
    /// Before the first line, or the lyrics have no times.
    None,
}

/// The TypeScript declarations of this module's mirror types, in the order
/// they are declared above.
pub const DECLARATIONS: [&str; 6] = [
    Line::DECL,
    Word::DECL,
    WordLine::DECL,
    Lyrics::DECL,
    Cursor::DECL,
    LyricPosition::DECL,
];

impl From<lyrics::Line> for Line {
    fn from(value: lyrics::Line) -> Self {
        let lyrics::Line { at, text } = value;
        Self { at, text }
    }
}

impl From<Line> for lyrics::Line {
    fn from(value: Line) -> Self {
        let Line { at, text } = value;
        Self { at, text }
    }
}

impl From<lyrics::Word> for Word {
    fn from(value: lyrics::Word) -> Self {
        let lyrics::Word { at, text } = value;
        Self { at, text }
    }
}

impl From<Word> for lyrics::Word {
    fn from(value: Word) -> Self {
        let Word { at, text } = value;
        Self { at, text }
    }
}

impl From<lyrics::WordLine> for WordLine {
    fn from(value: lyrics::WordLine) -> Self {
        let lyrics::WordLine { at, words } = value;
        Self {
            at,
            words: words.into_iter().map(Word::from).collect(),
        }
    }
}

impl From<WordLine> for lyrics::WordLine {
    fn from(value: WordLine) -> Self {
        let WordLine { at, words } = value;
        Self {
            at,
            words: words.into_iter().map(lyrics::Word::from).collect(),
        }
    }
}

impl From<lyrics::Lyrics> for Lyrics {
    fn from(value: lyrics::Lyrics) -> Self {
        match value {
            lyrics::Lyrics::Plain(lines) => Self::Plain(lines),
            lyrics::Lyrics::Lines(lines) => {
                Self::Lines(lines.into_iter().map(Line::from).collect())
            }
            lyrics::Lyrics::Words(lines) => {
                Self::Words(lines.into_iter().map(WordLine::from).collect())
            }
        }
    }
}

impl From<Lyrics> for lyrics::Lyrics {
    fn from(value: Lyrics) -> Self {
        match value {
            Lyrics::Plain(lines) => Self::Plain(lines),
            Lyrics::Lines(lines) => {
                Self::Lines(lines.into_iter().map(lyrics::Line::from).collect())
            }
            Lyrics::Words(lines) => {
                Self::Words(lines.into_iter().map(lyrics::WordLine::from).collect())
            }
        }
    }
}

fn index_u32(index: usize) -> u32 {
    u32::try_from(index).unwrap_or(u32::MAX)
}

impl From<lyrics::Cursor> for Cursor {
    fn from(value: lyrics::Cursor) -> Self {
        let lyrics::Cursor { line, word } = value;
        Self {
            line: index_u32(line),
            word: word.map(index_u32),
        }
    }
}

/// The line, and the word in it, that a player `millis` milliseconds into
/// the track is at.
#[must_use]
pub fn lyric_position(millis: u32, lyrics: Lyrics) -> LyricPosition {
    match lyrics::position(&lyrics.into(), millis) {
        Some(cursor) => LyricPosition::At(cursor.into()),
        None => LyricPosition::None,
    }
}

crate::export::export! {
    /// The browser's `lyricPosition`: [`lyric_position`], with its types converted.
    "lyricPosition": fn lyric_position_export = lyric_position(millis: u32; lyrics: Lyrics) -> LyricPosition
}

#[cfg(test)]
mod tests {
    use gunmetal_core::lyrics;

    use super::{
        Cursor, DECLARATIONS, Line, LyricPosition, Lyrics, Word, WordLine, lyric_position,
    };

    #[test]
    fn a_line_converts_field_by_field_and_back() {
        let core = lyrics::Line {
            at: 1_250,
            text: "Hello".to_owned(),
        };
        let mirrored = Line {
            at: 1_250,
            text: "Hello".to_owned(),
        };
        assert_eq!(Line::from(core.clone()), mirrored);
        assert_eq!(lyrics::Line::from(mirrored), core);
    }

    #[test]
    fn a_word_line_converts_field_by_field_and_back() {
        let core = lyrics::WordLine {
            at: 100,
            words: vec![
                lyrics::Word {
                    at: 100,
                    text: "Hel".to_owned(),
                },
                lyrics::Word {
                    at: 200,
                    text: "lo".to_owned(),
                },
            ],
        };
        let mirrored = WordLine {
            at: 100,
            words: vec![
                Word {
                    at: 100,
                    text: "Hel".to_owned(),
                },
                Word {
                    at: 200,
                    text: "lo".to_owned(),
                },
            ],
        };
        assert_eq!(WordLine::from(core.clone()), mirrored);
        assert_eq!(lyrics::WordLine::from(mirrored), core);
    }

    /// Plain lyrics, and a time before the first timed line, have no cursor.
    #[test]
    fn plain_lyrics_and_a_time_before_the_first_line_are_none() {
        assert_eq!(
            lyric_position(0, Lyrics::Plain(vec!["a".to_owned()])),
            LyricPosition::None
        );
        assert_eq!(
            lyrics::position(&lyrics::Lyrics::Plain(vec!["a".to_owned()]), 0),
            None
        );
        let timed = Lyrics::Lines(vec![Line {
            at: 1_000,
            text: "later".to_owned(),
        }]);
        assert_eq!(lyric_position(999, timed), LyricPosition::None);
    }

    /// The highlighted line is the latest whose time is not after `millis`,
    /// matching the core, and a later time never goes back.
    #[test]
    fn the_highlighted_line_matches_the_core_and_never_goes_back() {
        let lines = vec![
            Line {
                at: 0,
                text: "one".to_owned(),
            },
            Line {
                at: 1_000,
                text: "two".to_owned(),
            },
            Line {
                at: 2_000,
                text: "three".to_owned(),
            },
        ];
        let cases = [
            (0, 0),
            (999, 0),
            (1_000, 1),
            (1_500, 1),
            (2_000, 2),
            (9_000, 2),
        ];
        for (millis, line) in cases {
            assert_eq!(
                lyric_position(millis, Lyrics::Lines(lines.clone())),
                LyricPosition::At(Cursor { line, word: None })
            );
            let core_lines: Vec<lyrics::Line> =
                lines.iter().cloned().map(lyrics::Line::from).collect();
            assert_eq!(
                lyrics::position(&lyrics::Lyrics::Lines(core_lines), millis),
                Some(lyrics::Cursor {
                    line: usize::try_from(line).expect("small"),
                    word: None,
                })
            );
        }
    }

    /// Word-timed lyrics name the last word whose time is not after `millis`.
    #[test]
    fn a_word_timed_line_names_the_word_the_core_does() {
        let lyrics = Lyrics::Words(vec![WordLine {
            at: 0,
            words: vec![
                Word {
                    at: 0,
                    text: "a".to_owned(),
                },
                Word {
                    at: 400,
                    text: "b".to_owned(),
                },
            ],
        }]);
        assert_eq!(
            lyric_position(0, lyrics.clone()),
            LyricPosition::At(Cursor {
                line: 0,
                word: Some(0),
            })
        );
        assert_eq!(
            lyric_position(400, lyrics),
            LyricPosition::At(Cursor {
                line: 0,
                word: Some(1),
            })
        );
    }

    #[test]
    fn word_timed_lyrics_convert_from_the_core_and_back() {
        assert_eq!(
            Lyrics::from(lyrics::Lyrics::Words(vec![lyrics::WordLine::from(
                WordLine {
                    at: 0,
                    words: vec![Word {
                        at: 0,
                        text: "a".to_owned(),
                    }],
                }
            )])),
            Lyrics::Words(vec![WordLine {
                at: 0,
                words: vec![Word {
                    at: 0,
                    text: "a".to_owned(),
                }],
            }])
        );
        assert_eq!(
            lyrics::Lyrics::from(Lyrics::Words(vec![WordLine {
                at: 0,
                words: vec![Word {
                    at: 0,
                    text: "a".to_owned(),
                }],
            }])),
            lyrics::Lyrics::Words(vec![lyrics::WordLine {
                at: 0,
                words: vec![lyrics::Word {
                    at: 0,
                    text: "a".to_owned(),
                }],
            }])
        );
    }

    #[test]
    fn the_declarations_name_every_mirror() {
        assert_eq!(DECLARATIONS.len(), 6);
        assert!(DECLARATIONS[3].contains("export type Lyrics"));
        assert!(DECLARATIONS[5].contains("export type LyricPosition"));
    }

    #[test]
    fn plain_and_line_timed_lyrics_convert_from_the_core_and_back() {
        let plain = vec!["a".to_owned(), String::new(), "b".to_owned()];
        assert_eq!(
            Lyrics::from(lyrics::Lyrics::Plain(plain.clone())),
            Lyrics::Plain(plain.clone())
        );
        assert_eq!(
            lyrics::Lyrics::from(Lyrics::Plain(plain.clone())),
            lyrics::Lyrics::Plain(plain)
        );
        let lines = vec![Line {
            at: 250,
            text: "one".to_owned(),
        }];
        assert_eq!(
            Lyrics::from(lyrics::Lyrics::Lines(
                lines.iter().cloned().map(lyrics::Line::from).collect()
            )),
            Lyrics::Lines(lines.clone())
        );
        assert_eq!(
            lyrics::Lyrics::from(Lyrics::Lines(lines.clone())),
            lyrics::Lyrics::Lines(lines.into_iter().map(lyrics::Line::from).collect())
        );
    }

    #[test]
    fn a_word_converts_field_by_field_and_back() {
        let core = lyrics::Word {
            at: 40,
            text: "hi".to_owned(),
        };
        let mirrored = Word {
            at: 40,
            text: "hi".to_owned(),
        };
        assert_eq!(Word::from(core.clone()), mirrored);
        assert_eq!(lyrics::Word::from(mirrored), core);
    }

    #[test]
    fn a_cursor_past_u32_max_saturates() {
        assert_eq!(
            Cursor::from(lyrics::Cursor {
                line: usize::MAX,
                word: Some(usize::MAX),
            }),
            Cursor {
                line: u32::MAX,
                word: Some(u32::MAX),
            }
        );
        assert_eq!(
            Cursor::from(lyrics::Cursor {
                line: 0,
                word: None,
            }),
            Cursor {
                line: 0,
                word: None
            }
        );
        assert_eq!(super::index_u32(0), 0);
        assert_eq!(super::index_u32(u32::MAX as usize), u32::MAX);
        assert_eq!(super::index_u32(usize::MAX), u32::MAX);
    }
}
