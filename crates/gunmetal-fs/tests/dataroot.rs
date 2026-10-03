//! The data-root handle on a real filesystem: layout, owners and modes,
//! file operations and confinement.

#[allow(dead_code, reason = "each test binary uses part of the helpers")]
mod support;

use std::ffi::OsString;
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::symlink;
use std::os::unix::net::UnixListener;

use gunmetal_fs::dataroot::{
    DataRoot, DataRootError, Item, Kind, Modes, NetworkFilesystems, Op, Policy, Refused, Repair,
};
use gunmetal_fs::host::{Filesystem, HostFacts, NetworkFs};
use gunmetal_fs::path::{DataDir, DataPath};
use support::{TempDir, assert_not_root, local_host, mode, names, open, owner, set_mode, uid};

const LAYOUT: [&str; 7] = [
    "backups",
    "cache",
    "derived",
    "durable",
    "secrets",
    "snapshots",
    "tmp",
];

const ROOT_KEY: DataPath = DataPath::constant(DataDir::Secrets, "root.key");
const KEYS: DataPath = DataPath::constant(DataDir::Secrets, "keys.json");
const TLS_KEY: DataPath = DataPath::constant(DataDir::Secrets, "tls/key.pem");
const SEGMENT: DataPath = DataPath::constant(DataDir::Durable, "audit-0001.jsonl");

fn read(root: &DataRoot, path: &DataPath) -> Vec<u8> {
    let mut bytes = Vec::new();
    root.open_read(path)
        .expect("the file opens")
        .read_to_end(&mut bytes)
        .expect("the file reads");
    bytes
}

fn io(item: Item, op: Op, kind: ErrorKind) -> DataRootError {
    DataRootError::Io { item, op, kind }
}

/// What opening returns when it refuses before repairing anything.
fn refused(error: DataRootError) -> Result<Vec<Repair>, Refused> {
    Err(Refused {
        error,
        repairs: vec![],
    })
}

/// Verifies: SEC-OPS-012
#[test]
fn reports_the_repairs_it_made_before_refusing() {
    assert_not_root();
    let dir = TempDir::new();
    drop(open(&dir));
    let secrets = dir.join("data/secrets");
    fs::write(secrets.join("root.key"), b"root").expect("write the key");
    set_mode(&secrets.join("root.key"), 0o644);
    fs::write(secrets.join("secret~"), b"").expect("write a foreign file");
    assert_eq!(
        DataRoot::open(&dir.join("data"), &local_host(), Policy::DEFAULT)
            .map(|opened| opened.repairs),
        Err(Refused {
            error: DataRootError::ForeignEntry {
                parent: Item::Dir(DataDir::Secrets),
                name: OsString::from("secret~")
            },
            repairs: vec![Repair::Mode {
                item: Item::Path(ROOT_KEY),
                from: 0o644,
                to: 0o600
            }]
        })
    );
    assert_eq!(mode(&secrets.join("root.key")), 0o600);
}

/// Verifies: SEC-OPS-012
#[test]
fn creates_the_layout_owned_by_the_service_account_with_mode_0700() {
    assert_not_root();
    let dir = TempDir::new();
    let opened = open(&dir);
    assert_eq!(opened.repairs, []);
    let data = dir.join("data");
    assert_eq!(names(&data), LAYOUT);
    for name in LAYOUT {
        let path = data.join(name);
        assert!(path.is_dir(), "{name} is a directory");
        assert_eq!((owner(&path), mode(&path)), (uid(), 0o700), "{name}");
    }
}

/// Verifies: SEC-OPS-012
#[test]
fn opens_an_existing_layout_without_changing_it() {
    assert_not_root();
    let dir = TempDir::new();
    let first = open(&dir);
    first.root.create_new(&ROOT_KEY).expect("create the key");
    drop(first);
    let second = open(&dir);
    assert_eq!(second.repairs, []);
    assert_eq!(names(&dir.join("data")), LAYOUT);
    assert_eq!(names(&dir.join("data/secrets")), ["root.key"]);
}

