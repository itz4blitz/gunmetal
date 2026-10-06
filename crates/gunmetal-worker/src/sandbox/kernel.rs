//! The kernel calls confinement is made of, behind a trait.
//!
//! [`confine_with`](super::confine::confine_with) holds the order and the
//! decisions and is tested against a recording kernel. [`Linux`] is the
//! real one: each method is one call with no decision in it.
//!
//! No method here needs `unsafe`, and that sets one limit: the worker
//! cannot close a descriptor it has only a number for. The launcher marks
//! every extra descriptor close-on-exec before it starts the worker, in
//! the crate's one `unsafe` block ([`descriptors`](super::descriptors),
//! ADR 13), so a child started that way has only its socket.
//! [`confine_with`](super::confine::confine_with) still refuses if any
//! extra remains.

use super::limits::Limit;
use super::syscalls::{ALLOWLIST, Allowed, Check, Test, checks};
use super::tier::Landlock;
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
    /// far as the kernel's ABI goes, and says how far that is: the whole
    /// ruleset, part of it, or none where the kernel has no Landlock.
    fn landlock(&mut self) -> Landlock;
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
/// no path is granted, so all path-based access is denied. TCP bind and
/// connect are handled and no port is granted, and abstract sockets and
/// signals are scoped to the worker, each where the kernel's ABI has it
/// (SEC-MED-022).
///
/// Landlock has no rule for UDP, in this ABI or an earlier one. What
/// keeps a worker from UDP is the seccomp filter, which lists no call
/// that makes a socket; where the filter is missing nothing does, and the
/// notice names seccomp.
fn landlock_ruleset() -> Result<RulesetCreated, landlock::RulesetError> {
    let abi = ABI::V6;
    Ruleset::default()
        .handle_access(AccessFs::from_all(abi))
        .and_then(|ruleset| ruleset.handle_access(AccessNet::from_all(abi)))
        .and_then(|ruleset| ruleset.scope(Scope::from_all(abi)))
        .and_then(Ruleset::create)
}

/// What Landlock's own account of a ruleset comes to. A kernel older than
/// the ruleset enforces the rules it has and says so; that is reported as
/// it is, not as the whole ruleset (SEC-MED-024).
fn coverage(status: &RulesetStatus) -> Landlock {
    match status {
        RulesetStatus::FullyEnforced => Landlock::Full,
        RulesetStatus::PartiallyEnforced => Landlock::Partial,
        RulesetStatus::NotEnforced => Landlock::Missing,
    }
}

/// The rules for one allowlist row. A row without checks has no rule,
/// which is how seccompiler spells "always" (a rule with no condition is
/// an error there). A row with checks has one rule that needs all of
/// them, and an error building it is returned, so a guarded row can never
/// compile to an unconditional one.
fn rules(conditions: Vec<SeccompCondition>) -> Result<Vec<SeccompRule>, seccompiler::BackendError> {
    if conditions.is_empty() {
        Ok(Vec::new())
    } else {
        SeccompRule::new(conditions).map(|rule| vec![rule])
    }
}

