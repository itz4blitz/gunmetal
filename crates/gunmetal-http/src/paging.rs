//! The one list convention (INT-007): a page size capped at 500 unless the
//! route declares more (SEC-API-063, SEC-HIS-037), and an opaque cursor
//! (SEC-API-025).
//!
//! A list route takes a [`PageQuery`] and answers a [`Page`]. The cursor is
//! a handle the store issues and looks up again: a client cannot read
//! anything from it, and nothing in it is trusted for an authorisation
//! decision.

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};

/// The page size most routes allow.
pub const PAGE_CAP: u32 = 500;

/// The longest cursor accepted.
pub const CURSOR_MAX: usize = 128;

/// A page size from 1 to `MAX`, sent in the query as decimal digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageLimit<const MAX: u32 = PAGE_CAP>(u32);

impl<const MAX: u32> PageLimit<MAX> {
    /// A page size, if it is between 1 and `MAX`.
    #[must_use]
    pub const fn new(size: u32) -> Option<Self> {
        if size >= 1 && size <= MAX {
            Some(Self(size))
        } else {
            None
        }
    }

    /// The page size.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl<'de, const MAX: u32> Deserialize<'de> for PageLimit<MAX> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.bytes()
            .all(|byte| byte.is_ascii_digit())
            .then(|| text.parse::<u32>().ok())
            .flatten()
            .and_then(Self::new)
            .ok_or_else(|| de::Error::custom("page size out of range"))
    }
}

/// An opaque continuation handle: 1 to [`CURSOR_MAX`] characters from the
/// URL-safe base64 alphabet.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Cursor(String);

impl From<Cursor> for String {
    fn from(cursor: Cursor) -> Self {
        cursor.0
    }
}

impl Cursor {
    /// A cursor, if the text has its shape.
    #[must_use]
    pub fn new(text: &str) -> Option<Self> {
        let shaped = !text.is_empty()
            && text.len() <= CURSOR_MAX
            && text
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
        shaped.then(|| Self(text.to_owned()))
    }

    /// The cursor's text, for the store to look up.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Cursor {
    type Error = &'static str;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        Self::new(&text).ok_or("malformed cursor")
    }
}

/// The paging part of a list route's query.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageQuery<const MAX: u32 = PAGE_CAP> {
    /// How many items to return.
    pub limit: Option<PageLimit<MAX>>,
    /// Where to continue from.
    pub cursor: Option<Cursor>,
}

/// One page of a list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Page<T> {
    /// The items on this page.
    pub items: Vec<T>,
    /// Where the next page starts, or `None` on the last page.
    pub next: Option<Cursor>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn query<const MAX: u32>(value: serde_json::Value) -> Option<PageQuery<MAX>> {
        serde_json::from_value(value).ok()
    }

    /// Verifies: SEC-HIS-037
    #[test]
    fn page_sizes_run_from_one_to_the_cap() {
        let limit = |text: &str| query::<PAGE_CAP>(json!({ "limit": text })).map(|q| q.limit);
        assert_eq!(limit("1"), Some(PageLimit::new(1)));
        assert_eq!(limit("500"), Some(PageLimit::new(500)));
        assert_eq!(limit("500").flatten().map(PageLimit::get), Some(500));
        for refused in ["0", "501", "-1", "+5", " 5", "5 ", "", "1e2", "99999999999"] {
            assert_eq!(limit(refused), None);
        }
        assert_eq!(query::<PAGE_CAP>(json!({ "limit": 5 })), None);
    }

    #[test]
    fn a_route_may_declare_a_larger_cap() {
        let limit = |text: &str| query::<2000>(json!({ "limit": text })).map(|q| q.limit);
        assert_eq!(limit("2000"), Some(PageLimit::new(2000)));
        assert_eq!(limit("2001"), None);
        assert_eq!(PageLimit::<500>::new(0), None);
    }

    #[test]
    fn cursors_are_short_url_safe_text() {
        let longest = "a".repeat(CURSOR_MAX);
        assert_eq!(
            Cursor::new(&longest).map(|c| c.as_str().len()),
            Some(CURSOR_MAX)
        );
        assert_eq!(
            Cursor::new("Ab9-_").map(|c| c.as_str().to_owned()),
            Some("Ab9-_".to_owned())
        );
        for refused in ["", "a/b", "a+b", "a=", "ä"] {
            assert_eq!(Cursor::new(refused), None);
        }
        assert_eq!(Cursor::new(&"a".repeat(CURSOR_MAX + 1)), None);
    }

    #[test]
    fn decodes_and_refuses_cursors_in_queries() {
        assert_eq!(
            query::<PAGE_CAP>(json!({ "cursor": "abc" })),
            Some(PageQuery {
                limit: None,
                cursor: Cursor::new("abc")
            })
        );
        assert_eq!(query::<PAGE_CAP>(json!({ "cursor": "a.b" })), None);
        assert_eq!(query::<PAGE_CAP>(json!({ "cursor": 5 })), None);
        assert_eq!(query::<PAGE_CAP>(json!({ "offset": "5" })), None);
    }

    #[test]
    fn serialises_a_page() {
        let page = Page {
            items: vec![1, 2],
            next: Cursor::new("n1"),
        };
        assert_eq!(
            serde_json::to_string(&page).unwrap(),
            r#"{"items":[1,2],"next":"n1"}"#
        );
    }
}
