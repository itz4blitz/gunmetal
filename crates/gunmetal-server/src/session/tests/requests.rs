//! Requests through the pipeline: which principal each resolves to, and
//! which are refused.

use gunmetal_core::authz::{DeviceClass, Principal, PrincipalKind, Reach, Role};
use gunmetal_core::id::{IdKind, PublicId};
use gunmetal_core::problem::ProblemCode;
use gunmetal_durable::identity::error::{IdentityError, Step};
use gunmetal_fs::sqlite::{DbError, Query, Row, Value};
use gunmetal_http::problem::ApiError;

use super::http::{
    ELEVATED, ME, PASSKEYS, ROOTS, SERVER, SIGN_IN, STREAM, USERS, ask, calls, granted, internal,
    misplaced, request, serve, step_up, unauthenticated,
};
use super::world::{DEVICE, HANDLE, KIM, SAM, TOKEN, World, account, pair, principal, text, unhex};
use crate::session::directory::Standing;
use crate::session::hook::Need;
use crate::session::kind::{Listener, TokenKind};
use crate::session::lifetime::Lifetime;

/// A credential a request can present.
enum Presented<'a> {
    /// A session cookie, and the principal it resolves to if it is valid.
    Cookie(&'a str, Option<&'a Principal>),
    /// An `Authorization` header.
    Bearer,
}

/// Verifies: SEC-IAM-002
#[test]
fn a_request_resolves_to_exactly_one_principal_or_is_refused() {
    let world = World::fresh();
    let (sam, sam_is) = world.sign_in(SAM, DeviceClass::Personal, Lifetime::Personal);
    let (kim, kim_is) = world.sign_in(KIM, DeviceClass::Personal, Lifetime::Personal);
    let (sam, kim) = (pair(&sam), pair(&kim));
    // A token nobody was issued, and a real one with padding added.
    let unknown = "__Host-gm_session=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    let padded = format!("{sam}=");
    let pool = [
        Presented::Cookie(&sam, Some(&sam_is)),
        Presented::Cookie(&kim, Some(&kim_is)),
        Presented::Cookie(unknown, None),
        Presented::Cookie(&padded, None),
        Presented::Bearer,
    ];
    // Every request that presents none, one or two of them.
    let mut combinations: Vec<Vec<&Presented<'_>>> = vec![Vec::new()];
    for (index, one) in pool.iter().enumerate() {
        combinations.push(vec![one]);
        for other in &pool[index + 1..] {
            combinations.push(vec![one, other]);
        }
    }
    assert_eq!(combinations.len(), 16);
    let (client, seen) = serve(&world);
    let mut resolved = Vec::new();
    for combination in &combinations {
        let cookies: Vec<&str> = combination
            .iter()
            .filter_map(|presented| match presented {
                Presented::Cookie(cookie, _) => Some(*cookie),
                Presented::Bearer => None,
            })
            .collect();
        let cookie = cookies.join("; ");
        let mut headers = Vec::new();
        if !cookies.is_empty() {
            headers.push(("cookie", cookie.as_str()));
        }
        if cookies.len() < combination.len() {
            headers.push(("authorization", "Bearer abc"));
        }
        // One valid cookie and nothing else resolves to its principal. Two
        // credentials are refused by the pipeline before any is looked at,
        // and everything else is the one unauthenticated answer.
        let expected = match combination.as_slice() {
            [Presented::Cookie(_, Some(principal))] => {
                resolved.push((ME.path, Some((*principal).clone())));
                granted()
            }
            [] | [_] => unauthenticated(),
            _ => misplaced(),
        };
        assert_eq!(client.send(request(&ME, &headers)), expected);
    }
    // The handler ran twice, each time for exactly one principal.
    assert_eq!(calls(&seen), resolved);
    assert_eq!(
        resolved,
        [
            (
                ME.path,
                Some(principal(SAM, DEVICE, DeviceClass::Personal, HANDLE, 1))
            ),
            (ME.path, Some(kim_is.clone())),
        ]
    );
    assert_eq!(kim_is.facts.account, Some(account(KIM)));
}