/// Verifies: SEC-OPS-012
#[test]
fn tightens_a_loose_data_directory_and_reports_it() {
    assert_not_root();
    let dir = TempDir::new();
    let data = dir.join("data");
    fs::create_dir(&data).expect("create the data directory");
    set_mode(&data, 0o755);
    let opened = open(&dir);
    assert_eq!(
        opened.repairs,
        [Repair::Mode {
            item: Item::Root,
            from: 0o755,
            to: 0o700
        }]
    );
    assert_eq!(mode(&data), 0o700);
}

/// Verifies: SEC-OPS-012
#[test]
fn repairs_a_data_directory_its_owner_cannot_write() {
    assert_not_root();
    let dir = TempDir::new();
    let data = dir.join("data");
    fs::create_dir(&data).expect("create the data directory");
    set_mode(&data, 0o500);
    let opened = open(&dir);
    assert_eq!(
        opened.repairs,
        [Repair::Mode {
            item: Item::Root,
            from: 0o500,
            to: 0o700
        }]
    );
    assert_eq!(names(&data), LAYOUT);
}

/// Verifies: SEC-OPS-012
#[test]
fn repairs_loose_secrets_and_layout_directories_in_order() {
    assert_not_root();
    let dir = TempDir::new();
    drop(open(&dir));
    let data = dir.join("data");
    set_mode(&data.join("cache"), 0o755);
    fs::create_dir(data.join("secrets/tls")).expect("create tls");
    set_mode(&data.join("secrets/tls"), 0o750);
    fs::write(data.join("secrets/tls/key.pem"), b"key").expect("write the key");
    set_mode(&data.join("secrets/tls/key.pem"), 0o640);
    fs::write(data.join("secrets/root.key"), b"root").expect("write the key");
    set_mode(&data.join("secrets/root.key"), 0o644);
    fs::write(data.join("secrets/claim-code"), b"code").expect("write the code");
    set_mode(&data.join("secrets/claim-code"), 0o600);
    let opened = open(&dir);
    assert_eq!(
        opened.repairs,
        [
            Repair::Mode {
                item: Item::Dir(DataDir::Cache),
                from: 0o755,
                to: 0o700
            },
            Repair::Mode {
                item: Item::Path(ROOT_KEY),
                from: 0o644,
                to: 0o600
            },
            Repair::Mode {
                item: Item::Path(DataPath::constant(DataDir::Secrets, "tls")),
                from: 0o750,
                to: 0o700
            },
            Repair::Mode {
                item: Item::Path(TLS_KEY),
                from: 0o640,
                to: 0o600
            },
        ]
    );
    for (rel, expected) in [
        ("cache", 0o700),
        ("secrets/root.key", 0o600),
        ("secrets/claim-code", 0o600),
        ("secrets/tls", 0o700),
        ("secrets/tls/key.pem", 0o600),
    ] {
        assert_eq!(mode(&data.join(rel)), expected, "{rel}");
    }
}

/// Verifies: SEC-OPS-012
#[test]
fn refuses_a_loose_secret_when_told_not_to_repair() {
    assert_not_root();
    let dir = TempDir::new();
    drop(open(&dir));
    let data = dir.join("data");
    fs::write(data.join("secrets/root.key"), b"root").expect("write the key");
    set_mode(&data.join("secrets/root.key"), 0o644);
    let policy = Policy {
        modes: Modes::Refuse,
        network: NetworkFilesystems::Refuse,
    };
    assert_eq!(
        DataRoot::open(&data, &local_host(), policy).map(|opened| opened.repairs),
        refused(DataRootError::WrongMode {
            item: Item::Path(ROOT_KEY),
            mode: 0o644,
            required: 0o600
        })
    );
    assert_eq!(mode(&data.join("secrets/root.key")), 0o644);
}

