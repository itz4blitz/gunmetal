//! Links to outside sites, and where to go after signing in.
//!
//! [`Link::parse`] is the one URL validator the clients use, through WASM,
//! before they show a URL from metadata or a person as a link
//! (SEC-API-047, SEC-CLI-002, SEC-STD-015). It reads the scheme, host and
//! port the way the WHATWG URL standard does, so it agrees with the
//! browser about where a link goes, and accepts only `https` with a
//! non-empty host. Plain `http` is refused too: the owner accepted
//! "https only" where the requirements disagreed (decision D-78). The
//! accepted link carries a rewritten URL that starts with
//! `https://host[:port]/`, so no URL parser, WHATWG or not, can read a
//! different host from it, and the host to show on the sheet before it
//! opens.
//!
//! Two choices are stricter than the standard. A host with characters
//! outside ASCII is refused, so a look-alike name such as `exаmple.com`
//! with a Cyrillic `а` stays plain text; its punycode form is accepted and
//! shown as punycode. A user name or password in the URL is refused,
//! because `https://bank.example@evil.example` goes to `evil.example`.
//!
//! [`return_target`] validates the post-sign-in return target: a path that
//! starts with exactly one `/` and names a known client route, or the home
//! route (SEC-API-070, SEC-HIS-032).

use std::net::{Ipv4Addr, Ipv6Addr};

use crate::untrusted::Untrusted;

/// A link that may be shown as a link and opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    href: String,
    host: String,
}

/// Why a URL must be shown as plain text instead of a link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkError {
    /// It has no scheme, so it is relative, or begins like `//host`.
    NotAbsolute,
    /// Its scheme is not `https`.
    NotHttps,
    /// It carries a user name or password.
    Credentials,
    /// Its host is empty.
    NoHost,
    /// Its host is not one a browser would open, or holds characters
    /// outside ASCII.
    BadHost,
    /// Its port is not a number from 0 to 65535.
    BadPort,
}

impl Link {
    /// Validates a URL from metadata or a person.
    ///
    /// # Errors
    ///
    /// A [`LinkError`] for anything that must stay plain text.
    pub fn parse(raw: Untrusted<&str>) -> Result<Self, LinkError> {
        // The standard's first steps: strip leading and trailing C0
        // controls and spaces, then remove every tab and newline.
        let input: String = raw
            .into_inner()
            .trim_matches(|c: char| c <= ' ')
            .chars()
            .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
            .collect();
        let (scheme, rest) = input.split_once(':').ok_or(LinkError::NotAbsolute)?;
        let mut scheme_chars = scheme.chars();
        let is_scheme = scheme_chars.next().is_some_and(|c| c.is_ascii_alphabetic())
            && scheme_chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
        if !is_scheme {
            return Err(LinkError::NotAbsolute);
        }
        if !scheme.eq_ignore_ascii_case("https") {
            return Err(LinkError::NotHttps);
        }
        // A special scheme skips any run of slashes and backslashes, and
        // its authority ends at the first of these four.
        let rest = rest.trim_start_matches(['/', '\\']);
        let authority = rest.split(['/', '\\', '?', '#']).next().unwrap_or_default();
        let tail = rest.get(authority.len()..).unwrap_or_default();
        if authority.contains('@') {
            return Err(LinkError::Credentials);
        }
        let (host, port) = if let Some(bracketed) = authority.strip_prefix('[') {
            let (inside, after) = bracketed.split_once(']').ok_or(LinkError::BadHost)?;
            let port = if after.is_empty() {
                None
            } else {
                Some(after.strip_prefix(':').ok_or(LinkError::BadHost)?)
            };
            (ipv6_host(inside)?, port)
        } else {
            let (host, port) = authority
                .split_once(':')
                .map_or((authority, None), |(host, port)| (host, Some(port)));
            (domain_host(host)?, port)
        };
        // An empty port, as in `host:/`, is no port at all.
        let port = port.filter(|port| !port.is_empty());
        let port = match port.map(port_number).transpose()? {
            Some(443) | None => String::new(),
            Some(port) => format!(":{port}"),
        };
        let tail = escaped_tail(tail);
        let path = tail.strip_prefix('/').unwrap_or(&tail);
        Ok(Self {
            href: format!("https://{host}{port}/{path}"),
            host,
        })
    }

