//! The kernel calls confinement is made of, behind a trait.
//!
//! [`confine_with`](super::confine::confine_with) holds the order and the
//! decisions and is tested against a recording kernel. [`Linux`] is the
//! real one: each method is one call with no decision in it.
//!
//! No method here needs `unsafe`, and that sets one limit. A process
//! cannot close a descriptor it has only a number for without `unsafe`.
//! The launcher marks every extra descriptor close-on-exec before it
//! starts the worker, so a child started that way has only its socket.
//! [`confine_with`](super::confine::confine_with) still refuses if any
//! extra remains.

use super::limits::Limit;
use super::syscalls::{ALLOWLIST, Allowed, Check, Test, checks};
use landlock::{
    ABI, Access, AccessFs, AccessNet, Ruleset, RulesetAttr, RulesetCreated, RulesetStatus, Scope,
};
use rustix::fs::{CWD, Dir, Mode, OFlags};
use rustix::process::{DumpableBehavior, Rlimit};
use seccompiler::{
    BpfProgram, SeccompAction, SeccompCmpArgLen, SeccompCmpOp, SeccompCondition, SeccompFilter,
    SeccompRule, TargetArch,
};
use std::collections::BTreeMap;
use std::io;
use std::os::fd::AsRawFd;

/// What confinement asks of the kernel.
pub(crate) trait Kernel {
    /// How many threads the process has.
    fn threads(&mut self) -> io::Result<usize>;
    /// Sets one resource limit.
    fn limit(&mut self, limit: Limit) -> io::Result<()>;
    /// Clears the dumpable flag: no core dump, and no other process of
    /// the same user may read this one's memory or attach to it.
    fn undumpable(&mut self) -> io::Result<()>;
    /// The numbers of every open descriptor.
    fn descriptors(&mut self) -> io::Result<Vec<u32>>;
    /// Sets `no_new_privs`.
    fn no_new_privs(&mut self) -> io::Result<()>;
    /// Enforces a Landlock ruleset that grants no filesystem access, no
    /// TCP bind or connect, and scopes abstract sockets and signals, as
    /// far as the kernel's ABI goes. `false` when the kernel has no
    /// Landlock.
    fn landlock(&mut self) -> bool;
    /// Installs the seccomp allowlist. `false` when the architecture or
    /// the kernel has no seccomp filter.
    fn seccomp(&mut self) -> bool;
}

/// The running kernel.
pub(crate) struct Linux;

/// The architecture's seccomp target and the allowlist column that holds
/// its call numbers.
type NativeArch = (TargetArch, fn(&Allowed) -> i64);

/// The seccomp target and the allowlist column for the architecture this
/// build is for. `None` on an architecture seccompiler does not support,
/// 32-bit ARM among them, where workers run at the reduced tier
/// (SEC-MED-024).
#[cfg(target_arch = "x86_64")]
const NATIVE: Option<NativeArch> = Some((TargetArch::x86_64, |allowed| allowed.x86_64));
#[cfg(target_arch = "aarch64")]
const NATIVE: Option<NativeArch> = Some((TargetArch::aarch64, |allowed| allowed.aarch64));
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
const NATIVE: Option<NativeArch> = None;

/// Flags for listing a `/proc/self` directory: read-only, a directory,
/// and closed on exec so the listing descriptor is not inherited.
const PROC_DIR_FLAGS: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::CLOEXEC);

/// The numeric names in a directory of `/proc/self`, and the number of
/// the descriptor that was opened to read them.
fn numbers(directory: &str) -> io::Result<(i32, Vec<u32>)> {
    #[expect(
        clippy::disallowed_methods,
        reason = "the worker reads its own /proc/self entries, fixed paths, before it gives up the filesystem (SEC-MED-022)"
    )]
    let opened = rustix::fs::openat(CWD, directory, PROC_DIR_FLAGS, Mode::empty());
    opened
        .and_then(|descriptor| {
            let own = descriptor.as_raw_fd();
            Dir::new(descriptor).map(|entries| (own, entries))
        })
        .and_then(|(own, entries)| {
            entries
                .map(|entry| {
                    entry.map(|entry| {
                        entry
                            .file_name()
                            .to_str()
                            .ok()
                            .and_then(|name| name.parse().ok())
                    })
                })
                .collect::<Result<Vec<Option<u32>>, _>>()
                .map(|names| (own, names.into_iter().flatten().collect()))
        })
        .map_err(io::Error::from)
}