/// Verifies: SEC-OPS-012
#[test]
fn refuses_a_data_directory_another_user_owns() {
    assert_not_root();
    let dir = TempDir::new();
    let data = dir.join("data");
    fs::create_dir(&data).expect("create the data directory");
    set_mode(&data, 0o755);
    let someone_else = HostFacts {
        uid: uid() + 1,
        filesystem: Filesystem::Local,
    };
    assert_eq!(
        DataRoot::open(&data, &someone_else, Policy::DEFAULT).map(|opened| opened.repairs),
        refused(DataRootError::NotOwned {
            item: Item::Root,
            owner: uid(),
            uid: uid() + 1
        })
    );
    assert_eq!(mode(&data), 0o755);
    assert_eq!(names(&data), [] as [&str; 0]);
}

/// Verifies: SEC-HIS-016, SEC-TM-043, SEC-OPS-012
#[test]
fn refuses_a_symlink_planted_for_the_secrets_directory() {
    assert_not_root();
    let dir = TempDir::new();
    let data = dir.join("data");
    fs::create_dir(&data).expect("create the data directory");
    set_mode(&data, 0o700);
    fs::create_dir(dir.join("elsewhere")).expect("create the target");
    symlink(dir.join("elsewhere"), data.join("secrets")).expect("plant the symlink");
    assert_eq!(
        DataRoot::open(&data, &local_host(), Policy::DEFAULT).map(|opened| opened.repairs),
        refused(DataRootError::WrongKind {
            item: Item::Dir(DataDir::Secrets),
            found: Kind::Symlink
        })
    );
    assert_eq!(names(&dir.join("elsewhere")), [] as [&str; 0]);
}

/// Verifies: SEC-HIS-016, SEC-OPS-012
#[test]
fn refuses_a_symlink_inside_secrets() {
    assert_not_root();
    let dir = TempDir::new();
    drop(open(&dir));
    fs::write(dir.join("elsewhere"), b"not a key").expect("write the target");
    symlink(dir.join("elsewhere"), dir.join("data/secrets/root.key")).expect("plant");
    assert_eq!(
        DataRoot::open(&dir.join("data"), &local_host(), Policy::DEFAULT)
            .map(|opened| opened.repairs),
        refused(DataRootError::WrongKind {
            item: Item::Path(ROOT_KEY),
            found: Kind::Symlink
        })
    );
}

/// Verifies: SEC-OPS-012
#[test]
fn refuses_a_file_where_a_layout_directory_belongs() {
    assert_not_root();
    let dir = TempDir::new();
    let data = dir.join("data");
    fs::create_dir(&data).expect("create the data directory");
    set_mode(&data, 0o700);
    fs::write(data.join("cache"), b"").expect("write a file");
    assert_eq!(
        DataRoot::open(&data, &local_host(), Policy::DEFAULT).map(|opened| opened.repairs),
        refused(DataRootError::WrongKind {
            item: Item::Dir(DataDir::Cache),
            found: Kind::File
        })
    );
}

/// Verifies: SEC-OPS-012
#[test]
fn refuses_a_socket_in_secrets() {
    assert_not_root();
    let dir = TempDir::new();
    drop(open(&dir));
    let _socket = UnixListener::bind(dir.join("data/secrets/agent")).expect("bind");
    assert_eq!(
        DataRoot::open(&dir.join("data"), &local_host(), Policy::DEFAULT)
            .map(|opened| opened.repairs),
        refused(DataRootError::WrongKind {
            item: Item::Path(DataPath::constant(DataDir::Secrets, "agent")),
            found: Kind::Other
        })
    );
}

