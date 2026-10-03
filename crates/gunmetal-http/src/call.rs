//! What a handler receives and what it answers with.
//!
//! A [`Call`] reaches a handler only after every pipeline check has passed,
//! and holds the request already split into its typed parts. Its decoders
//! map every failure to the one `invalid_request` problem, without echoing
//! what was sent (SEC-API-067, SEC-API-072). Request types derive
//! `Deserialize` with `#[serde(deny_unknown_fields)]`, so an unknown field
//! or a repeated key is refused (SEC-IAM-072).
//!
//! A [`Reply`] is JSON built from a [`ResponseBody`] or nothing at all; a
//! handler cannot set a header, so the response layer's headers are the
//! only ones a response carries.

use std::sync::Arc;

use axum::body::Bytes;
use axum::http::Extensions;
use gunmetal_core::problem::ProblemCode;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::headers::BodyKind;
use crate::problem::ApiError;

/// One request, as a handler sees it.
#[derive(Debug)]
pub struct Call {
    pub(crate) params: Vec<(&'static str, String)>,
    pub(crate) query: Vec<(String, String)>,
    pub(crate) body: Bytes,
    pub(crate) grant: Arc<Extensions>,
}

impl Call {
    /// The value of a path parameter, such as `id` in `/api/v1/tracks/{id}`.
    ///
    /// # Errors
    ///
    /// `not_found` when the route has no parameter of that name.
    pub fn param(&self, name: &str) -> Result<&str, ApiError> {
        self.params
            .iter()
            .find(|(param, _)| *param == name)
            .map(|(_, value)| value.as_str())
            .ok_or(ApiError::new(ProblemCode::NotFound))
    }

    /// Decodes the query string into a typed request. Every value arrives
    /// as a string, so numeric parameters use a bounded type that parses
    /// one, such as [`crate::paging::PageLimit`].
    ///
    /// # Errors
    ///
    /// `invalid_request` when the query does not fit the type.
    pub fn query<T: DeserializeOwned>(&self) -> Result<T, ApiError> {
        let map: Map<String, Value> = self
            .query
            .iter()
            .map(|(name, value)| (name.clone(), Value::String(value.clone())))
            .collect();
        serde_json::from_value(Value::Object(map)).map_err(|_| invalid())
    }

    /// Decodes the body into a typed request.
    ///
    /// # Errors
    ///
    /// `invalid_request` when the body does not fit the type.
    pub fn json<T: DeserializeOwned>(&self) -> Result<T, ApiError> {
        serde_json::from_slice(&self.body).map_err(|_| invalid())
    }

    /// What the access hook granted: the principal and anything else the
    /// session and authorisation layers attach.
    #[must_use]
    pub fn grant(&self) -> &Extensions {
        &self.grant
    }
}

/// The `invalid_request` problem.
fn invalid() -> ApiError {
    ApiError::new(ProblemCode::InvalidRequest)
}

/// A type written for one response, and so allowed to be sent as one
/// (SEC-API-068). A storage record does not implement it, so it cannot be
/// serialised into a response by mistake.
/// Verifies: SEC-API-068
///
/// ```compile_fail,E0277
/// use gunmetal_http::call::Reply;
/// #[derive(serde::Serialize)]
/// struct AccountRow {
///     id: u64,
///     library_root: String,
/// }
/// let row = AccountRow { id: 1, library_root: "/srv/music".to_owned() };
/// let _ = Reply::json(&row);
/// ```
///
/// A type that says it is a response body compiles:
///
/// ```
/// use gunmetal_http::call::{Reply, ResponseBody};
/// #[derive(serde::Serialize)]
/// struct AccountSummary {
///     id: u64,
/// }
/// impl ResponseBody for AccountSummary {}
/// assert!(Reply::json(&AccountSummary { id: 1 }).is_ok());
/// ```
pub trait ResponseBody: Serialize {}

/// A handler's answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    pub(crate) kind: BodyKind,
    pub(crate) body: Vec<u8>,
}

