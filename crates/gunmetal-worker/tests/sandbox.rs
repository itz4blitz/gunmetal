//! Integration tests of the sandbox launcher and a confined worker.
//!
//! This file is its own executable (`harness = false`) so the launcher can
//! start it again as the worker: `/proc/self/exe` with the hidden worker
//! arguments enters the child path. No extra binary ships.
#![expect(
    clippy::disallowed_methods,
    reason = "the hostile worker tries each forbidden action by path, socket, clone, ptrace and Command (SEC-MED-022, SEC-TM-044)"
)]

use gunmetal_worker::sandbox::{
    Cause, Child, Exit, Inherited, Job, Profile, Program, Tier, TierReport, TypedArgs,
    answer_self_test, confine, launch, mark_others_close_on_exec, self_test,
};
use rustix::process::{PTracer, Pid, PidfdFlags, pidfd_open, set_ptracer};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::os::unix::process::ExitStatusExt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::process::{self, Command, Stdio};
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
/// Confine, then fork (clone a new process).
const FORK: u8 = b'f';
/// Confine, then ptrace.
const PTRACE: u8 = b't';

fn main() {
    mark_others_close_on_exec();
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
        ("forking_is_killed_or_refused", forking_is_killed_or_refused),
        ("ptrace_is_killed_or_refused", ptrace_is_killed_or_refused),
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
                FORK => try_fork(),
                PTRACE => try_ptrace(),
                MEMORY => exhaust_memory(),
                unknown => panic!("unknown hook {unknown}"),
            }
        }
    }
}

/// Confine, then report the post-confine environment (in-memory; Landlock
/// has taken `/proc` away) and the cwd and descriptors observed just
/// before the filter, which confinement does not change. The parent reads
/// Dumpable, Threads and rlimits from `/proc/{pid}`: those files stay
/// visible after `PR_SET_DUMPABLE` 0; `environ`, `cwd` and `fd` do not.
fn report_launch_state(profile: Profile) {
    let cwd = std::env::current_dir().expect("cwd before landlock");
    let descriptors = live_self_fds();
    confine(profile).expect("confine the inspect worker");
    let env: Vec<(Vec<u8>, Vec<u8>)> = std::env::vars_os()
        .map(|(key, value)| (key.into_encoded_bytes(), value.into_encoded_bytes()))
        .collect();
    let env_count = u8::try_from(env.len()).unwrap_or(255);
    let llvm = u8::from(env.iter().any(|(key, _)| key.starts_with(b"LLVM")));
    let cwd_bytes = cwd.as_os_str().as_encoded_bytes();
    let cwd_len = u8::try_from(cwd_bytes.len()).expect("cwd fits");
    let fd_count = u8::try_from(descriptors.len()).expect("fd count");
    let mut report = vec![1, env_count, llvm, cwd_len];
    report.extend_from_slice(cwd_bytes);
    report.push(fd_count);
    report.extend(descriptors.iter().map(|&fd| u8::try_from(fd).expect("fd")));
    for (key, value) in env {
        let key_len = u8::try_from(key.len()).expect("key");
        let value_len = u8::try_from(value.len()).expect("value");
        report.push(key_len);
        report.extend_from_slice(&key);
        report.push(value_len);
        report.extend_from_slice(&value);
    }
    io::stdout().write_all(&report).expect("report");
    io::stdout().flush().expect("flush");
    wait_until_stopped();
}

/// Open descriptors of this process, excluding the directory used to list
/// them.
fn live_self_fds() -> Vec<u32> {
    let names: Vec<u32> = fs::read_dir("/proc/self/fd")
        .expect("self fd")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .parse()
                .expect("fd number")
        })
        .collect();
    let mut live: Vec<u32> = names
        .into_iter()
        .filter(|&descriptor| fs::read_link(format!("/proc/self/fd/{descriptor}")).is_ok())
        .collect();
    live.sort_unstable();
    live
}

fn wait_until_stopped() {
    let mut sink = Vec::new();
    let _ = io::stdin().read_to_end(&mut sink);
}

/// Applies the real kernel confine path in this process so llvm-cov
/// records Landlock and seccomp, then waits so the parent can read
/// `/proc` before we exit.
fn cover_kernel() -> ! {
    let enforced = confine(Profile::Scan).expect("confine the cover-kernel worker");
    assert!(enforced.landlock, "Landlock must hold on this kernel");
    assert!(enforced.seccomp, "seccomp must hold on this kernel");
    wait_until_stopped();
    process::exit(0);
}

