//! The closed list of actions, and the rule each one is decided by.
//!
//! [`Action::rule`] matches every action exhaustively, so an action added
//! here does not compile until someone writes down what it needs
//! (SEC-IAM-068, SEC-TM-005). An action named by a client that is not in the
//! list, such as an unknown message type, is refused by
//! [`Action::from_name`] before any policy runs.

use super::capability::Capability;
use super::decide::Denial;
use super::principal::PrincipalKind;

/// Something a principal asks to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// End one's own session.
    SignOut,
    /// Read one's own data: history, playlists, settings, devices and one's
    /// own security log.
    ReadOwnData,
    /// Change one's own data.
    WriteOwnData,
    /// Add or remove one's own passkeys, device keys and API keys.
    ManageOwnCredentials,
    /// Browse or search the libraries, or open one item.
    BrowseLibrary,
    /// Stream an item.
    StreamMedia,
    /// Keep an offline copy of an item.
    DownloadMedia,
    /// Share one of one's own playlists with other accounts.
    SharePlaylist,
    /// Scan a library or edit its metadata, lyrics or artwork.
    ManageLibrary,
    /// Invite a guest.
    InviteGuest,
    /// Invite a household member.
    InviteMember,
    /// Create, disable or delete a person who is not an administrator, or
    /// set their grants.
    ManageUser,
    /// Give a member or a guest a recovery link.
    RecoverUser,
    /// End another person's sessions.
    EndSessions,
    /// Enrol or remove a household device.
    ManageHouseholdDevices,
    /// Manage managed profiles, content policy and PINs.
    ManageHouseholdProfiles,
    /// Read the security audit log.
    ReadAuditLog,
    /// Change scanning, playback and other non-security settings.
    ChangeServerSettings,
    /// See an item's file path and file details.
    InspectFile,
    /// Add, remove or relocate a library root.
    ManageLibraryRoots,
    /// Browse the server's file system.
    BrowseFileSystem,
    /// Create, promote, demote or recover an administrator.
    ManageAdministrators,
    /// Hand ownership of the server to another person.
    TransferOwnership,
    /// Change trusted proxies, posture and remote administration, TLS and
    /// naming, egress policy, adapters, session lifetimes or rate limits.
    ChangeSecuritySettings,
    /// Rotate every server key.
    RotateKeys,
    /// Configure the sign-in providers.
    ConfigureOidc,
    /// Install a plugin or approve its grants.
    ApprovePlugin,
    /// Download a backup.
    DownloadBackup,
    /// Restore a backup.
    RestoreBackup,
}

/// The actions that act on the host itself. Each needs a fresh user
/// verification (SEC-IAM-041, SEC-TM-017); all but the two that touch
/// library roots and the file system are owner-only, because their
/// capabilities are.
pub const HOST_EQUIVALENT: &[Action] = &[
    Action::ManageLibraryRoots,
    Action::BrowseFileSystem,
    Action::ManageAdministrators,
    Action::TransferOwnership,
    Action::ChangeSecuritySettings,
    Action::RotateKeys,
    Action::ConfigureOidc,
    Action::ApprovePlugin,
    Action::DownloadBackup,
    Action::RestoreBackup,
];

/// What an action needs before anything else is considered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Need {
    /// Nothing: any principal may ask.
    Nothing,
    /// This capability.
    Capability(Capability),
}

/// What an action acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    /// The server as a whole, or a listing filtered by the permit.
    Server,
    /// An item in a library, or a listing filtered by the permit.
    Library,
    /// Something the principal's own account or profile owns.
    Own,
    /// A person of one of these kinds.
    Person(&'static [PrincipalKind]),
}

/// How one action is decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rule {
    /// What it needs.
    pub need: Need,
    /// What it acts on.
    pub target: Target,
    /// Whether it creates or changes a credential, which a scoped
    /// credential may never do (SEC-API-020).
    pub credential: bool,
}

