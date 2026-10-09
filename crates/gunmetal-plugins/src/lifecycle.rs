//! Install, update and developer-mode decisions.
//!
//! Signature verification is an input. This module does not check a key.
//! A failed install leaves the caller's installed record untouched
//! (SEC-EXT-035). An update stays inactive until the owner approves it,
//! except an opted-in same-permission update from the same publisher after
//! 72 hours (SEC-EXT-040). Developer mode cannot be switched on from an API
//! (SEC-EXT-037). Install, update, grant, remove and adding an index are
//! owner actions, and adding an index also needs a confirmed fingerprint
//! (SEC-EXT-038).

use crate::version::PluginVersion;

/// Seventy-two hours, in milliseconds. Automatic same-permission updates
/// wait at least this long after the index published them.
const AUTO_HOLD_MS: i64 = 72 * 3_600 * 1_000;

/// The result of a signature check the caller already performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signature {
    /// The signature verifies against the delegated publisher key.
    Verified,
    /// The package has no signature.
    Missing,
    /// The signature does not verify.
    Rejected,
    /// The signing key is not delegated for this plugin.
    NotDelegated,
}

/// The package that is installed now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    /// The installed version.
    pub version: PluginVersion,
    /// The installed package hash.
    pub hash: [u8; 32],
    /// Whether it was accepted without a signature, in developer mode.
    pub unsigned: bool,
}

/// A package offered for install, with the checks the caller already made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Candidate {
    /// The version in the package.
    pub version: PluginVersion,
    /// The package hash to record if the install is accepted.
    pub hash: [u8; 32],
    /// Whether the package length matches the index target.
    pub length_matches: bool,
    /// Whether the package version matches the index target.
    pub index_version_matches: bool,
    /// The signature check the caller performed.
    pub signature: Signature,
    /// Whether developer mode is on in the configuration file.
    pub developer_mode: bool,
}

/// Why a candidate was not installed. The previous install stays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallError {
    /// The package has no signature.
    SignatureMissing,
    /// The signature does not verify.
    SignatureRejected,
    /// The signing key is not delegated for this plugin.
    NotDelegated,
    /// The package hash does not match the signed index target.
    HashMismatch,
    /// The package length does not match the index target.
    LengthMismatch,
    /// The package version does not match the index target.
    VersionMismatch,
    /// An unsigned package needs developer mode, which only the configuration
    /// file can set.
    UnsignedNeedsDeveloperMode,
}

/// Accepts a candidate, or refuses it without writing `current`.
///
/// A verified signature, a matching length and a matching index version
/// install as a signed package. A missing signature installs only in
/// developer mode, and then as unsigned. A rejected or undelegated signature
/// is refused even in developer mode. Signature is checked before length,
/// and length before version.
///
/// # Errors
///
/// [`InstallError::SignatureRejected`] when the signature is rejected,
/// [`InstallError::NotDelegated`] when the key is not delegated,
/// [`InstallError::UnsignedNeedsDeveloperMode`] when the signature is missing
/// and developer mode is off, [`InstallError::LengthMismatch`] when the
/// length does not match, and [`InstallError::VersionMismatch`] when the
/// version does not match.
#[must_use = "a refused install must not replace the installed package"]
pub const fn install(
    current: Option<&Installed>,
    candidate: &Candidate,
) -> Result<Installed, InstallError> {
    // Borrowed so this function cannot replace the installed package. On any
    // error the caller keeps `current`.
    let _ = current;
    let unsigned = match candidate.signature {
        Signature::Rejected => return Err(InstallError::SignatureRejected),
        Signature::NotDelegated => return Err(InstallError::NotDelegated),
        Signature::Missing => {
            if candidate.developer_mode {
                true
            } else {
                return Err(InstallError::UnsignedNeedsDeveloperMode);
            }
        }
        Signature::Verified => false,
    };
    if !candidate.length_matches {
        return Err(InstallError::LengthMismatch);
    }
    if !candidate.index_version_matches {
        return Err(InstallError::VersionMismatch);
    }
    Ok(Installed {
        version: candidate.version,
        hash: candidate.hash,
        unsigned,
    })
}

/// How the offered permissions compare with the granted set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionChange {
    /// Equal or narrower. Nothing new is asked.
    SameOrNarrower,
    /// A permission was added or widened.
    Wider,
}

