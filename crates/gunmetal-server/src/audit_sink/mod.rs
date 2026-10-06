//! The one server-side sink that records every security event from the bus
//! (WP-069).

use std::sync::Arc;

use gunmetal_core::audit_event::{AuditUnavailable, SecurityEvent, SecuritySink};
use gunmetal_core::client_context::ClientContext;
use gunmetal_core::time::Clock;
use gunmetal_durable::audit::error::AuditError;
use gunmetal_durable::audit::log::AuditLog;
use gunmetal_durable::audit::record::WriteClass;
use gunmetal_fs::dataroot::DataRoot;
use gunmetal_secrets::keyring::Purpose;
use gunmetal_secrets::random::OsRandom;
use gunmetal_secrets::root::Root;

/// The audit log wired as a [`SecuritySink`].
pub struct AuditSink {
    /// The store.
    pub log: AuditLog,
    clock: Arc<dyn Clock + Send + Sync>,
}

impl AuditSink {
    /// Opens the durable audit log and the keys it needs.
    ///
    /// # Errors
    ///
    /// When the root secret, a key ring or the log cannot be opened.
    pub fn open(data: DataRoot, clock: Arc<dyn Clock + Send + Sync>) -> Result<Self, AuditError> {
        let root = Root::load_or_create(&data, &OsRandom)?;
        let address = Arc::new(root.key_ring(Purpose::AuditAddress)?);
        let signing = Arc::new(root.key_ring(Purpose::AuditSigning)?);
        let log = AuditLog::open(data, address, signing, Arc::new(OsRandom))?;
        Ok(Self { log, clock })
    }
}

impl SecuritySink for AuditSink {
    fn record(&self, event: SecurityEvent) -> Result<(), AuditUnavailable> {
        let from = source_of(&event);
        self.log
            .append_security_event(self.clock.now(), &event, from, WriteClass::Ordinary)
            .map(|_| ())
            .map_err(|_| AuditUnavailable)
    }
}

