//! Properties of the policy over generated principals, actions, resources
//! and contexts.

use proptest::prelude::*;
use proptest::sample::{select, subsequence};

use super::action::{Action, HOST_EQUIVALENT};
use super::capability::{Capability, CapabilitySet};
use super::context::{Context, Network, RemoteAdmin};
use super::decide::{Denial, Owner, Permit, ResourceFacts, decide, may_issue};
use super::fixtures::{account, at_home, library, profile};
use super::principal::{
    DeviceClass, Elevation, PrincipalFacts, PrincipalKind, Reach, Scope, UserVerification,
};
use crate::client_context::PathClass;
use crate::id::PublicId;

/// The actions any principal may ask for with no capability: signing out,
/// and its own data.
const NEED_NOTHING: [Action; 4] = [
    Action::SignOut,
    Action::ReadOwnData,
    Action::WriteOwnData,
    Action::ManageOwnCredentials,
];

/// The capabilities nobody but the owner may hold.
const OWNER_ONLY: [Capability; 6] = [
    Capability::AdminManage,
    Capability::OwnershipTransfer,
    Capability::SecuritySettings,
    Capability::OidcConfigure,
    Capability::PluginApprove,
    Capability::Backup,
];

/// The paths, from the one that restricts least to the one that restricts
/// most.
const PATHS: [PathClass; 4] = [
    PathClass::Loopback,
    PathClass::Home,
    PathClass::Unknown,
    PathClass::Internet,
];

fn capabilities() -> impl Strategy<Value = CapabilitySet> {
    subsequence(Capability::ALL.to_vec(), 0..=Capability::ALL.len())
        .prop_map(|list| CapabilitySet::of(&list))
}

fn libraries() -> impl Strategy<Value = Vec<PublicId>> {
    subsequence(vec![0_u8, 1, 2, 3], 0..=4)
        .prop_map(|numbers| numbers.into_iter().map(library).collect())
}

fn scope() -> impl Strategy<Value = Scope> {
    (capabilities(), libraries()).prop_map(|(capabilities, libraries)| Scope {
        capabilities,
        libraries,
    })
}

fn kind() -> impl Strategy<Value = PrincipalKind> {
    select(PrincipalKind::ALL.to_vec())
}

fn facts_of(kind: impl Strategy<Value = PrincipalKind>) -> impl Strategy<Value = PrincipalFacts> {
    (
        kind,
        select(vec![None, Some(account(1))]),
        select(vec![None, Some(profile(1))]),
        capabilities(),
        libraries(),
        select(vec![DeviceClass::Personal, DeviceClass::Limited]),
        select(vec![Elevation::Ordinary, Elevation::Elevated]),
        select(vec![UserVerification::Stale, UserVerification::Fresh]),
        select(vec![Reach::Anywhere, Reach::HomeOnly]),
        proptest::option::of(scope()),
    )
        .prop_map(
            |(
                kind,
                account,
                profile,
                capabilities,
                libraries,
                device,
                elevation,
                verification,
                reach,
                scope,
            )| {
                PrincipalFacts {
                    kind,
                    account,
                    profile,
                    capabilities,
                    libraries,
                    device,
                    elevation,
                    verification,
                    reach,
                    scope,
                }
            },
        )
}

fn facts() -> impl Strategy<Value = PrincipalFacts> {
    facts_of(kind())
}

fn action() -> impl Strategy<Value = Action> {
    select(Action::ALL.to_vec())
}

fn resource() -> impl Strategy<Value = ResourceFacts> {
    prop_oneof![
        Just(ResourceFacts::Server),
        (0_u8..4).prop_map(|n| ResourceFacts::Library(library(n))),
        (1_u8..3).prop_map(|n| ResourceFacts::Owned(Owner::Account(account(n)))),
        (1_u8..3).prop_map(|n| ResourceFacts::Owned(Owner::Profile(profile(n)))),
        kind().prop_map(|kind| ResourceFacts::Person {
            account: account(2),
            kind
        }),
    ]
}

