//! The server's configuration: the file `durable/config.toml` and the
//! environment, checked rather than guessed (ADM-007).
//!
//! - **One schema.** [`KEYS`] lists every key the file may hold and
//!   [`VARIABLES`] every environment variable the server reads. The loader
//!   looks each name up there and refuses anything else by name, so a typo
//!   never passes silently and nothing reaches the server that the schema
//!   does not list.
//! - **Nothing dangerous to set.** No key or variable turns authentication
//!   off (SEC-HIS-004), and none takes a program, a command or a hook
//!   (SEC-HIS-021, SEC-TM-046): a setting's [`Kind`] is a directory, a flag,
//!   a log level or a file holding a secret, and nothing else. A snapshot
//!   test fails when the schema changes, so every new setting is reviewed
//!   against those rules.
//! - **Secrets come from files.** A secret is never read from a plain
//!   variable, which every process of the same user and every crash report
//!   can see. It comes from the file a `*_FILE` variable names or from a
//!   systemd credential; the plain form is refused with a message that
//!   shows the `_FILE` form, and the value is never repeated (SEC-OPS-014).
//! - **The level is never debug.** The configured level is info, warn or
//!   error. Debug level is only ever switched on for a time, through the
//!   logger (SEC-OPS-029).

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use toml::de::{DeTable, DeValue};

use crate::log::Level;

/// What every variable the server reads starts with.
const PREFIX: &str = "GUNMETAL_";

/// The variable systemd sets to the directory holding the unit's
/// credentials.
const CREDENTIALS_DIRECTORY: &str = "CREDENTIALS_DIRECTORY";

/// The systemd credential that holds the OIDC client secret.
pub const OIDC_CLIENT_SECRET_CREDENTIAL: &str = "gunmetal_oidc_client_secret";

/// What a setting's value is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A directory the server keeps its own data in.
    Directory,
    /// `true` or `false`.
    Flag,
    /// `info`, `warn` or `error`.
    LogLevel,
    /// The path of a file that holds a secret, which is read, never run.
    SecretFile,
}

impl Kind {
    /// What a value of this kind must be, for a message.
    const fn expected(self) -> &'static str {
        match self {
            Self::Directory | Self::SecretFile => "a path",
            Self::Flag => "\"true\" or \"false\"",
            Self::LogLevel => "\"info\", \"warn\" or \"error\"",
        }
    }
}

/// Which environment variable a [`VARIABLES`] entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variable {
    /// Where the data directory is.
    DataDir,
    /// The documented override that accepts a data directory on a network
    /// filesystem (ADM-079).
    AllowNetworkFilesystem,
    /// The log level, which wins over the file's.
    LogLevel,
    /// The file holding the OIDC client secret.
    OidcClientSecretFile,
}

/// One environment variable the server reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvSetting {
    /// The variable's name.
    pub name: &'static str,
    /// What its value is.
    pub kind: Kind,
    /// Which setting it is.
    pub id: Variable,
}

/// Which configuration file key a [`KEYS`] entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// The log level.
    LogLevel,
}

/// One key of the configuration file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileSetting {
    /// The key, with a `.` between a table and the key inside it.
    pub key: &'static str,
    /// What its value is.
    pub kind: Kind,
    /// Which setting it is.
    pub id: Key,
}

/// Every environment variable the server reads, besides systemd's
/// `CREDENTIALS_DIRECTORY`. A package that adds a setting adds its line.
pub const VARIABLES: [EnvSetting; 4] = [
    EnvSetting {
        name: "GUNMETAL_ALLOW_NETWORK_FILESYSTEM",
        kind: Kind::Flag,
        id: Variable::AllowNetworkFilesystem,
    },
    EnvSetting {
        name: "GUNMETAL_DATA_DIR",
        kind: Kind::Directory,
        id: Variable::DataDir,
    },
    EnvSetting {
        name: "GUNMETAL_LOG_LEVEL",
        kind: Kind::LogLevel,
        id: Variable::LogLevel,
    },
    EnvSetting {
        name: "GUNMETAL_OIDC_CLIENT_SECRET_FILE",
        kind: Kind::SecretFile,
        id: Variable::OidcClientSecretFile,
    },
];

