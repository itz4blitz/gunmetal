//! Schema parts, their data classes and the schema digest.
//!
//! Every SQLite store builds its schema from named parts, one per module
//! that owns tables, so many modules can add tables without numbering
//! migrations. Each part declares the data class of every column it
//! creates (SEC-TM-050, SEC-PRV-001), and the stores compare those
//! declarations with the columns SQLite reports when they open a database.
//!
//! The digest is SHA-256 over the parts in name order, with every field
//! length-prefixed, so any change to any part, its SQL or a column's class
//! changes it. The cache rebuilds when its digest changes; a development
//! identity store refuses to start.

use crate::crypto::sha256;
use std::collections::BTreeMap;

/// The privacy baseline's data classes (SEC-PRV-001). The class decides
/// who can read a value, whether it may leave the server, whether it may be
/// logged and how long it is kept.
///
/// The classes have no order: each has its own rules, so code matches on
/// the class rather than comparing sensitivities. SEC-TM-050 names a
/// different vocabulary; these are the privacy baseline's names, which the
/// build plan adopts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataClass {
    /// Anyone who can reach the server may read it, such as the server's
    /// display name.
    Public,
    /// What the library holds: titles, artists, artwork, technical
    /// attributes, file paths and provider IDs.
    Library,
    /// What people do: plays, resume points, ratings, playlists, the queue
    /// and what is playing now.
    Activity,
    /// Who people are: names, devices, sessions, invitations and security
    /// events.
    Identity,
    /// Tokens, keys and anything else nobody may read through the product.
    Secret,
}

impl DataClass {
    /// Every data class, in the order the privacy baseline lists them.
    pub const ALL: [Self; 5] = [
        Self::Public,
        Self::Library,
        Self::Activity,
        Self::Identity,
        Self::Secret,
    ];

    /// The class's stable lowercase name, as the schema digest frames it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Library => "library",
            Self::Activity => "activity",
            Self::Identity => "identity",
            Self::Secret => "secret",
        }
    }
}

/// A column a schema part creates, with its data class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Column {
    /// The table the column belongs to.
    pub table: &'static str,
    /// The column's name.
    pub name: &'static str,
    /// The column's data class.
    pub class: DataClass,
}

/// A named piece of a store's schema, owned by one module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemaPart {
    /// The part's name: printable ASCII with no spaces, unique in its store.
    pub name: &'static str,
    /// The statements that create the part's tables and indexes.
    pub sql: &'static str,
    /// Every column the statements create, each with its data class.
    pub columns: &'static [Column],
}

/// Why a set of schema parts was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaError {
    /// A part's name was empty or held a byte that is not printable ASCII.
    InvalidName {
        /// The name as written.
        name: &'static str,
    },
    /// Two parts had the same name.
    DuplicateName {
        /// The name both parts used.
        name: &'static str,
    },
    /// A column was classified twice, so it has no single class.
    DuplicateColumn {
        /// The column's table.
        table: &'static str,
        /// The column's name.
        column: &'static str,
        /// The parts that declared it, in name order; the same part twice
        /// when one part declared it twice.
        parts: [&'static str; 2],
    },
}

/// A column as the live database reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveColumn {
    /// The table the column belongs to.
    pub table: String,
    /// The column's name.
    pub name: String,
}

/// Why the live database and the declared classes disagree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClassError {
    /// The database has a column no part classifies.
    Unclassified {
        /// The column's table.
        table: String,
        /// The column's name.
        column: String,
    },
    /// A part classifies a column the database does not have.
    Missing {
        /// The part that declared the column.
        part: &'static str,
        /// The column's table.
        table: &'static str,
        /// The column's name.
        column: &'static str,
    },
}

/// A validated set of schema parts, held in name order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema {
    parts: Vec<SchemaPart>,
}

impl Schema {
    /// Validates `parts`, given in any order.
    ///
    /// # Errors
    ///
    /// Returns a [`SchemaError`] when a part's name is empty or not
    /// printable ASCII, when two parts share a name, or when a column is
    /// classified more than once.
    pub fn new(parts: &[SchemaPart]) -> Result<Self, SchemaError> {
        let mut parts = parts.to_vec();
        parts.sort_unstable_by_key(|part| part.name);
        check_sorted_parts(&parts)?;
        Ok(Self { parts })
    }

