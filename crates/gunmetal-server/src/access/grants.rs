//! Library grants: which account may see which library.
//!
//! Grants live in the identity store (ADR 3; record 7, section 8), in the
//! table this module registers as [`LIBRARY_GRANTS`]. The owner and
//! administrators see every library through a capability and hold no row
//! here; everyone else sees a library only while a row grants it, so a new
//! library is visible to nobody else until someone grants it.
//!
//! This module reads or changes the table in three ways, and each of the
//! three is held to a decision of the policy:
//!
//! - **The grant read that builds a `Permit`.** Asking the policy needs the
//!   principal's grants, and there is no permit yet to read them under, so
//!   this is one of the three written pre-principal lookups of the identity
//!   store. It is private to the layer: [`permit`](super::policy::permit)
//!   is its only caller, and a handler has no way to it.
//! - **Reading a person's grants** ([`granted`]) goes through the identity
//!   store's reader that takes a `Permit`, and shows the holder only the
//!   libraries the holder may see.
//! - **Changing a person's grants** ([`grant`], [`revoke`]) takes the permit
//!   the policy gave for managing that person. The person is the one the
//!   permit names, never one the caller names, and the library must be one
//!   the permit's holder may see.
//!
//! Those are the ways through this module, not the only ways to the table.
//! The others, and what holds each:
//!
//! - **The store's writer.** [`IdentityStore::write`] is public and takes
//!   no `Permit`, so any module of the server that holds the store can
//!   insert or delete a grant without asking the policy. Nothing but review
//!   holds that: the architecture test (`tests/storage_access.rs`) does not
//!   watch the writer.
//! - **The store's pre-principal grant lookup** is public too. The
//!   architecture test lets only this directory name it.
//! - **A raw connection.** A connection opened on the identity database
//!   with the SQLite door's `open_db` runs any statement on this table. The
//!   architecture test lets only this directory, `session/` and `verifier/`
//!   name the opener, and does not look at what those three open with it.
//!
//! Nothing here remembers a grant between calls. Every question to the
//! policy reads the table again, so a change takes effect on the next
//! request (SEC-IAM-076).

use gunmetal_core::authz::{Action, Permit};
use gunmetal_core::id::{IdKind, PublicId};
use gunmetal_core::problem::{Describe, Problem, ProblemCode};
use gunmetal_core::schema::{Column, DataClass, SchemaPart};
use gunmetal_durable::identity::error::IdentityError;
use gunmetal_durable::identity::permitted::{PermittedError, PersonalRead};
use gunmetal_durable::identity::pre_principal::PrePrincipal;
use gunmetal_durable::identity::store::IdentityStore;
use gunmetal_fs::sqlite::{Query, Row, Value};

/// The grants table: one row for each library an account may see. Who may
/// see what is identity data.
pub const LIBRARY_GRANTS: SchemaPart = SchemaPart {
    name: "access.library_grants",
    sql: "CREATE TABLE library_grants (\
              account TEXT NOT NULL, \
              library TEXT NOT NULL, \
              PRIMARY KEY (account, library)) WITHOUT ROWID;",
    columns: &[
        Column {
            table: "library_grants",
            name: "account",
            class: DataClass::Identity,
        },
        Column {
            table: "library_grants",
            name: "library",
            class: DataClass::Identity,
        },
    ],
};

/// The libraries one account holds a grant for.
const HELD: Query =
    Query::new("SELECT library FROM library_grants WHERE account = ?1 ORDER BY library");

/// A person's grants, for whoever was allowed to manage that person.
const GRANTS_OF: PersonalRead = PersonalRead {
    action: Action::ManageUser,
    query: Query::new(
        "SELECT account, library FROM library_grants WHERE account = ?1 ORDER BY library",
    ),
};

/// Stores a grant unless the account already holds it.
const GRANT: Query = Query::new(
    "INSERT INTO library_grants (account, library) VALUES (?1, ?2) \
     ON CONFLICT (account, library) DO NOTHING",
);

/// Removes a grant.
const REVOKE: Query = Query::new("DELETE FROM library_grants WHERE account = ?1 AND library = ?2");

/// Why grants were not read or changed.
#[derive(Debug, Clone, PartialEq)]
pub enum GrantError {
    /// The permit was not decided for managing a person, so it names nobody
    /// whose grants it could change.
    WrongPermit,
    /// The library is not one the permit's holder may see, or the
    /// identifier is not a library's. The two are one answer, as for any
    /// object the holder may not see (SEC-API-011).
    NotVisible,
    /// The identity store's reader refused the read of a person's grants.
    Read(PermittedError),
    /// The identity store failed or refused the statement.
    Store(IdentityError),
    /// A stored grant does not name a library, so none of the account's
    /// grants were returned.
    Malformed {
        /// The row as it was read.
        found: Row,
    },
}

