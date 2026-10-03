//! IP addresses: the classifier and network ranges.
//!
//! [`classify`] is the one address classifier. The forwarded-header parser
//! (WP-023), the egress guard (WP-048), the secure-context report and the
//! invitation warnings all call it. It turns an IPv4-mapped IPv6 address
//! into IPv4 first, and it never calls an address that carries another
//! address (NAT64, 6to4, Teredo) local, whatever that address is
//! (SEC-NET-025). Every range in the IANA IPv4 and IPv6 special-purpose
//! registries is something other than [`AddrClass::Public`], so the egress
//! client, which connects only to public addresses, refuses all of them,
//! including the few the registries mark as globally reachable
//! (SEC-API-077).

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// What kind of address an IP address is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AddrClass {
    /// `0.0.0.0/8` ("this network") and `::`.
    Unspecified,
    /// `127.0.0.0/8` and `::1`.
    Loopback,
    /// RFC 1918 (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`) and
    /// unique-local `fc00::/7`, which holds tailnet addresses such as
    /// `fd7a:115c:a1e0::/48`.
    Private,
    /// `169.254.0.0/16` and `fe80::/10`.
    LinkLocal,
    /// `100.64.0.0/10` (RFC 6598), used by carrier NAT and by tailnets.
    /// Whether it is local depends on the server's own interfaces
    /// (SEC-NET-024), so it has a class of its own.
    SharedAddressSpace,
    /// `224.0.0.0/4` and `ff00::/8`.
    Multicast,
    /// `192.0.2.0/24`, `198.51.100.0/24`, `203.0.113.0/24`,
    /// `2001:db8::/32` and `3fff::/20`.
    Documentation,
    /// An IPv6 address that carries another address: NAT64
    /// (`64:ff9b::/96`, `64:ff9b:1::/48`), 6to4 (`2002::/16`) and Teredo
    /// (`2001::/32`). Never local, and never public, whatever it carries.
    Translated,
    /// Every other range in the IANA special-purpose registries, the
    /// reserved `240.0.0.0/4` with the limited broadcast address, and IPv6
    /// outside global unicast `2000::/3`.
    Reserved,
    /// Globally reachable.
    Public,
}

/// The IPv4 ranges that are not public. None overlaps another.
const V4_RANGES: [(Ipv4Addr, u8, AddrClass); 18] = [
    (Ipv4Addr::UNSPECIFIED, 8, AddrClass::Unspecified),
    (Ipv4Addr::new(10, 0, 0, 0), 8, AddrClass::Private),
    (
        Ipv4Addr::new(100, 64, 0, 0),
        10,
        AddrClass::SharedAddressSpace,
    ),
    (Ipv4Addr::new(127, 0, 0, 0), 8, AddrClass::Loopback),
    (Ipv4Addr::new(169, 254, 0, 0), 16, AddrClass::LinkLocal),
    (Ipv4Addr::new(172, 16, 0, 0), 12, AddrClass::Private),
    // IETF protocol assignments.
    (Ipv4Addr::new(192, 0, 0, 0), 24, AddrClass::Reserved),
    (Ipv4Addr::new(192, 0, 2, 0), 24, AddrClass::Documentation),
    // AS112-v4.
    (Ipv4Addr::new(192, 31, 196, 0), 24, AddrClass::Reserved),
    // AMT.
    (Ipv4Addr::new(192, 52, 193, 0), 24, AddrClass::Reserved),
    // The deprecated 6to4 relay anycast block.
    (Ipv4Addr::new(192, 88, 99, 0), 24, AddrClass::Reserved),
    (Ipv4Addr::new(192, 168, 0, 0), 16, AddrClass::Private),
    // Direct delegation AS112 service.
    (Ipv4Addr::new(192, 175, 48, 0), 24, AddrClass::Reserved),
    // Benchmarking.
    (Ipv4Addr::new(198, 18, 0, 0), 15, AddrClass::Reserved),
    (Ipv4Addr::new(198, 51, 100, 0), 24, AddrClass::Documentation),
    (Ipv4Addr::new(203, 0, 113, 0), 24, AddrClass::Documentation),
    (Ipv4Addr::new(224, 0, 0, 0), 4, AddrClass::Multicast),
    // Reserved, including the limited broadcast address.
    (Ipv4Addr::new(240, 0, 0, 0), 4, AddrClass::Reserved),
];

