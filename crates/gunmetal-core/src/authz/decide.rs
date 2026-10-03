//! The policy: one deny-by-default pure function (SEC-IAM-068).
//!
//! [`decide`] takes the principal's facts, an action from the closed list,
//! a description of the resource and the request's context, and returns
//! either a [`Permit`] or the typed reason for refusing. It allows only
//! when every check passes, in this order:
//!
//! 1. the principal holds the capability the action needs, within its
//!    kind's ceiling; its credential's scope allows it; its device class
//!    allows it; and its session is elevated if the capability runs the
//!    server;
//! 2. a scoped credential is not creating or changing a credential;
//! 3. the resource is one the principal may act on: an item in a library
//!    it can see, something its own account or profile owns, or a person of
//!    a kind the action may touch;
//! 4. the context adds no restriction that applies;
//! 5. the session holds a fresh user verification if the action is
//!    host-equivalent, or runs the server from a changed network.
//!
//! Nothing after step 1 can allow anything, so the context only ever
//! narrows (SEC-IAM-013). [`may_issue`] is the no-escalation rule for
//! everything that hands out capabilities: invitations, roles, API keys and
//! share links (SEC-IAM-073, SEC-TM-027).

use super::action::{Action, HOST_EQUIVALENT, Need, Target};
use super::capability::{Capability, CapabilitySet, Tier};
use super::context::{Context, Network, RemoteAdmin};
use super::library::LibrarySet;
use super::principal::{PrincipalFacts, PrincipalKind, Reach, Scope, UserVerification};
use crate::id::PublicId;
use crate::problem::{Describe, Problem, ProblemCode};

/// What an action acts on, as far as the policy needs to know.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceFacts {
    /// The server as a whole, or a listing that the permit's library set
    /// will filter.
    Server,
    /// An item in this library.
    Library(PublicId),
    /// Something this account or profile owns.
    Owned(Owner),
    /// A person: their account and its kind.
    Person {
        /// The person's account.
        account: PublicId,
        /// The kind of principal the account is.
        kind: PrincipalKind,
    },
}

/// Who owns an object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    /// An account, such as a playlist's or a credential's.
    Account(PublicId),
    /// A profile, such as a listening history's.
    Profile(PublicId),
}

/// Why the policy refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Denial {
    /// The action's name is not in the closed list (SEC-TM-005).
    UnknownAction,
    /// The principal does not hold this capability, or its kind may never
    /// hold it.
    MissingCapability(Capability),
    /// The principal holds the capability, but the credential it presented
    /// is scoped without it (SEC-API-020).
    OutOfScope(Capability),
    /// The capability cannot be used from this class of device.
    DeviceNotAllowed(Capability),
    /// The capability runs the server, so it needs an elevated session
    /// (SEC-IAM-041).
    ElevationRequired,
    /// A scoped credential may not create, widen or change any credential,
    /// its own included (SEC-API-020).
    ScopedCredential,
    /// The object does not exist as far as this principal may know: it is
    /// in a library it cannot see, someone else owns it, or it is not the
    /// kind of object the action acts on (SEC-API-011).
    NotVisible,
    /// The person is of a kind this action may not touch.
    TargetNotAllowed(PrincipalKind),
    /// A restriction on where the request came from applies (SEC-API-075).
    Location,
    /// The action needs a fresh user verification (SEC-IAM-041,
    /// SEC-IAM-013, SEC-TM-017).
    FreshVerificationRequired,
    /// The capability is owner-only, so it can never be handed to anyone
    /// (SEC-IAM-075).
    OwnerOnly(Capability),
    /// The capability is more than the issuer holds (SEC-IAM-073).
    Escalation(Capability),
}

impl Describe for Denial {
    /// A capability that runs the server, and an object the principal may
    /// not see, answer as not found, so that people without management
    /// rights cannot even tell that the function exists (SEC-HIS-013,
    /// SEC-API-011). A missing step-up asks for one; anything else is
    /// forbidden.
    fn problem(&self) -> Problem {
        let code = match self {
            Self::NotVisible => ProblemCode::NotFound,
            Self::MissingCapability(capability) => match capability.tier() {
                Tier::Everyday => ProblemCode::Forbidden,
                Tier::Admin | Tier::OwnerOnly => ProblemCode::NotFound,
            },
            Self::ElevationRequired | Self::FreshVerificationRequired => {
                ProblemCode::StepUpRequired
            }
            Self::UnknownAction
            | Self::OutOfScope(_)
            | Self::DeviceNotAllowed(_)
            | Self::ScopedCredential
            | Self::TargetNotAllowed(_)
            | Self::Location
            | Self::OwnerOnly(_)
            | Self::Escalation(_) => ProblemCode::Forbidden,
        };
        Problem {
            code,
            args: Vec::new(),
        }
    }
}

