//! `xtask openapi`: the `OpenAPI` description of the server's routes,
//! generated from the route registry (SEC-API-091).
//!
//! - `openapi` writes the description to standard output. The committed
//!   copy, [`DESCRIPTION`], is that output and is never edited by hand:
//!   regenerate it with
//!   `cargo run --locked -p xtask -- openapi >crates/gunmetal-server/openapi.json`.
//! - `openapi check` fails when the committed copy is not what the registry
//!   generates, naming the first line that differs. This crate's tests run
//!   it against the real repository, so the gate fails on a route added,
//!   changed or removed without regenerating.
//!
//! This crate links the server's library as it ships, without its test
//! configuration, so the routes described are the routes a build serves: a
//! route that exists only in tests is not in the description, and a route
//! the registry holds is.
//!
//! The description says what the route table knows. Each path template is
//! a path item with one operation per method. An operation has the module
//! that registered it as its tag, its path parameters, a JSON request body
//! where the route takes one, with the route's size limits as
//! `x-gunmetal-body`, and the route's policy as `x-gunmetal-policy`: its
//! access class and what that class needs, whether it changes state, its
//! route tag, where its object identifiers sit and its rate class, each
//! named as the route table names it. It does not list the fields of a
//! request or a response, which the route table does not hold.
//!
//! The text is stable: paths in byte order, methods in the order of their
//! names, one operation per line, so a changed route is a changed line in
//! review.

use std::collections::BTreeMap;
use std::fmt::Debug;

use gunmetal_http::route::{Access, BodyRule, RouteSpec};
use gunmetal_server::routes::Registered;

use crate::tree::Tree;

/// The committed description.
pub const DESCRIPTION: &str = "crates/gunmetal-server/openapi.json";

/// Something `openapi check` found wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finding {
    /// The committed description is not the one the route registry
    /// generates. Regenerate it with the `openapi` subcommand.
    Stale {
        /// The first line that differs, counted from 1.
        line: usize,
    },
}

/// Compares the committed description with the one `routes` generate.
pub fn check(tree: &dyn Tree, routes: &[Registered]) -> Vec<Finding> {
    let committed = tree.read(DESCRIPTION).unwrap_or_default();
    difference(&committed, &render(routes))
        .map(|line| Finding::Stale { line })
        .into_iter()
        .collect()
}

/// The first line at which two texts differ, counted from 1, or `None`
/// when they are the same text.
fn difference(committed: &str, generated: &str) -> Option<usize> {
    if committed == generated {
        return None;
    }
    let same = committed
        .lines()
        .zip(generated.lines())
        .take_while(|(left, right)| left == right)
        .count();
    Some(same + 1)
}

/// The description of `routes`.
pub fn render(routes: &[Registered]) -> String {
    let mut paths: BTreeMap<&str, BTreeMap<String, String>> = BTreeMap::new();
    for route in routes {
        paths.entry(route.spec.path).or_default().insert(
            route.spec.method.as_str().to_ascii_lowercase(),
            operation(route),
        );
    }
    let items: Vec<String> = paths
        .iter()
        .map(|(path, operations)| {
            let lines: Vec<String> = operations
                .iter()
                .map(|(method, operation)| format!("      {}: {operation}", quoted(method)))
                .collect();
            format!("    {}: {{\n{}\n    }}", quoted(path), lines.join(",\n"))
        })
        .collect();
    let paths = if items.is_empty() {
        "{}".to_owned()
    } else {
        format!("{{\n{}\n  }}", items.join(",\n"))
    };
    format!(
        "{{\n  \"openapi\": \"3.1.0\",\n  \"info\": {{\"title\": \"Gunmetal API\", \"version\": \"1\"}},\n  \"paths\": {paths}\n}}\n"
    )
}

/// One route's operation object, on one line.
fn operation(route: &Registered) -> String {
    let mut members = vec![
        format!("\"tags\": [{}]", quoted(route.module)),
        format!("\"parameters\": [{}]", parameters(route.spec.path)),
    ];
    if let BodyRule::Json(limits) = route.spec.body {
        members.push(
            "\"requestBody\": {\"required\": true, \"content\": {\"application/json\": {\"schema\": {\"type\": \"object\"}}}}"
                .to_owned(),
        );
        members.push(format!(
            "\"x-gunmetal-body\": {{\"bytes\": {}, \"items\": {}}}",
            limits.bytes, limits.items
        ));
    }
    members.push(format!(
        "\"x-gunmetal-policy\": {{{}}}",
        policy(&route.spec)
    ));
    format!("{{{}}}", members.join(", "))
}