/// An update the index is offering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateOffer {
    /// The version that is installed.
    pub current: PluginVersion,
    /// The version the index is offering.
    pub offered: PluginVersion,
    /// How the offered permissions compare with the granted set.
    pub change: PermissionChange,
    /// Whether the offered package is signed by the same publisher key.
    pub same_publisher: bool,
    /// When the index published the offer, in milliseconds.
    pub published_at_ms: i64,
    /// The clock the caller injects, in milliseconds.
    pub now_ms: i64,
    /// Whether this plugin opted in to automatic same-permission updates.
    pub auto_same_permission: bool,
    /// Whether the owner has approved this offer.
    pub owner_approved: bool,
}

/// Whether to keep the running version or install the offer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateDecision {
    /// Leave the current version running.
    KeepCurrent,
    /// Install the offered version.
    InstallOffered,
}

/// Why an update is refused rather than held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateError {
    /// The offered version is lower than the installed one.
    Downgrade,
}

/// Decides whether an offered version may replace the current one.
///
/// A lower version is refused. A wider permission set installs only when the
/// owner has approved it. The same or a narrower set installs when the owner
/// has approved it, or when automatic same-permission updates are on, the
/// publisher is the same, and at least 72 hours have passed since publication.
///
/// # Errors
///
/// [`UpdateError::Downgrade`] when `offered` is lower than `current`. Approval,
/// the automatic opt-in and the elapsed hold do not make a downgrade legal.
#[must_use = "a held or refused update must not replace the running version"]
pub fn decide_update(offer: &UpdateOffer) -> Result<UpdateDecision, UpdateError> {
    if offer.offered < offer.current {
        return Err(UpdateError::Downgrade);
    }
    let install = match offer.change {
        PermissionChange::Wider => offer.owner_approved,
        PermissionChange::SameOrNarrower => offer.owner_approved || hold_elapsed(offer),
    };
    Ok(if install {
        UpdateDecision::InstallOffered
    } else {
        UpdateDecision::KeepCurrent
    })
}

/// Whether an opted-in same-publisher update has waited 72 hours.
///
/// A publication time that cannot be added to the hold has not elapsed. A
/// wrapping add would treat a far-future publication as already ready.
const fn hold_elapsed(offer: &UpdateOffer) -> bool {
    if !offer.auto_same_permission || !offer.same_publisher {
        return false;
    }
    match offer.published_at_ms.checked_add(AUTO_HOLD_MS) {
        Some(ready) => offer.now_ms >= ready,
        None => false,
    }
}

/// Who is asking to change a plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Actor {
    /// The owner, with a fresh verification.
    OwnerFresh,
    /// The owner, with a stale session.
    OwnerStale,
    /// Anyone who is not the owner.
    Member,
}

/// An owner-only plugin action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerAction {
    /// Install a plugin.
    Install,
    /// Update a plugin.
    Update,
    /// Grant a permission.
    Grant,
    /// Remove a plugin.
    Remove,
    /// Add a plugin index.
    AddIndex,
}

/// Why the action is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionError {
    /// The actor is not the owner.
    NotOwner,
    /// The owner session is not fresh.
    Stale,
    /// Adding an index needs the root-key fingerprint confirmed.
    FingerprintMissing,
}

/// Whether this actor may perform the action.
///
/// A member is not the owner. A stale owner session is not fresh. Adding an
/// index also needs the root-key fingerprint confirmed. Install, update,
/// grant and remove do not.
///
/// # Errors
///
/// [`ActionError::NotOwner`] for a member, [`ActionError::Stale`] for a stale
/// owner session, and [`ActionError::FingerprintMissing`] when a fresh owner
/// adds an index without confirming the fingerprint.
#[must_use = "a refusal must not be treated as permission"]
pub const fn authorise(
    actor: Actor,
    action: OwnerAction,
    fingerprint_confirmed: bool,
) -> Result<(), ActionError> {
    match actor {
        Actor::Member => Err(ActionError::NotOwner),
        Actor::OwnerStale => Err(ActionError::Stale),
        Actor::OwnerFresh => match action {
            OwnerAction::AddIndex if !fingerprint_confirmed => Err(ActionError::FingerprintMissing),
            OwnerAction::Install
            | OwnerAction::Update
            | OwnerAction::Grant
            | OwnerAction::Remove
            | OwnerAction::AddIndex => Ok(()),
        },
    }
}

/// Why developer mode cannot be enabled from here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeveloperRefuse {
    /// Developer mode is set only in the configuration file.
    FileOnly,
}

