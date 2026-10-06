//! What the session layer keeps in the identity store, and what it removes.

use core::net::{IpAddr, Ipv6Addr};
use std::sync::Arc;

use gunmetal_core::authz::{DeviceClass, SessionHandle};
use gunmetal_durable::identity::error::{IdentityError, Step};
use gunmetal_durable::identity::pre_principal::PrePrincipal;
use gunmetal_fs::sqlite::{DbError, Query, Row};

use super::http::{ME, ask, serve, unauthenticated};
use super::world::{
    HOME, KIM, NoRandom, Roster, SAM, TOKEN, WAL, World, account, data, file, from, holds, keys,
    pair, store, text,
};
use crate::session::epoch::EpochScope;
use crate::session::error::SessionError;
use crate::session::lifetime::Lifetime;
use crate::session::sessions::Sessions;
use crate::testing;

/// A public address no other test uses, ending in `last`.
fn peer(last: u16) -> IpAddr {
    IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0x5e, 0x17, 0, 0, 0xc0de, last))
}

/// Verifies: SEC-PRV-003
#[test]
fn a_client_address_leaves_the_store_and_its_log_when_the_session_ends() {
    let world = World::fresh();
    let written = "2001:db8:5e:17::c0de:1dea";
    let (_, issued) =
        world.sign_in_from(SAM, DeviceClass::Personal, Lifetime::Personal, peer(0x1dea));
    // While the session lives its row holds the address the listener
    // resolved, and a scan of the store's bytes finds it.
    assert_eq!(
        world.rows("SELECT address FROM sessions"),
        [Row(vec![text(written)])]
    );
    assert!(holds(&world.on_disk(), written.as_bytes()));
    assert_eq!(world.sessions.end(issued.session), Ok(()));
    // Once it has ended, neither the store's file nor its log holds the
    // address, and the log holds nothing at all.
    assert!(!holds(&world.on_disk(), written.as_bytes()));
    assert_eq!(file(&world.root, &WAL), [0_u8; 0]);
}

/// Verifies: SEC-PRV-003
#[test]
fn every_way_a_session_ends_removes_its_address() {
    let world = World::fresh();
    let (bumped, renewed, expired) = (
        "2001:db8:5e:17::c0de:a1",
        "2001:db8:5e:17::c0de:b2",
        "2001:db8:5e:17::c0de:c3",
    );
    let (_, first) = world.sign_in_from(SAM, DeviceClass::Limited, Lifetime::Personal, peer(0xa1));
    let (_, second) =
        world.sign_in_from(SAM, DeviceClass::Personal, Lifetime::Personal, peer(0xb2));
    let (third, _) = world.sign_in_from(KIM, DeviceClass::Personal, Lifetime::Personal, peer(0xc3));
    let kept = |address: &str| holds(&world.on_disk(), address.as_bytes());
    assert_eq!([bumped, renewed, expired].map(kept), [true, true, true]);
    // A revoked device.
    let revoked = first.device.expect("the session has a device");
    assert_eq!(
        world.sessions.bump_epoch(EpochScope::Device(revoked)),
        Ok(())
    );
    assert_eq!([bumped, renewed, expired].map(kept), [false, true, true]);
    // A new sign-in on the same device, from somewhere else.
    let again = second.device.expect("the session has a device");
    world
        .sessions
        .issue(account(SAM), again, Lifetime::Personal, &from(HOME))
        .expect("the session is issued");
    assert_eq!([bumped, renewed, expired].map(kept), [false, false, true]);
    // A session presented after its time has run out.
    world.manual.advance(604_800_000);
    let (client, _) = serve(&world);
    assert_eq!(ask(&client, &ME, &pair(&third)), unauthenticated());
    assert_eq!([bumped, renewed, expired].map(kept), [false, false, false]);
    assert_eq!(
        world.rows("SELECT address FROM sessions"),
        [Row(vec![text("192.168.1.20")])]
    );
}

#[test]
fn the_session_layer_does_not_open_on_a_store_without_its_tables() {
    let (_dir, root) = data();
    let (clock, _) = testing::clock();
    let opened = Sessions::open(
        store(&root, &[]),
        keys(&root),
        Arc::new(NoRandom),
        clock,
        Arc::new(Roster::default()),
    );
    assert_eq!(
        opened.map(|_| ()),
        Err(SessionError::Store(IdentityError::Db {
            step: Step::PrePrincipal(PrePrincipal::SessionToken),
            // SQLITE_ERROR: no such table
            error: DbError::Sqlite { code: 1 },
        }))
    );
}

#[test]
fn a_device_row_that_does_not_read_back_stops_the_session_layer() {
    for damage in [
        "INSERT INTO devices (device, account, class, enrolled) \
         VALUES ('tv', 'usr_00000000000000000000000001', 'limited', 0)",
        "INSERT INTO devices (device, account, class, enrolled) \
         VALUES (x'00', 'usr_00000000000000000000000001', 'limited', 0)",
    ] {
        let world = World::fresh();
        let (_, issued) = world.sign_in(SAM, DeviceClass::Personal, Lifetime::Personal);
        world.write(&[Query::new(damage)]);
        // A session layer does not start on such a store.
        assert_eq!(world.reopen().map(|_| ()), Err(SessionError::Corrupt));
        // The one already running trusts no epoch from the moment a change
        // shows it the row.
        assert!(world.sessions.is_current(&issued));
        assert_eq!(
            world.sessions.end(SessionHandle(7)),
            Err(SessionError::Corrupt)
        );
        assert!(!world.sessions.is_current(&issued));
    }
}

#[test]
fn the_session_part_makes_two_tables_their_indexes_and_one_trigger() {
    let world = World::fresh();
    assert_eq!(
        world.rows(
            "SELECT type, name FROM sqlite_schema \
             WHERE tbl_name IN ('devices', 'sessions') AND name NOT LIKE 'sqlite\\_%' ESCAPE '\\' \
             ORDER BY type, name"
        ),
        [
            Row(vec![text("index"), text("devices_by_account")]),
            Row(vec![text("index"), text("sessions_by_account")]),
            Row(vec![text("index"), text("sessions_by_device")]),
            Row(vec![text("table"), text("devices")]),
            Row(vec![text("table"), text("sessions")]),
            Row(vec![text("trigger"), text("sessions_kind_is_immutable")]),
        ]
    );
}

#[test]
fn nothing_the_session_layer_formats_holds_a_token() {
    let world = World::fresh();
    let (cookie, issued) = world.sign_in(SAM, DeviceClass::Personal, Lifetime::Personal);
    assert_eq!(
        format!("{cookie:?}"),
        "Cookie { name: \"__Host-gm_session\", .. }"
    );
    assert_eq!(pair(&cookie), format!("__Host-gm_session={TOKEN}"));
    assert!(!format!("{cookie:?} {issued:?}").contains(TOKEN));
}
