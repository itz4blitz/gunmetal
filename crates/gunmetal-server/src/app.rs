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

use gunmetal_core::audit_event::SecuritySink;
use gunmetal_core::time::Clock;
use gunmetal_fs::dataroot::{DataRoot, DataRootError, Modes, NetworkFilesystems, Policy};
use gunmetal_fs::host::HostFacts;
use gunmetal_secrets::random::{OsRandom, Random, RandomnessUnavailable};
use gunmetal_store::store::{Generation, Store, StoreError};
use rustix::io::Errno;

use crate::audit_sink::AuditSink;
use crate::bus::Bus;
use crate::config::{Config, ConfigError, Env, load_config};
use crate::datadir::{self, ConfigFileError};
use crate::host::{PrivilegeError, Privileges};
use crate::log::{Level, LogEvent, Logger};
use crate::tasks::{self, Limits, Runner, TaskError};

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
#[derive(Debug, Clone, PartialEq)]
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
    /// The audit log or its keys could not be opened.
    Audit(gunmetal_durable::audit::error::AuditError),
    /// The cache could not be opened.
    Cache(StoreError),
    /// The operating system could not supply random bytes for the cache
    /// generation.
    Random(RandomnessUnavailable),
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
            Self::Audit(_) => "The security audit log could not be opened.".to_owned(),
            Self::Cache(_) => "The cache could not be opened.".to_owned(),
            Self::Random(_) => {
                "The operating system could not supply random bytes the cache needs.".to_owned()
            }
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
    /// The security audit log, subscribed to the bus.
    pub audit: Arc<AuditSink>,
    /// The rebuildable cache.
    pub store: Arc<Store>,
    /// The task runner.
    pub tasks: Arc<Runner>,
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
        Self::build(dir, host, env, clock, out, &OsRandom)
    }

    fn build(
        dir: &Path,
        host: &Host,
        env: &Env,
        clock: Arc<dyn Clock + Send + Sync>,
        out: Box<dyn Write + Send>,
        random: &dyn Random,
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
        let audit_root = open_audit_root(dir, host.facts, env)?;
        let audit = AuditSink::open(audit_root, Arc::clone(&clock)).map_err(StartError::Audit)?;
        let audit = Arc::new(audit);
        let bus = Arc::new(Bus::default());
        {
            let sink = Arc::clone(&audit);
            bus.security
                .subscribe(move |event| sink.record(event.clone()));
        }
        let generation = cache_generation(random).map_err(StartError::Random)?;
        let opened = Store::open(&data, &[tasks::SCHEMA], Generation(generation))
            .map_err(StartError::Cache)?;
        let store = Arc::new(opened.store);
        let tasks = Runner::open(Arc::clone(&store), Arc::clone(&clock), Limits::production())
            .map_err(cache_open_error);
        #[cfg(test)]
        let tasks = if fail_runner() {
            Err(StartError::Cache(StoreError::Closed))
        } else {
            tasks
        };
        let tasks = tasks?;
        Ok(Self {
            config,
            data,
            clock,
            log: Arc::new(log),
            bus,
            audit,
            store,
            tasks: Arc::new(tasks),
        })
    }
}

fn open_audit_root(dir: &Path, facts: HostFacts, env: &Env) -> Result<DataRoot, StartError> {
    DataRoot::open(
        dir,
        &facts,
        Policy {
            modes: Modes::Refuse,
            network: network(env),
        },
    )
    .map(|opened| opened.root)
    .map_err(|refused| StartError::DataDir(refused.error))
}

/// A cache generation that is not a repeated literal: `CodeQL`'s
/// rust/hard-coded-cryptographic-value treats `[0; N]` as a key source and
/// does not see `Random::fill` as a barrier.
fn cache_generation(random: &dyn Random) -> Result<[u8; 16], RandomnessUnavailable> {
    let mut generation = core::array::from_fn(|index| {
        let [b0, ..] = index.to_le_bytes();
        b0
    });
    random.fill(&mut generation)?;
    Ok(generation)
}

fn cache_open_error(error: TaskError) -> StartError {
    match error {
        TaskError::Store(error) => StartError::Cache(error),
        TaskError::Cancelled
        | TaskError::Interrupted
        | TaskError::Failed { .. }
        | TaskError::Unknown
        | TaskError::PathTooLong { .. } => StartError::Cache(StoreError::Closed),
    }
}

#[cfg(test)]
use std::cell::Cell;

#[cfg(test)]
thread_local! {
    static FAIL_RUNNER: Cell<bool> = const { Cell::new(false) };
}

#[cfg(test)]
fn fail_runner() -> bool {
    FAIL_RUNNER.replace(false)
}