#[test]
fn refuses_secrets_it_cannot_have_created() {
    assert_not_root();
    let tls = || Item::Path(DataPath::constant(DataDir::Secrets, "tls"));
    for (rel, parent, name) in [
        ("Root.KEY", Item::Dir(DataDir::Secrets), "Root.KEY"),
        (".hidden", Item::Dir(DataDir::Secrets), ".hidden"),
        // Names a crash during a replace cannot leave: no file of that
        // name could have been replaced.
        ("..tmp", Item::Dir(DataDir::Secrets), "..tmp"),
        (
            ".Root.KEY.tmp",
            Item::Dir(DataDir::Secrets),
            ".Root.KEY.tmp",
        ),
        (".keys.json", Item::Dir(DataDir::Secrets), ".keys.json"),
        ("tls/.-x.tmp", tls(), ".-x.tmp"),
    ] {
        let dir = TempDir::new();
        drop(open(&dir));
        fs::create_dir(dir.join("data/secrets/tls")).expect("create tls");
        set_mode(&dir.join("data/secrets/tls"), 0o700);
        fs::create_dir(dir.join("data/secrets").join(rel)).expect("create the entry");
        assert_eq!(
            DataRoot::open(&dir.join("data"), &local_host(), Policy::DEFAULT)
                .map(|opened| opened.repairs),
            refused(DataRootError::ForeignEntry {
                parent,
                name: OsString::from(name)
            }),
            "{rel}"
        );
    }
}

/// Verifies: SEC-OPS-012
#[test]
fn refuses_secrets_nested_deeper_than_a_data_path_reaches() {
    assert_not_root();
    let dir = TempDir::new();
    drop(open(&dir));
    let secrets = dir.join("data/secrets");
    fs::create_dir_all(secrets.join("a/b/c/d/e")).expect("create the nesting");
    for rel in ["a", "a/b", "a/b/c", "a/b/c/d"] {
        set_mode(&secrets.join(rel), 0o750);
    }
    let repair = |rel: &'static str| Repair::Mode {
        item: Item::Path(DataPath::constant(DataDir::Secrets, rel)),
        from: 0o750,
        to: 0o700,
    };
    assert_eq!(
        DataRoot::open(&dir.join("data"), &local_host(), Policy::DEFAULT)
            .map(|opened| opened.repairs),
        Err(Refused {
            error: DataRootError::ForeignEntry {
                parent: Item::Path(DataPath::constant(DataDir::Secrets, "a/b/c/d")),
                name: OsString::from("e")
            },
            repairs: vec![
                repair("a"),
                repair("a/b"),
                repair("a/b/c"),
                repair("a/b/c/d")
            ]
        })
    );
}

#[test]
fn refuses_a_secret_whose_name_is_not_text() {
    assert_not_root();
    let dir = TempDir::new();
    drop(open(&dir));
    let name = OsString::from_vec(vec![b'k', 0xFF]);
    fs::write(dir.join("data/secrets").join(&name), b"").expect("write the file");
    assert_eq!(
        DataRoot::open(&dir.join("data"), &local_host(), Policy::DEFAULT)
            .map(|opened| opened.repairs),
        refused(DataRootError::ForeignEntry {
            parent: Item::Dir(DataDir::Secrets),
            name
        })
    );
}

#[test]
fn refuses_a_data_directory_on_a_network_filesystem_unless_allowed() {
    assert_not_root();
    let dir = TempDir::new();
    let data = dir.join("data");
    fs::create_dir(&data).expect("create the data directory");
    set_mode(&data, 0o700);
    let on_nfs = HostFacts {
        uid: uid(),
        filesystem: Filesystem::Network(NetworkFs::Nfs),
    };
    assert_eq!(
        DataRoot::open(&data, &on_nfs, Policy::DEFAULT).map(|opened| opened.repairs),
        refused(DataRootError::NetworkFilesystem(NetworkFs::Nfs))
    );
    assert_eq!(names(&data), [] as [&str; 0]);
    let allowed = Policy {
        modes: Modes::Repair,
        network: NetworkFilesystems::Allow,
    };
    assert_eq!(
        DataRoot::open(&data, &on_nfs, allowed).map(|opened| opened.repairs),
        Ok(vec![])
    );
    assert_eq!(names(&data), LAYOUT);
}

