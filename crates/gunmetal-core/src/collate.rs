//! Search folding and natural-order sort keys (MUS-020, DIS-085).
//!
//! [`fold`] is the matching form of a string: case, canonical and
//! compatibility decomposition (diacritics, width, ligatures), then
//! punctuation removed. Kana voicing marks and Hangul syllables are kept,
//! so が and か stay distinct (MUS-020); transliteration is DIS-092 (R2).
//! [`sort_key`] builds a total order that honours a sort-name tag when one
//! is present, strips one leading article for [`Lang`], and compares digit
//! runs as numbers so "Track 2" precedes "Track 10". The alphabet jump
//! letter is WP-147.

use std::cmp::Ordering;

use unicode_normalization::char::{decompose_compatible, is_combining_mark};

/// The language whose leading-article list [`sort_key`] applies when no
/// sort-name tag is used, or when the tag itself begins with an article.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lang {
    /// Danish: *en*, *et*, *den*, *det*, *de*.
    Danish,
    /// Dutch: *de*, *het*, *een*, *'t*.
    Dutch,
    /// English: *a*, *an*, *the*.
    English,
    /// French: *le*, *la*, *les*, *un*, *une*, *des*, *l'*.
    French,
    /// German: *der*, *die*, *das*, *den*, *dem*, *des*, *ein* and the
    /// inflected *eine-* forms.
    German,
    /// Greek: the nominative articles, including *το* and *τα*.
    Greek,
    /// Italian: *il*, *lo*, *la*, *i*, *gli*, *le*, *un*, *uno*, *una*,
    /// *l'*.
    Italian,
    /// No articles: Japanese, Chinese, Korean, Russian, and unknown.
    None,
    /// Norwegian: *en*, *ei*, *et*, *den*, *det*, *de*.
    Norwegian,
    /// Portuguese: *o*, *a*, *os*, *as*, *um*, *uma*, *uns*, *umas*.
    Portuguese,
    /// Spanish: *el*, *la*, *los*, *las*, *un*, *una*, *unos*, *unas*.
    Spanish,
    /// Swedish: *en*, *ett*, *den*, *det*, *de*.
    Swedish,
}

impl Lang {
    /// Every language this module has an article list for, including
    /// [`Self::None`].
    pub const ALL: &[Self] = &[
        Self::Danish,
        Self::Dutch,
        Self::English,
        Self::French,
        Self::German,
        Self::Greek,
        Self::Italian,
        Self::None,
        Self::Norwegian,
        Self::Portuguese,
        Self::Spanish,
        Self::Swedish,
    ];
}

/// A comparable key for one title, artist or album name.
///
/// Two keys compare equal when they fold to the same letters and the same
/// numeric values, so "Track 2" and "Track 02" sit together. The order is
/// a total order: [`Eq`] and [`Ord`] agree, and exactly one of `<`, `==`
/// and `>` holds for every pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortKey {
    parts: Vec<Part>,
}

/// One run of folded text or one decimal number, in the order they appear.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Part {
    Number(String),
    Text(String),
}

/// Folds `s` for search: case, diacritics, width, ligatures and
/// punctuation.
///
/// Combining marks are dropped after compatibility decomposition, so
/// "Björk" and "Bjork" match, except the kana voicing marks U+3099 and
/// U+309A, which keep が and か distinct. Hangul syllables are not
/// decomposed. Typographic ligatures and fullwidth Latin become ASCII.
/// Punctuation, including both straight and curly apostrophes, is dropped,
/// and whitespace collapses to single spaces.
#[must_use]
pub fn fold(s: &str) -> String {
    let mut out = String::new();
    let mut pending_space = false;
    for_each_folded(s, |c| {
        if c.is_whitespace() {
            pending_space = !out.is_empty();
            return;
        }
        if is_kana_voicing(c) {
            out.push(c);
            return;
        }
        if is_apostrophe(c) || !c.is_alphanumeric() {
            return;
        }
        if pending_space {
            out.push(' ');
            pending_space = false;
        }
        out.push(c);
    });
    out
}

/// The sort key for `display`, using `sort_tag` when it holds a non-empty
/// name, then stripping one leading article for `lang` and comparing digit
/// runs as numbers.
#[must_use]
pub fn sort_key(display: &str, sort_tag: Option<&str>, lang: Lang) -> SortKey {
    let tagged = sort_tag.map(str::trim).filter(|s| !s.is_empty());
    let source = match tagged {
        Some(tag) => tag,
        None => display,
    };
    let source = strip_leading_article(source, lang);
    SortKey {
        parts: parts(source),
    }
}

