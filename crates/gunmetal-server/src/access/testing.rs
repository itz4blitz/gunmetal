//! What the authorisation layer's unit tests share: a data directory whose
//! identity store holds the grants table, a few people, the requests they
//! make, and grant rows written and read without the layer.

use std::net::{IpAddr, Ipv4Addr};

use gunmetal_core::authz::{
    Action, CapabilitySet, Context, DeviceClass, Elevation, Epoch, Network, Permit, Principal,
    PrincipalFacts, PrincipalKind, Reach, RemoteAdmin, ResourceFacts, Role, SessionHandle,
    UserVerification, decide,
};
use gunmetal_core::client_context::{ClientContext, PathClass};
use gunmetal_core::http::forwarded::{ForwardingHeaders, HostNetwork, path_class};
use gunmetal_core::id::{IdKind, PublicId};
use gunmetal_durable::identity::pre_principal::PrePrincipal;
use gunmetal_durable::identity::store::{IdentityStore, Spec};
use gunmetal_fs::dataroot::{DataRoot, Policy};
use gunmetal_fs::host::HostFacts;
use gunmetal_fs::sqlite::{Query, Row, Value};
use gunmetal_testkit::tempdir::TempDir;

use super::grants::LIBRARY_GRANTS;
use super::policy::Ask;

/// A temporary data directory and the identity store in it.
pub struct Data {
    /// The identity store, with the grants table and nothing in it.
    pub identity: IdentityStore,
    /// The data directory, for a test that keeps another database in it.
    pub root: DataRoot,
    // Dropped last, after the store and the root.
    _dir: TempDir,
}

/// A fresh data directory with an identity store that holds the grants
/// table.
pub fn data() -> Data {
    let dir = TempDir::new("server-access").expect("a temporary directory");
    let host = HostFacts::probe(dir.path()).expect("the host is probed");
    let root = DataRoot::open(dir.path(), &host, Policy::DEFAULT)
        .expect("the data root opens")
        .root;
    let spec = Spec {
        parts: &[LIBRARY_GRANTS],
        migrations: &[],
        invariants: &[],
        settings: &[],
    };
    let identity = IdentityStore::open(&root, &spec)
        .expect("the identity store opens")
        .store;
    Data {
        identity,
        root,
        _dir: dir,
    }
}

/// The identifier of kind `kind` whose symbols are `n` in decimal, padded
/// with zeros.
fn id(prefix: &str, kind: IdKind, n: u8) -> PublicId {
    PublicId::parse(&format!("{prefix}_{n:026}"), kind).expect("a canonical identifier")
}

/// Library number `n`.
pub fn library(n: u8) -> PublicId {
    id("lib", IdKind::Library, n)
}

/// Account number `n`.
pub fn account(n: u8) -> PublicId {
    id("usr", IdKind::User, n)
}

/// Profile number `n`.
pub fn profile(n: u8) -> PublicId {
    id("prf", IdKind::Profile, n)
}

/// The principal of kind `kind` for account and profile `n`, holding
/// `capabilities`: on a personal device, in an elevated session with a
/// fresh user verification, with no location restriction and no credential
/// scope. Its facts claim no library.
pub fn principal(kind: PrincipalKind, capabilities: CapabilitySet, n: u8) -> Principal {
    Principal {
        facts: PrincipalFacts {
            kind,
            account: Some(account(n)),
            profile: Some(profile(n)),
            capabilities,
            libraries: Vec::new(),
            device: DeviceClass::Personal,
            elevation: Elevation::Elevated,
            verification: UserVerification::Fresh,
            reach: Reach::Anywhere,
            scope: None,
        },
        device: None,
        session: SessionHandle(u64::from(n)),
        epoch: Epoch(0),
    }
}

/// Member number `n`, with the member preset.
pub fn member(n: u8) -> Principal {
    principal(PrincipalKind::Member, Role::Member.preset(), n)
}

/// The owner, account 9.
pub fn owner() -> Principal {
    principal(PrincipalKind::Owner, CapabilitySet::EVERY, 9)
}

/// The person with account `n` and kind `kind`, as an action on a person
/// names them.
pub fn person(n: u8, kind: PrincipalKind) -> ResourceFacts {
    ResourceFacts::Person {
        account: account(n),
        kind,
    }
}

/// Where a request straight from `peer` came from, as the listener would
/// resolve it with no trusted proxy.
fn arriving_from(peer: Ipv4Addr) -> ClientContext {
    path_class(
        IpAddr::V4(peer),
        &ForwardingHeaders::default(),
        &[],
        &HostNetwork::default(),
    )
    .expect("a direct peer has a path class")
}

/// A request from the internet address 203.0.113.7.
pub fn internet() -> ClientContext {
    arriving_from(Ipv4Addr::new(203, 0, 113, 7))
}

/// A request from the server itself.
pub fn loopback() -> ClientContext {
    arriving_from(Ipv4Addr::LOCALHOST)
}

/// The question `principal` asks about `action` on `resource` from
/// `source`, on the network it last verified on, with remote administration
/// allowed.
pub fn ask<'a>(
    principal: &'a Principal,
    action: Action,
    resource: &'a ResourceFacts,
    source: &'a ClientContext,
) -> Ask<'a> {
    Ask {
        principal,
        action,
        resource,
        source,
        network: Network::Same,
        remote_admin: RemoteAdmin::Allowed,
    }
}

/// The permit the policy itself gives `principal` for `action` on
/// `resource` when it holds grants for `libraries`, asked from the server
/// itself. For a test that needs a permit and not the layer that asks for
/// one.
pub fn decided(
    principal: &Principal,
    libraries: &[u8],
    action: Action,
    resource: &ResourceFacts,
) -> Permit {
    let facts = PrincipalFacts {
        libraries: libraries.iter().map(|n| library(*n)).collect(),
        ..principal.facts.clone()
    };
    let at_home = Context {
        path: PathClass::Loopback,
        network: Network::Same,
        remote_admin: RemoteAdmin::Allowed,
    };
    decide(&facts, action, resource, &at_home).expect("the policy allows it")
}

const GIVE: Query = Query::new("INSERT INTO library_grants (account, library) VALUES (?1, ?2)");
const STORED: Query =
    Query::new("SELECT account, library FROM library_grants ORDER BY account, library");
const LOSE: Query = Query::new("DROP TABLE library_grants");

/// Writes a grant row with `library` as its library, straight into the
/// store.
pub fn give_raw(data: &Data, account: PublicId, library: Value) {
    data.identity
        .write(&[GIVE.bind(Value::Text(account.to_string())).bind(library)])
        .expect("the grant is written");
}

/// Writes a grant of `library` to `account` straight into the store.
pub fn give(data: &Data, account: PublicId, library: PublicId) {
    give_raw(data, account, Value::Text(library.to_string()));
}

/// The row a grant of `library` to `account` is stored as.
pub fn pair(account: PublicId, library: PublicId) -> Row {
    Row(vec![
        Value::Text(account.to_string()),
        Value::Text(library.to_string()),
    ])
}

/// Every stored grant, by account and then library.
pub fn stored(data: &Data) -> Vec<Row> {
    data.identity
        .read_pre_principal(PrePrincipal::Grant, &STORED)
        .expect("the grants are read")
}

/// Drops the grants table, so that every read and write of a grant fails
/// in the database.
pub fn lose_grants(data: &Data) {
    data.identity
        .write(&[LOSE])
        .expect("the grants table is dropped");
}
