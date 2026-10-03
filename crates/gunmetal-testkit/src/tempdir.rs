//! Temporary directories that remove themselves.
//!
//! Server tests open real database files and write real segment files in a
//! [`TempDir`], never a mocked filesystem.

// The testkit is the one written exception to the ban on path-based
// `std::fs` outside `gunmetal-fs` (owner decision 33 in the build plan): a
// test's scratch directory has no root handle to open it beneath. Every
// path-based call in the crate is in this module. The ban is not on the
// wave branch yet, so this is an `allow`; it becomes the plan's
// `#[expect(clippy::disallowed_methods)]` once the ban lands.
#![allow(
    clippy::disallowed_methods,
    reason = "the testkit's temporary directory is the one sanctioned path-based std::fs user (owner decision 33)"
)]

use std::fs::{self, DirBuilder};
use std::io;
use std::path::{Path, PathBuf};

/// How many names [`TempDir::new_in`] tries before it gives up. More than
/// any test needs at once; the limit only makes the search finite.
const ATTEMPTS: u32 = 256;

/// A directory that exists until this value is dropped, then is removed
/// with everything in it, including when the test that made it panics.
///
/// Dropping removes the directory's entries without following symbolic
/// links, so a link a test leaves inside cannot reach anything outside. A
/// failure to remove is ignored, as a test's result should not depend on
/// its cleanup.
#[derive(Debug)]
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// Creates an empty directory in the system's temporary directory. See
    /// [`TempDir::new_in`].
    ///
    /// # Errors
    ///
    /// As [`TempDir::new_in`].
    pub fn new(label: &str) -> io::Result<Self> {
        Self::new_in(&std::env::temp_dir(), label)
    }

    /// Creates an empty directory in `parent`, readable only by its owner,
    /// named `gunmetal-<label>-<process ID>-<n>` for the smallest `n` whose
    /// name is free.
    ///
    /// The directory is created exclusively, so two instances never share
    /// one: another thread's instance, another process's, or one a killed
    /// run left behind each make this one move on to the next `n`.
    ///
    /// # Errors
    ///
    /// Returns an [`io::ErrorKind::InvalidInput`] error when `label` is
    /// empty or holds anything but ASCII letters, digits, `-` and `_`, so
    /// it can never name a path outside `parent`. Returns an
    /// [`io::ErrorKind::AlreadyExists`] error when the first 256 names are
    /// all taken, and any other error creating the directory as it is.
    pub fn new_in(parent: &Path, label: &str) -> io::Result<Self> {
        if label.is_empty()
            || !label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "temporary directory label {label:?} must be ASCII letters, digits, \
                     '-' or '_', and not empty"
                ),
            ));
        }
        let process = std::process::id();
        for attempt in 0..ATTEMPTS {
            let path = parent.join(format!("gunmetal-{label}-{process}-{attempt}"));
            match private_directory().create(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "all {ATTEMPTS} names for temporary directory label {label:?} in {} are taken",
                parent.display()
            ),
        ))
    }

    /// Where the directory is.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Creates one directory, failing if it exists, with mode 0700 where the
