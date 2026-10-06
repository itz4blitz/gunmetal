//! The gate every outbound request passes, and its record.
//!
//! The [`Gate`] holds the owner's [`Configuration`] and takes a request
//! through its three questions: may this purpose reach this destination at
//! all ([`Gate::admit`]), may it connect to what the name resolved to
//! ([`Gate::pin`]), and may it follow this redirect ([`Gate::redirect`]).
//!
//! Every refusal sends exactly one security event to the sink the gate was
//! given, which the server wires to the audit log, and every attempt,
//! refused or not, becomes one [`Connection`] in the record the network
//! activity page reads (SEC-PRV-008, SEC-API-079). A record line holds the
//! purpose, the host and the port, and never a path or a query.
//!
//! One attempt is one line. A request that leaves directly is recorded
//! when its addresses have been checked, because that is when it is known
//! whether it connects; a request through the proxy is recorded when it is
//! admitted, because the proxy resolves the name.
//!
//! So a request that leaves directly has its line once [`Gate::pin`] has
//! been asked about it. The client asks for every request the gate
//! admitted, and when the name did not resolve it hands over the empty
//! answer, which is refused as [`Denial::NoAddress`] and recorded like
//! any refusal. Nothing in this crate can make the client ask: an admitted
//! request the client drops before asking leaves no line.
//!
//! The types keep the order of the questions. Only
//! [`Configuration::decide`], which [`Gate::admit`] asks, makes an
//! [`Admitted`] request, and only the gate's check makes the [`Pinned`]
//! addresses that the one place that connects takes. The count of
//! redirects a request followed travels inside it, where no caller can set
//! it.

use core::net::IpAddr;
use std::collections::VecDeque;
use std::sync::{Mutex, MutexGuard, PoisonError};

use gunmetal_core::audit_event::{SecurityEvent, SecuritySink};
use gunmetal_core::time::Timestamp;

use crate::address::{self, Pinned};
use crate::denial::Denial;
use crate::destination::{Destination, Host};
use crate::grant::{Admitted, Configuration, Route};
use crate::listening::Listening;
use crate::purpose::Purpose;
use crate::redirect;

/// How many attempts the record keeps. An older attempt makes way for a
/// newer one, so the record cannot grow without bound however many
/// requests are refused.
pub const ACTIVITY_CAPACITY: usize = 256;

/// One line of the record: an attempt to reach out, and how it ended at
/// the gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection {
    /// The purpose the request named.
    pub purpose: Purpose,
    /// The host it was for.
    pub host: Host,
    /// The port it was for.
    pub port: u16,
    /// Whether the gate let it out, or why not.
    pub outcome: Result<(), Denial>,
    /// When the gate decided.
    pub at: Timestamp,
}

/// The gate: the owner's configuration, where the server itself listens,
/// the sink that takes refusals to the audit log, and the record of
/// attempts.
pub struct Gate<S> {
    configuration: Configuration,
    listening: Listening,
    sink: S,
    activity: Mutex<VecDeque<Connection>>,
}

impl<S: SecuritySink> Gate<S> {
    /// A gate that decides by `configuration`, connects to no address in
    /// `listening` and reports each refusal to `sink`.
    #[must_use]
    pub fn new(configuration: Configuration, listening: Listening, sink: S) -> Self {
        Self {
            configuration,
            listening,
            sink,
            activity: Mutex::new(VecDeque::new()),
        }
    }

    /// Decides whether `purpose` may send a request to `destination`,
    /// before anything is resolved or connected.
    ///
    /// # Errors
    ///
    /// The [`Denial`] of [`Configuration::decide`], which is recorded and
    /// reported.
    pub fn admit(
        &self,
        purpose: Purpose,
        destination: &Destination,
        now: Timestamp,
    ) -> Result<Admitted, Denial> {
        match self.configuration.decide(purpose, destination) {
            Ok(admitted) => {
                if admitted.route() == Route::Proxy {
                    self.note(purpose, destination, Ok(()), now);
                }
                Ok(admitted)
            }
            Err(denial) => Err(self.refuse(purpose, destination, denial, now)),
        }
    }