    /// The parts in name order, the order a store runs their SQL in.
    #[must_use]
    pub fn parts(&self) -> &[SchemaPart] {
        &self.parts
    }

    /// The schema digest: SHA-256 over the parts in name order.
    ///
    /// The input is a fixed prefix, then for each part its name, its SQL,
    /// its column count, and each column's table, name and class name in
    /// declaration order. Each text is preceded by its length in octets,
    /// and every length and count is a 64-bit big-endian integer, so no
    /// two different schemas frame to the same bytes.
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        let mut frame = DIGEST_PREFIX.to_vec();
        for part in &self.parts {
            push_text(&mut frame, part.name);
            push_text(&mut frame, part.sql);
            push_len(&mut frame, part.columns.len());
            for column in part.columns {
                push_text(&mut frame, column.table);
                push_text(&mut frame, column.name);
                push_text(&mut frame, column.class.name());
            }
        }
        sha256(&frame)
    }

    /// The data class declared for `column` in `table`, if any part
    /// declares one.
    #[must_use]
    pub fn class_of(&self, table: &str, column: &str) -> Option<DataClass> {
        self.parts
            .iter()
            .flat_map(|part| part.columns)
            .find(|declared| declared.table == table && declared.name == column)
            .map(|declared| declared.class)
    }

    /// Compares the columns the live database reports with the declared
    /// classes.
    ///
    /// # Errors
    ///
    /// Returns [`ClassError::Unclassified`] for the first live column, in
    /// the order given, that no part classifies, and otherwise
    /// [`ClassError::Missing`] for the first declared column, in part name
    /// order, that the database does not have.
    pub fn check_classes(&self, live: &[LiveColumn]) -> Result<(), ClassError> {
        if let Some(column) = live
            .iter()
            .find(|column| self.class_of(&column.table, &column.name).is_none())
        {
            return Err(ClassError::Unclassified {
                table: column.table.clone(),
                column: column.name.clone(),
            });
        }
        for part in &self.parts {
            for declared in part.columns {
                if !live
                    .iter()
                    .any(|column| column.table == declared.table && column.name == declared.name)
                {
                    return Err(ClassError::Missing {
                        part: part.name,
                        table: declared.table,
                        column: declared.name,
                    });
                }
            }
        }
        Ok(())
    }
}

/// The start of every digest's input, which names the framing so that a
/// later change to it changes every digest.
const DIGEST_PREFIX: &[u8] = b"gunmetal schema digest v1";

/// Checks parts already sorted by name: each name valid and unique, and
/// each column classified once. Kept apart from [`Schema::new`] because
/// the mutation tool does not mutate constructors.
fn check_sorted_parts(parts: &[SchemaPart]) -> Result<(), SchemaError> {
    let mut previous = None;
    let mut classified = BTreeMap::new();
    for part in parts {
        if !is_valid_name(part.name) {
            return Err(SchemaError::InvalidName { name: part.name });
        }
        if previous == Some(part.name) {
            return Err(SchemaError::DuplicateName { name: part.name });
        }
        previous = Some(part.name);
        for column in part.columns {
            if let Some(first) = classified.insert((column.table, column.name), part.name) {
                return Err(SchemaError::DuplicateColumn {
                    table: column.table,
                    column: column.name,
                    parts: [first, part.name],
                });
            }
        }
    }
    Ok(())
}

/// Whether `name` is non-empty printable ASCII with no spaces.
fn is_valid_name(name: &str) -> bool {
    !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_graphic())
}

/// Appends `len` as a 64-bit big-endian integer, the same width on every
/// target.
fn push_len(frame: &mut Vec<u8>, len: usize) {
    // A usize is at most 64 bits on every target Rust supports, so the
    // conversion cannot fail and the fallback is never used.
    frame.extend_from_slice(&u64::try_from(len).unwrap_or(u64::MAX).to_be_bytes());
}

