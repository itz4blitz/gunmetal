//! Where a request goes: a scheme, a host and a port.
//!
//! A grant names its destinations exactly, and a request is compared with
//! them as whole values, so a host has one spelling: a name in lower case
//! with no trailing dot, or an address in its canonical form. A text that
//! some resolver would read as an address, such as `127.1` or `0x7f.1`, is
//! not a name.

use core::net::IpAddr;

/// How a request is carried.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    /// HTTP over TLS.
    Https,
    /// Plain HTTP, which only a LAN destination an admin granted may use.
    Http,
}

/// A DNS host name: labels of lower-case letters, digits and hyphens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name(String);

impl Name {
    /// The name, in lower case and without a trailing dot.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The host of a destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Host {
    /// A name, which is resolved before the request connects.
    Name(Name),
    /// An address written out. An IPv4-mapped IPv6 address is held as the
    /// IPv4 address it maps.
    Address(IpAddr),
}

/// Why a text is not a host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostError {
    /// It is longer than 253 octets.
    TooLong,
    /// One of its labels is empty, is longer than 63 octets, starts or ends
    /// with a hyphen, or holds something other than an ASCII letter, a
    /// digit or a hyphen.
    BadLabel,
    /// Its last label does not start with a letter, so a resolver could
    /// read the whole text as an address.
    NumericEnd,
}

impl Host {
    /// Reads a host: an IP address, or a DNS name in any case.
    ///
    /// # Errors
    ///
    /// A [`HostError`] for anything else, which includes the empty text, a
    /// name with a trailing dot, an address in brackets and a host followed
    /// by a port.
    pub fn parse(text: &str) -> Result<Self, HostError> {
        Ok(Self::Name(Name(text.to_owned())))
    }
}

/// A scheme, a host and a port: what a grant names and what a request is
/// checked against. It has no path, so nothing that is recorded about a
/// request can hold one (SEC-PRV-008).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Destination {
    /// How the request is carried.
    pub scheme: Scheme,
    /// The host it goes to.
    pub host: Host,
    /// The port it connects to.
    pub port: u16,
}

#[cfg(test)]
mod tests {
    use super::{Host, HostError, Name};
    use core::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use proptest::prelude::*;

    /// The host a name reads as.
    fn named(name: &str) -> Host {
        Host::Name(Name(name.to_owned()))
    }

    #[test]
    fn reads_a_name_in_any_case_as_its_lower_case_spelling() {
        let cases = [
            ("gunmetal.tv", "gunmetal.tv"),
            ("Gunmetal.TV", "gunmetal.tv"),
            (
                "ACME-v02.API.LetsEncrypt.org",
                "acme-v02.api.letsencrypt.org",
            ),
            ("localhost", "localhost"),
            ("homeassistant.local", "homeassistant.local"),
            ("a1.b2.c3", "a1.b2.c3"),
            ("9to5.example", "9to5.example"),
            ("xn--bcher-kva.example", "xn--bcher-kva.example"),
        ];
        for (text, name) in cases {
            assert_eq!(Host::parse(text), Ok(named(name)), "{text}");
        }
    }

    #[test]
    fn a_name_gives_its_text_back() {
        assert_eq!(Name("gunmetal.tv".to_owned()).as_str(), "gunmetal.tv");
    }

    #[test]
    fn a_name_may_be_253_octets_long_and_a_label_63() {
        let label = "a".repeat(63);
        let longest = format!("{label}.{label}.{label}.{}", "a".repeat(61));
        assert_eq!(longest.len(), 253);
        assert_eq!(Host::parse(&longest), Ok(named(&longest)));
        let too_long = format!("{label}.{label}.{label}.{}", "a".repeat(62));
        assert_eq!(Host::parse(&too_long), Err(HostError::TooLong));
        assert_eq!(Host::parse(&"a".repeat(300)), Err(HostError::TooLong));
        let long_label = format!("{}.example", "a".repeat(64));
        assert_eq!(Host::parse(&long_label), Err(HostError::BadLabel));
    }

    #[test]
    fn refuses_what_is_not_a_host_name() {
        let cases = [
            ("", HostError::BadLabel),
            (".", HostError::BadLabel),
            ("example.com.", HostError::BadLabel),
            (".example.com", HostError::BadLabel),
            ("a..b", HostError::BadLabel),
            ("-a.example", HostError::BadLabel),
            ("a-.example", HostError::BadLabel),
            ("example.-", HostError::BadLabel),
            ("exa_mple.com", HostError::BadLabel),
            ("exa mple.com", HostError::BadLabel),
            (" example.com", HostError::BadLabel),
            ("example.com:443", HostError::BadLabel),
            ("user@example.com", HostError::BadLabel),
            ("example.com/path", HostError::BadLabel),
            ("b\u{fc}cher.example", HostError::BadLabel),
            ("[::1]", HostError::BadLabel),
            ("fe80::1%eth0", HostError::BadLabel),
        ];
        for (text, error) in cases {
            assert_eq!(Host::parse(text), Err(error), "{text:?}");
        }
    }

    /// The forms of an address that some resolvers accept and the standard
    /// library does not: short, hexadecimal, octal and one number. None of
    /// them becomes a name that such a resolver would then read as
    /// loopback.
    #[test]
    fn refuses_a_text_that_a_resolver_could_read_as_an_address() {
        for text in [
            "127.1",
            "0x7f.1",
            "0x7f.0.0.1",
            "010.0.0.1",
            "256.1.1.1",
            "2130706433",
            "example.123",
            "example.0x7f",
        ] {
            assert_eq!(Host::parse(text), Err(HostError::NumericEnd), "{text}");
        }
    }

    #[test]
    fn reads_an_address_in_its_canonical_form() {
        let ten = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let cases = [
            ("192.0.2.7", IpAddr::V4(Ipv4Addr::new(192, 0, 2, 7))),
            ("127.0.0.1", IpAddr::V4(Ipv4Addr::LOCALHOST)),
            ("::1", IpAddr::V6(Ipv6Addr::LOCALHOST)),
            (
                "2001:DB8:0:0:0:0:0:1",
                IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)),
            ),
            ("::ffff:10.0.0.1", ten),
            ("::FFFF:a00:1", ten),
        ];
        for (text, address) in cases {
            assert_eq!(Host::parse(text), Ok(Host::Address(address)), "{text}");
        }
    }

    proptest! {
        /// A host has one spelling, however its text is cased.
        #[test]
        fn the_case_of_a_text_never_changes_what_it_reads_as(
            text in prop_oneof![
                "[a-zA-Z0-9-]{1,8}(\\.[a-zA-Z0-9-]{1,8}){0,3}",
                "[a-fA-F0-9:.]{0,24}",
                "[a-zA-Z0-9.:_-]{0,40}",
            ],
        ) {
            prop_assert_eq!(Host::parse(&text), Host::parse(&text.to_ascii_lowercase()));
        }
    }
}
