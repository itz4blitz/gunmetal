//! The root secret: 256 bits from the operating system's CSPRNG, made at
//! first start and kept in `secrets/root.key` (SEC-OPS-011, SEC-TM-012,
//! SEC-HIS-043).
//!
//! The file is reached only through the data-root handle (WP-126), which
//! creates it with mode 0600 inside the 0700 `secrets/` directory and,
//! when it opens the data directory, tightens or refuses a looser mode as
//! its policy says (SEC-OPS-012). Creating the file replaces it atomically,
//! so a crash leaves either no root secret or the whole of one.
//!
//! When the operating system cannot supply randomness at first start,
//! loading fails and writes nothing: there is no built-in or fallback key
//! for the server to start with (SEC-STD-022, SEC-HIS-043).

use std::io::{self, Read};

use gunmetal_fs::dataroot::{DataRoot, DataRootError};
use gunmetal_fs::path::{DataDir, DataPath};

use crate::crypto::kdf::hkdf_sha256;
use crate::random::Random;
use crate::secret::Secret;

/// Where the root secret lives.
pub const ROOT_KEY: DataPath = DataPath::constant(DataDir::Secrets, "root.key");

/// The root secret's length in bytes: 256 bits.
pub const ROOT_LEN: usize = 32;

/// A derived key's length in bytes: 256 bits.
pub const KEY_LEN: usize = 32;

/// Why the root secret could not be loaded or made. No variant carries any
/// of the secret's bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretsError {
    /// There is no root secret yet and the operating system could not
    /// supply the randomness to make one.
    RandomnessUnavailable,
    /// The data root refused to open or write the file.
    Storage(DataRootError),
    /// The file opened but could not be read.
    Read(io::ErrorKind),
    /// The file does not hold exactly [`ROOT_LEN`] bytes.
    Malformed,
    /// A cryptographic primitive refused its input: a derived key or a
    /// message longer than the algorithm allows. No call of this crate
    /// asks for either.
    Primitive,
}

/// The server's root secret, from which every other key is derived.
///
/// Inventory: root
#[derive(Debug)]
pub struct Root {
    secret: Secret<[u8; ROOT_LEN]>,
}

impl Root {
    /// Loads the root secret from the data directory, or makes one from
    /// `random` and stores it there when there is none yet.
    ///
    /// # Errors
    ///
    /// Returns [`SecretsError::RandomnessUnavailable`] when a root secret is
    /// needed and `random` cannot supply it, [`SecretsError::Storage`] when
    /// the data root refuses to open or write the file,
    /// [`SecretsError::Read`] when it cannot be read and
    /// [`SecretsError::Malformed`] when it holds anything but 32 bytes. The
    /// server must not start without its root secret.
    pub fn load_or_create(data: &DataRoot, random: &dyn Random) -> Result<Self, SecretsError> {
        match data.open_read(&ROOT_KEY) {
            Ok(mut file) => Self::read(&mut file),
            Err(DataRootError::Io {
                kind: io::ErrorKind::NotFound,
                ..
            }) => Self::create(data, random),
            Err(error) => Err(SecretsError::Storage(error)),
        }
    }

    /// Reads a root secret of exactly [`ROOT_LEN`] bytes from `file`.
    pub(crate) fn read(file: &mut dyn Read) -> Result<Self, SecretsError> {
        let mut secret = Secret::new([0; ROOT_LEN]);
        let mut beyond = Secret::new([0; 1]);
        file.read_exact(secret.value_mut())
            .map_err(|error| malformed_if_short(error.kind()))
            .and_then(|()| match file.read_exact(beyond.value_mut()) {
                Ok(()) => Err(SecretsError::Malformed),
                Err(error) => match malformed_if_short(error.kind()) {
                    SecretsError::Malformed => Ok(Self { secret }),
                    other => Err(other),
                },
            })
    }