/// The parameter objects for the parameters of a path template, in order.
fn parameters(path: &str) -> String {
    let parameters: Vec<String> = path
        .split('/')
        .filter_map(|segment| {
            segment
                .strip_prefix('{')
                .and_then(|rest| rest.strip_suffix('}'))
        })
        .map(|name| {
            format!(
                "{{\"name\": {}, \"in\": \"path\", \"required\": true, \"schema\": {{\"type\": \"string\"}}}}",
                quoted(name)
            )
        })
        .collect();
    parameters.join(", ")
}

/// The members of a route's policy object.
fn policy(spec: &RouteSpec) -> String {
    let mut members = vec![format!("\"access\": {}", named(&spec.access.class()))];
    match spec.access {
        Access::Public { .. } => {}
        Access::Exchange { kind, .. } => {
            members.push(format!("\"exchange\": {}", named(&kind)));
        }
        Access::Capability { kind, .. } => {
            members.push(format!("\"carries\": {}", named(&kind)));
        }
        Access::User { capability, .. } => {
            members.push(format!("\"capability\": {}", quoted(capability.name())));
        }
        Access::Admin {
            capability, target, ..
        } => {
            members.push(format!("\"capability\": {}", quoted(capability.name())));
            members.push(format!("\"target\": {}", named(&target)));
        }
    }
    members.push(format!("\"mutates\": {}", spec.mutates()));
    members.push(format!("\"tag\": {}", named(&spec.tag)));
    let ids: Vec<String> = spec.ids.iter().map(named).collect();
    members.push(format!("\"ids\": [{}]", ids.join(", ")));
    members.push(format!("\"rate\": {}", named(&spec.rate)));
    members.join(", ")
}

/// The name the route table gives a value, as a JSON string.
fn named<T: Debug>(value: &T) -> String {
    quoted(&format!("{value:?}"))
}

/// `text` as a JSON string.
fn quoted(text: &str) -> String {
    let escaped: String = text
        .chars()
        .map(|character| match character {
            '"' => "\\\"".to_owned(),
            '\\' => "\\\\".to_owned(),
            control if control < ' ' => format!("\\u{:04x}", u32::from(control)),
            other => other.to_string(),
        })
        .collect();
    format!("\"{escaped}\"")
}

#[cfg(test)]
mod tests {
    use gunmetal_http::route::{
        Access, AdminEffect, BodyRule, Capability, CapabilityKind, Effect, ExchangeKind, IdPlace,
        JsonLimits, Method, RateClass, RouteSpec, RouteTag, Target,
    };
    use gunmetal_server::routes::Registered;

    use super::{DESCRIPTION, Finding, check, quoted, render};
    use crate::tree::memory::Memory;
    use crate::tree::{Disk, Tree};
    use crate::{Failure, ROOT, dispatch, json};

    /// The plainest route: a public read with nothing else to say. The
    /// stand-in routes below say how each differs from it.
    const PLAIN: RouteSpec = RouteSpec {
        method: Method::Get,
        path: "/api/v1/alpha/open",
        access: Access::Public {
            effect: Effect::Reads,
        },
        tag: RouteTag::None,
        body: BodyRule::None,
        ids: &[],
        rate: RateClass::Read,
    };

    const WRITE: Access = Access::User {
        capability: Capability::new("library.write"),
        effect: Effect::Mutates,
    };

    /// The routes of a stand-in module, `beta`: a credential exchange with
    /// a body, a capability route with two parameters and an admin route.
    fn beta() -> Vec<Registered> {
        let specs = [
            RouteSpec {
                method: Method::Post,
                path: "/api/v1/beta/sign-in",
                access: Access::Exchange {
                    kind: ExchangeKind::SignIn,
                    effect: Effect::Mutates,
                },
                body: BodyRule::Json(JsonLimits::DEFAULT),
                rate: RateClass::Exchange,
                ..PLAIN
            },
            RouteSpec {
                path: "/api/v1/beta/media/{id}/parts/{part}",
                access: Access::Capability {
                    kind: CapabilityKind::Media,
                    effect: Effect::Reads,
                },
                ids: &[IdPlace::Path],
                ..PLAIN
            },
            RouteSpec {
                path: "/api/v1/beta/accounts",
                access: Access::Admin {
                    capability: Capability::new("accounts.manage"),
                    effect: AdminEffect::Reads,
                    target: Target::OtherPrincipals,
                },
                tag: RouteTag::Elevated,
                ids: &[IdPlace::Query],
                rate: RateClass::Expensive,
                ..PLAIN
            },
        ];
        specs
            .map(|spec| Registered {
                module: "beta",
                spec,
            })
            .to_vec()
    }

