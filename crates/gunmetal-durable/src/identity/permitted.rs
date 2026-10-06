//! The identity store's reader for one person's rows: it takes a `Permit`
//! (SEC-TM-024, SEC-API-010).
//!
//! Every read of the identity store that returns a user-visible object (an
//! account, device, grant, invitation or share) goes through
//! [`IdentityStore::read_permitted`]. Only the policy function in the core
//! mints a [`Permit`], so a handler that has not asked the policy has
//! nothing to hand this reader, and the store's general reader stays
//! private to this crate. The only public reads without a `Permit` are the
//! pre-principal lookups
//! ([`PrePrincipal`](crate::identity::pre_principal::PrePrincipal)) and the
//! public-ID mapping, which returns no user-visible object. That holds for
//! the store's own methods: a module that opens a connection of its own to
//! the database file, with the SQLite door's `open_db`, goes round all of
//! them. In the server crate, the architecture test
//! (`crates/gunmetal-server/tests/storage_access.rs`) holds that opener to
//! three directories by reading the server's sources as text.
//!
//! A permit says what the policy allowed, for whom and about whom. The
//! reader decides nothing itself. It holds a read to that answer in three
//! ways:
//!
//! - **The action.** A [`PersonalRead`] names the action it belongs to, and
//!   a permit decided for any other action is refused. A credential scoped
//!   to changing its holder's data holds a permit to write and none to
//!   read, and cannot spend the first as the second.
//! - **The person.** The rows are those of the person the permit was decided
//!   about: the account or profile that owns the object, for an action on
//!   one's own data, or the account that an action on a person targets. A
//!   permit decided about nobody, such as one to browse the library, opens
//!   no personal read.
//! - **The rows.** The reader binds that person as the statement's next
//!   parameter and then checks what came back: the first column of every
//!   row must be the same person. One row of anyone else and the whole read
//!   is refused, so a statement that forgets its filter returns nothing
//!   rather than another person's devices or invitations.

use gunmetal_core::authz::{Action, Owner, Permit};
use gunmetal_core::id::PublicId;
use gunmetal_fs::sqlite::{Query, Row, Value};

use crate::identity::error::IdentityError;
use crate::identity::store::IdentityStore;

/// A statement that reads one person's rows, with the decision it needs.
#[derive(Debug, Clone, PartialEq)]
pub struct PersonalRead {
    /// The action a permit must have been decided for to open this read.
    pub action: Action,
    /// The statement. The reader binds the person as its next parameter,
    /// after any the caller bound, and the first column of every row it
    /// returns must be that person.
    pub query: Query,
}

/// Why a permitted read returned nothing.
#[derive(Debug, Clone, PartialEq)]
pub enum PermittedError {
    /// The permit was decided for another action.
    WrongAction {
        /// The action the read belongs to.
        needed: Action,
        /// The action the permit was decided for.
        decided: Action,
    },
    /// The permit was decided about nobody, so it names no person whose
    /// rows it could open.
    NoPerson,
    /// The statement returned a row that is not the person's, so none of
    /// its rows were returned.
    ForeignRow {
        /// The position of the first such row, counted from 0.
        row: usize,
    },
    /// The store failed or refused the statement.
    Store(IdentityError),
}

impl IdentityStore {
    /// Runs `read` for the person `permit` was decided about and returns
    /// that person's rows.
    ///
    /// # Errors
    ///
    /// Returns [`PermittedError::WrongAction`] when `permit` was decided for
    /// an action other than `read`'s, [`PermittedError::NoPerson`] when it
    /// was decided about nobody, [`PermittedError::Store`] when SQLite fails
    /// or refuses the statement, which includes any write, and
    /// [`PermittedError::ForeignRow`] when a row's first column is not the
    /// person.
    pub fn read_permitted(
        &self,
        permit: &Permit,
        read: &PersonalRead,
    ) -> Result<Vec<Row>, PermittedError> {
        if permit.action() != read.action {
            return Err(PermittedError::WrongAction {
                needed: read.action,
                decided: permit.action(),
            });
        }
        let person = person_of(permit)
            .map(|person| Value::Text(person.to_string()))
            .ok_or(PermittedError::NoPerson)?;
        let rows = self
            .read(&read.query.clone().bind(person.clone()))
            .map_err(PermittedError::Store)?;
        let foreign = rows
            .iter()
            .position(|Row(values)| values.first() != Some(&person));
        match foreign {
            Some(row) => Err(PermittedError::ForeignRow { row }),
            None => Ok(rows),
        }
    }
}

