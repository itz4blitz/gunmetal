//! The data directory: where it is, the configuration file inside it, and
//! what to tell the admin when the data root refuses it.
//!
//! The directory is opened only through the data-root handle of
//! `gunmetal-fs` (WP-126), which creates the layout, checks owners and
//! modes and refuses a network filesystem. This module adds the server's
//! side: the configuration file `durable/config.toml`, read through that
//! handle, and a message for every refusal that names the fix, such as the
//! exact `chmod` to run (SEC-OPS-012, ADM-079).

use std::ffi::OsString;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use gunmetal_fs::dataroot::{DataRoot, DataRootError, Item, Kind, Op};
use gunmetal_fs::host::NetworkFs;
use gunmetal_fs::path::{DataDir, DataPath};

/// Where the data directory is when neither the command line nor the
/// environment says.
pub const DEFAULT_DATA_DIR: &str = "/var/lib/gunmetal";

/// The configuration file, inside the data directory.
pub const CONFIG_FILE: DataPath = DataPath::constant(DataDir::Durable, "config.toml");

/// The largest configuration file the server reads, in bytes.
pub const MAX_CONFIG_BYTES: usize = 64 * 1024;

/// How much of the file is read: one byte past the cap is enough to tell
/// that it is too long.
const READ_LIMIT: u64 = MAX_CONFIG_BYTES as u64 + 1;

/// Why the configuration file could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigFileError {
    /// The data root could not open it.
    Open(DataRootError),
    /// Reading it failed.
    Read(io::ErrorKind),
    /// It is longer than [`MAX_CONFIG_BYTES`].
    TooLarge,
    /// It is not UTF-8.
    NotUtf8,
}

impl ConfigFileError {
    /// What to tell the admin, for the data directory at `dir`.
    #[must_use]
    pub fn message(&self, dir: &Path) -> String {
        let file = item_path(dir, &Item::Path(CONFIG_FILE));
        match self {
            Self::Open(error) => explain(error, dir),
            Self::Read(kind) => format!("Could not read {file}: {kind}."),
            Self::TooLarge => format!(
                "{file} is longer than {MAX_CONFIG_BYTES} bytes, which no Gunmetal configuration needs."
            ),
            Self::NotUtf8 => format!("{file} is not UTF-8 text."),
        }
    }
}

/// Treats a configuration file that does not exist as an empty one.
fn absent_is_empty(opened: Result<File, DataRootError>) -> Result<Option<File>, ConfigFileError> {
    match opened {
        Ok(file) => Ok(Some(file)),
        Err(DataRootError::Io {
            kind: io::ErrorKind::NotFound,
            ..
        }) => Ok(None),
        Err(error) => Err(ConfigFileError::Open(error)),
    }
}

/// Reads an already-opened configuration file; a missing file reads as empty.
fn read_file(opened: Result<File, DataRootError>) -> Result<String, ConfigFileError> {
    let Some(file) = absent_is_empty(opened)? else {
        return Ok(String::new());
    };
    let mut bytes = Vec::new();
    file.take(READ_LIMIT)
        .read_to_end(&mut bytes)
        .map_err(|error| ConfigFileError::Read(error.kind()))?;
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(ConfigFileError::TooLarge);
    }
    String::from_utf8(bytes).map_err(|_| ConfigFileError::NotUtf8)
}

/// Reads the configuration file; a missing file reads as empty.
///
/// # Errors
///
/// A [`ConfigFileError`] when the file cannot be opened or read, is too
/// long, or is not UTF-8.
pub fn read_config(root: &DataRoot) -> Result<String, ConfigFileError> {
    read_file(root.open_read(&CONFIG_FILE))
}

/// The data directory: the one the command line names, else the one the
/// environment names, else [`DEFAULT_DATA_DIR`].
#[must_use]
pub fn choose(command_line: Option<&OsString>, environment: Option<&PathBuf>) -> PathBuf {
    command_line
        .map(PathBuf::from)
        .or_else(|| environment.cloned())
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DATA_DIR))
}

/// `item`'s path relative to the data directory, or `None` for the data
/// directory itself. A replacement is named by the temporary file it
/// writes, `.<name>.tmp` beside the file it replaces.
fn relative(item: &Item) -> Option<String> {
    match item {
        Item::Root => None,
        Item::Dir(dir) => Some(dir.name().to_owned()),
        Item::Path(path) => Some(format!("{}/{}", path.dir().name(), path.rel())),
        Item::Replacement(path) => {
            let (head, last) = path
                .rel()
                .rsplit_once('/')
                .map_or((String::new(), path.rel()), |(head, last)| {
                    (format!("{head}/"), last)
                });
            Some(format!("{}/{head}.{last}.tmp", path.dir().name()))
        }
    }
}