fn refuse_or_die(error: &io::Error) {
    let errno = u8::try_from(error.raw_os_error().unwrap_or(0)).unwrap_or(255);
    let _ = io::stdout().write_all(&[errno]);
    process::exit(2);
}

/// `clone` is Linux's fork family (SEC-MED-022 names `clone`/`fork`).
/// `fork(2)` itself needs `unsafe`, which the workspace forbids.
fn try_fork() {
    match thread::Builder::new().spawn(|| {}) {
        Ok(_) => panic!("clone ran"),
        Err(error) => refuse_or_die(&error),
    }
}

/// `ptrace(2)` needs `unsafe`. The worker instead issues the ptrace
/// attach family that rustix exposes as safe functions: `pidfd_open` of
/// another process and `PR_SET_PTRACER`. Neither is on the allowlist.
fn try_ptrace() {
    if let Some(init) = Pid::from_raw(1) {
        match pidfd_open(init, PidfdFlags::empty()) {
            Ok(_) => panic!("pidfd_open of init ran"),
            Err(error) => refuse_or_die(&io::Error::from(error)),
        }
    }
    match set_ptracer(PTracer::Any) {
        Ok(()) => panic!("PR_SET_PTRACER ran"),
        Err(error) => refuse_or_die(&io::Error::from(error)),
    }
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

/// The launcher clears the environment (`env_clear`); it does not copy
/// `LLVM*` into the child. The coverage runtime may `setenv` `__LLVM_*`
/// after exec. That is not a launcher exception: host keys still fail.
fn assert_empty_launch_environment(llvm: u8, env: &[(Vec<u8>, Vec<u8>)]) {
    let keys: Vec<&[u8]> = env.iter().map(|(key, _)| key.as_slice()).collect();
    assert!(
        env.iter()
            .all(|(key, _)| key.starts_with(b"LLVM") || key.starts_with(b"__LLVM")),
        "launcher must not pass host environment, got {keys:?}"
    );
    if env.is_empty() {
        assert_eq!(llvm, 0, "no LLVM* exception in the child environment");
        return;
    }
    if let Some((_, value)) = env
        .iter()
        .find(|(key, _)| key.as_slice() == b"LLVM_PROFILE_FILE")
    {
        if let Ok(parent) = std::env::var("LLVM_PROFILE_FILE") {
            assert_ne!(
                value.as_slice(),
                parent.as_bytes(),
                "production launch must not copy the parent's LLVM_PROFILE_FILE"
            );
        }
    }
}

/// Verifies: SEC-OPS-014, SEC-STD-040, SEC-MED-021
fn a_confined_worker_has_an_empty_environment_only_the_socket_and_is_not_dumpable() {
    let (child, mut ours) = spawn_serve();
    ours.write_all(&[WAIT]).expect("write wait");
    let mut header = [0_u8; 4];
    ours.read_exact(&mut header).expect("report header");
    assert_eq!(header[0], 1, "the worker must confine before it reports");
    let mut cwd = vec![0_u8; usize::from(header[3])];
    ours.read_exact(&mut cwd).expect("cwd");
    assert_eq!(cwd, b"/", "the child working directory must be /");
    let mut fd_count = [0_u8; 1];
    ours.read_exact(&mut fd_count).expect("fd count");
    let mut descriptors = vec![0_u8; usize::from(fd_count[0])];
    ours.read_exact(&mut descriptors).expect("fds");
    assert_eq!(
        descriptors,
        [0, 1, 2],
        "the socket is descriptors 0, 1 and 2, got {descriptors:?}"
    );
    let mut env = Vec::with_capacity(usize::from(header[1]));
    for _ in 0..header[1] {
        let mut key_len = [0_u8; 1];
        ours.read_exact(&mut key_len).expect("key len");
        let mut key = vec![0_u8; usize::from(key_len[0])];
        ours.read_exact(&mut key).expect("key");
        let mut value_len = [0_u8; 1];
        ours.read_exact(&mut value_len).expect("value len");
        let mut value = vec![0_u8; usize::from(value_len[0])];
        ours.read_exact(&mut value).expect("value");
        env.push((key, value));
    }
    assert_empty_launch_environment(header[2], &env);

    let pid = child.id();
    // Linux 6.17 dropped the Dumpable: line; PR_SET_DUMPABLE 0 is the
    // reason other processes get EACCES on environ, cwd and fd.
    let hidden = [
        fs::read(format!("/proc/{pid}/environ")).expect_err("environ"),
        fs::read_link(format!("/proc/{pid}/cwd")).expect_err("cwd"),
        fs::read_dir(format!("/proc/{pid}/fd")).expect_err("fd"),
    ];
    for error in hidden {
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied, "{error}");
    }
    assert_eq!(
        proc_status_field(pid, "Threads").as_deref(),
        Some("1"),
        "the worker must be single-threaded"
    );
    assert_eq!(
        proc_status_field(pid, "NoNewPrivs").as_deref(),
        Some("1"),
        "no_new_privs must hold"
    );
    let limits = proc_limits(pid);
    assert_eq!(limits.as_limit, (536_870_912, 536_870_912));
    assert_eq!(limits.core, (0, 0));
    assert_eq!(limits.nofile, (32, 32));
    assert_eq!(limits.nproc, (0, 0));

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

/// Verifies: SEC-MED-022, SEC-TM-044
fn forking_is_killed_or_refused() {
    assert_killed_or_refused(FORK);
}

/// Verifies: SEC-MED-022, SEC-TM-044
fn ptrace_is_killed_or_refused() {
    assert_killed_or_refused(PTRACE);
}

/// Verifies: SEC-MED-022
fn linux_landlock_and_seccomp_hold() {
    mark_others_close_on_exec();
    let exe = std::env::current_exe().expect("exe");
    let mut command = Command::new(exe);
    command.env("GUNMETAL_COVER_KERNEL", "1");
    // `%c` mmaps the profile before Landlock. rustc only defines the bias
    // symbols with `-C llvm-args=-runtime-counter-relocation` (the jail
    // grants no profile directory).
    command.env_remove("LLVM_PROFILE_FILE");
    if let Some(dir) = std::env::var("LLVM_PROFILE_FILE").ok().and_then(|file| {
        file.split('%')
            .next()
            .and_then(|prefix| prefix.rsplit_once('/'))
            .map(|(dir, _)| dir.to_owned())
    }) {
        let profile = format!("{dir}/wp045-kernel-{}.profraw", process::id());
        let _ = File::create(&profile);
        command.env("LLVM_PROFILE_FILE", format!("{profile}%c"));
    }
    command.stdin(Stdio::piped());
    let mut child = command.spawn().expect("cover-kernel child");
    let pid = child.id();
    let mut mode = None;
    for _ in 0..100 {
        thread::sleep(Duration::from_millis(10));
        mode = seccomp_mode(pid);
        if mode == Some(2) {
            break;
        }
    }
    assert_eq!(
        mode,
        Some(2),
        "seccomp filter must be installed, got {mode:?}"
    );
    drop(child.stdin.take());
    let status = child.wait().expect("wait cover-kernel");
    assert!(
        status.success() || status.signal() == Some(31),
        "cover-kernel must exit or die of SIGSYS after applying the filter, got {status:?}"
    );
}

fn seccomp_mode(pid: u32) -> Option<u8> {
    let status = fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    status.lines().find_map(|line| {
        line.strip_prefix("Seccomp:")
            .and_then(|rest| rest.trim().parse().ok())
    })
}

fn proc_status_field(pid: u32, name: &str) -> Option<String> {
    let status = fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    let prefix = format!("{name}:");
    status.lines().find_map(|line| {
        line.strip_prefix(&prefix)
            .map(|rest| rest.trim().to_owned())
    })
}

struct ProcLimits {
    as_limit: (u64, u64),
    core: (u64, u64),
    nofile: (u64, u64),
    nproc: (u64, u64),
}

fn proc_limits(pid: u32) -> ProcLimits {
    let text = fs::read_to_string(format!("/proc/{pid}/limits")).expect("limits");
    let pair = |label: &str| {
        let line = text
            .lines()
            .find(|line| line.starts_with(label))
            .unwrap_or_else(|| panic!("missing {label}"));
        let mut words = line[label.len()..].split_whitespace();
        let soft = words.next().expect("soft");
        let hard = words.next().expect("hard");
        (parse_limit(soft), parse_limit(hard))
    };
    ProcLimits {
        as_limit: pair("Max address space"),
        core: pair("Max core file size"),
        nofile: pair("Max open files"),
        nproc: pair("Max processes"),
    }
}

fn parse_limit(word: &str) -> u64 {
    if word == "unlimited" {
        u64::MAX
    } else {
        word.parse().unwrap_or_else(|_| panic!("{word}"))
    }
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

    // The parent keeps serving: a fresh worker still answers.
    let (child, mut ours) = spawn_serve();
    ours.write_all(&[WAIT]).expect("write wait");
    let mut ready = [0_u8; 1];
    ours.read_exact(&mut ready).expect("parent still serves");
    assert_eq!(ready, [1], "a later worker still confines");
    assert!(matches!(
        child.stop().expect("stop"),
        Exit::Killed(Cause::Kill) | Exit::Code(_)
    ));
}
