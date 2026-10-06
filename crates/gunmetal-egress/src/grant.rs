//! What the owner granted, and the decision it leads to.
//!
//! A [`Configuration`] is everything the owner has decided about egress.
//! The default grants nothing: a server that was only installed makes no
//! outbound connection at all (SEC-TM-048, SEC-PRV-013). Configuring HTTPS
//! for a domain the owner controls grants certificate issuance, which is
//! also the one purpose that may be used before the server is claimed.
//! Answering yes to the first-run question grants the update feed. Offline
//! mode refuses everything, whatever was granted (SEC-PRV-012).
//!
//! A grant names its destinations exactly, as scheme, host and port
//! (SEC-API-079). A destination on the internet is always HTTPS and may
//! resolve only to globally reachable addresses. A destination on the home
//! network is one an admin granted, as this exact scheme, host and port. It
//! alone may use plain HTTP, and it may resolve only to the private ranges
//! of a home network or a tailnet, never to loopback or link-local
//! addresses (SEC-EXT-002, SEC-EXT-004).
//!
//! When requests leave through the admin's proxy, the proxy resolves the
//! names, so no address can be checked here. Only a destination on the
//! internet that is named by host goes through it; an address written out
//! and a LAN destination are refused (SEC-PRV-012).
//!
//! [`Configuration::decide`] is the whole decision, as one pure function.

use gunmetal_core::net::AddrClass;

use crate::denial::Denial;
use crate::destination::{Destination, Host, Scheme};
use crate::purpose::{Purpose, Redirects};

/// The host of the project's update and advisory feed.
pub const UPDATE_FEED_HOST: &str = "gunmetal.tv";

/// The port of HTTPS.
const HTTPS_PORT: u16 = 443;

/// Which addresses a granted destination may resolve to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    /// Only globally reachable addresses: nothing in the IANA
    /// special-purpose registries, and nothing that carries another
    /// address.
    Global,
    /// Only the private ranges of a home network or a tailnet: RFC 1918,
    /// unique-local IPv6 and the shared address space. Never loopback,
    /// link-local or any other special range.
    Lan,
}

impl Reach {
    /// Whether an address of `class` may be connected to.
    #[must_use]
    pub fn admits(self, class: AddrClass) -> bool {
        match self {
            Self::Global => class == AddrClass::Public,
            Self::Lan => matches!(class, AddrClass::Private | AddrClass::SharedAddressSpace),
        }
    }
}

/// One destination a purpose may reach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Allowed {
    destination: Destination,
    reach: Reach,
}

impl Allowed {
    /// A destination on the internet: HTTPS to `host` on `port`, at
    /// globally reachable addresses only.
    #[must_use]
    pub fn public(host: Host, port: u16) -> Self {
        Self {
            destination: Destination {
                scheme: Scheme::Https,
                host,
                port,
            },
            reach: Reach::Global,
        }
    }

    /// A destination on the home network that an admin granted: exactly
    /// this scheme, host and port, at private addresses only.
    #[must_use]
    pub fn lan(scheme: Scheme, host: Host, port: u16) -> Self {
        Self {
            destination: Destination { scheme, host, port },
            reach: Reach::Lan,
        }
    }
}

/// How requests leave the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Route {
    /// The server resolves the name, checks every address and connects to
    /// one it checked.
    #[default]
    Direct,
    /// Everything goes through the admin's proxy, which resolves the name.
    Proxy,
}

/// What the owner has decided about egress.
///
/// The default is a server that is not claimed, reaches out directly and
/// has been granted nothing.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Configuration {
    claimed: bool,
    offline: bool,
    route: Route,
    acme: Option<Vec<Allowed>>,
    update_feed: Option<Vec<Allowed>>,
}

impl Configuration {
    /// The server has been claimed, so purposes other than certificate
    /// issuance may be used.
    #[must_use]
    pub fn claimed(self) -> Self {
        Self {
            claimed: true,
            ..self
        }
    }