/// Appends `text` preceded by its length in octets.
fn push_text(frame: &mut Vec<u8>, text: &str) {
    push_len(frame, text.len());
    frame.extend_from_slice(text.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use proptest::sample::subsequence;
    use std::fmt::Write as _;

    /// Writes a digest as lowercase hexadecimal.
    fn hex(digest: [u8; 32]) -> String {
        digest.iter().fold(String::new(), |mut out, byte| {
            write!(out, "{byte:02x}").unwrap();
            out
        })
    }

    /// A part with no columns.
    const fn bare(name: &'static str, sql: &'static str) -> SchemaPart {
        SchemaPart {
            name,
            sql,
            columns: &[],
        }
    }

    /// A column, written compactly.
    const fn column(table: &'static str, name: &'static str, class: DataClass) -> Column {
        Column { table, name, class }
    }

    /// A live column, as a store would read it from SQLite.
    fn live(table: &str, name: &str) -> LiveColumn {
        LiveColumn {
            table: table.to_owned(),
            name: name.to_owned(),
        }
    }

    const ACCOUNTS: SchemaPart = SchemaPart {
        name: "accounts",
        sql: "CREATE TABLE person (name, plays, key);",
        columns: &[
            column("person", "name", DataClass::Identity),
            column("person", "plays", DataClass::Activity),
            column("person", "key", DataClass::Secret),
        ],
    };

    const CATALOGUE: SchemaPart = SchemaPart {
        name: "catalogue",
        sql: "CREATE TABLE track (title);\nCREATE TABLE server (name);",
        columns: &[
            column("track", "title", DataClass::Library),
            column("server", "name", DataClass::Public),
        ],
    };

    /// Every column [`ACCOUNTS`] and [`CATALOGUE`] create, as SQLite would
    /// report them, in an order unlike the declarations.
    fn live_columns() -> Vec<LiveColumn> {
        vec![
            live("server", "name"),
            live("person", "key"),
            live("track", "title"),
            live("person", "name"),
            live("person", "plays"),
        ]
    }

    /// The digest's input for [`ACCOUNTS`] and [`CATALOGUE`], written out
    /// by hand: a fixed prefix, then each part in name order as its name,
    /// its SQL, its column count, and each column's table, name and class.
    /// Every text field is preceded by its length in octets and every count
    /// is written as a 64-bit big-endian integer.
    const FRAME: &[u8] = b"gunmetal schema digest v1\
        \0\0\0\0\0\0\0\x08accounts\
        \0\0\0\0\0\0\0\x27CREATE TABLE person (name, plays, key);\
        \0\0\0\0\0\0\0\x03\
        \0\0\0\0\0\0\0\x06person\0\0\0\0\0\0\0\x04name\0\0\0\0\0\0\0\x08identity\
        \0\0\0\0\0\0\0\x06person\0\0\0\0\0\0\0\x05plays\0\0\0\0\0\0\0\x08activity\
        \0\0\0\0\0\0\0\x06person\0\0\0\0\0\0\0\x03key\0\0\0\0\0\0\0\x06secret\
        \0\0\0\0\0\0\0\x09catalogue\
        \0\0\0\0\0\0\0\x37CREATE TABLE track (title);\nCREATE TABLE server (name);\
        \0\0\0\0\0\0\0\x02\
        \0\0\0\0\0\0\0\x05track\0\0\0\0\0\0\0\x05title\0\0\0\0\0\0\0\x07library\
        \0\0\0\0\0\0\0\x06server\0\0\0\0\0\0\0\x04name\0\0\0\0\0\0\0\x06public";

    /// SHA-256 of [`FRAME`], computed with Python's `hashlib`, which shares
    /// no code with the crate this module uses.
    const FRAME_DIGEST: &str = "b29183d5fa2b946daea1e711e8e553914652bb49a8d1346d92270ab793568b1d";

    #[test]
    fn digests_the_parts_in_name_order_with_every_field_length_prefixed() {
        assert_eq!(hex(sha256(FRAME)), FRAME_DIGEST);
        let schema = Schema::new(&[CATALOGUE, ACCOUNTS]).unwrap();
        assert_eq!(hex(schema.digest()), FRAME_DIGEST);
    }

    #[test]
    fn digests_an_empty_schema_as_the_prefix_alone() {
        // SHA-256 of "gunmetal schema digest v1", computed with hashlib.
        assert_eq!(
            hex(Schema::new(&[]).unwrap().digest()),
            "1a8772e43f1b3824c4cc55a59c7d40d3586711d23a8bc16b46e75cccfb36ff17"
        );
    }

    #[test]
    fn holds_the_parts_in_name_order() {
        let late = bare("zeta", "CREATE TABLE z (z);");
        assert_eq!(
            Schema::new(&[late, CATALOGUE, ACCOUNTS]).unwrap().parts(),
            [ACCOUNTS, CATALOGUE, late]
        );
    }

    #[test]
    fn changes_the_digest_when_one_character_moves_between_two_parts() {
        let before = Schema::new(&[
            bare("a", "CREATE TABLE a (x);"),
            bare("b", "CREATE TABLE b (y);"),
        ])
        .unwrap();
        let after = Schema::new(&[
            bare("a", "CREATE TABLE a (x)"),
            bare("b", ";CREATE TABLE b (y);"),
        ])
        .unwrap();
        assert_ne!(before.digest(), after.digest());
    }

    /// The classification is part of the schema itself: reclassifying a
    /// column is a schema change, so the cache built under the old class is
    /// rebuilt.
    ///
    /// Verifies: SEC-TM-050
    #[test]
    fn changes_the_digest_when_a_column_changes_class() {
        let digest = |class| {
            let columns = Box::leak(Box::new([column("t", "c", class)]));
            Schema::new(&[SchemaPart {
                name: "part",
                sql: "CREATE TABLE t (c);",
                columns,
            }])
            .unwrap()
            .digest()
        };
        for from in DataClass::ALL {
            for to in DataClass::ALL {
                assert_eq!(digest(from) == digest(to), from == to, "{from:?} to {to:?}");
            }
        }
    }

    /// The five classes are the privacy baseline's, each with a stable name.
    ///
    /// Verifies: SEC-PRV-001
    #[test]
    fn names_the_five_classes_of_the_privacy_baseline() {
        assert_eq!(
            DataClass::ALL.map(DataClass::name),
            ["public", "library", "activity", "identity", "secret"]
        );
    }

    /// Where each class stands in [`DataClass::ALL`]. The match is
    /// exhaustive, so a class added to the enum does not compile until it
    /// has a place here, and the test below fails until `ALL` holds it
    /// there.
    const fn place(class: DataClass) -> usize {
        match class {
            DataClass::Public => 0,
            DataClass::Library => 1,
            DataClass::Activity => 2,
            DataClass::Identity => 3,
            DataClass::Secret => 4,
        }
    }

    #[test]
    fn all_holds_every_class_once_in_the_order_of_the_privacy_baseline() {
        assert_eq!(DataClass::ALL.map(place), [0, 1, 2, 3, 4]);
    }

    #[test]
    fn refuses_two_parts_with_the_same_name() {
        assert_eq!(
            Schema::new(&[
                bare("catalogue", "CREATE TABLE a (x);"),
                ACCOUNTS,
                bare("catalogue", "CREATE TABLE b (y);"),
            ]),
            Err(SchemaError::DuplicateName { name: "catalogue" })
        );
    }

    #[test]
    fn refuses_names_that_are_empty_or_not_printable_ascii() {
        for name in ["", "two words", "line\nbreak", "tab\t", "del\x7f", "café"] {
            assert_eq!(
                Schema::new(&[ACCOUNTS, bare(name, "")]),
                Err(SchemaError::InvalidName { name }),
                "{name:?}"
            );
        }
    }

    #[test]
    fn accepts_names_at_both_ends_of_printable_ascii() {
        let parts = [bare("!", ""), bare("a.b-c_d/0", ""), bare("~", "")];
        assert_eq!(Schema::new(&parts).unwrap().parts(), parts);
    }

    /// A column classified by two parts has no single class, so the parts
    /// are refused before anything reads a class from them.
    ///
    /// Verifies: SEC-PRV-001, SEC-TM-050
    #[test]
    fn refuses_a_column_classified_by_two_parts() {
        const RIVAL: SchemaPart = SchemaPart {
            name: "activity",
            sql: "CREATE TABLE plays (at);",
            columns: &[
                column("plays", "at", DataClass::Activity),
                column("person", "plays", DataClass::Activity),
            ],
        };
        assert_eq!(
            Schema::new(&[ACCOUNTS, CATALOGUE, RIVAL]),
            Err(SchemaError::DuplicateColumn {
                table: "person",
                column: "plays",
                parts: ["accounts", "activity"],
            })
        );
    }

    /// Verifies: SEC-PRV-001, SEC-TM-050
    #[test]
    fn refuses_a_column_classified_twice_by_one_part() {
        const TWICE: SchemaPart = SchemaPart {
            name: "twice",
            sql: "CREATE TABLE t (c);",
            columns: &[
                column("t", "c", DataClass::Library),
                column("t", "d", DataClass::Library),
                column("t", "c", DataClass::Secret),
            ],
        };
        assert_eq!(
            Schema::new(&[TWICE]),
            Err(SchemaError::DuplicateColumn {
                table: "t",
                column: "c",
                parts: ["twice", "twice"],
            })
        );
    }

    #[test]
    fn allows_one_column_name_in_several_tables() {
        // "name" appears in both person and server: different columns.
        assert_eq!(
            Schema::new(&[ACCOUNTS, CATALOGUE]).map(|schema| schema.parts().to_vec()),
            Ok(vec![ACCOUNTS, CATALOGUE])
        );
    }

    /// Logging, export, backup encryption and admin views read a column's
    /// class from the schema rather than guessing it.
    ///
    /// Verifies: SEC-PRV-001
    #[test]
    fn looks_up_the_class_declared_for_each_column() {
        let schema = Schema::new(&[CATALOGUE, ACCOUNTS]).unwrap();
        let lookups = [
            ("person", "name", Some(DataClass::Identity)),
            ("person", "plays", Some(DataClass::Activity)),
            ("person", "key", Some(DataClass::Secret)),
            ("track", "title", Some(DataClass::Library)),
            ("server", "name", Some(DataClass::Public)),
            // A known table with an unknown column, a known column name in
            // an unknown table, and neither.
            ("person", "title", None),
            ("album", "title", None),
            ("album", "art", None),
        ];
        for (table, column, class) in lookups {
            assert_eq!(schema.class_of(table, column), class, "{table}.{column}");
        }
    }

    #[test]
    fn accepts_a_live_schema_with_exactly_the_declared_columns() {
        let schema = Schema::new(&[CATALOGUE, ACCOUNTS]).unwrap();
        assert_eq!(schema.check_classes(&live_columns()), Ok(()));
    }

    /// A column the database has but no part classifies fails the check,
    /// naming the column.
    ///
    /// Verifies: SEC-TM-050, SEC-PRV-001
    #[test]
    fn refuses_a_live_column_no_part_classifies() {
        let schema = Schema::new(&[CATALOGUE, ACCOUNTS]).unwrap();
        let mut columns = live_columns();
        columns.insert(2, live("track", "path"));
        assert_eq!(
            schema.check_classes(&columns),
            Err(ClassError::Unclassified {
                table: "track".to_owned(),
                column: "path".to_owned(),
            })
        );
    }

    /// A class declared for a column in one table does not classify a
    /// column of the same name in another.
    ///
    /// Verifies: SEC-TM-050, SEC-PRV-001
    #[test]
    fn refuses_a_live_column_classified_only_in_another_table() {
        let schema = Schema::new(&[CATALOGUE, ACCOUNTS]).unwrap();
        let mut columns = live_columns();
        columns.push(live("server", "title"));
        assert_eq!(
            schema.check_classes(&columns),
            Err(ClassError::Unclassified {
                table: "server".to_owned(),
                column: "title".to_owned(),
            })
        );
    }

    /// A class declared for a column the database does not have is
    /// reported with its part, so the inventory never lists a column that
    /// is not there.
    ///
    /// Verifies: SEC-PRV-001
    #[test]
    fn reports_a_classified_column_the_live_schema_lacks() {
        let schema = Schema::new(&[CATALOGUE, ACCOUNTS]).unwrap();
        let columns: Vec<_> = live_columns()
            .into_iter()
            .filter(|column| *column != live("person", "plays"))
            .collect();
        assert_eq!(
            schema.check_classes(&columns),
            Err(ClassError::Missing {
                part: "accounts",
                table: "person",
                column: "plays",
            })
        );
    }

    #[test]
    fn reports_the_missing_column_of_the_first_part_in_name_order() {
        let schema = Schema::new(&[CATALOGUE, ACCOUNTS]).unwrap();
        let columns = [live("person", "name"), live("person", "plays")];
        assert_eq!(
            schema.check_classes(&columns),
            Err(ClassError::Missing {
                part: "accounts",
                table: "person",
                column: "key",
            })
        );
    }

    #[test]
    fn reports_an_unclassified_column_before_a_missing_one() {
        let schema = Schema::new(&[CATALOGUE, ACCOUNTS]).unwrap();
        let columns = [live("person", "name"), live("person", "age")];
        assert_eq!(
            schema.check_classes(&columns),
            Err(ClassError::Unclassified {
                table: "person".to_owned(),
                column: "age".to_owned(),
            })
        );
    }

    #[test]
    fn reports_a_missing_column_whose_name_another_table_still_has() {
        // server.name is gone, but person still has a "name".
        let schema = Schema::new(&[CATALOGUE, ACCOUNTS]).unwrap();
        let columns: Vec<_> = live_columns()
            .into_iter()
            .filter(|column| *column != live("server", "name"))
            .collect();
        assert_eq!(
            schema.check_classes(&columns),
            Err(ClassError::Missing {
                part: "catalogue",
                table: "server",
                column: "name",
            })
        );
    }

    /// Text that splits into two fields at any point; every byte is
    /// printable ASCII, so each split yields a valid name.
    const SPLIT: &str = "catalogue.tracks";

    proptest! {
        #[test]
        fn gives_the_same_digest_for_the_parts_in_any_order(
            parts in subsequence(
                vec![
                    ACCOUNTS,
                    CATALOGUE,
                    bare("history", "CREATE TABLE play (at);"),
                    bare("settings", ""),
                ],
                0..=4,
            ).prop_shuffle(),
        ) {
            let mut sorted = parts.clone();
            sorted.sort_by_key(|part| part.name);
            let shuffled = Schema::new(&parts).unwrap();
            prop_assert_eq!(shuffled.parts(), &sorted[..]);
            prop_assert_eq!(shuffled.digest(), Schema::new(&sorted).unwrap().digest());
        }

        #[test]
        fn changes_the_digest_wherever_text_moves_between_two_parts(
            first in 0..=SPLIT.len(),
            second in 0..=SPLIT.len(),
        ) {
            let at = |cut: usize| {
                Schema::new(&[bare("a", &SPLIT[..cut]), bare("b", &SPLIT[cut..])])
                    .unwrap()
                    .digest()
            };
            prop_assert_eq!(at(first) == at(second), first == second);
        }

        #[test]
        fn changes_the_digest_wherever_text_moves_between_a_name_and_its_sql(
            first in 1..=SPLIT.len(),
            second in 1..=SPLIT.len(),
        ) {
            let at = |cut: usize| {
                Schema::new(&[bare(&SPLIT[..cut], &SPLIT[cut..])]).unwrap().digest()
            };
            prop_assert_eq!(at(first) == at(second), first == second);
        }

        #[test]
        fn changes_the_digest_wherever_text_moves_between_a_table_and_its_column(
            first in 0..=SPLIT.len(),
            second in 0..=SPLIT.len(),
        ) {
            let at = |cut: usize| {
                let columns = Box::leak(Box::new([
                    column(&SPLIT[..cut], &SPLIT[cut..], DataClass::Library),
                ]));
                Schema::new(&[SchemaPart { name: "part", sql: "", columns }])
                    .unwrap()
                    .digest()
            };
            prop_assert_eq!(at(first) == at(second), first == second);
        }

        #[test]
        fn accepts_a_name_exactly_when_it_is_non_empty_printable_ascii(
            name in prop_oneof![
                "[!-~]{0,12}",
                any::<String>(),
                "[ -\u{7f}\u{e9}]{0,12}",
            ],
        ) {
            // Independent oracle: the printable ASCII range is '!' to '~'.
            let valid = !name.is_empty() && name.chars().all(|c| ('!'..='~').contains(&c));
            let name: &'static str = Box::leak(name.into_boxed_str());
            let expected = if valid {
                Ok(vec![bare(name, "")])
            } else {
                Err(SchemaError::InvalidName { name })
            };
            prop_assert_eq!(
                Schema::new(&[bare(name, "")]).map(|schema| schema.parts().to_vec()),
                expected
            );
        }

        #[test]
        fn accepts_the_declared_columns_in_any_order(
            columns in Just(live_columns()).prop_shuffle(),
        ) {
            let schema = Schema::new(&[ACCOUNTS, CATALOGUE]).unwrap();
            prop_assert_eq!(schema.check_classes(&columns), Ok(()));
        }
    }
}
