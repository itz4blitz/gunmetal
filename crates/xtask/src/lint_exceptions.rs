//! `xtask lint-exceptions`: the bans in `clippy.toml` hold everywhere
//! except in the modules on a short written list.
//!
//! Each risky operation has one door (secure-coding guide, "One door per
//! risk"), and `clippy.toml` bans it everywhere else. Inside the one owning
//! module, a narrowly scoped `#[expect]` turns the ban lint off for the one
//! sanctioned call. This check fails when anything else turns a ban lint
//! off:
//!
//! - a Rust source line that names a ban lint, or a lint group holding one,
//!   in a module that is not on [`EXCEPTIONS`] for that lint (the groups
//!   matter: an `allow` of either group silences the bans, which a probe of
//!   clippy confirmed on 2026-10-03);
//! - a `clippy.toml` or `.clippy.toml` below `crates/`, which would replace
//!   the root configuration, bans and all, for its crate;
//! - a crate manifest that does not take the workspace lints unchanged.
//!   The one exception to this and the previous rule is the core
//!   ([`STRICTER`]): Clippy reads only the nearest `clippy.toml`, so the
//!   core's own allocation bans need a file of their own, and its
//!   deny-level lints a table of their own. The core's tests
//!   (`tests/workspace_rules.rs`) check that both repeat every workspace
//!   rule, and the gate allows no third `clippy.toml`;
//! - a cargo configuration file, `.cargo/config.toml` or the older
//!   `.cargo/config`, anywhere in the repository. `rustflags` in its
//!   `[build]` or `[target]` table turn a ban off for the whole build, and
//!   the gate's clippy step then passes, which a probe confirmed on
//!   2026-10-03. Its other settings (compiler wrappers, source replacement,
//!   aliases) change the build where no other check looks, so no such file
//!   is allowed at all; a package that needs one changes this check, under
//!   review.
//!
//! A list entry whose module no longer names its lint fails too, so the
//! list stays exactly as long as the exceptions it allows.
//!
//! The source search is textual and fails closed: a comment that names a
//! ban lint fails as an exception would, and so does a lint whose name
//! starts with a group's name (`allow_attributes` starts with `all`). The
//! lint names are spelled in two halves in this file so that it never
//! matches itself.

use crate::tree::{Tree, is_rust};

/// One sanctioned exception: a module that may turn one ban lint off.
#[derive(Debug, Clone, Copy)]
pub struct Exception {
    /// The module's path from the repository root, or a directory's path
    /// ending in `/`, which covers every module below it.
    pub path: &'static str,
    /// The lint, one of [`LINTS`].
    pub lint: &'static str,
    /// Why this module is the door, for the reviewer.
    pub reason: &'static str,
}

/// The written list, sorted by path.
pub const EXCEPTIONS: &[Exception] = &[
    Exception {
        path: "crates/gunmetal-core/src/crypto.rs",
        lint: LINTS[2],
        reason: "the core's one SHA-256 door (SEC-STD-018, WP-122)",
    },
    Exception {
        path: "crates/gunmetal-core/src/id.rs",
        lint: LINTS[1],
        reason: "a test helper stands in for the secrets crate's minting function (SEC-HIS-012, WP-006)",
    },
    Exception {
        path: "crates/gunmetal-core/src/parse/capacity.rs",
        lint: LINTS[1],
        reason: "the core's one bounded pre-sizing helper (SEC-MED-003, WP-004)",
    },
    Exception {
        path: "crates/gunmetal-fs/src/dataroot.rs",
        lint: LINTS[1],
        reason: "the data-root handle, the filesystem door (SEC-MED-033, WP-126)",
    },
    Exception {
        path: "crates/gunmetal-fs/src/host.rs",
        lint: LINTS[1],
        reason: "the probe of the data directory's filesystem (ADM-079, WP-126)",
    },
    Exception {
        path: "crates/gunmetal-fs/src/path.rs",
        lint: LINTS[1],
        reason: "the one place a data-directory path is built (SEC-MED-033, WP-126)",
    },
    Exception {
        path: "crates/gunmetal-fs/src/sqlite.rs",
        lint: LINTS[1],
        reason: "the one SQLite opener and statement door (SEC-HIS-016, WP-126)",
    },
    Exception {
        path: "crates/gunmetal-fs/tests/",
        lint: LINTS[1],
        reason: "the filesystem door's tests build hostile layouts on disk (WP-126)",
    },
    Exception {
        path: "crates/gunmetal-fuzz/tests/",
        lint: LINTS[1],
        reason: "corpus replay reads the committed seeds by path (SEC-MED-028)",
    },
    Exception {
        path: "crates/gunmetal-secrets/src/mint.rs",
        lint: LINTS[1],
        reason: "the one public-ID minting function (SEC-HIS-012, WP-047)",
    },
    Exception {
        path: "crates/gunmetal-secrets/src/random.rs",
        lint: LINTS[1],
        reason: "the one function over the OS CSPRNG (SEC-STD-022, WP-047)",
    },
    Exception {
        path: "crates/gunmetal-secrets/src/secret.rs",
        lint: LINTS[1],
        reason: "the test of the one method that exposes a secret (SEC-OPS-013, WP-047)",
    },
    Exception {
        path: "crates/gunmetal-testkit/src/tempdir.rs",
        lint: LINTS[1],
        reason: "test scratch directories (owner decision 33, WP-007)",
    },
    Exception {
        path: "crates/xtask/src/tree.rs",
        lint: LINTS[1],
        reason: "the xtask's one filesystem module (WP-008)",
    },
];