/// Verifies: SEC-IAM-002, SEC-IAM-037
#[test]
fn a_cookie_that_is_not_one_token_exactly_is_refused() {
    let world = World::fresh();
    let (cookie, _) = world.sign_in(SAM, DeviceClass::Personal, Lifetime::Personal);
    let (client, seen) = serve(&world);
    for value in [
        // Nothing; one byte short and one byte long; a character of no
        // alphabet and one of the other alphabet; a last character with
        // stray bits; padding; and one character changed.
        "",
        "SktMTU5PUFFSU1RVVldYWVpbXF1eX2BhYmNkZWZnaA",
        "SktMTU5PUFFSU1RVVldYWVpbXF1eX2BhYmNkZWZnaGlq",
        "SktMTU5PUFFSU1RVVldYWVpbXF1eX2BhYmNkZWZna*k",
        "SktMTU5PUFFSU1RVVldYWVpbXF1eX2BhYmNkZWZna+k",
        "SktMTU5PUFFSU1RVVldYWVpbXF1eX2BhYmNkZWZnaGl",
        "SktMTU5PUFFSU1RVVldYWVpbXF1eX2BhYmNkZWZnaGk=",
        "SktMTU5PUFFSU1RVVldYWVpbXF1eX2BhYmNkZWZnaGg",
    ] {
        let presented = format!("__Host-gm_session={value}");
        assert_eq!(ask(&client, &ME, &presented), unauthenticated());
    }
    assert_eq!(calls(&seen), []);
    assert_eq!(pair(&cookie), format!("__Host-gm_session={TOKEN}"));
    assert_eq!(ask(&client, &ME, &pair(&cookie)), granted());
}

/// The statement that puts a session of another kind where the web
/// session was: the same token hash, handle, account and device.
const REPLACE: Query = Query::new(
    "INSERT INTO sessions \
     (token_hash, handle, kind, lifetime, account, device, epoch, created, last_seen, address) \
     VALUES (?1, x'6f70717273747576', ?2, 'personal', 'usr_00000000000000000000000001', \
     'dev_154rkjga9a5cp2tbhf60rk4csm', 1, 1791028800000, 1791028800000, '192.168.1.20')",
);

/// Verifies: SEC-EXT-007
#[test]
fn every_listener_refuses_every_credential_kind_but_its_own() {
    let world = World::fresh();
    let (_, issued) = world.sign_in(SAM, DeviceClass::Personal, Lifetime::Personal);
    let hash = unhex("4e6eed699e6eb0f125180a95e19fb689e1142fa734d0177b6f30b5d8eaec0168");
    let plain = Need {
        admin: false,
        fresh: false,
    };
    let mut admitted = Vec::new();
    for kind in TokenKind::ALL {
        // A record of each kind, put straight into the store as an adapter
        // or a later release would write it.
        world.write(&[
            Query::new("DELETE FROM sessions"),
            REPLACE
                .bind(Value::Blob(hash.clone()))
                .bind(text(kind.name())),
        ]);
        for listener in Listener::ALL {
            match world
                .sessions
                .authenticate(listener, TOKEN.as_bytes(), plain)
            {
                Ok(principal) => admitted.push((kind, listener, principal)),
                Err(refusal) => assert_eq!(refusal, ApiError::new(ProblemCode::Unauthenticated)),
            }
        }
    }
    // Of the 21 pairs, only a web session at the native listener resolves.
    assert_eq!(
        admitted,
        [(TokenKind::WebSession, Listener::Native, issued)]
    );
}

/// Verifies: SEC-EXT-007
#[test]
fn the_kind_of_a_credential_never_changes() {
    let world = World::fresh();
    world.sign_in(SAM, DeviceClass::Personal, Lifetime::Personal);
    assert_eq!(
        world
            .store
            .write(&[Query::new("UPDATE sessions SET kind = 'api_key'")]),
        Err(IdentityError::Db {
            step: Step::Write,
            // SQLITE_CONSTRAINT_TRIGGER
            error: DbError::Sqlite { code: 1811 },
        })
    );
    assert_eq!(
        world.rows("SELECT kind FROM sessions"),
        [Row(vec![text("web_session")])]
    );
}

/// Verifies: SEC-IAM-041, SEC-TM-017
#[test]
fn an_ordinary_session_is_refused_on_every_admin_and_step_up_route() {
    let world = World::fresh();
    let (cookie, _) = world.sign_in(SAM, DeviceClass::Personal, Lifetime::Personal);
    let (client, seen) = serve(&world);
    for route in [USERS, ROOTS, ELEVATED, PASSKEYS] {
        // Signed in, the answer asks for a step-up; signed out, it is the
        // same answer every other route gives.
        assert_eq!(ask(&client, &route, &pair(&cookie)), step_up());
        assert_eq!(client.send(request(&route, &[])), unauthenticated());
    }
    assert_eq!(ask(&client, &ME, &pair(&cookie)), granted());
    assert_eq!(calls(&seen).len(), 1);
}

