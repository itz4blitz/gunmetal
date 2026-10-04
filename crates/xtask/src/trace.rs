//! `xtask trace <release>`: every requirement due in that release has a
//! test or a dated review record (SEC-STD-004), and every live SEC-HIS
//! incident has a rival-replay test (SEC-HIS-066).
//!
//! A test proves a requirement when a comment line that starts with
//! `Verifies:` names its ID, in a Rust or TypeScript source. A review
//! record is a file under [`REVIEWS`] that holds a `YYYY-MM-DD` date and
//! names the ID, in its file name or its text. Point releases are releases: an R1.1
//! requirement with no evidence fails `trace R1.1` and not `trace R1`.

use std::collections::BTreeSet;

use crate::docs_lint::{RELEASES, Requirement, cited_sec};
use crate::standards::date_seconds;
use crate::tree::Tree;

/// Dated review records.
pub const REVIEWS: &str = "docs/security/reviews";

/// The file name endings of sources whose tests carry `Verifies:` lines:
/// Rust, and the web client's TypeScript.
pub const SOURCES: [&str; 3] = [".rs", ".ts", ".tsx"];

/// Directories whose files are not this repository's sources.
pub const OUTSIDE: [&str; 2] = ["node_modules", "target"];

/// Something the traceability check found wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finding {
    /// A requirement due in the release has no test and no review.
    Unproven {
        /// The requirement.
        id: String,
        /// Its Release column.
        release: String,
    },
    /// A live SEC-HIS incident has no rival-replay test.
    NoReplay {
        /// The incident requirement.
        id: String,
    },
}

/// Requirements due in `release` that lack the evidence SEC-STD-004 asks for.
pub fn check(tree: &dyn Tree, release: &str) -> Vec<Finding> {
    let reqs = crate::docs_lint::requirements(tree);
    let verified = verified_ids(tree);
    let reviews = review_ids(tree);
    let mut findings = Vec::new();
    for req in reqs {
        if !due(&req, release) {
            continue;
        }
        if req.id.starts_with("SEC-HIS-") {
            if !verified.contains(&req.id) {
                findings.push(Finding::NoReplay { id: req.id });
            }
            continue;
        }
        if !verified.contains(&req.id) && !reviews.contains(&req.id) {
            findings.push(Finding::Unproven {
                id: req.id,
                release: req.release,
            });
        }
    }
    findings
}

/// A published mapping of due requirements to the evidence that covers them.
pub fn report(tree: &dyn Tree, release: &str) -> String {
    let reqs = crate::docs_lint::requirements(tree);
    let verified = verified_ids(tree);
    let reviews = review_ids(tree);
    let mut lines = vec![format!("# Traceability {release}\n")];
    for req in reqs {
        if !due(&req, release) {
            continue;
        }
        let evidence = if verified.contains(&req.id) {
            "test"
        } else if reviews.contains(&req.id) {
            "review"
        } else {
            "missing"
        };
        lines.push(format!("{} {} {evidence}\n", req.id, req.release));
    }
    lines.concat()
}

/// Whether `req` is live and its release is at or before `target`.
fn due(req: &Requirement, target: &str) -> bool {
    req.is_live() && release_index(&req.release) <= release_index(target)
}

/// Position of `release` in [`RELEASES`], or past the end when unknown.
fn release_index(release: &str) -> usize {
    RELEASES
        .iter()
        .position(|name| *name == release)
        .unwrap_or(RELEASES.len())
}

/// Requirement IDs named on `Verifies:` lines in the repository's sources.
fn verified_ids(tree: &dyn Tree) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for name in tree.files("") {
        if !is_source(&name) {
            continue;
        }
        let text = tree.read(&name).unwrap_or_default();
        for line in text.lines() {
            ids.extend(verifies(line));
        }
    }
    ids
}

/// Whether `path` is a Rust or TypeScript source of this repository: not
/// build output, an installed package or anything under a hidden directory,
/// where another checkout's tests could stand in for this one's.
fn is_source(path: &str) -> bool {
    path.split('/')
        .all(|part| !part.starts_with('.') && !OUTSIDE.contains(&part))
        && SOURCES.iter().any(|extension| path.ends_with(extension))
}

