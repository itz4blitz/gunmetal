//! The harness for the path rules in `gunmetal_core::path` (SEC-TM-043,
//! SEC-MED-034, SEC-MED-037, SEC-MED-039, SEC-MED-040).

use gunmetal_core::crypto;
use gunmetal_core::id::{IdKind, PublicId};
use gunmetal_core::path::{
    self, LinkVerdict, Overlap, PathError, RawPath, RelPath, RootRefusal, StorageName,
};
use gunmetal_core::untrusted::Untrusted;

/// The library root that relative names, links and containment are taken
/// against.
pub const MUSIC: &[u8] = b"/srv/music";

/// The directory beneath [`MUSIC`] that holds the link the input is read
/// as.
pub const LINK_DIR: [&[u8]; 2] = [b"Rock", b"Album"];

/// The target root approved for [`MUSIC`].
pub const DEBRID: &[u8] = b"/mnt/debrid";

/// Another library's root.
pub const FILMS: &[u8] = b"/srv/films";

/// Gunmetal's own directory that [`Outcome::refusal`] is taken against.
pub const DATA: &[u8] = b"/var/lib/gunmetal";

/// The names of a path and its display form, or why it was refused.
pub type Shown = Result<(Vec<Vec<u8>>, String), PathError>;

/// The names of a path, or why it was refused.
pub type Names = Result<Vec<Vec<u8>>, PathError>;

/// What the path rules reported for one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// [`path::normalise`] of the input split at each `/`, as a playlist
    /// entry is: the names of the path and its [`RelPath::display`], or
    /// why it was refused.
    pub split: Shown,
    /// [`path::normalise`] of the whole input as one name, as a name read
    /// from inside a media file would be.
    pub whole: Names,
    /// [`RawPath::parse`] of the input: its names and its
    /// [`RawPath::display`], or why it was refused.
    pub absolute: Shown,
    /// [`RawPath::beneath`] [`MUSIC`] of the absolute path, when it is one.
    pub beneath_music: Option<Vec<Vec<u8>>>,
    /// [`path::refuse_root`] of the absolute path, when it is one, with
    /// [`DATA`] as Gunmetal's own directory.
    pub refusal: Option<RootRefusal>,
    /// [`path::link_target`] of a link holding the input in [`LINK_DIR`]
    /// beneath [`MUSIC`].
    pub link: Names,
    /// [`path::classify_link`] of that target, when it has one, for
    /// [`MUSIC`] with [`DEBRID`] approved and [`FILMS`] another library.
    pub verdict: Option<Verdict>,
    /// [`path::display_name`] of the whole input.
    pub shown: String,
    /// [`StorageName::from_sha256`] of the input's SHA-256 digest.
    pub stored: String,
    /// [`StorageName::from_id`] of the input read as a track identifier,
    /// when it is one.
    pub stored_id: Option<String>,
}

/// A [`LinkVerdict`] with its paths written as their names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// [`LinkVerdict::Own`].
    Own(Vec<Vec<u8>>),
    /// [`LinkVerdict::Approved`].
    Approved {
        /// Which approved root.
        index: usize,
        /// The names beneath it.
        rest: Vec<Vec<u8>>,
    },
    /// [`LinkVerdict::OtherLibrary`].
    OtherLibrary {
        /// Which other library.
        index: usize,
    },
    /// [`LinkVerdict::Outside`].
    Outside,
}

/// Feeds `data` to every function of the path rules: [`path::normalise`],
/// split at each `/` and as one name; [`RawPath::parse`] and, when that
/// accepts it, [`RawPath::beneath`] [`MUSIC`] and [`path::refuse_root`];
/// [`path::link_target`] and [`path::classify_link`]; the display
/// functions; and the [`StorageName`] constructors.
///
/// # Panics
///
/// Panics when a result breaks an invariant that holds for every input: a
/// normalised path holds an empty name, `.`, `..`, or a name with `/` or
/// NUL in it, or changes when normalised again; the whole input, read as
/// one name, becomes anything but that one name, or the root for `.`; an
/// accepted absolute path is not written exactly as the input, or the path
/// beneath [`MUSIC`] does not lead back to it; `/` is not refused as the
/// filesystem root, or an overlap with [`DATA`] is missed or misnamed; a
/// link leads to a path that is not canonical, a relative link with no
/// `..` leaves its directory, or a verdict disagrees with which of
/// [`MUSIC`], [`DEBRID`] and [`FILMS`] holds the target; anything shown
/// holds a control character; or a storage name is not 64 lower-case
/// hexadecimal digits, or not the identifier it was made from.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let music = canonical(MUSIC);
    let (split, whole) = relative(data);
    let (absolute, beneath_music, refusal) = absolute(data, &music);
    let (link, verdict) = link(data, &music);
    let shown = shown(data, &split, &absolute);
    let (stored, stored_id) = stored(data);
    Outcome {
        split,
        whole,
        absolute,
        beneath_music,
        refusal,
        link,
        verdict,
        shown,
        stored,
        stored_id,
    }
}

/// Normalises `data` split at each `/` and as one name.
///
/// # Panics
///
/// Panics when a normalised path holds an empty name, `.`, `..`, or a name
/// with `/` or NUL in it, or changes when normalised again; or when the
/// whole input, read as one name, becomes anything but that one name, or
/// the root for `.`.
fn relative(data: &[u8]) -> (Shown, Names) {
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
    (
        split.map(|normalised| (normalised.components().to_vec(), normalised.display())),
        whole.map(|normalised| normalised.components().to_vec()),
    )
}