/// The result of an allowing decision: the only value storage readers
/// accept (SEC-API-010, SEC-TM-024).
///
/// Only [`decide`] can make one. It has no public constructor, its fields
/// are private, and it cannot be cloned, so holding one proves that the
/// policy allowed this action for this principal.
#[derive(Debug, PartialEq, Eq)]
pub struct Permit {
    action: Action,
    libraries: LibrarySet,
}

impl Permit {
    /// The action the policy allowed.
    #[must_use]
    pub const fn action(&self) -> Action {
        self.action
    }

    /// The libraries the principal may see, which every storage reader
    /// filters by (SEC-IAM-070).
    #[must_use]
    pub const fn libraries(&self) -> &LibrarySet {
        &self.libraries
    }
}

/// Decides whether the principal described by `principal` may perform
/// `action` on `resource` in `context`.
///
/// # Errors
///
/// Returns the first [`Denial`] that applies, in the order the module
/// documentation lists. Anything not explicitly allowed is refused.
pub fn decide(
    principal: &PrincipalFacts,
    action: Action,
    resource: &ResourceFacts,
    context: &Context,
) -> Result<Permit, Denial> {
    let rule = action.rule();
    let tier = match rule.need {
        Need::Nothing => Tier::Everyday,
        Need::Capability(capability) => {
            check_capability(principal, capability)?;
            capability.tier()
        }
    };
    if rule.credential && principal.scope.is_some() {
        return Err(Denial::ScopedCredential);
    }
    let libraries = visible_libraries(principal);
    match (rule.target, resource) {
        (Target::Server | Target::Library, ResourceFacts::Server) => {}
        (Target::Library, ResourceFacts::Library(library)) => {
            if !libraries.contains(library) {
                return Err(Denial::NotVisible);
            }
        }
        (Target::Own, ResourceFacts::Owned(owner)) => {
            let own = match owner {
                Owner::Account(account) => principal.account == Some(*account),
                Owner::Profile(profile) => principal.profile == Some(*profile),
            };
            if !own {
                return Err(Denial::NotVisible);
            }
        }
        (Target::Person(kinds), ResourceFacts::Person { kind, .. }) => {
            if !kinds.contains(kind) {
                return Err(Denial::TargetNotAllowed(*kind));
            }
        }
        _ => return Err(Denial::NotVisible),
    }
    let runs_server = !matches!(tier, Tier::Everyday);
    if principal.reach == Reach::HomeOnly && !context.is_home() {
        return Err(Denial::Location);
    }
    if runs_server && context.remote_admin == RemoteAdmin::Refused && !context.is_home() {
        return Err(Denial::Location);
    }
    let needs_fresh =
        HOST_EQUIVALENT.contains(&action) || (runs_server && context.network == Network::Changed);
    if needs_fresh && principal.verification != UserVerification::Fresh {
        return Err(Denial::FreshVerificationRequired);
    }
    Ok(Permit { action, libraries })
}

/// Checks the capability chain of step 1.
fn check_capability(principal: &PrincipalFacts, capability: Capability) -> Result<(), Denial> {
    if !principal
        .capabilities
        .intersection(principal.kind.ceiling())
        .contains(capability)
    {
        return Err(Denial::MissingCapability(capability));
    }
    if let Some(scope) = &principal.scope
        && !scope.capabilities.contains(capability)
    {
        return Err(Denial::OutOfScope(capability));
    }
    if !principal.device.ceiling().contains(capability) {
        return Err(Denial::DeviceNotAllowed(capability));
    }
    if !principal.elevation.ceiling().contains(capability) {
        return Err(Denial::ElevationRequired);
    }
    Ok(())
}

