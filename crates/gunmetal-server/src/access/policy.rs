//! Asking the policy: where the server gets a `Permit`.
//!
//! The decision is the core's ([`decide`], one deny-by-default pure
//! function, SEC-IAM-068). [`permit`] makes no decision of its own. It
//! loads the one fact the policy needs from storage, the libraries the
//! principal holds a grant for right now, asks the policy, and passes its
//! answer on:
//!
//! - **Grants come from the store, on every call.** Whatever libraries the
//!   principal's facts claim are replaced by the rows the grants table
//!   holds for its account at that moment, so a grant or a revocation takes
//!   effect on the next request (SEC-IAM-076), and a principal with no
//!   account holds no library.
//! - **An error is a denial.** If the grants cannot be read, or a stored
//!   grant does not name a library, no permit is given, whatever the
//!   principal could otherwise do; there is no weaker check to fall back on
//!   (SEC-IAM-069).
//! - **Every refusal of a permit is recorded.** A denial by the policy and
//!   a failed grant read each emit one typed event into the injected
//!   [`SecuritySink`], which the server wires to its bus. The refusal
//!   stands, with the same answer, whether or not the record could be
//!   written: an unwritten record never lets anything through, and never
//!   makes a hidden object answer differently from a missing one. The
//!   layer's other refusals record nothing: a hidden row's not-found from
//!   [`check`](super::visible::check) and its siblings, and a refusal or a
//!   failure of the grants API.
//! - **The answer comes from the catalogue.** A denial is rendered as the
//!   problem it describes, so an object the principal may not see is not
//!   found, exactly as one that does not exist (SEC-API-011).
//!
//! What makes this the place, and what does not. The core's [`decide`] is a
//! public function, and every field of the facts and of the context it
//! takes is public, so the compiler lets any module of the server call it
//! and get a real `Permit` while skipping the grant read, the listener's
//! path class and the refusal's event. Only the architecture test
//! (`tests/storage_access.rs`) stops that: it reads the server's sources as
//! text on every gate run, and fails when a module outside this directory
//! names `decide` on a line of code. It does not see a call that a macro
//! writes from other text or that another crate makes, nor a module of this
//! directory handing the function on.

use gunmetal_core::audit_event::{SecurityEvent, SecuritySink};
use gunmetal_core::authz::{
    Action, Context, Network, Permit, Principal, PrincipalFacts, RemoteAdmin, ResourceFacts, decide,
};
use gunmetal_core::client_context::ClientContext;
use gunmetal_durable::identity::store::IdentityStore;
use gunmetal_http::problem::ApiError;

use super::grants::{self, GrantError};

/// One question for the policy.
#[derive(Debug, Clone, Copy)]
pub struct Ask<'a> {
    /// Who asks: the one principal the request resolved to.
    pub principal: &'a Principal,
    /// What it asks to do.
    pub action: Action,
    /// What the action acts on.
    pub resource: &'a ResourceFacts,
    /// Where the request came from, as the listener resolved it. Its path
    /// class is the one the policy is asked with.
    pub source: &'a ClientContext,
    /// Whether the request came from a network other than the one where
    /// the session last passed user verification.
    pub network: Network,
    /// Whether the owner allows administration from outside the home
    /// network.
    pub remote_admin: RemoteAdmin,
}

/// Asks the policy whether `ask.principal` may perform `ask.action`, with
/// the grants `store` holds for it right now.
///
/// # Errors
///
/// Returns the problem the policy's denial describes, or the generic error
/// when the principal's grants cannot be read. Either way one
/// [`SecurityEvent::AuthzFail`] is handed to `sink`, and no permit is
/// given.
pub fn permit(
    store: &IdentityStore,
    sink: &dyn SecuritySink,
    ask: &Ask<'_>,
) -> Result<Permit, ApiError> {
    facts(store, ask.principal)
        .map_err(|error| ApiError::from_core(&error))
        .and_then(|facts| {
            let context = Context {
                path: ask.source.class(),
                network: ask.network,
                remote_admin: ask.remote_admin,
            };
            decide(&facts, ask.action, ask.resource, &context)
                .map_err(|denial| ApiError::from_core(&denial))
        })
        .inspect_err(|_| {
            // The refusal stands whether or not its record was written.
            let _ = sink.record(SecurityEvent::AuthzFail {
                source: *ask.source,
                account: ask.principal.facts.account,
            });
        })
}

/// The principal's facts, holding the libraries the store grants its
/// account right now in place of any its facts claim.
fn facts(store: &IdentityStore, principal: &Principal) -> Result<PrincipalFacts, GrantError> {
    let libraries = match principal.facts.account {
        Some(account) => grants::held(store, account)?,
        None => Vec::new(),
    };
    Ok(PrincipalFacts {
        libraries,
        ..principal.facts.clone()
    })
}

