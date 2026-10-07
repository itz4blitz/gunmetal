//! Hash chaining: SHA-256 over the domain separator, the previous digest
//! and the canonical record without its `hash` field (SEC-IAM-094).

use gunmetal_core::crypto::sha256;

use crate::audit::encode::DOMAIN;

/// SHA-256 of `gunmetal-audit-v1` ∥ `prev` ∥ `canonical`.
pub(crate) fn digest(prev: &[u8; 32], canonical: &str) -> [u8; 32] {
    let mut input = Vec::with_capacity(
        DOMAIN
            .len()
            .saturating_add(32)
            .saturating_add(canonical.len()),
    );
    input.extend_from_slice(DOMAIN);
    input.extend_from_slice(prev);
    input.extend_from_slice(canonical.as_bytes());
    sha256(&input)
}

#[cfg(test)]
mod tests {
    use super::digest;
    use crate::audit::encode::hex;

    /// Independent SHA-256 of `gunmetal-audit-v1` ∥ 32 zero bytes ∥
    /// `{"seq":1}`, from Python's hashlib.
    const SEQ1: &str = "fe3162ca88839492aa1c3b9d3e325627f7f84f14906d81b7973c10ca389e7868";

    /// Verifies: SEC-IAM-094, SEC-OPS-023
    #[test]
    fn hashes_the_domain_separator_previous_digest_and_canonical_record() {
        assert_eq!(hex(&digest(&[0; 32], r#"{"seq":1}"#)), SEQ1);
        assert_ne!(
            digest(&[0; 32], r#"{"seq":1}"#),
            digest(&[1; 32], r#"{"seq":1}"#)
        );
        assert_ne!(
            digest(&[0; 32], r#"{"seq":1}"#),
            digest(&[0; 32], r#"{"seq":2}"#)
        );
    }
}
