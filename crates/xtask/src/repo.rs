//! `xtask repo`: repository protections, workflow pinning, Scorecard
//! thresholds, settings drift and advisory regression tests.
//!
//! Checks are pure functions of a [`Tree`] plus recorded GitHub JSON. They
//! never touch the network. `repo` walks committed policy files; `repo
//! settings`, `repo scorecard` and `repo advisories` read a dump the
//! workflow saved.

use crate::codeowners;
use crate::json::{self, Value};
use crate::tree::{Tree, is_rust};

/// Seconds in 365 days, the `SECURITY.md` review window.
pub const YEAR: u64 = 31_536_000;

/// Scorecard checks that must score 10 (SEC-SUP-019).
pub const MUST_BE_TEN: &[&str] = &[
    "Binary-Artifacts",
    "Dangerous-Workflow",
    "Dependency-Update-Tool",
    "License",
    "Pinned-Dependencies",
    "Security-Policy",
    "Token-Permissions",
];

/// Scorecard checks with a recorded floor: the scores of the run on `main`
/// of 2026-10-03 (run 37139408544). Branch-Protection scored 4. There is no
/// release yet, so Signed-Releases is inconclusive (-1) and the release
/// package must raise this floor with the first release. scorecard-action
/// v2.4.4 does not run the SBOM check, so SBOM has no floor until it does.
pub const BASELINE: &[(&str, i32)] = &[("Branch-Protection", 4), ("Signed-Releases", -1)];

/// Compromise-runbook commands, in the order owners run them. `repo` checks
/// their presence and order only (SEC-OPS-072 also asks CI to run them).
pub const COMPROMISE_COMMANDS: &[&str] = &[
    "gunmetal audit verify",
    "gunmetal keys rotate",
    "gunmetal admin devices revoke-all",
    "gunmetal admin users review",
    "gunmetal restore --before COMPROMISE",
    "gunmetal update",
];

/// Status checks the `main` ruleset must require (SEC-SUP-018). The
/// supply-chain and DCO checks SEC-SUP-002 also names join this list once a
/// pull-request job produces them.
pub const REQUIRED_CHECKS: &[&str] = &["actionlint", "codeql", "gate", "zizmor"];

/// Paths that must have a code owner (SEC-SUP-005, SEC-STD-035).
pub const PROTECTED: &[&str] = &[
    ".github/CODEOWNERS",
    ".github/workflows/build.yml",
    ".github/workflows/ci.yml",
    ".github/workflows/release.yml",
    "Cargo.lock",
    "Cargo.toml",
    "SECURITY.md",
    "clippy.toml",
    "compose.yml",
    "crates/gunmetal-core/src/audit_event.rs",
    "crates/gunmetal-core/src/authz/mod.rs",
    "crates/gunmetal-core/src/client_context.rs",
    "crates/gunmetal-core/src/crypto.rs",
    "crates/gunmetal-core/src/formats/mod.rs",
    "crates/gunmetal-core/src/http/mod.rs",
    "crates/gunmetal-core/src/inflate.rs",
    "crates/gunmetal-core/src/lib.rs",
    "crates/gunmetal-core/src/provider/mod.rs",
    "crates/gunmetal-core/src/retention.rs",
    "crates/gunmetal-core/src/tags/mod.rs",
    "crates/gunmetal-core/src/token/mod.rs",
    "crates/gunmetal-core/src/webauthn/mod.rs",
    "crates/gunmetal-durable/src/audit/mod.rs",
    "crates/gunmetal-egress/src/lib.rs",
    "crates/gunmetal-fs/src/sqlite.rs",
    "crates/gunmetal-names/src/lib.rs",
    "crates/gunmetal-plugins/src/lib.rs",
    "crates/gunmetal-secrets/src/lib.rs",
    "crates/gunmetal-server/security/policy.rs",
    "crates/gunmetal-server/src/access/mod.rs",
    "crates/gunmetal-server/src/audit_cli/mod.rs",
    "crates/gunmetal-server/src/audit_sink/mod.rs",
    "crates/gunmetal-server/src/backup/mod.rs",
    "crates/gunmetal-server/src/erasure/mod.rs",
    "crates/gunmetal-server/src/keys/mod.rs",
    "crates/gunmetal-server/src/limiter/mod.rs",
    "crates/gunmetal-server/src/limits/mod.rs",
    "crates/gunmetal-server/src/listener.rs",
    "crates/gunmetal-server/src/oidc/mod.rs",
    "crates/gunmetal-server/src/passkey/mod.rs",
    "crates/gunmetal-server/src/posture/mod.rs",
    "crates/gunmetal-server/src/recovery/mod.rs",
    "crates/gunmetal-server/src/recovery_codes/mod.rs",
    "crates/gunmetal-server/src/restore/mod.rs",
    "crates/gunmetal-server/src/routes.rs",
    "crates/gunmetal-server/src/session/mod.rs",
    "crates/gunmetal-server/src/setup/mod.rs",
    "crates/gunmetal-server/src/shares/mod.rs",
    "crates/gunmetal-server/src/signin/mod.rs",
    "crates/gunmetal-server/src/stream/mod.rs",
    "crates/gunmetal-server/src/users/mod.rs",
    "crates/gunmetal-server/src/verifier/mod.rs",
    "crates/gunmetal-server/tests/route_security/mod.rs",
    "crates/gunmetal-worker/src/sandbox/mod.rs",
    "deny.toml",
    "docs/adr/0001-architecture.md",
    "docs/runbooks/compromise.md",
    "docs/security/README.md",
    "Dockerfile",
    "rust-toolchain.toml",
    "scripts/gate.sh",
    "supply-chain/js-direct-deps.toml",
];

/// `SECURITY.md` headings SEC-SUP-007 requires.
const SECTIONS: &[&str] = &[
    "## Advisories and CVEs",
    "## Coding agents",
    "## Coordinated disclosure",
    "## Reporting a vulnerability",
    "## Scope",
    "## Security roles",
    "## Succession",
    "## Supported versions",
    "## What happens after you report",
];

/// Phrases `SECURITY.md` must contain.
const SECURITY_NEEDLES: &[&str] = &["14 days", "7 days", "90 days", "CVSS v4.0", "SEC-SUP-055"];

/// Role names the policy must list (SEC-STD-036).
const ROLES: &[&str] = &["Incident lead", "Release manager", "Security lead"];

/// Workflows this package owns and the check requires.
const REQUIRED_WORKFLOWS: &[&str] = &[
    ".github/workflows/codeql.yml",
    ".github/workflows/scorecard.yml",
    ".github/workflows/settings-drift.yml",
    ".github/workflows/workflow-lint.yml",
];

/// Extensions treated as committed binaries or archives (SEC-SUP-032).
const BINARY_EXT: &[&str] = &[
    "7z", "a", "bin", "bz2", "class", "deb", "dll", "dylib", "exe", "gz", "jar", "lib", "o", "pyc",
    "rar", "rpm", "so", "tar", "tgz", "war", "wasm", "xz", "zip",
];

/// Something `repo` found wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finding {
    /// A required file is absent.
    Missing {
        /// Path from the repository root.
        path: String,
    },
    /// A file or dump is not UTF-8 JSON or TOML the check can read.
    Unreadable {
        /// Path from the repository root, or the dump's role.
        path: String,
    },
    /// A required heading or phrase is missing.
    MissingSection {
        /// The file.
        path: String,
        /// The heading or phrase.
        section: String,
    },
    /// `Last reviewed` is older than [`YEAR`] or not a date.
    StaleReview {
        /// The file.
        path: String,
        /// The date as written.
        date: String,
    },
    /// A protected path has no code owner.
    Unowned {
        /// The path.
        path: String,
    },
    /// `CODEOWNERS` has a pattern this crate cannot read.
    UnreadableOwners {
        /// The line, counted from 1.
        line: usize,
    },
    /// A `uses:` reference is not pinned to a commit SHA or digest.
    Unpinned {
        /// The workflow.
        path: String,
        /// The `uses:` value.
        uses: String,
    },
    /// The workflow has no top-level `permissions: {}`.
    MissingPermissions {
        /// The workflow.
        path: String,
    },
    /// An `actions/checkout` step without `persist-credentials: false`.
    PersistCredentials {
        /// The workflow.
        path: String,
        /// The step's `uses:` line, counted from 1.
        line: usize,
    },
    /// `pull_request_target` or `workflow_run`.
    DangerousTrigger {
        /// The workflow.
        path: String,
        /// The trigger key.
        trigger: String,
    },
    /// A live setting does not match the expected policy.
    Drift {
        /// The setting's name.
        setting: String,
    },
    /// A Scorecard check is under its threshold.
    ScoreBelow {
        /// The check name.
        check: String,
        /// The recorded score.
        score: i32,
        /// The required score.
        required: i32,
    },
    /// A `CodeQL` result at `error` level or security severity 7.0 or more.
    CodeqlAlert {
        /// The query's rule id.
        rule: String,
        /// The file the result points at, or empty when it names none.
        path: String,
    },
    /// A published advisory has no matching test name.
    UnmappedAdvisory {
        /// The GHSA id.
        id: String,
    },
    /// A compromise-runbook command is missing or out of order.
    MissingRunbookCommand {
        /// The expected command.
        command: String,
    },
    /// A Dependabot ecosystem the tree needs is absent.
    MissingEcosystem {
        /// `cargo`, `github-actions`, `docker` or `npm`.
        name: String,
    },
    /// A Dependabot cooldown shorter than seven days, or none.
    ShortCooldown {
        /// The days written, or 0 when the key is missing.
        days: u64,
    },
    /// A committed executable, archive or object file.
    Binary {
        /// The path.
        path: String,
    },
}

/// Checks committed policy files in `tree` at time `now` (Unix seconds).
pub fn check(tree: &dyn Tree, now: u64) -> Vec<Finding> {
    let mut findings = Vec::new();
    findings.extend(security_md(tree, now));
    findings.extend(owners(tree));
    findings.extend(dependabot(tree));
    findings.extend(workflows(tree));
    findings.extend(reuse(tree));
    findings.extend(runbooks(tree));
    findings.extend(needles(tree));
    findings.extend(binaries(tree));
    findings
}

/// Compares live dumps in `dir` with the expected repository protections.
pub fn settings(tree: &dyn Tree, dir: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    findings.extend(org_settings(tree, dir));
    findings.extend(repo_settings(tree, dir));
    findings.extend(reporting_settings(tree, dir));
    findings.extend(actions_settings(tree, dir));
    findings.extend(release_settings(tree, dir));
    findings.extend(ruleset_settings(tree, dir));
    findings
}

/// Fails when a required Scorecard check in `text` is under its threshold.
pub fn scorecard(text: &str) -> Vec<Finding> {
    let Some(value) = json::parse(text) else {
        return vec![Finding::Unreadable {
            path: "scorecard".to_owned(),
        }];
    };
    let Some(checks) = value.get("checks").and_then(Value::as_array) else {
        return vec![Finding::Unreadable {
            path: "scorecard".to_owned(),
        }];
    };
    let mut findings = Vec::new();
    let mut seen = Vec::new();
    for check in checks {
        let Some(name) = check.get("name").and_then(Value::as_str) else {
            return vec![Finding::Unreadable {
                path: "scorecard".to_owned(),
            }];
        };
        let Some(score) = number(check.get("score")) else {
            return vec![Finding::Unreadable {
                path: "scorecard".to_owned(),
            }];
        };
        seen.push((name.to_owned(), score));
        if let Some(required) = threshold(name) {
            if score < required {
                findings.push(Finding::ScoreBelow {
                    check: name.to_owned(),
                    score,
                    required,
                });
            }
        }
    }
    for name in MUST_BE_TEN {
        if seen.iter().all(|(found, _)| found != name) {
            findings.push(Finding::ScoreBelow {
                check: (*name).to_owned(),
                score: -1,
                required: 10,
            });
        }
    }
    for (name, required) in BASELINE {
        if seen.iter().all(|(found, _)| found != name) {
            findings.push(Finding::ScoreBelow {
                check: (*name).to_owned(),
                score: -1,
                required: *required,
            });
        }
    }
    findings
}

/// Fails on every result in the `CodeQL` SARIF log `text` whose rule has a
/// security severity of 7.0 or more, or whose level is `error`
/// (SEC-SUP-018). A log the check cannot read fails closed.
pub fn codeql(text: &str) -> Vec<Finding> {
    let unreadable = || {
        vec![Finding::Unreadable {
            path: "codeql".to_owned(),
        }]
    };
    let Some(log) = json::parse(text) else {
        return unreadable();
    };
    let Some(runs) = log.get("runs").and_then(Value::as_array) else {
        return unreadable();
    };
    let mut findings = Vec::new();
    for run in runs {
        let rules = sarif_rules(run);
        let Some(results) = run.get("results").and_then(Value::as_array) else {
            return unreadable();
        };
        for result in results {
            let Some(id) = result.get("ruleId").and_then(Value::as_str) else {
                return unreadable();
            };
            let Some(rule) = rules
                .iter()
                .find(|rule| rule.get("id").and_then(Value::as_str) == Some(id))
            else {
                return unreadable();
            };
            match blocks(result, rule) {
                None => return unreadable(),
                Some(false) => {}
                Some(true) => findings.push(Finding::CodeqlAlert {
                    rule: id.to_owned(),
                    path: sarif_path(result),
                }),
            }
        }
    }
    findings
}

/// Fails when a published advisory in `text` has no function named
/// `ghsa_…` after it in the Rust sources of `tree` (SEC-TM-003).
pub fn advisories(tree: &dyn Tree, text: &str) -> Vec<Finding> {
    let Some(items) = advisory_items(text) else {
        return vec![Finding::Unreadable {
            path: "advisories".to_owned(),
        }];
    };
    let mut findings = Vec::new();
    for item in items {
        let Some(id) = item.get("ghsa_id").and_then(Value::as_str) else {
            return vec![Finding::Unreadable {
                path: "advisories".to_owned(),
            }];
        };
        let state = item
            .get("state")
            .and_then(Value::as_str)
            .unwrap_or("published");
        if state != "published" {
            continue;
        }
        let needle = id.to_ascii_lowercase().replace('-', "_");
        if !mapped(tree, &needle) {
            findings.push(Finding::UnmappedAdvisory { id: id.to_owned() });
        }
    }
    findings
}

