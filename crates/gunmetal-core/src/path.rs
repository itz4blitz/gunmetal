//! Paths as raw byte names (WP-024): relative paths beneath a root
//! ([`normalise`]), canonical absolute paths and containment
//! ([`RawPath::beneath`]), the folders that may not be library roots
//! ([`refuse_root`]), where a symbolic link leads and whether it may be
//! followed ([`link_target`], [`classify_link`]), names for the files the
//! server writes ([`StorageName`]), and names as people see them
//! ([`display_name`]).
//!
//! Nothing here touches the filesystem. The filesystem crate (WP-060) opens
//! every [`RelPath`] beneath a root's directory handle. Names stay bytes for
//! access (SEC-MED-040), so a file whose name is not UTF-8, or holds a
//! newline or an escape character, is still reachable, and only its display
//! form is decoded.

use crate::id::PublicId;
use crate::problem::{Arg, Describe, Problem, ProblemCode};
use crate::untrusted::Untrusted;

/// A path beneath a root: zero or more names, none of them empty, `.` or
/// `..`, and none holding `/` or a NUL byte. The empty path is the root
/// itself. Only [`normalise`] makes one from outside names; the other
/// functions here take one from beneath a canonical [`RawPath`]. So a
/// `RelPath` can never climb out of the root it is opened beneath
/// (SEC-TM-043).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RelPath(Vec<Vec<u8>>);

impl RelPath {
    /// The names, from the root down.
    #[must_use]
    pub fn components(&self) -> &[Vec<u8>] {
        &self.0
    }

    /// The path as people see it: its names separated by `/`, each decoded
    /// as described in [`display_name`]. The root itself shows as nothing.
    #[must_use]
    pub fn display(&self) -> String {
        joined(&self.0)
    }
}

/// An absolute path in canonical form, as the filesystem crate reports a
/// root after resolving it: `/` alone, or `/` and names separated by single
/// slashes, with no `.` or `..`. Library roots, approved link targets,
/// browse roots and Gunmetal's own directories are `RawPath`s.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RawPath(Vec<Vec<u8>>);

impl RawPath {
    /// Reads an absolute path in canonical form. A path that is not
    /// canonical is refused rather than tidied, so a root that was never
    /// resolved cannot be compared as if it had been.
    ///
    /// # Errors
    ///
    /// [`PathError::NotAbsolute`] unless the path starts with `/`; a
    /// [`PathError`] naming the first empty name, `.`, `..` or NUL.
    pub fn parse(text: Untrusted<&[u8]>) -> Result<Self, PathError> {
        let rest = text
            .into_inner()
            .strip_prefix(b"/")
            .ok_or(PathError::NotAbsolute)?;
        if rest.is_empty() {
            return Ok(Self(Vec::new()));
        }
        rest.split(|&byte| byte == b'/')
            .enumerate()
            .map(|(component, name)| match name {
                [] => Err(PathError::EmptyName { component }),
                b"." | b".." => Err(PathError::DotName { component }),
                _ => check_name(name, component).map(|()| name.to_vec()),
            })
            .collect::<Result<_, _>>()
            .map(Self)
    }

    /// The names, from `/` down. The filesystem root has none.
    #[must_use]
    pub fn components(&self) -> &[Vec<u8>] {
        &self.0
    }

    /// The path as admins see it: `/` and each name decoded as described in
    /// [`display_name`].
    #[must_use]
    pub fn display(&self) -> String {
        format!("/{}", joined(&self.0))
    }

    /// The path beneath `root` that leads here, or `None` when this path is
    /// not `root` or beneath it. Whole names are compared, so `/srv/musicals`
    /// is not beneath `/srv/music`.
    #[must_use]
    pub fn beneath(&self, root: &Self) -> Option<RelPath> {
        self.0
            .strip_prefix(root.0.as_slice())
            .map(|rest| RelPath(rest.to_vec()))
    }
}

/// Why a path was refused. `component` counts names from zero and `offset`
/// counts bytes from the start of that name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathError {
    /// An absolute path did not start with `/`.
    NotAbsolute,
    /// A name was empty, as between the two slashes of `a//b` or before the
    /// first slash of an absolute path.
    EmptyName {
        /// Which name.
        component: usize,
    },
    /// An absolute path held `.` or `..`, so it was not canonical.
    DotName {
        /// Which name.
        component: usize,
    },
    /// A name held `/`, so it was more than one name.
    Separator {
        /// Which name.
        component: usize,
        /// Where the `/` is.
        offset: usize,
    },
    /// A name held a NUL byte, which no file name can hold.
    Nul {
        /// Which name.
        component: usize,
        /// Where the NUL byte is.
        offset: usize,
    },
    /// A `..` would climb above the root.
    Escapes {
        /// Which name.
        component: usize,
    },
}

/// Reads the names of a path beneath a root, as a walk found them or as a
/// playlist entry split them: `.` is dropped and `..` removes the name
/// before it.
///
/// # Errors
///
/// A [`PathError`] when a name is empty or holds `/` or NUL, or when a
/// `..` would climb above the root.
pub fn normalise(components: Untrusted<&[&[u8]]>) -> Result<RelPath, PathError> {
    let mut names: Vec<Vec<u8>> = Vec::new();
    for (component, &name) in components.into_inner().iter().enumerate() {
        match name {
            [] => return Err(PathError::EmptyName { component }),
            b"." => {}
            b".." => {
                names.pop().ok_or(PathError::Escapes { component })?;
            }
            _ => {
                check_name(name, component)?;
                names.push(name.to_vec());
            }
        }
    }
    Ok(RelPath(names))
}

/// Why a folder cannot be a library root, or an approved link target, which
/// is a root too (SEC-MED-037). The folder picker shows the reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootRefusal {
    /// The folder is `/`, which holds everything on the machine.
    FilesystemRoot,
    /// The folder is a system directory or lies inside one.
    SystemDirectory {
        /// The system directory, such as `/etc`.
        dir: &'static str,
    },
    /// The folder overlaps one of Gunmetal's own data, cache, configuration
    /// or log directories, so their files could be served.
    DataDirectory {
        /// Which of the directories `refuse_root` was given.
        index: usize,
        /// How the folder overlaps it.
        overlap: Overlap,
    },
}

impl Describe for RootRefusal {
    /// The reason the folder picker shows (SEC-MED-037).
    fn problem(&self) -> Problem {
        let (code, args) = match *self {
            Self::FilesystemRoot => (ProblemCode::LibraryRootFilesystemRoot, Vec::new()),
            Self::SystemDirectory { dir } => (
                ProblemCode::LibraryRootSystemFolder,
                vec![("folder", Arg::Name(dir))],
            ),
            Self::DataDirectory { overlap, .. } => (
                ProblemCode::LibraryRootOwnData,
                vec![("overlap", Arg::Name(overlap.name()))],
            ),
        };
        Problem { code, args }
    }
}

/// How a folder overlaps one of Gunmetal's own directories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlap {
    /// The folder is the directory.
    Equal,
    /// The directory lies inside the folder.
    Contains,
    /// The folder lies inside the directory.
    Inside,
}

impl Overlap {
    /// The overlap as a problem argument.
    const fn name(self) -> &'static str {
        match self {
            Self::Equal => "equal",
            Self::Contains => "contains",
            Self::Inside => "inside",
        }
    }
}

/// Whether `candidate` may not be a library root, and why: it is `/`, a
/// Linux system directory or inside one, or it equals, contains or lies
/// inside one of `data_dirs`, Gunmetal's own data, cache, configuration and
/// log directories (SEC-MED-037). Both are compared as canonical paths;
/// the filesystem crate also compares device and inode.
///
/// The system directories are those media-and-parser-safety.md lists for
/// Linux, where R1 servers run (D-09): `/bin`, `/boot`, `/dev`, `/etc`,
/// the `/lib` directories, `/proc`, `/root`, `/run`, `/sbin`, `/sys` and
/// `/usr`. Inside one is refused too, so `/proc/1/root`, which leads back
/// to `/`, cannot stand in for it. `/mnt`, `/media`, `/srv`, `/home` and
/// FUSE mounts such as Unraid's `/mnt/user` are allowed.
#[must_use]
pub fn refuse_root(candidate: &RawPath, data_dirs: &[RawPath]) -> Option<RootRefusal> {
    let Some(top) = candidate.0.first() else {
        return Some(RootRefusal::FilesystemRoot);
    };
    if let Some(&dir) = SYSTEM_DIRS
        .iter()
        .find(|dir| dir.as_bytes().get(1..) == Some(top.as_slice()))
    {
        return Some(RootRefusal::SystemDirectory { dir });
    }
    data_dirs.iter().enumerate().find_map(|(index, dir)| {
        let overlap = match (
            candidate.0.starts_with(&dir.0),
            dir.0.starts_with(&candidate.0),
        ) {
            (true, true) => Overlap::Equal,
            (false, true) => Overlap::Contains,
            (true, false) => Overlap::Inside,
            (false, false) => return None,
        };
        Some(RootRefusal::DataDirectory { index, overlap })
    })
}

