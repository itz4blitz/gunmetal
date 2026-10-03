//! The security-event catalogue: every event the audit log records, typed,
//! with its OWASP Logging Vocabulary name.
//!
//! Producers (the credential verifier, the authorisation layer, the rate
//! limiters, the egress client, the logger's debug switch) build a
//! [`SecurityEvent`] and hand it to the [`SecuritySink`] they were given,
//! which answers [`AuditUnavailable`] when the record cannot be written, so
//! that the producer can refuse its action (SEC-OPS-020). The server wires
//! that sink to its event bus, which carries every event to the audit log
//! (WP-043, WP-069), so every producer emits the same typed events without
//! depending on the audit store. A route declares the event it emits by its
//! [`EventName`].
//!
//! An event's fields take a [`ClientContext`] and public identifiers, never
//! a credential: every field type must be an [`AuditField`], a sealed trait
//! implemented only here, and the catalogue does not compile otherwise. The
//! time, the sequence number and the hash chain are the audit log's to add.
//!
//! This file is a registry. A package that needs a new event adds one entry
//! to the catalogue below, in sorted order of its vocabulary name, and one
//! name to the literal list in the tests. Names come from the OWASP Logging
//! Vocabulary where one fits; Gunmetal's own events use a `gm_` prefix.

use crate::client_context::ClientContext;
use crate::id::PublicId;
use crate::problem::{Describe, Problem, ProblemCode};

mod sealed {
    /// Seals [`AuditField`](super::AuditField), so that only this module can
    /// say which types may be event fields.
    pub trait Sealed {}
}

/// A type that may be a field of a security event.
///
/// The trait is sealed: code outside this module cannot implement it. The
/// types that implement it carry identifiers and resolved addresses, and
/// none can hold a credential, a token or text a client sent, so no event
/// can either.
pub trait AuditField: sealed::Sealed {}

impl sealed::Sealed for ClientContext {}
impl AuditField for ClientContext {}
impl sealed::Sealed for PublicId {}
impl AuditField for PublicId {}
impl<T: AuditField> sealed::Sealed for Option<T> {}
impl<T: AuditField> AuditField for Option<T> {}

