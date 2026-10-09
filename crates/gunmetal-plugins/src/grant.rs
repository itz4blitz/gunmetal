//! Network grants and returned URLs.
//!
//! A public grant names an exact host. A plugin may ask for that host only
//! over HTTPS on port 443. A URL the plugin returns is read here and admitted
//! only under the same grant; this module does not fetch it. A LAN destination
//! needs a separate admin grant, and loopback, link-local and unspecified
//! addresses are never admitted (SEC-EXT-026, SEC-EXT-027).

use std::net::IpAddr;

use gunmetal_core::net::{AddrClass, classify};
use gunmetal_egress::destination::HostError;

use crate::record::{ExactHost, HostRefuse, NetworkGrant};

/// The only port a public grant admits.
const PUBLIC_PORT: u16 = 443;

/// Why a public fetch, or a URL a plugin returned, is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchRefuse {
    /// No grant names this host. `host` is the text that was asked for.
    NotGranted {
        /// The host text, as given, not a rewritten spelling.
        host: String,
    },
    /// The host is an IP address. A public grant names a host, not an address.
    Address,
    /// The host contains `*`.
    Wildcard,
    /// The request is plain HTTP.
    PlainHttp,
    /// The port is not 443.
    WrongPort {
        /// The port that was asked for.
        port: u16,
    },
    /// The host is not a public DNS name, or the URL is not an accepted shape.
    Name(HostRefuse),
}

/// A public fetch a plugin asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fetch<'a> {
    /// The host text, as given.
    pub host: &'a str,
    /// The port.
    pub port: u16,
    /// Whether the scheme is HTTPS.
    pub https: bool,
}

/// A URL a plugin returned, reduced to the host and port the grant admits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnedUrl {
    /// The public host, in the spelling [`ExactHost`] keeps.
    pub host: ExactHost,
    /// Always 443. A returned URL has no other port.
    pub port: u16,
}

/// An exact LAN host and port an admin granted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanGrant {
    /// The host, matched without regard to ASCII case.
    pub host: String,
    /// The port.
    pub port: u16,
}

/// Why a LAN destination is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanRefuse {
    /// There is no admin grant, or it does not name this host and port.
    NotGranted,
    /// The address is loopback, link-local or unspecified, so it cannot be granted.
    Never,
}

/// Decides whether `fetch` is a granted public request.
///
/// The host is read first, through [`ExactHost::parse_public`], so a `*` is
/// refused before the name is read. An address, a name that is not public
/// DNS, plain HTTP and a port other than 443 are refused next, even when a
/// grant names the host and even when the grant list is empty. Only then is
/// the host compared with each grant, in the lower-case spelling
/// [`ExactHost`] keeps. This does not resolve the name or open a socket.
///
/// # Errors
///
/// [`FetchRefuse::Wildcard`] when the host contains `*`,
/// [`FetchRefuse::Address`] when it is an IP address,
/// [`FetchRefuse::Name`] when it is not a public DNS name,
/// [`FetchRefuse::PlainHttp`] when it is not HTTPS,
/// [`FetchRefuse::WrongPort`] when the port is not 443, and
/// [`FetchRefuse::NotGranted`] when no grant names the host. The host in
/// that error is the text that was asked for.
#[must_use = "a refused public fetch must not be dropped"]
pub fn admit_public(grants: &[NetworkGrant], fetch: Fetch<'_>) -> Result<(), FetchRefuse> {
    let host = public_host(fetch.host)?;
    if !fetch.https {
        return Err(FetchRefuse::PlainHttp);
    }
    if fetch.port != PUBLIC_PORT {
        return Err(FetchRefuse::WrongPort { port: fetch.port });
    }
    if is_granted(grants, &host) {
        Ok(())
    } else {
        Err(FetchRefuse::NotGranted {
            host: fetch.host.to_owned(),
        })
    }
}

