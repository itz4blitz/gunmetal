//! `gunmetal serve`, run as the binary runs it, in this test process: the
//! real probes, the real system calls and a real data directory.
//!
//! These tests are apart from the unit tests because `serve` switches core
//! dumps off for the whole process it runs in.

use std::ffi::OsString;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::sync::{Arc, Mutex};

use gunmetal_fs::dataroot::{DataRoot, Modes, NetworkFilesystems, Policy};
use gunmetal_fs::host::HostFacts;
use gunmetal_fs::path::{DataDir, DataPath};
use gunmetal_server::cli::{self, Exit};
use gunmetal_testkit::tempdir::TempDir;
use rustix::process::{DumpableBehavior, Resource, Rlimit};

/// Output a test can read back.
#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().expect("capture").extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Capture {
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().expect("capture").clone()).expect("UTF-8 output")
    }
}

/// Runs `gunmetal` with `args` and the environment `vars`, and returns its
/// exit, its standard output and what it told the console.
fn gunmetal(args: &[&str], vars: &[(&str, &str)]) -> (Exit, String, String) {
    let args: Vec<OsString> = args.iter().map(OsString::from).collect();
    let vars = vars
        .iter()
        .map(|(name, value)| (OsString::from(name), OsString::from(value)))
        .collect();
    let out = Capture::default();
    let mut err = Capture::default();
    let exit = cli::run(&args, vars, Box::new(out.clone()), &mut err);
    (exit, out.text(), err.text())
}

fn path(dir: &TempDir) -> &str {
    dir.path().to_str().expect("a UTF-8 temporary directory")
}

/// The facts about this host, which must not be running the suite as root:
/// the server would refuse to start, and root ignores the modes checked
/// here.
fn facts(dir: &TempDir) -> HostFacts {
    let facts = HostFacts::probe(dir.path()).expect("probed");
    assert_ne!(facts.uid, 0);
    facts
}

/// Opens the data directory as a check that changes nothing does: it opens
/// only when every owner and mode in the layout is already right.
fn open_strictly(dir: &TempDir) -> Option<usize> {
    let strict = Policy {
        modes: Modes::Refuse,
        network: NetworkFilesystems::Refuse,
    };
    DataRoot::open(dir.path(), &facts(dir), strict)
        .ok()
        .map(|opened| opened.repairs.len())
}

/// The one line `serve` logs on a clean start, without its time.
fn started(out: &str) -> bool {
    let line = format!(
        "\"level\":\"info\",\"event\":\"sys_startup\",\"version\":\"{}\"}}\n",
        env!("CARGO_PKG_VERSION")
    );
    out.starts_with("{\"ts\":\"20") && out.ends_with(&line) && out.matches('\n').count() == 1
}

/// Verifies: SEC-STD-023
#[test]
fn serve_on_an_empty_directory_creates_the_layout_and_switches_core_dumps_off() {
    let dir = TempDir::new("serve-empty").expect("scratch");
    let (exit, out, err) = gunmetal(&["serve", "--data-dir", path(&dir)], &[]);
    assert_eq!((exit, err.as_str()), (Exit::Ok, ""));
    assert!(started(&out));
    // Every layout directory exists with the owner and mode it must have.
    assert_eq!(open_strictly(&dir), Some(0));
    for layout in DataDir::ALL {
        let probe = DataPath::constant(layout, "probe");
        let root = DataRoot::open(dir.path(), &facts(&dir), Policy::DEFAULT)
            .expect("opens")
            .root;
        let mode = root
            .create_new(&probe)
            .map(|file| file.metadata().expect("metadata").permissions().mode());
        assert_eq!(mode, Ok(0o100_600));
    }
    // The process can no longer dump core, and reads its own flag as such.
    assert_eq!(
        rustix::process::dumpable_behavior(),
        Ok(DumpableBehavior::NotDumpable)
    );
    assert_eq!(
        rustix::process::getrlimit(Resource::Core),
        Rlimit {
            current: Some(0),
            maximum: Some(0),
        }
    );
}

#[test]
fn serve_takes_the_data_directory_from_the_environment_and_the_flag_wins() {
    let from_env = TempDir::new("serve-env").expect("scratch");
    let from_flag = TempDir::new("serve-flag").expect("scratch");
    let vars = [("GUNMETAL_DATA_DIR", path(&from_env)), ("HOME", "/nowhere")];
    let (exit, out, err) = gunmetal(&["serve"], &vars);
    assert_eq!((exit, err.as_str()), (Exit::Ok, ""));
    assert!(started(&out));
    assert_eq!(open_strictly(&from_env), Some(0));
    let (exit, _, err) = gunmetal(&["--data-dir", path(&from_flag), "serve"], &vars);
    assert_eq!((exit, err.as_str()), (Exit::Ok, ""));
    assert_eq!(open_strictly(&from_flag), Some(0));
}

