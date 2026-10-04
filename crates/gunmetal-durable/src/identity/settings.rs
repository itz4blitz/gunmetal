//! Settings a migration may not loosen or change.
//!
//! No migration may make an installation less strict, and a setting added
//! in a release starts at its strictest value on an upgraded install
//! (SEC-OPS-049). No migration may change a person's privacy choice, and a
//! new one starts at its most private value (SEC-PRV-023). Neither may drop
//! a setting to get through (SEC-OPS-051). Each package that keeps a
//! security setting or a privacy choice in the identity store declares it
//! as a [`Setting`] with its values in order, and the migration runner reads
//! every declared setting before and after the migration, inside its
//! transaction, and refuses to commit a result [`check`] rejects.

use gunmetal_fs::sqlite::{Query, Row, Value};

/// What kind of rule a [`Setting`] follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingKind {
    /// A security setting: a migration may leave it or make it stricter,
    /// never looser (SEC-OPS-049).
    Security,
    /// A person's privacy choice: a migration leaves it exactly as it was
    /// (SEC-PRV-023).
    Privacy,
}

/// A security setting or privacy choice kept in the identity store.
#[derive(Debug, Clone, PartialEq)]
pub struct Setting {
    /// The setting's name, for the error that names it.
    pub name: &'static str,
    /// Which rule it follows.
    pub kind: SettingKind,
    /// The first store format whose files hold it. Before that version the
    /// setting does not exist, so every value a migration gives it is new.
    pub since: usize,
    /// Reads every value: one row per holder (the server, a person, a
    /// library), each a key and an integer value.
    pub read: Query,
    /// Every value it may take, from the least strict (or least private) to
    /// the most.
    pub order: &'static [i64],
}

/// Why a migration's effect on a [`Setting`] was refused.
#[derive(Debug, Clone, PartialEq)]
pub enum SettingProblem {
    /// A row was not a key and an integer.
    Unreadable {
        /// The row.
        found: Row,
    },
    /// A value is not one the setting's order lists.
    Invalid {
        /// Whose value it is.
        key: Value,
        /// The value.
        value: i64,
    },
    /// The migration removed a value.
    Dropped {
        /// Whose value it was.
        key: Value,
    },
    /// The migration made a security setting less strict.
    Loosened {
        /// Whose value it is.
        key: Value,
        /// The value before.
        before: i64,
        /// The value after.
        after: i64,
    },
    /// The migration changed a privacy choice.
    Changed {
        /// Whose choice it is.
        key: Value,
        /// The value before.
        before: i64,
        /// The value after.
        after: i64,
    },
    /// A value the migration added is not the strictest.
    NotStrictest {
        /// Whose value it is.
        key: Value,
        /// The value.
        value: i64,
    },
}

/// Checks a migration's effect on `setting`, whose rows were `before` and
/// are now `after`.
///
/// # Errors
///
/// Returns the first [`SettingProblem`] found: an unreadable row, a value
/// outside the order, then for each key before, in order, a dropped,
/// loosened or changed value, then for each key added, in order, one that
/// is not the strictest.
pub fn check(setting: &Setting, before: &[Row], after: &[Row]) -> Result<(), SettingProblem> {
    let before = entries(setting, before)?;
    let after = entries(setting, after)?;
    for &(key, was, was_rank) in &before {
        let (_, now, now_rank) = *after
            .iter()
            .find(|(other, _, _)| *other == key)
            .ok_or_else(|| SettingProblem::Dropped { key: key.clone() })?;
        match setting.kind {
            SettingKind::Security if now_rank < was_rank => {
                return Err(SettingProblem::Loosened {
                    key: key.clone(),
                    before: was,
                    after: now,
                });
            }
            SettingKind::Privacy if now != was => {
                return Err(SettingProblem::Changed {
                    key: key.clone(),
                    before: was,
                    after: now,
                });
            }
            SettingKind::Security | SettingKind::Privacy => {}
        }
    }
    let strictest = setting.order.last();
    match after
        .iter()
        .filter(|(key, _, _)| !before.iter().any(|(other, _, _)| other == key))
        .find(|(_, value, _)| strictest != Some(value))
    {
        Some(&(key, value, _)) => Err(SettingProblem::NotStrictest {
            key: key.clone(),
            value,
        }),
        None => Ok(()),
    }
}

