//! The route table: every route the server answers, each with its policy,
//! and the exact matcher built from it.
//!
//! [`Table::new`] refuses a table that is not well formed: a path outside
//! `/api/v1` (SEC-API-092), a malformed template, the same method on the
//! same path twice, two templates that could match the same path, or a
//! `GET` route that changes state (SEC-API-036). Matching is exact: no
//! trailing slash, extension, `;` parameter, other case or encoded
//! character matches a literal segment, and a parameter segment takes only
//! letters, digits, `-` and `_`, so no variant of a path reaches a handler
//! (SEC-API-055).

use core::future::Future;
use core::pin::Pin;
use std::sync::Arc;

use crate::call::{Call, Reply};
use crate::problem::ApiError;
use crate::request::{Fields, Raw, typed};
use crate::route::{AccessClass, Method, RouteSpec};

/// Where every route lives (SEC-API-092).
pub const PREFIX: &str = "/api/v1/";

/// What a handler returns.
pub type HandlerFuture = Pin<Box<dyn Future<Output = Result<Reply, ApiError>> + Send>>;

/// A route's decoder and handler, shared by every request to the route.
type Handler = Arc<dyn Fn(Raw) -> HandlerFuture + Send + Sync>;

/// A route's spec, its request types and its handler. A handler cannot
/// reach the router any other way, and is not called until the request has
/// decoded into those types.
#[derive(Clone)]
pub struct RouteEntry {
    spec: RouteSpec,
    handler: Handler,
}

impl RouteEntry {
    /// Pairs a spec with its handler. The handler's argument names the
    /// route's query type `Q` and body type `B`; a route that takes neither
    /// has a handler over plain [`Call`]. The request is decoded into both
    /// before the handler is called, and a request that names a field
    /// either type does not, or does not fit them, is answered with
    /// `invalid_request` (SEC-API-067, SEC-IAM-072).
    pub fn new<Q, B, F, Fut>(spec: RouteSpec, handler: F) -> Self
    where
        Q: Fields,
        B: Fields,
        F: Fn(Call<Q, B>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Reply, ApiError>> + Send + 'static,
    {
        Self {
            spec,
            handler: Arc::new(move |raw| {
                let answer = typed::<Q, B>(raw).map(&handler);
                Box::pin(async move { answer?.await })
            }),
        }
    }

    /// The route's spec.
    #[must_use]
    pub const fn spec(&self) -> &RouteSpec {
        &self.spec
    }

    /// Decodes the request and, if it fits the route's types, calls the
    /// handler.
    pub(crate) fn call(&self, raw: Raw) -> HandlerFuture {
        (self.handler)(raw)
    }
}

impl core::fmt::Debug for RouteEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("RouteEntry").field(&self.spec).finish()
    }
}

