//! The stand-in route table, hooks and expected responses the pipeline
//! tests share. Every expected response is written out here, whole: status,
//! every header and the body.

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use axum::body::Body;
use axum::http::{Extensions, HeaderValue, Request};
use gunmetal_core::audit_event::EventName;
use gunmetal_core::problem::ProblemCode;
use gunmetal_http::call::{Call, Reply, ResponseBody};
use gunmetal_http::client::{TestClient, TestResponse};
use gunmetal_http::credential::Credential;
use gunmetal_http::host::HostAllowList;
use gunmetal_http::paging::{PageLimit, PageQuery};
use gunmetal_http::pipeline::{AccessRecord, Attempt, Hooks, router};
use gunmetal_http::problem::{ApiError, RequestId};
use gunmetal_http::request::{Fields, NoQuery};
use gunmetal_http::route::{
    Access, AdminEffect, BodyRule, Capability, Effect, IdPlace, JsonLimits, Method, RateClass,
    RouteSpec, RouteTag, Target,
};
use gunmetal_http::table::RouteEntry;
use serde::{Deserialize, Serialize};

pub const HOST: &str = "music.example.com";
pub const ORIGIN: &str = "https://music.example.com";
pub const JSON: (&str, &str) = ("content-type", "application/json");

/// A spec with the fields every stand-in route shares.
pub const fn spec(method: Method, path: &'static str, access: Access, body: BodyRule) -> RouteSpec {
    RouteSpec {
        method,
        path,
        access,
        tag: RouteTag::None,
        body,
        ids: &[],
        rate: RateClass::Read,
    }
}

const fn user(effect: Effect) -> Access {
    Access::User {
        capability: Capability::new("library.read"),
        effect,
    }
}

pub const PUBLIC: Access = Access::Public {
    effect: Effect::Reads,
};

const SMALL: JsonLimits = JsonLimits {
    bytes: 32,
    items: 4,
};

#[derive(Serialize)]
struct Server {
    name: &'static str,
}
impl ResponseBody for Server {}

#[derive(Serialize)]
struct Playlist {
    id: String,
    name: String,
}
impl ResponseBody for Playlist {}

// None of the stand-in request types refuses unknown fields itself: the
// pipeline has to, from the fields each one names.

#[derive(Deserialize)]
struct Rename {
    name: String,
}
impl Fields for Rename {
    const FIELDS: &'static [&'static str] = &["name"];
}

#[derive(Deserialize)]
struct Disable {
    user: String,
}
impl Fields for Disable {
    const FIELDS: &'static [&'static str] = &["user"];
}

/// A note. `owner` is the mistake SEC-API-013 guards against: a field that
/// names a principal, declared on a route that may not name one.
#[derive(Deserialize)]
struct Note {
    text: Option<String>,
    tags: Option<Vec<u8>>,
    owner: Option<String>,
}
impl Fields for Note {
    const FIELDS: &'static [&'static str] = &["owner", "tags", "text"];
}

/// The notes route's query, with the same mistake.
#[derive(Deserialize)]
struct NoteQuery {
    user_id: Option<String>,
}
impl Fields for NoteQuery {
    const FIELDS: &'static [&'static str] = &["user_id"];
}

#[derive(Serialize)]
struct Disabled {
    id: String,
    user: String,
}
impl ResponseBody for Disabled {}

#[derive(Serialize)]
struct Listed {
    limit: Option<u32>,
}
impl ResponseBody for Listed {}

#[derive(Serialize)]
struct Who {
    principal: String,
}
impl ResponseBody for Who {}

/// The principal the stand-in access hook grants.
#[derive(Clone)]
pub struct Principal(pub String);

/// A grant the stand-in rate hook refuses.
#[derive(Clone)]
struct Throttled;

/// What a listener would attach to a request: the client's address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Peer(pub u8);

/// A future that is pending once before it finishes, as a handler that
/// waits for storage would be.
struct YieldOnce(bool);