/// `SECURITY.md` sections, review date, roles, and either a `security@`
/// alias or the owner's recorded decision to defer one. The last is a guard
/// on that decision, not proof of SEC-OPS-064, which stays open until an
/// alias reaching two people exists.
fn security_md(tree: &dyn Tree, now: u64) -> Vec<Finding> {
    let path = "SECURITY.md";
    let Some(text) = tree.read(path) else {
        return vec![Finding::Missing {
            path: path.to_owned(),
        }];
    };
    let mut findings = Vec::new();
    for section in SECTIONS {
        if !text.contains(section) {
            findings.push(Finding::MissingSection {
                path: path.to_owned(),
                section: (*section).to_owned(),
            });
        }
    }
    for needle in SECURITY_NEEDLES {
        if !text.contains(needle) {
            findings.push(Finding::MissingSection {
                path: path.to_owned(),
                section: (*needle).to_owned(),
            });
        }
    }
    for role in ROLES {
        if !text.contains(role) {
            findings.push(Finding::MissingSection {
                path: path.to_owned(),
                section: (*role).to_owned(),
            });
        }
    }
    if !text.contains("security@") && !text.contains("not to run a separate email alias") {
        findings.push(Finding::MissingSection {
            path: path.to_owned(),
            section: "email alias".to_owned(),
        });
    }
    match reviewed_on(&text) {
        None => findings.push(Finding::MissingSection {
            path: path.to_owned(),
            section: "Last reviewed".to_owned(),
        }),
        Some(date) => {
            if unix_ymd(date).is_none_or(|reviewed| now > reviewed.saturating_add(YEAR)) {
                findings.push(Finding::StaleReview {
                    path: path.to_owned(),
                    date: date.to_owned(),
                });
            }
        }
    }
    findings
}

/// Every [`PROTECTED`] path has an owner.
fn owners(tree: &dyn Tree) -> Vec<Finding> {
    let path = ".github/CODEOWNERS";
    let Some(text) = tree.read(path) else {
        return vec![Finding::Missing {
            path: path.to_owned(),
        }];
    };
    let mut findings = Vec::new();
    for protected in PROTECTED {
        match codeowners::owners(&text, protected) {
            Err(line) => {
                return vec![Finding::UnreadableOwners { line }];
            }
            Ok(named) if named.is_empty() => findings.push(Finding::Unowned {
                path: (*protected).to_owned(),
            }),
            Ok(_) => {}
        }
    }
    findings
}

/// Dependabot ecosystems and the seven-day cooldown (SEC-SUP-028).
fn dependabot(tree: &dyn Tree) -> Vec<Finding> {
    let path = ".github/dependabot.yml";
    let Some(text) = tree.read(path) else {
        return vec![Finding::Missing {
            path: path.to_owned(),
        }];
    };
    let mut findings = Vec::new();
    for name in ["cargo", "github-actions"] {
        if !has_ecosystem(&text, name) {
            findings.push(Finding::MissingEcosystem {
                name: name.to_owned(),
            });
        }
    }
    if has_filename(tree, "Dockerfile") && !has_ecosystem(&text, "docker") {
        findings.push(Finding::MissingEcosystem {
            name: "docker".to_owned(),
        });
    }
    if has_filename(tree, "package.json") && !has_ecosystem(&text, "npm") {
        findings.push(Finding::MissingEcosystem {
            name: "npm".to_owned(),
        });
    }
    let days = cooldown_days(&text);
    if days.is_empty() {
        findings.push(Finding::ShortCooldown { days: 0 });
    }
    for day in days {
        if day < 7 {
            findings.push(Finding::ShortCooldown { days: day });
        }
    }
    findings
}

/// Workflow pinning, permissions, checkout credentials and triggers.
fn workflows(tree: &dyn Tree) -> Vec<Finding> {
    let mut findings = Vec::new();
    for path in REQUIRED_WORKFLOWS {
        if tree.read(path).is_none() {
            findings.push(Finding::Missing {
                path: (*path).to_owned(),
            });
        }
    }
    for name in tree.files(".github/workflows") {
        if !(has_ext(&name, "yml") || has_ext(&name, "yaml")) {
            continue;
        }
        let path = format!(".github/workflows/{name}");
        let Some(text) = tree.read(&path) else {
            continue;
        };
        findings.extend(workflow(&path, &text));
    }
    findings
}

/// One workflow file's hardening rules (SEC-SUP-010, SEC-SUP-012, SEC-SUP-013).
fn workflow(path: &str, text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    if !top_permissions(text) {
        findings.push(Finding::MissingPermissions {
            path: path.to_owned(),
        });
    }
    let lines: Vec<&str> = text.lines().map(comment).collect();
    for (index, line) in lines.iter().enumerate() {
        if let Some(spec) = uses_spec(line) {
            if spec.contains("actions/checkout@") && !drops_credentials(&lines, index) {
                findings.push(Finding::PersistCredentials {
                    path: path.to_owned(),
                    line: index.saturating_add(1),
                });
            }
            if !pinned(spec) {
                findings.push(Finding::Unpinned {
                    path: path.to_owned(),
                    uses: spec.to_owned(),
                });
            }
        }
        if let Some(trigger) = dangerous_trigger(line) {
            findings.push(Finding::DangerousTrigger {
                path: path.to_owned(),
                trigger: trigger.to_owned(),
            });
        }
    }
    findings
}

/// Whether the step holding line `at` of `lines` (comments removed) sets
/// `persist-credentials: false`. The step runs from the `- ` that opens it
/// to the next line indented no deeper than that dash.
fn drops_credentials(lines: &[&str], at: usize) -> bool {
    let depth = indent(lines[at]);
    let Some(start) = (0..=at).rev().find(|&index| {
        lines[index].trim_start().starts_with("- ") && (index == at || indent(lines[index]) < depth)
    }) else {
        return false;
    };
    let dash = indent(lines[start]);
    lines
        .iter()
        .skip(start.saturating_add(1))
        .take_while(|line| line.trim().is_empty() || indent(line) > dash)
        .any(|line| line.trim() == "persist-credentials: false")
}

/// Leading spaces on `line`.
fn indent(line: &str) -> usize {
    line.len().saturating_sub(line.trim_start().len())
}

/// REUSE metadata and the licence text (SEC-SUP-030).
fn reuse(tree: &dyn Tree) -> Vec<Finding> {
    let mut findings = Vec::new();
    let path = "REUSE.toml";
    match tree.read(path) {
        None => findings.push(Finding::Missing {
            path: path.to_owned(),
        }),
        Some(text) => {
            for needle in [
                "version = 1",
                "fuzz/seeds/**",
                "AGPL-3.0-or-later",
                "SPDX-FileCopyrightText",
            ] {
                if !text.contains(needle) {
                    findings.push(Finding::MissingSection {
                        path: path.to_owned(),
                        section: needle.to_owned(),
                    });
                }
            }
        }
    }
    let licence = "LICENSES/AGPL-3.0-or-later.txt";
    if tree.read(licence).is_none() {
        findings.push(Finding::Missing {
            path: licence.to_owned(),
        });
    }
    findings
}

/// Runbook files, and the presence and order of the compromise command block.
fn runbooks(tree: &dyn Tree) -> Vec<Finding> {
    let mut findings = Vec::new();
    for path in [
        "docs/runbooks/compromise.md",
        "docs/runbooks/signing-key.md",
        "docs/runbooks/supply-chain.md",
    ] {
        if tree.read(path).is_none() {
            findings.push(Finding::Missing {
                path: path.to_owned(),
            });
        }
    }
    if let Some(text) = tree.read("docs/runbooks/compromise.md") {
        let commands = runbook_block(&text).unwrap_or_default();
        findings.extend(runbook_presence_and_order(&commands));
    }
    findings
}

/// CONTRIBUTING, AGENTS and the pull-request template needles.
fn needles(tree: &dyn Tree) -> Vec<Finding> {
    let mut findings = Vec::new();
    findings.extend(contains(
        tree,
        "CONTRIBUTING.md",
        "docs/security/secure-coding.md",
    ));
    findings.extend(contains(tree, "AGENTS.md", "SEC-SUP-055"));
    let path = ".github/pull_request_template.md";
    match tree.read(path) {
        None => findings.push(Finding::Missing {
            path: path.to_owned(),
        }),
        Some(text) => {
            for needle in ["advisory", "class review", "root cause"] {
                if !text.contains(needle) {
                    findings.push(Finding::MissingSection {
                        path: path.to_owned(),
                        section: needle.to_owned(),
                    });
                }
            }
        }
    }
    findings
}

/// Committed binaries outside `target`, `.git` and fuzz seeds.
fn binaries(tree: &dyn Tree) -> Vec<Finding> {
    tree.files("")
        .into_iter()
        .filter(|path| !skip_binary(path) && is_binary(path))
        .map(|path| Finding::Binary { path })
        .collect()
}

/// Organisation 2FA (SEC-SUP-001).
fn org_settings(tree: &dyn Tree, dir: &str) -> Vec<Finding> {
    match dump(tree, dir, "org.json") {
        Err(finding) => vec![finding],
        Ok(value) => require_true(
            &value,
            "two_factor_requirement_enabled",
            "two_factor_requirement_enabled",
        ),
    }
}

/// Secret-scanning push protection (SEC-SUP-006).
fn repo_settings(tree: &dyn Tree, dir: &str) -> Vec<Finding> {
    match dump(tree, dir, "repo.json") {
        Err(finding) => vec![finding],
        Ok(value) => {
            let mut findings = Vec::new();
            if !status_enabled(
                &value,
                &["security_and_analysis", "secret_scanning", "status"],
            ) {
                findings.push(Finding::Drift {
                    setting: "secret_scanning".to_owned(),
                });
            }
            if !status_enabled(
                &value,
                &[
                    "security_and_analysis",
                    "secret_scanning_push_protection",
                    "status",
                ],
            ) {
                findings.push(Finding::Drift {
                    setting: "secret_scanning_push_protection".to_owned(),
                });
            }
            findings
        }
    }
}

/// Private vulnerability reporting (SEC-SUP-007).
fn reporting_settings(tree: &dyn Tree, dir: &str) -> Vec<Finding> {
    match dump(tree, dir, "private-vulnerability-reporting.json") {
        Err(finding) => vec![finding],
        Ok(value) => require_true(&value, "enabled", "private_vulnerability_reporting"),
    }
}

/// Actions SHA-pinning policy (SEC-SUP-010).
fn actions_settings(tree: &dyn Tree, dir: &str) -> Vec<Finding> {
    match dump(tree, dir, "actions-permissions.json") {
        Err(finding) => vec![finding],
        Ok(value) => require_true(&value, "sha_pinning_required", "sha_pinning_required"),
    }
}

/// Immutable releases (SEC-SUP-003).
fn release_settings(tree: &dyn Tree, dir: &str) -> Vec<Finding> {
    match dump(tree, dir, "immutable-releases.json") {
        Err(finding) => vec![finding],
        Ok(value) => require_true(&value, "enabled", "immutable_releases"),
    }
}

/// Branch and tag rulesets (SEC-SUP-003, SEC-SUP-004, SEC-SUP-005, SEC-SUP-018).
fn ruleset_settings(tree: &dyn Tree, dir: &str) -> Vec<Finding> {
    let mut rulesets = Vec::new();
    for name in tree.files(dir) {
        if name.starts_with("ruleset-") && has_ext(&name, "json") {
            match dump(tree, dir, &name) {
                Err(finding) => return vec![finding],
                Ok(value) => rulesets.push(value),
            }
        }
    }
    let mut findings = Vec::new();
    findings.extend(main_ruleset(&rulesets));
    findings.extend(tag_ruleset(&rulesets));
    findings
}

/// The active `main` ruleset's required rules.
fn main_ruleset(rulesets: &[Value]) -> Vec<Finding> {
    let Some(ruleset) = rulesets
        .iter()
        .find(|value| covers(value, "refs/heads/main") || covers(value, "~DEFAULT_BRANCH"))
    else {
        return vec![Finding::Drift {
            setting: "ruleset.main".to_owned(),
        }];
    };
    let mut findings = Vec::new();
    if !active(ruleset) {
        findings.push(Finding::Drift {
            setting: "ruleset.main.enforcement".to_owned(),
        });
    }
    for rule in ["deletion", "non_fast_forward", "required_signatures"] {
        if !has_rule(ruleset, rule) {
            findings.push(Finding::Drift {
                setting: format!("ruleset.main.{rule}"),
            });
        }
    }
    if !code_owner_review(ruleset) {
        findings.push(Finding::Drift {
            setting: "ruleset.main.require_code_owner_review".to_owned(),
        });
    }
    for context in REQUIRED_CHECKS {
        if !required_check(ruleset, context) {
            findings.push(Finding::Drift {
                setting: format!("ruleset.main.required_status_checks.{context}"),
            });
        }
    }
    if !bypass_actors(ruleset).is_some_and(<[Value]>::is_empty) {
        findings.push(Finding::Drift {
            setting: "ruleset.main.bypass_actors".to_owned(),
        });
    }
    findings
}

/// The active rulesets covering `v*` tags (SEC-SUP-003). A bypass list
/// covers every rule in its ruleset, so the rules live in separate rulesets:
/// `deletion` and `update` each in one nobody can bypass, and `creation` in
/// one only maintainers can bypass. Rulesets layer, so a second copy of
/// `deletion` or `update` beside `creation` does no harm. One ruleset holding
/// all three is drift: with a bypass list its actors can move and delete
/// release tags, and without one nobody can create them.
fn tag_ruleset(rulesets: &[Value]) -> Vec<Finding> {
    let covering: Vec<&Value> = rulesets
        .iter()
        .filter(|ruleset| covers(ruleset, "refs/tags/v*"))
        .collect();
    if covering.is_empty() {
        return vec![Finding::Drift {
            setting: "ruleset.tag".to_owned(),
        }];
    }
    let mut findings = Vec::new();
    let enforced: Vec<&Value> = covering
        .iter()
        .copied()
        .filter(|ruleset| active(ruleset))
        .collect();
    if enforced.len() != covering.len() {
        findings.push(Finding::Drift {
            setting: "ruleset.tag.enforcement".to_owned(),
        });
    }
    let mut bypassable = false;
    for rule in ["creation", "deletion", "update"] {
        let mut holders = enforced
            .iter()
            .filter(|ruleset| has_rule(ruleset, rule))
            .peekable();
        if holders.peek().is_none() {
            findings.push(Finding::Drift {
                setting: format!("ruleset.tag.{rule}"),
            });
        } else if !holders
            .any(|ruleset| bypass_actors(ruleset).is_some_and(|actors| bypass_suits(rule, actors)))
        {
            bypassable = true;
        }
    }
    if bypassable {
        findings.push(Finding::Drift {
            setting: "ruleset.tag.bypass_actors".to_owned(),
        });
    }
    findings
}

/// Whether `ruleset` is enforced.
fn active(ruleset: &Value) -> bool {
    ruleset.get("enforcement").and_then(Value::as_str) == Some("active")
}

/// The ruleset's bypass list, or `None` when the dump has none to read.
fn bypass_actors(ruleset: &Value) -> Option<&[Value]> {
    ruleset.get("bypass_actors").and_then(Value::as_array)
}

/// Whether the bypass list `actors` suits a tag ruleset holding `rule`:
/// somebody, and only maintainers, for `creation`, so maintainers alone
/// create release tags; nobody for `deletion` and `update` (SEC-SUP-003).
fn bypass_suits(rule: &str, actors: &[Value]) -> bool {
    if rule == "creation" {
        !actors.is_empty() && actors.iter().all(maintainer)
    } else {
        actors.is_empty()
    }
}

/// Whether a bypass `actor` is an organisation owner or GitHub's built-in
/// maintain (2) or admin (5) repository role.
fn maintainer(actor: &Value) -> bool {
    match actor.get("actor_type").and_then(Value::as_str) {
        Some("OrganizationAdmin") => true,
        Some("RepositoryRole") => matches!(
            actor.get("actor_id"),
            Some(Value::Number(id)) if id == "2" || id == "5"
        ),
        _ => false,
    }
}