    /// The routes of a stand-in module, `alpha`: three methods on one
    /// path, registered out of order, and the plainest route.
    fn alpha() -> Vec<Registered> {
        let specs = [
            RouteSpec {
                method: Method::Patch,
                path: "/api/v1/alpha/items/{id}",
                access: WRITE,
                tag: RouteTag::FreshUv,
                body: BodyRule::Json(JsonLimits {
                    bytes: 4096,
                    items: 8,
                }),
                ids: &[IdPlace::Path, IdPlace::Body],
                rate: RateClass::Write,
            },
            RouteSpec {
                path: "/api/v1/alpha/items/{id}",
                access: Access::User {
                    capability: Capability::new("library.read"),
                    effect: Effect::Reads,
                },
                ids: &[IdPlace::Path],
                ..PLAIN
            },
            RouteSpec {
                method: Method::Delete,
                path: "/api/v1/alpha/items/{id}",
                access: WRITE,
                ids: &[IdPlace::Path],
                rate: RateClass::Write,
                ..PLAIN
            },
            PLAIN,
        ];
        specs
            .map(|spec| Registered {
                module: "alpha",
                spec,
            })
            .to_vec()
    }

    /// Two stand-in modules' routes, in the order they were registered,
    /// which is not the order they are described in.
    fn stand_ins() -> Vec<Registered> {
        [beta(), alpha()].concat()
    }

