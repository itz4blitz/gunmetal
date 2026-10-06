//! The route registry: every module that answers HTTP requests, one line
//! each, and the one way the server's router is built from them.
//!
//! - **One table.** [`MODULES`] names every module that has routes. A
//!   module declares its routes as a static table of [`RouteSpec`] values,
//!   so the allow-list check, the anonymous-request suite and the API
//!   description read every route's policy without starting a server, and
//!   it builds one handler entry per spec over the shared [`AppState`].
//!   [`Registry::router`] is the only place the server asks the HTTP
//!   foundation for a router, and it takes nothing but the registry, so a
//!   route registered anywhere else is not served (SEC-API-001,
//!   SEC-IAM-067).
//! - **Checked at start-up.** The router is refused when two modules
//!   declare the same method on the same path, naming both; when the routes
//!   that answer without a session are not exactly the reviewed list in
//!   `security/public-routes.txt` (SEC-API-002); when a module's entries
//!   are not its declared routes, in order; or when the route table itself
//!   refuses the routes.
//! - **Deny by default.** The pipeline's hooks are put together in one
//!   function here, one line each. Until the session layer (WP-062) and the
//!   authorisation layer (WP-065) replace the access line, no credential
//!   can be verified, so only a route that needs none is admitted, and
//!   every other route answers the one uniform 401 (SEC-API-003,
//!   SEC-TM-004). A capability route needs a capability that nothing can
//!   verify yet, so it is refused too.
//! - **Fail closed.** Every request the listener serves carries the
//!   [`ClientContext`] it resolved. A request without one reached the
//!   router some other way, so nothing can say where it came from: it is
//!   refused with the generic 500 before its route is looked up
//!   (SEC-OPS-037).
//!
//! This file is a registry. A package that adds routes declares, in its
//! own module, `pub const ROUTES: Module`, and adds one line,
//! `crate::<module>::ROUTES,`, to [`MODULES`], sorted by name. For each
//! route that answers without a session, it adds one line to
//! `security/public-routes.txt`. The comment that opens [`MODULES`] keeps
//! the formatter from joining its lines, so two packages never edit the
//! same line.

use std::sync::Arc;

use axum::Router;
use axum::http::Extensions;
use axum::http::request::Parts;
use gunmetal_core::client_context::ClientContext;
use gunmetal_core::problem::ProblemCode;
use gunmetal_http::host::HostAllowList;
use gunmetal_http::pipeline::{self, AccessRecord, Attempt, HookFuture, Hooks, RequestIdHook};
use gunmetal_http::problem::ApiError;
use gunmetal_http::route::{AccessClass, Method, RouteSpec};
use gunmetal_http::table::{RouteEntry, TableError};

use crate::app::AppState;

/// One module's routes.
pub struct Module {
    /// The module's name, as a start-up refusal gives it.
    pub name: &'static str,
    /// Every route the module answers, each with its policy.
    pub specs: &'static [RouteSpec],
    /// Builds the module's entries over the shared state: one per spec, in
    /// the same order.
    pub entries: fn(&Arc<AppState>) -> Vec<RouteEntry>,
}

/// Every module that answers HTTP requests: one line per module, its
/// `ROUTES`, sorted by name.
pub const MODULES: &[Module] = &[
    // One line per module, sorted by name: `crate::<module>::ROUTES,`.
];

/// The reviewed list of the routes that answer without a session
/// (SEC-API-002).
pub const PUBLIC_ROUTES: &str = include_str!("../security/public-routes.txt");

/// The server's registry: its modules and the reviewed list.
pub const REGISTRY: Registry = Registry {
    modules: MODULES,
    public: PUBLIC_ROUTES,
};

/// A set of modules, and the reviewed list that the routes of theirs which
/// answer without a session must equal.
pub struct Registry {
    /// The modules, in registry order.
    pub modules: &'static [Module],
    /// The reviewed list: one line per route, a method, one space and the
    /// path template, sorted. A line that starts with `#` is a comment.
    pub public: &'static str,
}

/// One registered route and the module that registered it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Registered {
    /// The module's name.
    pub module: &'static str,
    /// The route.
    pub spec: RouteSpec,
}