/// A context as three ranks, each from least to most restrictive.
fn ranks() -> impl Strategy<Value = (usize, usize, usize)> {
    (0..PATHS.len(), 0_usize..2, 0_usize..2)
}

fn context((path, network, remote): (usize, usize, usize)) -> Context {
    Context {
        path: PATHS.get(path).copied().unwrap_or(PathClass::Internet),
        network: if network == 0 {
            Network::Same
        } else {
            Network::Changed
        },
        remote_admin: if remote == 0 {
            RemoteAdmin::Allowed
        } else {
            RemoteAdmin::Refused
        },
    }
}

fn allowed(result: &Result<Permit, Denial>) -> bool {
    result.is_ok()
}

/// The libraries a scope reaches, written independently of the policy.
/// Both answers are worked out on every call, so which of them the
/// generated cases exercise does not change the coverage.
fn scope_reaches(scope: &Scope, library: &PublicId) -> bool {
    [
        scope.capabilities.contains(Capability::LibraryAll),
        scope.libraries.contains(library),
    ]
    .contains(&true)
}

proptest! {
    /// Verifies: SEC-IAM-068
    #[test]
    fn a_principal_with_no_grants_may_only_sign_out_and_use_its_own_data(
        mut principal in facts(),
        action in action(),
        resource in resource(),
        ranks in ranks(),
    ) {
        principal.capabilities = CapabilitySet::EMPTY;
        principal.libraries = Vec::new();
        let result = decide(&principal, action, &resource, &context(ranks));
        prop_assert!(!allowed(&result) || NEED_NOTHING.contains(&action));
        prop_assert!(!allowed(&result) || matches!(resource, ResourceFacts::Server | ResourceFacts::Owned(_)));
    }

    /// Verifies: SEC-IAM-068
    #[test]
    fn adding_a_grant_never_removes_an_allowance(
        principal in facts(),
        action in action(),
        resource in resource(),
        ranks in ranks(),
        extra in capabilities(),
        more in libraries(),
    ) {
        let context = context(ranks);
        let before = decide(&principal, action, &resource, &context);
        let mut granted = principal.clone();
        granted.capabilities = granted.capabilities.union(extra);
        granted.libraries.extend(more);
        let after = decide(&granted, action, &resource, &context);
        prop_assert!(!allowed(&before) || allowed(&after));
        if let (Ok(before), Ok(after)) = (&before, &after) {
            for n in 0..4 {
                let library = library(n);
                prop_assert!(!before.libraries().contains(&library) || after.libraries().contains(&library));
            }
        }
    }

    /// Verifies: SEC-IAM-013, SEC-API-075
    #[test]
    fn a_more_restrictive_context_never_turns_a_denial_into_an_allowance(
        principal in facts(),
        action in action(),
        resource in resource(),
        first in ranks(),
        second in ranks(),
    ) {
        let looser = context(first);
        let stricter = context((first.0.max(second.0), first.1.max(second.1), first.2.max(second.2)));
        let loose = decide(&principal, action, &resource, &looser);
        let strict = decide(&principal, action, &resource, &stricter);
        prop_assert!(allowed(&loose) || !allowed(&strict));
        // No context allows what the least restrictive one refuses.
        prop_assert!(!allowed(&strict) || allowed(&decide(&principal, action, &resource, &at_home())));
    }

    /// Verifies: SEC-HIS-001, SEC-API-075
    #[test]
    fn without_a_location_restriction_the_path_never_changes_the_decision(
        mut principal in facts(),
        action in action(),
        resource in resource(),
        changed in any::<bool>(),
    ) {
        principal.reach = Reach::Anywhere;
        let decisions: Vec<Result<Action, Denial>> = PATHS
            .into_iter()
            .map(|path| {
                let context = Context {
                    path,
                    network: if changed { Network::Changed } else { Network::Same },
                    remote_admin: RemoteAdmin::Allowed,
                };
                decide(&principal, action, &resource, &context).map(|permit| permit.action())
            })
            .collect();
        let first = decisions.first().cloned();
        prop_assert!(decisions.iter().all(|decision| Some(decision) == first.as_ref()));
    }

    /// Verifies: SEC-API-020
    #[test]
    fn a_scoped_credential_is_the_intersection_of_its_scope_and_its_holder(
        principal in facts(),
        scope in scope(),
        action in action(),
        resource in resource(),
        ranks in ranks(),
    ) {
        let context = context(ranks);
        let mut scoped = principal.clone();
        scoped.scope = Some(scope.clone());
        let mut holder = principal;
        holder.scope = None;
        let with_scope = decide(&scoped, action, &resource, &context);
        let without = decide(&holder, action, &resource, &context);
        prop_assert!(!allowed(&with_scope) || allowed(&without));
        if let (Ok(with_scope), Ok(without)) = (&with_scope, &without) {
            for n in 0..4 {
                let library = library(n);
                let seen = with_scope.libraries().contains(&library);
                let reached = [
                    without.libraries().contains(&library),
                    scope_reaches(&scope, &library),
                ];
                prop_assert!(!seen || reached == [true, true]);
            }
        }
        let credential = matches!(
            action,
            Action::ManageOwnCredentials
                | Action::InviteGuest
                | Action::InviteMember
                | Action::RecoverUser
                | Action::ManageHouseholdDevices
                | Action::ManageAdministrators
        );
        prop_assert!(!credential || !allowed(&with_scope));
    }

    /// Verifies: SEC-IAM-073, SEC-IAM-075, SEC-TM-027
    #[test]
    fn may_issue_never_returns_more_than_the_creator_holds(
        creator in facts(),
        requested in scope(),
    ) {
        if let Ok(issued) = may_issue(&creator, &requested) {
            prop_assert_eq!(&issued, &requested);
            prop_assert!(creator.scope.is_none());
            prop_assert!(issued.capabilities.is_subset(creator.effective()));
            prop_assert!(OWNER_ONLY.iter().all(|capability| !issued.capabilities.contains(*capability)));
            let every = creator
                .capabilities
                .intersection(creator.kind.ceiling())
                .intersection(creator.device.ceiling())
                .contains(Capability::LibraryAll);
            for library in &issued.libraries {
                prop_assert!(every || creator.libraries.contains(library));
            }
        }
    }

    /// Verifies: SEC-IAM-075
    #[test]
    fn no_sequence_of_grants_gives_a_non_owner_an_owner_only_capability(
        mut principal in facts_of(select(PrincipalKind::ALL[1..].to_vec())),
        grants in proptest::collection::vec(capabilities(), 0..8),
        resource in resource(),
        ranks in ranks(),
    ) {
        for grant in grants {
            principal.capabilities = principal.capabilities.union(grant);
        }
        principal.capabilities = principal.capabilities.union(CapabilitySet::of(&OWNER_ONLY));
        let owner_only = [
            Action::ManageAdministrators,
            Action::TransferOwnership,
            Action::ChangeSecuritySettings,
            Action::RotateKeys,
            Action::ConfigureOidc,
            Action::ApprovePlugin,
            Action::DownloadBackup,
            Action::RestoreBackup,
        ];
        for action in owner_only {
            let result = decide(&principal, action, &resource, &context(ranks));
            prop_assert!(matches!(result, Err(Denial::MissingCapability(_))));
        }
        let mut issuer = principal.clone();
        issuer.scope = None;
        for capability in OWNER_ONLY {
            let requested = Scope { capabilities: CapabilitySet::of(&[capability]), libraries: Vec::new() };
            prop_assert_eq!(may_issue(&issuer, &requested), Err(Denial::OwnerOnly(capability)));
        }
    }

    /// Verifies: SEC-TM-017
    #[test]
    fn every_host_equivalent_action_needs_fresh_user_verification(
        mut principal in facts(),
        resource in resource(),
        ranks in ranks(),
    ) {
        principal.verification = UserVerification::Stale;
        for action in HOST_EQUIVALENT {
            prop_assert!(!allowed(&decide(&principal, *action, &resource, &context(ranks))));
        }
    }
}