/// The Linux system directories of [`refuse_root`], each a name beneath
/// `/`.
const SYSTEM_DIRS: [&str; 14] = [
    "/bin", "/boot", "/dev", "/etc", "/lib", "/lib32", "/lib64", "/libx32", "/proc", "/root",
    "/run", "/sbin", "/sys", "/usr",
];

/// Where one symbolic link leads, read without touching the filesystem:
/// `text` is what the link holds, and `dir` is the directory holding the
/// link, beneath `root`. An absolute link leads from `/`; a relative one
/// from `dir`. Empty names and `.` are dropped, and `..` removes the name
/// before it, stopping at `/` as the kernel does.
///
/// This is one step of a chain. The filesystem crate follows a relative
/// link that stays beneath its root through the kernel's
/// `RESOLVE_BENEATH`, and resolves any other with this function and
/// [`classify_link`], then opens what is left beneath the root that
/// verdict names, so the kernel still confines the final open (SEC-MED-034,
/// SEC-TM-043).
///
/// # Errors
///
/// [`PathError::Nul`] when the text holds a NUL byte; `component` counts
/// the names of the text, after a leading `/`.
pub fn link_target(
    root: &RawPath,
    dir: &RelPath,
    text: Untrusted<&[u8]>,
) -> Result<RawPath, PathError> {
    let text = text.into_inner();
    let (mut names, rest) = match text.strip_prefix(b"/") {
        Some(rest) => (Vec::new(), rest),
        None => (root.0.iter().chain(&dir.0).cloned().collect(), text),
    };
    for (component, name) in rest.split(|&byte| byte == b'/').enumerate() {
        match name {
            [] | b"." => {}
            b".." => {
                names.pop();
            }
            _ => {
                check_name(name, component)?;
                names.push(name.to_vec());
            }
        }
    }
    Ok(RawPath(names))
}

/// Whether a link whose target is `target` may be followed (SEC-MED-034).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkVerdict {
    /// Follow it: the target is beneath the library's own root, at this
    /// path.
    Own(RelPath),
    /// Follow it: the target is beneath a target root the admin approved
    /// for this library.
    Approved {
        /// Which approved root.
        index: usize,
        /// The path of the target beneath it.
        rest: RelPath,
    },
    /// Skip it: the target is in another library, whose files this
    /// library's users may not be allowed to see.
    OtherLibrary {
        /// Which other library's root.
        index: usize,
    },
    /// Skip it: the target is anywhere else, such as `/etc/passwd` or
    /// Gunmetal's own database.
    Outside,
}

/// Decides whether a link whose target is `target` may be followed, for a
/// library rooted at `own` with the `approved` target roots the admin
/// allowed for it, where `others` are the roots of every other library
/// (SEC-MED-034).
///
/// A target beneath `own` is followed. Otherwise the deepest root that
/// holds the target decides, so a link into another library is skipped
/// even when an approved root holds that library, unless the admin
/// approved that library's root, or a root inside it, explicitly. Among
/// equal roots, the first listed is named.
#[must_use]
pub fn classify_link(
    target: &RawPath,
    own: &RawPath,
    approved: &[RawPath],
    others: &[RawPath],
) -> LinkVerdict {
    if let Some(rest) = target.beneath(own) {
        return LinkVerdict::Own(rest);
    }
    match (deepest(target, approved), deepest(target, others)) {
        (Some((index, rest)), other)
            if other
                .as_ref()
                .is_none_or(|(_, beyond)| rest.0.len() <= beyond.0.len()) =>
        {
            LinkVerdict::Approved { index, rest }
        }
        (_, Some((index, _))) => LinkVerdict::OtherLibrary { index },
        (_, None) => LinkVerdict::Outside,
    }
}

/// The deepest of `roots` that holds `target`, with the path of the target
/// beneath it: the one that leaves the shortest path, the first of equals.
fn deepest(target: &RawPath, roots: &[RawPath]) -> Option<(usize, RelPath)> {
    roots
        .iter()
        .enumerate()
        .filter_map(|(index, root)| target.beneath(root).map(|rest| (index, rest)))
        .min_by_key(|(_, rest)| rest.0.len())
}

/// The name of a file the server writes in its data directory, such as an
/// artwork derivative or an extracted attachment (SEC-MED-039,
/// SEC-HIS-015). Only a SHA-256 digest of the file's content or a random
/// public identifier can make one, so no name read from media, metadata, a
/// client or an upload, such as a Matroska attachment's `../../etc/cron.d/x`
/// (Jellyfin CVE-2026-49246), can become part of a path. A digest is
/// written as 64 lower-case hexadecimal digits and an identifier as its
/// one text form, so the name is always one plain file name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StorageName(String);

impl StorageName {
    /// The name of content whose SHA-256 digest is `digest`, from
    /// [`crate::crypto::sha256`].
    #[must_use]
    pub fn from_sha256(digest: [u8; 32]) -> Self {
        Self(digest.iter().fold(String::new(), |mut name, &byte| {
            name.push(hex_digit(byte.wrapping_shr(4)));
            name.push(hex_digit(byte & 0x0F));
            name
        }))
    }

    /// The name of something the server identifies by `id`, an identifier
    /// minted from the operating system's CSPRNG.
    #[must_use]
    pub fn from_id(id: PublicId) -> Self {
        Self(id.to_string())
    }

    /// The name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A name as people see it (SEC-MED-040). The bytes are decoded as UTF-8
/// with each invalid sequence replaced by U+FFFD, and each character that
/// could break or reorder the text around it is written as its Rust escape,
/// such as `\u{a}` for a line feed or `\u{202e}` for a right-to-left
/// override: the C0 and C1 controls, and the characters a single-line text
/// field removes (`crate::text::Lines::Single`). The name used to open the
/// file stays the bytes.
#[must_use]
pub fn display_name(name: Untrusted<&[u8]>) -> String {
    escaped(name.into_inner())
}

/// The display form of one name, as [`display_name`] describes it.
fn escaped(name: &[u8]) -> String {
    let mut shown = String::new();
    for c in String::from_utf8_lossy(name).chars() {
        if c.is_control() || matches!(c, '\u{2028}'..='\u{202E}' | '\u{2066}'..='\u{2069}') {
            shown.extend(c.escape_unicode());
        } else {
            shown.push(c);
        }
    }
    shown
}

/// The display forms of `names`, separated by `/`.
fn joined(names: &[Vec<u8>]) -> String {
    names
        .iter()
        .map(|name| escaped(name))
        .collect::<Vec<_>>()
        .join("/")
}

/// One hexadecimal digit of a SHA-256 digest: `nibble` is 0 to 15.
fn hex_digit(nibble: u8) -> char {
    if nibble < 10 {
        char::from(nibble.wrapping_add(b'0'))
    } else {
        char::from(nibble.wrapping_add(b'a').wrapping_sub(10))
    }
}

/// Refuses a name that holds `/` or NUL, at the first such byte.
fn check_name(name: &[u8], component: usize) -> Result<(), PathError> {
    name.iter()
        .enumerate()
        .try_for_each(|(offset, &byte)| match byte {
            b'/' => Err(PathError::Separator { component, offset }),
            0 => Err(PathError::Nul { component, offset }),
            _ => Ok(()),
        })
}

/// Compile-fail tests: what code outside the core must not be able to do
/// with a path or a storage name. Rustdoc on stable does not check which
/// error a compile-fail test produced, so each shares its imports with the
/// control below, which compiles; a mistake in the imports would fail the
/// control.
#[cfg(doctest)]
mod compile_fail {
    /// Control: outside the core, paths come from reading untrusted names
    /// and storage names from a digest or an identifier.
    ///
    /// ```
    /// use gunmetal_core::id::PublicId;
    /// use gunmetal_core::path::{RawPath, RelPath, StorageName, normalise};
    /// use gunmetal_core::untrusted::Untrusted;
    ///
    /// fn paths(names: &[&[u8]], text: &[u8]) -> (Option<RelPath>, Option<RawPath>) {
    ///     (
    ///         normalise(Untrusted::new(names)).ok(),
    ///         RawPath::parse(Untrusted::new(text)).ok(),
    ///     )
    /// }
    ///
    /// fn storage(digest: [u8; 32], id: PublicId) -> (StorageName, StorageName) {
    ///     (StorageName::from_sha256(digest), StorageName::from_id(id))
    /// }
    /// ```
    struct Control;