/// Refuses to switch developer mode on from an API.
///
/// There is no success path. The flag lives in the configuration file.
///
/// # Errors
///
/// Always [`DeveloperRefuse::FileOnly`].
#[must_use = "the refusal is the only result; there is no success path"]
pub const fn enable_from_api() -> Result<bool, DeveloperRefuse> {
    Err(DeveloperRefuse::FileOnly)
}

/// The banner every client shows while developer mode is on and an unsigned
/// plugin is installed.
///
/// Names stay in the order given, separated by `", "`.
#[must_use = "dropping the banner hides unsigned plugins from every client"]
pub fn banner(developer_mode: bool, unsigned: &[&str]) -> Option<String> {
    if !developer_mode {
        return None;
    }
    let (first, rest) = unsigned.split_first()?;
    let mut text = String::from("Unsigned plugins: ");
    text.push_str(first);
    for name in rest {
        text.push_str(", ");
        text.push_str(name);
    }
    Some(text)
}

#[cfg(test)]
mod tests {
    use super::{
        ActionError, Actor, Candidate, DeveloperRefuse, InstallError, Installed, OwnerAction,
        PermissionChange, Signature, UpdateDecision, UpdateError, UpdateOffer, authorise, banner,
        decide_update, enable_from_api, install,
    };
    use crate::version::PluginVersion;

    const OLD_HASH: [u8; 32] = [
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e,
        0x1f, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d,
        0x2e, 0x2f,
    ];
    const NEW_HASH: [u8; 32] = [
        0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xab, 0xac, 0xad, 0xae,
        0xaf, 0xb0, 0xb1, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xbb, 0xbc, 0xbd,
        0xbe, 0xbf,
    ];

    fn version(text: &str) -> PluginVersion {
        PluginVersion::parse(text).expect(text)
    }

    fn installed(text: &str, hash: [u8; 32], unsigned: bool) -> Installed {
        Installed {
            version: version(text),
            hash,
            unsigned,
        }
    }

    fn candidate(
        text: &str,
        hash: [u8; 32],
        signature: Signature,
        developer_mode: bool,
    ) -> Candidate {
        Candidate {
            version: version(text),
            hash,
            length_matches: true,
            index_version_matches: true,
            signature,
            developer_mode,
        }
    }

    fn assert_install(
        current: &Installed,
        candidate: &Candidate,
        expected: &Result<Installed, InstallError>,
    ) {
        let snapshot = current.clone();
        assert_eq!(&install(Some(current), candidate), expected);
        assert_eq!(current, &snapshot);
        assert_eq!(&install(None, candidate), expected);
    }

    fn offer(current: &str, offered: &str) -> UpdateOffer {
        UpdateOffer {
            current: version(current),
            offered: version(offered),
            change: PermissionChange::SameOrNarrower,
            same_publisher: true,
            published_at_ms: 0,
            now_ms: 0,
            auto_same_permission: false,
            owner_approved: false,
        }
    }

    fn actions() -> [OwnerAction; 5] {
        [
            OwnerAction::Install,
            OwnerAction::Update,
            OwnerAction::Grant,
            OwnerAction::Remove,
            OwnerAction::AddIndex,
        ]
    }

    /// Verifies: SEC-EXT-035
    #[test]
    fn a_verified_match_installs_signed_and_leaves_the_current_record() {
        let current = installed("1.0.0", OLD_HASH, true);
        let verified = candidate("1.2.1", NEW_HASH, Signature::Verified, false);
        assert_install(
            &current,
            &verified,
            &Ok(installed("1.2.1", NEW_HASH, false)),
        );
        let mut in_developer_mode = verified;
        in_developer_mode.developer_mode = true;
        assert_install(
            &current,
            &in_developer_mode,
            &Ok(installed("1.2.1", NEW_HASH, false)),
        );
        let mut zero_hash = verified;
        zero_hash.hash = [0; 32];
        zero_hash.version = version("0.0.0");
        assert_install(
            &current,
            &zero_hash,
            &Ok(installed("0.0.0", [0; 32], false)),
        );
    }

    /// Verifies: SEC-EXT-035
    #[test]
    fn a_missing_signature_needs_developer_mode_even_when_hashes_match() {
        let current = installed("1.2.0", OLD_HASH, false);
        let mut matching = candidate("1.2.1", OLD_HASH, Signature::Missing, false);
        assert_install(
            &current,
            &matching,
            &Err(InstallError::UnsignedNeedsDeveloperMode),
        );
        matching.hash = NEW_HASH;
        matching.length_matches = false;
        matching.index_version_matches = false;
        assert_install(
            &current,
            &matching,
            &Err(InstallError::UnsignedNeedsDeveloperMode),
        );
    }

