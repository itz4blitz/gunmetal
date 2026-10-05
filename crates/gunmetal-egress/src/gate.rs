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

use core::net::{IpAddr, SocketAddr};
use std::collections::VecDeque;
use std::sync::{Mutex, MutexGuard, PoisonError};

use gunmetal_core::audit_event::{SecurityEvent, SecuritySink};
use gunmetal_core::time::Timestamp;

use crate::address;
use crate::denial::Denial;
use crate::destination::{Destination, Host};
use crate::grant::{Admitted, Configuration, Route};
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

/// The gate: the owner's configuration, the sink that takes refusals to
/// the audit log, and the record of attempts.
pub struct Gate<S> {
    configuration: Configuration,
    sink: S,
    activity: Mutex<VecDeque<Connection>>,
}

impl<S: SecuritySink> Gate<S> {
    /// A gate that decides by `configuration` and reports each refusal to
    /// `sink`.
    #[must_use]
    pub fn new(configuration: Configuration, sink: S) -> Self {
        Self {
            configuration,
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
                if admitted.route == Route::Proxy {
                    self.note(purpose, destination, Ok(()), now);
                }
                Ok(admitted)
            }
            Err(denial) => Err(self.refuse(purpose, destination, denial, now)),
        }
    }

    /// Decides which addresses an admitted request that leaves directly
    /// may connect to, given everything its host resolved to.
    ///
    /// # Errors
    ///
    /// The [`Denial`] of [`pin`](crate::address::pin), which is recorded and
    /// reported.
    pub fn pin(
        &self,
        admitted: &Admitted,
        resolved: &[IpAddr],
        now: Timestamp,
    ) -> Result<Vec<SocketAddr>, Denial> {
        let destination = &admitted.destination;
        match address::pin(admitted.reach, destination.port, resolved) {
            Ok(addresses) => {
                self.note(admitted.purpose, destination, Ok(()), now);
                Ok(addresses)
            }
            Err(denial) => Err(self.refuse(admitted.purpose, destination, denial, now)),
        }
    }

    /// Decides whether the request `from`, which has already followed
    /// `followed` redirects, may follow one more to `to`. The hop is
    /// admitted like a first request, so its addresses are checked next.
    ///
    /// # Errors
    ///
    /// The [`Denial`] of [`follow`](crate::redirect::follow) or of
    /// [`Gate::admit`], which is recorded and reported.
    pub fn redirect(
        &self,
        from: &Admitted,
        followed: u8,
        to: &Destination,
        now: Timestamp,
    ) -> Result<Admitted, Denial> {
        match redirect::follow(from.redirects, followed, &from.destination, to) {
            Ok(()) => self.admit(from.purpose, to, now),
            Err(denial) => Err(self.refuse(from.purpose, to, denial, now)),
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

#[cfg(test)]
mod tests {
    use super::{ACTIVITY_CAPACITY, Connection, Gate};
    use crate::denial::Denial;
    use crate::destination::{Destination, Host, Scheme};
    use crate::grant::{Admitted, Allowed, Configuration, Reach, Route};
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

    fn gate_for(configuration: Configuration) -> Gate<Recorder> {
        Gate::new(configuration, Recorder::default())
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

    /// An admitted request whose purpose follows redirects. No R1 purpose
    /// does, so the tests of the rule make one.
    fn following(destination: Destination) -> Admitted {
        Admitted {
            purpose: Purpose::Acme,
            destination,
            reach: Reach::Global,
            route: Route::Direct,
            redirects: Redirects::SameHost,
        }
    }

    /// Verifies: SEC-API-079, SEC-PRV-008, SEC-TM-048
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

    /// Verifies: SEC-PRV-008
    #[test]
    fn a_direct_request_is_recorded_once_its_addresses_have_passed() {
        let gate = gate_for(configured());
        let admitted = gate
            .admit(Purpose::UpdateFeed, &https("gunmetal.tv"), at(1_000))
            .expect("admitted");
        assert_eq!(gate.activity(), Vec::<Connection>::new());
        let public = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8));
        assert_eq!(
            gate.pin(&admitted, &[public], at(1_200)),
            Ok(vec![SocketAddr::new(public, 443)])
        );
        assert_eq!(
            gate.activity(),
            [line(Purpose::UpdateFeed, "gunmetal.tv", Ok(()), 1_200)]
        );
        assert_eq!(events(&gate), Vec::<SecurityEvent>::new());
    }

    /// Verifies: SEC-EXT-002, SEC-PRV-008, SEC-TM-048
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

    /// Verifies: SEC-PRV-008, SEC-PRV-012
    #[test]
    fn a_request_through_the_proxy_is_recorded_when_it_is_admitted() {
        let gate = gate_for(configured().through_proxy());
        assert_eq!(
            gate.admit(Purpose::UpdateFeed, &https("gunmetal.tv"), at(9)),
            Ok(Admitted {
                purpose: Purpose::UpdateFeed,
                destination: https("gunmetal.tv"),
                reach: Reach::Global,
                route: Route::Proxy,
                redirects: Redirects::Refused,
            })
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
        let gate = Gate::new(Configuration::default(), sink);
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

    /// Verifies: SEC-PRV-008
    #[test]
    fn the_record_keeps_the_latest_attempts_and_drops_the_oldest() {
        let gate = gate_for(configured().through_proxy());
        let feed = https("gunmetal.tv");
        for attempt in 0..=ACTIVITY_CAPACITY {
            let millis = i64::try_from(attempt).expect("a small number");
            assert_eq!(
                gate.admit(Purpose::UpdateFeed, &feed, at(millis))
                    .map(|admitted| admitted.route),
                Ok(Route::Proxy)
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
            gate.admit(Purpose::UpdateFeed, &https("gunmetal.tv"), at(7))
                .map(|admitted| admitted.route),
            Ok(Route::Proxy)
        );
        assert_eq!(
            gate.activity(),
            [line(Purpose::UpdateFeed, "gunmetal.tv", Ok(()), 7)]
        );
    }

    /// Verifies: SEC-API-078, SEC-EXT-003
    #[test]
    fn no_r1_purpose_follows_a_redirect() {
        let gate = gate_for(configured());
        for (purpose, name) in [
            (Purpose::Acme, "ca.example"),
            (Purpose::UpdateFeed, "gunmetal.tv"),
        ] {
            let admitted = gate.admit(purpose, &https(name), at(1)).expect("admitted");
            assert_eq!(
                gate.redirect(&admitted, 0, &https(name), at(2)),
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

    /// Verifies: SEC-EXT-003, SEC-PRV-008, SEC-TM-048
    #[test]
    fn a_followed_redirect_passes_the_gate_like_a_first_request() {
        let gate = gate_for(configured());
        let from = following(https("ca.example"));
        // The same host on the same port: the grant names it, so the hop
        // is admitted, under the purpose's own rules.
        assert_eq!(
            gate.redirect(&from, 0, &https("ca.example"), at(1)),
            Ok(Admitted {
                purpose: Purpose::Acme,
                destination: https("ca.example"),
                reach: Reach::Global,
                route: Route::Direct,
                redirects: Redirects::Refused,
            })
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
        let refusals = [
            (0, &other_port, Denial::DestinationNotGranted),
            (0, &plain, Denial::DestinationNotGranted),
            (0, &other_host, Denial::CrossHostRedirect),
            (3, &from.destination, Denial::TooManyRedirects),
        ];
        for (followed, to, denial) in refusals {
            assert_eq!(gate.redirect(&from, followed, to, at(2)), Err(denial));
        }
        // A third redirect is still followed.
        assert_eq!(
            gate.redirect(&from, 2, &https("ca.example"), at(3))
                .map(|admitted| admitted.destination),
            Ok(https("ca.example"))
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

    /// Verifies: SEC-EXT-003, SEC-HIS-023
    #[test]
    fn a_redirect_that_leads_to_a_private_address_is_refused() {
        let gate = gate_for(configured());
        let hop = gate
            .redirect(
                &following(https("ca.example")),
                1,
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
}
