//! Enrolling a device and issuing a session: the cookie, the principal and
//! what the store keeps.

use core::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;

use gunmetal_core::authz::{DeviceClass, Epoch};
use gunmetal_durable::identity::store::IDENTITY;
use gunmetal_fs::sqlite::{Query, Row, Value};
use gunmetal_secrets::root::ROOT_KEY;

use super::http::{ME, ask, granted, serve, unauthenticated};
use super::world::{
    DEVICE, HANDLE, HOME, KIM, NoRandom, SAM, TOKEN, WAL, World, account, device, file, from,
    holds, pair, principal, text, unhex,
};
use crate::session::cookie::INVENTORY;
use crate::session::error::SessionError;
use crate::session::lifetime::Lifetime;
use crate::testing::NOON;

/// The identifier of the second device a world enrols straight after its
/// first: the randomness source's second answer, the bytes 74 to 89, in
/// Crockford base32, worked out with Python.
const SECOND_DEVICE: &str = "dev_2a9d64tkjfa18n4mtmanb5ep2s";

/// Verifies: SEC-API-032, SEC-TM-058, SEC-STD-014
#[test]
fn a_personal_browser_gets_the_host_cookie_with_every_attribute() {
    let world = World::fresh();
    let (cookie, _) = world.sign_in(SAM, DeviceClass::Personal, Lifetime::Personal);
    assert_eq!(
        cookie.header(),
        "__Host-gm_session=SktMTU5PUFFSU1RVVldYWVpbXF1eX2BhYmNkZWZnaGk; \
         Secure; HttpOnly; SameSite=Lax; Path=/; Max-Age=2592000"
    );
    // The inventory is the one cookie, and its name and value together are
    // 60 bytes, far inside the 4096 a cookie may take.
    assert_eq!(INVENTORY, ["__Host-gm_session"]);
    assert_eq!(cookie.name(), "__Host-gm_session");
    assert_eq!(
        pair(&cookie),
        "__Host-gm_session=SktMTU5PUFFSU1RVVldYWVpbXF1eX2BhYmNkZWZnaGk"
    );
    assert_eq!(pair(&cookie).len(), 61);
}

/// Verifies: SEC-CLI-010
#[test]
fn a_shared_browser_gets_a_cookie_that_ends_with_the_browser() {
    let world = World::fresh();
    let (cookie, _) = world.sign_in(SAM, DeviceClass::Limited, Lifetime::Shared);
    assert_eq!(
        cookie.header(),
        "__Host-gm_session=SktMTU5PUFFSU1RVVldYWVpbXF1eX2BhYmNkZWZnaGk; \
         Secure; HttpOnly; SameSite=Lax; Path=/"
    );
}

/// Verifies: SEC-API-032
#[test]
fn the_cookie_does_not_depend_on_where_the_request_came_from() {
    let headers = [
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        HOME,
        IpAddr::V4(Ipv4Addr::new(203, 0, 113, 9)),
    ]
    .map(|peer| {
        World::fresh()
            .sign_in_from(SAM, DeviceClass::Personal, Lifetime::Personal, peer)
            .0
            .header()
    });
    assert_eq!(headers[0], headers[1]);
    assert_eq!(headers[1], headers[2]);
    assert!(headers[2].ends_with("; Secure; HttpOnly; SameSite=Lax; Path=/; Max-Age=2592000"));
}

/// Verifies: SEC-IAM-002, SEC-CLI-024
#[test]
fn a_session_resolves_to_the_one_principal_of_its_account_and_device() {
    let world = World::fresh();
    let (_, limited) = world.sign_in(SAM, DeviceClass::Limited, Lifetime::Personal);
    assert_eq!(
        limited,
        principal(SAM, DEVICE, DeviceClass::Limited, HANDLE, 1)
    );
    // The same draws on a device of the other class give the other class
    // and nothing else.
    let world = World::fresh();
    let (_, personal) = world.sign_in(SAM, DeviceClass::Personal, Lifetime::Personal);
    assert_eq!(
        personal,
        principal(SAM, DEVICE, DeviceClass::Personal, HANDLE, 1)
    );
    assert!(world.sessions.is_current(&personal));
}

