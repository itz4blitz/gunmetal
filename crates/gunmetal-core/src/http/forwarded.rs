//! Forwarding headers, and the one path-class function (SEC-OPS-037).
//!
//! [`path_class`] is where every request's client address and path class
//! come from. The listener (WP-118), the only module that sees a socket,
//! gathers the request's forwarding headers into [`ForwardingHeaders`] and
//! calls it with the socket peer, the trusted-proxy list and the server's
//! own network facts. The [`ClientContext`] it returns is the only address
//! input of the sessions, the credential verifier, the audit log and the
//! rate limiters, so they all agree where a request came from. Only this
//! module and the listener may name a forwarding header; the gate's search
//! refuses one anywhere else.
//!
//! - **Forwarding headers are believed only from a trusted proxy**
//!   (SEC-NET-016). The list is the owner's, empty by default. From any
//!   other peer every forwarding header is ignored for every purpose, even
//!   a malformed one, and its presence alone gives the request the internet
//!   class: a reverse proxy the owner has not declared, on the same host or
//!   not, never makes a request look local (SEC-NET-017, A-510).
//! - **Through a trusted proxy, the client is the first address that is
//!   not a trusted proxy, reading the chain from the right** (SEC-NET-018,
//!   SEC-HIS-003). The left of the chain is whatever the client wrote, so
//!   the walk never passes an untrusted hop, and never passes a hop whose
//!   address is hidden (`unknown`, an obfuscated identifier, or a
//!   `Forwarded` element without `for`): the address of the proxy that
//!   reported the hidden hop stands in for it. A chain of trusted hops only
//!   resolves to the leftmost.
//! - **A chain the trusted proxy sent that cannot be read is refused** with
//!   400 (SEC-NET-018): one that breaks its header's grammar, holds more
//!   than [`MAX_HOPS`] hops or [`MAX_CHAIN_OCTETS`] octets, or arrives in
//!   both `Forwarded` and `X-Forwarded-For`, since a proxy that appends to
//!   one passes the other through unchanged from the client.
//! - **The path class.** Through trusted proxies, a request is home only
//!   when every trusted hop it passed is declared a private overlay, and
//!   internet otherwise (SEC-NET-019's declaration, public by default). A
//!   direct request is unknown from the server's own gateway, bridge or
//!   container gateway (SEC-NET-068), loopback from loopback, home from
//!   RFC 1918, RFC 4193 and IPv6 link-local space, and from RFC 6598 space
//!   only inside the network of one of the server's own interfaces
//!   (SEC-NET-024), and internet from anywhere else, IPv4 link-local
//!   included. An IPv4-mapped address counts as its IPv4 address
//!   throughout (SEC-NET-025).
//!
//! The chain grammars are RFC 7239's `Forwarded` and the de facto
//! `X-Forwarded-For`: a comma-separated list of nodes, each an IPv4
//! address or an IPv6 address in brackets, either with a port, `unknown`
//! or an obfuscated identifier. `X-Forwarded-For` writes the node alone
//! and also takes a bare IPv6 address; `Forwarded` writes it as the `for`
//! parameter of an element, quoted when it holds a bracket or a colon,
//! exactly as RFC 7239 has it, with no whitespace around `;` or `=`.
//! Several field lines of one header are one list, joined with commas in
//! order (RFC 9110, section 5.3). Scheme and host never come from these
//! headers (SEC-NET-015).
//!
//! The parse charges one step per octet of the chain plus one (k = 1,
//! c = 1 in SEC-MED-007's bound), keeps no depth because no structure in
//! either header nests, and stores at most [`MAX_CHAIN_OCTETS`] octets of
//! each chain and [`MAX_HOPS`] hops.

use std::borrow::Cow;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use super::syntax::{count, is_token, split_once, trim};
use crate::client_context::{ClientContext, PathClass};
use crate::net::{AddrClass, IpNet, classify};
use crate::parse::{Budget, ParseFault};
use crate::problem::{Describe, Problem, ProblemCode};
use crate::untrusted::Untrusted;

/// The most hops a forwarding chain may name (a proposal; no requirement
/// states a number).
pub const MAX_HOPS: u64 = 32;

/// The most octets one forwarding chain may hold, its field lines joined
/// by commas: the 16 KiB header section of SEC-NET-048.
pub const MAX_CHAIN_OCTETS: u64 = 16_384;

/// Steps charged per octet of a chain (k in SEC-MED-007's bound).
const STEPS_PER_OCTET: u64 = 1;

/// Steps charged beyond those (c in SEC-MED-007's bound).
const FIXED_STEPS: u64 = 1;

/// The prefix length of RFC 6598's shared address space,
/// `100.64.0.0/10`.
const SHARED_PREFIX: u8 = 10;

/// `Forwarded` (RFC 7239).
const FORWARDED: &[u8] = b"forwarded";

/// `X-Forwarded-For`.
const X_FORWARDED_FOR: &[u8] = b"x-forwarded-for";

/// Every header whose name starts with this is a forwarding header:
/// `X-Forwarded-Host`, `X-Forwarded-Proto` and the rest.
const X_FORWARDED_PREFIX: &[u8] = b"x-forwarded-";

/// The other headers through which proxies and CDNs pass a client address
/// (SEC-NET-016: "and similar headers"), sorted.
const ADDRESS_HEADERS: [&[u8]; 12] = [
    b"cf-connecting-ip",
    b"cf-connecting-ipv6",
    b"client-ip",
    b"fastly-client-ip",
    b"forwarded-for",
    b"true-client-ip",
    b"x-client-ip",
    b"x-cluster-client-ip",
    b"x-forwarded",
    b"x-original-forwarded-for",
    b"x-proxyuser-ip",
    b"x-real-ip",
];

/// How the owner declared a trusted proxy (SEC-NET-019).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyKind {
    /// Reachable from the internet: Cloudflare Tunnel, Tailscale Funnel, a
    /// reverse proxy with a public port. The default.
    Public,
    /// A private overlay, such as Tailscale Serve, that only the owner's
    /// own devices reach.
    PrivateOverlay,
}

/// One entry of the owner's trusted-proxy list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrustedProxy {
    /// The proxy's address or addresses.
    pub network: IpNet,
    /// How the owner declared it.
    pub kind: ProxyKind,
}

/// What the server knows about its own network, probed by the listener.
#[derive(Debug, Clone, Copy, Default)]
pub struct HostNetwork<'a> {
    /// The networks the server's own interfaces are on. For an overlay
    /// that routes its whole range through one interface, as Tailscale
    /// routes `100.64.0.0/10`, that range.
    pub interfaces: &'a [IpNet],
    /// The default gateway and bridge addresses of those interfaces, and
    /// any container gateway the owner configured: peers behind which any
    /// client may hide (SEC-NET-068).
    pub gateways: &'a [IpAddr],
}

/// A header that carries a forwarding chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainHeader {
    /// `Forwarded` (RFC 7239).
    Forwarded,
    /// `X-Forwarded-For`.
    XForwardedFor,
}

/// Why a trusted proxy's forwarding chain was refused. Each is answered
/// 400 (SEC-NET-018).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForwardedError {
    /// Both `Forwarded` and `X-Forwarded-For` arrived, so which one the
    /// proxy wrote cannot be told.
    BothChains,
    /// The chain holds more than [`MAX_CHAIN_OCTETS`] octets.
    TooLong {
        /// The chain's octets, its field lines joined by commas.
        octets: u64,
        /// The most it may hold.
        max: u64,
    },
    /// The chain names more than [`MAX_HOPS`] hops.
    TooManyHops {
        /// Where the first hop past the limit starts, in octets from the
        /// start of the joined chain.
        offset: u64,
        /// The most hops it may name.
        max: u64,
    },
    /// The chain breaks its header's grammar.
    Malformed {
        /// The header.
        header: ChainHeader,
        /// Where the element that breaks it starts, in octets from the
        /// start of the joined chain.
        offset: u64,
    },
    /// The parse spent its step budget (SEC-MED-007).
    Fault(ParseFault),
}

impl Describe for ForwardedError {
    /// Every refusal is the one `bad_forwarding_header` problem, with no
    /// arguments: which part of the chain was wrong goes to the server's
    /// log, not to the client.
    fn problem(&self) -> Problem {
        Problem {
            code: ProblemCode::BadForwardingHeader,
            args: Vec::new(),
        }
    }
}

/// One header's forwarding chain as it arrived.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Chain {
    /// Whether the header arrived at all, even empty.
    present: bool,
    /// The octets of the field lines, trimmed and joined by commas,
    /// counted in full.
    octets: u64,
    /// The joined field lines, kept only while they fit in
    /// [`MAX_CHAIN_OCTETS`].
    value: Vec<u8>,
}

