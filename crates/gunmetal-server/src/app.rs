//! The application state, and the one way to build it: start-up.
//!
//! [`AppState::start`] is the order the server comes up in. It refuses root
//! and capabilities before it touches anything (SEC-OPS-053), opens the
//! data directory through the data-root handle, which creates the layout,
//! tightens loose modes and refuses a network filesystem (WP-126, ADM-079),
//! reads and checks the configuration, and only then builds the state the
//! rest of the server shares. What it learned about the host arrives as an
//! argument, so every refusal is tested without being root or mounting NFS.
//!
//! This file is a registry. A package whose module keeps state adds one
//! field to [`AppState`] and one line to [`AppState::start`].

use std::io::{self, Write};
use std::path::Path;
use std::sync::Arc;

use gunmetal_core::time::Clock;
use gunmetal_fs::dataroot::{DataRoot, DataRootError, Modes, NetworkFilesystems, Policy};
use gunmetal_fs::host::HostFacts;
use rustix::io::Errno;

use crate::bus::Bus;
use crate::config::{Config, ConfigError, Env, load_config};
use crate::datadir::{self, ConfigFileError};
use crate::host::{PrivilegeError, Privileges};
use crate::log::{Level, LogEvent, Logger};

/// The server's version, as the log and `--version` give it.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// What the probes found out about the host and this process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Host {
    /// Who the process runs as.
    pub privileges: Privileges,
    /// The service account and the data directory's filesystem.
    pub facts: HostFacts,
}

/// Why the server did not start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartError {
    /// The process could not read its own privileges or switch core dumps
    /// off.
    Os(Errno),
    /// The process is root or holds a capability.
    Privileged(PrivilegeError),
    /// The data root refused the data directory.
    DataDir(DataRootError),
    /// The configuration file could not be read.
    ConfigFile(ConfigFileError),
    /// The configuration file or the environment was refused.
    Config(ConfigError),
}

impl StartError {
    /// What to tell the admin, for the data directory at `dir`.
    #[must_use]
    pub fn message(&self, dir: &Path) -> String {
        match self {
            Self::Os(errno) => format!(
                "Could not read this process's privileges or switch off its core dumps: {}.",
                io::Error::from(*errno).kind()
            ),
            Self::Privileged(error) => error.message(),
            Self::DataDir(error) => datadir::explain(error, dir),
            Self::ConfigFile(error) => error.message(dir),
            Self::Config(error) => error.message(),
        }
    }
}

/// What every part of the server shares.
pub struct AppState {
    /// The configuration.
    pub config: Config,
    /// The data directory.
    pub data: DataRoot,
    /// The clock.
    pub clock: Arc<dyn Clock + Send + Sync>,
    /// The logger.
    pub log: Arc<Logger>,
    /// The event bus, which is also the sink for security events.
    pub bus: Arc<Bus>,
    // One line per module, appended by later packages.
}

/// Whether the environment accepts a data directory on a network
/// filesystem.
const fn network(env: &Env) -> NetworkFilesystems {
    if env.allow_network_filesystem {
        NetworkFilesystems::Allow
    } else {
        NetworkFilesystems::Refuse
    }
}

