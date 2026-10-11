//! Integration tests of the worker protocol with a real worker process.
//!
//! This file is its own executable (`harness = false`), as the sandbox's
//! tests are, so that the launcher can start it again as the worker:
//! `/proc/self/exe` with the hidden worker arguments enters the worker
//! path. No extra binary ships.
//!
//! The worker here runs the crate's own loop, [`serve`], with a stand-in
//! job: a small sans-I/O parser that asks for two windows of the request's
//! first file, its first eight octets and its last eight, and answers with
//! them. The real jobs are another package's (WP-079).
//!
//! A worker that is told to confine itself says whether the seccomp filter
//! now binds it, from what [`confine`] returned, before it reads a request.
//! With the filter, a worker cannot serve yet. The allowlist
//! (`sandbox/syscalls.rs`) is the minimal one the sandbox shipped with: it
//! lists `read` and `write` and none of `recvmsg`, `fstat` and `pread64`,
//! which receiving a descriptor and reading through it need, so the kernel
//! kills the worker at its first `recvmsg`. The tests assert that outcome
//! where the filter is in force, and the finished job where it is not.
//! Every test therefore runs once against the kernel as it is and again
//! against kernels with the Landlock calls, the `seccomp` call, or both
//! taken away, which are made here the way the sandbox's tests make them:
//! this executable starts itself again with a word from [`EMULATED`], and
//! that process first installs a filter under which the kernel answers
//! those calls with `ENOSYS`.
//!
//! No test here reads what the kernel offers. The one fact a test needs,
//! whether the filter binds the worker, is the worker's own report, and
//! what follows shows whether the report was true.
#![expect(
    clippy::disallowed_methods,
    reason = "the test opens its in-memory files again through /proc/self/fd, a fixed path, and starts itself again with Command for the kernels with calls taken away (SEC-MED-020, SEC-MED-024)"
)]

use gunmetal_core::parse::{Limits, ReadRequest, SansIo, Step, Window};
use gunmetal_worker::host::{HostError, serve, serve_one};
use gunmetal_worker::ipc::{
    AnswerError, ChannelError, Job, Received, Refusal, Revalidate, SendError, read_answer, send,
};
use gunmetal_worker::sandbox::{
    Cause, Child, Exit, Inherited, Job as Start, Profile, Program, TypedArgs, confine, launch,
};
use rustix::fs::{CWD, MemfdFlags, Mode, OFlags, memfd_create, openat};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{self, Read, Write};
use std::net::Shutdown;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};
use std::os::unix::net::UnixStream;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::process::{self, Command};
use std::time::Duration;

/// Serve without confining.
const PLAIN: u8 = b'p';
/// Confine, then serve.
const CONFINED: u8 = b'c';

/// How long the server's end waits for a worker before a read or a write
/// fails, so that a worker that never answers fails a test instead of
/// hanging it.
const WAIT: Duration = Duration::from_secs(10);

/// The words that start this executable again against a kernel with
/// something taken away, and for each whether the Landlock calls and the
/// `seccomp` call are what is taken.
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const EMULATED: [(&str, bool, bool); 3] = [
    ("--kernel-without-landlock", true, false),
    ("--kernel-without-seccomp", false, true),
    ("--kernel-without-landlock-or-seccomp", true, true),
];
/// An architecture seccompiler cannot write a filter for cannot take a
/// call away either, so only the kernel as it is runs there.
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
const EMULATED: [(&str, bool, bool); 0] = [];

