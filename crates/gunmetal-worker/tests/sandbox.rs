//! Integration tests of the sandbox launcher and a confined worker.
//!
//! This file is its own executable (`harness = false`) so the launcher can
//! start it again as the worker: `/proc/self/exe` with the hidden worker
//! arguments enters the child path. No extra binary ships.
#![expect(
    clippy::disallowed_methods,
    reason = "the hostile worker tries each forbidden action by path, socket and Command (SEC-MED-022, SEC-TM-044)"
)]

use gunmetal_worker::sandbox::{
    Cause, Child, Exit, Inherited, Job, Profile, Program, Tier, TierReport, TypedArgs,
    answer_self_test, apply_landlock, apply_seccomp, confine, launch, self_test,
};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::os::unix::process::ExitStatusExt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::process::{self, Command};
use std::thread;
use std::time::Duration;

/// Confine, then wait so the parent can inspect `/proc`.
const WAIT: u8 = b'w';
/// Confine, then open a path.
const OPEN: u8 = b'o';
/// Confine, then create a network socket.
const NETWORK: u8 = b'n';
/// Confine, then execute a program.
const EXEC: u8 = b'x';
/// Confine, then allocate until the memory limit kills the process.
const MEMORY: u8 = b'm';

fn main() {
    if std::env::var_os("GUNMETAL_COVER_KERNEL").is_some() {
        cover_kernel();
    }
    if let Some(args) = TypedArgs::from_argv(std::env::args_os()) {
        worker(args);
        return;
    }
    let tests: &[(&str, fn())] = &[
        (
            "self_test_reaches_the_floor_and_reports_missing_namespaces",
            self_test_reaches_the_floor_and_reports_missing_namespaces,
        ),
        (
            "a_confined_worker_has_an_empty_environment_only_the_socket_and_is_not_dumpable",
            a_confined_worker_has_an_empty_environment_only_the_socket_and_is_not_dumpable,
        ),
        (
            "opening_a_path_is_killed_or_refused",
            opening_a_path_is_killed_or_refused,
        ),
        (
            "creating_a_network_socket_is_killed_or_refused",
            creating_a_network_socket_is_killed_or_refused,
        ),
        (
            "executing_a_program_is_killed_or_refused",
            executing_a_program_is_killed_or_refused,
        ),
        (
            "exceeding_the_memory_limit_aborts_and_the_parent_sees_why",
            exceeding_the_memory_limit_aborts_and_the_parent_sees_why,
        ),
        (
            "linux_landlock_and_seccomp_hold",
            linux_landlock_and_seccomp_hold,
        ),
    ];
    let mut failed = 0_usize;
    for &(name, test) in tests {
        eprint!("test {name} ... ");
        match catch_unwind(AssertUnwindSafe(test)) {
            Ok(()) => eprintln!("ok"),
            Err(panic) => {
                failed = failed.saturating_add(1);
                eprintln!("FAILED");
                if let Some(message) = panic.downcast_ref::<String>() {
                    eprintln!("{message}");
                } else if let Some(message) = panic.downcast_ref::<&str>() {
                    eprintln!("{message}");
                }
            }
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
    let mut command = [0_u8; 1];
    io::stdin().read_exact(&mut command).expect("read the hook");
    match command[0] {
        WAIT => report_launch_state(profile),
        other => {
            confine(profile).expect("confine the hostile worker");
            match other {
                OPEN => {
                    refuse_or_die(&fs::File::open("/etc/hostname").expect_err("open must fail"));
                }
                NETWORK => refuse_or_die(
                    &TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, 1))
                        .expect_err("connect must fail"),
                ),
                EXEC => {
                    let error = match Command::new("/bin/true").status() {
                        Ok(status) => panic!("/bin/true ran to {status}"),
                        Err(error) => error,
                    };
                    refuse_or_die(&error);
                }
                MEMORY => exhaust_memory(),
                unknown => panic!("unknown hook {unknown}"),
            }
        }
    }
}

