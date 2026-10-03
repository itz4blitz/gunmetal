//! The harness for the network parser and the address classifier in
//! `gunmetal_core::net` (SEC-NET-025).

use std::net::IpAddr;

use gunmetal_core::net::{self, AddrClass, IpNet, NetError};
use gunmetal_core::untrusted::Untrusted;

/// What the network parser and the classifier reported for one input, read
/// as UTF-8 with each invalid sequence replaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outcome {
    /// [`IpNet::parse`]: the network address and prefix length, or why the
    /// text is not a network.
    pub network: Result<(IpAddr, u8), NetError>,
    /// [`net::classify`] of the text before the first `/`, when that is an
    /// IP address.
    pub class: Option<AddrClass>,
}

/// Feeds `data` to [`IpNet::parse`], and the address before its first `/`
/// to [`net::classify`].
///
/// # Panics
///
/// Panics when an accepted network does not read back unchanged from its
/// written form, or contains its own address exactly when that address is
/// IPv4-mapped IPv6, which [`IpNet::contains`] reads as IPv4; or when an
/// IPv4 address and its IPv4-mapped IPv6 form are classified differently.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let text = String::from_utf8_lossy(data);
    let parsed = IpNet::parse(Untrusted::new(&text));
    if let Ok(network) = parsed {
        let written = format!("{}/{}", network.addr(), network.prefix());
        assert!(
            network.contains(network.addr()) == (network.addr().to_canonical() == network.addr())
                && IpNet::parse(Untrusted::new(&written)) == Ok(network),
            "{text:?} read as {network:?}"
        );
    }
    let address = text
        .split('/')
        .next()
        .unwrap_or_default()
        .parse::<IpAddr>()
        .ok();
    let class = address.map(net::classify);
    if let Some(ip) = address {
        let (v4, v6) = match ip {
            IpAddr::V4(v4) => (Some(v4), v4.to_ipv6_mapped()),
            IpAddr::V6(v6) => (v6.to_ipv4_mapped(), v6),
        };
        assert!(
            v4.is_none_or(|v4| net::classify(IpAddr::V4(v4)) == net::classify(IpAddr::V6(v6))),
            "{ip} and {v6} are classified differently"
        );
    }
    Outcome {
        network: parsed.map(|network| (network.addr(), network.prefix())),
        class,
    }
}
