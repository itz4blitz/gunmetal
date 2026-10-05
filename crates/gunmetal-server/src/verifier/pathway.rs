//! The pathway inventory, and what a pathway's package implements.
//!
//! [`Pathway`] is the one list of the ways to authenticate (SEC-TM-014,
//! SEC-HIS-046). In R1 it holds the passkey, the paired browser, the claim
//! code, the recovery code, an administrator's recovery link, the
//! invitation and the pairing code. A later pathway (the share-link
//! password and OIDC in R1.2; device keys, API keys and adapter credentials
//! in R2) is one more entry, added by the package that brings it, and it
//! does not compile until it has a name and a strength here.
//!
//! A pathway's package implements [`PathwayCheck`] and nothing more of the
//! check: the verifier decides when its two steps run and what their
//! answers mean. The types here keep what must not go wrong from going
//! wrong by construction:
//!
//! - a pathway is only ever handed a [`Credential`], which only the
//!   verifier makes and which is never empty (SEC-HIS-004);
//! - a pathway reads stored credentials only through the
//!   [`PreAuth`] handle it is handed (SEC-TM-024, SEC-API-010);
//! - what was presented, and what it is compared with, cannot be printed:
//!   their debug forms are fixed words.

use core::fmt;

use gunmetal_core::id::PublicId;
use gunmetal_durable::identity::error::IdentityError;

use crate::verifier::preauth::PreAuth;

/// Declares the inventory. The type and the list of all of it come from one
/// table, so a pathway cannot be in one and missing from the other; the
/// tests pin the list against a literal one.
macro_rules! pathways {
    ($( $(#[$doc:meta])* $variant:ident, )*) => {
        /// One way to authenticate: an entry of the pathway inventory.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Pathway {
            $( $(#[$doc])* $variant, )*
        }

        impl Pathway {
            /// Every pathway, in the order of the inventory.
            pub const ALL: &'static [Self] = &[$(Self::$variant),*];
        }
    };
}

pathways! {
    /// A passkey assertion.
    Passkey,
    /// A paired browser's signature over the server's challenge.
    PairedBrowser,
    /// The code that claims an unclaimed server.
    ClaimCode,
    /// One of an account's single-use recovery codes.
    RecoveryCode,
    /// A recovery link an administrator issued.
    RecoveryLink,
    /// An invitation.
    Invitation,
    /// The code a browser shows to be paired.
    PairingCode,
}

/// How hard a pathway's secret is to guess, which decides how its wrong
/// guesses are limited (SEC-API-056).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strength {
    /// Short enough to guess, or one of the secrets the baseline names
    /// with them: each wrong guess makes its source wait longer for the
    /// next, on the delay schedule.
    Guessable,
    /// A key, or a secret too long to guess: its failures are held only to
    /// the per-source and server-wide ceilings, and never delay or disable
    /// anything, since a delay would only let a stranger keep the owner
    /// out.
    Strong,
}

impl Pathway {
    /// The pathway's name in the sign-in failure line and in the guess
    /// log.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Passkey => "passkey",
            Self::PairedBrowser => "paired_browser",
            Self::ClaimCode => "claim_code",
            Self::RecoveryCode => "recovery_code",
            Self::RecoveryLink => "recovery_link",
            Self::Invitation => "invitation",
            Self::PairingCode => "pairing_code",
        }
    }

    /// How hard the pathway's secret is to guess.
    #[must_use]
    pub const fn strength(self) -> Strength {
        match self {
            Self::ClaimCode | Self::PairingCode => Strength::Guessable,
            Self::Passkey
            | Self::PairedBrowser
            | Self::RecoveryCode
            | Self::RecoveryLink
            | Self::Invitation => Strength::Strong,
        }
    }
}

/// What a guess at a short secret is aimed at, when a pathway has more
/// than one thing to guess: the person approving a pairing, a share link,
/// a profile. Sixteen bytes the pathway's package takes from an identifier
/// it already has. The delay schedule counts wrong guesses for each target
/// and source, so guessing at one never delays another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Target([u8; 16]);

impl Target {
    /// The target that carries `bytes`.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// The bytes this target was made from.
    #[must_use]
    pub const fn bytes(self) -> [u8; 16] {
        self.0
    }
}

/// What a request presented on a pathway, before the verifier has looked
/// at it. It may be missing or empty; the verifier refuses both.
pub struct Presented<'a> {
    secret: Option<&'a [u8]>,
    target: Option<Target>,
}

impl<'a> Presented<'a> {
    /// The credential a request carried.
    #[must_use]
    pub const fn new(secret: &'a [u8]) -> Self {
        Self {
            secret: Some(secret),
            target: None,
        }
    }

    /// A request that carried no credential.
    #[must_use]
    pub const fn missing() -> Self {
        Self {
            secret: None,
            target: None,
        }
    }