#[test]
fn serve_tightens_a_loose_secret_and_says_so_in_the_log() {
    let dir = TempDir::new("serve-repair").expect("scratch");
    let key = DataPath::constant(DataDir::Secrets, "root.key");
    DataRoot::open(dir.path(), &facts(&dir), Policy::DEFAULT)
        .expect("opens")
        .root
        .create_new(&key)
        .expect("created")
        .set_permissions(std::fs::Permissions::from_mode(0o644))
        .expect("loosened");
    assert_eq!(open_strictly(&dir), None);
    let (exit, out, err) = gunmetal(&["serve", "--data-dir", path(&dir)], &[]);
    assert_eq!((exit, err.as_str()), (Exit::Ok, ""));
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].ends_with(
        "\"level\":\"warn\",\"event\":\"gm_data_root_mode_repaired\",\"item\":\"secrets/root.key\",\"from\":420,\"to\":384}"
    ));
    assert_eq!(open_strictly(&dir), Some(0));
}

#[test]
fn serve_refuses_a_data_directory_that_does_not_exist() {
    let dir = TempDir::new("serve-missing").expect("scratch");
    let missing = format!("{}/not-here", path(&dir));
    assert_eq!(
        gunmetal(&["serve", "--data-dir", &missing], &[]),
        (
            Exit::DataDir,
            String::new(),
            format!(
                "gunmetal: The data directory {missing} does not exist. Create it as the user Gunmetal runs as: mkdir -m 700 '{missing}'\n"
            )
        )
    );
}

/// Verifies: SEC-OPS-014
#[test]
fn serve_refuses_a_secret_in_the_environment_before_it_opens_anything() {
    let dir = TempDir::new("serve-secret").expect("scratch");
    let vars = [
        ("GUNMETAL_DATA_DIR", path(&dir)),
        ("GUNMETAL_OIDC_CLIENT_SECRET", "canary-oidc-secret-9d2e"),
    ];
    let (exit, out, err) = gunmetal(&["serve"], &vars);
    assert_eq!((exit, out.as_str()), (Exit::Config, ""));
    assert_eq!(
        err,
        "gunmetal: GUNMETAL_OIDC_CLIENT_SECRET holds a secret in a plain environment variable, which other processes and crash reports can read. Put the secret in a file only Gunmetal's user can read and set GUNMETAL_OIDC_CLIENT_SECRET_FILE to that file's path, or pass it as a systemd credential, then unset GUNMETAL_OIDC_CLIENT_SECRET.\n"
    );
    // Nothing was created: a strict open of the untouched directory finds
    // the layout missing and makes it, with nothing to repair.
    let secrets = DataPath::constant(DataDir::Secrets, "probe");
    let root = DataRoot::open(dir.path(), &facts(&dir), Policy::DEFAULT)
        .expect("opens")
        .root;
    assert!(root.create_new(&secrets).is_ok());
}

#[test]
fn help_and_an_unbuilt_subcommand_use_the_library_entry() {
    assert_eq!(gunmetal(&["--help"], &[]).0, Exit::Ok);
    assert_eq!(gunmetal(&["doctor"], &[]).0, Exit::Unavailable);
    let bus = gunmetal_server::bus::Bus::default();
    bus.security.subscribe(|_| Ok(()));
    assert_eq!(
        gunmetal_core::audit_event::SecuritySink::record(
            &bus,
            gunmetal_core::audit_event::SecurityEvent::GmDebugLoggingEnabled { account: None }
        ),
        Ok(())
    );
    assert_eq!(
        gunmetal_core::audit_event::SecuritySink::record(
            &gunmetal_server::bus::Bus::default(),
            gunmetal_core::audit_event::SecurityEvent::GmDebugLoggingEnabled { account: None }
        ),
        Err(gunmetal_core::audit_event::AuditUnavailable)
    );
}

#[test]
fn serve_refuses_a_configuration_file_with_an_unknown_key() {
    let dir = TempDir::new("serve-config").expect("scratch");
    DataRoot::open(dir.path(), &facts(&dir), Policy::DEFAULT)
        .expect("opens")
        .root
        .replace(
            &DataPath::constant(DataDir::Durable, "config.toml"),
            b"[log]\nlevel = \"warn\"\n\n[server]\nrequire_sign_in = false\n",
        )
        .expect("written");
    assert_eq!(
        gunmetal(&["serve", "--data-dir", path(&dir)], &[]),
        (
            Exit::Config,
            String::new(),
            "gunmetal: Line 4 of the configuration file sets \"server\", which is not a Gunmetal setting. Check the spelling, or remove it.\n".to_owned()
        )
    );
}