    /// Verifies: SEC-MED-039, SEC-HIS-015
    ///
    /// No conversion makes a storage name from text, so a name read from a
    /// media file cannot become one.
    ///
    /// ```compile_fail
    /// use gunmetal_core::id::PublicId;
    /// use gunmetal_core::path::{RawPath, RelPath, StorageName, normalise};
    /// use gunmetal_core::untrusted::Untrusted;
    ///
    /// fn from_text(name: String) -> StorageName {
    ///     StorageName::from(name)
    /// }
    /// ```
    struct NoStorageNameFromText;

    /// Verifies: SEC-MED-039, SEC-HIS-015
    ///
    /// The field is private, so a struct literal cannot make one either.
    ///
    /// ```compile_fail
    /// use gunmetal_core::id::PublicId;
    /// use gunmetal_core::path::{RawPath, RelPath, StorageName, normalise};
    /// use gunmetal_core::untrusted::Untrusted;
    ///
    /// fn literal() -> StorageName {
    ///     StorageName(String::from("../../etc/cron.d/x"))
    /// }
    /// ```
    struct NoStorageNameLiteral;

    /// Verifies: SEC-TM-043
    ///
    /// A relative path that climbs out of its root cannot be written as a
    /// literal; only `normalise` makes one.
    ///
    /// ```compile_fail
    /// use gunmetal_core::id::PublicId;
    /// use gunmetal_core::path::{RawPath, RelPath, StorageName, normalise};
    /// use gunmetal_core::untrusted::Untrusted;
    ///
    /// fn literal() -> RelPath {
    ///     RelPath(vec![b"..".to_vec(), b"etc".to_vec()])
    /// }
    /// ```
    struct NoRelPathLiteral;

    /// Verifies: SEC-TM-043
    ///
    /// Nor can an absolute path that is not canonical; only `parse` and
    /// `link_target` make one.
    ///
    /// ```compile_fail
    /// use gunmetal_core::id::PublicId;
    /// use gunmetal_core::path::{RawPath, RelPath, StorageName, normalise};
    /// use gunmetal_core::untrusted::Untrusted;
    ///
    /// fn literal() -> RawPath {
    ///     RawPath(vec![b"srv".to_vec(), b"..".to_vec()])
    /// }
    /// ```
    struct NoRawPathLiteral;
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles work with small, bounded positions"
)]
mod tests {
    use super::*;
    use proptest::collection::vec;
    use proptest::prelude::*;

    /// Normalises `names` as they arrive from a walk or a playlist.
    fn rel(names: &[&[u8]]) -> Result<RelPath, PathError> {
        normalise(Untrusted::new(names))
    }

    /// The names a normalised path holds, as owned byte strings.
    fn names(names: &[&[u8]]) -> Vec<Vec<u8>> {
        names.iter().map(|name| name.to_vec()).collect()
    }

    #[test]
    fn keeps_ordinary_names_in_order() {
        let path = rel(&[b"Artist", b"Album (2001)", b"01 Track.flac"]).unwrap();
        assert_eq!(
            path.components(),
            names(&[b"Artist", b"Album (2001)", b"01 Track.flac"])
        );
    }

    #[test]
    fn no_names_is_the_root_itself() {
        assert_eq!(rel(&[]), Ok(RelPath(Vec::new())));
    }

    /// Verifies: SEC-MED-040
    #[test]
    fn keeps_names_that_are_not_utf8_as_their_bytes() {
        let path = rel(&[b"Bj\xF6rk", b"\xFF\xFE", b"caf\xC3\xA9"]).unwrap();
        assert_eq!(
            path.components(),
            names(&[b"Bj\xF6rk", b"\xFF\xFE", b"caf\xC3\xA9"])
        );
    }

    /// Verifies: SEC-MED-040
    #[test]
    fn keeps_names_with_newlines_and_escape_characters_as_their_bytes() {
        let path = rel(&[b"two\nlines", b"\x1B[31mred\x1B[0m", b"tab\there"]).unwrap();
        assert_eq!(
            path.components(),
            names(&[b"two\nlines", b"\x1B[31mred\x1B[0m", b"tab\there"])
        );
    }

    #[test]
    fn drops_a_dot_at_every_position() {
        for path in [
            &[&b"."[..], b"a", b"b"][..],
            &[b"a", b".", b"b"],
            &[b"a", b"b", b"."],
            &[b".", b"a", b".", b".", b"b", b"."],
        ] {
            assert_eq!(rel(path), Ok(RelPath(names(&[b"a", b"b"]))), "{path:?}");
        }
        assert_eq!(rel(&[b"."]), Ok(RelPath(Vec::new())));
    }

    /// Verifies: SEC-TM-043
    #[test]
    fn resolves_a_parent_at_every_position_that_stays_beneath_the_root() {
        assert_eq!(rel(&[b"a", b"..", b"b"]), Ok(RelPath(names(&[b"b"]))));
        assert_eq!(rel(&[b"a", b"b", b".."]), Ok(RelPath(names(&[b"a"]))));
        assert_eq!(rel(&[b"a", b"b", b"..", b".."]), Ok(RelPath(Vec::new())));
        assert_eq!(
            rel(&[b"a", b"b", b"..", b"c", b".", b"..", b"d"]),
            Ok(RelPath(names(&[b"a", b"d"])))
        );
    }

    /// Verifies: SEC-TM-043
    #[test]
    fn refuses_a_parent_that_climbs_above_the_root() {
        assert_eq!(rel(&[b".."]), Err(PathError::Escapes { component: 0 }));
        assert_eq!(
            rel(&[b"..", b"a"]),
            Err(PathError::Escapes { component: 0 })
        );
        assert_eq!(
            rel(&[b"a", b"..", b".."]),
            Err(PathError::Escapes { component: 2 })
        );
        assert_eq!(
            rel(&[b"a", b".", b"..", b"b", b"..", b"..", b"c"]),
            Err(PathError::Escapes { component: 5 })
        );
    }

    /// Verifies: SEC-TM-043
    #[test]
    fn refuses_an_empty_name_and_so_an_absolute_path() {
        assert_eq!(
            rel(&[b"", b"etc", b"passwd"]),
            Err(PathError::EmptyName { component: 0 })
        );
        assert_eq!(
            rel(&[b"a", b"", b"b"]),
            Err(PathError::EmptyName { component: 1 })
        );
        assert_eq!(
            rel(&[b"a", b""]),
            Err(PathError::EmptyName { component: 1 })
        );
    }

    /// A name read from inside a media file, such as a Matroska attachment's
    /// (Jellyfin CVE-2026-49246), or a subtitle upload's format field
    /// (CVE-2026-35031), is one name and cannot become a path.
    ///
    /// Verifies: SEC-MED-039, SEC-TM-043, SEC-HIS-015
    #[test]
    fn refuses_a_name_that_holds_a_separator() {
        assert_eq!(
            rel(&[b"../../etc/cron.d/x"]),
            Err(PathError::Separator {
                component: 0,
                offset: 2
            })
        );
        assert_eq!(
            rel(&[b"Subs", b"../../x"]),
            Err(PathError::Separator {
                component: 1,
                offset: 2
            })
        );
        assert_eq!(
            rel(&[b"a", b"b/"]),
            Err(PathError::Separator {
                component: 1,
                offset: 1
            })
        );
    }