/// Reads `data` as an absolute path, and finds the path beneath `music`
/// and whether it may be a library root.
///
/// # Panics
///
/// Panics when an accepted absolute path is not written exactly as the
/// input; when the path beneath `music` does not lead back to the absolute
/// path; or when `/` is not refused as the filesystem root, or an overlap
/// with [`DATA`] is missed or misnamed.
fn absolute(data: &[u8], music: &RawPath) -> (Shown, Option<Vec<Vec<u8>>>, Option<RootRefusal>) {
    let path = match RawPath::parse(Untrusted::new(data)) {
        Ok(path) => path,
        Err(error) => return (Err(error), None, None),
    };
    let path = &path;
    assert!(
        written(path) == data,
        "{data:?} was read as {path:?}, which is written differently"
    );
    let beneath_music = path.beneath(music);
    if let Some(rest) = &beneath_music {
        assert!(
            music
                .components()
                .iter()
                .chain(rest.components())
                .eq(path.components()),
            "{path:?} is beneath {music:?} at {rest:?}"
        );
    }
    let own = canonical(DATA);
    let refusal = path::refuse_root(path, std::slice::from_ref(&own));
    let overlap = match (path.beneath(&own), own.beneath(path)) {
        (Some(_), Some(_)) => Some(Overlap::Equal),
        (None, Some(_)) => Some(Overlap::Contains),
        (Some(_), None) => Some(Overlap::Inside),
        (None, None) => None,
    };
    assert!(
        (path.components().is_empty() == (refusal == Some(RootRefusal::FilesystemRoot)))
            && match refusal {
                None => overlap.is_none(),
                Some(RootRefusal::DataDirectory {
                    index,
                    overlap: found,
                }) => index == 0 && Some(found) == overlap,
                Some(_) => true,
            },
        "{path:?} was refused as {refusal:?}"
    );
    (
        Ok((path.components().to_vec(), path.display())),
        beneath_music.map(|rest| rest.components().to_vec()),
        refusal,
    )
}

/// Reads `data` as the text of a link in [`LINK_DIR`] beneath `music`, and
/// decides whether it may be followed.
///
/// # Panics
///
/// Panics when the target is not a canonical absolute path; when a
/// relative link with no `..` leads anywhere but beneath its directory; or
/// when the verdict disagrees with which of [`MUSIC`], [`DEBRID`] and
/// [`FILMS`] holds the target.
fn link(data: &[u8], music: &RawPath) -> (Names, Option<Verdict>) {
    let dir = path::normalise(Untrusted::new(&LINK_DIR)).expect("the link's directory is valid");
    let target = match path::link_target(music, &dir, Untrusted::new(data)) {
        Ok(target) => target,
        Err(error) => return (Err(error), None),
    };
    assert!(
        RawPath::parse(Untrusted::new(&written(&target))).as_ref() == Ok(&target),
        "{data:?} led to {target:?}, which is not canonical"
    );
    assert!(
        data.starts_with(b"/")
            || data.split(|&byte| byte == b'/').any(|name| name == b"..")
            || target
                .beneath(music)
                .is_some_and(|rest| rest.components().starts_with(dir.components())),
        "{data:?} led out of its directory to {target:?}"
    );
    let (debrid, films) = (canonical(DEBRID), canonical(FILMS));
    let verdict = path::classify_link(
        &target,
        music,
        std::slice::from_ref(&debrid),
        std::slice::from_ref(&films),
    );
    let (in_music, in_debrid, in_films) = (
        target.beneath(music),
        target.beneath(&debrid),
        target.beneath(&films),
    );
    assert!(
        match &verdict {
            LinkVerdict::Own(rest) => in_music.as_ref() == Some(rest),
            LinkVerdict::Approved { index, rest } => {
                *index == 0 && in_music.is_none() && in_debrid.as_ref() == Some(rest)
            }
            LinkVerdict::OtherLibrary { index } => {
                *index == 0 && in_music.is_none() && in_debrid.is_none() && in_films.is_some()
            }
            LinkVerdict::Outside => in_music.is_none() && in_debrid.is_none() && in_films.is_none(),
        },
        "{target:?} was judged {verdict:?}"
    );
    let verdict = match verdict {
        LinkVerdict::Own(rest) => Verdict::Own(rest.components().to_vec()),
        LinkVerdict::Approved { index, rest } => Verdict::Approved {
            index,
            rest: rest.components().to_vec(),
        },
        LinkVerdict::OtherLibrary { index } => Verdict::OtherLibrary { index },
        LinkVerdict::Outside => Verdict::Outside,
    };
    (Ok(target.components().to_vec()), Some(verdict))
}

/// Shows `data` as one name.
///
/// # Panics
///
/// Panics when that, or the display form of an accepted relative or
/// absolute path, holds a control character.
fn shown(data: &[u8], split: &Shown, absolute: &Shown) -> String {
    let shown = path::display_name(Untrusted::new(data));
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
    shown
}

/// Names `data` as stored content, and as a track identifier when it is
/// one.
///
/// # Panics
///
/// Panics when the content's name is not 64 lower-case hexadecimal digits,
/// or an identifier's name is not the identifier.
fn stored(data: &[u8]) -> (String, Option<String>) {
    let stored = StorageName::from_sha256(crypto::sha256(data));
    assert!(
        stored.as_str().len() == 64
            && stored
                .as_str()
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')),
        "{data:?} was stored as {stored:?}"
    );
    let text = String::from_utf8_lossy(data);
    let stored_id = PublicId::parse(&text, IdKind::Track)
        .ok()
        .map(|id| StorageName::from_id(id).as_str().to_owned());
    if let Some(name) = &stored_id {
        assert!(*name == text, "{text:?} was stored as {name:?}");
    }
    (stored.as_str().to_owned(), stored_id)
}

/// Reads a path this harness names, which is canonical.
fn canonical(text: &[u8]) -> RawPath {
    RawPath::parse(Untrusted::new(text)).expect("the harness's own paths are canonical")
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
