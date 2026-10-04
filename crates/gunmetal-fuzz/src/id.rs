//! The harness for the public-identifier parser in `gunmetal_core::id`
//! (SEC-API-023, SEC-API-024).

use gunmetal_core::id::{IdError, IdKind, PublicId};

/// What the identifier parser reported for one input, read as UTF-8 with
/// each invalid sequence replaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outcome {
    /// The kind [`PublicId::parse`] read the input as an identifier of,
    /// if any. No input is an identifier of two kinds.
    pub kind: Option<IdKind>,
}

/// Feeds `data` to [`PublicId::parse`] once for every kind in [`IdKind::ALL`].
///
/// # Panics
///
/// Panics when the input is read as an identifier of more than one kind,
/// when an accepted identifier is not written back exactly as the input,
/// so that it has a second spelling, or when a refusal names a kind other
/// than the one asked for.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let text = String::from_utf8_lossy(data);
    let parsed = IdKind::ALL.map(|kind| PublicId::parse(&text, kind));
    assert!(
        parsed.iter().filter(|result| result.is_ok()).count() <= 1
            && parsed
                .iter()
                .zip(IdKind::ALL)
                .all(|(result, kind)| match result {
                    Ok(id) => id.to_string() == text,
                    Err(error) => *error == IdError { expected: kind },
                }),
        "{text:?} gave {parsed:?}"
    );
    Outcome {
        kind: parsed
            .iter()
            .zip(IdKind::ALL)
            .find_map(|(result, kind)| result.is_ok().then_some(kind)),
    }
}