    /// Verifies: SEC-TM-043
    #[test]
    fn refuses_a_name_that_holds_a_nul_byte() {
        assert_eq!(
            rel(&[b"ok", b"cover.jpg\0.flac"]),
            Err(PathError::Nul {
                component: 1,
                offset: 9
            })
        );
        assert_eq!(
            rel(&[b"\0"]),
            Err(PathError::Nul {
                component: 0,
                offset: 0
            })
        );
    }

    #[test]
    fn reports_the_first_forbidden_byte_of_a_name() {
        assert_eq!(
            rel(&[b"x\0/"]),
            Err(PathError::Nul {
                component: 0,
                offset: 1
            })
        );
        assert_eq!(
            rel(&[b"x/\0"]),
            Err(PathError::Separator {
                component: 0,
                offset: 1
            })
        );
    }

    #[test]
    fn reports_the_first_refused_name() {
        assert_eq!(
            rel(&[b"a", b"", b"b\0", b".."]),
            Err(PathError::EmptyName { component: 1 })
        );
        assert_eq!(
            rel(&[b"..", b"a/b"]),
            Err(PathError::Escapes { component: 0 })
        );
    }

    #[test]
    fn keeps_names_that_only_start_like_dot_or_dot_dot() {
        let path = rel(&[b"...", b".hidden", b".. ", b"..x", b" .."]).unwrap();
        assert_eq!(
            path.components(),
            names(&[b"...", b".hidden", b".. ", b"..x", b" .."])
        );
    }

    /// Pieces of hostile paths: traversal, absolute and UNC prefixes, device
    /// names, NUL bytes, both separators, names that are not UTF-8, and
    /// ordinary names, as one name each.
    fn piece() -> impl Strategy<Value = Vec<u8>> {
        prop_oneof![
            Just(b"".to_vec()),
            Just(b".".to_vec()),
            Just(b"..".to_vec()),
            Just(b"/".to_vec()),
            Just(b"\\".to_vec()),
            Just(b"..\\..".to_vec()),
            Just(b"\\\\server\\share".to_vec()),
            Just(b"\\\\?\\C:".to_vec()),
            Just(b"c:".to_vec()),
            Just(b"CON".to_vec()),
            Just(b"nul.txt".to_vec()),
            Just(b"\0".to_vec()),
            Just(b"a/../..".to_vec()),
            Just(b"\xFF\xFE".to_vec()),
            Just(b"line\nbreak".to_vec()),
            "[a-z]{1,3}".prop_map(String::into_bytes),
            vec(any::<u8>(), 0..6),
        ]
    }

    /// An independent statement of normalisation: every name is non-empty
    /// and holds neither `/` nor NUL, and counting each ordinary name as one
    /// step down and each `..` as one step up never goes above the root.
    /// It returns how deep the path ends, or `None` when it is refused.
    fn depth_oracle(pieces: &[Vec<u8>]) -> Option<usize> {
        let mut depth = 0_usize;
        for piece in pieces {
            if piece.is_empty() || piece.contains(&b'/') || piece.contains(&0) {
                return None;
            }
            if piece == b".." {
                depth = depth.checked_sub(1)?;
            } else if piece != b"." {
                depth = depth.saturating_add(1);
            }
        }
        Some(depth)
    }

    /// Reads an absolute path as the filesystem crate would hand it over.
    fn raw(text: &[u8]) -> Result<RawPath, PathError> {
        RawPath::parse(Untrusted::new(text))
    }

    /// An absolute path built directly from its names, so that tests of
    /// containment do not rely on the parser.
    fn abs(names: &[&[u8]]) -> RawPath {
        RawPath(self::names(names))
    }

    #[test]
    fn reads_the_filesystem_root_and_paths_beneath_it() {
        assert_eq!(raw(b"/"), Ok(RawPath(Vec::new())));
        assert_eq!(raw(b"/srv"), Ok(abs(&[b"srv"])));
        assert_eq!(
            raw(b"/mnt/user/Music").map(|path| path.components().to_vec()),
            Ok(names(&[b"mnt", b"user", b"Music"]))
        );
    }

    /// Verifies: SEC-MED-040
    #[test]
    fn reads_absolute_names_that_are_not_utf8_or_hold_controls() {
        assert_eq!(
            raw(b"/mnt/Bj\xF6rk/two\nlines\x1B"),
            Ok(abs(&[b"mnt", b"Bj\xF6rk", b"two\nlines\x1B"]))
        );
    }

    /// A Windows drive root, a UNC share and the `\\?\` prefix are not
    /// absolute paths on the Linux server, so none can become a root.
    ///
    /// Verifies: SEC-TM-043
    #[test]
    fn refuses_a_path_that_does_not_start_at_the_root() {
        for text in [
            &b""[..],
            b"srv/music",
            b"./srv",
            b"../etc",
            b"C:\\",
            b"C:\\Music",
            b"\\\\server\\share",
            b"\\\\?\\C:\\Music",
            b" /srv",
        ] {
            assert_eq!(raw(text), Err(PathError::NotAbsolute), "{text:?}");
        }
    }

    /// Verifies: SEC-TM-043
    #[test]
    fn refuses_an_absolute_path_that_is_not_canonical() {
        assert_eq!(raw(b"//srv"), Err(PathError::EmptyName { component: 0 }));
        assert_eq!(raw(b"/srv/"), Err(PathError::EmptyName { component: 1 }));
        assert_eq!(
            raw(b"/srv//music"),
            Err(PathError::EmptyName { component: 1 })
        );
        assert_eq!(raw(b"/."), Err(PathError::DotName { component: 0 }));
        assert_eq!(raw(b"/.."), Err(PathError::DotName { component: 0 }));
        assert_eq!(
            raw(b"/srv/./music"),
            Err(PathError::DotName { component: 1 })
        );
        assert_eq!(
            raw(b"/srv/music/../../etc"),
            Err(PathError::DotName { component: 2 })
        );
        assert_eq!(
            raw(b"/srv/mu\0sic/a"),
            Err(PathError::Nul {
                component: 1,
                offset: 2
            })
        );
        assert_eq!(
            raw(b"/a/\0/.."),
            Err(PathError::Nul {
                component: 1,
                offset: 0
            })
        );
    }

    #[test]
    fn keeps_absolute_names_that_only_start_like_dot_or_dot_dot() {
        assert_eq!(
            raw(b"/.config/.../..x/. "),
            Ok(abs(&[b".config", b"...", b"..x", b". "]))
        );
    }

    /// Verifies: SEC-TM-043
    #[test]
    fn finds_the_path_beneath_a_root() {
        let music = abs(&[b"srv", b"music"]);
        assert_eq!(
            abs(&[b"srv", b"music", b"a", b"b.flac"]).beneath(&music),
            Some(RelPath(names(&[b"a", b"b.flac"])))
        );
        assert_eq!(music.beneath(&music), Some(RelPath(Vec::new())));
        assert_eq!(
            music.beneath(&RawPath(Vec::new())),
            Some(RelPath(names(&[b"srv", b"music"])))
        );
    }

    /// A string-prefix check, as in Tautulli's CVE-2025-58761, would put
    /// `/srv/musicals` beneath `/srv/music`. Whole names are compared.
    ///
    /// Verifies: SEC-TM-043
    #[test]
    fn finds_nothing_beneath_a_root_that_is_only_a_prefix_of_a_name() {
        let music = abs(&[b"srv", b"music"]);
        assert_eq!(abs(&[b"srv", b"musicals", b"x"]).beneath(&music), None);
        assert_eq!(abs(&[b"srv", b"mus"]).beneath(&music), None);
        assert_eq!(abs(&[b"srv"]).beneath(&music), None);
        assert_eq!(RawPath(Vec::new()).beneath(&music), None);
        assert_eq!(abs(&[b"var", b"srv", b"music"]).beneath(&music), None);
    }

    /// Shows one name as people see it.
    fn shown(name: &[u8]) -> String {
        display_name(Untrusted::new(name))
    }