/// `item` named relative to the data directory, as the log writes it; the
/// data directory itself is `.`.
#[must_use]
pub fn item_name(item: &Item) -> String {
    relative(item).unwrap_or_else(|| ".".to_owned())
}

/// `item`'s full path, for a message to the admin.
fn item_path(dir: &Path, item: &Item) -> String {
    match relative(item) {
        Some(rel) => format!("{}/{rel}", dir.display()),
        None => dir.display().to_string(),
    }
}

/// `text` quoted for a POSIX shell, so a hint can be pasted as it is.
fn quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

/// What an operation is called in a message.
const fn verb(op: Op) -> &'static str {
    match op {
        Op::Probe => "examine",
        Op::Resolve => "resolve",
        Op::OpenRoot | Op::Open => "open",
        Op::Inspect => "inspect",
        Op::Repair => "change the mode of",
        Op::List => "list",
        Op::CreateDir => "create the directory",
        Op::Create => "create",
        Op::Write => "write",
        Op::Rename => "rename",
        Op::Sync => "sync",
        Op::Remove => "remove",
    }
}

/// What a kind of object is called in a message.
const fn noun(kind: Kind) -> &'static str {
    match kind {
        Kind::File => "regular file",
        Kind::Dir => "directory",
        Kind::Symlink => "symbolic link",
        Kind::Other => "socket, FIFO or device",
    }
}

/// What a network filesystem is called in a message.
const fn filesystem(kind: NetworkFs) -> &'static str {
    match kind {
        NetworkFs::Nfs => "NFS",
        NetworkFs::Smb => "SMB",
        NetworkFs::NineP => "9p",
        NetworkFs::Fuse => "FUSE",
    }
}

