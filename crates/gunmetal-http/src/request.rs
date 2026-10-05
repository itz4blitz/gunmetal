//! The typed request: the pipeline step that decodes the query and the body
//! before any handler runs (SEC-API-067, SEC-IAM-072).
//!
//! A route names the type of its query and the type of its body when it is
//! registered ([`crate::table::RouteEntry::new`]), and both types name their
//! fields ([`Fields`]). [`typed`] refuses a query parameter or a top-level
//! body key that the type does not name, then decodes both, so a handler
//! cannot be reached with input its route did not declare: at the top
//! level it does not depend on the handler asking, or on the type refusing
//! unknown fields itself. Below the top level it does: a nested struct in
//! a body type must carry `#[serde(deny_unknown_fields)]`, or serde drops
//! an extra nested key. A route without a query takes [`NoQuery`] and a
//! route without a body takes [`NoBody`]; neither names a field, so each
//! refuses everything.
//!
//! Every failure is the one `invalid_request` problem, which echoes nothing
//! that was sent (SEC-API-072).

use std::sync::Arc;

use axum::body::Bytes;
use axum::http::Extensions;
use gunmetal_core::problem::ProblemCode;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::call::Call;
use crate::problem::ApiError;

/// A request type: the query or the body of one action. It names every
/// field the action takes; the pipeline refuses any other name before it
/// decodes. Keep `FIELDS` equal to the type's own fields: a name listed
/// here that the type does not have would be accepted and ignored.
/// `FIELDS` covers the top level only; every struct nested inside a body
/// type must carry `#[serde(deny_unknown_fields)]` (SEC-API-067).
///
/// A name in `FIELDS` that names a principal, such as `owner` or
/// `user_id`, makes [`crate::table::Table::new`] refuse the route, unless
/// it is an admin route that acts on other principals: the acting
/// principal comes only from the credential (SEC-API-013).
pub trait Fields: DeserializeOwned + 'static {
    /// Every query parameter, or every top-level body key, the action takes.
    const FIELDS: &'static [&'static str];
}

/// The query of a route that takes no query parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoQuery {}

impl Fields for NoQuery {
    const FIELDS: &'static [&'static str] = &[];
}

/// The body of a route that takes no body, or an empty object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoBody {}

impl Fields for NoBody {
    const FIELDS: &'static [&'static str] = &[];
}

/// One request after every check before decoding has passed.
#[derive(Debug)]
pub(crate) struct Raw {
    /// The path parameters the route's template bound.
    pub(crate) params: Vec<(&'static str, String)>,
    /// The query parameters, each name once.
    pub(crate) query: Vec<(String, String)>,
    /// The body's bytes, or `None` on a route that takes no body.
    pub(crate) body: Option<Bytes>,
    /// The keys of the body's top-level object.
    pub(crate) keys: Vec<String>,
    /// What the access hook granted.
    pub(crate) grant: Arc<Extensions>,
}

/// Decodes a request into the types its route declared.
///
/// # Errors
///
/// `invalid_request` when the query has a parameter `Q` does not name, the
/// body is not one JSON object, the body has a top-level key `B` does not
/// name, or either does not fit its type.
pub(crate) fn typed<Q: Fields, B: Fields>(raw: Raw) -> Result<Call<Q, B>, ApiError> {
    let body = raw.body.as_deref().unwrap_or(b"{}");
    let query = declared(raw.query, Q::FIELDS, &raw.keys, B::FIELDS, body)?;
    match (serde_json::from_value(query), serde_json::from_slice(body)) {
        (Ok(query), Ok(body)) => Ok(Call {
            params: raw.params,
            query,
            body,
            grant: raw.grant,
        }),
        _ => Err(ApiError::new(ProblemCode::InvalidRequest)),
    }
}

/// Checks that the query names only `query_fields`, that the body is one
/// JSON object and that its top-level keys, `keys`, are all in
/// `body_fields`. Returns the query as the object its type decodes from.
fn declared(
    query: Vec<(String, String)>,
    query_fields: &[&str],
    keys: &[String],
    body_fields: &[&str],
    body: &[u8],
) -> Result<Value, ApiError> {
    let named = query
        .iter()
        .all(|(name, _)| query_fields.contains(&name.as_str()))
        && keys.iter().all(|key| body_fields.contains(&key.as_str()))
        && body.trim_ascii_start().starts_with(b"{");
    if !named {
        return Err(ApiError::new(ProblemCode::InvalidRequest));
    }
    let query: Map<String, Value> = query
        .into_iter()
        .map(|(name, value)| (name, Value::String(value)))
        .collect();
    Ok(Value::Object(query))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paging::{Cursor, PageLimit, PageQuery};

    /// A body type that does not refuse unknown fields itself.
    #[derive(Debug, PartialEq, Eq, Deserialize)]
    struct Rename {
        name: String,
        public: Option<bool>,
    }

    impl Fields for Rename {
        const FIELDS: &'static [&'static str] = &["name", "public"];
    }

    /// A query type that does not refuse unknown fields itself.
    #[derive(Debug, PartialEq, Eq, Deserialize)]
    struct Filter {
        genre: Option<String>,
    }

