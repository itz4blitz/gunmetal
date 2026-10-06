//! The route registry as a whole: the routes of every registered module are
//! served, a registry that is not well formed is refused, the routes that
//! answer without a session are exactly the reviewed list, and every other
//! route gives the one uniform refusal to a request without a valid
//! credential.
//!
//! The anonymous-request suite is generated from the registry, so a route a
//! later package registers is covered the moment its line is added. It runs
//! here over the server's own registry and over stand-in modules, and
//! fixtures that are broken on purpose show that it and the allow-list
//! check fail for the right reason, naming the route.

use core::net::{IpAddr, Ipv4Addr};
use std::io;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::request::Parts;
use axum::http::{Extensions, Request};
use gunmetal_core::audit_event::SecurityEvent;
use gunmetal_core::client_context::{ClientContext, PathClass};
use gunmetal_core::problem::ProblemCode;
use gunmetal_fs::host::HostFacts;
use gunmetal_http::call::{Call, Reply};
use gunmetal_http::client::{TestClient, TestResponse};
use gunmetal_http::host::HostAllowList;
use gunmetal_http::pipeline::{self, AccessRecord, Attempt, HookFuture, Hooks};
use gunmetal_http::problem::{ApiError, RequestId};
use gunmetal_http::route::{
    Access, AccessClass, BodyRule, Effect, JsonLimits, Method, RateClass, RouteSpec, RouteTag,
};
use gunmetal_http::table::{HandlerFuture, RouteEntry, TableError};
use gunmetal_server::app::{AppState, Host};
use gunmetal_server::clock::SystemClock;
use gunmetal_server::config::Env;
use gunmetal_server::host::Privileges;
use gunmetal_server::listener::Network;
use gunmetal_server::routes::{
    AllowListFinding, Edge, Module, REGISTRY, Registered, Registry, RegistryError,
};
use gunmetal_testkit::tempdir::TempDir;

const HOST: &str = "music.example.com";
const ORIGIN: &str = "https://music.example.com";

/// Where the stand-in requests come from: a device on the home network,
/// with no proxy in between.
const PEER: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20));

/// What a handler sees of where a request came from: the address, its path
/// class and whether a trusted proxy carried it.
type Seen = (IpAddr, PathClass, bool);

/// What a handler sees of a request from [`PEER`].
const FROM_PEER: Seen = (PEER, PathClass::Home, false);

/// A started server's state in a scratch directory, and every event its
/// security bus carried.
struct World {
    state: Arc<AppState>,
    events: Arc<Mutex<Vec<SecurityEvent>>>,
    _dir: TempDir,
}

impl World {
    fn start(label: &str) -> Self {
        let dir = TempDir::new(label).expect("scratch");
        let facts = HostFacts::probe(dir.path()).expect("probed");
        // The server refuses to start as root.
        assert_ne!(facts.uid, 0);
        let host = Host {
            privileges: Privileges {
                euid: facts.uid,
                effective: 0,
                permitted: 0,
            },
            facts,
        };
        let state = AppState::start(
            dir.path(),
            &host,
            &Env::default(),
            Arc::new(SystemClock),
            Box::new(io::sink()),
        )
        .expect("started");
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&events);
        state.bus.security.subscribe(move |event: &SecurityEvent| {
            sink.lock().expect("events").push(event.clone());
            Ok(())
        });
        Self {
            state: Arc::new(state),
            events,
            _dir: dir,
        }
    }

    /// What the stand-in handlers told the bus, in order.
    fn events(&self) -> Vec<SecurityEvent> {
        self.events.lock().expect("events").clone()
    }

    /// Where the request each stand-in handler answered came from, in
    /// order, or `None` for a handler that ran with nothing attached.
    fn seen(&self) -> Vec<Option<Seen>> {
        self.events()
            .iter()
            .map(|event| match event {
                SecurityEvent::ExcessRateLimitExceeded { source } => {
                    Some((source.addr(), source.class(), source.via_proxy()))
                }
                other => {
                    assert_eq!(*other, RAN, "only the stand-in handlers publish");
                    None
                }
            })
            .collect()
    }
}

/// What a stand-in handler tells the bus when the listener attached nothing
/// to its request.
const RAN: SecurityEvent = SecurityEvent::GmDebugLoggingEnabled { account: None };