/// Verifies: SEC-HIS-012
#[test]
fn a_device_identifier_is_what_the_randomness_source_gave() {
    let world = World::fresh();
    let sam = account(SAM);
    assert_eq!(
        world.sessions.enrol(sam, DeviceClass::Personal),
        Ok(device(DEVICE))
    );
    assert_eq!(
        world.sessions.enrol(sam, DeviceClass::Limited),
        Ok(device(SECOND_DEVICE))
    );
    assert_eq!(
        world.rows("SELECT device, account, class, epoch, enrolled FROM devices ORDER BY device"),
        [
            Row(vec![
                text(DEVICE),
                text(SAM),
                text("personal"),
                Value::Integer(0),
                Value::Integer(NOON),
            ]),
            Row(vec![
                text(SECOND_DEVICE),
                text(SAM),
                text("limited"),
                Value::Integer(0),
                Value::Integer(NOON),
            ]),
        ]
    );
}

/// Verifies: SEC-IAM-037, SEC-HIS-012
#[test]
fn nothing_is_minted_or_issued_when_randomness_is_unavailable() {
    let world = World::with(Arc::new(NoRandom));
    let sam = account(SAM);
    assert_eq!(
        world.sessions.enrol(sam, DeviceClass::Personal),
        Err(SessionError::RandomnessUnavailable)
    );
    // A device that is enrolled all the same gets no session either.
    world.write(&[Query::new(
        "INSERT INTO devices (device, account, class, enrolled) \
         VALUES ('dev_154rkjga9a5cp2tbhf60rk4csm', 'usr_00000000000000000000000001', 'personal', 0)",
    )]);
    let issued = world
        .sessions
        .issue(sam, device(DEVICE), Lifetime::Personal, &from(HOME));
    assert_eq!(
        issued.map(|(cookie, _)| cookie.header()),
        Err(SessionError::RandomnessUnavailable)
    );
    assert_eq!(
        world.rows("SELECT (SELECT count(*) FROM devices), (SELECT count(*) FROM sessions)"),
        [Row(vec![Value::Integer(1), Value::Integer(0)])]
    );
}

/// Verifies: SEC-OPS-016, SEC-HIS-044, SEC-IAM-037
#[test]
fn the_store_keeps_only_the_keyed_hash_of_a_token() {
    let world = World::fresh();
    let (cookie, _) = world.sign_in(SAM, DeviceClass::Personal, Lifetime::Personal);
    // HMAC-SHA-256 of "gunmetal/v1/session" and the token's 32 bytes, under
    // the HKDF-SHA-256 output for the root secret 0 to 31 and the context
    // "gunmetal/v1/session_hash/0", worked out with Python's hmac and
    // hashlib.
    let hash = unhex("4e6eed699e6eb0f125180a95e19fb689e1142fa734d0177b6f30b5d8eaec0168");
    assert_eq!(
        world.rows("SELECT token_hash, kind, lifetime, account, device, epoch FROM sessions"),
        [Row(vec![
            Value::Blob(hash.clone()),
            text("web_session"),
            text("personal"),
            text(SAM),
            text(DEVICE),
            Value::Integer(1),
        ])]
    );
    // The token is the 32 bytes the source gave, and no file the server
    // keeps holds it: raw, as the cookie spells it, or in hexadecimal.
    let token: Vec<u8> = (74..=105).collect();
    assert_eq!(pair(&cookie), format!("__Host-gm_session={TOKEN}"));
    let hexadecimal = "4a4b4c4d4e4f505152535455565758595a5b5c5d5e5f60616263646566676869";
    assert_eq!(unhex(hexadecimal), token);
    for kept in [
        file(&world.root, IDENTITY.path()),
        file(&world.root, &WAL),
        file(&world.root, &ROOT_KEY),
    ] {
        assert!(!holds(&kept, &token));
        assert!(!holds(&kept, TOKEN.as_bytes()));
        assert!(!holds(&kept, hexadecimal.as_bytes()));
    }
    // The scan would have seen it: the hash is there to find.
    assert!(holds(&world.on_disk(), &hash));
    // What the store keeps is no credential. The hash, spelt as a cookie
    // value would be, opens nothing; the token does.
    let (client, _) = serve(&world);
    let stolen = "__Host-gm_session=Tm7taZ5usPElGAqV4Z-2ieEUL6c00Bd7bzC12OrsAWg";
    assert_eq!(ask(&client, &ME, stolen), unauthenticated());
    assert_eq!(ask(&client, &ME, &pair(&cookie)), granted());
}

