//! The worker's arguments: a closed vocabulary, built only from typed
//! values (SEC-HIS-020).
//!
//! The launcher's argument list is three fixed words chosen by two enums.
//! No string from a request, a setting, a tag or a file name can reach it,
//! because [`TypedArgs`] has no constructor that takes one. Input and
//! output never appear here either: a worker receives them as descriptors
//! over its socket, never as paths.

use super::limits::Profile;
use std::ffi::OsString;

/// The hidden first argument that makes the executable run as a worker.
pub const ENTRY: &str = "--gunmetal-worker";

/// What a worker is started to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Job {
    /// Confine itself, then serve jobs from its socket.
    Serve,
    /// Confine itself and report what was enforced (SEC-MED-024).
    SelfTest,
}

impl Job {
    /// Every job, for reading the word back.
    const ALL: [Self; 2] = [Self::Serve, Self::SelfTest];

    /// The word that names this job in the worker's arguments.
    const fn word(self) -> &'static str {
        match self {
            Self::Serve => "serve",
            Self::SelfTest => "self-test",
        }
    }
}

/// The arguments the launcher gives a program.
///
/// A string does not convert into this type, so no text from outside can
/// become an argument:
///
/// ```compile_fail,E0308
/// use gunmetal_worker::sandbox::{Inherited, Program, launch};
///
/// let (fds, _ours) = Inherited::pair().unwrap();
/// let _ = launch(Program::Worker, "--config=/tmp/evil", fds);
/// ```
///
/// The same call with typed values compiles:
///
/// ```no_run
/// use gunmetal_worker::sandbox::{Inherited, Job, Profile, Program, TypedArgs, launch};
///
/// let (fds, _ours) = Inherited::pair().unwrap();
/// let _ = launch(Program::Worker, TypedArgs::new(Job::Serve, Profile::Scan), fds);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypedArgs {
    job: Job,
    profile: Profile,
}

impl TypedArgs {
    /// Arguments for a worker that does `job` under `profile`.
    #[must_use]
    pub const fn new(job: Job, profile: Profile) -> Self {
        Self { job, profile }
    }

    /// What the worker is to do.
    #[must_use]
    pub const fn job(self) -> Job {
        self.job
    }

    /// The profile the worker confines itself with.
    #[must_use]
    pub const fn profile(self) -> Profile {
        self.profile
    }

    /// The argument list, without the program's own name.
    #[must_use]
    pub(crate) const fn argv(self) -> [&'static str; 3] {
        [ENTRY, self.job.word(), self.profile.word()]
    }

    /// Reads a process's arguments back, the program's own name first.
    ///
    /// Returns the typed arguments when the process was started as a
    /// worker by the launcher, and `None` for every other argument list:
    /// one that is shorter or longer, or that holds any word outside the
    /// vocabulary. The executable calls this before anything else and
    /// enters its worker path only on `Some`.
    #[must_use]
    pub fn from_argv<I: IntoIterator<Item = OsString>>(args: I) -> Option<Self> {
        let mut args = args.into_iter().skip(1);
        let entry = args.next()?;
        let job = args.next()?;
        let profile = args.next()?;
        if args.next().is_some() || entry != ENTRY {
            return None;
        }
        let job = Job::ALL.into_iter().find(|known| job == known.word())?;
        let profile = Profile::ALL
            .into_iter()
            .find(|known| profile == known.word())?;
        Some(Self::new(job, profile))
    }
}

#[cfg(test)]
mod tests {
    use super::{ENTRY, Job, TypedArgs};
    use crate::sandbox::limits::Profile;
    use std::ffi::OsString;

    /// An argument list as the operating system hands it over.
    fn os(words: &[&str]) -> Vec<OsString> {
        words.iter().map(OsString::from).collect()
    }

    #[test]
    fn the_entry_word_is_an_option_no_test_harness_or_parser_accepts() {
        assert_eq!(ENTRY, "--gunmetal-worker");
    }

    /// Every value of the typed arguments, since there are only two.
    ///
    /// Verifies: SEC-HIS-020
    #[test]
    fn every_argument_list_is_three_words_from_the_vocabulary() {
        assert_eq!(
            TypedArgs::new(Job::Serve, Profile::Scan).argv(),
            ["--gunmetal-worker", "serve", "scan"]
        );
        assert_eq!(
            TypedArgs::new(Job::SelfTest, Profile::Scan).argv(),
            ["--gunmetal-worker", "self-test", "scan"]
        );
    }

    #[test]
    fn typed_arguments_give_back_their_job_and_profile() {
        let args = TypedArgs::new(Job::SelfTest, Profile::Scan);
        assert_eq!(args.job(), Job::SelfTest);
        assert_eq!(args.profile(), Profile::Scan);
        assert_eq!(TypedArgs::new(Job::Serve, Profile::Scan).job(), Job::Serve);
    }

    #[test]
    fn a_launched_workers_arguments_read_back() {
        assert_eq!(
            TypedArgs::from_argv(os(&[
                "/proc/self/exe",
                "--gunmetal-worker",
                "serve",
                "scan"
            ])),
            Some(TypedArgs::new(Job::Serve, Profile::Scan))
        );
        assert_eq!(
            TypedArgs::from_argv(os(&["gunmetal", "--gunmetal-worker", "self-test", "scan"])),
            Some(TypedArgs::new(Job::SelfTest, Profile::Scan))
        );
    }

    #[test]
    fn any_other_argument_list_is_not_a_worker() {
        for words in [
            &[][..],
            &["gunmetal"],
            &["gunmetal", "--gunmetal-worker"],
            &["gunmetal", "--gunmetal-worker", "serve"],
            &["gunmetal", "--gunmetal-worker", "serve", "scan", "extra"],
            &["gunmetal", "--worker", "serve", "scan"],
            &["gunmetal", "serve", "--gunmetal-worker", "scan"],
            &["gunmetal", "--gunmetal-worker", "Serve", "scan"],
            &["gunmetal", "--gunmetal-worker", "serve ", "scan"],
            &["gunmetal", "--gunmetal-worker", "serve", "scan/../x"],
            &["gunmetal", "--gunmetal-worker", "serve", ""],
            &["--gunmetal-worker", "serve", "scan"],
        ] {
            assert_eq!(TypedArgs::from_argv(os(words)), None);
        }
    }
}
