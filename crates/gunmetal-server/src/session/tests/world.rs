//! The world the session tests run in: a real identity store in a temporary
//! data directory, a clock the test moves, a randomness source that writes
//! a known sequence, and a roster of accounts the test edits.

use core::net::{IpAddr, Ipv4Addr};
use std::collections::HashMap;
use std::io::Read;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

use gunmetal_core::authz::{
    DeviceClass, Elevation, Epoch, Principal, PrincipalFacts, PrincipalKind, Reach, Role,
    SessionHandle, UserVerification,
};
use gunmetal_core::client_context::ClientContext;
use gunmetal_core::http::forwarded::{ForwardingHeaders, HostNetwork, path_class};
use gunmetal_core::id::{IdKind, PublicId};
use gunmetal_core::schema::SchemaPart;
use gunmetal_core::time::Clock;
use gunmetal_durable::identity::pre_principal::PrePrincipal;
use gunmetal_durable::identity::store::{IDENTITY, IdentityStore, Spec};
use gunmetal_fs::dataroot::{DataRoot, Policy};
use gunmetal_fs::host::HostFacts;
use gunmetal_fs::path::{DataDir, DataPath};
use gunmetal_fs::sqlite::{Query, Row, Value};
use gunmetal_secrets::random::{Random, RandomnessUnavailable};
use gunmetal_secrets::root::Root;
use gunmetal_testkit::clock::ManualClock;
use gunmetal_testkit::tempdir::TempDir;

use crate::session::cookie::Cookie;
use crate::session::directory::{Directory, Standing};
use crate::session::error::SessionError;
use crate::session::lifetime::Lifetime;
use crate::session::schema::PARTS;
use crate::session::sessions::Sessions;
use crate::testing;

/// Two accounts the roster knows from the start, both members.
pub const SAM: &str = "usr_00000000000000000000000001";
pub const KIM: &str = "usr_00000000000000000000000002";

/// The first device a world enrols. The world's randomness source gives
/// its first caller the bytes 37 to 52, and this is those sixteen bytes in
/// Crockford base32, worked out with Python.
pub const DEVICE: &str = "dev_154rkjga9a5cp2tbhf60rk4csm";

/// The token of the first session issued after that device was enrolled:
/// the source's second answer, the bytes 74 to 105, in unpadded URL-safe
/// base64 as Python's `base64` writes them.
pub const TOKEN: &str = "SktMTU5PUFFSU1RVVldYWVpbXF1eX2BhYmNkZWZnaGk";

/// That session's handle: the source's third answer, the bytes 111 to 118,
/// read as one big-endian number.
pub const HANDLE: u64 = 8_030_042_871_978_816_886;

/// An address on the home network.
pub const HOME: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20));

/// The identity store's write-ahead log.
pub const WAL: DataPath = DataPath::constant(DataDir::Durable, "identity.db-wal");

/// A randomness source that writes a known sequence: its n-th answer,
/// counting from the number it was started at, is the bytes `37 n`,
/// `37 n + 1` and so on, wrapping at 256.
pub struct Sequence(AtomicU8);

impl Sequence {
    /// A source whose first answer is the one numbered `first`.
    pub fn starting(first: u8) -> Self {
        Self(AtomicU8::new(first))
    }
}

impl Random for Sequence {
    fn fill(&self, bytes: &mut [u8]) -> Result<(), RandomnessUnavailable> {
        let start = self.0.fetch_add(1, Ordering::SeqCst).wrapping_mul(37);
        for (offset, byte) in (0..=u8::MAX).cycle().zip(bytes.iter_mut()) {
            *byte = start.wrapping_add(offset);
        }
        Ok(())
    }
}

/// A randomness source that has none to give.
pub struct NoRandom;

impl Random for NoRandom {
    fn fill(&self, _: &mut [u8]) -> Result<(), RandomnessUnavailable> {
        Err(RandomnessUnavailable)
    }
}

/// The accounts the server has, as the session layer's directory.
#[derive(Default)]
pub struct Roster(Mutex<HashMap<PublicId, Standing>>);

