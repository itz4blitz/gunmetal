//! Tests for the audit log.

use std::fs;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::Arc;

use gunmetal_core::audit_event::SecurityEvent;
use gunmetal_core::authz::{
    Action, Capability, CapabilitySet, Context, DeviceClass, Elevation, Network, Owner, Permit,
    PrincipalFacts, PrincipalKind, Reach, RemoteAdmin, ResourceFacts, Role, UserVerification,
    decide,
};
use gunmetal_core::client_context::PathClass;
use gunmetal_core::id::PublicId;
use gunmetal_core::retention::DEFAULT;
use gunmetal_core::time::Timestamp;
use gunmetal_fs::path::{AUDIT_RESERVE, AuditSeg, DataPath};
use proptest::prelude::*;

use super::{AuditLog, Limits};
use crate::audit::chain;
use crate::audit::encode::{hex, unhex32};
use crate::audit::error::{AuditError, BrokenAt};
use crate::audit::parse::{self, Json};
use crate::audit::record::{Outcome, TruncatedAddr, WriteClass};
use crate::audit::testing::{
    Counted, FailingMac, FailingRandom, MixMac, ORDINARY, account, data, handle, internet,
    internet_v6, limited, log, loopback, mix,
};

fn at(ms: i64) -> Timestamp {
    Timestamp::from_millis(ms).expect("in range")
}

fn ctx() -> Context {
    Context {
        path: PathClass::Loopback,
        network: Network::Same,
        remote_admin: RemoteAdmin::Allowed,
    }
}

fn own_permit(who: PublicId) -> Permit {
    let facts = PrincipalFacts {
        kind: PrincipalKind::Member,
        account: Some(who),
        profile: None,
        capabilities: CapabilitySet::of(&[Capability::OwnRead, Capability::OwnWrite]),
        libraries: Vec::new(),
        device: DeviceClass::Personal,
        elevation: Elevation::Ordinary,
        verification: UserVerification::Stale,
        reach: Reach::Anywhere,
        scope: None,
    };
    decide(
        &facts,
        Action::ReadOwnData,
        &ResourceFacts::Owned(Owner::Account(who)),
        &ctx(),
    )
    .expect("the policy allows it")
}

fn audit_permit() -> Permit {
    let facts = PrincipalFacts {
        kind: PrincipalKind::Administrator,
        account: Some(account()),
        profile: None,
        capabilities: Role::Administrator.preset(),
        libraries: Vec::new(),
        device: DeviceClass::Personal,
        elevation: Elevation::Elevated,
        verification: UserVerification::Fresh,
        reach: Reach::Anywhere,
        scope: None,
    };
    decide(&facts, Action::ReadAuditLog, &ResourceFacts::Server, &ctx())
        .expect("the policy allows it")
}

fn browse_permit() -> Permit {
    let facts = PrincipalFacts {
        kind: PrincipalKind::Member,
        account: Some(account()),
        profile: None,
        capabilities: Role::Member.preset(),
        libraries: Vec::new(),
        device: DeviceClass::Personal,
        elevation: Elevation::Ordinary,
        verification: UserVerification::Stale,
        reach: Reach::Anywhere,
        scope: None,
    };
    decide(
        &facts,
        Action::BrowseLibrary,
        &ResourceFacts::Server,
        &ctx(),
    )
    .expect("the policy allows it")
}

fn egress() -> SecurityEvent {
    SecurityEvent::GmEgressDenied {}
}

fn login_fail(who: Option<PublicId>) -> SecurityEvent {
    SecurityEvent::AuthnLoginFail {
        source: internet(),
        account: who,
    }
}

fn login_ok() -> SecurityEvent {
    SecurityEvent::AuthnLoginSuccess {
        source: internet(),
        account: account(),
    }
}

fn authz_fail() -> SecurityEvent {
    SecurityEvent::AuthzFail {
        source: loopback(),
        account: Some(account()),
    }
}

fn rate() -> SecurityEvent {
    SecurityEvent::ExcessRateLimitExceeded { source: internet() }
}

fn debug_on() -> SecurityEvent {
    SecurityEvent::GmDebugLoggingEnabled { account: None }
}

