//! The listener: the one module that sees a socket, and so the one module
//! that says where a request came from (SEC-OPS-037).
//!
//! What is here is the client-address resolver. For each request the
//! listener hands the socket's peer address and the request's header fields
//! to the one path-class function in the core, with the owner's
//! trusted-proxy list and what the server knows about its own network, and
//! attaches the [`ClientContext`] that comes back to the request before the
//! pipeline runs. The sessions, the credential verifier, the audit log and
//! the limiters read that value and nothing else, so they all agree where a
//! request came from.
//!
//! - **Forwarding headers are believed only from a trusted proxy**
//!   (SEC-NET-016). The list is empty unless the owner declared a proxy.
//!   From any other peer a forwarding header changes nothing but the class:
//!   the address stays the peer's own, and the request is never local.
//! - **A gateway is nobody in particular** (SEC-NET-068). A peer that is the
//!   server's default gateway, a bridge or a configured container gateway
//!   may be hiding any client, so its class is unknown.
//!
//! [`Network::admit`] is the one place a request is prepared for the
//! pipeline. Anything else the listener must attach to a request before the
//! pipeline runs, such as what the session layer reads from the request's
//! cookies (WP-062), is attached there.
//!
//! Binding and accepting, the connection limits and timeouts, the framing
//! checks and graceful shutdown are not here yet: they need an asynchronous
//! runtime's sockets and timers and an HTTP server, which the workspace does
//! not have.

use core::net::IpAddr;

use axum::http::{HeaderMap, Request};
use gunmetal_core::client_context::ClientContext;
use gunmetal_core::http::forwarded::{
    ForwardedError, ForwardingHeaders, HostNetwork, TrustedProxy, path_class,
};
use gunmetal_core::net::IpNet;
use gunmetal_core::untrusted::Untrusted;
use gunmetal_http::problem::ApiError;

/// What the resolver knows about the network the server is on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Network {
    /// The owner's trusted proxies. Empty unless the owner declared one
    /// (SEC-NET-016).
    pub proxies: Vec<TrustedProxy>,
    /// The networks the server's own interfaces are on.
    pub interfaces: Vec<IpNet>,
    /// The default gateway and bridge addresses of those interfaces, and
    /// any container gateway the owner configured (SEC-NET-068).
    pub gateways: Vec<IpAddr>,
}

impl Network {
    /// Where a request with these header fields, from the socket peer
    /// `peer`, came from.
    ///
    /// # Errors
    ///
    /// A [`ForwardedError`] when `peer` is a trusted proxy and its
    /// forwarding chain cannot be read (SEC-NET-018).
    pub fn resolve(
        &self,
        peer: IpAddr,
        headers: &HeaderMap,
    ) -> Result<ClientContext, ForwardedError> {
        let mut forwarding = ForwardingHeaders::default();
        for (name, value) in headers {
            forwarding.add(
                Untrusted::new(name.as_str().as_bytes()),
                Untrusted::new(value.as_bytes()),
            );
        }
        let host = HostNetwork {
            interfaces: &self.interfaces,
            gateways: &self.gateways,
        };
        path_class(peer, &forwarding, &self.proxies, &host)
    }

