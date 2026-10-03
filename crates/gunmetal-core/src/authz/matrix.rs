//! The capability matrix, written out literally: the expected decision for
//! every principal kind and every action.

use super::action::Action;
use super::capability::{Capability, CapabilitySet, Role};
use super::decide::{Denial, Owner, ResourceFacts, decide};
use super::fixtures::{account, at_home, facts, library};
use super::library::LibrarySet;
use super::principal::{Elevation, PrincipalKind, UserVerification};

use Action as A;
use Capability as C;

/// What a row acts on.
#[derive(Clone, Copy)]
enum On {
    /// The server.
    Server,
    /// An item in library 1, which every fixture principal was granted.
    Library,
    /// Something account 1, the fixture principal's own, owns.
    Mine,
    /// Account 2, a member.
    Member,
}

impl On {
    fn resource(self) -> ResourceFacts {
        match self {
            Self::Server => ResourceFacts::Server,
            Self::Library => ResourceFacts::Library(library(1)),
            Self::Mine => ResourceFacts::Owned(Owner::Account(account(1))),
            Self::Member => ResourceFacts::Person {
                account: account(2),
                kind: PrincipalKind::Member,
            },
        }
    }
}

/// One row per action: what it acts on, the capability it needs (if any),
/// and one letter per principal kind, in the order of
/// [`PrincipalKind::ALL`]: owner, administrator, member, managed profile,
/// guest, device, API client, plugin, peer server, link holder. `Y` is
/// allowed and `m` is refused for want of the capability.
type Row = (Action, On, Option<Capability>, &'static str);

/// Every kind holding every capability, so the answers show each kind's
/// ceiling.
const CEILINGS: [Row; 29] = [
    (A::SignOut, On::Server, None, "YYYYYYYYYY"),
    (A::ReadOwnData, On::Mine, None, "YYYYYYYYYY"),
    (A::WriteOwnData, On::Mine, None, "YYYYYYYYYY"),
    (A::ManageOwnCredentials, On::Mine, None, "YYYYYYYYYY"),
    (
        A::BrowseLibrary,
        On::Library,
        Some(C::LibraryRead),
        "YYYYYYYYYY",
    ),
    (
        A::StreamMedia,
        On::Library,
        Some(C::LibraryRead),
        "YYYYYYYYYY",
    ),
    (
        A::DownloadMedia,
        On::Library,
        Some(C::LibraryDownload),
        "YYYYYmYmmm",
    ),
    (
        A::SharePlaylist,
        On::Mine,
        Some(C::PlaylistShare),
        "YYYmYmYmmm",
    ),
    (
        A::ManageLibrary,
        On::Library,
        Some(C::LibraryManage),
        "YYYmmmmmmm",
    ),
    (
        A::InviteGuest,
        On::Server,
        Some(C::InviteGuest),
        "YYYmmmmmmm",
    ),
    (
        A::InviteMember,
        On::Server,
        Some(C::InviteMember),
        "YYmmmmmmmm",
    ),
    (A::ManageUser, On::Member, Some(C::UserManage), "YYmmmmmmmm"),
    (
        A::RecoverUser,
        On::Member,
        Some(C::UserRecover),
        "YYmmmmmmmm",
    ),
    (
        A::EndSessions,
        On::Member,
        Some(C::SessionManage),
        "YYmmmmmmmm",
    ),
    (
        A::ManageHouseholdDevices,
        On::Server,
        Some(C::HouseholdDevice),
        "YYYmmmmmmm",
    ),
    (
        A::ManageHouseholdProfiles,
        On::Server,
        Some(C::HouseholdProfile),
        "YYYmmmmmmm",
    ),
    (
        A::ReadAuditLog,
        On::Server,
        Some(C::AuditRead),
        "YYmmmmmmmm",
    ),
    (
        A::ChangeServerSettings,
        On::Server,
        Some(C::ServerSettings),
        "YYmmmmmmmm",
    ),
    (
        A::InspectFile,
        On::Library,
        Some(C::HostFiles),
        "YYmmmmmmmm",
    ),
    (
        A::ManageLibraryRoots,
        On::Server,
        Some(C::HostFiles),
        "YYmmmmmmmm",
    ),
    (
        A::BrowseFileSystem,
        On::Server,
        Some(C::HostFiles),
        "YYmmmmmmmm",
    ),
    (
        A::ManageAdministrators,
        On::Member,
        Some(C::AdminManage),
        "Ymmmmmmmmm",
    ),
    (
        A::TransferOwnership,
        On::Member,
        Some(C::OwnershipTransfer),
        "Ymmmmmmmmm",
    ),
    (
        A::ChangeSecuritySettings,
        On::Server,
        Some(C::SecuritySettings),
        "Ymmmmmmmmm",
    ),
    (
        A::RotateKeys,
        On::Server,
        Some(C::SecuritySettings),
        "Ymmmmmmmmm",
    ),
    (
        A::ConfigureOidc,
        On::Server,
        Some(C::OidcConfigure),
        "Ymmmmmmmmm",
    ),
    (
        A::ApprovePlugin,
        On::Server,
        Some(C::PluginApprove),
        "Ymmmmmmmmm",
    ),
    (A::DownloadBackup, On::Server, Some(C::Backup), "Ymmmmmmmmm"),
    (A::RestoreBackup, On::Server, Some(C::Backup), "Ymmmmmmmmm"),
];

/// The same actions for an account of each role holding only its role's
/// preset, in the order owner, administrator, member, guest.
const PRESETS: [(Action, &str); 29] = [
    (A::SignOut, "YYYY"),
    (A::ReadOwnData, "YYYY"),
    (A::WriteOwnData, "YYYY"),
    (A::ManageOwnCredentials, "YYYY"),
    (A::BrowseLibrary, "YYYY"),
    (A::StreamMedia, "YYYY"),
    (A::DownloadMedia, "YYYm"),
    (A::SharePlaylist, "YYYm"),
    (A::ManageLibrary, "YYmm"),
    (A::InviteGuest, "YYmm"),
    (A::InviteMember, "YYmm"),
    (A::ManageUser, "YYmm"),
    (A::RecoverUser, "YYmm"),
    (A::EndSessions, "YYmm"),
    (A::ManageHouseholdDevices, "YYmm"),
    (A::ManageHouseholdProfiles, "YYmm"),
    (A::ReadAuditLog, "YYmm"),
    (A::ChangeServerSettings, "YYmm"),
    (A::InspectFile, "YYmm"),
    (A::ManageLibraryRoots, "YYmm"),
    (A::BrowseFileSystem, "YYmm"),
    (A::ManageAdministrators, "Ymmm"),
    (A::TransferOwnership, "Ymmm"),
    (A::ChangeSecuritySettings, "Ymmm"),
    (A::RotateKeys, "Ymmm"),
    (A::ConfigureOidc, "Ymmm"),
    (A::ApprovePlugin, "Ymmm"),
    (A::DownloadBackup, "Ymmm"),
    (A::RestoreBackup, "Ymmm"),
];

/// The owner, holding everything, in three session states: ordinary with a
/// stale verification, elevated with a stale verification, and elevated
/// with a fresh one. `Y` is allowed, `e` needs elevation and `f` needs a
/// fresh user verification.
const SESSIONS: [(Action, &str); 29] = [
    (A::SignOut, "YYY"),
    (A::ReadOwnData, "YYY"),
    (A::WriteOwnData, "YYY"),
    (A::ManageOwnCredentials, "YYY"),
    (A::BrowseLibrary, "YYY"),
    (A::StreamMedia, "YYY"),
    (A::DownloadMedia, "YYY"),
    (A::SharePlaylist, "YYY"),
    (A::ManageLibrary, "eYY"),
    (A::InviteGuest, "eYY"),
    (A::InviteMember, "eYY"),
    (A::ManageUser, "eYY"),
    (A::RecoverUser, "eYY"),
    (A::EndSessions, "eYY"),
    (A::ManageHouseholdDevices, "eYY"),
    (A::ManageHouseholdProfiles, "eYY"),
    (A::ReadAuditLog, "eYY"),
    (A::ChangeServerSettings, "eYY"),
    (A::InspectFile, "eYY"),
    (A::ManageLibraryRoots, "efY"),
    (A::BrowseFileSystem, "efY"),
    (A::ManageAdministrators, "efY"),
    (A::TransferOwnership, "efY"),
    (A::ChangeSecuritySettings, "efY"),
    (A::RotateKeys, "efY"),
    (A::ConfigureOidc, "efY"),
    (A::ApprovePlugin, "efY"),
    (A::DownloadBackup, "efY"),
    (A::RestoreBackup, "efY"),
];

/// A decision as the tests compare it: the permit's action and library
/// set, or the denial.
type Outcome = Result<(Action, LibrarySet), Denial>;

fn outcome(result: Result<super::decide::Permit, Denial>) -> Outcome {
    result.map(|permit| (permit.action(), permit.libraries().clone()))
}

/// The libraries a principal of `kind` holding `capabilities` sees: every
/// library with `library.all` within its kind's ceiling, which only the
/// owner and administrators can hold, and otherwise its one grant.
fn seen(kind: PrincipalKind, capabilities: CapabilitySet) -> LibrarySet {
    let all = matches!(kind, PrincipalKind::Owner | PrincipalKind::Administrator)
        && capabilities.contains(C::LibraryAll);
    if all {
        LibrarySet::every()
    } else {
        LibrarySet::listed(vec![library(1)])
    }
}

/// The expected outcome for one letter of a table.
fn expected(
    letter: char,
    action: Action,
    need: Option<Capability>,
    libraries: LibrarySet,
) -> Outcome {
    match letter {
        'Y' => Ok((action, libraries)),
        'e' => Err(Denial::ElevationRequired),
        'f' => Err(Denial::FreshVerificationRequired),
        // `m`: every row that uses it names a capability.
        _ => Err(need.map_or(Denial::UnknownAction, Denial::MissingCapability)),
    }
}

/// The capability and target of each action, from the first table.
fn row(action: Action) -> (On, Option<Capability>) {
    CEILINGS
        .iter()
        .find(|(listed, ..)| *listed == action)
        .map(|(_, on, need, _)| (*on, *need))
        .unwrap()
}

/// Verifies: SEC-IAM-068, SEC-IAM-075
#[test]
fn every_kind_holding_everything_gets_exactly_its_ceiling() {
    let actions: Vec<Action> = CEILINGS.iter().map(|(action, ..)| *action).collect();
    assert_eq!(actions, Action::ALL);
    for (action, on, need, letters) in CEILINGS {
        let decided: Vec<Outcome> = PrincipalKind::ALL
            .iter()
            .map(|kind| {
                outcome(decide(
                    &facts(*kind, CapabilitySet::EVERY),
                    action,
                    &on.resource(),
                    &at_home(),
                ))
            })
            .collect();
        let wanted: Vec<Outcome> = PrincipalKind::ALL
            .iter()
            .zip(letters.chars())
            .map(|(kind, letter)| expected(letter, action, need, seen(*kind, CapabilitySet::EVERY)))
            .collect();
        assert_eq!(decided, wanted);
    }
}

/// Verifies: SEC-HIS-013, SEC-IAM-074
#[test]
fn every_role_holding_its_preset_gets_exactly_its_row() {
    let actions: Vec<Action> = PRESETS.iter().map(|(action, _)| *action).collect();
    assert_eq!(actions, Action::ALL);
    let roles = [
        (Role::Owner, PrincipalKind::Owner),
        (Role::Administrator, PrincipalKind::Administrator),
        (Role::Member, PrincipalKind::Member),
        (Role::Guest, PrincipalKind::Guest),
    ];
    for (action, letters) in PRESETS {
        let (on, need) = row(action);
        let decided: Vec<Outcome> = roles
            .iter()
            .map(|(role, kind)| {
                outcome(decide(
                    &facts(*kind, role.preset()),
                    action,
                    &on.resource(),
                    &at_home(),
                ))
            })
            .collect();
        let wanted: Vec<Outcome> = roles
            .iter()
            .zip(letters.chars())
            .map(|((role, kind), letter)| {
                expected(letter, action, need, seen(*kind, role.preset()))
            })
            .collect();
        assert_eq!(decided, wanted);
    }
}

/// Verifies: SEC-TM-017
#[test]
fn the_owner_needs_elevation_to_run_the_server_and_fresh_verification_for_the_host() {
    let actions: Vec<Action> = SESSIONS.iter().map(|(action, _)| *action).collect();
    assert_eq!(actions, Action::ALL);
    let states = [
        (Elevation::Ordinary, UserVerification::Stale),
        (Elevation::Elevated, UserVerification::Stale),
        (Elevation::Elevated, UserVerification::Fresh),
    ];
    for (action, letters) in SESSIONS {
        let (on, need) = row(action);
        let decided: Vec<Outcome> = states
            .iter()
            .map(|(elevation, verification)| {
                let mut owner = facts(PrincipalKind::Owner, CapabilitySet::EVERY);
                owner.elevation = *elevation;
                owner.verification = *verification;
                outcome(decide(&owner, action, &on.resource(), &at_home()))
            })
            .collect();
        let wanted: Vec<Outcome> = letters
            .chars()
            .map(|letter| expected(letter, action, need, LibrarySet::every()))
            .collect();
        assert_eq!(decided, wanted);
    }
}
