//! The check on what a name resolved to.
//!
//! A request connects only to an address that passed this check, and the
//! check sees every address the name resolved to: one refused address
//! among them refuses the request, because a name that answers with a
//! public address and a private one is how a rebinding attack looks
//! (SEC-EXT-002, SEC-API-077). The classes come from the core's one
//! classifier, which reads an IPv4-mapped IPv6 address as IPv4 and never
//! calls an address that carries another one public, so the NAT64, 6to4 and
//! Teredo forms of a private address are refused as well (SEC-HIS-023).
//!
//! An address of a kind the destination may reach is still refused when the
//! server itself listens on it, on whichever port ([`Listening`]). And a
//! name that resolves to more than [`MAX_RESOLVED`] addresses is refused
//! before any of them is looked at, so an answer cannot make the check as
//! long as it likes.
//!
//! What passes is a [`Pinned`], which only this check makes. The one place
//! that connects takes a [`Pinned`], so it cannot be handed an address that
//! was not checked. And the check is asked only with what a gate read out
//! of a request it admitted itself, which nothing but that gate can
//! produce, so no code in this crate or outside it can have addresses
//! pinned for a request no gate stands behind.

use core::net::{IpAddr, SocketAddr};

use gunmetal_core::net::classify;

use crate::denial::Denial;
use crate::gate::Direct;
use crate::listening::Listening;

/// The most addresses one name may resolve to.
///
/// A proposal: no requirement states a number, and neither limits type has
/// a row for it (the core's `parse::Limits` holds the media parsers' table,
/// and this crate's [`Limits`](crate::limits::Limits) the baseline's three
/// request limits). It is the number of the core's only other cap on a list
/// of addresses that comes from outside, `http::forwarded::MAX_HOPS`: room
/// for sixteen A and sixteen AAAA records, several times what a certificate
/// authority, a DNS provider or the update feed answers with.
pub const MAX_RESOLVED: usize = 32;

/// The socket addresses a request may connect to: every address its host
/// resolved to, each in its canonical form and on the destination's port,
/// in the order it was resolved. There is always at least one.
///
/// Only the check in this module makes one, and the only way to that check
/// is [`Gate::pin`](crate::gate::Gate::pin), which records what it decides.
#[derive(Debug, PartialEq, Eq)]
pub struct Pinned {
    addresses: Vec<SocketAddr>,
}

impl Pinned {
    /// The addresses, in the order the host resolved to them.
    #[must_use]
    pub fn addresses(&self) -> &[SocketAddr] {
        &self.addresses
    }
}

/// Decides which socket addresses a request may connect to, given what
/// its gate read out of it, everything its host resolved to and where the
/// server itself listens. For a host that is an address written out,
/// `resolved` is that address.
///
/// # Errors
///
/// The first of these that applies: [`Denial::NoAddress`] when `resolved`
/// is empty; [`Denial::TooManyAddresses`] when it holds more than
/// [`MAX_RESOLVED`] addresses; and, for the first address that is refused,
/// [`Denial::AddressRefused`] when the request's reach does not admit its
/// kind, or [`Denial::OwnAddress`] when the server listens on it.
pub(crate) fn pin(
    direct: &Direct,
    listening: &Listening,
    resolved: &[IpAddr],
) -> Result<Pinned, Denial> {
    if resolved.is_empty() {
        return Err(Denial::NoAddress);
    }
    if resolved.len() > MAX_RESOLVED {
        return Err(Denial::TooManyAddresses {
            resolved: resolved.len(),
        });
    }
    resolved
        .iter()
        .map(|address| {
            let address = address.to_canonical();
            let class = classify(address);
            if !direct.reach().admits(class) {
                return Err(Denial::AddressRefused { address, class });
            }
            if listening.holds(address) {
                return Err(Denial::OwnAddress { address });
            }
            Ok(SocketAddr::new(address, direct.port()))
        })
        .collect::<Result<Vec<SocketAddr>, Denial>>()
        .map(|addresses| Pinned { addresses })
}