impl Describe for GrantError {
    /// A permit that does not open the change, and a library its holder may
    /// not see, answer as not found, like the management function itself
    /// for someone who may not use it. A failure of the store is the
    /// generic error.
    fn problem(&self) -> Problem {
        let code = match self {
            Self::WrongPermit
            | Self::NotVisible
            | Self::Read(PermittedError::WrongAction { .. } | PermittedError::NoPerson) => {
                ProblemCode::NotFound
            }
            Self::Read(PermittedError::ForeignRow { .. } | PermittedError::Store(_))
            | Self::Store(_)
            | Self::Malformed { .. } => ProblemCode::InternalError,
        };
        Problem {
            code,
            args: Vec::new(),
        }
    }
}

/// The libraries `account` holds a grant for right now, in the order of
/// their identifiers.
///
/// This is the identity store's pre-principal grant lookup. It takes no
/// `Permit`, because its answer is what the policy is asked with, so it is
/// visible only inside the layer.
pub(super) fn held(store: &IdentityStore, account: PublicId) -> Result<Vec<PublicId>, GrantError> {
    store
        .read_pre_principal(
            PrePrincipal::Grant,
            &HELD.bind(Value::Text(account.to_string())),
        )
        .map_err(GrantError::Store)
        .and_then(|rows| rows.into_iter().map(|row| library_of(row, 0)).collect())
}

/// The library a stored grant names, read from column `column` of its row.
fn library_of(row: Row, column: usize) -> Result<PublicId, GrantError> {
    let library = match row.0.get(column) {
        Some(Value::Text(text)) => PublicId::parse(text, IdKind::Library).ok(),
        _ => None,
    };
    library.ok_or(GrantError::Malformed { found: row })
}

/// The libraries granted to the person `permit` was decided about, of those
/// the permit's holder may see, in the order of their identifiers.
///
/// # Errors
///
/// Returns [`GrantError::Read`] when the identity store's reader refuses:
/// `permit` was not decided for managing a person, or the store failed. It
/// returns [`GrantError::Malformed`] when a stored grant does not name a
/// library.
pub fn granted(store: &IdentityStore, permit: &Permit) -> Result<Vec<PublicId>, GrantError> {
    store
        .read_permitted(permit, &GRANTS_OF)
        .map_err(GrantError::Read)
        .and_then(|rows| {
            rows.into_iter()
                .map(|row| library_of(row, 1))
                .collect::<Result<Vec<_>, _>>()
        })
        .map(|libraries| {
            libraries
                .into_iter()
                .filter(|library| permit.libraries().contains(library))
                .collect()
        })
}

/// Grants `library` to the person `permit` was decided about, and returns
/// whether the grant is new.
///
/// # Errors
///
/// Returns [`GrantError::WrongPermit`] unless `permit` was decided for
/// managing a person, [`GrantError::NotVisible`] unless `library` is a
/// library the permit's holder may see, and [`GrantError::Store`] when the
/// identity store fails. Nothing is stored in any of these cases.
pub fn grant(
    store: &IdentityStore,
    permit: &Permit,
    library: PublicId,
) -> Result<bool, GrantError> {
    change(store, permit, library, GRANT)
}

/// Takes `library` from the person `permit` was decided about, and returns
/// whether they held it.
///
/// # Errors
///
/// As [`grant`]. Nothing is removed in any of those cases.
pub fn revoke(
    store: &IdentityStore,
    permit: &Permit,
    library: PublicId,
) -> Result<bool, GrantError> {
    change(store, permit, library, REVOKE)
}

/// Runs `statement`, whose parameters are an account and a library, for the
/// person `permit` was decided about, and returns whether it changed a row.
fn change(
    store: &IdentityStore,
    permit: &Permit,
    library: PublicId,
    statement: Query,
) -> Result<bool, GrantError> {
    let account = person(permit)?;
    let library = named(permit, library)?;
    store
        .write(&[statement
            .bind(Value::Text(account.to_string()))
            .bind(Value::Text(library))])
        .map_err(GrantError::Store)
        .map(|changed| changed == [1])
}

/// The account whose grants `permit` lets its holder change: the person an
/// allowed `user.manage` was decided about.
fn person(permit: &Permit) -> Result<PublicId, GrantError> {
    permit
        .target()
        .filter(|_| permit.action() == Action::ManageUser)
        .map(|person| person.account)
        .ok_or(GrantError::WrongPermit)
}

