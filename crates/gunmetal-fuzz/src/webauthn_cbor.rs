//! The harness for the `WebAuthn` CBOR subset in
//! [`gunmetal_core::webauthn::cbor`].

use gunmetal_core::parse::{Budget, Depth, Limits};
use gunmetal_core::webauthn::{self, Item, WebauthnError};

/// What [`webauthn::decode`] reported for one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome<'a> {
    /// The item at the start of the input, and any unused suffix.
    pub item: Result<Item<'a>, WebauthnError>,
}

/// Feeds `data` to [`webauthn::decode`] under the default limits, at the
/// top of a container, with a budget no input can spend.
///
/// # Panics
///
/// Panics when the reader breaks an invariant that holds for every input:
/// more than `n + 1` steps for `n` octets (SEC-MED-007), an error offset
/// past the end of the input, or an unused suffix that is not the tail of
/// the input.
#[must_use]
pub fn run(data: &[u8]) -> Outcome<'_> {
    let octets = u64::try_from(data.len()).unwrap_or(u64::MAX);
    let mut budget = Budget::for_input(0, 0, u64::MAX);
    let item = webauthn::decode(data, &Limits::DEFAULT, &mut budget, Depth::CONTAINER_ROOT);
    let spent = u64::MAX.saturating_sub(budget.remaining());
    assert!(
        spent <= octets.saturating_add(1),
        "more than one step per octet, plus one"
    );
    match &item {
        Ok(found) => {
            let rest = found.rest.len();
            let tail = data.len().checked_sub(rest).and_then(|at| data.get(at..));
            assert!(
                tail == Some(found.rest),
                "decode returned a suffix that is not the tail of the input"
            );
        }
        Err(error) => {
            assert!(
                error.offset() <= octets,
                "the error offset lies outside the input"
            );
        }
    }
    Outcome { item }
}