#[cfg(test)]
mod tests {
    use super::{MAX_RESOLVED, Pinned};
    use crate::denial::Denial;
    use crate::gate::Direct;
    use crate::grant::Reach;
    use crate::listening::Listening;
    use core::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
    use gunmetal_core::net::{AddrClass, classify};
    use proptest::prelude::*;
    // Direct imports: Qodana does not resolve these macros through `prelude::*`.
    use proptest::{prop_oneof, proptest};

    /// The check, asked as a gate asks it for a request it admitted with
    /// this reach and port.
    fn pin(
        reach: Reach,
        port: u16,
        listening: &Listening,
        resolved: &[IpAddr],
    ) -> Result<Pinned, Denial> {
        super::pin(&Direct::assumed(reach, port), listening, resolved)
    }

    fn ip(text: &str) -> IpAddr {
        text.parse().expect("an address")
    }

    fn socket(text: &str) -> SocketAddr {
        text.parse().expect("a socket address")
    }

    /// What a request may connect to when these addresses passed.
    fn pinned(addresses: &[&str]) -> Pinned {
        Pinned {
            addresses: addresses.iter().copied().map(socket).collect(),
        }
    }

    /// The refusal of `address`, which is of `class`.
    fn refused(address: &str, class: AddrClass) -> Result<Pinned, Denial> {
        Err(Denial::AddressRefused {
            address: ip(address),
            class,
        })
    }

    /// The refusal of `address`, which the server listens on.
    fn own(address: &str) -> Result<Pinned, Denial> {
        Err(Denial::OwnAddress {
            address: ip(address),
        })
    }

    /// Where a server with these addresses listens.
    fn listening(addresses: &[&str]) -> Listening {
        let addresses: Vec<IpAddr> = addresses.iter().copied().map(ip).collect();
        Listening::new(&addresses).expect("somewhere to listen")
    }

    /// A home server with a public address in each family, an address on
    /// its home network in each family and an address on a tailnet.
    fn home() -> Listening {
        listening(&[
            "8.8.4.4",
            "2606:4700::1001",
            "192.168.1.10",
            "fd7a:115c:a1e0::10",
            "100.64.0.9",
        ])
    }

    /// Supports: SEC-API-077, SEC-EXT-002, SEC-HIS-023, SEC-TM-048
    #[test]
    fn a_destination_on_the_internet_connects_only_to_globally_reachable_addresses() {
        let nowhere = Listening::none();
        assert_eq!(
            pin(
                Reach::Global,
                443,
                &nowhere,
                &[ip("8.8.8.8"), ip("2606:4700::1111")]
            ),
            Ok(pinned(&["8.8.8.8:443", "[2606:4700::1111]:443"]))
        );
        let cases = [
            ("127.0.0.1", AddrClass::Loopback),
            ("169.254.169.254", AddrClass::LinkLocal),
            ("10.1.2.3", AddrClass::Private),
            ("172.16.0.1", AddrClass::Private),
            ("192.168.1.1", AddrClass::Private),
            ("100.64.0.1", AddrClass::SharedAddressSpace),
            ("0.0.0.0", AddrClass::Unspecified),
            ("224.0.0.1", AddrClass::Multicast),
            ("255.255.255.255", AddrClass::Reserved),
            ("192.0.2.1", AddrClass::Documentation),
            ("::1", AddrClass::Loopback),
            ("::", AddrClass::Unspecified),
            ("fd7a:115c:a1e0::1", AddrClass::Private),
            ("fe80::1", AddrClass::LinkLocal),
            ("ff02::1", AddrClass::Multicast),
            // The NAT64, 6to4 and Teredo forms of loopback and of the
            // metadata address.
            ("64:ff9b::7f00:1", AddrClass::Translated),
            ("64:ff9b::a9fe:a9fe", AddrClass::Translated),
            ("2002:7f00:1::1", AddrClass::Translated),
            (
                "2001:0:4136:e378:8000:63bf:3fff:fdd2",
                AddrClass::Translated,
            ),
        ];
        for (address, class) in cases {
            assert_eq!(
                pin(Reach::Global, 443, &nowhere, &[ip(address)]),
                refused(address, class),
                "{address}"
            );
        }
    }