/// One comparison, in seccompiler's terms.
fn condition(check: &Check) -> Result<SeccompCondition, seccompiler::BackendError> {
    let operator = match check.test {
        Test::Is => SeccompCmpOp::Eq,
        Test::MaskedIs(mask) => SeccompCmpOp::MaskedEq(mask),
    };
    SeccompCondition::new(
        check.argument,
        SeccompCmpArgLen::Dword,
        operator,
        check.value,
    )
}

/// Accepts only a cleared dumpable flag.
fn require_not_dumpable(behavior: DumpableBehavior) -> io::Result<()> {
    match behavior {
        DumpableBehavior::NotDumpable => Ok(()),
        other => Err(io::Error::other(format!("dumpable is {other:?}"))),
    }
}

/// Builds the Landlock ruleset: every filesystem right is handled and
/// no path is granted, so all path-based access is denied. TCP, UDP and
/// the abstract-socket and signal scopes are handled where the ABI has
/// them (SEC-MED-022).
fn landlock_ruleset() -> Result<RulesetCreated, landlock::RulesetError> {
    let abi = ABI::V6;
    Ruleset::default()
        .handle_access(AccessFs::from_all(abi))
        .and_then(|ruleset| ruleset.handle_access(AccessNet::from_all(abi)))
        .and_then(|ruleset| ruleset.scope(Scope::from_all(abi)))
        .and_then(Ruleset::create)
}

/// The compiled filter for `native`: every call on the allowlist is
/// allowed when its checks hold, and anything else kills the process.
/// `None` when there is no native architecture, as on 32-bit ARM.
fn program_for(native: Option<NativeArch>, pid: u32) -> Option<BpfProgram> {
    native.and_then(|(arch, number)| {
        ALLOWLIST
            .iter()
            .copied()
            .map(|allowed| {
                checks(allowed.guard, pid)
                    .iter()
                    .map(condition)
                    .collect::<Result<Vec<_>, _>>()
                    // A rule with no condition is an error in seccompiler,
                    // and no rule at all is how it spells "always".
                    .map(|conditions| SeccompRule::new(conditions).into_iter().collect())
                    .map(|rules: Vec<SeccompRule>| (number(&allowed), rules))
            })
            .collect::<Result<BTreeMap<i64, Vec<SeccompRule>>, _>>()
            .and_then(|rules| {
                SeccompFilter::new(
                    rules,
                    SeccompAction::KillProcess,
                    SeccompAction::Allow,
                    arch,
                )
            })
            .and_then(BpfProgram::try_from)
            .ok()
    })
}

/// The compiled filter for this architecture.
fn program(pid: u32) -> Option<BpfProgram> {
    program_for(NATIVE, pid)
}

impl Kernel for Linux {
    fn threads(&mut self) -> io::Result<usize> {
        numbers("/proc/self/task").map(|(_, tasks)| tasks.len())
    }

    fn limit(&mut self, limit: Limit) -> io::Result<()> {
        rustix::process::setrlimit(
            limit.resource,
            Rlimit {
                current: Some(limit.soft),
                maximum: Some(limit.hard),
            },
        )
        .map_err(io::Error::from)
    }

    fn undumpable(&mut self) -> io::Result<()> {
        rustix::process::set_dumpable_behavior(DumpableBehavior::NotDumpable)
            .map_err(io::Error::from)
            .and_then(|()| rustix::process::dumpable_behavior().map_err(io::Error::from))
            .and_then(require_not_dumpable)
    }

    fn descriptors(&mut self) -> io::Result<Vec<u32>> {
        numbers("/proc/self/fd").map(|(own, open)| {
            open.into_iter()
                .filter(|&descriptor| i64::from(descriptor) != i64::from(own))
                .collect()
        })
    }

    fn no_new_privs(&mut self) -> io::Result<()> {
        rustix::thread::set_no_new_privs(true).map_err(io::Error::from)
    }

    fn landlock(&mut self) -> bool {
        landlock_ruleset()
            .and_then(RulesetCreated::restrict_self)
            .is_ok_and(|status| status.ruleset != RulesetStatus::NotEnforced)
    }