/// The person `permit` was decided about: the account or profile that owns
/// the object, or the account that an action on a person targets.
fn person_of(permit: &Permit) -> Option<PublicId> {
    match (permit.owner(), permit.target()) {
        (Some(Owner::Account(id) | Owner::Profile(id)), _) => Some(id),
        (None, Some(target)) => Some(target.account),
        (None, None) => None,
    }
}

/// What code outside this crate must not be able to do with the reader.
/// Rustdoc on stable does not check which error a compile-fail test
/// produced, so the test shares its imports with the control, which
/// compiles.
#[cfg(doctest)]
mod compile_fail {
    /// Control: outside this crate, a read of one person's rows hands the
    /// reader a permit.
    ///
    /// ```
    /// use gunmetal_core::authz::Permit;
    /// use gunmetal_durable::identity::permitted::{PermittedError, PersonalRead};
    /// use gunmetal_durable::identity::store::IdentityStore;
    /// use gunmetal_fs::sqlite::Row;
    ///
    /// fn devices(
    ///     store: &IdentityStore,
    ///     permit: &Permit,
    ///     read: &PersonalRead,
    /// ) -> Result<Vec<Row>, PermittedError> {
    ///     store.read_permitted(permit, read)
    /// }
    /// ```
    struct Control;

    /// Verifies: SEC-TM-024, SEC-API-010
    ///
    /// The reader cannot be called without a permit.
    ///
    /// ```compile_fail,E0061
    /// use gunmetal_core::authz::Permit;
    /// use gunmetal_durable::identity::permitted::{PermittedError, PersonalRead};
    /// use gunmetal_durable::identity::store::IdentityStore;
    /// use gunmetal_fs::sqlite::Row;
    ///
    /// fn devices(
    ///     store: &IdentityStore,
    ///     permit: &Permit,
    ///     read: &PersonalRead,
    /// ) -> Result<Vec<Row>, PermittedError> {
    ///     store.read_permitted(read)
    /// }
    /// ```
    struct NoPermit;
}

#[cfg(test)]
mod tests {
    use super::{PermittedError, PersonalRead};
    use crate::identity::error::{IdentityError, Step};
    use crate::identity::store::{IdentityStore, Spec};
    use gunmetal_core::authz::{
        Action, CapabilitySet, Context, DeviceClass, Elevation, Network, Owner, Permit,
        PrincipalFacts, PrincipalKind, Reach, RemoteAdmin, ResourceFacts, Role, UserVerification,
        decide,
    };
    use gunmetal_core::client_context::PathClass;
    use gunmetal_core::id::{IdKind, PublicId};
    use gunmetal_core::schema::{Column, DataClass, SchemaPart};
    use gunmetal_fs::dataroot::{DataRoot, Policy};
    use gunmetal_fs::host::HostFacts;
    use gunmetal_fs::sqlite::{DbError, Query, Row, Value};
    use gunmetal_testkit::tempdir::TempDir;

    /// Two members' accounts and profiles, and the owner's account, as the
    /// store writes them.
    const SAM: &str = "usr_00000000000000000000000001";
    const KIM: &str = "usr_00000000000000000000000002";
    const OWNER: &str = "usr_00000000000000000000000009";
    const SAM_PROFILE: &str = "prf_00000000000000000000000001";
    const KIM_PROFILE: &str = "prf_00000000000000000000000002";
    const OWNER_PROFILE: &str = "prf_00000000000000000000000009";

