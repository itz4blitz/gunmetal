//! The scopes a plugin may be granted.
//!
//! The list is the initial vocabulary in the security baseline, without the
//! administrator scopes. A plugin has no wildcard and no scope for
//! credentials, users, plugins, adapters or server settings (SEC-EXT-010,
//! SEC-EXT-029).

/// One scope a plugin manifest may name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scope {
    /// Browse metadata of libraries the person can see.
    LibraryRead,
    /// Obtain signed stream URLs and bytes.
    MediaStream,
    /// Download originals, if the person's policy allows.
    MediaDownload,
    /// Read the person's own history.
    HistoryRead,
    /// Report progress and scrobbles for the person.
    HistoryWrite,
    /// Change the person's own playlists.
    PlaylistsWrite,
    /// Change the person's own ratings and favourites.
    RatingsWrite,
    /// Receive the person's own events.
    EventsSelf,
}

/// Why a text is not a plugin scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeError {
    /// It is empty.
    Empty,
    /// It is `*` or another wildcard.
    Wildcard,
    /// It is an administrator scope. A plugin cannot hold one.
    Admin,
    /// It is not in the vocabulary.
    Unknown,
}

impl Scope {
    /// Every scope, in the vocabulary's order.
    pub const ALL: [Self; 8] = [
        Self::LibraryRead,
        Self::MediaStream,
        Self::MediaDownload,
        Self::HistoryRead,
        Self::HistoryWrite,
        Self::PlaylistsWrite,
        Self::RatingsWrite,
        Self::EventsSelf,
    ];

    /// The vocabulary spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LibraryRead => "library:read",
            Self::MediaStream => "media:stream",
            Self::MediaDownload => "media:download",
            Self::HistoryRead => "history:read",
            Self::HistoryWrite => "history:write",
            Self::PlaylistsWrite => "playlists:write",
            Self::RatingsWrite => "ratings:write",
            Self::EventsSelf => "events:self",
        }
    }

    /// Reads a scope.
    ///
    /// # Errors
    ///
    /// A [`ScopeError`] for an empty text, a wildcard, an administrator
    /// scope, or any other name.
    pub fn parse(text: &str) -> Result<Self, ScopeError> {
        if text.is_empty() {
            return Err(ScopeError::Empty);
        }
        if text.contains('*') || text == "all" {
            return Err(ScopeError::Wildcard);
        }
        if text.starts_with("admin:") {
            return Err(ScopeError::Admin);
        }
        Self::ALL
            .into_iter()
            .find(|scope| scope.as_str() == text)
            .ok_or(ScopeError::Unknown)
    }
}

#[cfg(test)]
mod tests {
    use super::{Scope, ScopeError};

    /// Verifies: SEC-EXT-010, SEC-EXT-029
    #[test]
    fn the_vocabulary_is_these_eight_scopes_and_no_administrator_scope() {
        let spellings: Vec<&str> = Scope::ALL.iter().map(|scope| scope.as_str()).collect();
        assert_eq!(
            spellings,
            [
                "library:read",
                "media:stream",
                "media:download",
                "history:read",
                "history:write",
                "playlists:write",
                "ratings:write",
                "events:self",
            ]
        );
        for scope in Scope::ALL {
            assert_eq!(Scope::parse(scope.as_str()), Ok(scope));
            assert!(!scope.as_str().starts_with("admin:"));
        }
    }

    /// Verifies: SEC-EXT-010, SEC-EXT-025
    #[test]
    fn an_unknown_empty_wildcard_or_administrator_scope_is_refused() {
        let cases = [
            ("", ScopeError::Empty),
            ("*", ScopeError::Wildcard),
            ("library:*", ScopeError::Wildcard),
            ("all", ScopeError::Wildcard),
            ("admin:library", ScopeError::Admin),
            ("admin:status", ScopeError::Admin),
            ("credentials:read", ScopeError::Unknown),
            ("plugins:install", ScopeError::Unknown),
            ("events:self ", ScopeError::Unknown),
        ];
        for (text, expected) in cases {
            assert_eq!(Scope::parse(text), Err(expected), "{text:?}");
        }
    }
}