    /// The description of [`stand_ins`], written out by hand.
    const EXPECTED: &str = r#"{
  "openapi": "3.1.0",
  "info": {"title": "Gunmetal API", "version": "1"},
  "paths": {
    "/api/v1/alpha/items/{id}": {
      "delete": {"tags": ["alpha"], "parameters": [{"name": "id", "in": "path", "required": true, "schema": {"type": "string"}}], "x-gunmetal-policy": {"access": "User", "capability": "library.write", "mutates": true, "tag": "None", "ids": ["Path"], "rate": "Write"}},
      "get": {"tags": ["alpha"], "parameters": [{"name": "id", "in": "path", "required": true, "schema": {"type": "string"}}], "x-gunmetal-policy": {"access": "User", "capability": "library.read", "mutates": false, "tag": "None", "ids": ["Path"], "rate": "Read"}},
      "patch": {"tags": ["alpha"], "parameters": [{"name": "id", "in": "path", "required": true, "schema": {"type": "string"}}], "requestBody": {"required": true, "content": {"application/json": {"schema": {"type": "object"}}}}, "x-gunmetal-body": {"bytes": 4096, "items": 8}, "x-gunmetal-policy": {"access": "User", "capability": "library.write", "mutates": true, "tag": "FreshUv", "ids": ["Path", "Body"], "rate": "Write"}}
    },
    "/api/v1/alpha/open": {
      "get": {"tags": ["alpha"], "parameters": [], "x-gunmetal-policy": {"access": "Public", "mutates": false, "tag": "None", "ids": [], "rate": "Read"}}
    },
    "/api/v1/beta/accounts": {
      "get": {"tags": ["beta"], "parameters": [], "x-gunmetal-policy": {"access": "Admin", "capability": "accounts.manage", "target": "OtherPrincipals", "mutates": false, "tag": "Elevated", "ids": ["Query"], "rate": "Expensive"}}
    },
    "/api/v1/beta/media/{id}/parts/{part}": {
      "get": {"tags": ["beta"], "parameters": [{"name": "id", "in": "path", "required": true, "schema": {"type": "string"}}, {"name": "part", "in": "path", "required": true, "schema": {"type": "string"}}], "x-gunmetal-policy": {"access": "Capability", "carries": "Media", "mutates": false, "tag": "None", "ids": ["Path"], "rate": "Read"}}
    },
    "/api/v1/beta/sign-in": {
      "post": {"tags": ["beta"], "parameters": [], "requestBody": {"required": true, "content": {"application/json": {"schema": {"type": "object"}}}}, "x-gunmetal-body": {"bytes": 65536, "items": 1000}, "x-gunmetal-policy": {"access": "Exchange", "exchange": "SignIn", "mutates": true, "tag": "None", "ids": [], "rate": "Exchange"}}
    }
  }
}
"#;

    /// Runs `xtask` with `args` against the real repository.
    fn run(args: &[&str]) -> (Result<(), Failure>, String) {
        let args: Vec<String> = args.iter().map(|&arg| arg.to_owned()).collect();
        let mut out = Vec::new();
        let result = dispatch(&args, ROOT, 0, &[], &mut out);
        (result, String::from_utf8(out).expect("output is UTF-8"))
    }

    /// Verifies: SEC-API-091
    #[test]
    fn describes_the_stand_in_routes_as_the_literal_document() {
        assert_eq!(render(&stand_ins()), EXPECTED);
        // The xtask's own reader takes it as one JSON document.
        assert!(json::parse(EXPECTED).is_some());
    }

    #[test]
    fn describes_no_routes_as_a_document_with_no_paths() {
        assert_eq!(
            render(&[]),
            "{\n  \"openapi\": \"3.1.0\",\n  \"info\": {\"title\": \"Gunmetal API\", \"version\": \"1\"},\n  \"paths\": {}\n}\n"
        );
    }

    #[test]
    fn writes_text_as_json_strings() {
        assert_eq!(
            quoted("a \"b\" \\ c\n\u{1f}d \u{e9}"),
            r#""a \"b\" \\ c\u000a\u001fd é""#
        );
        assert_eq!(
            json::parse(&quoted("a \"b\" \\ c\n\u{1f}d \u{e9}")),
            Some(json::Value::String(
                "a \"b\" \\ c\n\u{1f}d \u{e9}".to_owned()
            ))
        );
        assert_eq!(quoted(""), "\"\"");
    }

    /// Verifies: SEC-API-091
    #[test]
    fn a_route_changed_without_regenerating_fails_the_check() {
        let committed = Memory::default().with(DESCRIPTION, EXPECTED);
        assert_eq!(check(&committed, &stand_ins()), []);
        // A route added after the description was generated: the path item
        // before it now ends with a comma.
        let mut added = stand_ins();
        added.push(Registered {
            module: "gamma",
            spec: RouteSpec {
                path: "/api/v1/gamma/new",
                ..PLAIN
            },
        });
        assert_eq!(check(&committed, &added), [Finding::Stale { line: 21 }]);
        // A route removed: the first path item has one operation fewer.
        let mut removed = stand_ins();
        removed.retain(|route| route.spec.method != Method::Delete);
        assert_eq!(check(&committed, &removed), [Finding::Stale { line: 6 }]);
        // No routes at all against a description of some.
        assert_eq!(check(&committed, &[]), [Finding::Stale { line: 4 }]);
    }

    #[test]
    fn a_missing_or_edited_description_fails_the_check() {
        assert_eq!(
            check(&Memory::default(), &stand_ins()),
            [Finding::Stale { line: 1 }]
        );
        let edited = EXPECTED.replace("\"version\": \"1\"", "\"version\": \"2\"");
        assert_eq!(
            check(&Memory::default().with(DESCRIPTION, &edited), &stand_ins()),
            [Finding::Stale { line: 3 }]
        );
        // The text is compared whole: a copy without its last line break is
        // not the generated text, though every line of it is.
        let cut = EXPECTED.trim_end();
        assert_eq!(
            check(&Memory::default().with(DESCRIPTION, cut), &stand_ins()),
            [Finding::Stale { line: 24 }]
        );
    }

    /// The committed description is the one the server's registry
    /// generates, as this crate links it: the library as it ships, without
    /// its test configuration.
    ///
    /// Verifies: SEC-API-091
    #[test]
    fn the_committed_description_is_the_one_the_registry_generates() {
        assert_eq!(run(&["openapi", "check"]), (Ok(()), String::new()));
        let (result, written) = run(&["openapi"]);
        assert_eq!(result, Ok(()));
        assert_eq!(Disk::new(ROOT).read(DESCRIPTION), Some(written));
    }
}