impl PartialOrd for SortKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SortKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.parts.cmp(&other.parts)
    }
}

impl PartialOrd for Part {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Part {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Number(left), Self::Number(right)) => cmp_digits(left, right),
            (Self::Text(left), Self::Text(right)) => left.cmp(right),
            (Self::Number(_), Self::Text(_)) => Ordering::Less,
            (Self::Text(_), Self::Number(_)) => Ordering::Greater,
        }
    }
}

/// Decimal digit runs compared by numeric value. The stored digits have
/// already had leading zeros stripped.
fn cmp_digits(left: &str, right: &str) -> Ordering {
    left.len().cmp(&right.len()).then_with(|| left.cmp(right))
}

/// `digits` without leading zeros; a run of zeros is `"0"`.
fn significant(digits: &str) -> &str {
    let trimmed = digits.trim_start_matches('0');
    if trimmed.is_empty() { "0" } else { trimmed }
}

/// Compatibility-decomposed, case-folded characters of `s`, without
/// combining marks other than kana voicing. Hangul syllables stay composed.
fn for_each_folded(s: &str, mut emit: impl FnMut(char)) {
    for c in s.chars() {
        if is_hangul_syllable(c) {
            emit(c);
            continue;
        }
        decompose_compatible(c, |piece| {
            if is_kana_voicing(piece) {
                emit(piece);
                return;
            }
            if is_combining_mark(piece) {
                return;
            }
            fold_case(piece, &mut emit);
        });
    }
}

fn is_hangul_syllable(c: char) -> bool {
    matches!(c, '\u{AC00}'..='\u{D7A3}')
}

fn is_kana_voicing(c: char) -> bool {
    matches!(c, '\u{3099}' | '\u{309A}')
}

/// Default Unicode case folding for one already-decomposed character.
fn fold_case(c: char, mut emit: impl FnMut(char)) {
    for lower in c.to_lowercase() {
        match lower {
            'ß' => {
                emit('s');
                emit('s');
            }
            'ς' => emit('σ'),
            other => emit(other),
        }
    }
}

fn is_apostrophe(c: char) -> bool {
    matches!(c, '\'' | '\u{2018}' | '\u{2019}' | '\u{02BC}')
}

/// Leading-article lists, already case-folded. A match requires a word
/// boundary, so a shorter article cannot steal a longer one that shares
/// its prefix.
fn articles(lang: Lang) -> &'static [&'static str] {
    match lang {
        Lang::Danish => &["de", "den", "det", "en", "et"],
        Lang::Dutch => &["'t", "de", "een", "het"],
        Lang::English => &["a", "an", "the"],
        Lang::French => &["des", "l'", "la", "le", "les", "un", "une"],
        Lang::German => &[
            "das", "dem", "den", "der", "des", "die", "ein", "eine", "einem", "einen", "einer",
            "eines",
        ],
        Lang::Greek => &[
            "η", "ο", "τα", "τισ", "το", "τον", "του", "τουσ", "των", "τη", "την", "οι",
        ],
        Lang::Italian => &["gli", "i", "il", "l'", "la", "le", "lo", "un", "una", "uno"],
        Lang::None => &[],
        Lang::Norwegian => &["de", "den", "det", "ei", "en", "et"],
        Lang::Portuguese => &["a", "as", "o", "os", "um", "uma", "umas", "uns"],
        Lang::Spanish => &["el", "la", "las", "los", "un", "una", "unas", "unos"],
        Lang::Swedish => &["de", "den", "det", "en", "ett"],
    }
}

fn strip_leading_article(s: &str, lang: Lang) -> &str {
    let s = s.trim_start();
    for article in articles(lang) {
        if let Some(rest) = match_article(s, article) {
            if has_sort_content(rest) {
                return rest.trim_start();
            }
        }
    }
    s
}

/// When the folded prefix of `s` is exactly `article` and the next
/// character is a word boundary, the remainder after that prefix.
fn match_article<'a>(s: &'a str, article: &str) -> Option<&'a str> {
    let mut folded = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        fold_for_article(c, |piece| folded.push(piece));
        if !article.starts_with(folded.as_str()) {
            return None;
        }
        if folded == article {
            let rest = chars.as_str();
            return article_boundary(article, rest).then_some(rest);
        }
    }
    None
}

fn fold_for_article(c: char, mut emit: impl FnMut(char)) {
    decompose_compatible(c, |piece| {
        if is_combining_mark(piece) {
            return;
        }
        fold_case(piece, |folded| {
            if is_apostrophe(folded) {
                emit('\'');
            } else {
                emit(folded);
            }
        });
    });
}

