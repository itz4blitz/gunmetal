//! Text from outside: lossy decoding, control removal and length caps
//! (SEC-MED-013, SEC-API-048).
//!
//! Every function here decodes first, then removes what the field may not
//! hold, then caps the length in UTF-8 octets. The cap comes last because
//! Latin-1 text can double in size when it becomes UTF-8. The result is
//! data to store and to show as text, never markup, a path, a URL to fetch
//! or a format string.

use std::borrow::Cow;

use crate::untrusted::Untrusted;

/// How the octets of a text field are encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    /// UTF-8.
    Utf8,
    /// UTF-16 that should start with a byte-order mark. Without one it is
    /// read as big-endian, as RFC 2781 section 4.3 says.
    Utf16Bom,
    /// UTF-16, big-endian.
    Utf16Be,
    /// UTF-16, little-endian.
    Utf16Le,
    /// ISO 8859-1, one octet per character.
    Latin1,
}

/// Whether a field may hold more than one line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lines {
    /// A name, title or label: no control characters at all, and no
    /// bidirectional embedding, override or isolate, or line or paragraph
    /// separator, so the text cannot reorder or break what is shown around
    /// it (CVE-2021-42574).
    Single,
    /// A comment, description or lyric: tab and line feed stay.
    Multi,
}

/// Decoded text, safe to store and to show as text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Text {
    /// The text.
    pub value: String,
    /// Whether characters past the cap were dropped. Everything after the
    /// first character that did not fit is dropped, so the value is always
    /// a prefix of the whole text.
    pub truncated: bool,
    /// Whether the input held an invalid sequence, now replaced with
    /// U+FFFD.
    pub replaced: bool,
}

/// Decodes a text field from media metadata, removes the C0 and C1
/// control characters other than tab and line feed, and keeps at most
/// `cap` octets of UTF-8 (SEC-MED-013).
///
/// Invalid sequences, unpaired surrogates and a final odd octet of UTF-16
/// each become U+FFFD. One leading byte-order mark is dropped, whatever the
/// encoding.
#[must_use]
pub fn decode(bytes: Untrusted<&[u8]>, encoding: Encoding, cap: u32) -> Text {
    let bytes = bytes.into_inner();
    let (decoded, replaced) = match encoding {
        Encoding::Utf8 => utf8(bytes),
        Encoding::Utf16Bom => match bytes {
            [0xFF, 0xFE, rest @ ..] => utf16(rest, u16::from_le_bytes),
            _ => utf16(bytes, u16::from_be_bytes),
        },
        Encoding::Utf16Be => utf16(bytes, u16::from_be_bytes),
        Encoding::Utf16Le => utf16(bytes, u16::from_le_bytes),
        Encoding::Latin1 => (bytes.iter().copied().map(char::from).collect(), false),
    };
    let unmarked = decoded.strip_prefix('\u{FEFF}').unwrap_or(&decoded);
    clean(unmarked, Lines::Multi, cap, replaced)
}

/// Normalises a UTF-8 field received from a client or a provider: invalid
/// sequences replaced, the characters [`Lines`] forbids removed, and at
/// most `cap` octets kept (SEC-API-048).
///
/// Normalising the value again returns it unchanged.
#[must_use]
pub fn normalise(input: Untrusted<&[u8]>, lines: Lines, cap: u32) -> Text {
    let (decoded, replaced) = utf8(input.into_inner());
    clean(&decoded, lines, cap, replaced)
}

/// Decodes UTF-8, replacing each maximal invalid sequence.
fn utf8(bytes: &[u8]) -> (Cow<'_, str>, bool) {
    let decoded = String::from_utf8_lossy(bytes);
    let replaced = matches!(decoded, Cow::Owned(_));
    (decoded, replaced)
}

