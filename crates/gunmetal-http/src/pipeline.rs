//! The request pipeline and the router built from the route table
//! (web-and-api-security.md, "Request pipeline").
//!
//! [`router`] is the only way to get an [`axum::Router`] out of this crate,
//! and it takes nothing but [`RouteEntry`] values, so every route the
//! server answers has a [`RouteSpec`](crate::route::RouteSpec)
//! (SEC-API-001). The router has one fallback and no routes of its own:
//! every request runs the steps below, in this order, and each step runs
//! before anything more expensive.
//!
//! 1. The posture hook (WP-132).
//! 2. The `Host` allow-list, answering 421 (SEC-API-007).
//! 3. No credential in the URL, answering 400 (SEC-API-004). This runs
//!    before the route match because the exact matcher would otherwise
//!    answer 404 to a path that carries one.
//! 4. The exact route match, answering 404, then the method check,
//!    answering 405 with `Allow`. Method-override headers are never read
//!    (SEC-API-055, SEC-API-008).
//! 5. Credential extraction from the session cookie or `Authorization`
//!    only, and never two (SEC-API-004).
//! 6. For a cookie credential: `Gunmetal-Request: 1`, `Sec-Fetch-Site` and
//!    `Origin`, answering 403 before any authentication (SEC-API-033,
//!    SEC-API-034).
//! 7. The access hook: authentication, the access class and the capability
//!    (WP-062, WP-065).
//! 8. The rate-limit hook (WP-130).
//! 9. The body: no `Content-Encoding` and `application/json` only (415),
//!    the route's size cap (413, before anything is decoded), then the
//!    structural checks of [`crate::decode`] and the placement of every
//!    parameter (400) (SEC-API-035, SEC-API-060, SEC-API-065, SEC-API-067).
//! 10. The typed request: the query and the body decode into the types the
//!     route declared, and a name neither type declares is refused (400)
//!     (SEC-API-067, SEC-IAM-072; [`crate::request`]).
//! 11. The handler.
//!
//! Whatever happens, the response is built in one place: its headers come
//! from [`security_headers`] and its error body from the problem catalogue
//! (SEC-API-053, SEC-API-072). A panic anywhere in the steps above is
//! caught, the request's future is dropped, which rolls back whatever it
//! held, and the client gets the generic 500 (SEC-API-073). Every request
//! ends with one [`AccessRecord`], which names the route template and never
//! the path that was sent (SEC-API-095).

use core::any::Any;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, Bytes, to_bytes};
use axum::http::header::{ALLOW, CONTENT_ENCODING, CONTENT_TYPE, HOST};
use axum::http::request::Parts;
use axum::http::{Extensions, HeaderMap, HeaderValue, Request, Response, StatusCode};
use gunmetal_core::problem::ProblemCode;

use crate::browser::same_origin;
use crate::call::Reply;
use crate::credential::{self, Credential};
use crate::decode;
use crate::headers::{BodyKind, security_headers};
use crate::host::{Host, HostAllowList, HostKind};
use crate::problem::{ApiError, RequestId, render};
use crate::query::parse_query;
use crate::request::Raw;
use crate::route::{BodyRule, Method, RouteSpec};
use crate::table::{RouteEntry, Table, TableError};

/// What an asynchronous hook returns.
pub type HookFuture<T> = Pin<Box<dyn Future<Output = Result<T, ApiError>> + Send>>;

/// Draws a fresh identifier for each request.
pub type RequestIdHook = Arc<dyn Fn() -> RequestId + Send + Sync>;

/// Runs first, on every request: the server's posture may refuse it.
pub type PostureHook = Arc<dyn Fn(&Parts) -> Result<(), ApiError> + Send + Sync>;

/// Authenticates the credential and checks the route's access class and
/// capability. What it returns is the grant: the principal and whatever
/// else later steps need. It answers the uniform refusal itself.
pub type AccessHook = Arc<dyn Fn(Attempt) -> HookFuture<Extensions> + Send + Sync>;

/// Charges the request to its route's rate-limit class, given the grant.
pub type RateHook = Arc<dyn Fn(RouteSpec, Arc<Extensions>) -> HookFuture<()> + Send + Sync>;

/// Receives each request's access record, with the grant if the access
/// hook gave one and otherwise what the listener attached.
pub type LogHook = Arc<dyn Fn(&AccessRecord, &Extensions) + Send + Sync>;

/// What the access hook is asked: may this credential use this route?
#[derive(Debug)]
pub struct Attempt {
    /// The route that matched.
    pub spec: RouteSpec,
    /// The request's one credential, or none.
    pub credential: Credential,
    /// What the listener attached to the request, such as the client's
    /// address.
    pub context: Extensions,
}

