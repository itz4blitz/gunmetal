//! Where a request came from: the client address and its path class.
//!
//! Every request is classified once, at the edge of the server, by the one
//! path-class function in the core (WP-023), which takes the socket peer,
//! the forwarding headers, the trusted-proxy list and the server's own
//! interfaces. Its result is a [`ClientContext`]. The sessions, the
//! credential verifier, the audit log and the rate limiters take this type
//! as their only address input, so none of them can read a socket peer or
//! a forwarding header itself, and they all agree about where a request came
//! from (SEC-OPS-037).
//!
//! A `ClientContext` has no public constructor: outside the core, the only
//! way to get one is from the path-class function.

use core::net::IpAddr;

/// How a request reached the server, which decides its posture.
///
/// The classes follow the path-class table in the network security
/// baseline. A class can only remove access, never grant it: a loopback
/// request still has to sign in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PathClass {
    /// The server itself: a loopback peer that sent no forwarding headers.
    Loopback,
    /// The home network: a direct peer in private, shared or link-local
    /// space, or a trusted proxy the owner declared a private overlay.
    Home,
    /// A peer that is the server's own gateway, bridge or container
    /// gateway, behind which any client may hide. It is treated as
    /// non-local and has its own rate-limit bucket (SEC-NET-068).
    Unknown,
    /// Anything else that the server answers: a public peer or a public
    /// proxy.
    Internet,
    /// A peer that sent forwarding headers without being a trusted proxy.
    /// The headers are ignored and the address is the peer's own. It is
    /// never local, and it is told apart from [`PathClass::Internet`] so
    /// that the server can record the event and offer the owner the fix,
    /// which is to trust the proxy (SEC-NET-017).
    UntrustedProxy,
}

/// A resolved client address and its path class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientContext {
    addr: IpAddr,
    class: PathClass,
    via_proxy: bool,
}

impl ClientContext {
    /// Records what the path-class function resolved. Only the core can
    /// call this, so code outside it cannot make up where a request came
    /// from.
    #[allow(
        dead_code,
        reason = "the path-class function (WP-023) will be the caller outside tests"
    )]
    pub(crate) const fn new(addr: IpAddr, class: PathClass, via_proxy: bool) -> Self {
        Self {
            addr,
            class,
            via_proxy,
        }
    }

    /// The client's address: the socket peer, or through a trusted proxy
    /// the first address in the forwarding chain, read from the right, that
    /// is not itself a trusted proxy (SEC-NET-018).
    #[must_use]
    pub const fn addr(&self) -> IpAddr {
        self.addr
    }

    /// How the request reached the server.
    #[must_use]
    pub const fn class(&self) -> PathClass {
        self.class
    }

    /// Whether a trusted proxy carried the request, so that the address
    /// came from its forwarding headers rather than from the socket.
    #[must_use]
    pub const fn via_proxy(&self) -> bool {
        self.via_proxy
    }
}

/// Compile-fail tests: code outside the core cannot make a `ClientContext`.
/// Each shares its imports with the control, which compiles.
#[cfg(doctest)]
mod compile_fail {
    /// Control: outside the core, a context can be read.
    ///
    /// ```
    /// use core::net::{IpAddr, Ipv4Addr};
    /// use gunmetal_core::client_context::{ClientContext, PathClass};
    ///
    /// fn read(context: &ClientContext) -> (IpAddr, PathClass, bool) {
    ///     (context.addr(), context.class(), context.via_proxy())
    /// }
    /// ```
    struct Control;

    /// The constructor is private to the core.
    ///
    /// ```compile_fail
    /// use core::net::{IpAddr, Ipv4Addr};
    /// use gunmetal_core::client_context::{ClientContext, PathClass};
    ///
    /// fn claim_loopback() -> ClientContext {
    ///     ClientContext::new(IpAddr::V4(Ipv4Addr::LOCALHOST), PathClass::Loopback, false)
    /// }
    /// ```
    struct NoConstructor;

    /// The fields are private, so a struct literal cannot build one either.
    ///
    /// ```compile_fail
    /// use core::net::{IpAddr, Ipv4Addr};
    /// use gunmetal_core::client_context::{ClientContext, PathClass};
    ///
    /// fn claim_loopback() -> ClientContext {
    ///     ClientContext {
    ///         addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
    ///         class: PathClass::Loopback,
    ///         via_proxy: false,
    ///     }
    /// }
    /// ```
    struct NoStructLiteral;
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn keeps_the_address_class_and_proxy_flag_it_was_built_with() {
        let cases = [
            (IpAddr::V4(Ipv4Addr::LOCALHOST), PathClass::Loopback, false),
            (IpAddr::V6(Ipv6Addr::LOCALHOST), PathClass::Loopback, false),
            (
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20)),
                PathClass::Home,
                false,
            ),
            (
                IpAddr::V6(Ipv6Addr::new(0xFD00, 0, 0, 0, 0, 0, 0, 7)),
                PathClass::Home,
                true,
            ),
            (
                IpAddr::V4(Ipv4Addr::new(172, 17, 0, 1)),
                PathClass::Unknown,
                false,
            ),
            (
                IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7)),
                PathClass::Internet,
                false,
            ),
            (
                IpAddr::V6(Ipv6Addr::new(0x2001, 0xDB8, 0, 0, 0, 0, 0, 1)),
                PathClass::Internet,
                true,
            ),
            (
                IpAddr::V4(Ipv4Addr::new(198, 51, 100, 9)),
                PathClass::UntrustedProxy,
                false,
            ),
        ];
        for (addr, class, via_proxy) in cases {
            let context = ClientContext::new(addr, class, via_proxy);
            assert_eq!(
                (context.addr(), context.class(), context.via_proxy()),
                (addr, class, via_proxy)
            );
        }
    }
}
