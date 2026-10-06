//! What a handler may hold of a stored row: `Visible<T>` and `Editable<T>`.
//!
//! A handler is to hold a stored object only inside one of these two
//! wrappers, and only the functions here can make one: the wrapped row is a
//! private field, so no code outside this file can build a wrapper around a
//! row it fetched some other way (SEC-API-010). The type holds the second
//! half only. Nothing here makes a handler wrap a row before it serves it. Each function applies the one
//! visibility predicate, the library set inside the caller's `Permit`,
//! which the policy built from the principal's grants (SEC-IAM-070). Any
//! row type that says which library it belongs to, through the core's
//! `HasLibrary`, passes through the same check: tracks, playlist entries,
//! lyrics, artwork, search hits, sync records and pushed events alike
//! (SEC-API-014, SEC-TM-026, SEC-MED-051).
//!
//! - [`check`] is for one object a request named. A row in a library the
//!   caller may not see gets the same answer as an identifier that names
//!   nothing: not found, with no word about which (SEC-API-011,
//!   SEC-HIS-010).
//! - [`check_all`] is for a request that names several. If any one of them
//!   is hidden or missing the whole request is refused, and nothing of the
//!   others is returned (SEC-API-012).
//! - [`check_edit`] is for a row the request changes, and needs the permit
//!   the policy gave for managing the library.
//! - [`only_visible`] is for a list the server builds itself, such as
//!   search results or a sync page: rows the caller may not see are left
//!   out.
//! - [`visible_to`] is the check for one recipient of a pushed event: it
//!   wraps the event when that recipient's permit holds the event's
//!   library, and nothing here wraps an event for more than one recipient
//!   at a time (SEC-API-016). What it does not hold: the wrapper does not
//!   record which recipient it was made for, so the type does not stop a
//!   fan-out from writing one recipient's wrapper to another's socket; and
//!   the check is by library only, so an event that belongs to a person and
//!   not to a library needs a check this function does not make. Both are
//!   the fan-out's to hold (WP-083).

use gunmetal_core::authz::{Action, HasLibrary, Permit};
use gunmetal_core::problem::ProblemCode;
use gunmetal_http::problem::ApiError;

/// The one answer for an object the caller may not see and for one that
/// does not exist.
const NOT_FOUND: ApiError = ApiError::new(ProblemCode::NotFound);

/// A stored row the caller may see.
#[derive(Debug, PartialEq, Eq)]
pub struct Visible<T>(T);

impl<T> Visible<T> {
    /// The row.
    #[must_use]
    pub const fn get(&self) -> &T {
        &self.0
    }

    /// The row, given up by the wrapper, to build a response from.
    #[must_use]
    pub fn into_inner(self) -> T {
        self.0
    }
}

/// A stored row the caller may change.
#[derive(Debug, PartialEq, Eq)]
pub struct Editable<T>(T);

impl<T> Editable<T> {
    /// The row.
    #[must_use]
    pub const fn get(&self) -> &T {
        &self.0
    }

    /// The row, given up by the wrapper, to change and store.
    #[must_use]
    pub fn into_inner(self) -> T {
        self.0
    }
}

/// Wraps the row a request named, when `permit`'s holder may see it.
///
/// # Errors
///
/// Returns the not-found problem when there is no such row and when the
/// row is in a library the holder may not see. The two are the same value,
/// so no caller can answer them differently.
pub fn check<T: HasLibrary>(permit: &Permit, row: Option<T>) -> Result<Visible<T>, ApiError> {
    row.filter(|row| permit.libraries().admits(row))
        .map(Visible)
        .ok_or(NOT_FOUND)
}

/// Wraps every row a request named, when `permit`'s holder may see them
/// all.
///
/// # Errors
///
/// Returns the not-found problem when any one of the rows is missing or in
/// a library the holder may not see. None of the rows is returned then.
pub fn check_all<T: HasLibrary>(
    permit: &Permit,
    rows: impl IntoIterator<Item = Option<T>>,
) -> Result<Vec<Visible<T>>, ApiError> {
    rows.into_iter().map(|row| check(permit, row)).collect()
}