/// The IPv6 ranges inside global unicast `2000::/3` that are not public,
/// and the ones outside it with a class of their own. The first match
/// wins, so Teredo comes before the block that holds it.
const V6_RANGES: [(Ipv6Addr, u8, AddrClass); 13] = [
    (Ipv6Addr::UNSPECIFIED, 128, AddrClass::Unspecified),
    (Ipv6Addr::LOCALHOST, 128, AddrClass::Loopback),
    (
        Ipv6Addr::new(0x64, 0xff9b, 0, 0, 0, 0, 0, 0),
        96,
        AddrClass::Translated,
    ),
    (
        Ipv6Addr::new(0x64, 0xff9b, 1, 0, 0, 0, 0, 0),
        48,
        AddrClass::Translated,
    ),
    // Teredo.
    (
        Ipv6Addr::new(0x2001, 0, 0, 0, 0, 0, 0, 0),
        32,
        AddrClass::Translated,
    ),
    // IETF protocol assignments: benchmarking, AMT, AS112, ORCHID and
    // the rest.
    (
        Ipv6Addr::new(0x2001, 0, 0, 0, 0, 0, 0, 0),
        23,
        AddrClass::Reserved,
    ),
    (
        Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0),
        32,
        AddrClass::Documentation,
    ),
    // 6to4.
    (
        Ipv6Addr::new(0x2002, 0, 0, 0, 0, 0, 0, 0),
        16,
        AddrClass::Translated,
    ),
    // Direct delegation AS112 service.
    (
        Ipv6Addr::new(0x2620, 0x4f, 0x8000, 0, 0, 0, 0, 0),
        48,
        AddrClass::Reserved,
    ),
    (
        Ipv6Addr::new(0x3fff, 0, 0, 0, 0, 0, 0, 0),
        20,
        AddrClass::Documentation,
    ),
    (
        Ipv6Addr::new(0xfc00, 0, 0, 0, 0, 0, 0, 0),
        7,
        AddrClass::Private,
    ),
    (
        Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 0),
        10,
        AddrClass::LinkLocal,
    ),
    (
        Ipv6Addr::new(0xff00, 0, 0, 0, 0, 0, 0, 0),
        8,
        AddrClass::Multicast,
    ),
];

/// Global unicast, `2000::/3`.
const GLOBAL_UNICAST: Ipv6Addr = Ipv6Addr::new(0x2000, 0, 0, 0, 0, 0, 0, 0);

/// The mask of an IPv4 prefix of `prefix` bits.
fn mask_v4(prefix: u8) -> u32 {
    u32::MAX
        .checked_shl(32_u32.saturating_sub(u32::from(prefix)))
        .unwrap_or(0)
}

/// The mask of an IPv6 prefix of `prefix` bits.
fn mask_v6(prefix: u8) -> u128 {
    u128::MAX
        .checked_shl(128_u32.saturating_sub(u32::from(prefix)))
        .unwrap_or(0)
}

/// Classifies `ip`, after turning an IPv4-mapped IPv6 address into IPv4.
#[must_use]
pub fn classify(ip: IpAddr) -> AddrClass {
    match ip.to_canonical() {
        IpAddr::V4(v4) => {
            let bits = v4.to_bits();
            V4_RANGES
                .iter()
                .find(|&&(net, prefix, _)| bits & mask_v4(prefix) == net.to_bits())
                .map_or(AddrClass::Public, |&(_, _, class)| class)
        }
        IpAddr::V6(v6) => {
            let bits = v6.to_bits();
            let unlisted = if bits & mask_v6(3) == GLOBAL_UNICAST.to_bits() {
                AddrClass::Public
            } else {
                AddrClass::Reserved
            };
            V6_RANGES
                .iter()
                .find(|&&(net, prefix, _)| bits & mask_v6(prefix) == net.to_bits())
                .map_or(unlisted, |&(_, _, class)| class)
        }
    }
}