impl Roster {
    /// Gives an account a standing, as creating it or changing its role
    /// would.
    pub fn set(&self, account: PublicId, standing: Standing) {
        self.0.lock().expect("roster").insert(account, standing);
    }

    /// Takes an account's standing away, as disabling it would.
    pub fn remove(&self, account: PublicId) {
        self.0.lock().expect("roster").remove(&account);
    }
}

impl Directory for Roster {
    fn standing(&self, account: PublicId) -> Option<Standing> {
        self.0.lock().expect("roster").get(&account).cloned()
    }
}

/// A member's standing: the member preset, no profile, no library yet.
pub fn member() -> Standing {
    Standing {
        kind: PrincipalKind::Member,
        profile: None,
        capabilities: Role::Member.preset(),
        libraries: Vec::new(),
        reach: Reach::Anywhere,
    }
}

/// The account with this identifier.
pub fn account(text: &str) -> PublicId {
    PublicId::parse(text, IdKind::User).expect("an account identifier")
}

/// The device with this identifier.
pub fn device(text: &str) -> PublicId {
    PublicId::parse(text, IdKind::Device).expect("a device identifier")
}

/// The client context of a peer that connected directly from `peer`.
pub fn from(peer: IpAddr) -> ClientContext {
    path_class(
        peer,
        &ForwardingHeaders::default(),
        &[],
        &HostNetwork::default(),
    )
    .expect("a direct peer resolves")
}

/// The principal a member's session resolves to: an ordinary session with
/// no user verification, on a device of `class`.
pub fn principal(who: &str, on: &str, class: DeviceClass, handle: u64, epoch: u64) -> Principal {
    Principal {
        facts: PrincipalFacts {
            kind: PrincipalKind::Member,
            account: Some(account(who)),
            profile: None,
            capabilities: Role::Member.preset(),
            libraries: Vec::new(),
            device: class,
            elevation: Elevation::Ordinary,
            verification: UserVerification::Stale,
            reach: Reach::Anywhere,
            scope: None,
        },
        device: Some(device(on)),
        session: SessionHandle(handle),
        epoch: Epoch(epoch),
    }
}

/// A temporary data directory and its open root.
pub fn data() -> (TempDir, DataRoot) {
    let dir = TempDir::new("session").expect("a temporary directory");
    let host = HostFacts::probe(dir.path()).expect("the host is probed");
    let root = DataRoot::open(dir.path(), &host, Policy::DEFAULT)
        .expect("the data root opens")
        .root;
    (dir, root)
}

/// An identity store beneath `root` that holds `parts`.
pub fn store(root: &DataRoot, parts: &[SchemaPart]) -> Arc<IdentityStore> {
    let spec = Spec {
        parts,
        migrations: &[],
        invariants: &[],
        settings: &[],
    };
    Arc::new(
        IdentityStore::open(root, &spec)
            .expect("the store opens")
            .store,
    )
}

/// The root secret 0 to 31, which the expected hashes in these tests were
/// worked out from.
pub fn keys(root: &DataRoot) -> Arc<Root> {
    Arc::new(Root::load_or_create(root, &Sequence::starting(0)).expect("the root secret"))
}

/// Everything a session test needs.
pub struct World {
    /// Kept so the directory lives as long as the store.
    _dir: TempDir,
    /// The data directory.
    pub root: DataRoot,
    /// The identity store, for a test to read and damage directly.
    pub store: Arc<IdentityStore>,
    /// The root secret.
    pub keys: Arc<Root>,
    /// The clock the session layer reads.
    pub clock: Arc<dyn Clock + Send + Sync>,
    /// The handle that moves the clock.
    pub manual: Arc<ManualClock>,
    /// The accounts.
    pub roster: Arc<Roster>,
    /// The session layer under test.
    pub sessions: Arc<Sessions>,
}

impl World {
    /// A world whose randomness source answers the sequence from 1.
    pub fn fresh() -> Self {
        Self::with(Arc::new(Sequence::starting(1)))
    }

