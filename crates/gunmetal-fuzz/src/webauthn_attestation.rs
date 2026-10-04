//! The harness for the `WebAuthn` attestation-object parser in
//! [`gunmetal_core::webauthn::attestation`].

use gunmetal_core::parse::{Budget, Depth, Limits};
use gunmetal_core::webauthn::{self, Attestation, WebauthnError};

/// What [`webauthn::attestation_object`] reported for one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome<'a> {
    /// The attestation, or the error that stopped the parse.
    pub attestation: Result<Attestation<'a>, WebauthnError>,
}

/// Feeds `data` to [`webauthn::attestation_object`] under the default
/// limits, at the top of a container, with a budget no input can spend.
///
/// # Panics
///
/// Panics when the parser spends more than `n + 1` steps for `n` octets
/// (SEC-MED-007), or reports an error offset past the end of the input.
#[must_use]
pub fn run(data: &[u8]) -> Outcome<'_> {
    let octets = u64::try_from(data.len()).unwrap_or(u64::MAX);
    let mut budget = Budget::for_input(0, 0, u64::MAX);
    let parsed =
        webauthn::attestation_object(data, &Limits::DEFAULT, &mut budget, Depth::CONTAINER_ROOT);
    let spent = u64::MAX.saturating_sub(budget.remaining());
    assert!(
        spent <= octets.saturating_add(1),
        "more than one step per octet, plus one"
    );
    if let Err(error) = &parsed {
        assert!(
            error.offset() <= octets,
            "the error offset lies outside the input"
        );
    }
    Outcome {
        attestation: parsed,
    }
}
