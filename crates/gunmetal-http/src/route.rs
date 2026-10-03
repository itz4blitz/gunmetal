//! The route table's types: what every route must declare before it can be
//! served.
//!
//! A [`RouteSpec`] has no constructor and no `Default`: it is written as a
//! struct literal, so leaving out any of its fields is a compile error, and
//! so is an access class without its effect, or a mutating admin route
//! without the audit event it emits (SEC-TM-005, SEC-API-001, SEC-API-019,
//! SEC-IAM-041, SEC-IAM-067, SEC-HIS-005). A handler reaches the router
//! only inside a [`RouteEntry`](crate::table::RouteEntry), which holds its
//! spec, so a route cannot be registered without its policy.

use gunmetal_core::audit_event::EventName;

/// One route: a method on an exact path template, with its policy.
///
/// A route without an access class does not compile.
/// Verifies: SEC-TM-005, SEC-API-001, SEC-HIS-005, SEC-IAM-067
///
/// ```compile_fail,E0063
/// use gunmetal_http::route::*;
/// const SPEC: RouteSpec = RouteSpec {
///     method: Method::Get,
///     path: "/api/v1/server",
///     tag: RouteTag::None,
///     body: BodyRule::None,
///     ids: &[],
///     rate: RateClass::Read,
/// };
/// ```
///
/// A route without a route tag does not compile.
/// Verifies: SEC-IAM-041
///
/// ```compile_fail,E0063
/// use gunmetal_http::route::*;
/// const SPEC: RouteSpec = RouteSpec {
///     method: Method::Get,
///     path: "/api/v1/server",
///     access: Access::Public { effect: Effect::Reads },
///     body: BodyRule::None,
///     ids: &[],
///     rate: RateClass::Read,
/// };
/// ```
///
/// A signed-in user or admin route without the capability it requires
/// does not compile.
/// Verifies: SEC-API-019
///
/// ```compile_fail,E0063
/// use gunmetal_http::route::*;
/// const SPEC: RouteSpec = RouteSpec {
///     method: Method::Get,
///     path: "/api/v1/library",
///     access: Access::User {
///         effect: Effect::Reads,
///     },
///     tag: RouteTag::None,
///     body: BodyRule::None,
///     ids: &[],
///     rate: RateClass::Read,
/// };
/// ```
///
/// ```compile_fail,E0063
/// use gunmetal_http::route::*;
/// const SPEC: RouteSpec = RouteSpec {
///     method: Method::Get,
///     path: "/api/v1/admin/users",
///     access: Access::Admin {
///         effect: AdminEffect::Reads,
///         target: Target::Caller,
///     },
///     tag: RouteTag::None,
///     body: BodyRule::None,
///     ids: &[],
///     rate: RateClass::Read,
/// };
/// ```
///
/// A mutating admin route without the audit event it emits does not
/// compile (the route-table clause of SEC-OPS-020).
///
/// ```compile_fail,E0308
/// use gunmetal_http::route::*;
/// const SPEC: RouteSpec = RouteSpec {
///     method: Method::Post,
///     path: "/api/v1/admin/users",
///     access: Access::Admin {
///         capability: Capability::new("accounts.manage"),
///         effect: AdminEffect::Mutates,
///         target: Target::OtherPrincipals,
///     },
///     tag: RouteTag::FreshUv,
///     body: BodyRule::Json(JsonLimits::DEFAULT),
///     ids: &[],
///     rate: RateClass::Write,
/// };
/// ```
///
/// The same admin route with its event compiles:
///
/// ```
/// use gunmetal_core::audit_event::EventName;
/// use gunmetal_http::route::*;
/// const SPEC: RouteSpec = RouteSpec {
///     method: Method::Post,
///     path: "/api/v1/admin/users",
///     access: Access::Admin {
///         capability: Capability::new("accounts.manage"),
///         effect: AdminEffect::Mutates(EventName::AuthzFail),
///         target: Target::OtherPrincipals,
///     },
///     tag: RouteTag::FreshUv,
///     body: BodyRule::Json(JsonLimits::DEFAULT),
///     ids: &[],
///     rate: RateClass::Write,
/// };
/// assert!(SPEC.mutates());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteSpec {
    /// The one method this spec answers. A path with several methods has
    /// one spec per method.
    pub method: Method,
    /// The path template under `/api/v1`, matched exactly. A segment in
    /// braces, such as `{id}`, is a parameter.
    pub path: &'static str,
    /// Who may call the route, and whether it changes state.
    pub access: Access,
    /// The route's tag from the closed set (SEC-IAM-041).
    pub tag: RouteTag,
    /// The body the route accepts.
    pub body: BodyRule,
    /// Where the route's object identifiers sit, which the generated
    /// cross-principal suites read.
    pub ids: &'static [IdPlace],
    /// The route's rate-limit class.
    pub rate: RateClass,
}

impl RouteSpec {
    /// Whether the route changes state.
    #[must_use]
    pub const fn mutates(&self) -> bool {
        self.access.mutates()
    }
}

