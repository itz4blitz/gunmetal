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

use core::net::{IpAddr, SocketAddr};

use gunmetal_core::net::classify;

use crate::denial::Denial;
use crate::grant::Reach;

/// The socket addresses a request may connect to, given everything its
/// host resolved to: every address in its canonical form, on `port`, in
/// the order of `resolved`. For a host that is an address written out,
/// `resolved` is that address.
///
/// # Errors
///
/// [`Denial::NoAddress`] when `resolved` is empty, and
/// [`Denial::AddressRefused`] with the first address `reach` does not
/// admit.
pub fn pin(reach: Reach, port: u16, resolved: &[IpAddr]) -> Result<Vec<SocketAddr>, Denial> {
    if resolved.is_empty() {
        return Err(Denial::NoAddress);
    }
    resolved
        .iter()
        .map(|address| {
            let address = address.to_canonical();
            let class = classify(address);
            if reach.admits(class) {
                Ok(SocketAddr::new(address, port))
            } else {
                Err(Denial::AddressRefused { address, class })
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::pin;
    use crate::denial::Denial;
    use crate::grant::Reach;
    use core::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
    use gunmetal_core::net::{AddrClass, classify};
    use proptest::prelude::*;

    fn ip(text: &str) -> IpAddr {
        text.parse().expect("an address")
    }

    fn socket(text: &str) -> SocketAddr {
        text.parse().expect("a socket address")
    }

    /// The refusal of `address`, which is of `class`.
    fn refused(address: &str, class: AddrClass) -> Result<Vec<SocketAddr>, Denial> {
        Err(Denial::AddressRefused {
            address: ip(address),
            class,
        })
    }

    /// Verifies: SEC-API-077, SEC-EXT-002, SEC-HIS-023, SEC-TM-048
    #[test]
    fn a_destination_on_the_internet_connects_only_to_globally_reachable_addresses() {
        assert_eq!(
            pin(Reach::Global, 443, &[ip("8.8.8.8"), ip("2606:4700::1111")]),
            Ok(vec![socket("8.8.8.8:443"), socket("[2606:4700::1111]:443")])
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
                pin(Reach::Global, 443, &[ip(address)]),
                refused(address, class),
                "{address}"
            );
        }
    }

    /// Verifies: SEC-API-077, SEC-EXT-002, SEC-HIS-023
    #[test]
    fn an_ipv4_mapped_address_is_checked_and_connected_to_as_ipv4() {
        assert_eq!(
            pin(Reach::Global, 443, &[ip("::ffff:8.8.8.8")]),
            Ok(vec![socket("8.8.8.8:443")])
        );
        assert_eq!(
            pin(Reach::Global, 443, &[ip("::ffff:127.0.0.1")]),
            refused("127.0.0.1", AddrClass::Loopback)
        );
        assert_eq!(
            pin(Reach::Global, 443, &[ip("::ffff:169.254.169.254")]),
            refused("169.254.169.254", AddrClass::LinkLocal)
        );
    }

    /// A name that answers with a public address and a private one is
    /// refused, whichever comes first.
    ///
    /// Verifies: SEC-API-077, SEC-EXT-002, SEC-TM-048
    #[test]
    fn one_refused_address_among_the_answers_refuses_the_request() {
        assert_eq!(
            pin(Reach::Global, 443, &[ip("8.8.8.8"), ip("192.168.1.1")]),
            refused("192.168.1.1", AddrClass::Private)
        );
        assert_eq!(
            pin(Reach::Global, 443, &[ip("127.0.0.1"), ip("8.8.8.8")]),
            refused("127.0.0.1", AddrClass::Loopback)
        );
        assert_eq!(
            pin(
                Reach::Global,
                443,
                &[ip("8.8.8.8"), ip("10.0.0.1"), ip("127.0.0.1")]
            ),
            refused("10.0.0.1", AddrClass::Private)
        );
    }

    #[test]
    fn a_name_that_resolves_to_nothing_is_refused() {
        assert_eq!(pin(Reach::Global, 443, &[]), Err(Denial::NoAddress));
        assert_eq!(pin(Reach::Lan, 8123, &[]), Err(Denial::NoAddress));
    }

    /// Verifies: SEC-API-079, SEC-EXT-002
    #[test]
    fn a_lan_destination_reaches_private_addresses_and_never_loopback_or_link_local() {
        assert_eq!(
            pin(
                Reach::Lan,
                8123,
                &[
                    ip("192.168.1.20"),
                    ip("fd7a:115c:a1e0::20"),
                    ip("100.64.0.7")
                ]
            ),
            Ok(vec![
                socket("192.168.1.20:8123"),
                socket("[fd7a:115c:a1e0::20]:8123"),
                socket("100.64.0.7:8123")
            ])
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
                pin(Reach::Lan, 8123, &[ip(address)]),
                refused(address, class),
                "{address}"
            );
        }
        assert_eq!(
            pin(Reach::Lan, 8123, &[ip("192.168.1.20"), ip("127.0.0.1")]),
            refused("127.0.0.1", AddrClass::Loopback)
        );
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

    proptest! {
        /// Whatever a name resolves to, the request goes ahead only when
        /// every address is public, to exactly those addresses; otherwise
        /// the refusal names the first address that is not.
        ///
        /// Verifies: SEC-API-077, SEC-EXT-002
        #[test]
        fn every_answer_must_be_public_and_every_public_answer_is_kept(
            answers in proptest::collection::vec(address(), 1..6),
            port in any::<u16>(),
        ) {
            let canonical: Vec<IpAddr> = answers.iter().map(IpAddr::to_canonical).collect();
            let first_refused = canonical
                .iter()
                .find(|one| classify(**one) != AddrClass::Public);
            let expected: Result<Vec<SocketAddr>, Denial> = match first_refused {
                Some(address) => Err(Denial::AddressRefused {
                    address: *address,
                    class: classify(*address),
                }),
                None => Ok(canonical
                    .iter()
                    .map(|one| SocketAddr::new(*one, port))
                    .collect()),
            };
            prop_assert_eq!(pin(Reach::Global, port, &answers), expected);
        }
    }
}