    /// Prepares a request from the socket peer `peer` for the pipeline: it
    /// attaches where the request came from.
    ///
    /// # Errors
    ///
    /// `bad_forwarding_header`, to be answered 400, when a trusted proxy
    /// sent a forwarding chain that cannot be read. Nothing is attached.
    pub fn admit<B>(&self, peer: IpAddr, request: &mut Request<B>) -> Result<(), ApiError> {
        let context = self
            .resolve(peer, request.headers())
            .map_err(|error| ApiError::from_core(&error))?;
        request.extensions_mut().insert(context);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use core::net::{IpAddr, Ipv4Addr};

    use axum::http::{HeaderMap, HeaderValue, Request};
    use gunmetal_core::client_context::{ClientContext, PathClass};
    use gunmetal_core::http::forwarded::{ForwardedError, ProxyKind, TrustedProxy};
    use gunmetal_core::net::IpNet;
    use gunmetal_core::problem::ProblemCode;
    use gunmetal_core::untrusted::Untrusted;
    use gunmetal_http::problem::ApiError;

    use super::Network;

    const LOOPBACK: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
    const HOME: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20));
    const INTERNET: IpAddr = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7));
    /// A reverse proxy the owner declared, reachable from the internet.
    const PROXY: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2));
    /// A private overlay's proxy the owner declared.
    const OVERLAY: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 3));
    /// The container bridge in front of the server.
    const GATEWAY: IpAddr = IpAddr::V4(Ipv4Addr::new(172, 17, 0, 1));
    /// The client a forwarding header names.
    const CLIENT: IpAddr = IpAddr::V4(Ipv4Addr::new(198, 51, 100, 9));

    /// What a request's context holds.
    type Seen = (IpAddr, PathClass, bool);

    fn net(text: &str) -> IpNet {
        IpNet::parse(Untrusted::new(text)).expect("a network")
    }

    /// A server behind a container bridge whose owner declared two proxies.
    fn network() -> Network {
        Network {
            proxies: vec![
                TrustedProxy {
                    network: net("10.0.0.2/32"),
                    kind: ProxyKind::Public,
                },
                TrustedProxy {
                    network: net("10.0.0.3/32"),
                    kind: ProxyKind::PrivateOverlay,
                },
            ],
            interfaces: vec![net("192.168.1.0/24")],
            gateways: vec![GATEWAY],
        }
    }

    fn headers(fields: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in fields {
            headers.append(*name, HeaderValue::from_static(value));
        }
        headers
    }

    fn seen(context: &ClientContext) -> Seen {
        (context.addr(), context.class(), context.via_proxy())
    }

    /// Where `network` says a request from `peer` with `fields` came from.
    fn resolved(
        network: &Network,
        peer: IpAddr,
        fields: &[(&'static str, &'static str)],
    ) -> Result<Seen, ForwardedError> {
        network
            .resolve(peer, &headers(fields))
            .map(|context| seen(&context))
    }

    /// Verifies: SEC-NET-068, SEC-OPS-037
    #[test]
    fn a_direct_peer_is_classified_by_its_own_address() {
        // Headers that say nothing about forwarding change nothing.
        let plain = [("host", "music.example.com"), ("accept", "*/*")];
        assert_eq!(
            [LOOPBACK, HOME, INTERNET, GATEWAY].map(|peer| resolved(&network(), peer, &plain)),
            [
                Ok((LOOPBACK, PathClass::Loopback, false)),
                Ok((HOME, PathClass::Home, false)),
                Ok((INTERNET, PathClass::Internet, false)),
                // The bridge address is private space, and still not home.
                Ok((GATEWAY, PathClass::Unknown, false)),
            ]
        );
        // With no gateway known, the same peer is an ordinary private one.
        assert_eq!(
            resolved(&Network::default(), GATEWAY, &plain),
            Ok((GATEWAY, PathClass::Home, false))
        );
    }

    /// Every forwarding header, from each kind of peer that is not a
    /// trusted proxy: the address stays the socket peer's, and the request
    /// is never loopback or home.
    ///
    /// Verifies: SEC-NET-016
    #[test]
    fn a_forwarding_header_from_an_untrusted_peer_is_never_believed() {
        let spoofs = [
            ("forwarded", "for=198.51.100.9"),
            ("x-forwarded-for", "198.51.100.9"),
            ("x-forwarded-host", "music.example.com"),
            ("x-forwarded-proto", "https"),
            ("x-real-ip", "198.51.100.9"),
            ("x-client-ip", "198.51.100.9"),
            ("true-client-ip", "198.51.100.9"),
            ("cf-connecting-ip", "198.51.100.9"),
            // One that no proxy could have written is ignored like the rest.
            ("x-forwarded-for", "not an address"),
        ];
        for spoof in spoofs {
            assert_eq!(
                [LOOPBACK, HOME, INTERNET].map(|peer| resolved(&network(), peer, &[spoof])),
                [
                    Ok((LOOPBACK, PathClass::Internet, false)),
                    Ok((HOME, PathClass::Internet, false)),
                    Ok((INTERNET, PathClass::Internet, false)),
                ],
                "{spoof:?}"
            );
        }
    }

    /// Verifies: SEC-NET-016
    #[test]
    fn nothing_is_trusted_until_the_owner_declares_a_proxy() {
        assert_eq!(
            Network::default(),
            Network {
                proxies: Vec::new(),
                interfaces: Vec::new(),
                gateways: Vec::new(),
            }
        );
        // The address that is a declared proxy in the other tests is an
        // ordinary peer here, and what it forwards is ignored.
        assert_eq!(
            [
                resolved(&Network::default(), PROXY, &[]),
                resolved(
                    &Network::default(),
                    PROXY,
                    &[("x-forwarded-for", "198.51.100.9")]
                ),
                resolved(
                    &Network::default(),
                    PROXY,
                    &[("forwarded", "for=198.51.100.9")]
                ),
            ],
            [
                Ok((PROXY, PathClass::Home, false)),
                Ok((PROXY, PathClass::Internet, false)),
                Ok((PROXY, PathClass::Internet, false)),
            ]
        );
    }

    /// Verifies: SEC-NET-016
    #[test]
    fn a_trusted_proxy_s_chain_gives_the_client_address() {
        let network = network();
        assert_eq!(
            [
                resolved(&network, PROXY, &[("x-forwarded-for", "198.51.100.9")]),
                resolved(&network, PROXY, &[("forwarded", "for=198.51.100.9")]),
                // What the client wrote to the left of the proxy's own
                // entry is not believed.
                resolved(
                    &network,
                    PROXY,
                    &[("x-forwarded-for", "127.0.0.1, 198.51.100.9")]
                ),
                // Two field lines are one chain, in the order they arrived.
                resolved(
                    &network,
                    PROXY,
                    &[
                        ("x-forwarded-for", "127.0.0.1"),
                        ("x-forwarded-for", "198.51.100.9")
                    ]
                ),
                // A header that carries no chain leaves the proxy's own
                // address standing.
                resolved(&network, PROXY, &[("x-real-ip", "198.51.100.9")]),
                resolved(&network, PROXY, &[]),
                // Only a proxy the owner declared a private overlay makes a
                // request a home one.
                resolved(&network, OVERLAY, &[("x-forwarded-for", "198.51.100.9")]),
            ],
            [
                Ok((CLIENT, PathClass::Internet, true)),
                Ok((CLIENT, PathClass::Internet, true)),
                Ok((CLIENT, PathClass::Internet, true)),
                Ok((CLIENT, PathClass::Internet, true)),
                Ok((PROXY, PathClass::Internet, true)),
                Ok((PROXY, PathClass::Internet, true)),
                Ok((CLIENT, PathClass::Home, true)),
            ]
        );
        assert_eq!(
            resolved(
                &network,
                PROXY,
                &[
                    ("forwarded", "for=198.51.100.9"),
                    ("x-forwarded-for", "198.51.100.9")
                ]
            ),
            Err(ForwardedError::BothChains)
        );
    }

    /// Verifies: SEC-OPS-037
    #[test]
    fn admitting_a_request_attaches_where_it_came_from() {
        let mut request = Request::new(());
        *request.headers_mut() = headers(&[("x-forwarded-for", "198.51.100.9")]);
        assert_eq!(network().admit(PROXY, &mut request), Ok(()));
        assert_eq!(
            request.extensions().get::<ClientContext>().map(seen),
            Some((CLIENT, PathClass::Internet, true))
        );
        // A chain that cannot be read is refused, and nothing is attached.
        let mut request = Request::new(());
        *request.headers_mut() = headers(&[
            ("forwarded", "for=198.51.100.9"),
            ("x-forwarded-for", "198.51.100.9"),
        ]);
        assert_eq!(
            network().admit(PROXY, &mut request),
            Err(ApiError::new(ProblemCode::BadForwardingHeader))
        );
        assert_eq!(request.extensions().get::<ClientContext>().map(seen), None);
    }
}
