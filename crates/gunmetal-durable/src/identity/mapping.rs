//! The public-ID mapping: the identifier each library item shows clients,
//! kept against its content identity (ADR 3, section 14; owner decision
//! D-04).
//!
//! An identifier is minted once, by the secrets crate's minting function
//! (WP-047), and kept here, so a cache rebuild, a rescan or a move reissues
//! none (INT-008). This module never makes one: [`IdentityStore::assign`]
//! takes a freshly minted [`PublicId`] and stores it only if the item has
//! none yet, and nothing here builds an identifier from a content identity,
//! a path or a name (SEC-HIS-012, SEC-API-023, SEC-PRV-021). A public ID
//! names one item: storing it for a second is refused.
//!
//! The mapping returns no user-visible object, only identifiers the
//! catalogue then reads under a `Permit`.

use gunmetal_core::id::{IdKind, PublicId};
use gunmetal_core::schema::{Column, DataClass, SchemaPart};
use gunmetal_core::time::Timestamp;
use gunmetal_fs::sqlite::{Query, Row, Value};

use crate::identity::error::IdentityError;
use crate::identity::store::IdentityStore;

/// The mapping's table. A content identity is library data, and so is the
/// identifier clients see for it and the time the server first saw it, the
/// album's "date added" (DIS-038).
pub(crate) const PUBLIC_IDS: SchemaPart = SchemaPart {
    name: "identity.public_ids",
    sql: "CREATE TABLE public_ids (\
              kind TEXT NOT NULL, \
              content BLOB NOT NULL, \
              public_id TEXT NOT NULL UNIQUE, \
              first_seen INTEGER NOT NULL, \
              PRIMARY KEY (kind, content)) WITHOUT ROWID;",
    columns: &[
        Column {
            table: "public_ids",
            name: "kind",
            class: DataClass::Library,
        },
        Column {
            table: "public_ids",
            name: "content",
            class: DataClass::Library,
        },
        Column {
            table: "public_ids",
            name: "public_id",
            class: DataClass::Library,
        },
        Column {
            table: "public_ids",
            name: "first_seen",
            class: DataClass::Library,
        },
    ],
};

/// Stores a mapping unless the item already has one.
const ASSIGN: Query = Query::new(
    "INSERT INTO public_ids (kind, content, public_id, first_seen) \
     VALUES (?1, ?2, ?3, ?4) ON CONFLICT (kind, content) DO NOTHING",
);

/// Reads an item's mapping.
const LOOKUP: Query =
    Query::new("SELECT public_id, first_seen FROM public_ids WHERE kind = ?1 AND content = ?2");

/// An item's public ID and when the server first saw it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mapping {
    /// The identifier clients see.
    pub id: PublicId,
    /// When the server first saw the item.
    pub first_seen: Timestamp,
}

impl IdentityStore {
    /// Keeps `minted` as the public ID of the item of kind `kind` whose
    /// content identity is `content`, seen first at `now`, unless the item
    /// already has one, and returns the item's mapping either way. The
    /// first identifier kept is the one the item keeps.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::WrongKind`] when `minted` is of another
    /// kind, and [`IdentityError::Db`] when SQLite fails, including when
    /// `minted` already names another item.
    pub fn assign(
        &self,
        kind: IdKind,
        content: &[u8],
        minted: PublicId,
        now: Timestamp,
    ) -> Result<Mapping, IdentityError> {
        let text = minted.to_string();
        PublicId::parse(&text, kind).map_err(IdentityError::WrongKind)?;
        self.write(&[ASSIGN
            .bind(kind_value(kind))
            .bind(Value::Blob(content.to_vec()))
            .bind(Value::Text(text))
            .bind(Value::Integer(now.millis()))])
            .and_then(|_| self.public_id(kind, content))
            .and_then(|found| found.ok_or(IdentityError::Mapping { found: Vec::new() }))
    }

    /// The mapping of the item of kind `kind` whose content identity is
    /// `content`, if it has one.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::Mapping`] when the stored mapping cannot be
    /// read, and [`IdentityError::Db`] when SQLite fails.
    pub fn public_id(
        &self,
        kind: IdKind,
        content: &[u8],
    ) -> Result<Option<Mapping>, IdentityError> {
        self.read(
            &LOOKUP
                .bind(kind_value(kind))
                .bind(Value::Blob(content.to_vec())),
        )
        .and_then(|found| decode(kind, found))
    }
}

/// The name a kind is stored under.
fn kind_value(kind: IdKind) -> Value {
    Value::Text(
        match kind {
            IdKind::Track => "track",
            IdKind::Album => "album",
            IdKind::Artist => "artist",
            IdKind::ReleaseGroup => "release-group",
            IdKind::Playlist => "playlist",
            IdKind::User => "user",
            IdKind::Profile => "profile",
            IdKind::Device => "device",
            IdKind::Library => "library",
            IdKind::Invite => "invite",
            IdKind::Share => "share",
            IdKind::Token => "token",
        }
        .to_owned(),
    )
}