/// Why a table was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableError {
    /// The path is not under `/api/v1/`.
    Unversioned(&'static str),
    /// The path has an empty or malformed segment.
    Template(&'static str),
    /// The same method on the same path appears twice.
    Duplicate(Method, &'static str),
    /// Two different templates could match the same path.
    Overlap(&'static str, &'static str),
    /// A `GET` route is declared as changing state.
    ReadMutates(&'static str),
}

/// One segment of a path template.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Segment {
    Literal(&'static str),
    Param(&'static str),
}

impl Segment {
    fn parse(text: &'static str) -> Option<Self> {
        match text.strip_prefix('{').and_then(|t| t.strip_suffix('}')) {
            Some(name) => {
                word(name, |b| b.is_ascii_lowercase() || b == b'_').then_some(Self::Param(name))
            }
            None => word(text, |b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_'
            })
            .then_some(Self::Literal(text)),
        }
    }

    /// Whether this segment and another could match the same text.
    fn meets(self, other: Self) -> bool {
        match (self, other) {
            (Self::Literal(a), Self::Literal(b)) => a == b,
            _ => true,
        }
    }
}

/// Whether text is non-empty and every byte passes `allowed`.
fn word(text: &str, allowed: impl Fn(u8) -> bool) -> bool {
    !text.is_empty() && text.bytes().all(allowed)
}

/// The checked route table.
#[derive(Debug, Clone)]
pub struct Table {
    routes: Vec<(Vec<Segment>, RouteEntry)>,
}

/// The routes on the path a request named, and its parameters.
pub(crate) struct Found<'t> {
    pub(crate) path: &'static str,
    pub(crate) entries: Vec<&'t RouteEntry>,
    pub(crate) params: Vec<(&'static str, String)>,
}

impl Table {
    /// Checks a table.
    ///
    /// # Errors
    ///
    /// The first problem found, in the order of the entries.
    pub fn new(entries: Vec<RouteEntry>) -> Result<Self, TableError> {
        let mut routes: Vec<(Vec<Segment>, RouteEntry)> = Vec::with_capacity(entries.len());
        for entry in entries {
            let spec = entry.spec;
            let rest = spec
                .path
                .strip_prefix(PREFIX)
                .ok_or(TableError::Unversioned(spec.path))?;
            let segments = rest
                .split('/')
                .map(Segment::parse)
                .collect::<Option<Vec<_>>>()
                .ok_or(TableError::Template(spec.path))?;
            if spec.method == Method::Get && spec.mutates() {
                return Err(TableError::ReadMutates(spec.path));
            }
            for (other, earlier) in &routes {
                let meets = other.len() == segments.len()
                    && other.iter().zip(&segments).all(|(a, b)| a.meets(*b));
                if earlier.spec.path == spec.path {
                    if earlier.spec.method == spec.method {
                        return Err(TableError::Duplicate(spec.method, spec.path));
                    }
                } else if meets {
                    return Err(TableError::Overlap(earlier.spec.path, spec.path));
                }
            }
            routes.push((segments, entry));
        }
        Ok(Self { routes })
    }

    /// Every route's spec, in table order.
    pub fn specs(&self) -> impl Iterator<Item = &RouteSpec> {
        self.routes.iter().map(|(_, entry)| entry.spec())
    }

    /// Every route a caller can reach without a session: the public,
    /// credential-exchange and capability routes, in table order. This is
    /// the list the reviewed allow-list is compared with (SEC-HIS-005,
    /// SEC-IAM-067).
    #[must_use]
    pub fn anonymous(&self) -> Vec<(Method, &'static str)> {
        self.specs()
            .filter(|spec| !matches!(spec.access.class(), AccessClass::User | AccessClass::Admin))
            .map(|spec| (spec.method, spec.path))
            .collect()
    }

    /// The routes whose template matches a request path exactly.
    pub(crate) fn lookup(&self, path: &str) -> Option<Found<'_>> {
        let parts: Vec<&str> = path.strip_prefix(PREFIX)?.split('/').collect();
        let mut found: Option<Found<'_>> = None;
        for (segments, entry) in &self.routes {
            if let Some(params) = bind(segments, &parts) {
                found
                    .get_or_insert_with(|| Found {
                        path: entry.spec.path,
                        entries: Vec::new(),
                        params,
                    })
                    .entries
                    .push(entry);
            }
        }
        found
    }
}

/// The parameters a template binds from a path, if it matches.
fn bind(segments: &[Segment], parts: &[&str]) -> Option<Vec<(&'static str, String)>> {
    if segments.len() != parts.len() {
        return None;
    }
    let mut params = Vec::new();
    for (segment, part) in segments.iter().zip(parts) {
        match *segment {
            Segment::Literal(literal) if literal == *part => {}
            Segment::Param(name)
                if word(part, |b| {
                    b.is_ascii_alphanumeric() || b == b'-' || b == b'_'
                }) =>
            {
                params.push((name, (*part).to_owned()));
            }
            _ => return None,
        }
    }
    Some(params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::block_on;
    use crate::route::{Access, BodyRule, Effect, RateClass, RouteTag};
    use axum::http::Extensions;
    use gunmetal_core::problem::ProblemCode;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const fn spec(method: Method, path: &'static str, effect: Effect) -> RouteSpec {
        RouteSpec {
            method,
            path,
            access: Access::Public { effect },
            tag: RouteTag::None,
            body: BodyRule::None,
            ids: &[],
            rate: RateClass::Read,
        }
    }

    /// The handler every route in these tests has: it answers with nothing
    /// when it was called with an `id` parameter, and `not_found` otherwise.
    async fn handler(call: Call) -> Result<Reply, ApiError> {
        call.param("id").map(|_| Reply::empty())
    }

    fn entry(method: Method, path: &'static str) -> RouteEntry {
        let effect = if method == Method::Get {
            Effect::Reads
        } else {
            Effect::Mutates
        };
        RouteEntry::new(spec(method, path, effect), handler)
    }

    fn table(routes: &[(Method, &'static str)]) -> Result<Table, TableError> {
        Table::new(routes.iter().map(|(m, p)| entry(*m, p)).collect())
    }

    type Matched = (
        &'static str,
        Vec<(Method, &'static str)>,
        Vec<(&'static str, String)>,
    );

    fn paths(found: Option<Found<'_>>) -> Option<Matched> {
        found.map(|found| {
            (
                found.path,
                found
                    .entries
                    .iter()
                    .map(|e| (e.spec().method, e.spec().path))
                    .collect(),
                found.params,
            )
        })
    }

    #[test]
    fn accepts_a_well_formed_table() {
        let routes = [
            (Method::Get, "/api/v1/server"),
            (Method::Get, "/api/v1/playlists/{id}"),
            (Method::Delete, "/api/v1/playlists/{id}"),
            (Method::Get, "/api/v1/playlists/{id}/tracks"),
            (Method::Post, "/api/v1/me/sign-out_2"),
        ];
        let table = table(&routes).unwrap();
        let listed: Vec<(Method, &str)> = table.specs().map(|s| (s.method, s.path)).collect();
        assert_eq!(listed, routes);
    }

    /// Verifies: SEC-API-092
    #[test]
    fn refuses_paths_outside_the_versioned_prefix() {
        for path in ["/api/v2/server", "/server", "/api/v1", "api/v1/server"] {
            assert_eq!(
                table(&[(Method::Get, path)]).map(|_| ()),
                Err(TableError::Unversioned(path))
            );
        }
    }

    #[test]
    fn refuses_malformed_templates() {
        for path in [
            "/api/v1/",
            "/api/v1/server/",
            "/api/v1//server",
            "/api/v1/Server",
            "/api/v1/server.json",
            "/api/v1/{}",
            "/api/v1/{Id}",
            "/api/v1/{id",
            "/api/v1/a;b",
        ] {
            assert_eq!(
                table(&[(Method::Get, path)]).map(|_| ()),
                Err(TableError::Template(path))
            );
        }
    }

    #[test]
    fn refuses_duplicates_and_overlaps() {
        assert_eq!(
            table(&[(Method::Get, "/api/v1/a"), (Method::Get, "/api/v1/a")]).map(|_| ()),
            Err(TableError::Duplicate(Method::Get, "/api/v1/a"))
        );
        assert_eq!(
            table(&[
                (Method::Get, "/api/v1/a/{id}"),
                (Method::Get, "/api/v1/a/recent")
            ])
            .map(|_| ()),
            Err(TableError::Overlap("/api/v1/a/{id}", "/api/v1/a/recent"))
        );
        assert_eq!(
            table(&[
                (Method::Get, "/api/v1/a/{id}"),
                (Method::Post, "/api/v1/a/{key}")
            ])
            .map(|_| ()),
            Err(TableError::Overlap("/api/v1/a/{id}", "/api/v1/a/{key}"))
        );
        assert!(
            table(&[
                (Method::Get, "/api/v1/a/{id}"),
                (Method::Get, "/api/v1/a/{id}/b"),
                (Method::Get, "/api/v1/b/{id}"),
                (Method::Get, "/api/v1/a"),
            ])
            .is_ok()
        );
    }

    /// Verifies: SEC-API-036
    #[test]
    fn refuses_a_get_route_that_mutates() {
        let entries = vec![RouteEntry::new(
            spec(Method::Get, "/api/v1/a", Effect::Mutates),
            handler,
        )];
        assert_eq!(
            Table::new(entries).map(|_| ()),
            Err(TableError::ReadMutates("/api/v1/a"))
        );
        let entries = vec![RouteEntry::new(
            spec(Method::Post, "/api/v1/a", Effect::Reads),
            handler,
        )];
        assert!(Table::new(entries).is_ok());
    }

    /// Verifies: SEC-API-055
    #[test]
    fn matches_paths_exactly() {
        let table = table(&[
            (Method::Get, "/api/v1/server"),
            (Method::Get, "/api/v1/playlists/{id}"),
            (Method::Delete, "/api/v1/playlists/{id}"),
        ])
        .unwrap();
        assert_eq!(
            paths(table.lookup("/api/v1/server")),
            Some((
                "/api/v1/server",
                vec![(Method::Get, "/api/v1/server")],
                vec![]
            ))
        );
        assert_eq!(
            paths(table.lookup("/api/v1/playlists/pls_AB-9")),
            Some((
                "/api/v1/playlists/{id}",
                vec![
                    (Method::Get, "/api/v1/playlists/{id}"),
                    (Method::Delete, "/api/v1/playlists/{id}")
                ],
                vec![("id", "pls_AB-9".to_owned())]
            ))
        );
        for variant in [
            "/api/v1/server/",
            "/api/v1/server.json",
            "/api/v1/server;x=1",
            "/api/v1/Server",
            "/api/v1/%73erver",
            "/api/v1/server/extra",
            "/API/v1/server",
            "/api/v1//server",
            "/api/v1/playlists/",
            "/api/v1/playlists/a.b",
            "/api/v1/playlists/a%2Fb",
            "/api/v1/playlists/a;b",
            "/api/v1/playlists",
            "/api/v2/server",
            "/server",
        ] {
            assert_eq!(paths(table.lookup(variant)), None);
        }
    }

    fn raw(params: Vec<(&'static str, String)>, query: &[(&str, &str)]) -> Raw {
        Raw {
            params,
            query: query
                .iter()
                .map(|(n, v)| ((*n).to_owned(), (*v).to_owned()))
                .collect(),
            body: None,
            keys: Vec::new(),
            grant: Arc::new(Extensions::new()),
        }
    }

    #[test]
    fn an_entry_calls_its_handler_with_the_call() {
        let entry = entry(Method::Get, "/api/v1/a/{id}");
        assert_eq!(
            block_on(entry.call(raw(vec![("id", "a_1".to_owned())], &[]))),
            Ok(Reply::empty())
        );
        assert_eq!(
            block_on(entry.call(raw(Vec::new(), &[]))),
            Err(ApiError::new(ProblemCode::NotFound))
        );
    }

    /// Verifies: SEC-API-067
    #[test]
    fn an_entry_decodes_before_it_calls_its_handler() {
        let calls = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&calls);
        let entry = RouteEntry::new(
            spec(Method::Get, "/api/v1/a/{id}", Effect::Reads),
            move |_: Call| {
                counted.fetch_add(1, Ordering::SeqCst);
                async { Ok(Reply::empty()) }
            },
        );
        let id = || vec![("id", "a_1".to_owned())];
        assert_eq!(
            block_on(entry.call(raw(id(), &[("callback", "f")]))),
            Err(ApiError::new(ProblemCode::InvalidRequest))
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(block_on(entry.call(raw(id(), &[]))), Ok(Reply::empty()));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn debug_shows_only_the_spec() {
        let entry = entry(Method::Get, "/api/v1/a");
        assert_eq!(
            format!("{entry:?}"),
            format!("RouteEntry({:?})", entry.spec())
        );
    }
}