impl AppState {
    /// Starts the server's state in the data directory `dir`, on the host
    /// `host` describes, logging to `out`.
    ///
    /// # Errors
    ///
    /// A [`StartError`] when the process is privileged, the data root
    /// refuses the directory, or the configuration is refused. Modes the
    /// data root tightened before it refused are still logged.
    pub fn start(
        dir: &Path,
        host: &Host,
        env: &Env,
        clock: Arc<dyn Clock + Send + Sync>,
        out: Box<dyn Write + Send>,
    ) -> Result<Self, StartError> {
        host.privileges.check().map_err(StartError::Privileged)?;
        let log = Logger::new(Arc::clone(&clock), Level::Info, out);
        let policy = Policy {
            modes: Modes::Repair,
            network: network(env),
        };
        let (repairs, opened) = match DataRoot::open(dir, &host.facts, policy) {
            Ok(opened) => (opened.repairs, Ok(opened.root)),
            Err(refused) => (refused.repairs, Err(StartError::DataDir(refused.error))),
        };
        for repair in &repairs {
            log.log(&LogEvent::from(repair));
        }
        let data = opened?;
        let text = datadir::read_config(&data).map_err(StartError::ConfigFile)?;
        let config = load_config(&text, env).map_err(StartError::Config)?;
        log.set_base(config.log_level);
        log.log(&LogEvent::SysStartup { version: VERSION });
        Ok(Self {
            config,
            data,
            clock,
            log: Arc::new(log),
            bus: Arc::new(Bus::default()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datadir::CONFIG_FILE;
    use crate::testing::{self, Capture, Recording};
    use gunmetal_core::audit_event::{AuditUnavailable, SecurityEvent, SecuritySink};
    use gunmetal_fs::host::{Filesystem, NetworkFs};
    use gunmetal_fs::path::{DataDir, DataPath};
    use gunmetal_testkit::tempdir::TempDir;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::time::Duration;

    const STARTED: &str = "{\"ts\":\"2026-10-03T12:00:00.000Z\",\"level\":\"info\",\"event\":\"sys_startup\",\"version\":\"0.0.0\"}\n";

    fn uid() -> u32 {
        let uid = rustix::process::geteuid().as_raw();
        // Root ignores the modes these tests set.
        assert_ne!(uid, 0);
        uid
    }

    fn host(euid: u32, filesystem: Filesystem) -> Host {
        Host {
            privileges: Privileges {
                euid,
                effective: 0,
                permitted: 0,
            },
            facts: HostFacts {
                uid: uid(),
                filesystem,
            },
        }
    }

    fn local() -> Host {
        host(uid(), Filesystem::Local)
    }

    fn start(dir: &TempDir, host: &Host, env: &Env) -> (Result<AppState, StartError>, String) {
        let (clock, _) = testing::clock();
        let out = Capture::default();
        let started = AppState::start(dir.path(), host, env, clock, Box::new(out.clone()));
        (started, out.text())
    }

    /// The data directory, opened as the server would, to arrange a test.
    fn arrange(dir: &TempDir) -> DataRoot {
        DataRoot::open(dir.path(), &local().facts, Policy::DEFAULT)
            .expect("the data root opens")
            .root
    }

    /// Whether everything in the data directory has the owner and mode it
    /// must: the data root, told to change nothing, opens it.
    fn layout_is_sound(dir: &TempDir) -> bool {
        let strict = Policy {
            modes: Modes::Refuse,
            network: NetworkFilesystems::Refuse,
        };
        DataRoot::open(dir.path(), &local().facts, strict)
            .is_ok_and(|opened| opened.repairs.is_empty())
    }

    #[test]
    fn starts_on_an_empty_directory_with_the_defaults() {
        let dir = TempDir::new("app-empty").expect("scratch");
        let (started, out) = start(&dir, &local(), &Env::default());
        let state = started.expect("started");
        assert_eq!(
            state.config,
            Config {
                log_level: Level::Info,
                oidc_client_secret: None,
            }
        );
        assert_eq!(state.log.level(), Level::Info);
        assert_eq!(state.clock.now().millis(), testing::NOON);
        assert_eq!(out, STARTED);
        assert!(layout_is_sound(&dir));
        // The state's handle is the directory it was started in.
        state
            .data
            .replace(&CONFIG_FILE, b"log.level = \"error\"")
            .expect("written");
        assert_eq!(
            datadir::read_config(&arrange(&dir)),
            Ok("log.level = \"error\"".to_owned())
        );
    }

    /// Verifies: SEC-OPS-053, SEC-TM-041
    #[test]
    fn refuses_root_before_touching_the_data_directory() {
        let dir = TempDir::new("app-root").expect("scratch");
        // The override that accepts a network filesystem does not reach
        // this check, and no setting does.
        let env = Env {
            allow_network_filesystem: true,
            ..Env::default()
        };
        let (started, out) = start(&dir, &host(0, Filesystem::Local), &env);
        assert_eq!(
            started.err(),
            Some(StartError::Privileged(PrivilegeError::Root))
        );
        assert_eq!(out, "");
        // Nothing was created: the first real start still has work to do.
        let (started, out) = start(&dir, &local(), &Env::default());
        assert!(started.is_ok());
        assert_eq!(out, STARTED);
    }

    /// Verifies: SEC-OPS-053
    #[test]
    fn refuses_a_capability() {
        let dir = TempDir::new("app-caps").expect("scratch");
        let mut privileged = local();
        privileged.privileges.permitted = 0x400;
        let (started, out) = start(&dir, &privileged, &Env::default());
        assert_eq!(
            started.err(),
            Some(StartError::Privileged(PrivilegeError::Capabilities {
                effective: 0,
                permitted: 0x400,
            }))
        );
        assert_eq!(out, "");
    }

    #[test]
    fn refuses_a_network_filesystem_unless_the_override_is_set() {
        let dir = TempDir::new("app-nfs").expect("scratch");
        let nfs = host(uid(), Filesystem::Network(NetworkFs::Nfs));
        let (started, out) = start(&dir, &nfs, &Env::default());
        assert_eq!(
            started.err(),
            Some(StartError::DataDir(DataRootError::NetworkFilesystem(
                NetworkFs::Nfs
            )))
        );
        assert_eq!(out, "");
        let env = Env {
            allow_network_filesystem: true,
            ..Env::default()
        };
        let (started, out) = start(&dir, &nfs, &env);
        assert!(started.is_ok());
        assert_eq!(out, STARTED);
    }

    #[test]
    fn tightens_a_loose_mode_logs_it_and_continues() {
        let dir = TempDir::new("app-repair").expect("scratch");
        let key = DataPath::constant(DataDir::Secrets, "root.key");
        arrange(&dir)
            .create_new(&key)
            .expect("created")
            .set_permissions(std::fs::Permissions::from_mode(0o640))
            .expect("loosened");
        assert!(!layout_is_sound(&dir));
        let (started, out) = start(&dir, &local(), &Env::default());
        assert!(started.is_ok());
        assert_eq!(
            out,
            format!(
                "{}{STARTED}",
                "{\"ts\":\"2026-10-03T12:00:00.000Z\",\"level\":\"warn\",\"event\":\"gm_data_root_mode_repaired\",\"item\":\"secrets/root.key\",\"from\":416,\"to\":384}\n"
            )
        );
        assert!(layout_is_sound(&dir));
    }

    #[test]
    fn reads_the_configuration_file_and_lays_the_environment_over_it() {
        let dir = TempDir::new("app-config").expect("scratch");
        arrange(&dir)
            .replace(&CONFIG_FILE, b"[log]\nlevel = \"warn\"\n")
            .expect("written");
        let (started, out) = start(&dir, &local(), &Env::default());
        let state = started.expect("started");
        assert_eq!(state.config.log_level, Level::Warn);
        assert_eq!(state.log.level(), Level::Warn);
        // The start-up event is info, below the configured level.
        assert_eq!(out, "");
        let env = Env {
            log_level: Some(Level::Info),
            oidc_client_secret_file: Some(PathBuf::from("/run/keys/oidc")),
            ..Env::default()
        };
        let (started, out) = start(&dir, &local(), &env);
        assert_eq!(
            started.ok().map(|state| state.config),
            Some(Config {
                log_level: Level::Info,
                oidc_client_secret: Some(crate::config::SecretSource::File(PathBuf::from(
                    "/run/keys/oidc"
                ))),
            })
        );
        assert_eq!(out, STARTED);
    }

    #[test]
    fn refuses_a_configuration_file_it_cannot_read_or_accept() {
        let dir = TempDir::new("app-bad-config").expect("scratch");
        let root = arrange(&dir);
        root.replace(&CONFIG_FILE, b"[auth]\nrequired = false\n")
            .expect("written");
        let (started, out) = start(&dir, &local(), &Env::default());
        assert_eq!(
            started.err(),
            Some(StartError::Config(ConfigError::UnknownKey {
                key: "auth".to_owned(),
                line: 1,
            }))
        );
        assert_eq!(out, "");
        root.replace(&CONFIG_FILE, b"\xff").expect("written");
        let (started, out) = start(&dir, &local(), &Env::default());
        assert_eq!(
            started.err(),
            Some(StartError::ConfigFile(ConfigFileError::NotUtf8))
        );
        assert_eq!(out, "");
    }

    /// Verifies: SEC-OPS-029
    #[test]
    fn the_bus_carries_the_debug_switch_to_the_audit_sink() {
        let dir = TempDir::new("app-bus").expect("scratch");
        let (started, _) = start(&dir, &local(), &Env::default());
        let state = started.expect("started");
        // Nothing stores the record yet, so debug level stays off.
        assert_eq!(
            state
                .log
                .enable_debug(Duration::from_secs(60), None, &*state.bus),
            Err(AuditUnavailable)
        );
        assert_eq!(state.log.level(), Level::Info);
        let audit = Arc::new(Recording::new(true));
        let sink = Arc::clone(&audit);
        state
            .bus
            .security
            .subscribe(move |event| sink.record(event.clone()));
        assert!(
            state
                .log
                .enable_debug(Duration::from_secs(60), None, &*state.bus)
                .is_ok()
        );
        assert_eq!(
            audit.events(),
            [SecurityEvent::GmDebugLoggingEnabled { account: None }]
        );
        assert_eq!(state.log.level(), Level::Debug);
    }

    #[test]
    fn explains_every_refusal() {
        let dir = PathBuf::from("/data");
        let messages: Vec<String> = [
            StartError::Os(Errno::PERM),
            StartError::Privileged(PrivilegeError::Root),
            StartError::DataDir(DataRootError::NetworkFilesystem(NetworkFs::Smb)),
            StartError::ConfigFile(ConfigFileError::TooLarge),
            StartError::Config(ConfigError::UnknownVariable("GUNMETAL_X".to_owned())),
        ]
        .iter()
        .map(|error| error.message(&dir))
        .collect();
        assert_eq!(
            messages,
            [
                "Could not read this process's privileges or switch off its core dumps: permission denied.",
                "Gunmetal does not run as root, and no setting changes that. Run it as a dedicated unprivileged user that owns the data directory: User=gunmetal in a systemd unit, or --user in a container.",
                "The data directory /data is on a network filesystem (SMB). SQLite cannot lock its databases safely there. Move the data directory to a disk on this machine, or set GUNMETAL_ALLOW_NETWORK_FILESYSTEM=true to accept the risk.",
                "/data/durable/config.toml is longer than 65536 bytes, which no Gunmetal configuration needs.",
                "The environment variable \"GUNMETAL_X\" is not one Gunmetal reads. Check the spelling, or unset it.",
            ]
        );
    }
}
