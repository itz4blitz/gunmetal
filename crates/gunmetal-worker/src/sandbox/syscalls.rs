//! The worker's system call allowlist (SEC-MED-022).
//!
//! The seccomp filter kills the process on any call that is not listed
//! here, so the list is the whole of what a confined worker can ask the
//! kernel for. It is deliberately the minimum a worker needs to read its
//! socket, answer on it, manage memory, and end: no call here creates a
//! socket, a process or a thread, opens a path, runs a program, traces or
//! mounts.
//!
//! This file changes hands: WP-045 ships the mechanism and this minimal
//! list, proven against a stub worker; WP-079 extends it from audit runs
//! of the real jobs. Each row carries the call's number on both server
//! architectures, taken from the kernel's tables.
//!
//! The list is shaped by glibc. musl raises an abort through `tkill`, not
//! `tgkill`, so a static or Alpine build would report an out-of-memory
//! abort as a forbidden call; such a build needs this list and the
//! [`Guard::AbortSelf`] guard looked at again (a note for WP-079).

/// When a listed call is allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Guard {
    /// Always.
    Always,
    /// `mmap`: only without `PROT_EXEC`.
    Map,
    /// `tgkill`: only to send `SIGABRT` to the worker itself, which is how
    /// an abort (a failed allocation, a panic) ends the process with its
    /// real cause instead of a filter violation.
    AbortSelf,
}

/// One row of the allowlist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Allowed {
    /// The call's name.
    pub(crate) name: &'static str,
    /// Its number on x86-64.
    pub(crate) x86_64: i64,
    /// Its number on `AArch64`.
    pub(crate) aarch64: i64,
    /// When it is allowed.
    pub(crate) guard: Guard,
}

/// A row that is always allowed.
const fn always(name: &'static str, x86_64: i64, aarch64: i64) -> Allowed {
    Allowed {
        name,
        x86_64,
        aarch64,
        guard: Guard::Always,
    }
}

/// A row with a guard.
const fn guarded(name: &'static str, x86_64: i64, aarch64: i64, guard: Guard) -> Allowed {
    Allowed {
        name,
        x86_64,
        aarch64,
        guard,
    }
}

/// The allowlist, sorted by name.
pub(crate) const ALLOWLIST: [Allowed; 21] = [
    // Memory: the allocator grows and shrinks the heap.
    always("brk", 12, 214),
    // Time: normally answered without a call; listed for kernels where it
    // is not.
    always("clock_gettime", 228, 113),
    always("close", 3, 57),
    // Ending: a thread's and the process's exit.
    always("exit", 60, 93),
    always("exit_group", 231, 94),
    // Locks in the standard library.
    always("futex", 202, 98),
    // Abort: glibc reads the process and thread IDs, then signals itself.
    always("getpid", 39, 172),
    // Seeds for hash tables.
    always("getrandom", 318, 278),
    always("gettid", 186, 178),
    always("madvise", 28, 233),
    guarded("mmap", 9, 222, Guard::Map),
    always("mremap", 25, 216),
    always("munmap", 11, 215),
    // The socket, as descriptors 0, 1 and 2. A worker has no other
    // descriptor: confinement refuses to go on if it finds one.
    always("read", 0, 63),
    always("restart_syscall", 219, 128),
    // Signal masks and the return from a handler, used by abort and by
    // the standard library's stack-overflow guard.
    always("rt_sigprocmask", 14, 135),
    always("rt_sigreturn", 15, 139),
    always("sigaltstack", 131, 132),
    guarded("tgkill", 234, 131, Guard::AbortSelf),
    always("write", 1, 64),
    always("writev", 20, 66),
];

/// How a system call argument is compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Test {
    /// The argument equals the value.
    Is,
    /// The argument's bits under this mask equal the value.
    MaskedIs(u64),
}

/// One comparison of a system call's argument, as a 32-bit value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Check {
    /// The argument's index, from 0.
    pub(crate) argument: u8,
    /// The comparison.
    pub(crate) test: Test,
    /// The value compared with.
    pub(crate) value: u64,
}

/// `PROT_EXEC`, the `mmap` protection bit for executable memory.
const PROT_EXEC: u64 = 4;
/// `SIGABRT`'s number.
const SIGABRT: u64 = 6;

