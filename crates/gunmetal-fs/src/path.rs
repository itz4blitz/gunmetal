//! Paths beneath the data root: one directory of the layout and a checked
//! relative path inside it.
//!
//! A [`DataPath`] can only name something inside the data directory. Its
//! relative part is one to [`MAX_DEPTH`] names separated by `/`, and each
//! name is one to [`MAX_NAME_LEN`] bytes of lower-case ASCII letters, digits,
//! `.`, `-` and `_`, starting with a letter or a digit. So `..`, `.`, an
//! absolute path, a backslash, a NUL byte and an empty name cannot be
//! expressed (SEC-TM-043). The data-root handle resolves every path beneath
//! its directory handle as well, so a path that got through wrongly still
//! could not leave the root (SEC-HIS-016).
//!
//! Outside this crate a [`DataPath`] is built only from constants, with
//! [`DataPath::constant`], so no request or media data can name a file in
//! the data directory (SEC-HIS-015). A store that needs names it generates
//! at run time, such as a log segment per stream and month, gets a typed
//! constructor here that takes only the server's own identifier types.
#![expect(
    clippy::disallowed_methods,
    reason = "the one place a data-directory path is built, from names the data root checked (SEC-MED-033)"
)]

use std::borrow::Cow;
use std::path::{Path, PathBuf};

/// The directories of the data-directory layout, from
/// operations-and-incident-response.md, section 2, with `derived/` for
/// ADM-141 and `tmp/` for files being written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataDir {
    /// `secrets/`: the root secret and the other key files (SEC-OPS-012).
    Secrets,
    /// `durable/`: the identity store, the user log and the audit log.
    Durable,
    /// `cache/`: the rebuildable library cache.
    Cache,
    /// `snapshots/`: snapshots taken before a migration.
    Snapshots,
    /// `backups/`: encrypted, signed backups.
    Backups,
    /// `derived/`: the derived-data store.
    Derived,
    /// `tmp/`: files being prepared before they move into place.
    Tmp,
}

impl DataDir {
    /// Every layout directory, in the order the data root creates them.
    pub const ALL: [Self; 7] = [
        Self::Secrets,
        Self::Durable,
        Self::Cache,
        Self::Snapshots,
        Self::Backups,
        Self::Derived,
        Self::Tmp,
    ];

    /// The directory's name beneath the data root.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Secrets => "secrets",
            Self::Durable => "durable",
            Self::Cache => "cache",
            Self::Snapshots => "snapshots",
            Self::Backups => "backups",
            Self::Derived => "derived",
            Self::Tmp => "tmp",
        }
    }
}

/// The most names a relative path may hold.
pub const MAX_DEPTH: usize = 4;

/// The longest name, in bytes, a relative path may hold.
pub const MAX_NAME_LEN: usize = 64;

/// Why a relative path was refused. Offsets count bytes from the start of
/// the relative path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathError {
    /// The path was empty.
    Empty,
    /// A name was empty: the path started or ended with `/`, or held `//`.
    EmptyName {
        /// Where the empty name starts.
        offset: usize,
    },
    /// A byte outside lower-case ASCII letters, digits, `.`, `-` and `_`.
    Forbidden {
        /// Where the byte is.
        offset: usize,
        /// The byte.
        byte: u8,
    },
    /// A name started with `.`, `-` or `_`, which also refuses `.` and `..`.
    BadStart {
        /// Where the name starts.
        offset: usize,
        /// Its first byte.
        byte: u8,
    },
    /// A name was longer than [`MAX_NAME_LEN`] bytes.
    NameTooLong {
        /// Where the name starts.
        offset: usize,
    },
    /// The path held more than [`MAX_DEPTH`] names.
    TooDeep,
}