/// `library` as the store keeps it, when it is a library `permit`'s holder
/// may see.
fn named(permit: &Permit, library: PublicId) -> Result<String, GrantError> {
    let text = library.to_string();
    if permit.libraries().contains(&library) && PublicId::parse(&text, IdKind::Library).is_ok() {
        Ok(text)
    } else {
        Err(GrantError::NotVisible)
    }
}

#[cfg(test)]
mod tests {
    use super::{GrantError, grant, granted, revoke};
    use crate::access::testing::{
        account, data, decided, give, give_raw, library, lose_grants, member, owner, pair, person,
        principal, stored,
    };
    use gunmetal_core::authz::{
        Action, Capability, CapabilitySet, Owner, Permit, PrincipalKind, ResourceFacts,
    };
    use gunmetal_core::id::{IdKind, PublicId};
    use gunmetal_core::problem::{Describe, Problem, ProblemCode};
    use gunmetal_durable::identity::error::{IdentityError, Step};
    use gunmetal_durable::identity::permitted::PermittedError;
    use gunmetal_fs::sqlite::{DbError, Row, Value};

    /// The owner's permit to manage member `n`. The owner sees every
    /// library.
    fn manage(n: u8) -> Permit {
        decided(
            &owner(),
            &[],
            Action::ManageUser,
            &person(n, PrincipalKind::Member),
        )
    }

    /// The permit to manage member `n` of an administrator, account 8, who
    /// holds `user.manage` but not `library.all`, and so sees only library
    /// 1, which they were granted.
    fn manage_seeing_library_1(n: u8) -> Permit {
        let capabilities = CapabilitySet::of(&[Capability::LibraryRead, Capability::UserManage]);
        decided(
            &principal(PrincipalKind::Administrator, capabilities, 8),
            &[1],
            Action::ManageUser,
            &person(n, PrincipalKind::Member),
        )
    }

    /// A track's identifier, which is not a library's.
    fn track() -> PublicId {
        PublicId::parse("trk_00000000000000000000000001", IdKind::Track)
            .expect("a canonical identifier")
    }

    #[test]
    fn granting_and_revoking_change_one_row_and_say_whether_they_did() {
        let data = data();
        let sam = manage(1);
        assert_eq!(grant(&data.identity, &sam, library(1)), Ok(true));
        assert_eq!(grant(&data.identity, &sam, library(2)), Ok(true));
        assert_eq!(grant(&data.identity, &sam, library(1)), Ok(false));
        assert_eq!(grant(&data.identity, &manage(2), library(2)), Ok(true));
        assert_eq!(
            stored(&data),
            [
                pair(account(1), library(1)),
                pair(account(1), library(2)),
                pair(account(2), library(2)),
            ]
        );
        assert_eq!(revoke(&data.identity, &sam, library(2)), Ok(true));
        assert_eq!(revoke(&data.identity, &sam, library(2)), Ok(false));
        assert_eq!(revoke(&data.identity, &sam, library(3)), Ok(false));
        assert_eq!(
            stored(&data),
            [pair(account(1), library(1)), pair(account(2), library(2))]
        );
    }

    /// Verifies: SEC-TM-024
    #[test]
    fn only_a_permit_to_manage_a_person_changes_that_person_s_grants() {
        let data = data();
        give(&data, account(1), library(1));
        // Decided about member 1, but for ending their sessions.
        let end_sessions = decided(
            &owner(),
            &[],
            Action::EndSessions,
            &person(1, PrincipalKind::Member),
        );
        // Decided for an action that is about nobody.
        let browse = decided(&owner(), &[], Action::BrowseLibrary, &ResourceFacts::Server);
        // Member 1's own permit to change their own data.
        let own = decided(
            &member(1),
            &[1],
            Action::WriteOwnData,
            &ResourceFacts::Owned(Owner::Account(account(1))),
        );
        for permit in [&end_sessions, &browse, &own] {
            assert_eq!(
                grant(&data.identity, permit, library(2)),
                Err(GrantError::WrongPermit)
            );
            assert_eq!(
                revoke(&data.identity, permit, library(1)),
                Err(GrantError::WrongPermit)
            );
        }
        assert_eq!(stored(&data), [pair(account(1), library(1))]);
    }