    /// Supports: SEC-API-077, SEC-EXT-002, SEC-HIS-023
    #[test]
    fn an_ipv4_mapped_address_is_checked_and_connected_to_as_ipv4() {
        let nowhere = Listening::none();
        assert_eq!(
            pin(Reach::Global, 443, &nowhere, &[ip("::ffff:8.8.8.8")]),
            Ok(pinned(&["8.8.8.8:443"]))
        );
        assert_eq!(
            pin(Reach::Global, 443, &nowhere, &[ip("::ffff:127.0.0.1")]),
            refused("127.0.0.1", AddrClass::Loopback)
        );
        assert_eq!(
            pin(
                Reach::Global,
                443,
                &nowhere,
                &[ip("::ffff:169.254.169.254")]
            ),
            refused("169.254.169.254", AddrClass::LinkLocal)
        );
    }

    /// A name that answers with a public address and a private one is
    /// refused, whichever comes first.
    ///
    /// Supports: SEC-API-077, SEC-EXT-002, SEC-TM-048
    #[test]
    fn one_refused_address_among_the_answers_refuses_the_request() {
        let nowhere = Listening::none();
        assert_eq!(
            pin(
                Reach::Global,
                443,
                &nowhere,
                &[ip("8.8.8.8"), ip("192.168.1.1")]
            ),
            refused("192.168.1.1", AddrClass::Private)
        );
        assert_eq!(
            pin(
                Reach::Global,
                443,
                &nowhere,
                &[ip("127.0.0.1"), ip("8.8.8.8")]
            ),
            refused("127.0.0.1", AddrClass::Loopback)
        );
        assert_eq!(
            pin(
                Reach::Global,
                443,
                &nowhere,
                &[ip("8.8.8.8"), ip("10.0.0.1"), ip("127.0.0.1")]
            ),
            refused("10.0.0.1", AddrClass::Private)
        );
    }

    #[test]
    fn a_name_that_resolves_to_nothing_is_refused() {
        let nowhere = Listening::none();
        assert_eq!(
            pin(Reach::Global, 443, &nowhere, &[]),
            Err(Denial::NoAddress)
        );
        assert_eq!(pin(Reach::Lan, 8123, &nowhere, &[]), Err(Denial::NoAddress));
        assert_eq!(
            pin(Reach::Global, 443, &home(), &[]),
            Err(Denial::NoAddress)
        );
    }

    /// Supports: SEC-API-079, SEC-EXT-002
    #[test]
    fn a_lan_destination_reaches_private_addresses_and_never_loopback_or_link_local() {
        let nowhere = Listening::none();
        assert_eq!(
            pin(
                Reach::Lan,
                8123,
                &nowhere,
                &[
                    ip("192.168.1.20"),
                    ip("fd7a:115c:a1e0::20"),
                    ip("100.64.0.7")
                ]
            ),
            Ok(pinned(&[
                "192.168.1.20:8123",
                "[fd7a:115c:a1e0::20]:8123",
                "100.64.0.7:8123"
            ]))
        );
        let cases = [
            ("127.0.0.1", AddrClass::Loopback),
            ("::1", AddrClass::Loopback),
            ("169.254.169.254", AddrClass::LinkLocal),
            ("fe80::1", AddrClass::LinkLocal),
            ("0.0.0.0", AddrClass::Unspecified),
            ("224.0.0.251", AddrClass::Multicast),
            ("64:ff9b::c0a8:101", AddrClass::Translated),
            ("8.8.8.8", AddrClass::Public),
        ];
        for (address, class) in cases {
            assert_eq!(
                pin(Reach::Lan, 8123, &nowhere, &[ip(address)]),
                refused(address, class),
                "{address}"
            );
        }
        assert_eq!(
            pin(
                Reach::Lan,
                8123,
                &nowhere,
                &[ip("192.168.1.20"), ip("127.0.0.1")]
            ),
            refused("127.0.0.1", AddrClass::Loopback)
        );
    }