/// The IDs `line` names, when it is a comment that starts with `Verifies:`.
/// Text inside a string, as in this crate's own fixtures, proves nothing.
fn verifies(line: &str) -> Vec<String> {
    line.trim_start()
        .strip_prefix("//")
        .map(|comment| comment.trim_start_matches('/').trim_start())
        .and_then(|comment| comment.strip_prefix("Verifies:"))
        .map(cited_sec)
        .unwrap_or_default()
}

/// IDs named by dated review records: by the file's name or in its text.
fn review_ids(tree: &dyn Tree) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for name in tree.files(REVIEWS) {
        let text = tree.read(&format!("{REVIEWS}/{name}")).unwrap_or_default();
        if !has_date(&text) {
            continue;
        }
        ids.extend(cited_sec(&name));
        ids.extend(cited_sec(&text));
    }
    ids
}

/// Whether `text` holds a calendar date written `YYYY-MM-DD`.
fn has_date(text: &str) -> bool {
    text.split(|ch: char| !ch.is_ascii_digit() && ch != '-')
        .any(|token| date_seconds(token).is_some())
}

#[cfg(test)]
mod tests {
    use super::{Finding, check, report};
    use crate::tree::memory::Memory;
    use crate::{Failure, dispatch};

    /// One requirement table used by every fixture.
    const ROWS: &str = "\
# Security

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-AA-001 | An R1 rule. | ASVS 1.1.1 | R1 | test |
| SEC-AA-002 | An R1.1 rule. | ASVS 1.1.1 | R1.1 | test |
| SEC-HIS-001 | A rival incident. | SSDF RV.3.3 | R1 | replay |
| SEC-AA-003 | Withdrawn. | ASVS 1.1.1 | Withdrawn | n/a |
";

    /// A tree with the requirement table and optional evidence.
    fn tree(extra: &[(&str, &str)]) -> Memory {
        let mut memory = Memory::default().with("docs/security/rules.md", ROWS);
        for (path, text) in extra {
            memory = memory.with(path, text);
        }
        memory
    }

    /// Verifies: SEC-STD-004
    #[test]
    fn an_r1_requirement_needs_a_test_or_a_review() {
        let empty = tree(&[]);
        assert_eq!(
            check(&empty, "R1"),
            [
                Finding::Unproven {
                    id: "SEC-AA-001".to_owned(),
                    release: "R1".to_owned(),
                },
                Finding::NoReplay {
                    id: "SEC-HIS-001".to_owned(),
                },
            ]
        );
        let tested = tree(&[(
            "crates/core/src/lib.rs",
            "/// Verifies: SEC-AA-001, SEC-HIS-001\n#[test]\nfn replay() {}\n",
        )]);
        assert_eq!(check(&tested, "R1"), []);
        let reviewed = tree(&[(
            "docs/security/reviews/SEC-AA-001.md",
            "Reviewed SEC-AA-001 on 2026-10-02.\n",
        )]);
        assert_eq!(
            check(&reviewed, "R1"),
            [Finding::NoReplay {
                id: "SEC-HIS-001".to_owned(),
            }]
        );
    }

    /// Verifies: SEC-STD-004
    #[test]
    fn an_r11_requirement_fails_r11_and_not_r1() {
        let tested = tree(&[("src/lib.rs", "/// Verifies: SEC-AA-001, SEC-HIS-001\n")]);
        assert_eq!(check(&tested, "R1"), []);
        assert_eq!(
            check(&tested, "R1.1"),
            [Finding::Unproven {
                id: "SEC-AA-002".to_owned(),
                release: "R1.1".to_owned(),
            }]
        );
    }

