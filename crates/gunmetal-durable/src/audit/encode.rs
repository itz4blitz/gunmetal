//! Canonical JSON for audit records: one object per line, every string
//! escaped so no value can split a record (SEC-OPS-022, SEC-IAM-096).

use std::fmt::Write as _;

/// The domain separator hashed with each record (SEC-IAM-094).
pub(crate) const DOMAIN: &[u8] = b"gunmetal-audit-v1";

/// Whether a character is written as `\uXXXX`.
fn escaped(c: char) -> bool {
    c.is_control()
        || matches!(c, '\u{2028}' | '\u{2029}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}

/// Appends `text` to `line` as a JSON string.
pub(crate) fn push_string(line: &mut String, text: &str) {
    line.push('"');
    for c in text.chars() {
        match c {
            '"' | '\\' => {
                line.push('\\');
                line.push(c);
            }
            c if escaped(c) => {
                let _ = write!(line, "\\u{:04x}", u32::from(c));
            }
            c => line.push(c),
        }
    }
    line.push('"');
}

/// One nibble as an ASCII hex digit, without indexing a table.
fn hex_digit(nibble: u8) -> u8 {
    let letter = nibble.wrapping_sub(10);
    if nibble < 10 {
        nibble.wrapping_add(b'0')
    } else {
        letter.wrapping_add(b'a')
    }
}

/// `bytes` as lower-case hexadecimal.
pub(crate) fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        let hi = byte.wrapping_shr(4);
        let lo = byte.wrapping_sub(hi.wrapping_shl(4));
        out.push(char::from(hex_digit(hi)));
        out.push(char::from(hex_digit(lo)));
    }
    out
}

/// 32 bytes from hex digits; shorter or longer text is refused.
pub(crate) fn unhex32(text: &str) -> Option<[u8; 32]> {
    let bytes = text.as_bytes();
    let mut out = [0_u8; 32];
    let mut i: usize = 0;
    while i < 32 {
        let hi = nibble(*bytes.get(i.wrapping_mul(2))?)?;
        let lo = nibble(*bytes.get(i.wrapping_mul(2).wrapping_add(1))?)?;
        out[i] = hi.wrapping_shl(4).wrapping_add(lo);
        i = i.wrapping_add(1);
    }
    (bytes.len() == 64).then_some(out)
}

fn nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte.wrapping_sub(b'0')),
        b'a'..=b'f' => Some(byte.wrapping_sub(b'a').wrapping_add(10)),
        _ => None,
    }
}

/// Appends `"key":` to `line`.
pub(crate) fn push_key(line: &mut String, key: &str) {
    if !line.ends_with('{') {
        line.push(',');
    }
    push_string(line, key);
    line.push(':');
}

/// Appends a quoted string field.
pub(crate) fn field_str(line: &mut String, key: &str, value: &str) {
    push_key(line, key);
    push_string(line, value);
}

/// Appends an unsigned integer field.
pub(crate) fn field_u64(line: &mut String, key: &str, value: u64) {
    push_key(line, key);
    let _ = write!(line, "{value}");
}

#[cfg(test)]
mod tests {
    use super::{escaped, hex, hex_digit, nibble, push_string, unhex32};

    #[test]
    fn hex_round_trips_every_byte() {
        let mut bytes = [0_u8; 32];
        for (i, slot) in bytes.iter_mut().enumerate() {
            *slot = u8::try_from(i).expect("i < 32");
        }
        let text = hex(&bytes);
        assert_eq!(
            text,
            "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
        );
        assert_eq!(unhex32(&text), Some(bytes));
        assert_eq!(unhex32("00"), None);
        assert_eq!(unhex32(&format!("{text}0")), None);
        assert_eq!(unhex32(&"g".repeat(64)), None);
        assert_eq!(unhex32(&"0".repeat(63)), None);
        assert_eq!(unhex32(&format!("0g{}", "0".repeat(62))), None);
        assert_eq!(nibble(b'g'), None);
        assert_eq!(hex_digit(0), b'0');
        assert_eq!(hex_digit(9), b'9');
        assert_eq!(hex_digit(10), b'a');
        assert_eq!(hex_digit(15), b'f');
    }

    /// Verifies: SEC-OPS-022, SEC-IAM-096
    #[test]
    fn escapes_quotes_slashes_controls_and_bidi() {
        let mut line = String::new();
        push_string(&mut line, "a\"b\\c\n\u{2028}\u{202A}d");
        assert_eq!(line, r#""a\"b\\c\u000a\u2028\u202ad""#);
        assert!(escaped('\n'));
        assert!(escaped('\u{2066}'));
        assert!(!escaped('A'));
    }
}