/// Commands listed in a ` ```runbook ` fence, in order.
pub fn runbook_block(text: &str) -> Option<Vec<&str>> {
    let rest = text.split("```runbook").nth(1)?;
    let body = rest.split_once("```").map_or(rest, |(body, _)| body);
    Some(
        body.lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect(),
    )
}

/// Checks the presence and order of `commands` against
/// [`COMPROMISE_COMMANDS`]. Nothing is executed: running the commands
/// against a test server waits for the package that ships the CLI.
pub fn runbook_presence_and_order(commands: &[&str]) -> Vec<Finding> {
    match commands.get(..COMPROMISE_COMMANDS.len()) {
        Some(head) if head == COMPROMISE_COMMANDS => commands
            .get(COMPROMISE_COMMANDS.len())
            .map(|extra| Finding::MissingRunbookCommand {
                command: (*extra).to_owned(),
            })
            .into_iter()
            .collect(),
        _ => COMPROMISE_COMMANDS
            .iter()
            .enumerate()
            .filter(|(index, expected)| commands.get(*index) != Some(expected))
            .map(|(_, expected)| Finding::MissingRunbookCommand {
                command: (*expected).to_owned(),
            })
            .collect(),
    }
}

/// JSON dump `{dir}/{name}`.
fn dump(tree: &dyn Tree, dir: &str, name: &str) -> Result<Value, Finding> {
    let path = join(dir, name);
    let Some(text) = tree.read(&path) else {
        return Err(Finding::Unreadable { path });
    };
    json::parse(&text).ok_or(Finding::Unreadable { path })
}

/// `key` is JSON `true`, else [`Finding::Drift`] named `setting`.
fn require_true(value: &Value, key: &str, setting: &str) -> Vec<Finding> {
    if value.get(key) == Some(&Value::Bool(true)) {
        Vec::new()
    } else {
        vec![Finding::Drift {
            setting: setting.to_owned(),
        }]
    }
}

/// Nested `status` fields equal `"enabled"`.
fn status_enabled(value: &Value, keys: &[&str]) -> bool {
    let mut current = value;
    for key in keys {
        current = match current.get(key) {
            Some(next) => next,
            None => return false,
        };
    }
    current.as_str() == Some("enabled")
}

/// Whether `ruleset` include-patterns contain `wanted`.
fn covers(ruleset: &Value, wanted: &str) -> bool {
    ruleset
        .get("conditions")
        .and_then(|conditions| conditions.get("ref_name"))
        .and_then(|names| names.get("include"))
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(wanted)))
}

/// Whether `ruleset` has a rule of `rule_type`.
fn has_rule(ruleset: &Value, rule_type: &str) -> bool {
    ruleset
        .get("rules")
        .and_then(Value::as_array)
        .is_some_and(|rules| {
            rules
                .iter()
                .any(|rule| rule.get("type").and_then(Value::as_str) == Some(rule_type))
        })
}

/// Pull-request rule with `require_code_owner_review`.
fn code_owner_review(ruleset: &Value) -> bool {
    ruleset
        .get("rules")
        .and_then(Value::as_array)
        .is_some_and(|rules| {
            rules.iter().any(|rule| {
                rule.get("type").and_then(Value::as_str) == Some("pull_request")
                    && rule
                        .get("parameters")
                        .and_then(|parameters| parameters.get("require_code_owner_review"))
                        == Some(&Value::Bool(true))
            })
        })
}

/// Required status checks include `context`.
fn required_check(ruleset: &Value, context: &str) -> bool {
    let Some(rules) = ruleset.get("rules").and_then(Value::as_array) else {
        return false;
    };
    rules.iter().any(|rule| {
        rule.get("type").and_then(Value::as_str) == Some("required_status_checks")
            && rule
                .get("parameters")
                .and_then(|parameters| parameters.get("required_status_checks"))
                .and_then(Value::as_array)
                .is_some_and(|checks| {
                    checks
                        .iter()
                        .any(|check| check.get("context").and_then(Value::as_str) == Some(context))
                })
    })
}

/// Scorecard JSON number as `i32`.
fn number(value: Option<&Value>) -> Option<i32> {
    match value {
        Some(Value::Number(text)) => text.parse().ok(),
        _ => None,
    }
}

/// Required score for `name`, if it is a gated check.
fn threshold(name: &str) -> Option<i32> {
    if MUST_BE_TEN.contains(&name) {
        Some(10)
    } else {
        BASELINE
            .iter()
            .find(|(check, _)| *check == name)
            .map(|(_, required)| *required)
    }
}

/// Rules of a SARIF `run`: the driver's and every extension pack's.
fn sarif_rules(run: &Value) -> Vec<&Value> {
    let tool = run.get("tool");
    let extensions = tool
        .and_then(|tool| tool.get("extensions"))
        .and_then(Value::as_array)
        .unwrap_or_default();
    tool.and_then(|tool| tool.get("driver"))
        .into_iter()
        .chain(extensions)
        .filter_map(|component| component.get("rules").and_then(Value::as_array))
        .flatten()
        .collect()
}

/// Whether `result` of `rule` blocks a merge: its rule's security severity
/// is 7.0 or more, or its level (the rule's default when the result names
/// none) is `error`. `None` when the severity is not a finite number.
fn blocks(result: &Value, rule: &Value) -> Option<bool> {
    let severity = match rule
        .get("properties")
        .and_then(|properties| properties.get("security-severity"))
    {
        None => 0.0,
        Some(Value::String(text) | Value::Number(text)) => text
            .parse::<f64>()
            .ok()
            .filter(|severity| severity.is_finite())?,
        Some(_) => return None,
    };
    let level = result
        .get("level")
        .or_else(|| {
            rule.get("defaultConfiguration")
                .and_then(|configuration| configuration.get("level"))
        })
        .and_then(Value::as_str);
    Some(severity >= 7.0 || level == Some("error"))
}