    /// The URL to open: `https://`, the host, the port unless it is 443,
    /// then the path, query and fragment. Those three hold only ASCII
    /// letters, digits and ``-._~!$&()*+,;=:@/?#%``; every other octet is
    /// percent-encoded, so the URL can be placed in an attribute or a
    /// quoted string without ending it.
    #[must_use]
    pub fn href(&self) -> &str {
        &self.href
    }

    /// The host to show before opening it: a lower-case ASCII domain, a
    /// dotted IPv4 address or a bracketed IPv6 address.
    #[must_use]
    pub fn host(&self) -> &str {
        &self.host
    }
}

/// An IPv6 host written between brackets, in its normal form.
fn ipv6_host(inside: &str) -> Result<String, LinkError> {
    let address: Ipv6Addr = inside.parse().map_err(|_| LinkError::BadHost)?;
    Ok(format!("[{address}]"))
}

/// A domain or IPv4 host, read as the WHATWG host parser reads it, except
/// that anything outside ASCII is refused.
fn domain_host(text: &str) -> Result<String, LinkError> {
    if text.is_empty() {
        return Err(LinkError::NoHost);
    }
    let decoded = percent_decode(text);
    if !decoded.is_ascii() {
        return Err(LinkError::BadHost);
    }
    let domain: String = decoded
        .iter()
        .map(|octet| char::from(octet.to_ascii_lowercase()))
        .collect();
    if domain.chars().any(forbidden_in_domain) {
        return Err(LinkError::BadHost);
    }
    if ends_in_number(&domain) {
        return ipv4(&domain)
            .map(|address| address.to_string())
            .ok_or(LinkError::BadHost);
    }
    Ok(domain)
}

/// Replaces each `%` followed by two hexadecimal digits with the octet
/// they name, leaving any other `%` as it is.
fn percent_decode(text: &str) -> Vec<u8> {
    let mut decoded = Vec::new();
    let mut rest = text.as_bytes();
    while let Some((&first, after_first)) = rest.split_first() {
        match (first, after_first) {
            (b'%', [high, low, after @ ..])
                if high.is_ascii_hexdigit() && low.is_ascii_hexdigit() =>
            {
                let value = [high, low].iter().fold(0_u32, |value, digit| {
                    value
                        .saturating_mul(16)
                        .saturating_add(char::from(**digit).to_digit(16).unwrap_or_default())
                });
                decoded.push(u8::try_from(value).unwrap_or_default());
                rest = after;
            }
            _ => {
                decoded.push(first);
                rest = after_first;
            }
        }
    }
    decoded
}

/// The WHATWG URL standard's forbidden domain code points.
fn forbidden_in_domain(c: char) -> bool {
    c <= ' '
        || matches!(
            c,
            '#' | '%' | '/' | ':' | '<' | '>' | '?' | '@' | '[' | '\\' | ']' | '^' | '|' | '\u{7F}'
        )
}

/// Whether the last label, ignoring one trailing dot, is a number, which
/// makes the whole host an IPv4 address.
fn ends_in_number(domain: &str) -> bool {
    let last = domain
        .strip_suffix('.')
        .unwrap_or(domain)
        .rsplit('.')
        .next()
        .unwrap_or_default();
    (!last.is_empty() && last.bytes().all(|octet| octet.is_ascii_digit()))
        || ipv4_number(last).is_some()
}

/// One part of an IPv4 address in decimal, octal (a leading `0`) or
/// hexadecimal (a leading `0x`), saturated at `u64::MAX`. A lone `0` reads
/// as an empty octal number, which is zero, as the standard's decimal
/// reading of it is.
fn ipv4_number(part: &str) -> Option<u64> {
    if part.is_empty() {
        return None;
    }
    let (digits, radix) = if let Some(hex) = part.strip_prefix("0x") {
        (hex, 16)
    } else if let Some(octal) = part.strip_prefix('0') {
        (octal, 8)
    } else {
        (part, 10)
    };
    digits.chars().try_fold(0_u64, |value, c| {
        c.to_digit(radix).map(|digit| {
            value
                .saturating_mul(u64::from(radix))
                .saturating_add(u64::from(digit))
        })
    })
}