/// An article that ends with an apostrophe already includes its boundary;
/// any other article must sit at the end or before a non-letter. Combining
/// marks left by a prior decomposition are not a boundary.
fn article_boundary(article: &str, rest: &str) -> bool {
    if article.ends_with('\'') {
        return true;
    }
    match first_folded(rest) {
        None => true,
        Some(c) => !c.is_alphanumeric(),
    }
}

fn first_folded(s: &str) -> Option<char> {
    let mut first = None;
    for_each_folded(s, |c| {
        if first.is_none() {
            first = Some(c);
        }
    });
    first
}

/// Whether `s` still holds a letter, digit or apostrophe after folding, so
/// stripping an article would not empty the title ("The", "The The").
fn has_sort_content(s: &str) -> bool {
    let mut leftover = false;
    for_each_folded(s, |c| {
        leftover = leftover || c.is_alphanumeric();
    });
    leftover
}

fn parts(s: &str) -> Vec<Part> {
    let mut parts = Vec::new();
    let mut text = String::new();
    let mut digits = String::new();
    for_each_folded(s, |c| {
        if c.is_ascii_digit() {
            flush_text(&mut parts, &mut text);
            digits.push(c);
            return;
        }
        flush_digits(&mut parts, &mut digits);
        push_text_char(&mut text, c);
    });
    flush_text(&mut parts, &mut text);
    flush_digits(&mut parts, &mut digits);
    parts
}

fn push_text_char(text: &mut String, c: char) {
    if c.is_whitespace() {
        if !text.is_empty() && !text.ends_with(' ') {
            text.push(' ');
        }
        return;
    }
    if is_kana_voicing(c) {
        text.push(c);
        return;
    }
    if c.is_alphanumeric() && !is_apostrophe(c) {
        text.push(c);
    }
}

fn flush_text(parts: &mut Vec<Part>, text: &mut String) {
    let trimmed = text.trim();
    if !trimmed.is_empty() {
        parts.push(Part::Text(trimmed.to_owned()));
    }
    text.clear();
}

