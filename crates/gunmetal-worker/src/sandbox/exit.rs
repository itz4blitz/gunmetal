//! How a worker ended, read from its wait status.
//!
//! The server keeps serving when a worker dies (SEC-MED-018), and it needs
//! the reason to report: a forbidden system call, the CPU limit, an abort
//! (which is how running out of memory ends), a crash.

/// How a worker process ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    /// The worker exited by itself with this status code.
    Code(i32),
    /// A signal ended the worker.
    Killed(Cause),
}

/// Why a signal ended a worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    /// `SIGSYS`: the worker made a system call outside its allowlist.
    ForbiddenCall,
    /// `SIGXCPU`: the worker used up its CPU time.
    CpuTime,
    /// `SIGKILL`: the launcher, the kernel's hard CPU limit or the
    /// out-of-memory killer stopped it.
    Kill,
    /// `SIGABRT`: the worker aborted, which is how a failed allocation
    /// under the memory limit ends.
    Abort,
    /// `SIGSEGV` or `SIGBUS`: the worker crashed on a memory fault, for
    /// example by reading a mapped file past the end it was truncated to.
    ///
    /// A worker under the seccomp filter is never reported this way. The
    /// Rust runtime handles both signals itself, to tell a stack overflow
    /// from any other fault. For a stack overflow it prints a message and
    /// aborts, which is [`Cause::Abort`]. For any other fault it puts the
    /// signal's default action back with `rt_sigaction` and returns, so
    /// that the fault happens again and ends the process. `rt_sigaction`
    /// is not on the allowlist (`syscalls.rs`), so the filter kills the
    /// worker with `SIGSYS` at that call, and the server sees
    /// [`Cause::ForbiddenCall`] for what was a crash. Only a worker without
    /// the filter, at the reduced tier, ends as `Fault`.
    ///
    /// Whether to list `rt_sigaction`, so that a crash can be told from a
    /// forbidden call, is a question for WP-079, which takes over the
    /// allowlist.
    Fault,
    /// Any other signal, by number.
    Other(i32),
}

impl Cause {
    /// The cause a signal number stands for. The numbers are Linux's on
    /// every architecture Gunmetal's server builds for.
    const fn of(signal: i32) -> Self {
        match signal {
            31 => Self::ForbiddenCall,
            24 => Self::CpuTime,
            9 => Self::Kill,
            6 => Self::Abort,
            11 | 7 => Self::Fault,
            other => Self::Other(other),
        }
    }
}

impl Exit {
    /// Reads a raw wait status, as `waitpid` returns it.
    ///
    /// The low seven bits hold the signal that ended the process, or zero
    /// when it exited by itself; the next byte up then holds its code.
    #[must_use]
    pub const fn of_raw(status: i32) -> Self {
        let signal = status & 0x7f;
        if signal == 0 {
            Self::Code((status >> 8) & 0xff)
        } else {
            Self::Killed(Cause::of(signal))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Cause, Exit};

    #[test]
    fn a_status_without_a_signal_is_an_exit_code() {
        assert_eq!(Exit::of_raw(0), Exit::Code(0));
        assert_eq!(Exit::of_raw(0x0100), Exit::Code(1));
        assert_eq!(Exit::of_raw(0x6500), Exit::Code(101));
        assert_eq!(Exit::of_raw(0xff00), Exit::Code(255));
        assert_eq!(Exit::of_raw(0x0001_ff00), Exit::Code(255));
    }

    #[test]
    fn a_status_with_a_signal_names_the_cause() {
        assert_eq!(Exit::of_raw(31), Exit::Killed(Cause::ForbiddenCall));
        assert_eq!(Exit::of_raw(24), Exit::Killed(Cause::CpuTime));
        assert_eq!(Exit::of_raw(9), Exit::Killed(Cause::Kill));
        assert_eq!(Exit::of_raw(6), Exit::Killed(Cause::Abort));
        assert_eq!(Exit::of_raw(11), Exit::Killed(Cause::Fault));
        assert_eq!(Exit::of_raw(7), Exit::Killed(Cause::Fault));
        assert_eq!(Exit::of_raw(15), Exit::Killed(Cause::Other(15)));
    }

    #[test]
    fn the_core_dump_bit_and_the_code_byte_do_not_change_the_cause() {
        assert_eq!(Exit::of_raw(0x80 | 6), Exit::Killed(Cause::Abort));
        assert_eq!(
            Exit::of_raw(0x0100 | 31),
            Exit::Killed(Cause::ForbiddenCall)
        );
    }
}
