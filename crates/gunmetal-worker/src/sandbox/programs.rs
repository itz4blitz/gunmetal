//! The closed list of programs the launcher may start (SEC-HIS-020,
//! SEC-MED-063).
//!
//! This file is a registry. R1 has one program, the worker, which is the
//! running executable started again. A package that needs another program
//! adds one variant here, with its review, and one line to
//! [`Program::executable`].

/// A program the sandbox launcher may start.
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

    #[test]
    fn the_worker_is_the_running_executable() {
        assert_eq!(Program::Worker.executable(), "/proc/self/exe");
    }
}