    #[test]
    fn shows_a_printable_name_as_it_is() {
        assert_eq!(shown(b"01 Track.flac"), "01 Track.flac");
        assert_eq!(shown(b"AC\\DC - Back in Black"), "AC\\DC - Back in Black");
        assert_eq!(shown("Sigur Rós ✓".as_bytes()), "Sigur Rós ✓");
        assert_eq!(shown(b""), "");
    }

    /// Verifies: SEC-MED-040
    #[test]
    fn shows_bytes_that_are_not_utf8_as_replacement_characters() {
        assert_eq!(shown(b"Bj\xF6rk"), "Bj\u{FFFD}rk");
        assert_eq!(shown(b"\xFF\xFE"), "\u{FFFD}\u{FFFD}");
        assert_eq!(shown(b"caf\xC3"), "caf\u{FFFD}");
    }

    /// Verifies: SEC-MED-040
    #[test]
    fn escapes_newlines_escape_characters_and_other_controls() {
        assert_eq!(shown(b"two\nlines"), "two\\u{a}lines");
        assert_eq!(shown(b"\x1B[31mred\x1B[0m"), "\\u{1b}[31mred\\u{1b}[0m");
        assert_eq!(shown(b"tab\there\r"), "tab\\u{9}here\\u{d}");
        assert_eq!(shown(b"\x00\x7F"), "\\u{0}\\u{7f}");
        // NEL and CSI, the C1 controls, written in UTF-8.
        assert_eq!(shown(b"\xC2\x85\xC2\x9B"), "\\u{85}\\u{9b}");
    }

    /// A right-to-left override can make `evil\u{202e}calf.exe` show as
    /// `evilexe.flac` (CVE-2021-42574).
    ///
    /// Verifies: SEC-MED-040
    #[test]
    fn escapes_characters_that_reorder_or_break_the_text_around_them() {
        assert_eq!(
            shown("evil\u{202E}calf.exe".as_bytes()),
            "evil\\u{202e}calf.exe"
        );
        assert_eq!(
            shown("\u{2028}\u{2029}\u{202A}\u{2066}\u{2069}".as_bytes()),
            "\\u{2028}\\u{2029}\\u{202a}\\u{2066}\\u{2069}"
        );
        // The neighbours of those ranges stay as they are.
        assert_eq!(
            shown("\u{2027}\u{202F}\u{2065}\u{206A}".as_bytes()),
            "\u{2027}\u{202F}\u{2065}\u{206A}"
        );
    }

    #[test]
    fn shows_a_relative_path_with_slashes_between_its_names() {
        let path = rel(&[b"Bj\xF6rk", b"two\nlines", b"x.flac"]).unwrap();
        assert_eq!(path.display(), "Bj\u{FFFD}rk/two\\u{a}lines/x.flac");
        assert_eq!(rel(&[b"one"]).unwrap().display(), "one");
        assert_eq!(rel(&[]).unwrap().display(), "");
    }

    #[test]
    fn shows_an_absolute_path_from_the_root() {
        assert_eq!(
            abs(&[b"mnt", b"Bj\xF6rk", b"\x1B"]).display(),
            "/mnt/Bj\u{FFFD}rk/\\u{1b}"
        );
        assert_eq!(abs(&[b"srv"]).display(), "/srv");
        assert_eq!(RawPath(Vec::new()).display(), "/");
    }

    proptest! {
        /// Whatever a name holds, what people see holds no control and no
        /// character the single-line text rule removes: the two rules agree
        /// on every character.
        ///
        /// Verifies: SEC-MED-040
        #[test]
        fn escapes_exactly_what_a_single_line_field_removes(c in any::<char>()) {
            let mut bytes = [0; 4];
            let encoded = c.encode_utf8(&mut bytes).as_bytes();
            let removed = crate::text::normalise(
                Untrusted::new(encoded),
                crate::text::Lines::Single,
                8,
            )
            .value
            .is_empty();
            let expected = if removed {
                c.escape_unicode().to_string()
            } else {
                c.to_string()
            };
            prop_assert_eq!(shown(encoded), expected);
        }

        /// Verifies: SEC-MED-040
        #[test]
        fn shows_no_control_whatever_the_bytes(name in vec(any::<u8>(), 0..24)) {
            let shown = shown(&name);
            prop_assert!(!shown.chars().any(char::is_control), "{:?}", shown);
            prop_assert!(
                !shown.contains(['\u{2028}', '\u{2029}', '\u{202A}', '\u{202B}', '\u{202C}',
                    '\u{202D}', '\u{202E}', '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}']),
                "{:?}",
                shown
            );
        }
    }

    /// Reads a canonical absolute path written in a test.
    fn at(text: &str) -> RawPath {
        raw(text.as_bytes()).unwrap()
    }

    /// The Linux system directories media-and-parser-safety.md section 5
    /// lists, with `/lib*` written out as the directories distributions
    /// use, independently of the code.
    const SYSTEM_DIRS: [&str; 14] = [
        "/bin", "/boot", "/dev", "/etc", "/lib", "/lib32", "/lib64", "/libx32", "/proc", "/root",
        "/run", "/sbin", "/sys", "/usr",
    ];

    /// Gunmetal's own directories in a typical install: data, cache,
    /// configuration and logs.
    fn own_dirs() -> [RawPath; 4] {
        [
            at("/var/lib/gunmetal"),
            at("/var/cache/gunmetal"),
            at("/srv/gunmetal/config"),
            at("/var/log/gunmetal"),
        ]
    }

    /// Verifies: SEC-MED-037
    #[test]
    fn refuses_the_filesystem_root() {
        assert_eq!(
            refuse_root(&at("/"), &[]),
            Some(RootRefusal::FilesystemRoot)
        );
        assert_eq!(
            refuse_root(&at("/"), &own_dirs()),
            Some(RootRefusal::FilesystemRoot)
        );
    }

    /// Verifies: SEC-MED-037
    #[test]
    fn refuses_each_system_directory() {
        for dir in SYSTEM_DIRS {
            assert_eq!(
                refuse_root(&at(dir), &[]),
                Some(RootRefusal::SystemDirectory { dir }),
                "{dir}"
            );
        }
    }

    /// `/proc/1/root` leads back to `/`, `/run/secrets` holds container
    /// secrets and `/etc/ssl/private` holds keys.
    ///
    /// Verifies: SEC-MED-037
    #[test]
    fn refuses_a_folder_inside_a_system_directory() {
        for (folder, dir) in [
            ("/proc/1/root", "/proc"),
            ("/proc/self/root/home/music", "/proc"),
            ("/etc/ssl/private", "/etc"),
            ("/run/secrets", "/run"),
            ("/run/media/justin/usb", "/run"),
            ("/usr/share/sounds", "/usr"),
            ("/root/Music", "/root"),
            ("/dev/shm", "/dev"),
            ("/sys/fs/cgroup", "/sys"),
            ("/lib64/x", "/lib64"),
        ] {
            assert_eq!(
                refuse_root(&at(folder), &[]),
                Some(RootRefusal::SystemDirectory { dir }),
                "{folder}"
            );
        }
    }

    /// Unraid's `/mnt/user` is a FUSE mount, which the owner allowed on
    /// 2026-10-03. Names that only start like a system directory, such as
    /// a Docker mount at `/library`, are not one, and Linux names are case
    /// sensitive.
    ///
    /// Verifies: SEC-MED-037
    #[test]
    fn accepts_media_folders_outside_the_system_directories() {
        for folder in [
            "/mnt/user/Music",
            "/mnt/user",
            "/mnt",
            "/media/usb/Music",
            "/srv/music",
            "/home/justin/Music",
            "/var/media",
            "/data",
            "/music",
            "/library",
            "/libraries/music",
            "/lib-music",
            "/etcetera",
            "/usr2",
            "/user",
            "/binary",
            "/ETC",
            "/Run",
        ] {
            assert_eq!(refuse_root(&at(folder), &own_dirs()), None, "{folder}");
        }
    }

    /// Verifies: SEC-MED-037
    #[test]
    fn refuses_a_folder_that_is_gunmetals_own_directory() {
        assert_eq!(
            refuse_root(&at("/var/lib/gunmetal"), &own_dirs()),
            Some(RootRefusal::DataDirectory {
                index: 0,
                overlap: Overlap::Equal
            })
        );
        assert_eq!(
            refuse_root(&at("/var/log/gunmetal"), &own_dirs()),
            Some(RootRefusal::DataDirectory {
                index: 3,
                overlap: Overlap::Equal
            })
        );
    }

