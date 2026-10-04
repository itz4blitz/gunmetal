//! Integration tests of the sandbox launcher and a confined worker.
//!
//! This file is its own executable (`harness = false`) so the launcher can
//! start it again as the worker: `/proc/self/exe` with the hidden worker
//! arguments enters the child path. No extra binary ships, and the hostile
//! hooks exist only in this test executable.
//!
//! Every test runs once against the kernel as it is, and again against a
//! kernel that has no Landlock, no seccomp filter, and neither. Those
//! kernels are made here: the executable starts itself again with a word
//! from [`EMULATED`], and that process first installs a filter of its own
//! under which the kernel answers the Landlock calls, the `seccomp` call,
//! or both with `ENOSYS`, which is the answer of a kernel built without
//! them. Workers inherit that filter, so their confinement meets a kernel
//! that really refuses, and the reduced tier is proven against it rather
//! than skipped (SEC-MED-022, SEC-MED-024).
//!
//! What each kernel offers is decided here, never by the code under test:
//! from `/sys/kernel/security/lsm`, the kernel's release, the build's
//! architecture, and which calls this executable itself took away
//! ([`Host`]). Every expected report and outcome is written out per case.
#![expect(
    clippy::disallowed_methods,
    reason = "the hostile worker tries each forbidden action by path, socket and Command, and the test reads /proc to see what the kernel holds for a worker (SEC-MED-022, SEC-TM-044)"
)]

use gunmetal_worker::sandbox::{
    Cause, Child, Exit, Inherited, Job, Profile, Program, Tier, TierReport, TypedArgs,
    answer_self_test, confine, launch, self_test,
};
use rustix::io::{Errno, FdFlags, fcntl_setfd};
use rustix::process::{PTracer, Pid, getppid, getuid, set_ptracer, test_kill_process};
use std::fs;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::process::{self, Command};
use std::thread;

/// Report what the launcher handed over, confine, then wait.
const WAIT: u8 = b'w';
/// End without confining.
const QUIT: u8 = b'q';
/// Confine, then open a path.
const OPEN: u8 = b'o';
/// Confine, then connect a TCP socket.
const NETWORK: u8 = b'n';
/// Confine, then execute a program.
const EXEC: u8 = b'x';
/// Confine, then allocate until the memory limit ends the process.
const MEMORY: u8 = b'm';
/// Confine, then start a thread, which is a `clone`.
const CLONE: u8 = b'c';
/// Confine, then ask for any process to be allowed to trace this one.
const TRACE: u8 = b't';
/// Confine, then signal the parent.
const SIGNAL: u8 = b's';

/// The words that start this executable again against a kernel with
/// something taken away: Landlock, the seccomp filter, or both.
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const EMULATED: [&str; 3] = [
    "--kernel-without-landlock",
    "--kernel-without-seccomp",
    "--kernel-without-landlock-or-seccomp",
];
/// An architecture seccompiler cannot write a filter for cannot take a
/// call away either, so only the kernel as it is runs there.
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
const EMULATED: [&str; 0] = [];