/// Declares the catalogue. Each entry is a documented event, its vocabulary
/// name and its documented fields, so that one entry holds everything about
/// an event. The macro makes the event type, the name type and the mapping
/// between them, and bounds every field type by [`AuditField`].
///
/// Mutation testing does not see code a macro generates, and coverage does
/// not count its match arms one by one, so the tests pin every vocabulary
/// name against a literal list and build one event of every kind.
///
/// The macro is exported, and hidden from the documentation, only so that
/// the compile-fail tests can declare entries through it and show that the
/// bound refuses a field. It is not a way to add events from outside this
/// file: a catalogue declared elsewhere is a different type, which no
/// [`SecuritySink`] accepts.
#[doc(hidden)]
#[macro_export]
macro_rules! security_events {
    ($(
        $(#[$doc:meta])*
        $event:ident = $vocabulary:literal {
            $( $(#[$field_doc:meta])* $field:ident: $type:ty, )*
        }
    )*) => {
        /// One security event: what happened, to which account and from
        /// where.
        // The where clause holds only when the type of every field of every
        // event is an `AuditField`. It sits on the type itself, not on an
        // impl, so the type cannot be declared otherwise.
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum SecurityEvent
        where
            $( $( $type: $crate::audit_event::AuditField, )* )*
        {
            $( $(#[$doc])* $event { $( $(#[$field_doc])* $field: $type, )* }, )*
        }

        /// Which entry of the catalogue an event is, without its data: what a
        /// route declares it emits.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum EventName {
            $( $(#[$doc])* $event, )*
        }

        impl EventName {
            /// Every event in the catalogue, in its sorted order.
            pub const ALL: &'static [Self] = &[$(Self::$event),*];

            /// The name the audit log records: the OWASP Logging Vocabulary
            /// name, or a Gunmetal name with the `gm_` prefix where the
            /// vocabulary has none.
            #[must_use]
            pub const fn vocabulary(self) -> &'static str {
                match self {
                    $(Self::$event => $vocabulary,)*
                }
            }
        }

        impl SecurityEvent {
            /// Which entry of the catalogue this event is.
            #[must_use]
            pub const fn name(&self) -> EventName {
                match self {
                    $(Self::$event { .. } => EventName::$event,)*
                }
            }
        }
    };
}

security_events! {
    /// A sign-in attempt failed, on any sign-in surface.
    AuthnLoginFail = "authn_login_fail" {
        /// Where the attempt came from.
        source: ClientContext,
        /// The account the attempt was for, when it named one that exists.
        account: Option<PublicId>,
    }
    /// Someone signed in.
    AuthnLoginSuccess = "authn_login_success" {
        /// Where the sign-in came from.
        source: ClientContext,
        /// The account that signed in.
        account: PublicId,
    }
    /// The authorisation layer refused a request.
    AuthzFail = "authz_fail" {
        /// Where the request came from.
        source: ClientContext,
        /// The account that made the request, or none for an anonymous one.
        account: Option<PublicId>,
    }
    /// A rate limiter refused a request.
    ExcessRateLimitExceeded = "excess_rate_limit_exceeded" {
        /// Where the request came from, which the limiter keyed on.
        source: ClientContext,
    }
}

/// Where producers send security events.
///
/// The server gives each producer one implementation, wired to its event
/// bus, which carries every event to the audit log's sink (WP-043, WP-069).
/// Tests give producers a sink that records what it receives, or one that
/// cannot write.
///
/// A producer records its event before its action takes effect. When the
/// record cannot be written, the action does not take effect (SEC-OPS-020).
/// A producer whose event records a refusal, such as a failed sign-in or a
/// refused request, still refuses: an unwritten record never lets anything
/// through.
///
/// ```
/// use gunmetal_core::audit_event::{AuditUnavailable, SecurityEvent, SecuritySink};
///
/// /// Turns a setting on, but only once the audit log has its record.
/// fn turn_on(
///     setting: &mut bool,
///     sink: &dyn SecuritySink,
///     event: SecurityEvent,
/// ) -> Result<(), AuditUnavailable> {
///     sink.record(event)?;
///     *setting = true;
///     Ok(())
/// }
/// ```
pub trait SecuritySink {
    /// Takes one event, and answers `Ok` once the audit log has its record.
    /// Which records must be on disk before the answer is the audit log's
    /// rule (WP-069); a sink wired to the bus passes the log's own answer
    /// back.
    ///
    /// # Errors
    ///
    /// [`AuditUnavailable`] when the record cannot be written. The action
    /// the event records must then not take effect.
    fn record(&self, event: SecurityEvent) -> Result<(), AuditUnavailable>;
}

/// The audit log could not write a security event's record, so the action
/// the event records must not take effect (SEC-OPS-020).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuditUnavailable;

impl Describe for AuditUnavailable {
    /// The action was refused because it could not be recorded.
    fn problem(&self) -> Problem {
        Problem {
            code: ProblemCode::AuditUnavailable,
            args: Vec::new(),
        }
    }
}

/// Compile-fail tests: no event can hold a credential. The core cannot see
/// the secrets crate's `Secret` type (WP-047), so a stand-in plays it.
///
/// Rustdoc on stable does not check which error a compile-fail test
/// produced, so each test is the control below with one change, and the
/// control compiles. The tests that declare an entry change only the type
/// of its `held` field. The stand-in derives every trait an event derives,
/// so the catalogue's `AuditField` bound is the only thing that can refuse
/// it: with the bound removed, those tests compile and fail.
#[cfg(doctest)]
mod compile_fail {
    /// Control: an entry whose fields are a context and identifiers is
    /// declared through the catalogue's own macro, and those types are
    /// audit fields.
    ///
    /// ```
    /// use gunmetal_core::audit_event::AuditField;
    /// use gunmetal_core::client_context::ClientContext;
    /// use gunmetal_core::id::PublicId;
    ///
    /// #[derive(Debug, Clone, PartialEq, Eq)]
    /// struct Secret;
    ///
    /// fn field<T: AuditField>() {}
    ///
    /// gunmetal_core::security_events! {
    ///     GmHeld = "gm_held" {
    ///         source: ClientContext,
    ///         account: PublicId,
    ///         held: Option<PublicId>,
    ///     }
    /// }
    ///
    /// field::<ClientContext>();
    /// field::<PublicId>();
    /// field::<Option<PublicId>>();
    /// ```
    struct Control;

    /// No catalogue entry can declare a field that holds a secret.
    ///
    /// ```compile_fail
    /// use gunmetal_core::audit_event::AuditField;
    /// use gunmetal_core::client_context::ClientContext;
    /// use gunmetal_core::id::PublicId;
    ///
    /// #[derive(Debug, Clone, PartialEq, Eq)]
    /// struct Secret;
    ///
    /// gunmetal_core::security_events! {
    ///     GmHeld = "gm_held" {
    ///         source: ClientContext,
    ///         account: PublicId,
    ///         held: Secret,
    ///     }
    /// }
    /// ```
    struct NoSecretInAField;

    /// An optional field is no way round the bound: an option is an audit
    /// field only when what it holds is one.
    ///
    /// ```compile_fail
    /// use gunmetal_core::audit_event::AuditField;
    /// use gunmetal_core::client_context::ClientContext;
    /// use gunmetal_core::id::PublicId;
    ///
    /// #[derive(Debug, Clone, PartialEq, Eq)]
    /// struct Secret;
    ///
    /// gunmetal_core::security_events! {
    ///     GmHeld = "gm_held" {
    ///         source: ClientContext,
    ///         account: PublicId,
    ///         held: Option<Secret>,
    ///     }
    /// }
    /// ```
    struct NoSecretInAnOptionalField;

    /// Nor can a field hold free text, such as a token or anything a client
    /// sent.
    ///
    /// ```compile_fail
    /// use gunmetal_core::audit_event::AuditField;
    /// use gunmetal_core::client_context::ClientContext;
    /// use gunmetal_core::id::PublicId;
    ///
    /// #[derive(Debug, Clone, PartialEq, Eq)]
    /// struct Secret;
    ///
    /// gunmetal_core::security_events! {
    ///     GmHeld = "gm_held" {
    ///         source: ClientContext,
    ///         account: PublicId,
    ///         held: String,
    ///     }
    /// }
    /// ```
    struct NoTextInAField;

    /// A secret's type cannot be made an audit field from outside the core,
    /// so no future entry can get past the bound that way.
    ///
    /// ```compile_fail
    /// use gunmetal_core::audit_event::AuditField;
    /// use gunmetal_core::client_context::ClientContext;
    /// use gunmetal_core::id::PublicId;
    ///
    /// #[derive(Debug, Clone, PartialEq, Eq)]
    /// struct Secret;
    ///
    /// impl AuditField for Secret {}
    /// ```
    struct SecretCannotBecomeAField;

    /// Nor can the seal itself be implemented from outside the core to
    /// unlock the trait.
    ///
    /// ```compile_fail
    /// use gunmetal_core::audit_event::AuditField;
    /// use gunmetal_core::client_context::ClientContext;
    /// use gunmetal_core::id::PublicId;
    ///
    /// #[derive(Debug, Clone, PartialEq, Eq)]
    /// struct Secret;
    ///
    /// impl gunmetal_core::audit_event::sealed::Sealed for Secret {}
    /// impl AuditField for Secret {}
    /// ```
    struct TheSealIsPrivate;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_context::PathClass;
    use crate::id::IdKind;
    use core::net::{IpAddr, Ipv4Addr};

    /// Every event's vocabulary name, in order, written out independently of
    /// the declaration above.
    const VOCABULARY: [&str; 4] = [
        "authn_login_fail",
        "authn_login_success",
        "authz_fail",
        "excess_rate_limit_exceeded",
    ];

    fn source() -> ClientContext {
        ClientContext::new(
            IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7)),
            PathClass::Internet,
            false,
        )
    }

    fn account() -> PublicId {
        PublicId::parse("usr_0123456789abcdefghjkmnpqrs", IdKind::User)
            .expect("a canonical user ID")
    }

    #[test]
    fn names_every_event_by_its_vocabulary_name() {
        let names: Vec<&str> = EventName::ALL
            .iter()
            .map(|name| name.vocabulary())
            .collect();
        assert_eq!(names, VOCABULARY);
    }

    #[test]
    fn vocabulary_names_are_unique_sorted_lower_case_words() {
        let names: Vec<&str> = EventName::ALL
            .iter()
            .map(|name| name.vocabulary())
            .collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(names, sorted, "names must be unique and in sorted order");
        for name in names {
            let shaped = name.starts_with(|c: char| c.is_ascii_lowercase())
                && name.ends_with(|c: char| c.is_ascii_lowercase())
                && name.chars().all(|c| c.is_ascii_lowercase() || c == '_');
            assert!(shaped, "name {name:?}");
        }
    }

    /// Verifies: SEC-API-072
    #[test]
    fn describes_an_unwritten_record_as_audit_unavailable() {
        assert_eq!(
            AuditUnavailable.problem(),
            Problem {
                code: ProblemCode::AuditUnavailable,
                args: vec![],
            }
        );
    }

    #[test]
    fn each_event_reports_its_own_name() {
        let cases = [
            (
                SecurityEvent::AuthnLoginFail {
                    source: source(),
                    account: None,
                },
                EventName::AuthnLoginFail,
            ),
            (
                SecurityEvent::AuthnLoginSuccess {
                    source: source(),
                    account: account(),
                },
                EventName::AuthnLoginSuccess,
            ),
            (
                SecurityEvent::AuthzFail {
                    source: source(),
                    account: Some(account()),
                },
                EventName::AuthzFail,
            ),
            (
                SecurityEvent::ExcessRateLimitExceeded { source: source() },
                EventName::ExcessRateLimitExceeded,
            ),
        ];
        let reported: Vec<(SecurityEvent, EventName)> = cases
            .iter()
            .map(|(event, _)| (event.clone(), event.name()))
            .collect();
        assert_eq!(reported, cases);
        // One case per catalogue entry, in catalogue order.
        let named: Vec<EventName> = cases.iter().map(|(_, name)| *name).collect();
        assert_eq!(named, EventName::ALL);
    }
}
