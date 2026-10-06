//! Tests for the audit log.

use std::fs;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::{Arc, PoisonError};

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
use gunmetal_fs::path::{AUDIT_DIR, AUDIT_HEAD, AUDIT_RESERVE, AuditSeg, DataPath};
use proptest::prelude::*;

use super::{
    AuditLog, FAIL_ADDR_CKPT, FAIL_CKPT_HEAD, FAIL_CKPT_LINE, FAIL_COMMIT, FAIL_HEAD, FAIL_LINE,
    FAIL_PUT, FAIL_REMOVE, FAIL_RESERVE, FAIL_SHRINK, FAIL_SYNC, FAIL_WRITE, FIRST_SEG, Kind,
    Limits, arm_fail, checkpoint_mac_from_raw, ensure_reserve, load_head, load_segments,
    parse_line, persist_head, shrink_reserve, write_line,
};
use crate::audit::chain;
use crate::audit::encode::{hex, unhex32};
use crate::audit::error::{AuditError, BrokenAt};
use crate::audit::parse::{self, Json};
use crate::audit::record::{Outcome, TruncatedAddr, WriteClass};
use crate::audit::testing::{
    Counted, FailingMac, FailingRandom, MixMac, ORDINARY, account, data, handle, internet,
    internet_v6, limited, log, loopback, mix, proxied,
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

/// Verifies: SEC-PRV-003, SEC-PRV-005
#[test]
fn an_addressed_record_stores_an_hmac_commitment_not_the_address() {
    let data = data();
    let log = log(&data);
    let t = at(1_791_028_800_000);
    log.append_security_event(t, &login_ok(), Some(&internet()), ORDINARY)
        .expect("stored");
    let salt: [u8; 16] = core::array::from_fn(|index| u8::try_from(index).unwrap_or(0));
    let tag = mix(
        0,
        &crate::audit::addresses::commitment_msg(internet().addr(), &salt),
    );
    let mut text = String::new();
    data.root
        .open_read(&DataPath::audit_segment(AuditSeg::new(1).expect("1")))
        .expect("segment")
        .read_to_string(&mut text)
        .expect("read");
    assert!(
        text.contains(&format!("\"commit\":\"{}\"", hex(&tag))),
        "{text}"
    );
    assert!(!text.contains("203.0.113.7"), "{text}");
}

#[test]
fn a_missing_address_key_refuses_an_addressed_append() {
    let data = data();
    let log = AuditLog::open_with(
        handle(&data),
        Arc::new(FailingMac),
        Arc::new(MixMac::new(1)),
        Arc::new(Counted::new()),
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
        Err(AuditError::MacUnavailable)
    );
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

#[test]
fn a_proxied_request_is_recorded_as_via_proxy() {
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
    log.append_security_event(
        at(1_791_028_800_000),
        &login_fail(None),
        Some(&proxied()),
        ORDINARY,
    )
    .expect("stored");
    let mut text = String::new();
    data.root
        .open_read(&DataPath::audit_segment(AuditSeg::new(1).expect("1")))
        .expect("segment")
        .read_to_string(&mut text)
        .expect("read");
    assert!(text.contains("\"via\":\"proxy\""));
    assert!(!text.contains("127.0.0.1"));
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
    assert_eq!(
        log.read_all(&audit_permit(), 1..10),
        Err(AuditError::Halted)
    );
    assert_eq!(
        log.read_own(&own_permit(account()), 1..10),
        Err(AuditError::Halted)
    );
    assert_eq!(log.head(&audit_permit()), Err(AuditError::Halted));
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
    let limits = Limits {
        segment: 4_096,
        checkpoint_every: 1_000,
        checkpoint_ms: 10,
        reserve: 256,
    };
    let log = AuditLog::open_with(
        handle(&data),
        Arc::new(MixMac::new(0)),
        Arc::new(MixMac::new(1)),
        Arc::new(Counted::new()),
        limits,
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
    log.append_security_event(at(1_022), &egress(), None, ORDINARY)
        .expect("trail");
    assert_eq!(log.head(&audit_permit()).expect("trail held").at, at(1_021));
    drop(log);
    let log = limited(&data, limits);
    assert_eq!(log.head(&audit_permit()).expect("reloaded").at, at(1_021));
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

#[test]
fn open_uses_the_production_limits() {
    let data = data();
    let log = AuditLog::open(
        handle(&data),
        Arc::new(MixMac::new(0)),
        Arc::new(MixMac::new(1)),
        Arc::new(Counted::new()),
    )
    .expect("open");
    assert_eq!(
        log.append_security_event(at(1_791_028_800_000), &egress(), None, ORDINARY),
        Ok(1)
    );
    assert_eq!(log.verify_audit_log(), Ok(()));
}

#[test]
fn a_head_without_first_starts_at_one() {
    let data = data();
    let _ = log(&data);
    data.root
        .replace(
            &gunmetal_fs::path::AUDIT_HEAD,
            br#"{"next":1,"seg":1,"bytes":0,"head":"0000000000000000000000000000000000000000000000000000000000000000"}"#,
        )
        .expect("wrote");
    let opened = AuditLog::open_with(
        handle(&data),
        Arc::new(MixMac::new(0)),
        Arc::new(MixMac::new(1)),
        Arc::new(Counted::new()),
        Limits::test(),
    )
    .expect("open");
    assert_eq!(
        opened.append_security_event(at(1_791_028_800_000), &egress(), None, ORDINARY),
        Ok(1)
    );
}

#[test]
fn blank_lines_in_a_segment_are_skipped_on_reload() {
    let data = data();
    {
        let log = log(&data);
        log.append_security_event(at(1_791_028_800_000), &egress(), None, ORDINARY)
            .expect("stored");
    }
    let path = DataPath::audit_segment(AuditSeg::new(1).expect("1"));
    let mut text = String::new();
    data.root
        .open_read(&path)
        .expect("open")
        .read_to_string(&mut text)
        .expect("read");
    let with_blank = format!("\n{text}\n");
    data.root
        .replace(&path, with_blank.as_bytes())
        .expect("wrote");
    let log = limited(&data, Limits::test());
    assert_eq!(log.verify_audit_log(), Ok(()));
    assert_eq!(
        log.read_all(&audit_permit(), 1..10)
            .expect("read")
            .records
            .len(),
        1
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

#[test]
fn a_checkpoint_count_of_zero_never_checkpoints_on_count() {
    let data = data();
    let log = limited(
        &data,
        Limits {
            segment: 65_536,
            checkpoint_every: 0,
            checkpoint_ms: 3_600_000,
            reserve: 256,
        },
    );
    log.append_security_event(at(1_791_028_800_000), &egress(), None, ORDINARY)
        .expect("1");
    log.append_security_event(at(1_791_028_800_000), &egress(), None, ORDINARY)
        .expect("2");
    assert_eq!(
        log.head(&audit_permit()),
        Err(AuditError::Corrupt { seq: 0 })
    );
}

#[test]
fn retention_at_the_event_time_keeps_addresses() {
    let data = data();
    let log = log(&data);
    let t0 = at(1_791_028_800_000);
    log.append_security_event(t0, &login_ok(), Some(&internet()), ORDINARY)
        .expect("stored");
    let report = log.apply_retention(&DEFAULT, t0).expect("keep");
    assert_eq!(report.coarsened, 0);
    assert_eq!(report.removed, 0);
    assert_eq!(report.pruned, 0);
}

#[test]
fn coarsening_a_row_without_an_address_is_a_no_op() {
    let data = data();
    let log = log(&data);
    let t0 = at(1_791_028_800_000);
    log.append_security_event(t0, &login_ok(), Some(&internet()), ORDINARY)
        .expect("stored");
    let db = crate::audit::addresses::open(&data.root).expect("db");
    db.execute(&gunmetal_fs::sqlite::Query::new(
        "UPDATE addresses SET addr = NULL WHERE seq = 1",
    ))
    .expect("null addr");
    let day = 86_400_000_i64;
    let report = log
        .apply_retention(&DEFAULT, at(t0.millis() + 31 * day))
        .expect("coarsen");
    assert_eq!(report.coarsened, 0);
}

#[test]
fn a_head_ahead_of_the_segments_still_opens() {
    let data = data();
    let _ = log(&data);
    data.root
        .replace(
            &gunmetal_fs::path::AUDIT_HEAD,
            br#"{"next":5,"seg":2,"bytes":0,"head":"0000000000000000000000000000000000000000000000000000000000000000","first":5}"#,
        )
        .expect("wrote");
    let opened = limited(&data, Limits::test());
    assert_eq!(opened.verify_audit_log(), Ok(()));
    assert_eq!(
        opened.append_security_event(at(1_791_028_800_000), &egress(), None, ORDINARY),
        Ok(5)
    );
}

#[test]
fn an_event_without_an_outcome_reads_as_denied() {
    let data = data();
    {
        let log = log(&data);
        log.append_security_event(at(1_791_028_800_000), &egress(), None, ORDINARY)
            .expect("stored");
    }
    let path = DataPath::audit_segment(AuditSeg::new(1).expect("1"));
    let mut text = String::new();
    data.root
        .open_read(&path)
        .expect("open")
        .read_to_string(&mut text)
        .expect("read");
    let line = text.lines().next().expect("line");
    let stripped = line.replace(",\"outcome\":\"denied\"", "");
    let prefix = stripped.rsplit_once(",\"hash\":").expect("hash").0;
    let mut canonical = prefix.to_owned();
    canonical.push('}');
    let prev = [0_u8; 32];
    let hash = chain::digest(&prev, &canonical);
    let rewritten = format!(
        "{},\"hash\":\"{}\"}}\n",
        canonical.trim_end_matches('}'),
        hex(&hash)
    );
    data.root
        .replace(&path, rewritten.as_bytes())
        .expect("wrote");
    let log = limited(&data, Limits::test());
    let page = log.read_all(&audit_permit(), 1..10).expect("read");
    assert_eq!(page.records[0].outcome, Outcome::Denied);
    let own = log.read_own(&own_permit(account()), 1..10).expect("own");
    assert_eq!(own.records, []);
}

#[test]
fn a_pruned_record_without_a_signature_fails_verify() {
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
    let day = 86_400_000_i64;
    log.apply_retention(&DEFAULT, at(t0.millis() + 366 * day))
        .expect("prune");
    drop(log);
    let path = DataPath::audit_segment(AuditSeg::new(1).expect("1"));
    let mut text = String::new();
    data.root
        .open_read(&path)
        .expect("open")
        .read_to_string(&mut text)
        .expect("read");
    let mut rewritten = String::new();
    for line in text.lines() {
        if !line.contains("gm_audit_pruned") {
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
        let prefix = line.rsplit_once(",\"hash\":").expect("hash").0;
        let without_sig = prefix.replace(
            &format!(
                "\"sig\":\"{}\"",
                map.get("sig").and_then(Json::str).expect("sig")
            ),
            "",
        );
        let without_sig = without_sig.replace(",,", ",");
        let mut canonical = without_sig.trim_end_matches(',').to_owned();
        if !canonical.ends_with('}') {
            canonical.push('}');
        }
        let hash = chain::digest(&prev, &canonical);
        rewritten.push_str(canonical.trim_end_matches('}'));
        rewritten.push_str(",\"hash\":\"");
        rewritten.push_str(&hex(&hash));
        rewritten.push_str("\"}\n");
    }
    data.root
        .replace(&path, rewritten.as_bytes())
        .expect("wrote");
    let reopened = limited(&data, limits);
    assert!(reopened.verify_audit_log().is_err());
}

#[test]
fn a_corrupt_head_object_is_refused_on_open() {
    let data = data();
    let _ = log(&data);
    data.root
        .replace(&gunmetal_fs::path::AUDIT_HEAD, br#"{"seg":1}"#)
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
fn a_line_with_hash_first_fails_verify() {
    let data = data();
    let zeros = "0".repeat(64);
    let line = format!(
        r#"{{"hash":"{zeros}","seq":1,"ts":"2026-10-03T12:00:00.000Z","event":"gm_egress_denied","outcome":"denied","prev":"{zeros}"}}"#
    );
    let _ = log(&data);
    data.root
        .replace(
            &DataPath::audit_segment(AuditSeg::new(1).expect("1")),
            format!("{line}\n").as_bytes(),
        )
        .expect("wrote");
    data.root
        .replace(
            &gunmetal_fs::path::AUDIT_HEAD,
            format!(
                r#"{{"next":2,"seg":1,"bytes":{},"head":"{zeros}","first":1}}"#,
                line.len().saturating_add(1)
            )
            .as_bytes(),
        )
        .expect("head");
    let opened = limited(&data, Limits::test());
    assert_eq!(opened.verify_audit_log(), Err(BrokenAt { seq: 1 }));
}

#[test]
fn a_line_with_the_wrong_predecessor_fails_verify() {
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
    log.append_security_event(at(1_791_028_800_000), &egress(), None, ORDINARY)
        .expect("1");
    log.append_security_event(at(1_791_028_800_000), &egress(), None, ORDINARY)
        .expect("2");
    let path = DataPath::audit_segment(AuditSeg::new(1).expect("1"));
    let mut text = String::new();
    data.root
        .open_read(&path)
        .expect("open")
        .read_to_string(&mut text)
        .expect("read");
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines.len() >= 2);
    let zeros = "0".repeat(64);
    let second = lines[1].replacen(
        &format!(
            "\"prev\":\"{}\"",
            parse::object(lines[1])
                .expect("obj")
                .get("prev")
                .and_then(Json::str)
                .expect("prev")
        ),
        &format!("\"prev\":\"{zeros}\""),
        1,
    );
    let rewritten = format!("{}\n{second}\n", lines[0]);
    drop(lines);
    data.root
        .replace(&path, rewritten.as_bytes())
        .expect("wrote");
    let opened = limited(&data, Limits::test());
    assert_eq!(opened.verify_audit_log(), Err(BrokenAt { seq: 2 }));
}

#[test]
fn a_head_past_the_last_line_fails_verify() {
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
    log.append_security_event(at(1_791_028_800_000), &egress(), None, ORDINARY)
        .expect("1");
    drop(log);
    let mut head = String::new();
    data.root
        .open_read(&gunmetal_fs::path::AUDIT_HEAD)
        .expect("head")
        .read_to_string(&mut head)
        .expect("read");
    let bumped = head.replacen("\"next\":2", "\"next\":4", 1);
    data.root
        .replace(&gunmetal_fs::path::AUDIT_HEAD, bumped.as_bytes())
        .expect("head");
    let opened = limited(&data, Limits::test());
    assert_eq!(opened.verify_audit_log(), Err(BrokenAt { seq: 2 }));
}

#[test]
fn a_checkpoint_without_a_signing_key_fails_verify() {
    let data = data();
    let log = limited(
        &data,
        Limits {
            segment: 65_536,
            checkpoint_every: 1,
            checkpoint_ms: 3_600_000,
            reserve: 256,
        },
    );
    log.append_security_event(at(1_791_028_800_000), &egress(), None, ORDINARY)
        .expect("1");
    drop(log);
    let opened = AuditLog::open_with(
        handle(&data),
        Arc::new(MixMac::new(0)),
        Arc::new(FailingMac),
        Arc::new(Counted::new()),
        Limits::test(),
    )
    .expect("reopen");
    assert!(opened.verify_audit_log().is_err());
}

#[test]
fn optional_line_fields_can_be_absent_or_invalid() {
    let data = data();
    let zeros = "0".repeat(64);
    let line = format!(
        r#"{{"seq":1,"ts":"2026-10-03T12:00:00.000Z","event":"gm_egress_denied","actor":{{}},"source":{{"class":"spaceship","commit":"zz","kid":300}},"kid":300,"outcome":"nope","prev":"{zeros}","hash":"{zeros}"}}"#
    );
    let _ = log(&data);
    data.root
        .replace(
            &DataPath::audit_segment(AuditSeg::new(1).expect("1")),
            format!("{line}\n").as_bytes(),
        )
        .expect("wrote");
    data.root
        .replace(
            &gunmetal_fs::path::AUDIT_HEAD,
            format!(
                r#"{{"next":2,"seg":1,"bytes":{},"head":"{zeros}","first":1}}"#,
                line.len().saturating_add(1)
            )
            .as_bytes(),
        )
        .expect("head");
    let opened = limited(&data, Limits::test());
    let page = opened.read_all(&audit_permit(), 1..10).expect("read");
    assert_eq!(page.records[0].account, None);
    assert_eq!(page.records[0].class, None);
    assert_eq!(page.records[0].outcome, Outcome::Denied);
}

#[test]
fn a_line_missing_a_required_field_refuses_open() {
    let data = data();
    let zeros = "0".repeat(64);
    let line =
        format!(r#"{{"seq":1,"event":"gm_egress_denied","prev":"{zeros}","hash":"{zeros}"}}"#);
    let _ = log(&data);
    data.root
        .replace(
            &DataPath::audit_segment(AuditSeg::new(1).expect("1")),
            format!("{line}\n").as_bytes(),
        )
        .expect("wrote");
    data.root
        .replace(
            &gunmetal_fs::path::AUDIT_HEAD,
            format!(
                r#"{{"next":2,"seg":1,"bytes":{},"head":"{zeros}","first":1}}"#,
                line.len().saturating_add(1)
            )
            .as_bytes(),
        )
        .expect("head");
    assert_eq!(
        AuditLog::open_with(
            handle(&data),
            Arc::new(MixMac::new(0)),
            Arc::new(MixMac::new(1)),
            Arc::new(Counted::new()),
            Limits::test(),
        )
        .err(),
        Some(AuditError::Corrupt { seq: 1 })
    );
}

#[test]
fn parse_line_covers_required_and_optional_fields() {
    let zeros = "0".repeat(64);
    let ts = "2026-10-03T12:00:00.000Z";
    let account_id = account().to_string();
    assert_eq!(parse_line("{}").err(), Some(AuditError::Corrupt { seq: 0 }));
    assert_eq!(
        parse_line(&format!(
            r#"{{"seq":1,"event":"gm_egress_denied","prev":"{zeros}","hash":"{zeros}"}}"#
        ))
        .err(),
        Some(AuditError::Corrupt { seq: 1 })
    );
    assert_eq!(
        parse_line(&format!(
            r#"{{"seq":1,"ts":"not-a-time","event":"gm_egress_denied","prev":"{zeros}","hash":"{zeros}"}}"#
        ))
        .err(),
        Some(AuditError::Corrupt { seq: 1 })
    );
    assert_eq!(
        parse_line(&format!(
            r#"{{"seq":1,"ts":"{ts}","prev":"{zeros}","hash":"{zeros}"}}"#
        ))
        .err(),
        Some(AuditError::Corrupt { seq: 1 })
    );
    assert_eq!(
        parse_line(&format!(
            r#"{{"seq":1,"ts":"{ts}","event":"gm_egress_denied","prev":"{zeros}"}}"#
        ))
        .err(),
        Some(AuditError::Corrupt { seq: 1 })
    );
    assert_eq!(
        parse_line(&format!(
            r#"{{"seq":1,"ts":"{ts}","event":"gm_egress_denied","hash":"{zeros}"}}"#
        ))
        .err(),
        Some(AuditError::Corrupt { seq: 1 })
    );
    let with_account = parse_line(&format!(
        r#"{{"seq":1,"ts":"{ts}","event":"gm_egress_denied","actor":{{"account":"{account_id}"}},"source":{{}},"prev":"{zeros}","hash":"{zeros}"}}"#
    ))
    .expect("account");
    assert_eq!(with_account.account, Some(account()));
    assert_eq!(with_account.class, None);
    assert_eq!(with_account.commit, None);
    assert_eq!(with_account.kid, None);
    let bad_account = parse_line(&format!(
        r#"{{"seq":1,"ts":"{ts}","event":"gm_egress_denied","actor":{{"account":"nope"}},"prev":"{zeros}","hash":"{zeros}"}}"#
    ))
    .expect("bad account");
    assert_eq!(bad_account.account, None);
    let checkpoint = parse_line(&format!(
        r#"{{"seq":2,"ts":"{ts}","event":"gm_audit_checkpoint","prev":"{zeros}","hash":"{zeros}"}}"#
    ))
    .expect("checkpoint");
    assert_eq!(checkpoint.kind, Kind::Checkpoint);
    let pruned = parse_line(&format!(
        r#"{{"seq":3,"ts":"{ts}","event":"gm_audit_pruned","prev":"{zeros}","hash":"{zeros}"}}"#
    ))
    .expect("pruned");
    assert_eq!(pruned.kind, Kind::Pruned);
}

#[expect(
    clippy::disallowed_methods,
    reason = "replace a segment with a directory so append fails as Io (SEC-OPS-020)"
)]
#[test]
fn write_line_and_head_io_failures_are_typed() {
    let data = data();
    let log = log(&data);
    log.append_security_event(at(1_791_028_800_000), &egress(), None, ORDINARY)
        .expect("1");
    {
        let mut state = log.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.seg = AuditSeg::new(AuditSeg::MAX).expect("max");
        state.bytes = 400;
        assert_eq!(
            write_line(&log.root, &mut state, &Limits::test(), "{}"),
            Err(AuditError::Corrupt { seq: state.next })
        );
        state.seg = FIRST_SEG;
        state.bytes = 0;
        assert!(write_line(&log.root, &mut state, &Limits::test(), "{}").is_err());
    }
    let path = data.dir.path().join("durable/audit/s-00000001.jsonl");
    fs::remove_file(&path).expect("gone");
    fs::create_dir(&path).expect("dir");
    {
        let mut state = log.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.seg = FIRST_SEG;
        state.bytes = 10;
        assert!(write_line(&log.root, &mut state, &Limits::test(), "{}").is_err());
    }
}

#[expect(
    clippy::disallowed_methods,
    reason = "chmod of the audit directory proves a failed reserve write is typed (SEC-OPS-020)"
)]
#[test]
fn reserve_and_head_paths_that_are_directories_refuse_writes() {
    let data = data();
    drop(data.root.create_dir(&AUDIT_DIR));
    let audit = data.dir.path().join("durable/audit");
    let mut perms = fs::metadata(&audit).expect("meta").permissions();
    perms.set_mode(0o555);
    fs::set_permissions(&audit, perms).expect("ro");
    assert!(ensure_reserve(&data.root, 256).is_err());
    let mut perms = fs::metadata(&audit).expect("meta").permissions();
    perms.set_mode(0o700);
    fs::set_permissions(&audit, perms).expect("rw");
    let log = log(&data);
    let head = data.dir.path().join("durable/audit/head.json");
    fs::remove_file(&head).expect("head");
    fs::create_dir(&head).expect("head dir");
    {
        let state = log.state.lock().unwrap_or_else(PoisonError::into_inner);
        assert!(persist_head(&log.root, &state).is_err());
    }
    fs::remove_dir(&head).expect("head gone");
    let reserve = data.dir.path().join("durable/audit/reserve.bin");
    fs::remove_file(&reserve).expect("reserve");
    fs::create_dir(&reserve).expect("reserve dir");
    assert!(shrink_reserve(&log.root).is_err());
}

#[test]
fn load_head_and_segments_refuse_corrupt_bytes() {
    let data = data();
    let log = log(&data);
    let db = crate::audit::addresses::open(&handle(&data)).expect("db");
    assert_eq!(
        load_head("{}", db).err(),
        Some(AuditError::Corrupt { seq: 0 })
    );
    let db = crate::audit::addresses::open(&handle(&data)).expect("db");
    assert_eq!(
        load_head(r#"{"next":1,"seg":1}"#, db).err(),
        Some(AuditError::Corrupt { seq: 0 })
    );
    let db = crate::audit::addresses::open(&handle(&data)).expect("db");
    assert_eq!(
        load_head(r#"{"next":1,"seg":1,"bytes":x}"#, db).err(),
        Some(AuditError::Corrupt { seq: 0 })
    );
    let db = crate::audit::addresses::open(&handle(&data)).expect("db");
    assert_eq!(
        load_head(r#"{"next":1,"seg":0,"bytes":1,"head":"00","first":1}"#, db).err(),
        Some(AuditError::Corrupt { seq: 0 })
    );
    data.root
        .replace(&AUDIT_HEAD, &[0xff, 0xfe])
        .expect("bytes");
    assert!(
        AuditLog::open_with(
            handle(&data),
            Arc::new(MixMac::new(0)),
            Arc::new(MixMac::new(1)),
            Arc::new(Counted::new()),
            Limits::test(),
        )
        .is_err()
    );
    let path = DataPath::audit_segment(FIRST_SEG);
    data.root.replace(&path, &[0xff, 0xfe]).expect("seg");
    {
        let mut state = log.state.lock().unwrap_or_else(PoisonError::into_inner);
        assert!(load_segments(&log.root, &mut state).is_err());
    }
    assert_eq!(checkpoint_mac_from_raw("{}"), None);
    assert_eq!(checkpoint_mac_from_raw(r#"{"sig":"zz"}"#), None);
    assert_eq!(checkpoint_mac_from_raw("not-json"), None);
}

#[expect(
    clippy::disallowed_methods,
    reason = "chmod and plant a junk addresses database so open_with fails after the side store (SEC-OPS-020)"
)]
#[test]
fn open_with_fails_when_the_side_store_or_reserve_cannot_be_used() {
    {
        let data = data();
        drop(data.root.create_dir(&AUDIT_DIR));
        data.root
            .replace(crate::audit::addresses::ADDRESSES.path(), b"not sqlite")
            .expect("junk");
        assert!(
            AuditLog::open_with(
                handle(&data),
                Arc::new(MixMac::new(0)),
                Arc::new(MixMac::new(1)),
                Arc::new(Counted::new()),
                Limits::test(),
            )
            .is_err()
        );
    }
    let data = data();
    let _ = log(&data);
    let reserve = data.dir.path().join("durable/audit/reserve.bin");
    fs::remove_file(&reserve).expect("reserve");
    let audit = data.dir.path().join("durable/audit");
    let mut perms = fs::metadata(&audit).expect("meta").permissions();
    perms.set_mode(0o555);
    fs::set_permissions(&audit, perms).expect("ro");
    assert!(
        AuditLog::open_with(
            handle(&data),
            Arc::new(MixMac::new(0)),
            Arc::new(MixMac::new(1)),
            Arc::new(Counted::new()),
            Limits::test(),
        )
        .is_err()
    );
}

#[test]
fn retention_reports_a_failed_coarsen_or_remove() {
    let data = data();
    let log = log(&data);
    let t0 = at(1_791_028_800_000);
    log.append_security_event(t0, &login_ok(), Some(&internet()), ORDINARY)
        .expect("stored");
    {
        let state = log.state.lock().unwrap_or_else(PoisonError::into_inner);
        state
            .db
            .execute(&gunmetal_fs::sqlite::Query::new(
                "CREATE TRIGGER fail_up BEFORE UPDATE ON addresses \
                 BEGIN SELECT RAISE(ABORT, 'x'); END;",
            ))
            .expect("trigger");
    }
    let day = 86_400_000_i64;
    assert!(
        log.apply_retention(&DEFAULT, at(t0.millis() + 31 * day))
            .is_err()
    );
}

#[test]
fn a_dropped_address_table_fails_reads_and_retention() {
    let data = data();
    let log = log(&data);
    log.append_security_event(
        at(1_791_028_800_000),
        &login_ok(),
        Some(&internet()),
        ORDINARY,
    )
    .expect("1");
    {
        let state = log.state.lock().unwrap_or_else(PoisonError::into_inner);
        state
            .db
            .execute(&gunmetal_fs::sqlite::Query::new("DROP TABLE addresses"))
            .expect("drop");
    }
    assert!(log.read_own(&own_permit(account()), 1..10).is_err());
    assert!(log.read_all(&audit_permit(), 1..10).is_err());
    assert!(
        log.apply_retention(&DEFAULT, at(1_791_028_800_000))
            .is_err()
    );
    assert!(
        log.append_security_event(
            at(1_791_028_800_000),
            &login_ok(),
            Some(&internet()),
            ORDINARY
        )
        .is_err()
    );
}

#[test]
fn a_missing_signing_key_refuses_a_prune_checkpoint() {
    let data = data();
    let log = AuditLog::open_with(
        handle(&data),
        Arc::new(MixMac::new(0)),
        Arc::new(FailingMac),
        Arc::new(Counted::new()),
        Limits {
            segment: 65_536,
            checkpoint_every: 0,
            checkpoint_ms: 3_600_000,
            reserve: 256,
        },
    )
    .expect("open");
    let t0 = at(1_791_028_800_000);
    log.append_security_event(t0, &egress(), None, ORDINARY)
        .expect("1");
    let day = 86_400_000_i64;
    assert_eq!(
        log.apply_retention(&DEFAULT, at(t0.millis() + 366 * day)),
        Err(AuditError::MacUnavailable)
    );
}

#[expect(
    clippy::disallowed_methods,
    reason = "plant a directory at head.json so a fresh open cannot persist it (SEC-OPS-020)"
)]
#[test]
fn a_directory_where_the_head_should_be_refuses_a_fresh_open() {
    let data = data();
    drop(data.root.create_dir(&AUDIT_DIR));
    fs::create_dir(data.dir.path().join("durable/audit/head.json")).expect("head dir");
    assert!(
        AuditLog::open_with(
            handle(&data),
            Arc::new(MixMac::new(0)),
            Arc::new(MixMac::new(1)),
            Arc::new(Counted::new()),
            Limits::test(),
        )
        .is_err()
    );
}

fn open_fails(bit: u32) {
    let data = crate::audit::testing::data();
    arm_fail(bit);
    assert!(
        AuditLog::open_with(
            handle(&data),
            Arc::new(MixMac::new(0)),
            Arc::new(MixMac::new(1)),
            Arc::new(Counted::new()),
            Limits::test(),
        )
        .is_err()
    );
}

fn append_fails(bit: u32) {
    let data = crate::audit::testing::data();
    let opened = log(&data);
    arm_fail(bit);
    assert!(
        opened
            .append_security_event(at(1_791_028_800_000), &egress(), None, ORDINARY)
            .is_err()
    );
}

fn addressed_append_fails(bit: u32) {
    let data = crate::audit::testing::data();
    let opened = log(&data);
    arm_fail(bit);
    assert!(
        opened
            .append_security_event(
                at(1_791_028_800_000),
                &login_ok(),
                Some(&internet()),
                ORDINARY
            )
            .is_err()
    );
}

fn checkpoint_fails(bit: u32) {
    let data = crate::audit::testing::data();
    let opened = log(&data);
    let t0 = at(1_791_028_800_000);
    opened
        .append_security_event(t0, &egress(), None, ORDINARY)
        .expect("1");
    arm_fail(bit);
    assert!(
        opened
            .append_security_event(at(t0.millis() + 1), &egress(), None, ORDINARY)
            .is_err()
    );
}

#[test]
fn injected_io_failures_are_typed_at_each_call_site() {
    open_fails(FAIL_RESERVE);
    open_fails(FAIL_HEAD);
    append_fails(FAIL_HEAD);
    let data = crate::audit::testing::data();
    let opened = log(&data);
    opened
        .append_security_event(at(1_791_028_800_000), &egress(), None, ORDINARY)
        .expect("1");
    arm_fail(FAIL_HEAD);
    assert!(
        opened
            .append_security_event(at(1_791_028_800_001), &egress(), None, ORDINARY)
            .is_err()
    );
    append_fails(FAIL_WRITE);
    append_fails(FAIL_SYNC);
    append_fails(FAIL_LINE);
    let data = crate::audit::testing::data();
    let opened = log(&data);
    opened.simulate_full_disk();
    arm_fail(FAIL_SHRINK);
    assert!(
        opened
            .append_security_event(
                at(1_791_028_800_000),
                &debug_on(),
                None,
                WriteClass::Recovery
            )
            .is_err()
    );
    addressed_append_fails(FAIL_COMMIT);
    addressed_append_fails(FAIL_PUT);
    let t0 = at(1_791_028_800_000);
    let data = crate::audit::testing::data();
    let opened = log(&data);
    opened
        .append_security_event(t0, &login_ok(), Some(&internet()), ORDINARY)
        .expect("addr");
    arm_fail(FAIL_REMOVE);
    assert!(
        opened
            .apply_retention(&DEFAULT, at(t0.millis() + 91 * 86_400_000))
            .is_err()
    );
    let data = crate::audit::testing::data();
    let opened = log(&data);
    opened
        .append_security_event(t0, &login_ok(), Some(&internet()), ORDINARY)
        .expect("addr");
    arm_fail(FAIL_ADDR_CKPT);
    assert!(opened.apply_retention(&DEFAULT, t0).is_err());
    let data = crate::audit::testing::data();
    let opened = log(&data);
    opened
        .append_security_event(t0, &egress(), None, ORDINARY)
        .expect("1");
    arm_fail(FAIL_HEAD);
    assert!(
        opened
            .apply_retention(&DEFAULT, at(t0.millis() + 366 * 86_400_000))
            .is_err()
    );
    let data = crate::audit::testing::data();
    let opened = log(&data);
    opened
        .append_security_event(t0, &egress(), None, ORDINARY)
        .expect("1");
    arm_fail(FAIL_LINE);
    assert!(
        opened
            .apply_retention(&DEFAULT, at(t0.millis() + 366 * 86_400_000))
            .is_err()
    );
    checkpoint_fails(FAIL_CKPT_LINE);
    checkpoint_fails(FAIL_CKPT_HEAD);
}