    /// Makes a root secret from `random` and stores it.
    fn create(data: &DataRoot, random: &dyn Random) -> Result<Self, SecretsError> {
        let mut secret = Secret::new([0; ROOT_LEN]);
        random
            .fill(secret.value_mut())
            .map_err(|_| SecretsError::RandomnessUnavailable)
            .and_then(|()| {
                data.replace(&ROOT_KEY, secret.value())
                    .map_err(SecretsError::Storage)
            })
            .map(|()| Self { secret })
    }
}

impl Root {
    /// The key of generation `generation` for the purpose labelled
    /// `label`: the HKDF-SHA-256 output for the root secret, no salt and
    /// the context `gunmetal/v1/<label>/<generation>` (SEC-OPS-015).
    /// Record 9 decision 4 wrote the 8-bit key ID in that context; this
    /// uses the 64-bit generation instead, because a key ID wraps after
    /// 256 generations and must then select another key.
    pub(crate) fn derive(
        &self,
        label: &str,
        generation: u64,
    ) -> Result<Secret<[u8; KEY_LEN]>, SecretsError> {
        let mut key = Secret::new([0; KEY_LEN]);
        let info = format!("gunmetal/v1/{label}/{generation}");
        hkdf_sha256(&[], self.secret.value(), info.as_bytes(), key.value_mut())
            .or(Err(SecretsError::Primitive))
            .map(|()| key)
    }
}

/// A read that ran out of bytes means the file is too short; any other
/// failure is reported as it is.
const fn malformed_if_short(kind: io::ErrorKind) -> SecretsError {
    match kind {
        io::ErrorKind::UnexpectedEof => SecretsError::Malformed,
        kind => SecretsError::Read(kind),
    }
}

#[cfg(test)]
mod tests {
    use std::io::{self, Read, Write};
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    use gunmetal_fs::dataroot::{
        DataRoot, DataRootError, Item, Modes, NetworkFilesystems, Op, Policy, Refused, Repair,
    };
    use gunmetal_fs::host::HostFacts;
    use gunmetal_testkit::tempdir::TempDir;

    use super::{ROOT_KEY, ROOT_LEN, Root, SecretsError};
    use crate::random::OsRandom;
    use crate::random::fake::{Counting, Failing};
    use crate::testing::{COUNTED, counted_root, hex};

    /// Fails loudly when the tests run as root, which ignores file modes and
    /// would let a permission test pass for the wrong reason.
    fn assert_not_root(dir: &Path) {
        let host = HostFacts::probe(dir).unwrap();
        assert_ne!(host.uid, 0, "these tests check file modes");
    }

    /// Opens a data root in `dir` with the modes policy `modes`.
    fn open(dir: &Path, modes: Modes) -> Result<DataRoot, Refused> {
        let policy = Policy {
            modes,
            network: NetworkFilesystems::Refuse,
        };
        DataRoot::open(dir, &HostFacts::probe(dir).unwrap(), policy).map(|opened| opened.root)
    }

    /// The bytes of the root secret file.
    fn stored(data: &DataRoot) -> Vec<u8> {
        let mut bytes = Vec::new();
        data.open_read(&ROOT_KEY)
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        bytes
    }

    /// The permission bits of the root secret file.
    fn mode(data: &DataRoot) -> u32 {
        data.open_read(&ROOT_KEY)
            .unwrap()
            .metadata()
            .unwrap()
            .permissions()
            .mode()
            & 0o7777
    }

    /// Verifies: SEC-OPS-012
    ///
    /// The first start writes the secret it drew, whole, to a file with
    /// mode 0600, and every later start reads the same bytes back without
    /// drawing any randomness.
    #[test]
    fn the_first_start_stores_the_drawn_secret_and_later_starts_load_it() {
        let dir = TempDir::new("secrets-root").unwrap();
        assert_not_root(dir.path());
        let data = open(dir.path(), Modes::Refuse).unwrap();
        let made = Root::load_or_create(&data, &Counting).unwrap();
        assert_eq!(made.secret.value(), &COUNTED);
        assert_eq!(stored(&data), COUNTED);
        assert_eq!(mode(&data), 0o600);
        let loaded = Root::load_or_create(&data, &Failing).unwrap();
        assert_eq!(loaded.secret.value(), &COUNTED);
    }