/// Reads each row as a key, its value and the value's place in the order.
fn entries<'a>(
    setting: &Setting,
    rows: &'a [Row],
) -> Result<Vec<(&'a Value, i64, usize)>, SettingProblem> {
    rows.iter()
        .map(|row| match row.0.as_slice() {
            [key, Value::Integer(value)] => setting
                .order
                .iter()
                .position(|listed| listed == value)
                .map(|rank| (key, *value, rank))
                .ok_or_else(|| SettingProblem::Invalid {
                    key: key.clone(),
                    value: *value,
                }),
            _ => Err(SettingProblem::Unreadable { found: row.clone() }),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// Off, then on for administrators, then on for everyone: on is
    /// stricter.
    const ORDER: &[i64] = &[0, 5, 9];

    fn setting(kind: SettingKind) -> Setting {
        Setting {
            name: "posture",
            kind,
            since: 0,
            read: Query::new("SELECT holder, posture FROM settings"),
            order: ORDER,
        }
    }

    fn key(name: &str) -> Value {
        Value::Text(name.to_owned())
    }

    fn rows(pairs: &[(&str, i64)]) -> Vec<Row> {
        pairs
            .iter()
            .map(|&(name, value)| Row(vec![key(name), Value::Integer(value)]))
            .collect()
    }

    #[test]
    fn accepts_a_migration_that_keeps_or_tightens_a_security_setting() {
        let before = rows(&[("server", 0), ("library", 5)]);
        let after = rows(&[("library", 9), ("server", 0)]);
        assert_eq!(
            check(&setting(SettingKind::Security), &before, &after),
            Ok(())
        );
        assert_eq!(
            check(&setting(SettingKind::Security), &before, &before),
            Ok(())
        );
    }

    /// Verifies: SEC-OPS-049
    #[test]
    fn refuses_a_migration_that_loosens_a_security_setting() {
        let before = rows(&[("server", 0), ("library", 5)]);
        let after = rows(&[("server", 5), ("library", 0)]);
        assert_eq!(
            check(&setting(SettingKind::Security), &before, &after),
            Err(SettingProblem::Loosened {
                key: key("library"),
                before: 5,
                after: 0,
            })
        );
    }

    /// Verifies: SEC-PRV-023
    #[test]
    fn refuses_a_migration_that_changes_a_privacy_choice_even_to_stricter() {
        let before = rows(&[("sam", 0), ("kim", 5)]);
        for now in [9, 0] {
            assert_eq!(
                check(
                    &setting(SettingKind::Privacy),
                    &before,
                    &rows(&[("sam", 0), ("kim", now)])
                ),
                Err(SettingProblem::Changed {
                    key: key("kim"),
                    before: 5,
                    after: now,
                })
            );
        }
        assert_eq!(
            check(&setting(SettingKind::Privacy), &before, &before),
            Ok(())
        );
    }

    /// Verifies: SEC-OPS-051
    #[test]
    fn refuses_a_migration_that_drops_a_setting() {
        for kind in [SettingKind::Security, SettingKind::Privacy] {
            assert_eq!(
                check(
                    &setting(kind),
                    &rows(&[("server", 9), ("sam", 0)]),
                    &rows(&[("server", 9)])
                ),
                Err(SettingProblem::Dropped { key: key("sam") })
            );
        }
    }

    /// Verifies: SEC-OPS-049, SEC-PRV-023
    #[test]
    fn requires_a_setting_a_migration_adds_to_start_at_its_strictest() {
        for kind in [SettingKind::Security, SettingKind::Privacy] {
            assert_eq!(
                check(
                    &setting(kind),
                    &rows(&[("sam", 0)]),
                    &rows(&[("sam", 0), ("kim", 9)])
                ),
                Ok(())
            );
            assert_eq!(
                check(
                    &setting(kind),
                    &[],
                    &rows(&[("sam", 9), ("kim", 5), ("lee", 0)])
                ),
                Err(SettingProblem::NotStrictest {
                    key: key("kim"),
                    value: 5,
                })
            );
        }
    }

    #[test]
    fn refuses_a_new_value_when_the_order_is_empty() {
        let empty = Setting {
            order: &[],
            ..setting(SettingKind::Security)
        };
        assert_eq!(check(&empty, &[], &[]), Ok(()));
        assert_eq!(
            check(&empty, &[], &rows(&[("sam", 0)])),
            Err(SettingProblem::Invalid {
                key: key("sam"),
                value: 0,
            })
        );
    }

    #[test]
    fn refuses_a_value_outside_the_order_before_or_after() {
        let invalid = Err(SettingProblem::Invalid {
            key: key("sam"),
            value: 7,
        });
        let kind = SettingKind::Security;
        assert_eq!(
            check(&setting(kind), &rows(&[("sam", 7)]), &rows(&[("sam", 9)])),
            invalid
        );
        assert_eq!(
            check(&setting(kind), &rows(&[("sam", 0)]), &rows(&[("sam", 7)])),
            invalid
        );
    }

    #[test]
    fn refuses_a_row_that_is_not_a_key_and_an_integer() {
        let kind = SettingKind::Security;
        for found in [
            Row(vec![key("sam")]),
            Row(vec![key("sam"), Value::Integer(0), Value::Null]),
            Row(vec![key("sam"), Value::Text("0".to_owned())]),
        ] {
            let unreadable = Err(SettingProblem::Unreadable {
                found: found.clone(),
            });
            assert_eq!(
                check(&setting(kind), std::slice::from_ref(&found), &[]),
                unreadable
            );
            assert_eq!(check(&setting(kind), &[], &[found]), unreadable);
        }
    }

    /// Strictness under the documented order, written independently: the
    /// rank of a value is where it appears in [`ORDER`].
    fn rank(value: i64) -> usize {
        match value {
            0 => 0,
            5 => 1,
            _ => 2,
        }
    }

    proptest! {
        /// For any configuration before and any after, a security setting
        /// passes exactly when no holder's value got less strict and every
        /// holder the migration added starts at the strictest value.
        ///
        /// Verifies: SEC-OPS-049
        #[test]
        fn passes_a_security_setting_exactly_when_nothing_got_looser(
            before in prop::collection::vec(prop::sample::select(ORDER.to_vec()), 0..4),
            after in prop::collection::vec(prop::sample::select(ORDER.to_vec()), 0..5),
        ) {
            let named = |values: &[i64]| -> Vec<Row> {
                values.iter().enumerate().map(|(index, value)| {
                    Row(vec![Value::Text(format!("holder {index}")), Value::Integer(*value)])
                }).collect()
            };
            let kept = before.len() <= after.len()
                && before.iter().zip(&after).all(|(was, now)| rank(*now) >= rank(*was))
                && after.iter().skip(before.len()).all(|now| *now == 9);
            prop_assert_eq!(
                check(&setting(SettingKind::Security), &named(&before), &named(&after)).is_ok(),
                kept
            );
        }
    }
}