/// Verifies: SEC-OPS-020, SEC-OPS-021
#[test]
fn appends_each_catalogue_event_as_a_whole_record() {
    let data = data();
    let log = log(&data);
    let t = at(1_791_028_800_000);
    assert_eq!(
        log.append_security_event(t, &egress(), None, ORDINARY),
        Ok(1)
    );
    assert_eq!(
        log.append_security_event(t, &login_fail(None), Some(&internet()), ORDINARY),
        Ok(2)
    );
    assert_eq!(
        log.append_security_event(t, &login_ok(), Some(&internet()), ORDINARY),
        Ok(4)
    );
    assert_eq!(
        log.append_security_event(t, &authz_fail(), Some(&loopback()), ORDINARY),
        Ok(6)
    );
    assert_eq!(
        log.append_security_event(t, &rate(), Some(&internet()), ORDINARY),
        Ok(8)
    );
    assert_eq!(
        log.append_security_event(t, &debug_on(), None, ORDINARY),
        Ok(10)
    );
    assert_eq!(log.verify_audit_log(), Ok(()));
    let page = log.read_all(&audit_permit(), 1..100).expect("read");
    let names: Vec<&str> = page.records.iter().map(|r| r.event.as_str()).collect();
    assert_eq!(
        names,
        [
            "gm_egress_denied",
            "authn_login_fail",
            "authn_login_success",
            "authz_fail",
            "excess_rate_limit_exceeded",
            "gm_debug_logging_enabled"
        ]
    );
    assert_eq!(page.records[0].outcome, Outcome::Denied);
    assert_eq!(page.records[2].outcome, Outcome::Success);
    assert_eq!(page.records[2].account, Some(account()));
    assert_eq!(
        page.records[1].addr,
        Some(TruncatedAddr::V4Prefix("203.0.113.0/24".to_owned()))
    );
}

/// Verifies: SEC-TM-024, SEC-API-010, SEC-IAM-097, SEC-OPS-027
#[test]
fn own_reader_never_returns_another_persons_events_and_admin_sees_truncated_addresses() {
    let data = data();
    let log = log(&data);
    let t = at(1_791_028_800_000);
    log.append_security_event(t, &login_ok(), Some(&internet()), ORDINARY)
        .expect("stored");
    let other = PublicId::parse(
        "usr_0zzzzzzzzzzzzzzzzzzzzzzzzz",
        gunmetal_core::id::IdKind::User,
    )
    .expect("id");
    let own = log.read_own(&own_permit(account()), 1..10).expect("own");
    assert_eq!(own.records.len(), 1);
    assert_eq!(own.records[0].addr, Some(internet().addr()));
    let stranger = log.read_own(&own_permit(other), 1..10).expect("other");
    assert_eq!(stranger.records, []);
    assert_eq!(
        log.read_own(&browse_permit(), 1..10),
        Err(AuditError::Denied)
    );
    assert_eq!(
        log.read_all(&browse_permit(), 1..10),
        Err(AuditError::Denied)
    );
    assert_eq!(log.head(&browse_permit()), Err(AuditError::Denied));
    let all = log.read_all(&audit_permit(), 1..10).expect("all");
    assert_eq!(
        all.records[0].addr,
        Some(TruncatedAddr::V4Prefix("203.0.113.0/24".to_owned()))
    );
}

/// Verifies: SEC-OPS-020
#[test]
fn a_failed_write_refuses_the_action() {
    let data = data();
    let log = log(&data);
    log.simulate_full_disk();
    assert_eq!(
        log.append_security_event(at(1), &egress(), None, ORDINARY),
        Err(AuditError::DiskFull)
    );
    assert_eq!(
        log.append_security_event(at(1), &egress(), None, ORDINARY),
        Err(AuditError::DiskFull)
    );
}

/// Verifies: SEC-OPS-020
#[test]
fn recovery_actions_are_recorded_on_a_simulated_full_disk() {
    let data = data();
    let log = log(&data);
    log.simulate_full_disk();
    assert_eq!(
        log.append_security_event(at(1), &egress(), None, ORDINARY),
        Err(AuditError::DiskFull)
    );
    assert_eq!(
        log.append_security_event(
            at(1_791_028_800_000),
            &debug_on(),
            None,
            WriteClass::Recovery
        ),
        Ok(1)
    );
    let mut reserve = Vec::new();
    data.root
        .open_read(&AUDIT_RESERVE)
        .expect("reserve")
        .read_to_end(&mut reserve)
        .expect("read");
    assert_eq!(reserve.len(), 8);
    assert_eq!(log.verify_audit_log(), Ok(()));
}

