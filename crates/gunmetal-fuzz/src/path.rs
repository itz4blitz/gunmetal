//! The harness for the path rules in `gunmetal_core::path` (SEC-TM-043,
//! SEC-MED-040).

use gunmetal_core::path::{self, PathError, RawPath, RelPath};
use gunmetal_core::untrusted::Untrusted;

/// The library root that [`Outcome::beneath_music`] is taken against.
pub const MUSIC: &[u8] = b"/srv/music";

/// What the path rules reported for one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// [`path::normalise`] of the input split at each `/`, as a playlist
    /// entry is: the names of the path and its [`RelPath::display`], or
    /// why it was refused.
    pub split: Result<(Vec<Vec<u8>>, String), PathError>,
    /// [`path::normalise`] of the whole input as one name, as a name read
    /// from inside a media file would be.
    pub whole: Result<Vec<Vec<u8>>, PathError>,
    /// [`RawPath::parse`] of the input: its names and its
    /// [`RawPath::display`], or why it was refused.
    pub absolute: Result<(Vec<Vec<u8>>, String), PathError>,
    /// [`RawPath::beneath`] [`MUSIC`] of the absolute path, when it is one.
    pub beneath_music: Option<Vec<Vec<u8>>>,
    /// [`path::display_name`] of the whole input.
    pub shown: String,
}

/// Feeds `data` to [`path::normalise`], split at each `/` and as one name,
/// to [`RawPath::parse`] and, when that accepts it, to
/// [`RawPath::beneath`] [`MUSIC`], and to the display functions.
///
/// # Panics
///
/// Panics when a normalised path holds an empty name, `.`, `..`, or a name
/// with `/` or NUL in it, or changes when normalised again; when the whole
/// input, read as one name, becomes anything but that one name, or the root
/// for `.`; when an accepted absolute path is not written exactly as the
/// input; when the path beneath [`MUSIC`] does not lead back to the
/// absolute path; or when anything shown holds a control character.
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
    let absolute = RawPath::parse(Untrusted::new(data));
    let music = RawPath::parse(Untrusted::new(MUSIC)).expect("the library root is canonical");
    let beneath_music = absolute.as_ref().ok().and_then(|path| path.beneath(&music));
    if let Ok(path) = &absolute {
        assert!(
            written(path) == data,
            "{data:?} was read as {path:?}, which is written differently"
        );
    }
    if let (Ok(path), Some(rest)) = (&absolute, &beneath_music) {
        assert!(
            music
                .components()
                .iter()
                .chain(rest.components())
                .eq(path.components()),
            "{path:?} is beneath {music:?} at {rest:?}"
        );
    }
    let shown = path::display_name(Untrusted::new(data));
    let split = split.map(|normalised| (normalised.components().to_vec(), normalised.display()));
    let absolute = absolute.map(|path| (path.components().to_vec(), path.display()));
    for text in [
        Some(&shown),
        split.as_ref().ok().map(|(_, text)| text),
        absolute.as_ref().ok().map(|(_, text)| text),
    ]
    .into_iter()
    .flatten()
    {
        assert!(
            !text.chars().any(char::is_control),
            "{data:?} was shown as {text:?}"
        );
    }
    Outcome {
        split,
        whole: whole.map(|normalised| normalised.components().to_vec()),
        absolute,
        beneath_music: beneath_music.map(|rest| rest.components().to_vec()),
        shown,
    }
}

/// Normalises the names of a path that is already normalised.
fn renormalised(normalised: &RelPath) -> Result<RelPath, PathError> {
    let names: Vec<&[u8]> = normalised.components().iter().map(Vec::as_slice).collect();
    path::normalise(Untrusted::new(&names))
}

/// An absolute path written out as bytes: `/` alone, or each name after a
/// `/`.
fn written(path: &RawPath) -> Vec<u8> {
    if path.components().is_empty() {
        return b"/".to_vec();
    }
    path.components()
        .iter()
        .flat_map(|name| [&b"/"[..], name])
        .flatten()
        .copied()
        .collect()
}
