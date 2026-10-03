//! The harness for the `WebAuthn` COSE key parser in
//! [`gunmetal_core::webauthn::cose`].

use gunmetal_core::parse::{Budget, Depth, Limits};
use gunmetal_core::webauthn::{self, CoseKey, WebauthnError};

/// What [`webauthn::cose_key`] reported for one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The key, or the error that stopped the parse.
    pub key: Result<CoseKey, WebauthnError>,
}

/// Feeds `data` to [`webauthn::cose_key`] under the default limits, at the
/// top of a container, with a budget no input can spend.
///
/// # Panics
///
/// Panics when the parser spends more than `n + 1` steps for `n` octets
/// (SEC-MED-007), or reports an error offset past the end of the input.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let octets = u64::try_from(data.len()).unwrap_or(u64::MAX);
    let mut budget = Budget::for_input(0, 0, u64::MAX);
    let key = webauthn::cose_key(data, &Limits::DEFAULT, &mut budget, Depth::CONTAINER_ROOT);
    let spent = u64::MAX - budget.remaining();
    assert!(
        spent <= octets.saturating_add(1),
        "{spent} steps for {octets} octets"
    );
    if let Err(error) = &key {
        assert!(
            error.offset() <= octets,
            "{error:?} lies outside {octets} octets"
        );
    }
    Outcome { key }
}
