//! The written list of reads that need no `Permit`.
//!
//! Every read of the identity store that returns a user-visible object (an
//! account, device, grant, invitation or share) takes a `Permit` from the
//! policy (SEC-TM-024, SEC-API-010). WP-065 adds that reader in this crate;
//! the store's general reader stays private to the crate. Three lookups
//! happen before there is a principal to ask the policy about, and they are
//! the only public reads without a `Permit`. Each is a variant here, so the
//! list is closed, and a `disallowed-methods` entry confines
//! [`IdentityStore::read_pre_principal`](crate::identity::store::IdentityStore::read_pre_principal)
//! to the server module that owns each lookup.

/// One of the three reads made before there is a principal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrePrincipal {
    /// The session-token lookup that turns a presented token into a
    /// session (WP-062, `session/`).
    SessionToken,
    /// The credential lookup handed to pathway checks through the
    /// verifier's pre-authentication handle (WP-064, `verifier/`).
    Credential,
    /// The grant read that builds the `Permit` (WP-065, `access/`).
    Grant,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The written list, in a match with no wildcard: a fourth variant does
    /// not compile here until this list and its review change too.
    fn owner(lookup: PrePrincipal) -> &'static str {
        match lookup {
            PrePrincipal::SessionToken => "session/",
            PrePrincipal::Credential => "verifier/",
            PrePrincipal::Grant => "access/",
        }
    }

    #[test]
    fn lists_exactly_the_three_pre_principal_lookups() {
        let lookups = [
            PrePrincipal::SessionToken,
            PrePrincipal::Credential,
            PrePrincipal::Grant,
        ];
        assert_eq!(lookups.map(owner), ["session/", "verifier/", "access/"]);
    }
}