    /// Verifies: SEC-EXT-035
    #[test]
    fn developer_mode_installs_a_missing_signature_as_unsigned() {
        let current = installed("9.9.9", OLD_HASH, false);
        let unsigned = candidate("0.1.0", NEW_HASH, Signature::Missing, true);
        assert_install(&current, &unsigned, &Ok(installed("0.1.0", NEW_HASH, true)));
    }

    /// Verifies: SEC-EXT-035
    #[test]
    fn a_rejected_or_undelegated_signature_is_refused_even_in_developer_mode() {
        let current = installed("1.0.0", OLD_HASH, false);
        for developer_mode in [false, true] {
            let rejected = candidate("1.2.1", NEW_HASH, Signature::Rejected, developer_mode);
            assert_install(&current, &rejected, &Err(InstallError::SignatureRejected));
            let undelegated = candidate("1.2.1", NEW_HASH, Signature::NotDelegated, developer_mode);
            assert_install(&current, &undelegated, &Err(InstallError::NotDelegated));
        }
    }

    /// Verifies: SEC-EXT-035
    #[test]
    fn signature_is_checked_before_length_and_length_before_version() {
        let current = installed("1.0.0", OLD_HASH, false);
        let mut rejected = candidate("1.2.1", NEW_HASH, Signature::Rejected, true);
        rejected.length_matches = false;
        rejected.index_version_matches = false;
        assert_install(&current, &rejected, &Err(InstallError::SignatureRejected));

        let mut undelegated = candidate("1.2.1", NEW_HASH, Signature::NotDelegated, false);
        undelegated.length_matches = false;
        undelegated.index_version_matches = false;
        assert_install(&current, &undelegated, &Err(InstallError::NotDelegated));

        let mut missing = candidate("1.2.1", OLD_HASH, Signature::Missing, false);
        missing.length_matches = false;
        missing.index_version_matches = false;
        assert_install(
            &current,
            &missing,
            &Err(InstallError::UnsignedNeedsDeveloperMode),
        );

        let mut length = candidate("1.0.0", NEW_HASH, Signature::Verified, true);
        length.length_matches = false;
        length.index_version_matches = false;
        assert_install(&current, &length, &Err(InstallError::LengthMismatch));

        let mut unsigned_length = candidate("1.2.1", NEW_HASH, Signature::Missing, true);
        unsigned_length.length_matches = false;
        unsigned_length.index_version_matches = true;
        assert_install(
            &current,
            &unsigned_length,
            &Err(InstallError::LengthMismatch),
        );

        let mut version_flag = candidate("1.0.0", NEW_HASH, Signature::Verified, false);
        version_flag.index_version_matches = false;
        assert_install(&current, &version_flag, &Err(InstallError::VersionMismatch));

        let mut unsigned_version = candidate("2.0.0", NEW_HASH, Signature::Missing, true);
        unsigned_version.index_version_matches = false;
        assert_install(
            &current,
            &unsigned_version,
            &Err(InstallError::VersionMismatch),
        );
    }

    /// Verifies: SEC-EXT-040
    #[test]
    fn a_lower_version_is_a_downgrade_even_when_the_owner_approved() {
        let mut patch = offer("1.2.1", "1.2.0");
        patch.owner_approved = true;
        patch.auto_same_permission = true;
        patch.now_ms = 259_200_000;
        patch.change = PermissionChange::SameOrNarrower;
        assert_eq!(decide_update(&patch), Err(UpdateError::Downgrade));

        let mut minor = offer("1.3.0", "1.2.9");
        minor.owner_approved = true;
        minor.change = PermissionChange::Wider;
        assert_eq!(decide_update(&minor), Err(UpdateError::Downgrade));

        let mut major = offer("2.0.0", "1.9.9");
        major.same_publisher = false;
        major.auto_same_permission = true;
        major.now_ms = 259_200_000;
        major.owner_approved = true;
        major.change = PermissionChange::Wider;
        assert_eq!(decide_update(&major), Err(UpdateError::Downgrade));
    }