/// Wraps the row a request changes, when `permit` was decided for managing
/// the library and its holder may see the row.
///
/// # Errors
///
/// Returns the not-found problem when there is no such row, when the row is
/// in a library the holder may not see, and when `permit` was decided for
/// any other action: someone who may not manage a library is not told that
/// it can be managed.
pub fn check_edit<T: HasLibrary>(permit: &Permit, row: Option<T>) -> Result<Editable<T>, ApiError> {
    row.filter(|row| permit.action() == Action::ManageLibrary && permit.libraries().admits(row))
        .map(Editable)
        .ok_or(NOT_FOUND)
}

/// The rows of a list the server built that `permit`'s holder may see, in
/// the order given. The others are left out.
#[must_use]
pub fn only_visible<T: HasLibrary>(
    permit: &Permit,
    rows: impl IntoIterator<Item = T>,
) -> Vec<Visible<T>> {
    rows.into_iter()
        .filter(|row| permit.libraries().admits(row))
        .map(Visible)
        .collect()
}

/// A pushed event as one recipient may see it, or nothing when the event
/// is about a library that recipient may not see. `recipient` is the
/// permit the policy gave that recipient.
#[must_use]
pub fn visible_to<'e, T: HasLibrary>(recipient: &Permit, event: &'e T) -> Option<Visible<&'e T>> {
    recipient
        .libraries()
        .admits(event)
        .then_some(Visible(event))
}

/// What code outside the layer must not be able to do with the wrappers.
/// Rustdoc on stable does not check which error a compile-fail test
/// produced, so each test shares its imports and its row type with the
/// control, which compiles.
#[cfg(doctest)]
mod compile_fail {
    /// Control: outside the layer, a wrapper comes from a check under a
    /// permit, and the row inside it can be read.
    ///
    /// ```
    /// use gunmetal_core::authz::{HasLibrary, Permit};
    /// use gunmetal_core::id::PublicId;
    /// use gunmetal_http::problem::ApiError;
    /// use gunmetal_server::access::{Editable, Visible, check, check_edit};
    ///
    /// struct Track {
    ///     library: PublicId,
    /// }
    ///
    /// impl HasLibrary for Track {
    ///     fn library(&self) -> PublicId {
    ///         self.library
    ///     }
    /// }
    ///
    /// fn fetch(permit: &Permit, row: Option<Track>) -> Result<Visible<Track>, ApiError> {
    ///     check(permit, row)
    /// }
    ///
    /// fn change(permit: &Permit, row: Option<Track>) -> Result<Editable<Track>, ApiError> {
    ///     check_edit(permit, row)
    /// }
    ///
    /// fn read(seen: &Visible<Track>, held: &Editable<Track>) -> (PublicId, PublicId) {
    ///     (seen.get().library, held.get().library)
    /// }
    /// ```
    struct Control;

    /// Verifies: SEC-API-010
    ///
    /// `Visible` has no constructor a handler can call.
    ///
    /// ```compile_fail,E0599
    /// use gunmetal_core::authz::{HasLibrary, Permit};
    /// use gunmetal_core::id::PublicId;
    /// use gunmetal_http::problem::ApiError;
    /// use gunmetal_server::access::{Editable, Visible, check, check_edit};
    ///
    /// struct Track {
    ///     library: PublicId,
    /// }
    ///
    /// impl HasLibrary for Track {
    ///     fn library(&self) -> PublicId {
    ///         self.library
    ///     }
    /// }
    ///
    /// fn forge(row: Track) -> Visible<Track> {
    ///     Visible::new(row)
    /// }
    /// ```
    struct NoVisibleConstructor;