    /// Supports: SEC-EXT-002
    #[test]
    fn a_destination_on_the_internet_never_reaches_an_address_the_server_listens_on() {
        // The server's own public addresses, in either family and in the
        // IPv4-mapped form.
        let cases = [
            ("8.8.4.4", "8.8.4.4"),
            ("::ffff:8.8.4.4", "8.8.4.4"),
            ("2606:4700::1001", "2606:4700::1001"),
        ];
        for (answer, address) in cases {
            assert_eq!(
                pin(Reach::Global, 443, &home(), &[ip(answer)]),
                own(address),
                "{answer}"
            );
        }
        // Their neighbours are reached as before ...
        assert_eq!(
            pin(
                Reach::Global,
                443,
                &home(),
                &[ip("8.8.8.8"), ip("8.8.4.5"), ip("2606:4700::1002")]
            ),
            Ok(pinned(&[
                "8.8.8.8:443",
                "8.8.4.5:443",
                "[2606:4700::1002]:443"
            ]))
        );
        // ... and so are they, by a process that listens nowhere: with no
        // listener no address is its own.
        assert_eq!(
            pin(
                Reach::Global,
                443,
                &Listening::none(),
                &[ip("8.8.4.4"), ip("2606:4700::1001")]
            ),
            Ok(pinned(&["8.8.4.4:443", "[2606:4700::1001]:443"]))
        );
    }

    /// Supports: SEC-EXT-002
    #[test]
    fn a_lan_destination_never_reaches_an_address_the_server_listens_on() {
        // The server's own addresses on the home network and on the
        // tailnet, even for a destination an admin granted.
        let cases = [
            ("192.168.1.10", "192.168.1.10"),
            ("::ffff:192.168.1.10", "192.168.1.10"),
            ("fd7a:115c:a1e0::10", "fd7a:115c:a1e0::10"),
            ("100.64.0.9", "100.64.0.9"),
        ];
        for (answer, address) in cases {
            assert_eq!(
                pin(Reach::Lan, 8123, &home(), &[ip(answer)]),
                own(address),
                "{answer}"
            );
        }
        // Their neighbours are reached as before ...
        assert_eq!(
            pin(
                Reach::Lan,
                8123,
                &home(),
                &[
                    ip("192.168.1.11"),
                    ip("fd7a:115c:a1e0::11"),
                    ip("100.64.0.8")
                ]
            ),
            Ok(pinned(&[
                "192.168.1.11:8123",
                "[fd7a:115c:a1e0::11]:8123",
                "100.64.0.8:8123"
            ]))
        );
        // ... and so are they, by a process that listens nowhere.
        assert_eq!(
            pin(
                Reach::Lan,
                8123,
                &Listening::none(),
                &[
                    ip("192.168.1.10"),
                    ip("fd7a:115c:a1e0::10"),
                    ip("100.64.0.9")
                ]
            ),
            Ok(pinned(&[
                "192.168.1.10:8123",
                "[fd7a:115c:a1e0::10]:8123",
                "100.64.0.9:8123"
            ]))
        );
    }

    /// The rule is about the address: the server's own address is out of
    /// reach on every port, whichever port the server listens on.
    ///
    /// Supports: SEC-EXT-002
    #[test]
    fn the_servers_own_address_is_refused_on_every_port() {
        for port in [0, 80, 443, 8_080, 8_123, 65_535] {
            assert_eq!(
                pin(Reach::Global, port, &home(), &[ip("8.8.4.4")]),
                own("8.8.4.4"),
                "{port}"
            );
            assert_eq!(
                pin(Reach::Lan, port, &home(), &[ip("192.168.1.10")]),
                own("192.168.1.10"),
                "{port}"
            );
        }
    }