    /// Decides which addresses an admitted request that leaves directly
    /// may connect to, given everything its host resolved to: an empty
    /// `resolved` when the name did not resolve. The client asks this for
    /// every such request, so that each has its line in the record.
    ///
    /// # Errors
    ///
    /// [`Denial::NoAddress`], [`Denial::TooManyAddresses`],
    /// [`Denial::AddressRefused`] or [`Denial::OwnAddress`], which is
    /// recorded and reported.
    pub fn pin(
        &self,
        admitted: &Admitted,
        resolved: &[IpAddr],
        now: Timestamp,
    ) -> Result<Pinned, Denial> {
        let destination = admitted.destination();
        match address::pin(
            admitted.reach(),
            destination.port,
            &self.listening,
            resolved,
        ) {
            Ok(pinned) => {
                self.note(admitted.purpose(), destination, Ok(()), now);
                Ok(pinned)
            }
            Err(denial) => Err(self.refuse(admitted.purpose(), destination, denial, now)),
        }
    }

    /// Decides whether the request `from` may follow a redirect to `to`.
    /// The hop is admitted like a first request, so its addresses are
    /// checked next, and it has followed one redirect more than `from`.
    /// `from` is used up either way, so a count cannot start again from it.
    ///
    /// # Errors
    ///
    /// The [`Denial`] of [`follow`](crate::redirect::follow) or of
    /// [`Gate::admit`], which is recorded and reported.
    pub fn redirect(
        &self,
        from: Admitted,
        to: &Destination,
        now: Timestamp,
    ) -> Result<Admitted, Denial> {
        match redirect::follow(from.redirects(), from.followed(), from.destination(), to) {
            Ok(()) => self
                .admit(from.purpose(), to, now)
                .map(|hop| from.followed_to(hop)),
            Err(denial) => Err(self.refuse(from.purpose(), to, denial, now)),
        }
    }

    /// The record of attempts, oldest first: at most
    /// [`ACTIVITY_CAPACITY`] of the latest.
    #[must_use]
    pub fn activity(&self) -> Vec<Connection> {
        self.log().iter().cloned().collect()
    }

    /// Reports and records a refusal, and hands it back.
    fn refuse(
        &self,
        purpose: Purpose,
        destination: &Destination,
        denial: Denial,
        now: Timestamp,
    ) -> Denial {
        // A refusal stands whether or not the audit log could take its
        // record: an unwritten record never lets anything through.
        let _ = self.sink.record(SecurityEvent::GmEgressDenied {});
        self.note(purpose, destination, Err(denial), now);
        denial
    }

    /// Adds one line to the record, in place of the oldest when it is
    /// full.
    fn note(
        &self,
        purpose: Purpose,
        destination: &Destination,
        outcome: Result<(), Denial>,
        at: Timestamp,
    ) {
        let mut log = self.log();
        if log.len() == ACTIVITY_CAPACITY {
            log.pop_front();
        }
        log.push_back(Connection {
            purpose,
            host: destination.host.clone(),
            port: destination.port,
            outcome,
            at,
        });
    }