    /// Verifies: SEC-TM-024
    #[test]
    fn a_grant_names_only_a_library_its_giver_may_see() {
        let data = data();
        give(&data, account(1), library(2));
        let narrow = manage_seeing_library_1(1);
        assert_eq!(
            grant(&data.identity, &narrow, library(2)),
            Err(GrantError::NotVisible)
        );
        assert_eq!(
            revoke(&data.identity, &narrow, library(2)),
            Err(GrantError::NotVisible)
        );
        assert_eq!(grant(&data.identity, &narrow, library(1)), Ok(true));
        // The owner sees every library, and a track is not one.
        assert_eq!(
            grant(&data.identity, &manage(1), track()),
            Err(GrantError::NotVisible)
        );
        assert_eq!(
            stored(&data),
            [pair(account(1), library(1)), pair(account(1), library(2))]
        );
    }

    #[test]
    fn a_change_the_store_refuses_reports_the_store_s_error() {
        let data = data();
        lose_grants(&data);
        // SQLITE_ERROR: no such table
        let refused = Err(GrantError::Store(IdentityError::Db {
            step: Step::Write,
            error: DbError::Sqlite { code: 1 },
        }));
        assert_eq!(grant(&data.identity, &manage(1), library(1)), refused);
        assert_eq!(revoke(&data.identity, &manage(1), library(1)), refused);
    }

    /// Verifies: SEC-TM-024
    #[test]
    fn a_person_s_grants_are_read_under_a_permit_to_manage_them() {
        let data = data();
        give(&data, account(1), library(2));
        give(&data, account(1), library(1));
        give(&data, account(2), library(3));
        assert_eq!(
            granted(&data.identity, &manage(1)),
            Ok(vec![library(1), library(2)])
        );
        assert_eq!(granted(&data.identity, &manage(2)), Ok(vec![library(3)]));
        assert_eq!(granted(&data.identity, &manage(3)), Ok(vec![]));
        // An administrator who sees only library 1 learns of no other.
        assert_eq!(
            granted(&data.identity, &manage_seeing_library_1(1)),
            Ok(vec![library(1)])
        );
        assert_eq!(
            granted(&data.identity, &manage_seeing_library_1(2)),
            Ok(vec![])
        );
        // A permit decided about the same person for another action.
        let end_sessions = decided(
            &owner(),
            &[],
            Action::EndSessions,
            &person(1, PrincipalKind::Member),
        );
        assert_eq!(
            granted(&data.identity, &end_sessions),
            Err(GrantError::Read(PermittedError::WrongAction {
                needed: Action::ManageUser,
                decided: Action::EndSessions,
            }))
        );
    }

    #[test]
    fn a_stored_grant_that_names_no_library_is_refused_with_the_rest() {
        let malformed = [
            Value::Text("trk_00000000000000000000000001".to_owned()),
            Value::Text("lib_".to_owned()),
            Value::Blob(b"lib_00000000000000000000000002".to_vec()),
        ];
        for library_column in malformed {
            let data = data();
            give(&data, account(1), library(1));
            give_raw(&data, account(1), library_column.clone());
            assert_eq!(
                granted(&data.identity, &manage(1)),
                Err(GrantError::Malformed {
                    found: Row(vec![Value::Text(account(1).to_string()), library_column]),
                })
            );
        }
    }

    #[test]
    fn a_read_the_store_refuses_reports_the_store_s_error() {
        let data = data();
        lose_grants(&data);
        assert_eq!(
            granted(&data.identity, &manage(1)),
            Err(GrantError::Read(PermittedError::Store(IdentityError::Db {
                step: Step::Read,
                // SQLITE_ERROR: no such table
                error: DbError::Sqlite { code: 1 },
            })))
        );
    }

    #[test]
    fn each_refusal_describes_itself_by_the_catalogue() {
        let failed = IdentityError::Db {
            step: Step::Write,
            error: DbError::Sqlite { code: 1 },
        };
        let cases = [
            (GrantError::WrongPermit, ProblemCode::NotFound),
            (GrantError::NotVisible, ProblemCode::NotFound),
            (
                GrantError::Read(PermittedError::WrongAction {
                    needed: Action::ManageUser,
                    decided: Action::EndSessions,
                }),
                ProblemCode::NotFound,
            ),
            (
                GrantError::Read(PermittedError::NoPerson),
                ProblemCode::NotFound,
            ),
            (
                GrantError::Read(PermittedError::ForeignRow { row: 0 }),
                ProblemCode::InternalError,
            ),
            (
                GrantError::Read(PermittedError::Store(failed.clone())),
                ProblemCode::InternalError,
            ),
            (GrantError::Store(failed), ProblemCode::InternalError),
            (
                GrantError::Malformed {
                    found: Row(vec![Value::Null]),
                },
                ProblemCode::InternalError,
            ),
        ];
        for (error, code) in cases {
            assert_eq!(
                error.problem(),
                Problem {
                    code,
                    args: Vec::new(),
                }
            );
        }
    }
}
