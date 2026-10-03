//! The `Host` allow-list (SEC-API-007, SEC-IAM-010, SEC-NET-014, SEC-TM-009).
//!
//! A request is answered only when its `Host` names one of the hosts the
//! server was configured with: its public hostnames, its interface
//! addresses, `localhost` and its mDNS name. Everything else gets 421,
//! which is what stops a DNS-rebinding page from reaching the server under
//! the attacker's own name. Names compare without case, without a trailing
//! dot and without the port; IPv6 addresses compare in their canonical
//! form.

use core::net::{IpAddr, Ipv6Addr};

/// A host, normalised for comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Host {
    text: String,
    kind: HostKind,
}

/// What kind of host a [`Host`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostKind {
    /// A DNS name other than `localhost`.
    Name,
    /// `localhost` or a name under it.
    Localhost,
    /// An IP address literal.
    Address,
}

impl Host {
    /// Parses the value of a `Host` header or a request authority: a name or
    /// address, then an optional port. Returns `None` for anything else.
    #[must_use]
    pub fn parse(authority: &str) -> Option<Self> {
        if let Some(rest) = authority.strip_prefix('[') {
            let (inner, after) = rest.split_once(']')?;
            let address = inner.parse::<Ipv6Addr>().ok()?;
            return port_ok(after).then(|| Self {
                text: address.to_string(),
                kind: HostKind::Address,
            });
        }
        let (host, after) = authority
            .find(':')
            .map_or((authority, ""), |at| authority.split_at(at));
        if port_ok(after) {
            Self::name(host)
        } else {
            None
        }
    }

    /// Normalises a configured host: a name or an address, without a port.
    #[must_use]
    pub fn configured(host: &str) -> Option<Self> {
        match host.parse::<IpAddr>() {
            Ok(address) => Some(Self {
                text: address.to_string(),
                kind: HostKind::Address,
            }),
            Err(_) => Self::name(host),
        }
    }

    /// The host as compared.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Which kind of host this is.
    #[must_use]
    pub const fn kind(&self) -> HostKind {
        self.kind
    }

    fn name(host: &str) -> Option<Self> {
        let host = host.strip_suffix('.').unwrap_or(host);
        let shaped = !host.is_empty()
            && host
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.');
        if !shaped {
            return None;
        }
        let text = host.to_ascii_lowercase();
        let kind = if text.parse::<IpAddr>().is_ok() {
            HostKind::Address
        } else if text == "localhost" || text.ends_with(".localhost") {
            HostKind::Localhost
        } else {
            HostKind::Name
        };
        Some(Self { text, kind })
    }
}

/// Whether what follows a host is nothing, or a colon and a port number.
fn port_ok(after: &str) -> bool {
    match after.strip_prefix(':') {
        None => after.is_empty(),
        Some(port) => !port.starts_with('+') && port.parse::<u16>().is_ok(),
    }
}

/// The hosts the server answers to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostAllowList {
    hosts: Vec<Host>,
}

impl HostAllowList {
    /// Builds the list from configured hosts.
    ///
    /// # Errors
    ///
    /// The position of the first entry that is not a host name or address.
    pub fn new(hosts: &[&str]) -> Result<Self, usize> {
        hosts
            .iter()
            .enumerate()
            .map(|(at, host)| Host::configured(host).ok_or(at))
            .collect::<Result<Vec<_>, _>>()
            .map(|hosts| Self { hosts })
    }

