//! The harness for the path rules in `gunmetal_core::path` (SEC-TM-043).

use gunmetal_core::path::{self, PathError, RelPath};
use gunmetal_core::untrusted::Untrusted;

/// What the path rules reported for one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// [`path::normalise`] of the input split at each `/`, as a playlist
    /// entry is: the names of the path, or why it was refused.
    pub split: Result<Vec<Vec<u8>>, PathError>,
    /// [`path::normalise`] of the whole input as one name, as a name read
    /// from inside a media file would be.
    pub whole: Result<Vec<Vec<u8>>, PathError>,
}

/// Feeds `data` to [`path::normalise`], split at each `/` and as one name.
///
/// # Panics
///
/// Panics when a normalised path holds an empty name, `.`, `..`, or a name
/// with `/` or NUL in it, or changes when normalised again; or when the
/// whole input, read as one name, becomes anything but that one name, or
/// the root for `.`.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let names: Vec<&[u8]> = data.split(|&byte| byte == b'/').collect();
    let split = path::normalise(Untrusted::new(&names));
    let whole = path::normalise(Untrusted::new(&[data]));
    for normalised in [&split, &whole].into_iter().flatten() {
        assert!(
            normalised.components().iter().all(|name| {
                !name.is_empty()
                    && name != b"."
                    && name != b".."
                    && !name.contains(&b'/')
                    && !name.contains(&0)
            }) && renormalised(normalised).as_ref() == Ok(normalised),
            "{data:?} normalised to {normalised:?}"
        );
    }
    if let Ok(one) = &whole {
        assert!(
            one.components() == [data] || (data == b"." && one.components().is_empty()),
            "{data:?} as one name normalised to {one:?}"
        );
    }
    Outcome {
        split: split.map(|normalised| normalised.components().to_vec()),
        whole: whole.map(|normalised| normalised.components().to_vec()),
    }
}

/// Normalises the names of a path that is already normalised.
fn renormalised(normalised: &RelPath) -> Result<RelPath, PathError> {
    let names: Vec<&[u8]> = normalised.components().iter().map(Vec::as_slice).collect();
    path::normalise(Untrusted::new(&names))
}
