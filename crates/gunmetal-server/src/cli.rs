//! The `gunmetal` command line: its subcommands, parsed by hand, and the
//! table that sends each to the module that runs it.
//!
//! - **No secret is an argument.** Arguments are visible to every user of
//!   the host, in the process list and the shell history. [`FLAGS`] lists
//!   every flag the parser accepts, none takes a secret, and anything else
//!   is refused (SEC-OPS-014). A refusal never repeats the argument it
//!   refuses: a secret typed there by mistake must not go on to the
//!   console and the journal behind it. No flag turns authentication off
//!   either (SEC-HIS-004).
//! - **Exit codes are documented.** [`Exit`] gives each kind of refusal its
//!   own code, from `sysexits.h`, so a service manager or a script can tell
//!   a privileged start from a bad configuration (SEC-OPS-053).
//!
//! This file is a registry. [`SUBCOMMANDS`] names every subcommand of the
//! binary, including those later packages build, so the command line is
//! parsed in one place; a package that builds one replaces its line in
//! [`dispatch`] with a call into its own module.

use std::ffi::OsString;
use std::io::Write;
use std::sync::Arc;

use gunmetal_fs::host::HostFacts;

use crate::app::{AppState, Host, StartError, VERSION};
use crate::clock::SystemClock;
use crate::config::{ConfigError, Env};
use crate::datadir;
use crate::host::{self, Privileges};

/// What the binary was asked to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `serve`: run the server.
    Serve,
    /// `doctor`: check the installation (WP-116).
    Doctor,
    /// `admin recover`: recover administrator access (WP-106).
    AdminRecover,
    /// `migrate`: migrate the durable state (WP-095).
    Migrate,
    /// `rebuild`: rebuild the cache (WP-095).
    Rebuild,
    /// `snapshot restore`: go back to a snapshot (WP-095).
    SnapshotRestore,
    /// `restore`: restore a backup (WP-109).
    Restore,
    /// `service`: install or remove the service (WP-121).
    Service,
    /// `audit verify`: verify the audit log (WP-069).
    AuditVerify,
    /// `keys rotate`: rotate the keys (WP-106).
    KeysRotate,
    /// `worker`: the scan worker's entry, which the server starts and the
    /// help does not list (WP-061).
    Worker,
    /// `--help`.
    Help,
    /// `--version`.
    Version,
}

/// Every subcommand, by the words that name it.
pub const SUBCOMMANDS: [(&[&str], Action); 11] = [
    (&["serve"], Action::Serve),
    (&["doctor"], Action::Doctor),
    (&["admin", "recover"], Action::AdminRecover),
    (&["migrate"], Action::Migrate),
    (&["rebuild"], Action::Rebuild),
    (&["snapshot", "restore"], Action::SnapshotRestore),
    (&["restore"], Action::Restore),
    (&["service"], Action::Service),
    (&["audit", "verify"], Action::AuditVerify),
    (&["keys", "rotate"], Action::KeysRotate),
    (&["worker"], Action::Worker),
];

/// The flag that names the data directory.
const DATA_DIR: &str = "--data-dir";

/// Every flag the parser accepts. `--data-dir` takes the next argument as
/// its value; the others take none.
pub const FLAGS: [&str; 5] = [DATA_DIR, "--help", "-h", "--version", "-V"];

/// A parsed command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    /// What to do.
    pub action: Action,
    /// The data directory `--data-dir` names, if it was given.
    pub data_dir: Option<OsString>,
}

/// Why the command line was refused. No variant holds an argument's text:
/// what was typed may be a secret, so it is neither stored nor printed
/// (SEC-OPS-014). The words of an incomplete command are the one exception,
/// because they are words of [`SUBCOMMANDS`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsageError {
    /// No subcommand was given.
    NoCommand,
    /// The words name no subcommand.
    UnknownCommand,
    /// The words are the start of a subcommand that needs another word.
    IncompleteCommand(String),
    /// A subcommand was followed by an argument it does not take.
    UnexpectedArgument,
    /// An argument starts with `-` and is not one of [`FLAGS`].
    UnknownFlag,
    /// `--data-dir` was the last argument, or its value was empty.
    MissingValue,
    /// `--data-dir` was given twice.
    RepeatedFlag,
}

