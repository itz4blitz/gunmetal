//! `xtask js-deps`: every direct JavaScript dependency is listed in
//! `supply-chain/js-direct-deps.toml` with a written reason (SEC-SUP-035).
//!
//! The list is empty until the web client adds a package. A `package.json`
//! whose `dependencies` or `devDependencies` names a package the list does
//! not, or a list entry with no reason, fails. Tests use fixture manifests,
//! never the network.

use std::collections::BTreeMap;

use crate::json::{self, Value};
use crate::tree::Tree;

/// The reviewed list of direct JavaScript dependencies.
pub const LIST: &str = "supply-chain/js-direct-deps.toml";

/// Something `js-deps` found wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finding {
    /// A `package.json` that is not a JSON object with string dependency
    /// versions.
    Unreadable {
        /// The manifest's path from the repository root.
        path: String,
    },
    /// A direct dependency the list does not name.
    Unlisted {
        /// The manifest that names it.
        path: String,
        /// The package.
        name: String,
    },
    /// A list entry whose reason is empty.
    Unreasoned {
        /// The package.
        name: String,
    },
    /// A list entry no manifest names.
    Unused {
        /// The package.
        name: String,
    },
}

/// Checks every `package.json` in `tree` against [`LIST`].
pub fn check(tree: &dyn Tree) -> Vec<Finding> {
    let listed = listed(&tree.read(LIST).unwrap_or_default());
    let mut used = BTreeMap::new();
    for name in listed.keys() {
        used.insert(name.clone(), false);
    }
    let mut findings = Vec::new();
    for path in manifests(tree) {
        let Some(names) = direct_names(&tree.read(&path).unwrap_or_default()) else {
            findings.push(Finding::Unreadable { path });
            continue;
        };
        for name in names {
            if let Some(seen) = used.get_mut(&name) {
                *seen = true;
            } else {
                findings.push(Finding::Unlisted {
                    path: path.clone(),
                    name,
                });
            }
        }
    }
    for (name, reason) in &listed {
        if reason.is_empty() {
            findings.push(Finding::Unreasoned { name: name.clone() });
        }
    }
    for (name, seen) in used {
        if !seen {
            findings.push(Finding::Unused { name });
        }
    }
    findings
}

/// Every `package.json` path in `tree`, sorted, skipping `node_modules` and
/// `target`.
fn manifests(tree: &dyn Tree) -> Vec<String> {
    tree.files("")
        .into_iter()
        .filter(|path| path.rsplit('/').next() == Some("package.json") && !skip(path))
        .collect()
}

/// Whether `path` is inside a directory the check does not read.
fn skip(path: &str) -> bool {
    path.split('/')
        .any(|part| part == "node_modules" || part == "target")
}

/// The direct dependency names in `text`, or `None` when it is not a
/// manifest the check can read.
fn direct_names(text: &str) -> Option<Vec<String>> {
    let value = json::parse(text)?;
    let Value::Object(_) = &value else {
        return None;
    };
    let mut names = Vec::new();
    for key in ["dependencies", "devDependencies"] {
        match value.get(key) {
            None | Some(Value::Null) => {}
            Some(Value::Object(members)) => {
                for (name, version) in members {
                    if !matches!(version, Value::String(_)) {
                        return None;
                    }
                    names.push(name.clone());
                }
            }
            Some(_) => return None,
        }
    }
    Some(names)
}

/// Package names and reasons from `list`.
fn listed(list: &str) -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    for line in list.lines() {
        if let Some((name, reason)) = entry(line) {
            found.insert(name.to_owned(), reason.to_owned());
        }
    }
    found
}

/// The `name = "reason"` pair `line` holds.
fn entry(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let (name, rest) = line.split_once('=')?;
    let name = quoted(name.trim())?;
    let reason = rest.trim().strip_prefix('"')?.strip_suffix('"')?;
    Some((name, reason))
}

/// `text` without surrounding quotes, or the bare identifier.
fn quoted(text: &str) -> Option<&str> {
    if let Some(inner) = text.strip_prefix('"') {
        inner.strip_suffix('"')
    } else {
        let ident = !text.is_empty()
            && text
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "-_@/.".contains(character));
        ident.then_some(text)
    }
}

#[cfg(test)]
mod tests {
    use super::{Finding, LIST, check, direct_names, entry, listed, manifests, quoted, skip};
    use crate::ROOT;
    use crate::tree::memory::Memory;
    use crate::tree::{Disk, Tree};

    /// A manifest whose `dependencies` names `react`.
    const REACT: &str = r#"{"name":"web","dependencies":{"react":"18.0.0"}}"#;

    /// The list line that reasons about `react`.
    const REACT_LIST: &str = "react = \"the web client's UI library\"\n";

    #[test]
    fn a_tree_with_no_manifests_and_an_empty_list_passes() {
        let tree = Memory::default().with(LIST, "# none yet\n");
        assert_eq!(check(&tree), []);
        assert_eq!(check(&Memory::default()), []);
    }