#[test]
fn refuses_an_account_the_directory_does_not_know() {
    let world = World::fresh();
    let stranger = account("usr_00000000000000000000000009");
    assert_eq!(
        world.sessions.enrol(stranger, DeviceClass::Personal),
        Err(SessionError::UnknownAccount)
    );
    // A device enrolled while the account stood gets no session once the
    // account is gone.
    let sam = account(SAM);
    let enrolled = world
        .sessions
        .enrol(sam, DeviceClass::Personal)
        .expect("the device enrols");
    world.roster.remove(sam);
    let issued = world
        .sessions
        .issue(sam, enrolled, Lifetime::Personal, &from(HOME));
    assert_eq!(
        issued.map(|(cookie, _)| cookie.header()),
        Err(SessionError::UnknownAccount)
    );
    assert_eq!(
        world.rows("SELECT (SELECT count(*) FROM devices), (SELECT count(*) FROM sessions)"),
        [Row(vec![Value::Integer(1), Value::Integer(0)])]
    );
}

#[test]
fn refuses_a_device_that_is_not_enrolled_for_the_account() {
    let world = World::fresh();
    let enrolled = world
        .sessions
        .enrol(account(SAM), DeviceClass::Personal)
        .expect("the device enrols");
    for (who, on) in [
        // Someone else's device, and a device nobody enrolled.
        (KIM, enrolled),
        (SAM, device("dev_00000000000000000000000001")),
    ] {
        let issued = world
            .sessions
            .issue(account(who), on, Lifetime::Personal, &from(HOME));
        assert_eq!(
            issued.map(|(cookie, _)| cookie.header()),
            Err(SessionError::UnknownDevice)
        );
    }
    assert_eq!(
        world.rows("SELECT count(*) FROM sessions"),
        [Row(vec![Value::Integer(0)])]
    );
}

/// Verifies: SEC-CLI-024
#[test]
fn a_shared_browser_session_needs_a_limited_class_device() {
    let world = World::fresh();
    let sam = account(SAM);
    let own = world
        .sessions
        .enrol(sam, DeviceClass::Personal)
        .expect("the device enrols");
    let issued = world
        .sessions
        .issue(sam, own, Lifetime::Shared, &from(HOME));
    assert_eq!(
        issued.map(|(cookie, _)| cookie.header()),
        Err(SessionError::SharedNeedsLimited)
    );
    assert_eq!(
        world.rows("SELECT count(*) FROM sessions"),
        [Row(vec![Value::Integer(0)])]
    );
    // A limited-class device takes either mode: a shared browser, or a
    // paired browser that keeps its session.
    let (_, shared) = world.sign_in(SAM, DeviceClass::Limited, Lifetime::Shared);
    let (_, paired) = world.sign_in(KIM, DeviceClass::Limited, Lifetime::Personal);
    assert_eq!(
        [shared.facts.device, paired.facts.device],
        [DeviceClass::Limited, DeviceClass::Limited]
    );
}

/// Verifies: SEC-IAM-038
#[test]
fn signing_in_again_on_a_device_stops_its_previous_token() {
    let world = World::fresh();
    let sam = account(SAM);
    let phone = world
        .sessions
        .enrol(sam, DeviceClass::Personal)
        .expect("the device enrols");
    let (first, before) = world
        .sessions
        .issue(sam, phone, Lifetime::Personal, &from(HOME))
        .expect("the first session");
    let (second, after) = world
        .sessions
        .issue(sam, phone, Lifetime::Personal, &from(HOME))
        .expect("the second session");
    assert_ne!(pair(&first), pair(&second));
    assert_eq!((before.epoch, after.epoch), (Epoch(1), Epoch(2)));
    assert!(!world.sessions.is_current(&before));
    assert!(world.sessions.is_current(&after));
    let (client, _) = serve(&world);
    assert_eq!(ask(&client, &ME, &pair(&first)), unauthenticated());
    assert_eq!(ask(&client, &ME, &pair(&second)), granted());
    assert_eq!(
        world.rows("SELECT count(*) FROM sessions"),
        [Row(vec![Value::Integer(1)])]
    );
}