/// The compiled filter for `native`: every call on the allowlist is
/// allowed when its checks hold, and anything else kills the process.
/// `None` when there is no native architecture, as on 32-bit ARM, or when
/// any part of the filter fails to build.
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
                    .and_then(rules)
                    .map(|rules| (number(&allowed), rules))
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

    fn landlock(&mut self) -> Landlock {
        landlock_ruleset()
            .and_then(RulesetCreated::restrict_self)
            .map_or(Landlock::Missing, |status| coverage(&status.ruleset))
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
    use super::{Kernel, Linux, NativeArch, coverage, numbers, program, program_for, rules};
    use crate::sandbox::tier::Landlock;
    use landlock::RulesetStatus;
    use seccompiler::{
        SeccompCmpArgLen, SeccompCmpOp, SeccompCondition, SeccompRule, TargetArch, sock_filter,
    };
    use std::collections::BTreeSet;
    use std::io::ErrorKind;

    /// A row without checks has no rule, which seccompiler reads as
    /// "always"; a row with checks has one rule that needs all of them.
    #[test]
    fn a_row_is_unconditional_only_when_it_has_no_checks() {
        assert_eq!(rules(Vec::new()), Ok(Vec::new()));
        let checks = vec![
            SeccompCondition::new(0, SeccompCmpArgLen::Dword, SeccompCmpOp::Eq, 4321).unwrap(),
            SeccompCondition::new(2, SeccompCmpArgLen::Dword, SeccompCmpOp::Eq, 6).unwrap(),
        ];
        assert_eq!(
            rules(checks.clone()),
            Ok(vec![SeccompRule::new(checks).unwrap()])
        );
    }

    /// The verdict that lets a call through.
    const ALLOW: u32 = 0x7fff_0000;
    /// The verdict that kills the whole process.
    const KILL: u32 = 0x8000_0000;
    /// `AUDIT_ARCH_X86_64` and `AUDIT_ARCH_AARCH64`, the architecture
    /// values the kernel puts in `seccomp_data`.
    const X86_64: u32 = 0xc000_003e;
    const AARCH64: u32 = 0xc000_00b7;
    /// The worker's process ID in these cases.
    const PID: u64 = 4321;

    /// The kernel's `struct seccomp_data` for one call, little-endian:
    /// the call number at 0, the architecture at 4, the instruction
    /// pointer at 8 (left zero), and six 64-bit arguments from 16.
    fn seccomp_data(arch: u32, call: u32, args: [u64; 6]) -> [u8; 64] {
        let mut data = [0_u8; 64];
        data[0..4].copy_from_slice(&call.to_le_bytes());
        data[4..8].copy_from_slice(&arch.to_le_bytes());
        for (index, arg) in args.iter().enumerate() {
            let at = 16 + 8 * index;
            data[at..at + 8].copy_from_slice(&arg.to_le_bytes());
        }
        data
    }

    /// Classic BPF as the kernel runs a seccomp filter, for the
    /// instructions [`run`] knows: load a 32-bit word at an absolute
    /// offset, `and` with a constant, jump always, jump if equal to a
    /// constant, and return.
    const KNOWN: [u16; 5] = [0x05, 0x06, 0x15, 0x20, 0x54];

    /// The filter's verdict on `data`. The caller has checked that the
    /// program uses only [`KNOWN`] instructions, so anything not matched
    /// below is a return.
    fn run(program: &[sock_filter], data: &[u8; 64]) -> u32 {
        let mut accumulator = 0_u32;
        let mut next = 0_usize;
        loop {
            let instruction = &program[next];
            next += 1;
            let k = instruction.k;
            match instruction.code {
                0x20 => {
                    let at = usize::try_from(k).unwrap();
                    accumulator = u32::from_le_bytes(data[at..at + 4].try_into().unwrap());
                }
                0x54 => accumulator &= k,
                0x05 => next += usize::try_from(k).unwrap(),
                0x15 => {
                    let offset = if accumulator == k {
                        instruction.jt
                    } else {
                        instruction.jf
                    };
                    next += usize::from(offset);
                }
                _ => return k,
            }
        }
    }

    /// The x86-64 column of the allowlist.
    const ON_X86_64: NativeArch = (TargetArch::x86_64, |allowed| allowed.x86_64);
    /// The `AArch64` column of the allowlist.
    const ON_AARCH64: NativeArch = (TargetArch::aarch64, |allowed| allowed.aarch64);

    /// Runs the compiled filter for `native` over each case and returns
    /// the verdicts, after checking it uses only instructions [`run`]
    /// knows.
    fn verdicts(native: NativeArch, cases: &[(u32, u32, [u64; 6])]) -> Vec<u32> {
        let program = program_for(Some(native), 4321).unwrap();
        let used: BTreeSet<u16> = program.iter().map(|instruction| instruction.code).collect();
        assert_eq!(used, BTreeSet::from(KNOWN));
        cases
            .iter()
            .map(|&(arch, call, args)| run(&program, &seccomp_data(arch, call, args)))
            .collect()
    }

    /// The compiled program, not only the table, holds the two argument
    /// guards: `mmap` only without `PROT_EXEC` (bit 4 of argument 2),
    /// `tgkill` only as `SIGABRT` (6, argument 2) to the worker's own
    /// process (argument 0). The guards compare the low 32 bits, as the
    /// kernel reads these `int` arguments, so junk in the high half of the
    /// process ID does not change the verdict.
    ///
    /// Verifies: SEC-MED-022, SEC-TM-044
    #[test]
    fn the_compiled_filter_holds_its_argument_guards_on_x86_64() {
        let cases = [
            // mmap(0, 4096, PROT_READ | PROT_WRITE, ...)
            (X86_64, 9, [0, 4096, 3, 0x22, 0, 0]),
            // mmap with PROT_EXEC, alone or with read and write.
            (X86_64, 9, [0, 4096, 4, 0x22, 0, 0]),
            (X86_64, 9, [0, 4096, 5, 0x22, 0, 0]),
            (X86_64, 9, [0, 4096, 7, 0x22, 0, 0]),
            // tgkill(own pid, tid, SIGABRT)
            (X86_64, 234, [PID, 99, 6, 0, 0, 0]),
            (X86_64, 234, [PID | 0x1_0000_0000, 99, 6, 0, 0, 0]),
            // tgkill to another process, or with another signal.
            (X86_64, 234, [PID + 1, 99, 6, 0, 0, 0]),
            (X86_64, 234, [1, 1, 6, 0, 0, 0]),
            (X86_64, 234, [PID, 99, 9, 0, 0, 0]),
            (X86_64, 234, [PID, 6, 9, 0, 0, 0]),
            // write(1, ..) is listed without a guard.
            (X86_64, 1, [1, 0, 16, 0, 0, 0]),
            // openat and mprotect are not listed.
            (X86_64, 257, [0, 0, 0, 0, 0, 0]),
            (X86_64, 10, [0, 4096, 3, 0, 0, 0]),
            // Nor are the calls the requirement names as excluded: execve,
            // clone, clone3, socket, ptrace and mount.
            (X86_64, 59, [0, 0, 0, 0, 0, 0]),
            (X86_64, 56, [0, 0, 0, 0, 0, 0]),
            (X86_64, 435, [0, 0, 0, 0, 0, 0]),
            (X86_64, 41, [0, 0, 0, 0, 0, 0]),
            (X86_64, 101, [0, 0, 0, 0, 0, 0]),
            (X86_64, 165, [0, 0, 0, 0, 0, 0]),
            // write's number from a process of another architecture.
            (AARCH64, 1, [1, 0, 16, 0, 0, 0]),
        ];
        assert_eq!(
            verdicts(ON_X86_64, &cases),
            [
                ALLOW, KILL, KILL, KILL, ALLOW, ALLOW, KILL, KILL, KILL, KILL, ALLOW, KILL, KILL,
                KILL, KILL, KILL, KILL, KILL, KILL, KILL
            ]
        );
    }

    /// The same guards on `AArch64`, where `mmap` is 222, `tgkill` 131,
    /// `write` 64, `openat` 56 and `mprotect` 226, and the excluded calls
    /// are `execve` 221, `clone` 220, `clone3` 435, `socket` 198, `ptrace`
    /// 117 and `mount` 40.
    ///
    /// Verifies: SEC-MED-022, SEC-TM-044
    #[test]
    fn the_compiled_filter_holds_its_argument_guards_on_aarch64() {
        let cases = [
            (AARCH64, 222, [0, 4096, 3, 0x22, 0, 0]),
            (AARCH64, 222, [0, 4096, 4, 0x22, 0, 0]),
            (AARCH64, 222, [0, 4096, 5, 0x22, 0, 0]),
            (AARCH64, 222, [0, 4096, 7, 0x22, 0, 0]),
            (AARCH64, 131, [PID, 99, 6, 0, 0, 0]),
            (AARCH64, 131, [PID + 1, 99, 6, 0, 0, 0]),
            (AARCH64, 131, [PID, 99, 9, 0, 0, 0]),
            (AARCH64, 64, [1, 0, 16, 0, 0, 0]),
            (AARCH64, 56, [0, 0, 0, 0, 0, 0]),
            (AARCH64, 226, [0, 4096, 3, 0, 0, 0]),
            (AARCH64, 221, [0, 0, 0, 0, 0, 0]),
            (AARCH64, 220, [0, 0, 0, 0, 0, 0]),
            (AARCH64, 435, [0, 0, 0, 0, 0, 0]),
            (AARCH64, 198, [0, 0, 0, 0, 0, 0]),
            (AARCH64, 117, [0, 0, 0, 0, 0, 0]),
            (AARCH64, 40, [0, 0, 0, 0, 0, 0]),
            (X86_64, 64, [1, 0, 16, 0, 0, 0]),
        ];
        assert_eq!(
            verdicts(ON_AARCH64, &cases),
            [
                ALLOW, KILL, KILL, KILL, ALLOW, KILL, KILL, ALLOW, KILL, KILL, KILL, KILL, KILL,
                KILL, KILL, KILL, KILL
            ]
        );
    }

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

    /// Where the kernel lists its active security modules. The file is on
    /// securityfs, which a kernel can be built without and a container is
    /// usually not shown.
    const MODULE_LIST: &str = "/sys/kernel/security/lsm";

    /// The text of a file the kernel publishes, or `None` where the kernel
    /// publishes no file by that name. Any other failure to read it is an
    /// error, never taken for a missing file.
    fn published(path: &str) -> Result<Option<String>, ErrorKind> {
        #[expect(
            clippy::disallowed_methods,
            reason = "the tests read what the kernel publishes, its list of security modules first, by fixed paths, to know what to expect of Landlock (SEC-MED-024)"
        )]
        let read = std::fs::read_to_string(path);
        match read {
            Ok(text) => Ok(Some(text)),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.kind()),
        }
    }

    /// Whether a security module is active on the running kernel. Where
    /// the kernel publishes the list of its modules, the list decides, and
    /// a module is one whole name in it. Where it publishes none,
    /// `answered` decides: what the kernel said when it was asked about
    /// that module directly.
    fn active(list: Option<&str>, module: &str, answered: bool) -> bool {
        list.map_or(answered, |names| {
            names.trim().split(',').any(|name| name == module)
        })
    }

    /// Whether the running kernel answers the Landlock version query with
    /// a version. The query is `landlock_create_ruleset` with only the
    /// version flag: it creates no ruleset and restricts nothing. A kernel
    /// built without Landlock answers `ENOSYS`, and a kernel that has
    /// Landlock switched off answers `EOPNOTSUPP`.
    ///
    /// Making the call needs `unsafe`, which this crate allows in one
    /// place only (ADR 13), so it is made through the `landlock` crate: a
    /// builder that is given nothing to restrict and told to leave
    /// `no_new_privs` alone asks for the version and does nothing else.
    /// None of the code under test is involved.
    fn landlock_answers() -> bool {
        let answer = landlock::RestrictSelf::default()
            .no_new_privs(false)
            .apply()
            .unwrap();
        landlock::ABI::from(answer.landlock) != landlock::ABI::Unsupported
    }

    /// Whether the running kernel has Landlock active: read from the
    /// kernel's own list of security modules, or asked of the kernel
    /// directly where it publishes no list, and never from the code under
    /// test.
    fn kernel_has_landlock() -> bool {
        let list = published(MODULE_LIST).unwrap();
        active(list.as_deref(), "landlock", landlock_answers())
    }

    /// Where the kernel publishes the list of its security modules, the
    /// list decides and the kernel's direct answer changes nothing; a
    /// module is a whole name in the list. On a kernel that publishes
    /// none, as one without securityfs, the direct answer decides.
    #[test]
    fn the_module_list_decides_and_without_one_the_kernels_direct_answer_does() {
        let listed = Some("lockdown,capability,landlock,yama,apparmor\n");
        assert_eq!(
            [
                active(listed, "landlock", false),
                active(listed, "yama", false),
                active(listed, "lockdown", false),
                active(listed, "apparmor", false),
                active(listed, "selinux", true),
                active(listed, "lock", true),
                active(listed, "", true),
            ],
            [true, true, true, true, false, false, false]
        );
        // One name, as the kernel writes it, with no line end.
        assert!(active(Some("landlock"), "landlock", false));
        assert!(!active(Some("capability,yama"), "landlock", true));
        assert!(!active(Some(""), "landlock", true));
        assert_eq!(
            [
                active(None, "landlock", true),
                active(None, "landlock", false),
            ],
            [true, false]
        );
    }

    /// A kernel without securityfs has no file where the list would be.
    /// Reading it is "no list", not a failure, and that is what sends the
    /// tests to the kernel's direct answer. A file that is there is read
    /// whole. Any other failure stays an error, so a list that exists and
    /// cannot be read is never taken for a kernel without one.
    #[test]
    fn a_file_the_kernel_does_not_publish_is_no_list_and_no_other_failure_is() {
        assert_eq!(
            published("/proc/sys/kernel/ostype"),
            Ok(Some("Linux\n".to_owned()))
        );
        assert_eq!(published("/sys/kernel/security/no-such-list"), Ok(None));
        assert_eq!(
            published("/proc/self/status/lsm"),
            Err(ErrorKind::NotADirectory)
        );
    }

    /// Where the kernel publishes its module list, the list and the
    /// version query say the same of Landlock, which is what lets the
    /// query stand in for the list on a kernel that publishes none. On
    /// such a kernel the two are one answer and this proves nothing.
    #[test]
    fn the_version_query_and_the_module_list_say_the_same_of_landlock() {
        assert_eq!(landlock_answers(), kernel_has_landlock());
    }

    /// On a kernel without Landlock the version query says so: the three
    /// Landlock calls answer `ENOSYS` on this thread, as on a kernel built
    /// without them.
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    #[test]
    fn the_version_query_says_no_on_a_kernel_without_landlock() {
        let answered = std::thread::spawn(|| {
            take_away(&[444, 445, 446]);
            landlock_answers()
        })
        .join()
        .unwrap();
        assert!(!answered);
    }

    /// Asking leaves the thread's `no_new_privs` flag as it was. A process
    /// started afterwards inherits the flag, so a query that set it would
    /// hide a worker that did not set its own.
    #[test]
    fn asking_for_the_landlock_version_leaves_no_new_privs_as_it_was() {
        let before = rustix::thread::no_new_privs().unwrap();
        let _answer = landlock_answers();
        assert_eq!(rustix::thread::no_new_privs().unwrap(), before);
    }

    /// Whether the running kernel's Landlock has every rule the worker
    /// asks for. The newest of them, the signal and abstract-socket
    /// scopes, came with Linux 6.12. Read from the kernel's release and
    /// not from the code under test.
    fn kernel_has_every_landlock_rule() -> bool {
        #[expect(
            clippy::disallowed_methods,
            reason = "the test reads the kernel's release, a fixed path, to know how much of Landlock to expect (SEC-MED-024)"
        )]
        let release = std::fs::read_to_string("/proc/sys/kernel/osrelease").unwrap();
        let mut numbers = release
            .split(|character: char| !character.is_ascii_digit())
            .map(|number| number.parse::<u32>().unwrap());
        (numbers.next().unwrap(), numbers.next().unwrap()) >= (6, 12)
    }

    /// Landlock's three answers about a ruleset are reported as they are:
    /// one enforced in part is not passed off as enforced.
    ///
    /// Verifies: SEC-MED-024
    #[test]
    fn a_ruleset_enforced_in_part_is_reported_as_in_part() {
        assert_eq!(
            [
                coverage(&RulesetStatus::FullyEnforced),
                coverage(&RulesetStatus::PartiallyEnforced),
                coverage(&RulesetStatus::NotEnforced),
            ],
            [Landlock::Full, Landlock::Partial, Landlock::Missing]
        );
    }

    /// Landlock binds the thread that enforces it, so a thread of its own
    /// can try it and leave the test process free. Where the kernel has
    /// Landlock every path is refused, and the ruleset is reported as
    /// enforced whole or in part as the kernel's release says; where it
    /// has none the control is reported missing and the path still opens.
    ///
    /// Verifies: SEC-MED-022, SEC-MED-024
    #[test]
    fn landlock_refuses_every_path_on_the_thread_that_enforced_it() {
        let offered = kernel_has_landlock();
        let whole = kernel_has_every_landlock_rule();
        let (enforced, listing) = std::thread::spawn(|| (Linux.landlock(), list_by_path()))
            .join()
            .unwrap();
        assert_eq!(
            enforced,
            [Landlock::Missing, Landlock::Partial, Landlock::Full]
                [usize::from(offered) * (1 + usize::from(whole))]
        );
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
        assert_eq!(enforced, Landlock::Missing);
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

    /// What a thread's `/proc` status says of seccomp.
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Seccomp {
        /// The `Seccomp:` line: 0 without a filter, 2 with one or more.
        mode: u32,
        /// The `Seccomp_filters:` line: how many filters bind the thread.
        /// Linux 5.9 added the line; on an older kernel this is `None`.
        filters: Option<u32>,
    }

    /// Reads what the text of a `/proc` status file says of seccomp.
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    fn seccomp_in(status: &str) -> Seccomp {
        let number = |label: &str| {
            status
                .lines()
                .find_map(|line| line.strip_prefix(label))
                .map(|value| value.trim().parse::<u32>().unwrap())
        };
        Seccomp {
            mode: number("Seccomp:").unwrap(),
            filters: number("Seccomp_filters:"),
        }
    }

    /// What the kernel says of seccomp for one thread of this process.
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    fn seccomp_of(tid: i32) -> Seccomp {
        #[expect(
            clippy::disallowed_methods,
            reason = "the test reads a thread's own /proc status to see the filters the kernel holds for it (SEC-MED-022)"
        )]
        let status = std::fs::read_to_string(format!("/proc/self/task/{tid}/status")).unwrap();
        seccomp_in(&status)
    }

    /// How a thread's seccomp state changed: its mode before, its mode
    /// after, and how many filters it gained where the kernel counts them.
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    type Step = (u32, u32, Option<i64>);

    /// The change from one state of a thread to a later one.
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    fn change(before: Seccomp, after: Seccomp) -> Step {
        let gained = before
            .filters
            .zip(after.filters)
            .map(|(earlier, later)| i64::from(later) - i64::from(earlier));
        (before.mode, after.mode, gained)
    }

    /// The status of a process in a container, of one outside any, and of
    /// one on a kernel older than Linux 5.9, which does not count filters.
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    #[test]
    fn a_status_gives_the_seccomp_mode_and_the_filter_count_where_there_is_one() {
        assert_eq!(
            seccomp_in(
                "Name:\tsh\nNoNewPrivs:\t1\nSeccomp:\t2\nSeccomp_filters:\t1\n\
                 Speculation_Store_Bypass:\tthread vulnerable\n"
            ),
            Seccomp {
                mode: 2,
                filters: Some(1)
            }
        );
        assert_eq!(
            seccomp_in("NoNewPrivs:\t0\nSeccomp:\t0\nSeccomp_filters:\t0\n"),
            Seccomp {
                mode: 0,
                filters: Some(0)
            }
        );
        assert_eq!(
            seccomp_in("Seccomp:\t2\nSeccomp_filters:\t12\n"),
            Seccomp {
                mode: 2,
                filters: Some(12)
            }
        );
        assert_eq!(
            seccomp_in("NoNewPrivs:\t0\nSeccomp:\t2\nSpeculation_Store_Bypass:\tvulnerable\n"),
            Seccomp {
                mode: 2,
                filters: None
            }
        );
    }

    /// A change holds the two modes, and the filters gained only where the
    /// kernel counted them both times.
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    #[test]
    fn a_change_is_the_two_modes_and_the_filters_gained_where_they_are_counted() {
        let state = |mode, filters| Seccomp { mode, filters };
        assert_eq!(
            [
                change(state(0, Some(0)), state(2, Some(1))),
                change(state(2, Some(1)), state(2, Some(2))),
                change(state(2, Some(3)), state(2, Some(3))),
                change(state(2, Some(2)), state(2, Some(1))),
                change(state(0, None), state(2, None)),
                change(state(2, Some(1)), state(2, None)),
            ],
            [
                (0, 2, Some(1)),
                (2, 2, Some(1)),
                (2, 2, Some(0)),
                (2, 2, Some(-1)),
                (0, 2, None),
                (2, 2, None),
            ]
        );
    }

    /// Starts a thread that installs the worker's filter, and returns what
    /// that thread answered, how the calling thread started, and what the
    /// kernel showed: the change in the thread that asked, then the change
    /// in the calling thread, which asked for nothing.
    ///
    /// The helper thread reads its own state before it asks. Afterwards it
    /// only spins, which needs no system call, until the calling thread
    /// has read its state again.
    ///
    /// Neither thread can wait for ever. The calling thread gives the
    /// helper half a minute to say how it did, and lets it go when it
    /// leaves the scope, by a panic as much as by its last line, so a step
    /// that fails on either side fails the test instead of hanging it.
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    fn watch_a_new_thread_install_the_filter() -> (Option<u8>, u8, Seccomp, [Step; 2]) {
        use std::sync::OnceLock;
        use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU8, Ordering};
        use std::time::Duration;

        /// Sets the flag the helper waits for when it is dropped.
        struct Release<'a>(&'a AtomicBool);

        impl Drop for Release<'_> {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }

        /// How many times this thread sleeps a millisecond and looks for
        /// the helper's outcome before it gives up.
        const LOOKS: usize = 30_000;

        let own = rustix::thread::gettid().as_raw_nonzero().get();
        let started = seccomp_of(own);
        let tid = AtomicI32::new(0);
        let asked_from = OnceLock::new();
        // 0: not done yet. 1: no filter. 2: installed.
        let outcome = AtomicU8::new(0);
        let seen = AtomicBool::new(false);
        // Both waits run their step at least once, so every line here runs
        // however the two threads are scheduled.
        let (finished, asked_into) = std::thread::scope(|scope| {
            scope.spawn(|| {
                let helper = rustix::thread::gettid().as_raw_nonzero().get();
                asked_from.set(seccomp_of(helper)).unwrap();
                tid.store(helper, Ordering::SeqCst);
                Linux.no_new_privs().unwrap();
                let installed = Linux.seccomp();
                outcome.store(u8::from(installed).saturating_add(1), Ordering::SeqCst);
                let _spun = std::iter::repeat_with(|| {
                    std::hint::spin_loop();
                    seen.load(Ordering::SeqCst)
                })
                .any(|done| done);
            });
            let release = Release(&seen);
            let finished = std::iter::repeat_with(|| {
                std::thread::sleep(Duration::from_millis(1));
                outcome.load(Ordering::SeqCst)
            })
            .take(LOOKS)
            .find(|&state| state != 0);
            let asked_into = seccomp_of(tid.load(Ordering::SeqCst));
            drop(release);
            (finished, asked_into)
        });
        let asked_from = *asked_from.get().unwrap();
        (
            finished,
            outcome.load(Ordering::SeqCst),
            started,
            [
                change(asked_from, asked_into),
                change(started, seccomp_of(own)),
            ],
        )
    }

    /// A seccomp filter binds the thread that installs it, and no other.
    /// The kernel shows it as one more filter on the thread that asked and
    /// none more on this one, and as mode 2 on the thread that asked. No
    /// start is assumed: what this thread shows before decides which row
    /// below must hold, and where nothing could show the filter there is
    /// no row, so the test fails instead of passing on what it cannot see.
    ///
    /// Verifies: SEC-MED-022
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    #[test]
    fn seccomp_installs_the_filter_on_the_thread_that_asked() {
        let (finished, outcome, started, changes) = watch_a_new_thread_install_the_filter();
        assert_eq!(finished, Some(2));
        assert_eq!(outcome, 2);
        // The change in the thread that asked, then in this one, each as
        // the mode before, the mode after and the filters gained.
        let expected: [[Option<[Step; 2]>; 2]; 2] = [
            [
                // No filter at the start, on a kernel older than Linux 5.9,
                // which does not count filters: the mode alone shows it.
                Some([(0, 2, None), (0, 0, None)]),
                // No filter at the start, as on a hosted runner.
                Some([(0, 2, Some(1)), (0, 0, Some(0))]),
            ],
            [
                // A filter at the start and no count: one more filter
                // would not show, so nothing is accepted.
                None,
                // A filter at the start, as in a container with a seccomp
                // profile: the mode is 2 before and after, and only the
                // count shows the new filter.
                Some([(2, 2, Some(1)), (2, 2, Some(0))]),
            ],
        ];
        assert_eq!(
            Some(changes),
            expected[usize::from(started.mode != 0)][usize::from(started.filters.is_some())],
            "this thread started as {started:?}"
        );
    }

    /// The same on a thread that is filtered before it asks, as every
    /// thread is in a container with a seccomp profile. This thread first
    /// takes the Landlock calls away from itself, which the filter's
    /// installation does not need, and the thread it then starts inherits
    /// that. The mode is 2 before and after, so only the count can show
    /// the worker's filter: on a kernel older than Linux 5.9, which has no
    /// count, this test fails.
    ///
    /// Verifies: SEC-MED-022
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    #[test]
    fn seccomp_adds_its_filter_to_a_thread_that_is_filtered_already() {
        let (finished, outcome, _, changes) = std::thread::spawn(|| {
            take_away(&[444, 445, 446]);
            watch_a_new_thread_install_the_filter()
        })
        .join()
        .unwrap();
        assert_eq!(finished, Some(2));
        assert_eq!(outcome, 2);
        assert_eq!(changes, [(2, 2, Some(1)), (2, 2, Some(0))]);
    }

    /// The `prctl` call's number on this architecture.
    #[cfg(target_arch = "x86_64")]
    const PRCTL_CALL: i64 = 157;
    #[cfg(target_arch = "aarch64")]
    const PRCTL_CALL: i64 = 167;

    /// Makes the kernel answer `EINVAL`, 22, to `prctl(PR_SET_NO_NEW_PRIVS)`,
    /// option 38, on the calling thread from now on, as a kernel older than
    /// Linux 3.5 does. Every other `prctl` still goes through.
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    fn refuse_no_new_privs() {
        use seccompiler::{BpfProgram, SeccompAction, SeccompFilter};

        let (arch, _) = super::NATIVE.unwrap();
        let option =
            SeccompCondition::new(0, SeccompCmpArgLen::Dword, SeccompCmpOp::Eq, 38).unwrap();
        let rules = [(PRCTL_CALL, vec![SeccompRule::new(vec![option]).unwrap()])].into();
        let filter =
            SeccompFilter::new(rules, SeccompAction::Allow, SeccompAction::Errno(22), arch)
                .unwrap();
        seccompiler::apply_filter(&BpfProgram::try_from(filter).unwrap()).unwrap();
    }

    /// The `no_new_privs` step asks the kernel, and a refusal is not taken
    /// for success: on a thread where the kernel refuses that one `prctl`
    /// option, the step fails with the kernel's error number, which is
    /// what stops confinement. The bit cannot be cleared, so in a process
    /// that starts with it, as every process does in a container run with
    /// `no-new-privileges`, reading the bit afterwards cannot tell a step
    /// that asked from one that did not.
    ///
    /// Verifies: SEC-MED-022
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    #[test]
    fn a_kernel_that_refuses_no_new_privs_is_reported_with_its_error() {
        let refused = std::thread::spawn(|| {
            refuse_no_new_privs();
            Linux.no_new_privs().map_err(|error| error.raw_os_error())
        })
        .join()
        .unwrap();
        assert_eq!(refused, Err(Some(22)));
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