/// Every key of the configuration file. A package that adds a setting adds
/// its line.
pub const KEYS: [FileSetting; 1] = [FileSetting {
    key: "log.level",
    kind: Kind::LogLevel,
    id: Key::LogLevel,
}];

/// Why the configuration was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// The file is not TOML.
    Syntax {
        /// The line the problem is on, counting from 1.
        line: usize,
        /// What the TOML reader said.
        message: String,
    },
    /// The file holds a key the server does not have.
    UnknownKey {
        /// The key.
        key: String,
        /// The line it is on.
        line: usize,
    },
    /// A key's value is of the wrong type or outside its range.
    InvalidValue {
        /// The key.
        key: &'static str,
        /// The line it is on.
        line: usize,
        /// What the value must be.
        expected: &'static str,
    },
    /// The environment holds a `GUNMETAL_` variable the server does not
    /// read.
    UnknownVariable(String),
    /// A variable's value is of the wrong type or outside its range.
    InvalidVariable {
        /// The variable.
        variable: &'static str,
        /// What the value must be.
        expected: &'static str,
    },
    /// A secret was passed in a plain variable.
    SecretInPlainVariable {
        /// The variable that held it.
        variable: String,
        /// The variable to set instead, naming a file that holds the
        /// secret.
        file_form: &'static str,
    },
}

impl ConfigError {
    /// What to tell the admin.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::Syntax { line, message } => {
                format!("Line {line} of the configuration file is not valid TOML: {message}.")
            }
            Self::UnknownKey { key, line } => format!(
                "Line {line} of the configuration file sets {key:?}, which is not a Gunmetal setting. Check the spelling, or remove it."
            ),
            Self::InvalidValue {
                key,
                line,
                expected,
            } => format!("Line {line} of the configuration file: {key} must be {expected}."),
            Self::UnknownVariable(variable) => format!(
                "The environment variable {variable:?} is not one Gunmetal reads. Check the spelling, or unset it."
            ),
            Self::InvalidVariable { variable, expected } => {
                format!("The environment variable {variable} must be {expected}.")
            }
            Self::SecretInPlainVariable {
                variable,
                file_form,
            } => format!(
                "{variable} holds a secret in a plain environment variable, which other processes and crash reports can read. Put the secret in a file only Gunmetal's user can read and set {file_form} to that file's path, or pass it as a systemd credential, then unset {variable}."
            ),
        }
    }
}

/// What the environment sets.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Env {
    /// `GUNMETAL_DATA_DIR`.
    pub data_dir: Option<PathBuf>,
    /// `GUNMETAL_ALLOW_NETWORK_FILESYSTEM`.
    pub allow_network_filesystem: bool,
    /// `GUNMETAL_LOG_LEVEL`.
    pub log_level: Option<Level>,
    /// `GUNMETAL_OIDC_CLIENT_SECRET_FILE`.
    pub oidc_client_secret_file: Option<PathBuf>,
    /// systemd's `CREDENTIALS_DIRECTORY`.
    pub credentials_directory: Option<PathBuf>,
}

/// `value` as a path, which must not be empty.
fn path(variable: &'static str, value: OsString) -> Result<PathBuf, ConfigError> {
    if value.is_empty() {
        return Err(ConfigError::InvalidVariable {
            variable,
            expected: Kind::Directory.expected(),
        });
    }
    Ok(PathBuf::from(value))
}

/// The level `text` names, if it is one the configuration may set.
fn level(text: &str) -> Option<Level> {
    match text {
        "info" => Some(Level::Info),
        "warn" => Some(Level::Warn),
        "error" => Some(Level::Error),
        _ => None,
    }
}