    /// The same, aimed at `target`.
    #[must_use]
    pub fn aimed_at(self, target: Target) -> Self {
        Self {
            secret: self.secret,
            target: Some(target),
        }
    }

    /// What the guess is aimed at, when the request named something.
    #[must_use]
    pub const fn target(&self) -> Option<Target> {
        self.target
    }

    /// The credential, unless none was presented or it was empty.
    #[must_use]
    pub fn credential(&self) -> Option<Credential<'a>> {
        self.secret
            .filter(|secret| !secret.is_empty())
            .map(Credential)
    }
}

impl fmt::Debug for Presented<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Presented(..)")
    }
}

/// A credential the verifier has found to be there. Only the verifier
/// makes one, and never from nothing, so a pathway cannot be asked to
/// check an empty credential. It has no `==`: a pathway compares it in
/// its own way, in constant time where that matters.
#[derive(Clone, Copy)]
pub struct Credential<'a>(&'a [u8]);

impl<'a> Credential<'a> {
    /// The credential's bytes, for the pathway to check.
    #[must_use]
    pub const fn bytes(self) -> &'a [u8] {
        self.0
    }
}

impl fmt::Debug for Credential<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Credential(..)")
    }
}

/// What a presented credential is compared with: a public key, a keyed
/// hash of a code, whatever the pathway keeps. The verifier never reads
/// it.
pub struct Material(Vec<u8>);

impl Material {
    /// The material that holds `bytes`.
    #[must_use]
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// The material's bytes, for the pathway to compare with.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for Material {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Material(..)")
    }
}

/// A stored credential, as a pathway found it.
#[derive(Debug)]
pub struct Stored {
    /// The account it belongs to, when it belongs to one.
    pub account: Option<PublicId>,
    /// The pathway it was issued for, which never changes. The verifier
    /// accepts it only on that pathway (SEC-EXT-007).
    pub kind: Pathway,
    /// Whether it may still be used: not for a disabled account, nor when
    /// it is revoked, expired or used up.
    pub usable: bool,
    /// What a presented credential is compared with.
    pub material: Material,
}

/// A credential the verifier accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Verified {
    /// The pathway it was presented on.
    pub pathway: Pathway,
    /// The account it belongs to, when it belongs to one.
    pub account: Option<PublicId>,
}

/// Why a pathway could not finish its check. Every fault ends in the same
/// refusal as a wrong credential (SEC-IAM-069).
#[derive(Debug, Clone, PartialEq)]
pub enum Fault {
    /// The identity store failed or refused a read or a write.
    Storage(Box<IdentityError>),
    /// The check did not finish in the time it had.
    Timeout,
    /// Something the check needs is not there or cannot be read: a key, a
    /// stored field, the pathway's own state.
    Missing,
}

impl Fault {
    /// The word the sign-in failure line gives as the cause.
    #[must_use]
    pub fn cause(&self) -> &'static str {
        match self {
            Self::Storage(_) => "storage",
            Self::Timeout => "timeout",
            Self::Missing => "missing_data",
        }
    }
}

impl From<IdentityError> for Fault {
    fn from(error: IdentityError) -> Self {
        Self::Storage(Box::new(error))
    }
}

/// What a pathway's package implements: the two steps of its check.
///
/// Neither step may change anything. The verifier may still refuse after
/// both have answered, and what a sign-in does (using up a code, opening a
/// session) is done by the caller once [`Verified`] comes back. Neither
/// step may call the verifier: it looks at one guess at a short secret at
/// a time, and would wait for itself.
pub trait PathwayCheck {
    /// Which entry of the inventory this is.
    fn pathway(&self) -> Pathway;

    /// Looks up the stored credential the presented one names, through the
    /// handle. `None` when nothing is stored for it.
    ///
    /// # Errors
    ///
    /// A [`Fault`] when the lookup could not be finished.
    fn find(
        &self,
        credential: Credential<'_>,
        lookup: &PreAuth<'_>,
    ) -> Result<Option<Stored>, Fault>;

    /// Material nothing presented can be right for, of the shape the
    /// pathway stores. The verifier has the comparison run against it when
    /// nothing is stored, so that an unknown credential costs the work of
    /// a wrong one.
    fn decoy(&self) -> Material;

    /// Whether the presented credential is right for `material`. It runs
    /// once for every attempt that presented something, whatever
    /// [`PathwayCheck::find`] found.
    ///
    /// # Errors
    ///
    /// A [`Fault`] when the comparison could not be finished.
    fn matches(&self, credential: Credential<'_>, material: &Material) -> Result<bool, Fault>;
}

#[cfg(test)]
mod tests {
    use gunmetal_durable::identity::error::IdentityError;

    use super::*;