/// One line of the access log (SEC-API-095). It has no field that could
/// hold a path, a query, a header or a body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessRecord {
    /// The request's identifier, as in the problem body.
    pub request: RequestId,
    /// The method, if it is one a route may declare.
    pub method: Option<Method>,
    /// The template of the path that matched, if one did.
    pub route: Option<&'static str>,
    /// The status answered.
    pub status: u16,
    /// What a panic said, if request handling panicked. It goes to the
    /// server's log and never to the client.
    pub panic: Option<String>,
}

/// The settings and hooks a router is built with. The session,
/// authorisation, rate-limit and posture packages plug in here.
#[derive(Clone)]
pub struct Hooks {
    /// The hosts the server answers to.
    pub hosts: HostAllowList,
    /// The origins of the server's own pages, exactly as a browser sends
    /// them in `Origin`.
    pub origins: Vec<String>,
    /// Whether this router is served over HTTPS. It comes from the
    /// listener, never from a request header.
    pub https: bool,
    /// Draws each request's identifier.
    pub request_id: RequestIdHook,
    /// The posture hook (WP-132).
    pub posture: PostureHook,
    /// The access hook (WP-062, WP-065).
    pub access: AccessHook,
    /// The rate-limit hook (WP-130).
    pub rate: RateHook,
    /// The access log.
    pub log: LogHook,
}

/// Builds the router for a route table. Nothing else in this crate makes
/// one, so a route outside the table cannot be served.
///
/// # Errors
///
/// What [`Table::new`] refuses.
pub fn router(entries: &[RouteEntry], hooks: Hooks) -> Result<Router, TableError> {
    let pipeline = Arc::new(Pipeline {
        table: Table::new(entries.to_vec())?,
        hooks,
    });
    Ok(Router::new().fallback(move |request: Request<Body>| {
        let pipeline = Arc::clone(&pipeline);
        async move { pipeline.answer(request).await }
    }))
}

/// A refusal: a problem and, for 405, the methods the path does take.
struct Refusal {
    error: ApiError,
    allow: Option<HeaderValue>,
}

impl From<ApiError> for Refusal {
    fn from(error: ApiError) -> Self {
        Self { error, allow: None }
    }
}

impl From<ProblemCode> for Refusal {
    fn from(code: ProblemCode) -> Self {
        ApiError::new(code).into()
    }
}

/// What the pipeline learned about a request before it finished or failed.
#[derive(Default)]
struct Trace {
    route: Option<&'static str>,
    hsts: bool,
    grant: Option<Arc<Extensions>>,
}

/// The table and the hooks one router serves.
struct Pipeline {
    table: Table,
    hooks: Hooks,
}

impl Pipeline {
    /// Answers one request: runs the steps, renders the result and writes
    /// the access record.
    async fn answer(&self, request: Request<Body>) -> Response<Body> {
        let id = (self.hooks.request_id)();
        let (parts, body) = request.into_parts();
        let method = Method::from_name(parts.method.as_str());
        let mut trace = Trace::default();
        let outcome = Guarded(Box::pin(self.run(&parts, method, body, &mut trace))).await;
        let (result, panic) = match outcome {
            Ok(result) => (result, None),
            Err(text) => (Err(ProblemCode::InternalError.into()), Some(text)),
        };
        let (status, kind, bytes, allow) = match result {
            Ok(reply) => (reply.status(), reply.kind, reply.body, None),
            Err(refusal) => (
                refusal.error.status(),
                BodyKind::Problem,
                render(refusal.error, id),
                refusal.allow,
            ),
        };
        let mut response = Response::new(Body::from(bytes));
        *response.status_mut() =
            StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        *response.headers_mut() = security_headers(kind, trace.hsts);
        if let Some(allow) = allow {
            response.headers_mut().insert(ALLOW, allow);
        }
        let record = AccessRecord {
            request: id,
            method,
            route: trace.route,
            status,
            panic,
        };
        (self.hooks.log)(&record, trace.grant.as_deref().unwrap_or(&parts.extensions));
        response
    }

