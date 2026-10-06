//! Credential kinds and the listeners that take them (SEC-EXT-007).
//!
//! Every credential the server stores carries a kind, written when the
//! credential is made and never changed. Each listener takes only its own
//! kinds: the native API takes web sessions, device keys and API keys and
//! refuses every adapter kind, and an adapter's listener refuses the native
//! kinds. In R1 the server issues only web sessions and has only the native
//! listener; the other kinds and listeners are named here so that a record
//! of one, however it got into the store, is refused by rule and not by
//! accident.

/// The kind of a stored credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    /// A browser session, held in the session cookie.
    WebSession,
    /// A native app's device key (R2).
    DeviceKey,
    /// A scoped API key (R2).
    ApiKey,
    /// An app key of the `OpenSubsonic` adapter (R2).
    OpenSubsonicAppKey,
    /// An app password of the Jellyfin adapter (R2).
    JellyfinAppPassword,
    /// A device token of the Jellyfin adapter (R2).
    JellyfinDeviceToken,
    /// A plugin's principal (R2).
    Plugin,
}

impl TokenKind {
    /// Every kind, in declaration order.
    pub const ALL: [Self; 7] = [
        Self::WebSession,
        Self::DeviceKey,
        Self::ApiKey,
        Self::OpenSubsonicAppKey,
        Self::JellyfinAppPassword,
        Self::JellyfinDeviceToken,
        Self::Plugin,
    ];

    /// The name the store keeps the kind under.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::WebSession => "web_session",
            Self::DeviceKey => "device_key",
            Self::ApiKey => "api_key",
            Self::OpenSubsonicAppKey => "opensubsonic_app_key",
            Self::JellyfinAppPassword => "jellyfin_app_password",
            Self::JellyfinDeviceToken => "jellyfin_device_token",
            Self::Plugin => "plugin",
        }
    }
}

/// An entry point that takes credentials.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Listener {
    /// The native API under `/api/v1`.
    Native,
    /// The `OpenSubsonic` adapter (R2).
    OpenSubsonic,
    /// The Jellyfin adapter (R2).
    Jellyfin,
}

impl Listener {
    /// Every listener, in declaration order.
    pub const ALL: [Self; 3] = [Self::Native, Self::OpenSubsonic, Self::Jellyfin];

    /// Whether this listener takes credentials of `kind`. A plugin's
    /// principal comes in through no listener.
    #[must_use]
    pub const fn accepts(self, kind: TokenKind) -> bool {
        matches!(
            (self, kind),
            (
                Self::Native,
                TokenKind::WebSession | TokenKind::DeviceKey | TokenKind::ApiKey
            ) | (Self::OpenSubsonic, TokenKind::OpenSubsonicAppKey)
                | (
                    Self::Jellyfin,
                    TokenKind::JellyfinAppPassword | TokenKind::JellyfinDeviceToken
                )
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_its_own_stored_name() {
        assert_eq!(
            TokenKind::ALL.map(TokenKind::name),
            [
                "web_session",
                "device_key",
                "api_key",
                "opensubsonic_app_key",
                "jellyfin_app_password",
                "jellyfin_device_token",
                "plugin",
            ]
        );
    }

    /// Verifies: SEC-EXT-007
    #[test]
    fn each_listener_takes_only_its_own_kinds() {
        let taken = Listener::ALL.map(|listener| {
            let kinds: Vec<TokenKind> = TokenKind::ALL
                .into_iter()
                .filter(|kind| listener.accepts(*kind))
                .collect();
            (listener, kinds)
        });
        assert_eq!(
            taken,
            [
                (
                    Listener::Native,
                    vec![
                        TokenKind::WebSession,
                        TokenKind::DeviceKey,
                        TokenKind::ApiKey
                    ]
                ),
                (Listener::OpenSubsonic, vec![TokenKind::OpenSubsonicAppKey]),
                (
                    Listener::Jellyfin,
                    vec![
                        TokenKind::JellyfinAppPassword,
                        TokenKind::JellyfinDeviceToken
                    ]
                ),
            ]
        );
    }
}
