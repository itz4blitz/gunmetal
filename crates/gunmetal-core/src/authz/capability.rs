//! Capabilities, the sets that hold them, and the role presets.
//!
//! Authorisation tests capabilities, never role names (SEC-IAM-074). A
//! [`Role`] is only a named preset that fills in a [`CapabilitySet`] when an
//! account is created or changed; the policy never sees one. The
//! capabilities follow the table in the identity baseline
//! (`docs/security/identity-and-access.md`, design guidance section 3),
//! with three additions: [`Capability::LibraryAll`], which is how the owner
//! and administrators see every library, new ones included, without a
//! per-library grant; [`Capability::OwnRead`] and [`Capability::OwnWrite`],
//! which every role's preset holds and which exist so that a credential's
//! scope has to name its holder's own data before it can reach it
//! (SEC-API-020); and [`Capability::HostFiles`] and [`Capability::Backup`],
//! which the host-equivalent actions of SEC-TM-017 need.

/// One thing a principal may be allowed to do.
///
/// The set is closed, so a capability that is not here cannot be granted,
/// and every [`Action`](super::action::Action) names at most one of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    /// `library.read`: browse and play items in the libraries the principal
    /// can see.
    LibraryRead,
    /// `library.all`: see every library, including one added after this
    /// principal's grants were set, without a per-library grant. Only the
    /// owner and administrators can hold it (A-564).
    LibraryAll,
    /// `library.download`: keep offline copies of items.
    LibraryDownload,
    /// `playlist.share`: share a playlist with other accounts on the server.
    PlaylistShare,
    /// `own.read`: read what the principal's own account and profile own:
    /// history, playlists, settings, devices and its own security log.
    OwnRead,
    /// `own.write`: change what the principal's own account and profile
    /// own.
    OwnWrite,
    /// `library.manage`: scan, edit metadata, attach lyrics or artwork.
    LibraryManage,
    /// `invite.guest`: invite a guest.
    InviteGuest,
    /// `invite.member`: invite a household member.
    InviteMember,
    /// `user.manage`: create, disable and delete people who are not
    /// administrators, and set their grants.
    UserManage,
    /// `user.recover`: give members and guests a recovery link.
    UserRecover,
    /// `session.manage`: end the sessions of anyone but the owner.
    SessionManage,
    /// `household.device`: enrol and remove household devices.
    HouseholdDevice,
    /// `household.profile`: manage managed profiles, content policy and
    /// PINs.
    HouseholdProfile,
    /// `audit.read`: read every security event.
    AuditRead,
    /// `server.settings`: scanning, playback and other settings that are not
    /// security settings.
    ServerSettings,
    /// `host.files`: add and remove library roots, browse the server's file
    /// system and see file paths (SEC-API-068, SEC-TM-017).
    HostFiles,
    /// `admin.manage`: create, demote or recover administrators. Owner only.
    AdminManage,
    /// `ownership.transfer`: hand the server to another person. Owner only.
    OwnershipTransfer,
    /// `security.settings`: origins, trusted proxies, posture and remote
    /// administration, TLS and naming, egress policy, session lifetimes,
    /// rate limits, adapters and key rotation. Owner only.
    SecuritySettings,
    /// `oidc.configure`: configure sign-in providers. Owner only.
    OidcConfigure,
    /// `plugin.approve`: install plugins and approve their grants. Owner
    /// only.
    PluginApprove,
    /// `backup`: download and restore backups. Owner only (SEC-TM-017).
    Backup,
}

/// How much a capability lets its holder do, which decides the session it
/// needs and who may ever hold it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tier {
    /// Using the libraries: an ordinary session is enough.
    Everyday,
    /// Running the server: needs an elevated session (SEC-IAM-041) and a
    /// personal device.
    Admin,
    /// Only the owner may ever hold it, and it can never be granted to
    /// anyone else (SEC-IAM-075).
    OwnerOnly,
}

impl Capability {
    /// Every capability, in declaration order.
    pub const ALL: [Self; 23] = [
        Self::LibraryRead,
        Self::LibraryAll,
        Self::LibraryDownload,
        Self::PlaylistShare,
        Self::OwnRead,
        Self::OwnWrite,
        Self::LibraryManage,
        Self::InviteGuest,
        Self::InviteMember,
        Self::UserManage,
        Self::UserRecover,
        Self::SessionManage,
        Self::HouseholdDevice,
        Self::HouseholdProfile,
        Self::AuditRead,
        Self::ServerSettings,
        Self::HostFiles,
        Self::AdminManage,
        Self::OwnershipTransfer,
        Self::SecuritySettings,
        Self::OidcConfigure,
        Self::PluginApprove,
        Self::Backup,
    ];