/// The flag `text` names.
fn flag(text: &str) -> Option<bool> {
    match text {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// `value` read by `read`, or the refusal for a variable of `kind`.
fn parsed<T>(
    setting: &EnvSetting,
    value: &OsStr,
    read: fn(&str) -> Option<T>,
) -> Result<T, ConfigError> {
    value
        .to_str()
        .and_then(read)
        .ok_or(ConfigError::InvalidVariable {
            variable: setting.name,
            expected: setting.kind.expected(),
        })
}

/// The refusal for the `GUNMETAL_` variable `name`, which the schema does
/// not list: a secret in a plain variable when the schema has its `_FILE`
/// form, and an unknown variable otherwise.
fn unlisted(name: &str) -> ConfigError {
    let file_form = VARIABLES
        .iter()
        .filter(|setting| setting.kind == Kind::SecretFile)
        .find(|setting| setting.name.strip_suffix("_FILE") == Some(name));
    match file_form {
        Some(setting) => ConfigError::SecretInPlainVariable {
            variable: name.to_owned(),
            file_form: setting.name,
        },
        None => ConfigError::UnknownVariable(name.to_owned()),
    }
}

impl Env {
    /// Reads the server's variables out of the process environment `vars`.
    /// Variables that do not start with `GUNMETAL_` belong to something
    /// else and are ignored, except systemd's `CREDENTIALS_DIRECTORY`.
    ///
    /// # Errors
    ///
    /// A [`ConfigError`] for a `GUNMETAL_` variable the server does not
    /// read, a value of the wrong kind, or a secret in a plain variable.
    pub fn read(vars: impl IntoIterator<Item = (OsString, OsString)>) -> Result<Self, ConfigError> {
        let mut env = Self::default();
        for (name, value) in vars {
            let name = name.to_string_lossy();
            if name == CREDENTIALS_DIRECTORY {
                env.credentials_directory = Some(path(CREDENTIALS_DIRECTORY, value)?);
            } else if name.starts_with(PREFIX) {
                env.set(&name, value)?;
            }
        }
        Ok(env)
    }

    /// Sets the variable `name`, which starts with `GUNMETAL_`.
    fn set(&mut self, name: &str, value: OsString) -> Result<(), ConfigError> {
        let Some(setting) = VARIABLES.iter().find(|setting| setting.name == name) else {
            return Err(unlisted(name));
        };
        match setting.id {
            Variable::DataDir => self.data_dir = Some(path(setting.name, value)?),
            Variable::AllowNetworkFilesystem => {
                self.allow_network_filesystem = parsed(setting, &value, flag)?;
            }
            Variable::LogLevel => self.log_level = Some(parsed(setting, &value, level)?),
            Variable::OidcClientSecretFile => {
                self.oidc_client_secret_file = Some(path(setting.name, value)?);
            }
        }
        Ok(())
    }
}

/// Where a secret is read from. Reading it is its consumer's work, through
/// the secrets crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretSource {
    /// The file a `*_FILE` variable names.
    File(PathBuf),
    /// A systemd credential: the file `name` in the unit's credentials
    /// directory. The unit may not pass this credential, so a missing file
    /// means the secret is not set.
    Credential {
        /// The credentials directory.
        directory: PathBuf,
        /// The credential's name.
        name: &'static str,
    },
}

/// The server's configuration, from the file and the environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// The log level: info unless the file or the environment says
    /// otherwise.
    pub log_level: Level,
    /// Where the OIDC client secret is read from, if anywhere.
    pub oidc_client_secret: Option<SecretSource>,
}

/// What the configuration file sets.
#[derive(Debug, Default)]
struct FileSettings {
    log_level: Option<Level>,
}