#[cfg(test)]
mod tests {
    use super::{Ask, permit};
    use crate::access::grants::{grant, revoke};
    use crate::access::testing::{
        Data, account, ask, data, decided, give, give_raw, internet, library, loopback,
        lose_grants, member, owner, person, principal,
    };
    use crate::testing::Recording;
    use gunmetal_core::audit_event::SecurityEvent;
    use gunmetal_core::authz::{
        Action, Capability, CapabilitySet, Network, Owner, Principal, PrincipalKind, Reach,
        RemoteAdmin, ResourceFacts, Role, UserVerification,
    };
    use gunmetal_core::client_context::PathClass;
    use gunmetal_core::id::PublicId;
    use gunmetal_core::problem::ProblemCode;
    use gunmetal_fs::sqlite::Value;
    use gunmetal_http::problem::ApiError;

    /// What a permit says, or the refusal: the action it allows, the
    /// libraries its holder may see (`None` for every library) and the
    /// account it was decided for.
    type Outcome = Result<(Action, Option<Vec<PublicId>>, Option<PublicId>), ApiError>;

    /// Asks the layer and reads the permit it gives.
    fn outcome(data: &Data, sink: &Recording, ask: &Ask<'_>) -> Outcome {
        permit(&data.identity, sink, ask).map(|permit| {
            (
                permit.action(),
                permit.libraries().restriction().map(<[PublicId]>::to_vec),
                permit.account(),
            )
        })
    }

    /// What `principal` is answered for `action` on `resource`, asked from
    /// the internet, and the events that recorded.
    fn asked(
        data: &Data,
        principal: &Principal,
        action: Action,
        resource: &ResourceFacts,
    ) -> (Outcome, Vec<SecurityEvent>) {
        let sink = Recording::new(true);
        let outcome = outcome(data, &sink, &ask(principal, action, resource, &internet()));
        (outcome, sink.events())
    }

    /// The one event a refusal of account `n`, asked from the internet,
    /// records.
    fn refused(n: u8) -> Vec<SecurityEvent> {
        vec![SecurityEvent::AuthzFail {
            source: internet(),
            account: Some(account(n)),
        }]
    }

    fn denied(code: ProblemCode) -> Outcome {
        Err(ApiError::new(code))
    }

    /// An administrator, account 8, with the administrator preset.
    fn admin() -> Principal {
        principal(
            PrincipalKind::Administrator,
            Role::Administrator.preset(),
            8,
        )
    }

    #[test]
    fn the_test_requests_come_from_the_internet_and_from_the_server_itself() {
        assert_eq!(internet().class(), PathClass::Internet);
        assert_eq!(loopback().class(), PathClass::Loopback);
    }

    /// Verifies: SEC-IAM-070, SEC-TM-024
    #[test]
    fn a_permit_holds_the_libraries_the_store_grants_not_those_the_principal_claims() {
        let data = data();
        give(&data, account(1), library(3));
        give(&data, account(1), library(1));
        give(&data, account(2), library(2));
        // The principal's own facts claim library 2, which it was never
        // granted.
        let mut sam = member(1);
        sam.facts.libraries = vec![library(2)];
        assert_eq!(
            asked(&data, &sam, Action::BrowseLibrary, &ResourceFacts::Server),
            (
                Ok((
                    Action::BrowseLibrary,
                    Some(vec![library(1), library(3)]),
                    Some(account(1))
                )),
                vec![]
            )
        );
        assert_eq!(
            asked(
                &data,
                &sam,
                Action::StreamMedia,
                &ResourceFacts::Library(library(2))
            ),
            (denied(ProblemCode::NotFound), refused(1))
        );
        assert_eq!(
            asked(
                &data,
                &member(2),
                Action::StreamMedia,
                &ResourceFacts::Library(library(2))
            ),
            (
                Ok((
                    Action::StreamMedia,
                    Some(vec![library(2)]),
                    Some(account(2))
                )),
                vec![]
            )
        );
        // The owner sees every library and holds no row.
        assert_eq!(
            asked(
                &data,
                &owner(),
                Action::StreamMedia,
                &ResourceFacts::Library(library(7))
            ),
            (Ok((Action::StreamMedia, None, Some(account(9)))), vec![])
        );
    }