    /// Verifies: SEC-MED-037
    #[test]
    fn refuses_a_folder_that_contains_gunmetals_own_directory() {
        assert_eq!(
            refuse_root(&at("/var/lib"), &own_dirs()),
            Some(RootRefusal::DataDirectory {
                index: 0,
                overlap: Overlap::Contains
            })
        );
        assert_eq!(
            refuse_root(&at("/var/cache"), &own_dirs()),
            Some(RootRefusal::DataDirectory {
                index: 1,
                overlap: Overlap::Contains
            })
        );
        // `/srv` holds both the configuration and, in this install, music;
        // it is refused, for the first directory it contains.
        assert_eq!(
            refuse_root(&at("/srv"), &own_dirs()),
            Some(RootRefusal::DataDirectory {
                index: 2,
                overlap: Overlap::Contains
            })
        );
        assert_eq!(
            refuse_root(&at("/var"), &own_dirs()),
            Some(RootRefusal::DataDirectory {
                index: 0,
                overlap: Overlap::Contains
            })
        );
    }

    /// Verifies: SEC-MED-037
    #[test]
    fn refuses_a_folder_that_lies_inside_gunmetals_own_directory() {
        assert_eq!(
            refuse_root(&at("/var/lib/gunmetal/backups"), &own_dirs()),
            Some(RootRefusal::DataDirectory {
                index: 0,
                overlap: Overlap::Inside
            })
        );
        assert_eq!(
            refuse_root(&at("/srv/gunmetal/config/tls/keys"), &own_dirs()),
            Some(RootRefusal::DataDirectory {
                index: 2,
                overlap: Overlap::Inside
            })
        );
    }

    /// Verifies: SEC-MED-037
    #[test]
    fn accepts_a_folder_whose_name_only_starts_like_gunmetals_own() {
        for folder in [
            "/var/lib/gunmetal-music",
            "/var/lib/gunmeta",
            "/var/libs",
            "/srv/gunmetal/configs",
            "/srv/music",
        ] {
            assert_eq!(refuse_root(&at(folder), &own_dirs()), None, "{folder}");
        }
    }

    /// Verifies: SEC-MED-037
    #[test]
    fn describes_each_refusal_for_the_folder_picker() {
        assert_eq!(
            RootRefusal::FilesystemRoot.problem(),
            Problem {
                code: ProblemCode::LibraryRootFilesystemRoot,
                args: Vec::new(),
            }
        );
        assert_eq!(
            RootRefusal::SystemDirectory { dir: "/proc" }.problem(),
            Problem {
                code: ProblemCode::LibraryRootSystemFolder,
                args: vec![("folder", Arg::Name("/proc"))],
            }
        );
        for (overlap, name) in [
            (Overlap::Equal, "equal"),
            (Overlap::Contains, "contains"),
            (Overlap::Inside, "inside"),
        ] {
            assert_eq!(
                RootRefusal::DataDirectory { index: 3, overlap }.problem(),
                Problem {
                    code: ProblemCode::LibraryRootOwnData,
                    args: vec![("overlap", Arg::Name(name))],
                }
            );
        }
    }

    /// A system directory is named before an overlap with Gunmetal's own
    /// directories, and the filesystem root before either.
    #[test]
    fn names_the_first_reason_that_applies() {
        let in_etc = [at("/etc/gunmetal")];
        assert_eq!(
            refuse_root(&at("/etc/gunmetal"), &in_etc),
            Some(RootRefusal::SystemDirectory { dir: "/etc" })
        );
        assert_eq!(
            refuse_root(&at("/"), &in_etc),
            Some(RootRefusal::FilesystemRoot)
        );
        let twice = [at("/srv/b"), at("/srv/a"), at("/srv")];
        assert_eq!(
            refuse_root(&at("/srv"), &twice),
            Some(RootRefusal::DataDirectory {
                index: 0,
                overlap: Overlap::Contains
            })
        );
    }

    /// Where a link holding `text`, in `Rock/Album` beneath `/srv/music`,
    /// leads.
    fn leads(text: &[u8]) -> Result<RawPath, PathError> {
        let dir = rel(&[b"Rock", b"Album"]).unwrap();
        link_target(&at("/srv/music"), &dir, Untrusted::new(text))
    }

    #[test]
    fn follows_a_relative_link_from_the_directory_that_holds_it() {
        assert_eq!(leads(b"01.flac"), Ok(at("/srv/music/Rock/Album/01.flac")));
        assert_eq!(leads(b"../../Pop/x.flac"), Ok(at("/srv/music/Pop/x.flac")));
        assert_eq!(
            leads(b"./disc 2/./02.flac"),
            Ok(at("/srv/music/Rock/Album/disc 2/02.flac"))
        );
        assert_eq!(leads(b"a/../b"), Ok(at("/srv/music/Rock/Album/b")));
    }

    #[test]
    fn follows_a_relative_link_out_of_the_root() {
        assert_eq!(
            leads(b"../../../../mnt/debrid/x.flac"),
            Ok(at("/mnt/debrid/x.flac"))
        );
        assert_eq!(leads(b"../../.."), Ok(at("/srv")));
    }

    #[test]
    fn stops_climbing_at_the_filesystem_root() {
        assert_eq!(
            leads(b"../../../../../../../../etc/passwd"),
            Ok(at("/etc/passwd"))
        );
        assert_eq!(leads(b"/../../etc"), Ok(at("/etc")));
        assert_eq!(leads(b"../../../../../.."), Ok(at("/")));
    }

    #[test]
    fn follows_an_absolute_link_from_the_filesystem_root() {
        assert_eq!(leads(b"/etc/passwd"), Ok(at("/etc/passwd")));
        assert_eq!(
            leads(b"//mnt/./debrid//x.flac/"),
            Ok(at("/mnt/debrid/x.flac"))
        );
        assert_eq!(leads(b"/"), Ok(at("/")));
    }

    #[test]
    fn an_empty_or_dot_link_leads_to_its_own_directory() {
        assert_eq!(leads(b""), Ok(at("/srv/music/Rock/Album")));
        assert_eq!(leads(b"."), Ok(at("/srv/music/Rock/Album")));
        assert_eq!(leads(b"./"), Ok(at("/srv/music/Rock/Album")));
    }

    #[test]
    fn a_link_in_the_root_itself_leads_from_the_root() {
        let target = link_target(
            &at("/srv/music"),
            &rel(&[]).unwrap(),
            Untrusted::new(b"../films/x"),
        );
        assert_eq!(target, Ok(at("/srv/films/x")));
    }

    /// Verifies: SEC-MED-040
    #[test]
    fn keeps_link_names_that_are_not_utf8_or_hold_controls() {
        assert_eq!(
            leads(b"/mnt/Bj\xF6rk/two\nlines\x1B"),
            Ok(abs(&[b"mnt", b"Bj\xF6rk", b"two\nlines\x1B"]))
        );
    }

    #[test]
    fn refuses_link_text_that_holds_a_nul_byte() {
        assert_eq!(
            leads(b"a\0b"),
            Err(PathError::Nul {
                component: 0,
                offset: 1
            })
        );
        assert_eq!(
            leads(b"/mnt/x\0/y"),
            Err(PathError::Nul {
                component: 1,
                offset: 1
            })
        );
        assert_eq!(
            leads(b"../\0"),
            Err(PathError::Nul {
                component: 1,
                offset: 0
            })
        );
    }

    /// The library under test, its approved target root and two other
    /// libraries.
    fn judge(target: &str) -> LinkVerdict {
        classify_link(
            &at(target),
            &at("/srv/music"),
            &[at("/mnt/debrid")],
            &[at("/srv/films"), at("/srv/music-kids")],
        )
    }

    /// Navidrome's GHSA-r5qr-m328-qcf4: `passwd.wav`, a link to
    /// `/etc/passwd`, became a playable track.
    ///
    /// Verifies: SEC-MED-034, SEC-TM-043
    #[test]
    fn skips_a_link_to_a_system_file() {
        let target = leads(b"/etc/passwd").unwrap();
        assert_eq!(judge_path(&target), LinkVerdict::Outside);
        assert_eq!(judge("/etc/passwd"), LinkVerdict::Outside);
    }

