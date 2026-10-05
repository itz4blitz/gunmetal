//! The sandbox launcher: the one place in Gunmetal where a process starts
//! (SEC-HIS-020, SEC-MED-063).
//!
//! What it starts comes from the closed list in [`programs`](super::programs),
//! what it passes comes from [`TypedArgs`], and neither can carry text from
//! outside. The child gets an empty environment, so no secret reaches it
//! as a variable or an argument (SEC-OPS-014). Its only connection is one
//! end of a socket pair, as its descriptors 0, 1 and 2; there is no TCP,
//! not even on loopback (SEC-STD-040), and no shell.

use super::args::TypedArgs;
use super::descriptors::mark_from;
use super::exit::Exit;
use super::kernel::{Kernel, Linux};
use super::programs::Program;
use std::fmt;
use std::io;
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

/// What a launched program inherits: its end of a socket pair, and
/// nothing else.
///
/// The only way to make one is [`Inherited::pair`], so the launcher cannot
/// be handed a TCP socket, a file or a listening socket.
#[derive(Debug)]
pub struct Inherited {
    child_end: UnixStream,
}

impl Inherited {
    /// Makes a socket pair: the end the child inherits, and the end the
    /// caller keeps to talk to it.
    ///
    /// # Errors
    ///
    /// The operating system's error when it cannot make the pair.
    pub fn pair() -> io::Result<(Self, UnixStream)> {
        UnixStream::pair().map(|(ours, child_end)| (Self { child_end }, ours))
    }
}

/// Why the launcher could not start a program.
#[derive(Debug)]
pub enum SpawnError {
    /// The child's end of the socket pair could not be duplicated onto
    /// its three standard descriptors.
    Descriptor(io::Error),
    /// The operating system refused to start the program.
    Spawn(io::Error),
}

impl fmt::Display for SpawnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Descriptor(error) => {
                write!(f, "the worker's socket could not be shared: {error}")
            }
            Self::Spawn(error) => write!(f, "the worker could not be started: {error}"),
        }
    }
}

impl std::error::Error for SpawnError {}

/// A launched program.
///
/// Dropping it kills the process and reaps it, so a worker never outlives
/// the value that owns it.
#[derive(Debug)]
pub struct Child {
    process: std::process::Child,
}

impl Child {
    /// The process ID.
    #[must_use]
    pub fn id(&self) -> u32 {
        self.process.id()
    }

    /// Waits for the process to end and says how it ended.
    ///
    /// # Errors
    ///
    /// The operating system's error when the wait fails.
    pub fn wait(mut self) -> io::Result<Exit> {
        self.process
            .wait()
            .map(|status| Exit::of_raw(status.into_raw()))
    }

    /// Kills the process and says how it ended: [`Exit::Killed`] with
    /// [`Cause::Kill`](super::exit::Cause::Kill) unless it had already
    /// ended by itself.
    ///
    /// # Errors
    ///
    /// The operating system's error when the kill or the wait fails.
    pub fn stop(mut self) -> io::Result<Exit> {
        self.process
            .kill()
            .and_then(|()| self.process.wait())
            .map(|status| Exit::of_raw(status.into_raw()))
    }
}