    fn seccomp(&mut self) -> bool {
        let pid = rustix::process::getpid()
            .as_raw_nonzero()
            .get()
            .unsigned_abs();
        program(pid).is_some_and(|filter| seccompiler::apply_filter(&filter).is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::{Kernel, Linux, numbers, program, program_for};
    use std::io::ErrorKind;
    use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU8, Ordering};
    use std::time::Duration;

    #[test]
    fn the_running_process_lists_its_threads_and_descriptors() {
        let mut linux = Linux;
        assert!(linux.threads().unwrap() >= 1);
        let descriptors = linux.descriptors().unwrap();
        assert!(descriptors.contains(&0));
        assert!(descriptors.contains(&1));
        assert!(descriptors.contains(&2));
    }

    #[test]
    fn a_missing_or_non_directory_proc_entry_is_an_error() {
        assert_eq!(
            numbers("/proc/self/does-not-exist").unwrap_err().kind(),
            ErrorKind::NotFound
        );
        assert_eq!(
            numbers("/proc/self/status").unwrap_err().kind(),
            ErrorKind::NotADirectory
        );
    }

    #[test]
    fn a_listing_holds_numbers_only_and_names_its_own_descriptor() {
        let (own, open) = numbers("/proc/self/fd").unwrap();
        assert!(own > 2);
        assert!(open.contains(&0));
        assert!(open.contains(&u32::try_from(own).unwrap()));
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    #[test]
    fn the_seccomp_filter_compiles_for_this_architecture() {
        assert!(program(1).is_some_and(|filter| !filter.is_empty()));
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    #[test]
    fn the_seccomp_filter_is_absent_on_this_architecture() {
        assert_eq!(program(1), None);
    }

    /// An architecture seccompiler has no target for, 32-bit ARM among
    /// them, gets no filter, so [`Linux::seccomp`] reports the control
    /// missing there.
    #[test]
    fn no_native_architecture_means_no_filter() {
        assert_eq!(program_for(None, 1), None);
        assert_eq!(program_for(None, 4321), None);
    }

    /// Whether this thread can list a directory by path, or why not.
    fn list_by_path() -> Result<(), ErrorKind> {
        numbers("/proc/self/task")
            .map(|_| ())
            .map_err(|error| error.kind())
    }

    /// Whether the running kernel has Landlock active, read from the
    /// kernel's own list of security modules and not from the code under
    /// test.
    fn kernel_has_landlock() -> bool {
        #[expect(
            clippy::disallowed_methods,
            reason = "the test reads the kernel's list of security modules, a fixed path, to know what to expect of Landlock (SEC-MED-024)"
        )]
        let modules = std::fs::read_to_string("/sys/kernel/security/lsm").unwrap();
        modules.trim().split(',').any(|name| name == "landlock")
    }

    /// Landlock binds the thread that enforces it, so a thread of its own
    /// can try it and leave the test process free. Where the kernel has
    /// Landlock every path is refused; where it has none the control is
    /// reported missing and the path still opens.
    ///
    /// Verifies: SEC-MED-022
    #[test]
    fn landlock_refuses_every_path_on_the_thread_that_enforced_it() {
        let offered = kernel_has_landlock();
        let (enforced, listing) = std::thread::spawn(|| (Linux.landlock(), list_by_path()))
            .join()
            .unwrap();
        assert_eq!(enforced, offered);
        assert_eq!(
            listing,
            [Ok(()), Err(ErrorKind::PermissionDenied)][usize::from(offered)]
        );
        assert_eq!(list_by_path(), Ok(()));
    }

    /// Makes the kernel answer `ENOSYS`, as one built without the call
    /// does, to each of `calls` on the calling thread from now on.
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    fn take_away(calls: &[i64]) {
        use seccompiler::{BpfProgram, SeccompAction, SeccompFilter};

        let (arch, _) = super::NATIVE.unwrap();
        let rules = calls.iter().map(|&call| (call, Vec::new())).collect();
        let filter =
            SeccompFilter::new(rules, SeccompAction::Allow, SeccompAction::Errno(38), arch)
                .unwrap();
        seccompiler::apply_filter(&BpfProgram::try_from(filter).unwrap()).unwrap();
    }

    /// On a kernel without Landlock the control is reported missing, never
    /// assumed: the three Landlock calls, numbers 444 to 446 on every
    /// architecture, answer `ENOSYS` on this thread, and the path that
    /// Landlock would have refused still opens.
    ///
    /// Verifies: SEC-MED-024
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    #[test]
    fn landlock_is_reported_missing_on_a_kernel_without_it() {
        let (enforced, listing) = std::thread::spawn(|| {
            take_away(&[444, 445, 446]);
            (Linux.landlock(), list_by_path())
        })
        .join()
        .unwrap();
        assert!(!enforced);
        assert_eq!(listing, Ok(()));
    }

    /// The `seccomp` call's number on this architecture.
    #[cfg(target_arch = "x86_64")]
    const SECCOMP_CALL: i64 = 317;
    #[cfg(target_arch = "aarch64")]
    const SECCOMP_CALL: i64 = 277;

    /// On a kernel without seccomp filters the control is reported
    /// missing: the `seccomp` call answers `ENOSYS` on this thread, so
    /// the worker's filter cannot be installed.
    ///
    /// Verifies: SEC-MED-024
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    #[test]
    fn seccomp_is_reported_missing_on_a_kernel_without_it() {
        let installed = std::thread::spawn(|| {
            take_away(&[SECCOMP_CALL]);
            Linux.seccomp()
        })
        .join()
        .unwrap();
        assert!(!installed);
    }

    /// The `Seccomp:` line of a thread's status: 0 without a filter, 2
    /// with one.
    fn seccomp_mode(tid: i32) -> Option<String> {
        #[expect(
            clippy::disallowed_methods,
            reason = "the test reads a thread's own /proc status to see the filter the kernel holds for it (SEC-MED-022)"
        )]
        let status = std::fs::read_to_string(format!("/proc/self/task/{tid}/status")).unwrap();
        status
            .lines()
            .find_map(|line| line.strip_prefix("Seccomp:"))
            .map(|mode| mode.trim().to_owned())
    }

    /// A seccomp filter binds the thread that installs it. The helper
    /// thread installs the worker's filter and then only spins, which
    /// needs no system call, until this thread has read its status.
    ///
    /// Verifies: SEC-MED-022
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    #[test]
    fn seccomp_installs_the_filter_on_the_thread_that_asked() {
        let tid = AtomicI32::new(0);
        // 0: not done yet. 1: no filter. 2: installed.
        let outcome = AtomicU8::new(0);
        let seen = AtomicBool::new(false);
        let mode = std::thread::scope(|scope| {
            scope.spawn(|| {
                tid.store(
                    rustix::thread::gettid().as_raw_nonzero().get(),
                    Ordering::SeqCst,
                );
                Linux.no_new_privs().unwrap();
                let installed = Linux.seccomp();
                outcome.store(u8::from(installed).saturating_add(1), Ordering::SeqCst);
                while !seen.load(Ordering::SeqCst) {
                    std::hint::spin_loop();
                }
            });
            while outcome.load(Ordering::SeqCst) == 0 {
                std::thread::sleep(Duration::from_millis(1));
            }
            let mode = seccomp_mode(tid.load(Ordering::SeqCst));
            seen.store(true, Ordering::SeqCst);
            mode
        });
        assert_eq!(outcome.load(Ordering::SeqCst), 2);
        assert_eq!(mode.as_deref(), Some("2"));
        let own = rustix::thread::gettid().as_raw_nonzero().get();
        assert_eq!(seccomp_mode(own).as_deref(), Some("0"));
    }

    #[test]
    fn proc_directory_flags_are_read_only_directory_and_close_on_exec() {
        use rustix::fs::OFlags;

        assert_eq!(
            super::PROC_DIR_FLAGS,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC
        );
    }

    #[test]
    fn the_running_process_can_clear_dumpable_and_set_no_new_privs() {
        use rustix::process::DumpableBehavior;

        let mut linux = Linux;
        rustix::process::set_dumpable_behavior(DumpableBehavior::Dumpable).unwrap();
        linux.undumpable().unwrap();
        assert_eq!(
            rustix::process::dumpable_behavior().unwrap(),
            DumpableBehavior::NotDumpable
        );
        linux.no_new_privs().unwrap();
        assert!(rustix::thread::no_new_privs().unwrap());
    }

    #[test]
    fn the_running_process_can_set_a_core_file_limit() {
        use crate::sandbox::limits::Limit;
        use rustix::process::{Resource, getrlimit};

        let mut linux = Linux;
        linux
            .limit(Limit {
                resource: Resource::Core,
                soft: 0,
                hard: 0,
            })
            .unwrap();
        let limit = getrlimit(Resource::Core);
        assert_eq!(limit.current, Some(0));
        assert_eq!(limit.maximum, Some(0));
    }

    #[test]
    fn only_a_cleared_dumpable_flag_is_accepted() {
        use rustix::process::DumpableBehavior;

        let verdict = |behavior| super::require_not_dumpable(behavior).map_err(|e| e.to_string());
        assert_eq!(verdict(DumpableBehavior::NotDumpable), Ok(()));
        assert_eq!(
            verdict(DumpableBehavior::Dumpable),
            Err("dumpable is Dumpable".to_owned())
        );
        assert_eq!(
            verdict(DumpableBehavior::DumpableReadableOnlyByRoot),
            Err("dumpable is DumpableReadableOnlyByRoot".to_owned())
        );
    }
}