    /// The steps, in order.
    async fn run(
        &self,
        parts: &Parts,
        method: Option<Method>,
        body: Body,
        trace: &mut Trace,
    ) -> Result<Reply, Refusal> {
        (self.hooks.posture)(parts)?;
        let host = self.host(parts).ok_or(ProblemCode::UnknownHost)?;
        trace.hsts = self.hooks.https && host.kind() == HostKind::Name;
        let path = parts.uri.path();
        let query =
            parse_query(parts.uri.query().unwrap_or("")).ok_or(ProblemCode::InvalidRequest)?;
        if credential::in_url(path, &query) {
            return Err(ProblemCode::CredentialMisplaced.into());
        }
        let found = self.table.lookup(path).ok_or(ProblemCode::NotFound)?;
        trace.route = Some(found.path);
        let entry = found
            .entries
            .iter()
            .find(|entry| Some(entry.spec().method) == method)
            .ok_or_else(|| {
                let allowed: Vec<&str> = found
                    .entries
                    .iter()
                    .map(|entry| entry.spec().method.as_str())
                    .collect();
                Refusal {
                    error: ApiError::new(ProblemCode::MethodNotAllowed),
                    allow: HeaderValue::from_str(&allowed.join(", ")).ok(),
                }
            })?;
        let spec = *entry.spec();
        let credential = credential::extract(&parts.headers)
            .map_err(|credential::Misplaced| ProblemCode::CredentialMisplaced)?;
        if matches!(credential, Credential::Cookie(_))
            && !same_origin(&parts.headers, spec.method, &self.hooks.origins)
        {
            return Err(ProblemCode::CrossSiteRequest.into());
        }
        let grant = Arc::new(
            (self.hooks.access)(Attempt {
                spec,
                credential,
                context: parts.extensions.clone(),
            })
            .await?,
        );
        trace.grant = Some(Arc::clone(&grant));
        (self.hooks.rate)(spec, Arc::clone(&grant)).await?;
        let bytes = read_body(&spec, &parts.headers, body).await?;
        let names_principals = spec.access.names_principals();
        let (body, keys) = match spec.body {
            BodyRule::None => (None, Vec::new()),
            BodyRule::Json(limits) => {
                let keys = decode::check(&bytes, limits, names_principals)
                    .map_err(|_| ProblemCode::InvalidRequest)?;
                (Some(bytes), keys)
            }
        };
        if !decode::placed(&query, &found.params, &keys, names_principals) {
            return Err(ProblemCode::InvalidRequest.into());
        }
        let raw = Raw {
            params: found.params,
            query,
            body,
            keys,
            grant,
        };
        Ok(entry.call(raw).await?)
    }

    /// The host the request named, if the server answers to it. A request
    /// names its host in the `Host` header, in its target (HTTP/2's
    /// `:authority` arrives there) or in both; when it does both, they must
    /// agree.
    fn host(&self, parts: &Parts) -> Option<Host> {
        let mut named = parts.headers.get_all(HOST).iter();
        let header = named.next().map(|value| self.admit(value.as_bytes()));
        let target = parts
            .uri
            .authority()
            .map(|authority| self.admit(authority.as_str().as_bytes()));
        match (header, target, named.next()) {
            (Some(header), Some(target), None) if header == target => header,
            (Some(host), None, None) | (None, Some(host), None) => host,
            _ => None,
        }
    }

    /// The host these bytes name, if they are text and on the allow-list.
    fn admit(&self, authority: &[u8]) -> Option<Host> {
        core::str::from_utf8(authority)
            .ok()
            .and_then(|authority| self.hooks.hosts.admit(authority))
    }
}

/// What a request's `Content-Type` says.
#[derive(PartialEq, Eq)]
enum Declared {
    /// No `Content-Type` header.
    Nothing,
    /// One header, naming `application/json`.
    Json,
    /// Anything else, or more than one header.
    Other,
}

/// Reads the request's `Content-Type`. Parameters after the type, such as
/// `charset`, are allowed and ignored.
fn declared(headers: &HeaderMap) -> Declared {
    let mut values = headers.get_all(CONTENT_TYPE).iter();
    match (values.next(), values.next()) {
        (None, _) => Declared::Nothing,
        (Some(value), None) => {
            let kind = value.as_bytes().split(|byte| *byte == b';').next();
            if kind.is_some_and(|kind| kind.trim_ascii().eq_ignore_ascii_case(b"application/json"))
            {
                Declared::Json
            } else {
                Declared::Other
            }
        }
        _ => Declared::Other,
    }
}

/// Checks the body's declared type and encoding, then reads it up to the
/// route's cap. A route without a body takes no `Content-Type` and no
/// bytes.
async fn read_body(spec: &RouteSpec, headers: &HeaderMap, body: Body) -> Result<Bytes, ApiError> {
    let (wanted, cap) = match spec.body {
        BodyRule::None => (Declared::Nothing, 0),
        BodyRule::Json(limits) => (Declared::Json, limits.bytes),
    };
    if headers.contains_key(CONTENT_ENCODING) || declared(headers) != wanted {
        return Err(ApiError::new(ProblemCode::UnsupportedBody));
    }
    to_bytes(body, cap)
        .await
        .map_err(|_| ApiError::new(ProblemCode::BodyTooLarge))
}

/// The last-resort layer: a future that turns a panic in the future it
/// wraps into an error holding what the panic said.
struct Guarded<F>(Pin<Box<F>>);

impl<F: Future> Future for Guarded<F> {
    type Output = Result<F::Output, String>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        match catch_unwind(AssertUnwindSafe(|| self.0.as_mut().poll(context))) {
            Ok(poll) => poll.map(Ok),
            Err(payload) => Poll::Ready(Err(panic_text(payload.as_ref()))),
        }
    }
}

/// What a panic said, if it said it in text.
fn panic_text(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| {
            payload
                .downcast_ref::<&str>()
                .map(|text| (*text).to_owned())
        })
        .unwrap_or_default()
}
