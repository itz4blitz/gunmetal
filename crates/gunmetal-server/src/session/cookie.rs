//! The cookies the server sets, and nothing else (SEC-API-032,
//! SEC-STD-014).
//!
//! A [`Cookie`] is made only by the session layer, from a token it has just
//! drawn, with one of the fixed attribute sets below. Its attributes come
//! from the mode the person chose at sign-in and from nothing in the
//! request, so `Secure` cannot be talked off by a header. The `__Host-`
//! prefix makes a browser refuse the cookie unless it came over HTTPS with
//! `Path=/` and no `Domain`.

use core::fmt;

use gunmetal_http::credential::SESSION_COOKIE;

use super::lifetime::Lifetime;
use super::token::Token;

/// The cookie inventory: the name of every cookie the server sets.
pub const INVENTORY: [&str; 1] = [SESSION_COOKIE];

/// A personal browser keeps its session cookie for the 30 days a session
/// may last in all.
const KEPT: &str = "; Secure; HttpOnly; SameSite=Lax; Path=/; Max-Age=2592000";

/// A shared browser's session cookie has no `Max-Age` and no `Expires`, so
/// it goes when the browser closes (SEC-CLI-010).
const UNTIL_CLOSED: &str = "; Secure; HttpOnly; SameSite=Lax; Path=/";

/// A cookie for a sign-in route to set. It holds a token, so it has no
/// `==` and its `Debug` form shows only its name.
pub struct Cookie {
    name: &'static str,
    value: String,
    attributes: &'static str,
}

impl Cookie {
    /// The session cookie for `token`, in the mode the person chose.
    pub(super) fn session(token: &Token, mode: Lifetime) -> Self {
        Self {
            name: SESSION_COOKIE,
            value: token.text(),
            attributes: match mode {
                Lifetime::Personal => KEPT,
                Lifetime::Shared => UNTIL_CLOSED,
            },
        }
    }

    /// The cookie's name, one of the [`INVENTORY`].
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// The value of the `Set-Cookie` header that sets the cookie, exactly.
    #[must_use]
    pub fn header(&self) -> String {
        format!("{}={}{}", self.name, self.value, self.attributes)
    }
}

impl fmt::Debug for Cookie {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cookie")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}