impl Future for YieldOnce {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            Poll::Ready(())
        } else {
            self.0 = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

/// An access record, with the principal and peer the log hook was given.
pub type Logged = (AccessRecord, Option<String>, Option<Peer>);

/// What the stand-in hooks and handlers saw.
#[derive(Default)]
pub struct Seen {
    next: AtomicU64,
    access: Mutex<Vec<(&'static str, Credential, Option<Peer>)>>,
    rate: Mutex<Vec<(RateClass, Option<String>)>>,
    log: Mutex<Vec<Logged>>,
    handled: Mutex<Vec<&'static str>>,
    events: Mutex<Vec<&'static str>>,
}

fn push<T>(list: &Mutex<Vec<T>>, item: T) {
    list.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(item);
}

fn taken<T: Clone>(list: &Mutex<Vec<T>>) -> Vec<T> {
    list.lock().unwrap_or_else(PoisonError::into_inner).clone()
}

impl Seen {
    /// Each call of the access hook: the route's template, the credential
    /// and what the listener attached.
    pub fn accessed(&self) -> Vec<(&'static str, Credential, Option<Peer>)> {
        taken(&self.access)
    }

    /// Each call of the rate hook: the route's class and the granted
    /// principal.
    pub fn rated(&self) -> Vec<(RateClass, Option<String>)> {
        taken(&self.rate)
    }

    /// Each access record, with the principal and peer the log hook was
    /// given.
    pub fn logged(&self) -> Vec<Logged> {
        taken(&self.log)
    }

    /// The access records alone.
    pub fn records(&self) -> Vec<AccessRecord> {
        self.logged()
            .into_iter()
            .map(|(record, ..)| record)
            .collect()
    }

    /// The handlers that ran, by name.
    pub fn handled(&self) -> Vec<&'static str> {
        taken(&self.handled)
    }

    /// What the stand-in transaction did.
    pub fn events(&self) -> Vec<&'static str> {
        taken(&self.events)
    }
}

/// A stand-in for a request's transaction: it commits when the handler
/// says so and rolls back when it is dropped first.
struct Transaction {
    seen: Arc<Seen>,
    committed: bool,
}

impl Transaction {
    fn begin(seen: &Arc<Seen>) -> Self {
        push(&seen.events, "begin");
        Self {
            seen: Arc::clone(seen),
            committed: false,
        }
    }

    fn commit(mut self) {
        self.committed = true;
        push(&self.seen.events, "commit");
    }
}

impl Drop for Transaction {
    fn drop(&mut self) {
        if !self.committed {
            push(&self.seen.events, "rollback");
        }
    }
}

/// The stand-in route table.
#[expect(
    clippy::too_many_lines,
    reason = "one literal table of thirteen stand-in routes"
)]
pub fn entries(seen: &Arc<Seen>) -> Vec<RouteEntry> {
    let me = Arc::clone(seen);
    let write = Arc::clone(seen);
    let index = Arc::clone(seen);
    let text = Arc::clone(seen);
    let other = Arc::clone(seen);
    vec![
        RouteEntry::new(
            spec(Method::Get, "/api/v1/server", PUBLIC, BodyRule::None),
            |_: Call| async { Reply::json(&Server { name: "gm" }) },
        ),
        RouteEntry::new(
            spec(
                Method::Get,
                "/api/v1/playlists/{id}",
                user(Effect::Reads),
                BodyRule::None,
            ),
            |call: Call| async move {
                Reply::json(&Playlist {
                    id: call.param("id")?.to_owned(),
                    name: "Mix".to_owned(),
                })
            },
        ),
        RouteEntry::new(
            spec(
                Method::Delete,
                "/api/v1/playlists/{id}",
                user(Effect::Mutates),
                BodyRule::None,
            ),
            move |_: Call| {
                let seen = Arc::clone(&write);
                async move {
                    let transaction = Transaction::begin(&seen);
                    YieldOnce(false).await;
                    transaction.commit();
                    Ok(Reply::empty())
                }
            },
        ),
        RouteEntry::new(
            spec(
                Method::Patch,
                "/api/v1/playlists/{id}",
                user(Effect::Mutates),
                BodyRule::Json(JsonLimits::DEFAULT),
            ),
            |call: Call<NoQuery, Rename>| async move {
                Reply::json(&Playlist {
                    id: call.param("id")?.to_owned(),
                    name: call.body.name,
                })
            },
        ),
        RouteEntry::new(
            RouteSpec {
                method: Method::Post,
                path: "/api/v1/notes",
                access: user(Effect::Mutates),
                tag: RouteTag::None,
                body: BodyRule::Json(SMALL),
                ids: &[],
                rate: RateClass::Write,
            },
            |call: Call<NoteQuery, Note>| async move {
                drop((
                    call.query.user_id,
                    call.body.text,
                    call.body.tags,
                    call.body.owner,
                ));
                Ok(Reply::empty())
            },
        ),
        RouteEntry::new(
            RouteSpec {
                method: Method::Post,
                path: "/api/v1/admin/accounts/{id}/disable",
                access: Access::Admin {
                    capability: Capability::new("accounts.manage"),
                    effect: AdminEffect::Mutates(EventName::AuthzFail),
                    target: Target::OtherPrincipals,
                },
                tag: RouteTag::FreshUv,
                body: BodyRule::Json(JsonLimits::DEFAULT),
                ids: &[IdPlace::Path, IdPlace::Body],
                rate: RateClass::Write,
            },
            |call: Call<NoQuery, Disable>| async move {
                Reply::json(&Disabled {
                    id: call.param("id")?.to_owned(),
                    user: call.body.user,
                })
            },
        ),
        RouteEntry::new(
            spec(
                Method::Get,
                "/api/v1/tracks",
                user(Effect::Reads),
                BodyRule::None,
            ),
            |call: Call<PageQuery>| async move {
                Reply::json(&Listed {
                    limit: call.query.limit.map(PageLimit::get),
                })
            },
        ),
        RouteEntry::new(
            spec(
                Method::Get,
                "/api/v1/me",
                user(Effect::Reads),
                BodyRule::None,
            ),
            move |call: Call| {
                push(&me.handled, "me");
                async move {
                    let principal = call
                        .grant()
                        .get::<Principal>()
                        .map_or_else(String::new, |p| p.0.clone());
                    Reply::json(&Who { principal })
                }
            },
        ),
        RouteEntry::new(
            spec(Method::Get, "/api/v1/test/panic", PUBLIC, BodyRule::None),
            move |_: Call| {
                let seen = Arc::clone(&index);
                async move {
                    // A handler that fails mid-transaction in a way nobody
                    // planned for: it reads past the end of a list.
                    let _transaction = Transaction::begin(&seen);
                    YieldOnce(false).await;
                    let names = ["a", "b"];
                    let at = names.len() + seen.events().len();
                    Reply::json(&Server { name: names[at] })
                }
            },
        ),
        RouteEntry::new(
            spec(
                Method::Get,
                "/api/v1/test/panic-text",
                PUBLIC,
                BodyRule::None,
            ),
            move |_: Call| {
                let seen = Arc::clone(&text);
                async move {
                    let _transaction = Transaction::begin(&seen);
                    std::panic::resume_unwind(Box::new(
                        "SELECT secret FROM /srv/gunmetal/data.db failed in rusqlite 0.40.2",
                    ))
                }
            },
        ),
        RouteEntry::new(
            spec(
                Method::Get,
                "/api/v1/test/panic-other",
                PUBLIC,
                BodyRule::None,
            ),
            move |_: Call| {
                let seen = Arc::clone(&other);
                async move {
                    let _transaction = Transaction::begin(&seen);
                    std::panic::resume_unwind(Box::new(7_u8))
                }
            },
        ),
        RouteEntry::new(
            spec(Method::Get, "/api/v1/test/missing", PUBLIC, BodyRule::None),
            |_: Call| async { Err(ApiError::new(ProblemCode::NotFound)) },
        ),
        RouteEntry::new(
            spec(Method::Get, "/api/v1/test/slow", PUBLIC, BodyRule::None),
            |_: Call| async {
                YieldOnce(false).await;
                Reply::json(&Server { name: "slow" })
            },
        ),
    ]
}

