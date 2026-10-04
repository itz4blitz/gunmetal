//! Integration tests of the sandbox launcher and a confined worker.
//!
//! This file is its own executable (`harness = false`) so the launcher can
//! start it again as the worker: `/proc/self/exe` with the hidden worker
//! arguments enters the child path. No extra binary ships, and the hostile
//! hooks exist only in this test executable.
#![expect(
    clippy::disallowed_methods,
    reason = "the hostile worker tries each forbidden action by path, socket and Command, and the test reads /proc to see what the kernel holds for a worker (SEC-MED-022, SEC-TM-044)"
)]

use gunmetal_worker::sandbox::{
    Cause, Child, Exit, Inherited, Job, Profile, Program, Tier, TierReport, TypedArgs,
    answer_self_test, confine, launch, self_test,
};
use rustix::io::{Errno, FdFlags, fcntl_setfd};
use rustix::process::{PTracer, Pid, getppid, set_ptracer, test_kill_process};
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

fn main() {
    if let Some(args) = TypedArgs::from_argv(std::env::args_os()) {
        worker(args);
        return;
    }
    let tests: &[(&str, fn())] = &[
        (
            "self_test_reports_the_reduced_tier_and_names_namespaces",
            self_test_reports_the_reduced_tier_and_names_namespaces,
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
            "asking_to_be_traced_is_killed_or_refused",
            asking_to_be_traced_is_killed_or_refused,
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
    let mut failed = 0_usize;
    for &(name, test) in tests {
        eprint!("test {name} ... ");
        if catch_unwind(AssertUnwindSafe(test)).is_ok() {
            eprintln!("ok");
        } else {
            failed = failed.saturating_add(1);
            eprintln!("FAILED");
        }
    }
    let passed = tests.len().saturating_sub(failed);
    eprintln!(
        "\ntest result: {}. {passed} passed; {failed} failed",
        if failed == 0 { "ok" } else { "FAILED" }
    );
    if failed != 0 {
        process::exit(1);
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
            let error = match hostile {
                OPEN => fs::File::open("/etc/hostname")
                    .map(drop)
                    .expect_err("the open must fail"),
                NETWORK => TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, 1))
                    .map(drop)
                    .expect_err("the connect must fail"),
                // `exec` replaces this process, with no `clone` first,
                // and returns only when the kernel refuses.
                EXEC => Command::new("/bin/true").exec(),
                CLONE => thread::Builder::new()
                    .spawn(|| {})
                    .map(drop)
                    .expect_err("the clone must fail"),
                TRACE => io::Error::from(
                    set_ptracer(PTracer::Any).expect_err("PR_SET_PTRACER must fail"),
                ),
                SIGNAL => io::Error::from(
                    test_kill_process(parent.expect("a parent")).expect_err("the kill must fail"),
                ),
                MEMORY => exhaust_memory(),
                unknown => panic!("unknown hook {unknown}"),
            };
            let errno = u8::try_from(error.raw_os_error().unwrap_or(0)).unwrap_or(255);
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

/// The exact outcomes SEC-MED-022 allows a forbidden action: the filter
/// kills the worker with `SIGSYS`, or the kernel refuses the call with
/// `EPERM` (1) or `EACCES` (13) and the worker reports it. The filter ends
/// the worker at the first call an action makes that is off the
/// allowlist, which for a library routine can come before the call the
/// action is named for.
fn assert_killed_or_refused(hook: u8) {
    let outcome = ask(hook);
    assert!(
        matches!(
            outcome,
            (Exit::Killed(Cause::ForbiddenCall), None) | (Exit::Code(2), Some(1 | 13))
        ),
        "expected SIGSYS, EPERM or EACCES, got {outcome:?}"
    );
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

/// Verifies: SEC-MED-024
fn self_test_reports_the_reduced_tier_and_names_namespaces() {
    let report = self_test(Profile::Scan);
    assert_eq!(
        report,
        TierReport {
            tier: Tier::Reduced,
            notice: Some(
                "Reduced isolation: media workers run without namespaces. \
                 They still run in a separate process with resource limits \
                 and no new privileges."
                    .to_owned()
            ),
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
fn a_worker_gets_the_socket_three_fixed_words_and_nothing_else() {
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
fn a_confined_worker_is_limited_single_threaded_undumpable_and_filtered() {
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
    assert_eq!(status(pid, "Seccomp:").as_deref(), Some("2"));
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

/// Verifies: SEC-MED-022, SEC-TM-044
fn opening_a_path_is_killed_or_refused() {
    assert_killed_or_refused(OPEN);
}

/// Verifies: SEC-MED-022, SEC-TM-044
fn connecting_a_network_socket_is_killed_or_refused() {
    assert_killed_or_refused(NETWORK);
}

/// Verifies: SEC-MED-022, SEC-TM-044
fn executing_a_program_is_killed_or_refused() {
    assert_killed_or_refused(EXEC);
}

/// A thread is a `clone`, the call behind `fork` on Linux. `fork` by name
/// needs `unsafe`, which the workspace forbids; the allowlist's own test
/// shows that `fork`, `vfork`, `clone` and `clone3` are all off it.
///
/// Verifies: SEC-MED-022, SEC-TM-044
fn starting_a_thread_is_killed_or_refused() {
    assert_killed_or_refused(CLONE);
}

/// `ptrace` by name needs `unsafe`, which the workspace forbids. The hook
/// makes the tracing call safe code can make, `PR_SET_PTRACER`; the
/// allowlist's own test shows that `ptrace` and `prctl` are both off it.
///
/// Verifies: SEC-MED-022
fn asking_to_be_traced_is_killed_or_refused() {
    assert_killed_or_refused(TRACE);
}

/// Verifies: SEC-TM-044
fn signalling_the_parent_is_killed_or_refused() {
    assert_killed_or_refused(SIGNAL);
}

/// A worker that runs out of memory aborts, the launcher reports the
/// cause, and this process, standing in for the server, carries on and
/// starts another worker.
///
/// Verifies: SEC-MED-018, SEC-MED-021, SEC-TM-044
fn exceeding_the_memory_limit_aborts_and_the_parent_carries_on() {
    assert_eq!(ask(MEMORY).0, Exit::Killed(Cause::Abort));
    let (child, _ours, report) = inspect();
    assert_eq!(report.confined, 1);
    assert_eq!(child.stop().expect("stop"), Exit::Killed(Cause::Kill));
}

fn a_child_is_waited_for_stopped_or_reaped_when_dropped() {
    assert_eq!(ask(QUIT).0, Exit::Code(0));

    let (child, _ours) = spawn_serve();
    assert_eq!(child.stop().expect("stop"), Exit::Killed(Cause::Kill));

    let (child, _ours) = spawn_serve();
    let pid = Pid::from_raw(i32::try_from(child.id()).expect("a process ID")).expect("not zero");
    drop(child);
    assert_eq!(test_kill_process(pid), Err(Errno::SRCH));
}