    /// Verifies: SEC-EXT-040
    #[test]
    fn a_wider_permission_waits_unless_the_owner_approved() {
        let mut waiting = offer("1.0.0", "1.0.1");
        waiting.change = PermissionChange::Wider;
        waiting.auto_same_permission = true;
        waiting.now_ms = 259_200_001;
        waiting.same_publisher = true;
        waiting.owner_approved = false;
        assert_eq!(decide_update(&waiting), Ok(UpdateDecision::KeepCurrent));

        let mut approved = waiting;
        approved.owner_approved = true;
        approved.same_publisher = false;
        approved.auto_same_permission = false;
        approved.now_ms = 0;
        approved.published_at_ms = 0;
        assert_eq!(decide_update(&approved), Ok(UpdateDecision::InstallOffered));
    }

    /// Verifies: SEC-EXT-040
    #[test]
    fn a_same_or_narrower_update_waits_unless_approved_or_the_hold_elapsed() {
        let waiting = offer("1.0.0", "2.0.0");
        assert_eq!(decide_update(&waiting), Ok(UpdateDecision::KeepCurrent));

        let mut elapsed_without_auto = waiting;
        elapsed_without_auto.now_ms = 259_200_000;
        assert_eq!(
            decide_update(&elapsed_without_auto),
            Ok(UpdateDecision::KeepCurrent)
        );

        let mut approved_early = waiting;
        approved_early.owner_approved = true;
        approved_early.same_publisher = false;
        approved_early.auto_same_permission = false;
        approved_early.now_ms = 0;
        assert_eq!(
            decide_update(&approved_early),
            Ok(UpdateDecision::InstallOffered)
        );

        let mut equal = offer("1.2.0", "1.2.0");
        equal.owner_approved = true;
        assert_eq!(decide_update(&equal), Ok(UpdateDecision::InstallOffered));
    }

    /// Verifies: SEC-EXT-040
    #[test]
    fn automatic_same_permission_updates_wait_seventy_two_hours() {
        let mut early = offer("1.0.0", "1.0.1");
        early.auto_same_permission = true;
        early.same_publisher = true;
        early.owner_approved = false;
        early.published_at_ms = 0;
        early.now_ms = 259_199_999;
        assert_eq!(decide_update(&early), Ok(UpdateDecision::KeepCurrent));

        let mut ready = early;
        ready.now_ms = 259_200_000;
        assert_eq!(decide_update(&ready), Ok(UpdateDecision::InstallOffered));

        let mut later = early;
        later.now_ms = 259_200_001;
        assert_eq!(decide_update(&later), Ok(UpdateDecision::InstallOffered));

        let mut other_publisher = ready;
        other_publisher.same_publisher = false;
        assert_eq!(
            decide_update(&other_publisher),
            Ok(UpdateDecision::KeepCurrent)
        );

        let mut before_publication = ready;
        before_publication.published_at_ms = 1_000;
        before_publication.now_ms = 999;
        assert_eq!(
            decide_update(&before_publication),
            Ok(UpdateDecision::KeepCurrent)
        );

        let mut negative_early = ready;
        negative_early.published_at_ms = -1;
        negative_early.now_ms = 259_199_998;
        assert_eq!(
            decide_update(&negative_early),
            Ok(UpdateDecision::KeepCurrent)
        );

        let mut negative_ready = negative_early;
        negative_ready.now_ms = 259_199_999;
        assert_eq!(
            decide_update(&negative_ready),
            Ok(UpdateDecision::InstallOffered)
        );
    }

    /// Verifies: SEC-EXT-040
    #[test]
    fn an_overflowing_publish_time_does_not_auto_install() {
        let mut overflow = offer("1.0.0", "1.0.1");
        overflow.auto_same_permission = true;
        overflow.same_publisher = true;
        overflow.owner_approved = false;
        overflow.published_at_ms = i64::MAX;
        overflow.now_ms = i64::MAX;
        assert_eq!(decide_update(&overflow), Ok(UpdateDecision::KeepCurrent));

        let mut just_overflows = overflow;
        just_overflows.published_at_ms = i64::MAX - 259_200_000 + 1;
        assert_eq!(
            decide_update(&just_overflows),
            Ok(UpdateDecision::KeepCurrent)
        );

        let mut fits = overflow;
        fits.published_at_ms = i64::MAX - 259_200_000;
        fits.now_ms = i64::MAX - 1;
        assert_eq!(decide_update(&fits), Ok(UpdateDecision::KeepCurrent));
        fits.now_ms = i64::MAX;
        assert_eq!(decide_update(&fits), Ok(UpdateDecision::InstallOffered));

        let mut lowest = overflow;
        lowest.published_at_ms = i64::MIN;
        lowest.now_ms = i64::MIN;
        assert_eq!(decide_update(&lowest), Ok(UpdateDecision::KeepCurrent));
        lowest.now_ms = i64::MIN + 259_199_999;
        assert_eq!(decide_update(&lowest), Ok(UpdateDecision::KeepCurrent));
        lowest.now_ms = i64::MIN + 259_200_000;
        assert_eq!(decide_update(&lowest), Ok(UpdateDecision::InstallOffered));
    }