#[test]
fn a_recovery_write_on_a_disk_that_is_not_full_leaves_the_reserve() {
    let data = data();
    let log = log(&data);
    log.append_security_event(
        at(1_791_028_800_000),
        &debug_on(),
        None,
        WriteClass::Recovery,
    )
    .expect("stored");
    let mut reserve = Vec::new();
    data.root
        .open_read(&AUDIT_RESERVE)
        .expect("reserve")
        .read_to_end(&mut reserve)
        .expect("read");
    assert_eq!(reserve.len(), 256);
}

/// Verifies: SEC-OPS-020, SEC-OPS-023
#[test]
fn rotates_at_the_size_limit_and_still_verifies() {
    let data = data();
    let log = log(&data);
    let t = at(1_791_028_800_000);
    for _ in 0..6 {
        log.append_security_event(t, &egress(), None, ORDINARY)
            .expect("stored");
    }
    assert_eq!(log.verify_audit_log(), Ok(()));
    assert!(
        data.root
            .open_read(&DataPath::audit_segment(AuditSeg::new(2).expect("2")))
            .is_ok()
    );
}

#[test]
fn a_write_that_would_fill_the_segment_exactly_stays_on_it() {
    let probe = data();
    let measuring = limited(
        &probe,
        Limits {
            segment: 1_000_000,
            checkpoint_every: 10_000,
            checkpoint_ms: 3_600_000,
            reserve: 256,
        },
    );
    measuring
        .append_security_event(at(1_791_028_800_000), &egress(), None, ORDINARY)
        .expect("probe");
    let mut sample = String::new();
    probe
        .root
        .open_read(&DataPath::audit_segment(AuditSeg::new(1).expect("1")))
        .expect("seg")
        .read_to_string(&mut sample)
        .expect("read");
    let line_len = sample.len();
    let data = data();
    let log = limited(
        &data,
        Limits {
            segment: line_len.saturating_mul(2),
            checkpoint_every: 10_000,
            checkpoint_ms: 3_600_000,
            reserve: 256,
        },
    );
    log.append_security_event(at(1_791_028_800_000), &egress(), None, ORDINARY)
        .expect("1");
    log.append_security_event(at(1_791_028_800_000), &egress(), None, ORDINARY)
        .expect("2");
    assert!(
        data.root
            .open_read(&DataPath::audit_segment(AuditSeg::new(2).expect("2")))
            .is_err()
    );
    assert_eq!(log.verify_audit_log(), Ok(()));
}

/// Verifies: SEC-OPS-022
#[test]
fn source_addresses_are_not_in_the_chained_json() {
    let data = data();
    let log = log(&data);
    log.append_security_event(
        at(1_791_028_800_000),
        &login_ok(),
        Some(&internet()),
        ORDINARY,
    )
    .expect("stored");
    let mut file = data
        .root
        .open_read(&DataPath::audit_segment(AuditSeg::new(1).expect("1")))
        .expect("segment");
    let mut text = String::new();
    file.read_to_string(&mut text).expect("read");
    assert!(!text.contains("203.0.113.7"));
    assert!(text.contains("\"class\":\"internet\""));
    assert!(text.contains("\"via\":\"direct\""));
}

/// Verifies: SEC-OPS-023, SEC-IAM-094
#[test]
fn any_edit_of_a_valid_log_is_detected() {
    let data = data();
    let log = log(&data);
    let t = at(1_791_028_800_000);
    log.append_security_event(t, &egress(), None, ORDINARY)
        .expect("stored");
    log.append_security_event(t, &egress(), None, ORDINARY)
        .expect("stored");
    assert_eq!(log.verify_audit_log(), Ok(()));
    let path = DataPath::audit_segment(AuditSeg::new(1).expect("1"));
    let mut text = String::new();
    data.root
        .open_read(&path)
        .expect("open")
        .read_to_string(&mut text)
        .expect("read");
    let broken = text.replace("gm_egress_denied", "gm_egress_denieX");
    data.root.replace(&path, broken.as_bytes()).expect("wrote");
    let reopened = AuditLog::open_with(
        handle(&data),
        Arc::new(MixMac::new(0)),
        Arc::new(MixMac::new(1)),
        Arc::new(Counted::new()),
        Limits::test(),
    )
    .expect("reopen");
    assert_eq!(
        reopened.verify_audit_log().expect_err("broken"),
        BrokenAt { seq: 1 }
    );
}