    /// Stand-ins for tables other packages register: devices and
    /// invitations belong to an account, bookmarks to a profile.
    const PERSONAL: SchemaPart = SchemaPart {
        name: "test.personal",
        sql: "CREATE TABLE devices (owner TEXT NOT NULL, name TEXT NOT NULL); \
              CREATE TABLE invitations (owner TEXT NOT NULL, guest TEXT NOT NULL); \
              CREATE TABLE bookmarks (owner TEXT NOT NULL, title TEXT NOT NULL);",
        columns: &[
            Column {
                table: "devices",
                name: "owner",
                class: DataClass::Identity,
            },
            Column {
                table: "devices",
                name: "name",
                class: DataClass::Identity,
            },
            Column {
                table: "invitations",
                name: "owner",
                class: DataClass::Identity,
            },
            Column {
                table: "invitations",
                name: "guest",
                class: DataClass::Identity,
            },
            Column {
                table: "bookmarks",
                name: "owner",
                class: DataClass::Activity,
            },
            Column {
                table: "bookmarks",
                name: "title",
                class: DataClass::Activity,
            },
        ],
    };

    const ADD_DEVICE: Query = Query::new("INSERT INTO devices (owner, name) VALUES (?1, ?2)");
    const ADD_INVITATION: Query =
        Query::new("INSERT INTO invitations (owner, guest) VALUES (?1, ?2)");
    const ADD_BOOKMARK: Query = Query::new("INSERT INTO bookmarks (owner, title) VALUES (?1, ?2)");

    /// One's own devices, by name.
    const OWN_DEVICES: PersonalRead = PersonalRead {
        action: Action::ReadOwnData,
        query: Query::new("SELECT owner, name FROM devices WHERE owner = ?1 ORDER BY name"),
    };

    /// One's own invitations, by guest.
    const OWN_INVITATIONS: PersonalRead = PersonalRead {
        action: Action::ReadOwnData,
        query: Query::new("SELECT owner, guest FROM invitations WHERE owner = ?1 ORDER BY guest"),
    };

    fn text(value: &str) -> Value {
        Value::Text(value.to_owned())
    }

    /// A row of an owner and one more column.
    fn pair(owner: &str, what: &str) -> Row {
        Row(vec![text(owner), text(what)])
    }

    fn add(statement: Query, owner: &str, what: &str) -> Query {
        statement.bind(text(owner)).bind(text(what))
    }

    /// A store holding both members' rows, in a temporary data directory.
    struct Data {
        store: IdentityStore,
        _root: DataRoot,
        // Dropped last, after the store and the root.
        _dir: TempDir,
    }

    fn data() -> Data {
        let dir = TempDir::new("durable-permitted").expect("a temporary directory");
        let host = HostFacts::probe(dir.path()).expect("the host is probed");
        let root = DataRoot::open(dir.path(), &host, Policy::DEFAULT)
            .expect("the data root opens")
            .root;
        let spec = Spec {
            parts: &[PERSONAL],
            migrations: &[],
            invariants: &[],
            settings: &[],
        };
        let store = IdentityStore::open(&root, &spec)
            .expect("the store opens")
            .store;
        store
            .write(&[
                add(ADD_DEVICE, SAM, "laptop"),
                add(ADD_DEVICE, SAM, "phone"),
                add(ADD_DEVICE, KIM, "tablet"),
                add(ADD_INVITATION, SAM, "a guest of sam"),
                add(ADD_INVITATION, KIM, "a guest of kim"),
                add(ADD_BOOKMARK, SAM_PROFILE, "where sam stopped"),
                add(ADD_BOOKMARK, KIM_PROFILE, "where kim stopped"),
            ])
            .expect("the rows are written");
        Data {
            store,
            _root: root,
            _dir: dir,
        }
    }

    fn id(text: &str, kind: IdKind) -> PublicId {
        PublicId::parse(text, kind).expect("a canonical identifier")
    }

    /// A principal on a personal device, in an elevated session with a
    /// fresh user verification, with no scope and no location restriction.
    fn principal(
        kind: PrincipalKind,
        capabilities: CapabilitySet,
        account: &str,
        profile: &str,
    ) -> PrincipalFacts {
        PrincipalFacts {
            kind,
            account: Some(id(account, IdKind::User)),
            profile: Some(id(profile, IdKind::Profile)),
            capabilities,
            libraries: Vec::new(),
            device: DeviceClass::Personal,
            elevation: Elevation::Elevated,
            verification: UserVerification::Fresh,
            reach: Reach::Anywhere,
            scope: None,
        }
    }