/// One test: its name and its body, which is told what the kernel offers.
type Test = (&'static str, fn(&Host));

const TESTS: [Test; 11] = [
    (
        "self_test_reports_the_tier_the_kernel_allows_and_names_what_is_missing",
        self_test_reports_the_tier_the_kernel_allows_and_names_what_is_missing,
    ),
    (
        "a_worker_gets_the_socket_three_fixed_words_and_nothing_else",
        a_worker_gets_the_socket_three_fixed_words_and_nothing_else,
    ),
    (
        "a_confined_worker_is_limited_single_threaded_undumpable_and_filtered",
        a_confined_worker_is_limited_single_threaded_undumpable_and_filtered,
    ),
    (
        "opening_a_path_is_killed_or_refused",
        opening_a_path_is_killed_or_refused,
    ),
    (
        "connecting_a_network_socket_is_killed_or_refused",
        connecting_a_network_socket_is_killed_or_refused,
    ),
    (
        "executing_a_program_is_killed_or_refused",
        executing_a_program_is_killed_or_refused,
    ),
    (
        "starting_a_thread_is_killed_or_refused",
        starting_a_thread_is_killed_or_refused,
    ),
    (
        "asking_to_be_traced_is_killed_or_left_to_the_dumpable_flag",
        asking_to_be_traced_is_killed_or_left_to_the_dumpable_flag,
    ),
    (
        "signalling_the_parent_is_killed_or_refused",
        signalling_the_parent_is_killed_or_refused,
    ),
    (
        "exceeding_the_memory_limit_aborts_and_the_parent_carries_on",
        exceeding_the_memory_limit_aborts_and_the_parent_carries_on,
    ),
    (
        "a_child_is_waited_for_stopped_or_reaped_when_dropped",
        a_child_is_waited_for_stopped_or_reaped_when_dropped,
    ),
];

fn main() {
    if let Some(args) = TypedArgs::from_argv(std::env::args_os()) {
        worker(args);
        return;
    }
    // Resource limits and Landlock do not bind root, so every refusal
    // below would be for the wrong reason, or not happen.
    assert!(!getuid().is_root(), "these tests must not run as root");
    let word = std::env::args().nth(1).unwrap_or_default();
    let emulated = EMULATED.iter().position(|known| *known == word);
    let host = Host::read(emulated);
    eprintln!("\nkernel: {host:?}");
    let mut failed = 0_usize;
    for (name, test) in TESTS {
        eprint!("test {name} ... ");
        if catch_unwind(AssertUnwindSafe(|| test(&host))).is_ok() {
            eprintln!("ok");
        } else {
            failed = failed.saturating_add(1);
            eprintln!("FAILED");
        }
    }
    let passed = TESTS.len().saturating_sub(failed);
    eprintln!(
        "\ntest result: {}. {passed} passed; {failed} failed",
        if failed == 0 { "ok" } else { "FAILED" }
    );
    // The kernel as it is goes first; then one more process per kernel
    // with something taken away, because a filter cannot be removed.
    let mut kernels_failed = usize::from(failed != 0);
    if emulated.is_none() {
        for word in EMULATED {
            let passed = Command::new("/proc/self/exe")
                .arg(word)
                .status()
                .expect("start the tests again")
                .success();
            kernels_failed = kernels_failed.saturating_add(usize::from(!passed));
        }
    }
    if kernels_failed != 0 {
        process::exit(1);
    }
}

// What the kernel offers, decided without the code under test.

/// What the kernel a worker meets offers.
#[expect(
    clippy::struct_excessive_bools,
    reason = "each bool is one independent fact about the kernel"
)]
#[derive(Debug)]
struct Host {
    /// The worker can install its seccomp filter.
    seccomp: bool,
    /// The worker can enforce a Landlock ruleset.
    landlock: bool,
    /// Landlock restricts TCP connections: its ABI 4, Linux 6.7.
    landlock_network: bool,
    /// Landlock scopes signals: its ABI 6, Linux 6.12.
    landlock_signals: bool,
    /// The Yama module is active, so `PR_SET_PTRACER` exists.
    yama: bool,
    /// Some seccomp filter binds the worker: its own, or the one this
    /// executable installed to take calls away.
    filtered: bool,
}

/// `ENOSYS`: what a kernel answers for a call it does not have.
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const ENOSYS: u32 = 38;
/// `landlock_create_ruleset`, `landlock_add_rule` and
/// `landlock_restrict_self`, which have the same numbers on every
/// architecture.
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const LANDLOCK_CALLS: [i64; 3] = [444, 445, 446];
/// The `seccomp` call's number, and seccompiler's name for the
/// architecture.
#[cfg(target_arch = "x86_64")]
const SECCOMP_CALL: (i64, seccompiler::TargetArch) = (317, seccompiler::TargetArch::x86_64);
#[cfg(target_arch = "aarch64")]
const SECCOMP_CALL: (i64, seccompiler::TargetArch) = (277, seccompiler::TargetArch::aarch64);