    /// The host a request named, if the server answers to it.
    #[must_use]
    pub fn admit(&self, authority: &str) -> Option<Host> {
        Host::parse(authority).filter(|host| self.hosts.contains(host))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host(text: &str, kind: HostKind) -> Host {
        Host {
            text: text.to_owned(),
            kind,
        }
    }

    #[test]
    fn parses_names_addresses_and_ports() {
        let cases = [
            (
                "music.example.com",
                Some(host("music.example.com", HostKind::Name)),
            ),
            (
                "Music.Example.COM.",
                Some(host("music.example.com", HostKind::Name)),
            ),
            (
                "music.example.com:8443",
                Some(host("music.example.com", HostKind::Name)),
            ),
            (
                "music.example.com:65535",
                Some(host("music.example.com", HostKind::Name)),
            ),
            ("localhost", Some(host("localhost", HostKind::Localhost))),
            (
                "LOCALHOST:4533",
                Some(host("localhost", HostKind::Localhost)),
            ),
            (
                "gm.localhost",
                Some(host("gm.localhost", HostKind::Localhost)),
            ),
            ("notlocalhost", Some(host("notlocalhost", HostKind::Name))),
            (
                "192.168.1.20",
                Some(host("192.168.1.20", HostKind::Address)),
            ),
            (
                "192.168.1.20:4533",
                Some(host("192.168.1.20", HostKind::Address)),
            ),
            ("[::1]", Some(host("::1", HostKind::Address))),
            ("[0:0::1]:4533", Some(host("::1", HostKind::Address))),
            ("[FE80::A]", Some(host("fe80::a", HostKind::Address))),
        ];
        for (authority, expected) in cases {
            assert_eq!(Host::parse(authority), expected);
        }
    }

    #[test]
    fn refuses_what_is_not_a_host() {
        for authority in [
            "",
            ".",
            ":4533",
            "music.example.com:",
            "music.example.com:+1",
            "music.example.com:65536",
            "music.example.com:x",
            "music.example.com:1:2",
            "music_example.com",
            "music.example.com/evil",
            "user@music.example.com",
            "bücher.example",
            "::1",
            "[::1",
            "[::1]x",
            "[::1]:",
            "[nonsense]",
            "[::1]:65536",
        ] {
            assert_eq!(Host::parse(authority), None);
        }
    }

    #[test]
    fn configured_hosts_take_bare_addresses() {
        assert_eq!(
            Host::configured("::1"),
            Some(host("::1", HostKind::Address))
        );
        assert_eq!(
            Host::configured("10.0.0.2"),
            Some(host("10.0.0.2", HostKind::Address))
        );
        assert_eq!(
            Host::configured("Gunmetal.Local"),
            Some(host("gunmetal.local", HostKind::Name))
        );
        assert_eq!(Host::configured("bad host"), None);
    }

    #[test]
    fn exposes_its_text_and_kind() {
        let parsed = Host::parse("Music.Example.com:443");
        assert_eq!(
            parsed.as_ref().map(|h| (h.as_str(), h.kind())),
            Some(("music.example.com", HostKind::Name))
        );
    }

    /// Verifies: SEC-API-007, SEC-IAM-010, SEC-NET-014, SEC-TM-009
    #[test]
    fn admits_only_configured_hosts() {
        let list =
            HostAllowList::new(&["music.example.com", "localhost", "192.168.1.20", "::1"]).unwrap();
        let admitted: Vec<(&str, bool)> = [
            "music.example.com",
            "MUSIC.example.com.:443",
            "localhost:4533",
            "192.168.1.20:4533",
            "[::1]:4533",
            "attacker.example",
            "music.example.com.attacker.example",
            "sub.music.example.com",
            "127.0.0.1",
            "[::2]",
            "",
        ]
        .into_iter()
        .map(|authority| (authority, list.admit(authority).is_some()))
        .collect();
        assert_eq!(
            admitted,
            [
                ("music.example.com", true),
                ("MUSIC.example.com.:443", true),
                ("localhost:4533", true),
                ("192.168.1.20:4533", true),
                ("[::1]:4533", true),
                ("attacker.example", false),
                ("music.example.com.attacker.example", false),
                ("sub.music.example.com", false),
                ("127.0.0.1", false),
                ("[::2]", false),
                ("", false),
            ]
        );
    }

    #[test]
    fn names_the_first_bad_configured_host() {
        assert_eq!(HostAllowList::new(&["localhost", "a b", "c d"]), Err(1));
        assert_eq!(
            HostAllowList::new(&[]),
            Ok(HostAllowList { hosts: Vec::new() })
        );
    }
}