    /// Offline mode is on: every request is refused.
    #[must_use]
    pub fn offline(self) -> Self {
        Self {
            offline: true,
            ..self
        }
    }

    /// Requests leave through the admin's proxy.
    #[must_use]
    pub fn through_proxy(self) -> Self {
        Self {
            route: Route::Proxy,
            ..self
        }
    }

    /// The owner configured HTTPS for a domain they control, which grants
    /// certificate issuance: the certificate authority and the API of the
    /// owner's DNS provider, and nothing else.
    #[must_use]
    pub fn own_domain_https(self, authority: Allowed, dns_provider: Allowed) -> Self {
        Self {
            acme: Some(vec![authority, dns_provider]),
            ..self
        }
    }

    /// The owner said yes to the update check, which grants the update
    /// feed: HTTPS to [`UPDATE_FEED_HOST`], and nothing else.
    #[must_use]
    pub fn update_feed(self) -> Self {
        let feed = Host::parse(UPDATE_FEED_HOST)
            .ok()
            .map(|host| vec![Allowed::public(host, HTTPS_PORT)]);
        Self {
            update_feed: feed,
            ..self
        }
    }

    /// The purposes that have been granted, in the order of the inventory.
    #[must_use]
    pub fn granted(&self) -> Vec<Purpose> {
        Purpose::ALL
            .into_iter()
            .filter(|&purpose| self.destinations(purpose).is_some())
            .collect()
    }

    /// The destinations `purpose` was granted, if it was granted.
    fn destinations(&self, purpose: Purpose) -> Option<&[Allowed]> {
        match purpose {
            Purpose::Acme => self.acme.as_deref(),
            Purpose::UpdateFeed => self.update_feed.as_deref(),
        }
    }

    /// Decides whether `purpose` may send a request to `destination`.
    ///
    /// # Errors
    ///
    /// The first of these that applies: [`Denial::Offline`] in offline
    /// mode; [`Denial::BeforeClaim`] when the server is not claimed and the
    /// purpose must wait for the claim; [`Denial::PurposeNotGranted`];
    /// [`Denial::DestinationNotGranted`] when the grant does not name this
    /// scheme, host and port; and [`Denial::NotThroughProxy`] when requests
    /// leave through a proxy and the destination is an address or a LAN
    /// destination.
    pub fn decide(&self, purpose: Purpose, destination: &Destination) -> Result<Admitted, Denial> {
        let rules = purpose.rules();
        if self.offline {
            return Err(Denial::Offline);
        }
        if !self.claimed && !rules.before_claim {
            return Err(Denial::BeforeClaim);
        }
        let granted = self
            .destinations(purpose)
            .ok_or(Denial::PurposeNotGranted)?;
        let allowed = granted
            .iter()
            .find(|candidate| candidate.destination == *destination)
            .ok_or(Denial::DestinationNotGranted)?;
        let direct_only =
            allowed.reach == Reach::Lan || matches!(destination.host, Host::Address(_));
        if self.route == Route::Proxy && direct_only {
            return Err(Denial::NotThroughProxy);
        }
        Ok(Admitted {
            purpose,
            destination: destination.clone(),
            reach: allowed.reach,
            route: self.route,
            redirects: rules.redirects,
            followed: 0,
        })
    }
}

/// A request the configuration lets out: which purpose, to which
/// destination, at which addresses and by which route, and how many
/// redirects led to it.
///
/// Only [`Configuration::decide`] makes one, and only the gate moves one
/// on along a redirect, so no other code can write one or say how many
/// redirects it followed. It cannot be copied: following a redirect uses
/// the request up, so its count cannot be started again from an earlier
/// copy.
#[derive(Debug, PartialEq, Eq)]
pub struct Admitted {
    /// The purpose the request names.
    purpose: Purpose,
    /// Where the request goes.
    destination: Destination,
    /// Which addresses the destination may resolve to.
    reach: Reach,
    /// How the request leaves.
    route: Route,
    /// What happens to a redirect.
    redirects: Redirects,
    /// How many redirects the request has followed to get here.
    followed: u8,
}