/// The handler every stand-in route has. It reads the shared state: it tells
/// the security bus that it ran, with where the request came from when the
/// listener attached that, and answers with nothing.
fn handler(state: Arc<AppState>) -> impl Fn(Call) -> HandlerFuture + Send + Sync + 'static {
    move |call: Call| -> HandlerFuture {
        let event = call.grant().get::<ClientContext>().map_or(RAN, |source| {
            SecurityEvent::ExcessRateLimitExceeded { source: *source }
        });
        let told = state.bus.security.publish(&event);
        Box::pin(async move {
            told.map(|_| Reply::empty())
                .map_err(|_| ApiError::new(ProblemCode::AuditUnavailable))
        })
    }
}

/// One entry per spec, in order, each with the stand-in handler.
fn entries(specs: &[RouteSpec], state: &Arc<AppState>) -> Vec<RouteEntry> {
    specs
        .iter()
        .map(|spec| RouteEntry::new(*spec, handler(Arc::clone(state))))
        .collect()
}

const fn spec(method: Method, path: &'static str, access: Access, body: BodyRule) -> RouteSpec {
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

const PUBLIC: Access = Access::Public {
    effect: Effect::Reads,
};

const JSON: BodyRule = BodyRule::Json(JsonLimits::DEFAULT);

/// A stand-in module, written as a route package writes its own file: its
/// routes and the one `ROUTES` value that the registry lists on one line.
mod alpha {
    use std::sync::Arc;

    use gunmetal_http::route::{Access, BodyRule, Capability, Effect, Method, RouteSpec};
    use gunmetal_http::table::RouteEntry;
    use gunmetal_server::app::AppState;
    use gunmetal_server::routes::Module;

    use super::{JSON, PUBLIC, entries, spec};

    /// The module's routes.
    pub const SPECS: &[RouteSpec] = &[
        spec(Method::Get, "/api/v1/alpha/open", PUBLIC, BodyRule::None),
        spec(
            Method::Get,
            "/api/v1/alpha/items/{id}",
            Access::User {
                capability: Capability::new("library.read"),
                effect: Effect::Reads,
            },
            BodyRule::None,
        ),
        spec(
            Method::Post,
            "/api/v1/alpha/items",
            Access::User {
                capability: Capability::new("library.write"),
                effect: Effect::Mutates,
            },
            JSON,
        ),
    ];

    /// The module's entries over the shared state.
    fn build(state: &Arc<AppState>) -> Vec<RouteEntry> {
        entries(SPECS, state)
    }

    /// What the registry lists for this module.
    pub const ROUTES: Module = Module {
        name: "alpha",
        specs: SPECS,
        entries: build,
    };
}

/// The second stand-in module, written the same way.
mod beta {
    use std::sync::Arc;

    use gunmetal_http::route::{
        Access, AdminEffect, BodyRule, Capability, CapabilityKind, Effect, ExchangeKind, Method,
        RouteSpec, Target,
    };
    use gunmetal_http::table::RouteEntry;
    use gunmetal_server::app::AppState;
    use gunmetal_server::routes::Module;

    use super::{JSON, PUBLIC, entries, spec};

    /// The module's routes.
    pub const SPECS: &[RouteSpec] = &[
        spec(Method::Get, "/api/v1/beta/open", PUBLIC, BodyRule::None),
        spec(
            Method::Post,
            "/api/v1/beta/sign-in",
            Access::Exchange {
                kind: ExchangeKind::SignIn,
                effect: Effect::Mutates,
            },
            JSON,
        ),
        spec(
            Method::Get,
            "/api/v1/beta/media/{id}",
            Access::Capability {
                kind: CapabilityKind::Media,
                effect: Effect::Reads,
            },
            BodyRule::None,
        ),
        spec(
            Method::Delete,
            "/api/v1/beta/accounts/{id}",
            Access::Admin {
                capability: Capability::new("accounts.manage"),
                effect: AdminEffect::Reads,
                target: Target::OtherPrincipals,
            },
            BodyRule::None,
        ),
    ];

    /// The module's entries over the shared state.
    fn build(state: &Arc<AppState>) -> Vec<RouteEntry> {
        entries(SPECS, state)
    }

    /// What the registry lists for this module.
    pub const ROUTES: Module = Module {
        name: "beta",
        specs: SPECS,
        entries: build,
    };
}

/// The reviewed list for the two stand-in modules: their public,
/// credential-exchange and capability routes.
const STAND_IN_LIST: &str = "# The stand-in modules' open routes.\n\
                             GET /api/v1/alpha/open\n\
                             GET /api/v1/beta/media/{id}\n\
                             GET /api/v1/beta/open\n\
                             POST /api/v1/beta/sign-in\n";

/// Two stand-in modules, registered as a package registers its own.
const STAND_IN: Registry = Registry {
    modules: &[alpha::ROUTES, beta::ROUTES],
    public: STAND_IN_LIST,
};

/// What a listener would tell the pipeline, with a fixed request
/// identifier so that whole responses can be compared.
fn edge() -> Edge {
    Edge {
        hosts: HostAllowList::new(&[HOST]).expect("a host name"),
        origins: vec![ORIGIN.to_owned()],
        https: false,
        request_id: Arc::new(|| RequestId(0x2a)),
    }
}

/// A client for a registry's router over `world`.
fn client(registry: &Registry, world: &World) -> TestClient {
    TestClient::new(
        registry
            .router(&world.state, edge())
            .expect("the registry is accepted"),
    )
}

/// A request as the listener hands it to the router: built, then admitted
/// from [`PEER`] by a server that trusts no proxy.
fn request(method: &str, target: &str, headers: &[(&str, &str)], body: &str) -> Request<Body> {
    let mut request = unadmitted(method, target, headers, body);
    Network::default()
        .admit(PEER, &mut request)
        .expect("the listener admits a plain request");
    request
}

/// A request the listener never prepared, which only a path round the
/// listener could deliver.
fn unadmitted(method: &str, target: &str, headers: &[(&str, &str)], body: &str) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(target)
        .header("host", HOST);
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    builder
        .body(Body::from(body.to_owned()))
        .expect("a well-formed request")
}