/// An IP network: an address and a prefix length, with no bits set past
/// the prefix. Used for the trusted-proxy list and egress grants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IpNet {
    addr: IpAddr,
    prefix: u8,
}

/// Why a network could not be parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetError {
    /// No `/` and prefix length.
    NoPrefix,
    /// The part before `/` is not an IP address.
    BadAddress,
    /// The prefix length is not one to three decimal digits without a
    /// leading zero.
    BadPrefix,
    /// The prefix length is longer than the address.
    PrefixTooLong {
        /// The prefix length given.
        prefix: u16,
        /// The longest prefix for the address family.
        max: u8,
    },
    /// The address has bits set past the prefix, so it names a host, not
    /// a network.
    HostBitsSet,
}

impl IpNet {
    /// Reads `address/prefix`, such as `192.168.1.0/24` or
    /// `fd7a:115c:a1e0::/48`.
    ///
    /// # Errors
    ///
    /// A [`NetError`] for anything else.
    pub fn parse(text: &str) -> Result<Self, NetError> {
        let (addr, prefix) = text.split_once('/').ok_or(NetError::NoPrefix)?;
        let addr: IpAddr = addr.parse().map_err(|_| NetError::BadAddress)?;
        let decimal = !prefix.is_empty()
            && prefix.len() <= 3
            && prefix.bytes().all(|octet| octet.is_ascii_digit())
            && (prefix == "0" || !prefix.starts_with('0'));
        if !decimal {
            return Err(NetError::BadPrefix);
        }
        let prefix: u16 = prefix.parse().unwrap_or(u16::MAX);
        let max = if addr.is_ipv4() { 32 } else { 128 };
        let Some(prefix) = u8::try_from(prefix).ok().filter(|&prefix| prefix <= max) else {
            return Err(NetError::PrefixTooLong { prefix, max });
        };
        let host_bits = match addr {
            IpAddr::V4(v4) => u128::from(v4.to_bits() & !mask_v4(prefix)),
            IpAddr::V6(v6) => v6.to_bits() & !mask_v6(prefix),
        };
        if host_bits != 0 {
            return Err(NetError::HostBitsSet);
        }
        Ok(Self { addr, prefix })
    }

    /// Whether `ip` is in this network, after turning an IPv4-mapped IPv6
    /// address into IPv4.
    #[must_use]
    pub fn contains(&self, ip: IpAddr) -> bool {
        match (self.addr, ip.to_canonical()) {
            (IpAddr::V4(net), IpAddr::V4(ip)) => {
                (net.to_bits() ^ ip.to_bits()) & mask_v4(self.prefix) == 0
            }
            (IpAddr::V6(net), IpAddr::V6(ip)) => {
                (net.to_bits() ^ ip.to_bits()) & mask_v6(self.prefix) == 0
            }
            _ => false,
        }
    }

    /// The network address.
    #[must_use]
    pub const fn addr(&self) -> IpAddr {
        self.addr
    }