/// Decodes UTF-16 whose code units `unit` reads from pairs of octets.
fn utf16(bytes: &[u8], unit: fn([u8; 2]) -> u16) -> (Cow<'static, str>, bool) {
    let pairs = bytes.chunks_exact(2);
    let odd = !pairs.remainder().is_empty();
    let units = pairs.map(|pair| <[u8; 2]>::try_from(pair).map_or(0, unit));
    let mut decoded = String::new();
    let mut replaced = odd;
    for decoded_unit in char::decode_utf16(units) {
        if let Ok(c) = decoded_unit {
            decoded.push(c);
        } else {
            decoded.push(char::REPLACEMENT_CHARACTER);
            replaced = true;
        }
    }
    if odd {
        decoded.push(char::REPLACEMENT_CHARACTER);
    }
    (Cow::Owned(decoded), replaced)
}

/// Removes what `lines` forbids, then keeps whole characters up to `cap`
/// octets.
fn clean(decoded: &str, lines: Lines, cap: u32, replaced: bool) -> Text {
    let cap = usize::try_from(cap).unwrap_or(usize::MAX);
    let mut value = String::new();
    let mut truncated = false;
    for c in decoded.chars().filter(|&c| kept(c, lines)) {
        truncated = truncated || value.len().saturating_add(c.len_utf8()) > cap;
        if !truncated {
            value.push(c);
        }
    }
    Text {
        value,
        truncated,
        replaced,
    }
}

