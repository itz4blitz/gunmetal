//! Links to outside sites: the core's link filter, for the browser
//! (SEC-API-047, SEC-CLI-002).
//!
//! [`parse_link`] is the facade's link filter: one call of the core's one
//! validator, [`link::Link::parse`], with its answer turned into mirror
//! types, so the browser and the server give the same answer for every URL
//! the facade reads. They differ in one place. The facade reads no string
//! longer than [`MAX_LINK_OCTETS`], the core's long-text limit, and keeps a
//! longer one as plain text; the core's filter has no limit of its own and
//! would accept it. That limit, and a reason of its own for it, belong in
//! the core's filter (WP-005). Until they are there, it is the one thing
//! this module decides. The browser calls the filter as `parseLink`,
//! through the wrapper the `export!` line below it writes, which only
//! converts the answer (`export.rs` says why there are two functions).
//!
//! The doc comment of a mirror type or field is copied into its TypeScript
//! declaration, so each is one plain line.

use gunmetal_core::link;
use gunmetal_core::parse::{LimitKind, Limits};
use gunmetal_core::untrusted::Untrusted;
use serde::Serialize;
use tsify::Tsify;

/// The longest URL [`parse_link`] reads, in octets of UTF-8: the core's
/// default limit for a long text field, which is also the longest string
/// its wire decoder takes under its default limits. The core's filter
/// copies what it reads and can write a URL three times as long, and in the
/// browser a failed allocation ends the whole module, so a longer string is
/// kept as plain text without being read.
pub const MAX_LINK_OCTETS: u64 = Limits::DEFAULT.get(LimitKind::LongText);

// Mirrors `link::Link`, whose fields are private. The accessors read are
// `href` and `host`, and they are all it has; a core change that adds one
// is caught by review, not by the compiler (record 12, decision 13).
/// A link that may be shown as a link and opened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Tsify)]
pub struct Link {
    /// The URL to open, rewritten so that every URL parser reads the same host from it.
    pub href: String,
    /// The host to show before the link is opened.
    pub host: String,
}

/// Why a URL must be shown as plain text instead of a link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Tsify)]
pub enum LinkError {
    /// It has no scheme, so it is relative, or begins like `//host`; or it is longer than `MAX_LINK_OCTETS` and was not read.
    NotAbsolute,
    /// Its scheme is not `https`.
    NotHttps,
    /// It carries a user name or password.
    Credentials,
    /// Its host is empty.
    NoHost,
    /// Its host is not one a browser would open, or holds a character no domain may hold.
    BadHost,
    /// Its port is not a number from 0 to 65535.
    BadPort,
}

/// What reading a URL gave: a link, or the reason it stays plain text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Tsify)]
pub enum LinkOutcome {
    /// The URL may be shown as this link.
    Link(Link),
    /// The URL must be shown as plain text, for this reason.
    PlainText(LinkError),
}

/// The TypeScript declarations of this module's mirror types, in the order
/// they are declared above.
pub const DECLARATIONS: [&str; 3] = [Link::DECL, LinkError::DECL, LinkOutcome::DECL];

impl From<link::Link> for Link {
    fn from(value: link::Link) -> Self {
        Self {
            href: value.href().to_owned(),
            host: value.host().to_owned(),
        }
    }
}

impl TryFrom<Link> for link::Link {
    type Error = LinkError;

    /// Reads the mirror's `href` again with the core's validator, the one
    /// door to a core link. A mirror that came back from outside is
    /// untrusted, so its `host` is not believed.
    fn try_from(mirror: Link) -> Result<Self, LinkError> {
        let Link { href, host: _ } = mirror;
        Self::parse(Untrusted::new(href.as_str())).map_err(LinkError::from)
    }
}

impl From<link::LinkError> for LinkError {
    fn from(value: link::LinkError) -> Self {
        match value {
            link::LinkError::NotAbsolute => Self::NotAbsolute,
            link::LinkError::NotHttps => Self::NotHttps,
            link::LinkError::Credentials => Self::Credentials,
            link::LinkError::NoHost => Self::NoHost,
            link::LinkError::BadHost => Self::BadHost,
            link::LinkError::BadPort => Self::BadPort,
        }
    }
}

impl From<LinkError> for link::LinkError {
    fn from(value: LinkError) -> Self {
        match value {
            LinkError::NotAbsolute => Self::NotAbsolute,
            LinkError::NotHttps => Self::NotHttps,
            LinkError::Credentials => Self::Credentials,
            LinkError::NoHost => Self::NoHost,
            LinkError::BadHost => Self::BadHost,
            LinkError::BadPort => Self::BadPort,
        }
    }
}

