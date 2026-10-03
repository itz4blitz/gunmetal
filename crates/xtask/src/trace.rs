//! `xtask trace <release>`: every requirement due in that release has a
//! test or a dated review record (SEC-STD-004), and every live SEC-HIS
//! incident has a rival-replay test (SEC-HIS-066).
//!
//! A test proves a requirement when a `Verifies:` line names its ID. A
//! review record is a file under [`REVIEWS`] whose name or body names the
//! ID and a `YYYY-MM-DD` date. Point releases are releases: an R1.1
//! requirement with no evidence fails `trace R1.1` and not `trace R1`.

use std::collections::BTreeSet;

use crate::docs_lint::{RELEASES, Requirement, rest_from};
use crate::tree::{Tree, is_rust};

/// Dated review records.
pub const REVIEWS: &str = "docs/security/reviews";

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

/// Requirement IDs named on `Verifies:` lines in Rust sources.
fn verified_ids(tree: &dyn Tree) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for name in tree.files("") {
        if !is_rust(&name) {
            continue;
        }
        let Some(text) = tree.read(&name) else {
            continue;
        };
        for line in text.lines() {
            let Some(listed) = verifies_list(line) else {
                continue;
            };
            for id in listed {
                ids.insert(id);
            }
        }
    }
    ids
}

/// IDs after `Verifies:` on `line`.
fn verifies_list(line: &str) -> Option<Vec<String>> {
    let after = line.split("Verifies:").nth(1)?;
    let mut ids = Vec::new();
    let mut rest = after;
    for _ in 0..after.len() {
        let Some(at) = rest.find("SEC-") else {
            break;
        };
        let token = rest_from(rest, at);
        let id: String = token
            .chars()
            .take_while(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || *ch == '-')
            .collect();
        if is_sec(&id) {
            ids.push(id);
        }
        rest = rest_from(rest, at.saturating_add(4));
    }
    Some(ids)
}

/// IDs named by dated review records.
fn review_ids(tree: &dyn Tree) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for name in tree.files(REVIEWS) {
        let path = format!("{REVIEWS}/{name}");
        let Some(text) = tree.read(&path) else {
            continue;
        };
        if !has_date(&text) {
            continue;
        }
        if let Some(stem) = name.strip_suffix(".md")
            && is_sec(stem)
        {
            ids.insert(stem.to_owned());
        }
        for id in sec_ids(&text) {
            ids.insert(id);
        }
    }
    ids
}

/// `YYYY-MM-DD` in `text`.
fn has_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut i: usize = 0;
    while i != bytes.len() && i.saturating_add(10) <= bytes.len() {
        let slice = text.get(i..i.saturating_add(10)).map_or("", |s| s);
        if is_date(slice) {
            return true;
        }
        i = i.saturating_add(1);
    }
    false
}

/// A calendar date `YYYY-MM-DD`.
fn is_date(token: &str) -> bool {
    let mut chars = token.chars();
    for _ in 0..4 {
        if !chars.next().is_some_and(|ch| ch.is_ascii_digit()) {
            return false;
        }
    }
    if chars.next() != Some('-') {
        return false;
    }
    for _ in 0..2 {
        if !chars.next().is_some_and(|ch| ch.is_ascii_digit()) {
            return false;
        }
    }
    if chars.next() != Some('-') {
        return false;
    }
    for _ in 0..2 {
        if !chars.next().is_some_and(|ch| ch.is_ascii_digit()) {
            return false;
        }
    }
    true
}

/// SEC IDs in `text`.
fn sec_ids(text: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut rest = text;
    for _ in 0..text.len() {
        let Some(at) = rest.find("SEC-") else {
            break;
        };
        let token = rest_from(rest, at);
        let id: String = token
            .chars()
            .take_while(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || *ch == '-')
            .collect();
        if is_sec(&id) {
            ids.push(id);
        }
        rest = rest_from(rest, at.saturating_add(4));
    }
    ids
}

/// A `SEC-AREA-number` ID.
fn is_sec(id: &str) -> bool {
    let Some(rest) = id.strip_prefix("SEC-") else {
        return false;
    };
    let Some((area, number)) = rest.split_once('-') else {
        return false;
    };
    !area.is_empty()
        && area.chars().all(|ch| ch.is_ascii_uppercase())
        && !number.is_empty()
        && number.chars().all(|ch| ch.is_ascii_digit())
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

    #[test]
    fn unreadable_sources_and_undated_reviews_are_skipped() {
        let hidden = crate::docs_lint::tests::Hide {
            inner: tree(&[
                ("src/lib.rs", "/// Verifies: SEC-AA-001, SEC-HIS-001\n"),
                ("src/notes.md", "/// Verifies: SEC-AA-002\n"),
                (
                    "docs/security/reviews/notes.md",
                    "No date here SEC-AA-002.\n",
                ),
                (
                    "docs/security/reviews/other.md",
                    "2026/10/02 202X-10-02 2026-1X-02 2026-10-0X 2026-10X02 SEC-AA SEC-aa-001 NOT-AA-001\n",
                ),
                (
                    "docs/security/reviews/ghost.md",
                    "Reviewed SEC-AA-002 on 2026-10-02.\n",
                ),
                (
                    "docs/security/reviews/plain",
                    "Reviewed nothing on 2026-10-02.\n",
                ),
                (
                    "docs/security/reviews/tokens.md",
                    "2026-10-02 SEC-AA SEC-\n",
                ),
            ]),
            hidden: vec!["src/lib.rs", "docs/security/reviews/ghost.md"],
        };
        assert_eq!(
            check(&hidden, "R1"),
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
        assert!(!super::is_sec("NOT-AA-001"));
        assert!(!super::is_sec("SEC-AA"));
        assert_eq!(
            super::release_index("R99"),
            crate::docs_lint::RELEASES.len()
        );
        assert_eq!(super::verifies_list("/// Verifies:"), Some(Vec::new()));
        assert_eq!(
            super::verifies_list("/// Verifies: SEC-AA, SEC-"),
            Some(Vec::new())
        );
        assert_eq!(super::sec_ids("SEC-AA-001"), ["SEC-AA-001"]);
        assert_eq!(super::sec_ids("nope"), Vec::<String>::new());
        assert!(super::has_date("Reviewed on 2026-10-02."));
        assert!(!super::has_date("no date"));
        assert_eq!(super::verifies_list("nope"), None);
        assert!(!super::has_date("short"));
        assert!(!super::is_date("2026-10"));
        assert!(!super::is_sec("SEC--001"));
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
            run(&["standards-watch", "missing-feeds"]),
            (Ok(()), String::new())
        );
    }
}