    /// Verifies: SEC-SUP-035
    #[test]
    fn a_direct_dependency_missing_from_the_list_fails() {
        let tree = Memory::default()
            .with(LIST, "")
            .with("web/package.json", REACT);
        assert_eq!(
            check(&tree),
            [Finding::Unlisted {
                path: "web/package.json".to_owned(),
                name: "react".to_owned(),
            }]
        );
    }

    /// Verifies: SEC-SUP-035
    #[test]
    fn a_listed_reasoned_direct_dependency_passes() {
        let tree = Memory::default()
            .with(LIST, REACT_LIST)
            .with("web/package.json", REACT);
        assert_eq!(check(&tree), []);
    }

    /// Verifies: SEC-SUP-035
    #[test]
    fn an_empty_reason_fails() {
        let tree = Memory::default()
            .with(LIST, "react = \"\"\n")
            .with("web/package.json", REACT);
        assert_eq!(
            check(&tree),
            [Finding::Unreasoned {
                name: "react".to_owned(),
            }]
        );
        let unused = Memory::default().with(LIST, "left-pad = \"\"\n");
        assert_eq!(
            check(&unused),
            [
                Finding::Unreasoned {
                    name: "left-pad".to_owned(),
                },
                Finding::Unused {
                    name: "left-pad".to_owned(),
                },
            ]
        );
    }

    /// Verifies: SEC-SUP-035
    #[test]
    fn a_list_entry_no_manifest_names_fails() {
        let tree = Memory::default().with(LIST, REACT_LIST);
        assert_eq!(
            check(&tree),
            [Finding::Unused {
                name: "react".to_owned(),
            }]
        );
    }

    #[test]
    fn an_unreadable_manifest_fails_by_path() {
        for text in [
            "[]",
            "null",
            "\"pkg\"",
            "{\"dependencies\":[]}",
            "{\"dependencies\":{\"react\":1}}",
        ] {
            let tree = Memory::default().with("package.json", text);
            assert_eq!(
                check(&tree),
                [Finding::Unreadable {
                    path: "package.json".to_owned(),
                }],
                "{text}"
            );
        }
    }

    #[test]
    fn reads_dependencies_and_dev_dependencies() {
        let text = r#"{"dependencies":{"a":"1"},"devDependencies":{"b":"2"},"peerDependencies":{"c":"3"}}"#;
        assert_eq!(
            direct_names(text),
            Some(vec!["a".to_owned(), "b".to_owned()])
        );
        assert_eq!(direct_names("{}"), Some(vec![]));
        assert_eq!(direct_names(r#"{"dependencies":null}"#), Some(vec![]));
        assert_eq!(direct_names("not json"), None);
        assert_eq!(direct_names("[]"), None);
    }

    #[test]
    fn skips_node_modules_and_target() {
        let tree = Memory::default()
            .with("package.json", "{}")
            .with("web/package.json", "{}")
            .with("node_modules/x/package.json", "{}")
            .with("web/node_modules/y/package.json", "{}")
            .with("target/package.json", "{}");
        assert_eq!(manifests(&tree), ["package.json", "web/package.json"]);
        assert!(skip("node_modules/x/package.json"));
        assert!(skip("a/target/b/package.json"));
        assert!(!skip("web/package.json"));
    }

    #[test]
    fn reads_quoted_and_bare_list_entries() {
        assert_eq!(
            entry(r#"react = "the UI library""#),
            Some(("react", "the UI library"))
        );
        assert_eq!(
            entry(r#""@scope/pkg" = "a scoped package""#),
            Some(("@scope/pkg", "a scoped package"))
        );
        assert_eq!(entry("# comment"), None);
        assert_eq!(entry(""), None);
        assert_eq!(entry("react"), None);
        assert_eq!(entry("react = 1"), None);
        assert_eq!(entry(" = \"x\""), None);
        assert_eq!(entry("foo bar = \"x\""), None);
        assert_eq!(entry("\"noend = \"x\""), None);
        assert_eq!(entry("react = \"noend"), None);
        assert_eq!(quoted("react"), Some("react"));
        assert_eq!(quoted("\"@s/p\""), Some("@s/p"));
        assert_eq!(quoted("\"noend"), None);
        assert_eq!(quoted(""), None);
        assert_eq!(quoted("!!!"), None);
        let list = listed("# header\nreact = \"ui\"\n\"@s/p\" = \"scoped\"\n");
        assert_eq!(list.get("react").map(String::as_str), Some("ui"));
        assert_eq!(list.get("@s/p").map(String::as_str), Some("scoped"));
    }

    /// Verifies: SEC-SUP-035
    #[test]
    fn a_listed_dependency_used_in_two_manifests_is_not_unused() {
        let tree = Memory::default()
            .with(LIST, REACT_LIST)
            .with("web/package.json", REACT)
            .with("app/package.json", REACT);
        assert_eq!(check(&tree), []);
    }

    /// Verifies: SEC-SUP-035
    #[test]
    fn the_repository_lists_every_direct_javascript_dependency() {
        let tree = Disk::new(ROOT);
        assert_eq!(check(&tree), []);
        assert_eq!(
            tree.read(LIST).map(|text| text.contains("SEC-SUP-035")),
            Some(true)
        );
    }
}
