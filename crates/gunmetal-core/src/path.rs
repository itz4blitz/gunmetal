//! Paths as raw byte names (WP-024).
//!
//! Nothing here touches the filesystem. The filesystem crate (WP-060) opens
//! every [`RelPath`] beneath a root's directory handle. Names stay bytes for
//! access (SEC-MED-040), so a file whose name is not UTF-8, or holds a
//! newline or an escape character, is still reachable.

use crate::untrusted::Untrusted;

/// A path beneath a root: zero or more names, none of them empty, `.` or
/// `..`, and none holding `/` or a NUL byte. The empty path is the root
/// itself. Only [`normalise`] makes one, so a `RelPath` can never climb out
/// of the root it is opened beneath (SEC-TM-043).
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

#[cfg(test)]
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
            target.strip_prefix(b"/")?
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
