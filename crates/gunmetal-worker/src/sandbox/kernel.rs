//! The kernel calls confinement is made of, behind a trait.
//!
//! [`confine_with`](super::confine::confine_with) holds the order and the
//! decisions and is tested against a recording kernel. [`Linux`] is the
//! real one: each method is one call with no decision in it.
//!
//! No method here needs `unsafe`, and that sets one limit. A process
//! cannot close a descriptor it has only a number for without `unsafe`,
//! so a worker cannot close what it inherited by mistake. [`Linux`] lists
//! such descriptors instead and the seccomp filter refuses every call on
//! them, which takes them out of the worker's reach.

use super::limits::Limit;
use super::syscalls::{ALLOWLIST, Allowed, Check, Guard, Test, checks};
use landlock::{
    ABI, Access, AccessFs, AccessNet, Ruleset, RulesetAttr, RulesetCreated, RulesetCreatedAttr,
    RulesetStatus, Scope, path_beneath_rules,
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
    /// Installs the seccomp allowlist, which also refuses every call on
    /// `strays`. `false` when the architecture or the kernel has no
    /// seccomp filter.
    fn seccomp(&mut self, strays: &[u32]) -> bool;
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

/// The numeric names in a directory of `/proc/self`, and the number of
/// the descriptor that was opened to read them.
fn numbers(directory: &str) -> io::Result<(i32, Vec<u32>)> {
    #[expect(
        clippy::disallowed_methods,
        reason = "the worker reads its own /proc/self entries, fixed paths, before it gives up the filesystem (SEC-MED-022)"
    )]
    let opened = rustix::fs::openat(
        CWD,
        directory,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    );
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
        Test::IsNot => SeccompCmpOp::Ne,
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

/// The directory named by `LLVM_PROFILE_FILE`, ignoring llvm-cov's `%`
/// specifiers. `None` when the variable is missing or is not a path.
fn profile_dir_from(file: &str) -> Option<String> {
    file.split('%')
        .next()
        .and_then(|prefix| prefix.rsplit_once('/'))
        .map(|(dir, _)| dir.to_owned())
}

/// The directory llvm-cov asked this process to write a profile into.
fn profile_dir() -> Option<String> {
    std::env::var("LLVM_PROFILE_FILE")
        .ok()
        .as_deref()
        .and_then(profile_dir_from)
}

/// Extra calls a coverage runtime needs to flush a profile after
/// confinement. Production never sets `LLVM_PROFILE_FILE`, so the
/// allowlist stays the documented minimum (SEC-MED-022).
fn extra_profile_calls_from(has_profile: bool) -> Vec<Allowed> {
    if !has_profile {
        return Vec::new();
    }
    extra_profile_allowlist().to_vec()
}

/// The extra calls, when a profile file is in the environment.
const fn extra_profile_allowlist() -> [Allowed; 13] {
    [
        Allowed {
            name: "fcntl",
            x86_64: 72,
            aarch64: 25,
            guard: Guard::Always,
        },
        Allowed {
            name: "fstat",
            x86_64: 5,
            aarch64: 80,
            guard: Guard::Always,
        },
        Allowed {
            name: "ftruncate",
            x86_64: 77,
            aarch64: 46,
            guard: Guard::Always,
        },
        Allowed {
            name: "getdents64",
            x86_64: 217,
            aarch64: 61,
            guard: Guard::Always,
        },
        Allowed {
            name: "lseek",
            x86_64: 8,
            aarch64: 62,
            guard: Guard::Always,
        },
        Allowed {
            name: "newfstatat",
            x86_64: 262,
            aarch64: 79,
            guard: Guard::Always,
        },
        Allowed {
            name: "openat",
            x86_64: 257,
            aarch64: 56,
            guard: Guard::Always,
        },
        Allowed {
            name: "prctl",
            x86_64: 157,
            aarch64: 167,
            guard: Guard::Always,
        },
        Allowed {
            name: "pread64",
            x86_64: 17,
            aarch64: 67,
            guard: Guard::Always,
        },
        Allowed {
            name: "pwrite64",
            x86_64: 18,
            aarch64: 68,
            guard: Guard::Always,
        },
        Allowed {
            name: "readlinkat",
            x86_64: 267,
            aarch64: 78,
            guard: Guard::Always,
        },
        Allowed {
            name: "renameat",
            x86_64: 264,
            aarch64: 38,
            guard: Guard::Always,
        },
        Allowed {
            name: "statx",
            x86_64: 332,
            aarch64: 291,
            guard: Guard::Always,
        },
    ]
}

fn extra_profile_calls() -> Vec<Allowed> {
    extra_profile_calls_from(std::env::var_os("LLVM_PROFILE_FILE").is_some())
}

/// Builds the Landlock ruleset: no filesystem access, except the
/// coverage profile directory when llvm-cov named one.
fn landlock_ruleset(dir: Option<&str>) -> Result<RulesetCreated, landlock::RulesetError> {
    let abi = ABI::V6;
    let created = Ruleset::default()
        .handle_access(AccessFs::from_all(abi))
        .and_then(|ruleset| ruleset.handle_access(AccessNet::from_all(abi)))
        .and_then(|ruleset| ruleset.scope(Scope::from_all(abi)))
        .and_then(Ruleset::create);
    match dir {
        Some(dir) => created.and_then(|ruleset| {
            ruleset.add_rules(path_beneath_rules(
                [dir],
                AccessFs::ReadFile
                    | AccessFs::WriteFile
                    | AccessFs::ReadDir
                    | AccessFs::MakeReg
                    | AccessFs::RemoveFile
                    | AccessFs::Truncate,
            ))
        }),
        None => created,
    }
}

