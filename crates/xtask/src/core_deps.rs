//! `xtask core-deps`: `gunmetal-core` is compiled into every client, so its
//! normal dependencies, direct and transitive, must be exactly the reviewed
//! allowlist (SEC-SUP-025).
//!
//! The check reads what `cargo tree` reports rather than parsing manifests,
//! so a renamed or target-specific dependency cannot slip past it. It runs:
//!
//! ```text
//! cargo tree --locked -p gunmetal-core -e normal --target all --all-features \
//!   --prefix none --format '{p}' > target/core-deps.txt
//! cargo run --locked -q -p xtask -- core-deps target/core-deps.txt
//! ```
//!
//! `--all-features` matters. Another workspace member can turn on an
//! optional dependency of the core through one of the core's features, and
//! the workspace build then compiles it into the core; without the flag,
//! cargo tree resolves the core's own default features and never lists it.
//! The `core-deps` workflow runs these two commands on every pull request
//! until the integrator adds them to `scripts/gate.sh`.

use std::collections::BTreeSet;

use crate::toml;

/// The reviewed allowlist, which the integrator keeps in `supply-chain/`
/// (work package WP-001). Each allowed crate is one `name = "<crate>"` line;
/// any other line is ignored. A missing file is an empty list, the strictest
/// one, which is the list today.
pub const ALLOWLIST: &str = "supply-chain/core-allowlist.toml";

/// Something `core-deps` found wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finding {
    /// The input does not list `gunmetal-core` itself, so it is not the
    /// output of the `cargo tree` command above.
    NotATree,
    /// A crate the core depends on that the allowlist does not name.
    Unlisted {
        /// The crate.
        name: String,
    },
    /// A crate the allowlist names that the core does not depend on.
    Unused {
        /// The crate.
        name: String,
    },
}

/// Compares `tree`, the `cargo tree` output above, with `allowlist`.
pub fn check(tree: &str, allowlist: &str) -> Vec<Finding> {
    let mut crates: BTreeSet<&str> = tree
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .collect();
    if !crates.remove("gunmetal-core") {
        return vec![Finding::NotATree];
    }
    let allowed: BTreeSet<&str> = allowlist
        .lines()
        .filter_map(|line| toml::value(line, "name"))
        .collect();
    let unlisted = crates.difference(&allowed).map(|&name| Finding::Unlisted {
        name: name.to_owned(),
    });
    let unused = allowed.difference(&crates).map(|&name| Finding::Unused {
        name: name.to_owned(),
    });
    unlisted.chain(unused).collect()
}

#[cfg(test)]
mod tests {
    use super::{Finding, check};

    /// What the gate's `cargo tree` prints for a core with no dependencies.
    const ALONE: &str = "gunmetal-core v0.0.0 (/src/gunmetal/crates/gunmetal-core)\n";

    /// A core that depends on `sha2`, which brings its own dependencies; a
    /// crate seen before is printed again with `(*)`.
    const WITH_SHA2: &str = "\
gunmetal-core v0.0.0 (/src/gunmetal/crates/gunmetal-core)
sha2 v0.10.9
cfg-if v1.0.5
cpufeatures v0.2.17
libc v0.2.190
digest v0.10.7
block-buffer v0.10.4
generic-array v0.14.7
typenum v1.18.0
crypto-common v0.1.6
generic-array v0.14.7 (*)
typenum v1.18.0 (*)
";

    /// An allowlist naming every crate in [`WITH_SHA2`].
    const SHA2_ALLOWED: &str = r#"# The core's reviewed dependencies.

[[crate]]
name = "sha2"
reason = "SHA-256 in crypto.rs only (owner decision 38)"

[[crate]]
name = "block-buffer"
[[crate]]
name = "cfg-if"
[[crate]]
name = "cpufeatures"
[[crate]]
name = "crypto-common"
[[crate]]
name = "digest"
[[crate]]
name = "generic-array"
[[crate]]
name = "libc"
[[crate]]
name = "typenum"
"#;

    /// Verifies: SEC-SUP-025
    #[test]
    fn a_core_with_no_dependencies_passes_the_empty_allowlist() {
        assert_eq!(check(ALONE, ""), []);
    }

    /// Verifies: SEC-SUP-025
    #[test]
    fn a_dependency_tree_the_allowlist_names_in_full_passes() {
        assert_eq!(check(WITH_SHA2, SHA2_ALLOWED), []);
    }

    /// Verifies: SEC-SUP-025
    #[test]
    fn a_dependency_the_allowlist_does_not_name_fails() {
        let allowlist = "name = \"sha2\"\nname = \"digest\" # a comment hides the entry\n";
        let unlisted = |name: &str| Finding::Unlisted {
            name: name.to_owned(),
        };
        assert_eq!(
            check(WITH_SHA2, allowlist),
            [
                unlisted("block-buffer"),
                unlisted("cfg-if"),
                unlisted("cpufeatures"),
                unlisted("crypto-common"),
                unlisted("digest"),
                unlisted("generic-array"),
                unlisted("libc"),
                unlisted("typenum"),
            ]
        );
    }

    /// Verifies: SEC-SUP-025
    #[test]
    fn an_allowlist_entry_the_core_does_not_use_fails() {
        assert_eq!(
            check(ALONE, "name = \"serde\"\n"),
            [Finding::Unused {
                name: "serde".to_owned()
            }]
        );
    }

    /// Verifies: SEC-SUP-025
    #[test]
    fn a_dependency_another_member_turns_on_shows_only_with_all_features() {
        // Recorded from a workspace whose core declares
        // `extra = { path = "../extra", optional = true }` behind its feature
        // `x = ["dep:extra"]`, and whose server depends on the core with
        // `features = ["x"]`, so building the workspace compiles `extra`
        // into the core. Without `--all-features`, cargo tree resolves the
        // core's own default features and never sees `extra`.
        let default = include_str!("../fixtures/core-deps/feature-gated/default-features.txt");
        let all = include_str!("../fixtures/core-deps/feature-gated/all-features.txt");
        assert_eq!(check(default, ""), []);
        assert_eq!(
            check(all, ""),
            [Finding::Unlisted {
                name: "extra".to_owned()
            }]
        );
    }

    /// Verifies: SEC-SUP-025
    #[test]
    fn output_that_does_not_list_the_core_fails() {
        assert_eq!(check("", ""), [Finding::NotATree]);
        assert_eq!(
            check("sha2 v0.10.9\n", "name = \"sha2\"\n"),
            [Finding::NotATree]
        );
    }
}