/// Verifies: SEC-PRV-003, SEC-PRV-005, SEC-OPS-026, SEC-OPS-023
#[test]
fn retention_coarsens_then_removes_addresses_and_still_verifies() {
    let data = data();
    let log = log(&data);
    let t0 = at(1_791_028_800_000);
    log.append_security_event(t0, &login_ok(), Some(&internet()), ORDINARY)
        .expect("stored");
    log.append_security_event(t0, &login_ok(), Some(&internet_v6()), ORDINARY)
        .expect("stored");
    let day = 86_400_000_i64;
    let report = log
        .apply_retention(&DEFAULT, at(t0.millis() + 31 * day))
        .expect("coarsen");
    assert_eq!(report.coarsened, 2);
    assert_eq!(report.removed, 0);
    assert_eq!(log.verify_audit_log(), Ok(()));
    let all = log.read_all(&audit_permit(), 1..10).expect("all");
    assert_eq!(
        all.records[0].addr,
        Some(TruncatedAddr::V4Prefix("203.0.113.0/24".to_owned()))
    );
    let report = log
        .apply_retention(&DEFAULT, at(t0.millis() + 91 * day))
        .expect("remove");
    assert_eq!(report.removed, 2);
    assert_eq!(log.verify_audit_log(), Ok(()));
    scan_absent(data.dir.path(), b"203.0.113.7");
    scan_absent(
        data.dir.path(),
        &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    );
    let report = log
        .apply_retention(&DEFAULT, at(t0.millis() + 366 * day))
        .expect("prune");
    assert!(report.pruned >= 1);
    assert_eq!(log.verify_audit_log(), Ok(()));
}

/// Verifies: SEC-OPS-026, SEC-OPS-023
#[test]
fn a_prune_writes_a_pruned_record_and_drops_the_events() {
    let data = data();
    let limits = Limits {
        segment: 4_096,
        checkpoint_every: 1_000,
        checkpoint_ms: 3_600_000,
        reserve: 256,
    };
    let log = limited(&data, limits);
    let t0 = at(1_791_028_800_000);
    log.append_security_event(t0, &egress(), None, ORDINARY)
        .expect("1");
    log.append_security_event(t0, &egress(), None, ORDINARY)
        .expect("2");
    let day = 86_400_000_i64;
    let report = log
        .apply_retention(&DEFAULT, at(t0.millis() + 366 * day))
        .expect("prune");
    assert_eq!(report.pruned, 2);
    assert_eq!(
        log.read_all(&audit_permit(), 1..100).expect("live").records,
        []
    );
    let mut text = String::new();
    data.root
        .open_read(&DataPath::audit_segment(AuditSeg::new(1).expect("1")))
        .expect("seg")
        .read_to_string(&mut text)
        .expect("read");
    assert!(text.contains("gm_audit_pruned"));
    drop(log);
    let log = limited(&data, limits);
    assert_eq!(log.verify_audit_log(), Ok(()));
    let page = log.read_all(&audit_permit(), 1..100).expect("reopen");
    let names: Vec<&str> = page.records.iter().map(|r| r.event.as_str()).collect();
    assert_eq!(names, [] as [&str; 0]);
}