/// Checks a relative path against the rules in the module documentation.
///
/// It is a `const fn` so that [`DataPath::constant`] can run it at compile
/// time, which is why it walks the bytes by hand.
const fn check(rel: &str) -> Result<(), PathError> {
    let mut rest = rel.as_bytes();
    if rest.is_empty() {
        return Err(PathError::Empty);
    }
    // `offset` is the byte being read and `start` the first byte of the name
    // it belongs to; both stay within `rel`, so neither can overflow.
    let mut offset = 0;
    let mut start = 0;
    let mut depth = 1;
    while let [byte, tail @ ..] = rest {
        let byte = *byte;
        if byte == b'/' {
            if offset == start {
                return Err(PathError::EmptyName { offset });
            }
            if depth == MAX_DEPTH {
                return Err(PathError::TooDeep);
            }
            depth += 1;
            start = offset + 1;
        } else if !matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-' | b'_') {
            return Err(PathError::Forbidden { offset, byte });
        } else if offset == start && !matches!(byte, b'a'..=b'z' | b'0'..=b'9') {
            return Err(PathError::BadStart { offset, byte });
        } else if offset - start == MAX_NAME_LEN {
            return Err(PathError::NameTooLong { offset: start });
        }
        offset += 1;
        rest = tail;
    }
    if offset == start {
        Err(PathError::EmptyName { offset })
    } else {
        Ok(())
    }
}

/// A path inside one directory of the data-directory layout.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DataPath {
    dir: DataDir,
    rel: Cow<'static, str>,
}

impl DataPath {
    /// Builds a path from constants. Use it in a `const` item, where an
    /// invalid path is a compile error:
    ///
    /// ```
    /// use gunmetal_fs::path::{DataDir, DataPath};
    ///
    /// const ROOT_KEY: DataPath = DataPath::constant(DataDir::Secrets, "root.key");
    /// assert_eq!(ROOT_KEY.rel(), "root.key");
    /// ```
    ///
    /// `..` and absolute paths cannot be expressed:
    ///
    /// ```compile_fail,E0080
    /// use gunmetal_fs::path::{DataDir, DataPath};
    ///
    /// const ESCAPE: DataPath = DataPath::constant(DataDir::Secrets, "../root.key");
    /// assert_eq!(ESCAPE.rel(), "../root.key");
    /// ```
    ///
    /// ```compile_fail,E0080
    /// use gunmetal_fs::path::{DataDir, DataPath};
    ///
    /// const ABSOLUTE: DataPath = DataPath::constant(DataDir::Secrets, "/etc/passwd");
    /// assert_eq!(ABSOLUTE.rel(), "/etc/passwd");
    /// ```
    ///
    /// Text that is not a constant cannot name a file, through this
    /// function or any other:
    ///
    /// ```compile_fail,E0597
    /// use gunmetal_fs::path::{DataDir, DataPath};
    ///
    /// let name = String::from("root.key");
    /// let path = DataPath::constant(DataDir::Secrets, &name);
    /// assert_eq!(path.rel(), "root.key");
    /// ```
    ///
    /// ```compile_fail,E0624
    /// use gunmetal_fs::path::{DataDir, DataPath};
    ///
    /// let name = String::from("root.key");
    /// let path = DataPath::new(DataDir::Secrets, &name).unwrap();
    /// assert_eq!(path.rel(), "root.key");
    /// ```
    ///
    /// # Panics
    ///
    /// Panics when `rel` breaks the rules in the module documentation. In a
    /// `const` item that panic is a compile error, which is the point.
    #[must_use]
    pub const fn constant(dir: DataDir, rel: &'static str) -> Self {
        match check(rel) {
            Ok(()) => Self {
                dir,
                rel: Cow::Borrowed(rel),
            },
            Err(_) => panic!("a constant data path breaks the data path rules"),
        }
    }

    /// Builds a path from a name found in the data directory, so that the
    /// data root can check what it finds there. It is not public: outside
    /// this crate a path is built only from constants, so no request or
    /// media data can name a file (SEC-HIS-015, SEC-TM-031).
    pub(crate) fn new(dir: DataDir, rel: &str) -> Result<Self, PathError> {
        check(rel).map(|()| Self {
            dir,
            rel: Cow::Owned(rel.to_owned()),
        })
    }