    /// The record, even if a previous holder of its lock panicked: a line
    /// is added in one step, so a panic cannot have left one half-written.
    fn log(&self) -> MutexGuard<'_, VecDeque<Connection>> {
        self.activity.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// What code outside this crate must not be able to do with a request.
/// Rustdoc on stable does not check which error a compile-fail test
/// produced, so each shares its imports with the control, which compiles.
#[cfg(doctest)]
mod compile_fail {
    /// Control: outside this crate, the gate admits a request, pins the
    /// addresses it may connect to and moves it on along a redirect, and
    /// the addresses are read from what the gate pinned.
    ///
    /// ```
    /// use core::net::{IpAddr, SocketAddr};
    ///
    /// use gunmetal_core::audit_event::SecuritySink;
    /// use gunmetal_core::time::Timestamp;
    /// use gunmetal_egress::address::Pinned;
    /// use gunmetal_egress::denial::Denial;
    /// use gunmetal_egress::destination::Destination;
    /// use gunmetal_egress::gate::Gate;
    /// use gunmetal_egress::grant::{Admitted, Reach, Route};
    /// use gunmetal_egress::listening::Listening;
    /// use gunmetal_egress::purpose::{Purpose, Redirects};
    ///
    /// fn admit<S: SecuritySink>(
    ///     gate: &Gate<S>,
    ///     to: &Destination,
    ///     now: Timestamp,
    /// ) -> Result<Admitted, Denial> {
    ///     gate.admit(Purpose::Acme, to, now)
    /// }
    ///
    /// fn pin<S: SecuritySink>(
    ///     gate: &Gate<S>,
    ///     admitted: &Admitted,
    ///     resolved: &[IpAddr],
    ///     now: Timestamp,
    /// ) -> Result<Pinned, Denial> {
    ///     gate.pin(admitted, resolved, now)
    /// }
    ///
    /// fn connect_to(pinned: &Pinned) -> &[SocketAddr] {
    ///     pinned.addresses()
    /// }
    ///
    /// fn hop<S: SecuritySink>(
    ///     gate: &Gate<S>,
    ///     from: Admitted,
    ///     to: &Destination,
    ///     now: Timestamp,
    /// ) -> Result<Admitted, Denial> {
    ///     gate.redirect(from, to, now)
    /// }
    ///
    /// fn nowhere() -> Listening {
    ///     Listening::none()
    /// }
    /// ```
    struct Control;

    /// Supports: SEC-EXT-002, SEC-API-077
    ///
    /// No code outside this crate can write a set of addresses to connect
    /// to: only the gate's check makes one.
    ///
    /// ```compile_fail,E0451
    /// use core::net::{IpAddr, SocketAddr};
    ///
    /// use gunmetal_core::audit_event::SecuritySink;
    /// use gunmetal_core::time::Timestamp;
    /// use gunmetal_egress::address::Pinned;
    /// use gunmetal_egress::denial::Denial;
    /// use gunmetal_egress::destination::Destination;
    /// use gunmetal_egress::gate::Gate;
    /// use gunmetal_egress::grant::{Admitted, Reach, Route};
    /// use gunmetal_egress::listening::Listening;
    /// use gunmetal_egress::purpose::{Purpose, Redirects};
    ///
    /// fn forged(addresses: Vec<SocketAddr>) -> Pinned {
    ///     Pinned { addresses }
    /// }
    /// ```
    struct NoForgedAddresses;

    /// Supports: SEC-EXT-002, SEC-PRV-008
    ///
    /// Nor can it reach the check without the gate, which records and
    /// reports what the check decides.
    ///
    /// ```compile_fail,E0603
    /// use core::net::{IpAddr, SocketAddr};
    ///
    /// use gunmetal_core::audit_event::SecuritySink;
    /// use gunmetal_core::time::Timestamp;
    /// use gunmetal_egress::address::Pinned;
    /// use gunmetal_egress::denial::Denial;
    /// use gunmetal_egress::destination::Destination;
    /// use gunmetal_egress::gate::Gate;
    /// use gunmetal_egress::grant::{Admitted, Reach, Route};
    /// use gunmetal_egress::listening::Listening;
    /// use gunmetal_egress::purpose::{Purpose, Redirects};
    ///
    /// fn unrecorded(resolved: &[IpAddr]) -> Result<Pinned, Denial> {
    ///     gunmetal_egress::address::pin(Reach::Lan, 8123, &Listening::none(), resolved)
    /// }
    /// ```
    struct NoCheckWithoutTheGate;

    /// Supports: SEC-TM-048, SEC-API-079
    ///
    /// It cannot write an admitted request either, with whatever purpose,
    /// destination, reach, route or count of redirects.
    ///
    /// ```compile_fail,E0451
    /// use core::net::{IpAddr, SocketAddr};
    ///
    /// use gunmetal_core::audit_event::SecuritySink;
    /// use gunmetal_core::time::Timestamp;
    /// use gunmetal_egress::address::Pinned;
    /// use gunmetal_egress::denial::Denial;
    /// use gunmetal_egress::destination::Destination;
    /// use gunmetal_egress::gate::Gate;
    /// use gunmetal_egress::grant::{Admitted, Reach, Route};
    /// use gunmetal_egress::listening::Listening;
    /// use gunmetal_egress::purpose::{Purpose, Redirects};
    ///
    /// fn forged(destination: Destination) -> Admitted {
    ///     Admitted {
    ///         purpose: Purpose::Acme,
    ///         destination,
    ///         reach: Reach::Lan,
    ///         route: Route::Direct,
    ///         redirects: Redirects::SameHost,
    ///         followed: 0,
    ///     }
    /// }
    /// ```
    struct NoForgedAdmission;