/// The WHATWG IPv4 parser: one to four parts, the last filling every octet
/// the others leave.
fn ipv4(domain: &str) -> Option<Ipv4Addr> {
    let mut parts = domain.strip_suffix('.').unwrap_or(domain).rsplit('.');
    let last = ipv4_number(parts.next().unwrap_or_default())?;
    let mut leading = parts.map(ipv4_number).collect::<Option<Vec<u64>>>()?;
    leading.reverse();
    let (limit, shift) = match leading.len() {
        0 => (1 << 32, 32),
        1 => (1 << 24, 24),
        2 => (1 << 16, 16),
        3 => (1 << 8, 8),
        _ => return None,
    };
    if last >= limit || leading.iter().any(|&part| part > 255) {
        return None;
    }
    let high = leading
        .iter()
        .fold(0_u64, |high, &part| (high << 8).saturating_add(part));
    let [_, _, _, _, a, b, c, d] = (high << shift).saturating_add(last).to_be_bytes();
    Some(Ipv4Addr::new(a, b, c, d))
}

/// The characters, besides ASCII letters and digits, that the path, query
/// and fragment of an accepted link keep: RFC 3986's unreserved characters
/// and sub-delimiters without the apostrophe, the delimiters `:@/?#`, and
/// `%`, so that an escape already written is not escaped again.
const KEPT_IN_TAIL: &[u8] = b"-._~!$&()*+,;=:@/?#%";

/// The upper-case hexadecimal digits.
const HEX: [u8; 16] = *b"0123456789ABCDEF";

/// What follows the authority, written for [`Link::href`]. A backslash in
/// the path becomes the slash a browser reads it as; the path ends at the
/// first `?` or `#`. Every octet that is not a letter, a digit or in
/// [`KEPT_IN_TAIL`] is percent-encoded, as the octets of UTF-8 for a
/// character outside ASCII.
fn escaped_tail(tail: &str) -> String {
    let mut escaped = String::new();
    let mut in_path = true;
    for octet in tail.bytes() {
        in_path = in_path && !matches!(octet, b'?' | b'#');
        if octet == b'\\' && in_path {
            escaped.push('/');
        } else if octet.is_ascii_alphanumeric() || KEPT_IN_TAIL.contains(&octet) {
            escaped.push(char::from(octet));
        } else {
            escaped.push('%');
            for nibble in [octet >> 4, octet & 0xF] {
                let digit = HEX.get(usize::from(nibble)).copied().unwrap_or_default();
                escaped.push(char::from(digit));
            }
        }
    }
    escaped
}

/// A port: decimal digits, leading zeros allowed, at most 65535.
fn port_number(text: &str) -> Result<u16, LinkError> {
    text.bytes()
        .try_fold(0_u32, |value, octet| {
            char::from(octet)
                .to_digit(10)
                .map(|digit| value.saturating_mul(10).saturating_add(digit))
        })
        .and_then(|value| u16::try_from(value).ok())
        .ok_or(LinkError::BadPort)
}

/// The home route.
pub const HOME: &str = "/";

/// Where to send a person after they sign in: a known client route, or
/// [`HOME`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnTarget(String);