    /// Verifies: SEC-OPS-011, SEC-TM-012
    ///
    /// Two fresh data directories get different root secrets of 256 bits
    /// each, from the operating system's CSPRNG.
    #[test]
    fn two_fresh_data_directories_share_no_root_secret() {
        let first_dir = TempDir::new("secrets-first").unwrap();
        let second_dir = TempDir::new("secrets-second").unwrap();
        let first_data = open(first_dir.path(), Modes::Refuse).unwrap();
        let second_data = open(second_dir.path(), Modes::Refuse).unwrap();
        let first = Root::load_or_create(&first_data, &OsRandom).unwrap();
        let second = Root::load_or_create(&second_data, &OsRandom).unwrap();
        assert_ne!(first.secret.value(), second.secret.value());
        assert_ne!(first.secret.value(), &[0; ROOT_LEN]);
        assert_eq!(stored(&first_data), first.secret.value());
        assert_eq!(stored(&second_data), second.secret.value());
    }

    /// Verifies: SEC-STD-022, SEC-HIS-043
    ///
    /// Without randomness the first start fails, writes nothing and leaves
    /// no key behind for a later start to pick up.
    #[test]
    fn a_randomness_failure_at_first_start_stops_it_and_writes_nothing() {
        let dir = TempDir::new("secrets-norandom").unwrap();
        let data = open(dir.path(), Modes::Refuse).unwrap();
        assert_eq!(
            Root::load_or_create(&data, &Failing).map(|_| ()),
            Err(SecretsError::RandomnessUnavailable)
        );
        assert_eq!(
            data.open_read(&ROOT_KEY).map(|_| ()).unwrap_err(),
            DataRootError::Io {
                item: Item::Path(ROOT_KEY),
                op: Op::Open,
                kind: io::ErrorKind::NotFound
            }
        );
    }

    /// Verifies: SEC-OPS-012
    ///
    /// A root secret file with mode 0644 is refused when the policy says
    /// refuse, and tightened to 0600 and loaded unchanged when it says
    /// repair.
    #[test]
    fn a_loose_root_secret_file_is_refused_or_repaired_as_configured() {
        let dir = TempDir::new("secrets-loose").unwrap();
        assert_not_root(dir.path());
        let data = open(dir.path(), Modes::Refuse).unwrap();
        Root::load_or_create(&data, &Counting).unwrap();
        data.open_read(&ROOT_KEY)
            .unwrap()
            .set_permissions(PermissionsExt::from_mode(0o644))
            .unwrap();
        drop(data);
        assert_eq!(
            open(dir.path(), Modes::Refuse).map(|_| ()),
            Err(Refused {
                error: DataRootError::WrongMode {
                    item: Item::Path(ROOT_KEY),
                    mode: 0o644,
                    required: 0o600
                },
                repairs: vec![]
            })
        );
        let host = HostFacts::probe(dir.path()).unwrap();
        let opened = DataRoot::open(dir.path(), &host, Policy::DEFAULT).unwrap();
        assert_eq!(
            opened.repairs,
            [Repair::Mode {
                item: Item::Path(ROOT_KEY),
                from: 0o644,
                to: 0o600
            }]
        );
        let loaded = Root::load_or_create(&opened.root, &Failing).unwrap();
        assert_eq!(loaded.secret.value(), &COUNTED);
        assert_eq!(mode(&opened.root), 0o600);
    }

    #[test]
    fn a_root_secret_file_the_data_root_cannot_open_is_reported() {
        let dir = TempDir::new("secrets-unreadable").unwrap();
        assert_not_root(dir.path());
        let data = open(dir.path(), Modes::Refuse).unwrap();
        Root::load_or_create(&data, &Counting).unwrap();
        data.open_read(&ROOT_KEY)
            .unwrap()
            .set_permissions(PermissionsExt::from_mode(0o000))
            .unwrap();
        assert_eq!(
            Root::load_or_create(&data, &Counting).map(|_| ()),
            Err(SecretsError::Storage(DataRootError::Io {
                item: Item::Path(ROOT_KEY),
                op: Op::Open,
                kind: io::ErrorKind::PermissionDenied
            }))
        );
    }

