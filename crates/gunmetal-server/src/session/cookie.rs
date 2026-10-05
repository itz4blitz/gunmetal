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

/// The cookie inventory: the name of every cookie the server sets.
pub const INVENTORY: [&str; 1] = [SESSION_COOKIE];

/// A cookie for a sign-in route to set. It holds a token, so it has no
/// `==` and its `Debug` form shows only its name.
pub struct Cookie {
    name: &'static str,
    value: String,
    attributes: &'static str,
}

impl Cookie {
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