#[test]
fn a_route_that_takes_no_session_resolves_none() {
    let world = World::fresh();
    let (cookie, _) = world.sign_in(SAM, DeviceClass::Personal, Lifetime::Personal);
    let (client, seen) = serve(&world);
    let stale = "__Host-gm_session=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    for route in [SERVER, SIGN_IN, STREAM] {
        assert_eq!(client.send(request(&route, &[])), granted());
        assert_eq!(ask(&client, &route, &pair(&cookie)), granted());
        assert_eq!(ask(&client, &route, stale), granted());
    }
    // Every handler ran each time, and none was handed a principal.
    let ran: Vec<(&str, bool)> = calls(&seen)
        .into_iter()
        .map(|(path, principal)| (path, principal.is_some()))
        .collect();
    assert_eq!(
        ran,
        [
            ("/api/v1/server", false),
            ("/api/v1/server", false),
            ("/api/v1/server", false),
            ("/api/v1/sign-in", false),
            ("/api/v1/sign-in", false),
            ("/api/v1/sign-in", false),
            ("/api/v1/stream", false),
            ("/api/v1/stream", false),
            ("/api/v1/stream", false),
        ]
    );
}

/// Verifies: SEC-IAM-076, SEC-TM-028
#[test]
fn a_change_to_an_account_applies_from_its_next_request() {
    let world = World::fresh();
    let sam = account(SAM);
    let (cookie, member) = world.sign_in(SAM, DeviceClass::Personal, Lifetime::Personal);
    let (client, seen) = serve(&world);
    assert_eq!(ask(&client, &ME, &pair(&cookie)), granted());
    // The account becomes a guest with one library.
    let library = PublicId::parse("lib_00000000000000000000000007", IdKind::Library);
    let library = library.expect("a library identifier");
    world.roster.set(
        sam,
        Standing {
            kind: PrincipalKind::Guest,
            profile: None,
            capabilities: Role::Guest.preset(),
            libraries: vec![library],
            reach: Reach::HomeOnly,
        },
    );
    assert_eq!(ask(&client, &ME, &pair(&cookie)), granted());
    let mut guest = member.clone();
    guest.facts.kind = PrincipalKind::Guest;
    guest.facts.capabilities = Role::Guest.preset();
    guest.facts.libraries = vec![library];
    guest.facts.reach = Reach::HomeOnly;
    assert_eq!(
        calls(&seen),
        [(ME.path, Some(member)), (ME.path, Some(guest))]
    );
    // The account is disabled: its session resolves to nobody. It is
    // enabled again: the session, which nothing ended, resolves once more.
    world.roster.remove(sam);
    assert_eq!(ask(&client, &ME, &pair(&cookie)), unauthenticated());
    world.roster.set(sam, super::world::member());
    assert_eq!(ask(&client, &ME, &pair(&cookie)), granted());
    assert_eq!(calls(&seen).len(), 3);
}

#[test]
fn a_store_that_cannot_be_read_answers_the_generic_error() {
    let world = World::fresh();
    let (cookie, _) = world.sign_in(SAM, DeviceClass::Personal, Lifetime::Personal);
    let (client, seen) = serve(&world);
    world.write(&[Query::new("DROP TABLE sessions")]);
    assert_eq!(ask(&client, &ME, &pair(&cookie)), internal());
    assert_eq!(calls(&seen), []);
}

#[test]
fn a_session_row_that_does_not_read_back_is_not_trusted() {
    for damage in [
        "UPDATE sessions SET handle = x'00'",
        "UPDATE sessions SET account = 'nobody'",
        "UPDATE sessions SET account = x'00'",
        "UPDATE sessions SET device = 'tv'",
    ] {
        let world = World::fresh();
        let (cookie, _) = world.sign_in(SAM, DeviceClass::Personal, Lifetime::Personal);
        let (client, seen) = serve(&world);
        world.write(&[
            Query::new(
                "INSERT INTO devices (device, account, class, enrolled) \
                 VALUES ('tv', 'usr_00000000000000000000000001', 'personal', 0)",
            ),
            Query::new(damage),
        ]);
        assert_eq!(ask(&client, &ME, &pair(&cookie)), internal());
        assert_eq!(calls(&seen), []);
    }
}