    /// Supports: SEC-EXT-002
    #[test]
    fn one_own_address_among_the_answers_refuses_the_request() {
        assert_eq!(
            pin(Reach::Global, 443, &home(), &[ip("8.8.8.8"), ip("8.8.4.4")]),
            own("8.8.4.4")
        );
        assert_eq!(
            pin(Reach::Global, 443, &home(), &[ip("8.8.4.4"), ip("8.8.8.8")]),
            own("8.8.4.4")
        );
        assert_eq!(
            pin(
                Reach::Lan,
                8123,
                &home(),
                &[ip("192.168.1.20"), ip("192.168.1.10")]
            ),
            own("192.168.1.10")
        );
        // The first refused answer is the one told, whichever rule
        // refuses it.
        assert_eq!(
            pin(
                Reach::Global,
                443,
                &home(),
                &[ip("8.8.4.4"), ip("10.0.0.1")]
            ),
            own("8.8.4.4")
        );
        assert_eq!(
            pin(
                Reach::Global,
                443,
                &home(),
                &[ip("10.0.0.1"), ip("8.8.4.4")]
            ),
            refused("10.0.0.1", AddrClass::Private)
        );
    }

    /// An address is told as the server's own only when its kind is one
    /// the destination may reach. Any other is refused for its kind, as it
    /// is when the server listens nowhere, and so is an address that
    /// carries an own address inside another.
    ///
    /// Supports: SEC-EXT-002
    #[test]
    fn an_address_is_refused_for_its_kind_before_it_is_refused_as_the_servers_own() {
        let everywhere = listening(&[
            "127.0.0.1",
            "::1",
            "169.254.10.10",
            "fe80::10",
            "192.168.1.10",
            "8.8.4.4",
        ]);
        let global = [
            ("8.8.4.4", own("8.8.4.4")),
            ("127.0.0.1", refused("127.0.0.1", AddrClass::Loopback)),
            ("::1", refused("::1", AddrClass::Loopback)),
            (
                "169.254.10.10",
                refused("169.254.10.10", AddrClass::LinkLocal),
            ),
            ("fe80::10", refused("fe80::10", AddrClass::LinkLocal)),
            ("192.168.1.10", refused("192.168.1.10", AddrClass::Private)),
            // The NAT64 and 6to4 forms of the server's public address.
            (
                "64:ff9b::808:404",
                refused("64:ff9b::808:404", AddrClass::Translated),
            ),
            (
                "2002:808:404::1",
                refused("2002:808:404::1", AddrClass::Translated),
            ),
        ];
        for (answer, verdict) in global {
            assert_eq!(
                pin(Reach::Global, 443, &everywhere, &[ip(answer)]),
                verdict,
                "{answer}"
            );
        }
        let lan = [
            ("192.168.1.10", own("192.168.1.10")),
            ("127.0.0.1", refused("127.0.0.1", AddrClass::Loopback)),
            ("::1", refused("::1", AddrClass::Loopback)),
            (
                "169.254.10.10",
                refused("169.254.10.10", AddrClass::LinkLocal),
            ),
            ("fe80::10", refused("fe80::10", AddrClass::LinkLocal)),
            ("8.8.4.4", refused("8.8.4.4", AddrClass::Public)),
            // The NAT64 form of the server's address on the home network.
            (
                "64:ff9b::c0a8:10a",
                refused("64:ff9b::c0a8:10a", AddrClass::Translated),
            ),
        ];
        for (answer, verdict) in lan {
            assert_eq!(
                pin(Reach::Lan, 8123, &everywhere, &[ip(answer)]),
                verdict,
                "{answer}"
            );
        }
    }

    /// Supports: SEC-EXT-002
    #[test]
    fn a_listener_given_in_its_ipv4_mapped_form_is_the_ipv4_address() {
        let mapped = listening(&["::ffff:192.168.1.10", "::ffff:8.8.4.4"]);
        assert_eq!(
            pin(Reach::Lan, 8123, &mapped, &[ip("192.168.1.10")]),
            own("192.168.1.10")
        );
        assert_eq!(
            pin(Reach::Lan, 8123, &mapped, &[ip("::ffff:192.168.1.10")]),
            own("192.168.1.10")
        );
        assert_eq!(
            pin(Reach::Global, 443, &mapped, &[ip("8.8.4.4")]),
            own("8.8.4.4")
        );
        assert_eq!(
            pin(Reach::Global, 443, &mapped, &[ip("::ffff:8.8.4.4")]),
            own("8.8.4.4")
        );
    }