    /// Verifies: SEC-EXT-040
    #[test]
    fn an_equal_version_is_not_a_downgrade() {
        let mut equal = offer("1.2.0", "1.2.0");
        equal.auto_same_permission = true;
        equal.same_publisher = true;
        equal.published_at_ms = 0;
        equal.now_ms = 259_200_000;
        assert_eq!(decide_update(&equal), Ok(UpdateDecision::InstallOffered));

        let mut higher_minor = offer("1.2.9", "1.3.0");
        higher_minor.owner_approved = true;
        assert_eq!(
            decide_update(&higher_minor),
            Ok(UpdateDecision::InstallOffered)
        );
        let mut higher_major = offer("1.9.9", "2.0.0");
        higher_major.owner_approved = true;
        higher_major.change = PermissionChange::Wider;
        assert_eq!(
            decide_update(&higher_major),
            Ok(UpdateDecision::InstallOffered)
        );
    }

    /// Verifies: SEC-EXT-038
    #[test]
    fn a_member_is_not_the_owner_for_any_action() {
        for action in actions() {
            assert_eq!(
                authorise(Actor::Member, action, false),
                Err(ActionError::NotOwner)
            );
            assert_eq!(
                authorise(Actor::Member, action, true),
                Err(ActionError::NotOwner)
            );
        }
    }

    /// Verifies: SEC-EXT-038
    #[test]
    fn a_stale_owner_is_refused_for_any_action() {
        for action in actions() {
            assert_eq!(
                authorise(Actor::OwnerStale, action, false),
                Err(ActionError::Stale)
            );
            assert_eq!(
                authorise(Actor::OwnerStale, action, true),
                Err(ActionError::Stale)
            );
        }
    }

    /// Verifies: SEC-EXT-038
    #[test]
    fn a_fresh_owner_installs_updates_grants_and_removes_without_a_fingerprint() {
        for action in [
            OwnerAction::Install,
            OwnerAction::Update,
            OwnerAction::Grant,
            OwnerAction::Remove,
        ] {
            assert_eq!(authorise(Actor::OwnerFresh, action, false), Ok(()));
            assert_eq!(authorise(Actor::OwnerFresh, action, true), Ok(()));
        }
    }

    /// Verifies: SEC-EXT-038
    #[test]
    fn adding_an_index_needs_the_root_key_fingerprint() {
        assert_eq!(
            authorise(Actor::OwnerFresh, OwnerAction::AddIndex, false),
            Err(ActionError::FingerprintMissing)
        );
        assert_eq!(
            authorise(Actor::OwnerFresh, OwnerAction::AddIndex, true),
            Ok(())
        );
    }

    /// Verifies: SEC-EXT-037
    #[test]
    fn an_api_cannot_enable_developer_mode() {
        assert_eq!(enable_from_api(), Err(DeveloperRefuse::FileOnly));
    }

    /// Verifies: SEC-EXT-037
    #[test]
    fn the_banner_names_unsigned_plugins_only_in_developer_mode() {
        assert_eq!(banner(false, &[]), None);
        assert_eq!(banner(false, &["org.example.local"]), None);
        assert_eq!(banner(false, &["a", "b"]), None);
        assert_eq!(banner(true, &[]), None);
        assert_eq!(
            banner(true, &["org.example.local"]),
            Some("Unsigned plugins: org.example.local".to_owned())
        );
        assert_eq!(
            banner(true, &["a", "b"]),
            Some("Unsigned plugins: a, b".to_owned())
        );
        assert_eq!(
            banner(true, &["b", "a"]),
            Some("Unsigned plugins: b, a".to_owned())
        );
        assert_eq!(
            banner(true, &["a", "b", "c"]),
            Some("Unsigned plugins: a, b, c".to_owned())
        );
    }
}