/// Verifies: SEC-OPS-023
#[test]
fn a_checkpoint_whose_mac_does_not_match_the_payload_is_detected() {
    let data = data();
    let limits = Limits {
        segment: 4_096,
        checkpoint_every: 2,
        checkpoint_ms: 3_600_000,
        reserve: 256,
    };
    {
        let log = limited(&data, limits);
        let t = at(1_791_028_800_000);
        log.append_security_event(t, &egress(), None, ORDINARY)
            .expect("1");
        log.append_security_event(t, &egress(), None, ORDINARY)
            .expect("2");
    }
    let path = DataPath::audit_segment(AuditSeg::new(1).expect("1"));
    let mut text = String::new();
    data.root
        .open_read(&path)
        .expect("open")
        .read_to_string(&mut text)
        .expect("read");
    let mut rewritten = String::new();
    for line in text.lines() {
        if !line.contains("gm_audit_checkpoint") {
            rewritten.push_str(line);
            rewritten.push('\n');
            continue;
        }
        let map = parse::object(line).expect("object");
        let prev = map
            .get("prev")
            .and_then(Json::str)
            .and_then(unhex32)
            .expect("prev");
        let sig = map.get("sig").and_then(Json::str).expect("sig");
        let prefix = line.rsplit_once(",\"hash\":").expect("hash").0;
        let mut canonical = prefix.replacen(sig, &"0".repeat(64), 1);
        canonical.push('}');
        let hash = chain::digest(&prev, &canonical);
        rewritten.push_str(canonical.trim_end_matches('}'));
        rewritten.push_str(",\"hash\":\"");
        rewritten.push_str(&hex(&hash));
        rewritten.push_str("\"}\n");
    }
    assert!(
        rewritten.contains(
            "\"sig\":\"0000000000000000000000000000000000000000000000000000000000000000\""
        ),
        "{rewritten}"
    );
    data.root
        .replace(&path, rewritten.as_bytes())
        .expect("wrote");
    let reopened = limited(&data, limits);
    assert_eq!(
        reopened.verify_audit_log().expect_err("broken mac"),
        BrokenAt { seq: 3 }
    );
}

fn scan_absent(root: &Path, needle: &[u8]) {
    let mut found = false;
    walk(root, needle, &mut found);
    assert!(!found, "sentinel still on disk");
}

#[expect(
    clippy::disallowed_methods,
    reason = "raw-byte scan of the audit tree (SEC-PRV-003)"
)]
fn walk(path: &Path, needle: &[u8], found: &mut bool) {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return;
    };
    if meta.is_dir() {
        let Ok(entries) = fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            walk(&entry.path(), needle, found);
        }
        return;
    }
    if let Ok(buf) = fs::read(path) {
        *found |= buf.windows(needle.len()).any(|w| w == needle);
    }
}

/// Verifies: SEC-OPS-075
#[test]
fn a_checkpoint_is_signed_and_readable_with_the_audit_permit() {
    let data = data();
    let log = log(&data);
    let t = at(1_791_028_800_000);
    log.append_security_event(t, &egress(), None, ORDINARY)
        .expect("1");
    log.append_security_event(t, &egress(), None, ORDINARY)
        .expect("2");
    let head = log.head(&audit_permit()).expect("head");
    assert!(head.seq >= 1);
    assert_eq!(head.kid, 1);
    let mut payload = Vec::from(b"gunmetal-audit-v1");
    payload.extend_from_slice(&head.seq.to_be_bytes());
    payload.extend_from_slice(&head.head);
    assert_eq!(head.mac, mix(head.kid, &payload));
    assert_eq!(log.verify_audit_log(), Ok(()));
}

#[test]
fn randomness_unavailable_refuses_an_addressed_event() {
    let data = data();
    let log = AuditLog::open_with(
        handle(&data),
        Arc::new(MixMac::new(0)),
        Arc::new(MixMac::new(1)),
        Arc::new(FailingRandom),
        Limits::test(),
    )
    .expect("open");
    assert_eq!(
        log.append_security_event(
            at(1_791_028_800_000),
            &login_ok(),
            Some(&internet()),
            ORDINARY
        ),
        Err(AuditError::Random(
            gunmetal_secrets::random::RandomnessUnavailable
        ))
    );
    assert_eq!(
        log.append_security_event(at(1), &egress(), None, ORDINARY),
        Err(AuditError::Halted)
    );
}

#[test]
fn a_missing_signing_key_halts_at_the_checkpoint() {
    let data = data();
    let log = AuditLog::open_with(
        handle(&data),
        Arc::new(MixMac::new(0)),
        Arc::new(FailingMac),
        Arc::new(Counted::new()),
        Limits::test(),
    )
    .expect("open");
    log.append_security_event(at(1_791_028_800_000), &egress(), None, ORDINARY)
        .expect("1");
    assert_eq!(
        log.append_security_event(at(1_791_028_800_000), &egress(), None, ORDINARY),
        Err(AuditError::MacUnavailable)
    );
}

#[test]
fn reserve_file_is_created_on_open() {
    let data = data();
    let _ = log(&data);
    assert!(data.root.open_read(&AUDIT_RESERVE).is_ok());
}