    impl Fields for Filter {
        const FIELDS: &'static [&'static str] = &["genre"];
    }

    /// The request the pipeline would hand over: `keys` is what
    /// `decode::check` reports for the body, written out by each test.
    fn raw(query: &[(&str, &str)], body: Option<&str>, keys: &[&str]) -> Raw {
        Raw {
            params: vec![("id", "pls_1".to_owned())],
            query: query
                .iter()
                .map(|(n, v)| ((*n).to_owned(), (*v).to_owned()))
                .collect(),
            body: body.map(|text| Bytes::from(text.to_owned())),
            keys: keys.iter().map(|key| (*key).to_owned()).collect(),
            grant: Arc::new(Extensions::new()),
        }
    }

    /// A request's path parameters.
    type Params = Vec<(&'static str, String)>;

    /// What a request decodes to: its path parameters, query and body.
    fn parts<Q: Fields, B: Fields>(raw: Raw) -> Result<(Params, Q, B), ApiError> {
        typed::<Q, B>(raw).map(|call| (call.params, call.query, call.body))
    }

    fn invalid<T>() -> Result<T, ApiError> {
        Err(ApiError::new(ProblemCode::InvalidRequest))
    }

    #[test]
    fn decodes_the_query_and_the_body_into_their_types() {
        assert_eq!(
            parts::<Filter, Rename>(raw(
                &[("genre", "jazz")],
                Some(r#"{"name":"Mix","public":false}"#),
                &["name", "public"]
            )),
            Ok((
                vec![("id", "pls_1".to_owned())],
                Filter {
                    genre: Some("jazz".to_owned())
                },
                Rename {
                    name: "Mix".to_owned(),
                    public: Some(false)
                }
            ))
        );
        assert_eq!(
            parts::<Filter, Rename>(raw(&[], Some(" \n{\"name\":\"Mix\"}"), &["name"])),
            Ok((
                vec![("id", "pls_1".to_owned())],
                Filter { genre: None },
                Rename {
                    name: "Mix".to_owned(),
                    public: None
                }
            ))
        );
        assert_eq!(
            parts::<NoQuery, NoBody>(raw(&[], None, &[])),
            Ok((vec![("id", "pls_1".to_owned())], NoQuery {}, NoBody {}))
        );
        assert_eq!(
            parts::<NoQuery, NoBody>(raw(&[], Some("{}"), &[])),
            Ok((vec![("id", "pls_1".to_owned())], NoQuery {}, NoBody {}))
        );
    }

    #[test]
    fn hands_the_grant_to_the_call() {
        let mut grant = Extensions::new();
        grant.insert(7_u32);
        let mut raw = raw(&[], None, &[]);
        raw.grant = Arc::new(grant);
        assert_eq!(
            typed::<NoQuery, NoBody>(raw).map(|call| call.grant().get::<u32>().copied()),
            Ok(Some(7))
        );
    }

    /// Verifies: SEC-API-067, SEC-IAM-072
    #[test]
    fn refuses_names_the_types_do_not_declare() {
        // Neither type refuses unknown fields itself: serde would ignore
        // `role` and `callback`.
        assert_eq!(
            serde_json::from_str::<Rename>(r#"{"name":"Mix","role":"admin"}"#).ok(),
            Some(Rename {
                name: "Mix".to_owned(),
                public: None
            })
        );
        assert_eq!(
            serde_json::from_str::<Filter>(r#"{"callback":"f"}"#).ok(),
            Some(Filter { genre: None })
        );
        assert_eq!(
            [
                parts::<Filter, Rename>(raw(
                    &[],
                    Some(r#"{"name":"Mix","role":"admin"}"#),
                    &["name", "role"]
                )),
                parts::<Filter, Rename>(raw(
                    &[("callback", "f")],
                    Some(r#"{"name":"Mix"}"#),
                    &["name"]
                )),
                parts::<Filter, Rename>(raw(
                    &[("genre", "jazz"), ("Genre", "jazz")],
                    Some(r#"{"name":"Mix"}"#),
                    &["name"]
                )),
            ],
            [invalid(), invalid(), invalid()]
        );
        assert_eq!(
            [
                parts::<NoQuery, NoBody>(raw(&[("callback", "f")], None, &[])),
                parts::<NoQuery, NoBody>(raw(&[], Some(r#"{"name":"Mix"}"#), &["name"])),
            ],
            [invalid(), invalid()]
        );
    }

    /// Verifies: SEC-API-067
    #[test]
    fn refuses_what_does_not_fit_the_types() {
        assert_eq!(
            [
                // A field of the wrong type, and a required field left out.
                parts::<Filter, Rename>(raw(&[], Some(r#"{"name":5}"#), &["name"])),
                parts::<Filter, Rename>(raw(&[], Some(r#"{"public":true}"#), &["public"])),
                parts::<Filter, Rename>(raw(&[], None, &[])),
                // A body that is not an object, though serde would read an
                // array as the fields in order.
                parts::<Filter, Rename>(raw(&[], Some(r#"["Mix"]"#), &[])),
                parts::<Filter, Rename>(raw(&[], Some(r#""Mix""#), &[])),
            ],
            [invalid(), invalid(), invalid(), invalid(), invalid()]
        );
        assert_eq!(
            serde_json::from_str::<Rename>(r#"["Mix",null]"#).ok(),
            Some(Rename {
                name: "Mix".to_owned(),
                public: None
            })
        );
        assert_eq!(
            parts::<NoQuery, NoBody>(raw(&[], Some("[]"), &[])),
            invalid()
        );
        // A query parameter of the right name and the wrong value.
        assert_eq!(
            parts::<PageQuery, NoBody>(raw(&[("limit", "0")], None, &[])),
            invalid()
        );
        assert_eq!(
            parts::<PageQuery, NoBody>(raw(&[("limit", "7"), ("cursor", "n1")], None, &[])),
            Ok((
                vec![("id", "pls_1".to_owned())],
                PageQuery {
                    limit: PageLimit::new(7),
                    cursor: Cursor::new("n1")
                },
                NoBody {}
            ))
        );
    }
}
