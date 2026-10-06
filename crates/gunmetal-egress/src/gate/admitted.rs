//! A request a gate admitted, sealed for that gate.
//!
//! An [`Admitted`] request is made in this module and nowhere else, by the
//! gate whose configuration decided for it, and it carries that gate's
//! identity. Its terms (the addresses it may reach, the route it takes,
//! what happens to a redirect and how many it has followed) can be read
//! only by presenting a gate's identity, and are told only to the gate that
//! admitted the request. Any other gate is answered
//! [`Denial::AdmittedElsewhere`].
//!
//! The gate's own code has no other way to those terms. So nothing it does
//! now, and nothing it gains later, can act on a request that another gate
//! admitted under another configuration: there is no check a caller could
//! leave out.
//!
//! What a request says about itself, its purpose and its destination, is
//! readable without an identity, because a refusal is recorded under them.

use core::sync::atomic::{AtomicU64, Ordering};

use crate::denial::Denial;
use crate::destination::Destination;
use crate::grant::{Decision, Reach, Route};
use crate::purpose::{Purpose, Redirects};

/// How many gates this process has made, which numbers the next one.
static GATES: AtomicU64 = AtomicU64::new(0);

/// Which gate admitted a request: a number no two gates of one process
/// share. Only [`Issuer::fresh`] makes one, and only a gate holds one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Issuer(u64);

impl Issuer {
    /// The identity of a new gate.
    pub(super) fn fresh() -> Self {
        Self(GATES.fetch_add(1, Ordering::Relaxed))
    }
}

/// What the address check needs to know about a request that leaves
/// directly: which addresses its destination may resolve to, and its port.
///
/// Only [`Admitted::direct`] makes one, for the gate that admitted the
/// request. The address check takes nothing else, so it cannot be asked
/// about a request no gate stands behind.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Direct {
    reach: Reach,
    port: u16,
}

impl Direct {
    /// Which addresses the destination may resolve to.
    pub(crate) fn reach(&self) -> Reach {
        self.reach
    }

    /// The port the request connects to.
    pub(crate) fn port(&self) -> u16 {
        self.port
    }
}

#[cfg(test)]
impl Direct {
    /// The terms of a request as if a gate had admitted it, for the tests
    /// of the address check, which ask it without a gate. The build
    /// outside tests has no such door.
    pub(crate) fn assumed(reach: Reach, port: u16) -> Self {
        Self { reach, port }
    }
}

/// A request a gate lets out: which purpose, to which destination, at
/// which addresses and by which route, and how many redirects led to it.
///
/// Only the gate whose configuration decided for the request makes one,
/// and only that gate can pin its addresses or move it on along a
/// redirect: another gate refuses it. No other code can write one or say
/// how many redirects it followed. It cannot be copied either: following a
/// redirect uses the request up, so its count cannot be started again from
/// an earlier copy.
#[derive(Debug, PartialEq, Eq)]
pub struct Admitted {
    /// The gate that admitted the request.
    issuer: Issuer,
    /// What that gate's configuration decided for it.
    decision: Decision,
    /// How many redirects the request has followed to get here.
    followed: u8,
}

impl Admitted {
    /// Seals what `gate`'s configuration decided into a request that has
    /// followed no redirect.
    pub(super) fn issue(gate: Issuer, decision: Decision) -> Self {
        Self {
            issuer: gate,
            decision,
            followed: 0,
        }
    }

    /// The purpose the request names.
    pub(super) fn purpose(&self) -> Purpose {
        self.decision.purpose
    }

    /// Where the request goes.
    pub(super) fn destination(&self) -> &Destination {
        &self.decision.destination
    }

    /// What was decided for the request, told only to the gate that
    /// admitted it. Every reader of the terms goes through here.
    fn terms(&self, gate: Issuer) -> Result<&Decision, Denial> {
        if self.issuer == gate {
            Ok(&self.decision)
        } else {
            Err(Denial::AdmittedElsewhere)
        }
    }

    /// What the address check needs, for the gate that admitted the
    /// request, when the request leaves directly.
    ///
    /// # Errors
    ///
    /// [`Denial::AdmittedElsewhere`] when `gate` did not admit the
    /// request, then [`Denial::RoutedThroughProxy`] when the request leaves
    /// through the proxy.
    pub(super) fn direct(&self, gate: Issuer) -> Result<Direct, Denial> {
        let decision = self.terms(gate)?;
        if decision.route == Route::Proxy {
            return Err(Denial::RoutedThroughProxy);
        }
        Ok(Direct {
            reach: decision.reach,
            port: decision.destination.port,
        })
    }