/// The forwarding headers of one request, gathered by the listener with
/// [`ForwardingHeaders::add`] and read by [`path_class`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ForwardingHeaders {
    forwarded: Chain,
    x_forwarded_for: Chain,
    /// Whether any other forwarding header arrived.
    other: bool,
}

impl ForwardingHeaders {
    /// Records one field line of the request. The listener passes every
    /// field line; ones that are not forwarding headers are dropped.
    pub fn add(&mut self, name: Untrusted<&[u8]>, value: Untrusted<&[u8]>) {
        let name = name.into_inner();
        if name.eq_ignore_ascii_case(FORWARDED) {
            self.forwarded.push(value.into_inner());
        } else if name.eq_ignore_ascii_case(X_FORWARDED_FOR) {
            self.x_forwarded_for.push(value.into_inner());
        } else if is_address_header(name) {
            self.other = true;
        }
    }

    /// Whether any forwarding header arrived.
    const fn any(&self) -> bool {
        self.forwarded.present || self.x_forwarded_for.present || self.other
    }
}

/// Whether `name` is a forwarding header other than the two that carry a
/// chain.
fn is_address_header(name: &[u8]) -> bool {
    let prefixed = name
        .get(..X_FORWARDED_PREFIX.len())
        .is_some_and(|start| start.eq_ignore_ascii_case(X_FORWARDED_PREFIX));
    prefixed
        || ADDRESS_HEADERS
            .iter()
            .any(|header| name.eq_ignore_ascii_case(header))
}

impl Chain {
    /// Adds one field line: trimmed, and joined to the lines before it by a
    /// comma. Octets past [`MAX_CHAIN_OCTETS`] are counted, not kept.
    fn push(&mut self, line: &[u8]) {
        let (_, line) = trim(line);
        let separator = u64::from(self.present);
        self.present = true;
        self.octets = self
            .octets
            .saturating_add(separator)
            .saturating_add(count(line.len()));
        if self.octets <= MAX_CHAIN_OCTETS {
            if separator == 1 {
                self.value.push(b',');
            }
            self.value.extend_from_slice(line);
        }
    }
}

/// One hop of a forwarding chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hop {
    /// The address the hop gave, its port dropped.
    Address(IpAddr),
    /// `unknown`, an obfuscated identifier, or no `for` parameter.
    Hidden,
}

/// The client address and path class of a request from `peer` carrying
/// `headers`, given the owner's trusted `proxies` and the server's own
/// network.
///
/// # Errors
///
/// A [`ForwardedError`] when `peer` is a trusted proxy and its forwarding
/// chain cannot be read; the request is answered 400.
pub fn path_class(
    peer: IpAddr,
    headers: &ForwardingHeaders,
    proxies: &[TrustedProxy],
    host: &HostNetwork<'_>,
) -> Result<ClientContext, ForwardedError> {
    let peer = peer.to_canonical();
    let Some(kind) = proxy_kind(peer, proxies) else {
        let class = if headers.any() {
            PathClass::Internet
        } else {
            direct(peer, host)
        };
        return Ok(ClientContext::new(peer, class, false));
    };
    let chain = match (headers.forwarded.present, headers.x_forwarded_for.present) {
        (true, true) => return Err(ForwardedError::BothChains),
        (true, false) => Some((ChainHeader::Forwarded, &headers.forwarded)),
        (false, true) => Some((ChainHeader::XForwardedFor, &headers.x_forwarded_for)),
        (false, false) => None,
    };
    let hops = match chain {
        Some((header, chain)) => {
            if chain.octets > MAX_CHAIN_OCTETS {
                return Err(ForwardedError::TooLong {
                    octets: chain.octets,
                    max: MAX_CHAIN_OCTETS,
                });
            }
            let mut budget = Budget::for_input(chain.octets, STEPS_PER_OCTET, FIXED_STEPS);
            hops(header, &chain.value, &mut budget)?
        }
        None => Vec::new(),
    };
    // Walk from the right. Each trusted hop moves the client one proxy
    // further out; the first hop that is not a trusted proxy is the
    // client, and a hidden hop leaves the proxy that reported it.
    let mut public = kind == ProxyKind::Public;
    let mut client = peer;
    for &hop in hops.iter().rev() {
        let Hop::Address(addr) = hop else {
            break;
        };
        client = addr.to_canonical();
        let Some(kind) = proxy_kind(client, proxies) else {
            break;
        };
        public = public || kind == ProxyKind::Public;
    }
    let class = if public {
        PathClass::Internet
    } else {
        PathClass::Home
    };
    Ok(ClientContext::new(client, class, true))
}

/// How the owner declared `addr`, or `None` when it is not a trusted
/// proxy. An address in a public entry is public even when a private
/// overlay entry holds it too.
fn proxy_kind(addr: IpAddr, proxies: &[TrustedProxy]) -> Option<ProxyKind> {
    proxies
        .iter()
        .filter(|proxy| proxy.network.contains(addr))
        .map(|proxy| proxy.kind)
        .reduce(|kind, next| {
            if kind == ProxyKind::Public {
                kind
            } else {
                next
            }
        })
}

/// The path class of a direct peer, already in canonical form.
fn direct(peer: IpAddr, host: &HostNetwork<'_>) -> PathClass {
    if host
        .gateways
        .iter()
        .any(|gateway| gateway.to_canonical() == peer)
    {
        return PathClass::Unknown;
    }
    match classify(peer) {
        AddrClass::Loopback => PathClass::Loopback,
        AddrClass::Private => PathClass::Home,
        AddrClass::LinkLocal if peer.is_ipv6() => PathClass::Home,
        AddrClass::SharedAddressSpace
            if host
                .interfaces
                .iter()
                .any(|network| network.prefix() >= SHARED_PREFIX && network.contains(peer)) =>
        {
            PathClass::Home
        }
        _ => PathClass::Internet,
    }
}

/// Reads a joined chain into its hops, under `budget`.
///
/// Charges each list element with the comma after it, so a chain costs its
/// length plus one.
fn hops(
    header: ChainHeader,
    value: &[u8],
    budget: &mut Budget,
) -> Result<Vec<Hop>, ForwardedError> {
    let mut hops = Vec::new();
    for (offset, piece) in pieces(value, b',') {
        let steps = count(piece.len()).saturating_add(1);
        budget
            .charge(steps, offset)
            .map_err(ForwardedError::Fault)?;
        let (inner, element) = trim(piece);
        if element.is_empty() {
            continue;
        }
        let offset = offset.saturating_add(inner);
        if count(hops.len()) >= MAX_HOPS {
            return Err(ForwardedError::TooManyHops {
                offset,
                max: MAX_HOPS,
            });
        }
        let hop = match header {
            ChainHeader::Forwarded => forwarded_element(element),
            ChainHeader::XForwardedFor => node(element, true),
        };
        hops.push(hop.ok_or(ForwardedError::Malformed { header, offset })?);
    }
    Ok(hops)
}

/// Reads one `Forwarded` element: parameters `token=value` separated by
/// `;`, each value a token or a quoted string, of which only `for`, at
/// most once, matters.
fn forwarded_element(element: &[u8]) -> Option<Hop> {
    let mut hop = None;
    for (_, pair) in pieces(element, b';') {
        if pair.is_empty() {
            continue;
        }
        let (name, value) = split_once(pair, b'=')?;
        let value = parameter_value(value)?;
        if !is_token(name) {
            return None;
        }
        if name.eq_ignore_ascii_case(b"for") {
            if hop.is_some() {
                return None;
            }
            hop = Some(node(&value, false)?);
        }
    }
    Some(hop.unwrap_or(Hop::Hidden))
}

/// A parameter's value: a token as it is, or a quoted string unescaped.
fn parameter_value(value: &[u8]) -> Option<Cow<'_, [u8]>> {
    if value.first() == Some(&b'"') {
        return unquote(value).map(Cow::Owned);
    }
    is_token(value).then_some(Cow::Borrowed(value))
}

/// The text of a quoted string (RFC 9110, section 5.6.4) that makes up all
/// of `value`, with its escapes undone.
fn unquote(value: &[u8]) -> Option<Vec<u8>> {
    let mut text = Vec::new();
    let mut octets = value.get(1..).unwrap_or_default().iter();
    loop {
        match *octets.next()? {
            b'"' => return octets.next().is_none().then_some(text),
            b'\\' => {
                let escaped = *octets.next()?;
                if !(escaped == b'\t' || escaped >= b' ') || escaped == 0x7F {
                    return None;
                }
                text.push(escaped);
            }
            octet if octet == b'\t' || (b' '..0x7F).contains(&octet) || octet >= 0x80 => {
                text.push(octet);
            }
            _ => return None,
        }
    }
}