/// The methods a route may declare.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    /// `GET`, which never changes state (SEC-API-036).
    Get,
    /// `POST`.
    Post,
    /// `PUT`.
    Put,
    /// `PATCH`.
    Patch,
    /// `DELETE`.
    Delete,
}

impl Method {
    /// The method with this name on the wire, exactly: names are case
    /// sensitive, and a method no route may declare (`HEAD`, `OPTIONS`,
    /// `TRACE`, `CONNECT` or anything else) is `None`.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        [Self::Get, Self::Post, Self::Put, Self::Patch, Self::Delete]
            .into_iter()
            .find(|method| method.as_str() == name)
    }

    /// The method's name on the wire.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
        }
    }
}

/// The access class of a route, with what it needs (SEC-API-001).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// Anyone may call it. It must be on the reviewed allow-list
    /// (SEC-API-002).
    Public {
        /// Whether it changes state.
        effect: Effect,
    },
    /// A credential exchange: setup, sign-in, refresh, invitation or
    /// pairing.
    Exchange {
        /// Which exchange.
        kind: ExchangeKind,
        /// Whether it changes state.
        effect: Effect,
    },
    /// A capability carried by the request itself, such as a signed media
    /// URL.
    Capability {
        /// Which capability.
        kind: CapabilityKind,
        /// Whether it changes state.
        effect: Effect,
    },
    /// A signed-in principal holding a capability.
    User {
        /// The capability the principal must hold.
        capability: Capability,
        /// Whether it changes state.
        effect: Effect,
    },
    /// A signed-in principal holding an admin capability.
    Admin {
        /// The admin capability the principal must hold.
        capability: Capability,
        /// Whether it changes state, and the audit event it emits if so.
        effect: AdminEffect,
        /// Whether the route acts on principals other than the caller.
        target: Target,
    },
}

impl Access {
    /// Whether a route with this access changes state.
    #[must_use]
    pub const fn mutates(self) -> bool {
        match self {
            Self::Public { effect }
            | Self::Exchange { effect, .. }
            | Self::Capability { effect, .. }
            | Self::User { effect, .. } => matches!(effect, Effect::Mutates),
            Self::Admin { effect, .. } => matches!(effect, AdminEffect::Mutates(_)),
        }
    }

    /// Whether a request field may name another user, profile or owner:
    /// only on an admin route that acts on other principals (SEC-API-013).
    #[must_use]
    pub const fn names_principals(self) -> bool {
        matches!(
            self,
            Self::Admin {
                target: Target::OtherPrincipals,
                ..
            }
        )
    }

    /// The access class, without its data.
    #[must_use]
    pub const fn class(self) -> AccessClass {
        match self {
            Self::Public { .. } => AccessClass::Public,
            Self::Exchange { .. } => AccessClass::Exchange,
            Self::Capability { .. } => AccessClass::Capability,
            Self::User { .. } => AccessClass::User,
            Self::Admin { .. } => AccessClass::Admin,
        }
    }
}

/// The five access classes, without their data: what the allow-list and
/// the role matrix are keyed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessClass {
    /// [`Access::Public`].
    Public,
    /// [`Access::Exchange`].
    Exchange,
    /// [`Access::Capability`].
    Capability,
    /// [`Access::User`].
    User,
    /// [`Access::Admin`].
    Admin,
}

/// Whether a route changes state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// It only reads.
    Reads,
    /// It changes state.
    Mutates,
}

/// Whether an admin route changes state. A mutating admin route names the
/// security event it emits, so it cannot be declared without one
/// (SEC-OPS-020).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminEffect {
    /// It only reads.
    Reads,
    /// It changes state and emits this event.
    Mutates(EventName),
}

/// Whom an admin route acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// Only the caller's own account and server settings.
    Caller,
    /// Other principals, which its requests may name (SEC-API-013).
    OtherPrincipals,
}

/// The credential exchanges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExchangeKind {
    /// First-run setup with the claim code.
    Setup,
    /// A sign-in ceremony.
    SignIn,
    /// Token refresh.
    Refresh,
    /// Invitation redemption.
    Invitation,
    /// Device pairing.
    Pairing,
}

/// The capabilities a request can carry itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapabilityKind {
    /// A signed media or image URL (SEC-API-026).
    Media,
    /// A single-use event-channel ticket (SEC-API-042).
    EventTicket,
}

/// A capability a signed-in principal must hold, by its stable name. The
/// authorisation layer (WP-065) maps each name to an action of the policy
/// (WP-033) and refuses a name it does not know.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Capability(&'static str);

impl Capability {
    /// Names a capability.
    #[must_use]
    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }

    /// The capability's stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.0
    }
}

/// The closed set of route tags (SEC-IAM-041).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RouteTag {
    /// No extra condition.
    None,
    /// Needs the separate admin session.
    Elevated,
    /// Needs a user-verified assertion no older than five minutes.
    FreshUv,
}

/// The body a route accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyRule {
    /// No body: a request with a body or a content type is refused.
    None,
    /// A JSON body within these limits, as `application/json` only
    /// (SEC-API-035).
    Json(JsonLimits),
}