impl Admitted {
    /// The purpose the request names.
    pub(crate) fn purpose(&self) -> Purpose {
        self.purpose
    }

    /// Where the request goes.
    pub(crate) fn destination(&self) -> &Destination {
        &self.destination
    }

    /// Which addresses the destination may resolve to.
    pub(crate) fn reach(&self) -> Reach {
        self.reach
    }

    /// How the request leaves.
    pub(crate) fn route(&self) -> Route {
        self.route
    }

    /// What happens to a redirect.
    pub(crate) fn redirects(&self) -> Redirects {
        self.redirects
    }

    /// How many redirects the request has followed to get here.
    pub(crate) fn followed(&self) -> u8 {
        self.followed
    }

    /// The request this one becomes by following a redirect, given `hop`,
    /// what the configuration decided for the redirect's target: `hop`,
    /// with one more redirect behind it than this request has.
    pub(crate) fn followed_to(self, hop: Self) -> Self {
        // Not yet counted.
        let _ = self.followed;
        hop
    }
}

#[cfg(test)]
impl Admitted {
    /// A request as the configuration would let it out, for the tests of
    /// what no R1 purpose does yet: following a redirect, and having
    /// followed some already. The build outside tests has no such door.
    pub(crate) fn assumed(
        purpose: Purpose,
        destination: Destination,
        reach: Reach,
        route: Route,
        redirects: Redirects,
        followed: u8,
    ) -> Self {
        Self {
            purpose,
            destination,
            reach,
            route,
            redirects,
            followed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Admitted, Allowed, Configuration, Reach, Route, UPDATE_FEED_HOST};
    use crate::denial::Denial;
    use crate::destination::{Destination, Host, Scheme};
    use crate::purpose::{Purpose, Redirects};
    use gunmetal_core::net::AddrClass;

    fn host(text: &str) -> Host {
        Host::parse(text).expect("a host")
    }

    fn https(text: &str) -> Destination {
        Destination {
            scheme: Scheme::Https,
            host: host(text),
            port: 443,
        }
    }

    /// What is let out directly to a destination on the internet.
    fn direct(purpose: Purpose, destination: Destination) -> Admitted {
        Admitted {
            purpose,
            destination,
            reach: Reach::Global,
            route: Route::Direct,
            redirects: Redirects::Refused,
            followed: 0,
        }
    }

    /// An owner who configured HTTPS for a domain of their own.
    fn own_domain() -> Configuration {
        Configuration::default().own_domain_https(
            Allowed::public(host("acme.example.org"), 443),
            Allowed::public(host("api.dns.example"), 443),
        )
    }

    /// A DNS server on the home network, reached over plain HTTP.
    fn lan_dns() -> Destination {
        Destination {
            scheme: Scheme::Http,
            host: host("dns.home.arpa"),
            port: 8081,
        }
    }

    /// Verifies: SEC-TM-075, SEC-PRV-013
    /// Supports: SEC-TM-048
    #[test]
    fn the_default_configuration_grants_no_purpose() {
        let default = Configuration::default();
        assert_eq!(default.granted(), Vec::<Purpose>::new());
        assert_eq!(
            default.decide(Purpose::Acme, &https("acme.example.org")),
            Err(Denial::PurposeNotGranted)
        );
        let claimed = default.claimed();
        for purpose in Purpose::ALL {
            assert_eq!(
                claimed.decide(purpose, &https("gunmetal.tv")),
                Err(Denial::PurposeNotGranted),
                "{purpose:?}"
            );
        }
    }

    /// Supports: SEC-TM-048, SEC-API-079
    #[test]
    fn configuring_an_own_domain_grants_exactly_certificate_issuance() {
        let configuration = own_domain();
        assert_eq!(configuration.granted(), [Purpose::Acme]);
        for name in ["acme.example.org", "api.dns.example"] {
            assert_eq!(
                configuration.decide(Purpose::Acme, &https(name)),
                Ok(direct(Purpose::Acme, https(name))),
                "{name}"
            );
        }
        assert_eq!(
            configuration.decide(Purpose::Acme, &https("gunmetal.tv")),
            Err(Denial::DestinationNotGranted)
        );
        assert_eq!(
            configuration
                .claimed()
                .decide(Purpose::UpdateFeed, &https("gunmetal.tv")),
            Err(Denial::PurposeNotGranted)
        );
    }

    /// Supports: SEC-API-078, SEC-API-079, SEC-EXT-004, SEC-PRV-008
    #[test]
    fn the_update_feed_reaches_only_the_project_feed_over_https() {
        assert_eq!(UPDATE_FEED_HOST, "gunmetal.tv");
        let configuration = Configuration::default().claimed().update_feed();
        assert_eq!(configuration.granted(), [Purpose::UpdateFeed]);
        assert_eq!(
            configuration.decide(Purpose::UpdateFeed, &https("gunmetal.tv")),
            Ok(direct(Purpose::UpdateFeed, https("gunmetal.tv")))
        );
        let refused = [
            https("www.gunmetal.tv"),
            https("gunmetal.tv.example"),
            https("tracker.example"),
            Destination {
                port: 8443,
                ..https("gunmetal.tv")
            },
            Destination {
                scheme: Scheme::Http,
                ..https("gunmetal.tv")
            },
            Destination {
                scheme: Scheme::Http,
                host: host("gunmetal.tv"),
                port: 80,
            },
        ];
        for destination in refused {
            assert_eq!(
                configuration.decide(Purpose::UpdateFeed, &destination),
                Err(Denial::DestinationNotGranted),
                "{destination:?}"
            );
        }
        assert_eq!(
            configuration.decide(Purpose::Acme, &https("gunmetal.tv")),
            Err(Denial::PurposeNotGranted)
        );
    }

    #[test]
    fn only_certificate_issuance_may_be_used_before_the_claim() {
        let unclaimed = own_domain().update_feed();
        assert_eq!(unclaimed.granted(), [Purpose::Acme, Purpose::UpdateFeed]);
        assert_eq!(
            unclaimed.decide(Purpose::Acme, &https("acme.example.org")),
            Ok(direct(Purpose::Acme, https("acme.example.org")))
        );
        assert_eq!(
            unclaimed.decide(Purpose::UpdateFeed, &https("gunmetal.tv")),
            Err(Denial::BeforeClaim)
        );
        let claimed = unclaimed.claimed();
        assert_eq!(
            claimed.decide(Purpose::Acme, &https("acme.example.org")),
            Ok(direct(Purpose::Acme, https("acme.example.org")))
        );
        assert_eq!(
            claimed.decide(Purpose::UpdateFeed, &https("gunmetal.tv")),
            Ok(direct(Purpose::UpdateFeed, https("gunmetal.tv")))
        );
    }

    /// Supports: SEC-PRV-012
    #[test]
    fn offline_mode_refuses_every_purpose() {
        let offline = own_domain().update_feed().claimed().offline();
        // What was granted stays granted for the day offline mode ends.
        assert_eq!(offline.granted(), [Purpose::Acme, Purpose::UpdateFeed]);
        assert_eq!(
            offline.decide(Purpose::Acme, &https("acme.example.org")),
            Err(Denial::Offline)
        );
        assert_eq!(
            offline.decide(Purpose::UpdateFeed, &https("gunmetal.tv")),
            Err(Denial::Offline)
        );
        // Offline is the answer even where another refusal would apply.
        assert_eq!(
            Configuration::default()
                .offline()
                .decide(Purpose::UpdateFeed, &https("tracker.example")),
            Err(Denial::Offline)
        );
    }

    /// Supports: SEC-API-078, SEC-API-079, SEC-EXT-002, SEC-EXT-004
    #[test]
    fn a_lan_destination_is_one_exact_scheme_host_and_port() {
        let configuration = Configuration::default().own_domain_https(
            Allowed::public(host("acme.example.org"), 443),
            Allowed::lan(Scheme::Http, host("dns.home.arpa"), 8081),
        );
        assert_eq!(
            configuration.decide(Purpose::Acme, &lan_dns()),
            Ok(Admitted {
                purpose: Purpose::Acme,
                destination: lan_dns(),
                reach: Reach::Lan,
                route: Route::Direct,
                redirects: Redirects::Refused,
                followed: 0,
            })
        );
        let refused = [
            Destination {
                scheme: Scheme::Https,
                ..lan_dns()
            },
            Destination {
                port: 80,
                ..lan_dns()
            },
            Destination {
                host: host("router.home.arpa"),
                ..lan_dns()
            },
            // Plain HTTP stays refused for the destination on the internet.
            Destination {
                scheme: Scheme::Http,
                ..https("acme.example.org")
            },
            Destination {
                scheme: Scheme::Http,
                host: host("acme.example.org"),
                port: 80,
            },
        ];
        for destination in refused {
            assert_eq!(
                configuration.decide(Purpose::Acme, &destination),
                Err(Denial::DestinationNotGranted),
                "{destination:?}"
            );
        }
        assert_eq!(
            configuration.decide(Purpose::Acme, &https("acme.example.org")),
            Ok(direct(Purpose::Acme, https("acme.example.org")))
        );
    }

    /// Supports: SEC-API-077, SEC-API-079, SEC-EXT-002
    #[test]
    fn a_destination_on_the_internet_admits_public_addresses_and_a_lan_destination_private_ones() {
        let cases = [
            (AddrClass::Public, true, false),
            (AddrClass::Private, false, true),
            (AddrClass::SharedAddressSpace, false, true),
            (AddrClass::Loopback, false, false),
            (AddrClass::LinkLocal, false, false),
            (AddrClass::Unspecified, false, false),
            (AddrClass::Multicast, false, false),
            (AddrClass::Documentation, false, false),
            (AddrClass::Translated, false, false),
            (AddrClass::Reserved, false, false),
        ];
        for (class, global, lan) in cases {
            assert_eq!(
                (Reach::Global.admits(class), Reach::Lan.admits(class)),
                (global, lan),
                "{class:?}"
            );
        }
    }

    /// Supports: SEC-PRV-012
    #[test]
    fn through_a_proxy_only_a_destination_on_the_internet_named_by_host_goes_out() {
        let configuration = Configuration::default()
            .own_domain_https(
                Allowed::public(host("192.0.2.53"), 443),
                Allowed::lan(Scheme::Http, host("dns.home.arpa"), 8081),
            )
            .update_feed()
            .claimed();
        // Directly, each of the three is let out.
        assert_eq!(
            configuration.decide(Purpose::Acme, &https("192.0.2.53")),
            Ok(direct(Purpose::Acme, https("192.0.2.53")))
        );
        assert_eq!(
            configuration
                .decide(Purpose::Acme, &lan_dns())
                .map(|admitted| admitted.reach),
            Ok(Reach::Lan)
        );
        assert_eq!(
            configuration.decide(Purpose::UpdateFeed, &https("gunmetal.tv")),
            Ok(direct(Purpose::UpdateFeed, https("gunmetal.tv")))
        );
        let proxied = configuration.through_proxy();
        assert_eq!(
            proxied.decide(Purpose::UpdateFeed, &https("gunmetal.tv")),
            Ok(Admitted {
                purpose: Purpose::UpdateFeed,
                destination: https("gunmetal.tv"),
                reach: Reach::Global,
                route: Route::Proxy,
                redirects: Redirects::Refused,
                followed: 0,
            })
        );
        assert_eq!(
            proxied.decide(Purpose::Acme, &https("192.0.2.53")),
            Err(Denial::NotThroughProxy)
        );
        assert_eq!(
            proxied.decide(Purpose::Acme, &lan_dns()),
            Err(Denial::NotThroughProxy)
        );
        assert_eq!(
            proxied.decide(Purpose::UpdateFeed, &https("tracker.example")),
            Err(Denial::DestinationNotGranted)
        );
    }
}
