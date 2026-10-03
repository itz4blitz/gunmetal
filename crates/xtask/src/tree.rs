//! Read access to the repository. Every check is a pure function of what it
//! reads through [`Tree`], so its tests run against small trees held in
//! memory, and this module is the only one that touches the filesystem.

use std::fs;
use std::path::{Path, PathBuf};

/// Files addressed by `/`-separated paths relative to the repository root.
pub trait Tree {
    /// The text of the file at `path`, or `None` when there is no such file
    /// or it is not UTF-8.
    fn read(&self, path: &str) -> Option<String>;

    /// The path of every file beneath directory `dir`, relative to `dir` and
    /// sorted; empty when `dir` does not exist. The empty `dir` is the whole
    /// tree, build output and git's own files included. A symbolic link is
    /// listed as a file and never followed, so a link cannot make the walk
    /// loop.
    fn files(&self, dir: &str) -> Vec<String>;
}

/// Whether `path` names a Rust source file. Cargo only compiles modules
/// named with a lowercase `.rs`, so the comparison is exact.
pub fn is_rust(path: &str) -> bool {
    Path::new(path)
        .extension()
        .is_some_and(|extension| extension == "rs")
}

/// The repository on disk.
pub struct Disk {
    root: PathBuf,
}

impl Disk {
    /// The repository whose top directory is `root`.
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }
}

impl Tree for Disk {
    fn read(&self, path: &str) -> Option<String> {
        fs::read_to_string(self.root.join(path)).ok()
    }

    fn files(&self, dir: &str) -> Vec<String> {
        let mut found = Vec::new();
        walk(&self.root.join(dir), "", &mut found);
        found.sort();
        found
    }
}

/// Adds to `found` the path of every file beneath `dir`, each written after
/// `prefix`.
fn walk(dir: &Path, prefix: &str, found: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = format!("{prefix}{}", entry.file_name().to_string_lossy());
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            walk(&entry.path(), &format!("{path}/"), found);
        } else {
            found.push(path);
        }
    }
}

#[cfg(test)]
pub mod memory {
    //! A tree held in memory, for tests.

    use std::collections::BTreeMap;

    use super::Tree;

    /// Files and their text, by path.
    #[derive(Default)]
    pub struct Memory(BTreeMap<String, String>);

    impl Memory {
        /// This tree with one more file, `path`, holding `text`.
        pub fn with(mut self, path: &str, text: &str) -> Self {
            self.0.insert(path.to_owned(), text.to_owned());
            self
        }
    }

    impl Tree for Memory {
        fn read(&self, path: &str) -> Option<String> {
            self.0.get(path).cloned()
        }

        fn files(&self, dir: &str) -> Vec<String> {
            let prefix = if dir.is_empty() {
                String::new()
            } else {
                format!("{dir}/")
            };
            self.0
                .keys()
                .filter_map(|path| path.strip_prefix(&prefix))
                .map(str::to_owned)
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::memory::Memory;
    use super::{Disk, Tree, is_rust};
    use crate::ROOT;
    use std::path::Path;

    #[test]
    fn recognises_rust_sources_by_their_exact_extension() {
        assert!(is_rust("crates/xtask/src/main.rs"));
        assert!(is_rust("lib.rs"));
        assert!(!is_rust("notes.md"));
        assert!(!is_rust("sample.rs.txt"));
        assert!(!is_rust("SHOUT.RS"));
        assert!(!is_rust("rs"));
    }

    /// A small committed tree: `alpha.txt` and `nested/beta.txt`.
    const FIXTURE: &str = "crates/xtask/fixtures/tree";

    #[test]
    fn reads_a_file_by_its_path_from_the_root() {
        let disk = Disk::new(Path::new(ROOT));
        assert_eq!(
            disk.read(&format!("{FIXTURE}/alpha.txt")),
            Some("alpha\n".to_owned())
        );
        assert_eq!(
            disk.read(&format!("{FIXTURE}/nested/beta.txt")),
            Some("beta\n".to_owned())
        );
    }

    #[test]
    fn reads_nothing_for_a_missing_file_or_a_directory() {
        let disk = Disk::new(Path::new(ROOT));
        assert_eq!(disk.read(&format!("{FIXTURE}/missing.txt")), None);
        assert_eq!(disk.read(&format!("{FIXTURE}/nested")), None);
    }

    #[test]
    fn lists_every_file_beneath_a_directory_relative_to_it() {
        let disk = Disk::new(Path::new(ROOT));
        assert_eq!(disk.files(FIXTURE), ["alpha.txt", "nested/beta.txt"]);
        assert_eq!(disk.files(&format!("{FIXTURE}/nested")), ["beta.txt"]);
    }

    #[test]
    fn lists_every_file_in_the_tree_for_the_empty_directory() {
        let disk = Disk::new(&Path::new(ROOT).join(FIXTURE));
        assert_eq!(disk.files(""), ["alpha.txt", "nested/beta.txt"]);
        let memory = Memory::default()
            .with("alpha.txt", "")
            .with("nested/beta.txt", "");
        assert_eq!(memory.files(""), ["alpha.txt", "nested/beta.txt"]);
        assert_eq!(memory.files("nested"), ["beta.txt"]);
        assert_eq!(memory.files("nest"), [""; 0]);
    }

    #[test]
    fn lists_nothing_for_a_missing_directory() {
        let disk = Disk::new(Path::new(ROOT));
        assert_eq!(disk.files(&format!("{FIXTURE}/missing")), [""; 0]);
    }
}