/// Reads a URL a plugin returned and admits it only under `grants`.
///
/// Accepts `https://host`, `https://host:443`, and the same with a path or
/// a query. The scheme is compared without regard to ASCII case. Anything
/// else is dropped: plain HTTP, a missing scheme, userinfo, a fragment, a
/// non-canonical port, a port other than 443, an IP address, a wildcard, or
/// a host that is not granted. The URL is not fetched.
///
/// # Errors
///
/// A [`FetchRefuse`]. Plain HTTP is [`FetchRefuse::PlainHttp`] even when the
/// host is an address. A host outside the grant is
/// [`FetchRefuse::NotGranted`] with the host text as written in the URL. A
/// URL that is not one of the accepted shapes is [`FetchRefuse::Name`] with
/// [`HostRefuse::Name`] of [`HostError::BadLabel`].
#[must_use = "a URL outside the grant must not be fetched"]
pub fn admit_returned_url(grants: &[NetworkGrant], url: &str) -> Result<ReturnedUrl, FetchRefuse> {
    let (host, port) = read_returned(url)?;
    let parsed = public_host(host)?;
    if port != PUBLIC_PORT {
        return Err(FetchRefuse::WrongPort { port });
    }
    if is_granted(grants, &parsed) {
        Ok(ReturnedUrl {
            host: parsed,
            port: PUBLIC_PORT,
        })
    } else {
        Err(FetchRefuse::NotGranted {
            host: host.to_owned(),
        })
    }
}

/// Decides whether an admin grant admits this LAN host and port.
///
/// No admin grant is [`LanRefuse::NotGranted`], including for an address
/// that can never be granted. When a grant is present, a text that parses
/// as an IP address is classified, and loopback, link-local and unspecified
/// addresses are [`LanRefuse::Never`] even if the grant names them. A zone
/// id, as in `fe80::1%eth0`, is not a separate host: the address before `%`
/// is what is classified. Any other host is admitted only when the grant
/// names that host, without regard to ASCII case, and that port. This does
/// not resolve a name or open a socket.
///
/// # Errors
///
/// [`LanRefuse::NotGranted`] when `admin` is absent or does not name this
/// host and port. [`LanRefuse::Never`] when a grant is present and the host
/// is loopback, link-local or unspecified.
#[must_use = "a refused LAN destination must not be opened"]
pub fn admit_lan(admin: Option<&LanGrant>, host: &str, port: u16) -> Result<(), LanRefuse> {
    let Some(admin) = admin else {
        return Err(LanRefuse::NotGranted);
    };
    if never_address(host) {
        return Err(LanRefuse::Never);
    }
    if admin.port == port && admin.host.eq_ignore_ascii_case(host) {
        Ok(())
    } else {
        Err(LanRefuse::NotGranted)
    }
}

/// The public host, or the refusal `parse_public` already decided.
fn public_host(text: &str) -> Result<ExactHost, FetchRefuse> {
    ExactHost::parse_public(text).map_err(|error| match error {
        HostRefuse::Address => FetchRefuse::Address,
        HostRefuse::Wildcard => FetchRefuse::Wildcard,
        HostRefuse::Name(_) | HostRefuse::NoDot => FetchRefuse::Name(error),
    })
}

/// Whether one grant names `host` in the spelling [`ExactHost`] keeps.
fn is_granted(grants: &[NetworkGrant], host: &ExactHost) -> bool {
    grants
        .iter()
        .any(|grant| grant.host.as_str() == host.as_str())
}

/// A URL that is not one of the accepted shapes.
const fn bad_shape() -> FetchRefuse {
    FetchRefuse::Name(HostRefuse::Name(HostError::BadLabel))
}

/// The host and port of an accepted returned URL, or why it is not one.
fn read_returned(url: &str) -> Result<(&str, u16), FetchRefuse> {
    let rest = split_scheme(url)?;
    if rest.contains('#') {
        return Err(bad_shape());
    }
    let authority = authority_of(rest);
    if authority.is_empty() || authority.contains('@') {
        return Err(bad_shape());
    }
    if let Some(bracketed) = authority.strip_prefix('[') {
        return Err(bracket_body(bracketed));
    }
    name_and_port(authority)
}

/// The text after `://`, or why the scheme is not HTTPS.
fn split_scheme(url: &str) -> Result<&str, FetchRefuse> {
    let Some((scheme, rest)) = url.split_once("://") else {
        return Err(bad_shape());
    };
    if scheme.eq_ignore_ascii_case("http") {
        return Err(FetchRefuse::PlainHttp);
    }
    if scheme.eq_ignore_ascii_case("https") {
        return Ok(rest);
    }
    Err(bad_shape())
}

