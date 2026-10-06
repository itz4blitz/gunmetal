//! What the limiter's and the verifier's tests share: client contexts, made
//! the only way code outside the core can make one, by asking the core's
//! path-class function about a peer.

use core::net::IpAddr;

use gunmetal_core::client_context::ClientContext;
use gunmetal_core::http::forwarded::{ForwardingHeaders, HostNetwork, path_class};

/// What the listener resolves for a direct peer at `addr` that sent no
/// forwarding headers, when the server's own gateways are `gateways`.
fn resolve(addr: &str, gateways: &[IpAddr]) -> ClientContext {
    let peer: IpAddr = addr.parse().expect("an address");
    let host = HostNetwork {
        interfaces: &[],
        gateways,
    };
    path_class(peer, &ForwardingHeaders::default(), &[], &host).expect("a direct peer resolves")
}

/// The context of a direct peer at `addr`: loopback, the home network or
/// the internet, as the address says.
pub fn source(addr: &str) -> ClientContext {
    resolve(addr, &[])
}

/// The context of a peer at `addr` when that address is the server's own
/// gateway, behind which any client may hide: the unknown class.
pub fn gateway(addr: &str) -> ClientContext {
    let peer: IpAddr = addr.parse().expect("an address");
    resolve(addr, &[peer])
}