/// Why a registry was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    /// The same method on the same path is declared twice.
    Duplicate {
        /// The method.
        method: Method,
        /// The path template.
        path: &'static str,
        /// The module that declares it first.
        first: &'static str,
        /// The module that declares it again.
        second: &'static str,
    },
    /// The routes that answer without a session are not the reviewed list
    /// (SEC-API-002).
    AllowList(Vec<AllowListFinding>),
    /// A module's entries are not its declared routes, in order.
    Mismatch {
        /// The module.
        module: &'static str,
    },
    /// The route table refused the routes.
    Table(TableError),
}

/// One way the routes that answer without a session differ from the
/// reviewed list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllowListFinding {
    /// A line of the list does not come after the line before it. The list
    /// is sorted and names each route once.
    Unsorted {
        /// The line, counted from 1.
        line: usize,
    },
    /// A line of the list names no registered route that answers without a
    /// session.
    Unregistered {
        /// The line, counted from 1.
        line: usize,
    },
    /// A route answers without a session and is not on the list.
    Unlisted {
        /// The module that registered the route.
        module: &'static str,
        /// The route's method.
        method: Method,
        /// The route's path template.
        path: &'static str,
    },
}

/// What the listener knows about how requests reach the server, which the
/// pipeline needs.
pub struct Edge {
    /// The hosts the server answers to (SEC-API-007).
    pub hosts: HostAllowList,
    /// The origins of the server's own pages, exactly as a browser sends
    /// them in `Origin`.
    pub origins: Vec<String>,
    /// Whether the listener serves HTTPS.
    pub https: bool,
    /// Draws each request's identifier.
    pub request_id: RequestIdHook,
}

impl Registry {
    /// Every registered route with its module, in registry order.
    #[must_use]
    pub fn routes(&self) -> Vec<Registered> {
        self.modules
            .iter()
            .flat_map(|module| {
                module.specs.iter().map(move |spec| Registered {
                    module: module.name,
                    spec: *spec,
                })
            })
            .collect()
    }

    /// Checks everything about the registry that needs no running server:
    /// no method on a path is declared twice, and the routes that answer
    /// without a session are exactly the reviewed list.
    ///
    /// # Errors
    ///
    /// The first route declared twice, with both modules; otherwise every
    /// difference from the reviewed list.
    pub fn check(&self) -> Result<(), RegistryError> {
        let routes = self.routes();
        duplicate(&routes)?;
        let findings = allow_list(&routes, self.public);
        if findings.is_empty() {
            Ok(())
        } else {
            Err(RegistryError::AllowList(findings))
        }
    }

    /// Builds the server's router: every module's entries over `state`,
    /// behind the request pipeline.
    ///
    /// # Errors
    ///
    /// What [`Registry::check`] refuses, a module whose entries are not its
    /// declared routes, and what the route table refuses.
    pub fn router(&self, state: &Arc<AppState>, edge: Edge) -> Result<Router, RegistryError> {
        self.check()?;
        let entries = self.entries(state)?;
        pipeline::router(&entries, hooks(edge)).map_err(RegistryError::Table)
    }

    /// Every module's entries, each module's checked against the routes it
    /// declared.
    fn entries(&self, state: &Arc<AppState>) -> Result<Vec<RouteEntry>, RegistryError> {
        let mut entries = Vec::new();
        for module in self.modules {
            let built = (module.entries)(state);
            if !built.iter().map(RouteEntry::spec).eq(module.specs) {
                return Err(RegistryError::Mismatch {
                    module: module.name,
                });
            }
            entries.extend(built);
        }
        Ok(entries)
    }
}

/// Refuses the first route that repeats the method and path of a route
/// before it.
fn duplicate(routes: &[Registered]) -> Result<(), RegistryError> {
    for (at, route) in routes.iter().enumerate() {
        let earlier = routes.iter().take(at).find(|earlier| {
            earlier.spec.method == route.spec.method && earlier.spec.path == route.spec.path
        });
        if let Some(earlier) = earlier {
            return Err(RegistryError::Duplicate {
                method: route.spec.method,
                path: route.spec.path,
                first: earlier.module,
                second: route.module,
            });
        }
    }
    Ok(())
}

