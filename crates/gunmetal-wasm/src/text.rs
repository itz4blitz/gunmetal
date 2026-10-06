//! Text from a person or a server: the core's normalisation, for the browser
//! (SEC-MED-077).
//!
//! [`normalise_text`] is one call of the core's [`text::normalise`] with its
//! types converted, so the browser removes the same characters and keeps the
//! same number of octets as the server does. The browser calls it as
//! `normaliseText`, through the wrapper the `export!` line below it writes.
//! What comes back is text to show as text, never markup.
//!
//! A JavaScript string reaches this crate as UTF-8, with any unpaired
//! surrogate already replaced by U+FFFD on the way in, so such a
//! replacement is not reported. Nor can any other be: [`normalise_text`]
//! takes a `&str`, which is always UTF-8, so `replaced` in its answer is
//! always false. The field is there because the mirror takes the core's
//! `Text` apart field by field (record 12, decision 13).
//!
//! The doc comment of a mirror type or field is copied into its TypeScript
//! declaration, so each is one plain line.

use gunmetal_core::text;
use gunmetal_core::untrusted::Untrusted;
use serde::{Deserialize, Serialize};
use tsify::Tsify;

/// Whether a field may hold more than one line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub enum Lines {
    /// A name, title or label: no control character, and nothing that reorders or breaks the text around it.
    Single,
    /// A comment, description or lyric: tab and line feed stay.
    Multi,
}

// Mirrors `text::Text`, whose fields are public, under another name: `Text`
// is already a type in TypeScript's declarations for the browser. It is an
// answer only. Nothing converts it back to the core's type, because text
// returns to the core through `normalise_text` and no other way.
/// Text that is safe to store and to show as text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Tsify)]
pub struct NormalisedText {
    /// The text.
    pub value: String,
    /// Whether characters past the cap were dropped, so that the value is a prefix of the whole text.
    pub truncated: bool,
    /// Whether the input held octets that were not UTF-8, now replaced with U+FFFD.
    pub replaced: bool,
}

/// The TypeScript declarations of this module's mirror types, in the order
/// they are declared above.
pub const DECLARATIONS: [&str; 2] = [Lines::DECL, NormalisedText::DECL];

impl From<text::Lines> for Lines {
    fn from(value: text::Lines) -> Self {
        match value {
            text::Lines::Single => Self::Single,
            text::Lines::Multi => Self::Multi,
        }
    }
}

impl From<Lines> for text::Lines {
    fn from(value: Lines) -> Self {
        match value {
            Lines::Single => Self::Single,
            Lines::Multi => Self::Multi,
        }
    }
}

impl From<text::Text> for NormalisedText {
    fn from(normalised: text::Text) -> Self {
        let text::Text {
            value,
            truncated,
            replaced,
        } = normalised;
        Self {
            value,
            truncated,
            replaced,
        }
    }
}

/// Normalises text from a person or a server with the core's rules: the
/// characters `lines` forbids removed, and at most `cap` octets of UTF-8
/// kept.
#[must_use]
pub fn normalise_text(input: &str, cap: u32, lines: Lines) -> NormalisedText {
    text::normalise(Untrusted::new(input.as_bytes()), lines.into(), cap).into()
}

crate::export::export! {
    /// The browser's `normaliseText`: [`normalise_text`], with its types converted.
    ///
    /// `cap` arrives as a JavaScript number. As far as can be told from how
    /// `wasm-bindgen` 0.2.129 passes a number as a `u32` (WebAssembly's
    /// 32-bit integer conversion, read without a sign), and not from any run,
    /// since no test here runs a wrapper: `-1` becomes 4294967295, so nothing
    /// is cut, and `NaN` or `undefined` becomes 0, so everything is cut. Pass
    /// a whole number from 0 to 4294967295.
    "normaliseText": fn normalise_text_export = normalise_text(input: &str, cap: u32; lines: Lines) -> NormalisedText
}

#[cfg(test)]
mod tests {
    use gunmetal_core::text;

    use super::{DECLARATIONS, Lines, NormalisedText, normalise_text};

    /// A mirror of normalised text.
    fn mirror(value: &str, truncated: bool, replaced: bool) -> NormalisedText {
        NormalisedText {
            value: value.to_owned(),
            truncated,
            replaced,
        }
    }

    /// Both line rules of the core, each beside its mirror.
    const LINES: [(text::Lines, Lines); 2] = [
        (text::Lines::Single, Lines::Single),
        (text::Lines::Multi, Lines::Multi),
    ];

    #[test]
    fn each_line_rule_converts_to_its_mirror_and_back() {
        for (rule, mirrored) in LINES {
            assert_eq!(Lines::from(rule), mirrored);
            assert_eq!(text::Lines::from(mirrored), rule);
        }
    }