    #[test]
    fn a_directory_in_place_of_the_root_secret_is_reported() {
        let dir = TempDir::new("secrets-directory").unwrap();
        let data = open(dir.path(), Modes::Refuse).unwrap();
        data.create_dir(&ROOT_KEY).unwrap();
        assert_eq!(
            Root::load_or_create(&data, &Counting).map(|_| ()),
            Err(SecretsError::Read(io::ErrorKind::IsADirectory))
        );
    }

    /// Verifies: SEC-HIS-043
    ///
    /// A root secret file that is too short or too long is refused, never
    /// padded, cut or replaced with a new key.
    #[test]
    fn a_root_secret_of_the_wrong_length_is_refused() {
        let lengths = [0, 1, ROOT_LEN - 1, ROOT_LEN + 1, 2 * ROOT_LEN];
        let outcomes: Vec<_> = lengths
            .iter()
            .map(|&length| {
                let dir = TempDir::new("secrets-length").unwrap();
                let data = open(dir.path(), Modes::Refuse).unwrap();
                data.replace(&ROOT_KEY, &vec![7; length]).unwrap();
                let loaded = Root::load_or_create(&data, &Counting).map(|_| ());
                (loaded, stored(&data) == vec![7; length])
            })
            .collect();
        assert_eq!(outcomes, vec![(Err(SecretsError::Malformed), true); 5]);
    }

    /// A reader that gives `good` bytes of zeros and then fails.
    struct Breaks {
        good: usize,
    }

    impl Read for Breaks {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.good == 0 {
                return Err(io::Error::from(io::ErrorKind::ConnectionReset));
            }
            let n = buf.len().min(self.good);
            buf[..n].fill(0);
            self.good -= n;
            Ok(n)
        }
    }

    #[test]
    fn a_read_that_fails_is_reported_whether_or_not_the_secret_was_whole() {
        let outcomes: Vec<_> = [0, 5, ROOT_LEN]
            .into_iter()
            .map(|good| Root::read(&mut Breaks { good }).map(|_| ()))
            .collect();
        assert_eq!(
            outcomes,
            vec![Err(SecretsError::Read(io::ErrorKind::ConnectionReset)); 3]
        );
    }

    /// Verifies: SEC-OPS-015
    ///
    /// A key is the HKDF-SHA-256 output for the root secret and a context
    /// that names the purpose and the generation, so another purpose or
    /// another generation is another key. The expected keys come from an
    /// HKDF written in Python over `hmac` and `hashlib`.
    #[test]
    fn a_derived_key_is_the_hkdf_output_for_its_purpose_and_generation() {
        let root = counted_root();
        let key = |label, generation| root.derive(label, generation).map(|key| hex(key.value()));
        assert_eq!(
            [
                key("url_signing", 0),
                key("url_signing", 1),
                key("url_signing", 256),
                key("vault", 0),
            ],
            [
                Ok("951eaa9d6a3c7a9f455da3ff79af2f324383970254da470d81520e926f9dcae4".to_owned()),
                Ok("0c1a2aedac0fe7b531244df566ed38ccc3f1efd5a764ff3f1745b70c8701d216".to_owned()),
                Ok("59e9e5288daf15d47003ea9e3bf403d560266133f0733ad7588224e592a38892".to_owned()),
                Ok("a4b26e1f369a124ae47b55eecaca7afc8e48d88530b7e1d8bad65d39d844ad05".to_owned()),
            ]
        );
    }

    #[test]
    fn a_reader_that_ends_after_exactly_the_secret_gives_it() {
        let root = Root::read(&mut &COUNTED[..]).unwrap();
        assert_eq!(root.secret.value(), &COUNTED);
        let mut debug = Vec::new();
        write!(debug, "{root:?}").unwrap();
        assert_eq!(debug, b"Root { secret: Secret([redacted]) }");
    }
}