/// The stand-in hooks. The access hook treats the credential's text as the
/// principal's name, refuses `Bearer refused` and marks `Bearer throttled`
/// for the rate hook to refuse; the posture hook refuses any request with
/// an `x-test-posture` header.
pub fn hooks(seen: &Arc<Seen>, https: bool) -> Hooks {
    let id_seen = Arc::clone(seen);
    let access_seen = Arc::clone(seen);
    let rate_seen = Arc::clone(seen);
    let log_seen = Arc::clone(seen);
    Hooks {
        hosts: HostAllowList::new(&[HOST, "localhost", "192.168.1.20"]).unwrap(),
        origins: vec![ORIGIN.to_owned()],
        https,
        request_id: Arc::new(move || {
            RequestId(u128::from(id_seen.next.fetch_add(1, Ordering::SeqCst)))
        }),
        posture: Arc::new(|parts| {
            if parts.headers.contains_key("x-test-posture") {
                Err(ApiError::new(ProblemCode::AuditUnavailable))
            } else {
                Ok(())
            }
        }),
        access: Arc::new(move |attempt: Attempt| {
            push(
                &access_seen.access,
                (
                    attempt.spec.path,
                    attempt.credential.clone(),
                    attempt.context.get::<Peer>().copied(),
                ),
            );
            let mut grant = attempt.context;
            let result = match attempt.credential {
                Credential::Header(token) if token.as_bytes() == b"Bearer refused" => {
                    Err(ApiError::new(ProblemCode::NotFound))
                }
                Credential::Header(token) if token.as_bytes() == b"Bearer throttled" => {
                    grant.insert(Throttled);
                    Ok(grant)
                }
                Credential::Header(token) | Credential::Cookie(token) => {
                    grant.insert(Principal(
                        String::from_utf8_lossy(token.as_bytes()).into_owned(),
                    ));
                    Ok(grant)
                }
                Credential::None => Ok(grant),
            };
            Box::pin(async move {
                YieldOnce(false).await;
                result
            })
        }),
        rate: Arc::new(move |spec: RouteSpec, grant: Arc<Extensions>| {
            push(
                &rate_seen.rate,
                (spec.rate, grant.get::<Principal>().map(|p| p.0.clone())),
            );
            Box::pin(async move {
                if grant.get::<Throttled>().is_some() {
                    Err(ApiError::new(ProblemCode::AuditUnavailable))
                } else {
                    Ok(())
                }
            })
        }),
        log: Arc::new(move |record: &AccessRecord, context: &Extensions| {
            push(
                &log_seen.log,
                (
                    record.clone(),
                    context.get::<Principal>().map(|p| p.0.clone()),
                    context.get::<Peer>().copied(),
                ),
            );
        }),
    }
}