impl UsageError {
    /// What to tell the person who typed the command.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::NoCommand => "No command given.".to_owned(),
            Self::UnknownCommand => "That is not a gunmetal command.".to_owned(),
            Self::IncompleteCommand(words) => {
                format!("{words:?} is not a whole command; it needs another word.")
            }
            Self::UnexpectedArgument => {
                "The command was followed by an argument it does not take.".to_owned()
            }
            Self::UnknownFlag => "An option was given that gunmetal does not have. Options take no secrets: those come from files (see the documentation on secrets).".to_owned(),
            Self::MissingValue => "--data-dir needs a directory after it.".to_owned(),
            Self::RepeatedFlag => "--data-dir was given twice.".to_owned(),
        }
    }
}

/// The subcommand `words` name.
fn action(words: &[&str]) -> Result<Action, UsageError> {
    let exact = SUBCOMMANDS.iter().find(|(path, _)| *path == words);
    let extra = SUBCOMMANDS.iter().any(|(path, _)| words.starts_with(path));
    let partial = SUBCOMMANDS.iter().any(|(path, _)| path.starts_with(words));
    match (exact, extra) {
        (Some((_, action)), _) => Ok(*action),
        (None, true) => Err(UsageError::UnexpectedArgument),
        (None, false) if words.is_empty() => Err(UsageError::NoCommand),
        (None, false) if partial => Err(UsageError::IncompleteCommand(words.join(" "))),
        (None, false) => Err(UsageError::UnknownCommand),
    }
}

/// Parses the arguments after the program's name.
///
/// # Errors
///
/// A [`UsageError`] for anything that is not a subcommand with the flags
/// in [`FLAGS`].
pub fn parse_args(args: &[OsString]) -> Result<Command, UsageError> {
    let mut words = Vec::new();
    let mut data_dir = None;
    let mut help = false;
    let mut version = false;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        let text = arg.to_string_lossy();
        match &*text {
            "--help" | "-h" => help = true,
            "--version" | "-V" => version = true,
            DATA_DIR => {
                let value = rest
                    .next()
                    .filter(|value| !value.is_empty())
                    .ok_or(UsageError::MissingValue)?;
                if data_dir.replace(value.clone()).is_some() {
                    return Err(UsageError::RepeatedFlag);
                }
            }
            flag if flag.starts_with('-') => return Err(UsageError::UnknownFlag),
            _ => words.push(text),
        }
    }
    let words: Vec<&str> = words.iter().map(|word| &**word).collect();
    let action = match (help, version) {
        (true, _) => Action::Help,
        (false, true) => Action::Version,
        (false, false) => action(&words)?,
    };
    Ok(Command { action, data_dir })
}

/// How the binary ended, as its exit code says. The codes are those of
/// `sysexits.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    /// The command did what it was asked.
    Ok,
    /// The command line was refused.
    Usage,
    /// The subcommand is not in this build.
    Unavailable,
    /// The process could not read its own privileges or switch core dumps
    /// off.
    Os,
    /// The data directory was refused or could not be used.
    DataDir,
    /// The process is root or holds a capability.
    Privileged,
    /// The configuration file or the environment was refused.
    Config,
}

impl Exit {
    /// The exit code.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Ok => 0,
            Self::Usage => 64,
            Self::Unavailable => 69,
            Self::Os => 71,
            Self::DataDir => 74,
            Self::Privileged => 77,
            Self::Config => 78,
        }
    }

    /// The code a start-up refusal ends with.
    #[must_use]
    pub const fn of(error: &StartError) -> Self {
        match error {
            StartError::Os(_) => Self::Os,
            StartError::Privileged(_) => Self::Privileged,
            StartError::DataDir(_) => Self::DataDir,
            StartError::ConfigFile(_) | StartError::Config(_) => Self::Config,
        }
    }
}

/// What `--help` prints. The worker's entry is left out on purpose.
pub const HELP: &str = "\
Usage: gunmetal [--data-dir DIR] <command>

Commands:
  serve             Run the server
  doctor            Check the installation
  admin recover     Recover administrator access
  migrate           Migrate the durable state
  rebuild           Rebuild the cache from the library and the user log
  snapshot restore  Go back to a snapshot taken before a migration
  restore           Restore a backup
  service           Install or remove the system service
  audit verify      Verify the audit log
  keys rotate       Rotate the server's keys

Options:
  --data-dir DIR    The data directory (default: GUNMETAL_DATA_DIR, then
                    /var/lib/gunmetal)
  -h, --help        Print this help
  -V, --version     Print the version

