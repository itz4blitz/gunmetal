//! The pieces of HTTP field syntax both header parsers share (RFC 9110,
//! section 5.6): optional whitespace, tokens, and splitting on a separator.
//!
//! Nothing here is public outside the core: the parsers in [`super::range`]
//! and [`super::forwarded`] are the entry points, and their fuzz harnesses
//! reach every line of this file through them.

/// Whether `byte` is optional whitespace (`OWS`): a space or a horizontal
/// tab.
pub(crate) const fn is_ows(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t')
}

/// The token characters that are neither letters nor digits.
const TCHAR_SYMBOLS: &[u8] = b"!#$%&'*+-.^_`|~";

/// Whether `byte` may appear in a token (`tchar`).
pub(crate) fn is_tchar(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || TCHAR_SYMBOLS.contains(&byte)
}

/// Whether `bytes` is a token: one or more token characters.
pub(crate) fn is_token(bytes: &[u8]) -> bool {
    !bytes.is_empty() && bytes.iter().all(|&byte| is_tchar(byte))
}

/// `bytes` without its leading and trailing optional whitespace, and how
/// many octets of whitespace led.
pub(crate) fn trim(bytes: &[u8]) -> (u64, &[u8]) {
    let lead = bytes.iter().take_while(|&&byte| is_ows(byte)).count();
    let rest = bytes.get(lead..).unwrap_or_default();
    let trail = rest.iter().rev().take_while(|&&byte| is_ows(byte)).count();
    let kept = rest.len().saturating_sub(trail);
    (count(lead), rest.get(..kept).unwrap_or_default())
}

/// `bytes` before and after its first `separator`, or `None` when it holds
/// none.
pub(crate) fn split_once(bytes: &[u8], separator: u8) -> Option<(&[u8], &[u8])> {
    let at = bytes.iter().position(|&byte| byte == separator)?;
    let before = bytes.get(..at).unwrap_or_default();
    let after = bytes.get(at.saturating_add(1)..).unwrap_or_default();
    Some((before, after))
}

/// A length or count as a `u64`, which every offset and limit in the core
/// uses. No length in memory is longer, so nothing saturates.
pub(crate) fn count(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The token characters, written out from RFC 9110, section 5.6.2:
    /// `!#$%&'*+-.^_` and backtick, `|`, `~`, digits and letters.
    fn tchars() -> Vec<u8> {
        let mut tchars: Vec<u8> = b"!#$%&'*+-.^_`|~".to_vec();
        tchars.extend(b'0'..=b'9');
        tchars.extend(b'A'..=b'Z');
        tchars.extend(b'a'..=b'z');
        tchars
    }

    #[test]
    fn whitespace_is_space_and_tab_only() {
        let ows: Vec<u8> = (0..=u8::MAX).filter(|&byte| is_ows(byte)).collect();
        assert_eq!(ows, b"\t ");
    }

    #[test]
    fn token_characters_are_exactly_those_of_rfc_9110() {
        let mut expected = tchars();
        expected.sort_unstable();
        let actual: Vec<u8> = (0..=u8::MAX).filter(|&byte| is_tchar(byte)).collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn a_token_is_one_or_more_token_characters() {
        let cases: [(&[u8], bool); 9] = [
            (b"", false),
            (b"bytes", true),
            (b"x", true),
            (b"!#$%&'*+-.^_`|~09AZaz", true),
            (b"by tes", false),
            (b"bytes ", false),
            (b"\"bytes\"", false),
            (b"a=b", false),
            (b"caf\xc3\xa9", false),
        ];
        for (bytes, token) in cases {
            assert_eq!(is_token(bytes), token, "{bytes:?}");
        }
    }

    #[test]
    fn trims_whitespace_at_both_ends_and_counts_what_led() {
        let cases: [(&[u8], u64, &[u8]); 8] = [
            (b"", 0, b""),
            (b" ", 1, b""),
            (b" \t \t", 4, b""),
            (b"a", 0, b"a"),
            (b" a", 1, b"a"),
            (b"a\t", 0, b"a"),
            (b"\t a b \t", 2, b"a b"),
            (b"\na\r", 0, b"\na\r"),
        ];
        for (bytes, lead, rest) in cases {
            assert_eq!(trim(bytes), (lead, rest), "{bytes:?}");
        }
    }

    /// Text and the halves it splits into.
    type Split<'a> = (&'a [u8], Option<(&'a [u8], &'a [u8])>);

    #[test]
    fn splits_at_the_first_separator_only() {
        let cases: [Split<'_>; 6] = [
            (b"", None),
            (b"abc", None),
            (b"a-b", Some((b"a", b"b"))),
            (b"-b", Some((b"", b"b"))),
            (b"a-", Some((b"a", b""))),
            (b"a-b-c", Some((b"a", b"b-c"))),
        ];
        for (bytes, halves) in cases {
            assert_eq!(split_once(bytes, b'-'), halves, "{bytes:?}");
        }
    }

    #[test]
    fn counts_lengths_exactly() {
        assert_eq!([0, 1, 16_384].map(count), [0, 1, 16_384]);
    }
}
