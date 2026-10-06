//! Where the server itself listens.
//!
//! No request may connect to an address the server listens on (SEC-EXT-002,
//! and the deny set in docs/security/plugins-and-integrations-security.md).
//! A granted name that resolves to the server's own address would turn a
//! request on the server and on whatever else answers at that address, so
//! the address is out of reach on every port, whichever port the server
//! listens on, as loopback is.
//!
//! This crate opens no socket and reads no interface, so whoever builds the
//! gate tells it where the server listens, as a [`Listening`]:
//!
//! - An address is kept and compared in its canonical form, so a listener
//!   given as `::ffff:192.168.1.10` and a name that resolves to
//!   `192.168.1.10` are the same address, and so the other way round.
//! - A listener bound to a wildcard address, `0.0.0.0` or `::`, takes
//!   connections on every address the machine holds, and those cannot be
//!   read from the wildcard. A wildcard is therefore refused: in its place
//!   the caller passes every address of the machine's interfaces.
//! - An empty list is refused as well, because a list that was read from
//!   the machine and came back empty has more likely failed than found a
//!   server without a listener. A process that listens on no address says
//!   so with [`Listening::none`].

use core::net::IpAddr;
use std::collections::BTreeSet;

/// Why a list of addresses does not say where a server listens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListeningError {
    /// The list is empty. [`Listening::none`] is how a process that listens
    /// on no address says so.
    Empty,
    /// The list holds a wildcard address, which stands for every address
    /// of the machine and names none of them.
    Wildcard {
        /// The wildcard, in its canonical form.
        address: IpAddr,
    },
}

/// The addresses the server itself listens on, each in its canonical form
/// and without a port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listening(BTreeSet<IpAddr>);

impl Listening {
    /// A process that listens on no address, so that no address is its
    /// own.
    #[must_use]
    pub fn none() -> Self {
        Self(BTreeSet::new())
    }

    /// A server that listens on `addresses`, in any order: the address of
    /// each listener bound to one address, and every address of the
    /// machine's interfaces if a listener is bound to a wildcard.
    ///
    /// # Errors
    ///
    /// [`ListeningError::Empty`] for an empty list, and
    /// [`ListeningError::Wildcard`] with the first wildcard address in it,
    /// `0.0.0.0`, `::` or `::ffff:0.0.0.0`.
    pub fn new(addresses: &[IpAddr]) -> Result<Self, ListeningError> {
        if addresses.is_empty() {
            return Err(ListeningError::Empty);
        }
        addresses
            .iter()
            .map(|address| {
                let address = address.to_canonical();
                if address.is_unspecified() {
                    Err(ListeningError::Wildcard { address })
                } else {
                    Ok(address)
                }
            })
            .collect::<Result<BTreeSet<IpAddr>, ListeningError>>()
            .map(Self)
    }

    /// Whether the server listens on `address`, in whichever form it is
    /// written.
    #[must_use]
    pub fn holds(&self, address: IpAddr) -> bool {
        self.0.contains(&address.to_canonical())
    }
}

#[cfg(test)]
mod tests {
    use super::{Listening, ListeningError};
    use core::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use proptest::prelude::*;
    // Direct imports: Qodana does not resolve these macros through `prelude::*`.
    use proptest::{prop_oneof, proptest};

    fn ip(text: &str) -> IpAddr {
        text.parse().expect("an address")
    }

    /// Where a server with these addresses listens.
    fn listening(addresses: &[&str]) -> Result<Listening, ListeningError> {
        let addresses: Vec<IpAddr> = addresses.iter().copied().map(ip).collect();
        Listening::new(&addresses)
    }

    /// Supports: SEC-EXT-002
    #[test]
    fn the_server_listens_on_the_addresses_it_was_given_and_on_no_other() {
        let home = listening(&[
            "192.168.1.10",
            "8.8.4.4",
            "fd7a:115c:a1e0::10",
            "100.64.0.9",
            "127.0.0.1",
        ])
        .expect("somewhere to listen");
        let cases = [
            ("192.168.1.10", true),
            ("8.8.4.4", true),
            ("fd7a:115c:a1e0::10", true),
            ("100.64.0.9", true),
            ("127.0.0.1", true),
            // The neighbours of each, and the other family's loopback.
            ("192.168.1.11", false),
            ("192.168.1.9", false),
            ("8.8.8.8", false),
            ("fd7a:115c:a1e0::11", false),
            ("100.64.0.10", false),
            ("127.0.0.2", false),
            ("::1", false),
        ];
        for (address, own) in cases {
            assert_eq!(home.holds(ip(address)), own, "{address}");
        }
    }