No option takes a secret. Secrets are read from files.
";

/// Writes `text` and a newline to the console. A console that cannot be
/// written to changes nothing about the command's result.
fn say(console: &mut dyn Write, text: &str) {
    let _ = writeln!(console, "{text}");
}

/// Runs the binary: `args` are the arguments after the program's name and
/// `vars` the process environment. Help, the version and log lines go to
/// `out`; messages for the person at the console go to `err`.
pub fn run(
    args: &[OsString],
    vars: Vec<(OsString, OsString)>,
    out: Box<dyn Write + Send>,
    err: &mut dyn Write,
) -> Exit {
    match parse_args(args) {
        Ok(command) => dispatch(&command, vars, out, err),
        Err(error) => {
            say(err, &format!("gunmetal: {}", error.message()));
            say(err, "Run gunmetal --help for the commands and options.");
            Exit::Usage
        }
    }
}

/// Sends `command` to the module that runs it.
fn dispatch(
    command: &Command,
    vars: Vec<(OsString, OsString)>,
    mut out: Box<dyn Write + Send>,
    err: &mut dyn Write,
) -> Exit {
    match command.action {
        Action::Help => print(&mut *out, HELP.trim_end()),
        Action::Version => print(&mut *out, &format!("gunmetal {VERSION}")),
        Action::Serve => serve(command, vars, out, err),
        // One line per subcommand. The package that builds a subcommand
        // replaces its line with a call into its own module.
        Action::Doctor => not_built(err, "doctor"),
        Action::AdminRecover => not_built(err, "admin recover"),
        Action::Migrate => not_built(err, "migrate"),
        Action::Rebuild => not_built(err, "rebuild"),
        Action::SnapshotRestore => not_built(err, "snapshot restore"),
        Action::Restore => not_built(err, "restore"),
        Action::Service => not_built(err, "service"),
        Action::AuditVerify => not_built(err, "audit verify"),
        Action::KeysRotate => not_built(err, "keys rotate"),
        Action::Worker => not_built(err, "worker"),
    }
}

/// Prints `text` for a command that only prints.
fn print(out: &mut dyn Write, text: &str) -> Exit {
    say(out, text);
    Exit::Ok
}

/// Refuses a subcommand whose module is not built yet.
fn not_built(err: &mut dyn Write, name: &str) -> Exit {
    say(
        err,
        &format!("gunmetal: {name} is not part of this build yet."),
    );
    Exit::Unavailable
}

/// Runs `serve`: switches core dumps off, probes the host, and starts the
/// application state. The listener (WP-118) takes the state from here.
fn serve(
    command: &Command,
    vars: Vec<(OsString, OsString)>,
    out: Box<dyn Write + Send>,
    err: &mut dyn Write,
) -> Exit {
    let env = Env::read(vars);
    let from_env = env.as_ref().ok().and_then(|env| env.data_dir.as_ref());
    let dir = datadir::choose(command.data_dir.as_ref(), from_env);
    let started = start_from_probes(
        host::disable_core_dumps().and_then(|()| Privileges::probe()),
        HostFacts::probe(&dir),
        &dir,
        env,
        out,
    );
    match started {
        Ok(_) => Exit::Ok,
        Err(error) => {
            say(err, &format!("gunmetal: {}", error.message(&dir)));
            Exit::of(&error)
        }
    }
}