/// Whether a field of `lines` may hold `c`.
fn kept(c: char, lines: Lines) -> bool {
    match lines {
        Lines::Multi => !c.is_control() || matches!(c, '\t' | '\n'),
        Lines::Single => {
            !c.is_control() && !matches!(c, '\u{2028}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::collection::vec;
    use proptest::prelude::*;
    // Direct imports: Qodana does not resolve these macros through `prelude::*`.
    use proptest::sample::select;
    use proptest::{prop_oneof, proptest};

    fn text(value: &str, truncated: bool, replaced: bool) -> Text {
        Text {
            value: value.to_owned(),
            truncated,
            replaced,
        }
    }

    /// Decodes `bytes` as they arrive from a media file.
    fn decode(bytes: &[u8], encoding: Encoding, cap: u32) -> Text {
        super::decode(Untrusted::new(bytes), encoding, cap)
    }

    fn single(bytes: &[u8], cap: u32) -> Text {
        normalise(Untrusted::new(bytes), Lines::Single, cap)
    }

    fn multi(bytes: &[u8], cap: u32) -> Text {
        normalise(Untrusted::new(bytes), Lines::Multi, cap)
    }

    /// Every encoding, for the properties.
    const ENCODINGS: [Encoding; 5] = [
        Encoding::Utf8,
        Encoding::Utf16Bom,
        Encoding::Utf16Be,
        Encoding::Utf16Le,
        Encoding::Latin1,
    ];

    /// Characters the single-line rule removes although they are not
    /// controls: the bidirectional embeddings, overrides and isolates
    /// (CVE-2021-42574), and the line and paragraph separators. Written out
    /// here from the Unicode code charts, independently of the code.
    const SINGLE_LINE_REMOVED: [char; 11] = [
        '\u{2028}', '\u{2029}', '\u{202A}', '\u{202B}', '\u{202C}', '\u{202D}', '\u{202E}',
        '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}',
    ];

    /// Verifies: SEC-MED-013
    #[test]
    fn decodes_valid_text_in_every_encoding() {
        let cases: [(&[u8], Encoding); 6] = [
            (b"Caf\xC3\xA9 \xF0\x9D\x84\x9E", Encoding::Utf8),
            (
                b"\xFF\xFEC\0a\0f\0\xE9\0 \0\x34\xD8\x1E\xDD",
                Encoding::Utf16Bom,
            ),
            (
                b"\xFE\xFF\0C\0a\0f\0\xE9\0 \xD8\x34\xDD\x1E",
                Encoding::Utf16Bom,
            ),
            (b"\0C\0a\0f\0\xE9\0 \xD8\x34\xDD\x1E", Encoding::Utf16Be),
            (b"C\0a\0f\0\xE9\0 \0\x34\xD8\x1E\xDD", Encoding::Utf16Le),
            (b"Caf\xE9 \xA0\xFF", Encoding::Latin1),
        ];
        let expected = [
            "Café 𝄞",
            "Café 𝄞",
            "Café 𝄞",
            "Café 𝄞",
            "Café 𝄞",
            "Café \u{A0}ÿ",
        ];
        for ((bytes, encoding), value) in cases.into_iter().zip(expected) {
            assert_eq!(
                decode(bytes, encoding, 64),
                text(value, false, false),
                "{encoding:?} {bytes:02X?}"
            );
        }
    }

    /// Verifies: SEC-MED-013
    #[test]
    fn replaces_invalid_utf8_sequences() {
        assert_eq!(
            decode(b"a\xFFb\xC3(c\xE2\x82", Encoding::Utf8, 64),
            text("a\u{FFFD}b\u{FFFD}(c\u{FFFD}", false, true)
        );
    }

    /// Verifies: SEC-MED-013
    #[test]
    fn replaces_unpaired_surrogates_in_utf16() {
        let cases: [(&[u8], Encoding, &str); 4] = [
            (b"\xD8\x00\0a", Encoding::Utf16Be, "\u{FFFD}a"),
            (b"\0a\xDC\x00", Encoding::Utf16Be, "a\u{FFFD}"),
            (b"\0\xD8a\0", Encoding::Utf16Le, "\u{FFFD}a"),
            (
                b"\xFF\xFE\0\xDC\0\xD8",
                Encoding::Utf16Bom,
                "\u{FFFD}\u{FFFD}",
            ),
        ];
        for (bytes, encoding, value) in cases {
            assert_eq!(
                decode(bytes, encoding, 64),
                text(value, false, true),
                "{encoding:?} {bytes:02X?}"
            );
        }
    }

    /// Verifies: SEC-MED-013
    #[test]
    fn replaces_the_odd_octet_at_the_end_of_utf16() {
        let cases: [(&[u8], Encoding, &str); 4] = [
            (b"\0a\x42", Encoding::Utf16Be, "a\u{FFFD}"),
            (b"a\0\x42", Encoding::Utf16Le, "a\u{FFFD}"),
            (b"\xFF\xFEa\0\x42", Encoding::Utf16Bom, "a\u{FFFD}"),
            (b"\x42", Encoding::Utf16Be, "\u{FFFD}"),
        ];
        for (bytes, encoding, value) in cases {
            assert_eq!(
                decode(bytes, encoding, 64),
                text(value, false, true),
                "{encoding:?} {bytes:02X?}"
            );
        }
    }

    /// Verifies: SEC-MED-013
    #[test]
    fn drops_a_leading_byte_order_mark_and_keeps_one_inside() {
        let cases: [(&[u8], Encoding, &str); 8] = [
            (b"\xEF\xBB\xBF", Encoding::Utf8, ""),
            (b"\xEF\xBB\xBFa\xEF\xBB\xBF", Encoding::Utf8, "a\u{FEFF}"),
            (b"\xFF\xFE", Encoding::Utf16Bom, ""),
            (b"\xFE\xFF", Encoding::Utf16Bom, ""),
            // Without a mark, UTF-16 is big-endian (RFC 2781, section 4.3).
            (b"\0a", Encoding::Utf16Bom, "a"),
            (b"\xFE\xFF\0a", Encoding::Utf16Be, "a"),
            (b"\xFF\xFEa\0", Encoding::Utf16Le, "a"),
            (b"", Encoding::Utf16Bom, ""),
        ];
        for (bytes, encoding, value) in cases {
            assert_eq!(
                decode(bytes, encoding, 64),
                text(value, false, false),
                "{encoding:?} {bytes:02X?}"
            );
        }
    }

    /// Verifies: SEC-MED-013
    #[test]
    fn removes_c0_and_c1_controls_but_keeps_tab_and_line_feed() {
        assert_eq!(
            decode(b"a\0b\tc\nd\re\x1Bf\x7Fg\x1Fh", Encoding::Utf8, 64),
            text("ab\tc\ndefgh", false, false)
        );
        // In Latin-1, 0x80 to 0x9F are the C1 controls and 0xA0 is a space.
        assert_eq!(
            decode(b"a\x80\x85b\x9F\xA0", Encoding::Latin1, 64),
            text("ab\u{A0}", false, false)
        );
        assert_eq!(
            decode(b"\0a\0\x85\0\x9F\0\x0B\0b", Encoding::Utf16Be, 64),
            text("ab", false, false)
        );
    }

    /// Latin-1 grows to two UTF-8 octets per accented letter, so the cap
    /// counts the decoded text, not the input.
    ///
    /// Verifies: SEC-MED-013
    #[test]
    fn caps_the_decoded_text_not_the_input() {
        let bytes = b"\xE9\xE9\xE9\xE9";
        assert_eq!(decode(bytes, Encoding::Latin1, 5), text("éé", true, false));
        assert_eq!(decode(bytes, Encoding::Latin1, 6), text("ééé", true, false));
        assert_eq!(decode(bytes, Encoding::Latin1, 7), text("ééé", true, false));
        assert_eq!(
            decode(bytes, Encoding::Latin1, 8),
            text("éééé", false, false)
        );
    }

    /// Verifies: SEC-MED-013
    #[test]
    fn keeps_nothing_after_the_first_character_that_does_not_fit() {
        assert_eq!(
            decode(b"\xE9abc", Encoding::Latin1, 1),
            text("", true, false)
        );
        assert_eq!(decode(b"a", Encoding::Utf8, 0), text("", true, false));
        assert_eq!(decode(b"", Encoding::Utf8, 0), text("", false, false));
    }

    /// Verifies: SEC-MED-013
    #[test]
    fn counts_only_the_characters_it_keeps_against_the_cap() {
        assert_eq!(
            decode(b"a\0\0\0b", Encoding::Utf8, 2),
            text("ab", false, false)
        );
    }

    /// Verifies: SEC-API-048
    #[test]
    fn removes_controls_bidi_controls_and_separators_from_a_single_line() {
        let input = "a\tb\nc\r\u{202E}d\u{2066}e\u{2069}f\u{2028}g\u{2029}h\u{200E}i\u{85}j";
        assert_eq!(
            single(input.as_bytes(), 64),
            text("abcdefgh\u{200E}ij", false, false)
        );
    }

    /// The characters on either side of each removed range stay.
    ///
    /// Verifies: SEC-API-048
    #[test]
    fn keeps_the_neighbours_of_every_removed_range_in_a_single_line() {
        let kept = "\u{2027}\u{202F}\u{2065}\u{206A}\u{200F}\u{61C}";
        assert_eq!(single(kept.as_bytes(), 64), text(kept, false, false));
        let removed: String = SINGLE_LINE_REMOVED.iter().collect();
        assert_eq!(single(removed.as_bytes(), 64), text("", false, false));
    }

    /// Verifies: SEC-API-048
    #[test]
    fn keeps_tabs_line_feeds_and_bidi_controls_in_several_lines() {
        let input = "a\tb\nc\r\u{202E}d\u{2066}e\u{2028}f\0";
        assert_eq!(
            multi(input.as_bytes(), 64),
            text("a\tb\nc\u{202E}d\u{2066}e\u{2028}f", false, false)
        );
    }

    /// Verifies: SEC-API-048
    #[test]
    fn normalises_invalid_utf8_and_caps_the_result() {
        assert_eq!(single(b"ab\xFFcd", 5), text("ab\u{FFFD}", true, true));
        assert_eq!(multi(b"ab\xFF\ncd", 4), text("ab", true, true));
    }

    /// A byte-order mark in a field from a client is kept, so that
    /// normalising twice changes nothing.
    ///
    /// Verifies: SEC-API-048
    #[test]
    fn keeps_a_byte_order_mark_from_a_client() {
        assert_eq!(
            single(b"\xEF\xBB\xBF\xEF\xBB\xBFa", 64),
            text("\u{FEFF}\u{FEFF}a", false, false)
        );
    }

    /// Bytes that reach the interesting cases more often than uniform
    /// random bytes would: controls, separators, bidirectional controls,
    /// surrogate halves, byte-order marks and invalid sequences.
    fn hostile_bytes() -> impl Strategy<Value = Vec<u8>> {
        let piece = prop_oneof![
            any::<u8>().prop_map(|byte| vec![byte]),
            any::<char>().prop_map(|c| c.to_string().into_bytes()),
            select(SINGLE_LINE_REMOVED.to_vec()).prop_map(|c| c.to_string().into_bytes()),
            select(vec![
                b"\t".to_vec(),
                b"\n".to_vec(),
                b"\r".to_vec(),
                b"\0".to_vec(),
                b"\xC2\x85".to_vec(),
                b"\xEF\xBB\xBF".to_vec(),
                b"\xFF\xFE".to_vec(),
                b"\xFE\xFF".to_vec(),
                b"\xD8\x00".to_vec(),
                b"\x00\xDC".to_vec(),
                b"\xED\xA0\x80".to_vec(),
            ]),
        ];
        vec(piece, 0..24).prop_map(|pieces| pieces.concat())
    }

    /// What a field may hold after decoding: no control but tab and line
    /// feed, at most `cap` octets, and when truncated, no room for the
    /// next character.
    fn assert_within_cap(out: &Text, cap: u32) {
        let cap = usize::try_from(cap).unwrap();
        assert!(out.value.len() <= cap, "{out:?} over {cap}");
        assert!(
            out.value
                .chars()
                .all(|c| !c.is_control() || c == '\t' || c == '\n'),
            "{out:?} holds a control"
        );
    }

    proptest! {
        /// Verifies: SEC-MED-013
        #[test]
        fn decoded_text_holds_no_control_and_fits_the_cap(
            bytes in hostile_bytes(),
            encoding in select(ENCODINGS.to_vec()),
            cap in 0_u32..48,
        ) {
            let out = decode(&bytes, encoding, cap);
            assert_within_cap(&out, cap);
            let whole = decode(&bytes, encoding, u32::MAX);
            prop_assert!(whole.value.starts_with(&out.value));
            prop_assert_eq!(out.truncated, whole.value.len() > out.value.len());
            prop_assert_eq!(out.replaced, whole.replaced);
        }

        /// Verifies: SEC-API-048
        #[test]
        fn a_single_line_holds_no_control_bidi_control_or_separator(
            bytes in hostile_bytes(),
            cap in 0_u32..48,
        ) {
            let out = single(&bytes, cap);
            assert_within_cap(&out, cap);
            prop_assert!(
                out.value.chars().all(|c| c != '\t' && c != '\n' && !SINGLE_LINE_REMOVED.contains(&c)),
                "{:?}", out
            );
        }

        /// Verifies: SEC-API-048
        #[test]
        fn normalising_twice_changes_nothing(
            bytes in hostile_bytes(),
            lines in select(vec![Lines::Single, Lines::Multi]),
            cap in 0_u32..48,
        ) {
            let once = normalise(Untrusted::new(&bytes), lines, cap);
            let twice = normalise(Untrusted::new(once.value.as_bytes()), lines, cap);
            prop_assert_eq!(twice, text(&once.value, false, false));
        }

        /// Text from media agrees with an independent lossy decoder written
        /// here for UTF-8 and Latin-1.
        ///
        /// Verifies: SEC-MED-013
        #[test]
        fn agrees_with_a_reference_decoder(bytes in hostile_bytes()) {
            let keep = |c: &char| !c.is_control() || *c == '\t' || *c == '\n';
            let utf8 = String::from_utf8_lossy(&bytes);
            let utf8 = utf8.strip_prefix('\u{FEFF}').unwrap_or(&utf8);
            prop_assert_eq!(
                decode(&bytes, Encoding::Utf8, u32::MAX).value,
                utf8.chars().filter(keep).collect::<String>()
            );
            prop_assert_eq!(
                decode(&bytes, Encoding::Latin1, u32::MAX).value,
                bytes.iter().map(|&b| char::from(b)).filter(keep).collect::<String>()
            );
        }
    }
}