    /// Verifies: SEC-HIS-066
    #[test]
    fn a_review_does_not_stand_in_for_a_rival_replay() {
        let tree = tree(&[(
            "docs/security/reviews/SEC-HIS-001.md",
            "Reviewed SEC-HIS-001 on 2026-10-02.\n",
        )]);
        assert_eq!(
            check(&tree, "R1"),
            [
                Finding::Unproven {
                    id: "SEC-AA-001".to_owned(),
                    release: "R1".to_owned(),
                },
                Finding::NoReplay {
                    id: "SEC-HIS-001".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn the_report_lists_each_due_requirement() {
        let tested = tree(&[("src/lib.rs", "/// Verifies: SEC-AA-001, SEC-HIS-001\n")]);
        assert_eq!(
            report(&tested, "R1.1"),
            "# Traceability R1.1\nSEC-AA-001 R1 test\nSEC-AA-002 R1.1 missing\nSEC-HIS-001 R1 test\n"
        );
        let reviewed = tree(&[
            (
                "docs/security/reviews/SEC-AA-001.md",
                "Reviewed SEC-AA-001 on 2026-10-02.\n",
            ),
            ("src/lib.rs", "/// Verifies: SEC-HIS-001\n"),
        ]);
        assert_eq!(
            report(&reviewed, "R1"),
            "# Traceability R1\nSEC-AA-001 R1 review\nSEC-HIS-001 R1 test\n"
        );
    }

    /// The two findings of a tree with no evidence at all.
    fn nothing_proven() -> [Finding; 2] {
        [
            Finding::Unproven {
                id: "SEC-AA-001".to_owned(),
                release: "R1".to_owned(),
            },
            Finding::NoReplay {
                id: "SEC-HIS-001".to_owned(),
            },
        ]
    }

    /// Verifies: SEC-STD-004
    #[test]
    fn only_a_comment_line_that_starts_with_verifies_counts() {
        let quoted = tree(&[(
            "src/lib.rs",
            "let fixture = \"/// Verifies: SEC-AA-001, SEC-HIS-001\";\n//! Verifies: SEC-AA-001, SEC-HIS-001\n/// It verifies: SEC-AA-001, SEC-HIS-001\nVerifies: SEC-AA-001, SEC-HIS-001\n",
        )]);
        assert_eq!(check(&quoted, "R1"), nothing_proven());
        let indented = tree(&[(
            "src/lib.rs",
            "    /// Verifies: SEC-AA-001\n    #[test]\n\t//Verifies: SEC-HIS-001\n",
        )]);
        assert_eq!(check(&indented, "R1"), []);
    }

    /// Verifies: SEC-STD-004
    #[test]
    fn typescript_tests_prove_requirements_too() {
        let typescript = tree(&[
            (
                "apps/web/src/title.test.ts",
                "  // Verifies: SEC-AA-001\n  test(\"a title is shown as text\", () => {});\n",
            ),
            (
                "apps/web/src/replay.test.tsx",
                "// Verifies: SEC-HIS-001\ntest(\"the incident is replayed\", () => {});\n",
            ),
        ]);
        assert_eq!(check(&typescript, "R1"), []);
        let other = tree(&[
            ("apps/web/src/title.test.js", "// Verifies: SEC-AA-001\n"),
            ("docs/notes.md", "// Verifies: SEC-HIS-001\n"),
            ("src/rs", "// Verifies: SEC-AA-001\n"),
            ("src/libxrs", "// Verifies: SEC-HIS-001\n"),
        ]);
        assert_eq!(check(&other, "R1"), nothing_proven());
    }

    /// Verifies: SEC-STD-004
    #[test]
    fn build_output_installed_packages_and_hidden_directories_prove_nothing() {
        let line = "/// Verifies: SEC-AA-001, SEC-HIS-001\n";
        for path in [
            "target/package/src/lib.rs",
            "apps/web/node_modules/pkg/index.ts",
            ".claude/worktrees/other/crates/core/src/lib.rs",
            "crates/core/.cache/lib.rs",
        ] {
            assert_eq!(check(&tree(&[(path, line)]), "R1"), nothing_proven());
        }
        for path in ["crates/target.rs", "crates/targets/src/lib.rs"] {
            assert_eq!(check(&tree(&[(path, line)]), "R1"), []);
        }
    }

    /// Verifies: SEC-STD-004
    #[test]
    fn a_review_record_needs_a_date_and_the_requirement_id() {
        let proven = [Finding::NoReplay {
            id: "SEC-HIS-001".to_owned(),
        }];
        let named_by_file = tree(&[(
            "docs/security/reviews/SEC-AA-001.md",
            "Reviewed on 2026-10-02.\n",
        )]);
        assert_eq!(check(&named_by_file, "R1"), proven);
        let named_in_text = tree(&[(
            "docs/security/reviews/2026-10-crypto.md",
            "Date: 2026-10-02\n\nCovers SEC-AA-001.\n",
        )]);
        assert_eq!(check(&named_in_text, "R1"), proven);
        for undated in [
            "Reviewed SEC-AA-001.\n",
            "Reviewed SEC-AA-001 on 2026/10/02.\n",
            "Reviewed SEC-AA-001 on 2026-13-02.\n",
            "Reviewed SEC-AA-001 in 2026-10.\n",
        ] {
            let record = tree(&[("docs/security/reviews/SEC-AA-001.md", undated)]);
            assert_eq!(check(&record, "R1"), nothing_proven());
        }
        let elsewhere = tree(&[(
            "docs/security/SEC-AA-001.md",
            "Reviewed SEC-AA-001 on 2026-10-02.\n",
        )]);
        assert_eq!(check(&elsewhere, "R1"), nothing_proven());
    }

    #[test]
    fn a_file_that_cannot_be_read_proves_nothing() {
        let hidden = crate::docs_lint::tests::Hide {
            inner: tree(&[
                ("src/lib.rs", "/// Verifies: SEC-AA-001, SEC-HIS-001\n"),
                (
                    "docs/security/reviews/SEC-AA-001.md",
                    "Reviewed SEC-AA-001 on 2026-10-02.\n",
                ),
            ]),
            hidden: vec!["src/lib.rs", "docs/security/reviews/SEC-AA-001.md"],
        };
        assert_eq!(check(&hidden, "R1"), nothing_proven());
    }

    #[test]
    fn an_unknown_release_is_after_every_known_one() {
        let tested = tree(&[("src/lib.rs", "/// Verifies: SEC-HIS-001\n")]);
        assert_eq!(
            report(&tested, "R9"),
            "# Traceability R9\nSEC-AA-001 R1 missing\nSEC-AA-002 R1.1 missing\nSEC-HIS-001 R1 test\n"
        );
    }

    #[test]
    fn the_subcommands_are_wired_to_the_checks() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures");
        let run = |cmd: &[&str]| {
            let args: Vec<String> = cmd.iter().map(|&arg| arg.to_owned()).collect();
            let mut out = Vec::new();
            let result = dispatch(&args, root, 0, &[], &mut out);
            (result, String::from_utf8(out).expect("utf8"))
        };
        assert_eq!(run(&["trace", "R1"]), (Ok(()), String::new()));
        let (result, text) = run(&["trace-report", "R1"]);
        assert_eq!(result, Ok(()));
        assert_eq!(text, "# Traceability R1\n");
        assert_eq!(
            run(&["standards-coverage"]).0,
            Err(Failure::Findings(vec!["MissingCoverage".to_owned()]))
        );
        assert_eq!(
            run(&["standards-watch", "missing-feeds"]).0,
            Err(Failure::Findings(vec![
                "NoFeed { standard: \"asvs\", pinned: \"5.0.0\" }".to_owned(),
                "NoFeed { standard: \"top10\", pinned: \"2025\" }".to_owned(),
                "NoFeed { standard: \"api-top10\", pinned: \"2023\" }".to_owned(),
                "NoFeed { standard: \"masvs\", pinned: \"2.1.0\" }".to_owned(),
                "NoFeed { standard: \"cwe-top25\", pinned: \"2025\" }".to_owned(),
                "NoFeed { standard: \"ssdf\", pinned: \"1.1\" }".to_owned(),
            ]))
        );
    }
}