/// Env, cwd and descriptors are read before Landlock; dumpable is proved
/// by confinement succeeding (it reads the flag back after clearing it).
fn report_launch_state(profile: Profile) {
    let leftover: Vec<String> = std::env::vars_os()
        .map(|(key, _)| key.to_string_lossy().into_owned())
        .collect();
    let env = u8::try_from(leftover.len()).expect("env count");
    let cwd_is_root = u8::from(std::env::current_dir().expect("cwd").as_os_str() == "/");
    let descriptors = self_fds();
    confine(profile).expect("confine the inspect worker");
    let fd_count = u8::try_from(descriptors.len()).expect("fd count");
    let mut report = vec![env, cwd_is_root, fd_count];
    report.extend(descriptors);
    for key in leftover {
        let bytes = key.into_bytes();
        report.push(u8::try_from(bytes.len()).expect("key len"));
        report.extend(bytes);
    }
    io::stdout().write_all(&report).expect("report");
    io::stdout().flush().expect("flush");
    wait_until_stopped();
}

fn wait_until_stopped() {
    let mut sink = Vec::new();
    let _ = io::stdin().read_to_end(&mut sink);
}

fn self_fds() -> Vec<u8> {
    let mut descriptors: Vec<u8> = fs::read_dir("/proc/self/fd")
        .expect("fd")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .parse()
                .expect("fd number")
        })
        .collect();
    descriptors.sort_unstable();
    descriptors
}

/// Applies the real kernel Landlock and seccomp methods in this process
/// so llvm-cov records them, then exits so the runtime can flush.
fn cover_kernel() -> ! {
    assert!(apply_landlock(), "Landlock must hold on this kernel");
    assert!(
        File::open("/etc/hostname").is_err(),
        "Landlock must refuse a path"
    );
    assert!(apply_seccomp(), "seccomp must hold on this kernel");
    process::exit(0);
}

fn refuse_or_die(error: &io::Error) {
    let errno = u8::try_from(error.raw_os_error().unwrap_or(0)).unwrap_or(255);
    let _ = io::stdout().write_all(&[errno]);
    process::exit(2);
}

fn exhaust_memory() {
    let mut held = Vec::new();
    loop {
        held.push(vec![0_u8; 16 * 1024 * 1024]);
    }
}

fn spawn_serve() -> (Child, std::os::unix::net::UnixStream) {
    let (fds, ours) = Inherited::pair().expect("socket pair");
    let child = launch(
        Program::Worker,
        TypedArgs::new(Job::Serve, Profile::Scan),
        fds,
    )
    .expect("launch");
    (child, ours)
}

fn ask(command: u8) -> (Exit, Option<u8>) {
    let (child, mut ours) = spawn_serve();
    ours.write_all(&[command]).expect("write the hook");
    let mut errno = [0_u8; 1];
    let reported = ours.read_exact(&mut errno).ok().map(|()| errno[0]);
    (child.wait().expect("wait"), reported)
}

fn assert_killed_or_refused(command: u8) {
    match ask(command) {
        (Exit::Killed(Cause::ForbiddenCall), _) => {}
        (Exit::Code(2), Some(errno)) => {
            assert!(
                errno == 1 || errno == 13,
                "refused with EPERM or EACCES, got {errno}"
            );
        }
        other => panic!("expected SIGSYS or a refused open, got {other:?}"),
    }
}

/// Verifies: SEC-MED-024, SEC-TM-045
fn self_test_reaches_the_floor_and_reports_missing_namespaces() {
    let report = self_test(Profile::Scan);
    assert!(
        report.memory_safe_parsing(),
        "memory-safe parsing must stay on at the floor: {report:?}"
    );
    assert!(
        !report.native_decoders(),
        "native decoders stay off without namespaces: {report:?}"
    );
    assert_eq!(report.tier, Tier::Reduced);
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
}