/// Reads a URL from metadata or a person with the core's link filter.
///
/// A string longer than [`MAX_LINK_OCTETS`] is not read. It stays plain
/// text with [`LinkError::NotAbsolute`], the reason the filter gives text
/// that is not a URL at all, because the core's [`link::LinkError`] has no
/// reason of its own for length. The bound does not cover the copy of the
/// JavaScript string that `wasm-bindgen` makes before this function runs:
/// a caller that may hold a string of many megabytes checks its length
/// first.
#[must_use]
pub fn parse_link(raw: &str) -> LinkOutcome {
    if u64::try_from(raw.len()).unwrap_or(u64::MAX) > MAX_LINK_OCTETS {
        return LinkOutcome::PlainText(LinkError::NotAbsolute);
    }
    match link::Link::parse(Untrusted::new(raw)) {
        Ok(accepted) => LinkOutcome::Link(accepted.into()),
        Err(reason) => LinkOutcome::PlainText(reason.into()),
    }
}

crate::export::export! {
    /// The browser's `parseLink`: [`parse_link`], with its answer converted.
    ///
    /// A string longer than 65,536 octets, counted as UTF-8, is not read. It
    /// comes back as plain text with the reason `NotAbsolute`, the answer for
    /// text that is not a URL, because the core's filter has no reason of its
    /// own for length.
    "parseLink": fn parse_link_export = parse_link(raw: &str) -> LinkOutcome
}

#[cfg(test)]
mod tests {
    use gunmetal_core::link;
    use gunmetal_core::untrusted::Untrusted;

    use super::{DECLARATIONS, Link, LinkError, LinkOutcome, MAX_LINK_OCTETS, parse_link};

    /// The mirror of an accepted link.
    fn mirror(href: &str, host: &str) -> Link {
        Link {
            href: href.to_owned(),
            host: host.to_owned(),
        }
    }

    /// Every reason the core gives, beside its mirror.
    const REASONS: [(link::LinkError, LinkError); 6] = [
        (link::LinkError::NotAbsolute, LinkError::NotAbsolute),
        (link::LinkError::NotHttps, LinkError::NotHttps),
        (link::LinkError::Credentials, LinkError::Credentials),
        (link::LinkError::NoHost, LinkError::NoHost),
        (link::LinkError::BadHost, LinkError::BadHost),
        (link::LinkError::BadPort, LinkError::BadPort),
    ];

    #[test]
    fn a_core_link_becomes_its_mirror_through_both_accessors() {
        let parsed = link::Link::parse(Untrusted::new("HTTPS://Example.COM:8443/x"))
            .expect("the core accepts an https URL");
        assert_eq!(
            Link::from(parsed),
            mirror("https://example.com:8443/x", "example.com")
        );
    }

    #[test]
    fn every_reason_converts_to_its_mirror_and_back() {
        for (reason, mirrored) in REASONS {
            assert_eq!(LinkError::from(reason), mirrored);
            assert_eq!(link::LinkError::from(mirrored), reason);
        }
    }

    /// A mirror goes back to the core only through the core's validator,
    /// so a forged one is refused with the core's own reason and its
    /// `host` is never believed.
    #[test]
    fn a_mirror_returns_to_the_core_only_through_its_validator() {
        let forged_host = mirror("https://example.com:443/x", "evil.example");
        let back = link::Link::try_from(forged_host).expect("the href is an https URL");
        assert_eq!(
            (back.href(), back.host()),
            ("https://example.com/x", "example.com")
        );
        let cases = [
            ("javascript:alert(1)", LinkError::NotHttps),
            ("//example.com", LinkError::NotAbsolute),
            ("https://example.com@evil.example", LinkError::Credentials),
            ("https://", LinkError::NoHost),
            ("https://x\"onclick=alert(1)\"/", LinkError::BadHost),
            ("https://example.com:65536", LinkError::BadPort),
        ];
        for (href, reason) in cases {
            assert_eq!(
                link::Link::try_from(mirror(href, "example.com")),
                Err(reason),
                "{href:?}"
            );
        }
    }