/// Makes the kernel answer `ENOSYS` to these calls, for this process and
/// every process it starts, from now on.
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
fn take_away(calls: &[i64]) {
    use seccompiler::{BpfProgram, SeccompAction, SeccompFilter};

    let rules = calls.iter().map(|&call| (call, Vec::new())).collect();
    let filter = SeccompFilter::new(
        rules,
        SeccompAction::Allow,
        SeccompAction::Errno(ENOSYS),
        SECCOMP_CALL.1,
    )
    .expect("a filter");
    let program = BpfProgram::try_from(filter).expect("a compiled filter");
    seccompiler::apply_filter(&program).expect("install the filter");
}

/// Takes away what the emulated kernel at this position in [`EMULATED`]
/// lacks, and returns whether Landlock and seccomp are still there.
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
fn emulate(position: usize) -> (bool, bool) {
    let (landlock, seccomp) = [(false, true), (true, false), (false, false)][position];
    // The Landlock calls go first: once the `seccomp` call is gone, no
    // further filter can be installed.
    if !landlock {
        take_away(&LANDLOCK_CALLS);
    }
    if !seccomp {
        take_away(&[SECCOMP_CALL.0]);
    }
    (landlock, seccomp)
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
fn emulate(_position: usize) -> (bool, bool) {
    (true, true)
}

impl Host {
    /// Reads what the running kernel offers, less what the emulated
    /// kernel at this position in [`EMULATED`] lacks.
    fn read(emulated: Option<usize>) -> Self {
        let lsm = fs::read_to_string("/sys/kernel/security/lsm").expect("the kernel's LSM list");
        let active = |module: &str| lsm.trim().split(',').any(|name| name == module);
        let release =
            fs::read_to_string("/proc/sys/kernel/osrelease").expect("the kernel's release");
        let mut numbers = release
            .split(|character: char| !character.is_ascii_digit())
            .map(|number| number.parse::<u32>().expect("a release number"));
        let version = (
            numbers.next().expect("a major version"),
            numbers.next().expect("a minor version"),
        );
        let (landlock_left, seccomp_left) = emulated.map_or((true, true), emulate);
        Self {
            seccomp: seccomp_left
                && cfg!(any(target_arch = "x86_64", target_arch = "aarch64"))
                && status(process::id(), "Seccomp:").is_some(),
            landlock: landlock_left && active("landlock"),
            landlock_network: version >= (6, 7),
            landlock_signals: version >= (6, 12),
            yama: active("yama"),
            filtered: emulated.is_some(),
        }
    }
}

// The worker side: what the test executable does when the launcher starts
// it again.

fn worker(args: TypedArgs) {
    match args.job() {
        Job::SelfTest => {
            answer_self_test(args.profile(), &mut io::stdout())
                .expect("write the self-test answer");
            wait_until_stopped();
        }
        Job::Serve => serve(args.profile()),
    }
}

fn serve(profile: Profile) {
    let mut hook = [0_u8; 1];
    io::stdin().read_exact(&mut hook).expect("read the hook");
    match hook[0] {
        QUIT => {}
        WAIT => report_then_confine(profile),
        hostile => {
            // The parent's ID is read first: asking for it is itself a
            // call the filter refuses.
            let parent = getppid();
            confine(profile).expect("confine the hostile worker");
            // What the action came to: the kernel's error number, or 0
            // when the kernel allowed it.
            let outcome = match hostile {
                OPEN => fs::File::open("/etc/hostname").map(drop),
                NETWORK => TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, 1)).map(drop),
                // `exec` replaces this process, with no `clone` first,
                // and returns only when the kernel refuses.
                EXEC => Err(Command::new("/bin/true").exec()),
                CLONE => thread::Builder::new().spawn(|| {}).map(drop),
                TRACE => set_ptracer(PTracer::Any).map_err(io::Error::from),
                SIGNAL => test_kill_process(parent.expect("a parent")).map_err(io::Error::from),
                MEMORY => exhaust_memory(),
                unknown => panic!("unknown hook {unknown}"),
            };
            let errno = outcome.err().map_or(0, |error| {
                u8::try_from(error.raw_os_error().unwrap_or(255)).unwrap_or(255)
            });
            let _ = io::stdout().write_all(&[errno]);
            process::exit(2);
        }
    }
}

