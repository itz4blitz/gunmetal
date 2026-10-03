//! Who is asking: the principal and the facts the policy decides on.
//!
//! The server's session extractor (WP-062) resolves every request to one
//! [`Principal`] and hands its [`PrincipalFacts`] to
//! [`decide`](super::decide::decide). Nothing here is secret or
//! authoritative on its own: a principal is only an input, and the only
//! value storage accepts is the [`Permit`](super::decide::Permit) the policy
//! returns.

use super::capability::{Capability, CapabilitySet, Tier};
use crate::id::PublicId;

/// The kind of a principal, from the closed set of SEC-IAM-002.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrincipalKind {
    /// The person who claimed the server. Exactly one.
    Owner,
    /// A trusted person who manages people and libraries.
    Administrator,
    /// An adult in the household.
    Member,
    /// A child, or anyone who should not hold credentials (R2).
    ManagedProfile,
    /// A friend from outside the household.
    Guest,
    /// A shared household device acting on its own (R2).
    Device,
    /// A script or third-party app holding an API key (Later).
    ApiClient,
    /// Code the owner installed (Later).
    Plugin,
    /// Another Gunmetal server (Later).
    PeerServer,
    /// Someone holding a public share link (Later).
    LinkHolder,
}

impl PrincipalKind {
    /// Every kind, in declaration order.
    pub const ALL: [Self; 10] = [
        Self::Owner,
        Self::Administrator,
        Self::Member,
        Self::ManagedProfile,
        Self::Guest,
        Self::Device,
        Self::ApiClient,
        Self::Plugin,
        Self::PeerServer,
        Self::LinkHolder,
    ];

    /// The most a principal of this kind may ever hold, whatever it was
    /// granted. Only the owner's ceiling holds the owner-only capabilities
    /// (SEC-IAM-075), and only the owner's and administrators' hold any
    /// capability that runs the server.
    #[must_use]
    pub const fn ceiling(self) -> CapabilitySet {
        match self {
            Self::Owner => CapabilitySet::EVERY,
            Self::Administrator => {
                CapabilitySet::EVERY.difference(CapabilitySet::tier(Tier::OwnerOnly))
            }
            Self::Member => CapabilitySet::of(&[
                Capability::LibraryRead,
                Capability::LibraryDownload,
                Capability::PlaylistShare,
                Capability::LibraryManage,
                Capability::InviteGuest,
                Capability::HouseholdDevice,
                Capability::HouseholdProfile,
            ]),
            Self::ManagedProfile => {
                CapabilitySet::of(&[Capability::LibraryRead, Capability::LibraryDownload])
            }
            Self::Guest | Self::ApiClient => CapabilitySet::of(&[
                Capability::LibraryRead,
                Capability::LibraryDownload,
                Capability::PlaylistShare,
            ]),
            Self::Device | Self::Plugin | Self::PeerServer | Self::LinkHolder => {
                CapabilitySet::of(&[Capability::LibraryRead])
            }
        }
    }
}

/// The class of the device a request came from (SEC-CLI-024).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceClass {
    /// One person's own phone, computer or personal-mode browser.
    Personal,
    /// A TV, a shared-mode browser or another device several people use. It
    /// may browse and play, nothing more (A-116, A-118).
    Limited,
}

impl DeviceClass {
    /// The most a request from a device of this class may do.
    #[must_use]
    pub const fn ceiling(self) -> CapabilitySet {
        match self {
            Self::Personal => CapabilitySet::EVERY,
            Self::Limited => CapabilitySet::of(&[Capability::LibraryRead, Capability::LibraryAll]),
        }
    }
}

/// Whether the session is an administrator session (SEC-IAM-041).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Elevation {
    /// An ordinary session: everyday capabilities only.
    Ordinary,
    /// A separate administrator session, created by a user-verifying
    /// sign-in.
    Elevated,
}

impl Elevation {
    /// The most a session in this state may do: an ordinary session never
    /// authorises anything that runs the server (SEC-IAM-041).
    #[must_use]
    pub const fn ceiling(self) -> CapabilitySet {
        match self {
            Self::Ordinary => CapabilitySet::tier(Tier::Everyday),
            Self::Elevated => CapabilitySet::EVERY,
        }
    }
}