pub fn client_with(https: bool) -> (TestClient, Arc<Seen>) {
    let seen = Arc::new(Seen::default());
    let router = router(&entries(&seen), hooks(&seen, https)).unwrap();
    (TestClient::new(router), seen)
}

pub fn client() -> (TestClient, Arc<Seen>) {
    client_with(false)
}

pub fn request(method: &str, target: &str, headers: &[(&str, &str)], body: &str) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(target);
    for (name, value) in headers {
        builder = builder.header(*name, HeaderValue::from_bytes(value.as_bytes()).unwrap());
    }
    builder.body(Body::from(body.to_owned())).unwrap()
}

pub fn get(target: &str) -> Request<Body> {
    request("GET", target, &[("host", HOST)], "")
}

pub fn bearer(method: &str, target: &str, extra: &[(&str, &str)], body: &str) -> Request<Body> {
    let mut headers = vec![("host", HOST), ("authorization", "Bearer alice")];
    headers.extend_from_slice(extra);
    request(method, target, &headers, body)
}

/// The headers every non-HTML response carries, apart from its length.
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

/// The golden header set: the five above, the body's length and whatever
/// else one response adds.
pub fn headers(length: usize, extra: &[(&str, &str)]) -> Vec<(String, String)> {
    let length = length.to_string();
    let mut all: Vec<(String, String)> = BASE
        .iter()
        .chain(extra)
        .chain(&[("content-length", length.as_str())])
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect();
    all.sort();
    all
}

pub fn json(status: u16, body: &str) -> TestResponse {
    TestResponse {
        status,
        headers: headers(body.len(), &[("content-type", "application/json")]),
        body: body.as_bytes().to_vec(),
    }
}

pub fn empty() -> TestResponse {
    TestResponse {
        status: 204,
        headers: headers(0, &[]),
        body: Vec::new(),
    }
}

/// A problem response, written out whole.
pub fn problem(
    code: &str,
    status: u16,
    title: &str,
    request: u64,
    extra: &[(&str, &str)],
) -> TestResponse {
    let body = format!(
        r#"{{"type":"urn:gunmetal:problem:{code}","title":"{title}","status":{status},"request":"{request:032x}"}}"#
    );
    let mut more = vec![("content-type", "application/problem+json")];
    more.extend_from_slice(extra);
    TestResponse {
        status,
        headers: headers(body.len(), &more),
        body: body.into_bytes(),
    }
}

pub const NOT_FOUND: &str =
    "We couldn't find that. It may have been removed, or you may not have access to it.";
pub const MISPLACED: &str =
    "Send credentials only in the Authorization header or the session cookie, and only once.";
pub const CROSS_SITE: &str =
    "This request was refused because it didn't come from this server's own pages.";
pub const INVALID: &str = "The request wasn't in the expected form.";
pub const UNSUPPORTED: &str = "The request body must be JSON, sent without compression.";
pub const TOO_LARGE: &str = "That request is too large.";
pub const UNKNOWN_HOST: &str = "This server doesn't answer to that name.";
pub const NOT_ALLOWED: &str = "That action isn't available here.";
pub const INTERNAL: &str = "Something went wrong on the server. Try again later.";
pub const AUDIT: &str = "We couldn't record this action, so it didn't happen. Try again later.";

pub fn not_found(request: u64) -> TestResponse {
    problem("not_found", 404, NOT_FOUND, request, &[])
}

pub fn invalid(request: u64) -> TestResponse {
    problem("invalid_request", 400, INVALID, request, &[])
}

pub fn misplaced(request: u64) -> TestResponse {
    problem("credential_misplaced", 400, MISPLACED, request, &[])
}

pub fn cross_site(request: u64) -> TestResponse {
    problem("cross_site_request", 403, CROSS_SITE, request, &[])
}

/// An access record of a request that did not panic.
pub fn record(
    request: u128,
    method: Option<Method>,
    route: Option<&'static str>,
    status: u16,
) -> AccessRecord {
    AccessRecord {
        request: RequestId(request),
        method,
        route,
        status,
        panic: None,
    }
}
