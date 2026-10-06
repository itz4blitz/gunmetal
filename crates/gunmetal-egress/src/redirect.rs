//! Whether a redirect is followed.
//!
//! A request follows no redirect unless its purpose says so, and then at
//! most three, each to the host it was already talking to (SEC-EXT-003,
//! SEC-API-078, SEC-TM-048). A hop that passes this rule is still a new
//! request: it passes the gate again, so its scheme, host and port must be
//! ones the purpose was granted and its addresses are checked like the
//! first request's.

use crate::denial::Denial;
use crate::destination::Destination;
use crate::purpose::Redirects;

/// The most redirects one request follows.
pub const MAX_REDIRECTS: u8 = 3;

/// Decides whether a request that has already followed `followed`
/// redirects may follow one more, from `from` to `to`.
///
/// # Errors
///
/// The first of these that applies: [`Denial::RedirectsRefused`] when
/// `policy` follows none, [`Denial::TooManyRedirects`] when
/// [`MAX_REDIRECTS`] were followed already, and
/// [`Denial::CrossHostRedirect`] when `to` is on another host.
pub fn follow(
    policy: Redirects,
    followed: u8,
    from: &Destination,
    to: &Destination,
) -> Result<(), Denial> {
    if policy == Redirects::Refused {
        return Err(Denial::RedirectsRefused);
    }
    if followed >= MAX_REDIRECTS {
        return Err(Denial::TooManyRedirects);
    }
    if to.host != from.host {
        return Err(Denial::CrossHostRedirect);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{MAX_REDIRECTS, follow};
    use crate::denial::Denial;
    use crate::destination::{Destination, Host, Scheme};
    use crate::purpose::Redirects;

    fn host(text: &str) -> Host {
        Host::parse(text).expect("a host")
    }

    fn https(text: &str, port: u16) -> Destination {
        Destination {
            scheme: Scheme::Https,
            host: host(text),
            port,
        }
    }

    /// Supports: SEC-API-078, SEC-EXT-003
    #[test]
    fn a_request_that_follows_no_redirects_follows_none() {
        let feed = https("feed.example", 443);
        for followed in [0, 1, 2, 3, 200] {
            assert_eq!(
                follow(Redirects::Refused, followed, &feed, &feed),
                Err(Denial::RedirectsRefused),
                "{followed}"
            );
        }
    }

    /// Supports: SEC-API-078, SEC-EXT-003
    #[test]
    fn at_most_three_redirects_are_followed() {
        assert_eq!(MAX_REDIRECTS, 3);
        let feed = https("feed.example", 443);
        let cases = [
            (0, Ok(())),
            (1, Ok(())),
            (2, Ok(())),
            (3, Err(Denial::TooManyRedirects)),
            (4, Err(Denial::TooManyRedirects)),
            (255, Err(Denial::TooManyRedirects)),
        ];
        for (followed, verdict) in cases {
            assert_eq!(
                follow(Redirects::SameHost, followed, &feed, &feed),
                verdict,
                "{followed}"
            );
        }
    }

    /// Supports: SEC-EXT-003, SEC-PRV-008, SEC-TM-048
    #[test]
    fn a_redirect_to_another_host_is_refused() {
        let feed = https("feed.example", 443);
        for other in [
            "cdn.feed.example",
            "feed.example.org",
            "example",
            "192.0.2.1",
        ] {
            assert_eq!(
                follow(Redirects::SameHost, 0, &feed, &https(other, 443)),
                Err(Denial::CrossHostRedirect),
                "{other}"
            );
        }
        // Another port or scheme on the same host is not for this rule to
        // refuse: the hop passes the gate next, and the grant decides.
        assert_eq!(
            follow(Redirects::SameHost, 0, &feed, &https("feed.example", 8443)),
            Ok(())
        );
        let plain = Destination {
            scheme: Scheme::Http,
            host: host("feed.example"),
            port: 80,
        };
        assert_eq!(follow(Redirects::SameHost, 0, &feed, &plain), Ok(()));
        // One refusal is told at a time, the count before the host.
        assert_eq!(
            follow(Redirects::SameHost, 3, &feed, &https("other.example", 443)),
            Err(Denial::TooManyRedirects)
        );
    }
}