    /// Supports: SEC-EXT-002
    #[test]
    fn an_address_and_its_ipv4_mapped_form_are_one_address() {
        let plain = listening(&["192.168.1.10"]).expect("somewhere to listen");
        let mapped = listening(&["::ffff:192.168.1.10"]).expect("somewhere to listen");
        assert_eq!(plain, mapped);
        for home in [plain, mapped] {
            assert!(home.holds(ip("192.168.1.10")));
            assert!(home.holds(ip("::ffff:192.168.1.10")));
            assert!(home.holds(ip("::ffff:c0a8:10a")));
            // An address that carries this one is another address, which
            // the address rule refuses for what it is.
            assert!(!home.holds(ip("64:ff9b::c0a8:10a")));
            assert!(!home.holds(ip("2002:c0a8:10a::1")));
            assert!(!home.holds(ip("::c0a8:10a")));
        }
    }

    /// Supports: SEC-EXT-002
    #[test]
    fn a_wildcard_address_does_not_say_where_a_server_listens() {
        let any_v4 = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
        let any_v6 = IpAddr::V6(Ipv6Addr::UNSPECIFIED);
        let cases: [(&[&str], IpAddr); 7] = [
            (&["0.0.0.0"], any_v4),
            (&["::"], any_v6),
            (&["::ffff:0.0.0.0"], any_v4),
            (&["192.168.1.10", "0.0.0.0"], any_v4),
            (&["::", "192.168.1.10"], any_v6),
            (&["192.168.1.10", "::", "0.0.0.0"], any_v6),
            (&["0.0.0.0", "::"], any_v4),
        ];
        for (addresses, address) in cases {
            assert_eq!(
                listening(addresses),
                Err(ListeningError::Wildcard { address }),
                "{addresses:?}"
            );
        }
        // Only the wildcard itself stands for every address: its neighbour
        // is an address like any other.
        let neighbour = listening(&["0.0.0.1"]).expect("one address");
        assert!(neighbour.holds(ip("0.0.0.1")));
        assert!(!neighbour.holds(ip("0.0.0.0")));
    }

    #[test]
    fn an_empty_list_is_refused_and_a_process_without_a_listener_says_so() {
        assert_eq!(Listening::new(&[]), Err(ListeningError::Empty));
        let nowhere = Listening::none();
        for address in [
            "192.168.1.10",
            "8.8.4.4",
            "127.0.0.1",
            "::1",
            "0.0.0.0",
            "::",
        ] {
            assert!(!nowhere.holds(ip(address)), "{address}");
        }
        assert_ne!(Ok(nowhere), listening(&["192.168.1.10"]));
    }

    #[test]
    fn the_order_of_the_list_and_its_repeats_do_not_matter() {
        assert_eq!(
            listening(&["192.168.1.10", "8.8.4.4"]),
            listening(&["8.8.4.4", "192.168.1.10", "::ffff:8.8.4.4", "8.8.4.4"])
        );
        assert_ne!(
            listening(&["192.168.1.10", "8.8.4.4"]),
            listening(&["192.168.1.10"])
        );
    }

    /// Addresses of both families and IPv4-mapped ones, none of them a
    /// wildcard.
    fn address() -> impl Strategy<Value = IpAddr> {
        prop_oneof![
            any::<u32>().prop_map(|bits| IpAddr::V4(Ipv4Addr::from(bits))),
            any::<u128>().prop_map(|bits| IpAddr::V6(Ipv6Addr::from(bits))),
            any::<u32>().prop_map(|bits| IpAddr::V6(Ipv4Addr::from(bits).to_ipv6_mapped())),
        ]
        .prop_filter("a wildcard", |one| !one.to_canonical().is_unspecified())
    }

    proptest! {
        /// The server listens on every address of its list, in the form
        /// given and in the IPv4-mapped form, and on another address only
        /// if the list holds that one too.
        ///
        /// Supports: SEC-EXT-002
        #[test]
        fn the_list_decides_which_addresses_are_the_servers_own(
            addresses in proptest::collection::vec(address(), 1..6),
            other in address(),
        ) {
            let home = Listening::new(&addresses).expect("somewhere to listen");
            let canonical: Vec<IpAddr> = addresses.iter().map(IpAddr::to_canonical).collect();
            for one in &addresses {
                prop_assert!(home.holds(*one));
                prop_assert!(home.holds(one.to_canonical()));
            }
            prop_assert_eq!(home.holds(other), canonical.contains(&other.to_canonical()));
        }
    }
}