    /// The path of `rel` inside this one, for the same check.
    pub(crate) fn join(&self, rel: &str) -> Result<Self, PathError> {
        Self::new(self.dir, &format!("{}/{rel}", self.rel))
    }

    /// The layout directory the path is in.
    #[must_use]
    pub const fn dir(&self) -> DataDir {
        self.dir
    }

    /// The checked path inside the layout directory.
    #[must_use]
    pub fn rel(&self) -> &str {
        &self.rel
    }

    /// The path relative to the data root, such as `secrets/root.key`.
    pub(crate) fn beneath(&self) -> PathBuf {
        Path::new(self.dir.name()).join(self.rel.as_ref())
    }

    /// The directory holding the path, relative to the data root, and the
    /// path's last name.
    fn split(&self) -> (PathBuf, &str) {
        let dir = Path::new(self.dir.name());
        match self.rel.rsplit_once('/') {
            Some((head, last)) => (dir.join(head), last),
            None => (dir.to_path_buf(), &self.rel),
        }
    }

    /// The directory holding the path, relative to the data root.
    pub(crate) fn parent(&self) -> PathBuf {
        self.split().0
    }

    /// Where a replacement for the path is written before it is renamed
    /// over it. The leading `.` keeps it apart from every [`DataPath`].
    pub(crate) fn temp_sibling(&self) -> PathBuf {
        let (parent, last) = self.split();
        parent.join(format!(".{last}.tmp"))
    }
}