/// The authority: the text before the first `/` or `?`.
fn authority_of(rest: &str) -> &str {
    match (rest.split_once('/'), rest.split_once('?')) {
        (Some((slash, _)), Some((query, _))) => {
            if slash.len() <= query.len() {
                slash
            } else {
                query
            }
        }
        (Some((slash, _)), None) => slash,
        (None, Some((query, _))) => query,
        (None, None) => rest,
    }
}

/// An authority that started with `[`: an address, or not a host.
fn bracket_body(rest: &str) -> FetchRefuse {
    let Some((inner, _)) = rest.split_once(']') else {
        return bad_shape();
    };
    if inner.parse::<IpAddr>().is_ok() {
        FetchRefuse::Address
    } else {
        bad_shape()
    }
}

/// A host and its port. An address written with no brackets is refused here
/// when the whole authority is that address; a host that is an address after
/// the port is split off is refused by [`public_host`].
fn name_and_port(authority: &str) -> Result<(&str, u16), FetchRefuse> {
    if authority.parse::<IpAddr>().is_ok() {
        return Err(FetchRefuse::Address);
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, text)) => (host, explicit_port(text)?),
        None => (authority, PUBLIC_PORT),
    };
    if host.is_empty() {
        return Err(bad_shape());
    }
    Ok((host, port))
}

/// A canonical decimal port, with no leading zero.
fn explicit_port(text: &str) -> Result<u16, FetchRefuse> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(bad_shape());
    }
    if text.len() > 1 && text.starts_with('0') {
        return Err(bad_shape());
    }
    let mut value: u16 = 0;
    for byte in text.bytes() {
        let digit = u16::from(byte.saturating_sub(b'0'));
        value = value
            .checked_mul(10)
            .and_then(|scaled| scaled.checked_add(digit))
            .ok_or_else(bad_shape)?;
    }
    Ok(value)
}

/// Whether `host` is an address that can never be granted.
fn never_address(host: &str) -> bool {
    let address = host.split_once('%').map_or(host, |(address, _)| address);
    let Ok(ip) = address.parse::<IpAddr>() else {
        return false;
    };
    matches!(
        classify(ip),
        AddrClass::Loopback | AddrClass::LinkLocal | AddrClass::Unspecified
    )
}

#[cfg(test)]
mod tests {
    use super::{
        Fetch, FetchRefuse, LanGrant, LanRefuse, ReturnedUrl, admit_lan, admit_public,
        admit_returned_url,
    };
    use crate::record::{ExactHost, HostRefuse, NetworkGrant};
    use gunmetal_egress::destination::HostError;

    fn grant(host: &str, why: &str) -> NetworkGrant {
        NetworkGrant {
            host: ExactHost::parse_public(host).expect("a public host"),
            why: why.to_owned(),
        }
    }