    /// The same verdicts, for a target already resolved.
    fn judge_path(target: &RawPath) -> LinkVerdict {
        classify_link(
            target,
            &at("/srv/music"),
            &[at("/mnt/debrid")],
            &[at("/srv/films"), at("/srv/music-kids")],
        )
    }

    /// Verifies: SEC-MED-034
    #[test]
    fn skips_a_link_to_gunmetals_own_database() {
        assert_eq!(
            judge("/var/lib/gunmetal/cache/library.db"),
            LinkVerdict::Outside
        );
    }

    /// Verifies: SEC-MED-034, SEC-TM-043
    #[test]
    fn follows_a_link_that_stays_beneath_its_own_root() {
        assert_eq!(
            judge("/srv/music/Pop/x.flac"),
            LinkVerdict::Own(rel(&[b"Pop", b"x.flac"]).unwrap())
        );
        assert_eq!(judge("/srv/music"), LinkVerdict::Own(rel(&[]).unwrap()));
    }

    /// Verifies: SEC-MED-034, SEC-TM-043
    #[test]
    fn follows_an_absolute_link_into_an_approved_root() {
        assert_eq!(
            judge("/mnt/debrid/Album/x.flac"),
            LinkVerdict::Approved {
                index: 0,
                rest: rel(&[b"Album", b"x.flac"]).unwrap()
            }
        );
        assert_eq!(
            judge("/mnt/debrid"),
            LinkVerdict::Approved {
                index: 0,
                rest: rel(&[]).unwrap()
            }
        );
    }

    /// Verifies: SEC-MED-034
    #[test]
    fn skips_a_link_into_another_library() {
        assert_eq!(
            judge("/srv/films/x.mkv"),
            LinkVerdict::OtherLibrary { index: 0 }
        );
        assert_eq!(
            judge("/srv/music-kids/x.flac"),
            LinkVerdict::OtherLibrary { index: 1 }
        );
        assert_eq!(judge("/srv/films"), LinkVerdict::OtherLibrary { index: 0 });
    }

    /// Whole names are compared, so `/mnt/debrid-old` is not beneath the
    /// approved `/mnt/debrid`, nor `/srv/musicals` beneath `/srv/music`.
    ///
    /// Verifies: SEC-MED-034, SEC-TM-043
    #[test]
    fn skips_a_link_whose_target_only_starts_like_a_root() {
        assert_eq!(judge("/mnt/debrid-old/x.flac"), LinkVerdict::Outside);
        assert_eq!(judge("/srv/musicals/x.flac"), LinkVerdict::Outside);
        assert_eq!(judge("/srv/film"), LinkVerdict::Outside);
        assert_eq!(judge("/"), LinkVerdict::Outside);
    }

    /// Approving `/mnt` does not approve another library inside it.
    ///
    /// Verifies: SEC-MED-034
    #[test]
    fn skips_a_link_into_another_library_inside_an_approved_root() {
        let verdict = classify_link(
            &at("/mnt/films/x.mkv"),
            &at("/srv/music"),
            &[at("/mnt")],
            &[at("/mnt/films")],
        );
        assert_eq!(verdict, LinkVerdict::OtherLibrary { index: 0 });
        let beside = classify_link(
            &at("/mnt/other/x.flac"),
            &at("/srv/music"),
            &[at("/mnt")],
            &[at("/mnt/films")],
        );
        assert_eq!(
            beside,
            LinkVerdict::Approved {
                index: 0,
                rest: rel(&[b"other", b"x.flac"]).unwrap()
            }
        );
    }

    /// The admin can approve another library's root, or a folder inside
    /// it, explicitly.
    ///
    /// Verifies: SEC-MED-034
    #[test]
    fn follows_a_link_into_another_library_the_admin_approved() {
        let at_its_root = classify_link(
            &at("/srv/films/x.mkv"),
            &at("/srv/music"),
            &[at("/srv/films")],
            &[at("/srv/films")],
        );
        assert_eq!(
            at_its_root,
            LinkVerdict::Approved {
                index: 0,
                rest: rel(&[b"x.mkv"]).unwrap()
            }
        );
        let inside_it = classify_link(
            &at("/srv/films/Shared/x.mkv"),
            &at("/srv/music"),
            &[at("/srv/films/Shared")],
            &[at("/srv/films")],
        );
        assert_eq!(
            inside_it,
            LinkVerdict::Approved {
                index: 0,
                rest: rel(&[b"x.mkv"]).unwrap()
            }
        );
    }

    /// A library inside this one is reached by walking, so a link into it
    /// is followed like any link beneath the root.
    #[test]
    fn follows_a_link_into_a_library_inside_its_own_root() {
        let verdict = classify_link(
            &at("/srv/music/Kids/x.flac"),
            &at("/srv/music"),
            &[],
            &[at("/srv/music/Kids")],
        );
        assert_eq!(
            verdict,
            LinkVerdict::Own(rel(&[b"Kids", b"x.flac"]).unwrap())
        );
    }

    #[test]
    fn names_the_deepest_root_that_holds_the_target_and_the_first_of_equals() {
        let approved = [at("/mnt"), at("/mnt/debrid"), at("/mnt/debrid")];
        assert_eq!(
            classify_link(&at("/mnt/debrid/x"), &at("/srv/music"), &approved, &[]),
            LinkVerdict::Approved {
                index: 1,
                rest: rel(&[b"x"]).unwrap()
            }
        );
        let others = [at("/srv"), at("/srv/films"), at("/srv/films")];
        assert_eq!(
            classify_link(&at("/srv/films/x"), &at("/srv/music"), &[], &others),
            LinkVerdict::OtherLibrary { index: 1 }
        );
    }

    /// SHA-256 of `abc`, the first example of FIPS 180-4.
    const ABC: [u8; 32] = [
        0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae, 0x22,
        0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61, 0xf2, 0x00,
        0x15, 0xad,
    ];