    /// Verifies: SEC-IAM-076
    #[test]
    fn a_grant_and_a_revocation_take_effect_on_the_next_request() {
        let data = data();
        let sam = member(1);
        let stream = |data: &Data| {
            asked(
                data,
                &sam,
                Action::StreamMedia,
                &ResourceFacts::Library(library(1)),
            )
        };
        let allowed = (
            Ok((
                Action::StreamMedia,
                Some(vec![library(1)]),
                Some(account(1)),
            )),
            vec![],
        );
        assert_eq!(stream(&data), (denied(ProblemCode::NotFound), refused(1)));
        let manage = decided(
            &owner(),
            &[],
            Action::ManageUser,
            &person(1, PrincipalKind::Member),
        );
        assert_eq!(grant(&data.identity, &manage, library(1)), Ok(true));
        assert_eq!(stream(&data), allowed);
        assert_eq!(revoke(&data.identity, &manage, library(1)), Ok(true));
        assert_eq!(stream(&data), (denied(ProblemCode::NotFound), refused(1)));
    }

    /// Verifies: SEC-IAM-069
    #[test]
    fn every_denial_is_answered_from_the_catalogue_and_recorded_once() {
        let data = data();
        give(&data, account(1), library(1));
        let sam = member(1);
        let own = ResourceFacts::Owned(Owner::Account(account(2)));
        let cases = [
            // A library the member holds no grant for.
            (
                Action::StreamMedia,
                ResourceFacts::Library(library(2)),
                ProblemCode::NotFound,
            ),
            // Someone else's data.
            (Action::ReadOwnData, own, ProblemCode::NotFound),
            // A capability that runs the server, which a member lacks.
            (
                Action::ReadAuditLog,
                ResourceFacts::Server,
                ProblemCode::NotFound,
            ),
        ];
        for (action, resource, code) in cases {
            assert_eq!(
                asked(&data, &sam, action, &resource),
                (denied(code), refused(1))
            );
        }
        // An everyday capability the guest preset does not hold.
        let guest = principal(PrincipalKind::Guest, Role::Guest.preset(), 3);
        give(&data, account(3), library(1));
        assert_eq!(
            asked(
                &data,
                &guest,
                Action::DownloadMedia,
                &ResourceFacts::Library(library(1))
            ),
            (denied(ProblemCode::Forbidden), refused(3))
        );
        // A host-equivalent action without a fresh user verification.
        let mut stale = owner();
        stale.facts.verification = UserVerification::Stale;
        assert_eq!(
            asked(&data, &stale, Action::RotateKeys, &ResourceFacts::Server),
            (denied(ProblemCode::StepUpRequired), refused(9))
        );
        // What is allowed records nothing.
        assert_eq!(
            asked(&data, &owner(), Action::RotateKeys, &ResourceFacts::Server),
            (Ok((Action::RotateKeys, None, Some(account(9)))), vec![])
        );
    }

    /// Verifies: SEC-IAM-069
    #[test]
    fn a_failed_grant_read_ends_in_denial_and_a_security_event_never_an_allowance() {
        let data = data();
        give(&data, account(1), library(1));
        lose_grants(&data);
        // The owner sees every library through a capability and needs no
        // grant, and is refused all the same: nothing was decided.
        assert_eq!(
            asked(
                &data,
                &owner(),
                Action::BrowseLibrary,
                &ResourceFacts::Server
            ),
            (denied(ProblemCode::InternalError), refused(9))
        );
        assert_eq!(
            asked(
                &data,
                &member(1),
                Action::StreamMedia,
                &ResourceFacts::Library(library(1))
            ),
            (denied(ProblemCode::InternalError), refused(1))
        );
        // Signing out needs no capability and no library, and is refused
        // too.
        assert_eq!(
            asked(&data, &member(1), Action::SignOut, &ResourceFacts::Server),
            (denied(ProblemCode::InternalError), refused(1))
        );
    }

    /// Verifies: SEC-IAM-069
    #[test]
    fn a_stored_grant_that_names_no_library_ends_in_denial_and_a_security_event() {
        let malformed = [
            Value::Text("trk_00000000000000000000000001".to_owned()),
            Value::Text("lib_".to_owned()),
            Value::Blob(b"lib_00000000000000000000000002".to_vec()),
        ];
        for library_column in malformed {
            let data = data();
            give(&data, account(1), library(1));
            give_raw(&data, account(1), library_column);
            // Library 1 is granted by a good row beside the bad one.
            assert_eq!(
                asked(
                    &data,
                    &member(1),
                    Action::StreamMedia,
                    &ResourceFacts::Library(library(1))
                ),
                (denied(ProblemCode::InternalError), refused(1))
            );
            // Another account's grants are still read.
            give(&data, account(2), library(1));
            assert_eq!(
                asked(
                    &data,
                    &member(2),
                    Action::BrowseLibrary,
                    &ResourceFacts::Server
                ),
                (
                    Ok((
                        Action::BrowseLibrary,
                        Some(vec![library(1)]),
                        Some(account(2))
                    )),
                    vec![]
                )
            );
        }
    }