#[test]
fn ipv6_events_truncate_to_slash_48() {
    let data = data();
    let log = log(&data);
    log.append_security_event(
        at(1_791_028_800_000),
        &login_fail(None),
        Some(&internet_v6()),
        ORDINARY,
    )
    .expect("stored");
    let all = log.read_all(&audit_permit(), 1..10).expect("all");
    assert_eq!(
        all.records[0].addr,
        Some(TruncatedAddr::V6Prefix("2001:db8::/48".to_owned()))
    );
}

#[test]
fn reopening_a_sound_log_keeps_verifying_and_appending() {
    let data = data();
    {
        let log = log(&data);
        log.append_security_event(at(1_791_028_800_000), &egress(), None, ORDINARY)
            .expect("stored");
        assert_eq!(log.verify_audit_log(), Ok(()));
    }
    let log = limited(&data, Limits::test());
    assert_eq!(log.verify_audit_log(), Ok(()));
    assert_eq!(
        log.append_security_event(at(1_791_028_800_001), &egress(), None, ORDINARY),
        Ok(2)
    );
    assert_eq!(log.verify_audit_log(), Ok(()));
}

#[test]
fn reopening_after_a_checkpoint_keeps_the_signed_head() {
    let data = data();
    {
        let log = log(&data);
        let t = at(1_791_028_800_000);
        log.append_security_event(t, &egress(), None, ORDINARY)
            .expect("1");
        log.append_security_event(t, &egress(), None, ORDINARY)
            .expect("2");
        assert_eq!(log.head(&audit_permit()).expect("live").kid, 1);
    }
    let log = limited(&data, Limits::test());
    assert_eq!(log.head(&audit_permit()).expect("reloaded").kid, 1);
    let page = log.read_all(&audit_permit(), 1..100).expect("read");
    let names: Vec<&str> = page.records.iter().map(|r| r.event.as_str()).collect();
    assert_eq!(names, ["gm_egress_denied", "gm_egress_denied"]);
    assert_eq!(log.verify_audit_log(), Ok(()));
}

#[test]
fn a_checkpoint_is_due_after_the_time_limit() {
    let data = data();
    let log = AuditLog::open_with(
        handle(&data),
        Arc::new(MixMac::new(0)),
        Arc::new(MixMac::new(1)),
        Arc::new(Counted::new()),
        Limits {
            segment: 4_096,
            checkpoint_every: 1_000,
            checkpoint_ms: 10,
            reserve: 256,
        },
    )
    .expect("open");
    log.append_security_event(at(1_000), &egress(), None, ORDINARY)
        .expect("1");
    log.append_security_event(at(1_000), &egress(), None, ORDINARY)
        .expect("still no checkpoint");
    log.append_security_event(at(1_011), &egress(), None, ORDINARY)
        .expect("time due");
    let head = log.head(&audit_permit()).expect("head");
    assert_eq!(head.at, at(1_011));
    log.append_security_event(at(1_012), &egress(), None, ORDINARY)
        .expect("not yet");
    assert_eq!(log.head(&audit_permit()).expect("held").at, at(1_011));
    log.append_security_event(at(1_021), &egress(), None, ORDINARY)
        .expect("due again");
    assert_eq!(log.head(&audit_permit()).expect("again").at, at(1_021));
}

#[test]
fn head_without_a_checkpoint_is_corrupt_and_does_not_halt() {
    let data = data();
    let log = log(&data);
    assert_eq!(
        log.head(&audit_permit()),
        Err(AuditError::Corrupt { seq: 0 })
    );
    assert_eq!(
        log.append_security_event(at(1_791_028_800_000), &egress(), None, ORDINARY),
        Ok(1)
    );
}

#[test]
fn a_corrupt_head_file_is_refused_on_open() {
    let data = data();
    let _ = log(&data);
    data.root
        .replace(
            &gunmetal_fs::path::AUDIT_HEAD,
            br#"{"next":1,"seg":1,"bytes":0,"head":"zz","first":1}"#,
        )
        .expect("broke");
    let opened = AuditLog::open_with(
        handle(&data),
        Arc::new(MixMac::new(0)),
        Arc::new(MixMac::new(1)),
        Arc::new(Counted::new()),
        Limits::test(),
    );
    assert_eq!(opened.err(), Some(AuditError::Corrupt { seq: 0 }));
}

