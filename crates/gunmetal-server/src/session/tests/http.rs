//! A stand-in route table served through the real request pipeline with
//! the session layer as its access hook, and the whole responses the tests
//! expect from it.

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Extensions, Request};
use gunmetal_core::audit_event::EventName;
use gunmetal_core::authz::Principal;
use gunmetal_http::call::{Call, Reply};
use gunmetal_http::client::{TestClient, TestResponse};
use gunmetal_http::host::HostAllowList;
use gunmetal_http::pipeline::{AccessRecord, Hooks, router};
use gunmetal_http::problem::RequestId;
use gunmetal_http::route::{
    Access, AdminEffect, BodyRule, Capability, CapabilityKind, Effect, ExchangeKind, Method,
    RateClass, RouteSpec, RouteTag, Target,
};
use gunmetal_http::table::RouteEntry;

use super::world::World;
use crate::session::hook::access_hook;
use crate::session::kind::Listener;

/// The host and origin the stand-in server answers to.
const HOST: &str = "music.example.com";
const ORIGIN: &str = "https://music.example.com";

/// A stand-in route that takes no body and names no object.
macro_rules! route {
    ($method:ident, $path:literal, $tag:ident, $access:expr) => {
        RouteSpec {
            method: Method::$method,
            path: $path,
            access: $access,
            tag: RouteTag::$tag,
            body: BodyRule::None,
            ids: &[],
            rate: RateClass::Read,
        }
    };
}

/// A route anyone may call.
pub const SERVER: RouteSpec = route!(
    Get,
    "/api/v1/server",
    None,
    Access::Public {
        effect: Effect::Reads,
    }
);

/// A credential exchange.
pub const SIGN_IN: RouteSpec = route!(
    Post,
    "/api/v1/sign-in",
    None,
    Access::Exchange {
        kind: ExchangeKind::SignIn,
        effect: Effect::Mutates,
    }
);

/// A capability route.
pub const STREAM: RouteSpec = route!(
    Get,
    "/api/v1/stream",
    None,
    Access::Capability {
        kind: CapabilityKind::Media,
        effect: Effect::Reads,
    }
);

/// A route for any signed-in person.
pub const ME: RouteSpec = route!(
    Get,
    "/api/v1/me",
    None,
    Access::User {
        capability: Capability::new("library.read"),
        effect: Effect::Reads,
    }
);

/// A signed-in person's route that changes their own sign-in methods, and
/// so needs a fresh user verification.
pub const PASSKEYS: RouteSpec = route!(
    Post,
    "/api/v1/passkeys",
    FreshUv,
    Access::User {
        capability: Capability::new("own.write"),
        effect: Effect::Mutates,
    }
);

/// A route tagged as needing the administrator session.
pub const ELEVATED: RouteSpec = route!(
    Get,
    "/api/v1/elevated",
    Elevated,
    Access::User {
        capability: Capability::new("library.read"),
        effect: Effect::Reads,
    }
);

/// An admin route that only reads.
pub const USERS: RouteSpec = route!(
    Get,
    "/api/v1/admin/users",
    None,
    Access::Admin {
        capability: Capability::new("user.manage"),
        effect: AdminEffect::Reads,
        target: Target::Caller,
    }
);

/// A host-equivalent admin route: adding a library root.
pub const ROOTS: RouteSpec = route!(
    Post,
    "/api/v1/admin/roots",
    FreshUv,
    Access::Admin {
        capability: Capability::new("server.settings"),
        effect: AdminEffect::Mutates(EventName::AuthzFail),
        target: Target::Caller,
    }
);

/// Every stand-in route.
const ROUTES: [RouteSpec; 8] = [
    SERVER, SIGN_IN, STREAM, ME, PASSKEYS, ELEVATED, USERS, ROOTS,
];