/// Whether the session holds a fresh user verification: a passkey or
/// device-key assertion with user verification, made in the last five
/// minutes. The session layer (WP-062) decides this; an OIDC sign-in alone
/// never makes it fresh (SEC-IAM-107).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UserVerification {
    /// No user-verified assertion in the last five minutes.
    Stale,
    /// A user-verified assertion in the last five minutes.
    Fresh,
}

/// Where a person may use the server from. The only location rule a person
/// can carry is a restriction (SEC-API-075).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reach {
    /// From any path.
    Anywhere,
    /// Only from the server itself or the home network.
    HomeOnly,
}

/// What a credential narrower than its holder may do: an API key, a share
/// link, an invitation or a role being handed out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    /// The capabilities the credential may use.
    pub capabilities: CapabilitySet,
    /// The libraries it may see, unless its capabilities hold
    /// [`Capability::LibraryAll`].
    pub libraries: Vec<PublicId>,
}

/// Everything the policy needs to know about the principal of one request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalFacts {
    /// Its kind.
    pub kind: PrincipalKind,
    /// The account it acts for, if it has one.
    pub account: Option<PublicId>,
    /// The profile it acts as, if it has one.
    pub profile: Option<PublicId>,
    /// The capabilities its account holds: its role's preset and any it was
    /// granted.
    pub capabilities: CapabilitySet,
    /// The libraries it was granted.
    pub libraries: Vec<PublicId>,
    /// The class of the device the request came from.
    pub device: DeviceClass,
    /// Whether the session is elevated.
    pub elevation: Elevation,
    /// Whether the session holds a fresh user verification.
    pub verification: UserVerification,
    /// Where the person may use the server from.
    pub reach: Reach,
    /// The scope of the credential, when it is narrower than its holder.
    pub scope: Option<Scope>,
}

impl PrincipalFacts {
    /// The capabilities this principal may use before the action and its
    /// context are considered: what it holds, within its kind's, device's
    /// and session's ceilings and its credential's scope.
    #[must_use]
    pub fn effective(&self) -> CapabilitySet {
        let held = self
            .capabilities
            .intersection(self.kind.ceiling())
            .intersection(self.device.ceiling())
            .intersection(self.elevation.ceiling());
        match &self.scope {
            Some(scope) => held.intersection(scope.capabilities),
            None => held,
        }
    }
}

/// An opaque handle on the session a request arrived on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionHandle(pub u64);

/// The revocation epoch the session or signed URL carries (WP-062).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Epoch(pub u64);

/// The one principal a request resolved to (SEC-IAM-002): the facts the
/// policy decides on, and the handles the server keeps beside them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Principal {
    /// What the policy decides on.
    pub facts: PrincipalFacts,
    /// The device the request came from, if it is enrolled.
    pub device: Option<PublicId>,
    /// The session the request arrived on.
    pub session: SessionHandle,
    /// The revocation epoch it carries.
    pub epoch: Epoch,
}

#[cfg(test)]
mod tests {
    use super::*;
    use Capability as C;

    fn listed(set: CapabilitySet) -> Vec<Capability> {
        set.iter().collect()
    }

    #[test]
    fn the_kinds_are_the_closed_set_of_the_baseline() {
        assert_eq!(
            PrincipalKind::ALL,
            [
                PrincipalKind::Owner,
                PrincipalKind::Administrator,
                PrincipalKind::Member,
                PrincipalKind::ManagedProfile,
                PrincipalKind::Guest,
                PrincipalKind::Device,
                PrincipalKind::ApiClient,
                PrincipalKind::Plugin,
                PrincipalKind::PeerServer,
                PrincipalKind::LinkHolder,
            ]
        );
    }

