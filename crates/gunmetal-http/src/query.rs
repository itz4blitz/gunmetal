//! Percent-decoding and query-string parsing.
//!
//! The request target is at most 4 KiB by the time a request gets here
//! (SEC-API-060, enforced by the listener), so both run in time linear in
//! a small input.

/// Decodes `%XX` escapes, and `+` as a space where `plus_is_space` is set.
/// Returns `None` for a malformed escape or a result that is not UTF-8.
pub(crate) fn percent_decode(text: &str, plus_is_space: bool) -> Option<String> {
    let mut out = Vec::with_capacity(text.len());
    let mut bytes = text.bytes();
    while let Some(byte) = bytes.next() {
        out.push(match byte {
            b'%' => {
                let high = hex(bytes.next()?)?;
                let low = hex(bytes.next()?)?;
                high * 16 + low
            }
            b'+' if plus_is_space => b' ',
            other => other,
        });
    }
    String::from_utf8(out).ok()
}

/// The value of one hexadecimal digit.
fn hex(digit: u8) -> Option<u8> {
    char::from(digit)
        .to_digit(16)
        .and_then(|value| u8::try_from(value).ok())
}

/// Splits a query string into decoded name and value pairs, in order.
/// Empty pieces (`a=1&&b=2`) are skipped; a piece without `=` has an empty
/// value. Returns `None` if any name or value does not decode.
pub(crate) fn parse_query(query: &str) -> Option<Vec<(String, String)>> {
    query
        .split('&')
        .filter(|piece| !piece.is_empty())
        .map(|piece| {
            let (name, value) = piece.split_once('=').unwrap_or((piece, ""));
            Some((percent_decode(name, true)?, percent_decode(value, true)?))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_escapes_and_plus() {
        assert_eq!(percent_decode("a%20b+c", true), Some("a b c".to_owned()));
        assert_eq!(percent_decode("a%20b+c", false), Some("a b+c".to_owned()));
        assert_eq!(percent_decode("%e2%82%AC", false), Some("€".to_owned()));
        assert_eq!(percent_decode("%7a%7A", false), Some("zz".to_owned()));
        assert_eq!(percent_decode("", false), Some(String::new()));
    }

    #[test]
    fn refuses_malformed_escapes_and_bad_utf8() {
        for text in ["%", "%4", "%g0", "%0g", "a%2", "%ff", "%c3"] {
            assert_eq!(percent_decode(text, true), None);
        }
    }

    #[test]
    fn parses_pairs_in_order() {
        assert_eq!(
            parse_query("limit=50&&cursor=a%2Bb&flag&q=two+words"),
            Some(vec![
                ("limit".to_owned(), "50".to_owned()),
                ("cursor".to_owned(), "a+b".to_owned()),
                ("flag".to_owned(), String::new()),
                ("q".to_owned(), "two words".to_owned()),
            ])
        );
        assert_eq!(parse_query(""), Some(Vec::new()));
        assert_eq!(
            parse_query("a=b=c"),
            Some(vec![("a".to_owned(), "b=c".to_owned())])
        );
    }

    #[test]
    fn refuses_a_query_with_a_bad_name_or_value() {
        assert_eq!(parse_query("a=1&b%zz=2"), None);
        assert_eq!(parse_query("a=1&b=%zz"), None);
    }
}