/// The file a SARIF `result` first points at, or empty.
fn sarif_path(result: &Value) -> String {
    result
        .get("locations")
        .and_then(Value::as_array)
        .and_then(<[Value]>::first)
        .and_then(|location| location.get("physicalLocation"))
        .and_then(|physical| physical.get("artifactLocation"))
        .and_then(|artifact| artifact.get("uri"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// Published-advisory objects from a list, a list of pages (`gh api
/// --paginate --slurp`), `{ "advisories": [...] }`, or one object.
fn advisory_items(text: &str) -> Option<Vec<Value>> {
    let value = json::parse(text)?;
    match &value {
        Value::Array(pages)
            if pages
                .first()
                .is_some_and(|page| matches!(page, Value::Array(_))) =>
        {
            let mut items = Vec::new();
            for page in pages {
                items.extend(page.as_array()?.iter().cloned());
            }
            Some(items)
        }
        Value::Array(items) => Some(items.clone()),
        Value::Object(_) => {
            if let Some(Value::Array(items)) = value.get("advisories") {
                Some(items.clone())
            } else if value.get("ghsa_id").is_some() {
                Some(vec![value])
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Whether any Rust source declares a function whose name starts with
/// `needle`: a line that opens with `fn` and the name. The ID in a comment,
/// a string or the middle of another name is not a regression test.
fn mapped(tree: &dyn Tree, needle: &str) -> bool {
    let declaration = format!("fn {needle}");
    tree.files("").into_iter().any(|path| {
        is_rust(&path)
            && !generated(&path)
            && tree.read(&path).is_some_and(|text| {
                text.lines()
                    .any(|line| line.trim_start().starts_with(&declaration))
            })
    })
}

/// `Last reviewed: YYYY-MM-DD` as the ten-character date.
fn reviewed_on(text: &str) -> Option<&str> {
    let rest = text.split("Last reviewed:").nth(1)?.trim_start();
    let date = rest.get(..10)?;
    (date.as_bytes()[4] == b'-' && date.as_bytes()[7] == b'-').then_some(date)
}

/// Unix seconds at `YYYY-MM-DD` midnight UTC, or `None` when the date is impossible.
fn unix_ymd(date: &str) -> Option<u64> {
    let (year, rest) = date.split_once('-')?;
    let (month, day) = rest.split_once('-')?;
    let year: u64 = year.parse().ok()?;
    let month: u64 = month.parse().ok()?;
    let day: u64 = day.parse().ok()?;
    days_since_epoch(year, month, day).map(|days| days.saturating_mul(86_400))
}

/// Whole days from 1970-01-01 to `year-month-day`.
fn days_since_epoch(year: u64, month: u64, day: u64) -> Option<u64> {
    if year < 1970 || !(1..=12).contains(&month) {
        return None;
    }
    let leap = is_leap(year);
    let max = days_in(month, leap);
    if day < 1 || day > max {
        return None;
    }
    let mut days = 0_u64;
    for previous in 1970..year {
        days = days.saturating_add(365);
        if is_leap(previous) {
            days = days.saturating_add(1);
        }
    }
    for previous in 1..month {
        days = days.saturating_add(match previous {
            2 if leap => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        });
    }
    Some(days.saturating_add(day.saturating_sub(1)))
}

/// Gregorian leap year.
fn is_leap(year: u64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// Days in `month` of a year that is `leap`, or 0 when `month` is not 1..=12.
fn days_in(month: u64, leap: bool) -> u64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    }
}

/// Whether `text` names Dependabot ecosystem `name`.
fn has_ecosystem(text: &str, name: &str) -> bool {
    text.lines().any(|line| {
        let trimmed = comment(line).trim();
        let rest = trimmed.strip_prefix("- ").map_or(trimmed, str::trim);
        rest.strip_prefix("package-ecosystem:")
            .is_some_and(|rest| rest.trim().trim_matches('"') == name)
    })
}

/// `default-days` values in Dependabot YAML.
fn cooldown_days(text: &str) -> Vec<u64> {
    text.lines()
        .filter_map(|line| {
            comment(line)
                .trim()
                .strip_prefix("default-days:")
                .and_then(|rest| rest.trim().parse().ok())
        })
        .collect()
}

/// Whether `tree` has a file whose last component is `name`.
fn has_filename(tree: &dyn Tree, name: &str) -> bool {
    tree.files("")
        .iter()
        .any(|path| !generated(path) && path.rsplit('/').next() == Some(name))
}

/// YAML `uses:` value on `line`, ignoring comments and quoted canaries.
fn uses_spec(line: &str) -> Option<&str> {
    let trimmed = comment(line).trim_start();
    let rest = trimmed.strip_prefix("- ").map_or(trimmed, str::trim_start);
    rest.strip_prefix("uses:")
        .map(str::trim)
        .filter(|spec| !spec.is_empty())
}

/// Whether `spec` is a commit SHA, an image digest, or an in-repo action.
fn pinned(spec: &str) -> bool {
    if spec.starts_with("./") {
        return true;
    }
    let Some((_, rev)) = spec.rsplit_once('@') else {
        return false;
    };
    if let Some(digest) = rev.strip_prefix("sha256:") {
        return digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit());
    }
    rev.len() == 40 && rev.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// `pull_request_target` or `workflow_run` keys.
fn dangerous_trigger(line: &str) -> Option<&str> {
    let trimmed = comment(line).trim();
    let trimmed = trimmed.strip_prefix('-').map_or(trimmed, str::trim);
    let trimmed = trimmed.strip_prefix("on:").map_or(trimmed, str::trim);
    let items: Vec<&str> = trimmed
        .strip_prefix('[')
        .and_then(|list| list.strip_suffix(']'))
        .map_or_else(|| vec![trimmed], |list| list.split(',').collect());
    ["pull_request_target", "workflow_run"]
        .into_iter()
        .find(|trigger| {
            items.iter().any(|item| {
                let item = item.trim().trim_matches(['"', '\'']);
                item == *trigger
                    || item
                        .strip_prefix(trigger)
                        .is_some_and(|rest| rest.starts_with(':'))
            })
        })
}

/// Top-level `permissions: {}` (SEC-SUP-012).
fn top_permissions(text: &str) -> bool {
    text.lines().any(|line| {
        let trimmed = comment(line);
        !trimmed.starts_with(' ')
            && !trimmed.starts_with('\t')
            && matches!(
                trimmed.trim(),
                "permissions: {}" | "permissions:{}" | "permissions: { }"
            )
    })
}

/// `line` without a trailing `#` comment; a whole-line comment is empty.
fn comment(line: &str) -> &str {
    match line.find('#') {
        Some(at) if line[..at].trim().is_empty() => "",
        Some(at) => &line[..at],
        None => line,
    }
}

/// `dir/name`, or `name` when `dir` is empty.
fn join(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_owned()
    } else {
        format!("{dir}/{name}")
    }
}

/// `target`, `.git` or `node_modules` in the path.
fn generated(path: &str) -> bool {
    path.split('/')
        .any(|part| matches!(part, "target" | ".git" | "node_modules"))
}

/// Skip generated trees and fuzz seeds for the binary scan.
fn skip_binary(path: &str) -> bool {
    generated(path) || path.starts_with("fuzz/seeds/")
}

/// Whether `path`'s extension is a committed binary or archive.
fn is_binary(path: &str) -> bool {
    path.rsplit('/')
        .next()
        .and_then(|name| name.rsplit_once('.'))
        .is_some_and(|(_, ext)| BINARY_EXT.contains(&ext))
}

/// Whether `name` has extension `ext`, ignoring ASCII case.
fn has_ext(name: &str, ext: &str) -> bool {
    name.rsplit_once('.')
        .is_some_and(|(_, found)| found.eq_ignore_ascii_case(ext))
}

/// Missing file, or missing `needle` in it.
fn contains(tree: &dyn Tree, path: &str, needle: &str) -> Vec<Finding> {
    match tree.read(path) {
        None => vec![Finding::Missing {
            path: path.to_owned(),
        }],
        Some(text) if text.contains(needle) => Vec::new(),
        Some(_) => vec![Finding::MissingSection {
            path: path.to_owned(),
            section: needle.to_owned(),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BASELINE, BINARY_EXT, COMPROMISE_COMMANDS, Finding, MUST_BE_TEN, PROTECTED, YEAR,
        advisories, check, codeql, comment, cooldown_days, days_in, days_since_epoch,
        has_ecosystem, is_binary, is_leap, pinned, runbook_block, runbook_presence_and_order,
        scorecard, settings, skip_binary, unix_ymd, uses_spec,
    };
    use crate::ROOT;
    use crate::tree::memory::Memory;
    use crate::tree::{Disk, Tree};

    /// Whether `finding` is a stale `SECURITY.md` review.
    fn is_stale(finding: &Finding) -> bool {
        matches!(finding, Finding::StaleReview { .. })
    }

    /// Whether `finding` is the deferred email-alias section.
    fn is_email_alias(finding: &Finding) -> bool {
        matches!(finding, Finding::MissingSection { section, .. } if section == "email alias")
    }

    /// Whether `finding` names a committed binary.
    fn is_binary_finding(finding: &Finding) -> bool {
        matches!(finding, Finding::Binary { .. })
    }

    /// Whether `finding` is drift on the `main` ruleset.
    fn is_main_ruleset_drift(finding: &Finding) -> bool {
        matches!(finding, Finding::Drift { setting } if setting.starts_with("ruleset.main"))
    }

    /// Whether `finding` is an unpinned action or leftover checkout credentials.
    fn is_pinning(finding: &Finding) -> bool {
        matches!(
            finding,
            Finding::Unpinned { .. } | Finding::PersistCredentials { .. }
        )
    }

    /// Whether `finding` is a Dependabot ecosystem or cooldown problem.
    fn is_dependabot(finding: &Finding) -> bool {
        matches!(
            finding,
            Finding::MissingEcosystem { .. } | Finding::ShortCooldown { .. }
        )
    }

    /// `Last reviewed: 2026-10-03` as Unix seconds.
    const REVIEWED: u64 = 1_790_985_600;

    /// A SHA-pinned checkout step.
    const PINNED_SHA: &str = "3d3c42e5aac5ba805825da76410c181273ba90b1";

    /// Minimal `SECURITY.md` that satisfies the section check.
    const SECURITY: &str = "\
Last reviewed: 2026-10-03.

## Reporting a vulnerability
GitHub private vulnerability reporting. The owner decided not to run a separate email alias.

## Scope
This repository.

## Supported versions
None yet.

## What happens after you report
Acknowledge within 7 days, triage within 14 days, fix within 90 days.

## Coordinated disclosure
Embargo and credit.

## Advisories and CVEs
CVSS v4.0 and a CVE.

## Security roles
Security lead @itz4blitz
Release manager @itz4blitz
Incident lead @itz4blitz

## Succession
30 days.

## Coding agents
Without release credentials (SEC-SUP-055).
";

    /// Dependabot with cargo, github-actions and a seven-day cooldown.
    const DEPENDABOT: &str = "\
version: 2
updates:
  - package-ecosystem: cargo
    directory: /
    schedule:
      interval: weekly
    cooldown:
      default-days: 7
  - package-ecosystem: github-actions
    directory: /
    schedule:
      interval: weekly
    cooldown:
      default-days: 7
";

    /// A hardened workflow.
    fn workflow_text() -> String {
        format!(
            "\
name: ci
on: push
permissions: {{}}
jobs:
  gate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@{PINNED_SHA}
        with:
          persist-credentials: false
"
        )
    }

    /// Compromise runbook with the required command block.
    const COMPROMISE: &str = "\
# Compromise

```runbook
gunmetal audit verify
gunmetal keys rotate
gunmetal admin devices revoke-all
gunmetal admin users review
gunmetal restore --before COMPROMISE
gunmetal update
```
";

    /// REUSE catch-all plus fuzz seeds.
    const REUSE: &str = "\
version = 1
path = [\"**\"]
SPDX-FileCopyrightText = \"2026 Justin Scroggins\"
SPDX-License-Identifier = \"AGPL-3.0-or-later\"
path = [\"fuzz/seeds/**\"]
";

    /// Pull-request template needles.
    const TEMPLATE: &str = "advisory ID, class review, root cause\n";

    /// CODEOWNERS covering every protected path.
    fn owners_text() -> String {
        let mut text = String::new();
        for path in PROTECTED {
            text.push_str(path);
            text.push_str(" @itz4blitz\n");
        }
        text
    }

    /// A tree that passes `check` at [`REVIEWED`].
    fn passing() -> Memory {
        let workflow = workflow_text();
        let mut tree = Memory::default()
            .with("SECURITY.md", SECURITY)
            .with(".github/CODEOWNERS", &owners_text())
            .with(".github/dependabot.yml", DEPENDABOT)
            .with(".github/pull_request_template.md", TEMPLATE)
            .with("REUSE.toml", REUSE)
            .with("LICENSES/AGPL-3.0-or-later.txt", "GNU")
            .with("docs/runbooks/compromise.md", COMPROMISE)
            .with("docs/runbooks/signing-key.md", "key")
            .with("docs/runbooks/supply-chain.md", "supply")
            .with("CONTRIBUTING.md", "see docs/security/secure-coding.md\n")
            .with("AGENTS.md", "agents (SEC-SUP-055)\n");
        for path in [
            ".github/workflows/codeql.yml",
            ".github/workflows/scorecard.yml",
            ".github/workflows/settings-drift.yml",
            ".github/workflows/workflow-lint.yml",
        ] {
            tree = tree.with(path, &workflow);
        }
        tree
    }

    /// A passing Scorecard JSON with every gated check at its floor.
    fn scorecard_ok() -> String {
        let mut checks = Vec::new();
        for name in MUST_BE_TEN {
            checks.push(format!(r#"{{"name":"{name}","score":10}}"#));
        }
        for (name, score) in BASELINE {
            checks.push(format!(r#"{{"name":"{name}","score":{score}}}"#));
        }
        format!("{{\"checks\":[{}]}}", checks.join(","))
    }

    /// Live settings dumps that match the policy.
    fn live_ok() -> Memory {
        live_without_immutable_releases()
            .with("live/immutable-releases.json", r#"{"enabled":true}"#)
    }

    /// [`live_ok`] without the immutable-releases dump.
    fn live_without_immutable_releases() -> Memory {
        Memory::default()
            .with("live/org.json", r#"{"two_factor_requirement_enabled":true}"#)
            .with(
                "live/repo.json",
                r#"{"security_and_analysis":{"secret_scanning":{"status":"enabled"},"secret_scanning_push_protection":{"status":"enabled"}}}"#,
            )
            .with(
                "live/private-vulnerability-reporting.json",
                r#"{"enabled":true}"#,
            )
            .with(
                "live/actions-permissions.json",
                r#"{"sha_pinning_required":true}"#,
            )
            .with(
                "live/ruleset-1.json",
                r#"{"enforcement":"active","conditions":{"ref_name":{"include":["refs/heads/main"]}},"rules":[{"type":"deletion"},{"type":"non_fast_forward"},{"type":"required_signatures"},{"type":"pull_request","parameters":{"require_code_owner_review":true}},{"type":"required_status_checks","parameters":{"required_status_checks":[{"context":"actionlint"},{"context":"codeql"},{"context":"gate"},{"context":"zizmor"}]}}],"bypass_actors":[]}"#,
            )
            .with(
                "live/ruleset-2.json",
                r#"{"enforcement":"active","conditions":{"ref_name":{"include":["refs/tags/v*"]}},"rules":[{"type":"creation"}],"bypass_actors":[{"actor_id":5,"actor_type":"RepositoryRole","bypass_mode":"always"}]}"#,
            )
            .with(
                "live/ruleset-3.json",
                r#"{"enforcement":"active","conditions":{"ref_name":{"include":["refs/tags/v*"]}},"rules":[{"type":"deletion"},{"type":"update"}],"bypass_actors":[]}"#,
            )
    }

    #[test]
    fn a_complete_tree_passes() {
        assert_eq!(check(&passing(), REVIEWED), []);
    }

    /// Verifies: SEC-SUP-007
    #[test]
    fn a_security_policy_holding_only_its_scope_misses_every_other_section() {
        let tree = passing().with("SECURITY.md", "## Scope\n");
        let expected: Vec<Finding> = [
            "## Advisories and CVEs",
            "## Coding agents",
            "## Coordinated disclosure",
            "## Reporting a vulnerability",
            "## Security roles",
            "## Succession",
            "## Supported versions",
            "## What happens after you report",
            "14 days",
            "7 days",
            "90 days",
            "CVSS v4.0",
            "SEC-SUP-055",
            "Incident lead",
            "Release manager",
            "Security lead",
            "email alias",
            "Last reviewed",
        ]
        .into_iter()
        .map(|section| Finding::MissingSection {
            path: "SECURITY.md".to_owned(),
            section: section.to_owned(),
        })
        .collect();
        assert_eq!(check(&tree, REVIEWED), expected);
        let scopeless = passing().with("SECURITY.md", &SECURITY.replace("## Scope", "## Reach"));
        assert_eq!(
            check(&scopeless, REVIEWED),
            [Finding::MissingSection {
                path: "SECURITY.md".to_owned(),
                section: "## Scope".to_owned(),
            }]
        );
    }

    /// Verifies: SEC-SUP-007
    #[test]
    fn a_missing_security_policy_fails() {
        let tree = Memory::default();
        assert!(check(&tree, REVIEWED).contains(&Finding::Missing {
            path: "SECURITY.md".to_owned(),
        }));
    }

    /// Verifies: SEC-SUP-007
    #[test]
    fn a_stale_security_review_fails() {
        assert_eq!(
            check(&passing(), REVIEWED.saturating_add(YEAR).saturating_add(1))
                .into_iter()
                .find(is_stale),
            Some(Finding::StaleReview {
                path: "SECURITY.md".to_owned(),
                date: "2026-10-03".to_owned(),
            })
        );
        assert_eq!(
            check(&passing(), REVIEWED.saturating_add(YEAR))
                .into_iter()
                .find(is_stale),
            None
        );
    }

    /// A guard on the owner's deferral, not proof of SEC-OPS-064: the
    /// requirement asks for an alias that reaches two people, and the
    /// recorded decision not to run one yet passes this check.
    #[test]
    fn an_email_alias_or_the_recorded_deferral_is_required() {
        let text = SECURITY.replace("not to run a separate email alias", "mailbox later");
        let tree = passing().with("SECURITY.md", &text);
        assert_eq!(
            check(&tree, REVIEWED).into_iter().find(is_email_alias),
            Some(Finding::MissingSection {
                path: "SECURITY.md".to_owned(),
                section: "email alias".to_owned(),
            })
        );
        let aliased = text.replace("mailbox later", "security@example.test");
        let tree = passing().with("SECURITY.md", &aliased);
        assert_eq!(
            check(&tree, REVIEWED).into_iter().find(is_email_alias),
            None
        );
    }

    /// Verifies: SEC-SUP-005, SEC-STD-035
    #[test]
    fn a_codeowners_file_that_owns_nothing_leaves_every_protected_path_unowned() {
        let tree = passing().with(".github/CODEOWNERS", "NOTES.md @itz4blitz\n");
        let expected: Vec<Finding> = [
            ".github/CODEOWNERS",
            ".github/workflows/build.yml",
            ".github/workflows/ci.yml",
            ".github/workflows/release.yml",
            "Cargo.lock",
            "Cargo.toml",
            "SECURITY.md",
            "clippy.toml",
            "compose.yml",
            "crates/gunmetal-core/src/audit_event.rs",
            "crates/gunmetal-core/src/authz/mod.rs",
            "crates/gunmetal-core/src/client_context.rs",
            "crates/gunmetal-core/src/crypto.rs",
            "crates/gunmetal-core/src/formats/mod.rs",
            "crates/gunmetal-core/src/http/mod.rs",
            "crates/gunmetal-core/src/inflate.rs",
            "crates/gunmetal-core/src/lib.rs",
            "crates/gunmetal-core/src/provider/mod.rs",
            "crates/gunmetal-core/src/retention.rs",
            "crates/gunmetal-core/src/tags/mod.rs",
            "crates/gunmetal-core/src/token/mod.rs",
            "crates/gunmetal-core/src/webauthn/mod.rs",
            "crates/gunmetal-durable/src/audit/mod.rs",
            "crates/gunmetal-egress/src/lib.rs",
            "crates/gunmetal-fs/src/sqlite.rs",
            "crates/gunmetal-names/src/lib.rs",
            "crates/gunmetal-plugins/src/lib.rs",
            "crates/gunmetal-secrets/src/lib.rs",
            "crates/gunmetal-server/security/policy.rs",
            "crates/gunmetal-server/src/access/mod.rs",
            "crates/gunmetal-server/src/audit_cli/mod.rs",
            "crates/gunmetal-server/src/audit_sink/mod.rs",
            "crates/gunmetal-server/src/backup/mod.rs",
            "crates/gunmetal-server/src/erasure/mod.rs",
            "crates/gunmetal-server/src/keys/mod.rs",
            "crates/gunmetal-server/src/limiter/mod.rs",
            "crates/gunmetal-server/src/limits/mod.rs",
            "crates/gunmetal-server/src/listener.rs",
            "crates/gunmetal-server/src/oidc/mod.rs",
            "crates/gunmetal-server/src/passkey/mod.rs",
            "crates/gunmetal-server/src/posture/mod.rs",
            "crates/gunmetal-server/src/recovery/mod.rs",
            "crates/gunmetal-server/src/recovery_codes/mod.rs",
            "crates/gunmetal-server/src/restore/mod.rs",
            "crates/gunmetal-server/src/routes.rs",
            "crates/gunmetal-server/src/session/mod.rs",
            "crates/gunmetal-server/src/setup/mod.rs",
            "crates/gunmetal-server/src/shares/mod.rs",
            "crates/gunmetal-server/src/signin/mod.rs",
            "crates/gunmetal-server/src/stream/mod.rs",
            "crates/gunmetal-server/src/users/mod.rs",
            "crates/gunmetal-server/src/verifier/mod.rs",
            "crates/gunmetal-server/tests/route_security/mod.rs",
            "crates/gunmetal-worker/src/sandbox/mod.rs",
            "deny.toml",
            "docs/adr/0001-architecture.md",
            "docs/runbooks/compromise.md",
            "docs/security/README.md",
            "Dockerfile",
            "rust-toolchain.toml",
            "scripts/gate.sh",
            "supply-chain/js-direct-deps.toml",
        ]
        .into_iter()
        .map(|path| Finding::Unowned {
            path: path.to_owned(),
        })
        .collect();
        assert_eq!(check(&tree, REVIEWED), expected);
    }

    /// Verifies: SEC-SUP-005
    #[test]
    fn a_wildcard_codeowners_pattern_is_unreadable() {
        let tree = passing().with(".github/CODEOWNERS", "*.md @itz4blitz\n");
        assert_eq!(
            check(&tree, REVIEWED)
                .into_iter()
                .find(|finding| matches!(finding, Finding::UnreadableOwners { .. })),
            Some(Finding::UnreadableOwners { line: 1 })
        );
    }

    /// Verifies: SEC-SUP-010
    #[test]
    fn an_unpinned_action_fails() {
        let text = workflow_text().replace(PINNED_SHA, "v4");
        let tree = passing().with(".github/workflows/codeql.yml", &text);
        assert!(check(&tree, REVIEWED).contains(&Finding::Unpinned {
            path: ".github/workflows/codeql.yml".to_owned(),
            uses: "actions/checkout@v4".to_owned(),
        }));
    }

    /// Verifies: SEC-SUP-012
    #[test]
    fn a_workflow_without_empty_top_level_permissions_fails() {
        let text = workflow_text().replace("permissions: {}", "permissions:\n  contents: read");
        let tree = passing().with(".github/workflows/codeql.yml", &text);
        assert!(
            check(&tree, REVIEWED).contains(&Finding::MissingPermissions {
                path: ".github/workflows/codeql.yml".to_owned(),
            })
        );
    }

    #[test]
    fn checkout_must_drop_credentials() {
        let text =
            workflow_text().replace("persist-credentials: false", "persist-credentials: true");
        let tree = passing().with(".github/workflows/codeql.yml", &text);
        assert!(
            check(&tree, REVIEWED).contains(&Finding::PersistCredentials {
                path: ".github/workflows/codeql.yml".to_owned(),
                line: 8,
            })
        );
    }

    /// Every finding for `text` as `codeql.yml`, next to the
    /// `PersistCredentials` findings expected at `lines`.
    fn credential_lines(text: &str, lines: &[usize]) -> (Vec<Finding>, Vec<Finding>) {
        let found = check(
            &passing().with(".github/workflows/codeql.yml", text),
            REVIEWED,
        );
        let expected = lines
            .iter()
            .map(|&line| Finding::PersistCredentials {
                path: ".github/workflows/codeql.yml".to_owned(),
                line,
            })
            .collect();
        (found, expected)
    }

    #[test]
    fn each_checkout_step_must_drop_its_own_credentials() {
        let second = format!(
            "{}      - name: second
        uses: actions/checkout@{PINNED_SHA}
        with:
          fetch-depth: 0
      - run: echo
        env:
          persist-credentials: false
",
            workflow_text()
        );
        let (found, expected) = credential_lines(&second, &[12]);
        assert_eq!(found, expected);
        let named = format!(
            "{}      - name: second

        # keep the token off the disk
        uses: actions/checkout@{PINNED_SHA}
        with:
          persist-credentials: false
",
            workflow_text()
        );
        let (found, expected) = credential_lines(&named, &[]);
        assert_eq!(found, expected);
        let compact = format!(
            "{}      - name: compact
        labels:
        - kept
        uses: actions/checkout@{PINNED_SHA}
        with:
          persist-credentials: false
",
            workflow_text()
        );
        let (found, expected) = credential_lines(&compact, &[]);
        assert_eq!(found, expected);
        let listed = format!(
            "{}      - uses: actions/checkout@{PINNED_SHA}
        with:
          sparse-checkout: |
            - persist-credentials: false
",
            workflow_text()
        );
        let (found, expected) = credential_lines(&listed, &[11]);
        assert_eq!(found, expected);
        let first_only = workflow_text().replace(
            "      - uses: actions/checkout",
            &format!("      - uses: actions/checkout@{PINNED_SHA}\n      - uses: actions/checkout"),
        );
        let (found, expected) = credential_lines(&first_only, &[8]);
        assert_eq!(found, expected);
        let bare = format!(
            "name: bare\non: push\npermissions: {{}}\njobs:\n  x:\n    uses: actions/checkout@{PINNED_SHA}\n    with:\n      persist-credentials: false\n"
        );
        let (found, expected) = credential_lines(&bare, &[6]);
        assert_eq!(found, expected);
    }

    /// Verifies: SEC-SUP-013
    #[test]
    fn dangerous_triggers_in_a_flow_sequence_fail() {
        for (on, trigger) in [
            (
                "on: [push, pull_request_target]",
                Some("pull_request_target"),
            ),
            ("on: [ \"workflow_run\" ]", Some("workflow_run")),
            ("on: ['pull_request_target']", Some("pull_request_target")),
            ("on: [push, pull_request]", None),
        ] {
            let text = workflow_text().replace("on: push", on);
            let tree = passing().with(".github/workflows/codeql.yml", &text);
            let found: Vec<Finding> = check(&tree, REVIEWED)
                .into_iter()
                .filter(|finding| matches!(finding, Finding::DangerousTrigger { .. }))
                .collect();
            let expected: Vec<Finding> = trigger
                .into_iter()
                .map(|trigger| Finding::DangerousTrigger {
                    path: ".github/workflows/codeql.yml".to_owned(),
                    trigger: trigger.to_owned(),
                })
                .collect();
            assert_eq!(found, expected);
        }
    }

    /// Verifies: SEC-SUP-013
    #[test]
    fn pull_request_target_and_workflow_run_fail() {
        for trigger in ["pull_request_target", "workflow_run"] {
            let text = workflow_text().replace("on: push", &format!("on:\n  {trigger}:"));
            let tree = passing().with(".github/workflows/codeql.yml", &text);
            assert!(check(&tree, REVIEWED).contains(&Finding::DangerousTrigger {
                path: ".github/workflows/codeql.yml".to_owned(),
                trigger: trigger.to_owned(),
            }));
        }
    }

    #[test]
    fn matching_live_settings_pass() {
        assert_eq!(settings(&live_ok(), "live"), []);
    }

    /// Verifies: SEC-SUP-006
    #[test]
    fn secret_scanning_push_protection_off_is_drift() {
        let tree = live_ok().with(
            "live/repo.json",
            r#"{"security_and_analysis":{"secret_scanning":{"status":"enabled"},"secret_scanning_push_protection":{"status":"disabled"}}}"#,
        );
        assert_eq!(
            settings(&tree, "live"),
            drift("secret_scanning_push_protection")
        );
        let tree = live_ok().with(
            "live/repo.json",
            r#"{"security_and_analysis":{"secret_scanning":{"status":"disabled"},"secret_scanning_push_protection":{"status":"enabled"}}}"#,
        );
        assert_eq!(settings(&tree, "live"), drift("secret_scanning"));
        let tree = live_ok().with("live/repo.json", r#"{"security_and_analysis":{}}"#);
        assert_eq!(
            settings(&tree, "live"),
            [
                Finding::Drift {
                    setting: "secret_scanning".to_owned(),
                },
                Finding::Drift {
                    setting: "secret_scanning_push_protection".to_owned(),
                },
            ]
        );
    }

    /// Verifies: SEC-SUP-007
    #[test]
    fn private_vulnerability_reporting_off_is_drift() {
        let tree = live_ok().with(
            "live/private-vulnerability-reporting.json",
            r#"{"enabled":false}"#,
        );
        assert_eq!(
            settings(&tree, "live"),
            drift("private_vulnerability_reporting")
        );
        let tree = live_ok().with(
            "live/private-vulnerability-reporting.json",
            r#"{"message":"Not Found","status":"404"}"#,
        );
        assert_eq!(
            settings(&tree, "live"),
            drift("private_vulnerability_reporting")
        );
    }

    /// Verifies: SEC-SUP-010
    #[test]
    fn an_actions_policy_that_does_not_require_sha_pinning_is_drift() {
        let tree = live_ok().with(
            "live/actions-permissions.json",
            r#"{"sha_pinning_required":false}"#,
        );
        assert_eq!(settings(&tree, "live"), drift("sha_pinning_required"));
        let tree = live_ok().with("live/actions-permissions.json", r#"{"enabled":true}"#);
        assert_eq!(settings(&tree, "live"), drift("sha_pinning_required"));
    }

    /// Verifies: SEC-SUP-005, SEC-SUP-018
    #[test]
    fn a_main_ruleset_with_no_rules_drifts_on_every_expected_rule() {
        let tree = live_ok().with(
            "live/ruleset-1.json",
            r#"{"enforcement":"active","conditions":{"ref_name":{"include":["refs/heads/main"]}},"rules":[]}"#,
        );
        let expected: Vec<Finding> = [
            "ruleset.main.deletion",
            "ruleset.main.non_fast_forward",
            "ruleset.main.required_signatures",
            "ruleset.main.require_code_owner_review",
            "ruleset.main.required_status_checks.actionlint",
            "ruleset.main.required_status_checks.codeql",
            "ruleset.main.required_status_checks.gate",
            "ruleset.main.required_status_checks.zizmor",
            "ruleset.main.bypass_actors",
        ]
        .into_iter()
        .map(|setting| Finding::Drift {
            setting: setting.to_owned(),
        })
        .collect();
        assert_eq!(settings(&tree, "live"), expected);
    }

    /// The `main` ruleset with every rule the policy expects and the
    /// required status checks `checks`.
    fn main_ruleset(checks: &[&str], bypass: &str) -> String {
        let contexts: Vec<String> = checks
            .iter()
            .map(|check| format!(r#"{{"context":"{check}"}}"#))
            .collect();
        format!(
            r#"{{"enforcement":"active","conditions":{{"ref_name":{{"include":["~DEFAULT_BRANCH"]}}}},"rules":[{{"type":"deletion"}},{{"type":"non_fast_forward"}},{{"type":"required_signatures"}},{{"type":"pull_request","parameters":{{"require_code_owner_review":true}}}},{{"type":"required_status_checks","parameters":{{"required_status_checks":[{}]}}}}]{bypass}}}"#,
            contexts.join(",")
        )
    }

    /// Verifies: SEC-SUP-018
    #[test]
    fn the_main_ruleset_must_require_the_gate_codeql_zizmor_and_actionlint() {
        let tree = live_ok().with(
            "live/ruleset-1.json",
            &main_ruleset(&["gate"], r#","bypass_actors":[]"#),
        );
        assert_eq!(
            settings(&tree, "live"),
            [
                Finding::Drift {
                    setting: "ruleset.main.required_status_checks.actionlint".to_owned(),
                },
                Finding::Drift {
                    setting: "ruleset.main.required_status_checks.codeql".to_owned(),
                },
                Finding::Drift {
                    setting: "ruleset.main.required_status_checks.zizmor".to_owned(),
                },
            ]
        );
        let tree = live_ok().with(
            "live/ruleset-1.json",
            &main_ruleset(
                &["zizmor", "gate", "codeql", "actionlint"],
                r#","bypass_actors":[]"#,
            ),
        );
        assert_eq!(settings(&tree, "live"), []);
    }

    #[test]
    fn any_bypass_actor_on_the_main_ruleset_is_drift() {
        let all = ["actionlint", "codeql", "gate", "zizmor"];
        for bypass in [
            r#","bypass_actors":[{"actor_id":5,"actor_type":"RepositoryRole","bypass_mode":"always"}]"#,
            r#","bypass_actors":{}"#,
            "",
        ] {
            let tree = live_ok().with("live/ruleset-1.json", &main_ruleset(&all, bypass));
            assert_eq!(settings(&tree, "live"), drift("ruleset.main.bypass_actors"));
        }
    }

    /// A `v*` tag ruleset dump, `enforcement` as given, holding `rules` and
    /// the `bypass` member.
    fn tag_ruleset_dump(enforcement: &str, rules: &str, bypass: &str) -> String {
        format!(
            r#"{{"enforcement":"{enforcement}","conditions":{{"ref_name":{{"include":["refs/tags/v*"]}}}},"rules":[{rules}]{bypass}}}"#
        )
    }

    /// [`live_ok`] whose two tag rulesets are replaced by `first` and
    /// `second`.
    fn live_with_tag_rulesets(first: &str, second: &str) -> Memory {
        live_ok()
            .with("live/ruleset-2.json", first)
            .with("live/ruleset-3.json", second)
    }

    /// A ruleset for another tag pattern, which the `v*` check ignores.
    const OTHER_TAGS: &str = r#"{"enforcement":"active","conditions":{"ref_name":{"include":["refs/tags/nightly-*"]}},"rules":[{"type":"creation"},{"type":"deletion"},{"type":"update"}],"bypass_actors":[]}"#;

    /// The bypass list holding the repository admin role.
    const ADMIN_BYPASS: &str =
        r#","bypass_actors":[{"actor_id":5,"actor_type":"RepositoryRole","bypass_mode":"always"}]"#;

    /// An empty bypass list.
    const NO_BYPASS: &str = r#","bypass_actors":[]"#;

    /// One drift finding named `setting`.
    fn drift(setting: &str) -> Vec<Finding> {
        vec![Finding::Drift {
            setting: setting.to_owned(),
        }]
    }

    /// Verifies: SEC-SUP-003
    #[test]
    fn a_bypass_list_on_the_ruleset_that_stops_tag_moves_and_deletes_is_drift() {
        let all = r#"{"type":"creation"},{"type":"deletion"},{"type":"update"}"#;
        let single =
            live_with_tag_rulesets(&tag_ruleset_dump("active", all, ADMIN_BYPASS), OTHER_TAGS);
        assert_eq!(
            settings(&single, "live"),
            drift("ruleset.tag.bypass_actors")
        );
        let creation = tag_ruleset_dump("active", r#"{"type":"creation"}"#, ADMIN_BYPASS);
        let protect = r#"{"type":"deletion"},{"type":"update"}"#;
        for bypass in [
            ADMIN_BYPASS,
            r#","bypass_actors":[{"actor_id":1,"actor_type":"OrganizationAdmin"}]"#,
            r#","bypass_actors":{}"#,
            "",
        ] {
            let tree =
                live_with_tag_rulesets(&creation, &tag_ruleset_dump("active", protect, bypass));
            assert_eq!(settings(&tree, "live"), drift("ruleset.tag.bypass_actors"));
        }
        let delete_only = live_with_tag_rulesets(
            &tag_ruleset_dump(
                "active",
                r#"{"type":"creation"},{"type":"update"}"#,
                ADMIN_BYPASS,
            ),
            &tag_ruleset_dump("active", r#"{"type":"deletion"}"#, NO_BYPASS),
        );
        assert_eq!(
            settings(&delete_only, "live"),
            drift("ruleset.tag.bypass_actors")
        );
        let update_only = live_with_tag_rulesets(
            &tag_ruleset_dump(
                "active",
                r#"{"type":"creation"},{"type":"deletion"}"#,
                ADMIN_BYPASS,
            ),
            &tag_ruleset_dump("active", r#"{"type":"update"}"#, NO_BYPASS),
        );
        assert_eq!(
            settings(&update_only, "live"),
            drift("ruleset.tag.bypass_actors")
        );
    }

    /// Verifies: SEC-SUP-003
    #[test]
    fn the_split_tag_layout_passes_and_a_missing_tag_rule_is_drift() {
        assert_eq!(settings(&live_ok(), "live"), []);
        let creation = tag_ruleset_dump("active", r#"{"type":"creation"}"#, ADMIN_BYPASS);
        let layered = live_with_tag_rulesets(
            &tag_ruleset_dump(
                "active",
                r#"{"type":"creation"},{"type":"deletion"},{"type":"update"}"#,
                ADMIN_BYPASS,
            ),
            &tag_ruleset_dump(
                "active",
                r#"{"type":"deletion"},{"type":"update"}"#,
                NO_BYPASS,
            ),
        );
        assert_eq!(settings(&layered, "live"), []);
        let one_rule_each = live_with_tag_rulesets(
            &creation,
            &tag_ruleset_dump("active", r#"{"type":"deletion"}"#, NO_BYPASS),
        )
        .with(
            "live/ruleset-4.json",
            &tag_ruleset_dump("active", r#"{"type":"update"}"#, NO_BYPASS),
        );
        assert_eq!(settings(&one_rule_each, "live"), []);
        for (rules, setting) in [
            (r#"{"type":"update"}"#, "ruleset.tag.deletion"),
            (r#"{"type":"deletion"}"#, "ruleset.tag.update"),
        ] {
            let tree =
                live_with_tag_rulesets(&creation, &tag_ruleset_dump("active", rules, NO_BYPASS));
            assert_eq!(settings(&tree, "live"), drift(setting));
        }
        let no_creation = live_with_tag_rulesets(
            OTHER_TAGS,
            &tag_ruleset_dump(
                "active",
                r#"{"type":"deletion"},{"type":"update"}"#,
                NO_BYPASS,
            ),
        );
        assert_eq!(
            settings(&no_creation, "live"),
            drift("ruleset.tag.creation")
        );
        let none = live_with_tag_rulesets(OTHER_TAGS, OTHER_TAGS);
        assert_eq!(settings(&none, "live"), drift("ruleset.tag"));
    }

    /// Verifies: SEC-SUP-003
    #[test]
    fn only_maintainers_may_bypass_the_tag_creation_rule() {
        let protect = tag_ruleset_dump(
            "active",
            r#"{"type":"deletion"},{"type":"update"}"#,
            NO_BYPASS,
        );
        let with_creation_bypass = |bypass: &str| {
            live_with_tag_rulesets(
                &tag_ruleset_dump("active", r#"{"type":"creation"}"#, bypass),
                &protect,
            )
        };
        for allowed in [
            r#","bypass_actors":[{"actor_id":2,"actor_type":"RepositoryRole"}]"#,
            r#","bypass_actors":[{"actor_id":2,"actor_type":"RepositoryRole"},{"actor_id":5,"actor_type":"RepositoryRole"},{"actor_id":1,"actor_type":"OrganizationAdmin"}]"#,
        ] {
            assert_eq!(settings(&with_creation_bypass(allowed), "live"), []);
        }
        for refused in [
            r#","bypass_actors":[{"actor_id":4,"actor_type":"RepositoryRole"}]"#,
            r#","bypass_actors":[{"actor_id":5,"actor_type":"RepositoryRole"},{"actor_id":4,"actor_type":"RepositoryRole"}]"#,
            r#","bypass_actors":[{"actor_id":"5","actor_type":"RepositoryRole"}]"#,
            r#","bypass_actors":[{"actor_id":5,"actor_type":"Integration"}]"#,
            r#","bypass_actors":[{"actor_id":5}]"#,
            r#","bypass_actors":[]"#,
            r#","bypass_actors":{}"#,
            "",
        ] {
            assert_eq!(
                settings(&with_creation_bypass(refused), "live"),
                drift("ruleset.tag.bypass_actors")
            );
        }
    }

    /// Verifies: SEC-SUP-003
    #[test]
    fn a_tag_ruleset_that_is_not_active_is_drift_and_its_rules_do_not_count() {
        let creation = tag_ruleset_dump("active", r#"{"type":"creation"}"#, ADMIN_BYPASS);
        let protect = r#"{"type":"deletion"},{"type":"update"}"#;
        let tree =
            live_with_tag_rulesets(&creation, &tag_ruleset_dump("evaluate", protect, NO_BYPASS));
        assert_eq!(
            settings(&tree, "live"),
            [
                Finding::Drift {
                    setting: "ruleset.tag.enforcement".to_owned(),
                },
                Finding::Drift {
                    setting: "ruleset.tag.deletion".to_owned(),
                },
                Finding::Drift {
                    setting: "ruleset.tag.update".to_owned(),
                },
            ]
        );
        let spare = live_ok().with(
            "live/ruleset-4.json",
            &tag_ruleset_dump("disabled", protect, NO_BYPASS),
        );
        assert_eq!(settings(&spare, "live"), drift("ruleset.tag.enforcement"));
        let unstated = live_with_tag_rulesets(
            r#"{"conditions":{"ref_name":{"include":["refs/tags/v*"]}},"rules":[{"type":"creation"}],"bypass_actors":[{"actor_id":5,"actor_type":"RepositoryRole"}]}"#,
            &tag_ruleset_dump("active", protect, NO_BYPASS),
        );
        assert_eq!(
            settings(&unstated, "live"),
            [
                Finding::Drift {
                    setting: "ruleset.tag.enforcement".to_owned(),
                },
                Finding::Drift {
                    setting: "ruleset.tag.creation".to_owned(),
                },
            ]
        );
    }

    /// Verifies: SEC-SUP-003
    #[test]
    fn releases_that_are_not_immutable_are_drift() {
        let tree = live_ok().with("live/immutable-releases.json", r#"{"enabled":false}"#);
        assert_eq!(settings(&tree, "live"), drift("immutable_releases"));
        assert_eq!(
            settings(&live_without_immutable_releases(), "live"),
            [Finding::Unreadable {
                path: "live/immutable-releases.json".to_owned(),
            }]
        );
    }

    /// Verifies: SEC-SUP-001
    #[test]
    fn organisation_two_factor_off_is_drift() {
        let tree = live_ok().with(
            "live/org.json",
            r#"{"two_factor_requirement_enabled":false}"#,
        );
        assert_eq!(
            settings(&tree, "live"),
            drift("two_factor_requirement_enabled")
        );
    }

    /// A `CodeQL` SARIF log whose run has `driver` rules, one extension pack
    /// with `extension` rules, and `results`.
    fn sarif(driver: &str, extension: &str, results: &str) -> String {
        format!(
            r#"{{"version":"2.1.0","runs":[{{"tool":{{"driver":{{"name":"CodeQL","rules":[{driver}]}},"extensions":[{{"name":"codeql/rust-queries","rules":[{extension}]}}]}},"results":[{results}]}}]}}"#
        )
    }

    /// A rule `id` with a default `level` and, when not empty, a security
    /// severity written as `severity` (a JSON string or number).
    fn sarif_rule(id: &str, level: &str, severity: &str) -> String {
        let properties = if severity.is_empty() {
            String::new()
        } else {
            format!(r#","properties":{{"security-severity":{severity}}}"#)
        };
        format!(r#"{{"id":"{id}","defaultConfiguration":{{"level":"{level}"}}{properties}}}"#)
    }

    /// A result of rule `id` in `src/lib.rs`, with `level` when not empty.
    fn sarif_result(id: &str, level: &str) -> String {
        let level = if level.is_empty() {
            String::new()
        } else {
            format!(r#""level":"{level}","#)
        };
        format!(
            r#"{{"ruleId":"{id}",{level}"message":{{"text":"m"}},"locations":[{{"physicalLocation":{{"artifactLocation":{{"uri":"src/lib.rs"}},"region":{{"startLine":3}}}}}}]}}"#
        )
    }

    /// The finding for a blocking result of rule `id` in `src/lib.rs`.
    fn alert(id: &str) -> Finding {
        Finding::CodeqlAlert {
            rule: id.to_owned(),
            path: "src/lib.rs".to_owned(),
        }
    }

    /// Verifies: SEC-SUP-018
    #[test]
    fn a_codeql_result_of_high_security_severity_or_error_level_fails() {
        let id = "rust/sql-injection";
        let result = sarif_result(id, "");
        let at = |level: &str, severity: &str| {
            codeql(&sarif("", &sarif_rule(id, level, severity), &result))
        };
        assert_eq!(at("warning", r#""7.0""#), [alert(id)]);
        assert_eq!(at("warning", r#""8.8""#), [alert(id)]);
        assert_eq!(at("warning", r#""6.9""#), []);
        assert_eq!(at("note", ""), []);
        assert_eq!(at("error", ""), [alert(id)]);
        assert_eq!(at("error", r#""2.0""#), [alert(id)]);
        let raised = sarif(
            "",
            &sarif_rule(id, "warning", r#""6.9""#),
            &sarif_result(id, "error"),
        );
        assert_eq!(codeql(&raised), [alert(id)]);
        let lowered = sarif(
            "",
            &sarif_rule(id, "error", r#""6.9""#),
            &sarif_result(id, "warning"),
        );
        assert_eq!(codeql(&lowered), []);
    }

    #[test]
    fn codeql_reads_rules_from_the_driver_and_every_extension_and_reports_each_result() {
        let driver = sarif_rule("actions/a", "warning", r#""9.0""#);
        let extension = sarif_rule("rust/b", "warning", "7.5");
        let results = [
            sarif_result("rust/b", ""),
            sarif_result("actions/a", ""),
            sarif_result("rust/b", "note"),
        ]
        .join(",");
        assert_eq!(
            codeql(&sarif(&driver, &extension, &results)),
            [alert("rust/b"), alert("actions/a"), alert("rust/b")]
        );
        let two_runs = format!(
            r#"{{"runs":[{{"tool":{{"driver":{{"rules":[{driver}]}}}},"results":[]}},{{"tool":{{"driver":{{"rules":[{driver}]}}}},"results":[{{"ruleId":"actions/a"}}]}}]}}"#
        );
        assert_eq!(
            codeql(&two_runs),
            [Finding::CodeqlAlert {
                rule: "actions/a".to_owned(),
                path: String::new(),
            }]
        );
        assert_eq!(codeql(&sarif(&driver, &extension, "")), []);
    }

    #[test]
    fn codeql_fails_closed_on_a_log_it_cannot_read() {
        let unreadable = [Finding::Unreadable {
            path: "codeql".to_owned(),
        }];
        let rule = sarif_rule("rust/a", "warning", r#""5.0""#);
        for text in [
            "not json".to_owned(),
            "{}".to_owned(),
            r#"{"runs":{}}"#.to_owned(),
            r#"{"runs":[{"tool":{"driver":{"rules":[]}}}]}"#.to_owned(),
            sarif("", &rule, r#"{"level":"note"}"#),
            sarif("", &rule, &sarif_result("rust/unknown", "")),
            sarif(
                "",
                &sarif_rule("rust/a", "warning", r#""high""#),
                &sarif_result("rust/a", ""),
            ),
            sarif(
                "",
                &sarif_rule("rust/a", "warning", r#""NaN""#),
                &sarif_result("rust/a", ""),
            ),
            sarif(
                "",
                &sarif_rule("rust/a", "warning", "true"),
                &sarif_result("rust/a", ""),
            ),
            sarif("", &rule, &format!("{},5", sarif_result("rust/a", ""))),
        ] {
            assert_eq!(codeql(&text), unreadable);
        }
    }

    /// Verifies: SEC-SUP-019
    #[test]
    fn a_scorecard_result_under_the_threshold_fails() {
        let text = scorecard_ok().replace(
            r#""name":"Dangerous-Workflow","score":10"#,
            r#""name":"Dangerous-Workflow","score":9"#,
        );
        assert_eq!(
            scorecard(&text),
            [Finding::ScoreBelow {
                check: "Dangerous-Workflow".to_owned(),
                score: 9,
                required: 10,
            }]
        );
        assert_eq!(scorecard(&scorecard_ok()), []);
    }

    /// The Scorecard run on `main` of 2026-10-03 (run 37139408544, SARIF
    /// format). Its results name the checks scored under the action's
    /// policy: Branch-Protection 4, Security-Policy 4, and 0 for SAST,
    /// Code-Review, Maintained and CII-Best-Practices. The other gated
    /// checks have no result, so they are written here as 10; there is no
    /// release, so Signed-Releases is inconclusive (-1); and the run has no
    /// SBOM rule at all.
    const MAIN_2026_10_03: &str = r#"{"checks":[{"name":"Binary-Artifacts","score":10},{"name":"Branch-Protection","score":4},{"name":"CII-Best-Practices","score":0},{"name":"Code-Review","score":0},{"name":"Dangerous-Workflow","score":10},{"name":"Dependency-Update-Tool","score":10},{"name":"License","score":10},{"name":"Maintained","score":0},{"name":"Pinned-Dependencies","score":10},{"name":"SAST","score":0},{"name":"Security-Policy","score":4},{"name":"Signed-Releases","score":-1},{"name":"Token-Permissions","score":10}]}"#;

    /// Verifies: SEC-SUP-019
    #[test]
    fn main_s_recorded_scores_fail_only_on_the_security_policy() {
        assert_eq!(
            scorecard(MAIN_2026_10_03),
            [Finding::ScoreBelow {
                check: "Security-Policy".to_owned(),
                score: 4,
                required: 10,
            }]
        );
    }

    /// [`MAIN_2026_10_03`] with the security policy at 10.
    fn recorded_with_policy_fixed() -> String {
        MAIN_2026_10_03.replace(
            r#""name":"Security-Policy","score":4"#,
            r#""name":"Security-Policy","score":10"#,
        )
    }

    /// Verifies: SEC-SUP-019
    #[test]
    fn branch_protection_under_its_recorded_baseline_fails() {
        assert_eq!(scorecard(&recorded_with_policy_fixed()), []);
        let lower = recorded_with_policy_fixed().replace(
            r#""name":"Branch-Protection","score":4"#,
            r#""name":"Branch-Protection","score":3"#,
        );
        assert_eq!(
            scorecard(&lower),
            [Finding::ScoreBelow {
                check: "Branch-Protection".to_owned(),
                score: 3,
                required: 4,
            }]
        );
    }

    /// Verifies: SEC-SUP-019
    #[test]
    fn a_missing_scorecard_check_fails_closed() {
        assert_eq!(
            scorecard(r#"{"checks":[]}"#).first(),
            Some(&Finding::ScoreBelow {
                check: "Binary-Artifacts".to_owned(),
                score: -1,
                required: 10,
            })
        );
        assert_eq!(
            scorecard("not json"),
            [Finding::Unreadable {
                path: "scorecard".to_owned(),
            }]
        );
    }

    /// Verifies: SEC-TM-003, SEC-OPS-066, SEC-HIS-064
    #[test]
    fn an_advisory_with_no_matching_test_fails() {
        let id = "GHSA-aaaa-bbbb-cccc";
        let json = r#"[{"ghsa_id":"GHSA-aaaa-bbbb-cccc","state":"published"}]"#;
        assert_eq!(
            advisories(&passing(), json),
            [Finding::UnmappedAdvisory { id: id.to_owned() }]
        );
        let tree = passing().with(
            "crates/demo/tests/advisory.rs",
            "#[test]\nfn ghsa_aaaa_bbbb_cccc_rejects() {}\n",
        );
        assert_eq!(advisories(&tree, json), []);
        let nested = passing().with(
            "crates/demo/src/lib.rs",
            "mod tests {\n    #[test]\n    fn ghsa_aaaa_bbbb_cccc_rejects() {}\n}\n",
        );
        assert_eq!(advisories(&nested, json), []);
    }

    /// Verifies: SEC-TM-003
    #[test]
    fn an_advisory_id_in_a_comment_or_a_string_is_not_a_regression_test() {
        let json = r#"[{"ghsa_id":"GHSA-aaaa-bbbb-cccc","state":"published"}]"#;
        for text in [
            "// ghsa_aaaa_bbbb_cccc is fixed\n",
            "// fn ghsa_aaaa_bbbb_cccc_rejects() {}\n",
            "const ID: &str = \"ghsa_aaaa_bbbb_cccc\";\n",
            "const ID: &str = \"fn ghsa_aaaa_bbbb_cccc\";\n",
            "fn not_ghsa_aaaa_bbbb_cccc() {}\n",
            "fn ghsa_aaaa_bbbb() {}\n",
        ] {
            let tree = passing().with("crates/demo/tests/advisory.rs", text);
            assert_eq!(
                advisories(&tree, json),
                [Finding::UnmappedAdvisory {
                    id: "GHSA-aaaa-bbbb-cccc".to_owned(),
                }]
            );
        }
        let markdown = passing().with("docs/advisory.md", "fn ghsa_aaaa_bbbb_cccc_rejects() {}\n");
        assert_eq!(
            advisories(&markdown, json),
            [Finding::UnmappedAdvisory {
                id: "GHSA-aaaa-bbbb-cccc".to_owned(),
            }]
        );
    }

    /// Verifies: SEC-TM-003
    #[test]
    fn advisories_on_every_page_of_a_slurped_listing_are_checked() {
        let pages = r#"[[{"ghsa_id":"GHSA-aaaa-bbbb-cccc","state":"published"}],[],[{"ghsa_id":"GHSA-dddd-eeee-ffff","state":"published"}]]"#;
        assert_eq!(
            advisories(&passing(), pages),
            [
                Finding::UnmappedAdvisory {
                    id: "GHSA-aaaa-bbbb-cccc".to_owned(),
                },
                Finding::UnmappedAdvisory {
                    id: "GHSA-dddd-eeee-ffff".to_owned(),
                },
            ]
        );
        assert_eq!(advisories(&passing(), "[[],[]]"), []);
        assert_eq!(
            advisories(&passing(), r#"[[],{"ghsa_id":"GHSA-aaaa-bbbb-cccc"}]"#),
            [Finding::Unreadable {
                path: "advisories".to_owned(),
            }]
        );
    }

    /// Verifies: SEC-TM-003
    #[test]
    fn a_draft_advisory_does_not_need_a_test() {
        let json = r#"[{"ghsa_id":"GHSA-aaaa-bbbb-cccc","state":"draft"}]"#;
        assert_eq!(advisories(&passing(), json), []);
    }

    /// Verifies: SEC-SUP-028
    #[test]
    fn dependabot_needs_cargo_actions_and_a_week_of_cooldown() {
        let tree = passing().with(".github/dependabot.yml", "version: 2\n");
        let findings = check(&tree, REVIEWED);
        assert!(findings.contains(&Finding::MissingEcosystem {
            name: "cargo".to_owned(),
        }));
        assert!(findings.contains(&Finding::ShortCooldown { days: 0 }));
        let tree = passing()
            .with("Dockerfile", "FROM scratch\n")
            .with("web/package.json", "{}\n");
        let findings = check(&tree, REVIEWED);
        assert!(findings.contains(&Finding::MissingEcosystem {
            name: "docker".to_owned(),
        }));
        assert!(findings.contains(&Finding::MissingEcosystem {
            name: "npm".to_owned(),
        }));
        let short = DEPENDABOT.replace("default-days: 7", "default-days: 6");
        let tree = passing().with(".github/dependabot.yml", &short);
        assert!(check(&tree, REVIEWED).contains(&Finding::ShortCooldown { days: 6 }));
    }

    /// Verifies: SEC-SUP-030
    #[test]
    fn reuse_and_the_licence_text_are_required() {
        let tree = passing().with("REUSE.toml", "version = 2\n");
        let expected: Vec<Finding> = [
            "version = 1",
            "fuzz/seeds/**",
            "AGPL-3.0-or-later",
            "SPDX-FileCopyrightText",
        ]
        .into_iter()
        .map(|section| Finding::MissingSection {
            path: "REUSE.toml".to_owned(),
            section: section.to_owned(),
        })
        .collect();
        assert_eq!(check(&tree, REVIEWED), expected);
    }

    #[test]
    fn the_compromise_runbook_lists_every_command_present_and_in_order() {
        let commands = runbook_block(COMPROMISE).expect("block");
        assert_eq!(commands, COMPROMISE_COMMANDS);
        assert_eq!(runbook_presence_and_order(&commands), []);
        assert_eq!(
            runbook_presence_and_order(&["gunmetal update"]).first(),
            Some(&Finding::MissingRunbookCommand {
                command: "gunmetal audit verify".to_owned(),
            })
        );
        let mut extra: Vec<&str> = COMPROMISE_COMMANDS.to_vec();
        extra.push("gunmetal extra");
        assert_eq!(
            runbook_presence_and_order(&extra),
            [Finding::MissingRunbookCommand {
                command: "gunmetal extra".to_owned(),
            }]
        );
        let wrong = ["wrong", "wrong", "wrong", "wrong", "wrong", "wrong"];
        assert_eq!(
            runbook_presence_and_order(&wrong).first(),
            Some(&Finding::MissingRunbookCommand {
                command: "gunmetal audit verify".to_owned(),
            })
        );
    }

    /// Verifies: SEC-SUP-032
    #[test]
    fn a_committed_binary_fails_and_generated_trees_are_skipped() {
        let tree = passing().with("tools/helper.exe", "");
        assert_eq!(
            check(&tree, REVIEWED).into_iter().find(is_binary_finding),
            Some(Finding::Binary {
                path: "tools/helper.exe".to_owned(),
            })
        );
        let tree = passing()
            .with("target/debug/gunmetal.so", "")
            .with("fuzz/seeds/ebml/payload.bin", "");
        assert_eq!(
            check(&tree, REVIEWED).into_iter().find(is_binary_finding),
            None
        );
        assert!(skip_binary("target/debug/x.so"));
        assert!(skip_binary("fuzz/seeds/x.bin"));
        assert!(!skip_binary("tools/helper.exe"));
        assert!(is_binary("a.zip"));
        assert!(!is_binary("a.rs"));
        assert!(BINARY_EXT.contains(&"exe"));
    }

    /// Verifies: SEC-SUP-055
    #[test]
    fn agents_and_contributing_needles_are_required() {
        let tree = passing().with("AGENTS.md", "no credentials line\n");
        assert!(check(&tree, REVIEWED).contains(&Finding::MissingSection {
            path: "AGENTS.md".to_owned(),
            section: "SEC-SUP-055".to_owned(),
        }));
    }

    /// Verifies: SEC-SUP-007, SEC-SUP-005, SEC-SUP-010, SEC-SUP-012, SEC-SUP-028, SEC-SUP-030, SEC-SUP-055
    #[test]
    fn the_repository_passes_the_repo_check() {
        let tree = Disk::new(ROOT);
        assert_eq!(check(&tree, REVIEWED), []);
        assert!(
            tree.read("SECURITY.md")
                .is_some_and(|text| text.contains("SEC-SUP-055"))
        );
    }

    #[test]
    fn date_math_matches_known_unix_midnights() {
        assert_eq!(unix_ymd("1970-01-01"), Some(0));
        assert_eq!(unix_ymd("2026-10-03"), Some(REVIEWED));
        assert_eq!(unix_ymd("2025-10-03"), Some(1_759_449_600));
        assert_eq!(unix_ymd("2024-02-29"), Some(1_709_164_800));
        assert_eq!(unix_ymd("2000-02-29"), Some(951_782_400));
        assert_eq!(unix_ymd("2023-02-29"), None);
        assert_eq!(unix_ymd("2023-02-30"), None);
        assert_eq!(unix_ymd("2023-13-01"), None);
        assert_eq!(unix_ymd("1969-12-31"), None);
        assert_eq!(unix_ymd("not-a-date"), None);
        assert!(is_leap(2000));
        assert!(is_leap(2024));
        assert!(!is_leap(1900));
        assert!(!is_leap(2023));
        assert!(!is_leap(2100));
        assert_eq!(days_in(2, true), 29);
        assert_eq!(days_in(2, false), 28);
        assert_eq!(days_in(4, false), 30);
        assert_eq!(days_in(13, false), 0);
        assert_eq!(days_since_epoch(1970, 1, 1), Some(0));
        assert_eq!(YEAR, 365 * 86_400);
    }

    #[test]
    fn uses_pins_sha_digests_and_local_actions() {
        assert!(pinned(&format!("actions/checkout@{PINNED_SHA}")));
        assert!(!pinned("actions/checkout@v4"));
        assert!(!pinned(
            "actions/checkout@aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        ));
        assert!(pinned("./.github/actions/local"));
        assert!(!pinned("actions/checkout"));
        let digest = "a".repeat(64);
        assert!(pinned(&format!("docker://example.com/ci@sha256:{digest}")));
        assert!(!pinned("docker://example.com/ci@sha256:abcd"));
        let line = format!("      - uses: actions/checkout@{PINNED_SHA} # v7");
        let expected = format!("actions/checkout@{PINNED_SHA}");
        assert_eq!(uses_spec(&line), Some(expected.as_str()));
        assert_eq!(
            uses_spec("            '      - uses: actions/checkout@v4' \\"),
            None
        );
        assert_eq!(uses_spec("# uses: actions/checkout@v4"), None);
        assert_eq!(comment("# whole"), "");
        assert_eq!(comment("  # indented"), "");
        assert_eq!(comment("uses: x # c"), "uses: x ");
        assert!(has_ecosystem(DEPENDABOT, "cargo"));
        assert!(!has_ecosystem(DEPENDABOT, "npm"));
        assert!(has_ecosystem("package-ecosystem: \"npm\"\n", "npm"));
        assert_eq!(cooldown_days(DEPENDABOT), [7, 7]);
        assert_eq!(cooldown_days("default-days: 6\n"), [6]);
    }

    #[test]
    fn settings_unreadable_dumps_and_missing_rulesets_are_named() {
        assert_eq!(
            settings(&Memory::default(), "live").first(),
            Some(&Finding::Unreadable {
                path: "live/org.json".to_owned(),
            })
        );
        let tree = live_ok().with("live/org.json", "not json");
        assert!(settings(&tree, "live").contains(&Finding::Unreadable {
            path: "live/org.json".to_owned(),
        }));
        let tree = Memory::default()
            .with("live/org.json", r#"{"two_factor_requirement_enabled":true}"#)
            .with(
                "live/repo.json",
                r#"{"security_and_analysis":{"secret_scanning":{"status":"enabled"},"secret_scanning_push_protection":{"status":"enabled"}}}"#,
            )
            .with(
                "live/private-vulnerability-reporting.json",
                r#"{"enabled":true}"#,
            )
            .with(
                "live/actions-permissions.json",
                r#"{"sha_pinning_required":true}"#,
            );
        let findings = settings(&tree, "live");
        assert!(findings.contains(&Finding::Drift {
            setting: "ruleset.main".to_owned(),
        }));
        assert!(findings.contains(&Finding::Drift {
            setting: "ruleset.tag".to_owned(),
        }));
    }

    #[test]
    fn scorecard_baseline_and_advisory_shapes() {
        let text = scorecard_ok().replace(
            r#""name":"Signed-Releases","score":-1"#,
            r#""name":"Signed-Releases","score":-2"#,
        );
        assert!(scorecard(&text).contains(&Finding::ScoreBelow {
            check: "Signed-Releases".to_owned(),
            score: -2,
            required: -1,
        }));
        let wrapped = r#"{"advisories":[{"ghsa_id":"GHSA-aaaa-bbbb-cccc"}]}"#;
        assert_eq!(
            advisories(&passing(), wrapped),
            [Finding::UnmappedAdvisory {
                id: "GHSA-aaaa-bbbb-cccc".to_owned(),
            }]
        );
        let one = r#"{"ghsa_id":"GHSA-aaaa-bbbb-cccc","state":"published"}"#;
        assert_eq!(advisories(&passing(), one).len(), 1);
        assert_eq!(
            advisories(&passing(), "{}"),
            [Finding::Unreadable {
                path: "advisories".to_owned(),
            }]
        );
        let tree = passing().with(
            "crates/demo/tests/advisory.rs",
            "fn ghsa_aaaa_bbbb_cccc_rejects() {}\n",
        );
        assert_eq!(advisories(&tree, one), []);
    }

    #[test]
    fn protected_paths_and_scorecard_lists_match_the_plan() {
        assert!(PROTECTED.contains(&"crates/gunmetal-core/src/crypto.rs"));
        assert!(PROTECTED.contains(&"crates/gunmetal-server/src/listener.rs"));
        assert!(PROTECTED.contains(&".github/CODEOWNERS"));
        assert!(MUST_BE_TEN.contains(&"Dangerous-Workflow"));
        assert!(MUST_BE_TEN.contains(&"Token-Permissions"));
        assert!(MUST_BE_TEN.contains(&"Pinned-Dependencies"));
        assert!(MUST_BE_TEN.contains(&"Binary-Artifacts"));
        assert!(MUST_BE_TEN.contains(&"Security-Policy"));
        assert!(MUST_BE_TEN.contains(&"Dependency-Update-Tool"));
        assert!(MUST_BE_TEN.contains(&"License"));
        assert_eq!(
            BASELINE,
            &[("Branch-Protection", 4), ("Signed-Releases", -1),]
        );
        assert_eq!(COMPROMISE_COMMANDS[0], "gunmetal audit verify");
        assert_eq!(COMPROMISE_COMMANDS[5], "gunmetal update");
        assert_eq!(COMPROMISE_COMMANDS.len(), 6);
    }

    #[test]
    fn an_empty_tree_misses_every_required_file() {
        let expected: Vec<Finding> = [
            "SECURITY.md",
            ".github/CODEOWNERS",
            ".github/dependabot.yml",
            ".github/workflows/codeql.yml",
            ".github/workflows/scorecard.yml",
            ".github/workflows/settings-drift.yml",
            ".github/workflows/workflow-lint.yml",
            "REUSE.toml",
            "LICENSES/AGPL-3.0-or-later.txt",
            "docs/runbooks/compromise.md",
            "docs/runbooks/signing-key.md",
            "docs/runbooks/supply-chain.md",
            "CONTRIBUTING.md",
            "AGENTS.md",
            ".github/pull_request_template.md",
        ]
        .into_iter()
        .map(|path| Finding::Missing {
            path: path.to_owned(),
        })
        .collect();
        assert_eq!(check(&Memory::default(), REVIEWED), expected);
    }

    #[test]
    fn last_reviewed_must_be_a_date() {
        let tree = passing().with("SECURITY.md", "Last reviewed: x");
        assert!(check(&tree, REVIEWED).contains(&Finding::MissingSection {
            path: "SECURITY.md".to_owned(),
            section: "Last reviewed".to_owned(),
        }));
        let text = SECURITY.replace("Last reviewed: 2026-10-03.", "Last reviewed: soon.");
        let tree = passing().with("SECURITY.md", &text);
        assert!(check(&tree, REVIEWED).contains(&Finding::MissingSection {
            path: "SECURITY.md".to_owned(),
            section: "Last reviewed".to_owned(),
        }));
        let text = SECURITY.replace("Last reviewed: 2026-10-03.", "reviewed sometime");
        let tree = passing().with("SECURITY.md", &text);
        assert!(check(&tree, REVIEWED).contains(&Finding::MissingSection {
            path: "SECURITY.md".to_owned(),
            section: "Last reviewed".to_owned(),
        }));
        let text = SECURITY.replace("2026-10-03", "2026-10-32");
        let tree = passing().with("SECURITY.md", &text);
        assert!(check(&tree, REVIEWED).contains(&Finding::StaleReview {
            path: "SECURITY.md".to_owned(),
            date: "2026-10-32".to_owned(),
        }));
    }

    #[test]
    fn default_branch_include_and_sha_pinning_false() {
        let tree = live_ok().with(
            "live/ruleset-1.json",
            r#"{"enforcement":"active","conditions":{"ref_name":{"include":["~DEFAULT_BRANCH"]}},"rules":[{"type":"deletion"},{"type":"non_fast_forward"},{"type":"required_signatures"},{"type":"pull_request","parameters":{"require_code_owner_review":true}},{"type":"required_status_checks","parameters":{"required_status_checks":[{"context":"actionlint"},{"context":"codeql"},{"context":"gate"},{"context":"zizmor"}]}}],"bypass_actors":[]}"#,
        );
        assert_eq!(
            settings(&tree, "live")
                .into_iter()
                .find(is_main_ruleset_drift),
            None
        );
        let tree = live_ok().with(
            "live/actions-permissions.json",
            r#"{"sha_pinning_required":false}"#,
        );
        assert!(settings(&tree, "live").contains(&Finding::Drift {
            setting: "sha_pinning_required".to_owned(),
        }));
        let tree = live_ok().with(
            "live/repo.json",
            r#"{"security_and_analysis":{"secret_scanning":{"status":"disabled"},"secret_scanning_push_protection":{"status":"enabled"}}}"#,
        );
        assert!(settings(&tree, "live").contains(&Finding::Drift {
            setting: "secret_scanning".to_owned(),
        }));
    }

    #[test]
    fn local_action_and_workflow_without_checkout_pass_pinning() {
        let text = "\
name: local
on: push
permissions: {}
jobs:
  x:
    runs-on: ubuntu-latest
    steps:
      - uses: ./.github/actions/thing
";
        let tree = passing().with(".github/workflows/codeql.yml", text);
        assert_eq!(check(&tree, REVIEWED).into_iter().find(is_pinning), None);
    }

    #[test]
    fn quoted_dependabot_ecosystems_and_yaml_workflows() {
        let text = "\
version: 2
updates:
  - package-ecosystem: \"cargo\"
    cooldown:
      default-days: 8
  - package-ecosystem: \"github-actions\"
    cooldown:
      default-days: 7
";
        let tree = passing().with(".github/dependabot.yml", text);
        assert_eq!(check(&tree, REVIEWED).into_iter().find(is_dependabot), None);
        let tree = passing().with(".github/workflows/extra.yaml", &workflow_text());
        assert_eq!(check(&tree, REVIEWED), []);
        let tree = passing().with(".github/workflows/notes.txt", "uses: actions/checkout@v4\n");
        assert_eq!(check(&tree, REVIEWED), []);
    }

    #[test]
    fn scorecard_equal_to_required_passes_and_malformed_scores_fail() {
        assert_eq!(scorecard(&scorecard_ok()), []);
        assert_eq!(
            scorecard(r#"{"checks":[{"name":"Dangerous-Workflow"}]}"#),
            [Finding::Unreadable {
                path: "scorecard".to_owned(),
            }]
        );
        assert_eq!(
            scorecard(r#"{"checks":[{"score":10}]}"#),
            [Finding::Unreadable {
                path: "scorecard".to_owned(),
            }]
        );
        assert_eq!(runbook_block("no fence"), None);
        assert_eq!(runbook_presence_and_order(&[]).len(), 6);
    }

    #[test]
    fn join_and_empty_dir_settings() {
        let tree = Memory::default().with("org.json", r#"{"two_factor_requirement_enabled":true}"#);
        assert_eq!(
            settings(&tree, "").first(),
            Some(&Finding::Unreadable {
                path: "repo.json".to_owned(),
            })
        );
    }

    /// A tree that lists a workflow file `read` cannot open.
    struct Ghost(Memory);

    impl Tree for Ghost {
        fn read(&self, path: &str) -> Option<String> {
            if path == ".github/workflows/ghost.yml" {
                None
            } else {
                self.0.read(path)
            }
        }

        fn files(&self, dir: &str) -> Vec<String> {
            let mut names = self.0.files(dir);
            if dir == ".github/workflows" {
                names.push("ghost.yml".to_owned());
            }
            names
        }
    }

    #[test]
    fn scorecard_ignores_ungated_checks_and_advisories_need_an_id() {
        assert_eq!(
            scorecard("{}"),
            [Finding::Unreadable {
                path: "scorecard".to_owned(),
            }]
        );
        let extra = scorecard_ok().replace(
            r#""score":-1}]}"#,
            r#""score":-1},{"name":"CII-Best-Practices","score":5}]}"#,
        );
        assert_eq!(scorecard(&extra), []);
        assert_eq!(
            advisories(&passing(), r"[{}]"),
            [Finding::Unreadable {
                path: "advisories".to_owned(),
            }]
        );
        assert_eq!(
            advisories(&passing(), "null"),
            [Finding::Unreadable {
                path: "advisories".to_owned(),
            }]
        );
        let tree = passing().with(
            "target/ghsa_aaaa_bbbb_cccc.rs",
            "fn ghsa_aaaa_bbbb_cccc_rejects() {}\n",
        );
        assert_eq!(
            advisories(
                &tree,
                r#"[{"ghsa_id":"GHSA-aaaa-bbbb-cccc","state":"published"}]"#
            ),
            [Finding::UnmappedAdvisory {
                id: "GHSA-aaaa-bbbb-cccc".to_owned(),
            }]
        );
        assert_eq!(check(&Ghost(passing()), REVIEWED), []);
        let tree = passing().with(".github/pull_request_template.md", "a fix\n");
        let expected: Vec<Finding> = ["advisory", "class review", "root cause"]
            .into_iter()
            .map(|section| Finding::MissingSection {
                path: ".github/pull_request_template.md".to_owned(),
                section: section.to_owned(),
            })
            .collect();
        assert_eq!(check(&tree, REVIEWED), expected);
    }

    #[test]
    fn settings_report_push_protection_ruleset_enforcement_and_tag_rules() {
        let tree = live_ok().with(
            "live/repo.json",
            r#"{"security_and_analysis":{"secret_scanning":{"status":"enabled"},"secret_scanning_push_protection":{"status":"disabled"}}}"#,
        );
        assert!(settings(&tree, "live").contains(&Finding::Drift {
            setting: "secret_scanning_push_protection".to_owned(),
        }));
        let tree = live_ok().with("live/repo.json", r"{}");
        assert!(settings(&tree, "live").contains(&Finding::Drift {
            setting: "secret_scanning".to_owned(),
        }));
        let tree = live_ok().with("live/ruleset-bad.json", "not json");
        assert!(settings(&tree, "live").contains(&Finding::Unreadable {
            path: "live/ruleset-bad.json".to_owned(),
        }));
        let tree = live_ok().with(
            "live/ruleset-1.json",
            r#"{"enforcement":"disabled","conditions":{"ref_name":{"include":["refs/heads/main"]}},"rules":[{"type":"deletion"},{"type":"non_fast_forward"},{"type":"required_signatures"},{"type":"pull_request","parameters":{"require_code_owner_review":true}},{"type":"required_status_checks","parameters":{"required_status_checks":[{"context":"actionlint"},{"context":"codeql"},{"context":"gate"},{"context":"zizmor"}]}}],"bypass_actors":[]}"#,
        );
        assert!(settings(&tree, "live").contains(&Finding::Drift {
            setting: "ruleset.main.enforcement".to_owned(),
        }));
        let tree = live_ok().with(
            "live/ruleset-1.json",
            r#"{"enforcement":"active","conditions":{"ref_name":{"include":["refs/heads/main"]}}}"#,
        );
        assert!(settings(&tree, "live").contains(&Finding::Drift {
            setting: "ruleset.main.required_status_checks.gate".to_owned(),
        }));
    }

    #[test]
    fn runbook_permissions_variants_and_generated_filenames() {
        assert_eq!(
            runbook_block("```runbook\ngunmetal audit verify\n"),
            Some(vec!["gunmetal audit verify"])
        );
        assert_eq!(
            runbook_block("```runbook\n\ngunmetal audit verify\n```\n"),
            Some(vec!["gunmetal audit verify"])
        );
        let text = workflow_text()
            .replace("permissions: {}", "permissions:{}")
            .replace("on: push", "on: pull_request_target");
        let tree = passing().with(".github/workflows/codeql.yml", &text);
        assert!(check(&tree, REVIEWED).contains(&Finding::DangerousTrigger {
            path: ".github/workflows/codeql.yml".to_owned(),
            trigger: "pull_request_target".to_owned(),
        }));
        let text = workflow_text().replace("permissions: {}", "permissions: { }");
        let tree = passing().with(".github/workflows/codeql.yml", &text);
        assert_eq!(check(&tree, REVIEWED).into_iter().find(is_pinning), None);
        let tree = passing()
            .with("target/Dockerfile", "FROM scratch\n")
            .with("node_modules/pkg/package.json", "{}\n");
        assert_eq!(check(&tree, REVIEWED).into_iter().find(is_dependabot), None);
        let tree = passing().with("CONTRIBUTING.md", "no secure-coding link\n");
        assert!(check(&tree, REVIEWED).contains(&Finding::MissingSection {
            path: "CONTRIBUTING.md".to_owned(),
            section: "docs/security/secure-coding.md".to_owned(),
        }));
        let text = SECURITY.replace("Last reviewed: 2026-10-03.", "Last reviewed: 2026/10/03.");
        let tree = passing().with("SECURITY.md", &text);
        assert!(check(&tree, REVIEWED).contains(&Finding::MissingSection {
            path: "SECURITY.md".to_owned(),
            section: "Last reviewed".to_owned(),
        }));
    }

    #[test]
    fn date_edges_binary_extensions_and_yaml_triggers() {
        assert_eq!(unix_ymd("nodash"), None);
        assert_eq!(unix_ymd("2026-10"), None);
        assert_eq!(unix_ymd("2026-xx-01"), None);
        assert_eq!(unix_ymd("2026-10-ab"), None);
        assert_eq!(unix_ymd("2026-10-00"), None);
        assert_eq!(unix_ymd("2000-03-01"), Some(951_868_800));
        assert_eq!(unix_ymd("2024-03-01"), Some(1_709_251_200));
        for ext in BINARY_EXT {
            assert!(is_binary(&format!("a.{ext}")));
        }
        let text = "\
name: listed
on:
  - workflow_run
permissions: {}
jobs:
  x:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
";
        let tree = passing().with(".github/workflows/NOTES.YML", text);
        assert!(check(&tree, REVIEWED).contains(&Finding::DangerousTrigger {
            path: ".github/workflows/NOTES.YML".to_owned(),
            trigger: "workflow_run".to_owned(),
        }));
        let missing = Finding::Missing {
            path: "x".to_owned(),
        };
        let stale = Finding::StaleReview {
            path: "SECURITY.md".to_owned(),
            date: "2026-10-03".to_owned(),
        };
        let alias = Finding::MissingSection {
            path: "SECURITY.md".to_owned(),
            section: "email alias".to_owned(),
        };
        let other_section = Finding::MissingSection {
            path: "SECURITY.md".to_owned(),
            section: "## Scope".to_owned(),
        };
        let binary = Finding::Binary {
            path: "a.exe".to_owned(),
        };
        let main_drift = Finding::Drift {
            setting: "ruleset.main.deletion".to_owned(),
        };
        let tag_drift = Finding::Drift {
            setting: "ruleset.tag".to_owned(),
        };
        assert!(is_stale(&stale));
        assert!(!is_stale(&missing));
        assert!(is_email_alias(&alias));
        assert!(!is_email_alias(&other_section));
        assert!(is_binary_finding(&binary));
        assert!(!is_binary_finding(&missing));
        assert!(is_main_ruleset_drift(&main_drift));
        assert!(!is_main_ruleset_drift(&tag_drift));
        assert!(is_pinning(&Finding::Unpinned {
            path: "x".to_owned(),
            uses: "y".to_owned(),
        }));
        assert!(is_pinning(&Finding::PersistCredentials {
            path: "x".to_owned(),
            line: 1,
        }));
        assert!(!is_pinning(&missing));
        assert!(is_dependabot(&Finding::ShortCooldown { days: 1 }));
        assert!(is_dependabot(&Finding::MissingEcosystem {
            name: "github-actions".to_owned(),
        }));
        assert!(!is_dependabot(&missing));
        let cargo_only = "\
version: 2
updates:
  - package-ecosystem: cargo
    cooldown:
      default-days: 7
";
        let tree = passing().with(".github/dependabot.yml", cargo_only);
        assert!(check(&tree, REVIEWED).contains(&Finding::MissingEcosystem {
            name: "github-actions".to_owned(),
        }));
    }

    #[test]
    fn missing_gated_scorecard_checks_are_below_and_ruleset_conjunctions_hold() {
        let license_only = r#"{"checks":[{"name":"License","score":10}]}"#;
        let findings = scorecard(license_only);
        assert!(findings.contains(&Finding::ScoreBelow {
            check: "Binary-Artifacts".to_owned(),
            score: -1,
            required: 10,
        }));
        assert!(findings.contains(&Finding::ScoreBelow {
            check: "Branch-Protection".to_owned(),
            score: -1,
            required: 4,
        }));
        let tree = live_ok().with("live/ruleset-note.txt", "not a ruleset");
        assert_eq!(settings(&tree, "live"), []);
        let tree = live_ok().with(
            "live/ruleset-1.json",
            r#"{"enforcement":"active","conditions":{"ref_name":{"include":["refs/heads/main"]}},"rules":[{"type":"deletion"}]}"#,
        );
        let findings = settings(&tree, "live");
        assert!(findings.contains(&Finding::Drift {
            setting: "ruleset.main.non_fast_forward".to_owned(),
        }));
        assert!(findings.contains(&Finding::Drift {
            setting: "ruleset.main.require_code_owner_review".to_owned(),
        }));
        assert!(findings.contains(&Finding::Drift {
            setting: "ruleset.main.required_status_checks.gate".to_owned(),
        }));
        let tree = live_ok().with(
            "live/ruleset-1.json",
            r#"{"enforcement":"active","conditions":{"ref_name":{"include":["refs/heads/main"]}},"rules":[{"type":"deletion"},{"type":"non_fast_forward"},{"type":"required_signatures"},{"type":"pull_request","parameters":{"require_code_owner_review":false}},{"type":"required_status_checks","parameters":{"required_status_checks":[{"context":"gate"}]}}]}"#,
        );
        assert!(settings(&tree, "live").contains(&Finding::Drift {
            setting: "ruleset.main.require_code_owner_review".to_owned(),
        }));
        let tree = live_ok().with(
            "live/ruleset-1.json",
            r#"{"enforcement":"active","conditions":{"ref_name":{"include":["refs/heads/main"]}},"rules":[{"type":"deletion"},{"type":"non_fast_forward"},{"type":"required_signatures"},{"type":"pull_request","parameters":{"require_code_owner_review":true}},{"type":"required_status_checks","parameters":{"required_status_checks":[{"context":"ci"}]}}]}"#,
        );
        assert!(settings(&tree, "live").contains(&Finding::Drift {
            setting: "ruleset.main.required_status_checks.gate".to_owned(),
        }));
        let text = SECURITY.replace("Last reviewed: 2026-10-03.", "Last reviewed: 2026-10/03.");
        let tree = passing().with("SECURITY.md", &text);
        assert!(check(&tree, REVIEWED).contains(&Finding::MissingSection {
            path: "SECURITY.md".to_owned(),
            section: "Last reviewed".to_owned(),
        }));
    }
}