/// The libraries `capabilities` and `listed` reach.
fn reach(capabilities: CapabilitySet, listed: &[PublicId]) -> LibrarySet {
    if capabilities.contains(Capability::LibraryAll) {
        LibrarySet::every()
    } else {
        LibrarySet::listed(listed.to_vec())
    }
}

/// The libraries the principal may see: its own reach, within its
/// credential's scope (SEC-API-020).
fn visible_libraries(principal: &PrincipalFacts) -> LibrarySet {
    let own = reach(
        principal
            .capabilities
            .intersection(principal.kind.ceiling())
            .intersection(principal.device.ceiling()),
        &principal.libraries,
    );
    match &principal.scope {
        Some(scope) => own.intersection(reach(scope.capabilities, &scope.libraries)),
        None => own,
    }
}

/// Checks that `creator` may hand out `requested`: as an invitation, a
/// role, an API key or a share link, on creation or on update.
///
/// # Errors
///
/// - [`Denial::ScopedCredential`] when the creator is itself a scoped
///   credential, which may never mint one (SEC-API-020);
/// - [`Denial::OwnerOnly`] for the first owner-only capability requested,
///   which nobody may hand out, the owner included (SEC-IAM-075);
/// - [`Denial::Escalation`] for the first capability the creator cannot use
///   itself (SEC-IAM-073);
/// - [`Denial::NotVisible`] for a library the creator cannot see.
pub fn may_issue(creator: &PrincipalFacts, requested: &Scope) -> Result<Scope, Denial> {
    if creator.scope.is_some() {
        return Err(Denial::ScopedCredential);
    }
    if let Some(capability) = requested
        .capabilities
        .intersection(CapabilitySet::tier(Tier::OwnerOnly))
        .iter()
        .next()
    {
        return Err(Denial::OwnerOnly(capability));
    }
    if let Some(capability) = requested
        .capabilities
        .difference(creator.effective())
        .iter()
        .next()
    {
        return Err(Denial::Escalation(capability));
    }
    let libraries = visible_libraries(creator);
    if requested
        .libraries
        .iter()
        .any(|library| !libraries.contains(library))
    {
        return Err(Denial::NotVisible);
    }
    Ok(requested.clone())
}

/// Compile-fail tests: code outside the policy cannot make a `Permit`, so a
/// storage reader that takes one cannot be called without asking the policy.
/// Each shares its imports with the control, which compiles.
#[cfg(doctest)]
mod compile_fail {
    /// Control: outside the core, a permit comes from `decide` and can be
    /// read.
    ///
    /// ```
    /// use gunmetal_core::authz::{Action, Context, Denial, LibrarySet, Permit, PrincipalFacts, ResourceFacts, decide};
    ///
    /// fn ask(p: &PrincipalFacts, c: &Context) -> Result<(Action, Option<usize>), Denial> {
    ///     let permit: Permit = decide(p, Action::BrowseLibrary, &ResourceFacts::Server, c)?;
    ///     let libraries: &LibrarySet = permit.libraries();
    ///     Ok((permit.action(), libraries.restriction().map(<[_]>::len)))
    /// }
    /// ```
    struct Control;

    /// There is no constructor.
    /// Verifies: SEC-API-010
    ///
    /// ```compile_fail,E0599
    /// use gunmetal_core::authz::{Action, Context, Denial, LibrarySet, Permit, PrincipalFacts, ResourceFacts, decide};
    ///
    /// fn forge() -> Permit {
    ///     Permit::new(Action::BrowseLibrary)
    /// }
    /// ```
    struct NoConstructor;

    /// The fields are private, so a struct literal cannot build one.
    /// Verifies: SEC-API-010
    ///
    /// ```compile_fail,E0451
    /// use gunmetal_core::authz::{Action, Context, Denial, LibrarySet, Permit, PrincipalFacts, ResourceFacts, decide};
    ///
    /// fn forge(libraries: LibrarySet) -> Permit {
    ///     Permit { action: Action::BrowseLibrary, libraries }
    /// }
    /// ```
    struct NoStructLiteral;

    /// A permit cannot be copied into a second one.
    /// Verifies: SEC-API-010
    ///
    /// ```compile_fail,E0599
    /// use gunmetal_core::authz::{Action, Context, Denial, LibrarySet, Permit, PrincipalFacts, ResourceFacts, decide};
    ///
    /// fn copy(permit: &Permit) -> Permit {
    ///     permit.clone()
    /// }
    /// ```
    struct NoClone;