/// One test: its name and its body, which is told what was taken from the
/// kernel.
type Test = (&'static str, fn(&Kernel));

const TESTS: [Test; 4] = [
    (
        "a_worker_process_reads_two_windows_of_a_file_passed_by_descriptor",
        a_worker_process_reads_two_windows_of_a_file_passed_by_descriptor,
    ),
    (
        "a_descriptor_the_worker_cannot_read_is_reported_and_the_worker_goes_on",
        a_descriptor_the_worker_cannot_read_is_reported_and_the_worker_goes_on,
    ),
    (
        "a_request_the_worker_cannot_read_is_refused_and_the_worker_ends",
        a_request_the_worker_cannot_read_is_refused_and_the_worker_ends,
    ),
    (
        "a_confined_worker_serves_the_file_or_is_killed_at_its_first_call_off_the_allowlist",
        a_confined_worker_serves_the_file_or_is_killed_at_its_first_call_off_the_allowlist,
    ),
];

pub fn run() {
    if let Some(args) = TypedArgs::from_argv(std::env::args_os()) {
        worker(args);
    }
    let word = std::env::args().nth(1).unwrap_or_default();
    let emulated = EMULATED.iter().find(|(known, _, _)| *known == word);
    let kernel = Kernel {
        without_seccomp: emulated
            .is_some_and(|&(_, landlock, seccomp)| take_away(landlock, seccomp)),
    };
    eprintln!("\nkernel: {kernel:?}");
    let mut failed = 0_usize;
    for (name, test) in TESTS {
        eprint!("test {name} ... ");
        if catch_unwind(AssertUnwindSafe(|| test(&kernel))).is_ok() {
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
        for (word, _, _) in EMULATED {
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

// The kernel a worker meets.

/// What this process took from the kernel its workers meet.
#[derive(Debug)]
struct Kernel {
    /// The `seccomp` call answers `ENOSYS`, so no worker started from here
    /// can install its filter.
    without_seccomp: bool,
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

/// Makes the kernel answer `ENOSYS` to the Landlock calls, the `seccomp`
/// call or both, for this process and every process it starts, from now
/// on. One filter takes them all: once the `seccomp` call is gone, no
/// further filter can be installed. Returns whether the `seccomp` call was
/// taken.
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
fn take_away(landlock: bool, seccomp: bool) -> bool {
    use seccompiler::{BpfProgram, SeccompAction, SeccompFilter};

    let landlock_calls: &[i64] = if landlock { &LANDLOCK_CALLS } else { &[] };
    let seccomp_calls: &[i64] = if seccomp { &[SECCOMP_CALL.0] } else { &[] };
    let rules = landlock_calls
        .iter()
        .chain(seccomp_calls)
        .map(|&call| (call, Vec::new()))
        .collect();
    let filter = SeccompFilter::new(
        rules,
        SeccompAction::Allow,
        SeccompAction::Errno(ENOSYS),
        SECCOMP_CALL.1,
    )
    .expect("a filter");
    let program = BpfProgram::try_from(filter).expect("a compiled filter");
    seccompiler::apply_filter(&program).expect("install the filter");
    seccomp
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
fn take_away(_landlock: bool, _seccomp: bool) -> bool {
    false
}

// The worker side: what the test executable does when the launcher starts
// it again.

/// A parser that asks for the first eight octets of its file and then the
/// last eight, and returns the sixteen.
#[derive(Default)]
struct Ends {
    /// The octets it has been given so far.
    octets: Vec<u8>,
    /// How many times it has been resumed.
    resumes: u8,
}

impl SansIo for Ends {
    type Output = Vec<u8>;

    fn resume(&mut self, window: Window<'_>) -> Step<Vec<u8>> {
        self.octets.extend_from_slice(window.bytes);
        self.resumes = self.resumes.saturating_add(1);
        match self.resumes {
            1 => Step::Need(ReadRequest { offset: 0, len: 8 }),
            2 => Step::Need(ReadRequest {
                offset: window.file_len.saturating_sub(8),
                len: 8,
            }),
            _ => Step::Done(std::mem::take(&mut self.octets)),
        }
    }
}

/// What the stand-in job answers.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum Reply {
    /// The first eight octets of the file and its last eight.
    Ends(Vec<u8>),
    /// The system refused to read the file, with this error number.
    Unreadable(Option<i32>),
    /// The host loop stopped the parser for another reason.
    Stopped,
}

impl Revalidate for Reply {
    type Checked = Self;
    type Invalid = usize;

    /// Two windows of eight octets are sixteen; any other number is not
    /// what was asked for.
    fn revalidate(self) -> Result<Self, usize> {
        match self {
            Self::Ends(octets) if octets.len() != 16 => Err(octets.len()),
            checked => Ok(checked),
        }
    }
}

/// The stand-in job: the two windows of the request's first file, read
/// through the host loop. A request without a file is refused.
fn ends(received: Received) -> Result<Reply, Refusal> {
    // Held until the answer is made, then closed.
    let files = received.files;
    let Some(file) = files.first() else {
        return Err(Refusal::Unsupported);
    };
    Ok(
        match serve_one(file.as_fd(), Ends::default(), &Limits::DEFAULT) {
            Ok(octets) => Reply::Ends(octets),
            Err(HostError::Read { errno, .. }) => Reply::Unreadable(errno),
            Err(_) => Reply::Stopped,
        },
    )
}

/// Reads the hook, confines the process if the hook says so, reports
/// whether the seccomp filter binds it, and serves requests until the
/// server closes the socket.
fn worker(args: TypedArgs) -> ! {
    let stdin = io::stdin();
    let socket = stdin.as_fd();
    let mut hook = [0_u8; 1];
    let read = rustix::io::read(socket, hook.as_mut_slice()).expect("read the hook");
    assert_eq!(read, 1, "one octet of hook");
    let filtered =
        hook[0] == CONFINED && confine(args.profile()).expect("confine the worker").seccomp;
    let mut out = io::stdout();
    out.write_all(&[u8::from(filtered)]).expect("report");
    out.flush().expect("flush the report");
    let served = serve(socket, &mut out, &Limits::DEFAULT, ends);
    process::exit(i32::from(served.is_err()))
}

// The test side.

/// Starts a worker and gives it `hook`. Returns the worker, the server's
/// end of its socket, and whether the worker says the seccomp filter binds
/// it.
fn start(hook: u8) -> (Child, UnixStream, bool) {
    let (fds, server) = Inherited::pair().expect("socket pair");
    server.set_read_timeout(Some(WAIT)).expect("read timeout");
    server.set_write_timeout(Some(WAIT)).expect("write timeout");
    let child = launch(
        Program::Worker,
        TypedArgs::new(Start::Serve, Profile::Scan),
        fds,
    )
    .expect("launch");
    let mut channel = &server;
    channel.write_all(&[hook]).expect("write the hook");
    let mut report = [0_u8; 1];
    channel
        .read_exact(&mut report)
        .expect("the worker's report");
    (child, server, report[0] == 1)
}

/// A new file in memory that holds `octets`, opened again with `flags`:
/// read-only, as the server's path rules open a library file, or `O_PATH`,
/// which names the file and cannot read it.
fn file_of(octets: &[u8], flags: OFlags) -> OwnedFd {
    let memory = memfd_create("gunmetal-worker-ipc", MemfdFlags::CLOEXEC).expect("a file");
    let mut writer = File::from(memory);
    writer.write_all(octets).expect("fill the file");
    openat(
        CWD,
        format!("/proc/self/fd/{}", writer.as_raw_fd()),
        flags | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .expect("open the file again")
}

/// Sends the worker behind `server` a request with `files`, and returns
/// its answer once the server has read and checked it.
fn ask(server: &UnixStream, files: &[BorrowedFd<'_>]) -> Result<Reply, AnswerError<usize>> {
    assert_eq!(send(server, Job::Probe { hint: None }, files), Ok(()));
    read_answer::<Reply>(server, &Limits::DEFAULT)
}

/// The file's octets reach the parser in another process, and only
/// through the descriptor passed over the socket pair the launcher made:
/// the request names no path, and the worker's answer comes back over the
/// same socket. The sixteen octets returned are the file's first eight
/// and its last eight.
///
/// Verifies: SEC-MED-018, SEC-MED-020, SEC-STD-040
fn a_worker_process_reads_two_windows_of_a_file_passed_by_descriptor(_kernel: &Kernel) {
    let (child, server, filtered) = start(PLAIN);
    assert!(!filtered);
    let counting: Vec<u8> = (0..64).collect();
    let file = file_of(&counting, OFlags::RDONLY);
    assert_eq!(
        ask(&server, &[file.as_fd()]),
        Ok(Reply::Ends(vec![
            0, 1, 2, 3, 4, 5, 6, 7, 56, 57, 58, 59, 60, 61, 62, 63
        ]))
    );
    // The same worker serves the next file, and refuses a request that
    // comes with no file.
    let letters = file_of(b"abcdefghijklmnopqrstuvwxyz", OFlags::RDONLY);
    assert_eq!(
        ask(&server, &[letters.as_fd()]),
        Ok(Reply::Ends(b"abcdefghstuvwxyz".to_vec()))
    );
    assert_eq!(
        ask(&server, &[]),
        Err(AnswerError::Refused(Refusal::Unsupported))
    );
    drop(server);
    assert_eq!(child.wait().expect("wait"), Exit::Code(0));
}

/// The descriptor is opened with `O_PATH`: the worker can read its length
/// and not its octets, and a read fails with `EBADF`, error number 9, as a
/// read of a closed descriptor does. The worker says so in its answer and
/// serves the next request.
fn a_descriptor_the_worker_cannot_read_is_reported_and_the_worker_goes_on(_kernel: &Kernel) {
    let (child, server, _) = start(PLAIN);
    let unreadable = file_of(&[7; 32], OFlags::PATH);
    assert_eq!(
        ask(&server, &[unreadable.as_fd()]),
        Ok(Reply::Unreadable(Some(9)))
    );
    let readable = file_of(&[7; 32], OFlags::RDONLY);
    assert_eq!(
        ask(&server, &[readable.as_fd()]),
        Ok(Reply::Ends(vec![7; 16]))
    );
    drop(server);
    assert_eq!(child.wait().expect("wait"), Exit::Code(0));
}

/// The length field declares 1,048,577 octets, one over the cap on a
/// request. The worker answers that it could not read the request and
/// ends with status 1, while the server's end is still open.
fn a_request_the_worker_cannot_read_is_refused_and_the_worker_ends(_kernel: &Kernel) {
    let (child, server, _) = start(PLAIN);
    let mut channel = &server;
    channel
        .write_all(&[0x01, 0x00, 0x10, 0x00])
        .expect("write the length");
    assert_eq!(
        read_answer::<Reply>(&server, &Limits::DEFAULT),
        Err(AnswerError::Refused(Refusal::Request))
    );
    assert_eq!(child.wait().expect("wait"), Exit::Code(1));
}

/// A worker that has confined itself holds no path and no way to open
/// one; the file still reaches it as a descriptor. Where the seccomp
/// filter is not in force, the confined worker serves the request.
///
/// Where it is, the worker is killed at its first `recvmsg`, which the
/// allowlist does not list yet, before any request is sent. The server's
/// side then gets an error from the read and from the send, not a hang,
/// and goes on: a fresh worker does the job.
///
/// Neither branch waits for a worker without a bound. Under the filter
/// the first thing the server does is read, which its socket's `WAIT`
/// bounds: a worker that was killed has closed its end, and one that is
/// still waiting for a request leaves the read to time out, which fails
/// the test. The read reports the end of the stream only once the
/// worker's end is closed, so the send after it meets a worker that is
/// gone. The server's writing side is shut down before the wait for the
/// worker's exit all the same, so that, whatever comes before it, the
/// wait is never for a worker that waits for a request: such a worker
/// would read the end of its socket and exit by itself. Once the
/// allowlist lists the three calls, this branch fails within `WAIT` and
/// is to be rewritten; it does not hang.
///
/// SEC-MED-020 is supported here, not proved: only where the filter does
/// not bind the worker does a confined worker read the file through its
/// descriptor. Under the filter it is killed before a file reaches it,
/// and the fresh worker that does the job has not confined itself. The
/// first test of this file proves it for a worker that serves.
///
/// Verifies: SEC-MED-018
/// Supports: SEC-MED-020
fn a_confined_worker_serves_the_file_or_is_killed_at_its_first_call_off_the_allowlist(
    kernel: &Kernel,
) {
    let expected = Ok(Reply::Ends(vec![
        100, 101, 102, 103, 104, 105, 106, 107, 156, 157, 158, 159, 160, 161, 162, 163,
    ]));
    let counting: Vec<u8> = (100..164).collect();
    let file = file_of(&counting, OFlags::RDONLY);
    let (child, server, filtered) = start(CONFINED);
    // Which branch this kernel takes, for whoever reads the log.
    eprint!("(the filter binds the confined worker: {filtered}) ");
    if kernel.without_seccomp {
        assert!(!filtered, "no filter can be installed on this kernel");
    }
    if filtered {
        assert_eq!(
            read_answer::<Reply>(&server, &Limits::DEFAULT),
            Err(AnswerError::Channel(ChannelError::Closed)),
            "a worker under the filter is killed at its first recvmsg; if the allowlist \
             now lets it read a request, this branch is out of date"
        );
        assert_eq!(
            send(&server, Job::Probe { hint: None }, &[file.as_fd()]),
            Err(SendError::Io(io::ErrorKind::BrokenPipe))
        );
        server
            .shutdown(Shutdown::Write)
            .expect("shut the server's writing side down");
        assert_eq!(
            child.wait().expect("wait"),
            Exit::Killed(Cause::ForbiddenCall)
        );
    } else {
        assert_eq!(ask(&server, &[file.as_fd()]), expected);
        drop(server);
        assert_eq!(child.wait().expect("wait"), Exit::Code(0));
    }
    let (child, server, _) = start(PLAIN);
    assert_eq!(ask(&server, &[file.as_fd()]), expected);
    drop(server);
    assert_eq!(child.wait().expect("wait"), Exit::Code(0));
}