    /// The facade's filter gives the core's answer for a link. It is the
    /// function the browser calls as `parseLink`: the wrapper between them
    /// converts the answer and makes this one call, and the check
    /// `xtask facade-wrappers` fails if the macro that writes it does more.
    ///
    /// Verifies: SEC-API-047, SEC-CLI-002
    #[test]
    fn the_filter_accepts_what_the_core_accepts() {
        let cases = [
            (
                "https://example.com:8443/x",
                "https://example.com:8443/x",
                "example.com",
            ),
            (
                "https://example.com:443/x",
                "https://example.com/x",
                "example.com",
            ),
            (
                "https://xn--bcher-kva.example",
                "https://xn--bcher-kva.example/",
                "xn--bcher-kva.example",
            ),
            (
                "https://[2001:db8::1]:8443/",
                "https://[2001:db8::1]:8443/",
                "[2001:db8::1]",
            ),
        ];
        for (raw, href, host) in cases {
            assert_eq!(
                parse_link(raw),
                LinkOutcome::Link(mirror(href, host)),
                "{raw:?}"
            );
        }
    }

    /// The facade's filter keeps as plain text everything the core's
    /// filter refuses: script and data URLs however they are written,
    /// plain `http`, relative values, credentials and bad hosts.
    ///
    /// Verifies: SEC-API-047, SEC-CLI-002
    #[test]
    fn the_filter_refuses_what_the_core_refuses() {
        let cases = [
            ("", LinkError::NotAbsolute),
            ("example.com", LinkError::NotAbsolute),
            ("//example.com", LinkError::NotAbsolute),
            ("http://example.com", LinkError::NotHttps),
            ("javascript:alert(1)", LinkError::NotHttps),
            ("JaVaScRiPt:alert(1)", LinkError::NotHttps),
            ("java\tscript:alert(1)", LinkError::NotHttps),
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
            ("https://user:pass@example.com", LinkError::Credentials),
            ("https://", LinkError::NoHost),
            ("https://x\"onclick=alert(1)\"/", LinkError::BadHost),
            ("https://example.com:65536", LinkError::BadPort),
        ];
        for (raw, reason) in cases {
            assert_eq!(parse_link(raw), LinkOutcome::PlainText(reason), "{raw:?}");
        }
    }

    /// The filter reads a URL of at most [`MAX_LINK_OCTETS`] octets of
    /// UTF-8, the core's long-text limit, and keeps one octet more as plain
    /// text unread, with the reason it gives text that is not a URL at all.
    /// The limit counts octets, not characters. The core alone accepts every
    /// URL here, the longer ones too, each with the whole link it would give.
    ///
    /// Supports: SEC-MED-077
    #[test]
    fn the_filter_reads_a_url_up_to_the_long_text_limit_and_keeps_a_longer_one_as_plain_text() {
        assert_eq!(MAX_LINK_OCTETS, 65_536);
        let start = "https://example.com/";
        let narrow = format!("{start}{}", "a".repeat(65_536 - start.len()));
        let wide = format!("{start}{}", "\u{E9}".repeat((65_536 - start.len()) / 2));
        assert_eq!((narrow.len(), wide.len()), (65_536, 65_536));
        assert_eq!(
            parse_link(&narrow),
            LinkOutcome::Link(mirror(&narrow, "example.com"))
        );
        let escaped = format!("{start}{}", "%C3%A9".repeat((65_536 - start.len()) / 2));
        assert_eq!(
            parse_link(&wide),
            LinkOutcome::Link(mirror(&escaped, "example.com"))
        );
        let longer = [
            (format!("{narrow}a"), format!("{narrow}a")),
            (format!("{wide}a"), format!("{escaped}a")),
        ];
        for (over, href) in longer {
            assert_eq!(over.len(), 65_537);
            let characters = over.chars().count();
            let alone = link::Link::parse(Untrusted::new(over.as_str()))
                .expect("the core alone accepts the longer URL");
            assert_eq!(
                (alone.href(), alone.host()),
                (href.as_str(), "example.com"),
                "{characters} characters"
            );
            assert_eq!(
                parse_link(&over),
                LinkOutcome::PlainText(LinkError::NotAbsolute),
                "{characters} characters"
            );
        }
    }

    #[test]
    fn the_declarations_are_the_literal_typescript() {
        assert_eq!(
            DECLARATIONS,
            [
                "/**\n * A link that may be shown as a link and opened.\n */\n\
                 export interface Link {\n    \
                 /**\n     * The URL to open, rewritten so that every URL parser reads the same host from it.\n     */\n    \
                 href: string;\n    \
                 /**\n     * The host to show before the link is opened.\n     */\n    \
                 host: string;\n}",
                "/**\n * Why a URL must be shown as plain text instead of a link.\n */\n\
                 export type LinkError = \"NotAbsolute\" | \"NotHttps\" | \"Credentials\" | \"NoHost\" | \"BadHost\" | \"BadPort\";",
                "/**\n * What reading a URL gave: a link, or the reason it stays plain text.\n */\n\
                 export type LinkOutcome = { Link: Link } | { PlainText: LinkError };",
            ]
        );
    }
}