    /// A library set, the thing a permit carries, cannot be made outside
    /// the core either.
    /// Verifies: SEC-API-010
    ///
    /// ```compile_fail,E0624
    /// use gunmetal_core::authz::{Action, Context, Denial, LibrarySet, Permit, PrincipalFacts, ResourceFacts, decide};
    ///
    /// fn every() -> LibrarySet {
    ///     LibrarySet::every()
    /// }
    /// ```
    struct NoLibrarySet;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authz::capability::Role;
    use crate::authz::context::{Network, RemoteAdmin};
    use crate::authz::fixtures::{account, at, at_home, facts, library, profile};
    use crate::authz::principal::{DeviceClass, Elevation};
    use crate::client_context::PathClass;

    use Capability as C;
    use PrincipalKind as K;

    type Outcome = Result<(Action, LibrarySet), Denial>;

    fn ask(
        principal: &PrincipalFacts,
        action: Action,
        resource: &ResourceFacts,
        context: &Context,
    ) -> Outcome {
        decide(principal, action, resource, context)
            .map(|permit| (permit.action(), permit.libraries().clone()))
    }

    fn member() -> PrincipalFacts {
        facts(K::Member, Role::Member.preset())
    }

    fn owner() -> PrincipalFacts {
        facts(K::Owner, CapabilitySet::EVERY)
    }

    fn admin() -> PrincipalFacts {
        facts(K::Administrator, Role::Administrator.preset())
    }

    fn only(libraries: &[u8]) -> LibrarySet {
        LibrarySet::listed(libraries.iter().map(|n| library(*n)).collect())
    }

    fn scope(capabilities: &[Capability], libraries: &[u8]) -> Scope {
        Scope {
            capabilities: CapabilitySet::of(capabilities),
            libraries: libraries.iter().map(|n| library(*n)).collect(),
        }
    }

    fn person(kind: PrincipalKind) -> ResourceFacts {
        ResourceFacts::Person {
            account: account(2),
            kind,
        }
    }

    /// Verifies: SEC-TM-024
    #[test]
    fn a_library_item_is_visible_only_through_a_grant_or_library_all() {
        let mut granted = member();
        granted.libraries = vec![library(1), library(3)];
        let outcomes: Vec<Outcome> = (0..5)
            .map(|n| {
                ask(
                    &granted,
                    Action::StreamMedia,
                    &ResourceFacts::Library(library(n)),
                    &at_home(),
                )
            })
            .collect();
        let held = || Ok((Action::StreamMedia, only(&[1, 3])));
        assert_eq!(
            outcomes,
            [
                Err(Denial::NotVisible),
                held(),
                Err(Denial::NotVisible),
                held(),
                Err(Denial::NotVisible)
            ]
        );
        assert_eq!(
            ask(
                &granted,
                Action::BrowseLibrary,
                &ResourceFacts::Server,
                &at_home()
            ),
            Ok((Action::BrowseLibrary, only(&[1, 3])))
        );
        assert_eq!(
            ask(
                &admin(),
                Action::StreamMedia,
                &ResourceFacts::Library(library(7)),
                &at_home()
            ),
            Ok((Action::StreamMedia, LibrarySet::every()))
        );
    }

    /// A member granted `library.all` still sees only its grants: its kind
    /// may not hold it (A-564).
    #[test]
    fn only_the_owner_and_administrators_reach_every_library() {
        let mut member = member();
        member.capabilities = member.capabilities.with(C::LibraryAll);
        assert_eq!(
            ask(
                &member,
                Action::StreamMedia,
                &ResourceFacts::Library(library(2)),
                &at_home()
            ),
            Err(Denial::NotVisible)
        );
        let mut limited = owner();
        limited.device = DeviceClass::Limited;
        assert_eq!(
            ask(
                &limited,
                Action::StreamMedia,
                &ResourceFacts::Library(library(2)),
                &at_home()
            ),
            Ok((Action::StreamMedia, LibrarySet::every()))
        );
        let mut without = owner();
        without.capabilities = without
            .capabilities
            .difference(CapabilitySet::of(&[C::LibraryAll]));
        assert_eq!(
            ask(
                &without,
                Action::StreamMedia,
                &ResourceFacts::Library(library(2)),
                &at_home()
            ),
            Err(Denial::NotVisible)
        );
    }