/// The one crate that keeps its own `clippy.toml` and lint tables, because
/// it adds rules to the workspace's (WP-001).
pub const STRICTER: &str = "crates/gunmetal-core/";

/// The ban lints, and the clippy lint groups that hold them.
pub const LINTS: [&str; 5] = [
    concat!("disallowed", "_macros"),
    concat!("disallowed", "_methods"),
    concat!("disallowed", "_types"),
    concat!("clippy::", "all"),
    concat!("clippy::", "style"),
];

/// Something `lint-exceptions` found wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finding {
    /// A source line names a ban lint in a module not listed for it.
    Unlisted {
        /// The module's path from the repository root.
        path: String,
        /// The line, counted from 1.
        line: usize,
        /// The lint the line names.
        lint: &'static str,
    },
    /// A list entry whose module does not name its lint.
    Stale {
        /// The entry's module.
        path: &'static str,
        /// The entry's lint.
        lint: &'static str,
        /// Why the entry was added, so the reader can tell whether the door
        /// moved or went away.
        reason: &'static str,
    },
    /// A clippy configuration below `crates/`.
    LocalConfig {
        /// Its path from the repository root.
        path: String,
    },
    /// A crate manifest that sets its own lints.
    OwnLints {
        /// Its path from the repository root.
        path: String,
    },
    /// A cargo configuration file.
    CargoConfig {
        /// Its path from the repository root.
        path: String,
    },
}

/// Checks every file below `crates/` against `exceptions`, and the whole
/// repository for cargo configuration files.
pub fn check(tree: &dyn Tree, exceptions: &[Exception]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut used = vec![false; exceptions.len()];
    for file in tree.files("crates") {
        let path = format!("crates/{file}");
        let name = file.rsplit('/').next().unwrap_or_default();
        let stricter = path.strip_prefix(STRICTER) == Some(name);
        if name == "clippy.toml" || name == ".clippy.toml" {
            if !stricter {
                findings.push(Finding::LocalConfig { path });
            }
        } else if name == "Cargo.toml"
            && !stricter
            && !takes_workspace_lints(&tree.read(&path).unwrap_or_default())
        {
            findings.push(Finding::OwnLints { path });
        } else if is_rust(name) {
            let source = tree.read(&path).unwrap_or_default();
            for (index, line) in source.lines().enumerate() {
                for lint in LINTS.into_iter().filter(|lint| line.contains(lint)) {
                    match exceptions.iter().position(|exception| {
                        covers(exception.path, &path) && exception.lint == lint
                    }) {
                        Some(listed) => used[listed] = true,
                        None => findings.push(Finding::Unlisted {
                            path: path.clone(),
                            line: index + 1,
                            lint,
                        }),
                    }
                }
            }
        }
    }
    for path in tree.files("") {
        let mut parts = path.rsplit('/');
        let (name, dir) = (parts.next(), parts.next());
        if dir == Some(".cargo") && matches!(name, Some("config" | "config.toml")) {
            findings.push(Finding::CargoConfig { path });
        }
    }
    for (exception, used) in exceptions.iter().zip(used) {
        if !used {
            findings.push(Finding::Stale {
                path: exception.path,
                lint: exception.lint,
                reason: exception.reason,
            });
        }
    }
    findings
}

/// Whether the list entry `entry` covers the module at `path`.
fn covers(entry: &str, path: &str) -> bool {
    match entry.strip_suffix('/') {
        Some(dir) => path
            .strip_prefix(dir)
            .is_some_and(|rest| rest.starts_with('/')),
        None => entry == path,
    }
}

/// Whether a crate `manifest` takes the workspace lints and sets none of its
/// own.
fn takes_workspace_lints(manifest: &str) -> bool {
    manifest.contains("\n[lints]\nworkspace = true\n")
        && !manifest
            .lines()
            .any(|line| line.trim_start().starts_with("[lints."))
}

