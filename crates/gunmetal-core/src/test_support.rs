//! Helpers shared by this crate's unit tests.

use std::fmt::Write as _;

/// Writes a digest as lowercase hexadecimal, the way the specifications
/// print their examples.
#[must_use]
pub(crate) fn hex_lower(digest: [u8; 32]) -> String {
    digest.iter().fold(String::new(), |mut out, byte| {
        write!(out, "{byte:02x}").unwrap();
        out
    })
}

/// Whether `name` is one or more lower-case ASCII words joined by `_`.
#[must_use]
pub(crate) fn is_lower_snake(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_lowercase())
        && name.ends_with(|c: char| c.is_ascii_lowercase())
        && name.chars().all(|c| c.is_ascii_lowercase() || c == '_')
}