/// The compiled filter: every call on the allowlist is allowed when its
/// checks hold, and anything else kills the process.
fn program(strays: &[u32], pid: u32) -> Option<BpfProgram> {
    program_with(strays, pid, extra_profile_calls())
}

fn program_with(
    strays: &[u32],
    pid: u32,
    extra: impl IntoIterator<Item = Allowed>,
) -> Option<BpfProgram> {
    NATIVE.and_then(|(arch, number)| {
        ALLOWLIST
            .iter()
            .copied()
            .chain(extra)
            .map(|allowed| {
                checks(allowed.guard, strays, pid)
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
        landlock_ruleset(profile_dir().as_deref())
            .and_then(RulesetCreated::restrict_self)
            .is_ok_and(|status| status.ruleset != RulesetStatus::NotEnforced)
    }

    fn seccomp(&mut self, strays: &[u32]) -> bool {
        let pid = rustix::process::getpid()
            .as_raw_nonzero()
            .get()
            .unsigned_abs();
        program(strays, pid).is_some_and(|filter| seccompiler::apply_filter(&filter).is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::{Kernel, Linux, numbers, program};
    use crate::sandbox::syscalls::ALLOWLIST;

    #[test]
    fn the_running_process_lists_its_threads_and_descriptors() {
        let mut linux = Linux;
        assert!(linux.threads().unwrap() >= 1);
        let descriptors = linux.descriptors().unwrap();
        assert!(descriptors.contains(&0), "{descriptors:?}");
        assert!(descriptors.contains(&1), "{descriptors:?}");
        assert!(descriptors.contains(&2), "{descriptors:?}");
    }

    #[test]
    fn a_missing_or_non_directory_proc_entry_is_an_error() {
        assert!(numbers("/proc/self/does-not-exist").is_err());
        assert!(numbers("/proc/self/status").is_err());
    }

    #[test]
    fn numeric_proc_names_are_kept_and_the_listing_descriptor_is_dropped() {
        let (own, tasks) = numbers("/proc/self/task").unwrap();
        assert!(own >= 0);
        assert!(!tasks.contains(&u32::try_from(own).unwrap()));
        assert!(!tasks.is_empty());
    }

    #[test]
    fn the_seccomp_filter_compiles_for_this_architecture() {
        assert!(program(&[], 1).is_some());
        assert!(program(&[5, 9], 4321).is_some());
        assert!(super::program_with(&[], 1, super::extra_profile_allowlist()).is_some());
        assert!(super::program_with(&[], 1, Vec::new()).is_some());
    }

    #[test]
    fn the_profile_directory_is_the_path_before_llvm_specifiers() {
        assert_eq!(
            super::profile_dir_from("/work/target/wp-%p-%16m.profraw").as_deref(),
            Some("/work/target")
        );
        assert_eq!(
            super::profile_dir_from("/abs/p.profraw").as_deref(),
            Some("/abs")
        );
        assert_eq!(super::profile_dir_from("p.profraw"), None);
        assert_eq!(super::profile_dir_from(""), None);
        let _ = super::profile_dir();
    }

    #[test]
    fn extra_profile_calls_are_empty_without_a_profile_and_listed_with_one() {
        assert!(super::extra_profile_calls_from(false).is_empty());
        assert_eq!(super::extra_profile_calls_from(true).len(), 13);
        assert_eq!(
            super::extra_profile_calls(),
            super::extra_profile_calls_from(std::env::var_os("LLVM_PROFILE_FILE").is_some())
        );
        let extras = super::extra_profile_allowlist();
        let names: Vec<&str> = extras.iter().map(|row| row.name).collect();
        assert_eq!(
            names,
            [
                "fcntl",
                "fstat",
                "ftruncate",
                "getdents64",
                "lseek",
                "newfstatat",
                "openat",
                "prctl",
                "pread64",
                "pwrite64",
                "readlinkat",
                "renameat",
                "statx"
            ]
        );
        for row in extras {
            assert!(ALLOWLIST.iter().all(|allowed| allowed.name != row.name
                && allowed.x86_64 != row.x86_64
                && allowed.aarch64 != row.aarch64));
        }
    }

    #[test]
    fn a_landlock_ruleset_builds_with_and_without_a_profile_directory() {
        assert!(super::landlock_ruleset(None).is_ok());
        assert!(super::landlock_ruleset(Some("/tmp")).is_ok());
        assert!(super::landlock_ruleset(Some("/does-not-exist-gunmetal-wp045")).is_ok());
    }

    #[test]
    fn the_running_process_can_clear_dumpable_and_set_no_new_privs() {
        let mut linux = Linux;
        linux.undumpable().unwrap();
        linux.no_new_privs().unwrap();
    }

    #[test]
    fn the_running_process_can_set_a_core_file_limit() {
        use crate::sandbox::limits::Limit;
        use rustix::process::Resource;

        let mut linux = Linux;
        linux
            .limit(Limit {
                resource: Resource::Core,
                soft: 0,
                hard: 0,
            })
            .unwrap();
    }

    #[test]
    fn only_a_cleared_dumpable_flag_is_accepted() {
        use rustix::process::DumpableBehavior;

        assert!(super::require_not_dumpable(DumpableBehavior::NotDumpable).is_ok());
        assert_eq!(
            super::require_not_dumpable(DumpableBehavior::Dumpable)
                .unwrap_err()
                .to_string(),
            "dumpable is Dumpable"
        );
        assert_eq!(
            super::require_not_dumpable(DumpableBehavior::DumpableReadableOnlyByRoot)
                .unwrap_err()
                .to_string(),
            "dumpable is DumpableReadableOnlyByRoot"
        );
    }
}