    /// Supports: SEC-EXT-003, SEC-API-078
    ///
    /// A caller does not say how many redirects a request followed ...
    ///
    /// ```compile_fail,E0061
    /// use core::net::{IpAddr, SocketAddr};
    ///
    /// use gunmetal_core::audit_event::SecuritySink;
    /// use gunmetal_core::time::Timestamp;
    /// use gunmetal_egress::address::Pinned;
    /// use gunmetal_egress::denial::Denial;
    /// use gunmetal_egress::destination::Destination;
    /// use gunmetal_egress::gate::Gate;
    /// use gunmetal_egress::grant::{Admitted, Reach, Route};
    /// use gunmetal_egress::listening::Listening;
    /// use gunmetal_egress::purpose::{Purpose, Redirects};
    ///
    /// fn hop<S: SecuritySink>(
    ///     gate: &Gate<S>,
    ///     from: Admitted,
    ///     to: &Destination,
    ///     now: Timestamp,
    /// ) -> Result<Admitted, Denial> {
    ///     gate.redirect(from, 0, to, now)
    /// }
    /// ```
    struct NoCountFromTheCaller;

    /// Supports: SEC-EXT-003, SEC-API-078
    ///
    /// ... and cannot start the count again from a request that already
    /// followed one, ...
    ///
    /// ```compile_fail,E0382
    /// use core::net::{IpAddr, SocketAddr};
    ///
    /// use gunmetal_core::audit_event::SecuritySink;
    /// use gunmetal_core::time::Timestamp;
    /// use gunmetal_egress::address::Pinned;
    /// use gunmetal_egress::denial::Denial;
    /// use gunmetal_egress::destination::Destination;
    /// use gunmetal_egress::gate::Gate;
    /// use gunmetal_egress::grant::{Admitted, Reach, Route};
    /// use gunmetal_egress::listening::Listening;
    /// use gunmetal_egress::purpose::{Purpose, Redirects};
    ///
    /// fn twice<S: SecuritySink>(
    ///     gate: &Gate<S>,
    ///     from: Admitted,
    ///     to: &Destination,
    ///     now: Timestamp,
    /// ) -> Result<Admitted, Denial> {
    ///     let _first = gate.redirect(from, to, now);
    ///     gate.redirect(from, to, now)
    /// }
    /// ```
    struct NoSecondHopFromOneRequest;

    /// Supports: SEC-EXT-003, SEC-API-078
    ///
    /// ... or from a copy of one.
    ///
    /// ```compile_fail,E0599
    /// use core::net::{IpAddr, SocketAddr};
    ///
    /// use gunmetal_core::audit_event::SecuritySink;
    /// use gunmetal_core::time::Timestamp;
    /// use gunmetal_egress::address::Pinned;
    /// use gunmetal_egress::denial::Denial;
    /// use gunmetal_egress::destination::Destination;
    /// use gunmetal_egress::gate::Gate;
    /// use gunmetal_egress::grant::{Admitted, Reach, Route};
    /// use gunmetal_egress::listening::Listening;
    /// use gunmetal_egress::purpose::{Purpose, Redirects};
    ///
    /// fn copy(from: Admitted) -> (Admitted, Admitted) {
    ///     (from.clone(), from)
    /// }
    /// ```
    struct NoCopiedRequest;
}

#[cfg(test)]
mod tests {
    use super::{ACTIVITY_CAPACITY, Connection, Gate};
    use crate::address::Pinned;
    use crate::denial::Denial;
    use crate::destination::{Destination, Host, Scheme};
    use crate::grant::{Admitted, Allowed, Configuration, Reach, Route};
    use crate::listening::Listening;
    use crate::purpose::{Purpose, Redirects};
    use core::net::{IpAddr, Ipv4Addr, SocketAddr};
    use gunmetal_core::audit_event::{AuditUnavailable, SecurityEvent, SecuritySink};
    use gunmetal_core::net::AddrClass;
    use gunmetal_core::time::Timestamp;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::Mutex;

