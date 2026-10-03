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
use super::exit::Exit;
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

/// Coverage runtimes read `LLVM_PROFILE_FILE` before `main`. The child's
/// working directory is `/`, so a relative profile path is made absolute.
/// Production has none of these variables, so the environment stays empty
/// (SEC-OPS-014).
fn coverage_env(
    vars: Vec<(std::ffi::OsString, std::ffi::OsString)>,
    cwd: Option<&std::path::PathBuf>,
) -> Vec<(std::ffi::OsString, std::ffi::OsString)> {
    vars.into_iter()
        .filter(|(key, _)| key.to_string_lossy().contains("LLVM"))
        .map(|(key, value)| {
            if key != "LLVM_PROFILE_FILE" {
                return (key, value);
            }
            let mut path = value.to_string_lossy().into_owned();
            if !path.starts_with('/') {
                if let Some(cwd) = cwd {
                    path = format!("{}{}{path}", cwd.display(), std::path::MAIN_SEPARATOR);
                }
            }
            (key, path.into())
        })
        .collect()
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
/// descriptor the standard library opens is close-on-exec, so nothing
/// else of this process's is inherited; a descriptor the process itself
/// inherited without that flag is accounted for by the worker's
/// confinement.
///
/// # Errors
///
/// A [`SpawnError`] when the socket cannot be duplicated or the program
/// cannot be started.
pub fn launch(program: Program, args: TypedArgs, fds: Inherited) -> Result<Child, SpawnError> {
    spawn(program.executable(), args, fds)
}

fn empty_argument_error(argv: [&str; 3]) -> Result<(), SpawnError> {
    if argv.iter().any(|word| word.is_empty()) {
        Err(SpawnError::Spawn(io::Error::new(
            io::ErrorKind::InvalidInput,
            "a worker argument was empty",
        )))
    } else {
        Ok(())
    }
}

fn spawn(
    executable: impl AsRef<std::ffi::OsStr>,
    args: TypedArgs,
    fds: Inherited,
) -> Result<Child, SpawnError> {
    spawn_argv(executable, args.argv(), fds)
}

fn spawn_argv(
    executable: impl AsRef<std::ffi::OsStr>,
    argv: [&'static str; 3],
    fds: Inherited,
) -> Result<Child, SpawnError> {
    empty_argument_error(argv)?;
    let input = OwnedFd::from(fds.child_end);
    let output = input.try_clone();
    let errors = input.try_clone();
    output
        .and_then(|output| errors.map(|errors| (output, errors)))
        .map_err(SpawnError::Descriptor)
        .and_then(|(output, errors)| {
            #[expect(
                clippy::disallowed_methods,
                reason = "the sandbox launcher is the one door that starts a process (SEC-MED-063)"
            )]
            let mut command = Command::new(executable);
            command.args(argv).env_clear();
            for (key, value) in coverage_env(
                std::env::vars_os().collect(),
                std::env::current_dir().ok().as_ref(),
            ) {
                command.env(key, value);
            }
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
    use super::SpawnError;
    use std::io;

    #[test]
    fn coverage_variables_are_kept_and_profile_paths_are_made_absolute() {
        use std::ffi::OsString;
        use std::path::PathBuf;

        let work = PathBuf::from("/work");
        assert_eq!(
            super::coverage_env(
                vec![
                    (OsString::from("HOME"), OsString::from("/home/x")),
                    (
                        OsString::from("LLVM_PROFILE_FILE"),
                        OsString::from("p.profraw")
                    ),
                    (
                        OsString::from("__LLVM_PROFILE_RT_INIT_ONCE"),
                        OsString::from("1")
                    ),
                ],
                Some(&work),
            ),
            [
                (
                    OsString::from("LLVM_PROFILE_FILE"),
                    OsString::from("/work/p.profraw")
                ),
                (
                    OsString::from("__LLVM_PROFILE_RT_INIT_ONCE"),
                    OsString::from("1")
                ),
            ]
        );
        assert_eq!(
            super::coverage_env(
                vec![(
                    OsString::from("LLVM_PROFILE_FILE"),
                    OsString::from("/abs/p.profraw")
                )],
                Some(&work),
            ),
            [(
                OsString::from("LLVM_PROFILE_FILE"),
                OsString::from("/abs/p.profraw")
            )]
        );
        assert_eq!(
            super::coverage_env(
                vec![(
                    OsString::from("LLVM_PROFILE_FILE"),
                    OsString::from("p.profraw")
                )],
                None,
            ),
            [(
                OsString::from("LLVM_PROFILE_FILE"),
                OsString::from("p.profraw")
            )]
        );
        assert_eq!(
            super::coverage_env(vec![(OsString::from("HOME"), OsString::from("/"))], None),
            []
        );
        assert_eq!(
            super::coverage_env(
                vec![(
                    OsString::from("LLVM_PROFILE_FILE"),
                    OsString::from("/abs/p.profraw")
                )],
                None,
            ),
            [(
                OsString::from("LLVM_PROFILE_FILE"),
                OsString::from("/abs/p.profraw")
            )]
        );
    }

    #[test]
    fn a_missing_executable_is_a_spawn_error() {
        use super::{Inherited, spawn};
        use crate::sandbox::args::{Job, TypedArgs};
        use crate::sandbox::limits::Profile;

        let (fds, _ours) = Inherited::pair().unwrap();
        let error = spawn(
            "no-such-gunmetal-worker-045",
            TypedArgs::new(Job::SelfTest, Profile::Scan),
            fds,
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .starts_with("the worker could not be started:"),
            "{error}"
        );
    }

    #[test]
    fn empty_argument_words_are_refused() {
        let error = super::empty_argument_error(["", "serve", "scan"]).unwrap_err();
        assert_eq!(
            error.to_string(),
            "the worker could not be started: a worker argument was empty"
        );
        assert!(super::empty_argument_error(["--gunmetal-worker", "serve", "scan"]).is_ok());
        assert!(super::empty_argument_error(["--gunmetal-worker", "", "scan"]).is_err());
        assert!(super::empty_argument_error(["--gunmetal-worker", "serve", ""]).is_err());
        let (fds, _ours) = super::Inherited::pair().unwrap();
        assert_eq!(
            super::spawn_argv("no-such-gunmetal-worker-045", ["", "serve", "scan"], fds)
                .unwrap_err()
                .to_string(),
            "the worker could not be started: a worker argument was empty"
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
        use std::io::{Read, Write};

        let (inherited, mut ours) = super::Inherited::pair().unwrap();
        let mut child_end = inherited.child_end;
        ours.write_all(&[0x5a]).unwrap();
        let mut octet = [0_u8; 1];
        child_end.read_exact(&mut octet).unwrap();
        assert_eq!(octet, [0x5a]);
    }

    #[test]
    fn launch_starts_the_running_executable() {
        use super::{Inherited, launch};
        use crate::sandbox::args::{Job, TypedArgs};
        use crate::sandbox::limits::Profile;
        use crate::sandbox::programs::Program;

        let (fds, _ours) = Inherited::pair().unwrap();
        let child = launch(
            Program::Worker,
            TypedArgs::new(Job::SelfTest, Profile::Scan),
            fds,
        )
        .unwrap();
        let first = child.id();
        assert_ne!(first, 0);
        let (fds, _ours) = Inherited::pair().unwrap();
        let other = launch(
            Program::Worker,
            TypedArgs::new(Job::SelfTest, Profile::Scan),
            fds,
        )
        .unwrap();
        assert_ne!(other.id(), first);
        let _ = child.wait().unwrap();
        let _ = other.wait().unwrap();
    }

    #[test]
    fn stop_and_drop_reap_the_child() {
        use super::{Inherited, launch};
        use crate::sandbox::args::{Job, TypedArgs};
        use crate::sandbox::exit::{Cause, Exit};
        use crate::sandbox::limits::Profile;
        use crate::sandbox::programs::Program;

        let (fds, _ours) = Inherited::pair().unwrap();
        let child = launch(
            Program::Worker,
            TypedArgs::new(Job::Serve, Profile::Scan),
            fds,
        )
        .unwrap();
        assert_eq!(child.stop().unwrap(), Exit::Killed(Cause::Kill));

        let (fds, _ours) = Inherited::pair().unwrap();
        let leaked = launch(
            Program::Worker,
            TypedArgs::new(Job::Serve, Profile::Scan),
            fds,
        )
        .unwrap();
        let pid = rustix::process::Pid::from_raw(i32::try_from(leaked.id()).unwrap()).unwrap();
        drop(leaked);
        assert!(
            rustix::process::test_kill_process(pid).is_err(),
            "Drop must stop the child"
        );
    }
}