fn get(target: &str) -> Request<Body> {
    request("GET", target, &[], "")
}

/// A whole response: the status, every header and the body.
fn response(status: u16, headers: &[(&str, &str)], body: &str) -> TestResponse {
    TestResponse {
        status,
        headers: headers
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect(),
        body: body.as_bytes().to_vec(),
    }
}

/// What a stand-in handler answers.
fn empty() -> TestResponse {
    response(
        204,
        &[
            ("cache-control", "no-store"),
            ("content-length", "0"),
            (
                "content-security-policy",
                "default-src 'none'; frame-ancestors 'none'; sandbox",
            ),
            ("cross-origin-resource-policy", "same-origin"),
            ("referrer-policy", "no-referrer"),
            ("x-content-type-options", "nosniff"),
        ],
        "",
    )
}

/// The one answer to a request without a valid credential, written out
/// whole (SEC-API-003).
fn refused() -> TestResponse {
    response(
        401,
        &[
            ("cache-control", "no-store"),
            ("content-length", "136"),
            (
                "content-security-policy",
                "default-src 'none'; frame-ancestors 'none'; sandbox",
            ),
            ("content-type", "application/problem+json"),
            ("cross-origin-resource-policy", "same-origin"),
            ("referrer-policy", "no-referrer"),
            ("x-content-type-options", "nosniff"),
        ],
        r#"{"type":"urn:gunmetal:problem:unauthenticated","title":"Sign in to continue.","status":401,"request":"0000000000000000000000000000002a"}"#,
    )
}

/// The answer to a request the server cannot account for: the generic
/// failure, which says nothing about why.
fn internal() -> TestResponse {
    response(
        500,
        &[
            ("cache-control", "no-store"),
            ("content-length", "167"),
            (
                "content-security-policy",
                "default-src 'none'; frame-ancestors 'none'; sandbox",
            ),
            ("content-type", "application/problem+json"),
            ("cross-origin-resource-policy", "same-origin"),
            ("referrer-policy", "no-referrer"),
            ("x-content-type-options", "nosniff"),
        ],
        r#"{"type":"urn:gunmetal:problem:internal_error","title":"Something went wrong on the server. Try again later.","status":500,"request":"0000000000000000000000000000002a"}"#,
    )
}