    /// A sink that keeps what it is given, and can stand for an audit log
    /// that cannot write.
    #[derive(Default)]
    struct Recorder {
        events: Mutex<Vec<SecurityEvent>>,
        broken: bool,
    }

    impl SecuritySink for Recorder {
        fn record(&self, event: SecurityEvent) -> Result<(), AuditUnavailable> {
            self.events.lock().expect("events").push(event);
            if self.broken {
                Err(AuditUnavailable)
            } else {
                Ok(())
            }
        }
    }

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

    fn at(millis: i64) -> Timestamp {
        Timestamp::from_millis(millis).expect("a time")
    }

    /// A claimed server whose owner configured a domain of their own and
    /// said yes to the update check.
    fn configured() -> Configuration {
        Configuration::default()
            .claimed()
            .own_domain_https(
                Allowed::public(host("ca.example"), 443),
                Allowed::public(host("dns.example"), 443),
            )
            .update_feed()
    }

    /// A gate for a process that listens nowhere.
    fn gate_for(configuration: Configuration) -> Gate<Recorder> {
        Gate::new(configuration, Listening::none(), Recorder::default())
    }

    /// The events the gate's sink was given.
    fn events(gate: &Gate<Recorder>) -> Vec<SecurityEvent> {
        gate.sink.events.lock().expect("events").clone()
    }

    /// A line of the record for an HTTPS destination.
    fn line(purpose: Purpose, name: &str, outcome: Result<(), Denial>, millis: i64) -> Connection {
        Connection {
            purpose,
            host: host(name),
            port: 443,
            outcome,
            at: at(millis),
        }
    }

    /// An admitted request whose purpose follows redirects, which has
    /// followed `followed` already. No R1 purpose follows any, so the
    /// tests of the rule make one.
    fn following(destination: Destination, followed: u8) -> Admitted {
        Admitted::assumed(
            Purpose::Acme,
            destination,
            Reach::Global,
            Route::Direct,
            Redirects::SameHost,
            followed,
        )
    }

    /// What certificate issuance lets out directly to `destination` on the
    /// internet, after `followed` redirects.
    fn issuance(destination: Destination, followed: u8) -> Admitted {
        Admitted::assumed(
            Purpose::Acme,
            destination,
            Reach::Global,
            Route::Direct,
            Redirects::Refused,
            followed,
        )
    }

    /// What the update feed lets out through the proxy.
    fn proxied_feed() -> Admitted {
        Admitted::assumed(
            Purpose::UpdateFeed,
            https("gunmetal.tv"),
            Reach::Global,
            Route::Proxy,
            Redirects::Refused,
            0,
        )
    }

    /// The addresses `pinned` lets the request connect to.
    fn addresses(pinned: &Pinned) -> Vec<SocketAddr> {
        pinned.addresses().to_vec()
    }

    /// Supports: SEC-API-079, SEC-PRV-008, SEC-TM-048
    #[test]
    fn a_refusal_is_recorded_once_and_sends_exactly_one_event() {
        let cases = [
            (
                Configuration::default().claimed(),
                Purpose::UpdateFeed,
                "gunmetal.tv",
                Denial::PurposeNotGranted,
            ),
            (
                configured(),
                Purpose::UpdateFeed,
                "tracker.example",
                Denial::DestinationNotGranted,
            ),
            (
                configured().offline(),
                Purpose::Acme,
                "ca.example",
                Denial::Offline,
            ),
        ];
        for (configuration, purpose, name, denial) in cases {
            let gate = gate_for(configuration);
            assert_eq!(gate.admit(purpose, &https(name), at(1_000)), Err(denial));
            assert_eq!(events(&gate), [SecurityEvent::GmEgressDenied {}]);
            assert_eq!(gate.activity(), [line(purpose, name, Err(denial), 1_000)]);
        }
    }