    /// Verifies: SEC-TM-024
    #[test]
    fn own_objects_belong_to_the_principal_s_account_or_profile() {
        let cases = [
            (
                Owner::Account(account(1)),
                Ok((Action::ReadOwnData, only(&[1]))),
            ),
            (Owner::Account(account(2)), Err(Denial::NotVisible)),
            (
                Owner::Profile(profile(1)),
                Ok((Action::ReadOwnData, only(&[1]))),
            ),
            (Owner::Profile(profile(2)), Err(Denial::NotVisible)),
        ];
        for (owner, outcome) in cases {
            assert_eq!(
                ask(
                    &member(),
                    Action::ReadOwnData,
                    &ResourceFacts::Owned(owner),
                    &at_home()
                ),
                outcome
            );
        }
        let mut anonymous = facts(K::LinkHolder, CapabilitySet::EVERY);
        anonymous.account = None;
        anonymous.profile = None;
        for owner in [Owner::Account(account(1)), Owner::Profile(profile(1))] {
            assert_eq!(
                ask(
                    &anonymous,
                    Action::WriteOwnData,
                    &ResourceFacts::Owned(owner),
                    &at_home()
                ),
                Err(Denial::NotVisible)
            );
        }
    }

    #[test]
    fn each_action_on_a_person_touches_only_its_kinds() {
        let actions = [
            (Action::ManageUser, "..YYY....."),
            (Action::RecoverUser, "..YYY....."),
            (Action::EndSessions, ".YYYYY...."),
            (Action::ManageAdministrators, ".YY......."),
            (Action::TransferOwnership, ".YY......."),
        ];
        for (action, letters) in actions {
            let outcomes: Vec<Outcome> = K::ALL
                .iter()
                .map(|kind| ask(&owner(), action, &person(*kind), &at_home()))
                .collect();
            let expected: Vec<Outcome> = K::ALL
                .iter()
                .zip(letters.chars())
                .map(|(kind, letter)| {
                    if letter == 'Y' {
                        Ok((action, LibrarySet::every()))
                    } else {
                        Err(Denial::TargetNotAllowed(*kind))
                    }
                })
                .collect();
            assert_eq!(outcomes, expected);
        }
    }

    #[test]
    fn a_resource_of_the_wrong_shape_is_not_visible() {
        let cases = [
            (Action::SignOut, ResourceFacts::Library(library(1))),
            (
                Action::SignOut,
                ResourceFacts::Owned(Owner::Account(account(1))),
            ),
            (Action::ReadOwnData, ResourceFacts::Server),
            (
                Action::BrowseLibrary,
                ResourceFacts::Owned(Owner::Account(account(1))),
            ),
            (Action::BrowseLibrary, person(K::Member)),
            (Action::ManageUser, ResourceFacts::Server),
            (Action::ManageUser, ResourceFacts::Library(library(1))),
            (Action::ReadAuditLog, person(K::Member)),
        ];
        for (action, resource) in cases {
            assert_eq!(
                ask(&owner(), action, &resource, &at_home()),
                Err(Denial::NotVisible)
            );
        }
    }