/// One field of the worker's report: a length octet, then the octets.
fn field(report: &mut Vec<u8>, octets: &[u8]) {
    report.push(u8::try_from(octets.len()).expect("a field fits one length octet"));
    report.extend_from_slice(octets);
}

/// Reads what the launcher handed this process, from `/proc/self` while it
/// can still be read, then confines and reports. The environment is the
/// block the process was started with, so a variable the coverage runtime
/// sets for itself later is not in it; only its length is reported.
fn report_then_confine(profile: Profile) {
    let cwd = fs::read_link("/proc/self/cwd").expect("cwd");
    let descriptors = open_descriptors();
    let sockets: Vec<u8> = descriptors
        .iter()
        .map(|descriptor| {
            let target = fs::read_link(format!("/proc/self/fd/{descriptor}")).expect("target");
            u8::from(
                target
                    .as_os_str()
                    .as_encoded_bytes()
                    .starts_with(b"socket:["),
            )
        })
        .collect();
    let environment = fs::read("/proc/self/environ").expect("environ");
    let arguments = fs::read("/proc/self/cmdline").expect("cmdline");
    confine(profile).expect("confine the inspected worker");
    let mut report = vec![1];
    field(&mut report, cwd.as_os_str().as_encoded_bytes());
    field(&mut report, &descriptors);
    field(&mut report, &sockets);
    field(&mut report, &arguments);
    report.push(u8::try_from(environment.len()).unwrap_or(255));
    io::stdout().write_all(&report).expect("report");
    io::stdout().flush().expect("flush");
    wait_until_stopped();
}

/// The numbers of this process's open descriptors, without the one that
/// listed them.
fn open_descriptors() -> Vec<u8> {
    let names: Vec<u8> = fs::read_dir("/proc/self/fd")
        .expect("list descriptors")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .parse()
                .expect("a descriptor number")
        })
        .collect();
    let mut open: Vec<u8> = names
        .into_iter()
        .filter(|descriptor| fs::read_link(format!("/proc/self/fd/{descriptor}")).is_ok())
        .collect();
    open.sort_unstable();
    open
}

fn wait_until_stopped() {
    let mut sink = Vec::new();
    let _ = io::stdin().read_to_end(&mut sink);
}

fn exhaust_memory() -> ! {
    let mut held = Vec::new();
    loop {
        held.push(vec![0_u8; 16 * 1024 * 1024]);
    }
}

// The test side.

fn spawn_serve() -> (Child, UnixStream) {
    let (fds, ours) = Inherited::pair().expect("socket pair");
    let child = launch(
        Program::Worker,
        TypedArgs::new(Job::Serve, Profile::Scan),
        fds,
    )
    .expect("launch");
    (child, ours)
}

/// Starts a worker with `hook` and returns how it ended and the error
/// number it reported, if it lived to report one.
fn ask(hook: u8) -> (Exit, Option<u8>) {
    let (child, mut ours) = spawn_serve();
    ours.write_all(&[hook]).expect("write the hook");
    let mut errno = [0_u8; 1];
    let reported = ours.read_exact(&mut errno).ok().map(|()| errno[0]);
    (child.wait().expect("wait"), reported)
}

/// The filter killed the worker with `SIGSYS` before it could report.
const KILLED: (Exit, Option<u8>) = (Exit::Killed(Cause::ForbiddenCall), None);

/// The worker lived to report this error number, or 0 for "allowed".
const fn reported(errno: u8) -> (Exit, Option<u8>) {
    (Exit::Code(2), Some(errno))
}