/// The answer to a path no module registered.
fn not_found() -> TestResponse {
    response(
        404,
        &[
            ("cache-control", "no-store"),
            ("content-length", "192"),
            (
                "content-security-policy",
                "default-src 'none'; frame-ancestors 'none'; sandbox",
            ),
            ("content-type", "application/problem+json"),
            ("cross-origin-resource-policy", "same-origin"),
            ("referrer-policy", "no-referrer"),
            ("x-content-type-options", "nosniff"),
        ],
        r#"{"type":"urn:gunmetal:problem:not_found","title":"We couldn't find that. It may have been removed, or you may not have access to it.","status":404,"request":"0000000000000000000000000000002a"}"#,
    )
}

/// The states of a request without a valid credential that need no session
/// to produce: no credential at all, a malformed `Authorization` header,
/// and a malformed session cookie sent as the server's own pages send one.
/// WP-131 adds the expired, revoked and disabled-account states.
const STATES: [(&str, &[(&str, &str)]); 3] = [
    ("no credential", &[]),
    (
        "a malformed Authorization header",
        &[("authorization", "Bearer")],
    ),
    (
        "a malformed session cookie",
        &[
            ("cookie", "__Host-gm_session=%%%"),
            ("gunmetal-request", "1"),
            ("sec-fetch-site", "same-origin"),
            ("origin", ORIGIN),
        ],
    ),
];

