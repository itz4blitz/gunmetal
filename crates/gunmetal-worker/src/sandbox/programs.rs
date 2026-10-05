//! The closed list of programs the launcher may start (SEC-HIS-020,
//! SEC-MED-063).
//!
//! This file is a registry. R1 has one program, the worker, which is the
//! running executable started again. A package that needs another program
//! adds one variant here, with its review, and one line to
//! [`Program::executable`].

/// A program the sandbox launcher may start.
///
/// The launcher takes one of these, never a path, so no request, setting,
/// tag or plugin can choose what it runs (SEC-HIS-021, SEC-TM-046):
///
/// ```compile_fail,E0308
/// use gunmetal_worker::sandbox::{Inherited, Job, Profile, TypedArgs, launch};
///
/// let (fds, _ours) = Inherited::pair().unwrap();
/// let _ = launch("/bin/sh", TypedArgs::new(Job::Serve, Profile::Scan), fds);
/// ```
///
/// Nor can code outside this crate ask a program for its location:
///
/// ```compile_fail,E0624
/// use gunmetal_worker::sandbox::Program;
///
/// let _ = Program::Worker.executable();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Program {
    /// The media worker: this same executable, entered through its hidden
    /// worker arguments.
    Worker,
}

impl Program {
    /// Where the program's executable is.
    ///
    /// The location is fixed here at build time. No request, setting, tag
    /// or file name reaches it (SEC-HIS-021, SEC-TM-046). The worker is
    /// `/proc/self/exe`, the kernel's own link to the image that is
    /// running, so it is the installed server binary even after the file
    /// on disk has been replaced by an upgrade.
    #[must_use]
    pub(crate) const fn executable(self) -> &'static str {
        match self {
            Self::Worker => "/proc/self/exe",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Program;

    /// The location is the kernel's link to the running image, not a
    /// path on disk. That no caller can choose it is shown by the
    /// compile-fail examples on [`Program`].
    #[test]
    fn the_worker_is_the_running_executable() {
        assert_eq!(Program::Worker.executable(), "/proc/self/exe");
    }
}