    fn fetch(host: &str, port: u16, https: bool) -> Fetch<'_> {
        Fetch { host, port, https }
    }

    fn admitted(host: &str) -> ReturnedUrl {
        ReturnedUrl {
            host: ExactHost::parse_public(host).expect("a public host"),
            port: 443,
        }
    }

    fn bad_shape() -> FetchRefuse {
        FetchRefuse::Name(HostRefuse::Name(HostError::BadLabel))
    }

    /// Verifies: SEC-EXT-026
    #[test]
    fn empty_grants_refuse_a_public_fetch_and_keep_the_host_as_given() {
        assert_eq!(
            admit_public(&[], fetch("api.listenbrainz.org", 443, true)),
            Err(FetchRefuse::NotGranted {
                host: "api.listenbrainz.org".to_owned(),
            })
        );
        assert_eq!(
            admit_public(&[], fetch("API.ListenBrainz.Org", 443, true)),
            Err(FetchRefuse::NotGranted {
                host: "API.ListenBrainz.Org".to_owned(),
            })
        );
        assert_eq!(
            admit_public(&[], fetch("covers.example", 443, true)),
            Err(FetchRefuse::NotGranted {
                host: "covers.example".to_owned(),
            })
        );
    }

    /// Verifies: SEC-EXT-026
    #[test]
    fn a_public_fetch_is_admitted_only_for_https_on_443_to_a_granted_host() {
        let first = grant("covers.example", "covers");
        let second = grant("api.listenbrainz.org", "scrobbles");
        let stored_mixed = grant("API.ListenBrainz.Org", "scrobbles");
        assert_eq!(stored_mixed.host.as_str(), "api.listenbrainz.org");
        let grants = [first, second];

        assert_eq!(
            admit_public(&grants, fetch("api.listenbrainz.org", 443, true)),
            Ok(())
        );
        assert_eq!(
            admit_public(&grants, fetch("API.ListenBrainz.Org", 443, true)),
            Ok(())
        );
        assert_eq!(
            admit_public(&grants, fetch("covers.example", 443, true)),
            Ok(())
        );
        assert_eq!(
            admit_public(&[stored_mixed], fetch("api.listenbrainz.org", 443, true)),
            Ok(())
        );
        assert_eq!(
            admit_public(&grants, fetch("api.listenbrainz.org", 443, false)),
            Err(FetchRefuse::PlainHttp)
        );
        assert_eq!(
            admit_public(&grants, fetch("api.listenbrainz.org", 80, false)),
            Err(FetchRefuse::PlainHttp)
        );
        assert_eq!(
            admit_public(&[], fetch("api.listenbrainz.org", 443, false)),
            Err(FetchRefuse::PlainHttp)
        );
        assert_eq!(
            admit_public(&grants, fetch("api.listenbrainz.org", 80, true)),
            Err(FetchRefuse::WrongPort { port: 80 })
        );
        assert_eq!(
            admit_public(&grants, fetch("api.listenbrainz.org", 8443, true)),
            Err(FetchRefuse::WrongPort { port: 8443 })
        );
        assert_eq!(
            admit_public(&grants, fetch("api.listenbrainz.org", 0, true)),
            Err(FetchRefuse::WrongPort { port: 0 })
        );
        assert_eq!(
            admit_public(&grants, fetch("api.listenbrainz.org", 65535, true)),
            Err(FetchRefuse::WrongPort { port: 65535 })
        );
        assert_eq!(
            admit_public(&grants, fetch("other.example", 80, true)),
            Err(FetchRefuse::WrongPort { port: 80 })
        );
        assert_eq!(
            admit_public(&grants, fetch("other.example", 443, true)),
            Err(FetchRefuse::NotGranted {
                host: "other.example".to_owned(),
            })
        );
        assert_eq!(
            admit_public(&grants, fetch("API.Other.Example", 443, true)),
            Err(FetchRefuse::NotGranted {
                host: "API.Other.Example".to_owned(),
            })
        );
        assert_eq!(
            admit_public(&grants, fetch("notapi.listenbrainz.org", 443, true)),
            Err(FetchRefuse::NotGranted {
                host: "notapi.listenbrainz.org".to_owned(),
            })
        );
        assert_eq!(
            admit_public(
                &grants,
                fetch("api.listenbrainz.org.evil.example", 443, true)
            ),
            Err(FetchRefuse::NotGranted {
                host: "api.listenbrainz.org.evil.example".to_owned(),
            })
        );
        assert_eq!(
            admit_public(&grants, fetch("listenbrainz.org", 443, true)),
            Err(FetchRefuse::NotGranted {
                host: "listenbrainz.org".to_owned(),
            })
        );
    }

    /// Verifies: SEC-EXT-026
    #[test]
    fn a_public_host_that_is_not_a_name_is_refused_before_the_grant() {
        let grants = [grant("api.listenbrainz.org", "scrobbles")];
        let long = "a".repeat(254);
        let cases = [
            ("*.listenbrainz.org", FetchRefuse::Wildcard),
            ("api.*.org", FetchRefuse::Wildcard),
            ("*", FetchRefuse::Wildcard),
            ("1.2.3.*", FetchRefuse::Wildcard),
            ("1.2.3.4", FetchRefuse::Address),
            ("::1", FetchRefuse::Address),
            ("0:0:0:0:0:0:0:1", FetchRefuse::Address),
            ("::ffff:1.2.3.4", FetchRefuse::Address),
            ("localhost", FetchRefuse::Name(HostRefuse::NoDot)),
            (
                "bad_host.com",
                FetchRefuse::Name(HostRefuse::Name(HostError::BadLabel)),
            ),
            ("", FetchRefuse::Name(HostRefuse::Name(HostError::BadLabel))),
            (
                "api.listenbrainz.org.",
                FetchRefuse::Name(HostRefuse::Name(HostError::BadLabel)),
            ),
            (
                "[::1]",
                FetchRefuse::Name(HostRefuse::Name(HostError::BadLabel)),
            ),
            (
                "example.123",
                FetchRefuse::Name(HostRefuse::Name(HostError::NumericEnd)),
            ),
        ];
        for (host, expected) in cases {
            assert_eq!(
                admit_public(&grants, fetch(host, 80, false)),
                Err(expected.clone()),
                "{host:?}"
            );
            assert_eq!(
                admit_public(&[], fetch(host, 443, true)),
                Err(expected),
                "{host:?}"
            );
        }
        assert_eq!(
            admit_public(&grants, fetch(&long, 443, true)),
            Err(FetchRefuse::Name(HostRefuse::Name(HostError::TooLong)))
        );
    }

    /// Verifies: SEC-EXT-027
    #[test]
    fn a_returned_url_is_admitted_only_under_the_plugin_grant() {
        let grants = [
            grant("covers.example", "covers"),
            grant("api.listenbrainz.org", "scrobbles"),
        ];
        let expected = admitted("api.listenbrainz.org");
        let accepted = [
            "https://api.listenbrainz.org",
            "https://api.listenbrainz.org:443",
            "https://api.listenbrainz.org/art.jpg",
            "https://api.listenbrainz.org:443/art.jpg",
            "https://api.listenbrainz.org?size=1",
            "https://api.listenbrainz.org:443?size=1",
            "https://api.listenbrainz.org/art.jpg?size=1",
            "https://api.listenbrainz.org:443/art.jpg?size=1",
            "https://api.listenbrainz.org/",
            "https://api.listenbrainz.org/art:1.jpg",
            "https://api.listenbrainz.org/user@host",
            "https://api.listenbrainz.org?size=1/extra",
            "https://api.listenbrainz.org?",
            "https://API.ListenBrainz.Org/art.jpg",
            "HTTPS://api.listenbrainz.org/art.jpg",
            "HtTpS://api.listenbrainz.org/art.jpg",
            "https://covers.example/cover.jpg",
        ];
        for url in accepted {
            let expected = if url.contains("covers.example") {
                admitted("covers.example")
            } else {
                expected.clone()
            };
            assert_eq!(admit_returned_url(&grants, url), Ok(expected), "{url:?}");
        }
        assert_eq!(
            admit_returned_url(&[], "https://api.listenbrainz.org/art.jpg"),
            Err(FetchRefuse::NotGranted {
                host: "api.listenbrainz.org".to_owned(),
            })
        );
        assert_eq!(
            admit_returned_url(&grants, "https://Evil.Example/art.jpg"),
            Err(FetchRefuse::NotGranted {
                host: "Evil.Example".to_owned(),
            })
        );
        assert_eq!(
            admit_returned_url(&grants, "https://Evil.Example:443/art.jpg"),
            Err(FetchRefuse::NotGranted {
                host: "Evil.Example".to_owned(),
            })
        );
        assert_eq!(
            admit_returned_url(&grants, "https://api.listenbrainz.org.evil.example/art.jpg"),
            Err(FetchRefuse::NotGranted {
                host: "api.listenbrainz.org.evil.example".to_owned(),
            })
        );
        assert_eq!(
            admit_returned_url(
                &grants,
                "https://evil.example/https://api.listenbrainz.org/art.jpg"
            ),
            Err(FetchRefuse::NotGranted {
                host: "evil.example".to_owned(),
            })
        );
        assert_eq!(
            admit_returned_url(
                &grants,
                "https://evil.example?next=https://api.listenbrainz.org"
            ),
            Err(FetchRefuse::NotGranted {
                host: "evil.example".to_owned(),
            })
        );
    }

    /// Verifies: SEC-EXT-027
    #[test]
    fn a_returned_url_outside_the_grant_or_the_accepted_shape_is_dropped() {
        let grants = [grant("api.listenbrainz.org", "scrobbles")];
        let cases = [
            (
                "http://api.listenbrainz.org/art.jpg",
                FetchRefuse::PlainHttp,
            ),
            ("HTTP://api.listenbrainz.org", FetchRefuse::PlainHttp),
            ("http://1.2.3.4/art.jpg", FetchRefuse::PlainHttp),
            ("http://[::1]/", FetchRefuse::PlainHttp),
            ("https://1.2.3.4/art.jpg", FetchRefuse::Address),
            ("https://1.2.3.4:443/art.jpg", FetchRefuse::Address),
            ("https://1.2.3.4:80/art.jpg", FetchRefuse::Address),
            ("https://[::1]/art.jpg", FetchRefuse::Address),
            ("https://[::1]:443/art.jpg", FetchRefuse::Address),
            ("https://[::1]:80/art.jpg", FetchRefuse::Address),
            ("https://[1.2.3.4]/art.jpg", FetchRefuse::Address),
            ("https://::1/", FetchRefuse::Address),
            ("https://::1:443/", FetchRefuse::Address),
            ("https://*.listenbrainz.org/art.jpg", FetchRefuse::Wildcard),
            (
                "https://*.listenbrainz.org:80/art.jpg",
                FetchRefuse::Wildcard,
            ),
            (
                "https://api.listenbrainz.org:80/art.jpg",
                FetchRefuse::WrongPort { port: 80 },
            ),
            (
                "https://api.listenbrainz.org:8443",
                FetchRefuse::WrongPort { port: 8443 },
            ),
            (
                "https://evil.example:80/art.jpg",
                FetchRefuse::WrongPort { port: 80 },
            ),
            (
                "https://api.listenbrainz.org:0",
                FetchRefuse::WrongPort { port: 0 },
            ),
            (
                "https://api.listenbrainz.org:65535",
                FetchRefuse::WrongPort { port: 65535 },
            ),
            (
                "https://localhost/art.jpg",
                FetchRefuse::Name(HostRefuse::NoDot),
            ),
            (
                "https://localhost:80/art.jpg",
                FetchRefuse::Name(HostRefuse::NoDot),
            ),
            (
                "https://example.123/art.jpg",
                FetchRefuse::Name(HostRefuse::Name(HostError::NumericEnd)),
            ),
            ("https://user@api.listenbrainz.org", bad_shape()),
            (
                "https://user:pass@api.listenbrainz.org/art.jpg",
                bad_shape(),
            ),
            ("https://api.listenbrainz.org@evil.example", bad_shape()),
            ("api.listenbrainz.org/art.jpg", bad_shape()),
            ("//api.listenbrainz.org/art.jpg", bad_shape()),
            ("ftp://api.listenbrainz.org/art.jpg", bad_shape()),
            ("https:/api.listenbrainz.org", bad_shape()),
            ("https://api.listenbrainz.org/art.jpg#x", bad_shape()),
            ("https://api.listenbrainz.org#x", bad_shape()),
            ("https://api.listenbrainz.org?x=1#y", bad_shape()),
            (" https://api.listenbrainz.org", bad_shape()),
            ("https://api.listenbrainz.org\n", bad_shape()),
            ("https://api.listenbrainz%2Eorg/art.jpg", bad_shape()),
            ("https://api.listenbrainz.org\\.evil.example", bad_shape()),
            ("https://[::1/art.jpg", bad_shape()),
            ("https://[api.listenbrainz.org]/art.jpg", bad_shape()),
            ("https://api.listenbrainz.org:0443", bad_shape()),
            ("https://api.listenbrainz.org:443a", bad_shape()),
            ("https://api.listenbrainz.org:", bad_shape()),
            ("https://api.listenbrainz.org:65536", bad_shape()),
            ("https://", bad_shape()),
            ("https:///art.jpg", bad_shape()),
            ("https://:443", bad_shape()),
            ("https://api.listenbrainz.org:443:80", bad_shape()),
        ];
        for (url, expected) in cases {
            assert_eq!(admit_returned_url(&grants, url), Err(expected), "{url:?}");
        }
    }

    /// Verifies: SEC-EXT-026
    #[test]
    fn a_lan_destination_needs_that_exact_admin_grant() {
        assert_eq!(
            admit_lan(None, "homeassistant.local", 8123),
            Err(LanRefuse::NotGranted)
        );
        assert_eq!(admit_lan(None, "127.0.0.1", 80), Err(LanRefuse::NotGranted));
        assert_eq!(admit_lan(None, "::1", 80), Err(LanRefuse::NotGranted));
        assert_eq!(
            admit_lan(None, "169.254.169.254", 80),
            Err(LanRefuse::NotGranted)
        );
        assert_eq!(admit_lan(None, "0.0.0.0", 80), Err(LanRefuse::NotGranted));

        let home = LanGrant {
            host: "homeassistant.local".to_owned(),
            port: 8123,
        };
        let home_mixed = LanGrant {
            host: "HomeAssistant.Local".to_owned(),
            port: 8123,
        };
        assert_eq!(admit_lan(Some(&home), "homeassistant.local", 8123), Ok(()));
        assert_eq!(admit_lan(Some(&home), "HomeAssistant.Local", 8123), Ok(()));
        assert_eq!(
            admit_lan(Some(&home_mixed), "homeassistant.local", 8123),
            Ok(())
        );
        assert_eq!(
            admit_lan(Some(&home), "homeassistant.local", 8124),
            Err(LanRefuse::NotGranted)
        );
        assert_eq!(
            admit_lan(Some(&home), "other.local", 8123),
            Err(LanRefuse::NotGranted)
        );
        assert_eq!(
            admit_lan(Some(&home), "homeassistant.local.", 8123),
            Err(LanRefuse::NotGranted)
        );
        assert_eq!(
            admit_lan(Some(&home), " homeassistant.local", 8123),
            Err(LanRefuse::NotGranted)
        );
        assert_eq!(
            admit_lan(Some(&home), "homeassistant.local%eth0", 8123),
            Err(LanRefuse::NotGranted)
        );

        let lan = LanGrant {
            host: "192.168.1.20".to_owned(),
            port: 8123,
        };
        assert_eq!(admit_lan(Some(&lan), "192.168.1.20", 8123), Ok(()));
        assert_eq!(
            admit_lan(Some(&lan), "192.168.1.21", 8123),
            Err(LanRefuse::NotGranted)
        );
        assert_eq!(
            admit_lan(Some(&lan), "192.168.1.20", 80),
            Err(LanRefuse::NotGranted)
        );
        assert_eq!(
            admit_lan(Some(&lan), "192.168.001.020", 8123),
            Err(LanRefuse::NotGranted)
        );
        let other_private = LanGrant {
            host: "10.1.2.3".to_owned(),
            port: 9,
        };
        assert_eq!(admit_lan(Some(&other_private), "10.1.2.3", 9), Ok(()));
        let mapped_private = LanGrant {
            host: "::ffff:192.168.1.20".to_owned(),
            port: 8123,
        };
        assert_eq!(
            admit_lan(Some(&mapped_private), "::ffff:192.168.1.20", 8123),
            Ok(())
        );
        let tailnet = LanGrant {
            host: "fd7a:115c:a1e0::1".to_owned(),
            port: 443,
        };
        assert_eq!(admit_lan(Some(&tailnet), "FD7A:115c:a1e0::1", 443), Ok(()));
        assert_eq!(
            admit_lan(Some(&tailnet), "fd7a:115c:a1e0:0:0:0:0:1", 443),
            Err(LanRefuse::NotGranted)
        );
        let documentation = LanGrant {
            host: "192.0.2.1".to_owned(),
            port: 443,
        };
        assert_eq!(admit_lan(Some(&documentation), "192.0.2.1", 443), Ok(()));
        assert_eq!(
            admit_lan(Some(&documentation), "192.0.2.2", 443),
            Err(LanRefuse::NotGranted)
        );
    }

    /// Verifies: SEC-EXT-026
    #[test]
    fn loopback_link_local_and_unspecified_addresses_are_never_granted() {
        let cases = [
            "127.0.0.1",
            "127.0.0.2",
            "::1",
            "0:0:0:0:0:0:0:1",
            "::FFFF:127.0.0.1",
            "169.254.169.254",
            "169.254.1.1",
            "fe80::1",
            "FE80::1",
            "0.0.0.0",
            "::",
            "::0",
            "127.0.0.1%lo",
            "fe80::1%eth0",
        ];
        for host in cases {
            let named = LanGrant {
                host: host.to_owned(),
                port: 80,
            };
            assert_eq!(
                admit_lan(Some(&named), host, 80),
                Err(LanRefuse::Never),
                "{host:?}"
            );
            assert_eq!(
                admit_lan(Some(&named), host, 81),
                Err(LanRefuse::Never),
                "{host:?}"
            );
        }
        let other = LanGrant {
            host: "192.168.1.20".to_owned(),
            port: 80,
        };
        assert_eq!(
            admit_lan(Some(&other), "127.0.0.1", 80),
            Err(LanRefuse::Never)
        );
        assert_eq!(admit_lan(Some(&other), "::1", 9), Err(LanRefuse::Never));
        assert_eq!(
            admit_lan(Some(&other), "169.254.169.254", 80),
            Err(LanRefuse::Never)
        );
    }
}