/// A path that a template matches: every parameter is given a value.
fn concrete(template: &str) -> String {
    template
        .split('/')
        .map(|segment| {
            if segment.starts_with('{') {
                "x1"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// The anonymous-request suite. It sends one request in each of [`STATES`]
/// to every route that is not on the reviewed list `list`, and to every
/// capability route, listed or not, since none of the states carries a
/// capability. It returns each route and state that was answered with
/// anything but [`refused`].
///
/// Every request carries a body of a type no route accepts. A pipeline that
/// read the body before it refused the caller would answer 415, so the
/// uniform 401 also shows that the body was not read.
fn anonymous_suite(routes: &[Registered], list: &str, client: &TestClient) -> Vec<String> {
    let listed: Vec<&str> = list.lines().collect();
    let mut failures = Vec::new();
    for route in routes {
        let method = route.spec.method.as_str();
        let name = format!("{method} {}", route.spec.path);
        let capability = route.spec.access.class() == AccessClass::Capability;
        if listed.contains(&name.as_str()) && !capability {
            continue;
        }
        for (state, credential) in STATES {
            let mut headers = vec![("content-type", "text/plain")];
            headers.extend_from_slice(credential);
            let target = concrete(route.spec.path);
            let answer = client.send(request(method, &target, &headers, "not read"));
            if answer != refused() {
                failures.push(format!("{name} with {state}"));
            }
        }
    }
    failures
}

/// The hooks of a fixture that is broken on purpose: its access hook admits
/// a request to every route `admits` says, whatever credential it carries.
fn hooks_admitting(admits: fn(&RouteSpec) -> bool) -> Hooks {
    let edge = edge();
    Hooks {
        hosts: edge.hosts,
        origins: edge.origins,
        https: edge.https,
        request_id: edge.request_id,
        posture: Arc::new(|_: &Parts| Ok(())),
        access: Arc::new(move |attempt: Attempt| -> HookFuture<Extensions> {
            let granted = if admits(&attempt.spec) {
                Ok(attempt.context)
            } else {
                Err(ApiError::new(ProblemCode::Unauthenticated))
            };
            Box::pin(async move { granted })
        }),
        rate: Arc::new(|_: RouteSpec, _: Arc<Extensions>| -> HookFuture<()> {
            Box::pin(async { Ok(()) })
        }),
        log: Arc::new(|_: &AccessRecord, _: &Extensions| {}),
    }
}

/// A client for the stand-in modules' routes behind [`hooks_admitting`],
/// built beside the registry, as no server code may.
fn broken_client(world: &World, admits: fn(&RouteSpec) -> bool) -> TestClient {
    let entries: Vec<RouteEntry> = STAND_IN
        .modules
        .iter()
        .flat_map(|module| (module.entries)(&world.state))
        .collect();
    TestClient::new(
        pipeline::router(&entries, hooks_admitting(admits)).expect("a well-formed table"),
    )
}

#[test]
fn the_routes_of_every_registered_module_are_served() {
    let world = World::start("routes-served");
    let client = client(&STAND_IN, &world);
    assert_eq!(
        [
            client.send(get("/api/v1/alpha/open")),
            client.send(get("/api/v1/beta/open")),
            client.send(request(
                "POST",
                "/api/v1/beta/sign-in",
                &[("content-type", "application/json")],
                "{}"
            )),
        ],
        [empty(), empty(), empty()]
    );
    // Each handler was built over the shared state and reached it, and saw
    // where the listener said the request came from.
    assert_eq!(world.seen(), [Some(FROM_PEER); 3]);
    // A path no module registered is not served, and nor is a route whose
    // capability nothing can verify yet.
    assert_eq!(
        [
            client.send(get("/api/v1/gamma/open")),
            client.send(get("/api/v1/beta/media/x1")),
        ],
        [not_found(), refused()]
    );
    assert_eq!(world.seen(), [Some(FROM_PEER); 3]);
    assert_eq!(
        STAND_IN.routes(),
        [
            Registered {
                module: "alpha",
                spec: alpha::SPECS[0]
            },
            Registered {
                module: "alpha",
                spec: alpha::SPECS[1]
            },
            Registered {
                module: "alpha",
                spec: alpha::SPECS[2]
            },
            Registered {
                module: "beta",
                spec: beta::SPECS[0]
            },
            Registered {
                module: "beta",
                spec: beta::SPECS[1]
            },
            Registered {
                module: "beta",
                spec: beta::SPECS[2]
            },
            Registered {
                module: "beta",
                spec: beta::SPECS[3]
            },
        ]
    );
}

const GAMMA: &[RouteSpec] = &[
    spec(Method::Get, "/api/v1/gamma/open", PUBLIC, BodyRule::None),
    spec(Method::Get, "/api/v1/alpha/open", PUBLIC, BodyRule::None),
];

fn gamma(state: &Arc<AppState>) -> Vec<RouteEntry> {
    entries(GAMMA, state)
}

const DELTA: &[RouteSpec] = &[spec(
    Method::Post,
    "/api/v1/alpha/open",
    Access::Public {
        effect: Effect::Mutates,
    },
    BodyRule::None,
)];

fn delta(state: &Arc<AppState>) -> Vec<RouteEntry> {
    entries(DELTA, state)
}

const DOUBLE: &[RouteSpec] = &[
    spec(Method::Get, "/api/v1/double", PUBLIC, BodyRule::None),
    spec(Method::Get, "/api/v1/double", PUBLIC, BodyRule::None),
];

fn double(state: &Arc<AppState>) -> Vec<RouteEntry> {
    entries(DOUBLE, state)
}

/// Two modules that both declare `GET /api/v1/alpha/open`.
const TWICE: Registry = Registry {
    modules: &[
        alpha::ROUTES,
        Module {
            name: "gamma",
            specs: GAMMA,
            entries: gamma,
        },
    ],
    public: "GET /api/v1/alpha/open\nGET /api/v1/gamma/open\n",
};

/// One module that declares a route twice.
const DOUBLED: Registry = Registry {
    modules: &[Module {
        name: "double",
        specs: DOUBLE,
        entries: double,
    }],
    public: "GET /api/v1/double\n",
};

/// Two modules with different methods on one path.
const SHARED: Registry = Registry {
    modules: &[
        alpha::ROUTES,
        Module {
            name: "delta",
            specs: DELTA,
            entries: delta,
        },
    ],
    public: "GET /api/v1/alpha/open\nPOST /api/v1/alpha/open\n",
};

#[test]
fn a_route_declared_twice_is_refused_with_both_modules_names() {
    let world = World::start("routes-twice");
    let refusal = RegistryError::Duplicate {
        method: Method::Get,
        path: "/api/v1/alpha/open",
        first: "alpha",
        second: "gamma",
    };
    assert_eq!(TWICE.check(), Err(refusal.clone()));
    assert_eq!(TWICE.router(&world.state, edge()).map(|_| ()), Err(refusal));
    // A module that repeats its own route is named twice.
    assert_eq!(
        DOUBLED.router(&world.state, edge()).map(|_| ()),
        Err(RegistryError::Duplicate {
            method: Method::Get,
            path: "/api/v1/double",
            first: "double",
            second: "double",
        })
    );
    // Another method on a path another module has is a route of its own.
    assert_eq!(SHARED.check(), Ok(()));
    let client = client(&SHARED, &world);
    assert_eq!(
        [
            client.send(get("/api/v1/alpha/open")),
            client.send(request("POST", "/api/v1/alpha/open", &[], "")),
        ],
        [empty(), empty()]
    );
}

const EPSILON: &[RouteSpec] = &[
    spec(Method::Get, "/api/v1/epsilon/a", PUBLIC, BodyRule::None),
    spec(Method::Get, "/api/v1/epsilon/b", PUBLIC, BodyRule::None),
];

fn epsilon_reversed(state: &Arc<AppState>) -> Vec<RouteEntry> {
    let mut built = entries(EPSILON, state);
    built.reverse();
    built
}

fn epsilon_short(state: &Arc<AppState>) -> Vec<RouteEntry> {
    entries(&EPSILON[..1], state)
}

fn epsilon_long(state: &Arc<AppState>) -> Vec<RouteEntry> {
    let mut built = entries(EPSILON, state);
    built.extend(entries(DELTA, state));
    built
}

fn epsilon_exact(state: &Arc<AppState>) -> Vec<RouteEntry> {
    entries(EPSILON, state)
}

#[test]
fn a_module_whose_entries_are_not_its_declared_routes_is_refused() {
    let world = World::start("routes-mismatch");
    let built = |entries: fn(&Arc<AppState>) -> Vec<RouteEntry>| {
        let modules: &'static [Module] = Box::leak(Box::new([
            alpha::ROUTES,
            Module {
                name: "epsilon",
                specs: EPSILON,
                entries,
            },
        ]));
        Registry {
            modules,
            public: "GET /api/v1/alpha/open\nGET /api/v1/epsilon/a\nGET /api/v1/epsilon/b\n",
        }
        .router(&world.state, edge())
        .map(|_| ())
    };
    let refusal = Err(RegistryError::Mismatch { module: "epsilon" });
    assert_eq!(
        [
            built(epsilon_reversed),
            built(epsilon_short),
            built(epsilon_long),
            built(epsilon_exact),
        ],
        [refusal.clone(), refusal.clone(), refusal, Ok(())]
    );
}

const ZETA: &[RouteSpec] = &[spec(Method::Get, "/zeta", PUBLIC, BodyRule::None)];

fn zeta(state: &Arc<AppState>) -> Vec<RouteEntry> {
    entries(ZETA, state)
}

const ETA: &[RouteSpec] = &[spec(
    Method::Get,
    "/api/v1/alpha/{id}",
    PUBLIC,
    BodyRule::None,
)];

fn eta(state: &Arc<AppState>) -> Vec<RouteEntry> {
    entries(ETA, state)
}

/// A module with a route outside `/api/v1`.
const UNVERSIONED: Registry = Registry {
    modules: &[Module {
        name: "zeta",
        specs: ZETA,
        entries: zeta,
    }],
    public: "GET /zeta\n",
};

/// Two modules whose templates could match the same path.
const OVERLAPPING: Registry = Registry {
    modules: &[
        alpha::ROUTES,
        Module {
            name: "eta",
            specs: ETA,
            entries: eta,
        },
    ],
    public: "GET /api/v1/alpha/open\nGET /api/v1/alpha/{id}\n",
};

#[test]
fn what_the_route_table_refuses_is_refused() {
    let world = World::start("routes-table");
    assert_eq!(
        UNVERSIONED.router(&world.state, edge()).map(|_| ()),
        Err(RegistryError::Table(TableError::Unversioned("/zeta")))
    );
    assert_eq!(
        OVERLAPPING.router(&world.state, edge()).map(|_| ()),
        Err(RegistryError::Table(TableError::Overlap(
            "/api/v1/alpha/open",
            "/api/v1/alpha/{id}"
        )))
    );
}

/// Verifies: SEC-API-002, SEC-IAM-067
#[test]
fn the_server_s_open_routes_are_exactly_the_reviewed_list() {
    assert_eq!(REGISTRY.check(), Ok(()));
    let world = World::start("routes-server");
    assert_eq!(REGISTRY.router(&world.state, edge()).map(|_| ()), Ok(()));
    // The same check accepts the stand-in modules and their list.
    assert_eq!(STAND_IN.check(), Ok(()));
}

/// A fixture broken on purpose: a public route that is not on the reviewed
/// list. The check names it, the router is not built, and the suite, run
/// against a router built beside the registry, names it too.
///
/// Verifies: SEC-API-002, SEC-IAM-067
#[test]
fn an_open_route_that_is_not_on_the_reviewed_list_is_not_served() {
    let world = World::start("routes-unlisted");
    let list = "GET /api/v1/alpha/open\n\
                GET /api/v1/beta/media/{id}\n\
                POST /api/v1/beta/sign-in\n";
    let unlisted = Registry {
        modules: STAND_IN.modules,
        public: list,
    };
    let refusal = RegistryError::AllowList(vec![AllowListFinding::Unlisted {
        module: "beta",
        method: Method::Get,
        path: "/api/v1/beta/open",
    }]);
    assert_eq!(unlisted.check(), Err(refusal.clone()));
    assert_eq!(
        unlisted.router(&world.state, edge()).map(|_| ()),
        Err(refusal)
    );
    // A list that names a route which needs a session is refused as well.
    let closed = Registry {
        modules: STAND_IN.modules,
        public: "DELETE /api/v1/beta/accounts/{id}\n\
                 GET /api/v1/alpha/open\n\
                 GET /api/v1/beta/media/{id}\n\
                 GET /api/v1/beta/open\n\
                 POST /api/v1/beta/sign-in\n",
    };
    assert_eq!(
        closed.router(&world.state, edge()).map(|_| ()),
        Err(RegistryError::AllowList(vec![
            AllowListFinding::Unregistered { line: 1 }
        ]))
    );
    // Served all the same, by a router that skipped the check, the route
    // answers a caller with no credential, and the suite says so.
    let served = broken_client(&world, |spec| {
        matches!(
            spec.access.class(),
            AccessClass::Public | AccessClass::Exchange
        )
    });
    assert_eq!(
        anonymous_suite(&unlisted.routes(), list, &served),
        [
            "GET /api/v1/beta/open with no credential",
            "GET /api/v1/beta/open with a malformed Authorization header",
            "GET /api/v1/beta/open with a malformed session cookie",
        ]
    );
}

/// Every route that is not on the reviewed list, and every capability
/// route, answers a request that carries no credential or a malformed one,
/// and no capability, with the same 401, byte for byte, before it reads the
/// body or runs the handler. WP-131 adds the expired, revoked and
/// disabled-account states.
///
/// Verifies: SEC-API-003, SEC-TM-004
#[test]
fn every_route_outside_the_list_refuses_a_request_without_a_valid_credential() {
    // The server's own registry, as every route package extends it.
    let world = World::start("anonymous-server");
    let server = client(&REGISTRY, &world);
    assert_eq!(
        anonymous_suite(&REGISTRY.routes(), REGISTRY.public, &server),
        [""; 0]
    );
    assert_eq!(world.events(), []);
    // The stand-in modules: a signed-in route with a parameter, one with a
    // body, and an admin route.
    let world = World::start("anonymous-stand-ins");
    let stand_in = client(&STAND_IN, &world);
    assert_eq!(
        [
            stand_in.send(get("/api/v1/alpha/items/x1")),
            stand_in.send(request(
                "POST",
                "/api/v1/alpha/items",
                &[
                    ("content-type", "application/json"),
                    ("authorization", "Bearer")
                ],
                "{}"
            )),
            stand_in.send(request("DELETE", "/api/v1/beta/accounts/x1", &[], "")),
        ],
        [refused(), refused(), refused()]
    );
    assert_eq!(
        anonymous_suite(&STAND_IN.routes(), STAND_IN.public, &stand_in),
        [""; 0]
    );
    // No handler ran.
    assert_eq!(world.events(), []);
}

/// A fixture broken on purpose: an access hook that asks a `GET` for no
/// credential, so a signed-in route and a capability route answer
/// anonymously. The suite names both routes, in every state.
///
/// Verifies: SEC-API-003, SEC-TM-004
#[test]
fn the_suite_names_a_route_that_answers_without_a_valid_credential() {
    let world = World::start("anonymous-broken");
    let broken = broken_client(&world, |spec| spec.method == Method::Get);
    assert_eq!(broken.send(get("/api/v1/alpha/items/x1")), empty());
    assert_eq!(world.seen(), [Some(FROM_PEER)]);
    assert_eq!(
        anonymous_suite(&STAND_IN.routes(), STAND_IN.public, &broken),
        [
            "GET /api/v1/alpha/items/{id} with no credential",
            "GET /api/v1/alpha/items/{id} with a malformed Authorization header",
            "GET /api/v1/alpha/items/{id} with a malformed session cookie",
            "GET /api/v1/beta/media/{id} with no credential",
            "GET /api/v1/beta/media/{id} with a malformed Authorization header",
            "GET /api/v1/beta/media/{id} with a malformed session cookie",
        ]
    );
}

/// A fixture broken on purpose: an access hook that, like a session hook
/// wired in before the capability check, lets through every route that
/// needs no session, so the capability route answers a request that
/// carries no capability. That route is on the reviewed list, and the suite
/// names it all the same, in every state.
///
/// Verifies: SEC-TM-004
#[test]
fn the_suite_names_a_capability_route_that_answers_without_its_capability() {
    let world = World::start("anonymous-capability");
    let broken = broken_client(&world, |spec| {
        !matches!(spec.access.class(), AccessClass::User | AccessClass::Admin)
    });
    assert_eq!(broken.send(get("/api/v1/beta/media/x1")), empty());
    assert_eq!(world.seen(), [Some(FROM_PEER)]);
    assert_eq!(
        anonymous_suite(&STAND_IN.routes(), STAND_IN.public, &broken),
        [
            "GET /api/v1/beta/media/{id} with no credential",
            "GET /api/v1/beta/media/{id} with a malformed Authorization header",
            "GET /api/v1/beta/media/{id} with a malformed session cookie",
        ]
    );
}

/// Verifies: SEC-OPS-037
#[test]
fn a_handler_sees_only_where_the_listener_says_the_request_came_from() {
    const BRIDGE: IpAddr = IpAddr::V4(Ipv4Addr::new(172, 17, 0, 1));
    let world = World::start("routes-context");
    let client = client(&STAND_IN, &world);
    let network = Network {
        gateways: vec![BRIDGE],
        ..Network::default()
    };
    // A peer nobody declared a proxy claims another address.
    let mut spoofed = unadmitted(
        "GET",
        "/api/v1/alpha/open",
        &[("x-forwarded-for", "198.51.100.9")],
        "",
    );
    assert_eq!(network.admit(PEER, &mut spoofed), Ok(()));
    let mut bridged = unadmitted("GET", "/api/v1/beta/open", &[], "");
    assert_eq!(network.admit(BRIDGE, &mut bridged), Ok(()));
    assert_eq!(
        [client.send(spoofed), client.send(bridged)],
        [empty(), empty()]
    );
    assert_eq!(
        world.seen(),
        [
            Some((PEER, PathClass::Internet, false)),
            Some((BRIDGE, PathClass::Unknown, false)),
        ]
    );
}

/// A request that reaches the router without passing the listener carries
/// no `ClientContext`, so nothing can say where it came from. It is refused
/// as the server's own failure before its route is looked up, and no
/// handler runs: the router fails closed.
///
/// Verifies: SEC-OPS-037
#[test]
fn a_request_the_listener_did_not_admit_is_refused() {
    let world = World::start("routes-unadmitted");
    let client = client(&STAND_IN, &world);
    assert_eq!(
        [
            client.send(unadmitted("GET", "/api/v1/alpha/open", &[], "")),
            client.send(unadmitted(
                "POST",
                "/api/v1/beta/sign-in",
                &[("content-type", "application/json")],
                "{}"
            )),
            client.send(unadmitted("GET", "/api/v1/gamma/open", &[], "")),
        ],
        [internal(), internal(), internal()]
    );
    assert_eq!(world.events(), []);
}