    /// The prefix length.
    #[must_use]
    pub const fn prefix(&self) -> u8 {
        self.prefix
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use AddrClass::{
        Documentation, LinkLocal, Loopback, Multicast, Private, Public, Reserved,
        SharedAddressSpace, Translated, Unspecified,
    };
    use proptest::prelude::*;
    use proptest::sample::select;
    use std::net::{Ipv4Addr, Ipv6Addr};

    /// The IANA IPv4 Special-Purpose Address Registry (RFC 6890 and its
    /// updates), the multicast block of the IPv4 address space registry,
    /// and the class each range gets, written out by hand from the
    /// registries. Nested entries are listed too; the longest match wins.
    const V4_REGISTRY: [(&str, AddrClass); 26] = [
        ("0.0.0.0/8", Unspecified),
        ("0.0.0.0/32", Unspecified),
        ("10.0.0.0/8", Private),
        ("100.64.0.0/10", SharedAddressSpace),
        ("127.0.0.0/8", Loopback),
        ("169.254.0.0/16", LinkLocal),
        ("172.16.0.0/12", Private),
        ("192.0.0.0/24", Reserved),
        ("192.0.0.0/29", Reserved),
        ("192.0.0.8/32", Reserved),
        ("192.0.0.9/32", Reserved),
        ("192.0.0.10/32", Reserved),
        ("192.0.0.170/32", Reserved),
        ("192.0.0.171/32", Reserved),
        ("192.0.2.0/24", Documentation),
        ("192.31.196.0/24", Reserved),
        ("192.52.193.0/24", Reserved),
        ("192.88.99.0/24", Reserved),
        ("192.168.0.0/16", Private),
        ("192.175.48.0/24", Reserved),
        ("198.18.0.0/15", Reserved),
        ("198.51.100.0/24", Documentation),
        ("203.0.113.0/24", Documentation),
        ("224.0.0.0/4", Multicast),
        ("240.0.0.0/4", Reserved),
        ("255.255.255.255/32", Reserved),
    ];

    /// The IANA IPv6 Special-Purpose Address Registry, the multicast and
    /// link-local blocks of the IPv6 address space registry, and the class
    /// each gets. `::ffff:0:0/96` is absent on purpose: those addresses
    /// are classified as the IPv4 address they map. Outside these ranges,
    /// only global unicast `2000::/3` is public.
    const V6_REGISTRY: [(&str, AddrClass); 25] = [
        ("::/128", Unspecified),
        ("::1/128", Loopback),
        ("64:ff9b::/96", Translated),
        ("64:ff9b:1::/48", Translated),
        ("100::/64", Reserved),
        ("100:0:0:1::/64", Reserved),
        ("2001::/23", Reserved),
        ("2001::/32", Translated),
        ("2001:1::1/128", Reserved),
        ("2001:1::2/128", Reserved),
        ("2001:1::3/128", Reserved),
        ("2001:2::/48", Reserved),
        ("2001:3::/32", Reserved),
        ("2001:4:112::/48", Reserved),
        ("2001:10::/28", Reserved),
        ("2001:20::/28", Reserved),
        ("2001:30::/28", Reserved),
        ("2001:db8::/32", Documentation),
        ("2002::/16", Translated),
        ("2620:4f:8000::/48", Reserved),
        ("3fff::/20", Documentation),
        ("5f00::/16", Reserved),
        ("fc00::/7", Private),
        ("fe80::/10", LinkLocal),
        ("ff00::/8", Multicast),
    ];

    /// A range of the oracle: first address, last address, prefix, class.
    type Range = (u128, u128, u32, AddrClass);

    /// Parses an oracle entry with code that shares nothing with `IpNet`.
    fn range(cidr: &str, class: AddrClass) -> Range {
        let (addr, prefix) = cidr.split_once('/').unwrap();
        let prefix: u32 = prefix.parse().unwrap();
        let (bits, width) = match addr.parse::<IpAddr>().unwrap() {
            IpAddr::V4(v4) => (u128::from(u32::from(v4)), 32),
            IpAddr::V6(v6) => (u128::from(v6), 128),
        };
        // No registry range is a /0, so the shift stays below 128.
        let size_minus_one = (1_u128 << (width - prefix)) - 1;
        (bits, bits + size_minus_one, prefix, class)
    }

    /// The class the registries give `ip`, worked out from the tables.
    fn oracle(ip: IpAddr) -> AddrClass {
        let (bits, table, fallback): (u128, &[(&str, AddrClass)], AddrClass) = match ip {
            IpAddr::V4(v4) => (u128::from(u32::from(v4)), &V4_REGISTRY, Public),
            IpAddr::V6(v6) => {
                if let Some(v4) = v6.to_ipv4_mapped() {
                    return oracle(IpAddr::V4(v4));
                }
                let bits = u128::from(v6);
                let global_unicast = bits >> 125 == 0b001;
                (
                    bits,
                    &V6_REGISTRY,
                    if global_unicast { Public } else { Reserved },
                )
            }
        };
        table
            .iter()
            .map(|&(cidr, class)| range(cidr, class))
            .filter(|&(first, last, _, _)| (first..=last).contains(&bits))
            .max_by_key(|&(_, _, prefix, _)| prefix)
            .map_or(fallback, |(_, _, _, class)| class)
    }

    fn v4(text: &str) -> IpAddr {
        IpAddr::V4(text.parse::<Ipv4Addr>().unwrap())
    }

    fn v6(text: &str) -> IpAddr {
        IpAddr::V6(text.parse::<Ipv6Addr>().unwrap())
    }

    /// Verifies: SEC-NET-025, SEC-API-077
    #[test]
    fn classifies_well_known_addresses() {
        let cases = [
            ("127.0.0.1", Loopback),
            ("10.1.2.3", Private),
            ("172.15.255.255", Public),
            ("172.16.0.0", Private),
            ("172.31.255.255", Private),
            ("172.32.0.0", Public),
            ("192.168.1.10", Private),
            ("100.63.255.255", Public),
            ("100.64.0.1", SharedAddressSpace),
            ("100.127.255.255", SharedAddressSpace),
            ("100.128.0.0", Public),
            ("169.254.169.254", LinkLocal),
            ("0.0.0.0", Unspecified),
            ("0.1.2.3", Unspecified),
            ("224.0.0.1", Multicast),
            ("239.255.255.250", Multicast),
            ("255.255.255.255", Reserved),
            ("198.18.0.1", Reserved),
            ("192.0.2.1", Documentation),
            ("8.8.8.8", Public),
            ("1.1.1.1", Public),
        ];
        for (text, class) in cases {
            assert_eq!(classify(v4(text)), class, "{text}");
        }
    }

    /// Verifies: SEC-NET-025, SEC-API-077
    #[test]
    fn classifies_ipv6_addresses_and_never_calls_a_translated_one_local() {
        let cases = [
            ("::1", Loopback),
            ("::", Unspecified),
            // IPv4-mapped: classified as the IPv4 address.
            ("::ffff:127.0.0.1", Loopback),
            ("::ffff:10.0.0.1", Private),
            ("::ffff:169.254.169.254", LinkLocal),
            ("::ffff:8.8.8.8", Public),
            // IPv4-compatible (deprecated) is not mapped, so not loopback.
            ("::127.0.0.1", Reserved),
            // Addresses that embed loopback or private IPv4 addresses.
            ("64:ff9b::7f00:1", Translated),
            ("64:ff9b::c0a8:101", Translated),
            ("64:ff9b::808:808", Translated),
            ("64:ff9b:1::a00:1", Translated),
            ("2002:7f00:1::1", Translated),
            ("2002:c0a8:101::1", Translated),
            ("2001:0:4136:e378:8000:63bf:3fff:fdd2", Translated),
            ("fd7a:115c:a1e0::1", Private),
            ("fc00::1", Private),
            ("fe80::1", LinkLocal),
            ("febf:ffff::1", LinkLocal),
            ("fec0::1", Reserved),
            ("ff02::1", Multicast),
            ("2001:db8::1", Documentation),
            ("3fff::1", Documentation),
            ("2001:2::1", Reserved),
            ("2001:1ff::1", Reserved),
            ("2001:200::1", Public),
            ("2606:4700::1111", Public),
            ("1fff:ffff::1", Reserved),
            ("4000::1", Reserved),
            ("5f00::1", Reserved),
        ];
        for (text, class) in cases {
            assert_eq!(classify(v6(text)), class, "{text}");
        }
    }

    /// Each registry range at its first and last address, and the
    /// addresses just outside it, agree with the oracle.
    ///
    /// Verifies: SEC-NET-025, SEC-API-077
    #[test]
    fn agrees_with_the_registries_at_the_edge_of_every_range() {
        for (cidr, class) in V4_REGISTRY {
            let (first, last, _, _) = range(cidr, class);
            for bits in [first.wrapping_sub(1), first, last, last.wrapping_add(1)] {
                let Ok(bits) = u32::try_from(bits) else {
                    continue;
                };
                let ip = IpAddr::V4(Ipv4Addr::from(bits));
                assert_eq!(classify(ip), oracle(ip), "{ip} near {cidr}");
            }
        }
        for (cidr, class) in V6_REGISTRY {
            let (first, last, _, _) = range(cidr, class);
            for bits in [first.wrapping_sub(1), first, last, last.wrapping_add(1)] {
                let ip = IpAddr::V6(Ipv6Addr::from(bits));
                assert_eq!(classify(ip), oracle(ip), "{ip} near {cidr}");
            }
        }
    }

    /// Addresses inside a chosen registry range, so that small ranges are
    /// reached, mixed with uniformly random ones.
    fn address() -> impl Strategy<Value = IpAddr> {
        let in_v4_range =
            (select(V4_REGISTRY.to_vec()), any::<u32>()).prop_map(|((cidr, class), r)| {
                let (first, last, _, _) = range(cidr, class);
                let offset = u128::from(r) % (last - first + 1);
                IpAddr::V4(Ipv4Addr::from(u32::try_from(first + offset).unwrap()))
            });
        let in_v6_range =
            (select(V6_REGISTRY.to_vec()), any::<u128>()).prop_map(|((cidr, class), r)| {
                let (first, last, _, _) = range(cidr, class);
                IpAddr::V6(Ipv6Addr::from(first + r % (last - first + 1)))
            });
        prop_oneof![
            in_v4_range,
            in_v6_range,
            any::<u32>().prop_map(|bits| IpAddr::V4(Ipv4Addr::from(bits))),
            any::<u128>().prop_map(|bits| IpAddr::V6(Ipv6Addr::from(bits))),
            any::<u32>().prop_map(|bits| IpAddr::V6(Ipv4Addr::from(bits).to_ipv6_mapped())),
        ]
    }

    proptest! {
        /// Verifies: SEC-NET-025, SEC-API-077
        #[test]
        fn agrees_with_the_registries_everywhere(ip in address()) {
            prop_assert_eq!(classify(ip), oracle(ip));
        }

        /// Verifies: SEC-NET-025
        #[test]
        fn an_ipv4_mapped_address_is_classified_as_its_ipv4_address(bits in any::<u32>()) {
            let v4 = Ipv4Addr::from(bits);
            prop_assert_eq!(classify(IpAddr::V6(v4.to_ipv6_mapped())), classify(IpAddr::V4(v4)));
        }
    }

    #[test]
    fn reads_networks_of_both_families() {
        let cases = [
            ("10.0.0.0/8", v4("10.0.0.0"), 8),
            ("10.128.0.0/9", v4("10.128.0.0"), 9),
            ("0.0.0.0/0", v4("0.0.0.0"), 0),
            ("192.168.1.7/32", v4("192.168.1.7"), 32),
            ("fd7a:115c:a1e0::/48", v6("fd7a:115c:a1e0::"), 48),
            ("::/0", v6("::"), 0),
            ("::1/128", v6("::1"), 128),
        ];
        for (text, addr, prefix) in cases {
            let net = IpNet::parse(text);
            assert_eq!(net, Ok(IpNet { addr, prefix }), "{text}");
            assert_eq!(
                net.map(|net| (net.addr(), net.prefix())),
                Ok((addr, prefix))
            );
        }
    }

    #[test]
    fn refuses_malformed_networks() {
        let cases = [
            ("10.0.0.0", NetError::NoPrefix),
            ("", NetError::NoPrefix),
            ("10.0.0.0/", NetError::BadPrefix),
            ("10.0.0.0/08", NetError::BadPrefix),
            ("10.0.0.0/+8", NetError::BadPrefix),
            ("10.0.0.0/-1", NetError::BadPrefix),
            ("10.0.0.0/8/8", NetError::BadPrefix),
            ("10.0.0.0/ 8", NetError::BadPrefix),
            ("10.0.0.0/1000", NetError::BadPrefix),
            (
                "10.0.0.0/33",
                NetError::PrefixTooLong {
                    prefix: 33,
                    max: 32,
                },
            ),
            (
                "10.0.0.0/999",
                NetError::PrefixTooLong {
                    prefix: 999,
                    max: 32,
                },
            ),
            (
                "::/129",
                NetError::PrefixTooLong {
                    prefix: 129,
                    max: 128,
                },
            ),
            ("10.0.0.1/8", NetError::HostBitsSet),
            ("10.64.0.0/9", NetError::HostBitsSet),
            ("10.128.0.0/8", NetError::HostBitsSet),
            ("192.168.1.7/31", NetError::HostBitsSet),
            ("fd00::1/8", NetError::HostBitsSet),
            ("::1/127", NetError::HostBitsSet),
            ("010.0.0.0/8", NetError::BadAddress),
            ("10.0.0/8", NetError::BadAddress),
            ("[::1]/128", NetError::BadAddress),
            (" 10.0.0.0/8", NetError::BadAddress),
            ("fe80::1%eth0/64", NetError::BadAddress),
            ("/8", NetError::BadAddress),
        ];
        for (text, error) in cases {
            assert_eq!(IpNet::parse(text), Err(error), "{text:?}");
        }
    }

    #[test]
    fn contains_exactly_the_addresses_under_its_prefix() {
        let net = |text| IpNet::parse(text).unwrap();
        let cases = [
            ("10.0.0.0/8", "10.0.0.0", true),
            ("10.0.0.0/8", "10.255.255.255", true),
            ("10.0.0.0/8", "11.0.0.0", false),
            ("10.0.0.0/8", "9.255.255.255", false),
            ("10.0.0.0/8", "::ffff:10.1.2.3", true),
            ("10.0.0.0/8", "::a01:203", false),
            ("192.168.1.7/32", "192.168.1.7", true),
            ("192.168.1.7/32", "192.168.1.6", false),
            ("192.168.1.6/31", "192.168.1.7", true),
            ("0.0.0.0/0", "255.255.255.255", true),
            ("0.0.0.0/0", "::ffff:1.2.3.4", true),
            ("0.0.0.0/0", "::1", false),
            ("::/0", "2001:db8::1", true),
            ("::/0", "1.2.3.4", false),
            ("::/0", "::ffff:1.2.3.4", false),
            ("fd7a:115c:a1e0::/48", "fd7a:115c:a1e0:ffff::1", true),
            ("fd7a:115c:a1e0::/48", "fd7a:115c:a1e1::", false),
            ("::1/128", "::1", true),
            ("::1/128", "::2", false),
        ];
        for (network, ip, inside) in cases {
            let ip: IpAddr = ip.parse().unwrap();
            assert_eq!(net(network).contains(ip), inside, "{ip} in {network}");
        }
    }

    proptest! {
        /// Agrees with a mask computed here for any network and address.
        #[test]
        fn containment_agrees_with_a_reference_mask(
            net_bits in any::<u32>(),
            ip_bits in any::<u32>(),
            prefix in 0_u32..=32,
            v6_net in any::<u128>(),
            v6_ip in any::<u128>(),
            v6_prefix in 0_u32..=128,
        ) {
            let mask = if prefix == 0 { 0 } else { u32::MAX << (32 - prefix) };
            let network = Ipv4Addr::from(net_bits & mask);
            let parsed = IpNet::parse(&format!("{network}/{prefix}")).unwrap();
            let ip = IpAddr::V4(Ipv4Addr::from(ip_bits));
            prop_assert_eq!(parsed.contains(ip), ip_bits & mask == net_bits & mask);
            prop_assert!(parsed.contains(IpAddr::V4(Ipv4Addr::from(net_bits))));

            let mask = if v6_prefix == 0 { 0 } else { u128::MAX << (128 - v6_prefix) };
            let network = Ipv6Addr::from(v6_net & mask);
            let parsed = IpNet::parse(&format!("{network}/{v6_prefix}")).unwrap();
            let ip = Ipv6Addr::from(v6_ip);
            let expected = ip.to_ipv4_mapped().is_none() && v6_ip & mask == v6_net & mask;
            prop_assert_eq!(parsed.contains(IpAddr::V6(ip)), expected);
        }
    }
}