#[test]
fn reports_a_data_directory_that_is_missing_or_not_a_directory() {
    let dir = TempDir::new();
    assert_eq!(
        DataRoot::open(&dir.join("missing"), &local_host(), Policy::DEFAULT)
            .map(|opened| opened.repairs),
        refused(io(Item::Root, Op::Resolve, ErrorKind::NotFound))
    );
    fs::write(dir.join("file"), b"").expect("write a file");
    assert_eq!(
        DataRoot::open(&dir.join("file"), &local_host(), Policy::DEFAULT)
            .map(|opened| opened.repairs),
        refused(io(Item::Root, Op::OpenRoot, ErrorKind::NotADirectory))
    );
}

#[test]
fn resolves_a_data_directory_reached_through_a_symlink() {
    assert_not_root();
    let dir = TempDir::new();
    drop(open(&dir));
    symlink(dir.join("data"), dir.join("alias")).expect("plant the alias");
    let opened = DataRoot::open(&dir.join("alias"), &local_host(), Policy::DEFAULT)
        .expect("the data root opens");
    opened.root.create_new(&ROOT_KEY).expect("create the key");
    assert_eq!(names(&dir.join("data/secrets")), ["root.key"]);
}

/// Verifies: SEC-OPS-012, SEC-PRV-045
#[test]
fn creates_files_with_mode_0600_and_only_once() {
    assert_not_root();
    let dir = TempDir::new();
    let root = open(&dir).root;
    let mut file = root.create_new(&ROOT_KEY).expect("create the key");
    file.write_all(b"secret").expect("write the key");
    drop(file);
    let path = dir.join("data/secrets/root.key");
    assert_eq!((owner(&path), mode(&path)), (uid(), 0o600));
    assert_eq!(read(&root, &ROOT_KEY), b"secret");
    assert_eq!(
        root.create_new(&ROOT_KEY).map(|_| ()),
        Err(io(
            Item::Path(ROOT_KEY),
            Op::Create,
            ErrorKind::AlreadyExists
        ))
    );
    assert_eq!(read(&root, &ROOT_KEY), b"secret");
}

/// Verifies: SEC-PRV-045
#[test]
fn appends_to_a_log_segment_created_with_mode_0600() {
    assert_not_root();
    let dir = TempDir::new();
    let root = open(&dir).root;
    for line in [&b"one\n"[..], b"two\n"] {
        root.append(&SEGMENT)
            .expect("open the segment")
            .write_all(line)
            .expect("append");
    }
    assert_eq!(read(&root, &SEGMENT), b"one\ntwo\n");
    assert_eq!(mode(&dir.join("data/durable/audit-0001.jsonl")), 0o600);
}

/// Verifies: SEC-OPS-012
#[test]
fn replaces_a_file_whole_and_leaves_nothing_beside_it() {
    assert_not_root();
    let dir = TempDir::new();
    let root = open(&dir).root;
    root.replace(&KEYS, b"first").expect("write the keys");
    root.replace(&KEYS, b"second").expect("replace the keys");
    assert_eq!(read(&root, &KEYS), b"second");
    assert_eq!(names(&dir.join("data/secrets")), ["keys.json"]);
    assert_eq!(mode(&dir.join("data/secrets/keys.json")), 0o600);
}

/// Verifies: SEC-OPS-012
#[test]
fn replaces_over_a_temporary_file_a_crash_left_behind() {
    assert_not_root();
    let dir = TempDir::new();
    let root = open(&dir).root;
    let stale = dir.join("data/secrets/.keys.json.tmp");
    fs::write(&stale, b"half written").expect("leave a temporary file");
    set_mode(&stale, 0o644);
    root.replace(&KEYS, b"whole").expect("replace the keys");
    assert_eq!(read(&root, &KEYS), b"whole");
    assert_eq!(names(&dir.join("data/secrets")), ["keys.json"]);
    assert_eq!(mode(&dir.join("data/secrets/keys.json")), 0o600);
}