impl Drop for Child {
    fn drop(&mut self) {
        // Both fail only when the process is already gone and reaped,
        // which is the state this is after.
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

/// Starts `program` with `args`, connected only through `fds`.
///
/// The child runs with an empty environment, the root directory as its
/// working directory, and the socket as descriptors 0, 1 and 2. Every
/// extra descriptor in this process is marked close-on-exec first, so a
/// descriptor this process inherited without that flag (a log file, a
/// coverage handle, a parent application's socket) is not passed on.
/// Confinement then refuses if any extra still remains.
///
/// # Errors
///
/// A [`SpawnError`] when the socket cannot be duplicated or the program
/// cannot be started.
pub fn launch(program: Program, args: TypedArgs, fds: Inherited) -> Result<Child, SpawnError> {
    spawn(program.executable(), args, fds)
}

/// Marks every descriptor numbered 3 and above close-on-exec.
///
/// A child started by the standard library's `Command` still inherits
/// the descriptors that lack that flag. With it set, a child started
/// afterwards receives only the descriptors `Command` is told to pass
/// (SEC-MED-022). When `/proc/self/fd` cannot be listed nothing is
/// marked, and confinement refuses the worker if an extra reached it.
fn mark_others_close_on_exec() {
    Linux
        .descriptors()
        .iter()
        .for_each(|open| mark_from(3, open));
}

/// Starts the executable at `executable`. [`launch`] is the only caller
/// outside the tests, and passes a path from the closed program list.
fn spawn(executable: &str, args: TypedArgs, fds: Inherited) -> Result<Child, SpawnError> {
    let input = OwnedFd::from(fds.child_end);
    let output = input.try_clone();
    let errors = input.try_clone();
    output
        .and_then(|output| errors.map(|errors| (output, errors)))
        .map_err(SpawnError::Descriptor)
        .and_then(|(output, errors)| {
            mark_others_close_on_exec();
            #[expect(
                clippy::disallowed_methods,
                reason = "the sandbox launcher is the one door that starts a process (SEC-MED-063)"
            )]
            let mut command = Command::new(executable);
            command.args(args.argv()).env_clear();
            command
                .current_dir("/")
                .stdin(input)
                .stdout(output)
                .stderr(errors)
                .spawn()
                .map(|process| Child { process })
                .map_err(SpawnError::Spawn)
        })
}

#[cfg(test)]
mod tests {
    use super::{Inherited, SpawnError, mark_others_close_on_exec, spawn};
    use crate::sandbox::args::{Job, TypedArgs};
    use crate::sandbox::descriptors::SERIAL;
    use crate::sandbox::limits::Profile;
    use std::io::{self, Read, Write};

    #[test]
    fn a_missing_executable_is_a_spawn_error() {
        let _serial = SERIAL.lock().unwrap();
        let (fds, _ours) = Inherited::pair().unwrap();
        let error = spawn(
            "/no-such-gunmetal-worker-045",
            TypedArgs::new(Job::SelfTest, Profile::Scan),
            fds,
        )
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "the worker could not be started: No such file or directory (os error 2)"
        );
    }

    #[test]
    fn spawn_errors_say_what_failed() {
        assert_eq!(
            SpawnError::Descriptor(io::Error::other("no descriptors")).to_string(),
            "the worker's socket could not be shared: no descriptors"
        );
        assert_eq!(
            SpawnError::Spawn(io::Error::other("no such file")).to_string(),
            "the worker could not be started: no such file"
        );
        let error: &dyn std::error::Error = &SpawnError::Spawn(io::Error::other("no such file"));
        assert_eq!(
            error.to_string(),
            "the worker could not be started: no such file"
        );
    }

    #[test]
    fn a_socket_pair_connects_the_two_ends() {
        let (inherited, mut ours) = Inherited::pair().unwrap();
        let mut child_end = inherited.child_end;
        ours.write_all(&[0x5a]).unwrap();
        let mut octet = [0_u8; 1];
        child_end.read_exact(&mut octet).unwrap();
        assert_eq!(octet, [0x5a]);
    }

    #[test]
    fn extra_descriptors_are_marked_close_on_exec() {
        use rustix::io::{FdFlags, fcntl_getfd, fcntl_setfd};
        use std::os::unix::net::UnixStream;

        let _serial = SERIAL.lock().unwrap();
        let (probe, _peer) = UnixStream::pair().unwrap();
        fcntl_setfd(&probe, FdFlags::empty()).unwrap();
        mark_others_close_on_exec();
        assert_eq!(fcntl_getfd(&probe).unwrap(), FdFlags::CLOEXEC);
    }
}