/// Compares the routes that answer without a session, which are the public,
/// credential-exchange and capability routes, with the reviewed list
/// `text`. Findings about the list's lines come first, in line order, then
/// the routes that are not on it, in registry order.
#[must_use]
pub fn allow_list(routes: &[Registered], text: &str) -> Vec<AllowListFinding> {
    let open: Vec<(String, &Registered)> = routes
        .iter()
        .filter(|route| {
            !matches!(
                route.spec.access.class(),
                AccessClass::User | AccessClass::Admin
            )
        })
        .map(|route| {
            let name = format!("{} {}", route.spec.method.as_str(), route.spec.path);
            (name, route)
        })
        .collect();
    let mut findings = Vec::new();
    let mut listed: Vec<&str> = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.starts_with('#') {
            continue;
        }
        let number = index + 1;
        if listed.last().is_some_and(|last| *last >= line) {
            findings.push(AllowListFinding::Unsorted { line: number });
        }
        if !open.iter().any(|(name, _)| name == line) {
            findings.push(AllowListFinding::Unregistered { line: number });
        }
        listed.push(line);
    }
    findings.extend(
        open.iter()
            .filter(|(name, _)| !listed.contains(&name.as_str()))
            .map(|(_, route)| AllowListFinding::Unlisted {
                module: route.module,
                method: route.spec.method,
                path: route.spec.path,
            }),
    );
    findings
}

/// Whether a caller whose credential nothing has verified may use a route
/// of this class. No package verifies a credential or a capability yet, so
/// only the classes that need neither are admitted: the public routes, and
/// the credential exchanges, whose handlers check what they are handed.
fn admit(class: AccessClass) -> Result<(), ApiError> {
    match class {
        AccessClass::Public | AccessClass::Exchange => Ok(()),
        AccessClass::Capability | AccessClass::User | AccessClass::Admin => {
            Err(ApiError::new(ProblemCode::Unauthenticated))
        }
    }
}

/// Whether the listener prepared this request: every request it serves
/// carries the [`ClientContext`] it resolved. A request without one came
/// some other way, so nothing can say where it came from, and it is refused
/// as the server's own failure, never served (SEC-OPS-037).
fn located(parts: &Parts) -> Result<(), ApiError> {
    if parts.extensions.get::<ClientContext>().is_some() {
        Ok(())
    } else {
        Err(ApiError::new(ProblemCode::InternalError))
    }
}

/// The pipeline's hooks: what the listener knows, then one line per hook.
/// The package that builds a hook replaces its line.
fn hooks(edge: Edge) -> Hooks {
    Hooks {
        hosts: edge.hosts,
        origins: edge.origins,
        https: edge.https,
        request_id: edge.request_id,
        // The posture step first asks for the listener's context, so a
        // request without one is refused before anything else (fail
        // closed). WP-132's posture checks go after it; until then no
        // request is refused for how it arrived.
        posture: Arc::new(located),
        // WP-062's authentication and WP-065's capability check go here.
        // Until then the grant is what the listener attached and nothing
        // more.
        access: Arc::new(|attempt: Attempt| -> HookFuture<Extensions> {
            let granted = admit(attempt.spec.access.class()).map(|()| attempt.context);
            Box::pin(async move { granted })
        }),
        // WP-130's rate-limit hook goes here.
        rate: Arc::new(|_: RouteSpec, _: Arc<Extensions>| -> HookFuture<()> {
            Box::pin(async { Ok(()) })
        }),
        // The access log goes here, once the logger has an event for it.
        log: Arc::new(|_: &AccessRecord, _: &Extensions| {}),
    }
}

#[cfg(test)]
mod tests {
    use gunmetal_http::route::{
        Access, AdminEffect, BodyRule, Capability, CapabilityKind, Effect, ExchangeKind, Method,
        RateClass, RouteSpec, RouteTag, Target,
    };

    use super::{AllowListFinding, Registered, allow_list};

    const PUBLIC: Access = Access::Public {
        effect: Effect::Reads,
    };

    const USER: Access = Access::User {
        capability: Capability::new("library.read"),
        effect: Effect::Reads,
    };

    const ADMIN: Access = Access::Admin {
        capability: Capability::new("accounts.manage"),
        effect: AdminEffect::Reads,
        target: Target::OtherPrincipals,
    };

    const EXCHANGE: Access = Access::Exchange {
        kind: ExchangeKind::SignIn,
        effect: Effect::Mutates,
    };

    const CAPABILITY: Access = Access::Capability {
        kind: CapabilityKind::Media,
        effect: Effect::Reads,
    };

    /// A route that module `module` registered.
    fn route(
        module: &'static str,
        method: Method,
        path: &'static str,
        access: Access,
    ) -> Registered {
        Registered {
            module,
            spec: RouteSpec {
                method,
                path,
                access,
                tag: RouteTag::None,
                body: BodyRule::None,
                ids: &[],
                rate: RateClass::Read,
            },
        }
    }

