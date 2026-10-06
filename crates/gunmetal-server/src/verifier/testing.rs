//! What the verifier's tests share: a real identity store in a temporary
//! data directory, and the table a stand-in pathway keeps its stored
//! credentials' names in.

use std::sync::Arc;

use gunmetal_core::schema::{Column, DataClass, SchemaPart};
use gunmetal_durable::identity::store::{IdentityStore, Spec};
use gunmetal_fs::dataroot::{DataRoot, Policy};
use gunmetal_fs::host::HostFacts;
use gunmetal_fs::sqlite::{Query, Value};
use gunmetal_testkit::tempdir::TempDir;

/// A temporary data directory and its open root.
pub struct Data {
    /// Kept so that the directory lives as long as the root.
    _dir: TempDir,
    /// The data root.
    pub root: DataRoot,
}

/// A new, empty data directory.
pub fn data(label: &str) -> Data {
    let dir = TempDir::new(label).expect("a temporary directory");
    let host = HostFacts::probe(dir.path()).expect("the host is probed");
    let root = DataRoot::open(dir.path(), &host, Policy::DEFAULT)
        .expect("the data root opens")
        .root;
    Data { _dir: dir, root }
}

/// The stand-in pathway's stored credentials: only their names, since a
/// name being there or not is all its lookup needs to learn.
pub const STAND_IN: SchemaPart = SchemaPart {
    name: "test.stand_in",
    sql: "CREATE TABLE stand_in (name TEXT PRIMARY KEY) WITHOUT ROWID;",
    columns: &[Column {
        table: "stand_in",
        name: "name",
        class: DataClass::Identity,
    }],
};

const ADD_NAME: Query = Query::new("INSERT OR IGNORE INTO stand_in (name) VALUES (?1)");

/// Stores a credential for the stand-in pathway under `name`.
pub fn add_name(name: &str) -> Query {
    ADD_NAME.bind(text(name))
}

/// `value` as SQLite text.
pub fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}

/// Opens the identity store beneath `root` with `parts`, creating it when
/// the directory is new.
pub fn open(root: &DataRoot, parts: &[SchemaPart]) -> Arc<IdentityStore> {
    let spec = Spec {
        parts,
        migrations: &[],
        invariants: &[],
        settings: &[],
    };
    let opened = IdentityStore::open(root, &spec).expect("the identity store opens");
    Arc::new(opened.store)
}
