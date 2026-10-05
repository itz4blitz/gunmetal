//! The principal extractor as the request pipeline's access hook
//! (web-and-api-security.md, "Request pipeline", step 7).
//!
//! The hook answers one question for every request that matched a route:
//! which principal, if any, is asking. A route anyone may call, a
//! credential exchange and a capability route resolve no session, so a
//! stale cookie can neither help nor hurt there. Every other route needs
//! the session cookie, and gets either the one principal the cookie
//! resolves to or the one `unauthenticated` answer (SEC-IAM-002,
//! SEC-API-003). The `Authorization` header names no credential the native
//! listener takes in R1, so a request that carries one is refused
//! (SEC-EXT-007).
//!
//! The grant the hook returns is what the listener attached to the request
//! with the [`Principal`](gunmetal_core::authz::Principal) added.

use std::sync::Arc;

use gunmetal_core::problem::ProblemCode;
use gunmetal_http::pipeline::{AccessHook, Attempt};
use gunmetal_http::problem::ApiError;
use gunmetal_http::route::RouteSpec;

use super::kind::Listener;
use super::sessions::Sessions;

/// What a route needs of a session beyond its being live.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Need {
    /// The separate administrator session: every admin route, and every
    /// route tagged elevated (SEC-IAM-041).
    pub admin: bool,
    /// A user verification no older than five minutes: every route tagged
    /// fresh-uv (SEC-TM-017).
    pub fresh: bool,
}

impl Need {
    /// What `spec` needs of a session, or `None` for a route that takes no
    /// session at all: one anyone may call, a credential exchange or a
    /// capability route.
    #[must_use]
    pub const fn of(_spec: &RouteSpec) -> Option<Self> {
        None
    }
}

/// The access hook for `listener`: it resolves each request's session
/// cookie through `sessions`.
#[must_use]
pub fn access_hook(_sessions: Arc<Sessions>, _listener: Listener) -> AccessHook {
    Arc::new(|_attempt: Attempt| Box::pin(async { Err(ApiError::new(ProblemCode::InternalError)) }))
}

#[cfg(test)]
mod tests {
    use gunmetal_core::audit_event::EventName;
    use gunmetal_http::route::{
        Access, AdminEffect, BodyRule, Capability, CapabilityKind, Effect, ExchangeKind, Method,
        RateClass, RouteSpec, RouteTag, Target,
    };

    use super::Need;

    /// Verifies: SEC-IAM-041, SEC-TM-017
    #[test]
    fn a_route_needs_a_session_by_its_class_and_its_tag() {
        let classes = [
            Access::Public {
                effect: Effect::Reads,
            },
            Access::Exchange {
                kind: ExchangeKind::SignIn,
                effect: Effect::Mutates,
            },
            Access::Capability {
                kind: CapabilityKind::Media,
                effect: Effect::Reads,
            },
            Access::User {
                capability: Capability::new("library.read"),
                effect: Effect::Mutates,
            },
            Access::Admin {
                capability: Capability::new("server.settings"),
                effect: AdminEffect::Mutates(EventName::AuthzFail),
                target: Target::Caller,
            },
        ];
        let needs: Vec<Option<(bool, bool)>> = classes
            .iter()
            .flat_map(|access| {
                [RouteTag::None, RouteTag::Elevated, RouteTag::FreshUv].map(|tag| {
                    Need::of(&RouteSpec {
                        method: Method::Post,
                        path: "/api/v1/stand-in",
                        access: *access,
                        tag,
                        body: BodyRule::None,
                        ids: &[],
                        rate: RateClass::Write,
                    })
                    .map(|need| (need.admin, need.fresh))
                })
            })
            .collect();
        // Each class under no tag, the elevated tag and the fresh-uv tag,
        // as (administrator session, fresh verification).
        assert_eq!(
            needs,
            [
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some((false, false)),
                Some((true, false)),
                Some((false, true)),
                Some((true, false)),
                Some((true, false)),
                Some((true, true)),
            ]
        );
    }
}