/// The name of the file whose replacement writes the temporary file called
/// `temp` (see [`DataPath::temp_sibling`]), or `None` when `temp` is not
/// named like one. The name returned is not checked.
pub(crate) fn replaced_by(temp: &str) -> Option<&str> {
    temp.strip_prefix('.')?.strip_suffix(".tmp")
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    // Qodana does not expand `proptest!` or resolve `prop_oneof!` through `prelude::*`.
    use proptest::prop_oneof;
    use proptest::test_runner::{Config, TestRunner};

    /// An independent statement of the rules, written with iterators so it
    /// shares nothing with the byte loop in `check`.
    fn oracle(rel: &str) -> bool {
        let names: Vec<&str> = rel.split('/').collect();
        names.len() <= MAX_DEPTH
            && names.iter().all(|name| {
                let starts_well = name
                    .bytes()
                    .next()
                    .is_some_and(|first| first.is_ascii_lowercase() || first.is_ascii_digit());
                let allowed = name.bytes().all(|byte| {
                    byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
                });
                starts_well && allowed && name.len() <= MAX_NAME_LEN
            })
    }

    fn path(rel: &str) -> Result<DataPath, PathError> {
        DataPath::new(DataDir::Secrets, rel)
    }

    #[test]
    fn names_every_layout_directory() {
        let names: Vec<&str> = DataDir::ALL.iter().map(|dir| dir.name()).collect();
        assert_eq!(
            names,
            [
                "secrets",
                "durable",
                "cache",
                "snapshots",
                "backups",
                "derived",
                "tmp"
            ]
        );
    }

    #[test]
    fn accepts_names_the_rules_allow() {
        let built = path("log/2026-10.seg").expect("a valid path");
        assert_eq!(built.dir(), DataDir::Secrets);
        assert_eq!(built.rel(), "log/2026-10.seg");
        assert_eq!(built.beneath(), PathBuf::from("secrets/log/2026-10.seg"));
    }

    #[test]
    fn accepts_the_longest_name_and_the_deepest_path() {
        let longest = "a".repeat(MAX_NAME_LEN);
        assert_eq!(path(&longest).map(|built| built.rel().len()), Ok(64));
        let deep = format!("{longest}/b/{longest}/0_-.x");
        assert_eq!(path(&deep).map(|built| built.rel().len()), Ok(137));
        // A name's length is counted from its own start, wherever it is.
        let second = format!("a/{longest}");
        assert_eq!(path(&second).map(|built| built.rel().len()), Ok(66));
    }

    /// Verifies: SEC-TM-043
    #[test]
    fn refuses_parent_and_current_names() {
        assert_eq!(
            path(".."),
            Err(PathError::BadStart {
                offset: 0,
                byte: b'.'
            })
        );
        assert_eq!(
            path("log/../../etc"),
            Err(PathError::BadStart {
                offset: 4,
                byte: b'.'
            })
        );
        assert_eq!(
            path("log/./x"),
            Err(PathError::BadStart {
                offset: 4,
                byte: b'.'
            })
        );
        assert_eq!(
            path("-rf"),
            Err(PathError::BadStart {
                offset: 0,
                byte: b'-'
            })
        );
        assert_eq!(
            path("a/_x"),
            Err(PathError::BadStart {
                offset: 2,
                byte: b'_'
            })
        );
    }

    /// Verifies: SEC-TM-043
    #[test]
    fn refuses_absolute_paths_and_empty_names() {
        assert_eq!(path(""), Err(PathError::Empty));
        assert_eq!(path("/etc/passwd"), Err(PathError::EmptyName { offset: 0 }));
        assert_eq!(path("log//x"), Err(PathError::EmptyName { offset: 4 }));
        assert_eq!(path("log/"), Err(PathError::EmptyName { offset: 4 }));
    }

    /// Verifies: SEC-TM-043
    #[test]
    fn refuses_bytes_outside_the_allowed_set() {
        assert_eq!(
            path("..\\..\\x"),
            Err(PathError::BadStart {
                offset: 0,
                byte: b'.'
            })
        );
        assert_eq!(
            path("a\\b"),
            Err(PathError::Forbidden {
                offset: 1,
                byte: b'\\'
            })
        );
        assert_eq!(
            path("key\0"),
            Err(PathError::Forbidden { offset: 3, byte: 0 })
        );
        assert_eq!(
            path("c:"),
            Err(PathError::Forbidden {
                offset: 1,
                byte: b':'
            })
        );
        assert_eq!(
            path("Root.key"),
            Err(PathError::Forbidden {
                offset: 0,
                byte: b'R'
            })
        );
        assert_eq!(
            path("log/r\u{e9}"),
            Err(PathError::Forbidden {
                offset: 5,
                byte: 0xC3
            })
        );
    }

    #[test]
    fn refuses_long_names_where_they_start() {
        let long = "a".repeat(MAX_NAME_LEN + 1);
        assert_eq!(path(&long), Err(PathError::NameTooLong { offset: 0 }));
        assert_eq!(
            path(&format!("ab/{long}")),
            Err(PathError::NameTooLong { offset: 3 })
        );
        assert_eq!(
            path(&format!("ab/cd/{long}")),
            Err(PathError::NameTooLong { offset: 6 })
        );
    }

    #[test]
    fn refuses_more_than_four_names() {
        assert_eq!(path("a/b/c/d").map(|built| built.rel().len()), Ok(7));
        assert_eq!(path("a/b/c/d/e"), Err(PathError::TooDeep));
        assert_eq!(path("a/b/c/d/"), Err(PathError::TooDeep));
    }

    #[test]
    fn joins_a_name_under_the_same_rules() {
        let log = path("log").expect("a valid path");
        assert_eq!(
            log.join("2026-10.seg"),
            DataPath::new(DataDir::Secrets, "log/2026-10.seg")
        );
        assert_eq!(
            log.join(".."),
            Err(PathError::BadStart {
                offset: 4,
                byte: b'.'
            })
        );
        assert_eq!(log.join("a/b/c/d"), Err(PathError::TooDeep));
    }

    #[test]
    fn builds_a_constant_at_run_time_too() {
        assert_eq!(
            DataPath::constant(DataDir::Cache, "library.db"),
            DataPath::new(DataDir::Cache, "library.db").expect("a valid path")
        );
    }

    #[test]
    #[should_panic(expected = "a constant data path breaks the data path rules")]
    fn panics_on_an_invalid_constant() {
        let _ = DataPath::constant(DataDir::Cache, "../library.db");
    }

    #[test]
    fn places_the_parent_and_the_temporary_sibling_beside_the_path() {
        let top = DataPath::constant(DataDir::Secrets, "keys.json");
        assert_eq!(top.parent(), PathBuf::from("secrets"));
        assert_eq!(top.temp_sibling(), PathBuf::from("secrets/.keys.json.tmp"));
        let nested = DataPath::constant(DataDir::Durable, "log/main/2026-10.seg");
        assert_eq!(nested.parent(), PathBuf::from("durable/log/main"));
        assert_eq!(
            nested.temp_sibling(),
            PathBuf::from("durable/log/main/.2026-10.seg.tmp")
        );
    }

    #[test]
    fn recognises_the_name_of_a_temporary_file_and_nothing_else() {
        assert_eq!(replaced_by(".keys.json.tmp"), Some("keys.json"));
        assert_eq!(replaced_by(".a.tmp.tmp"), Some("a.tmp"));
        assert_eq!(replaced_by("..tmp"), Some(""));
        for name in ["keys.json", ".keys.json", "keys.json.tmp", ".tmp", ""] {
            assert_eq!(replaced_by(name), None, "{name}");
        }
    }

    #[test]
    fn recognises_every_temporary_sibling_it_names() {
        for (path, last) in [
            (
                DataPath::constant(DataDir::Secrets, "keys.json"),
                "keys.json",
            ),
            (
                DataPath::constant(DataDir::Secrets, "tls/key.pem"),
                "key.pem",
            ),
        ] {
            let temp = path.temp_sibling();
            let name = temp.file_name().and_then(|name| name.to_str());
            assert_eq!(name.and_then(replaced_by), Some(last));
        }
    }

    /// Pieces of hostile paths: traversal, absolute and UNC prefixes,
    /// device names, NUL bytes, mixed separators, upper case and long names.
    fn piece() -> impl Strategy<Value = String> {
        prop_oneof![
            Just(String::new()),
            Just(".".to_owned()),
            Just("..".to_owned()),
            Just("/".to_owned()),
            Just("\\".to_owned()),
            Just("\\\\server\\share".to_owned()),
            Just("c:".to_owned()),
            Just("con".to_owned()),
            Just("nul.txt".to_owned()),
            Just("\0".to_owned()),
            Just("Key".to_owned()),
            Just("a".repeat(MAX_NAME_LEN + 1)),
            "[a-z0-9][a-z0-9._-]{0,8}",
            "\\PC{0,6}",
        ]
    }

    /// Verifies: SEC-TM-043
    #[test]
    fn accepts_exactly_what_the_rules_allow() {
        TestRunner::new(Config::default())
            .run(
                &(
                    proptest::collection::vec(piece(), 1..8),
                    prop_oneof![Just("/"), Just("\\"), Just("//")],
                ),
                |(pieces, separator)| {
                    let rel = pieces.join(separator);
                    let built = path(&rel);
                    prop_assert_eq!(built.is_ok(), oracle(&rel));
                    if let Ok(built) = built {
                        prop_assert!(
                            built
                                .rel()
                                .split('/')
                                .all(|name| name != ".." && name != ".")
                        );
                        prop_assert!(built.beneath().starts_with("secrets"));
                    }
                    Ok(())
                },
            )
            .unwrap();
    }

    #[test]
    fn accepts_exactly_what_the_rules_allow_for_any_text() {
        TestRunner::new(Config::default())
            .run(&"\\PC{0,80}", |rel| {
                prop_assert_eq!(path(&rel).is_ok(), oracle(&rel));
                Ok(())
            })
            .unwrap();
    }
}