impl FileSettings {
    /// Sets the key `setting` from `value`, found on `line`.
    fn set(
        &mut self,
        setting: &FileSetting,
        line: usize,
        value: &DeValue<'_>,
    ) -> Result<(), ConfigError> {
        let invalid = ConfigError::InvalidValue {
            key: setting.key,
            line,
            expected: setting.kind.expected(),
        };
        match setting.id {
            Key::LogLevel => {
                self.log_level = Some(value.as_str().and_then(level).ok_or(invalid)?);
            }
        }
        Ok(())
    }

    /// Reads `table`, whose keys are named with `prefix` before them.
    fn read(&mut self, text: &str, prefix: &str, table: &DeTable<'_>) -> Result<(), ConfigError> {
        for (key, value) in table {
            let name = format!("{prefix}{}", key.get_ref());
            let inside = format!("{name}.");
            let line = line_of(text, key.span().start);
            match value.get_ref() {
                DeValue::Table(inner)
                    if KEYS.iter().any(|setting| setting.key.starts_with(&inside)) =>
                {
                    self.read(text, &inside, inner)?;
                }
                value => {
                    let Some(setting) = KEYS.iter().find(|setting| setting.key == name) else {
                        return Err(ConfigError::UnknownKey { key: name, line });
                    };
                    self.set(setting, line, value)?;
                }
            }
        }
        Ok(())
    }
}