/// Starts from already-probed privileges and host facts, so a refused
/// probe is tested without forcing this process to fail its own syscalls.
fn start_from_probes(
    privileges: Result<Privileges, rustix::io::Errno>,
    facts: Result<HostFacts, gunmetal_fs::dataroot::DataRootError>,
    dir: &std::path::Path,
    env: Result<Env, ConfigError>,
    out: Box<dyn Write + Send>,
) -> Result<AppState, StartError> {
    let privileges = privileges.map_err(StartError::Os)?;
    privileges.check().map_err(StartError::Privileged)?;
    let env = env.map_err(StartError::Config)?;
    let facts = facts.map_err(StartError::DataDir)?;
    AppState::start(
        dir,
        &Host { privileges, facts },
        &env,
        Arc::new(SystemClock),
        out,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigError;
    use crate::datadir::ConfigFileError;
    use crate::host::PrivilegeError;
    use crate::testing::Capture;
    use gunmetal_fs::dataroot::DataRootError;
    use gunmetal_fs::host::NetworkFs;
    use proptest::prelude::*;
    use rustix::io::Errno;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    fn parse(list: &[&str]) -> Result<Command, UsageError> {
        parse_args(&args(list))
    }

    fn plain(action: Action) -> Command {
        Command {
            action,
            data_dir: None,
        }
    }

    /// Runs the binary with no environment and returns its exit, what it
    /// printed and what it told the console.
    fn ran(list: &[&str]) -> (Exit, String, String) {
        let out = Capture::default();
        let mut err = Capture::default();
        let exit = run(&args(list), Vec::new(), Box::new(out.clone()), &mut err);
        (exit, out.text(), err.text())
    }

    #[test]
    fn parses_every_subcommand() {
        let cases: [(&[&str], Action); 11] = [
            (&["serve"], Action::Serve),
            (&["doctor"], Action::Doctor),
            (&["admin", "recover"], Action::AdminRecover),
            (&["migrate"], Action::Migrate),
            (&["rebuild"], Action::Rebuild),
            (&["snapshot", "restore"], Action::SnapshotRestore),
            (&["restore"], Action::Restore),
            (&["service"], Action::Service),
            (&["audit", "verify"], Action::AuditVerify),
            (&["keys", "rotate"], Action::KeysRotate),
            (&["worker"], Action::Worker),
        ];
        for (words, action) in cases {
            assert_eq!(parse(words), Ok(plain(action)));
        }
        assert_eq!(SUBCOMMANDS, cases);
    }

    #[test]
    fn takes_the_data_directory_before_between_or_after_the_words() {
        let expected = Ok(Command {
            action: Action::AdminRecover,
            data_dir: Some(OsString::from("/srv/gunmetal")),
        });
        for list in [
            ["--data-dir", "/srv/gunmetal", "admin", "recover"],
            ["admin", "--data-dir", "/srv/gunmetal", "recover"],
            ["admin", "recover", "--data-dir", "/srv/gunmetal"],
        ] {
            assert_eq!(parse(&list), expected);
        }
        // The value is taken as it is, even when it looks like a flag or a
        // command.
        assert_eq!(
            parse(&["serve", "--data-dir", "--help"]),
            Ok(Command {
                action: Action::Serve,
                data_dir: Some(OsString::from("--help")),
            })
        );
    }

    #[cfg(unix)]
    #[test]
    fn keeps_a_data_directory_that_is_not_utf8() {
        use std::os::unix::ffi::OsStringExt;
        let dir = OsString::from_vec(vec![b'/', 0xff]);
        assert_eq!(
            parse_args(&[
                OsString::from("serve"),
                OsString::from("--data-dir"),
                dir.clone()
            ]),
            Ok(Command {
                action: Action::Serve,
                data_dir: Some(dir),
            })
        );
    }

    #[test]
    fn help_and_version_need_no_command_and_help_wins() {
        for flag in ["--help", "-h"] {
            assert_eq!(parse(&[flag]), Ok(plain(Action::Help)));
            assert_eq!(parse(&["serve", flag]), Ok(plain(Action::Help)));
            assert_eq!(parse(&["--version", flag]), Ok(plain(Action::Help)));
            assert_eq!(
                parse(&["no", "such", "command", flag]),
                Ok(plain(Action::Help))
            );
        }
        for flag in ["--version", "-V"] {
            assert_eq!(parse(&[flag]), Ok(plain(Action::Version)));
            assert_eq!(parse(&[flag, "doctor"]), Ok(plain(Action::Version)));
        }
    }

    #[test]
    fn refuses_every_malformed_command_line() {
        let cases: [(&[&str], UsageError); 14] = [
            (&[], UsageError::NoCommand),
            (&["--data-dir", "/d"], UsageError::NoCommand),
            (&["run"], UsageError::UnknownCommand),
            (&["admin", "reset"], UsageError::UnknownCommand),
            (&["Serve"], UsageError::UnknownCommand),
            (&[""], UsageError::UnknownCommand),
            (
                &["admin"],
                UsageError::IncompleteCommand("admin".to_owned()),
            ),
            (
                &["snapshot"],
                UsageError::IncompleteCommand("snapshot".to_owned()),
            ),
            (&["serve", "now"], UsageError::UnexpectedArgument),
            (
                &["snapshot", "restore", "2026-10-01", "x"],
                UsageError::UnexpectedArgument,
            ),
            (&["serve", "--data-dir"], UsageError::MissingValue),
            (&["serve", "--data-dir", ""], UsageError::MissingValue),
            (
                &["--data-dir", "/a", "serve", "--data-dir", "/b"],
                UsageError::RepeatedFlag,
            ),
            (&["serve", "--data-dir=/srv"], UsageError::UnknownFlag),
        ];
        for (list, error) in cases {
            assert_eq!(parse(list), Err(error));
        }
    }

    /// Verifies: SEC-OPS-014, SEC-HIS-004
    #[test]
    fn the_flags_are_exactly_these_and_none_takes_a_secret() {
        // A change here is a change to what can be typed on a command line
        // that every user of the host can read. Before updating this list,
        // check the new flag: it must not carry a secret, and it must not
        // turn authentication off or weaken it.
        assert_eq!(FLAGS, ["--data-dir", "--help", "-h", "--version", "-V"]);
        for flag in [
            "--root-key",
            "--root-secret",
            "--token",
            "--api-key",
            "--password",
            "--recovery-code",
            "--oidc-client-secret",
            "--no-auth",
            "--disable-auth",
            "--insecure",
            "-p",
            "-",
            "--",
        ] {
            assert_eq!(
                parse(&["serve", flag, "canary-value"]),
                Err(UsageError::UnknownFlag)
            );
        }
    }

    /// Verifies: SEC-OPS-014
    #[test]
    fn a_refused_argument_is_never_repeated_to_the_console() {
        // A secret typed by mistake as a word, or glued to a flag with no
        // `=`, must not reach the console or the journal behind it.
        let option = "gunmetal: An option was given that gunmetal does not have. Options take no secrets: those come from files (see the documentation on secrets).
Run gunmetal --help for the commands and options.
";
        let command = "gunmetal: That is not a gunmetal command.
Run gunmetal --help for the commands and options.
";
        let argument = "gunmetal: The command was followed by an argument it does not take.
Run gunmetal --help for the commands and options.
";
        let cases: [(&[&str], &str); 9] = [
            (&["serve", "-pSECRET-canary-9f2a"], option),
            (&["serve", "--passwordSECRET-canary-9f2a"], option),
            (&["serve", "--password", "SECRET-canary-9f2a"], option),
            (&["serve", "--password=SECRET-canary-9f2a"], option),
            (&["serve", "--password=SE=CRET-canary-9f2a"], option),
            (&["serve", "-p=SECRET-canary-9f2a"], option),
            (&["SECRET-canary-9f2a"], command),
            (&["admin", "SECRET-canary-9f2a"], command),
            (&["admin", "recover", "SECRET-canary-9f2a"], argument),
        ];
        for (list, expected) in cases {
            assert_eq!(ran(list), (Exit::Usage, String::new(), expected.to_owned()));
        }
    }

    proptest! {
        /// Verifies: SEC-OPS-014, SEC-HIS-004
        #[test]
        fn a_flag_that_is_not_listed_is_refused(
            flag in "-{1,2}[a-zA-Z=-]{0,16}",
            at in 0_usize..3,
        ) {
            prop_assume!(!["--data-dir", "--help", "-h", "--version", "-V"].contains(&flag.as_str()));
            let mut list = vec!["admin", "recover"];
            list.insert(at, &flag);
            prop_assert_eq!(parse(&list), Err(UsageError::UnknownFlag));
        }
    }

    #[test]
    fn says_what_is_wrong_with_the_command_line() {
        let messages: Vec<String> = [
            UsageError::NoCommand,
            UsageError::UnknownCommand,
            UsageError::IncompleteCommand("admin".to_owned()),
            UsageError::UnexpectedArgument,
            UsageError::UnknownFlag,
            UsageError::MissingValue,
            UsageError::RepeatedFlag,
        ]
        .iter()
        .map(UsageError::message)
        .collect();
        assert_eq!(
            messages,
            [
                "No command given.",
                "That is not a gunmetal command.",
                "\"admin\" is not a whole command; it needs another word.",
                "The command was followed by an argument it does not take.",
                "An option was given that gunmetal does not have. Options take no secrets: those come from files (see the documentation on secrets).",
                "--data-dir needs a directory after it.",
                "--data-dir was given twice.",
            ]
        );
    }

    /// Verifies: SEC-OPS-053
    #[test]
    fn every_refusal_has_its_documented_exit_code() {
        let codes = [
            Exit::Ok,
            Exit::Usage,
            Exit::Unavailable,
            Exit::Os,
            Exit::DataDir,
            Exit::Privileged,
            Exit::Config,
        ]
        .map(Exit::code);
        assert_eq!(codes, [0, 64, 69, 71, 74, 77, 78]);
        let exits = [
            StartError::Os(Errno::PERM),
            StartError::Privileged(PrivilegeError::Root),
            StartError::Privileged(PrivilegeError::Capabilities {
                effective: 1,
                permitted: 1,
            }),
            StartError::DataDir(DataRootError::NetworkFilesystem(NetworkFs::Nfs)),
            StartError::ConfigFile(ConfigFileError::NotUtf8),
            StartError::Config(ConfigError::UnknownVariable("GUNMETAL_X".to_owned())),
        ]
        .map(|error| Exit::of(&error));
        assert_eq!(
            exits,
            [
                Exit::Os,
                Exit::Privileged,
                Exit::Privileged,
                Exit::DataDir,
                Exit::Config,
                Exit::Config
            ]
        );
    }

    #[test]
    fn prints_the_help_without_the_worker_entry() {
        let (exit, out, err) = ran(&["--help"]);
        assert_eq!((exit, err.as_str()), (Exit::Ok, ""));
        assert_eq!(
            out,
            "Usage: gunmetal [--data-dir DIR] <command>\n\
             \n\
             Commands:\n\
             \x20 serve             Run the server\n\
             \x20 doctor            Check the installation\n\
             \x20 admin recover     Recover administrator access\n\
             \x20 migrate           Migrate the durable state\n\
             \x20 rebuild           Rebuild the cache from the library and the user log\n\
             \x20 snapshot restore  Go back to a snapshot taken before a migration\n\
             \x20 restore           Restore a backup\n\
             \x20 service           Install or remove the system service\n\
             \x20 audit verify      Verify the audit log\n\
             \x20 keys rotate       Rotate the server's keys\n\
             \n\
             Options:\n\
             \x20 --data-dir DIR    The data directory (default: GUNMETAL_DATA_DIR, then\n\
             \x20                   /var/lib/gunmetal)\n\
             \x20 -h, --help        Print this help\n\
             \x20 -V, --version     Print the version\n\
             \n\
             No option takes a secret. Secrets are read from files.\n"
        );
    }

    #[test]
    fn prints_the_version() {
        assert_eq!(
            ran(&["--version"]),
            (Exit::Ok, "gunmetal 0.0.0\n".to_owned(), String::new())
        );
    }

    #[test]
    fn a_console_that_cannot_be_written_does_not_change_the_result() {
        struct Full;
        impl Write for Full {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::from(std::io::ErrorKind::WriteZero))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        assert_eq!(Full.flush().ok(), Some(()));
        assert_eq!(
            run(&args(&["--help"]), Vec::new(), Box::new(Full), &mut Full),
            Exit::Ok
        );
        assert_eq!(
            run(
                &args(&["serve", "--token"]),
                Vec::new(),
                Box::new(Full),
                &mut Full
            ),
            Exit::Usage
        );
    }

    #[test]
    fn a_refused_command_line_exits_with_the_usage_code_and_says_why() {
        assert_eq!(
            ran(&["serve", "--token", "canary-value"]),
            (
                Exit::Usage,
                String::new(),
                "gunmetal: An option was given that gunmetal does not have. Options take no secrets: those come from files (see the documentation on secrets).\nRun gunmetal --help for the commands and options.\n".to_owned()
            )
        );
    }

    #[test]
    fn serve_on_a_scratch_directory_starts_and_a_missing_one_is_refused() {
        let dir = gunmetal_testkit::tempdir::TempDir::new("cli-serve").expect("scratch");
        let path = dir.path().to_str().expect("UTF-8");
        let (exit, out, err) = ran(&["serve", "--data-dir", path]);
        assert_eq!((exit, err.as_str()), (Exit::Ok, ""));
        assert!(out.contains("\"event\":\"sys_startup\""));
        let missing = format!("{path}/gone");
        assert_eq!(ran(&["serve", "--data-dir", &missing]).0, Exit::DataDir);
    }

    #[test]
    fn a_failed_privilege_or_host_probe_keeps_its_exit() {
        let dir = gunmetal_testkit::tempdir::TempDir::new("cli-probe").expect("scratch");
        let env = Env::default();
        let out = Capture::default();
        let os = start_from_probes(
            Err(Errno::PERM),
            HostFacts::probe(dir.path()),
            dir.path(),
            Ok(env.clone()),
            Box::new(out.clone()),
        );
        assert_eq!(os.err().map(|error| Exit::of(&error)), Some(Exit::Os));
        assert_eq!(out.text(), "");
        let facts = Err(DataRootError::NetworkFilesystem(NetworkFs::Nfs));
        let data = start_from_probes(
            Ok(Privileges {
                euid: 1000,
                effective: 0,
                permitted: 0,
            }),
            facts,
            dir.path(),
            Ok(env.clone()),
            Box::new(Capture::default()),
        );
        assert_eq!(
            data.err().map(|error| Exit::of(&error)),
            Some(Exit::DataDir)
        );
        let started = start_from_probes(
            Ok(Privileges {
                euid: 1000,
                effective: 0,
                permitted: 0,
            }),
            HostFacts::probe(dir.path()),
            dir.path(),
            Ok(env),
            Box::new(Capture::default()),
        );
        assert!(started.is_ok());
    }

    /// Verifies: SEC-OPS-053
    #[test]
    fn root_is_refused_before_a_missing_data_directory_or_a_stray_variable() {
        let dir = gunmetal_testkit::tempdir::TempDir::new("cli-root-first").expect("scratch");
        let missing = std::path::PathBuf::from(format!("{}/gone", dir.path().display()));
        let root = Privileges {
            euid: 0,
            effective: 0,
            permitted: 0,
        };
        let out = Capture::default();
        let missing_dir = start_from_probes(
            Ok(root),
            Err(DataRootError::Io {
                item: gunmetal_fs::dataroot::Item::Root,
                op: gunmetal_fs::dataroot::Op::Probe,
                kind: std::io::ErrorKind::NotFound,
            }),
            &missing,
            Ok(Env::default()),
            Box::new(out.clone()),
        );
        assert_eq!(
            missing_dir.err().map(|error| Exit::of(&error)),
            Some(Exit::Privileged)
        );
        assert_eq!(out.text(), "");
        let stray = start_from_probes(
            Ok(root),
            HostFacts::probe(dir.path()),
            dir.path(),
            Err(ConfigError::UnknownVariable("GUNMETAL_X".to_owned())),
            Box::new(Capture::default()),
        );
        assert_eq!(
            stray.err().map(|error| Exit::of(&error)),
            Some(Exit::Privileged)
        );
        assert!(crate::testing::durable_missing(&dir));
        let caps = start_from_probes(
            Ok(Privileges {
                euid: 1000,
                effective: 0,
                permitted: 0x400,
            }),
            HostFacts::probe(dir.path()),
            dir.path(),
            Ok(Env::default()),
            Box::new(Capture::default()),
        );
        assert_eq!(
            caps.err().map(|error| Exit::of(&error)),
            Some(Exit::Privileged)
        );
        let unprivileged = Privileges {
            euid: 1000,
            effective: 0,
            permitted: 0,
        };
        let stray_after_check = start_from_probes(
            Ok(unprivileged),
            HostFacts::probe(dir.path()),
            dir.path(),
            Err(ConfigError::UnknownVariable("GUNMETAL_X".to_owned())),
            Box::new(Capture::default()),
        );
        assert_eq!(
            stray_after_check.err().map(|error| Exit::of(&error)),
            Some(Exit::Config)
        );
        assert!(crate::testing::durable_missing(&dir));
    }

    #[test]
    fn a_subcommand_another_package_builds_says_so_and_exits_unavailable() {
        let cases: [(&[&str], &str); 10] = [
            (&["doctor"], "doctor"),
            (&["admin", "recover"], "admin recover"),
            (&["migrate"], "migrate"),
            (&["rebuild"], "rebuild"),
            (&["snapshot", "restore"], "snapshot restore"),
            (&["restore"], "restore"),
            (&["service"], "service"),
            (&["audit", "verify"], "audit verify"),
            (&["keys", "rotate"], "keys rotate"),
            (&["worker"], "worker"),
        ];
        for (words, name) in cases {
            assert_eq!(
                ran(words),
                (
                    Exit::Unavailable,
                    String::new(),
                    format!("gunmetal: {name} is not part of this build yet.\n")
                )
            );
        }
    }
}