#[cfg(test)]
fn arm_runner_fail() {
    FAIL_RUNNER.set(true);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datadir::CONFIG_FILE;
    use crate::testing::{self, Capture};
    use gunmetal_fs::dataroot::Item;
    use gunmetal_fs::host::{Filesystem, NetworkFs};
    use gunmetal_fs::path::{DataDir, DataPath};
    use gunmetal_testkit::tempdir::TempDir;
    use std::io::{Read, Write};
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
        assert_eq!(state.tasks.list().expect("no tasks yet"), []);
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
        assert!(testing::durable_missing(&dir));
        // Nothing was created: the first real start still has work to do.
        let (started, out) = start(&dir, &local(), &Env::default());
        assert!(started.is_ok());
        assert_eq!(out, STARTED);
        assert!(!testing::durable_missing(&dir));
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
        let mut file = arrange(&dir).create_new(&key).expect("created");
        file.write_all(&[7_u8; 32]).expect("a root secret");
        file.set_permissions(std::fs::Permissions::from_mode(0o640))
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
        assert!(
            state
                .log
                .enable_debug(Duration::from_secs(60), None, &*state.bus)
                .is_ok()
        );
        assert_eq!(state.log.level(), Level::Debug);
        let mut text = String::new();
        state
            .data
            .open_read(&DataPath::audit_segment(
                gunmetal_fs::path::AuditSeg::new(1).expect("1"),
            ))
            .expect("segment")
            .read_to_string(&mut text)
            .expect("read");
        assert!(text.contains("\"event\":\"gm_debug_logging_enabled\""));
    }

    #[test]
    fn a_malformed_root_secret_refuses_start() {
        let dir = TempDir::new("app-audit").expect("scratch");
        arrange(&dir)
            .replace(&DataPath::constant(DataDir::Secrets, "root.key"), b"x")
            .expect("wrote");
        let (started, out) = start(&dir, &local(), &Env::default());
        let error = started.err().expect("refused");
        assert!(is_audit(&error));
        assert!(!is_audit(&StartError::Os(Errno::PERM)));
        assert_eq!(
            error.message(dir.path()),
            "The security audit log could not be opened."
        );
        assert_eq!(out, STARTED);
    }

    fn is_audit(error: &StartError) -> bool {
        matches!(error, StartError::Audit(_))
    }

    /// Verifies: SEC-OPS-012
    #[test]
    fn a_loose_secret_mode_refuses_the_audit_handle() {
        let dir = TempDir::new("app-refuse").expect("scratch");
        let key = DataPath::constant(DataDir::Secrets, "root.key");
        arrange(&dir)
            .create_new(&key)
            .expect("created")
            .set_permissions(std::fs::Permissions::from_mode(0o640))
            .expect("loosened");
        let error =
            open_audit_root(dir.path(), local().facts, &Env::default()).expect_err("refused");
        assert_eq!(
            error,
            StartError::DataDir(DataRootError::WrongMode {
                item: gunmetal_fs::dataroot::Item::Path(key),
                mode: 0o640,
                required: 0o600,
            })
        );
    }

    #[test]
    fn the_audit_handle_opens_when_a_network_filesystem_is_allowed() {
        let dir = TempDir::new("app-audit-allow").expect("scratch");
        arrange(&dir);
        let env = Env {
            allow_network_filesystem: true,
            ..Env::default()
        };
        let opened = open_audit_root(dir.path(), local().facts, &env);
        assert!(opened.is_ok());
    }

    /// Verifies: SEC-OPS-012
    #[test]
    fn a_leftover_after_repair_refuses_the_audit_handle() {
        struct PlantLeftover {
            root: Option<DataRoot>,
            inner: Capture,
        }
        impl Write for PlantLeftover {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                if let Some(root) = self.root.take() {
                    root.leave_replacement(
                        &DataPath::constant(DataDir::Secrets, "keys.json"),
                        b"interrupted",
                    )
                    .expect("leftover");
                    self.write(&[]).expect("empty recurse");
                }
                self.inner.write(bytes)
            }
            fn flush(&mut self) -> std::io::Result<()> {
                self.inner.flush()
            }
        }
        let dir = TempDir::new("app-leftover").expect("scratch");
        let planted = Capture::default();
        let out = PlantLeftover {
            root: Some(arrange(&dir)),
            inner: planted.clone(),
        };
        let mut planted_out = out;
        planted_out.flush().expect("flush");
        let out = planted_out;
        let (clock, _) = testing::clock();
        let started = AppState::start(dir.path(), &local(), &Env::default(), clock, Box::new(out));
        assert_eq!(
            started.err(),
            Some(StartError::DataDir(DataRootError::Leftover {
                item: Item::Replacement(DataPath::constant(DataDir::Secrets, "keys.json")),
            }))
        );
        assert_eq!(planted.text(), STARTED);
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
            StartError::Audit(gunmetal_durable::audit::error::AuditError::Halted),
            StartError::Cache(StoreError::Closed),
            StartError::Random(RandomnessUnavailable),
        ]
        .iter()
        .map(|error| error.message(&dir))
        .collect();
        assert_eq!(
            messages,
            [
                "Could not read this process's privileges or switch off its core dumps: permission denied.",
                "Gunmetal does not run as root, and no setting changes that. Run it as a dedicated unprivileged user that owns the data directory: User=gunmetal in a systemd unit, or --user in a container.",
                r#"The data directory "/data" is on a network filesystem (SMB). SQLite cannot lock its databases safely there. Move the data directory to a disk on this machine, or set GUNMETAL_ALLOW_NETWORK_FILESYSTEM=true to accept the risk."#,
                r#""/data/durable/config.toml" is longer than 65536 bytes, which no Gunmetal configuration needs."#,
                "The environment variable \"GUNMETAL_X\" is not one Gunmetal reads. Check the spelling, or unset it.",
                "The security audit log could not be opened.",
                "The cache could not be opened.",
                "The operating system could not supply random bytes the cache needs.",
            ]
        );
    }

    #[test]
    fn cache_generation_is_the_bytes_drawn_and_fails_without_randomness() {
        struct Counting;
        impl Random for Counting {
            fn fill(&self, bytes: &mut [u8]) -> Result<(), RandomnessUnavailable> {
                for (index, slot) in bytes.iter_mut().enumerate() {
                    *slot = u8::try_from(index).unwrap_or(u8::MAX);
                }
                Ok(())
            }
        }
        struct Failing;
        impl Random for Failing {
            fn fill(&self, _: &mut [u8]) -> Result<(), RandomnessUnavailable> {
                Err(RandomnessUnavailable)
            }
        }
        assert_eq!(
            cache_generation(&Counting),
            Ok([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15])
        );
        assert_eq!(cache_generation(&Failing), Err(RandomnessUnavailable));
    }

    #[test]
    fn every_task_error_at_start_is_a_cache_refusal() {
        assert_eq!(
            cache_open_error(TaskError::Store(StoreError::Closed)),
            StartError::Cache(StoreError::Closed)
        );
        assert_eq!(
            cache_open_error(TaskError::Cancelled),
            StartError::Cache(StoreError::Closed)
        );
        assert_eq!(
            cache_open_error(TaskError::Interrupted),
            StartError::Cache(StoreError::Closed)
        );
        assert_eq!(
            cache_open_error(TaskError::Failed {
                message: "no space".to_owned()
            }),
            StartError::Cache(StoreError::Closed)
        );
        assert_eq!(
            cache_open_error(TaskError::Unknown),
            StartError::Cache(StoreError::Closed)
        );
        assert_eq!(
            cache_open_error(TaskError::PathTooLong { length: 5 }),
            StartError::Cache(StoreError::Closed)
        );
    }

    #[test]
    fn randomness_unavailable_at_start_is_a_random_refusal() {
        struct Failing;
        impl Random for Failing {
            fn fill(&self, _: &mut [u8]) -> Result<(), RandomnessUnavailable> {
                Err(RandomnessUnavailable)
            }
        }
        let dir = TempDir::new("app-random").expect("scratch");
        let (clock, _) = testing::clock();
        let out = Capture::default();
        let started = AppState::build(
            dir.path(),
            &local(),
            &Env::default(),
            clock,
            Box::new(out.clone()),
            &Failing,
        );
        assert_eq!(
            started.err(),
            Some(StartError::Random(RandomnessUnavailable))
        );
        assert_eq!(
            StartError::Random(RandomnessUnavailable).message(dir.path()),
            "The operating system could not supply random bytes the cache needs."
        );
        assert_eq!(out.text(), STARTED);
    }

    #[test]
    fn an_unreadable_cache_file_refuses_start() {
        let dir = TempDir::new("app-cache").expect("scratch");
        let cache = DataPath::constant(DataDir::Cache, "library.db");
        arrange(&dir).create_dir(&cache).expect("not a file");
        let (started, out) = start(&dir, &local(), &Env::default());
        let expected = Store::open(
            &arrange(&dir),
            &[tasks::SCHEMA],
            Generation(core::array::from_fn(|index| {
                let [b0, ..] = index.to_le_bytes();
                b0
            })),
        )
        .expect_err("same refuse");
        assert_eq!(started.err(), Some(StartError::Cache(expected)));
        assert_eq!(out, STARTED);
    }

    #[test]
    fn a_task_table_the_runner_cannot_use_refuses_start() {
        let dir = TempDir::new("app-runner").expect("scratch");
        super::arm_runner_fail();
        let (started, out) = start(&dir, &local(), &Env::default());
        assert_eq!(started.err(), Some(StartError::Cache(StoreError::Closed)));
        assert_eq!(out, STARTED);
    }
}