/// Reads a node: an address with an optional port, `unknown` or an
/// obfuscated identifier (RFC 7239, section 6). An IPv6 address goes in
/// brackets, except in `X-Forwarded-For` (`bare_ipv6`), which also takes
/// one bare, without a port.
fn node(text: &[u8], bare_ipv6: bool) -> Option<Hop> {
    if bare_ipv6 && let Some(v6) = ipv6(text) {
        return Some(Hop::Address(IpAddr::V6(v6)));
    }
    let (hop, port) = if let Some(bracketed) = text.strip_prefix(b"[") {
        let (inside, after) = split_once(bracketed, b']')?;
        let port = if after.is_empty() {
            None
        } else {
            Some(after.strip_prefix(b":")?)
        };
        (Hop::Address(IpAddr::V6(ipv6(inside)?)), port)
    } else {
        let (name, port) = match split_once(text, b':') {
            Some((name, port)) => (name, Some(port)),
            None => (text, None),
        };
        (node_name(name)?, port)
    };
    port.is_none_or(is_port).then_some(hop)
}

/// A node name that is not in brackets: an IPv4 address, `unknown` or an
/// obfuscated identifier.
fn node_name(name: &[u8]) -> Option<Hop> {
    if name.eq_ignore_ascii_case(b"unknown") || is_obfuscated(name) {
        return Some(Hop::Hidden);
    }
    let v4: Ipv4Addr = core::str::from_utf8(name).ok()?.parse().ok()?;
    Some(Hop::Address(IpAddr::V4(v4)))
}

/// Reads an IPv6 address written out in full text, with no zone.
fn ipv6(text: &[u8]) -> Option<Ipv6Addr> {
    core::str::from_utf8(text).ok()?.parse().ok()
}

/// Whether `port` is one to five digits or an obfuscated port.
fn is_port(port: &[u8]) -> bool {
    let digits = (1..=5).contains(&port.len()) && port.iter().all(u8::is_ascii_digit);
    digits || is_obfuscated(port)
}

/// Whether `text` is `_` followed by one or more letters, digits, `.`,
/// `_` or `-`: an obfuscated node or port.
fn is_obfuscated(text: &[u8]) -> bool {
    text.strip_prefix(b"_").is_some_and(|rest| {
        !rest.is_empty()
            && rest
                .iter()
                .all(|&octet| octet.is_ascii_alphanumeric() || b"._-".contains(&octet))
    })
}

/// The pieces of `bytes` between `separator`s outside quoted strings, each
/// with its offset, empty ones included: one more than the separators.
///
/// Each piece moves on past its separator, so the walk ends after at most
/// one piece per octet plus one, whatever [`piece_len`] answers
/// (SEC-MED-008).
fn pieces(bytes: &[u8], separator: u8) -> impl Iterator<Item = (u64, &[u8])> {
    let mut rest = Some(bytes);
    let mut at = 0_u64;
    core::iter::from_fn(move || {
        let bytes = rest?;
        let len = piece_len(bytes, separator);
        rest = bytes.get(len.saturating_add(1)..);
        let offset = at;
        at = at.saturating_add(count(len)).saturating_add(1);
        Some((offset, bytes.get(..len).unwrap_or(bytes)))
    })
}