    /// What happens to a redirect and how many the request has followed,
    /// for the gate that admitted the request.
    ///
    /// # Errors
    ///
    /// [`Denial::AdmittedElsewhere`] when `gate` did not admit the
    /// request.
    pub(super) fn redirects(&self, gate: Issuer) -> Result<(Redirects, u8), Denial> {
        self.terms(gate)
            .map(|decision| (decision.redirects, self.followed))
    }

    /// The request this one becomes by following a redirect, given `hop`,
    /// what the same gate admitted for the redirect's target: `hop`, with
    /// one more redirect behind it than this request has.
    pub(super) fn followed_to(self, hop: Self) -> Self {
        Self {
            followed: self.followed.saturating_add(1),
            ..hop
        }
    }
}

#[cfg(test)]
impl Admitted {
    /// A request as if `gate` had admitted it with `decision` after
    /// `followed` redirects, for the tests of what no R1 purpose does yet:
    /// following a redirect, and having followed some already. The build
    /// outside tests has no such door.
    pub(super) fn assumed(gate: Issuer, decision: Decision, followed: u8) -> Self {
        Self {
            issuer: gate,
            decision,
            followed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Admitted, Direct, Issuer};
    use crate::denial::Denial;
    use crate::destination::{Destination, Host, Scheme};
    use crate::grant::{Decision, Reach, Route};
    use crate::purpose::{Purpose, Redirects};

    /// What a configuration decided for a request to `host` on `port`.
    fn decision(host: &str, port: u16, reach: Reach, route: Route) -> Decision {
        Decision {
            purpose: Purpose::Acme,
            destination: Destination {
                scheme: Scheme::Https,
                host: Host::parse(host).expect("a host"),
                port,
            },
            reach,
            route,
            redirects: Redirects::SameHost,
        }
    }

    /// Supports: SEC-TM-048, SEC-API-079
    #[test]
    fn no_two_gates_share_an_identity() {
        let identities: Vec<Issuer> = core::iter::repeat_with(Issuer::fresh).take(64).collect();
        for (at, one) in identities.iter().enumerate() {
            for (other_at, other) in identities.iter().enumerate() {
                assert_eq!(one == other, at == other_at, "{at} and {other_at}");
            }
        }
    }

    /// What a request stands for is told only to the gate that admitted
    /// it. What it says about itself is told to any, so that a refusal can
    /// be recorded under it.
    ///
    /// Supports: SEC-TM-048, SEC-API-079, SEC-EXT-002
    #[test]
    fn the_terms_of_a_request_are_told_only_to_the_gate_that_admitted_it() {
        let gate = Issuer::fresh();
        let another = Issuer::fresh();
        let cases = [
            (Reach::Global, 443, 0),
            (Reach::Lan, 8123, 2),
            (Reach::Lan, 6379, 3),
        ];
        for (reach, port, followed) in cases {
            let request = Admitted::assumed(
                gate,
                decision("10.0.0.5", port, reach, Route::Direct),
                followed,
            );
            assert_eq!(request.direct(gate), Ok(Direct { reach, port }));
            assert_eq!(request.direct(another), Err(Denial::AdmittedElsewhere));
            assert_eq!(request.redirects(gate), Ok((Redirects::SameHost, followed)));
            assert_eq!(request.redirects(another), Err(Denial::AdmittedElsewhere));
            assert_eq!(request.purpose(), Purpose::Acme);
            assert_eq!(
                request.destination(),
                &Destination {
                    scheme: Scheme::Https,
                    host: Host::parse("10.0.0.5").expect("a host"),
                    port,
                }
            );
        }
    }

    /// Supports: SEC-PRV-012
    #[test]
    fn a_request_that_leaves_through_the_proxy_has_no_address_to_pin() {
        let gate = Issuer::fresh();
        let request = Admitted::issue(
            gate,
            decision("gunmetal.tv", 443, Reach::Global, Route::Proxy),
        );
        assert_eq!(request.direct(gate), Err(Denial::RoutedThroughProxy));
        // Its redirects are still the admitting gate's to decide, and
        // another gate is told nothing, not even how the request leaves.
        assert_eq!(request.redirects(gate), Ok((Redirects::SameHost, 0)));
        assert_eq!(
            request.direct(Issuer::fresh()),
            Err(Denial::AdmittedElsewhere)
        );
    }
}