/// `EPERM`.
const EPERM: u8 = 1;
/// `EAGAIN`.
const EAGAIN: u8 = 11;
/// `EACCES`.
const EACCES: u8 = 13;
/// `ECONNREFUSED`: the connection was tried and nothing was listening.
const ECONNREFUSED: u8 = 111;
/// The kernel carried the action out.
const ALLOWED: u8 = 0;

/// Starts a worker with `hook` and asserts the one outcome this kernel
/// gives it. With the seccomp filter that is `SIGSYS` for every hostile
/// action: each makes a call that is off the allowlist, and for a library
/// routine that can come before the call the action is named for.
/// `without_seccomp` is the outcome where the filter is missing.
fn assert_outcome(host: &Host, hook: u8, without_seccomp: (Exit, Option<u8>)) {
    let expected = if host.seccomp {
        KILLED
    } else {
        without_seccomp
    };
    assert_eq!(ask(hook), expected);
}

/// What a `WAIT` worker reported.
#[derive(Debug, PartialEq, Eq)]
struct Report {
    confined: u8,
    cwd: Vec<u8>,
    descriptors: Vec<u8>,
    sockets: Vec<u8>,
    arguments: Vec<u8>,
    environment_length: u8,
}

fn read_octet(channel: &mut UnixStream) -> u8 {
    let mut octet = [0_u8; 1];
    channel.read_exact(&mut octet).expect("an octet");
    octet[0]
}

fn read_field(channel: &mut UnixStream) -> Vec<u8> {
    let mut octets = vec![0_u8; usize::from(read_octet(channel))];
    channel.read_exact(&mut octets).expect("a field");
    octets
}

/// Starts a worker, lets it confine itself, and returns it, still
/// running, with its report.
fn inspect() -> (Child, UnixStream, Report) {
    let (child, mut ours) = spawn_serve();
    ours.write_all(&[WAIT]).expect("write the hook");
    let report = Report {
        confined: read_octet(&mut ours),
        cwd: read_field(&mut ours),
        descriptors: read_field(&mut ours),
        sockets: read_field(&mut ours),
        arguments: read_field(&mut ours),
        environment_length: read_octet(&mut ours),
    };
    (child, ours, report)
}

/// The value of one line of `/proc/<pid>/status`.
fn status(pid: u32, name: &str) -> Option<String> {
    let status = fs::read_to_string(format!("/proc/{pid}/status")).expect("status");
    status
        .lines()
        .find_map(|line| line.strip_prefix(name))
        .map(|value| value.trim().to_owned())
}