    /// Two stand-in modules' routes: one of every access class.
    fn routes() -> Vec<Registered> {
        vec![
            route("alpha", Method::Get, "/api/v1/alpha/open", PUBLIC),
            route("alpha", Method::Get, "/api/v1/alpha/items", USER),
            route("beta", Method::Post, "/api/v1/beta/sign-in", EXCHANGE),
            route("beta", Method::Get, "/api/v1/beta/media/{id}", CAPABILITY),
            route("beta", Method::Delete, "/api/v1/beta/accounts/{id}", ADMIN),
        ]
    }

    /// Verifies: SEC-API-002
    #[test]
    fn a_list_that_names_exactly_the_open_routes_has_no_findings() {
        let list = "# Reviewed.\n\
                    GET /api/v1/alpha/open\n\
                    GET /api/v1/beta/media/{id}\n\
                    POST /api/v1/beta/sign-in\n";
        assert_eq!(allow_list(&routes(), list), []);
        // Routes that need a session are on no list.
        let closed = [
            route("alpha", Method::Get, "/api/v1/alpha/items", USER),
            route("beta", Method::Delete, "/api/v1/beta/accounts/{id}", ADMIN),
        ];
        assert_eq!(allow_list(&closed, ""), []);
        assert_eq!(allow_list(&closed, "# Nothing is open.\n"), []);
        assert_eq!(allow_list(&[], ""), []);
    }

    /// Verifies: SEC-API-002, SEC-IAM-067
    #[test]
    fn an_open_route_that_is_not_on_the_list_is_named() {
        assert_eq!(
            allow_list(&routes(), "GET /api/v1/beta/media/{id}\n"),
            [
                AllowListFinding::Unlisted {
                    module: "alpha",
                    method: Method::Get,
                    path: "/api/v1/alpha/open",
                },
                AllowListFinding::Unlisted {
                    module: "beta",
                    method: Method::Post,
                    path: "/api/v1/beta/sign-in",
                },
            ]
        );
        // A comment that looks like a route lists nothing.
        assert_eq!(
            allow_list(
                &[route("alpha", Method::Get, "/api/v1/alpha/open", PUBLIC)],
                "#GET /api/v1/alpha/open\n"
            ),
            [AllowListFinding::Unlisted {
                module: "alpha",
                method: Method::Get,
                path: "/api/v1/alpha/open",
            }]
        );
    }

    /// Verifies: SEC-API-002
    #[test]
    fn a_line_that_names_no_open_route_is_refused_by_its_number() {
        // Line 2 names an admin route and line 3 a signed-in one, line 6 a
        // route nobody registered, line 8 a method in the wrong case and
        // line 9 nothing at all.
        let list = "# Reviewed.\n\
                    DELETE /api/v1/beta/accounts/{id}\n\
                    GET /api/v1/alpha/items\n\
                    GET /api/v1/alpha/open\n\
                    GET /api/v1/beta/media/{id}\n\
                    GET /api/v1/gone\n\
                    POST /api/v1/beta/sign-in\n\
                    get /api/v1/alpha/open\n\
                    post\n";
        assert_eq!(
            allow_list(&routes(), list),
            [
                AllowListFinding::Unregistered { line: 2 },
                AllowListFinding::Unregistered { line: 3 },
                AllowListFinding::Unregistered { line: 6 },
                AllowListFinding::Unregistered { line: 8 },
                AllowListFinding::Unregistered { line: 9 },
            ]
        );
        // A blank line is a line like any other.
        assert_eq!(
            allow_list(&[], "\n"),
            [AllowListFinding::Unregistered { line: 1 }]
        );
    }

    #[test]
    fn a_list_out_of_order_or_with_a_route_twice_is_refused_by_line() {
        let list = "POST /api/v1/beta/sign-in\n\
                    GET /api/v1/alpha/open\n\
                    GET /api/v1/alpha/open\n\
                    GET /api/v1/beta/media/{id}\n";
        assert_eq!(
            allow_list(&routes(), list),
            [
                AllowListFinding::Unsorted { line: 2 },
                AllowListFinding::Unsorted { line: 3 },
            ]
        );
        // A comment between two lines does not hide their order.
        let list = "GET /api/v1/beta/media/{id}\n\
                    # Out of place:\n\
                    GET /api/v1/alpha/open\n\
                    POST /api/v1/beta/sign-in\n";
        assert_eq!(
            allow_list(&routes(), list),
            [AllowListFinding::Unsorted { line: 3 }]
        );
    }
}