impl Reply {
    /// A JSON response, 200, from a response type.
    ///
    /// # Errors
    ///
    /// `internal_error` when the value cannot be serialised.
    pub fn json<T: ResponseBody>(value: &T) -> Result<Self, ApiError> {
        serde_json::to_vec(value)
            .map(|body| Self {
                kind: BodyKind::Json,
                body,
            })
            .map_err(|_| ApiError::new(ProblemCode::InternalError))
    }

    /// An empty response, 204.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            kind: BodyKind::Empty,
            body: Vec::new(),
        }
    }

    /// The response's status: 204 without a body, 200 with one.
    pub(crate) fn status(&self) -> u16 {
        if self.kind == BodyKind::Empty {
            204
        } else {
            200
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serializer};

    #[derive(Debug, PartialEq, Eq, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Rename {
        name: String,
        public: bool,
    }

    #[derive(Debug, PartialEq, Eq, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Filter {
        genre: String,
        sort: Option<String>,
    }

    fn call(query: &[(&str, &str)], body: &str) -> Call {
        granted(query, body, Extensions::new())
    }

    fn granted(query: &[(&str, &str)], body: &str, grant: Extensions) -> Call {
        Call {
            params: vec![("id", "pls_1".to_owned())],
            query: query
                .iter()
                .map(|(n, v)| ((*n).to_owned(), (*v).to_owned()))
                .collect(),
            body: Bytes::from(body.to_owned()),
            grant: Arc::new(grant),
        }
    }

    #[test]
    fn reads_path_parameters() {
        let call = call(&[], "");
        assert_eq!(call.param("id"), Ok("pls_1"));
        assert_eq!(
            call.param("other"),
            Err(ApiError::new(ProblemCode::NotFound))
        );
    }

    /// Verifies: SEC-API-067, SEC-IAM-072
    #[test]
    fn decodes_bodies_strictly() {
        let invalid = Err(ApiError::new(ProblemCode::InvalidRequest));
        assert_eq!(
            call(&[], r#"{"name":"Mix","public":false}"#).json::<Rename>(),
            Ok(Rename {
                name: "Mix".to_owned(),
                public: false
            })
        );
        assert_eq!(
            call(&[], r#"{"name":"Mix","public":false,"owner":"x"}"#).json::<Rename>(),
            invalid
        );
        assert_eq!(
            call(&[], r#"{"name":"Mix","name":"Mix","public":false}"#).json::<Rename>(),
            invalid
        );
        assert_eq!(
            call(&[], r#"{"name":"Mix","public":"no"}"#).json::<Rename>(),
            invalid
        );
    }

    /// Verifies: SEC-API-067
    #[test]
    fn decodes_queries_strictly() {
        assert_eq!(
            call(&[("genre", "jazz")], "").query::<Filter>(),
            Ok(Filter {
                genre: "jazz".to_owned(),
                sort: None
            })
        );
        assert_eq!(
            call(&[("genre", "jazz"), ("callback", "f")], "").query::<Filter>(),
            Err(ApiError::new(ProblemCode::InvalidRequest))
        );
    }

    #[test]
    fn exposes_the_grant() {
        let mut grant = Extensions::new();
        grant.insert(7_u32);
        let call = granted(&[], "", grant);
        assert_eq!(call.grant().get::<u32>(), Some(&7));
        assert_eq!(call.grant().get::<u64>(), None);
    }

    struct Unserialisable;

    impl ResponseBody for Unserialisable {}

    impl Serialize for Unserialisable {
        fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("no"))
        }
    }

    #[derive(Serialize)]
    struct Counts {
        a: u32,
    }

    impl ResponseBody for Counts {}

    #[test]
    fn builds_replies() {
        assert_eq!(
            Reply::json(&Counts { a: 1 }),
            Ok(Reply {
                kind: BodyKind::Json,
                body: br#"{"a":1}"#.to_vec()
            })
        );
        assert_eq!(
            Reply::json(&Unserialisable),
            Err(ApiError::new(ProblemCode::InternalError))
        );
        assert_eq!(
            Reply::empty(),
            Reply {
                kind: BodyKind::Empty,
                body: Vec::new()
            }
        );
        assert_eq!(Reply::empty().status(), 204);
        assert_eq!(
            Reply::json(&Counts { a: 1 }).map(|reply| reply.status()),
            Ok(200)
        );
    }
}