/// The soft and hard values of one line of `/proc/<pid>/limits`.
fn limit(limits: &str, label: &str) -> Vec<String> {
    limits
        .lines()
        .find_map(|line| line.strip_prefix(label))
        .map(|values| {
            values
                .split_whitespace()
                .take(2)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// Namespaces are never created, so the tier is always the reduced one;
/// the notice names them and whichever of seccomp and Landlock this
/// kernel lacks.
///
/// Verifies: SEC-MED-024
fn self_test_reports_the_tier_the_kernel_allows_and_names_what_is_missing(host: &Host) {
    let notice = match (host.seccomp, host.landlock) {
        (true, true) => {
            "Reduced isolation: media workers run without namespaces. \
             They still run in a separate process with resource limits \
             and no new privileges."
        }
        (true, false) => {
            "Reduced isolation: media workers run without Landlock and namespaces. \
             They still run in a separate process with resource limits \
             and no new privileges."
        }
        (false, true) => {
            "Reduced isolation: media workers run without system call filtering (seccomp) \
             and namespaces. \
             They still run in a separate process with resource limits \
             and no new privileges."
        }
        (false, false) => {
            "Reduced isolation: media workers run without system call filtering (seccomp), \
             Landlock and namespaces. \
             They still run in a separate process with resource limits \
             and no new privileges."
        }
    };
    let report = self_test(Profile::Scan);
    assert_eq!(
        report,
        TierReport {
            tier: Tier::Reduced,
            notice: Some(notice.to_owned()),
        }
    );
    assert!(report.memory_safe_parsing());
    assert!(!report.native_decoders());
}

/// The test process holds a descriptor without close-on-exec while it
/// launches, as a server started by another program might. The worker
/// still gets only its socket, as descriptors 0, 1 and 2; its arguments
/// are the three fixed words; its environment is empty, with no exception.
///
/// Verifies: SEC-OPS-014, SEC-STD-040, SEC-HIS-020
fn a_worker_gets_the_socket_three_fixed_words_and_nothing_else(_host: &Host) {
    let (stray, _peer) = UnixStream::pair().expect("stray pair");
    fcntl_setfd(&stray, FdFlags::empty()).expect("clear close-on-exec");
    let (child, _ours, report) = inspect();
    assert_eq!(
        report,
        Report {
            confined: 1,
            cwd: b"/".to_vec(),
            descriptors: vec![0, 1, 2],
            sockets: vec![1, 1, 1],
            arguments: b"/proc/self/exe\0--gunmetal-worker\0serve\0scan\0".to_vec(),
            environment_length: 0,
        }
    );
    // This process has an environment of its own, so the worker's empty
    // one is the launcher's doing.
    assert!(std::env::vars_os().next().is_some());
    assert_eq!(child.stop().expect("stop"), Exit::Killed(Cause::Kill));
}

/// Verifies: SEC-MED-021, SEC-MED-022
fn a_confined_worker_is_limited_single_threaded_undumpable_and_filtered(host: &Host) {
    // Before the worker confines itself, this process, which runs as the
    // same user, can read where its working directory is.
    let (before, _ours) = spawn_serve();
    assert_eq!(
        fs::read_link(format!("/proc/{}/cwd", before.id())).ok(),
        Some(PathBuf::from("/"))
    );
    drop(before);

    let (child, _ours, report) = inspect();
    assert_eq!(report.confined, 1);
    let pid = child.id();
    // The dumpable flag is 0: the same reads are now refused.
    assert_eq!(
        [
            fs::read_link(format!("/proc/{pid}/cwd")).map(drop),
            fs::read(format!("/proc/{pid}/environ")).map(drop),
            fs::read_dir(format!("/proc/{pid}/fd")).map(drop),
        ]
        .map(|read| read.map_err(|error| error.kind())),
        [Err(io::ErrorKind::PermissionDenied); 3]
    );
    assert_eq!(status(pid, "Threads:").as_deref(), Some("1"));
    assert_eq!(status(pid, "NoNewPrivs:").as_deref(), Some("1"));
    // Mode 2 is a filter, mode 0 none.
    let mode = if host.seccomp || host.filtered {
        "2"
    } else {
        "0"
    };
    assert_eq!(status(pid, "Seccomp:").as_deref(), Some(mode));
    let limits = fs::read_to_string(format!("/proc/{pid}/limits")).expect("limits");
    assert_eq!(
        limit(&limits, "Max address space"),
        ["536870912", "536870912"]
    );
    assert_eq!(limit(&limits, "Max core file size"), ["0", "0"]);
    assert_eq!(limit(&limits, "Max open files"), ["32", "32"]);
    assert_eq!(limit(&limits, "Max cpu time"), ["60", "61"]);
    assert_eq!(limit(&limits, "Max processes"), ["0", "0"]);
    assert_eq!(limit(&limits, "Max file size"), ["0", "0"]);
    assert_eq!(child.stop().expect("stop"), Exit::Killed(Cause::Kill));
}

/// Without the filter Landlock refuses the open with `EACCES`. With
/// neither, the floor does not stop a worker opening a path, which is why
/// the notice names both.
///
/// Verifies: SEC-MED-022, SEC-TM-044
fn opening_a_path_is_killed_or_refused(host: &Host) {
    let without_seccomp = if host.landlock { EACCES } else { ALLOWED };
    assert_outcome(host, OPEN, reported(without_seccomp));
}

/// Without the filter Landlock refuses the TCP connection with `EACCES`
/// where its ABI covers the network. Otherwise the connection is tried,
/// and fails only because nothing listens on port 1.
///
/// Verifies: SEC-MED-022, SEC-TM-044
fn connecting_a_network_socket_is_killed_or_refused(host: &Host) {
    let without_seccomp = if host.landlock && host.landlock_network {
        EACCES
    } else {
        ECONNREFUSED
    };
    assert_outcome(host, NETWORK, reported(without_seccomp));
}

/// Without the filter Landlock refuses to execute the file with `EACCES`.
/// With neither, the program runs in the worker's place and exits with
/// its own status, 0.
///
/// Verifies: SEC-MED-022, SEC-TM-044
fn executing_a_program_is_killed_or_refused(host: &Host) {
    let without_seccomp = if host.landlock {
        reported(EACCES)
    } else {
        (Exit::Code(0), None)
    };
    assert_outcome(host, EXEC, without_seccomp);
}

/// A thread is a `clone`, the call behind `fork` on Linux. `fork` by name
/// needs `unsafe`, which the workspace forbids; the allowlist's own test
/// shows that `fork`, `vfork`, `clone` and `clone3` are all off it.
/// Without the filter the process limit of 0 refuses it with `EAGAIN`.
///
/// Verifies: SEC-MED-022, SEC-TM-044
fn starting_a_thread_is_killed_or_refused(host: &Host) {
    assert_outcome(host, CLONE, reported(EAGAIN));
}

/// `ptrace` by name needs `unsafe`, which the workspace forbids. The hook
/// makes the tracing call safe code can make, `PR_SET_PTRACER`, which
/// asks Yama to let any process trace this one; the allowlist's own test
/// shows that `ptrace` and `prctl` are both off it. Without the filter
/// the call is allowed where Yama is active and is `EINVAL` where it is
/// not, and what then keeps another process from attaching is the cleared
/// dumpable flag, which the limits test observes.
///
/// Verifies: SEC-MED-022
fn asking_to_be_traced_is_killed_or_left_to_the_dumpable_flag(host: &Host) {
    /// `EINVAL`.
    const EINVAL: u8 = 22;
    let without_seccomp = if host.yama { ALLOWED } else { EINVAL };
    assert_outcome(host, TRACE, reported(without_seccomp));
}

/// Without the filter Landlock refuses the signal with `EPERM` where its
/// ABI scopes signals. Otherwise the floor does not stop a worker
/// signalling a process of the same user.
///
/// Verifies: SEC-MED-022, SEC-TM-044
fn signalling_the_parent_is_killed_or_refused(host: &Host) {
    let without_seccomp = if host.landlock && host.landlock_signals {
        EPERM
    } else {
        ALLOWED
    };
    assert_outcome(host, SIGNAL, reported(without_seccomp));
}

/// A worker that runs out of memory aborts, the launcher reports the
/// cause, and this process, standing in for the server, carries on and
/// starts another worker.
///
/// Verifies: SEC-MED-018, SEC-MED-021, SEC-TM-044
fn exceeding_the_memory_limit_aborts_and_the_parent_carries_on(_host: &Host) {
    assert_eq!(ask(MEMORY).0, Exit::Killed(Cause::Abort));
    let (child, _ours, report) = inspect();
    assert_eq!(report.confined, 1);
    assert_eq!(child.stop().expect("stop"), Exit::Killed(Cause::Kill));
}

fn a_child_is_waited_for_stopped_or_reaped_when_dropped(_host: &Host) {
    assert_eq!(ask(QUIT).0, Exit::Code(0));

    let (child, _ours) = spawn_serve();
    assert_eq!(child.stop().expect("stop"), Exit::Killed(Cause::Kill));

    let (child, _ours) = spawn_serve();
    let pid = Pid::from_raw(i32::try_from(child.id()).expect("a process ID")).expect("not zero");
    drop(child);
    assert_eq!(test_kill_process(pid), Err(Errno::SRCH));
}