    /// Verifies: SEC-MED-039, SEC-HIS-015
    #[test]
    fn names_content_by_its_digest_in_lower_case_hexadecimal() {
        assert_eq!(
            StorageName::from_sha256(ABC).as_str(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            StorageName::from_sha256([0; 32]).as_str(),
            "0000000000000000000000000000000000000000000000000000000000000000"
        );
        let mut nibbles = [0; 32];
        nibbles[0] = 0x0F;
        nibbles[31] = 0xF0;
        assert_eq!(
            StorageName::from_sha256(nibbles).as_str(),
            "0f000000000000000000000000000000000000000000000000000000000000f0"
        );
    }

    /// An attachment named `../../etc/cron.d/x` inside a Matroska file
    /// (Jellyfin CVE-2026-49246) is stored under the digest of its bytes;
    /// its name has no way in.
    ///
    /// Verifies: SEC-MED-039, SEC-HIS-015
    #[test]
    fn names_an_attachment_by_its_content_whatever_it_calls_itself() {
        let attachment = b"abc";
        let name = StorageName::from_sha256(crate::crypto::sha256(attachment));
        assert_eq!(
            name.as_str(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    /// Verifies: SEC-MED-039, SEC-HIS-015
    #[test]
    fn names_a_record_by_its_identifier() {
        use crate::id::IdKind;
        let id = PublicId::parse("trk_0123456789abcdefghjkmnpqrs", IdKind::Track).unwrap();
        assert_eq!(
            StorageName::from_id(id).as_str(),
            "trk_0123456789abcdefghjkmnpqrs"
        );
        let id = PublicId::parse("pls_7zzzzzzzzzzzzzzzzzzzzzzzzz", IdKind::Playlist).unwrap();
        assert_eq!(
            StorageName::from_id(id).as_str(),
            "pls_7zzzzzzzzzzzzzzzzzzzzzzzzz"
        );
    }

    /// Names for generated absolute paths: few, and prefixes of one another,
    /// so that generated paths often share a start without sharing a name.
    fn short_name() -> impl Strategy<Value = Vec<u8>> {
        prop_oneof![
            Just(b"a".to_vec()),
            Just(b"ab".to_vec()),
            Just(b"b".to_vec()),
            Just(b"\xFF".to_vec()),
            Just(b"a\nb".to_vec()),
        ]
    }

    /// An absolute path of up to four short names.
    fn short_path() -> impl Strategy<Value = RawPath> {
        vec(short_name(), 0..4).prop_map(RawPath)
    }

    /// The path written out as bytes, as a shell would print it.
    fn written(path: &RawPath) -> Vec<u8> {
        if path.0.is_empty() {
            return b"/".to_vec();
        }
        path.0.iter().fold(Vec::new(), |mut out, name| {
            out.push(b'/');
            out.extend_from_slice(name);
            out
        })
    }

    /// An independent statement of containment over the written bytes: the
    /// target is the root itself, or starts with the root followed by `/`.
    /// What follows is split back into names.
    fn beneath_oracle(target: &RawPath, root: &RawPath) -> Option<Vec<Vec<u8>>> {
        let (target, root) = (written(target), written(root));
        let rest = if root == b"/" {
            // Every written path starts with `/`.
            &target[1..]
        } else if target == root {
            &[]
        } else {
            target.strip_prefix(root.as_slice())?.strip_prefix(b"/")?
        };
        if rest.is_empty() {
            return Some(Vec::new());
        }
        Some(
            rest.split(|&byte| byte == b'/')
                .map(<[u8]>::to_vec)
                .collect(),
        )
    }

    /// One name of a link's text: a short name, `.`, `..` or nothing.
    fn link_token() -> impl Strategy<Value = Vec<u8>> {
        prop_oneof![
            short_name(),
            Just(b".".to_vec()),
            Just(b"..".to_vec()),
            Just(b"..".to_vec()),
            Just(Vec::new()),
        ]
    }

    /// An independent statement of link resolution as rewriting: start
    /// from the names of the link's directory, or from none when the text
    /// starts with `/`; add the text's names without the empty ones and
    /// `.`; then delete a `..` at the start, or a name followed by `..`,
    /// until no `..` is left.
    fn link_oracle(root: &RawPath, dir: &RelPath, text: &[u8]) -> Vec<Vec<u8>> {
        let mut names: Vec<Vec<u8>> = if text.first() == Some(&b'/') {
            Vec::new()
        } else {
            root.0.iter().chain(&dir.0).cloned().collect()
        };
        names.extend(
            text.split(|&byte| byte == b'/')
                .filter(|name| !name.is_empty() && *name != b".")
                .map(<[u8]>::to_vec),
        );
        while let Some(at) = names.iter().position(|name| name == b"..") {
            if at == 0 {
                names.remove(0);
            } else {
                names.drain(at - 1..=at);
            }
        }
        names
    }

    /// An independent reference for [`classify_link`], over the written-path
    /// check: own root first, then the approved or other root that leaves
    /// the fewest names, an approved root winning a tie, the first index
    /// among equals.
    fn classify_oracle(
        target: &RawPath,
        own: &RawPath,
        approved: &[RawPath],
        others: &[RawPath],
    ) -> LinkVerdict {
        if let Some(rest) = beneath_oracle(target, own) {
            return LinkVerdict::Own(RelPath(rest));
        }
        let closest = |roots: &[RawPath]| {
            let mut best: Option<(usize, Vec<Vec<u8>>)> = None;
            for (index, root) in roots.iter().enumerate() {
                if let Some(rest) = beneath_oracle(target, root) {
                    if best
                        .as_ref()
                        .is_none_or(|(_, kept)| rest.len() < kept.len())
                    {
                        best = Some((index, rest));
                    }
                }
            }
            best
        };
        match (closest(approved), closest(others)) {
            (Some((index, rest)), None) => LinkVerdict::Approved {
                index,
                rest: RelPath(rest),
            },
            (Some((index, rest)), Some((_, beyond))) if rest.len() <= beyond.len() => {
                LinkVerdict::Approved {
                    index,
                    rest: RelPath(rest),
                }
            }
            (_, Some((index, _))) => LinkVerdict::OtherLibrary { index },
            (None, None) => LinkVerdict::Outside,
        }
    }

    proptest! {
        /// Verifies: SEC-TM-043
        #[test]
        fn containment_agrees_with_a_check_on_the_written_path(
            target in short_path(),
            root in short_path(),
        ) {
            prop_assert_eq!(
                target.beneath(&root).map(|rest| rest.0),
                beneath_oracle(&target, &root)
            );
        }

        /// None of the short names is a system directory, so a candidate is
        /// refused exactly for the first of Gunmetal's own directories that
        /// it equals, contains or lies inside, by the written-path check.
        ///
        /// Verifies: SEC-MED-037
        #[test]
        fn refuses_exactly_the_overlaps_a_check_on_the_written_path_finds(
            candidate in short_path().prop_filter("not the root", |path| !path.0.is_empty()),
            dirs in vec(short_path(), 0..4),
        ) {
            let expected = dirs.iter().enumerate().find_map(|(index, dir)| {
                let overlap = match (
                    beneath_oracle(&candidate, dir).is_some(),
                    beneath_oracle(dir, &candidate).is_some(),
                ) {
                    (true, true) => Overlap::Equal,
                    (true, false) => Overlap::Inside,
                    (false, true) => Overlap::Contains,
                    (false, false) => return None,
                };
                Some(RootRefusal::DataDirectory { index, overlap })
            });
            prop_assert_eq!(refuse_root(&candidate, &dirs), expected);
        }

        /// Verifies: SEC-TM-043
        #[test]
        fn resolves_a_link_as_rewriting_its_parents_away_does(
            root in short_path(),
            dir in vec(short_name(), 0..3),
            absolute in any::<bool>(),
            tokens in vec(link_token(), 0..8),
        ) {
            let mut text = if absolute { b"/".to_vec() } else { Vec::new() };
            text.extend(tokens.join(&b'/'));
            let dir = RelPath(dir);
            let target = link_target(&root, &dir, Untrusted::new(&text));
            prop_assert_eq!(target, Ok(RawPath(link_oracle(&root, &dir, &text))));
        }

        /// Verifies: SEC-MED-034, SEC-TM-043
        #[test]
        fn classifies_a_link_as_the_written_path_check_says(
            target in short_path(),
            own in short_path(),
            approved in vec(short_path(), 0..3),
            others in vec(short_path(), 0..3),
        ) {
            prop_assert_eq!(
                classify_link(&target, &own, &approved, &others),
                classify_oracle(&target, &own, &approved, &others)
            );
        }

        /// Every digest makes one plain file name, which reads back as the
        /// digest, so different content never shares a name.
        ///
        /// Verifies: SEC-MED-039, SEC-HIS-015
        #[test]
        fn names_every_digest_with_64_hexadecimal_digits_that_read_back(
            digest in proptest::array::uniform32(any::<u8>()),
        ) {
            let name = StorageName::from_sha256(digest);
            let text = name.as_str();
            prop_assert_eq!(text.len(), 64);
            prop_assert!(text.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')));
            let read: Vec<u8> = (0..32)
                .map(|at| u8::from_str_radix(&text[2 * at..2 * at + 2], 16).unwrap())
                .collect();
            prop_assert_eq!(read, digest.to_vec());
        }

        /// Verifies: SEC-TM-043
        #[test]
        fn reads_back_every_canonical_path_it_writes(path in short_path()) {
            prop_assert_eq!(raw(&written(&path)), Ok(path));
        }

        /// Verifies: SEC-TM-043
        #[test]
        fn normalises_exactly_what_stays_beneath_the_root(pieces in vec(piece(), 0..10)) {
            let slices: Vec<&[u8]> = pieces.iter().map(Vec::as_slice).collect();
            let normalised = rel(&slices);
            prop_assert_eq!(
                normalised.as_ref().map(|path| path.components().len()).ok(),
                depth_oracle(&pieces)
            );
            if let Ok(path) = normalised {
                for name in path.components() {
                    prop_assert!(name != b".." && name != b"." && !name.is_empty());
                    prop_assert!(!name.contains(&b'/') && !name.contains(&0));
                }
                let again: Vec<&[u8]> = path.components().iter().map(Vec::as_slice).collect();
                prop_assert_eq!(rel(&again), Ok(path));
            }
        }
    }
}