    /// Verifies: SEC-API-020
    #[test]
    fn a_scoped_credential_gets_the_intersection_of_its_scope_and_its_holder() {
        // An API key scoped to reading libraries 1 and 3, held by a member
        // granted libraries 1 and 2.
        let mut key = member();
        key.libraries = vec![library(1), library(2)];
        key.scope = Some(scope(&[C::LibraryRead], &[1, 3]));
        assert_eq!(
            ask(
                &key,
                Action::BrowseLibrary,
                &ResourceFacts::Server,
                &at_home()
            ),
            Ok((Action::BrowseLibrary, only(&[1])))
        );
        assert_eq!(
            ask(
                &key,
                Action::StreamMedia,
                &ResourceFacts::Library(library(3)),
                &at_home()
            ),
            Err(Denial::NotVisible)
        );
        assert_eq!(
            ask(
                &key,
                Action::DownloadMedia,
                &ResourceFacts::Library(library(1)),
                &at_home()
            ),
            Err(Denial::OutOfScope(C::LibraryDownload))
        );
        // A scope cannot add what the holder lacks.
        key.scope = Some(scope(&[C::LibraryRead, C::LibraryManage], &[1]));
        assert_eq!(
            ask(
                &key,
                Action::ManageLibrary,
                &ResourceFacts::Library(library(1)),
                &at_home()
            ),
            Err(Denial::MissingCapability(C::LibraryManage))
        );
        // The owner's key narrows every library to its list, unless the
        // scope holds `library.all` too.
        let mut owner_key = owner();
        owner_key.scope = Some(scope(&[C::LibraryRead], &[2]));
        assert_eq!(
            ask(
                &owner_key,
                Action::BrowseLibrary,
                &ResourceFacts::Server,
                &at_home()
            ),
            Ok((Action::BrowseLibrary, only(&[2])))
        );
        owner_key.scope = Some(scope(&[C::LibraryRead, C::LibraryAll], &[]));
        assert_eq!(
            ask(
                &owner_key,
                Action::BrowseLibrary,
                &ResourceFacts::Server,
                &at_home()
            ),
            Ok((Action::BrowseLibrary, LibrarySet::every()))
        );
    }

    /// Verifies: SEC-API-020
    #[test]
    fn a_scoped_credential_never_touches_a_credential() {
        let mut key = owner();
        key.scope = Some(Scope {
            capabilities: CapabilitySet::EVERY,
            libraries: Vec::new(),
        });
        let credential_actions = [
            (
                Action::ManageOwnCredentials,
                ResourceFacts::Owned(Owner::Account(account(1))),
            ),
            (Action::InviteGuest, ResourceFacts::Server),
            (Action::InviteMember, ResourceFacts::Server),
            (Action::RecoverUser, person(K::Member)),
            (Action::ManageHouseholdDevices, ResourceFacts::Server),
            (Action::ManageAdministrators, person(K::Member)),
        ];
        for (action, resource) in &credential_actions {
            assert_eq!(
                ask(&key, *action, resource, &at_home()),
                Err(Denial::ScopedCredential)
            );
            assert_eq!(
                ask(&owner(), *action, resource, &at_home()),
                Ok((*action, LibrarySet::every()))
            );
        }
        assert_eq!(
            ask(&key, Action::ManageUser, &person(K::Member), &at_home()),
            Ok((Action::ManageUser, LibrarySet::every()))
        );
    }

    #[test]
    fn a_limited_device_may_browse_and_play_and_nothing_more() {
        let mut tv = owner();
        tv.device = DeviceClass::Limited;
        let cases = [
            (
                Action::StreamMedia,
                ResourceFacts::Library(library(1)),
                Ok((Action::StreamMedia, LibrarySet::every())),
            ),
            (
                Action::DownloadMedia,
                ResourceFacts::Library(library(1)),
                Err(Denial::DeviceNotAllowed(C::LibraryDownload)),
            ),
            (
                Action::ReadAuditLog,
                ResourceFacts::Server,
                Err(Denial::DeviceNotAllowed(C::AuditRead)),
            ),
            (
                Action::SignOut,
                ResourceFacts::Server,
                Ok((Action::SignOut, LibrarySet::every())),
            ),
        ];
        for (action, resource, outcome) in cases {
            assert_eq!(ask(&tv, action, &resource, &at_home()), outcome);
        }
    }

    /// Verifies: SEC-API-075
    #[test]
    fn a_home_only_person_is_refused_away_from_home() {
        let mut child = member();
        child.reach = Reach::HomeOnly;
        let resource = ResourceFacts::Library(library(1));
        let outcomes: Vec<Outcome> = [
            PathClass::Loopback,
            PathClass::Home,
            PathClass::Unknown,
            PathClass::Internet,
        ]
        .into_iter()
        .map(|path| ask(&child, Action::StreamMedia, &resource, &at(path)))
        .collect();
        let allowed = || Ok((Action::StreamMedia, only(&[1])));
        assert_eq!(
            outcomes,
            [
                allowed(),
                allowed(),
                Err(Denial::Location),
                Err(Denial::Location)
            ]
        );
        // A refusal for another reason stays that refusal.
        assert_eq!(
            ask(
                &child,
                Action::StreamMedia,
                &ResourceFacts::Library(library(2)),
                &at(PathClass::Internet)
            ),
            Err(Denial::NotVisible)
        );
    }