#[cfg(test)]
mod tests {
    use super::{Exception, Finding, LINTS, check};
    use crate::tree::memory::Memory;

    /// The workspace-lints table every crate manifest carries.
    const MANIFEST: &str = "[package]\nname = \"demo\"\n\n[lints]\nworkspace = true\n";

    /// A module-level `#[expect]` of the clippy lint `lint`, as the one
    /// owning module of a door writes it.
    fn expect(lint: &str) -> String {
        format!("#[expect(\n    clippy::{lint},\n    reason = \"the one door\"\n)]\n")
    }

    #[test]
    fn a_tree_with_no_exceptions_passes() {
        let tree = Memory::default()
            .with("crates/demo/Cargo.toml", MANIFEST)
            .with("crates/demo/src/lib.rs", "#![allow(clippy::pedantic)]\n");
        assert_eq!(check(&tree, &[]), []);
    }

    #[test]
    fn an_exception_outside_the_list_fails_at_its_line() {
        let source = format!("use std::fs;\n\n{}fn open() {{}}\n", expect(LINTS[1]));
        let tree = Memory::default()
            .with("crates/demo/Cargo.toml", MANIFEST)
            .with("crates/demo/src/open.rs", &source);
        assert_eq!(
            check(&tree, &[]),
            [Finding::Unlisted {
                path: "crates/demo/src/open.rs".to_owned(),
                line: 4,
                lint: LINTS[1],
            }]
        );
    }