/// The people `user.manage` acts on: never an administrator or the owner.
const NON_ADMINS: &[PrincipalKind] = &[
    PrincipalKind::Member,
    PrincipalKind::ManagedProfile,
    PrincipalKind::Guest,
];

/// The people `user.recover` acts on: members and guests. A managed profile
/// holds no credential to recover, and an administrator is recovered by the
/// owner alone.
const RECOVERABLE: &[PrincipalKind] = &[PrincipalKind::Member, PrincipalKind::Guest];

/// The people `session.manage` acts on: anyone but the owner.
const NON_OWNERS: &[PrincipalKind] = &[
    PrincipalKind::Administrator,
    PrincipalKind::Member,
    PrincipalKind::ManagedProfile,
    PrincipalKind::Guest,
    PrincipalKind::Device,
];

/// The people the owner may make or unmake administrators, or hand the
/// server to.
const ADULTS: &[PrincipalKind] = &[PrincipalKind::Administrator, PrincipalKind::Member];

impl Action {
    /// Every action, in declaration order.
    pub const ALL: [Self; 29] = [
        Self::SignOut,
        Self::ReadOwnData,
        Self::WriteOwnData,
        Self::ManageOwnCredentials,
        Self::BrowseLibrary,
        Self::StreamMedia,
        Self::DownloadMedia,
        Self::SharePlaylist,
        Self::ManageLibrary,
        Self::InviteGuest,
        Self::InviteMember,
        Self::ManageUser,
        Self::RecoverUser,
        Self::EndSessions,
        Self::ManageHouseholdDevices,
        Self::ManageHouseholdProfiles,
        Self::ReadAuditLog,
        Self::ChangeServerSettings,
        Self::InspectFile,
        Self::ManageLibraryRoots,
        Self::BrowseFileSystem,
        Self::ManageAdministrators,
        Self::TransferOwnership,
        Self::ChangeSecuritySettings,
        Self::RotateKeys,
        Self::ConfigureOidc,
        Self::ApprovePlugin,
        Self::DownloadBackup,
        Self::RestoreBackup,
    ];