/// The limits on a JSON body (SEC-API-060).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonLimits {
    /// The most bytes the body may hold.
    pub bytes: usize,
    /// The most elements any one array in it may hold.
    pub items: usize,
}

impl JsonLimits {
    /// The limits a route has unless it declares others: 64 KiB and 1,000
    /// elements per array.
    pub const DEFAULT: Self = Self {
        bytes: 64 * 1024,
        items: 1000,
    };

    /// The deepest nesting any JSON body may have.
    pub const DEPTH: usize = 32;
}

/// Where a route's object identifiers sit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdPlace {
    /// In a path parameter.
    Path,
    /// In a query parameter.
    Query,
    /// In the body.
    Body,
}

/// The rate-limit classes the rate-limit hook (WP-130) keys its limits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RateClass {
    /// Cheap reads.
    Read,
    /// Writes.
    Write,
    /// Credential exchanges and anything that checks a guessable secret.
    Exchange,
    /// Expensive work, such as search or exports.
    Expensive,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One access of every class and effect, with what each must answer:
    /// its class, whether it mutates and whether it may name principals.
    const CASES: [(Access, AccessClass, bool, bool); 10] = [
        (
            Access::Public {
                effect: Effect::Reads,
            },
            AccessClass::Public,
            false,
            false,
        ),
        (
            Access::Public {
                effect: Effect::Mutates,
            },
            AccessClass::Public,
            true,
            false,
        ),
        (
            Access::Exchange {
                kind: ExchangeKind::SignIn,
                effect: Effect::Mutates,
            },
            AccessClass::Exchange,
            true,
            false,
        ),
        (
            Access::Exchange {
                kind: ExchangeKind::Setup,
                effect: Effect::Reads,
            },
            AccessClass::Exchange,
            false,
            false,
        ),
        (
            Access::Capability {
                kind: CapabilityKind::Media,
                effect: Effect::Reads,
            },
            AccessClass::Capability,
            false,
            false,
        ),
        (
            Access::Capability {
                kind: CapabilityKind::EventTicket,
                effect: Effect::Mutates,
            },
            AccessClass::Capability,
            true,
            false,
        ),
        (
            Access::User {
                capability: Capability::new("library.read"),
                effect: Effect::Reads,
            },
            AccessClass::User,
            false,
            false,
        ),
        (
            Access::User {
                capability: Capability::new("playlists.edit"),
                effect: Effect::Mutates,
            },
            AccessClass::User,
            true,
            false,
        ),
        (
            Access::Admin {
                capability: Capability::new("accounts.manage"),
                effect: AdminEffect::Reads,
                target: Target::OtherPrincipals,
            },
            AccessClass::Admin,
            false,
            true,
        ),
        (
            Access::Admin {
                capability: Capability::new("accounts.manage"),
                effect: AdminEffect::Mutates(EventName::AuthzFail),
                target: Target::Caller,
            },
            AccessClass::Admin,
            true,
            false,
        ),
    ];

    #[test]
    fn an_access_answers_for_its_class_and_effect() {
        let answers: Vec<(AccessClass, bool, bool)> = CASES
            .iter()
            .map(|(access, ..)| (access.class(), access.mutates(), access.names_principals()))
            .collect();
        let expected: Vec<(AccessClass, bool, bool)> = CASES
            .iter()
            .map(|(_, class, mutates, names)| (*class, *mutates, *names))
            .collect();
        assert_eq!(answers, expected);
    }

    #[test]
    fn a_spec_mutates_when_its_access_does() {
        let spec = |access| RouteSpec {
            method: Method::Post,
            path: "/api/v1/a",
            access,
            tag: RouteTag::None,
            body: BodyRule::None,
            ids: &[],
            rate: RateClass::Write,
        };
        assert!(spec(CASES[1].0).mutates());
        assert!(!spec(CASES[0].0).mutates());
    }

    #[test]
    fn only_the_five_declarable_methods_have_names() {
        let named: Vec<Option<Method>> = [
            "GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS", "TRACE", "CONNECT", "get",
            "Get", "GET ", "", "PROPFIND",
        ]
        .into_iter()
        .map(Method::from_name)
        .collect();
        assert_eq!(
            named,
            [
                Some(Method::Get),
                Some(Method::Post),
                Some(Method::Put),
                Some(Method::Patch),
                Some(Method::Delete),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ]
        );
        assert_eq!(
            [
                Method::Get.as_str(),
                Method::Post.as_str(),
                Method::Put.as_str(),
                Method::Patch.as_str(),
                Method::Delete.as_str(),
            ],
            ["GET", "POST", "PUT", "PATCH", "DELETE"]
        );
    }

    #[test]
    fn a_capability_keeps_its_name_and_the_defaults_are_the_baseline_limits() {
        assert_eq!(Capability::new("library.read").name(), "library.read");
        assert_eq!(
            JsonLimits::DEFAULT,
            JsonLimits {
                bytes: 65_536,
                items: 1000
            }
        );
        assert_eq!(JsonLimits::DEPTH, 32);
    }
}