    /// The inventory, written out here on its own: every pathway R1 has, by
    /// the name its failures are logged under, and whether its secret is one
    /// the delay schedule applies to.
    const INVENTORY: [(&str, Strength); 7] = [
        ("passkey", Strength::Strong),
        ("paired_browser", Strength::Strong),
        ("claim_code", Strength::Guessable),
        ("recovery_code", Strength::Strong),
        ("recovery_link", Strength::Strong),
        ("invitation", Strength::Strong),
        ("pairing_code", Strength::Guessable),
    ];

    /// Where a pathway is in the inventory. There is no catch-all arm, so a
    /// pathway added to the inventory does not compile here until it has a
    /// place, and then the literal list above has to gain its row.
    fn place(pathway: Pathway) -> usize {
        match pathway {
            Pathway::Passkey => 0,
            Pathway::PairedBrowser => 1,
            Pathway::ClaimCode => 2,
            Pathway::RecoveryCode => 3,
            Pathway::RecoveryLink => 4,
            Pathway::Invitation => 5,
            Pathway::PairingCode => 6,
        }
    }

    /// Verifies: SEC-TM-014, SEC-HIS-046
    #[test]
    fn the_inventory_is_exactly_the_pathways_of_the_first_release() {
        let names: Vec<&str> = Pathway::ALL.iter().map(|pathway| pathway.name()).collect();
        assert_eq!(names, INVENTORY.map(|(name, _)| name));
        let places: Vec<usize> = Pathway::ALL.iter().copied().map(place).collect();
        assert_eq!(places, [0, 1, 2, 3, 4, 5, 6]);
    }

    /// Verifies: SEC-API-056
    #[test]
    fn only_the_claim_code_and_the_pairing_code_are_delayed_as_guessable() {
        let strengths: Vec<Strength> = Pathway::ALL
            .iter()
            .map(|pathway| pathway.strength())
            .collect();
        assert_eq!(strengths, INVENTORY.map(|(_, strength)| strength));
    }

    #[test]
    fn a_target_keeps_the_bytes_it_was_made_from() {
        let mut bytes = [7_u8; 16];
        bytes[0] = 1;
        bytes[15] = 2;
        assert_eq!(Target::from_bytes(bytes).bytes(), bytes);
    }

    /// The bytes of the credential `presented` holds, when it holds one.
    fn bytes<'a>(presented: &Presented<'a>) -> Option<&'a [u8]> {
        presented.credential().map(Credential::bytes)
    }

    /// Verifies: SEC-HIS-004
    #[test]
    fn a_missing_or_empty_presentation_holds_no_credential() {
        assert!(Presented::missing().credential().is_none());
        assert!(Presented::new(b"").credential().is_none());
        let target = Target::from_bytes([3; 16]);
        assert!(Presented::new(b"").aimed_at(target).credential().is_none());
        assert_eq!(bytes(&Presented::new(b"k")), Some(&b"k"[..]));
        assert_eq!(bytes(&Presented::new(b"\0")), Some(&b"\0"[..]));
        assert_eq!(
            bytes(&Presented::new(b"a longer code").aimed_at(target)),
            Some(&b"a longer code"[..])
        );
    }

    #[test]
    fn a_presentation_keeps_what_it_is_aimed_at() {
        let target = Target::from_bytes([9; 16]);
        assert_eq!(Presented::new(b"code").target(), None);
        assert_eq!(Presented::missing().target(), None);
        assert_eq!(
            Presented::new(b"code").aimed_at(target).target(),
            Some(target)
        );
        assert_eq!(Presented::missing().aimed_at(target).target(), Some(target));
    }

    #[test]
    fn what_was_presented_and_what_is_stored_cannot_be_printed() {
        let presented = Presented::new(b"canary7f3a").aimed_at(Target::from_bytes([1; 16]));
        let credential = presented.credential().expect("a credential");
        let stored = Stored {
            account: None,
            kind: Pathway::ClaimCode,
            usable: true,
            material: Material::new(b"canary7f3a".to_vec()),
        };
        assert_eq!(format!("{presented:?}"), "Presented(..)");
        assert_eq!(format!("{credential:?}"), "Credential(..)");
        assert_eq!(format!("{:?}", stored.material), "Material(..)");
        assert_eq!(
            format!("{stored:?}"),
            "Stored { account: None, kind: ClaimCode, usable: true, material: Material(..) }"
        );
        assert_eq!(stored.material.as_bytes(), b"canary7f3a");
    }

    #[test]
    fn names_the_cause_of_each_fault() {
        let causes = [
            Fault::from(IdentityError::Foreign),
            Fault::Timeout,
            Fault::Missing,
        ]
        .map(|fault| fault.cause());
        assert_eq!(causes, ["storage", "timeout", "missing_data"]);
        assert_eq!(
            Fault::from(IdentityError::Foreign),
            Fault::Storage(Box::new(IdentityError::Foreign))
        );
    }
}