    /// The addresses `8.8.8.0` to `8.8.8.<count - 1>`, all public.
    fn public_run(count: u8) -> Vec<IpAddr> {
        (0..count)
            .map(|last| IpAddr::V4(Ipv4Addr::new(8, 8, 8, last)))
            .collect()
    }

    /// The addresses `192.168.7.0` to `192.168.7.<count - 1>`, all private.
    fn private_run(count: u8) -> Vec<IpAddr> {
        (0..count)
            .map(|last| IpAddr::V4(Ipv4Addr::new(192, 168, 7, last)))
            .collect()
    }

    /// What a request to `port` may connect to when all of `addresses`
    /// passed, in their order.
    fn all(addresses: &[IpAddr], port: u16) -> Pinned {
        Pinned {
            addresses: addresses
                .iter()
                .map(|address| SocketAddr::new(*address, port))
                .collect(),
        }
    }

    #[test]
    fn a_name_may_resolve_to_thirty_two_addresses_and_no_more() {
        assert_eq!(MAX_RESOLVED, 32);
        let nowhere = Listening::none();
        // Thirty-two are all looked at and all kept.
        assert_eq!(
            pin(Reach::Global, 443, &nowhere, &public_run(32)),
            Ok(all(&public_run(32), 443))
        );
        assert_eq!(
            pin(Reach::Lan, 8123, &nowhere, &private_run(32)),
            Ok(all(&private_run(32), 8123))
        );
        // One more is refused, and so are far more.
        assert_eq!(
            pin(Reach::Global, 443, &nowhere, &public_run(33)),
            Err(Denial::TooManyAddresses { resolved: 33 })
        );
        assert_eq!(
            pin(Reach::Lan, 8123, &nowhere, &private_run(33)),
            Err(Denial::TooManyAddresses { resolved: 33 })
        );
        let flood = vec![ip("8.8.8.8"); 4_000];
        assert_eq!(
            pin(Reach::Global, 443, &nowhere, &flood),
            Err(Denial::TooManyAddresses { resolved: 4_000 })
        );
    }

    #[test]
    fn the_count_is_checked_first_and_every_address_within_it_is_looked_at() {
        let nowhere = Listening::none();
        // An address that would be refused does not change the refusal of
        // an answer that is too long ...
        let mut too_many = public_run(33);
        too_many[0] = ip("10.0.0.1");
        assert_eq!(
            pin(Reach::Global, 443, &nowhere, &too_many),
            Err(Denial::TooManyAddresses { resolved: 33 })
        );
        // ... and the thirty-second address of an answer that is not too
        // long is checked like the first.
        let mut last_private = public_run(32);
        last_private[31] = ip("10.0.0.1");
        assert_eq!(
            pin(Reach::Global, 443, &nowhere, &last_private),
            refused("10.0.0.1", AddrClass::Private)
        );
        let mut last_own = public_run(32);
        last_own[31] = ip("8.8.4.4");
        assert_eq!(pin(Reach::Global, 443, &home(), &last_own), own("8.8.4.4"));
    }

