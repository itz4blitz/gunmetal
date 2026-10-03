//! Checks on a JSON body before any handler decodes it (SEC-API-060,
//! SEC-API-067, SEC-API-013, SEC-HIS-009).
//!
//! The pipeline reads the body once, as a stream of JSON tokens, and refuses
//! it when it nests deeper than [`JsonLimits::DEPTH`], holds an array longer
//! than the route allows, gives one key twice in an object, or has a key
//! that names a user, profile, owner, account or household on a route that
//! may not name principals: the acting principal comes only from the
//! credential. Nothing is built from the body here, so the checks cost no
//! more memory than the deepest path through it. The handler then decodes
//! the same bytes into its typed request, which refuses unknown fields
//! ([`crate::call::Call::json`]).

use core::cell::Cell;
use core::fmt;
use std::collections::BTreeSet;

use serde::de::{DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};

use crate::route::JsonLimits;

/// Key names that name a principal, compared without case, `_` or `-`.
const IDENTITY_KEYS: [&str; 13] = [
    "account",
    "accountid",
    "household",
    "householdid",
    "owner",
    "ownerid",
    "principal",
    "principalid",
    "profile",
    "profileid",
    "uid",
    "user",
    "userid",
];

/// Why a body was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyError {
    /// Not JSON.
    Syntax,
    /// Nested too deeply.
    TooDeep,
    /// An array with too many elements.
    TooLong,
    /// A key that names a principal.
    NamesPrincipal,
    /// The same key twice in one object.
    Duplicate,
}

/// Checks a JSON body and returns the keys of its top-level object, in
/// sorted order, which the pipeline compares with the query and path
/// parameters.
///
/// # Errors
///
/// The first rule the body breaks.
pub fn check(
    bytes: &[u8],
    limits: JsonLimits,
    names_principals: bool,
) -> Result<Vec<String>, BodyError> {
    let rules = Rules {
        items: limits.items,
        names_principals,
        broken: Cell::new(None),
    };
    let mut keys = BTreeSet::new();
    let node = Node {
        rules: &rules,
        room: JsonLimits::DEPTH,
        keys: Some(&mut keys),
    };
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    node.deserialize(&mut deserializer)
        .and_then(|()| deserializer.end())
        .map_err(|_| rules.broken.get().unwrap_or(BodyError::Syntax))?;
    Ok(keys.into_iter().collect())
}

/// The rules one body is checked against, and the first one it broke.
struct Rules {
    items: usize,
    names_principals: bool,
    broken: Cell<Option<BodyError>>,
}

impl Rules {
    /// Records a broken rule and returns the error that stops the parse.
    fn refuse<E: serde::de::Error>(&self, error: BodyError) -> E {
        self.broken.set(Some(error));
        E::custom("refused")
    }
}

/// One value in the body. `room` is how many more levels of nesting may
/// open; `keys` collects the keys of the top-level object.
struct Node<'r, 'k> {
    rules: &'r Rules,
    room: usize,
    keys: Option<&'k mut BTreeSet<String>>,
}

impl<'de> DeserializeSeed<'de> for Node<'_, '_> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Node<'_, '_> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON value")
    }

    fn visit_bool<E>(self, _: bool) -> Result<(), E> {
        Ok(())
    }

    fn visit_i64<E>(self, _: i64) -> Result<(), E> {
        Ok(())
    }

    fn visit_u64<E>(self, _: u64) -> Result<(), E> {
        Ok(())
    }

    fn visit_f64<E>(self, _: f64) -> Result<(), E> {
        Ok(())
    }

    fn visit_str<E>(self, _: &str) -> Result<(), E> {
        Ok(())
    }

    fn visit_unit<E>(self) -> Result<(), E> {
        Ok(())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        let room = self.inner::<A::Error>()?;
        let mut count = 0_usize;
        while seq.next_element_seed(room.child())?.is_some() {
            count += 1;
            if count > self.rules.items {
                return Err(self.rules.refuse(BodyError::TooLong));
            }
        }
        Ok(())
    }

    fn visit_map<A: MapAccess<'de>>(mut self, mut map: A) -> Result<(), A::Error> {
        let room = self.inner::<A::Error>()?;
        let mut seen = BTreeSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !self.rules.names_principals && identity_key(&key) {
                return Err(self.rules.refuse(BodyError::NamesPrincipal));
            }
            if seen.contains(&key) {
                return Err(self.rules.refuse(BodyError::Duplicate));
            }
            map.next_value_seed(room.child())?;
            seen.insert(key);
        }
        if let Some(keys) = self.keys.take() {
            *keys = seen;
        }
        Ok(())
    }
}