/// platform has modes.
fn private_directory() -> DirBuilder {
    let mut builder = DirBuilder::new();
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    builder
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;
    use std::panic::{self, AssertUnwindSafe};
    use std::sync::{Mutex, RwLock};
    use std::thread;

    /// How many names `TempDir::new_in` tries before it gives up.
    const TRIES: u32 = 256;

    /// The name `TempDir::new_in` gives its `attempt`-th try for `label`.
    fn name(label: &str, attempt: u32) -> String {
        format!("gunmetal-{label}-{}-{attempt}", std::process::id())
    }

    /// The kind of `path`'s directory entry, without following a symlink,
    /// or the error that looking it up gave.
    fn entry(path: &Path) -> Result<fs::FileType, ErrorKind> {
        fs::symlink_metadata(path)
            .map(|metadata| metadata.file_type())
            .map_err(|error| error.kind())
    }

    /// Creates the directories `TempDir::new_in` would try first for
    /// `label`, as if other instances or a crashed run held them.
    fn occupy(parent: &Path, label: &str, count: u32) {
        for attempt in 0..count {
            fs::create_dir(parent.join(name(label, attempt))).unwrap();
        }
    }

    #[test]
    fn creates_an_empty_directory_named_after_label_and_process() {
        let parent = TempDir::new("testkit-parent").unwrap();
        let dir = TempDir::new_in(parent.path(), "fresh").unwrap();
        assert_eq!(dir.path(), parent.path().join(name("fresh", 0)));
        assert!(entry(dir.path()).unwrap().is_dir());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[cfg(unix)]
    #[test]
    fn creates_the_directory_readable_by_its_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TempDir::new("testkit-private").unwrap();
        let mode = fs::metadata(dir.path()).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700);
    }

    #[test]
    fn creates_the_directory_in_the_system_temporary_directory_by_default() {
        let dir = TempDir::new("testkit-default").unwrap();
        assert_eq!(dir.path().parent(), Some(std::env::temp_dir().as_path()));
        // A run killed before it could clean up may have left the first
        // names behind, so the attempt number is not fixed.
        let file_name = dir.path().file_name().unwrap().to_str().unwrap();
        let prefix = format!("gunmetal-testkit-default-{}-", std::process::id());
        let attempt = file_name
            .strip_prefix(&prefix)
            .and_then(|attempt| attempt.parse::<u32>().ok());
        assert!(
            attempt.is_some_and(|attempt| attempt < TRIES),
            "unexpected name {file_name}"
        );
    }

    #[test]
    fn gives_two_instances_with_the_same_label_different_directories() {
        let parent = TempDir::new("testkit-parent").unwrap();
        let first = TempDir::new_in(parent.path(), "twin").unwrap();
        let second = TempDir::new_in(parent.path(), "twin").unwrap();
        assert_eq!(first.path(), parent.path().join(name("twin", 0)));
        assert_eq!(second.path(), parent.path().join(name("twin", 1)));
    }

    #[test]
    fn gives_threads_creating_at_once_one_directory_each() {
        const THREADS: u32 = 8;
        let parent = TempDir::new("testkit-parent").unwrap();
        // The threads wait at this gate until every one is spawned, so their
        // creations race. The gate opens however spawning ends, since
        // unwinding drops the guard too, so no thread can wait forever.
        let gate = RwLock::new(());
        // Each thread hands over what it got, directory or error, and ends.
        // A failed creation is then a failed assertion, never a hang, and
        // every directory lives until all threads are done, so no name is
        // freed and taken a second time during the race.
        let outcomes = Mutex::new(Vec::new());
        thread::scope(|scope| {
            let closed = gate.write().unwrap();
            for _ in 0..THREADS {
                scope.spawn(|| {
                    let _opened = gate.read();
                    let made = TempDir::new_in(parent.path(), "crowd");
                    outcomes.lock().unwrap().push(made);
                });
            }
            drop(closed);
        });
        let made = outcomes.into_inner().unwrap();
        let mut got: Vec<Result<PathBuf, ErrorKind>> = made
            .iter()
            .map(|made| {
                made.as_ref()
                    .map(|dir| dir.path().to_path_buf())
                    .map_err(io::Error::kind)
            })
            .collect();
        got.sort();
        let mut expected: Vec<Result<PathBuf, ErrorKind>> = (0..THREADS)
            .map(|attempt| Ok(parent.path().join(name("crowd", attempt))))
            .collect();
        expected.sort();
        assert_eq!(got, expected);
    }

    #[test]
    fn skips_names_already_taken_and_leaves_them_alone() {
        let parent = TempDir::new("testkit-parent").unwrap();
        occupy(parent.path(), "busy", 2);
        let dir = TempDir::new_in(parent.path(), "busy").unwrap();
        assert_eq!(dir.path(), parent.path().join(name("busy", 2)));
        drop(dir);
        assert!(
            entry(&parent.path().join(name("busy", 0)))
                .unwrap()
                .is_dir()
        );
        assert!(
            entry(&parent.path().join(name("busy", 1)))
                .unwrap()
                .is_dir()
        );
    }

    #[test]
    fn takes_the_last_name_it_tries() {
        let parent = TempDir::new("testkit-parent").unwrap();
        occupy(parent.path(), "nearly", TRIES - 1);
        let dir = TempDir::new_in(parent.path(), "nearly").unwrap();
        assert_eq!(dir.path(), parent.path().join(name("nearly", TRIES - 1)));
    }

    #[test]
    fn gives_up_when_every_name_it_tries_is_taken() {
        let parent = TempDir::new("testkit-parent").unwrap();
        occupy(parent.path(), "full", TRIES);
        let error = TempDir::new_in(parent.path(), "full").unwrap_err();
        assert_eq!(
            (error.kind(), error.to_string()),
            (
                ErrorKind::AlreadyExists,
                format!(
                    "all 256 names for temporary directory label \"full\" in {} are taken",
                    parent.path().display()
                )
            )
        );
        assert_eq!(fs::read_dir(parent.path()).unwrap().count(), 256);
    }

    #[test]
    fn passes_on_any_other_error() {
        let parent = TempDir::new("testkit-parent").unwrap();
        let missing = parent.path().join("missing");
        let error = TempDir::new_in(&missing, "orphan").unwrap_err();
        assert_eq!(
            (error.kind(), error.raw_os_error()),
            (ErrorKind::NotFound, Some(2))
        );
    }

    #[test]
    fn accepts_labels_of_letters_digits_hyphens_and_underscores() {
        let parent = TempDir::new("testkit-parent").unwrap();
        for label in ["a", "Z", "7", "-", "_", "Flac-frames_2"] {
            let dir = TempDir::new_in(parent.path(), label).unwrap();
            assert_eq!(dir.path(), parent.path().join(name(label, 0)));
        }
    }

    #[test]
    fn refuses_a_label_that_could_leave_the_parent_or_is_empty() {
        let parent = TempDir::new("testkit-parent").unwrap();
        for label in ["", "..", "a/b", "/abs", "a b", "a\\b", "caf\u{e9}", "a\nb"] {
            let error = TempDir::new_in(parent.path(), label).unwrap_err();
            assert_eq!(
                (error.kind(), error.to_string()),
                (
                    ErrorKind::InvalidInput,
                    format!(
                        "temporary directory label {label:?} must be ASCII letters, \
                         digits, '-' or '_', and not empty"
                    )
                )
            );
        }
        assert_eq!(fs::read_dir(parent.path()).unwrap().count(), 0);
    }

    #[test]
    fn removes_the_directory_and_everything_in_it_on_drop() {
        let parent = TempDir::new("testkit-parent").unwrap();
        let dir = TempDir::new_in(parent.path(), "full-of-files").unwrap();
        let path = dir.path().to_path_buf();
        fs::create_dir(path.join("nested")).unwrap();
        fs::write(path.join("nested").join("track.flac"), b"fLaC").unwrap();
        fs::write(path.join("cover.png"), b"\x89PNG").unwrap();
        drop(dir);
        assert_eq!(entry(&path), Err(ErrorKind::NotFound));
        assert_eq!(fs::read_dir(parent.path()).unwrap().count(), 0);
    }

    #[test]
    fn removes_the_directory_when_the_test_body_panics() {
        let parent = TempDir::new("testkit-parent").unwrap();
        let created = Mutex::new(None);
        let outcome = panic::catch_unwind(AssertUnwindSafe(|| {
            let dir = TempDir::new_in(parent.path(), "panicking").unwrap();
            fs::write(dir.path().join("partial.wav"), b"RIFF").unwrap();
            *created.lock().unwrap() = Some(dir.path().to_path_buf());
            panic!("the test body failed");
        }));
        let payload = outcome.unwrap_err();
        assert_eq!(
            payload.downcast_ref::<&str>(),
            Some(&"the test body failed")
        );
        let path = created.into_inner().unwrap().unwrap();
        assert_eq!(path, parent.path().join(name("panicking", 0)));
        assert_eq!(entry(&path), Err(ErrorKind::NotFound));
    }

    #[cfg(unix)]
    #[test]
    fn removes_a_symlink_inside_without_following_it() {
        let outside = TempDir::new("testkit-outside").unwrap();
        fs::write(outside.path().join("keep.txt"), b"keep").unwrap();
        let dir = TempDir::new("testkit-linking").unwrap();
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(outside.path(), &link).unwrap();
        drop(dir);
        assert_eq!(entry(&link), Err(ErrorKind::NotFound));
        assert_eq!(fs::read(outside.path().join("keep.txt")).unwrap(), b"keep");
    }
}
