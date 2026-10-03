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
}

/// Why a path was refused. `component` counts names from zero and `offset`
/// counts bytes from the start of that name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathError {
    /// A name was empty, as between the two slashes of `a//b` or before the
    /// first slash of an absolute path.
    EmptyName {
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

    proptest! {
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