#[test]
fn login_fail_without_an_override_uses_the_event_source() {
    let data = data();
    let log = log(&data);
    log.append_security_event(
        at(1_791_028_800_000),
        &login_fail(Some(account())),
        None,
        ORDINARY,
    )
    .expect("stored");
    let own = log.read_own(&own_permit(account()), 1..10).expect("own");
    assert_eq!(own.records[0].addr, Some(internet().addr()));
    assert_eq!(own.records[0].class, Some(PathClass::Internet));
}

/// Verifies: SEC-OPS-023, SEC-IAM-094
#[test]
fn reorder_and_drop_of_lines_are_detected() {
    let data = data();
    let log = limited(
        &data,
        Limits {
            segment: 65_536,
            checkpoint_every: 1_000,
            checkpoint_ms: 3_600_000,
            reserve: 256,
        },
    );
    let t = at(1_791_028_800_000);
    log.append_security_event(t, &egress(), None, ORDINARY)
        .expect("1");
    log.append_security_event(t, &debug_on(), None, ORDINARY)
        .expect("2");
    let path = DataPath::audit_segment(AuditSeg::new(1).expect("1"));
    let mut text = String::new();
    data.root
        .open_read(&path)
        .expect("open")
        .read_to_string(&mut text)
        .expect("read");
    let dropped = text.lines().nth(1).unwrap_or("");
    data.root
        .replace(&path, dropped.as_bytes())
        .expect("dropped first");
    let reopened = AuditLog::open_with(
        handle(&data),
        Arc::new(MixMac::new(0)),
        Arc::new(MixMac::new(1)),
        Arc::new(Counted::new()),
        Limits::test(),
    );
    assert!(
        match reopened {
            Ok(log) => log.verify_audit_log().is_err(),
            Err(_) => true,
        },
        "reversed lines must not verify"
    );
}

proptest! {
    /// Verifies: SEC-OPS-023, SEC-IAM-094
    #[test]
    fn a_byte_flip_in_a_valid_segment_is_detected(index in 0usize..32, bit in 0u8..8) {
        let data = data();
        let log = log(&data);
        let t = at(1_791_028_800_000);
        log.append_security_event(t, &egress(), None, ORDINARY).expect("stored");
        log.append_security_event(t, &egress(), None, ORDINARY).expect("stored");
        prop_assert_eq!(log.verify_audit_log(), Ok(()));
        let path = DataPath::audit_segment(AuditSeg::new(1).expect("1"));
        let mut bytes = Vec::new();
        data.root.open_read(&path).expect("open").read_to_end(&mut bytes).expect("read");
        let i = index.min(bytes.len().saturating_sub(1));
        if let Some(slot) = bytes.get_mut(i) {
            *slot ^= 1u8.wrapping_shl(u32::from(bit));
        }
        data.root.replace(&path, &bytes).expect("wrote");
        let reopened = AuditLog::open_with(
            handle(&data),
            Arc::new(MixMac::new(0)),
            Arc::new(MixMac::new(1)),
            Arc::new(Counted::new()),
            Limits::test(),
        );
        let detected = match reopened {
            Ok(log) => log.verify_audit_log().is_err(),
            Err(_) => true,
        };
        prop_assert!(detected);
    }
}

#[test]
fn audit_error_maps_io_and_randomness() {
    assert_eq!(
        AuditError::from(std::io::Error::from(std::io::ErrorKind::PermissionDenied)),
        AuditError::Io(std::io::ErrorKind::PermissionDenied)
    );
    assert_eq!(
        AuditError::from(gunmetal_secrets::random::RandomnessUnavailable),
        AuditError::Random(gunmetal_secrets::random::RandomnessUnavailable)
    );
}

#[expect(
    clippy::disallowed_methods,
    reason = "chmod of the reserve path proves a failed write refuses the action (SEC-OPS-020)"
)]
#[test]
fn a_read_only_file_refuses_the_append() {
    let data = data();
    let log = log(&data);
    let path = data.dir.path().join("durable/audit");
    let mut perms = fs::metadata(&path).expect("meta").permissions();
    perms.set_mode(0o555);
    fs::set_permissions(&path, perms).expect("ro");
    let result = log.append_security_event(at(1_791_028_800_000), &egress(), None, ORDINARY);
    assert!(matches!(
        result,
        Err(AuditError::Root(_) | AuditError::Io(_))
    ));
}