    /// Verifies: SEC-API-010
    ///
    /// The row inside a `Visible` is a private field, so a handler cannot
    /// wrap a row itself.
    ///
    /// ```compile_fail,E0423
    /// use gunmetal_core::authz::{HasLibrary, Permit};
    /// use gunmetal_core::id::PublicId;
    /// use gunmetal_http::problem::ApiError;
    /// use gunmetal_server::access::{Editable, Visible, check, check_edit};
    ///
    /// struct Track {
    ///     library: PublicId,
    /// }
    ///
    /// impl HasLibrary for Track {
    ///     fn library(&self) -> PublicId {
    ///         self.library
    ///     }
    /// }
    ///
    /// fn forge(row: Track) -> Visible<Track> {
    ///     Visible(row)
    /// }
    /// ```
    struct NoVisibleLiteral;

    /// Verifies: SEC-API-010
    ///
    /// `Editable` has no constructor a handler can call.
    ///
    /// ```compile_fail,E0599
    /// use gunmetal_core::authz::{HasLibrary, Permit};
    /// use gunmetal_core::id::PublicId;
    /// use gunmetal_http::problem::ApiError;
    /// use gunmetal_server::access::{Editable, Visible, check, check_edit};
    ///
    /// struct Track {
    ///     library: PublicId,
    /// }
    ///
    /// impl HasLibrary for Track {
    ///     fn library(&self) -> PublicId {
    ///         self.library
    ///     }
    /// }
    ///
    /// fn forge(row: Track) -> Editable<Track> {
    ///     Editable::new(row)
    /// }
    /// ```
    struct NoEditableConstructor;

    /// Verifies: SEC-API-010
    ///
    /// The row inside an `Editable` is a private field, and a `Visible` is
    /// not an `Editable`: seeing a row does not let a handler change it.
    ///
    /// ```compile_fail,E0423
    /// use gunmetal_core::authz::{HasLibrary, Permit};
    /// use gunmetal_core::id::PublicId;
    /// use gunmetal_http::problem::ApiError;
    /// use gunmetal_server::access::{Editable, Visible, check, check_edit};
    ///
    /// struct Track {
    ///     library: PublicId,
    /// }
    ///
    /// impl HasLibrary for Track {
    ///     fn library(&self) -> PublicId {
    ///         self.library
    ///     }
    /// }
    ///
    /// fn forge(seen: Visible<Track>) -> Editable<Track> {
    ///     Editable(seen.into_inner())
    /// }
    /// ```
    struct NoEditableLiteral;
}

#[cfg(test)]
mod tests {
    use super::{Editable, Visible, check, check_all, check_edit, only_visible, visible_to};
    use crate::access::grants::revoke;
    use crate::access::policy::permit;
    use crate::access::testing::{
        Data, account, ask, data, decided, give, internet, library, member, owner, person,
        principal,
    };
    use crate::testing::Recording;
    use gunmetal_core::authz::{
        Action, Capability, CapabilitySet, HasLibrary, Owner, Permit, Principal, PrincipalKind,
        ResourceFacts, Role,
    };
    use gunmetal_core::id::PublicId;
    use gunmetal_core::problem::ProblemCode;
    use gunmetal_core::schema::{Column, DataClass, SchemaPart};
    use gunmetal_fs::path::DataDir;
    use gunmetal_fs::sqlite::{Db, DbFile, Pragmas, Query, Row, Synchronous, Value, open_db};
    use gunmetal_http::problem::{ApiError, RequestId, render};
    use proptest::prelude::*;

