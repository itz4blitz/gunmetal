//! The harness for the forwarding-header parser and the path-class
//! function in `gunmetal_core::http::forwarded` (SEC-NET-016,
//! SEC-NET-018).
//!
//! An input is a request's header section: one field per line, its name
//! before the first `:` and its value after. Header names come only from
//! the input, so this file names no forwarding header itself; only the
//! listener and the core's parser may (SEC-OPS-037).

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use gunmetal_core::client_context::{ClientContext, PathClass};
use gunmetal_core::http::forwarded::{
    ForwardedError, ForwardingHeaders, HostNetwork, ProxyKind, TrustedProxy, path_class,
};
use gunmetal_core::net::IpNet;
use gunmetal_core::untrusted::Untrusted;

/// A resolved request as a value the replay tests can write out: the
/// client address, its path class and whether a trusted proxy carried it.
pub type Resolved = Result<(IpAddr, PathClass, bool), ForwardedError>;

/// What the path-class function reported for one header section, from
/// four peers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outcome {
    /// From `127.0.0.1`, with no trusted proxy.
    pub loopback: Resolved,
    /// From `192.168.1.20`, with no trusted proxy.
    pub lan: Resolved,
    /// From `10.0.0.1`, inside the public proxy entry `10.0.0.0/8`.
    pub public_proxy: Resolved,
    /// From `fd00::1`, inside the private overlay entry `fd00::/8`.
    pub overlay_proxy: Resolved,
}

/// The trusted-proxy list the two proxied peers resolve against.
fn proxies() -> [TrustedProxy; 2] {
    let network = |text| IpNet::parse(Untrusted::new(text)).expect("a valid network");
    [
        TrustedProxy {
            network: network("10.0.0.0/8"),
            kind: ProxyKind::Public,
        },
        TrustedProxy {
            network: network("fd00::/8"),
            kind: ProxyKind::PrivateOverlay,
        },
    ]
}

fn resolved(answer: Result<ClientContext, ForwardedError>) -> Resolved {
    answer.map(|context| (context.addr(), context.class(), context.via_proxy()))
}

/// Gathers the fields of `data` with [`ForwardingHeaders::add`] and
/// resolves them with [`path_class`] from four peers.
///
/// # Panics
///
/// Panics when a peer with no trusted proxy is refused, gets another
/// address than its own or a proxied context, or keeps a home class
/// despite forwarding headers that the other peer's class shows it saw;
/// or when the two proxied peers, which read the same chain against the
/// same list, disagree about whether it can be read.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let mut headers = ForwardingHeaders::default();
    for line in data.split(|&octet| octet == b'\n') {
        let colon = line.iter().position(|&octet| octet == b':');
        let name = colon.map_or(line, |colon| line.get(..colon).unwrap_or(line));
        let value = colon.map_or(&[][..], |colon| {
            line.get(colon.saturating_add(1)..).unwrap_or_default()
        });
        headers.add(Untrusted::new(name), Untrusted::new(value));
    }
    let host = HostNetwork::default();
    let loopback = IpAddr::V4(Ipv4Addr::LOCALHOST);
    let lan = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20));
    let direct = |peer: IpAddr| {
        let answer = path_class(peer, &headers, &[], &host);
        assert!(
            answer.is_ok_and(|context| context.addr() == peer && !context.via_proxy()),
            "{peer}: {answer:?}"
        );
        resolved(answer)
    };
    let (loopback, lan) = (direct(loopback), direct(lan));
    let forwarded = loopback.is_ok_and(|(_, class, _)| class == PathClass::Internet);
    assert_eq!(
        lan.map(|(_, class, _)| class == PathClass::Internet),
        Ok(forwarded),
        "the loopback and LAN peers disagree about the headers"
    );
    let proxies = proxies();
    let public_proxy = resolved(path_class(
        IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
        &headers,
        &proxies,
        &host,
    ));
    let overlay_proxy = resolved(path_class(
        IpAddr::V6(Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 1)),
        &headers,
        &proxies,
        &host,
    ));
    assert_eq!(public_proxy.err(), overlay_proxy.err(), "{data:?}");
    for &(_, _, via_proxy) in public_proxy.iter().chain(overlay_proxy.iter()) {
        assert!(via_proxy, "{data:?}");
    }
    Outcome {
        loopback,
        lan,
        public_proxy,
        overlay_proxy,
    }
}