    /// Each field of the core's text arrives in the field of the same name:
    /// the two flags differ in every case, so neither can stand in for the
    /// other.
    #[test]
    fn the_core_s_text_becomes_its_mirror_field_by_field() {
        let cut = text::Text {
            value: "Caf\u{E9}".to_owned(),
            truncated: true,
            replaced: false,
        };
        assert_eq!(NormalisedText::from(cut), mirror("Caf\u{E9}", true, false));
        let repaired = text::Text {
            value: "a\u{FFFD}".to_owned(),
            truncated: false,
            replaced: true,
        };
        assert_eq!(
            NormalisedText::from(repaired),
            mirror("a\u{FFFD}", false, true)
        );
    }

    /// The function the browser calls as `normaliseText` removes from a
    /// name what the core's single-line rule removes: every control
    /// character, tab and line feed among them, and the characters that
    /// reorder or break the text around them. A server's text is shown only
    /// after this, and as text.
    ///
    /// Verifies: SEC-MED-077
    #[test]
    fn a_name_loses_every_control_and_every_character_that_reorders_or_breaks_a_line() {
        let hostile = "a\u{0}b\tc\nd\re\u{1B}f\u{7F}g\u{85}h\u{9F}i\
                       \u{2028}j\u{2029}k\u{202A}l\u{202B}m\u{202C}n\u{202D}o\u{202E}p\
                       \u{2066}q\u{2067}r\u{2068}s\u{2069}t";
        assert_eq!(
            normalise_text(hostile, 64, Lines::Single),
            mirror("abcdefghijklmnopqrst", false, false)
        );
        // Markup is not a control: it stays, and is shown as the text it is.
        assert_eq!(
            normalise_text("<b onclick=\"x()\">Caf\u{E9}</b> &amp;", 64, Lines::Single),
            mirror("<b onclick=\"x()\">Caf\u{E9}</b> &amp;", false, false)
        );
    }

    /// A comment or a lyric keeps its tabs and line feeds, and loses every
    /// other control character, as the core's multi-line rule says.
    ///
    /// Verifies: SEC-MED-077
    #[test]
    fn a_comment_keeps_tab_and_line_feed_and_loses_every_other_control() {
        assert_eq!(
            normalise_text(
                "a\u{0}b\tc\nd\re\u{1B}f\u{7F}g\u{85}h\u{9F}i\u{202E}j\u{2066}k",
                64,
                Lines::Multi
            ),
            mirror("ab\tc\ndefghi\u{202E}j\u{2066}k", false, false)
        );
    }

    /// The cap is the core's: it counts octets of UTF-8, keeps whole
    /// characters, and says when it cut.
    ///
    /// Verifies: SEC-MED-077
    #[test]
    fn text_past_the_cap_is_cut_at_a_whole_character_and_reported() {
        let four = "\u{E9}\u{E9}\u{E9}\u{E9}";
        let cases = [
            (0, "", true),
            (1, "", true),
            (2, "\u{E9}", true),
            (5, "\u{E9}\u{E9}", true),
            (7, "\u{E9}\u{E9}\u{E9}", true),
            (8, "\u{E9}\u{E9}\u{E9}\u{E9}", false),
            (u32::MAX, "\u{E9}\u{E9}\u{E9}\u{E9}", false),
        ];
        for (cap, value, truncated) in cases {
            for lines in [Lines::Single, Lines::Multi] {
                assert_eq!(
                    normalise_text(four, cap, lines),
                    mirror(value, truncated, false),
                    "{cap} {lines:?}"
                );
            }
        }
        // Characters the rule removes do not count toward the cap.
        assert_eq!(
            normalise_text("a\tb\tc", 3, Lines::Single),
            mirror("abc", false, false)
        );
        assert_eq!(
            normalise_text("a\tb\tc", 3, Lines::Multi),
            mirror("a\tb", true, false)
        );
    }

    /// A replacement character that arrives as text is text: nothing was
    /// replaced here, so nothing is reported.
    #[test]
    fn a_replacement_character_in_the_input_is_kept_and_not_reported() {
        assert_eq!(
            normalise_text("a\u{FFFD}b", 64, Lines::Single),
            mirror("a\u{FFFD}b", false, false)
        );
        assert_eq!(
            normalise_text("", 64, Lines::Multi),
            mirror("", false, false)
        );
    }

    #[test]
    fn the_declarations_are_the_literal_typescript() {
        assert_eq!(
            DECLARATIONS,
            [
                "/**\n * Whether a field may hold more than one line.\n */\n\
                 export type Lines = \"Single\" | \"Multi\";",
                "/**\n * Text that is safe to store and to show as text.\n */\n\
                 export interface NormalisedText {\n    \
                 /**\n     * The text.\n     */\n    \
                 value: string;\n    \
                 /**\n     * Whether characters past the cap were dropped, so that the value is a prefix of the whole text.\n     */\n    \
                 truncated: boolean;\n    \
                 /**\n     * Whether the input held octets that were not UTF-8, now replaced with U+FFFD.\n     */\n    \
                 replaced: boolean;\n}",
            ]
        );
    }
}