    fn sam() -> PrincipalFacts {
        principal(
            PrincipalKind::Member,
            Role::Member.preset(),
            SAM,
            SAM_PROFILE,
        )
    }

    fn kim() -> PrincipalFacts {
        principal(
            PrincipalKind::Member,
            Role::Member.preset(),
            KIM,
            KIM_PROFILE,
        )
    }

    fn owner() -> PrincipalFacts {
        principal(
            PrincipalKind::Owner,
            CapabilitySet::EVERY,
            OWNER,
            OWNER_PROFILE,
        )
    }

    /// The permit the policy gives `principal` for `action` on `resource`,
    /// asked from the server itself.
    fn allowed(principal: &PrincipalFacts, action: Action, resource: &ResourceFacts) -> Permit {
        let at_home = Context {
            path: PathClass::Loopback,
            network: Network::Same,
            remote_admin: RemoteAdmin::Allowed,
        };
        decide(principal, action, resource, &at_home).expect("the policy allows it")
    }

    /// What the account `account` owns.
    fn account_of(account: &str) -> ResourceFacts {
        ResourceFacts::Owned(Owner::Account(id(account, IdKind::User)))
    }

    /// Verifies: SEC-TM-024, SEC-API-010
    #[test]
    fn a_permit_for_one_s_own_data_reads_that_account_s_rows_and_nobody_else_s() {
        let data = data();
        let sam = allowed(&sam(), Action::ReadOwnData, &account_of(SAM));
        let kim = allowed(&kim(), Action::ReadOwnData, &account_of(KIM));
        assert_eq!(
            data.store.read_permitted(&sam, &OWN_DEVICES),
            Ok(vec![pair(SAM, "laptop"), pair(SAM, "phone")])
        );
        assert_eq!(
            data.store.read_permitted(&sam, &OWN_INVITATIONS),
            Ok(vec![pair(SAM, "a guest of sam")])
        );
        assert_eq!(
            data.store.read_permitted(&kim, &OWN_DEVICES),
            Ok(vec![pair(KIM, "tablet")])
        );
        assert_eq!(
            data.store.read_permitted(&kim, &OWN_INVITATIONS),
            Ok(vec![pair(KIM, "a guest of kim")])
        );
    }

    /// Verifies: SEC-TM-024
    #[test]
    fn a_statement_that_returns_someone_else_s_row_returns_nothing() {
        let data = data();
        let sam = allowed(&sam(), Action::ReadOwnData, &account_of(SAM));
        let kim = allowed(&kim(), Action::ReadOwnData, &account_of(KIM));
        // A statement that forgot its filter: it takes the person and lets
        // every row through, the laptop and the phone before the tablet.
        let unfiltered = PersonalRead {
            action: Action::ReadOwnData,
            query: Query::new("SELECT owner, name FROM devices WHERE ?1 IS NOT NULL ORDER BY name"),
        };
        assert_eq!(
            data.store.read_permitted(&sam, &unfiltered),
            Err(PermittedError::ForeignRow { row: 2 })
        );
        assert_eq!(
            data.store.read_permitted(&kim, &unfiltered),
            Err(PermittedError::ForeignRow { row: 0 })
        );
        // A statement that filters but does not put the person first.
        let unmarked = PersonalRead {
            action: Action::ReadOwnData,
            query: Query::new("SELECT name, owner FROM devices WHERE owner = ?1 ORDER BY name"),
        };
        assert_eq!(
            data.store.read_permitted(&sam, &unmarked),
            Err(PermittedError::ForeignRow { row: 0 })
        );
    }

    /// Verifies: SEC-TM-024
    #[test]
    fn a_permit_decided_for_another_action_opens_no_read() {
        let data = data();
        // A permit to change one's own data is not one to read it.
        let write = allowed(&sam(), Action::WriteOwnData, &account_of(SAM));
        assert_eq!(
            data.store.read_permitted(&write, &OWN_DEVICES),
            Err(PermittedError::WrongAction {
                needed: Action::ReadOwnData,
                decided: Action::WriteOwnData,
            })
        );
    }