/// Leaves `secrets/` as a crash during a replace of `keys.json` and of
/// `tls/key.pem` would: the old files in place and a temporary file beside
/// each.
fn interrupted_replaces(dir: &TempDir) {
    drop(open(dir));
    let secrets = dir.join("data/secrets");
    fs::write(secrets.join("keys.json"), b"old keys").expect("write the keys");
    set_mode(&secrets.join("keys.json"), 0o600);
    fs::write(secrets.join(".keys.json.tmp"), b"half written").expect("leave a temporary file");
    set_mode(&secrets.join(".keys.json.tmp"), 0o600);
    fs::create_dir(secrets.join("tls")).expect("create tls");
    set_mode(&secrets.join("tls"), 0o700);
    fs::write(secrets.join("tls/.key.pem.tmp"), b"half").expect("leave a temporary file");
    set_mode(&secrets.join("tls/.key.pem.tmp"), 0o644);
}

/// Verifies: SEC-OPS-012
#[test]
fn opens_after_a_crash_during_a_replace_and_removes_what_it_left() {
    assert_not_root();
    let dir = TempDir::new();
    interrupted_replaces(&dir);
    let opened = DataRoot::open(&dir.join("data"), &local_host(), Policy::DEFAULT)
        .expect("the data root opens");
    assert_eq!(
        opened.repairs,
        [
            Repair::Removed {
                item: Item::Replacement(KEYS)
            },
            Repair::Removed {
                item: Item::Replacement(TLS_KEY)
            },
        ]
    );
    assert_eq!(names(&dir.join("data/secrets")), ["keys.json", "tls"]);
    assert_eq!(names(&dir.join("data/secrets/tls")), [] as [&str; 0]);
    assert_eq!(read(&opened.root, &KEYS), b"old keys");
}

/// Verifies: SEC-OPS-012
#[test]
fn refuses_what_a_crash_during_a_replace_left_when_told_not_to_repair() {
    assert_not_root();
    let dir = TempDir::new();
    interrupted_replaces(&dir);
    let policy = Policy {
        modes: Modes::Refuse,
        network: NetworkFilesystems::Refuse,
    };
    assert_eq!(
        DataRoot::open(&dir.join("data"), &local_host(), policy).map(|opened| opened.repairs),
        refused(DataRootError::Leftover {
            item: Item::Replacement(KEYS)
        })
    );
    assert_eq!(
        names(&dir.join("data/secrets")),
        [".keys.json.tmp", "keys.json", "tls"]
    );
}

/// Verifies: SEC-HIS-016, SEC-OPS-012
#[test]
fn refuses_a_symlink_named_like_a_temporary_file() {
    assert_not_root();
    let dir = TempDir::new();
    drop(open(&dir));
    fs::write(dir.join("elsewhere"), b"not ours").expect("write the target");
    symlink(
        dir.join("elsewhere"),
        dir.join("data/secrets/.keys.json.tmp"),
    )
    .expect("plant");
    assert_eq!(
        DataRoot::open(&dir.join("data"), &local_host(), Policy::DEFAULT)
            .map(|opened| opened.repairs),
        refused(DataRootError::WrongKind {
            item: Item::Replacement(KEYS),
            found: Kind::Symlink
        })
    );
    assert_eq!(names(&dir.join("data/secrets")), [".keys.json.tmp"]);
    assert_eq!(fs::read(dir.join("elsewhere")).expect("read"), b"not ours");
}

#[test]
fn replaces_a_file_in_a_nested_directory() {
    assert_not_root();
    let dir = TempDir::new();
    let root = open(&dir).root;
    let log = DataPath::constant(DataDir::Durable, "log");
    let segment = DataPath::constant(DataDir::Durable, "log/2026-10.seg");
    root.create_dir(&log).expect("create the log directory");
    root.replace(&segment, b"frames")
        .expect("write the segment");
    assert_eq!(read(&root, &segment), b"frames");
    assert_eq!(names(&dir.join("data/durable/log")), ["2026-10.seg"]);
}

