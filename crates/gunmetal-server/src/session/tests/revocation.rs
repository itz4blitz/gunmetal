//! Ending sessions: signing out, epoch bumps, and what the session layer
//! holds in memory about them.

use gunmetal_core::authz::{DeviceClass, SessionHandle};
use gunmetal_durable::identity::error::{IdentityError, Step};
use gunmetal_durable::identity::store::IDENTITY;
use gunmetal_fs::sqlite::{DbError, Pragmas, Query, Row, Synchronous, Value, open_db};

use super::http::{ME, ask, granted, internal, serve, unauthenticated};
use super::world::{HOME, KIM, SAM, World, account, device, from, pair, text};
use crate::session::epoch::EpochScope;
use crate::session::error::SessionError;
use crate::session::lifetime::Lifetime;

/// Each device's account and epoch, lowest epoch first.
const EPOCHS: &str = "SELECT account, epoch FROM devices ORDER BY epoch, account";

/// Verifies: SEC-IAM-043, SEC-API-017, SEC-TM-028
#[test]
fn bumping_an_accounts_epoch_fails_its_next_request_on_every_device() {
    let world = World::fresh();
    let (phone, on_phone) = world.sign_in(SAM, DeviceClass::Personal, Lifetime::Personal);
    let (tv, on_tv) = world.sign_in(SAM, DeviceClass::Limited, Lifetime::Personal);
    let (other, on_other) = world.sign_in(KIM, DeviceClass::Personal, Lifetime::Personal);
    let (client, _) = serve(&world);
    for cookie in [&phone, &tv, &other] {
        assert_eq!(ask(&client, &ME, &pair(cookie)), granted());
    }
    assert_eq!(
        [&on_phone, &on_tv, &on_other].map(|principal| world.sessions.is_current(principal)),
        [true, true, true]
    );
    assert_eq!(
        world.sessions.bump_epoch(EpochScope::Account(account(SAM))),
        Ok(())
    );
    // The very next request of each of the account's sessions is refused,
    // and whoever still holds one of its principals is told so at once.
    assert_eq!(ask(&client, &ME, &pair(&phone)), unauthenticated());
    assert_eq!(ask(&client, &ME, &pair(&tv)), unauthenticated());
    assert_eq!(ask(&client, &ME, &pair(&other)), granted());
    assert_eq!(
        [&on_phone, &on_tv, &on_other].map(|principal| world.sessions.is_current(principal)),
        [false, false, true]
    );
    assert_eq!(
        world.rows("SELECT account FROM sessions"),
        [Row(vec![text(KIM)])]
    );
    assert_eq!(
        world.rows("SELECT account, epoch FROM devices ORDER BY account, epoch"),
        [
            Row(vec![text(SAM), Value::Integer(2)]),
            Row(vec![text(SAM), Value::Integer(2)]),
            Row(vec![text(KIM), Value::Integer(1)]),
        ]
    );
}

/// Verifies: SEC-IAM-043, SEC-API-017, SEC-TM-028
#[test]
fn revoking_one_device_leaves_the_others() {
    let world = World::fresh();
    let (phone, on_phone) = world.sign_in(SAM, DeviceClass::Personal, Lifetime::Personal);
    let (tv, on_tv) = world.sign_in(SAM, DeviceClass::Limited, Lifetime::Personal);
    let (client, _) = serve(&world);
    let revoked = on_tv.device.expect("the session has a device");
    assert_eq!(
        world.sessions.bump_epoch(EpochScope::Device(revoked)),
        Ok(())
    );
    assert_eq!(ask(&client, &ME, &pair(&tv)), unauthenticated());
    assert_eq!(ask(&client, &ME, &pair(&phone)), granted());
    assert_eq!(
        [&on_phone, &on_tv].map(|principal| world.sessions.is_current(principal)),
        [true, false]
    );
    assert_eq!(
        world.rows(EPOCHS),
        [
            Row(vec![text(SAM), Value::Integer(1)]),
            Row(vec![text(SAM), Value::Integer(2)]),
        ]
    );
    // A device nobody enrolled has no sessions to end.
    let nobody = device("dev_00000000000000000000000001");
    assert_eq!(
        world.sessions.bump_epoch(EpochScope::Device(nobody)),
        Ok(())
    );
    assert_eq!(ask(&client, &ME, &pair(&phone)), granted());
}