    /// Addresses of every kind: any IPv4 or IPv6 address, any IPv4-mapped
    /// one, and the ranges that random bits rarely reach.
    fn address() -> impl Strategy<Value = IpAddr> {
        prop_oneof![
            any::<u32>().prop_map(|bits| IpAddr::V4(Ipv4Addr::from(bits))),
            any::<u128>().prop_map(|bits| IpAddr::V6(Ipv6Addr::from(bits))),
            any::<u32>().prop_map(|bits| IpAddr::V6(Ipv4Addr::from(bits).to_ipv6_mapped())),
            any::<[u8; 3]>().prop_map(|[second, third, fourth]| {
                IpAddr::V4(Ipv4Addr::new(127, second, third, fourth))
            }),
            any::<[u8; 3]>().prop_map(|[second, third, fourth]| {
                IpAddr::V4(Ipv4Addr::new(10, second, third, fourth))
            }),
            any::<[u8; 2]>()
                .prop_map(|[third, fourth]| IpAddr::V4(Ipv4Addr::new(169, 254, third, fourth))),
            any::<[u16; 2]>().prop_map(|[high, low]| {
                IpAddr::V6(Ipv6Addr::new(0x64, 0xff9b, 0, 0, 0, 0, high, low))
            }),
        ]
    }

    /// Public addresses of both families and in the IPv4-mapped form: all
    /// of `8.0.0.0/8` and of `2606:4700::/32` is globally reachable.
    fn public() -> impl Strategy<Value = IpAddr> {
        prop_oneof![
            any::<[u8; 3]>().prop_map(|[second, third, fourth]| {
                IpAddr::V4(Ipv4Addr::new(8, second, third, fourth))
            }),
            any::<[u8; 3]>().prop_map(|[second, third, fourth]| {
                IpAddr::V6(Ipv4Addr::new(8, second, third, fourth).to_ipv6_mapped())
            }),
            any::<[u16; 2]>().prop_map(|[high, low]| {
                IpAddr::V6(Ipv6Addr::new(0x2606, 0x4700, 0, 0, 0, 0, high, low))
            }),
        ]
    }

    proptest! {
        /// Whatever a name resolves to, the request goes ahead only when
        /// every address is public, to exactly those addresses; otherwise
        /// the refusal names the first address that is not.
        ///
        /// Supports: SEC-API-077, SEC-EXT-002
        #[test]
        fn every_answer_must_be_public_and_every_public_answer_is_kept(
            answers in proptest::collection::vec(address(), 1..6),
            port in any::<u16>(),
        ) {
            let canonical: Vec<IpAddr> = answers.iter().map(IpAddr::to_canonical).collect();
            let first_refused = canonical
                .iter()
                .find(|one| classify(**one) != AddrClass::Public);
            let expected: Result<Pinned, Denial> = match first_refused {
                Some(address) => Err(Denial::AddressRefused {
                    address: *address,
                    class: classify(*address),
                }),
                None => Ok(all(&canonical, port)),
            };
            prop_assert_eq!(pin(Reach::Global, port, &Listening::none(), &answers), expected);
        }

        /// Whatever public addresses a name resolves to and wherever the
        /// server listens, the request goes ahead only when the server
        /// listens on none of them; otherwise the refusal names the first
        /// one it listens on.
        ///
        /// Supports: SEC-EXT-002
        #[test]
        fn no_request_connects_to_an_address_the_server_listens_on(
            answers in proptest::collection::vec(public(), 1..6),
            listens in proptest::collection::vec(any::<bool>(), 5),
            elsewhere in public(),
            port in any::<u16>(),
        ) {
            // The server listens on the answers marked for it, and on one
            // more address, so that it always listens somewhere.
            let mut listeners = vec![elsewhere];
            listeners.extend(
                answers
                    .iter()
                    .zip(&listens)
                    .filter(|(_, marked)| **marked)
                    .map(|(answer, _)| *answer),
            );
            let server = Listening::new(&listeners).expect("somewhere to listen");
            let canonical: Vec<IpAddr> = answers.iter().map(IpAddr::to_canonical).collect();
            let held: Vec<IpAddr> = listeners.iter().map(IpAddr::to_canonical).collect();
            let first_own = canonical.iter().find(|one| held.contains(one));
            let expected: Result<Pinned, Denial> = match first_own {
                Some(address) => Err(Denial::OwnAddress { address: *address }),
                None => Ok(all(&canonical, port)),
            };
            prop_assert_eq!(pin(Reach::Global, port, &server, &answers), expected);
        }
    }
}