    /// Supports: SEC-PRV-008
    #[test]
    fn a_direct_request_is_recorded_once_its_addresses_have_passed() {
        let gate = gate_for(configured());
        let admitted = gate
            .admit(Purpose::UpdateFeed, &https("gunmetal.tv"), at(1_000))
            .expect("admitted");
        assert_eq!(gate.activity(), Vec::<Connection>::new());
        let public = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8));
        assert_eq!(
            gate.pin(&admitted, &[public], at(1_200))
                .as_ref()
                .map(addresses),
            Ok(vec![SocketAddr::new(public, 443)])
        );
        assert_eq!(
            gate.activity(),
            [line(Purpose::UpdateFeed, "gunmetal.tv", Ok(()), 1_200)]
        );
        assert_eq!(events(&gate), Vec::<SecurityEvent>::new());
    }

    /// Supports: SEC-EXT-002, SEC-PRV-008, SEC-TM-048
    #[test]
    fn a_refused_address_is_recorded_and_sends_exactly_one_event() {
        let gate = gate_for(configured());
        let admitted = gate
            .admit(Purpose::Acme, &https("ca.example"), at(5))
            .expect("admitted");
        let metadata = IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254));
        let denial = Denial::AddressRefused {
            address: metadata,
            class: AddrClass::LinkLocal,
        };
        assert_eq!(gate.pin(&admitted, &[metadata], at(6)), Err(denial));
        assert_eq!(events(&gate), [SecurityEvent::GmEgressDenied {}]);
        assert_eq!(
            gate.activity(),
            [line(Purpose::Acme, "ca.example", Err(denial), 6)]
        );
    }

    /// Supports: SEC-PRV-008, SEC-PRV-012
    #[test]
    fn a_request_through_the_proxy_is_recorded_when_it_is_admitted() {
        let gate = gate_for(configured().through_proxy());
        assert_eq!(
            gate.admit(Purpose::UpdateFeed, &https("gunmetal.tv"), at(9)),
            Ok(proxied_feed())
        );
        assert_eq!(
            gate.activity(),
            [line(Purpose::UpdateFeed, "gunmetal.tv", Ok(()), 9)]
        );
        assert_eq!(events(&gate), Vec::<SecurityEvent>::new());
    }

    #[test]
    fn a_refusal_stands_when_the_audit_log_cannot_take_its_record() {
        let sink = Recorder {
            broken: true,
            ..Recorder::default()
        };
        let gate = Gate::new(Configuration::default(), Listening::none(), sink);
        assert_eq!(
            gate.admit(Purpose::Acme, &https("ca.example"), at(3)),
            Err(Denial::PurposeNotGranted)
        );
        assert_eq!(events(&gate), [SecurityEvent::GmEgressDenied {}]);
        assert_eq!(
            gate.activity(),
            [line(
                Purpose::Acme,
                "ca.example",
                Err(Denial::PurposeNotGranted),
                3
            )]
        );
    }

    /// Supports: SEC-PRV-008
    #[test]
    fn the_record_keeps_the_latest_attempts_and_drops_the_oldest() {
        let gate = gate_for(configured().through_proxy());
        let feed = https("gunmetal.tv");
        for attempt in 0..=ACTIVITY_CAPACITY {
            let millis = i64::try_from(attempt).expect("a small number");
            assert_eq!(
                gate.admit(Purpose::UpdateFeed, &feed, at(millis)),
                Ok(proxied_feed())
            );
        }
        let times: Vec<i64> = gate
            .activity()
            .iter()
            .map(|attempt| attempt.at.millis())
            .collect();
        let latest: Vec<i64> = (1..=256).collect();
        assert_eq!(times, latest);
    }

    #[test]
    fn the_record_outlives_a_panic_while_it_was_held() {
        let gate = gate_for(configured().through_proxy());
        let poisoned = catch_unwind(AssertUnwindSafe(|| {
            let _held = gate.activity.lock().expect("the first lock");
            panic!("poison the record");
        }));
        assert!(poisoned.is_err());
        assert_eq!(
            gate.admit(Purpose::UpdateFeed, &https("gunmetal.tv"), at(7)),
            Ok(proxied_feed())
        );
        assert_eq!(
            gate.activity(),
            [line(Purpose::UpdateFeed, "gunmetal.tv", Ok(()), 7)]
        );
    }

    /// Supports: SEC-API-078, SEC-EXT-003
    #[test]
    fn no_r1_purpose_follows_a_redirect() {
        let gate = gate_for(configured());
        for (purpose, name) in [
            (Purpose::Acme, "ca.example"),
            (Purpose::UpdateFeed, "gunmetal.tv"),
        ] {
            let admitted = gate.admit(purpose, &https(name), at(1)).expect("admitted");
            assert_eq!(
                gate.redirect(admitted, &https(name), at(2)),
                Err(Denial::RedirectsRefused),
                "{name}"
            );
        }
        assert_eq!(
            events(&gate),
            [
                SecurityEvent::GmEgressDenied {},
                SecurityEvent::GmEgressDenied {}
            ]
        );
        assert_eq!(
            gate.activity(),
            [
                line(
                    Purpose::Acme,
                    "ca.example",
                    Err(Denial::RedirectsRefused),
                    2
                ),
                line(
                    Purpose::UpdateFeed,
                    "gunmetal.tv",
                    Err(Denial::RedirectsRefused),
                    2
                ),
            ]
        );
    }

    /// Supports: SEC-EXT-003, SEC-PRV-008, SEC-TM-048
    #[test]
    fn a_followed_redirect_passes_the_gate_like_a_first_request() {
        let gate = gate_for(configured());
        // The same host on the same port: the grant names it, so the hop
        // is admitted, under the purpose's own rules.
        assert_eq!(
            gate.redirect(
                following(https("ca.example"), 0),
                &https("ca.example"),
                at(1)
            ),
            Ok(issuance(https("ca.example"), 1))
        );
        // The same host on a port, and over a scheme, the grant does not
        // name.
        let other_port = Destination {
            port: 8443,
            ..https("ca.example")
        };
        let plain = Destination {
            scheme: Scheme::Http,
            host: host("ca.example"),
            port: 80,
        };
        // Another host, even one the purpose was granted.
        let other_host = https("dns.example");
        let same = https("ca.example");
        let refusals = [
            (0, &other_port, Denial::DestinationNotGranted),
            (0, &plain, Denial::DestinationNotGranted),
            (0, &other_host, Denial::CrossHostRedirect),
            (3, &same, Denial::TooManyRedirects),
        ];
        for (followed, to, denial) in refusals {
            assert_eq!(
                gate.redirect(following(https("ca.example"), followed), to, at(2)),
                Err(denial)
            );
        }
        // A third redirect is still followed.
        assert_eq!(
            gate.redirect(
                following(https("ca.example"), 2),
                &https("ca.example"),
                at(3)
            ),
            Ok(issuance(https("ca.example"), 3))
        );
        assert_eq!(events(&gate), vec![SecurityEvent::GmEgressDenied {}; 4]);
        let outcomes: Vec<Result<(), Denial>> = gate
            .activity()
            .iter()
            .map(|attempt| attempt.outcome)
            .collect();
        assert_eq!(
            outcomes,
            [
                Err(Denial::DestinationNotGranted),
                Err(Denial::DestinationNotGranted),
                Err(Denial::CrossHostRedirect),
                Err(Denial::TooManyRedirects),
            ]
        );
    }

    /// Supports: SEC-EXT-003, SEC-HIS-023
    #[test]
    fn a_redirect_that_leads_to_a_private_address_is_refused() {
        let gate = gate_for(configured());
        let hop = gate
            .redirect(
                following(https("ca.example"), 1),
                &https("ca.example"),
                at(1),
            )
            .expect("followed");
        let private = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5));
        let denial = Denial::AddressRefused {
            address: private,
            class: AddrClass::Private,
        };
        assert_eq!(gate.pin(&hop, &[private], at(2)), Err(denial));
        assert_eq!(events(&gate), [SecurityEvent::GmEgressDenied {}]);
        assert_eq!(
            gate.activity(),
            [line(Purpose::Acme, "ca.example", Err(denial), 2)]
        );
    }

    /// The count of redirects is the gate's: a request it admits has
    /// followed none, and each hop has followed one more than the request
    /// it came from, up to the last one allowed.
    ///
    /// Supports: SEC-API-078, SEC-EXT-003
    #[test]
    fn the_gate_counts_the_redirects_a_request_has_followed() {
        let gate = gate_for(configured());
        assert_eq!(
            gate.admit(Purpose::Acme, &https("ca.example"), at(1)),
            Ok(issuance(https("ca.example"), 0))
        );
        for (before, after) in [(0, 1), (1, 2), (2, 3)] {
            assert_eq!(
                gate.redirect(
                    following(https("ca.example"), before),
                    &https("ca.example"),
                    at(2)
                ),
                Ok(issuance(https("ca.example"), after)),
                "{before}"
            );
        }
        for before in [3, 4, 255] {
            assert_eq!(
                gate.redirect(
                    following(https("ca.example"), before),
                    &https("ca.example"),
                    at(3)
                ),
                Err(Denial::TooManyRedirects),
                "{before}"
            );
        }
        assert_eq!(events(&gate), vec![SecurityEvent::GmEgressDenied {}; 3]);
    }

    /// Supports: SEC-EXT-002, SEC-PRV-008
    #[test]
    fn an_address_the_server_listens_on_is_refused_recorded_and_reported() {
        let own_public = IpAddr::V4(Ipv4Addr::new(8, 8, 4, 4));
        let own_lan = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10));
        // The admin granted the server's own address on the home network,
        // written out, as the DNS provider.
        let dns = Destination {
            scheme: Scheme::Http,
            host: host("192.168.1.10"),
            port: 8081,
        };
        let configuration = Configuration::default().own_domain_https(
            Allowed::public(host("ca.example"), 443),
            Allowed::lan(Scheme::Http, host("192.168.1.10"), 8081),
        );
        let listening = Listening::new(&[own_public, own_lan]).expect("somewhere to listen");
        let gate = Gate::new(configuration, listening, Recorder::default());
        let authority = gate
            .admit(Purpose::Acme, &https("ca.example"), at(1))
            .expect("admitted");
        let public_denial = Denial::OwnAddress {
            address: own_public,
        };
        assert_eq!(
            gate.pin(
                &authority,
                &[IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), own_public],
                at(2)
            ),
            Err(public_denial)
        );
        let provider = gate.admit(Purpose::Acme, &dns, at(3)).expect("admitted");
        let lan_denial = Denial::OwnAddress { address: own_lan };
        assert_eq!(gate.pin(&provider, &[own_lan], at(4)), Err(lan_denial));
        // A server that listens elsewhere lets the same answer through.
        let elsewhere = Gate::new(
            configured(),
            Listening::new(&[IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))]).expect("somewhere to listen"),
            Recorder::default(),
        );
        let feed = elsewhere
            .admit(Purpose::UpdateFeed, &https("gunmetal.tv"), at(5))
            .expect("admitted");
        assert_eq!(
            elsewhere
                .pin(&feed, &[own_public], at(6))
                .as_ref()
                .map(addresses),
            Ok(vec![SocketAddr::new(own_public, 443)])
        );
        assert_eq!(events(&gate), vec![SecurityEvent::GmEgressDenied {}; 2]);
        assert_eq!(
            gate.activity(),
            [
                line(Purpose::Acme, "ca.example", Err(public_denial), 2),
                Connection {
                    purpose: Purpose::Acme,
                    host: host("192.168.1.10"),
                    port: 8081,
                    outcome: Err(lan_denial),
                    at: at(4),
                },
            ]
        );
        assert_eq!(events(&elsewhere), Vec::<SecurityEvent>::new());
        assert_eq!(
            elsewhere.activity(),
            [line(Purpose::UpdateFeed, "gunmetal.tv", Ok(()), 6)]
        );
    }
}