/// The line of `text`, counting from 1, that byte `offset` is on.
fn line_of(text: &str, offset: usize) -> usize {
    text.bytes()
        .take(offset)
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

/// The line a TOML syntax error names, or 1 when the reader gave no span.
fn syntax_line(text: &str, span: Option<std::ops::Range<usize>>) -> usize {
    span.map_or(1, |span| line_of(text, span.start))
}

/// Reads the configuration file's `text` and lays the environment over it:
/// a variable wins over the file's key for the same setting.
///
/// # Errors
///
/// A [`ConfigError`] when the file is not TOML, holds a key the server does
/// not have, or gives a key a value it cannot take.
pub fn load_config(text: &str, env: &Env) -> Result<Config, ConfigError> {
    let table = DeTable::parse(text).map_err(|error| ConfigError::Syntax {
        line: syntax_line(text, error.span()),
        message: error.message().to_owned(),
    })?;
    let mut file = FileSettings::default();
    file.read(text, "", table.get_ref())?;
    let credential = || {
        env.credentials_directory
            .clone()
            .map(|directory| SecretSource::Credential {
                directory,
                name: OIDC_CLIENT_SECRET_CREDENTIAL,
            })
    };
    Ok(Config {
        log_level: env.log_level.or(file.log_level).unwrap_or(Level::Info),
        oidc_client_secret: env
            .oidc_client_secret_file
            .clone()
            .map(SecretSource::File)
            .or_else(credential),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn vars(list: &[(&str, &str)]) -> Vec<(OsString, OsString)> {
        list.iter()
            .map(|(name, value)| (OsString::from(name), OsString::from(value)))
            .collect()
    }

    fn file(text: &str) -> Result<Config, ConfigError> {
        load_config(text, &Env::default())
    }

    fn with_level(log_level: Level) -> Config {
        Config {
            log_level,
            oidc_client_secret: None,
        }
    }

    // ----- The schema.

    /// Verifies: SEC-HIS-004, SEC-HIS-021, SEC-TM-046
    #[test]
    fn the_schema_is_exactly_these_settings() {
        // A change here is a change to what an admin can set. Before
        // updating this list, check the new setting against the rules: it
        // must not turn authentication off or weaken it, and it must not
        // name a program, a command, a script, a hook or a template.
        assert_eq!(
            VARIABLES,
            [
                EnvSetting {
                    name: "GUNMETAL_ALLOW_NETWORK_FILESYSTEM",
                    kind: Kind::Flag,
                    id: Variable::AllowNetworkFilesystem,
                },
                EnvSetting {
                    name: "GUNMETAL_DATA_DIR",
                    kind: Kind::Directory,
                    id: Variable::DataDir,
                },
                EnvSetting {
                    name: "GUNMETAL_LOG_LEVEL",
                    kind: Kind::LogLevel,
                    id: Variable::LogLevel,
                },
                EnvSetting {
                    name: "GUNMETAL_OIDC_CLIENT_SECRET_FILE",
                    kind: Kind::SecretFile,
                    id: Variable::OidcClientSecretFile,
                },
            ]
        );
        assert_eq!(
            KEYS,
            [FileSetting {
                key: "log.level",
                kind: Kind::LogLevel,
                id: Key::LogLevel,
            }]
        );
        let names: Vec<String> = VARIABLES
            .iter()
            .map(|setting| setting.name.to_lowercase())
            .chain(KEYS.iter().map(|setting| setting.key.to_owned()))
            .collect();
        let forbidden = [
            "auth",
            "login",
            "password",
            "passkey",
            "anonymous",
            "insecure",
            "disable",
            "skip",
            "bypass",
            "exec",
            "command",
            "cmd",
            "hook",
            "script",
            "shell",
            "program",
            "binary",
            "bin",
            "template",
            "plugin",
            "ffmpeg",
        ];
        for name in &names {
            for word in forbidden {
                assert!(!name.contains(word));
            }
        }
        // Every kind of value there is: none of them is run.
        for kind in VARIABLES
            .iter()
            .map(|setting| setting.kind)
            .chain(KEYS.iter().map(|setting| setting.kind))
        {
            match kind {
                Kind::Directory | Kind::Flag | Kind::LogLevel | Kind::SecretFile => {}
            }
        }
    }

    /// Where an unknown dotted key is refused: at its first part that the
    /// schema does not know. Written for the test; `log` is the one table
    /// the schema has.
    fn refused_at(key: &str) -> String {
        let mut parts = key.split('.');
        let first = parts.next().expect("a first part");
        match (first, parts.next()) {
            ("log", Some(second)) => format!("log.{second}"),
            _ => first.to_owned(),
        }
    }

    proptest! {
        /// Verifies: SEC-HIS-004, SEC-HIS-021, SEC-TM-046
        #[test]
        fn a_key_the_schema_does_not_list_is_refused_by_name(
            key in "[a-z_]{1,10}(\\.[a-z_]{1,10}){0,2}",
            value in prop::sample::select(vec!["true", "\"/bin/sh\"", "1", "[]"]),
        ) {
            prop_assume!(key != "log.level" && !key.starts_with("log.level."));
            prop_assert_eq!(
                file(&format!("{key} = {value}\n")),
                Err(ConfigError::UnknownKey { key: refused_at(&key), line: 1 })
            );
        }

        /// Verifies: SEC-HIS-004, SEC-HIS-021, SEC-TM-046, SEC-OPS-014
        #[test]
        fn a_variable_the_schema_does_not_list_is_refused_by_name(
            suffix in "[A-Z_]{0,24}",
            value in "[ -~]{0,16}",
        ) {
            let name = format!("GUNMETAL_{suffix}");
            let listed = [
                "GUNMETAL_ALLOW_NETWORK_FILESYSTEM",
                "GUNMETAL_DATA_DIR",
                "GUNMETAL_LOG_LEVEL",
                "GUNMETAL_OIDC_CLIENT_SECRET_FILE",
                "GUNMETAL_OIDC_CLIENT_SECRET",
            ];
            prop_assume!(!listed.contains(&name.as_str()));
            prop_assert_eq!(
                Env::read(vars(&[(&name, &value)])),
                Err(ConfigError::UnknownVariable(name))
            );
        }
    }

    // ----- The file.

    /// Verifies: SEC-OPS-029
    #[test]
    fn an_empty_file_and_an_empty_environment_give_the_defaults() {
        assert_eq!(file(""), Ok(with_level(Level::Info)));
        assert_eq!(file("# nothing set\n[log]\n"), Ok(with_level(Level::Info)));
    }

    #[test]
    fn reads_the_log_level_in_either_toml_form() {
        assert_eq!(
            file("[log]\nlevel = \"warn\"\n"),
            Ok(with_level(Level::Warn))
        );
        assert_eq!(file("log.level = \"error\""), Ok(with_level(Level::Error)));
        assert_eq!(
            file("log = { level = \"info\" }"),
            Ok(with_level(Level::Info))
        );
    }

    #[test]
    fn refuses_an_unknown_key_with_its_name_and_line() {
        let cases = [
            ("auth_disabled = true\n", "auth_disabled", 1),
            ("# a comment\n\n[log]\nlevle = \"warn\"\n", "log.levle", 4),
            ("[log]\nlevel = \"warn\"\n\n[auth]\n", "auth", 4),
            (
                "[transcoder]\npath = \"/usr/bin/ffmpeg\"\n",
                "transcoder",
                1,
            ),
            (
                "[log]\nlevel = \"warn\"\nhook.on_start = \"x\"\n",
                "log.hook",
                3,
            ),
        ];
        for (text, key, line) in cases {
            assert_eq!(
                file(text),
                Err(ConfigError::UnknownKey {
                    key: key.to_owned(),
                    line
                })
            );
        }
        assert_eq!(refused_at("log.hook"), "log.hook");
        assert_eq!(refused_at("server"), "server");
        assert_eq!(refused_at("log"), "log");
    }

    /// Verifies: SEC-OPS-029
    #[test]
    fn refuses_a_level_the_configuration_may_not_set() {
        let cases = [
            ("log.level = \"debug\"", 1),
            ("\n[log]\nlevel = \"INFO\"", 3),
            ("[log]\nlevel = 3", 2),
            ("[log]\nlevel = \"\"", 2),
            ("[log.level]\nname = \"warn\"", 1),
        ];
        for (text, line) in cases {
            assert_eq!(
                file(text),
                Err(ConfigError::InvalidValue {
                    key: "log.level",
                    line,
                    expected: "\"info\", \"warn\" or \"error\"",
                })
            );
        }
    }

    #[test]
    fn reports_the_line_of_a_syntax_error() {
        assert_eq!(
            file("[log]\nlevel = \"warn\"\nlevel = \"error\"\n"),
            Err(ConfigError::Syntax {
                line: 3,
                message: "duplicate key".to_owned(),
            })
        );
        assert_eq!(
            file("[log\n"),
            Err(ConfigError::Syntax {
                line: 1,
                message: "unclosed table, expected `]`".to_owned(),
            })
        );
    }

    #[test]
    fn deep_nesting_is_refused_without_exhausting_the_stack() {
        let depth = 30_000;
        let text = format!("a = {}{}", "[".repeat(depth), "]".repeat(depth));
        assert_eq!(
            file(&text),
            Err(ConfigError::Syntax {
                line: 1,
                message: "cannot recurse further; max recursion depth met".to_owned(),
            })
        );
    }

    #[test]
    fn counts_lines_from_one() {
        let text = "a\nbc\n\nd";
        let lines: Vec<usize> = (0..=text.len()).map(|at| line_of(text, at)).collect();
        assert_eq!(lines, [1, 1, 2, 2, 2, 3, 4, 4]);
        assert_eq!(syntax_line(text, None), 1);
        assert_eq!(syntax_line(text, Some(5..6)), 3);
    }

    // ----- The environment.

    #[test]
    fn reads_every_variable_and_ignores_other_programs_variables() {
        let read = Env::read(vars(&[
            ("PATH", "/usr/bin"),
            ("GUNMETAL", "not ours: no underscore"),
            ("XGUNMETAL_LOG_LEVEL", "not ours either"),
            ("GUNMETAL_DATA_DIR", "/srv/gunmetal"),
            ("GUNMETAL_ALLOW_NETWORK_FILESYSTEM", "true"),
            ("GUNMETAL_LOG_LEVEL", "error"),
            ("GUNMETAL_OIDC_CLIENT_SECRET_FILE", "/run/keys/oidc"),
            ("CREDENTIALS_DIRECTORY", "/run/credentials/gunmetal.service"),
        ]));
        assert_eq!(
            read,
            Ok(Env {
                data_dir: Some(PathBuf::from("/srv/gunmetal")),
                allow_network_filesystem: true,
                log_level: Some(Level::Error),
                oidc_client_secret_file: Some(PathBuf::from("/run/keys/oidc")),
                credentials_directory: Some(PathBuf::from("/run/credentials/gunmetal.service")),
            })
        );
        assert_eq!(Env::read(vars(&[("HOME", "/root")])), Ok(Env::default()));
        assert_eq!(
            Env::read(vars(&[
                ("GUNMETAL_ALLOW_NETWORK_FILESYSTEM", "false"),
                ("GUNMETAL_LOG_LEVEL", "warn"),
            ])),
            Ok(Env {
                log_level: Some(Level::Warn),
                ..Env::default()
            })
        );
        assert_eq!(
            Env::read(vars(&[("GUNMETAL_LOG_LEVEL", "info")])),
            Ok(Env {
                log_level: Some(Level::Info),
                ..Env::default()
            })
        );
    }

    #[test]
    fn refuses_a_variable_value_of_the_wrong_kind() {
        let cases = [
            (
                "GUNMETAL_ALLOW_NETWORK_FILESYSTEM",
                "yes",
                "\"true\" or \"false\"",
            ),
            (
                "GUNMETAL_ALLOW_NETWORK_FILESYSTEM",
                "",
                "\"true\" or \"false\"",
            ),
            (
                "GUNMETAL_LOG_LEVEL",
                "debug",
                "\"info\", \"warn\" or \"error\"",
            ),
            ("GUNMETAL_DATA_DIR", "", "a path"),
            ("GUNMETAL_OIDC_CLIENT_SECRET_FILE", "", "a path"),
            ("CREDENTIALS_DIRECTORY", "", "a path"),
        ];
        for (variable, value, expected) in cases {
            assert_eq!(
                Env::read(vars(&[(variable, value)])),
                Err(ConfigError::InvalidVariable { variable, expected })
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn refuses_a_flag_that_is_not_text_and_keeps_a_path_that_is_not() {
        use std::os::unix::ffi::OsStringExt;
        let bytes = || OsString::from_vec(vec![b'/', 0xff]);
        assert_eq!(
            Env::read([(OsString::from("GUNMETAL_LOG_LEVEL"), bytes())]),
            Err(ConfigError::InvalidVariable {
                variable: "GUNMETAL_LOG_LEVEL",
                expected: "\"info\", \"warn\" or \"error\"",
            })
        );
        assert_eq!(
            Env::read([(OsString::from("GUNMETAL_DATA_DIR"), bytes())]),
            Ok(Env {
                data_dir: Some(PathBuf::from(bytes())),
                ..Env::default()
            })
        );
        assert_eq!(
            Env::read([(OsString::from_vec(b"GUNMETAL_\xff".to_vec()), bytes())]),
            Err(ConfigError::UnknownVariable("GUNMETAL_\u{fffd}".to_owned()))
        );
    }

    /// Verifies: SEC-OPS-014
    #[test]
    fn refuses_a_secret_in_a_plain_variable_and_shows_the_file_form() {
        const CANARY: &str = "canary-oidc-secret-4f1c";
        let refused = Env::read(vars(&[("GUNMETAL_OIDC_CLIENT_SECRET", CANARY)]));
        assert_eq!(
            refused,
            Err(ConfigError::SecretInPlainVariable {
                variable: "GUNMETAL_OIDC_CLIENT_SECRET".to_owned(),
                file_form: "GUNMETAL_OIDC_CLIENT_SECRET_FILE",
            })
        );
        let message = refused.expect_err("refused").message();
        assert_eq!(
            message,
            "GUNMETAL_OIDC_CLIENT_SECRET holds a secret in a plain environment variable, which other processes and crash reports can read. Put the secret in a file only Gunmetal's user can read and set GUNMETAL_OIDC_CLIENT_SECRET_FILE to that file's path, or pass it as a systemd credential, then unset GUNMETAL_OIDC_CLIENT_SECRET."
        );
        assert!(!message.contains(CANARY));
        // Only a secret's variable has a `_FILE` form to point at.
        assert_eq!(
            Env::read(vars(&[("GUNMETAL_LOG_LEVEL_FILE", "/x")])),
            Err(ConfigError::UnknownVariable(
                "GUNMETAL_LOG_LEVEL_FILE".to_owned()
            ))
        );
        assert_eq!(
            Env::read(vars(&[("GUNMETAL_DATA", "/x")])),
            Err(ConfigError::UnknownVariable("GUNMETAL_DATA".to_owned()))
        );
    }

    /// Verifies: SEC-OPS-014
    #[test]
    fn a_secret_comes_from_its_file_or_a_systemd_credential() {
        let from_file = Env {
            oidc_client_secret_file: Some(PathBuf::from("/run/keys/oidc")),
            credentials_directory: Some(PathBuf::from("/run/credentials/unit")),
            ..Env::default()
        };
        assert_eq!(
            load_config("", &from_file).map(|config| config.oidc_client_secret),
            Ok(Some(SecretSource::File(PathBuf::from("/run/keys/oidc"))))
        );
        let from_credential = Env {
            credentials_directory: Some(PathBuf::from("/run/credentials/unit")),
            ..Env::default()
        };
        assert_eq!(
            load_config("", &from_credential).map(|config| config.oidc_client_secret),
            Ok(Some(SecretSource::Credential {
                directory: PathBuf::from("/run/credentials/unit"),
                name: "gunmetal_oidc_client_secret",
            }))
        );
        // The file has no key for a secret, in any spelling.
        assert_eq!(
            file("[oidc]\nclient_secret = \"hunter2\"\n"),
            Err(ConfigError::UnknownKey {
                key: "oidc".to_owned(),
                line: 1
            })
        );
    }

    #[test]
    fn the_environment_wins_over_the_file() {
        let env = Env {
            log_level: Some(Level::Error),
            ..Env::default()
        };
        assert_eq!(
            load_config("log.level = \"warn\"", &env),
            Ok(with_level(Level::Error))
        );
        // The file is still checked.
        assert_eq!(
            load_config("log.level = \"loud\"", &env),
            Err(ConfigError::InvalidValue {
                key: "log.level",
                line: 1,
                expected: "\"info\", \"warn\" or \"error\"",
            })
        );
    }

    #[test]
    fn says_what_is_wrong_and_what_to_do() {
        let messages: Vec<String> = [
            ConfigError::Syntax {
                line: 3,
                message: "duplicate key".to_owned(),
            },
            ConfigError::UnknownKey {
                key: "log.levle".to_owned(),
                line: 4,
            },
            ConfigError::InvalidValue {
                key: "log.level",
                line: 2,
                expected: "\"info\", \"warn\" or \"error\"",
            },
            ConfigError::UnknownVariable("GUNMETAL_LOGLEVEL\n".to_owned()),
            ConfigError::InvalidVariable {
                variable: "GUNMETAL_DATA_DIR",
                expected: "a path",
            },
        ]
        .iter()
        .map(ConfigError::message)
        .collect();
        assert_eq!(
            messages,
            [
                "Line 3 of the configuration file is not valid TOML: duplicate key.",
                "Line 4 of the configuration file sets \"log.levle\", which is not a Gunmetal setting. Check the spelling, or remove it.",
                "Line 2 of the configuration file: log.level must be \"info\", \"warn\" or \"error\".",
                "The environment variable \"GUNMETAL_LOGLEVEL\\n\" is not one Gunmetal reads. Check the spelling, or unset it.",
                "The environment variable GUNMETAL_DATA_DIR must be a path.",
            ]
        );
    }
}