    /// The tier this capability belongs to.
    #[must_use]
    pub const fn tier(self) -> Tier {
        match self {
            Self::LibraryRead
            | Self::LibraryAll
            | Self::LibraryDownload
            | Self::PlaylistShare
            | Self::OwnRead
            | Self::OwnWrite => Tier::Everyday,
            Self::LibraryManage
            | Self::InviteGuest
            | Self::InviteMember
            | Self::UserManage
            | Self::UserRecover
            | Self::SessionManage
            | Self::HouseholdDevice
            | Self::HouseholdProfile
            | Self::AuditRead
            | Self::ServerSettings
            | Self::HostFiles => Tier::Admin,
            Self::AdminManage
            | Self::OwnershipTransfer
            | Self::SecuritySettings
            | Self::OidcConfigure
            | Self::PluginApprove
            | Self::Backup => Tier::OwnerOnly,
        }
    }

    /// The one bit that stands for this capability in a [`CapabilitySet`].
    const fn bit(self) -> u32 {
        // A fieldless enum's discriminant is its declaration index, below
        // 23, so the shift stays inside 32 bits.
        1_u32.wrapping_shl(self as u32)
    }
}

/// A set of capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CapabilitySet(u32);

impl CapabilitySet {
    /// The set that holds nothing.
    pub const EMPTY: Self = Self(0);

    /// The set of every capability.
    pub const EVERY: Self = Self::of(&Capability::ALL);

    /// The capabilities of one tier.
    #[must_use]
    pub const fn tier(tier: Tier) -> Self {
        let mut set = Self::EMPTY;
        let mut rest: &[Capability] = &Capability::ALL;
        while let [first, tail @ ..] = rest {
            if first.tier() as u8 == tier as u8 {
                set = set.with(*first);
            }
            rest = tail;
        }
        set
    }

    /// The set holding exactly `capabilities`.
    #[must_use]
    pub const fn of(capabilities: &[Capability]) -> Self {
        let mut set = Self::EMPTY;
        let mut rest = capabilities;
        while let [first, tail @ ..] = rest {
            set = set.with(*first);
            rest = tail;
        }
        set
    }

    /// This set with `capability` added.
    #[must_use]
    pub const fn with(self, capability: Capability) -> Self {
        Self(self.0 | capability.bit())
    }

    /// Whether this set holds `capability`.
    #[must_use]
    pub const fn contains(self, capability: Capability) -> bool {
        self.0 & capability.bit() != 0
    }

    /// The capabilities in both sets.
    #[must_use]
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// The capabilities in either set.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// The capabilities in this set that are not in `other`.
    #[must_use]
    pub const fn difference(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    /// Whether every capability in this set is also in `other`.
    #[must_use]
    pub const fn is_subset(self, other: Self) -> bool {
        self.0 & !other.0 == 0
    }

    /// The capabilities in this set, in declaration order.
    pub fn iter(self) -> impl Iterator<Item = Capability> {
        Capability::ALL
            .into_iter()
            .filter(move |capability| self.contains(*capability))
    }
}

/// A named preset of capabilities: what an account of that role starts
/// with.
///
/// The policy never takes a role. Code that sets up or changes an account
/// reads the role's preset and stores the capabilities, and the policy tests
/// those (SEC-IAM-074).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// The person who claimed the server.
    Owner,
    /// A trusted person who manages people and libraries.
    Administrator,
    /// An adult in the household.
    Member,
    /// A friend from outside the household.
    Guest,
}

impl Role {
    /// Every role.
    pub const ALL: [Self; 4] = [Self::Owner, Self::Administrator, Self::Member, Self::Guest];