    #[test]
    fn a_refusal_stands_with_the_same_answer_when_its_record_cannot_be_written() {
        let data = data();
        let unwritable = Recording::new(false);
        let sam = member(1);
        let hidden = ResourceFacts::Library(library(1));
        assert_eq!(
            outcome(
                &data,
                &unwritable,
                &ask(&sam, Action::StreamMedia, &hidden, &internet())
            ),
            denied(ProblemCode::NotFound)
        );
        assert_eq!(unwritable.events(), vec![]);
        // And what is allowed needs no record.
        assert_eq!(
            outcome(
                &data,
                &unwritable,
                &ask(
                    &sam,
                    Action::BrowseLibrary,
                    &ResourceFacts::Server,
                    &internet()
                )
            ),
            Ok((Action::BrowseLibrary, Some(vec![]), Some(account(1))))
        );
    }

    #[test]
    fn a_principal_with_no_account_holds_no_library_whatever_its_facts_claim() {
        let data = data();
        give(&data, account(1), library(1));
        let mut link = principal(
            PrincipalKind::LinkHolder,
            CapabilitySet::of(&[Capability::LibraryRead]),
            1,
        );
        link.facts.account = None;
        link.facts.libraries = vec![library(1)];
        assert_eq!(
            asked(&data, &link, Action::BrowseLibrary, &ResourceFacts::Server),
            (Ok((Action::BrowseLibrary, Some(vec![]), None)), vec![])
        );
        assert_eq!(
            asked(
                &data,
                &link,
                Action::StreamMedia,
                &ResourceFacts::Library(library(1))
            ),
            (
                denied(ProblemCode::NotFound),
                vec![SecurityEvent::AuthzFail {
                    source: internet(),
                    account: None,
                }]
            )
        );
    }

    #[test]
    fn the_request_s_path_network_and_the_owner_s_setting_reach_the_policy() {
        let data = data();
        give(&data, account(1), library(1));
        let sink = Recording::new(true);
        let server = ResourceFacts::Server;
        let (from_internet, from_server) = (internet(), loopback());

        // A person who may use the server only from home.
        let mut child = member(1);
        child.facts.reach = Reach::HomeOnly;
        let browsing = Ok((
            Action::BrowseLibrary,
            Some(vec![library(1)]),
            Some(account(1)),
        ));
        assert_eq!(
            outcome(
                &data,
                &sink,
                &ask(&child, Action::BrowseLibrary, &server, &from_internet)
            ),
            denied(ProblemCode::Forbidden)
        );
        assert_eq!(
            outcome(
                &data,
                &sink,
                &ask(&child, Action::BrowseLibrary, &server, &from_server)
            ),
            browsing
        );

        // Administration from outside, when the owner refuses it.
        let boss = admin();
        let reading = Ok((Action::ReadAuditLog, None, Some(account(8))));
        let refusing = Ask {
            remote_admin: RemoteAdmin::Refused,
            ..ask(&boss, Action::ReadAuditLog, &server, &from_internet)
        };
        assert_eq!(
            outcome(&data, &sink, &refusing),
            denied(ProblemCode::Forbidden)
        );
        assert_eq!(
            outcome(
                &data,
                &sink,
                &ask(&boss, Action::ReadAuditLog, &server, &from_internet)
            ),
            reading
        );

        // Administration from a changed network, without a fresh user
        // verification.
        let mut stale = admin();
        stale.facts.verification = UserVerification::Stale;
        let moved = Ask {
            network: Network::Changed,
            ..ask(&stale, Action::ReadAuditLog, &server, &from_internet)
        };
        assert_eq!(
            outcome(&data, &sink, &moved),
            denied(ProblemCode::StepUpRequired)
        );
        assert_eq!(
            outcome(
                &data,
                &sink,
                &ask(&stale, Action::ReadAuditLog, &server, &from_internet)
            ),
            reading
        );
        // Each of the three refusals was recorded with where it came from.
        assert_eq!(
            sink.events(),
            vec![
                SecurityEvent::AuthzFail {
                    source: from_internet,
                    account: Some(account(1)),
                },
                SecurityEvent::AuthzFail {
                    source: from_internet,
                    account: Some(account(8)),
                },
                SecurityEvent::AuthzFail {
                    source: from_internet,
                    account: Some(account(8)),
                },
            ]
        );
    }
}
