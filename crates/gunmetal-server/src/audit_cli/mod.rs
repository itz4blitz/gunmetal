//! `gunmetal audit verify`: walk the hash chain of the security audit log
//! (WP-069).

use std::ffi::OsString;
use std::io::Write;

use gunmetal_durable::audit::error::BrokenAt;

use crate::app::AppState;
use crate::cli::{self, Command, Exit};

/// Opens the data directory and reports whether the audit log's chain holds.
pub fn verify(
    command: &Command,
    vars: Vec<(OsString, OsString)>,
    mut out: Box<dyn Write + Send>,
    err: &mut dyn Write,
) -> Exit {
    let env = crate::config::Env::read(vars);
    let from_env = env.as_ref().ok().and_then(|env| env.data_dir.as_ref());
    let dir = crate::datadir::choose(command.data_dir.as_ref(), from_env);
    let started = cli::start_from_probes(
        crate::host::disable_core_dumps().and_then(|()| crate::host::Privileges::probe()),
        gunmetal_fs::host::HostFacts::probe(&dir),
        &dir,
        env,
        Box::new(std::io::sink()),
    );
    match started {
        Ok(state) => report(&state, &mut *out, err),
        Err(error) => {
            say(err, &format!("gunmetal: {}", error.message(&dir)));
            Exit::of(&error)
        }
    }
}

fn report(state: &AppState, out: &mut dyn Write, err: &mut dyn Write) -> Exit {
    match state.audit.log.verify_audit_log() {
        Ok(()) => {
            say(out, "audit log: ok");
            Exit::Ok
        }
        Err(BrokenAt { seq }) => {
            say(err, &format!("audit log: broken at sequence {seq}"));
            Exit::DataDir
        }
    }
}

fn say(out: &mut dyn Write, text: &str) {
    let _ = writeln!(out, "{text}");
}

#[cfg(test)]
mod tests {
    use super::{report, say, verify};
    use crate::app::{AppState, Host};
    use crate::cli::{Action, Command, Exit};
    use crate::config::Env;
    use crate::host::Privileges;
    use crate::testing::{self, Capture};
    use gunmetal_core::audit_event::{SecurityEvent, SecuritySink};
    use gunmetal_fs::host::HostFacts;
    use gunmetal_fs::path::{AuditSeg, DataPath};
    use gunmetal_testkit::tempdir::TempDir;
    use std::ffi::OsString;
    use std::io::Read;
    use std::path::PathBuf;

    fn uid() -> u32 {
        let uid = rustix::process::geteuid().as_raw();
        assert_ne!(uid, 0);
        uid
    }

    fn local() -> Host {
        Host {
            privileges: Privileges {
                euid: uid(),
                effective: 0,
                permitted: 0,
            },
            facts: HostFacts {
                uid: uid(),
                filesystem: gunmetal_fs::host::Filesystem::Local,
            },
        }
    }

    fn started(dir: &TempDir) -> AppState {
        let (clock, _) = testing::clock();
        AppState::start(
            dir.path(),
            &local(),
            &Env::default(),
            clock,
            Box::new(Capture::default()),
        )
        .expect("started")
    }

    #[test]
    fn a_fresh_log_verifies() {
        let dir = TempDir::new("audit-cli-ok").expect("scratch");
        let state = started(&dir);
        let mut out = Capture::default();
        let mut err = Capture::default();
        assert_eq!(report(&state, &mut out, &mut err), Exit::Ok);
        assert_eq!(out.text(), "audit log: ok\n");
        assert_eq!(err.text(), "");
    }

    #[test]
    fn a_broken_chain_is_named() {
        let dir = TempDir::new("audit-cli-broken").expect("scratch");
        let state = started(&dir);
        state
            .bus
            .record(SecurityEvent::GmEgressDenied {})
            .expect("recorded");
        let path = DataPath::audit_segment(AuditSeg::new(1).expect("1"));
        let mut text = String::new();
        state
            .data
            .open_read(&path)
            .expect("open")
            .read_to_string(&mut text)
            .expect("read");
        let broken = text.replace("gm_egress_denied", "gm_egress_denieX");
        state.data.replace(&path, broken.as_bytes()).expect("wrote");
        drop(state);
        let reopened = started(&dir);
        let mut out = Capture::default();
        let mut err = Capture::default();
        assert_eq!(report(&reopened, &mut out, &mut err), Exit::DataDir);
        assert_eq!(err.text(), "audit log: broken at sequence 1\n");
        assert_eq!(out.text(), "");
    }

    #[test]
    fn verify_opens_the_data_directory() {
        let dir = TempDir::new("audit-cli-run").expect("scratch");
        let _ = started(&dir);
        let out = Capture::default();
        let mut err = Capture::default();
        let command = Command {
            action: Action::AuditVerify,
            data_dir: Some(OsString::from(dir.path())),
        };
        let exit = verify(&command, Vec::new(), Box::new(out.clone()), &mut err);
        assert_eq!(exit, Exit::Ok);
        assert_eq!(out.text(), "audit log: ok\n");
    }

    #[test]
    fn a_missing_data_directory_is_refused() {
        let missing = PathBuf::from("/no/such/gunmetal-audit-verify");
        let out = Capture::default();
        let mut err = Capture::default();
        let command = Command {
            action: Action::AuditVerify,
            data_dir: Some(OsString::from(&missing)),
        };
        let exit = verify(&command, Vec::new(), Box::new(out.clone()), &mut err);
        assert_eq!(exit, Exit::DataDir);
        assert!(err.text().contains("gunmetal:"));
    }

    #[test]
    fn say_writes_a_line() {
        let mut out = Capture::default();
        say(&mut out, "hello");
        assert_eq!(out.text(), "hello\n");
    }
}