    /// The capabilities an account of this role starts with. A member and a
    /// guest start with no management capability (SEC-HIS-013); which
    /// libraries they see comes from their grants. Every role reads and
    /// changes its own data.
    #[must_use]
    pub const fn preset(self) -> CapabilitySet {
        match self {
            Self::Owner => CapabilitySet::EVERY,
            Self::Administrator => {
                CapabilitySet::EVERY.difference(CapabilitySet::tier(Tier::OwnerOnly))
            }
            Self::Member => CapabilitySet::of(&[
                Capability::LibraryRead,
                Capability::LibraryDownload,
                Capability::PlaylistShare,
                Capability::OwnRead,
                Capability::OwnWrite,
            ]),
            Self::Guest => CapabilitySet::of(&[
                Capability::LibraryRead,
                Capability::OwnRead,
                Capability::OwnWrite,
            ]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use proptest::sample::subsequence;

    use Capability as C;

    /// The tiers, written out independently of [`Capability::tier`].
    const TIERS: [(Capability, Tier); 23] = [
        (C::LibraryRead, Tier::Everyday),
        (C::LibraryAll, Tier::Everyday),
        (C::LibraryDownload, Tier::Everyday),
        (C::PlaylistShare, Tier::Everyday),
        (C::OwnRead, Tier::Everyday),
        (C::OwnWrite, Tier::Everyday),
        (C::LibraryManage, Tier::Admin),
        (C::InviteGuest, Tier::Admin),
        (C::InviteMember, Tier::Admin),
        (C::UserManage, Tier::Admin),
        (C::UserRecover, Tier::Admin),
        (C::SessionManage, Tier::Admin),
        (C::HouseholdDevice, Tier::Admin),
        (C::HouseholdProfile, Tier::Admin),
        (C::AuditRead, Tier::Admin),
        (C::ServerSettings, Tier::Admin),
        (C::HostFiles, Tier::Admin),
        (C::AdminManage, Tier::OwnerOnly),
        (C::OwnershipTransfer, Tier::OwnerOnly),
        (C::SecuritySettings, Tier::OwnerOnly),
        (C::OidcConfigure, Tier::OwnerOnly),
        (C::PluginApprove, Tier::OwnerOnly),
        (C::Backup, Tier::OwnerOnly),
    ];

    /// Membership written as a plain list, the oracle for the bit set.
    fn listed(set: CapabilitySet) -> Vec<Capability> {
        set.iter().collect()
    }

    fn capabilities() -> impl Strategy<Value = Vec<Capability>> {
        subsequence(Capability::ALL.to_vec(), 0..=Capability::ALL.len())
    }

    #[test]
    fn every_capability_has_its_tier() {
        let tiers: Vec<(Capability, Tier)> = Capability::ALL
            .iter()
            .map(|capability| (*capability, capability.tier()))
            .collect();
        assert_eq!(tiers, TIERS);
    }

    #[test]
    fn each_tier_set_holds_exactly_its_capabilities() {
        assert_eq!(
            listed(CapabilitySet::tier(Tier::Everyday)),
            [
                C::LibraryRead,
                C::LibraryAll,
                C::LibraryDownload,
                C::PlaylistShare,
                C::OwnRead,
                C::OwnWrite,
            ]
        );
        assert_eq!(
            listed(CapabilitySet::tier(Tier::Admin)),
            [
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
        );
        assert_eq!(
            listed(CapabilitySet::tier(Tier::OwnerOnly)),
            [
                C::AdminManage,
                C::OwnershipTransfer,
                C::SecuritySettings,
                C::OidcConfigure,
                C::PluginApprove,
                C::Backup,
            ]
        );
    }

    #[test]
    fn the_empty_and_full_sets_hold_nothing_and_everything() {
        assert_eq!(listed(CapabilitySet::EMPTY), []);
        assert_eq!(listed(CapabilitySet::EVERY), Capability::ALL);
    }

    #[test]
    fn a_set_of_one_holds_only_that_one() {
        for capability in Capability::ALL {
            assert_eq!(listed(CapabilitySet::of(&[capability])), [capability]);
        }
    }

    /// Verifies: SEC-HIS-013, SEC-IAM-075
    #[test]
    fn the_presets_hold_exactly_the_capabilities_of_the_table() {
        assert_eq!(listed(Role::Owner.preset()), Capability::ALL);
        assert_eq!(
            listed(Role::Administrator.preset()),
            [
                C::LibraryRead,
                C::LibraryAll,
                C::LibraryDownload,
                C::PlaylistShare,
                C::OwnRead,
                C::OwnWrite,
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
        );
        assert_eq!(
            listed(Role::Member.preset()),
            [
                C::LibraryRead,
                C::LibraryDownload,
                C::PlaylistShare,
                C::OwnRead,
                C::OwnWrite,
            ]
        );
        assert_eq!(
            listed(Role::Guest.preset()),
            [C::LibraryRead, C::OwnRead, C::OwnWrite]
        );
        assert_eq!(
            Role::ALL,
            [Role::Owner, Role::Administrator, Role::Member, Role::Guest]
        );
    }

    proptest! {
        #[test]
        fn a_set_holds_exactly_what_it_was_built_from(list in capabilities()) {
            prop_assert_eq!(listed(CapabilitySet::of(&list)), list);
        }

        #[test]
        fn set_operations_agree_with_list_operations(a in capabilities(), b in capabilities()) {
            let (left, right) = (CapabilitySet::of(&a), CapabilitySet::of(&b));
            let both: Vec<Capability> = a.iter().copied().filter(|c| b.contains(c)).collect();
            let either: Vec<Capability> = Capability::ALL
                .into_iter()
                .filter(|c| a.contains(c) || b.contains(c))
                .collect();
            let only_left: Vec<Capability> = a.iter().copied().filter(|c| !b.contains(c)).collect();
            prop_assert_eq!(listed(left.intersection(right)), both);
            prop_assert_eq!(listed(left.union(right)), either);
            prop_assert_eq!(listed(left.difference(right)), only_left.clone());
            prop_assert_eq!(left.is_subset(right), only_left.is_empty());
            for capability in Capability::ALL {
                prop_assert_eq!(left.contains(capability), a.contains(&capability));
                let added: Vec<Capability> = Capability::ALL
                    .into_iter()
                    .filter(|c| a.contains(c) || *c == capability)
                    .collect();
                prop_assert_eq!(listed(left.with(capability)), added);
            }
        }
    }
}