fn flush_digits(parts: &mut Vec<Part>, digits: &mut String) {
    if !digits.is_empty() {
        parts.push(Part::Number(significant(digits).to_owned()));
        digits.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn key(display: &str) -> SortKey {
        sort_key(display, None, Lang::English)
    }

    fn key_lang(display: &str, lang: Lang) -> SortKey {
        sort_key(display, None, lang)
    }

    fn arb_char() -> impl Strategy<Value = char> {
        prop_oneof![
            8 => any::<char>(),
            1 => prop_oneof![
                Just('\u{304C}'),
                Just('\u{3071}'),
                Just('\u{3099}'),
                Just('\u{309A}'),
                Just('\u{AC00}'),
                Just('\u{D55C}'),
                Just('\u{D7A3}'),
            ],
        ]
    }

    fn arb_text() -> impl Strategy<Value = String> {
        proptest::collection::vec(arb_char(), 0..48).prop_map(|chars| chars.into_iter().collect())
    }

    fn arb_lang() -> impl Strategy<Value = Lang> {
        proptest::sample::select(Lang::ALL.to_vec())
    }

    #[test]
    fn folds_bjork_with_and_without_a_diacritic() {
        assert_eq!(fold("Björk"), "bjork");
        assert_eq!(fold("Bjork"), "bjork");
        assert_eq!(fold("BJÖRK"), "bjork");
        assert_eq!(fold("bjork"), "bjork");
        assert_eq!(key("Björk"), key("Bjork"));
        assert_eq!(key("BJÖRK"), key("bjork"));
    }

    #[test]
    fn folds_amelie_with_and_without_an_accent() {
        assert_eq!(fold("Amélie"), "amelie");
        assert_eq!(fold("Amelie"), "amelie");
        assert_eq!(fold("AMÉLIE"), "amelie");
    }

    #[test]
    fn folds_punctuation_width_ligatures_and_apostrophes() {
        assert_eq!(fold("AC/DC"), "acdc");
        assert_eq!(fold("ACDC"), "acdc");
        assert_eq!(fold("don't"), "dont");
        assert_eq!(fold("don’t"), "dont");
        assert_eq!(fold("don\u{02BC}t"), "dont");
        assert_eq!(key("don\u{02BC}t"), key("dont"));
        assert_eq!(fold("‘quoted’"), "quoted");
        assert_eq!(fold("ﬁle"), "file");
        assert_eq!(fold("ﬃ"), "ffi");
        assert_eq!(fold("Ｂｊｏｒｋ"), "bjork");
        assert_eq!(fold("ＡＣ／ＤＣ"), "acdc");
        assert_eq!(fold("Straße"), "strasse");
        assert_eq!(fold("STRASSE"), "strasse");
        assert_eq!(fold("İstanbul"), "istanbul");
        assert_eq!(fold("istanbul"), "istanbul");
        assert_eq!(key("AC/DC"), key("ACDC"));
    }

    #[test]
    fn folds_whitespace_and_drops_a_title_of_only_punctuation() {
        assert_eq!(fold(""), "");
        assert_eq!(fold("   "), "");
        assert_eq!(fold("..."), "");
        assert_eq!(fold("***"), "");
        assert_eq!(fold("!@#"), "");
        assert_eq!(fold("  foo   bar  "), "foo bar");
        assert_eq!(fold("The The"), "the the");
        assert_eq!(key(""), key("..."));
        assert_eq!(key(""), key("***"));
    }

    #[test]
    fn folds_japanese_greek_and_cyrillic() {
        assert_eq!(fold("東京"), "東京");
        assert_eq!(fold("あいう"), "あいう");
        assert_eq!(fold("Άμλετ"), "αμλετ");
        assert_eq!(fold("Αμλετ"), "αμλετ");
        assert_eq!(fold("άμλετ"), "αμλετ");
        assert_eq!(fold("Ёлка"), "елка");
        assert_eq!(fold("Елка"), "елка");
        assert_eq!(fold("ёлка"), "елка");
        assert_eq!(key("Άμλετ"), key("Αμλετ"));
        assert_eq!(key("Ёлка"), key("Елка"));
        assert!(key("あ") < key("い"));
        assert!(key("Б") < key("В"));
        assert!(key("東京") < key("京都") || key("京都") < key("東京"));
        assert_ne!(key("東京"), key("京都"));
    }

    #[test]
    fn voiced_kana_and_hangul_syllables_keep_their_identity() {
        // NFKD of が is か + U+3099; of ぱ is は + U+309A. Dropping every
        // combining mark would make がき and かき compare equal (MUS-020).
        // Transliteration is DIS-092 (R2). Hangul syllables stay composed.
        assert_eq!(fold("が"), "か\u{3099}");
        assert_eq!(fold("か"), "か");
        assert_ne!(fold("が"), fold("か"));
        assert_eq!(fold("か\u{3099}"), "か\u{3099}");
        assert_eq!(fold("ぱ"), "は\u{309A}");
        assert_eq!(fold("は"), "は");
        assert_eq!(key("がき"), key("か\u{3099}き"));
        assert_ne!(key("がき"), key("かき"));
        assert_eq!(fold("한"), "한");
        assert_eq!(fold("가"), "가");
        assert_eq!(fold("힣"), "힣");
        assert_ne!(fold("한"), "\u{1112}\u{1161}\u{11AB}");
    }

    #[test]
    fn greek_final_sigma_folds_to_sigma() {
        assert_eq!(fold("Ὀδυσσεύς"), fold("οδυσσευσ"));
        assert_eq!(fold("ς"), "σ");
        assert_eq!(fold("Σ"), "σ");
    }

    #[test]
    fn the_beatles_sorts_under_b() {
        assert_eq!(key("The Beatles"), key("Beatles"));
        assert_eq!(key("the beatles"), key("Beatles"));
        assert_eq!(key("THE BEATLES"), key("Beatles"));
        assert!(key("Arcade Fire") < key("The Beatles"));
        assert!(key("The Beatles") < key("Blur"));
        assert!(key("The Beatles") < key("The Chemical Brothers"));
    }

    #[test]
    fn the_the_strips_only_the_leading_article() {
        assert_eq!(key("The The"), key("The"));
        assert_ne!(key("The The"), key(""));
        assert!(key("Talking Heads") < key("The The"));
        assert!(key("The The") < key("This Heat"));
        assert_eq!(key("The   "), key("The"));
        assert_ne!(key("The   "), key(""));
    }

    #[test]
    fn an_is_not_stripped_as_a() {
        assert_eq!(key("An Album"), key("Album"));
        assert_ne!(key("An Album"), key("n Album"));
        assert_eq!(key("A Tribe Called Quest"), key("Tribe Called Quest"));
        assert_eq!(key("Theatre of Tragedy"), key("Theatre of Tragedy"));
        assert_ne!(key("Theatre of Tragedy"), key("atre of Tragedy"));
    }

    #[test]
    fn each_language_strips_its_own_article_and_not_anothers() {
        assert_eq!(
            key_lang("Die Ärzte", Lang::German),
            key_lang("Ärzte", Lang::German)
        );
        assert_ne!(
            key_lang("Die Ärzte", Lang::English),
            key_lang("Ärzte", Lang::English)
        );
        assert_eq!(
            key_lang("Les Rita Mitsouko", Lang::French),
            key_lang("Rita Mitsouko", Lang::French)
        );
        assert_eq!(
            key_lang("L'Argent", Lang::French),
            key_lang("Argent", Lang::French)
        );
        assert_eq!(
            key_lang("Despacito", Lang::French),
            key_lang("Despacito", Lang::None)
        );
        assert_eq!(
            key_lang("L’Argent", Lang::French),
            key_lang("Argent", Lang::French)
        );
        assert_eq!(
            key_lang("Los Lobos", Lang::Spanish),
            key_lang("Lobos", Lang::Spanish)
        );
        assert_eq!(
            key_lang("Il Volo", Lang::Italian),
            key_lang("Volo", Lang::Italian)
        );
        assert_eq!(
            key_lang("Gli Animali", Lang::Italian),
            key_lang("Animali", Lang::Italian)
        );
        assert_eq!(
            key_lang("Het Zweet", Lang::Dutch),
            key_lang("Zweet", Lang::Dutch)
        );
        assert_eq!(
            key_lang("Os Mutantes", Lang::Portuguese),
            key_lang("Mutantes", Lang::Portuguese)
        );
        assert_eq!(
            key_lang("Ett Dagsverke", Lang::Swedish),
            key_lang("Dagsverke", Lang::Swedish)
        );
        assert_eq!(
            key_lang("Ei Dame", Lang::Norwegian),
            key_lang("Dame", Lang::Norwegian)
        );
        assert_eq!(
            key_lang("Et Barn", Lang::Danish),
            key_lang("Barn", Lang::Danish)
        );
        assert_eq!(
            key_lang("Το Άλμπουμ", Lang::Greek),
            key_lang("Άλμπουμ", Lang::Greek)
        );
        assert_eq!(
            key_lang("Οι Επιτυχίες", Lang::Greek),
            key_lang("Επιτυχίες", Lang::Greek)
        );
        assert_eq!(
            key_lang("The Beatles", Lang::None),
            key_lang("The Beatles", Lang::None)
        );
        assert_ne!(
            key_lang("The Beatles", Lang::None),
            key_lang("Beatles", Lang::None)
        );
        assert_eq!(
            key_lang("東京", Lang::None),
            key_lang("東京", Lang::English)
        );
    }

    #[test]
    fn a_sort_name_tag_replaces_the_display_name() {
        let tagged = sort_key("The Beatles", Some("Beatles, The"), Lang::English);
        assert_eq!(tagged, key("Beatles The"));
        assert!(tagged < key("Blur"));
        assert!(sort_key("Zebra", Some("AAA"), Lang::English) < key("Aardvark"));
        assert_eq!(
            sort_key("Ignored", Some("The Beatles"), Lang::English),
            key("Beatles")
        );
        assert_eq!(
            sort_key("The Beatles", Some(""), Lang::English),
            key("Beatles")
        );
        assert_eq!(
            sort_key("The Beatles", Some("   "), Lang::English),
            key("Beatles")
        );
        assert_eq!(sort_key("The Beatles", None, Lang::English), key("Beatles"));
    }

    #[test]
    fn natural_order_puts_track_2_before_track_10() {
        assert!(key("Track 2") < key("Track 10"));
        assert!(key("Track 02") < key("Track 10"));
        assert_eq!(key("Track 2"), key("Track 02"));
        assert!(key("Part 2") < key("Part 10"));
        assert!(key("Vol. 2") < key("Vol. 10"));
        assert!(key("a2") < key("a10"));
        assert!(key("a2b") < key("a10b"));
        assert!(key("2") < key("10"));
        assert!(key("2") < key("a"));
        assert!(key("") < key("a"));
        assert!(key("...") < key("a"));
        assert!(key("Track 2") < key("Track 2b"));
        assert_eq!(key("1.2"), key("1.02"));
        assert!(key("1.2") < key("1.10"));
        assert_ne!(key("xy z"), key("xyz"));
        assert_eq!(key("xy  z"), key("xy z"));
        assert!(key("xy z") < key("xyz"));
    }

    #[test]
    fn numbers_with_only_zeros_compare_equal() {
        assert_eq!(key("0"), key("00"));
        assert_eq!(key("0"), key("000"));
        assert!(key("0") < key("1"));
        assert_eq!(
            Part::Number("2".into()).partial_cmp(&Part::Number("10".into())),
            Some(Ordering::Less)
        );
        assert_eq!(
            Part::Number("2".into()).partial_cmp(&Part::Text("a".into())),
            Some(Ordering::Less)
        );
        assert_eq!(
            Part::Text("a".into()).partial_cmp(&Part::Number("2".into())),
            Some(Ordering::Greater)
        );
    }

    #[test]
    fn catalog_numbers_longer_than_machine_integers_compare_as_digit_strings() {
        // 42- and 43-digit catalog numbers overflow u128 (~39 digits) and
        // round as f64. Significant-digit strings keep order and leading-zero
        // equality without parse/unwrap on the production path.
        const TWO: &str = concat!("1", "0000000000000000000000000000000000000000", "2");
        const TEN: &str = concat!("1", "0000000000000000000000000000000000000000", "10");
        const TWO_PADDED: &str = concat!("01", "0000000000000000000000000000000000000000", "2");
        const BARE_TWO: &str = concat!("0000000000000000000000000000000000000000", "2");
        const BARE_TEN: &str = concat!("0000000000000000000000000000000000000000", "10");
        assert_eq!(TWO.len(), 42);
        assert_eq!(TEN.len(), 43);
        assert!(key(TWO) < key(TEN));
        assert_eq!(key(TWO), key(TWO_PADDED));
        assert_eq!(key(BARE_TWO), key("2"));
        assert_eq!(key(BARE_TWO), key("02"));
        assert!(key(BARE_TWO) < key(BARE_TEN));
    }

    #[test]
    fn every_language_has_an_article_list() {
        assert_eq!(Lang::ALL.len(), 12);
        assert!(articles(Lang::None).is_empty());
        assert!(!articles(Lang::English).is_empty());
        for lang in Lang::ALL {
            let _ = articles(*lang);
            let _ = sort_key("The Title", None, *lang);
        }
    }

    #[test]
    fn documented_article_lists_match_the_literals() {
        // Independent copy of the documented lists. cargo-mutants does not
        // mutate string literals, so each string also has a strip case.
        assert_eq!(articles(Lang::Danish), ["de", "den", "det", "en", "et"]);
        assert_eq!(articles(Lang::Dutch), ["'t", "de", "een", "het"]);
        assert_eq!(articles(Lang::English), ["a", "an", "the"]);
        assert_eq!(
            articles(Lang::French),
            ["des", "l'", "la", "le", "les", "un", "une"]
        );
        assert_eq!(
            articles(Lang::German),
            [
                "das", "dem", "den", "der", "des", "die", "ein", "eine", "einem", "einen", "einer",
                "eines",
            ]
        );
        assert_eq!(
            articles(Lang::Greek),
            [
                "η", "ο", "τα", "τισ", "το", "τον", "του", "τουσ", "των", "τη", "την", "οι",
            ]
        );
        assert_eq!(
            articles(Lang::Italian),
            ["gli", "i", "il", "l'", "la", "le", "lo", "un", "una", "uno"]
        );
        assert_eq!(articles(Lang::None), [] as [&str; 0]);
        assert_eq!(
            articles(Lang::Norwegian),
            ["de", "den", "det", "ei", "en", "et"]
        );
        assert_eq!(
            articles(Lang::Portuguese),
            ["a", "as", "o", "os", "um", "uma", "umas", "uns"]
        );
        assert_eq!(
            articles(Lang::Spanish),
            ["el", "la", "las", "los", "un", "una", "unas", "unos"]
        );
        assert_eq!(articles(Lang::Swedish), ["de", "den", "det", "en", "ett"]);
    }

    /// One positive strip per documented article, including Dutch `'t`,
    /// Italian `l'`, German *eine-* forms and Portuguese *o*/*a*.
    const ARTICLE_STRIP_CASES: &[(Lang, &str, &str)] = &[
        (Lang::Danish, "De Unge", "Unge"),
        (Lang::Danish, "Den Gale", "Gale"),
        (Lang::Danish, "Det Ny", "Ny"),
        (Lang::Danish, "En Aften", "Aften"),
        (Lang::Danish, "Et Barn", "Barn"),
        (Lang::Dutch, "'t Zweet", "Zweet"),
        (Lang::Dutch, "De Jeugd", "Jeugd"),
        (Lang::Dutch, "Een Huis", "Huis"),
        (Lang::Dutch, "Het Zweet", "Zweet"),
        (Lang::English, "A Tribe Called Quest", "Tribe Called Quest"),
        (Lang::English, "An Album", "Album"),
        (Lang::English, "The Beatles", "Beatles"),
        (Lang::French, "Des Airs", "Airs"),
        (Lang::French, "L'Argent", "Argent"),
        (Lang::French, "La Roux", "Roux"),
        (Lang::French, "Le Tigre", "Tigre"),
        (Lang::French, "Les Rita Mitsouko", "Rita Mitsouko"),
        (Lang::French, "Un Homme", "Homme"),
        (Lang::French, "Une Nuit", "Nuit"),
        (Lang::German, "Das Boot", "Boot"),
        (Lang::German, "Dem Himmel", "Himmel"),
        (Lang::German, "Den Sternen", "Sternen"),
        (Lang::German, "Der Mond", "Mond"),
        (Lang::German, "Des Lebens", "Lebens"),
        (Lang::German, "Die Ärzte", "Ärzte"),
        (Lang::German, "Ein Lied", "Lied"),
        (Lang::German, "Eine Nacht", "Nacht"),
        (Lang::German, "Einem Freund", "Freund"),
        (Lang::German, "Einen Tag", "Tag"),
        (Lang::German, "Einer Frau", "Frau"),
        (Lang::German, "Eines Morgens", "Morgens"),
        (Lang::Greek, "Η Σιωπή", "Σιωπή"),
        (Lang::Greek, "Ο Δίσκος", "Δίσκος"),
        (Lang::Greek, "Τα Τραγούδια", "Τραγούδια"),
        (Lang::Greek, "Τις Μέρες", "Μέρες"),
        (Lang::Greek, "Το Άλμπουμ", "Άλμπουμ"),
        (Lang::Greek, "Τον Άνθρωπο", "Άνθρωπο"),
        (Lang::Greek, "Του Χρόνου", "Χρόνου"),
        (Lang::Greek, "Τους Φίλους", "Φίλους"),
        (Lang::Greek, "Των Αστέρων", "Αστέρων"),
        (Lang::Greek, "Τη Νύχτα", "Νύχτα"),
        (Lang::Greek, "Την Αγάπη", "Αγάπη"),
        (Lang::Greek, "Οι Επιτυχίες", "Επιτυχίες"),
        (Lang::Italian, "Gli Animali", "Animali"),
        (Lang::Italian, "I Pooh", "Pooh"),
        (Lang::Italian, "Il Volo", "Volo"),
        (Lang::Italian, "L'Italiano", "Italiano"),
        (Lang::Italian, "La Voce", "Voce"),
        (Lang::Italian, "Le Orme", "Orme"),
        (Lang::Italian, "Lo Stato", "Stato"),
        (Lang::Italian, "Un Uomo", "Uomo"),
        (Lang::Italian, "Una Notte", "Notte"),
        (Lang::Italian, "Uno Studio", "Studio"),
        (Lang::Norwegian, "De Unge", "Unge"),
        (Lang::Norwegian, "Den Dag", "Dag"),
        (Lang::Norwegian, "Det Hus", "Hus"),
        (Lang::Norwegian, "Ei Dame", "Dame"),
        (Lang::Norwegian, "En Natt", "Natt"),
        (Lang::Norwegian, "Et Hus", "Hus"),
        (Lang::Portuguese, "A Banda", "Banda"),
        (Lang::Portuguese, "As Flores", "Flores"),
        (Lang::Portuguese, "O Rappa", "Rappa"),
        (Lang::Portuguese, "Os Mutantes", "Mutantes"),
        (Lang::Portuguese, "Um Dia", "Dia"),
        (Lang::Portuguese, "Uma Noite", "Noite"),
        (Lang::Portuguese, "Umas Horas", "Horas"),
        (Lang::Portuguese, "Uns Dias", "Dias"),
        (Lang::Spanish, "El Camino", "Camino"),
        (Lang::Spanish, "La Ley", "Ley"),
        (Lang::Spanish, "Las Ketchup", "Ketchup"),
        (Lang::Spanish, "Los Lobos", "Lobos"),
        (Lang::Spanish, "Un Día", "Día"),
        (Lang::Spanish, "Una Noche", "Noche"),
        (Lang::Spanish, "Unas Flores", "Flores"),
        (Lang::Spanish, "Unos Tipos", "Tipos"),
        (Lang::Swedish, "De Unga", "Unga"),
        (Lang::Swedish, "Den Blå", "Blå"),
        (Lang::Swedish, "Det Regnar", "Regnar"),
        (Lang::Swedish, "En Natt", "Natt"),
        (Lang::Swedish, "Ett Dagsverke", "Dagsverke"),
    ];

    #[test]
    fn every_documented_article_strips_once() {
        assert_eq!(ARTICLE_STRIP_CASES.len(), 80);
        for (lang, title, rest) in ARTICLE_STRIP_CASES {
            assert_eq!(
                key_lang(title, *lang),
                key_lang(rest, *lang),
                "{title} under {lang:?}"
            );
            assert_ne!(
                key_lang(title, *lang),
                key_lang(title, Lang::None),
                "{title} must strip under {lang:?}"
            );
        }
    }

    #[test]
    fn amor_imagine_and_arcade_do_not_strip() {
        for lang in Lang::ALL {
            for title in ["Amor", "Imagine", "Arcade"] {
                assert_eq!(
                    key_lang(title, *lang),
                    key_lang(title, Lang::None),
                    "{title} under {lang:?}"
                );
            }
        }
    }

    proptest! {
        #[test]
        fn fold_is_idempotent(s in arb_text()) {
            let once = fold(&s);
            prop_assert_eq!(fold(&once), once);
        }

        #[test]
        fn fold_never_holds_punctuation_or_folded_marks(s in arb_text()) {
            let folded = fold(&s);
            let letters_spaces_or_voicing = folded
                .chars()
                .all(|c| c.is_alphanumeric() || c == ' ' || is_kana_voicing(c));
            prop_assert!(letters_spaces_or_voicing);
            prop_assert!(!folded.starts_with(' '));
            prop_assert!(!folded.ends_with(' '));
            prop_assert!(!folded.contains("  "));
            let only_kept_marks = folded
                .chars()
                .all(|c| !is_combining_mark(c) || is_kana_voicing(c));
            prop_assert!(only_kept_marks);
        }

        #[test]
        fn sort_key_is_a_total_order(
            a in arb_text(),
            b in arb_text(),
            c in arb_text(),
            lang in arb_lang(),
        ) {
            let ka = sort_key(&a, None, lang);
            let kb = sort_key(&b, None, lang);
            let kc = sort_key(&c, None, lang);
            let ab = ka.cmp(&kb);
            let ba = kb.cmp(&ka);
            match ab {
                Ordering::Equal => {
                    prop_assert_eq!(&ka, &kb);
                    prop_assert_eq!(ba, Ordering::Equal);
                }
                Ordering::Less => prop_assert_eq!(ba, Ordering::Greater),
                Ordering::Greater => prop_assert_eq!(ba, Ordering::Less),
            }
            let bc = kb.cmp(&kc);
            let ac = ka.cmp(&kc);
            if ab == Ordering::Equal {
                prop_assert_eq!(ac, bc);
            }
            if ab == Ordering::Less && bc == Ordering::Less {
                prop_assert_eq!(ac, Ordering::Less);
            }
            if ab == Ordering::Greater && bc == Ordering::Greater {
                prop_assert_eq!(ac, Ordering::Greater);
            }
        }

        /// Compatibility decomposition and case folding of the input do not
        /// change the key, except Hangul syllables stay composed.
        #[test]
        fn sort_key_is_stable_under_folding(
            s in arb_text(),
            lang in arb_lang(),
        ) {
            let decomposed: String = {
                let mut out = String::new();
                for c in s.chars() {
                    if matches!(c, '\u{AC00}'..='\u{D7A3}') {
                        out.push(c);
                        continue;
                    }
                    decompose_compatible(c, |piece| out.push(piece));
                }
                out
            };
            let lowered: String = decomposed.to_lowercase();
            prop_assert_eq!(
                sort_key(&s, None, lang),
                sort_key(&lowered, None, lang)
            );
        }

        #[test]
        fn a_sort_tag_is_stable_under_folding(
            display in arb_text(),
            tag in arb_text(),
            lang in arb_lang(),
        ) {
            let tag = if tag.trim().is_empty() {
                None
            } else {
                Some(tag.as_str())
            };
            let lowered_display = display.to_lowercase();
            let lowered_tag = tag.map(str::to_lowercase);
            prop_assert_eq!(
                sort_key(&display, tag, lang),
                sort_key(
                    &lowered_display,
                    lowered_tag.as_deref(),
                    lang
                )
            );
        }
    }
}