    #[test]
    fn administration_from_outside_is_refused_when_the_owner_says_so() {
        let refused = |path| Context {
            path,
            network: Network::Same,
            remote_admin: RemoteAdmin::Refused,
        };
        let paths = [
            PathClass::Loopback,
            PathClass::Home,
            PathClass::Unknown,
            PathClass::Internet,
        ];
        let audit: Vec<Outcome> = paths
            .into_iter()
            .map(|path| {
                ask(
                    &admin(),
                    Action::ReadAuditLog,
                    &ResourceFacts::Server,
                    &refused(path),
                )
            })
            .collect();
        let allowed = || Ok((Action::ReadAuditLog, LibrarySet::every()));
        assert_eq!(
            audit,
            [
                allowed(),
                allowed(),
                Err(Denial::Location),
                Err(Denial::Location)
            ]
        );
        // Everyday use from outside is not administration.
        let browse: Vec<Outcome> = paths
            .into_iter()
            .map(|path| {
                ask(
                    &admin(),
                    Action::BrowseLibrary,
                    &ResourceFacts::Server,
                    &refused(path),
                )
            })
            .collect();
        assert_eq!(
            browse,
            [(); 4].map(|()| Ok((Action::BrowseLibrary, LibrarySet::every())))
        );
        // With remote administration allowed, the path makes no difference.
        assert_eq!(
            ask(
                &admin(),
                Action::ReadAuditLog,
                &ResourceFacts::Server,
                &at(PathClass::Internet)
            ),
            allowed()
        );
    }

    /// Verifies: SEC-IAM-013
    #[test]
    fn running_the_server_from_a_changed_network_needs_fresh_verification() {
        let moved = Context {
            path: PathClass::Internet,
            network: Network::Changed,
            remote_admin: RemoteAdmin::Allowed,
        };
        let mut stale = admin();
        stale.verification = UserVerification::Stale;
        assert_eq!(
            ask(&stale, Action::ReadAuditLog, &ResourceFacts::Server, &moved),
            Err(Denial::FreshVerificationRequired)
        );
        assert_eq!(
            ask(
                &stale,
                Action::ReadAuditLog,
                &ResourceFacts::Server,
                &at(PathClass::Internet)
            ),
            Ok((Action::ReadAuditLog, LibrarySet::every()))
        );
        assert_eq!(
            ask(
                &admin(),
                Action::ReadAuditLog,
                &ResourceFacts::Server,
                &moved
            ),
            Ok((Action::ReadAuditLog, LibrarySet::every()))
        );
        // Everyday use needs no step-up after a network change.
        assert_eq!(
            ask(
                &stale,
                Action::BrowseLibrary,
                &ResourceFacts::Server,
                &moved
            ),
            Ok((Action::BrowseLibrary, LibrarySet::every()))
        );
    }

    /// Verifies: SEC-TM-017, SEC-IAM-075
    #[test]
    fn host_equivalent_actions_are_owner_only_except_roots_and_file_browsing() {
        for action in HOST_EQUIVALENT {
            let resource = if matches!(
                action,
                Action::ManageAdministrators | Action::TransferOwnership
            ) {
                person(K::Member)
            } else {
                ResourceFacts::Server
            };
            let for_admin = ask(&admin(), *action, &resource, &at_home());
            let shared = matches!(
                action,
                Action::ManageLibraryRoots | Action::BrowseFileSystem
            );
            assert_eq!(for_admin.is_ok(), shared);
            let mut stale = owner();
            stale.verification = UserVerification::Stale;
            assert_eq!(
                ask(&stale, *action, &resource, &at_home()),
                Err(Denial::FreshVerificationRequired)
            );
        }
    }