/// How many octets of `bytes` come before its first `separator` outside a
/// quoted string: all of them when there is none.
fn piece_len(bytes: &[u8], separator: u8) -> usize {
    let mut quoted = false;
    let mut escaped = false;
    let end = bytes.iter().position(|&octet| {
        let found = !quoted && octet == separator;
        if escaped {
            escaped = false;
        } else if quoted {
            match octet {
                b'\\' => escaped = true,
                b'"' => quoted = false,
                _ => {}
            }
        } else if octet == b'"' {
            quoted = true;
        }
        found
    });
    end.unwrap_or(bytes.len())
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use super::*;
    use proptest::collection::vec;
    use proptest::prelude::*;
    use proptest::sample::select;

    /// The stack size SEC-MED-001 names, in octets.
    const SMALL_STACK: usize = 262_144;

    /// Runs `work` on a 256 KiB stack so a recursive forwarding parse fails
    /// its test instead of passing on the runner's larger stack (SEC-MED-001).
    fn on_small_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(SMALL_STACK)
            .spawn(work)
            .expect("the test thread starts")
            .join()
            .expect("the code under test returned instead of panicking")
    }

    fn ip(text: &str) -> IpAddr {
        text.parse().unwrap()
    }

    fn net(text: &str) -> IpNet {
        IpNet::parse(Untrusted::new(text)).unwrap()
    }

    fn public(network: &str) -> TrustedProxy {
        TrustedProxy {
            network: net(network),
            kind: ProxyKind::Public,
        }
    }

    fn overlay(network: &str) -> TrustedProxy {
        TrustedProxy {
            network: net(network),
            kind: ProxyKind::PrivateOverlay,
        }
    }

    fn headers(lines: &[(&str, &str)]) -> ForwardingHeaders {
        let mut headers = ForwardingHeaders::default();
        for (name, value) in lines {
            headers.add(
                Untrusted::new(name.as_bytes()),
                Untrusted::new(value.as_bytes()),
            );
        }
        headers
    }

    const NO_HOST: HostNetwork<'static> = HostNetwork {
        interfaces: &[],
        gateways: &[],
    };

    fn context(addr: &str, class: PathClass, via_proxy: bool) -> ClientContext {
        ClientContext::new(ip(addr), class, via_proxy)
    }

    /// The path class of a request from `peer` with `lines`, through
    /// `proxies`, on a server with no network facts.
    fn resolve(
        peer: &str,
        lines: &[(&str, &str)],
        proxies: &[TrustedProxy],
    ) -> Result<ClientContext, ForwardedError> {
        path_class(ip(peer), &headers(lines), proxies, &NO_HOST)
    }

    /// `piece` written `times` times.
    fn repeated(piece: &str, times: usize) -> String {
        (0..times).map(|_| piece).collect()
    }

    /// A budget too large to run out, to measure what a parse spends.
    fn unlimited() -> Budget {
        Budget::for_input(0, 0, u64::MAX)
    }

    /// One proxy, the public `10.0.0.0/8`, and a request from `10.0.0.1`
    /// through it.
    fn through_proxy(lines: &[(&str, &str)]) -> Result<ClientContext, ForwardedError> {
        resolve("10.0.0.1", lines, &[public("10.0.0.0/8")])
    }

    /// The context of a request through a public proxy from `addr`.
    fn proxied(addr: &str) -> ClientContext {
        context(addr, PathClass::Internet, true)
    }

    fn xff(value: &str) -> Result<ClientContext, ForwardedError> {
        through_proxy(&[("X-Forwarded-For", value)])
    }

    fn fwd(value: &str) -> Result<ClientContext, ForwardedError> {
        through_proxy(&[("Forwarded", value)])
    }

    const fn malformed(header: ChainHeader, offset: u64) -> ForwardedError {
        ForwardedError::Malformed { header, offset }
    }

    /// Every header name that counts as a forwarding header, as the
    /// network baseline lists them (SEC-NET-016) and in other cases.
    const FORWARDING_NAMES: [&str; 22] = [
        "Forwarded",
        "forwarded",
        "X-Forwarded-For",
        "x-forwarded-for",
        "X-FORWARDED-FOR",
        "X-Forwarded-Host",
        "X-Forwarded-Proto",
        "X-Forwarded-Port",
        "X-Forwarded-Prefix",
        "x-forwarded-",
        "X-Forwarded",
        "X-Real-IP",
        "True-Client-IP",
        "CF-Connecting-IP",
        "CF-Connecting-IPv6",
        "X-Client-IP",
        "X-Cluster-Client-IP",
        "Fastly-Client-IP",
        "Forwarded-For",
        "X-Original-Forwarded-For",
        "Client-IP",
        "X-ProxyUser-IP",
    ];

    #[test]
    fn gathers_the_chains_and_notes_other_forwarding_headers() {
        let gathered = headers(&[
            ("X-Forwarded-For", " 192.0.2.1 "),
            ("Host", "example.com"),
            ("x-forwarded-for", ""),
            ("Forwarded", "for=192.0.2.9"),
            ("X-FORWARDED-FOR", "\t198.51.100.2, 10.0.0.2"),
        ]);
        assert_eq!(
            gathered,
            ForwardingHeaders {
                forwarded: Chain {
                    present: true,
                    octets: 13,
                    value: b"for=192.0.2.9".to_vec(),
                },
                x_forwarded_for: Chain {
                    present: true,
                    octets: 33,
                    value: b"192.0.2.1,,198.51.100.2, 10.0.0.2".to_vec(),
                },
                other: false,
            }
        );
        for name in FORWARDING_NAMES {
            let gathered = headers(&[(name, "192.0.2.1")]);
            assert!(
                gathered.other || gathered.forwarded.present || gathered.x_forwarded_for.present,
                "{name}"
            );
        }
        let other = headers(&[("X-Real-IP", "192.0.2.1")]);
        assert_eq!(
            other,
            ForwardingHeaders {
                other: true,
                ..ForwardingHeaders::default()
            }
        );
        let empty = headers(&[("Forwarded", "")]);
        assert_eq!(
            empty.forwarded,
            Chain {
                present: true,
                octets: 0,
                value: Vec::new(),
            }
        );
    }

    #[test]
    fn ordinary_headers_are_not_forwarding_headers() {
        let names = [
            "Host",
            "Via",
            "User-Agent",
            "Forward",
            "Forwardedd",
            "X-Forward-For",
            "XForwarded-For",
            "X-Forwarded_For",
            "X-Real-IPs",
            "Client-IPv6",
            "",
            "X-Forwarde",
        ];
        for name in names {
            assert_eq!(
                headers(&[(name, "192.0.2.1")]),
                ForwardingHeaders::default(),
                "{name}"
            );
        }
    }

    /// Stops keeping a chain once it is longer than the limit, but goes on
    /// counting it.
    ///
    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn keeps_at_most_the_octet_limit_of_a_chain() {
        let long = repeated("1", 16_384);
        let gathered = headers(&[("X-Forwarded-For", &long), ("X-Forwarded-For", "2")]);
        assert_eq!(gathered.x_forwarded_for.octets, 16_386);
        assert_eq!(gathered.x_forwarded_for.value, long.as_bytes());
        let gathered = headers(&[("X-Forwarded-For", &long[1..]), ("X-Forwarded-For", "")]);
        assert_eq!(gathered.x_forwarded_for.octets, 16_384);
        assert_eq!(gathered.x_forwarded_for.value.len(), 16_384);
    }

    /// The path class of a direct peer, from a table written from the
    /// network baseline's path-class table and SEC-NET-024.
    #[test]
    fn classifies_a_direct_peer_by_its_address() {
        use PathClass::{Home, Internet, Loopback};
        let cases = [
            ("127.0.0.1", "127.0.0.1", Loopback),
            ("127.255.0.9", "127.255.0.9", Loopback),
            ("::1", "::1", Loopback),
            ("::ffff:127.0.0.1", "127.0.0.1", Loopback),
            ("10.1.2.3", "10.1.2.3", Home),
            ("172.16.0.1", "172.16.0.1", Home),
            ("192.168.1.20", "192.168.1.20", Home),
            ("::ffff:192.168.1.20", "192.168.1.20", Home),
            ("fd7a:115c:a1e0::1", "fd7a:115c:a1e0::1", Home),
            ("fc00::1", "fc00::1", Home),
            ("fe80::1", "fe80::1", Home),
            ("169.254.1.1", "169.254.1.1", Internet),
            ("::ffff:169.254.1.1", "169.254.1.1", Internet),
            ("100.64.0.1", "100.64.0.1", Internet),
            ("203.0.113.7", "203.0.113.7", Internet),
            ("8.8.8.8", "8.8.8.8", Internet),
            ("2606:4700::1111", "2606:4700::1111", Internet),
            ("2001:db8::1", "2001:db8::1", Internet),
            ("64:ff9b::a00:1", "64:ff9b::a00:1", Internet),
            ("2002:c0a8:101::1", "2002:c0a8:101::1", Internet),
            ("0.0.0.0", "0.0.0.0", Internet),
            ("::", "::", Internet),
            ("224.0.0.1", "224.0.0.1", Internet),
            ("ff02::1", "ff02::1", Internet),
            ("255.255.255.255", "255.255.255.255", Internet),
        ];
        for (peer, addr, class) in cases {
            assert_eq!(
                resolve(peer, &[], &[]),
                Ok(context(addr, class, false)),
                "{peer}"
            );
        }
    }

    /// Verifies: SEC-NET-068
    #[test]
    fn a_gateway_or_bridge_peer_is_unknown() {
        let gateways = [
            ip("172.17.0.1"),
            ip("192.168.1.1"),
            ip("fe80::1"),
            ip("::ffff:10.0.0.1"),
        ];
        let host = HostNetwork {
            interfaces: &[],
            gateways: &gateways,
        };
        let cases = [
            ("172.17.0.1", "172.17.0.1", PathClass::Unknown),
            ("::ffff:172.17.0.1", "172.17.0.1", PathClass::Unknown),
            ("192.168.1.1", "192.168.1.1", PathClass::Unknown),
            ("fe80::1", "fe80::1", PathClass::Unknown),
            ("10.0.0.1", "10.0.0.1", PathClass::Unknown),
            ("172.17.0.2", "172.17.0.2", PathClass::Home),
            ("127.0.0.1", "127.0.0.1", PathClass::Loopback),
            ("203.0.113.7", "203.0.113.7", PathClass::Internet),
        ];
        for (peer, addr, class) in cases {
            assert_eq!(
                path_class(ip(peer), &headers(&[]), &[], &host),
                Ok(context(addr, class, false)),
                "{peer}"
            );
        }
        // A gateway that sends forwarding headers is an undeclared proxy.
        assert_eq!(
            path_class(
                ip("172.17.0.1"),
                &headers(&[("X-Forwarded-For", "192.168.1.5")]),
                &[],
                &host
            ),
            Ok(context("172.17.0.1", PathClass::Internet, false))
        );
    }

    #[test]
    fn shared_address_space_is_home_only_inside_an_interface_network() {
        let cases = [
            ("100.64.0.0/10", "100.64.0.1", PathClass::Home),
            ("100.64.0.0/10", "100.127.255.255", PathClass::Home),
            ("100.64.0.0/10", "100.128.0.1", PathClass::Internet),
            ("100.101.0.0/16", "100.101.2.3", PathClass::Home),
            ("100.101.0.0/16", "100.102.0.1", PathClass::Internet),
            ("100.101.2.3/32", "100.101.2.3", PathClass::Home),
            ("100.0.0.0/8", "100.64.0.1", PathClass::Internet),
            ("100.0.0.0/9", "100.64.0.1", PathClass::Internet),
            ("0.0.0.0/0", "100.64.0.1", PathClass::Internet),
            ("192.168.1.0/24", "100.64.0.1", PathClass::Internet),
            ("203.0.113.0/24", "203.0.113.7", PathClass::Internet),
        ];
        for (interface, peer, class) in cases {
            let interfaces = [net(interface)];
            let host = HostNetwork {
                interfaces: &interfaces,
                gateways: &[],
            };
            assert_eq!(
                path_class(ip(peer), &headers(&[]), &[], &host),
                Ok(context(peer, class, false)),
                "{peer} on {interface}"
            );
        }
    }

    /// Every forwarding header, from a peer that is not a trusted proxy, is
    /// ignored: the address stays the peer's and the class is internet,
    /// even from loopback and even when the header is malformed.
    ///
    /// Verifies: SEC-NET-016
    #[test]
    fn ignores_forwarding_headers_from_a_peer_that_is_not_a_trusted_proxy() {
        let peers = [
            "203.0.113.7",
            "192.168.1.20",
            "127.0.0.1",
            "::1",
            "::ffff:127.0.0.1",
        ];
        let values = ["127.0.0.1", "for=127.0.0.1", "garbage[", ""];
        for proxies in [&[][..], &[public("10.0.0.0/8")][..]] {
            for peer in peers {
                let addr = ip(peer).to_canonical();
                for name in FORWARDING_NAMES {
                    for value in values {
                        assert_eq!(
                            resolve(peer, &[(name, value)], proxies),
                            Ok(ClientContext::new(addr, PathClass::Internet, false)),
                            "{name}: {value} from {peer}"
                        );
                    }
                }
                let both = [("Forwarded", "for=127.0.0.1"), ("X-Forwarded-For", "::1")];
                assert_eq!(
                    resolve(peer, &both, proxies),
                    Ok(ClientContext::new(addr, PathClass::Internet, false)),
                    "both from {peer}"
                );
                let over_long = repeated("1.1.1.1,", 3_000);
                assert_eq!(
                    resolve(peer, &[("X-Forwarded-For", &over_long)], proxies),
                    Ok(ClientContext::new(addr, PathClass::Internet, false)),
                    "over long from {peer}"
                );
            }
        }
    }

    /// Verifies: SEC-NET-018, SEC-HIS-003
    #[test]
    fn through_a_trusted_proxy_the_client_is_the_first_untrusted_hop_from_the_right() {
        let cases = [
            (xff("203.0.113.7"), "203.0.113.7"),
            (xff("203.0.113.7, 10.0.0.2"), "203.0.113.7"),
            (xff("127.0.0.1, 198.51.100.9"), "198.51.100.9"),
            (xff("192.168.1.1,198.51.100.9,10.9.9.9"), "198.51.100.9"),
            (xff("1.1.1.1, 198.51.100.9, 10.0.0.2"), "198.51.100.9"),
            (xff("10.0.0.3, 10.0.0.2"), "10.0.0.3"),
            (xff("203.0.113.7, 10.0.0.3, 10.0.0.2"), "203.0.113.7"),
            (xff("::ffff:203.0.113.7"), "203.0.113.7"),
            (xff("2001:db8::1"), "2001:db8::1"),
            (xff("[2001:db8::1]"), "2001:db8::1"),
            (xff("[2001:db8::1]:443"), "2001:db8::1"),
            (xff("203.0.113.7:5678"), "203.0.113.7"),
            (xff("203.0.113.7, [::ffff:10.0.0.2]:80"), "203.0.113.7"),
            (xff(""), "10.0.0.1"),
            (xff(" , ,"), "10.0.0.1"),
            (through_proxy(&[]), "10.0.0.1"),
            (through_proxy(&[("X-Real-IP", "127.0.0.1")]), "10.0.0.1"),
            (
                through_proxy(&[
                    ("X-Forwarded-For", "127.0.0.1"),
                    ("X-Forwarded-For", "198.51.100.9, 10.0.0.2"),
                ]),
                "198.51.100.9",
            ),
            (
                through_proxy(&[
                    ("X-Forwarded-For", "198.51.100.9"),
                    ("X-Forwarded-For", "10.0.0.2"),
                ]),
                "198.51.100.9",
            ),
            (
                fwd("for=192.0.2.60;proto=http;by=203.0.113.43"),
                "192.0.2.60",
            ),
            (fwd("for=127.0.0.1, for=192.0.2.60"), "192.0.2.60"),
            (fwd("for=192.0.2.60, for=10.0.0.2"), "192.0.2.60"),
            (fwd("for=\"[2001:db8:cafe::17]:4711\""), "2001:db8:cafe::17"),
            (fwd("for=\"[2001:db8:cafe::17]\""), "2001:db8:cafe::17"),
            (fwd("for=\"192.0.2.60:8080\""), "192.0.2.60"),
            (fwd("for=\"192.0.2.60:_hidden\""), "192.0.2.60"),
            (fwd("for=\"[::ffff:10.0.0.2]\""), "10.0.0.2"),
            (fwd("For=192.0.2.60"), "192.0.2.60"),
            (fwd("FOR=192.0.2.60"), "192.0.2.60"),
        ];
        for (actual, expected) in cases {
            assert_eq!(actual, Ok(proxied(expected)), "{expected}");
        }
    }

    /// A hidden hop is where the walk stops: the client is the proxy that
    /// reported it, never an entry further left.
    ///
    /// Verifies: SEC-NET-018, SEC-HIS-003
    #[test]
    fn a_hidden_hop_leaves_the_address_of_the_proxy_that_reported_it() {
        let cases = [
            (fwd("for=_gazonk"), "10.0.0.1"),
            (fwd("For=\"_gazonk\""), "10.0.0.1"),
            (fwd("for=unknown"), "10.0.0.1"),
            (fwd("FOR=UNKNOWN"), "10.0.0.1"),
            (fwd("for=\"unknown:443\""), "10.0.0.1"),
            (fwd("for=\"_abc:_port\""), "10.0.0.1"),
            (fwd("for=unknown, for=10.0.0.2"), "10.0.0.2"),
            (
                fwd("for=203.0.113.7, for=_hidden, for=10.0.0.2"),
                "10.0.0.2",
            ),
            (fwd("by=10.0.0.9;proto=https"), "10.0.0.1"),
            (fwd("for=203.0.113.7, proto=https"), "10.0.0.1"),
            (fwd(";"), "10.0.0.1"),
            (xff("unknown"), "10.0.0.1"),
            (xff("203.0.113.7, unknown"), "10.0.0.1"),
            (xff("203.0.113.7, Unknown, 10.0.0.2"), "10.0.0.2"),
            (xff("203.0.113.7, _a.b-c_9, 10.0.0.2"), "10.0.0.2"),
        ];
        for (actual, expected) in cases {
            assert_eq!(actual, Ok(proxied(expected)), "{expected}");
        }
    }

    #[test]
    fn reads_quoted_strings_with_escapes_and_separators_inside() {
        let cases = [
            ("for=\"[2001:db8::1]:443\";host=\"a,b;c\"", "2001:db8::1"),
            ("for=\"\\[2001:db8::1\\]\"", "2001:db8::1"),
            (
                "host=\"x\\\",for=203.0.113.9\";for=192.0.2.60",
                "192.0.2.60",
            ),
            ("host=\"x\\\\\";for=192.0.2.60", "192.0.2.60"),
            ("for=192.0.2.60;;proto=http;", "192.0.2.60"),
            (
                "proto=https;for=192.0.2.60;by=\"[2001:db8::1]\"",
                "192.0.2.60",
            ),
            ("secret=\"\t !#[]~\";for=192.0.2.60", "192.0.2.60"),
            ("note=\"a\\\t\\ \\~\";for=192.0.2.60", "192.0.2.60"),
        ];
        for (value, expected) in cases {
            assert_eq!(fwd(value), Ok(proxied(expected)), "{value}");
        }
        // Octets above 0x7F, alone and after a backslash.
        let raw = path_class(
            ip("10.0.0.1"),
            &{
                let mut gathered = ForwardingHeaders::default();
                gathered.add(
                    Untrusted::new(b"Forwarded"),
                    Untrusted::new(b"x=\"\x80\\\xff\";for=192.0.2.60"),
                );
                gathered
            },
            &[public("10.0.0.0/8")],
            &NO_HOST,
        );
        assert_eq!(raw, Ok(proxied("192.0.2.60")));
    }

    /// Verifies: SEC-NET-018
    #[test]
    fn refuses_a_malformed_chain_from_a_trusted_proxy() {
        use ChainHeader::{Forwarded, XForwardedFor};
        let cases = [
            (xff("203.0.113.7 198.51.100.9"), malformed(XForwardedFor, 0)),
            (xff("010.0.0.1"), malformed(XForwardedFor, 0)),
            (xff("203.0.113.7, 999.1.1.1"), malformed(XForwardedFor, 13)),
            (xff("203.0.113.7,\t, x"), malformed(XForwardedFor, 15)),
            (xff("fe80::1%eth0"), malformed(XForwardedFor, 0)),
            (xff("[1.2.3.4]"), malformed(XForwardedFor, 0)),
            (xff("[2001:db8::1"), malformed(XForwardedFor, 0)),
            (xff("[2001:db8::1]x"), malformed(XForwardedFor, 0)),
            (xff("[2001:db8::1]:"), malformed(XForwardedFor, 0)),
            (xff("[2001:db8::1]:123456"), malformed(XForwardedFor, 0)),
            (xff("2001:db8::1:443:"), malformed(XForwardedFor, 0)),
            (xff("203.0.113.7:"), malformed(XForwardedFor, 0)),
            (xff("203.0.113.7:123456"), malformed(XForwardedFor, 0)),
            (xff("203.0.113.7:12a"), malformed(XForwardedFor, 0)),
            (xff("203.0.113.7:_"), malformed(XForwardedFor, 0)),
            (xff("203.0.113.7:_a:b"), malformed(XForwardedFor, 0)),
            (xff("_"), malformed(XForwardedFor, 0)),
            (xff("_a b"), malformed(XForwardedFor, 0)),
            (xff("_a/b"), malformed(XForwardedFor, 0)),
            (xff("unknownx"), malformed(XForwardedFor, 0)),
            (xff("\"203.0.113.7\""), malformed(XForwardedFor, 0)),
            (xff("example.com"), malformed(XForwardedFor, 0)),
            (fwd("for=[2001:db8::1]"), malformed(Forwarded, 0)),
            (fwd("for=192.0.2.60:80"), malformed(Forwarded, 0)),
            (
                fwd("for=192.0.2.60;for=198.51.100.1"),
                malformed(Forwarded, 0),
            ),
            (fwd("for=unknown;For=_x"), malformed(Forwarded, 0)),
            (fwd("for"), malformed(Forwarded, 0)),
            (fwd("for="), malformed(Forwarded, 0)),
            (fwd("=192.0.2.60"), malformed(Forwarded, 0)),
            (fwd("for=\"\""), malformed(Forwarded, 0)),
            (fwd("for=\"192.0.2.60"), malformed(Forwarded, 0)),
            (fwd("for=\"192.0.2.60\"x"), malformed(Forwarded, 0)),
            (fwd("for=\"192.0.2.60\\"), malformed(Forwarded, 0)),
            (fwd("for=192.0.2.60; proto=http"), malformed(Forwarded, 0)),
            (fwd("for=192.0.2.60 ;proto=http"), malformed(Forwarded, 0)),
            (fwd("for =192.0.2.60"), malformed(Forwarded, 0)),
            (fwd("for= 192.0.2.60"), malformed(Forwarded, 0)),
            (fwd("for=\"[fe80::1%25eth0]\""), malformed(Forwarded, 0)),
            (fwd("f@r=192.0.2.60"), malformed(Forwarded, 0)),
            (fwd("for=a\"b"), malformed(Forwarded, 0)),
            (fwd("for=\"a\x01b\""), malformed(Forwarded, 0)),
            (fwd("for=\"a\x7fb\""), malformed(Forwarded, 0)),
            (fwd("for=\"a\\\x01\""), malformed(Forwarded, 0)),
            (fwd("for=\"a\\\x1f\""), malformed(Forwarded, 0)),
            (fwd("for=\"a\\\x7f\""), malformed(Forwarded, 0)),
            (fwd("for=\"a\x1fb\""), malformed(Forwarded, 0)),
            (fwd("for=example.com"), malformed(Forwarded, 0)),
            // Every parameter's value follows the grammar, not only for's.
            (fwd("x=\"\x01\";for=192.0.2.60"), malformed(Forwarded, 0)),
            (fwd("x=\"\x1f\";for=192.0.2.60"), malformed(Forwarded, 0)),
            (fwd("x=\"\x7f\";for=192.0.2.60"), malformed(Forwarded, 0)),
            (fwd("x=\"\\\x01\";for=192.0.2.60"), malformed(Forwarded, 0)),
            (fwd("x=\"\\\x1f\";for=192.0.2.60"), malformed(Forwarded, 0)),
            (fwd("x=\"\\\x7f\";for=192.0.2.60"), malformed(Forwarded, 0)),
            (fwd("for=192.0.2.60, for=bogus"), malformed(Forwarded, 16)),
            (fwd("proto=a b"), malformed(Forwarded, 0)),
        ];
        for (actual, expected) in cases {
            assert_eq!(actual, Err(expected));
        }
        let invalid_utf8 = path_class(
            ip("10.0.0.1"),
            &{
                let mut gathered = ForwardingHeaders::default();
                gathered.add(
                    Untrusted::new(b"X-Forwarded-For"),
                    Untrusted::new(b"192.0.2.\xff"),
                );
                gathered
            },
            &[public("10.0.0.0/8")],
            &NO_HOST,
        );
        assert_eq!(invalid_utf8, Err(malformed(XForwardedFor, 0)));
    }

    /// Verifies: SEC-NET-018
    #[test]
    fn refuses_both_chain_headers_from_a_trusted_proxy() {
        let both = [
            [
                ("Forwarded", "for=192.0.2.60"),
                ("X-Forwarded-For", "192.0.2.60"),
            ],
            [("X-Forwarded-For", "192.0.2.60"), ("Forwarded", "")],
            [("forwarded", ""), ("x-forwarded-for", "")],
        ];
        for lines in both {
            assert_eq!(through_proxy(&lines), Err(ForwardedError::BothChains));
        }
    }

    /// Verifies: SEC-NET-018, SEC-MED-006, SEC-TM-032
    #[test]
    fn refuses_a_chain_of_more_than_32_hops() {
        let hops = |trusted: usize| {
            let mut chain = vec!["198.51.100.1"];
            chain.extend((0..trusted).map(|_| "10.0.0.2"));
            chain.join(", ")
        };
        assert_eq!(xff(&hops(31)), Ok(proxied("198.51.100.1")));
        // The 33rd hop starts after "198.51.100.1" and 31 of ", 10.0.0.2",
        // and its own ", ".
        assert_eq!(
            xff(&hops(32)),
            Err(ForwardedError::TooManyHops {
                offset: 324,
                max: 32,
            })
        );
        let with_gaps = format!("{},,", hops(31));
        assert_eq!(xff(&with_gaps), Ok(proxied("198.51.100.1")));
        let forwarded = |count: usize| {
            let elements: Vec<&str> = (0..count).map(|_| "for=10.0.0.2").collect();
            elements.join(",")
        };
        assert_eq!(fwd(&forwarded(32)), Ok(proxied("10.0.0.2")));
        assert_eq!(
            fwd(&forwarded(33)),
            Err(ForwardedError::TooManyHops {
                offset: 416,
                max: 32,
            })
        );
    }

    /// Verifies: SEC-NET-018, SEC-MED-006, SEC-TM-032
    #[test]
    fn refuses_a_chain_of_more_than_16_kib() {
        let padded = |spaces: usize| format!("198.51.100.1,{}10.0.0.2", repeated(" ", spaces));
        assert_eq!(padded(16_363).len(), 16_384);
        assert_eq!(xff(&padded(16_363)), Ok(proxied("198.51.100.1")));
        assert_eq!(
            xff(&padded(16_364)),
            Err(ForwardedError::TooLong {
                octets: 16_385,
                max: 16_384,
            })
        );
        // Across two field lines, the comma that joins them counts.
        let first = format!("198.51.100.1,{}10.0.0.3", repeated(" ", 8_171));
        assert_eq!(first.len(), 8_192);
        let second = |spaces: usize| format!("10.0.0.2,{}10.0.0.4", repeated(" ", spaces));
        assert_eq!(second(8_174).len(), 8_191);
        assert_eq!(
            through_proxy(&[
                ("X-Forwarded-For", &first),
                ("X-Forwarded-For", &second(8_174))
            ]),
            Ok(proxied("198.51.100.1"))
        );
        assert_eq!(
            through_proxy(&[
                ("X-Forwarded-For", &first),
                ("X-Forwarded-For", &second(8_175))
            ]),
            Err(ForwardedError::TooLong {
                octets: 16_385,
                max: 16_384,
            })
        );
    }

    /// A request is home only when every trusted hop it passed is a
    /// private overlay.
    #[test]
    fn every_trusted_hop_on_the_path_decides_the_class() {
        use PathClass::{Home, Internet};
        let both = [overlay("fd00::/8"), public("10.0.0.0/8")];
        // Peer, X-Forwarded-For (or none), client, class.
        let cases = [
            ("fd00::1", Some("192.168.1.5"), "192.168.1.5", Home),
            ("fd00::1", Some("203.0.113.7"), "203.0.113.7", Home),
            ("fd00::1", None, "fd00::1", Home),
            ("fd00::1", Some("203.0.113.7, fd00::2"), "203.0.113.7", Home),
            (
                "fd00::1",
                Some("203.0.113.7, 10.0.0.2"),
                "203.0.113.7",
                Internet,
            ),
            ("fd00::1", Some("10.0.0.3"), "10.0.0.3", Internet),
            (
                "10.0.0.1",
                Some("192.168.1.5, fd00::2"),
                "192.168.1.5",
                Internet,
            ),
            ("10.0.0.1", None, "10.0.0.1", Internet),
        ];
        for (peer, chain, addr, class) in cases {
            let lines: Vec<(&str, &str)> = chain
                .map(|chain| ("X-Forwarded-For", chain))
                .into_iter()
                .collect();
            assert_eq!(
                resolve(peer, &lines, &both),
                Ok(context(addr, class, true)),
                "{peer} {chain:?}"
            );
        }
    }

    /// Overlapping entries, a proxy on loopback, and a proxy at the
    /// gateway address.
    #[test]
    fn a_public_declaration_outweighs_an_overlay_one_for_the_same_address() {
        use PathClass::{Home, Internet};
        /// Peer, X-Forwarded-For (or none), proxies, client, class.
        type Case<'a> = (
            &'a str,
            Option<&'a str>,
            &'a [TrustedProxy],
            &'a str,
            PathClass,
        );
        let overlay_first = [overlay("10.0.0.0/8"), public("10.0.0.1/32")];
        let public_first = [public("10.0.0.1/32"), overlay("10.0.0.0/8")];
        let loopback_public = [public("127.0.0.1/32")];
        let loopback_overlay = [overlay("127.0.0.1/32")];
        let cases: [Case<'_>; 8] = [
            (
                "10.0.0.1",
                Some("10.0.0.2"),
                &overlay_first,
                "10.0.0.2",
                Internet,
            ),
            (
                "10.0.0.1",
                Some("10.0.0.2"),
                &public_first,
                "10.0.0.2",
                Internet,
            ),
            ("10.0.0.2", None, &public_first, "10.0.0.2", Home),
            ("10.0.0.2", None, &overlay_first, "10.0.0.2", Home),
            (
                "10.0.0.2",
                Some("10.0.0.1, 10.0.0.3"),
                &overlay_first,
                "10.0.0.1",
                Internet,
            ),
            (
                "127.0.0.1",
                Some("203.0.113.7"),
                &loopback_public,
                "203.0.113.7",
                Internet,
            ),
            ("127.0.0.1", None, &loopback_public, "127.0.0.1", Internet),
            (
                "::ffff:127.0.0.1",
                None,
                &loopback_overlay,
                "127.0.0.1",
                Home,
            ),
        ];
        for (peer, chain, proxies, addr, class) in cases {
            let lines: Vec<(&str, &str)> = chain
                .map(|chain| ("X-Forwarded-For", chain))
                .into_iter()
                .collect();
            assert_eq!(
                resolve(peer, &lines, proxies),
                Ok(context(addr, class, true)),
                "{peer} {chain:?}"
            );
        }
        // The owner's declaration of a proxy outweighs the gateway rule.
        let gateways = [ip("10.0.0.1")];
        let host = HostNetwork {
            interfaces: &[],
            gateways: &gateways,
        };
        assert_eq!(
            path_class(
                ip("10.0.0.1"),
                &headers(&[]),
                &[public("10.0.0.0/8")],
                &host
            ),
            Ok(context("10.0.0.1", PathClass::Internet, true))
        );
    }

    /// Verifies: SEC-NET-018
    #[test]
    fn describes_every_refusal_as_one_problem() {
        let errors = [
            ForwardedError::BothChains,
            ForwardedError::TooLong {
                octets: 16_385,
                max: 16_384,
            },
            ForwardedError::TooManyHops {
                offset: 324,
                max: 32,
            },
            malformed(ChainHeader::Forwarded, 3),
            ForwardedError::Fault(ParseFault::BudgetExceeded { offset: 7 }),
        ];
        for error in errors {
            assert_eq!(
                error.problem(),
                Problem {
                    code: ProblemCode::BadForwardingHeader,
                    args: Vec::new(),
                },
                "{error:?}"
            );
        }
        assert_eq!(ProblemCode::BadForwardingHeader.status(), Some(400));
    }

    #[test]
    fn a_piece_ends_at_a_separator_outside_quoted_strings() {
        let cases: [(&[u8], usize); 13] = [
            (b"", 0),
            (b"a", 1),
            (b"a,b", 1),
            (b"a,", 1),
            (b",", 0),
            (b"ab,c,d", 2),
            (b"\"a,b\",c", 5),
            (b"\"a\\\",b\",c", 7),
            (b"\"a\\\\\",c", 5),
            (b"\"a,b", 4),
            (b"a\\,b", 2),
            (b"a\"b,c\"d,e", 7),
            (b"\"\"\",\",", 5),
        ];
        for (bytes, len) in cases {
            assert_eq!(piece_len(bytes, b','), len, "{bytes:?}");
        }
    }

    /// Every loop over a list moves on by at least one octet, so it ends
    /// after at most one piece per octet plus one, whatever the pieces are.
    ///
    /// Verifies: SEC-MED-008
    #[test]
    fn walks_every_piece_once_and_stops() {
        /// A list and its pieces with their offsets.
        type Walk<'a> = (&'a [u8], &'a [(u64, &'a [u8])]);
        let cases: [Walk<'_>; 5] = [
            (b"", &[(0, b"")]),
            (b",", &[(0, b""), (1, b"")]),
            (b"a, b", &[(0, b"a"), (2, b" b")]),
            (b"\"a,b\",c,", &[(0, b"\"a,b\""), (6, b"c"), (8, b"")]),
            (b";;", &[(0, b""), (1, b""), (2, b"")]),
        ];
        for (bytes, expected) in cases {
            let separator = if bytes.starts_with(b";") { b';' } else { b',' };
            let pieces: Vec<(u64, &[u8])> =
                pieces(bytes, separator).take(bytes.len() + 2).collect();
            assert_eq!(pieces, expected, "{bytes:?}");
        }
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn the_budget_runs_out_at_exactly_the_step_it_should() {
        // "203.0.113.7" costs 12 steps at offset 0 and " 10.0.0.2" 10 at
        // offset 12: 22 steps for 21 octets.
        let value = b"203.0.113.7, 10.0.0.2";
        let both = vec![
            Hop::Address(ip("203.0.113.7")),
            Hop::Address(ip("10.0.0.2")),
        ];
        assert_eq!(
            hops(
                ChainHeader::XForwardedFor,
                value,
                &mut Budget::for_input(0, 0, 22)
            ),
            Ok(both)
        );
        for (steps, offset) in [(21, 12), (12, 12), (11, 0), (0, 0)] {
            assert_eq!(
                hops(
                    ChainHeader::XForwardedFor,
                    value,
                    &mut Budget::for_input(0, 0, steps)
                ),
                Err(ForwardedError::Fault(ParseFault::BudgetExceeded { offset })),
                "{steps} steps"
            );
        }
        assert_eq!(
            hops(ChainHeader::Forwarded, b"", &mut Budget::for_input(0, 0, 0)),
            Err(ForwardedError::Fault(ParseFault::BudgetExceeded {
                offset: 0
            }))
        );
    }

    /// Verifies: SEC-MED-007, SEC-MED-008
    #[test]
    fn spends_one_step_per_octet_plus_one_on_separators_alone() {
        for value in ["", ",", ",,,,,,,,", " , ,\t,"] {
            for header in [ChainHeader::Forwarded, ChainHeader::XForwardedFor] {
                let mut budget = unlimited();
                assert_eq!(hops(header, value.as_bytes(), &mut budget), Ok(Vec::new()));
                assert_eq!(u64::MAX - budget.remaining(), value.len() as u64 + 1);
            }
        }
    }

    /// Neither header nests, so the parser keeps no depth and never
    /// recurses: text that looks like deep nesting comes back as a typed
    /// result on a 256 KiB stack.
    ///
    /// Verifies: SEC-MED-005, SEC-MED-001
    #[test]
    fn never_recurses_on_text_that_looks_deeply_nested() {
        let cases = [
            (ChainHeader::XForwardedFor, repeated("[", 16_384)),
            (ChainHeader::XForwardedFor, repeated("\"", 16_384)),
            (ChainHeader::XForwardedFor, repeated("\\", 16_384)),
            (
                ChainHeader::Forwarded,
                format!("for={}", repeated("\"", 16_380)),
            ),
            (
                ChainHeader::Forwarded,
                format!("for=\"{}", repeated("\\\"", 8_189)),
            ),
            (
                ChainHeader::Forwarded,
                format!("for=\"{}\"", repeated("[", 16_378)),
            ),
            (ChainHeader::Forwarded, repeated(";", 16_384)),
            (ChainHeader::Forwarded, repeated("for=", 4_096)),
        ];
        for (header, value) in cases {
            let name = match header {
                ChainHeader::Forwarded => "Forwarded",
                ChainHeader::XForwardedFor => "X-Forwarded-For",
            };
            let expected = if value.starts_with(';') {
                Ok(proxied("10.0.0.1"))
            } else {
                Err(malformed(header, 0))
            };
            let answer = on_small_stack(move || through_proxy(&[(name, &value)]));
            assert_eq!(answer, expected, "{name}");
        }
    }

    /// The trusted networks of the properties below.
    fn trusted() -> [TrustedProxy; 2] {
        [public("10.0.0.0/8"), public("fd00::/8")]
    }

    /// Whether `addr` is in one of [`trusted`]'s networks, worked out from
    /// its octets.
    fn is_trusted(addr: IpAddr) -> bool {
        match addr.to_canonical() {
            IpAddr::V4(v4) => v4.octets()[0] == 10,
            IpAddr::V6(v6) => v6.octets()[0] == 0xfd,
        }
    }

    /// Addresses of every kind, IPv4-mapped ones among them, with trusted
    /// ones forced in.
    fn any_address() -> impl Strategy<Value = IpAddr> {
        prop_oneof![
            any::<u32>().prop_map(|bits| IpAddr::V4(Ipv4Addr::from(bits))),
            any::<u128>().prop_map(|bits| IpAddr::V6(Ipv6Addr::from(bits))),
            any::<u32>().prop_map(|bits| IpAddr::V6(Ipv4Addr::from(bits).to_ipv6_mapped())),
            trusted_address(),
        ]
    }

    /// Addresses inside [`trusted`]'s networks.
    fn trusted_address() -> impl Strategy<Value = IpAddr> {
        prop_oneof![
            (0_u32..1 << 24).prop_map(|low| IpAddr::V4(Ipv4Addr::from(0x0A00_0000 | low))),
            (0_u32..1 << 24)
                .prop_map(|low| IpAddr::V6(Ipv4Addr::from(0x0A00_0000 | low).to_ipv6_mapped())),
            any::<u128>().prop_map(|bits| IpAddr::V6(Ipv6Addr::from((0xfd << 120) | (bits >> 8)))),
        ]
    }

    /// One hop of `X-Forwarded-For`, written in the style `style` picks.
    fn xff_node(addr: IpAddr, style: u8) -> String {
        match (addr, style % 3) {
            (IpAddr::V4(v4), 0) => v4.to_string(),
            (IpAddr::V4(v4), _) => format!("{v4}:{}", u16::from(style) + 1),
            (IpAddr::V6(v6), 0) => v6.to_string(),
            (IpAddr::V6(v6), 1) => format!("[{v6}]"),
            (IpAddr::V6(v6), _) => format!("[{v6}]:443"),
        }
    }

    /// One element of `Forwarded`, written in the style `style` picks.
    fn forwarded_element(addr: IpAddr, style: u8) -> String {
        let node = match (addr, style % 3) {
            (IpAddr::V4(v4), 0) => v4.to_string(),
            (IpAddr::V4(v4), 1) => format!("\"{v4}:8080\""),
            (IpAddr::V4(v4), _) => format!("\"{v4}\""),
            (IpAddr::V6(v6), 1) => format!("\"[{v6}]:4711\""),
            (IpAddr::V6(v6), _) => format!("\"[{v6}]\""),
        };
        let name = ["for", "For", "FOR"][usize::from(style / 3 % 3)];
        match style / 9 % 3 {
            0 => format!("{name}={node}"),
            1 => format!("proto=https;{name}={node}"),
            _ => format!("{name}={node};by=_proxy"),
        }
    }

    /// `hops` written as field lines of one chain header: each hop in its
    /// style, separated as `gaps` say, split into lines where `breaks` say.
    fn written(
        forwarded: bool,
        hops: &[(IpAddr, u8)],
        gaps: &[&str],
        breaks: &[bool],
    ) -> Vec<(&'static str, String)> {
        let name = if forwarded {
            "Forwarded"
        } else {
            "X-Forwarded-For"
        };
        let mut lines = vec![(name, String::new())];
        for (index, &(addr, style)) in hops.iter().enumerate() {
            let node = if forwarded {
                forwarded_element(addr, style)
            } else {
                xff_node(addr, style)
            };
            let line = &mut lines.last_mut().unwrap().1;
            line.push_str(&node);
            if index + 1 < hops.len() {
                if breaks[index % breaks.len()] {
                    lines.push((name, String::new()));
                } else {
                    line.push_str(gaps[index % gaps.len()]);
                }
            }
        }
        lines
    }

    /// The ways hops may be separated.
    fn gaps() -> impl Strategy<Value = Vec<&'static str>> {
        vec(select(vec![",", ", ", " ,\t", ",,", ", , "]), 1..4)
    }

    proptest! {
        /// For any attacker-chosen prefix followed by any sequence of
        /// trusted proxies, the client is the last untrusted hop.
        ///
        /// Verifies: SEC-NET-018, SEC-HIS-003
        #[test]
        fn the_client_is_the_last_untrusted_hop_whatever_the_attacker_writes(
            prefix in vec((any_address(), any::<u8>()), 0..12),
            client in (any_address().prop_filter("untrusted", |&addr| !is_trusted(addr)), any::<u8>()),
            suffix in vec((trusted_address(), any::<u8>()), 0..12),
            peer in trusted_address(),
            forwarded in any::<bool>(),
            gaps in gaps(),
            breaks in vec(any::<bool>(), 1..4),
        ) {
            let mut chain = prefix;
            chain.push(client);
            chain.extend(suffix);
            let lines = written(forwarded, &chain, &gaps, &breaks);
            let lines: Vec<(&str, &str)> = lines.iter().map(|(name, value)| (*name, value.as_str())).collect();
            let resolved = path_class(peer, &headers(&lines), &trusted(), &NO_HOST);
            prop_assert_eq!(
                resolved,
                Ok(ClientContext::new(client.0.to_canonical(), PathClass::Internet, true)),
                "{:?}", lines
            );
        }

        /// A chain of trusted hops only resolves to its leftmost hop.
        ///
        /// Verifies: SEC-NET-018, SEC-HIS-003
        #[test]
        fn a_chain_of_trusted_hops_resolves_to_the_leftmost(
            chain in vec((trusted_address(), any::<u8>()), 1..20),
            peer in trusted_address(),
            forwarded in any::<bool>(),
            gaps in gaps(),
        ) {
            let lines = written(forwarded, &chain, &gaps, &[false]);
            let lines: Vec<(&str, &str)> = lines.iter().map(|(name, value)| (*name, value.as_str())).collect();
            let resolved = path_class(peer, &headers(&lines), &trusted(), &NO_HOST);
            prop_assert_eq!(
                resolved,
                Ok(ClientContext::new(chain[0].0.to_canonical(), PathClass::Internet, true))
            );
        }

        /// With an empty trusted list, the address is always the peer and
        /// any forwarding header makes the request internet.
        ///
        /// Verifies: SEC-NET-016
        #[test]
        fn with_no_trusted_proxy_the_address_is_always_the_peer(
            peer in any_address(),
            lines in vec(
                (
                    prop_oneof![
                        select(FORWARDING_NAMES.to_vec()).prop_map(|name| (name.as_bytes().to_vec(), true)),
                        select(vec!["Host", "Via", "User-Agent"]).prop_map(|name| (name.as_bytes().to_vec(), false)),
                    ],
                    vec(any::<u8>(), 0..32),
                ),
                0..6,
            ),
        ) {
            let mut gathered = ForwardingHeaders::default();
            for ((name, _), value) in &lines {
                gathered.add(Untrusted::new(name), Untrusted::new(value));
            }
            let forwarding = lines.iter().any(|((_, forwarding), _)| *forwarding);
            let resolved = path_class(peer, &gathered, &[], &NO_HOST).unwrap();
            let bare = path_class(peer, &ForwardingHeaders::default(), &[], &NO_HOST).unwrap();
            prop_assert_eq!(resolved.addr(), peer.to_canonical());
            prop_assert!(!resolved.via_proxy());
            prop_assert_eq!(resolved.class(), if forwarding { PathClass::Internet } else { bare.class() });
        }

        /// Whatever the headers, the path-class function returns rather
        /// than panics, and through a trusted proxy says so.
        ///
        /// Verifies: SEC-MED-001
        #[test]
        fn returns_for_any_headers_from_any_peer(
            peer in prop_oneof![any_address(), trusted_address()],
            lines in vec(
                (
                    prop_oneof![
                        select(vec!["Forwarded", "X-Forwarded-For", "X-Real-IP"]).prop_map(|name| name.as_bytes().to_vec()),
                        vec(any::<u8>(), 0..16),
                    ],
                    prop_oneof![
                        vec(any::<u8>(), 0..64),
                        "([0-9a-f:.\\[\\]_\" ,;=\\\\]|for|unknown){0,24}".prop_map(String::into_bytes),
                    ],
                ),
                0..4,
            ),
        ) {
            let answer = on_small_stack(move || {
                let mut gathered = ForwardingHeaders::default();
                for (name, value) in &lines {
                    gathered.add(Untrusted::new(name), Untrusted::new(value));
                }
                path_class(peer, &gathered, &trusted(), &NO_HOST)
            });
            if let Ok(resolved) = answer {
                prop_assert_eq!(resolved.via_proxy(), is_trusted(peer));
            }
        }

        /// Verifies: SEC-MED-007, SEC-MED-008, SEC-TM-032
        #[test]
        fn spends_at_most_one_step_per_octet_plus_one(
            value in prop_oneof![
                vec(any::<u8>(), 0..64),
                "([0-9a-f:.\\[\\]_\" ,;=\\\\]|for|unknown){0,24}".prop_map(String::into_bytes),
                (select(vec![",", "\"", "\\\"", ";", "for=", "[", " ,", "1.1.1.1,"]), 0_usize..2_000)
                    .prop_map(|(piece, times)| repeated(piece, times).into_bytes()),
            ],
            forwarded in any::<bool>(),
        ) {
            let header = if forwarded { ChainHeader::Forwarded } else { ChainHeader::XForwardedFor };
            let mut budget = unlimited();
            let answer = hops(header, &value, &mut budget);
            let spent = u64::MAX - budget.remaining();
            prop_assert!(spent <= value.len() as u64 + 1, "{} steps for {} octets", spent, value.len());
            let mut exact = Budget::for_input(value.len() as u64, STEPS_PER_OCTET, FIXED_STEPS);
            prop_assert_eq!(hops(header, &value, &mut exact), answer);
        }
    }
}
