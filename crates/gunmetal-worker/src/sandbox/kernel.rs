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
//! extra remains: listing them for seccomp is not closing them.

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
    /// Installs the seccomp allowlist. `strays` is the descriptors the
    /// filter must also refuse; production confinement passes none,
    /// because a worker with extra descriptors is not confined.
    /// `false` when the architecture or the kernel has no seccomp filter.
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
fn program_for(native: Option<NativeArch>, strays: &[u32], pid: u32) -> Option<BpfProgram> {
    native.and_then(|(arch, number)| {
        ALLOWLIST
            .iter()
            .copied()
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

/// The compiled filter for this architecture.
fn program(strays: &[u32], pid: u32) -> Option<BpfProgram> {
    program_for(NATIVE, strays, pid)
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

    /// Verifies: SEC-MED-024
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    #[test]
    fn the_seccomp_filter_compiles_for_this_architecture() {
        assert!(program(&[], 1).is_some());
        assert!(program(&[5, 9], 4321).is_some());
    }

    /// Verifies: SEC-MED-024
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    #[test]
    fn the_seccomp_filter_is_absent_on_this_architecture() {
        assert!(program(&[], 1).is_none());
        assert!(program(&[5, 9], 4321).is_none());
    }

    /// Verifies: SEC-MED-024
    #[test]
    fn a_missing_native_arch_reports_reduced_and_names_seccomp() {
        use crate::sandbox::tier::{Enforced, Tier};

        assert!(super::program_for(None, &[], 1).is_none());
        assert!(super::program_for(None, &[5, 9], 4321).is_none());
        let report = Enforced {
            process: true,
            limits: true,
            no_new_privs: true,
            seccomp: false,
            landlock: true,
            namespaces: true,
        }
        .report();
        assert_eq!(report.tier, Tier::Reduced);
        let notice = report.notice.as_deref().expect("reduced notice");
        assert!(
            notice.contains("system call filtering (seccomp)"),
            "{notice}"
        );
        assert!(
            !notice.contains("Landlock") && !notice.contains("namespaces"),
            "{notice}"
        );
    }

    #[test]
    fn a_landlock_ruleset_grants_no_filesystem_path() {
        assert!(super::landlock_ruleset().is_ok());
    }

    /// Directory of a `LLVM_PROFILE_FILE` value, or `/tmp` when the value
    /// has no directory so the probe always has one path to mmap.
    fn child_coverage_profile(llvm_profile_file: Option<&str>, pid: u32) -> String {
        let dir = llvm_profile_file
            .and_then(|file| {
                file.split('%')
                    .next()
                    .and_then(|prefix| prefix.rsplit_once('/'))
                    .map(|(dir, _)| dir)
            })
            .unwrap_or("/tmp");
        format!("{dir}/wp045-landlock-{pid}.profraw")
    }

    #[test]
    fn child_coverage_profile_uses_the_file_directory_or_tmp() {
        assert_eq!(
            child_coverage_profile(Some("/cov/out.profraw%m"), 7),
            "/cov/wp045-landlock-7.profraw"
        );
        assert_eq!(
            child_coverage_profile(Some("/cov/out.profraw"), 7),
            "/cov/wp045-landlock-7.profraw"
        );
        assert_eq!(
            child_coverage_profile(Some("nodir.profraw"), 1),
            "/tmp/wp045-landlock-1.profraw"
        );
        assert_eq!(
            child_coverage_profile(None, 1),
            "/tmp/wp045-landlock-1.profraw"
        );
    }

    /// A `landlock` method that returns `true` without `restrict_self`
    /// still lets a path open. The probe is a child so this process is
    /// not Landlock'd.
    ///
    /// Verifies: SEC-MED-022
    #[test]
    fn landlock_denies_a_path_when_it_reports_enforced() {
        if std::env::var_os("GUNMETAL_PROBE_LANDLOCK").is_some() {
            let mut linux = Linux;
            linux.no_new_privs().unwrap();
            assert!(linux.landlock(), "Landlock must hold on this kernel");
            #[expect(
                clippy::disallowed_methods,
                reason = "the probe opens a path to observe Landlock, not to read a file (SEC-MED-022)"
            )]
            let error =
                std::fs::File::open("/etc/hostname").expect_err("Landlock must deny the path");
            assert_eq!(
                error.kind(),
                std::io::ErrorKind::PermissionDenied,
                "{error}"
            );
            std::process::exit(0);
        }
        #[expect(
            clippy::disallowed_methods,
            reason = "the sandbox launcher is the one door that starts a process (SEC-MED-063); this probe re-enters the unit-test binary"
        )]
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command.env("GUNMETAL_PROBE_LANDLOCK", "1").args([
            "--exact",
            "sandbox::kernel::tests::landlock_denies_a_path_when_it_reports_enforced",
        ]);
        // `%c` mmaps the profile before Landlock. rustc only defines the
        // bias symbols with `-C llvm-args=-runtime-counter-relocation`.
        let profile = child_coverage_profile(
            std::env::var("LLVM_PROFILE_FILE").ok().as_deref(),
            std::process::id(),
        );
        #[expect(
            clippy::disallowed_methods,
            reason = "create the coverage profile the probe child mmaps before Landlock (SEC-MED-022)"
        )]
        let _ = std::fs::File::create(&profile);
        command.env("LLVM_PROFILE_FILE", format!("{profile}%c"));
        let status = command.status().unwrap();
        assert!(status.success(), "{status:?}");
    }

    #[test]
    fn proc_directory_flags_are_read_only_directory_and_close_on_exec() {
        use rustix::fs::OFlags;

        assert!(super::PROC_DIR_FLAGS.contains(OFlags::RDONLY));
        assert!(super::PROC_DIR_FLAGS.contains(OFlags::DIRECTORY));
        assert!(super::PROC_DIR_FLAGS.contains(OFlags::CLOEXEC));
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