#[test]
fn reports_files_in_a_directory_that_does_not_exist() {
    let dir = TempDir::new();
    let root = open(&dir).root;
    let missing = DataPath::constant(DataDir::Durable, "log/2026-10.seg");
    let item = || Item::Path(missing.clone());
    assert_eq!(
        root.create_new(&missing).map(|_| ()),
        Err(io(item(), Op::Create, ErrorKind::NotFound))
    );
    assert_eq!(
        root.append(&missing).map(|_| ()),
        Err(io(item(), Op::Open, ErrorKind::NotFound))
    );
    assert_eq!(
        root.open_read(&missing).map(|_| ()),
        Err(io(item(), Op::Open, ErrorKind::NotFound))
    );
    assert_eq!(
        root.replace(&missing, b"x"),
        Err(io(item(), Op::Create, ErrorKind::NotFound))
    );
    assert_eq!(
        root.create_dir(&DataPath::constant(DataDir::Durable, "log/main")),
        Err(io(
            Item::Path(DataPath::constant(DataDir::Durable, "log/main")),
            Op::CreateDir,
            ErrorKind::NotFound
        ))
    );
}

/// Verifies: SEC-OPS-012
#[test]
fn creates_directories_with_mode_0700_and_only_once() {
    assert_not_root();
    let dir = TempDir::new();
    let root = open(&dir).root;
    let log = DataPath::constant(DataDir::Durable, "log");
    root.create_dir(&log).expect("create the log directory");
    assert_eq!(mode(&dir.join("data/durable/log")), 0o700);
    assert_eq!(
        root.create_dir(&log),
        Err(io(Item::Path(log), Op::CreateDir, ErrorKind::AlreadyExists))
    );
}

/// Verifies: SEC-HIS-016, SEC-MED-033, SEC-TM-043
#[test]
fn refuses_symlinks_that_lead_out_of_the_root() {
    assert_not_root();
    let dir = TempDir::new();
    let root = open(&dir).root;
    fs::create_dir(dir.join("outside")).expect("create the outside directory");
    fs::write(dir.join("outside/secret"), b"outside").expect("write the outside file");
    symlink(dir.join("outside"), dir.join("data/durable/escape")).expect("plant");
    symlink(dir.join("outside/secret"), dir.join("data/durable/leak")).expect("plant");
    let escape = DataPath::constant(DataDir::Durable, "escape/planted");
    let leak = DataPath::constant(DataDir::Durable, "leak");
    let denied =
        |path: &DataPath, op| io(Item::Path(path.clone()), op, ErrorKind::PermissionDenied);
    assert_eq!(
        root.create_new(&escape).map(|_| ()),
        Err(denied(&escape, Op::Create))
    );
    assert_eq!(
        root.append(&escape).map(|_| ()),
        Err(denied(&escape, Op::Open))
    );
    assert_eq!(
        root.replace(&escape, b"x"),
        Err(denied(&escape, Op::Create))
    );
    assert_eq!(
        root.create_dir(&escape),
        Err(denied(&escape, Op::CreateDir))
    );
    assert_eq!(
        root.open_read(&leak).map(|_| ()),
        Err(denied(&leak, Op::Open))
    );
    assert_eq!(names(&dir.join("outside")), ["secret"]);
    assert_eq!(
        fs::read(dir.join("outside/secret")).expect("read"),
        b"outside"
    );
}

/// Verifies: SEC-TM-043, SEC-HIS-016
#[test]
fn keeps_working_in_the_directory_it_opened_after_the_path_is_swapped() {
    assert_not_root();
    let dir = TempDir::new();
    let root = open(&dir).root;
    fs::rename(dir.join("data"), dir.join("moved")).expect("move the data directory");
    fs::create_dir(dir.join("data")).expect("plant a new directory at the old path");
    root.create_new(&ROOT_KEY).expect("create the key");
    assert_eq!(names(&dir.join("moved/secrets")), ["root.key"]);
    assert_eq!(names(&dir.join("data")), [] as [&str; 0]);
}