/// What the stand-in handlers saw, in order: the route that ran and the
/// principal its grant held. A handler that ran changed this state, and a
/// refused request did not.
pub type Seen = Arc<Mutex<Vec<(&'static str, Option<Principal>)>>>;

/// A handler for `spec` that records what it saw and answers with nothing.
fn entry(spec: RouteSpec, seen: &Seen) -> RouteEntry {
    let seen = Arc::clone(seen);
    RouteEntry::new(spec, move |call: Call| {
        let seen = Arc::clone(&seen);
        async move {
            let principal = call.grant().get::<Principal>().cloned();
            seen.lock().expect("seen").push((spec.path, principal));
            Ok(Reply::empty())
        }
    })
}

/// The stand-in server over `world`, taking requests as the native
/// listener, and what its handlers see.
pub fn serve(world: &World) -> (TestClient, Seen) {
    let seen = Seen::default();
    let entries: Vec<RouteEntry> = ROUTES.iter().map(|spec| entry(*spec, &seen)).collect();
    let hooks = Hooks {
        hosts: HostAllowList::new(&[HOST]).expect("a host"),
        origins: vec![ORIGIN.to_owned()],
        https: false,
        request_id: Arc::new(|| RequestId(7)),
        posture: Arc::new(|_| Ok(())),
        access: access_hook(Arc::clone(&world.sessions), Listener::Native),
        rate: Arc::new(|_: RouteSpec, _: Arc<Extensions>| Box::pin(async { Ok(()) })),
        log: Arc::new(|_: &AccessRecord, _: &Extensions| {}),
    };
    let router = router(&entries, hooks).expect("the table is sound");
    (TestClient::new(router), seen)
}

/// What the handlers have seen so far.
pub fn calls(seen: &Seen) -> Vec<(&'static str, Option<Principal>)> {
    seen.lock().expect("seen").clone()
}

/// A request for `route` as the server's own page sends it, with `headers`
/// added.
pub fn request(route: &RouteSpec, headers: &[(&str, &str)]) -> Request<Body> {
    let mut builder = Request::builder()
        .method(route.method.as_str())
        .uri(route.path)
        .header("host", HOST)
        .header("gunmetal-request", "1")
        .header("origin", ORIGIN);
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    builder.body(Body::empty()).expect("a request")
}

/// Sends a request for `route` that carries `cookie` in its `Cookie`
/// header.
pub fn ask(client: &TestClient, route: &RouteSpec, cookie: &str) -> TestResponse {
    client.send(request(route, &[("cookie", cookie)]))
}

/// The headers every response of the pipeline carries, apart from its
/// length and type. There is no `Set-Cookie` among them: the pipeline sets
/// none, so no cookie outside the inventory can come from it.
const BASE: [(&str, &str); 5] = [
    ("cache-control", "no-store"),
    (
        "content-security-policy",
        "default-src 'none'; frame-ancestors 'none'; sandbox",
    ),
    ("cross-origin-resource-policy", "same-origin"),
    ("referrer-policy", "no-referrer"),
    ("x-content-type-options", "nosniff"),
];

/// A whole response: the status, every header and the body.
fn response(status: u16, extra: &[(&str, &str)], body: &str) -> TestResponse {
    let length = body.len().to_string();
    let mut headers: Vec<(String, String)> = BASE
        .iter()
        .chain(extra)
        .chain(&[("content-length", length.as_str())])
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect();
    headers.sort();
    TestResponse {
        status,
        headers,
        body: body.as_bytes().to_vec(),
    }
}

/// The response of a handler that ran.
pub fn granted() -> TestResponse {
    response(204, &[], "")
}

/// A problem response, written out whole.
fn problem(code: &str, status: u16, title: &str) -> TestResponse {
    let body = format!(
        r#"{{"type":"urn:gunmetal:problem:{code}","title":"{title}","status":{status},"request":"00000000000000000000000000000007"}}"#
    );
    response(
        status,
        &[("content-type", "application/problem+json")],
        &body,
    )
}

/// The one answer to a request with no valid credential.
pub fn unauthenticated() -> TestResponse {
    problem("unauthenticated", 401, "Sign in to continue.")
}

/// The answer to a session that may not use a route as it is.
pub fn step_up() -> TestResponse {
    problem(
        "step_up_required",
        403,
        "Confirm it's you with your passkey, then try again.",
    )
}

/// The pipeline's own answer to a request that carries two credentials.
pub fn misplaced() -> TestResponse {
    problem(
        "credential_misplaced",
        400,
        "Send credentials only in the Authorization header or the session cookie, and only once.",
    )
}

/// The answer to a request the server could not check.
pub fn internal() -> TestResponse {
    problem(
        "internal_error",
        500,
        "Something went wrong on the server. Try again later.",
    )
}