    /// Verifies: SEC-TM-024
    #[test]
    fn a_permit_decided_about_nobody_opens_no_personal_read() {
        let data = data();
        let browse = allowed(&sam(), Action::BrowseLibrary, &ResourceFacts::Server);
        let browsed = PersonalRead {
            action: Action::BrowseLibrary,
            query: Query::new("SELECT owner, name FROM devices WHERE owner = ?1 ORDER BY name"),
        };
        assert_eq!(
            data.store.read_permitted(&browse, &browsed),
            Err(PermittedError::NoPerson)
        );
    }

    /// Verifies: SEC-TM-024
    #[test]
    fn a_permit_about_a_profile_reads_that_profile_s_rows() {
        let data = data();
        let own_bookmarks = PersonalRead {
            action: Action::ReadOwnData,
            query: Query::new("SELECT owner, title FROM bookmarks WHERE owner = ?1"),
        };
        let profile = ResourceFacts::Owned(Owner::Profile(id(KIM_PROFILE, IdKind::Profile)));
        let kim = allowed(&kim(), Action::ReadOwnData, &profile);
        assert_eq!(
            data.store.read_permitted(&kim, &own_bookmarks),
            Ok(vec![pair(KIM_PROFILE, "where kim stopped")])
        );
        // A profile is not an account, so it opens no account's devices.
        assert_eq!(data.store.read_permitted(&kim, &OWN_DEVICES), Ok(vec![]));
    }

    /// Verifies: SEC-TM-024
    #[test]
    fn a_permit_about_another_person_reads_that_person_s_rows() {
        let data = data();
        let kim = ResourceFacts::Person {
            account: id(KIM, IdKind::User),
            kind: PrincipalKind::Member,
        };
        let manage = allowed(&owner(), Action::ManageUser, &kim);
        let invitations_of = PersonalRead {
            action: Action::ManageUser,
            query: Query::new(
                "SELECT owner, guest FROM invitations WHERE owner = ?1 ORDER BY guest",
            ),
        };
        assert_eq!(
            data.store.read_permitted(&manage, &invitations_of),
            Ok(vec![pair(KIM, "a guest of kim")])
        );
        // The same permit is not one to read the owner's own data.
        assert_eq!(
            data.store.read_permitted(&manage, &OWN_DEVICES),
            Err(PermittedError::WrongAction {
                needed: Action::ReadOwnData,
                decided: Action::ManageUser,
            })
        );
    }

    #[test]
    fn the_person_is_bound_after_the_statement_s_own_parameters() {
        let data = data();
        let named = PersonalRead {
            action: Action::ReadOwnData,
            query: Query::new("SELECT owner, name FROM devices WHERE name = ?1 AND owner = ?2")
                .bind(text("phone")),
        };
        let sam = allowed(&sam(), Action::ReadOwnData, &account_of(SAM));
        let kim = allowed(&kim(), Action::ReadOwnData, &account_of(KIM));
        assert_eq!(
            data.store.read_permitted(&sam, &named),
            Ok(vec![pair(SAM, "phone")])
        );
        assert_eq!(data.store.read_permitted(&kim, &named), Ok(vec![]));
    }

    #[test]
    fn a_read_the_store_refuses_reports_the_store_s_error_and_changes_nothing() {
        let data = data();
        let sam = allowed(&sam(), Action::ReadOwnData, &account_of(SAM));
        let missing = PersonalRead {
            action: Action::ReadOwnData,
            query: Query::new("SELECT owner FROM sessions WHERE owner = ?1"),
        };
        assert_eq!(
            data.store.read_permitted(&sam, &missing),
            Err(PermittedError::Store(IdentityError::Db {
                step: Step::Read,
                // SQLITE_ERROR: no such table
                error: DbError::Sqlite { code: 1 },
            }))
        );
        let write = PersonalRead {
            action: Action::ReadOwnData,
            query: Query::new("DELETE FROM devices WHERE owner = ?1"),
        };
        assert_eq!(
            data.store.read_permitted(&sam, &write),
            Err(PermittedError::Store(IdentityError::Db {
                step: Step::Read,
                // SQLITE_READONLY
                error: DbError::Sqlite { code: 8 },
            }))
        );
        assert_eq!(
            data.store.read_permitted(&sam, &OWN_DEVICES),
            Ok(vec![pair(SAM, "laptop"), pair(SAM, "phone")])
        );
    }
}