    /// A world with `random` as its randomness source.
    pub fn with(random: Arc<dyn Random + Send + Sync>) -> Self {
        let (dir, root) = data();
        let store = store(&root, &PARTS);
        let keys = keys(&root);
        let (clock, manual) = testing::clock();
        let clock: Arc<dyn Clock + Send + Sync> = clock;
        let roster = Arc::new(Roster::default());
        roster.set(account(SAM), member());
        roster.set(account(KIM), member());
        let sessions = open(&store, &keys, random, &clock, &roster).expect("the session layer");
        Self {
            _dir: dir,
            root,
            store,
            keys,
            clock,
            manual,
            roster,
            sessions: Arc::new(sessions),
        }
    }

    /// Opens a second session layer over this world's store, as a restart
    /// would.
    pub fn reopen(&self) -> Result<Sessions, SessionError> {
        open(
            &self.store,
            &self.keys,
            Arc::new(Sequence::starting(100)),
            &self.clock,
            &self.roster,
        )
    }

    /// Enrols a device of `class` for the account `who` and signs it in
    /// from `peer`.
    pub fn sign_in_from(
        &self,
        who: &str,
        class: DeviceClass,
        mode: Lifetime,
        peer: IpAddr,
    ) -> (Cookie, Principal) {
        let device = self
            .sessions
            .enrol(account(who), class)
            .expect("the device enrols");
        self.sessions
            .issue(account(who), device, mode, &from(peer))
            .expect("the session is issued")
    }

    /// Enrols a device of `class` for the account `who` and signs it in
    /// from the home network.
    pub fn sign_in(&self, who: &str, class: DeviceClass, mode: Lifetime) -> (Cookie, Principal) {
        self.sign_in_from(who, class, mode, HOME)
    }

    /// Reads rows straight from the store.
    pub fn rows(&self, query: &'static str) -> Vec<Row> {
        self.store
            .read_pre_principal(PrePrincipal::SessionToken, &Query::new(query))
            .expect("the rows read")
    }

    /// Writes straight to the store, as nothing but a test may.
    pub fn write(&self, statements: &[Query]) {
        self.store.write(statements).expect("the statements run");
    }

    /// Every byte the identity store has on disk: its file, then its log.
    pub fn on_disk(&self) -> Vec<u8> {
        let mut bytes = file(&self.root, IDENTITY.path());
        bytes.extend(file(&self.root, &WAL));
        bytes
    }
}

/// Opens a session layer over the given store, keys, clock and roster.
fn open(
    store: &Arc<IdentityStore>,
    keys: &Arc<Root>,
    random: Arc<dyn Random + Send + Sync>,
    clock: &Arc<dyn Clock + Send + Sync>,
    roster: &Arc<Roster>,
) -> Result<Sessions, SessionError> {
    let directory = Arc::clone(roster);
    Sessions::open(
        Arc::clone(store),
        Arc::clone(keys),
        random,
        Arc::clone(clock),
        directory,
    )
}

/// The bytes of a file in the data directory.
pub fn file(root: &DataRoot, path: &DataPath) -> Vec<u8> {
    let mut bytes = Vec::new();
    root.open_read(path)
        .expect("the file opens")
        .read_to_end(&mut bytes)
        .expect("the file reads");
    bytes
}

/// Whether `haystack` holds `needle` anywhere.
pub fn holds(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// The bytes a hexadecimal text spells.
pub fn unhex(text: &str) -> Vec<u8> {
    text.as_bytes()
        .chunks(2)
        .map(|pair| {
            let digits = core::str::from_utf8(pair).expect("ASCII");
            u8::from_str_radix(digits, 16).expect("hexadecimal")
        })
        .collect()
}

/// A text value, as a row holds it.
pub fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}

/// The `name=value` pair a browser sends back for a cookie it was set.
pub fn pair(cookie: &Cookie) -> String {
    cookie
        .header()
        .split(';')
        .next()
        .expect("a pair")
        .to_owned()
}