/// Verifies: SEC-IAM-043, SEC-TM-028
#[test]
fn ending_a_session_stops_its_token_at_once() {
    let world = World::fresh();
    let (phone, on_phone) = world.sign_in(SAM, DeviceClass::Personal, Lifetime::Personal);
    let (tv, on_tv) = world.sign_in(SAM, DeviceClass::Limited, Lifetime::Personal);
    let (client, _) = serve(&world);
    assert_eq!(world.sessions.end(on_phone.session), Ok(()));
    assert_eq!(ask(&client, &ME, &pair(&phone)), unauthenticated());
    assert_eq!(ask(&client, &ME, &pair(&tv)), granted());
    assert_eq!(
        [&on_phone, &on_tv].map(|principal| world.sessions.is_current(principal)),
        [false, true]
    );
    // Ending it again, or ending a session that never was, changes
    // nothing.
    assert_eq!(world.sessions.end(on_phone.session), Ok(()));
    assert_eq!(world.sessions.end(SessionHandle(7)), Ok(()));
    assert_eq!(
        world.rows(EPOCHS),
        [
            Row(vec![text(SAM), Value::Integer(1)]),
            Row(vec![text(SAM), Value::Integer(2)]),
        ]
    );
    assert_eq!(ask(&client, &ME, &pair(&tv)), granted());
}

/// Verifies: SEC-IAM-043
#[test]
fn a_session_layer_opened_again_knows_every_devices_epoch() {
    let world = World::fresh();
    let (_, on_phone) = world.sign_in(SAM, DeviceClass::Personal, Lifetime::Personal);
    let reopened = world.reopen().expect("the session layer opens again");
    assert!(reopened.is_current(&on_phone));
    // A principal that names no device, or a device nobody enrolled, is
    // never current.
    let mut detached = on_phone.clone();
    detached.device = None;
    let mut elsewhere = on_phone.clone();
    elsewhere.device = Some(device("dev_00000000000000000000000001"));
    assert_eq!(
        [&detached, &elsewhere].map(|principal| reopened.is_current(principal)),
        [false, false]
    );
    // What one layer ends, the other has not read back: the store is the
    // server's, and one server has one session layer.
    assert_eq!(reopened.end(on_phone.session), Ok(()));
    assert!(!reopened.is_current(&on_phone));
    assert!(world.sessions.is_current(&on_phone));
}

#[test]
fn a_write_the_store_refuses_changes_nothing() {
    let world = World::fresh();
    let sam = account(SAM);
    let (cookie, on_phone) = world.sign_in(SAM, DeviceClass::Personal, Lifetime::Personal);
    let phone = on_phone.device.expect("the session has a device");
    let (client, _) = serve(&world);
    // Another connection holds the store's one write lock.
    let other = open_db(&world.root, &IDENTITY, Pragmas::new(Synchronous::Full))
        .expect("a second connection opens");
    other
        .execute(&Query::new("BEGIN IMMEDIATE"))
        .expect("the second connection takes the write lock");
    let busy = SessionError::Store(IdentityError::Db {
        step: Step::Write,
        // SQLITE_BUSY
        error: DbError::Sqlite { code: 5 },
    });
    assert_eq!(
        world.sessions.bump_epoch(EpochScope::Account(sam)),
        Err(busy.clone())
    );
    assert_eq!(world.sessions.end(on_phone.session), Err(busy.clone()));
    assert_eq!(
        world.sessions.enrol(sam, DeviceClass::Limited),
        Err(busy.clone())
    );
    let issued = world
        .sessions
        .issue(sam, phone, Lifetime::Personal, &from(HOME));
    assert_eq!(issued.map(|(again, _)| again.header()), Err(busy));
    // The session is as it was, in the store and in memory.
    assert!(world.sessions.is_current(&on_phone));
    assert_eq!(ask(&client, &ME, &pair(&cookie)), granted());
    // A request whose use is due to be recorded is refused rather than let
    // through unrecorded.
    world.manual.advance(60_000);
    assert_eq!(ask(&client, &ME, &pair(&cookie)), internal());
    other
        .execute(&Query::new("ROLLBACK"))
        .expect("the second connection lets go");
    assert_eq!(ask(&client, &ME, &pair(&cookie)), granted());
}