impl ReturnTarget {
    /// The path, which starts with exactly one `/`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validates a post-sign-in return target against the client's routes.
///
/// A route is a path whose segments are literal or `*`, which matches one
/// segment of ASCII letters, digits, `-` and `_`. The target is returned
/// unchanged when it starts with exactly one `/`, is not followed by `/` or
/// `\`, and matches a route; anything else becomes [`HOME`]. Nothing is
/// decoded, so a percent-encoded slash never matches.
#[must_use]
pub fn return_target(raw: Untrusted<&str>, routes: &[&str]) -> ReturnTarget {
    let raw = raw.into_inner();
    let known = raw
        .strip_prefix('/')
        .filter(|path| !path.starts_with(['/', '\\']))
        .is_some_and(|path| {
            routes.iter().any(|route| {
                route
                    .strip_prefix('/')
                    .is_some_and(|route| matches_route(route, path))
            })
        });
    ReturnTarget(if known { raw } else { HOME }.to_owned())
}

/// Whether `path` has as many segments as `route` and each matches.
fn matches_route(route: &str, path: &str) -> bool {
    route.split('/').count() == path.split('/').count()
        && route
            .split('/')
            .zip(path.split('/'))
            .all(|(expected, segment)| {
                if expected == "*" {
                    !segment.is_empty()
                        && segment.bytes().all(|octet| {
                            octet.is_ascii_alphanumeric() || matches!(octet, b'-' | b'_')
                        })
                } else {
                    expected == segment
                }
            })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use proptest::sample::select;

    fn link(raw: &str) -> Result<Link, LinkError> {
        Link::parse(Untrusted::new(raw))
    }

    fn accepted(href: &str, host: &str) -> Link {
        Link {
            href: href.to_owned(),
            host: host.to_owned(),
        }
    }

    /// Verifies: SEC-API-047, SEC-CLI-002, SEC-STD-015
    #[test]
    fn accepts_https_links_and_names_their_host() {
        let cases = [
            ("https://example.com", "https://example.com/", "example.com"),
            (
                "HTTPS://Example.COM/Path?q=1#frag",
                "https://example.com/Path?q=1#frag",
                "example.com",
            ),
            (
                "  https://example.com/  ",
                "https://example.com/",
                "example.com",
            ),
            (
                "\0\u{1F} https://example.com",
                "https://example.com/",
                "example.com",
            ),
            (
                "ht\ttps://exa\nmple.com/a\rb",
                "https://example.com/ab",
                "example.com",
            ),
            ("https:example.com", "https://example.com/", "example.com"),
            (
                "https:\\\\example.com\\path",
                "https://example.com/path",
                "example.com",
            ),
            (
                "https:////example.com",
                "https://example.com/",
                "example.com",
            ),
            (
                "https://example.com?x",
                "https://example.com/?x",
                "example.com",
            ),
            (
                "https://example.com#top",
                "https://example.com/#top",
                "example.com",
            ),
            (
                "https://EXAMPLE.com.",
                "https://example.com./",
                "example.com.",
            ),
            (
                "https://%65xample.com",
                "https://example.com/",
                "example.com",
            ),
            ("https://a..b", "https://a..b/", "a..b"),
            // No label is a number, so this is a domain, as the standard says.
            ("https://..", "https://../", ".."),
            (
                "https://xn--bcher-kva.example",
                "https://xn--bcher-kva.example/",
                "xn--bcher-kva.example",
            ),
        ];
        for (raw, href, host) in cases {
            assert_eq!(link(raw), Ok(accepted(href, host)), "{raw:?}");
        }
    }

    /// The port is kept unless it is https's own.
    ///
    /// Verifies: SEC-API-047, SEC-STD-015
    #[test]
    fn keeps_a_port_other_than_443() {
        let cases = [
            ("https://example.com:443/x", "https://example.com/x"),
            ("https://example.com:0443", "https://example.com/"),
            ("https://example.com:/x", "https://example.com/x"),
            ("https://[2001:db8::1]:", "https://[2001:db8::1]/"),
            ("https://example.com:8443/x", "https://example.com:8443/x"),
            ("https://example.com:0", "https://example.com:0/"),
            ("https://example.com:65535", "https://example.com:65535/"),
            ("https://[2001:db8::1]:8443/", "https://[2001:db8::1]:8443/"),
        ];
        for (raw, href) in cases {
            assert_eq!(
                link(raw).map(|link| link.href().to_owned()),
                Ok(href.to_owned()),
                "{raw}"
            );
        }
    }

    /// What follows the host is written so that it means the same to
    /// every URL parser and cannot end an attribute or a quoted string it
    /// is placed in: a space, a quote, an angle bracket, a control or
    /// anything outside ASCII becomes a percent escape.
    #[test]
    fn percent_encodes_what_follows_the_host() {
        let cases = [
            ("https://example.com/a b", "https://example.com/a%20b"),
            (
                "https://example.com/\"><script>alert(1)</script>",
                "https://example.com/%22%3E%3Cscript%3Ealert(1)%3C/script%3E",
            ),
            (
                "https://example.com/' onclick='x",
                "https://example.com/%27%20onclick=%27x",
            ),
            (
                "https://example.com/caf\u{E9}",
                "https://example.com/caf%C3%A9",
            ),
            (
                "https://example.com/\u{1D11E}?\u{20AC}#\u{FEFF}",
                "https://example.com/%F0%9D%84%9E?%E2%82%AC#%EF%BB%BF",
            ),
            (
                "https://example.com/a\u{1}b\u{1F}c\u{7F}d\u{80}e\0f",
                "https://example.com/a%01b%1Fc%7Fd%C2%80e%00f",
            ),
            ("https://example.com?a b", "https://example.com/?a%20b"),
            ("https://example.com#a b", "https://example.com/#a%20b"),
            (
                "https://example.com:8443/a b",
                "https://example.com:8443/a%20b",
            ),
            ("https://[2001:db8::1]/[a]", "https://[2001:db8::1]/%5Ba%5D"),
        ];
        for (raw, href) in cases {
            assert_eq!(
                link(raw).map(|link| link.href().to_owned()),
                Ok(href.to_owned()),
                "{raw:?}"
            );
        }
    }

    /// Every printable ASCII character after the host, in the path, the
    /// query and the fragment. Letters, digits and the characters RFC 3986
    /// allows there stay, an apostrophe aside; `%` stays so that an escape
    /// already written is not escaped twice.
    #[test]
    fn keeps_only_url_characters_after_the_host() {
        let punctuation = " !\"$%&'()*+,-./:;<=>@[]^_`{|}~";
        let encoded = "%20!%22$%&%27()*+,-./:;%3C=%3E@%5B%5D%5E_%60%7B%7C%7D~";
        let alphanumeric = "09AZaz";
        for lead in ["/", "/?", "/#", "/?#", "/#?"] {
            assert_eq!(
                link(&format!(
                    "https://example.com{lead}{punctuation}{alphanumeric}"
                ))
                .map(|link| link.href().to_owned()),
                Ok(format!("https://example.com{lead}{encoded}{alphanumeric}")),
                "{lead}"
            );
        }
    }

    /// A browser reads a backslash in the path of an https URL as a slash,
    /// so it is written as one. In the query and the fragment it is only a
    /// character, and is escaped.
    #[test]
    fn writes_a_backslash_as_a_slash_in_the_path_only() {
        let cases = [
            ("https://example.com/a\\b\\", "https://example.com/a/b/"),
            ("https://example.com\\a\\b", "https://example.com/a/b"),
            (
                "https://example.com/a\\b?c\\d",
                "https://example.com/a/b?c%5Cd",
            ),
            (
                "https://example.com/a\\b#c\\d",
                "https://example.com/a/b#c%5Cd",
            ),
            (
                "https://example.com/a?b\\c#d\\e",
                "https://example.com/a?b%5Cc#d%5Ce",
            ),
            (
                "https://example.com/a#b\\c?d\\e/f\\g",
                "https://example.com/a#b%5Cc?d%5Ce/f%5Cg",
            ),
            ("https://example.com?\\", "https://example.com/?%5C"),
            ("https://example.com#\\", "https://example.com/#%5C"),
        ];
        for (raw, href) in cases {
            assert_eq!(
                link(raw).map(|link| link.href().to_owned()),
                Ok(href.to_owned()),
                "{raw:?}"
            );
        }
    }

    /// An escape already in the URL is kept as written, valid or not, so
    /// the link reads back unchanged and nothing is decoded.
    #[test]
    fn keeps_percent_escapes_after_the_host_as_written() {
        for tail in ["%41%2f%2F%5c", "%", "%zz%4", "%25%2525", "?%20#%20"] {
            let raw = format!("https://example.com/{tail}");
            assert_eq!(
                link(&raw).map(|link| link.href().to_owned()),
                Ok(raw.clone()),
                "{raw}"
            );
        }
    }

    /// Hosts that end in a number are IPv4 addresses, written in any of
    /// the forms the WHATWG URL standard reads, and shown in dotted form.
    ///
    /// Verifies: SEC-API-047, SEC-STD-015
    #[test]
    fn reads_ip_address_hosts_the_way_browsers_do() {
        let cases = [
            ("https://192.0.2.1/", "192.0.2.1"),
            ("https://192.0.2.1.", "192.0.2.1"),
            ("https://0xC0.0.2.1", "192.0.2.1"),
            ("https://0XC0.0.2.1", "192.0.2.1"),
            ("https://0300.0.2.1", "192.0.2.1"),
            ("https://3221225985", "192.0.2.1"),
            ("https://192.513", "192.0.2.1"),
            ("https://192.0.513", "192.0.2.1"),
            ("https://0x", "0.0.0.0"),
            ("https://4294967295", "255.255.255.255"),
            ("https://255.255.255.255", "255.255.255.255"),
            ("https://[2001:DB8:0:0:0:0:0:1]", "[2001:db8::1]"),
            ("https://[::ffff:192.0.2.1]", "[::ffff:192.0.2.1]"),
        ];
        for (raw, host) in cases {
            let parsed = link(raw);
            assert_eq!(parsed.as_ref().map(Link::host), Ok(host), "{raw}");
            assert_eq!(
                parsed.as_ref().map(Link::href),
                Ok(format!("https://{host}/").as_str()),
                "{raw}"
            );
        }
    }

    /// Verifies: SEC-API-047, SEC-CLI-002, SEC-STD-015
    #[test]
    fn refuses_every_scheme_but_https() {
        let cases = [
            ("", LinkError::NotAbsolute),
            ("example.com", LinkError::NotAbsolute),
            ("//example.com", LinkError::NotAbsolute),
            ("/\\example.com", LinkError::NotAbsolute),
            ("\\\\example.com", LinkError::NotAbsolute),
            ("/path:x", LinkError::NotAbsolute),
            ("1https://example.com", LinkError::NotAbsolute),
            ("ht tps://example.com", LinkError::NotAbsolute),
            ("\u{A0}https://example.com", LinkError::NotAbsolute),
            ("ｈｔｔｐｓ://example.com", LinkError::NotAbsolute),
            (":https://example.com", LinkError::NotAbsolute),
            ("http://example.com", LinkError::NotHttps),
            ("javascript:alert(1)", LinkError::NotHttps),
            ("JaVaScRiPt:alert(1)", LinkError::NotHttps),
            (" javascript:alert(1)", LinkError::NotHttps),
            ("java\tscript:alert(1)", LinkError::NotHttps),
            ("\u{1}javascript:alert(1)", LinkError::NotHttps),
            (
                "data:text/html,<script>alert(1)</script>",
                LinkError::NotHttps,
            ),
            ("vbscript:msgbox(1)", LinkError::NotHttps),
            ("file:///etc/passwd", LinkError::NotHttps),
            (
                "intent://scan/#Intent;scheme=zxing;end",
                LinkError::NotHttps,
            ),
            ("ftp://example.com", LinkError::NotHttps),
            ("wss://example.com", LinkError::NotHttps),
            ("https+x://example.com", LinkError::NotHttps),
            ("httpss://example.com", LinkError::NotHttps),
            ("a1+-.:x", LinkError::NotHttps),
        ];
        for (raw, error) in cases {
            assert_eq!(link(raw), Err(error), "{raw:?}");
        }
    }

    /// Verifies: SEC-API-047, SEC-STD-015
    #[test]
    fn refuses_credentials_missing_hosts_and_bad_ports() {
        let cases = [
            ("https://user:pass@example.com", LinkError::Credentials),
            ("https://@example.com", LinkError::Credentials),
            ("https://example.com@evil.example", LinkError::Credentials),
            ("https://", LinkError::NoHost),
            ("https:///", LinkError::NoHost),
            ("https:", LinkError::NoHost),
            ("https://?x", LinkError::NoHost),
            ("https://#x", LinkError::NoHost),
            ("https://:443", LinkError::NoHost),
            ("https://example.com:65536", LinkError::BadPort),
            (
                "https://example.com:99999999999999999999",
                LinkError::BadPort,
            ),
            ("https://example.com:x", LinkError::BadPort),
            ("https://example.com:-1", LinkError::BadPort),
            ("https://example.com:+443", LinkError::BadPort),
            ("https://example.com:443:443", LinkError::BadPort),
            ("https://[::1]:x", LinkError::BadPort),
        ];
        for (raw, error) in cases {
            assert_eq!(link(raw), Err(error), "{raw:?}");
        }
    }

    /// Hosts with characters no domain may hold, including look-alike
    /// letters from other scripts, which stay plain text so they cannot
    /// pass for a familiar name. Their punycode form is accepted and shown
    /// as punycode.
    ///
    /// Verifies: SEC-API-047, SEC-CLI-002, SEC-STD-015
    #[test]
    fn refuses_hosts_a_browser_would_not_open_or_that_look_like_another() {
        for raw in [
            "https://ex\u{430}mple.com",
            "https://\u{FF45}xample.com",
            "https://b\u{FC}cher.example",
            "https://%C3%BCber.example",
            "https://exa mple.com",
            "https://ex%20ample.com",
            "https://exa<mple.com",
            "https://ex^ample.com",
            "https://ex|ample.com",
            "https://ex%2fample.com",
            "https://ex%5Cample.com",
            "https://ex%00ample.com",
            "https://ex%7Fample.com",
            "https://ex%ample.com",
            // Not an escape, so the % stays and is refused; read as one it
            // would give a backtick, which a domain may hold.
            "https://ex%6zample.com",
            "https://example%",
            "https://example.com]",
            "https://[::1",
            "https://[::1]x",
            "https://[]",
            "https://[not-ipv6]",
            "https://[fe80::1%25eth0]",
            "https://1.2.3.4.5",
            "https://256.0.0.1",
            "https://1.256.0.1",
            "https://1.2.65536",
            "https://192.16777216",
            "https://4294967296",
            "https://0x100000000",
            "https://example.123",
            "https://1.2.3.09",
            "https://0x1g.example.0x1",
            "https://x.1",
            "https://1..2",
        ] {
            assert_eq!(link(raw), Err(LinkError::BadHost), "{raw:?}");
        }
    }

    /// Schemes an attacker would try, each written in mixed case and padded
    /// with the characters URL parsers strip.
    fn obfuscated(scheme: &'static str) -> impl Strategy<Value = String> {
        (
            proptest::collection::vec(any::<bool>(), scheme.len()),
            proptest::collection::vec(select(vec!["", "\t", "\n", "\r"]), scheme.len()),
            select(vec!["", " ", "\0", "\u{1F}", " \u{8} "]),
            ".{0,24}",
        )
            .prop_map(move |(upper, inserts, lead, tail)| {
                let mut text = lead.to_owned();
                for ((c, upper), insert) in scheme.chars().zip(upper).zip(inserts) {
                    text.push_str(insert);
                    text.push(if upper { c.to_ascii_uppercase() } else { c });
                }
                text.push(':');
                text.push_str(&tail);
                text
            })
    }

    proptest! {
        /// Verifies: SEC-API-047, SEC-CLI-002, SEC-STD-015
        #[test]
        fn refuses_dangerous_and_plain_http_schemes_however_written(
            raw in prop_oneof![
                obfuscated("javascript"),
                obfuscated("data"),
                obfuscated("vbscript"),
                obfuscated("file"),
                obfuscated("intent"),
                obfuscated("http"),
            ],
        ) {
            prop_assert_eq!(link(&raw), Err(LinkError::NotHttps));
        }

        /// Verifies: SEC-API-047, SEC-CLI-002
        #[test]
        fn refuses_scheme_relative_and_path_values(
            raw in "(//|/\\\\|\\\\\\\\|/)[a-z.:@]{0,20}",
        ) {
            prop_assert_eq!(link(&raw), Err(LinkError::NotAbsolute));
        }

        /// Whatever is accepted opens an https URL whose host is the one
        /// shown, with nothing between the host and the path that another
        /// URL parser could read differently, and reads back unchanged.
        ///
        /// Verifies: SEC-API-047, SEC-CLI-002, SEC-STD-015
        #[test]
        fn an_accepted_link_opens_the_host_it_shows(
            raw in prop_oneof![
                obfuscated("https"),
                "[hH][tT][tT][pP][sS]:[/\\\\]{0,3}[a-zA-Z0-9.%-]{1,12}(:[0-9]{0,6})?([/?#\\\\].{0,12})?",
                ".{0,40}",
            ],
        ) {
            let parsed = link(&raw);
            let shown = parsed.as_ref().map(|accepted| {
                let host = accepted.host();
                let after = accepted.href().strip_prefix("https://").and_then(|rest| rest.strip_prefix(host));
                let tail = after.map(|after| after.trim_start_matches(|c: char| c == ':' || c.is_ascii_digit()));
                (
                    !host.is_empty() && host.bytes().all(|b| b.is_ascii_graphic() && !b"/\\?#@%".contains(&b)),
                    tail.is_some_and(|tail| {
                        tail.starts_with('/')
                            && tail.bytes().all(|b| b.is_ascii_alphanumeric() || b"-._~!$&()*+,;=:@/?#%".contains(&b))
                    }),
                    link(accepted.href()) == Ok(accepted.clone()),
                )
            });
            prop_assert!(shown.is_err() || shown == Ok((true, true, true)), "{raw:?} gave {parsed:?}");
        }
    }

    /// The client routes these tests use.
    const ROUTES: [&str; 5] = [
        "/",
        "/library",
        "/albums/*",
        "/albums/*/tracks",
        "/settings/account",
    ];

    fn target(raw: &str) -> String {
        return_target(Untrusted::new(raw), &ROUTES)
            .as_str()
            .to_owned()
    }

    /// Verifies: SEC-API-070, SEC-HIS-032
    #[test]
    fn returns_to_a_known_client_route() {
        for raw in [
            "/",
            "/library",
            "/albums/alb_0123456789abcdefghjkmnpqrs",
            "/albums/x/tracks",
            "/albums/A-b_9/tracks",
            "/settings/account",
        ] {
            assert_eq!(target(raw), raw);
        }
    }

    /// Verifies: SEC-API-070, SEC-HIS-032
    #[test]
    fn sends_anything_else_home() {
        for raw in [
            "",
            "library",
            "//evil.example",
            "/\\evil.example",
            "\\\\evil.example",
            "https://evil.example",
            "javascript:alert(1)",
            "/library/",
            "/library?x=1",
            "/library#x",
            "/LIBRARY",
            "/library\n",
            " /library",
            "/albums",
            "/albums/",
            "/albums//tracks",
            "/albums/*",
            "/albums/x/tracks/extra",
            "/albums/../settings/account",
            "/albums/./tracks",
            "/albums/%2e%2e",
            "/albums/a b",
            "/albums/x:y",
            "/albums/\u{FF41}",
            "/unknown",
            "/%2F%2Fevil.example",
            "/\u{FF0F}evil.example",
        ] {
            assert_eq!(target(raw), HOME, "{raw:?}");
        }
    }

    /// A value starting with two slashes or a slash and a backslash goes
    /// home even when a route would match it, so the rule does not depend
    /// on the route list.
    ///
    /// Verifies: SEC-API-070, SEC-HIS-032
    #[test]
    fn sends_scheme_relative_values_home_whatever_the_routes() {
        let routes = ["//*", "/\\evil", "library"];
        for raw in ["//evil", "/\\evil", "library", "/library"] {
            assert_eq!(
                return_target(Untrusted::new(raw), &routes).as_str(),
                HOME,
                "{raw:?}"
            );
        }
    }

    proptest! {
        /// Verifies: SEC-API-070, SEC-HIS-032
        #[test]
        fn sends_hostile_targets_home(
            raw in prop_oneof![
                obfuscated("javascript"),
                obfuscated("data"),
                obfuscated("https"),
                "(//|/\\\\|\\\\\\\\|/%2[fF]|/%5[cC])[a-zA-Z0-9.:@%/\\\\]{0,20}",
                "[/\\\\]?[a-zA-Z]{0,8}:.{0,16}",
            ],
        ) {
            prop_assert_eq!(target(&raw), HOME);
        }

        /// The result is the value itself or the home route, never
        /// anything built from it.
        ///
        /// Verifies: SEC-API-070, SEC-HIS-032
        #[test]
        fn returns_the_value_itself_or_home(
            raw in prop_oneof![
                "/(library|albums|settings)(/[a-zA-Z0-9_*.-]{0,6}){0,3}",
                ".{0,24}",
            ],
        ) {
            let returned = target(&raw);
            prop_assert!(returned == raw || returned == HOME, "{raw:?} gave {returned:?}");
        }
    }
}