/// The comparisons that must all hold for a call with this guard to be
/// allowed. An empty list allows the call always. `pid` is the worker's
/// own process ID.
pub(crate) fn checks(guard: Guard, pid: u32) -> Vec<Check> {
    match guard {
        Guard::Always => Vec::new(),
        Guard::Map => vec![Check {
            argument: 2,
            test: Test::MaskedIs(PROT_EXEC),
            value: 0,
        }],
        Guard::AbortSelf => vec![
            Check {
                argument: 0,
                test: Test::Is,
                value: u64::from(pid),
            },
            Check {
                argument: 2,
                test: Test::Is,
                value: SIGABRT,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::{ALLOWLIST, Allowed, Check, Guard, Test, always, checks, guarded};

    /// The calls SEC-MED-022 and SEC-TM-044 name as excluded, and the
    /// other ways to do the same things, with their numbers on x86-64 and
    /// `AArch64` (-1 where the architecture has no such call).
    const FORBIDDEN: [(&str, i64, i64); 22] = [
        ("accept", 43, 202),
        ("bind", 49, 200),
        ("clone", 56, 220),
        ("clone3", 435, 435),
        ("connect", 42, 203),
        ("execve", 59, 221),
        ("execveat", 322, 281),
        ("fork", 57, -1),
        ("kill", 62, 129),
        ("mount", 165, 40),
        ("mprotect", 10, 226),
        ("open", 2, -1),
        ("openat", 257, 56),
        ("openat2", 437, 437),
        ("prctl", 157, 167),
        ("ptrace", 101, 117),
        ("seccomp", 317, 277),
        ("sendto", 44, 206),
        ("socket", 41, 198),
        ("socketpair", 53, 199),
        ("unshare", 272, 97),
        ("vfork", 58, -1),
    ];

    #[test]
    fn the_allowlist_excludes_sockets_exec_clone_fork_ptrace_and_mount() {
        for (name, x86_64, aarch64) in FORBIDDEN {
            assert_eq!(ALLOWLIST.iter().find(|row| row.name == name), None);
            assert_eq!(ALLOWLIST.iter().find(|row| row.x86_64 == x86_64), None);
            assert_eq!(ALLOWLIST.iter().find(|row| row.aarch64 == aarch64), None);
        }
    }

    #[test]
    fn the_allowlist_is_sorted_and_names_each_call_and_number_once() {
        let names: Vec<&str> = ALLOWLIST.iter().map(|row| row.name).collect();
        assert_eq!(
            names,
            [
                "brk",
                "clock_gettime",
                "close",
                "exit",
                "exit_group",
                "futex",
                "getpid",
                "getrandom",
                "gettid",
                "madvise",
                "mmap",
                "mremap",
                "munmap",
                "read",
                "restart_syscall",
                "rt_sigprocmask",
                "rt_sigreturn",
                "sigaltstack",
                "tgkill",
                "write",
                "writev",
            ]
        );
        let mut x86_64: Vec<i64> = ALLOWLIST.iter().map(|row| row.x86_64).collect();
        x86_64.sort_unstable();
        assert_eq!(
            x86_64,
            [
                0, 1, 3, 9, 11, 12, 14, 15, 20, 25, 28, 39, 60, 131, 186, 202, 219, 228, 231, 234,
                318
            ]
        );
        let mut aarch64: Vec<i64> = ALLOWLIST.iter().map(|row| row.aarch64).collect();
        aarch64.sort_unstable();
        assert_eq!(
            aarch64,
            [
                57, 63, 64, 66, 93, 94, 98, 113, 128, 131, 132, 135, 139, 172, 178, 214, 215, 216,
                222, 233, 278
            ]
        );
    }

    #[test]
    fn only_mmap_and_tgkill_are_guarded() {
        let guarded: Vec<(&str, Guard)> = ALLOWLIST
            .iter()
            .filter(|row| row.guard != Guard::Always)
            .map(|row| (row.name, row.guard))
            .collect();
        assert_eq!(
            guarded,
            [("mmap", Guard::Map), ("tgkill", Guard::AbortSelf)]
        );
    }

    #[test]
    fn the_row_constructors_fill_every_field() {
        assert_eq!(
            always("brk", 12, 214),
            Allowed {
                name: "brk",
                x86_64: 12,
                aarch64: 214,
                guard: Guard::Always,
            }
        );
        assert_eq!(
            guarded("mmap", 9, 222, Guard::Map),
            Allowed {
                name: "mmap",
                x86_64: 9,
                aarch64: 222,
                guard: Guard::Map,
            }
        );
    }

    #[test]
    fn an_unguarded_call_has_no_checks() {
        assert_eq!(checks(Guard::Always, 77), []);
    }

    #[test]
    fn mmap_refuses_executable_memory() {
        assert_eq!(
            checks(Guard::Map, 77),
            [Check {
                argument: 2,
                test: Test::MaskedIs(4),
                value: 0
            }]
        );
    }

    #[test]
    fn tgkill_only_sends_abort_to_the_worker_itself() {
        assert_eq!(
            checks(Guard::AbortSelf, 4321),
            [
                Check {
                    argument: 0,
                    test: Test::Is,
                    value: 4321
                },
                Check {
                    argument: 2,
                    test: Test::Is,
                    value: 6
                },
            ]
        );
    }
}