fn source_of(event: &SecurityEvent) -> Option<&ClientContext> {
    match event {
        SecurityEvent::AuthnLoginFail { source, .. }
        | SecurityEvent::AuthnLoginSuccess { source, .. }
        | SecurityEvent::AuthzFail { source, .. }
        | SecurityEvent::ExcessRateLimitExceeded { source } => Some(source),
        SecurityEvent::GmDebugLoggingEnabled { .. } | SecurityEvent::GmEgressDenied {} => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{AuditSink, source_of};
    use crate::bus::Bus;
    use crate::log::Level;
    use crate::testing;
    use gunmetal_core::audit_event::{SecurityEvent, SecuritySink};
    use gunmetal_core::http::forwarded::{ForwardingHeaders, HostNetwork, path_class};
    use gunmetal_core::id::{IdKind, PublicId};
    use gunmetal_fs::dataroot::{DataRoot, Policy};
    use gunmetal_fs::host::HostFacts;
    use gunmetal_fs::path::{AuditSeg, DataDir, DataPath};
    use gunmetal_testkit::tempdir::TempDir;
    use std::io::Read;
    use std::net::{IpAddr, Ipv4Addr};
    use std::sync::Arc;
    use std::time::Duration;

    fn account() -> PublicId {
        PublicId::parse("usr_0123456789abcdefghjkmnpqrs", IdKind::User).expect("id")
    }

    fn internet() -> gunmetal_core::client_context::ClientContext {
        path_class(
            IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7)),
            &ForwardingHeaders::default(),
            &[],
            &HostNetwork::default(),
        )
        .expect("class")
    }

    fn sink() -> (TempDir, AuditSink) {
        let dir = TempDir::new("audit-sink").expect("scratch");
        let host = HostFacts::probe(dir.path()).expect("host");
        let root = DataRoot::open(dir.path(), &host, Policy::DEFAULT)
            .expect("root")
            .root;
        let (clock, _) = testing::clock();
        let sink = AuditSink::open(root, clock).expect("sink");
        (dir, sink)
    }

    fn event_names(dir: &TempDir) -> Vec<String> {
        let host = HostFacts::probe(dir.path()).expect("host");
        let root = DataRoot::open(dir.path(), &host, Policy::DEFAULT)
            .expect("root")
            .root;
        let mut text = String::new();
        root.open_read(&DataPath::audit_segment(AuditSeg::new(1).expect("1")))
            .expect("segment")
            .read_to_string(&mut text)
            .expect("read");
        text.lines()
            .filter_map(|line| {
                let rest = line.split("\"event\":\"").nth(1)?;
                let name = rest.split('"').next().filter(|n| !n.is_empty())?;
                match name {
                    "gm_audit_checkpoint" | "gm_audit_pruned" => None,
                    other => Some(other.to_owned()),
                }
            })
            .collect()
    }

    #[test]
    fn event_names_skips_malformed_and_internal_lines() {
        let (dir, sink) = sink();
        sink.record(SecurityEvent::GmEgressDenied {})
            .expect("recorded");
        let host = HostFacts::probe(dir.path()).expect("host");
        let root = DataRoot::open(dir.path(), &host, Policy::DEFAULT)
            .expect("root")
            .root;
        let path = DataPath::audit_segment(AuditSeg::new(1).expect("1"));
        let mut text = String::new();
        root.open_read(&path)
            .expect("segment")
            .read_to_string(&mut text)
            .expect("read");
        text.push_str("not json\n");
        text.push_str("{\"event\":\"gm_audit_checkpoint\"}\n");
        text.push_str("{\"event\":\"gm_audit_pruned\"}\n");
        text.push_str("{\"event\":\"\"}\n");
        root.replace(&path, text.as_bytes()).expect("wrote");
        assert_eq!(event_names(&dir), ["gm_egress_denied"]);
    }

    /// Verifies: SEC-OPS-020, SEC-OPS-029, SEC-PRV-008, SEC-IAM-069
    #[test]
    fn each_producer_event_on_the_bus_yields_exactly_one_record() {
        let (dir, sink) = sink();
        let bus = Bus::default();
        let rec = Arc::new(sink);
        bus.security
            .subscribe(move |event| rec.record(event.clone()));
        let events = [
            SecurityEvent::GmEgressDenied {},
            SecurityEvent::AuthnLoginFail {
                source: internet(),
                account: None,
            },
            SecurityEvent::AuthzFail {
                source: internet(),
                account: Some(account()),
            },
            SecurityEvent::GmDebugLoggingEnabled { account: None },
        ];
        for event in &events {
            assert_eq!(bus.record(event.clone()), Ok(()));
        }
        assert_eq!(
            event_names(&dir),
            [
                "gm_egress_denied",
                "authn_login_fail",
                "authz_fail",
                "gm_debug_logging_enabled"
            ]
        );
    }

    #[test]
    fn source_of_names_every_catalogue_variant() {
        let src = internet();
        assert!(
            source_of(&SecurityEvent::AuthnLoginFail {
                source: src,
                account: None
            })
            .is_some()
        );
        assert!(
            source_of(&SecurityEvent::AuthnLoginSuccess {
                source: src,
                account: account()
            })
            .is_some()
        );
        assert!(
            source_of(&SecurityEvent::AuthzFail {
                source: src,
                account: None
            })
            .is_some()
        );
        assert!(source_of(&SecurityEvent::ExcessRateLimitExceeded { source: src }).is_some());
        assert!(source_of(&SecurityEvent::GmDebugLoggingEnabled { account: None }).is_none());
        assert!(source_of(&SecurityEvent::GmEgressDenied {}).is_none());
    }

    #[test]
    fn a_refused_append_comes_back_as_unavailable() {
        let (_dir, sink) = sink();
        sink.log.simulate_full_disk();
        assert_eq!(
            sink.record(SecurityEvent::GmEgressDenied {}),
            Err(gunmetal_core::audit_event::AuditUnavailable)
        );
    }

    #[test]
    fn enable_debug_records_through_the_bus() {
        let (dir, sink) = sink();
        let (clock, _) = testing::clock();
        let log =
            crate::log::Logger::new(clock, Level::Info, Box::new(testing::Capture::default()));
        let bus = Bus::default();
        let rec = Arc::new(sink);
        bus.security
            .subscribe(move |event| rec.record(event.clone()));
        assert!(
            log.enable_debug(Duration::from_secs(60), None, &bus)
                .is_ok()
        );
        assert_eq!(event_names(&dir), ["gm_debug_logging_enabled"]);
    }

    #[test]
    fn open_fails_on_a_malformed_root_secret() {
        let dir = TempDir::new("audit-sink-bad-root").expect("scratch");
        let host = HostFacts::probe(dir.path()).expect("host");
        let root = DataRoot::open(dir.path(), &host, Policy::DEFAULT)
            .expect("root")
            .root;
        root.replace(&DataPath::constant(DataDir::Secrets, "root.key"), b"x")
            .expect("wrote");
        let (clock, _) = testing::clock();
        assert!(AuditSink::open(root, clock).is_err());
    }

    #[test]
    fn open_fails_when_the_audit_log_cannot_open() {
        let (dir, sink) = sink();
        drop(sink);
        let host = HostFacts::probe(dir.path()).expect("host");
        let root = DataRoot::open(dir.path(), &host, Policy::DEFAULT)
            .expect("root")
            .root;
        root.replace(
            &DataPath::constant(DataDir::Durable, "audit/addresses.db"),
            b"not sqlite",
        )
        .expect("junk");
        let (clock, _) = testing::clock();
        assert!(AuditSink::open(root, clock).is_err());
    }
}
