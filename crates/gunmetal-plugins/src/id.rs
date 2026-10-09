//! A plugin's stable id.
//!
//! An id is the name the index delegates a publisher key for. It is not a
//! path and not a host.

/// The longest id, in bytes.
pub const MAX_LEN: usize = 64;

/// A plugin id: dotted labels of lower-case letters, digits and hyphens.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PluginId(String);

/// Why a text is not a plugin id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdError {
    /// It is empty.
    Empty,
    /// It is longer than [`MAX_LEN`].
    TooLong {
        /// How many bytes it has.
        got: usize,
    },
    /// A byte is not a lower-case letter, a digit, a dot or a hyphen.
    BadCharacter {
        /// The byte's index.
        index: usize,
    },
    /// It has no dot, an empty label, a label that starts or ends with a
    /// hyphen, or a label longer than 63 bytes.
    Shape,
}

impl PluginId {
    /// Reads an id.
    ///
    /// # Errors
    ///
    /// An [`IdError`] for anything that is not a dotted id.
    pub fn parse(text: &str) -> Result<Self, IdError> {
        if text.is_empty() {
            return Err(IdError::Empty);
        }
        if text.len() > MAX_LEN {
            return Err(IdError::TooLong { got: text.len() });
        }
        if !text.contains('.') {
            return Err(IdError::Shape);
        }
        for (index, byte) in text.bytes().enumerate() {
            let allowed =
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'.' || byte == b'-';
            if !allowed {
                return Err(IdError::BadCharacter { index });
            }
        }
        if !text.split('.').all(label_ok) {
            return Err(IdError::Shape);
        }
        Ok(Self(text.to_owned()))
    }

    /// The id.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Whether `label` is one label of an id.
fn label_ok(label: &str) -> bool {
    let bytes = label.as_bytes();
    let (Some(first), Some(last)) = (bytes.first(), bytes.last()) else {
        return false;
    };
    (1..=63).contains(&bytes.len()) && *first != b'-' && *last != b'-'
}

#[cfg(test)]
mod tests {
    use super::{IdError, MAX_LEN, PluginId};
    use proptest::prelude::*;

    /// Verifies: SEC-EXT-025
    #[test]
    fn a_dotted_lower_case_id_is_kept_whole() {
        let id = PluginId::parse("org.listenbrainz.scrobbler").expect("the example id");
        assert_eq!(id.as_str(), "org.listenbrainz.scrobbler");
        assert_eq!(PluginId::parse(id.as_str()), Ok(id));
    }

    /// Verifies: SEC-EXT-025
    #[test]
    fn an_id_is_refused_for_the_reason_the_text_fails() {
        let long = "a.".to_owned() + &"b".repeat(MAX_LEN);
        let cases = [
            ("", Err(IdError::Empty)),
            ("listenbrainz", Err(IdError::Shape)),
            (".listenbrainz", Err(IdError::Shape)),
            ("listenbrainz.", Err(IdError::Shape)),
            ("org..scrobbler", Err(IdError::Shape)),
            ("org.-scrobbler", Err(IdError::Shape)),
            ("org.scrobbler-", Err(IdError::Shape)),
            ("Org.Scrobbler", Err(IdError::BadCharacter { index: 0 })),
            ("org.scrobbler/x", Err(IdError::BadCharacter { index: 13 })),
            ("org scrobbler", Err(IdError::Shape)),
        ];
        for (text, expected) in cases {
            assert_eq!(PluginId::parse(text), expected, "{text:?}");
        }
        assert_eq!(
            PluginId::parse(&long),
            Err(IdError::TooLong { got: long.len() })
        );
        assert!(reference_ok("ab.cd"));
        assert!(!reference_ok("ab.-c"));
        assert!(!reference_ok("ab.c-"));
        assert!(!reference_ok("a..b"));
    }

    fn reference_ok(text: &str) -> bool {
        !text.is_empty()
            && text.len() <= MAX_LEN
            && text.contains('.')
            && text.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'.' || byte == b'-'
            })
            && text.split('.').all(|label| {
                let bytes = label.as_bytes();
                (1..=63).contains(&bytes.len())
                    && bytes.first().is_some_and(|first| *first != b'-')
                    && bytes.last().is_some_and(|last| *last != b'-')
            })
    }

    proptest! {
        /// Verifies: SEC-EXT-025
        #[test]
        fn parse_agrees_with_an_independent_label_check(text in "\\PC{0,80}") {
            let parsed = PluginId::parse(&text);
            prop_assert_eq!(parsed.is_ok(), reference_ok(&text));
            if let Ok(id) = parsed {
                prop_assert_eq!(id.as_str(), text.as_str());
            }
        }
    }
}
