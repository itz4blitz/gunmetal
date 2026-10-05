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
    use proptest::prelude::*;

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

    /// The labels of a name before its last, each with its dot.
    const LABELS: &str = "([a-z0-9-]{1,8}\\.){0,2}";

    /// A last label that starts with a letter other than `l`, so a name
    /// that ends with it is neither an address nor a `localhost` name.
    const LAST_LABEL: &str = "[a-km-z][a-z0-9-]{0,7}";

    /// A whole name of that kind, in the form hosts are compared in.
    const NAME: &str = "([a-z0-9-]{1,8}\\.){0,2}[a-km-z][a-z0-9-]{0,7}";

    /// How a client may write a host: which of its characters are in upper
    /// case, whether a dot follows it, and whether a port does.
    #[derive(Debug, Clone, Copy)]
    struct Spelling {
        upper: [bool; 32],
        dot: bool,
        with_port: bool,
        port: u16,
    }

    /// Any spelling. The port is drawn whether or not it is written, and
    /// the helpers below choose by position, never by a branch, so every
    /// line of these tests runs on every case.
    fn any_spelling() -> impl Strategy<Value = Spelling> {
        (
            any::<[bool; 32]>(),
            any::<bool>(),
            any::<bool>(),
            any::<u16>(),
        )
            .prop_map(|(upper, dot, with_port, port)| Spelling {
                upper,
                dot,
                with_port,
                port,
            })
    }

    /// What follows a host in this spelling: its port, or nothing.
    fn port_suffix(spelling: Spelling) -> String {
        let suffixes = [String::new(), format!(":{}", spelling.port)];
        suffixes[usize::from(spelling.with_port)].clone()
    }

    /// `host`, which is in the form hosts are compared in and at most 32
    /// characters long, as this spelling writes it.
    fn written(host: &str, spelling: Spelling) -> String {
        let cased: String = host
            .chars()
            .zip(spelling.upper)
            .map(|(character, upper)| {
                [character, character.to_ascii_uppercase()][usize::from(upper)]
            })
            .collect();
        let dot = ["", "."][usize::from(spelling.dot)];
        format!("{cased}{dot}{}", port_suffix(spelling))
    }

    proptest! {
        /// A name is the same host in any case, with or without a trailing
        /// dot and with or without a port, and no other name is taken for
        /// it. `localhost` and the names under it are told apart from
        /// other names however they are written.
        ///
        /// Verifies: SEC-NET-014
        #[test]
        fn a_name_is_one_host_however_it_is_written(
            labels in LABELS,
            ending in LAST_LABEL,
            local in any::<bool>(),
            other in NAME,
            spelling in any_spelling(),
        ) {
            let (ending, kind) = [
                (ending.as_str(), HostKind::Name),
                ("localhost", HostKind::Localhost),
            ][usize::from(local)];
            let name = format!("{labels}{ending}");
            let expected = host(&name, kind);
            let authority = written(&name, spelling);
            prop_assert_eq!(Host::parse(&authority), Some(expected.clone()));
            let allowed = HostAllowList::new(&[name.as_str()]).unwrap();
            prop_assert_eq!(allowed.admit(&authority), Some(expected));
            // Another name is admitted only when it is the configured one.
            prop_assert_eq!(
                allowed.admit(&written(&other, spelling)),
                Some(host(&other, HostKind::Name)).filter(|_| other == name)
            );
        }

        /// An IPv4 address is the same host with or without a trailing dot
        /// or a port.
        ///
        /// Verifies: SEC-NET-014
        #[test]
        fn an_ipv4_address_is_one_host_however_it_is_written(
            octets in any::<[u8; 4]>(),
            spelling in any_spelling(),
        ) {
            let address = octets.map(|octet| octet.to_string()).join(".");
            let expected = host(&address, HostKind::Address);
            let authority = written(&address, spelling);
            prop_assert_eq!(Host::parse(&authority), Some(expected.clone()));
            prop_assert_eq!(Host::configured(&address), Some(expected.clone()));
            let allowed = HostAllowList::new(&[address.as_str()]).unwrap();
            prop_assert_eq!(allowed.admit(&authority), Some(expected));
        }

        /// An IPv6 address is the same host in upper or lower case, with or
        /// without leading zeros and with its zeros written out or as `::`,
        /// and it is a `Host` value only in brackets. The form hosts are
        /// compared in is written out here for two shapes: no zero segment,
        /// which leaves eight segments in lower-case hex without leading
        /// zeros, and six zero segments in the middle, which become `::`.
        ///
        /// Verifies: SEC-NET-014
        #[test]
        fn an_ipv6_literal_is_one_host_however_it_is_written(
            segments in proptest::array::uniform8(1_u16..),
            first in 1_u16..,
            last in 1_u16..,
            padded in any::<bool>(),
            spelling in any_spelling(),
        ) {
            let whole = segments.map(|segment| format!("{segment:x}")).join(":");
            let whole_padded = segments.map(|segment| format!("{segment:04X}")).join(":");
            let short = format!("{first:x}::{last:x}");
            let short_padded = format!("{first:04X}:0:0000:0:0:0000:0:{last:04x}");
            let port = port_suffix(spelling);
            for (compared, spellings) in [
                (&whole, [&whole, &whole_padded]),
                (&short, [&short, &short_padded]),
            ] {
                let expected = host(compared, HostKind::Address);
                let address = spellings[usize::from(padded)];
                let authority = format!("[{address}]{port}");
                prop_assert_eq!(Host::parse(&authority), Some(expected.clone()));
                prop_assert_eq!(Host::configured(address), Some(expected.clone()));
                let allowed = HostAllowList::new(&[address.as_str()]).unwrap();
                prop_assert_eq!(allowed.admit(&authority), Some(expected));
                // Without brackets its colons would be read as a port, and
                // an address in brackets takes no trailing dot.
                prop_assert_eq!(Host::parse(address), None);
                prop_assert_eq!(Host::parse(&format!("[{address}].{port}")), None);
            }
        }

        /// Only a port number from 0 to 65535, with or without leading
        /// zeros, may follow a host, and a host holds only letters, digits,
        /// hyphens and dots.
        ///
        /// Verifies: SEC-NET-014
        #[test]
        fn nothing_but_a_port_number_follows_a_host(
            name in NAME,
            port in any::<u16>(),
            beyond in 65_536_u32..,
            stray in "[^A-Za-z0-9.:-]",
        ) {
            prop_assert_eq!(
                Host::parse(&format!("{name}:{port:05}")),
                Some(host(&name, HostKind::Name))
            );
            let refused = [
                format!("{name}:"),
                format!("{name}:{beyond}"),
                format!("{name}:+{port}"),
                format!("{name}:-{port}"),
                format!("{name}:{port}:{port}"),
                format!("{name}:{port}."),
                format!("[::1]:{beyond}"),
                format!("{name}{stray}"),
                format!("{stray}{name}"),
                format!("{name}{stray}{name}"),
            ];
            for authority in refused {
                prop_assert_eq!(Host::parse(&authority), None);
            }
        }
    }
}