    /// Verifies: SEC-IAM-075
    #[test]
    fn each_kind_has_the_ceiling_of_the_table() {
        let read = vec![C::LibraryRead];
        let ceilings: Vec<(PrincipalKind, Vec<Capability>)> = PrincipalKind::ALL
            .iter()
            .map(|kind| (*kind, listed(kind.ceiling())))
            .collect();
        assert_eq!(
            ceilings,
            [
                (PrincipalKind::Owner, Capability::ALL.to_vec()),
                (
                    PrincipalKind::Administrator,
                    vec![
                        C::LibraryRead,
                        C::LibraryAll,
                        C::LibraryDownload,
                        C::PlaylistShare,
                        C::LibraryManage,
                        C::InviteGuest,
                        C::InviteMember,
                        C::UserManage,
                        C::UserRecover,
                        C::SessionManage,
                        C::HouseholdDevice,
                        C::HouseholdProfile,
                        C::AuditRead,
                        C::ServerSettings,
                        C::HostFiles,
                    ]
                ),
                (
                    PrincipalKind::Member,
                    vec![
                        C::LibraryRead,
                        C::LibraryDownload,
                        C::PlaylistShare,
                        C::LibraryManage,
                        C::InviteGuest,
                        C::HouseholdDevice,
                        C::HouseholdProfile,
                    ]
                ),
                (
                    PrincipalKind::ManagedProfile,
                    vec![C::LibraryRead, C::LibraryDownload]
                ),
                (
                    PrincipalKind::Guest,
                    vec![C::LibraryRead, C::LibraryDownload, C::PlaylistShare]
                ),
                (PrincipalKind::Device, read.clone()),
                (
                    PrincipalKind::ApiClient,
                    vec![C::LibraryRead, C::LibraryDownload, C::PlaylistShare]
                ),
                (PrincipalKind::Plugin, read.clone()),
                (PrincipalKind::PeerServer, read.clone()),
                (PrincipalKind::LinkHolder, read),
            ]
        );
    }

    #[test]
    fn devices_and_sessions_have_the_ceilings_of_the_table() {
        assert_eq!(listed(DeviceClass::Personal.ceiling()), Capability::ALL);
        assert_eq!(
            listed(DeviceClass::Limited.ceiling()),
            [C::LibraryRead, C::LibraryAll]
        );
        assert_eq!(listed(Elevation::Elevated.ceiling()), Capability::ALL);
        assert_eq!(
            listed(Elevation::Ordinary.ceiling()),
            [
                C::LibraryRead,
                C::LibraryAll,
                C::LibraryDownload,
                C::PlaylistShare
            ]
        );
    }

    fn facts(kind: PrincipalKind) -> PrincipalFacts {
        PrincipalFacts {
            kind,
            account: None,
            profile: None,
            capabilities: CapabilitySet::EVERY,
            libraries: Vec::new(),
            device: DeviceClass::Personal,
            elevation: Elevation::Elevated,
            verification: UserVerification::Fresh,
            reach: Reach::Anywhere,
            scope: None,
        }
    }

    #[test]
    fn effective_capabilities_are_the_intersection_of_every_ceiling() {
        let mut owner = facts(PrincipalKind::Owner);
        assert_eq!(listed(owner.effective()), Capability::ALL);

        owner.capabilities = CapabilitySet::of(&[C::Backup, C::LibraryRead, C::HostFiles]);
        assert_eq!(
            listed(owner.effective()),
            [C::LibraryRead, C::HostFiles, C::Backup]
        );

        owner.elevation = Elevation::Ordinary;
        assert_eq!(listed(owner.effective()), [C::LibraryRead]);

        owner.elevation = Elevation::Elevated;
        owner.device = DeviceClass::Limited;
        assert_eq!(listed(owner.effective()), [C::LibraryRead]);

        owner.device = DeviceClass::Personal;
        owner.scope = Some(Scope {
            capabilities: CapabilitySet::of(&[C::HostFiles, C::AuditRead]),
            libraries: Vec::new(),
        });
        assert_eq!(listed(owner.effective()), [C::HostFiles]);

        let guest = facts(PrincipalKind::Guest);
        assert_eq!(
            listed(guest.effective()),
            [C::LibraryRead, C::LibraryDownload, C::PlaylistShare]
        );
    }
}