    #[test]
    fn every_ban_lint_and_group_that_silences_one_is_caught() {
        let source = format!(
            "{}{}{}#![allow({})]\n#[cfg_attr(test, allow({}))]\n",
            expect(LINTS[0]),
            expect(LINTS[1]),
            expect(LINTS[2]),
            LINTS[3],
            LINTS[4]
        );
        let tree = Memory::default().with("crates/demo/src/main.rs", &source);
        let unlisted = |line: usize, lint: &'static str| Finding::Unlisted {
            path: "crates/demo/src/main.rs".to_owned(),
            line,
            lint,
        };
        assert_eq!(
            check(&tree, &[]),
            [
                unlisted(2, LINTS[0]),
                unlisted(6, LINTS[1]),
                unlisted(10, LINTS[2]),
                unlisted(13, LINTS[3]),
                unlisted(14, LINTS[4]),
            ]
        );
    }

    #[test]
    fn a_listed_exception_passes_only_for_its_own_module_and_lint() {
        let listed = [Exception {
            path: "crates/door/src/open.rs",
            lint: LINTS[1],
            reason: "the one door for opening files",
        }];
        let door = format!("{}{}", expect(LINTS[1]), expect(LINTS[2]));
        let tree = Memory::default()
            .with("crates/door/src/open.rs", &door)
            .with("crates/door/src/other.rs", &expect(LINTS[1]));
        assert_eq!(
            check(&tree, &listed),
            [
                Finding::Unlisted {
                    path: "crates/door/src/open.rs".to_owned(),
                    line: 6,
                    lint: LINTS[2],
                },
                Finding::Unlisted {
                    path: "crates/door/src/other.rs".to_owned(),
                    line: 2,
                    lint: LINTS[1],
                },
            ]
        );
    }

    #[test]
    fn a_directory_entry_covers_every_module_below_it_and_nothing_else() {
        let listed = [
            Exception {
                path: "crates/door/tests/",
                lint: LINTS[1],
                reason: "the door's tests build fixtures on disk",
            },
            Exception {
                path: "crates/gone/",
                lint: LINTS[1],
                reason: "a crate since deleted",
            },
        ];
        let tree = Memory::default()
            .with("crates/door/tests/a.rs", &expect(LINTS[1]))
            .with("crates/door/tests/c.rs", &expect(LINTS[2]))
            .with("crates/door/tests/deep/b.rs", &expect(LINTS[1]))
            .with("crates/door/testsuite.rs", &expect(LINTS[1]));
        assert_eq!(
            check(&tree, &listed),
            [
                Finding::Unlisted {
                    path: "crates/door/tests/c.rs".to_owned(),
                    line: 2,
                    lint: LINTS[2],
                },
                Finding::Unlisted {
                    path: "crates/door/testsuite.rs".to_owned(),
                    line: 2,
                    lint: LINTS[1],
                },
                Finding::Stale {
                    path: "crates/gone/",
                    lint: LINTS[1],
                    reason: "a crate since deleted",
                },
            ]
        );
    }

    #[test]
    fn only_the_core_keeps_its_stricter_clippy_configuration_and_lints() {
        let own = "[package]\nname = \"own\"\n\n[lints.clippy]\nunwrap_used = \"deny\"\n";
        let tree = Memory::default()
            .with("crates/gunmetal-core-extra/Cargo.toml", own)
            .with("crates/gunmetal-core-extra/clippy.toml", "")
            .with("crates/gunmetal-core/Cargo.toml", own)
            .with("crates/gunmetal-core/clippy.toml", "")
            .with("crates/gunmetal-core/tests/Cargo.toml", own)
            .with("crates/gunmetal-core/tests/clippy.toml", "");
        assert_eq!(
            check(&tree, &[]),
            [
                Finding::OwnLints {
                    path: "crates/gunmetal-core-extra/Cargo.toml".to_owned()
                },
                Finding::LocalConfig {
                    path: "crates/gunmetal-core-extra/clippy.toml".to_owned()
                },
                Finding::OwnLints {
                    path: "crates/gunmetal-core/tests/Cargo.toml".to_owned()
                },
                Finding::LocalConfig {
                    path: "crates/gunmetal-core/tests/clippy.toml".to_owned()
                },
            ]
        );
    }

    #[test]
    fn a_list_entry_with_no_exception_left_fails() {
        let listed = [
            Exception {
                path: "crates/door/src/open.rs",
                lint: LINTS[1],
                reason: "the one door for opening files",
            },
            Exception {
                path: "crates/door/src/gone.rs",
                lint: LINTS[2],
                reason: "a module since deleted",
            },
        ];
        let tree = Memory::default().with("crates/door/src/open.rs", &expect(LINTS[1]));
        assert_eq!(
            check(&tree, &listed),
            [Finding::Stale {
                path: "crates/door/src/gone.rs",
                lint: LINTS[2],
                reason: "a module since deleted",
            }]
        );
    }

    #[test]
    fn a_clippy_configuration_below_crates_fails() {
        let tree = Memory::default()
            .with("crates/demo/clippy.toml", "")
            .with("crates/other/.clippy.toml", "")
            .with("crates/demo/notes.toml", "");
        assert_eq!(
            check(&tree, &[]),
            [
                Finding::LocalConfig {
                    path: "crates/demo/clippy.toml".to_owned()
                },
                Finding::LocalConfig {
                    path: "crates/other/.clippy.toml".to_owned()
                },
            ]
        );
    }

    #[test]
    fn a_manifest_that_does_not_take_the_workspace_lints_unchanged_fails() {
        let own = format!(
            "[package]\nname = \"own\"\n\n[lints.clippy]\n{} = \"allow\"\n",
            LINTS[1]
        );
        let both = format!("{MANIFEST}\n[lints.rust]\nunsafe_code = \"allow\"\n");
        let tree = Memory::default()
            .with("crates/both/Cargo.toml", &both)
            .with("crates/good/Cargo.toml", MANIFEST)
            .with("crates/none/Cargo.toml", "[package]\nname = \"none\"\n")
            .with("crates/own/Cargo.toml", &own);
        let own_lints = |path: &str| Finding::OwnLints {
            path: path.to_owned(),
        };
        assert_eq!(
            check(&tree, &[]),
            [
                own_lints("crates/both/Cargo.toml"),
                own_lints("crates/none/Cargo.toml"),
                own_lints("crates/own/Cargo.toml"),
            ]
        );
    }

    #[test]
    fn a_cargo_configuration_anywhere_in_the_repository_fails() {
        // Rustflags in either file silence a ban for the whole build, which
        // a probe of cargo clippy confirmed on 2026-10-03.
        let rustflags = format!("[build]\nrustflags = [\"-Aclippy::{}\"]\n", LINTS[1]);
        let tree = Memory::default()
            .with(".cargo/config.toml", &rustflags)
            .with(".cargo/notes.md", "")
            .with("crates/demo/.cargo/config.toml", "[alias]\nx = \"run\"\n")
            .with("docs/cargo/config.toml", "")
            .with("docs/old.cargo/config.toml", "")
            .with("fuzz/.cargo/config", &rustflags)
            .with("fuzz/config.toml", "");
        let config = |path: &str| Finding::CargoConfig {
            path: path.to_owned(),
        };
        assert_eq!(
            check(&tree, &[]),
            [
                config(".cargo/config.toml"),
                config("crates/demo/.cargo/config.toml"),
                config("fuzz/.cargo/config"),
            ]
        );
    }

    #[test]
    fn files_that_are_neither_sources_nor_manifests_are_not_searched() {
        let text = expect(LINTS[1]);
        let tree = Memory::default()
            .with("crates/demo/README.md", &text)
            .with("crates/demo/fixtures/sample.rs.txt", &text);
        assert_eq!(check(&tree, &[]), []);
    }
}