    /// The action's stable name, as route specifications and message types
    /// spell it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::SignOut => "session.sign_out",
            Self::ReadOwnData => "own.read",
            Self::WriteOwnData => "own.write",
            Self::ManageOwnCredentials => "own.credentials",
            Self::BrowseLibrary => "library.browse",
            Self::StreamMedia => "library.stream",
            Self::DownloadMedia => "library.download",
            Self::SharePlaylist => "playlist.share",
            Self::ManageLibrary => "library.manage",
            Self::InviteGuest => "invite.guest",
            Self::InviteMember => "invite.member",
            Self::ManageUser => "user.manage",
            Self::RecoverUser => "user.recover",
            Self::EndSessions => "session.end",
            Self::ManageHouseholdDevices => "household.device",
            Self::ManageHouseholdProfiles => "household.profile",
            Self::ReadAuditLog => "audit.read",
            Self::ChangeServerSettings => "server.settings",
            Self::InspectFile => "host.inspect",
            Self::ManageLibraryRoots => "host.roots",
            Self::BrowseFileSystem => "host.browse",
            Self::ManageAdministrators => "admin.manage",
            Self::TransferOwnership => "ownership.transfer",
            Self::ChangeSecuritySettings => "security.settings",
            Self::RotateKeys => "security.keys",
            Self::ConfigureOidc => "oidc.configure",
            Self::ApprovePlugin => "plugin.approve",
            Self::DownloadBackup => "backup.download",
            Self::RestoreBackup => "backup.restore",
        }
    }

    /// The action named `name`.
    ///
    /// # Errors
    ///
    /// Returns [`Denial::UnknownAction`] for any name that is not exactly
    /// one action's name, so an unknown action is refused (SEC-TM-005).
    pub fn from_name(name: &str) -> Result<Self, Denial> {
        Self::ALL
            .into_iter()
            .find(|action| action.name() == name)
            .ok_or(Denial::UnknownAction)
    }

    /// How this action is decided.
    pub(crate) const fn rule(self) -> Rule {
        const fn rule(need: Need, target: Target, credential: bool) -> Rule {
            Rule {
                need,
                target,
                credential,
            }
        }
        const fn needs(capability: Capability) -> Need {
            Need::Capability(capability)
        }
        use Capability as C;
        match self {
            Self::SignOut => rule(Need::Nothing, Target::Server, false),
            Self::ReadOwnData | Self::WriteOwnData => rule(Need::Nothing, Target::Own, false),
            Self::ManageOwnCredentials => rule(Need::Nothing, Target::Own, true),
            Self::BrowseLibrary | Self::StreamMedia => {
                rule(needs(C::LibraryRead), Target::Library, false)
            }
            Self::DownloadMedia => rule(needs(C::LibraryDownload), Target::Library, false),
            Self::SharePlaylist => rule(needs(C::PlaylistShare), Target::Own, false),
            Self::ManageLibrary => rule(needs(C::LibraryManage), Target::Library, false),
            Self::InviteGuest => rule(needs(C::InviteGuest), Target::Server, true),
            Self::InviteMember => rule(needs(C::InviteMember), Target::Server, true),
            Self::ManageUser => rule(needs(C::UserManage), Target::Person(NON_ADMINS), false),
            Self::RecoverUser => rule(needs(C::UserRecover), Target::Person(RECOVERABLE), true),
            Self::EndSessions => rule(needs(C::SessionManage), Target::Person(NON_OWNERS), false),
            Self::ManageHouseholdDevices => rule(needs(C::HouseholdDevice), Target::Server, true),
            Self::ManageHouseholdProfiles => {
                rule(needs(C::HouseholdProfile), Target::Server, false)
            }
            Self::ReadAuditLog => rule(needs(C::AuditRead), Target::Server, false),
            Self::ChangeServerSettings => rule(needs(C::ServerSettings), Target::Server, false),
            Self::InspectFile => rule(needs(C::HostFiles), Target::Library, false),
            Self::ManageLibraryRoots | Self::BrowseFileSystem => {
                rule(needs(C::HostFiles), Target::Server, false)
            }
            Self::ManageAdministrators => rule(needs(C::AdminManage), Target::Person(ADULTS), true),
            Self::TransferOwnership => {
                rule(needs(C::OwnershipTransfer), Target::Person(ADULTS), false)
            }
            Self::ChangeSecuritySettings | Self::RotateKeys => {
                rule(needs(C::SecuritySettings), Target::Server, false)
            }
            Self::ConfigureOidc => rule(needs(C::OidcConfigure), Target::Server, false),
            Self::ApprovePlugin => rule(needs(C::PluginApprove), Target::Server, false),
            Self::DownloadBackup | Self::RestoreBackup => {
                rule(needs(C::Backup), Target::Server, false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every action's name, written out independently of [`Action::name`].
    const NAMES: [(Action, &str); 29] = [
        (Action::SignOut, "session.sign_out"),
        (Action::ReadOwnData, "own.read"),
        (Action::WriteOwnData, "own.write"),
        (Action::ManageOwnCredentials, "own.credentials"),
        (Action::BrowseLibrary, "library.browse"),
        (Action::StreamMedia, "library.stream"),
        (Action::DownloadMedia, "library.download"),
        (Action::SharePlaylist, "playlist.share"),
        (Action::ManageLibrary, "library.manage"),
        (Action::InviteGuest, "invite.guest"),
        (Action::InviteMember, "invite.member"),
        (Action::ManageUser, "user.manage"),
        (Action::RecoverUser, "user.recover"),
        (Action::EndSessions, "session.end"),
        (Action::ManageHouseholdDevices, "household.device"),
        (Action::ManageHouseholdProfiles, "household.profile"),
        (Action::ReadAuditLog, "audit.read"),
        (Action::ChangeServerSettings, "server.settings"),
        (Action::InspectFile, "host.inspect"),
        (Action::ManageLibraryRoots, "host.roots"),
        (Action::BrowseFileSystem, "host.browse"),
        (Action::ManageAdministrators, "admin.manage"),
        (Action::TransferOwnership, "ownership.transfer"),
        (Action::ChangeSecuritySettings, "security.settings"),
        (Action::RotateKeys, "security.keys"),
        (Action::ConfigureOidc, "oidc.configure"),
        (Action::ApprovePlugin, "plugin.approve"),
        (Action::DownloadBackup, "backup.download"),
        (Action::RestoreBackup, "backup.restore"),
    ];

    /// The position of `action` in [`Action::ALL`]. The match is
    /// exhaustive, so a new action does not compile until it is placed here,
    /// and the test below then fails until it is in `ALL` and `NAMES` too.
    const fn position(action: Action) -> usize {
        match action {
            Action::SignOut => 0,
            Action::ReadOwnData => 1,
            Action::WriteOwnData => 2,
            Action::ManageOwnCredentials => 3,
            Action::BrowseLibrary => 4,
            Action::StreamMedia => 5,
            Action::DownloadMedia => 6,
            Action::SharePlaylist => 7,
            Action::ManageLibrary => 8,
            Action::InviteGuest => 9,
            Action::InviteMember => 10,
            Action::ManageUser => 11,
            Action::RecoverUser => 12,
            Action::EndSessions => 13,
            Action::ManageHouseholdDevices => 14,
            Action::ManageHouseholdProfiles => 15,
            Action::ReadAuditLog => 16,
            Action::ChangeServerSettings => 17,
            Action::InspectFile => 18,
            Action::ManageLibraryRoots => 19,
            Action::BrowseFileSystem => 20,
            Action::ManageAdministrators => 21,
            Action::TransferOwnership => 22,
            Action::ChangeSecuritySettings => 23,
            Action::RotateKeys => 24,
            Action::ConfigureOidc => 25,
            Action::ApprovePlugin => 26,
            Action::DownloadBackup => 27,
            Action::RestoreBackup => 28,
        }
    }

    #[test]
    fn every_action_is_listed_once_in_order() {
        let positions: Vec<usize> = Action::ALL.iter().map(|action| position(*action)).collect();
        let expected: Vec<usize> = (0..29).collect();
        assert_eq!(positions, expected);
    }

    #[test]
    fn every_action_has_its_name_and_is_found_by_it() {
        let named: Vec<(Action, &str)> = Action::ALL
            .iter()
            .map(|action| (*action, action.name()))
            .collect();
        assert_eq!(named, NAMES);
        for (action, name) in NAMES {
            assert_eq!(Action::from_name(name), Ok(action));
        }
    }

    /// Verifies: SEC-TM-005
    #[test]
    fn an_unknown_action_is_denied() {
        let unknown = [
            "",
            "library",
            "library.browse ",
            "Library.Browse",
            "library.browse\0",
            "library.delete",
            "admin",
            "*",
        ];
        for name in unknown {
            assert_eq!(Action::from_name(name), Err(Denial::UnknownAction));
        }
    }

    /// Verifies: SEC-TM-017
    #[test]
    fn the_host_equivalent_actions_are_those_of_the_baseline() {
        assert_eq!(
            HOST_EQUIVALENT,
            [
                Action::ManageLibraryRoots,
                Action::BrowseFileSystem,
                Action::ManageAdministrators,
                Action::TransferOwnership,
                Action::ChangeSecuritySettings,
                Action::RotateKeys,
                Action::ConfigureOidc,
                Action::ApprovePlugin,
                Action::DownloadBackup,
                Action::RestoreBackup,
            ]
        );
    }
}