    /// Verifies: SEC-HIS-013, SEC-API-011
    #[test]
    fn denials_describe_themselves_by_the_catalogue() {
        let cases = [
            (Denial::UnknownAction, ProblemCode::Forbidden),
            (
                Denial::MissingCapability(C::LibraryDownload),
                ProblemCode::Forbidden,
            ),
            (
                Denial::MissingCapability(C::AuditRead),
                ProblemCode::NotFound,
            ),
            (Denial::MissingCapability(C::Backup), ProblemCode::NotFound),
            (Denial::OutOfScope(C::LibraryRead), ProblemCode::Forbidden),
            (
                Denial::DeviceNotAllowed(C::AuditRead),
                ProblemCode::Forbidden,
            ),
            (Denial::ElevationRequired, ProblemCode::StepUpRequired),
            (Denial::ScopedCredential, ProblemCode::Forbidden),
            (Denial::NotVisible, ProblemCode::NotFound),
            (Denial::TargetNotAllowed(K::Owner), ProblemCode::Forbidden),
            (Denial::Location, ProblemCode::Forbidden),
            (
                Denial::FreshVerificationRequired,
                ProblemCode::StepUpRequired,
            ),
            (Denial::OwnerOnly(C::Backup), ProblemCode::Forbidden),
            (Denial::Escalation(C::UserManage), ProblemCode::Forbidden),
        ];
        for (denial, code) in cases {
            assert_eq!(
                denial.problem(),
                Problem {
                    code,
                    args: Vec::new()
                }
            );
        }
    }

    /// Verifies: SEC-IAM-073, SEC-TM-027
    #[test]
    fn nobody_issues_more_than_they_hold() {
        let mut parent = member();
        parent.capabilities = parent.capabilities.with(C::InviteGuest);
        parent.libraries = vec![library(1), library(2)];
        let invite = scope(&[C::LibraryRead], &[2]);
        assert_eq!(may_issue(&parent, &invite), Ok(invite.clone()));
        assert_eq!(
            may_issue(&parent, &scope(&[C::LibraryRead, C::LibraryDownload], &[1])),
            Ok(scope(&[C::LibraryRead, C::LibraryDownload], &[1]))
        );
        assert_eq!(
            may_issue(&parent, &scope(&[C::LibraryRead], &[3])),
            Err(Denial::NotVisible)
        );
        assert_eq!(
            may_issue(
                &parent,
                &scope(&[C::LibraryRead, C::UserManage, C::AuditRead], &[1])
            ),
            Err(Denial::Escalation(C::UserManage))
        );
        // Held but not usable in an ordinary session is not held.
        assert_eq!(
            may_issue(&parent, &scope(&[C::InviteGuest], &[])),
            Ok(scope(&[C::InviteGuest], &[]))
        );
        parent.elevation = Elevation::Ordinary;
        assert_eq!(
            may_issue(&parent, &scope(&[C::InviteGuest], &[])),
            Err(Denial::Escalation(C::InviteGuest))
        );
        // An administrator may hand out every library and any capability
        // it holds.
        let admin_role = scope(
            &Role::Administrator.preset().iter().collect::<Vec<_>>(),
            &[4],
        );
        assert_eq!(may_issue(&admin(), &admin_role), Ok(admin_role.clone()));
        // A member cannot pass on `library.all`, which its kind cannot hold.
        let mut greedy = member();
        greedy.capabilities = CapabilitySet::EVERY;
        assert_eq!(
            may_issue(&greedy, &scope(&[C::LibraryAll], &[])),
            Err(Denial::Escalation(C::LibraryAll))
        );
    }

    /// Verifies: SEC-IAM-075
    #[test]
    fn no_one_issues_an_owner_only_capability_not_even_the_owner() {
        for capability in [
            C::AdminManage,
            C::OwnershipTransfer,
            C::SecuritySettings,
            C::OidcConfigure,
            C::PluginApprove,
            C::Backup,
        ] {
            assert_eq!(
                may_issue(&owner(), &scope(&[C::LibraryRead, capability], &[])),
                Err(Denial::OwnerOnly(capability))
            );
        }
    }

    /// Verifies: SEC-API-020
    #[test]
    fn a_scoped_credential_issues_nothing() {
        let mut key = owner();
        key.scope = Some(scope(&[C::LibraryRead], &[1]));
        assert_eq!(
            may_issue(&key, &scope(&[C::LibraryRead], &[1])),
            Err(Denial::ScopedCredential)
        );
        assert_eq!(
            may_issue(&key, &scope(&[], &[])),
            Err(Denial::ScopedCredential)
        );
    }
}