impl<'r> Node<'r, '_> {
    /// The room left inside this array or object, or the depth error when
    /// there is none.
    fn inner<E: serde::de::Error>(&self) -> Result<Room<'r>, E> {
        self.room
            .checked_sub(1)
            .map(|room| Room {
                rules: self.rules,
                room,
            })
            .ok_or_else(|| self.rules.refuse(BodyError::TooDeep))
    }
}

/// The room inside one array or object, from which its children are made.
#[derive(Clone, Copy)]
struct Room<'r> {
    rules: &'r Rules,
    room: usize,
}

impl<'r> Room<'r> {
    fn child(self) -> Node<'r, 'static> {
        Node {
            rules: self.rules,
            room: self.room,
            keys: None,
        }
    }
}

/// Whether a request's query parameters are well placed: no name twice,
/// no name that a path parameter or a top-level body key also has, and, on
/// a route that may not name principals, no name that names one
/// (SEC-API-067, SEC-API-013).
#[must_use]
pub fn placed(
    query: &[(String, String)],
    path: &[(&'static str, String)],
    body: &[String],
    names_principals: bool,
) -> bool {
    let mut seen = BTreeSet::new();
    query.iter().all(|(name, _)| {
        seen.insert(name.as_str())
            && path.iter().all(|(param, _)| param != name)
            && !body.contains(name)
            && (names_principals || !identity_key(name))
    })
}

/// Whether a key names a principal.
#[must_use]
pub fn identity_key(key: &str) -> bool {
    let folded: String = key
        .chars()
        .filter(|c| *c != '_' && *c != '-')
        .map(|c| c.to_ascii_lowercase())
        .collect();
    IDENTITY_KEYS.contains(&folded.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMITS: JsonLimits = JsonLimits {
        bytes: 1024,
        items: 3,
    };

    fn nested(levels: usize) -> String {
        format!("{}{}", "[".repeat(levels), "]".repeat(levels))
    }

    fn keys(names: &[&str]) -> Result<Vec<String>, BodyError> {
        names.iter().map(|name| Ok((*name).to_owned())).collect()
    }

    fn checked(bodies: &[&str], names_principals: bool) -> Vec<Result<Vec<String>, BodyError>> {
        bodies
            .iter()
            .map(|body| check(body.as_bytes(), LIMITS, names_principals))
            .collect()
    }

    #[test]
    fn returns_the_top_level_keys_in_order() {
        assert_eq!(
            checked(
                &[
                    r#"{"name":"Mix","tracks":[1,2,3],"meta":{"a":[],"z":1}}"#,
                    "[1,2]",
                    "7",
                    "{}",
                    r#"[{"a":1}]"#,
                ],
                false
            ),
            [
                keys(&["meta", "name", "tracks"]),
                keys(&[]),
                keys(&[]),
                keys(&[]),
                keys(&[]),
            ]
        );
    }

    #[test]
    fn accepts_every_kind_of_value() {
        assert_eq!(
            check(
                br#"{"t":true,"f":false,"n":null,"i":-1,"u":18446744073709551615,"x":1.5,"s":"a\"b","e":"","o":{},"l":[]}"#,
                LIMITS,
                false
            ),
            keys(&["e", "f", "i", "l", "n", "o", "s", "t", "u", "x"])
        );
    }

    #[test]
    fn describes_what_it_expects() {
        let rules = Rules {
            items: 0,
            names_principals: false,
            broken: Cell::new(None),
        };
        let node = Node {
            rules: &rules,
            room: 0,
            keys: None,
        };
        assert_eq!(
            format!("{}", &node as &dyn serde::de::Expected),
            "a JSON value"
        );
    }

    #[test]
    fn refuses_what_is_not_json() {
        assert_eq!(
            checked(
                &[
                    "",
                    "{",
                    "{\"a\":1,}",
                    "{} {}",
                    "\u{7f}",
                    "[1,]",
                    "{\"a\" 1}",
                    "1e999"
                ],
                false
            ),
            vec![Err(BodyError::Syntax); 8]
        );
        assert_eq!(check(b"\xff", LIMITS, false), Err(BodyError::Syntax));
    }

    /// Verifies: SEC-API-060
    #[test]
    fn allows_thirty_two_levels_and_no_more() {
        let objects =
            |levels: usize| format!("{}1{}", r#"{"a":"#.repeat(levels), "}".repeat(levels));
        assert_eq!(
            [
                check(nested(32).as_bytes(), LIMITS, false),
                check(nested(33).as_bytes(), LIMITS, false),
                check(objects(32).as_bytes(), LIMITS, false),
                check(objects(33).as_bytes(), LIMITS, false),
                check(nested(200).as_bytes(), LIMITS, false),
            ],
            [
                keys(&[]),
                Err(BodyError::TooDeep),
                keys(&["a"]),
                Err(BodyError::TooDeep),
                Err(BodyError::TooDeep),
            ]
        );
    }

    /// Verifies: SEC-API-060
    #[test]
    fn caps_every_array() {
        assert_eq!(
            checked(
                &[
                    "[1,2,3]",
                    "[1,2,3,4]",
                    r#"{"a":[[1],[1,2,3,4]]}"#,
                    "[[],[],[]]"
                ],
                false
            ),
            [
                keys(&[]),
                Err(BodyError::TooLong),
                Err(BodyError::TooLong),
                keys(&[]),
            ]
        );
    }

    /// Verifies: SEC-API-013, SEC-HIS-009
    #[test]
    fn refuses_keys_that_name_a_principal() {
        let bodies = [
            r#"{"userId":"usr_x"}"#,
            r#"{"user_id":"usr_x"}"#,
            r#"{"OWNER":"usr_x"}"#,
            r#"{"profile-id":"prf_x"}"#,
            r#"{"uid":1}"#,
            r#"{"items":[{"name":"a","account":"x"}]}"#,
            r#"{"a":{"b":{"household":"h"}}}"#,
        ];
        assert_eq!(
            checked(&bodies, false),
            vec![Err(BodyError::NamesPrincipal); 7]
        );
        assert_eq!(
            checked(&bodies, true),
            [
                keys(&["userId"]),
                keys(&["user_id"]),
                keys(&["OWNER"]),
                keys(&["profile-id"]),
                keys(&["uid"]),
                keys(&["items"]),
                keys(&["a"]),
            ]
        );
        assert_eq!(
            check(br#"{"profile_name":"Kids","users_seen":2}"#, LIMITS, false),
            keys(&["profile_name", "users_seen"])
        );
    }

    /// Verifies: SEC-API-067
    #[test]
    fn refuses_duplicate_keys_at_any_depth() {
        assert_eq!(
            checked(
                &[
                    r#"{"a":1,"a":2}"#,
                    r#"{"m":{"k":1,"k":1}}"#,
                    r#"[{"x":1,"y":2,"x":1}]"#,
                    r#"{"a":1,"a":2}"#,
                ],
                false
            ),
            vec![Err(BodyError::Duplicate); 4]
        );
        assert_eq!(
            check(br#"{"a":{"k":1},"b":{"k":1}}"#, LIMITS, false),
            keys(&["a", "b"])
        );
    }

    fn pairs(names: &[&str]) -> Vec<(String, String)> {
        names
            .iter()
            .map(|name| ((*name).to_owned(), "v".to_owned()))
            .collect()
    }

    /// Verifies: SEC-API-067, SEC-API-013
    #[test]
    fn query_parameters_appear_once_and_in_one_place() {
        let path = [("id", "pls_1".to_owned())];
        let body = ["name".to_owned(), "tags".to_owned()];
        let cases: [(&[&str], bool, bool); 10] = [
            (&[], false, true),
            (&["limit", "cursor"], false, true),
            (&["limit", "limit"], false, false),
            (&["limit", "cursor", "limit"], false, false),
            (&["id"], false, false),
            (&["limit", "tags"], false, false),
            (&["Name", "ID"], false, true),
            (&["user_id"], false, false),
            (&["limit", "Owner"], false, false),
            (&["limit", "Owner"], true, true),
        ];
        let answers: Vec<bool> = cases
            .iter()
            .map(|(names, names_principals, _)| {
                placed(&pairs(names), &path, &body, *names_principals)
            })
            .collect();
        let expected: Vec<bool> = cases.iter().map(|(.., expected)| *expected).collect();
        assert_eq!(answers, expected);
    }
}