/// Reads what [`LOOKUP`] returned: no rows, or one row of a public ID of
/// kind `kind` and a timestamp.
fn decode(kind: IdKind, found: Vec<Row>) -> Result<Option<Mapping>, IdentityError> {
    let mapping = match found.as_slice() {
        [] => return Ok(None),
        [Row(values)] => match values.as_slice() {
            [Value::Text(id), Value::Integer(millis)] => PublicId::parse(id, kind)
                .ok()
                .zip(Timestamp::from_millis(*millis).ok())
                .map(|(id, first_seen)| Mapping { id, first_seen }),
            _ => None,
        },
        _ => None,
    };
    mapping.map(Some).ok_or(IdentityError::Mapping { found })
}

/// What code outside this crate must not be able to do with the mapping.
#[cfg(doctest)]
mod compile_fail {
    /// Control: a mapping keeps an identifier the caller minted.
    ///
    /// ```
    /// use gunmetal_core::id::{IdKind, PublicId};
    /// use gunmetal_core::time::Timestamp;
    /// use gunmetal_durable::identity::error::IdentityError;
    /// use gunmetal_durable::identity::mapping::Mapping;
    /// use gunmetal_durable::identity::store::IdentityStore;
    ///
    /// fn keep(store: &IdentityStore, minted: PublicId) -> Result<Mapping, IdentityError> {
    ///     store.assign(IdKind::Track, b"content identity", minted, Timestamp::MIN)
    /// }
    /// ```
    struct Control;

    /// Verifies: SEC-HIS-012, SEC-PRV-021
    ///
    /// No call stores a mapping without a minted identifier, so the store
    /// cannot derive one from the content identity, a path or a name.
    ///
    /// ```compile_fail,E0061
    /// use gunmetal_core::id::{IdKind, PublicId};
    /// use gunmetal_core::time::Timestamp;
    /// use gunmetal_durable::identity::error::IdentityError;
    /// use gunmetal_durable::identity::mapping::Mapping;
    /// use gunmetal_durable::identity::store::IdentityStore;
    ///
    /// fn derive(store: &IdentityStore) -> Result<Mapping, IdentityError> {
    ///     store.assign(IdKind::Track, b"content identity", Timestamp::MIN)
    /// }
    /// ```
    struct NoDerivedIdentifier;
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRACK: &str = "trk_0123456789abcdefghjkmnpqrs";

    fn track() -> PublicId {
        PublicId::parse(TRACK, IdKind::Track).unwrap()
    }

    fn row(values: Vec<Value>) -> Row {
        Row(values)
    }

    #[test]
    fn stores_each_kind_under_its_own_name() {
        let kinds = [
            (IdKind::Track, "track"),
            (IdKind::Album, "album"),
            (IdKind::Artist, "artist"),
            (IdKind::ReleaseGroup, "release-group"),
            (IdKind::Playlist, "playlist"),
            (IdKind::User, "user"),
            (IdKind::Profile, "profile"),
            (IdKind::Device, "device"),
            (IdKind::Library, "library"),
            (IdKind::Invite, "invite"),
            (IdKind::Share, "share"),
            (IdKind::Token, "token"),
        ];
        for (kind, name) in kinds {
            assert_eq!(kind_value(kind), Value::Text(name.to_owned()));
        }
    }

    #[test]
    fn decodes_no_row_as_no_mapping_and_one_row_as_its_mapping() {
        assert_eq!(decode(IdKind::Track, vec![]), Ok(None));
        assert_eq!(
            decode(
                IdKind::Track,
                vec![row(vec![Value::Text(TRACK.to_owned()), Value::Integer(-5)])]
            ),
            Ok(Some(Mapping {
                id: track(),
                first_seen: Timestamp::from_millis(-5).unwrap(),
            }))
        );
    }

    #[test]
    fn refuses_a_stored_mapping_it_cannot_read() {
        let good = || row(vec![Value::Text(TRACK.to_owned()), Value::Integer(0)]);
        for found in [
            vec![good(), good()],
            vec![row(vec![Value::Text(TRACK.to_owned())])],
            vec![row(vec![Value::Integer(0), Value::Integer(0)])],
            vec![row(vec![
                Value::Text(TRACK.to_owned()),
                Value::Text("0".to_owned()),
            ])],
            vec![row(vec![Value::Text("trk_".to_owned()), Value::Integer(0)])],
            vec![row(vec![
                Value::Text(TRACK.to_owned()),
                Value::Integer(i64::MAX),
            ])],
        ] {
            assert_eq!(
                decode(IdKind::Track, found.clone()),
                Err(IdentityError::Mapping { found })
            );
        }
        // A stored identifier of another kind is not this kind's.
        let found = vec![good()];
        assert_eq!(
            decode(IdKind::Album, found.clone()),
            Err(IdentityError::Mapping { found })
        );
    }
}
