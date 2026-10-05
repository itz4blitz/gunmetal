//! What a handler receives and what it answers with.
//!
//! A [`Call`] reaches a handler only after every pipeline check has passed,
//! and holds the request already decoded into the types its route declared
//! ([`crate::request`]): a handler never sees the raw query or body, so it
//! cannot skip the decoding or decode into a looser type.
//!
//! A [`Reply`] is JSON built from a [`ResponseBody`] or nothing at all; a
//! handler cannot set a header, so the response layer's headers are the
//! only ones a response carries.

use std::sync::Arc;

use axum::http::Extensions;
use gunmetal_core::problem::ProblemCode;
use serde::Serialize;

use crate::headers::BodyKind;
use crate::problem::ApiError;
use crate::request::{NoBody, NoQuery};

/// One request, as a handler sees it: `Q` is the route's query type and `B`
/// its body type.
#[derive(Debug)]
pub struct Call<Q = NoQuery, B = NoBody> {
    pub(crate) params: Vec<(&'static str, String)>,
    /// The query, decoded.
    pub query: Q,
    /// The body, decoded.
    pub body: B,
    pub(crate) grant: Arc<Extensions>,
}

impl<Q, B> Call<Q, B> {
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

    /// What the access hook granted: the principal and anything else the
    /// session and authorisation layers attach.
    #[must_use]
    pub fn grant(&self) -> &Extensions {
        &self.grant
    }
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
    use serde::Serializer;

    fn call(grant: Extensions) -> Call {
        Call {
            params: vec![("id", "pls_1".to_owned())],
            query: NoQuery {},
            body: NoBody {},
            grant: Arc::new(grant),
        }
    }

    #[test]
    fn reads_path_parameters() {
        let call = call(Extensions::new());
        assert_eq!(call.param("id"), Ok("pls_1"));
        assert_eq!(
            call.param("other"),
            Err(ApiError::new(ProblemCode::NotFound))
        );
    }

    #[test]
    fn exposes_the_grant() {
        let mut grant = Extensions::new();
        grant.insert(7_u32);
        let call = call(grant);
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