    /// The fixture row type: something stored that belongs to one library.
    /// Its kind says what it stands in for.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Item {
        id: String,
        library: PublicId,
        kind: String,
    }

    impl HasLibrary for Item {
        fn library(&self) -> PublicId {
            self.library
        }
    }

    /// The test schema part: the fixture rows' table in the cache.
    const ITEMS: SchemaPart = SchemaPart {
        name: "test.items",
        sql: "CREATE TABLE items (\
                  id TEXT PRIMARY KEY, \
                  library TEXT NOT NULL, \
                  kind TEXT NOT NULL) STRICT;",
        columns: &[
            Column {
                table: "items",
                name: "id",
                class: DataClass::Library,
            },
            Column {
                table: "items",
                name: "library",
                class: DataClass::Library,
            },
            Column {
                table: "items",
                name: "kind",
                class: DataClass::Library,
            },
        ],
    };

    const CACHE: DbFile = DbFile::new(DataDir::Cache, "library.db");
    const ADD: Query = Query::new("INSERT INTO items (id, library, kind) VALUES (?1, ?2, ?3)");
    const BY_ID: Query = Query::new("SELECT id, library, kind FROM items WHERE id = ?1");
    const EVERY: Query = Query::new("SELECT id, library, kind FROM items ORDER BY id");

    /// The rows the fixture table holds, in two libraries: a track, a
    /// playlist entry, lyrics and artwork in each.
    const ROWS: [(&str, u8, &str); 8] = [
        ("a1", 1, "track"),
        ("a2", 2, "track"),
        ("b1", 1, "playlist entry"),
        ("b2", 2, "playlist entry"),
        ("c1", 1, "lyrics"),
        ("c2", 2, "lyrics"),
        ("d1", 1, "artwork"),
        ("d2", 2, "artwork"),
    ];

    fn text(value: &str) -> Value {
        Value::Text(value.to_owned())
    }

    /// The fixture item `id` of kind `kind` in library `n`, written out
    /// independently of the table.
    fn item(id: &str, n: u8, kind: &str) -> Item {
        Item {
            id: id.to_owned(),
            library: library(n),
            kind: kind.to_owned(),
        }
    }

    /// The values an item is stored as: its identifier, its library and
    /// its kind.
    fn stored(item: &Item) -> Vec<Value> {
        vec![
            text(&item.id),
            text(&item.library.to_string()),
            text(&item.kind),
        ]
    }

    /// A cache database holding [`ROWS`], in the data directory of `data`.
    fn cache(data: &Data) -> Db {
        let db = open_db(&data.root, &CACHE, Pragmas::new(Synchronous::Normal))
            .expect("the cache opens");
        db.execute_batch(ITEMS.sql).expect("the table is created");
        for (id, n, kind) in ROWS {
            let add = stored(&item(id, n, kind))
                .into_iter()
                .fold(ADD, Query::bind);
            db.execute(&add).expect("the row is written");
        }
        db
    }

    /// Reads a stored row as an item: the one row of [`ROWS`] that is
    /// stored as exactly the values read.
    fn decode(row: &Row) -> Item {
        ROWS.into_iter()
            .map(|(id, n, kind)| item(id, n, kind))
            .find(|item| stored(item) == row.0)
            .expect("a row of the fixture table")
    }

    /// The stored item `id`, if there is one, as a storage reader hands it
    /// to the layer.
    fn find(cache: &Db, id: &str) -> Option<Item> {
        cache
            .query(&BY_ID.bind(text(id)))
            .expect("the query runs")
            .first()
            .map(decode)
    }

    /// Every stored item, by identifier.
    fn every(cache: &Db) -> Vec<Item> {
        cache
            .query(&EVERY)
            .expect("the query runs")
            .iter()
            .map(decode)
            .collect()
    }

    /// The identifiers of wrapped items.
    fn ids(seen: &[Visible<Item>]) -> Vec<&str> {
        seen.iter().map(|item| item.get().id.as_str()).collect()
    }

    /// Member `n`'s permit to browse, holding grants for `libraries`.
    fn browse(n: u8, libraries: &[u8]) -> Permit {
        decided(
            &member(n),
            libraries,
            Action::BrowseLibrary,
            &ResourceFacts::Server,
        )
    }

    /// The permit the layer gives `principal` for `action`, with whatever
    /// grants the store holds for it at this moment.
    fn asked(data: &Data, principal: &Principal, action: Action) -> Permit {
        permit(
            &data.identity,
            &Recording::new(true),
            &ask(principal, action, &ResourceFacts::Server, &internet()),
        )
        .expect("the policy allows it")
    }

    /// The whole response a refusal becomes, for request 7.
    fn response<T>(outcome: Result<T, ApiError>) -> Option<(u16, String)> {
        outcome.err().map(|error| {
            (
                error.status(),
                String::from_utf8(render(error, RequestId(7))).expect("a UTF-8 body"),
            )
        })
    }

    /// The not-found response for request 7, whole.
    const NOT_FOUND_BODY: &str = r#"{"type":"urn:gunmetal:problem:not_found","title":"We couldn't find that. It may have been removed, or you may not have access to it.","status":404,"request":"00000000000000000000000000000007"}"#;

    /// Verifies: SEC-API-011, SEC-HIS-010
    #[test]
    fn a_hidden_row_gets_exactly_the_response_of_a_missing_one_and_nothing_changes() {
        let data = data();
        let cache = cache(&data);
        give(&data, account(1), library(1));
        let sam = asked(&data, &member(1), Action::BrowseLibrary);
        let before = every(&cache);

        // "a2" is in library 2, which member 1 holds no grant for; "zz"
        // names nothing.
        let hidden = check(&sam, find(&cache, "a2"));
        let missing = check(&sam, find(&cache, "zz"));
        assert_eq!(hidden, Err(ApiError::new(ProblemCode::NotFound)));
        assert_eq!(missing, Err(ApiError::new(ProblemCode::NotFound)));
        assert_eq!(response(hidden), Some((404, NOT_FOUND_BODY.to_owned())));
        assert_eq!(response(missing), Some((404, NOT_FOUND_BODY.to_owned())));
        // A change to either is refused the same way.
        let manager = principal(
            PrincipalKind::Member,
            Role::Member.preset().with(Capability::LibraryManage),
            1,
        );
        let manage = asked(&data, &manager, Action::ManageLibrary);
        assert_eq!(
            response(check_edit(&manage, find(&cache, "a2"))),
            Some((404, NOT_FOUND_BODY.to_owned()))
        );
        assert_eq!(
            response(check_edit(&manage, find(&cache, "zz"))),
            Some((404, NOT_FOUND_BODY.to_owned()))
        );
        assert_eq!(every(&cache), before);
        assert_eq!(before.len(), 8);
    }

    /// Verifies: SEC-IAM-070
    #[test]
    fn every_fetch_list_batch_and_change_applies_the_caller_s_grants() {
        let data = data();
        let cache = cache(&data);
        give(&data, account(1), library(1));
        give(&data, account(2), library(2));
        let sam = asked(&data, &member(1), Action::BrowseLibrary);
        let kim = asked(&data, &member(2), Action::BrowseLibrary);
        let everyone = asked(&data, &owner(), Action::BrowseLibrary);

        // One object.
        assert_eq!(
            check(&sam, find(&cache, "a1")),
            Ok(Visible(item("a1", 1, "track")))
        );
        assert_eq!(
            check(&kim, find(&cache, "a1")),
            Err(ApiError::new(ProblemCode::NotFound))
        );
        assert_eq!(
            check(&everyone, find(&cache, "a2")),
            Ok(Visible(item("a2", 2, "track")))
        );
        // A list.
        assert_eq!(
            ids(&only_visible(&sam, every(&cache))),
            ["a1", "b1", "c1", "d1"]
        );
        assert_eq!(
            ids(&only_visible(&kim, every(&cache))),
            ["a2", "b2", "c2", "d2"]
        );
        assert_eq!(
            ids(&only_visible(&everyone, every(&cache))),
            ["a1", "a2", "b1", "b2", "c1", "c2", "d1", "d2"]
        );
        // A batch.
        let batch = |ids: [&str; 2]| ids.map(|id| find(&cache, id));
        assert_eq!(
            check_all(&sam, batch(["a1", "b1"])),
            Ok(vec![
                Visible(item("a1", 1, "track")),
                Visible(item("b1", 1, "playlist entry")),
            ])
        );
        assert_eq!(
            check_all(&kim, batch(["a1", "b1"])),
            Err(ApiError::new(ProblemCode::NotFound))
        );
        // A change.
        let manager = principal(
            PrincipalKind::Member,
            Role::Member.preset().with(Capability::LibraryManage),
            1,
        );
        let manage = asked(&data, &manager, Action::ManageLibrary);
        assert_eq!(
            check_edit(&manage, find(&cache, "c1")),
            Ok(Editable(item("c1", 1, "lyrics")))
        );
        assert_eq!(
            check_edit(&manage, find(&cache, "c2")),
            Err(ApiError::new(ProblemCode::NotFound))
        );
    }

    #[test]
    fn a_wrapper_lends_its_row_and_gives_it_up() {
        let sam = browse(1, &[1]);
        let seen = check(&sam, Some(item("a1", 1, "track"))).expect("visible");
        assert_eq!(seen.get(), &item("a1", 1, "track"));
        assert_eq!(seen.into_inner(), item("a1", 1, "track"));
        let manage = decided(&owner(), &[], Action::ManageLibrary, &ResourceFacts::Server);
        let held = check_edit(&manage, Some(item("d2", 2, "artwork"))).expect("editable");
        assert_eq!(held.get(), &item("d2", 2, "artwork"));
        assert_eq!(held.into_inner(), item("d2", 2, "artwork"));
    }

    #[test]
    fn only_a_permit_to_manage_the_library_makes_a_visible_row_editable() {
        let row = || Some(item("a1", 1, "track"));
        let refused = Err(ApiError::new(ProblemCode::NotFound));
        // A member who sees the row, asking to browse, stream or change
        // their own data.
        let own = ResourceFacts::Owned(Owner::Account(account(1)));
        let asking = [
            (Action::BrowseLibrary, ResourceFacts::Server),
            (Action::StreamMedia, ResourceFacts::Library(library(1))),
            (Action::WriteOwnData, own),
        ];
        for (action, resource) in asking {
            let seeing = decided(&member(1), &[1], action, &resource);
            assert_eq!(check(&seeing, row()), Ok(Visible(item("a1", 1, "track"))));
            assert_eq!(check_edit(&seeing, row()), refused);
        }
        // The owner's permit to manage someone is not one to manage a
        // library.
        let people = decided(
            &owner(),
            &[],
            Action::ManageUser,
            &person(1, PrincipalKind::Member),
        );
        assert_eq!(check_edit(&people, row()), refused);
        // A permit to manage libraries, held by someone who sees library 1
        // and someone who sees library 2.
        let manager = |libraries: &[u8]| {
            decided(
                &principal(
                    PrincipalKind::Member,
                    CapabilitySet::of(&[Capability::LibraryRead, Capability::LibraryManage]),
                    1,
                ),
                libraries,
                Action::ManageLibrary,
                &ResourceFacts::Server,
            )
        };
        assert_eq!(
            check_edit(&manager(&[1]), row()),
            Ok(Editable(item("a1", 1, "track")))
        );
        assert_eq!(check_edit(&manager(&[2]), row()), refused);
        assert_eq!(check_edit::<Item>(&manager(&[1]), None), refused);
    }

    /// Verifies: SEC-API-012
    #[test]
    fn a_batch_with_one_hidden_or_missing_row_is_rejected_whole() {
        let data = data();
        let cache = cache(&data);
        give(&data, account(1), library(1));
        let sam = asked(&data, &member(1), Action::BrowseLibrary);
        let batch =
            |ids: &[&str]| -> Vec<Option<Item>> { ids.iter().map(|id| find(&cache, id)).collect() };
        assert_eq!(
            check_all(&sam, batch(&["a1", "b1", "c1", "d1"])).map(|seen| seen.len()),
            Ok(4)
        );
        assert_eq!(check_all(&sam, batch(&[])), Ok(vec![]));
        for hostile in [
            ["a2", "b1", "c1", "d1"],
            ["a1", "b1", "c2", "d1"],
            ["a1", "b1", "c1", "d2"],
            ["zz", "b1", "c1", "d1"],
            ["a1", "b1", "c1", "zz"],
        ] {
            assert_eq!(
                check_all(&sam, batch(&hostile)),
                Err(ApiError::new(ProblemCode::NotFound)),
                "{hostile:?}"
            );
        }
    }

    /// Verifies: SEC-MED-051, SEC-API-014
    #[test]
    fn playlist_entries_lyrics_and_artwork_come_only_from_libraries_granted_at_that_moment() {
        let data = data();
        let cache = cache(&data);
        give(&data, account(1), library(1));
        give(&data, account(2), library(1));
        give(&data, account(2), library(2));
        let of_kind = |permit: &Permit, kind: &str| -> Vec<String> {
            only_visible(permit, every(&cache))
                .into_iter()
                .map(Visible::into_inner)
                .filter(|item| item.kind == kind)
                .map(|item| item.id)
                .collect()
        };
        let sam = asked(&data, &member(1), Action::BrowseLibrary);
        let kim = asked(&data, &member(2), Action::BrowseLibrary);
        for (kind, in_library_1, in_library_2) in [
            ("playlist entry", "b1", "b2"),
            ("lyrics", "c1", "c2"),
            ("artwork", "d1", "d2"),
        ] {
            assert_eq!(of_kind(&sam, kind), [in_library_1], "{kind}");
            assert_eq!(of_kind(&kim, kind), [in_library_1, in_library_2], "{kind}");
            assert_eq!(
                check(&sam, find(&cache, in_library_2)),
                Err(ApiError::new(ProblemCode::NotFound)),
                "{kind}"
            );
            assert_eq!(
                check(&kim, find(&cache, in_library_2)),
                Ok(Visible(item(in_library_2, 2, kind))),
                "{kind}"
            );
        }
        // Member 2 loses library 2. The permit of the request after that
        // shows none of it.
        let manage = decided(
            &owner(),
            &[],
            Action::ManageUser,
            &person(2, PrincipalKind::Member),
        );
        assert_eq!(revoke(&data.identity, &manage, library(2)), Ok(true));
        let kim = asked(&data, &member(2), Action::BrowseLibrary);
        for (kind, in_library_1, in_library_2) in [
            ("playlist entry", "b1", "b2"),
            ("lyrics", "c1", "c2"),
            ("artwork", "d1", "d2"),
        ] {
            assert_eq!(of_kind(&kim, kind), [in_library_1], "{kind}");
            assert_eq!(
                check(&kim, find(&cache, in_library_2)),
                Err(ApiError::new(ProblemCode::NotFound)),
                "{kind}"
            );
        }
    }

    /// Verifies: SEC-API-016
    #[test]
    fn a_pushed_event_reaches_only_the_recipients_whose_grants_hold_its_library() {
        let data = data();
        give(&data, account(1), library(1));
        give(&data, account(2), library(2));
        give(&data, account(3), library(1));
        give(&data, account(3), library(2));
        // Five connected recipients, each with the permit of its own
        // session. Member 4 holds no grant.
        let recipients = [
            ("sam", asked(&data, &member(1), Action::BrowseLibrary)),
            ("kim", asked(&data, &member(2), Action::BrowseLibrary)),
            ("lee", asked(&data, &member(3), Action::BrowseLibrary)),
            ("ray", asked(&data, &member(4), Action::BrowseLibrary)),
            ("the owner", asked(&data, &owner(), Action::BrowseLibrary)),
        ];
        let events = [
            item("now playing a1", 1, "playback"),
            item("playlist b2 changed", 2, "playlist"),
            item("lyrics c1 added", 1, "library"),
        ];
        // What each recipient's socket is handed, in order.
        let delivered: Vec<(&str, Vec<&str>)> = recipients
            .iter()
            .map(|(name, permit)| {
                let seen = events
                    .iter()
                    .filter_map(|event| visible_to(permit, event))
                    .map(|event| event.into_inner().id.as_str())
                    .collect();
                (*name, seen)
            })
            .collect();
        assert_eq!(
            delivered,
            [
                ("sam", vec!["now playing a1", "lyrics c1 added"]),
                ("kim", vec!["playlist b2 changed"]),
                (
                    "lee",
                    vec!["now playing a1", "playlist b2 changed", "lyrics c1 added"]
                ),
                ("ray", vec![]),
                (
                    "the owner",
                    vec!["now playing a1", "playlist b2 changed", "lyrics c1 added"]
                ),
            ]
        );
    }

    /// Libraries 1 to 4.
    fn a_library() -> impl Strategy<Value = u8> {
        1_u8..=4
    }

    /// Up to four of libraries 1 to 4, in any order, with repeats.
    fn some_libraries() -> impl Strategy<Value = Vec<u8>> {
        prop::collection::vec(a_library(), 0..=4)
    }

    proptest! {
        /// A batch of rows the caller may see, with one row from a library
        /// it was never granted put in at any position, is refused whole;
        /// without that row it is returned whole and in order.
        ///
        /// Verifies: SEC-API-012
        #[test]
        fn a_generated_batch_with_one_forbidden_row_is_rejected_whole(
            granted in prop::collection::vec(a_library(), 1..=3),
            picks in prop::collection::vec(any::<prop::sample::Index>(), 0..12),
            at in any::<prop::sample::Index>(),
            forbidden in 5_u8..=8,
        ) {
            let permit = browse(1, &granted);
            let allowed: Vec<Item> = picks
                .iter()
                .enumerate()
                .map(|(place, pick)| item(&format!("row {place}"), *pick.get(&granted), "track"))
                .collect();
            prop_assert_eq!(
                check_all(&permit, allowed.iter().cloned().map(Some)),
                Ok(allowed.iter().cloned().map(Visible).collect::<Vec<_>>())
            );
            let mut hostile = allowed;
            hostile.insert(at.index(hostile.len() + 1), item("forbidden", forbidden, "track"));
            prop_assert_eq!(
                check_all(&permit, hostile.into_iter().map(Some)),
                Err(ApiError::new(ProblemCode::NotFound))
            );
        }

        /// A list built for a restricted caller holds exactly the rows whose
        /// library the caller was granted, in the order given.
        ///
        /// Verifies: SEC-API-014, SEC-TM-026
        #[test]
        fn a_generated_list_holds_exactly_the_rows_of_granted_libraries(
            granted in some_libraries(),
            rows in prop::collection::vec(a_library(), 0..24),
        ) {
            let permit = browse(1, &granted);
            let stored: Vec<Item> = rows
                .iter()
                .enumerate()
                .map(|(place, n)| item(&format!("row {place}"), *n, "search hit"))
                .collect();
            // The oracle works on the library numbers alone.
            let expected: Vec<String> = rows
                .iter()
                .enumerate()
                .filter(|(_, n)| granted.contains(n))
                .map(|(place, _)| format!("row {place}"))
                .collect();
            let seen: Vec<String> = only_visible(&permit, stored.clone())
                .into_iter()
                .map(|item| item.into_inner().id)
                .collect();
            prop_assert_eq!(seen, expected);
            // Each row alone gets the answer the list gave it.
            for (row, n) in stored.into_iter().zip(&rows) {
                prop_assert_eq!(check(&permit, Some(row)).is_ok(), granted.contains(n));
            }
        }
    }
}