/// What to tell the admin when the data root refuses the data directory at
/// `dir`, naming the fix.
#[must_use]
pub fn explain(error: &DataRootError, dir: &Path) -> String {
    let root = dir.display();
    match error {
        DataRootError::NetworkFilesystem(kind) => format!(
            "The data directory {root} is on a network filesystem ({}). SQLite cannot lock its databases safely there. Move the data directory to a disk on this machine, or set GUNMETAL_ALLOW_NETWORK_FILESYSTEM=true to accept the risk.",
            filesystem(*kind)
        ),
        DataRootError::Io {
            item: Item::Root,
            kind: io::ErrorKind::NotFound,
            ..
        } => format!(
            "The data directory {root} does not exist. Create it as the user Gunmetal runs as: mkdir -m 700 {}",
            quoted(&root.to_string())
        ),
        DataRootError::Io { item, op, kind } => {
            format!("Could not {} {}: {kind}.", verb(*op), item_path(dir, item))
        }
        DataRootError::NotOwned { item, owner, uid } => {
            let path = item_path(dir, item);
            format!(
                "{path} belongs to user {owner}, but Gunmetal runs as user {uid}. As root, run: chown {uid} {}",
                quoted(&path)
            )
        }
        DataRootError::WrongKind { item, found } => format!(
            "{} is a {}, which does not belong there. Move it out of the data directory.",
            item_path(dir, item),
            noun(*found)
        ),
        DataRootError::WrongMode {
            item,
            mode,
            required,
        } => {
            let path = item_path(dir, item);
            format!(
                "{path} has mode {mode:o}, which lets other users in. Run: chmod {required:o} {}",
                quoted(&path)
            )
        }
        DataRootError::CannotRepair {
            item,
            mode,
            required,
        } => format!(
            "{} still has mode {mode:o} after Gunmetal set {required:o}: its filesystem does not keep permissions. Move the data directory to a disk on this machine.",
            item_path(dir, item)
        ),
        DataRootError::Leftover { item } => format!(
            "{} was left by an interrupted write. Remove it, then start Gunmetal again.",
            item_path(dir, item)
        ),
        DataRootError::ForeignEntry { parent, name } => format!(
            "{}/{} was not created by Gunmetal. Move it out of the secrets directory.",
            item_path(dir, parent),
            name.to_string_lossy()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gunmetal_fs::dataroot::Policy;
    use gunmetal_fs::host::{Filesystem, HostFacts};
    use gunmetal_testkit::tempdir::TempDir;

    const KEY: DataPath = DataPath::constant(DataDir::Secrets, "root.key");
    const NESTED: DataPath = DataPath::constant(DataDir::Secrets, "tls/cert.pem");

    fn dir() -> PathBuf {
        PathBuf::from("/srv/gun metal's")
    }

    #[test]
    fn names_items_relative_to_the_data_directory() {
        let names: Vec<String> = [
            Item::Root,
            Item::Dir(DataDir::Cache),
            Item::Path(NESTED),
            Item::Replacement(KEY),
            Item::Replacement(NESTED),
        ]
        .iter()
        .map(item_name)
        .collect();
        assert_eq!(
            names,
            [
                ".",
                "cache",
                "secrets/tls/cert.pem",
                "secrets/.root.key.tmp",
                "secrets/tls/.cert.pem.tmp"
            ]
        );
    }

    #[test]
    fn a_group_readable_directory_gets_the_exact_chmod() {
        assert_eq!(
            explain(
                &DataRootError::WrongMode {
                    item: Item::Root,
                    mode: 0o750,
                    required: 0o700,
                },
                &dir()
            ),
            r"/srv/gun metal's has mode 750, which lets other users in. Run: chmod 700 '/srv/gun metal'\''s'"
        );
        assert_eq!(
            explain(
                &DataRootError::WrongMode {
                    item: Item::Path(KEY),
                    mode: 0o640,
                    required: 0o600,
                },
                &PathBuf::from("/var/lib/gunmetal")
            ),
            "/var/lib/gunmetal/secrets/root.key has mode 640, which lets other users in. Run: chmod 600 '/var/lib/gunmetal/secrets/root.key'"
        );
    }

    #[test]
    fn explains_every_other_refusal_with_its_fix() {
        let dir = PathBuf::from("/data");
        let cases = [
            (
                DataRootError::NetworkFilesystem(NetworkFs::Nfs),
                "The data directory /data is on a network filesystem (NFS). SQLite cannot lock its databases safely there. Move the data directory to a disk on this machine, or set GUNMETAL_ALLOW_NETWORK_FILESYSTEM=true to accept the risk.",
            ),
            (
                DataRootError::Io {
                    item: Item::Root,
                    op: Op::Probe,
                    kind: io::ErrorKind::NotFound,
                },
                "The data directory /data does not exist. Create it as the user Gunmetal runs as: mkdir -m 700 '/data'",
            ),
            (
                DataRootError::Io {
                    item: Item::Dir(DataDir::Durable),
                    op: Op::Inspect,
                    kind: io::ErrorKind::NotFound,
                },
                "Could not inspect /data/durable: entity not found.",
            ),
            (
                DataRootError::Io {
                    item: Item::Root,
                    op: Op::OpenRoot,
                    kind: io::ErrorKind::PermissionDenied,
                },
                "Could not open /data: permission denied.",
            ),
            (
                DataRootError::NotOwned {
                    item: Item::Dir(DataDir::Secrets),
                    owner: 0,
                    uid: 1000,
                },
                "/data/secrets belongs to user 0, but Gunmetal runs as user 1000. As root, run: chown 1000 '/data/secrets'",
            ),
            (
                DataRootError::WrongKind {
                    item: Item::Path(KEY),
                    found: Kind::Symlink,
                },
                "/data/secrets/root.key is a symbolic link, which does not belong there. Move it out of the data directory.",
            ),
            (
                DataRootError::CannotRepair {
                    item: Item::Root,
                    mode: 0o777,
                    required: 0o700,
                },
                "/data still has mode 777 after Gunmetal set 700: its filesystem does not keep permissions. Move the data directory to a disk on this machine.",
            ),
            (
                DataRootError::Leftover {
                    item: Item::Replacement(KEY),
                },
                "/data/secrets/.root.key.tmp was left by an interrupted write. Remove it, then start Gunmetal again.",
            ),
            (
                DataRootError::ForeignEntry {
                    parent: Item::Dir(DataDir::Secrets),
                    name: OsString::from("Notes.txt"),
                },
                "/data/secrets/Notes.txt was not created by Gunmetal. Move it out of the secrets directory.",
            ),
        ];
        let explained: Vec<(DataRootError, String)> = cases
            .iter()
            .map(|(error, _)| (error.clone(), explain(error, &dir)))
            .collect();
        let expected: Vec<(DataRootError, String)> = cases
            .iter()
            .map(|(error, text)| (error.clone(), (*text).to_owned()))
            .collect();
        assert_eq!(explained, expected);
    }

    #[test]
    fn names_every_operation_kind_and_filesystem() {
        let ops = [
            Op::Probe,
            Op::Resolve,
            Op::OpenRoot,
            Op::Inspect,
            Op::Repair,
            Op::List,
            Op::CreateDir,
            Op::Create,
            Op::Open,
            Op::Write,
            Op::Rename,
            Op::Sync,
            Op::Remove,
        ];
        assert_eq!(
            ops.map(verb),
            [
                "examine",
                "resolve",
                "open",
                "inspect",
                "change the mode of",
                "list",
                "create the directory",
                "create",
                "open",
                "write",
                "rename",
                "sync",
                "remove"
            ]
        );
        assert_eq!(
            [Kind::File, Kind::Dir, Kind::Symlink, Kind::Other].map(noun),
            [
                "regular file",
                "directory",
                "symbolic link",
                "socket, FIFO or device"
            ]
        );
        assert_eq!(
            [
                NetworkFs::Nfs,
                NetworkFs::Smb,
                NetworkFs::NineP,
                NetworkFs::Fuse
            ]
            .map(filesystem),
            ["NFS", "SMB", "9p", "FUSE"]
        );
    }

    #[test]
    fn chooses_the_command_line_then_the_environment_then_the_default() {
        let flag = OsString::from("/flag");
        let env = PathBuf::from("/env");
        assert_eq!(choose(Some(&flag), Some(&env)), PathBuf::from("/flag"));
        assert_eq!(choose(None, Some(&env)), PathBuf::from("/env"));
        assert_eq!(choose(None, None), PathBuf::from("/var/lib/gunmetal"));
    }

    fn data_root(scratch: &TempDir) -> DataRoot {
        let host = HostFacts {
            uid: rustix::process::geteuid().as_raw(),
            filesystem: Filesystem::Local,
        };
        DataRoot::open(scratch.path(), &host, Policy::DEFAULT)
            .expect("the data root opens")
            .root
    }

    #[test]
    fn a_missing_configuration_file_reads_as_empty() {
        let scratch = TempDir::new("server-config-missing").expect("scratch");
        assert_eq!(read_config(&data_root(&scratch)), Ok(String::new()));
    }

    #[test]
    fn reads_the_configuration_file_up_to_its_cap() {
        let scratch = TempDir::new("server-config-cap").expect("scratch");
        let root = data_root(&scratch);
        let full = "#".repeat(MAX_CONFIG_BYTES);
        root.replace(&CONFIG_FILE, full.as_bytes()).expect("write");
        assert_eq!(read_config(&root), Ok(full.clone()));
        root.replace(&CONFIG_FILE, format!("{full}#").as_bytes())
            .expect("write");
        assert_eq!(read_config(&root), Err(ConfigFileError::TooLarge));
        root.replace(&CONFIG_FILE, b"level = \"\xff\"")
            .expect("write");
        assert_eq!(read_config(&root), Err(ConfigFileError::NotUtf8));
    }

    #[test]
    fn reports_a_configuration_file_it_cannot_read() {
        let scratch = TempDir::new("server-config-dir").expect("scratch");
        let root = data_root(&scratch);
        root.create_dir(&CONFIG_FILE)
            .expect("a directory where the file belongs");
        assert_eq!(
            read_config(&root),
            Err(ConfigFileError::Read(io::ErrorKind::IsADirectory))
        );
    }

    #[test]
    fn passes_on_every_open_failure_but_a_missing_file() {
        let denied = DataRootError::Io {
            item: Item::Path(CONFIG_FILE),
            op: Op::Open,
            kind: io::ErrorKind::PermissionDenied,
        };
        assert_eq!(
            absent_is_empty(Err(denied.clone())).err(),
            Some(ConfigFileError::Open(denied.clone()))
        );
        assert_eq!(
            read_file(Err(denied.clone())).err(),
            Some(ConfigFileError::Open(denied))
        );
    }

    #[test]
    fn explains_why_the_configuration_file_could_not_be_read() {
        let dir = PathBuf::from("/data");
        let denied = DataRootError::Io {
            item: Item::Path(CONFIG_FILE),
            op: Op::Open,
            kind: io::ErrorKind::PermissionDenied,
        };
        let messages: Vec<String> = [
            ConfigFileError::Open(denied),
            ConfigFileError::Read(io::ErrorKind::IsADirectory),
            ConfigFileError::TooLarge,
            ConfigFileError::NotUtf8,
        ]
        .iter()
        .map(|error| error.message(&dir))
        .collect();
        assert_eq!(
            messages,
            [
                "Could not open /data/durable/config.toml: permission denied.",
                "Could not read /data/durable/config.toml: is a directory.",
                "/data/durable/config.toml is longer than 65536 bytes, which no Gunmetal configuration needs.",
                "/data/durable/config.toml is not UTF-8 text.",
            ]
        );
    }
}
