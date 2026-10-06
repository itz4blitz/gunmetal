//! The harness for the inbound-link reader in `gunmetal_core::deeplink`
//! (SEC-CLI-025).

use gunmetal_core::base64::{self, Alphabet};
use gunmetal_core::deeplink::{Route, parse_link};
use gunmetal_core::untrusted::Untrusted;

/// What [`parse_link`] made of one input, read as UTF-8 with each invalid
/// sequence replaced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The route, in plain values a replay test can write out.
    pub route: Seen,
    /// What [`Route::needs_confirmation`] answered for it.
    pub needs_confirmation: bool,
}

/// A [`Route`] in plain values. A route has no `==`, because the secret it
/// may hold has none (SEC-OPS-013), so a replay test compares this copy of
/// all a route holds. The octets here are the fuzzer's input or a committed
/// seed's, never a secret a server issued.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Seen {
    /// [`Route::Claim`].
    Claim {
        /// The server's origin.
        server: String,
        /// The claim code's grouped text.
        code: String,
    },
    /// [`Route::Invitation`].
    Invitation {
        /// The server's origin.
        server: String,
        /// The secret's octets.
        secret: [u8; 16],
    },
    /// [`Route::Pairing`].
    Pairing {
        /// The server's origin.
        server: String,
        /// The pairing code's grouped text.
        code: String,
        /// The octets of the server's identity key.
        key: [u8; 32],
    },
    /// [`Route::Recovery`].
    Recovery {
        /// The server's origin.
        server: String,
        /// The secret's octets.
        secret: [u8; 16],
    },
    /// [`Route::NotRecognised`].
    NotRecognised,
}

/// Feeds `data` to [`parse_link`].
///
/// # Panics
///
/// Panics when the answer breaks an invariant that holds for every input.
/// A route must ask for a confirmation screen exactly when it would change
/// something, which each of the four recognised routes does (SEC-CLI-025).
/// A recognised input must be, to the octet, the link this harness writes
/// for the route it got: the server's origin, the route's path and, in the
/// fragment, the canonical text of its code, secret and key, so that
/// nothing but the one spelling a server writes is ever read. And that
/// origin must be at most 267 octets, which is `https://`, a host as long
/// as a name can be and a port of five digits, so that a confirmation
/// screen can show all of it; and it must be `https://` with a host, or
/// this machine's `http://localhost` alone or with a port in decimal
/// digits.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let raw = String::from_utf8_lossy(data);
    let route = parse_link(Untrusted::new(&raw));
    let (seen, origin, changes_something, written) = match &route {
        Route::Claim { server, code } => (
            Seen::Claim {
                server: server.origin().to_owned(),
                code: code.text(),
            },
            server.origin(),
            true,
            format!("{}/claim#{}", server.origin(), code.text()),
        ),
        Route::Invitation { server, secret } => (
            Seen::Invitation {
                server: server.origin().to_owned(),
                secret: secret.bytes(),
            },
            server.origin(),
            true,
            format!(
                "{}/invite#{}",
                server.origin(),
                base64::encode(&secret.bytes(), Alphabet::UrlSafe)
            ),
        ),
        Route::Pairing { server, code, key } => (
            Seen::Pairing {
                server: server.origin().to_owned(),
                code: code.text(),
                key: key.bytes(),
            },
            server.origin(),
            true,
            format!(
                "{}/pair#{}.{}",
                server.origin(),
                code.text(),
                base64::encode(&key.bytes(), Alphabet::UrlSafe)
            ),
        ),
        Route::Recovery { server, secret } => (
            Seen::Recovery {
                server: server.origin().to_owned(),
                secret: secret.bytes(),
            },
            server.origin(),
            true,
            format!(
                "{}/recover#{}",
                server.origin(),
                base64::encode(&secret.bytes(), Alphabet::UrlSafe)
            ),
        ),
        Route::NotRecognised => (Seen::NotRecognised, "", false, String::new()),
    };
    assert!(
        changes_something == route.needs_confirmation()
            && (!changes_something
                || (written == raw
                    && origin.len() <= 267
                    && (origin
                        .strip_prefix("https://")
                        .is_some_and(|host| !host.is_empty())
                        || origin == "http://localhost"
                        || origin
                            .strip_prefix("http://localhost:")
                            .is_some_and(|port| !port.is_empty()
                                && port.bytes().all(|digit| digit.is_ascii_digit()))))),
        "{raw:?} gave {route:?}"
    );
    Outcome {
        route: seen,
        needs_confirmation: route.needs_confirmation(),
    }
}