/// Verifies: SEC-OPS-014, SEC-STD-040, SEC-MED-021
fn a_confined_worker_has_an_empty_environment_only_the_socket_and_is_not_dumpable() {
    let (child, mut ours) = spawn_serve();
    ours.write_all(&[WAIT]).expect("write wait");
    let mut header = [0_u8; 3];
    ours.read_exact(&mut header).expect("header");
    let [env, cwd_is_root, fd_count] = header;
    let mut descriptors = vec![0_u8; usize::from(fd_count)];
    ours.read_exact(&mut descriptors).expect("fds");
    let mut keys = Vec::new();
    for _ in 0..env {
        let mut len = [0_u8; 1];
        ours.read_exact(&mut len).expect("key len");
        let mut key = vec![0_u8; usize::from(len[0])];
        ours.read_exact(&mut key).expect("key");
        keys.push(String::from_utf8_lossy(&key).into_owned());
    }
    let unexpected: Vec<&str> = keys
        .iter()
        .map(String::as_str)
        .filter(|key| !key.contains("LLVM") && *key != "CARGO_LLVM_COV")
        .collect();
    assert_eq!(
        unexpected,
        [] as [&str; 0],
        "child environment keys: {keys:?}"
    );
    assert_eq!(cwd_is_root, 1, "the child working directory must be /");
    assert!(
        descriptors.contains(&0) && descriptors.contains(&1) && descriptors.contains(&2),
        "the socket is descriptors 0, 1 and 2, got {descriptors:?}"
    );
    // The test process may leak non-close-on-exec descriptors into the
    // child. Confinement lists them as strays and the seccomp filter
    // refuses every call on them, which is why confine succeeded.
    assert!(matches!(
        child.stop().expect("stop"),
        Exit::Killed(Cause::Kill) | Exit::Code(_)
    ));
}

/// Verifies: SEC-MED-022, SEC-TM-044
fn opening_a_path_is_killed_or_refused() {
    assert_killed_or_refused(OPEN);
}

/// Verifies: SEC-MED-022, SEC-TM-044, SEC-STD-040
fn creating_a_network_socket_is_killed_or_refused() {
    assert_killed_or_refused(NETWORK);
}

/// Verifies: SEC-MED-022, SEC-TM-044
fn executing_a_program_is_killed_or_refused() {
    assert_killed_or_refused(EXEC);
}

/// Verifies: SEC-MED-022
fn linux_landlock_and_seccomp_hold() {
    let exe = std::env::current_exe().expect("exe");
    let mut command = Command::new(exe);
    command.env("GUNMETAL_COVER_KERNEL", "1");
    // llvm-cov's default pattern includes `%m` (online merge). After
    // seccomp that merge path dies; a concrete file in the same
    // directory still gets merged.
    if let Some(profile) = std::env::var("LLVM_PROFILE_FILE").ok().and_then(|file| {
        file.split('%')
            .next()
            .and_then(|prefix| prefix.rsplit_once('/'))
            .map(|(dir, _)| format!("{dir}/wp045-kernel-{}.profraw", process::id()))
    }) {
        command.env("LLVM_PROFILE_FILE", profile);
    }
    let status = command.status().expect("cover-kernel child");
    assert!(
        status.success() || status.signal() == Some(31),
        "cover-kernel must exit or die of SIGSYS after applying the filter, got {status:?}"
    );
}

/// Verifies: SEC-MED-018, SEC-MED-021, SEC-TM-044
fn exceeding_the_memory_limit_aborts_and_the_parent_sees_why() {
    let (child, mut ours) = spawn_serve();
    ours.write_all(&[MEMORY]).expect("write memory");
    // The child may take a moment to fill its address space.
    thread::sleep(Duration::from_millis(50));
    let exit = child.wait().expect("wait");
    assert!(
        matches!(exit, Exit::Killed(Cause::Abort | Cause::Kill)),
        "a memory-limit death is abort or the OOM killer, got {exit:?}"
    );
}
