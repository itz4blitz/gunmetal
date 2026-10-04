//! `xtask docs-lint` and `xtask last-reviewed <tag>`: the security
//! baseline's documents are tested like code (security README, "Security
//! in the test-first process").
//!
//! `docs-lint` reads the requirement tables in `docs/security/`, the feature
//! tables in `docs/features/`, the threat model's tables and the work
//! packages, and fails when:
//!
//! - a requirement row's Release is not one of [`RELEASES`] or
//!   [`WITHDRAWN`], or a feature row's is not one of [`RELEASES`] or
//!   [`NEVER`] (register D-10);
//! - a requirement ID is defined twice;
//! - any document under `docs/`, or one of [`ROOT_DOCS`], cites a
//!   requirement, trust boundary (TB) or threat (TM-T) that no table
//!   defines (SEC-TM-001);
//! - a live requirement row, or a feature row that is built, cites a
//!   withdrawn requirement (SEC-STD-006);
//! - a requirement the control-ownership table lists as merged into an
//!   owner is still live, or does not name its owner;
//! - a live requirement restates a cookie attribute of the
//!   security-parameters table, a directive of the Content Security
//!   Policy's owning row, or an egress-inventory default, with a
//!   different value;
//! - a feature file, or a work package in either plan of [`PLANS`], names
//!   no TB or no TM-T (SEC-TM-001);
//! - SEC-IAM-025 (no passwords) is live while SEC-TM-013, or the password
//!   and two-factor feature rows ACC-052 and ACC-053, are live
//!   (SEC-STD-006);
//! - a requirement's Standards column cites an ID the pinned version of
//!   its standard does not have, or names a superseded edition
//!   ([`crate::standards`], SEC-STD-001).
//!
//! `last-reviewed <tag>` fails unless the threat model's "Last reviewed"
//! line names the release tag being built (SEC-TM-002).

use crate::standards;
use crate::tree::Tree;

/// The baseline's requirement files.
pub const SECURITY: &str = "docs/security";

/// The feature map.
pub const FEATURES: &str = "docs/features";

/// Every planning and design document.
pub const DOCS: &str = "docs";

/// The threat model, which defines the boundaries, the threats and the
/// tables that own controls and parameters.
pub const THREAT_MODEL: &str = "docs/security/threat-model.md";

/// The work packages.
pub const WORK_PACKAGES: &str = "docs/plan/work-packages.md";

/// The web client's work packages.
pub const CLIENT_PACKAGES: &str = "docs/plan/client-packages.md";

/// The plans, each with the prefix of its package IDs.
pub const PLANS: [(&str, &str); 2] = [(WORK_PACKAGES, "WP-"), (CLIENT_PACKAGES, "CP-")];

/// The documents at the top of the repository that cite requirements.
pub const ROOT_DOCS: [&str; 4] = ["AGENTS.md", "CONTRIBUTING.md", "README.md", "SECURITY.md"];

/// The releases, in order (register D-10).
pub const RELEASES: [&str; 7] = ["R1", "R1.1", "R1.2", "R1.3", "R2", "R3", "Later"];

/// The Release of a retired requirement.
pub const WITHDRAWN: &str = "Withdrawn";

/// The Release of a feature that is never built.
pub const NEVER: &str = "No";

/// The requirement that rules out account passwords and TOTP.
pub const NO_PASSWORDS: &str = "SEC-IAM-025";

/// The rows SEC-STD-006 names as contradicting [`NO_PASSWORDS`]: the
/// password fallback and two-factor features, and the requirement that
/// allowed passwords.
pub const PASSWORD_ROWS: [&str; 3] = ["ACC-052", "ACC-053", "SEC-TM-013"];

/// The heading of the control-ownership table in the threat model.
pub const OWNERSHIP: &str = "### Control ownership";

/// The control-ownership row whose owner states the policy.
pub const CSP: &str = "Content Security Policy";

/// Whether `name` is a Markdown file.
fn is_markdown(name: &str) -> bool {
    name.rsplit_once('.')
        .is_some_and(|(_, ext)| ext.eq_ignore_ascii_case("md"))
}

/// A feature-map page, not the README index.
fn is_feature_page(name: &str) -> bool {
    is_markdown(name) && name != "README.md" && !name.ends_with("/README.md")
}

/// A row of a requirement table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    /// Its ID, `SEC-<area>-<number>`.
    pub id: String,
    /// The requirement text.
    pub text: String,
    /// The Standards column.
    pub standards: String,
    /// The Release column.
    pub release: String,
    /// The file that defines it.
    pub path: String,
    /// Its line, counted from 1.
    pub line: usize,
}

impl Requirement {
    /// Whether it has not been withdrawn.
    pub fn is_live(&self) -> bool {
        self.release != WITHDRAWN
    }
}

/// Something `docs-lint` or `last-reviewed` found wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finding {
    /// A row's Release is not an allowed value.
    UnknownRelease {
        /// The file.
        path: String,
        /// The line, counted from 1.
        line: usize,
        /// The row's ID.
        id: String,
        /// What its Release column says.
        release: String,
    },
    /// A requirement ID defined a second time.
    Duplicate {
        /// The file of the second definition.
        path: String,
        /// Its line, counted from 1.
        line: usize,
        /// The ID.
        id: String,
    },
    /// A citation of an ID no table defines.
    UnknownId {
        /// The file.
        path: String,
        /// The line, counted from 1.
        line: usize,
        /// The ID cited.
        id: String,
    },
    /// A live row cites a withdrawn requirement.
    CitesWithdrawn {
        /// The file.
        path: String,
        /// The line, counted from 1.
        line: usize,
        /// The citing row.
        id: String,
        /// The withdrawn requirement.
        cites: String,
    },
    /// A requirement merged into an owner is still live.
    NotWithdrawn {
        /// The control.
        control: String,
        /// The requirement.
        id: String,
    },
    /// A requirement merged into an owner does not name it.
    NoPointer {
        /// The control.
        control: String,
        /// The requirement.
        id: String,
    },
    /// A live requirement restates an owned value differently.
    Conflict {
        /// The file.
        path: String,
        /// The line, counted from 1.
        line: usize,
        /// The requirement.
        id: String,
        /// The parameter key or the control.
        key: String,
        /// What the requirement says.
        stated: String,
        /// What the owner says.
        owned: String,
    },
    /// A feature file or work package names no boundary or no threat.
    Unmodelled {
        /// The file.
        path: String,
        /// The feature file or work package.
        entry: String,
        /// `TB` or `TM-T`.
        missing: &'static str,
    },
    /// A password row is live while [`NO_PASSWORDS`] is.
    PasswordsLive {
        /// The row.
        id: String,
    },
    /// A Standards column problem.
    Standard(standards::Finding),
    /// The threat model's "Last reviewed" line does not name the tag.
    NotReviewed {
        /// The release tag.
        tag: String,
        /// The line, if there is one.
        line: Option<String>,
    },
}

/// Every requirement row in the baseline.
pub fn requirements(tree: &dyn Tree) -> Vec<Requirement> {
    let mut found = Vec::new();
    for name in tree.files(SECURITY) {
        if !is_markdown(&name) {
            continue;
        }
        let path = format!("{SECURITY}/{name}");
        let Some(text) = tree.read(&path) else {
            continue;
        };
        found.extend(requirement_rows(&path, &text));
    }
    found
}

/// Checks the documents in `tree`.
pub fn check(tree: &dyn Tree) -> Vec<Finding> {
    let reqs = requirements(tree);
    let features = feature_rows(tree);
    let defined: std::collections::BTreeSet<String> = reqs.iter().map(|r| r.id.clone()).collect();
    let (boundaries, threats) = model_ids(tree);
    let mut findings = Vec::new();
    findings.extend(unknown_releases(&reqs, &features));
    findings.extend(duplicates(&reqs));
    findings.extend(unknown_citations(tree, &defined, &boundaries, &threats));
    findings.extend(cites_withdrawn(&reqs, &features));
    findings.extend(control_ownership(tree, &reqs));
    findings.extend(parameter_conflicts(tree, &reqs));
    findings.extend(egress_conflicts(tree, &reqs));
    findings.extend(unmodelled(tree));
    findings.extend(passwords_live(&reqs, &features));
    findings.extend(
        standards::citations(tree, &reqs)
            .into_iter()
            .map(Finding::Standard),
    );
    findings
}

/// Checks that the threat model's "Last reviewed" line names `tag`.
pub fn reviewed(tree: &dyn Tree, tag: &str) -> Vec<Finding> {
    let text = tree.read(THREAT_MODEL).unwrap_or_default();
    let line = text.lines().find(|line| line.starts_with("Last reviewed:"));
    match line {
        Some(line) if names_tag(line, tag) => Vec::new(),
        Some(line) => vec![Finding::NotReviewed {
            tag: tag.to_owned(),
            line: Some(line.to_owned()),
        }],
        None => vec![Finding::NotReviewed {
            tag: tag.to_owned(),
            line: None,
        }],
    }
}

/// One feature-table row.
#[derive(Debug, Clone)]
struct Feature {
    id: String,
    release: String,
    security: String,
    path: String,
    line: usize,
}

impl Feature {
    fn is_built(&self) -> bool {
        self.release != NEVER
    }
}

/// Requirement rows in `text` of `path`.
fn requirement_rows(path: &str, text: &str) -> Vec<Requirement> {
    let mut found = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let Some(cells) = cells(line) else {
            continue;
        };
        if cells.len() != 5 {
            continue;
        }
        let Some(id) = cells.first().filter(|id| is_sec_id(id)).cloned() else {
            continue;
        };
        found.push(Requirement {
            id,
            text: cells.get(1).cloned().unwrap_or_default(),
            standards: cells.get(2).cloned().unwrap_or_default(),
            release: cells.get(3).cloned().unwrap_or_default(),
            path: path.to_owned(),
            line: index.saturating_add(1),
        });
    }
    found
}

/// Feature rows under [`FEATURES`], except the README.
fn feature_rows(tree: &dyn Tree) -> Vec<Feature> {
    let mut found = Vec::new();
    for name in tree.files(FEATURES) {
        if !is_feature_page(&name) {
            continue;
        }
        let path = format!("{FEATURES}/{name}");
        let Some(text) = tree.read(&path) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            let Some(cells) = cells(line) else {
                continue;
            };
            if cells.len() != 10 {
                continue;
            }
            let Some(id) = cells.first().filter(|id| is_feature_id(id)).cloned() else {
                continue;
            };
            found.push(Feature {
                id,
                release: cells.get(5).cloned().unwrap_or_default(),
                security: cells.get(9).cloned().unwrap_or_default(),
                path: path.clone(),
                line: index.saturating_add(1),
            });
        }
    }
    found
}

/// TB and TM-T IDs defined in the threat model.
fn model_ids(
    tree: &dyn Tree,
) -> (
    std::collections::BTreeSet<String>,
    std::collections::BTreeSet<String>,
) {
    let mut boundaries = std::collections::BTreeSet::new();
    let mut threats = std::collections::BTreeSet::new();
    let Some(text) = tree.read(THREAT_MODEL) else {
        return (boundaries, threats);
    };
    for line in text.lines() {
        let Some(cells) = cells(line) else {
            continue;
        };
        let id = cells.first().cloned().unwrap_or_default();
        if is_tb(&id) {
            boundaries.insert(id.clone());
        }
        if is_tmt(&id) {
            threats.insert(id);
        }
    }
    (boundaries, threats)
}

/// Release columns that are not allowed values.
fn unknown_releases(reqs: &[Requirement], features: &[Feature]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for req in reqs {
        if !allowed_requirement_release(&req.release) {
            findings.push(Finding::UnknownRelease {
                path: req.path.clone(),
                line: req.line,
                id: req.id.clone(),
                release: req.release.clone(),
            });
        }
    }
    for feature in features {
        if !allowed_feature_release(&feature.release) {
            findings.push(Finding::UnknownRelease {
                path: feature.path.clone(),
                line: feature.line,
                id: feature.id.clone(),
                release: feature.release.clone(),
            });
        }
    }
    findings
}

/// Whether `release` is a requirement Release value.
fn allowed_requirement_release(release: &str) -> bool {
    release == WITHDRAWN || RELEASES.contains(&release)
}

/// Whether `release` is a feature Release value.
fn allowed_feature_release(release: &str) -> bool {
    release == NEVER || RELEASES.contains(&release)
}

/// A requirement ID defined a second time.
fn duplicates(reqs: &[Requirement]) -> Vec<Finding> {
    let mut seen = std::collections::BTreeSet::new();
    let mut findings = Vec::new();
    for req in reqs {
        if !seen.insert(req.id.clone()) {
            findings.push(Finding::Duplicate {
                path: req.path.clone(),
                line: req.line,
                id: req.id.clone(),
            });
        }
    }
    findings
}

/// Citations of IDs no table defines.
fn unknown_citations(
    tree: &dyn Tree,
    defined: &std::collections::BTreeSet<String>,
    boundaries: &std::collections::BTreeSet<String>,
    threats: &std::collections::BTreeSet<String>,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for name in tree.files(DOCS) {
        let path = format!("{DOCS}/{name}");
        findings.extend(unknown_in_file(
            &path,
            &tree.read(&path).unwrap_or_default(),
            defined,
            boundaries,
            threats,
        ));
    }
    for path in ROOT_DOCS {
        findings.extend(unknown_in_file(
            path,
            &tree.read(path).unwrap_or_default(),
            defined,
            boundaries,
            threats,
        ));
    }
    findings
}

/// Citations in one file.
fn unknown_in_file(
    path: &str,
    text: &str,
    defined: &std::collections::BTreeSet<String>,
    boundaries: &std::collections::BTreeSet<String>,
    threats: &std::collections::BTreeSet<String>,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (index, line) in text.lines().enumerate() {
        for id in cited_ids(line) {
            let known = defined.contains(&id) || boundaries.contains(&id) || threats.contains(&id);
            if !known {
                findings.push(Finding::UnknownId {
                    path: path.to_owned(),
                    line: index.saturating_add(1),
                    id,
                });
            }
        }
    }
    findings
}

/// Live rows that name a withdrawn requirement.
fn cites_withdrawn(reqs: &[Requirement], features: &[Feature]) -> Vec<Finding> {
    let withdrawn: std::collections::BTreeSet<&str> = reqs
        .iter()
        .filter(|req| !req.is_live())
        .map(|req| req.id.as_str())
        .collect();
    let mut findings = Vec::new();
    for req in reqs.iter().filter(|req| req.is_live()) {
        for cites in cited_sec(&req.text) {
            if withdrawn.contains(cites.as_str()) {
                findings.push(Finding::CitesWithdrawn {
                    path: req.path.clone(),
                    line: req.line,
                    id: req.id.clone(),
                    cites,
                });
            }
        }
    }
    for feature in features.iter().filter(|feature| feature.is_built()) {
        for cites in cited_sec(&feature.security) {
            if withdrawn.contains(cites.as_str()) {
                findings.push(Finding::CitesWithdrawn {
                    path: feature.path.clone(),
                    line: feature.line,
                    id: feature.id.clone(),
                    cites,
                });
            }
        }
    }
    findings
}

/// Merged-into-owner rules from the control-ownership table.
fn control_ownership(tree: &dyn Tree, reqs: &[Requirement]) -> Vec<Finding> {
    let Some(text) = tree.read(THREAT_MODEL) else {
        return Vec::new();
    };
    let by_id: std::collections::BTreeMap<&str, &Requirement> =
        reqs.iter().map(|req| (req.id.as_str(), req)).collect();
    let mut findings = Vec::new();
    for line in section(&text, OWNERSHIP) {
        let Some(cells) = cells(line) else {
            continue;
        };
        if cells.len() != 3 {
            continue;
        }
        let control = cells.first().cloned().unwrap_or_default();
        let owners = cited_sec(cells.get(1).map_or("", String::as_str));
        if owners.is_empty() {
            continue;
        }
        for id in merged_ids(cells.get(2).map_or("", String::as_str)) {
            let Some(req) = by_id.get(id.as_str()) else {
                continue;
            };
            if req.is_live() {
                findings.push(Finding::NotWithdrawn {
                    control: control.clone(),
                    id,
                });
                continue;
            }
            if !owners.iter().any(|owner| req.text.contains(owner)) {
                findings.push(Finding::NoPointer {
                    control: control.clone(),
                    id,
                });
            }
        }
    }
    findings
}

/// The lines of `text` under the heading that starts with `heading`, up to
/// the next heading of that depth.
pub(crate) fn section<'a>(text: &'a str, heading: &str) -> Vec<&'a str> {
    let mut in_table = false;
    let mut rows = Vec::new();
    for line in text.lines() {
        if line.starts_with(heading) {
            in_table = true;
            continue;
        }
        if in_table && line.starts_with("### ") {
            break;
        }
        if in_table {
            rows.push(line);
        }
    }
    rows
}

/// IDs the last column lists as merged, not as citing.
fn merged_ids(column: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for part in column.split(';') {
        if is_citing_part(part) {
            continue;
        }
        ids.extend(cited_sec(part));
    }
    ids
}

/// Whether `part` lists citing rows rather than merged ones.
fn is_citing_part(part: &str) -> bool {
    part.split_whitespace()
        .any(|word| word == "cite" || word == "cites")
}

/// Live requirements that restate an owned `SameSite` or CSP value differently.
fn parameter_conflicts(tree: &dyn Tree, reqs: &[Requirement]) -> Vec<Finding> {
    let Some(model) = tree.read(THREAT_MODEL) else {
        return Vec::new();
    };
    let params = parameters(&model);
    let csp_owner = csp_owner(&model);
    let csp_text = csp_owner.as_ref().and_then(|owner| {
        reqs.iter()
            .find(|req| req.id == *owner)
            .map(|req| req.text.as_str())
    });
    let owned_csp = csp_text.map(csp_directives).unwrap_or_default();
    let mut findings = Vec::new();
    for req in reqs.iter().filter(|req| req.is_live()) {
        findings.extend(samesite_conflicts(req, &params));
        if csp_owner.as_ref().is_some_and(|owner| owner == &req.id) {
            continue;
        }
        if owned_csp.is_empty() {
            continue;
        }
        for (directive, stated) in csp_directives(&req.text) {
            if let Some(owned) = owned_csp.get(&directive)
                && stated != *owned
            {
                findings.push(Finding::Conflict {
                    path: req.path.clone(),
                    line: req.line,
                    id: req.id.clone(),
                    key: CSP.to_owned(),
                    stated: format!("{directive} {stated}"),
                    owned: format!("{directive} {owned}"),
                });
            }
        }
    }
    findings
}

/// The security-parameters table.
fn parameters(model: &str) -> Vec<(String, String)> {
    let mut in_table = false;
    let mut rows = Vec::new();
    for line in model.lines() {
        if line.starts_with("### Security parameters") {
            in_table = true;
            continue;
        }
        if in_table && line.starts_with("### ") {
            break;
        }
        let Some(cells) = cells(line) else {
            continue;
        };
        if in_table && cells.len() == 3 {
            let key = cells.first().cloned().unwrap_or_default();
            if key == "Key" {
                continue;
            }
            rows.push((key, cells.get(1).cloned().unwrap_or_default()));
        }
    }
    rows
}

/// The owning requirement of the Content Security Policy, if it is an ID.
fn csp_owner(model: &str) -> Option<String> {
    for line in section(model, OWNERSHIP) {
        let Some(cells) = cells(line) else {
            continue;
        };
        if cells.first().is_some_and(|name| name == CSP) {
            return cited_sec(cells.get(1).map_or("", String::as_str))
                .into_iter()
                .next();
        }
    }
    None
}

/// `SameSite` restatements that disagree with the parameters table.
fn samesite_conflicts(req: &Requirement, params: &[(String, String)]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for sentence in sentences(&req.text) {
        let Some(stated) = samesite(sentence) else {
            continue;
        };
        if sentence.contains("`__Host-gm_session`") {
            if let Some((key, owned)) = param_samesite(params, "cookie.session")
                && stated != owned
            {
                findings.push(Finding::Conflict {
                    path: req.path.clone(),
                    line: req.line,
                    id: req.id.clone(),
                    key,
                    stated,
                    owned,
                });
            }
            continue;
        }
        if sentence.contains("admin")
            && sentence.contains("`__Host-`")
            && let Some((key, owned)) = param_samesite(params, "session.admin")
            && stated != owned
        {
            findings.push(Finding::Conflict {
                path: req.path.clone(),
                line: req.line,
                id: req.id.clone(),
                key,
                stated,
                owned,
            });
        }
    }
    findings
}

/// The `SameSite` value of parameter `key`, with that key.
fn param_samesite(params: &[(String, String)], key: &str) -> Option<(String, String)> {
    params
        .iter()
        .find(|(name, _)| name == key)
        .and_then(|(name, value)| samesite(value).map(|owned| (name.clone(), owned)))
}

/// `SameSite=` in `text`.
fn samesite(text: &str) -> Option<String> {
    let rest = text.split("SameSite=").nth(1)?;
    let value: String = rest.chars().take_while(char::is_ascii_alphabetic).collect();
    if value.is_empty() {
        None
    } else {
        Some(format!("SameSite={value}"))
    }
}

/// Sentences of `text`, split on `. ` and `; `.
fn sentences(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut rest = text;
    loop {
        let dot = rest.find(". ");
        let semi = rest.find("; ");
        let cut = match (dot, semi) {
            (Some(d), Some(s)) => Some(d.min(s)),
            (Some(d), None) => Some(d),
            (None, Some(s)) => Some(s),
            (None, None) => None,
        };
        let Some(at) = cut else {
            if !rest.is_empty() {
                parts.push(rest);
            }
            break;
        };
        let (head, tail) = rest.split_at(at);
        if !head.is_empty() {
            parts.push(head);
        }
        rest = rest_from(tail, 2);
    }
    parts
}

/// CSP directives restated in `text`.
fn csp_directives(text: &str) -> std::collections::BTreeMap<String, String> {
    let names = [
        "default-src",
        "img-src",
        "style-src",
        "script-src",
        "font-src",
        "connect-src",
    ];
    let mut found = std::collections::BTreeMap::new();
    for name in names {
        let Some(after) = find_directive(text, name) else {
            continue;
        };
        found.insert(name.to_owned(), after);
    }
    found
}

/// The value after directive `name`, if `text` restates one.
fn find_directive(text: &str, name: &str) -> Option<String> {
    let mut rest = text;
    while let Some(at) = rest.find(name) {
        let before = rest.get(..at).and_then(|h| h.chars().next_back());
        let after_name = rest_after(rest, at, name.len());
        let after_char = after_name.chars().next();
        rest = after_name;
        let bounded = before.is_none_or(|ch| !ch.is_ascii_alphanumeric() && ch != '-')
            && after_char.is_none_or(|ch| matches!(ch, '*' | '\'' | '"' | ' ' | '\t'));
        if !bounded {
            continue;
        }
        let trimmed = after_name.trim_start();
        let Some(first) = trimmed.chars().next() else {
            continue;
        };
        // A restated directive is followed by a quoted list or `*`. Bare
        // `img-src` in prose is not a value.
        if !matches!(first, '*' | '\'' | '"') {
            continue;
        }
        let value: String = trimmed
            .chars()
            .take_while(|&ch| ch != ';' && ch != '`')
            .collect();
        return Some(value.trim().to_owned());
    }
    None
}

/// Live requirements that restate an egress-inventory default differently.
///
/// A sentence is about an inventory row when it names one of the things
/// the row's Purpose cell lists ([`purpose_terms`]), and it restates the
/// default when it says `on by default` or `off by default`. Both have to
/// be in the one sentence, as for the cookie attributes.
/// Only the first default a sentence states is read.
fn egress_conflicts(tree: &dyn Tree, reqs: &[Requirement]) -> Vec<Finding> {
    let Some(model) = tree.read(THREAT_MODEL) else {
        return Vec::new();
    };
    let mut findings = Vec::new();
    for (purpose, default, owners) in egress_rows(&model) {
        let Some(owned_on) = default_on(&default) else {
            continue;
        };
        let terms = purpose_terms(&purpose);
        for req in reqs.iter().filter(|req| req.is_live()) {
            if owners.iter().any(|owner| owner == &req.id) {
                continue;
            }
            for sentence in sentences(&req.text) {
                let said = words(sentence);
                let Some((stated, stated_on)) = restated_default(&said) else {
                    continue;
                };
                if stated_on != owned_on && terms.iter().any(|term| holds(&said, term)) {
                    findings.push(Finding::Conflict {
                        path: req.path.clone(),
                        line: req.line,
                        id: req.id.clone(),
                        key: purpose.clone(),
                        stated: stated.to_owned(),
                        owned: default.clone(),
                    });
                }
            }
        }
    }
    findings
}

/// The words of `text` for comparing prose: letters and digits in lower
/// case, without `the` and without a final `s`, so that `a metadata
/// provider` and `Metadata providers` read alike.
fn words(text: &str) -> Vec<String> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(str::to_ascii_lowercase)
        .filter(|word| !word.is_empty() && word != "the")
        .map(|word| word.strip_suffix('s').unwrap_or(&word).to_owned())
        .collect()
}

/// The things a Purpose cell lists, each as its [`words`]: the cell cut at
/// every `:`, `,` and `and`. `Relays, address lookup and the browser edge`
/// lists `relay`, `addres lookup` and `browser edge`.
fn purpose_terms(purpose: &str) -> Vec<Vec<String>> {
    purpose
        .replace(" and ", ",")
        .split([',', ':'])
        .map(words)
        .filter(|term| !term.is_empty())
        .collect()
}

/// Whether the words `said` hold the words of `term` in a row.
fn holds(said: &[String], term: &[String]) -> bool {
    (0..said.len()).any(|at| said.get(at..).is_some_and(|rest| rest.starts_with(term)))
}

/// Rows of the egress inventory: purpose, default and owners.
fn egress_rows(model: &str) -> Vec<(String, String, Vec<String>)> {
    let mut in_table = false;
    let mut rows = Vec::new();
    for line in model.lines() {
        if line.starts_with("### Egress inventory") {
            in_table = true;
            continue;
        }
        if in_table && line.starts_with("### ") {
            break;
        }
        let Some(cells) = cells(line) else {
            continue;
        };
        if in_table && cells.len() == 6 {
            let purpose = cells.first().cloned().unwrap_or_default();
            if purpose == "Purpose" {
                continue;
            }
            rows.push((
                purpose,
                cells.get(1).cloned().unwrap_or_default(),
                cited_sec(cells.get(5).map_or("", String::as_str)),
            ));
        }
    }
    rows
}

/// Whether `default` starts with On (`true`) or Off (`false`).
fn default_on(default: &str) -> Option<bool> {
    let first = default
        .split(|ch: char| !ch.is_ascii_alphabetic())
        .find(|part| !part.is_empty())?;
    match first.to_ascii_lowercase().as_str() {
        "on" => Some(true),
        "off" => Some(false),
        _ => None,
    }
}

/// `on by default` or `off by default` among the words `said`.
fn restated_default(said: &[String]) -> Option<(&'static str, bool)> {
    said.iter()
        .zip(said.iter().skip(1))
        .zip(said.iter().skip(2))
        .find_map(|((state, by), default)| {
            if by != "by" || default != "default" {
                return None;
            }
            match state.as_str() {
                "on" => Some(("on by default", true)),
                "off" => Some(("off by default", false)),
                _ => None,
            }
        })
}

/// Feature files and work packages that name no TB or no TM-T.
fn unmodelled(tree: &dyn Tree) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (path, prefix) in PLANS {
        let text = tree.read(path).unwrap_or_default();
        for (entry, body) in work_packages(&text, prefix) {
            if !has_tb(&body) {
                findings.push(Finding::Unmodelled {
                    path: path.to_owned(),
                    entry: entry.clone(),
                    missing: "TB",
                });
            }
            if !has_tmt(&body) {
                findings.push(Finding::Unmodelled {
                    path: path.to_owned(),
                    entry,
                    missing: "TM-T",
                });
            }
        }
    }
    for name in tree.files(FEATURES) {
        if !is_feature_page(&name) {
            continue;
        }
        let path = format!("{FEATURES}/{name}");
        let text = tree.read(&path).unwrap_or_default();
        if !has_tb(&text) {
            findings.push(Finding::Unmodelled {
                path: path.clone(),
                entry: path.clone(),
                missing: "TB",
            });
        }
        if !has_tmt(&text) {
            findings.push(Finding::Unmodelled {
                path: path.clone(),
                entry: path,
                missing: "TM-T",
            });
        }
    }
    findings
}

/// The headings that start with a `prefix` package ID, each with the text
/// until the next heading.
fn work_packages(text: &str, prefix: &str) -> Vec<(String, String)> {
    let mut current: Option<(String, String)> = None;
    let mut out = Vec::new();
    for line in text.lines() {
        if let Some(title) = heading_title(line) {
            if let Some(prev) = current.take() {
                out.push(prev);
            }
            if let Some(id) = wp_id(title, prefix) {
                current = Some((id, String::new()));
            }
            continue;
        }
        if let Some((_, body)) = &mut current {
            body.push_str(line);
            body.push('\n');
        }
    }
    if let Some(prev) = current {
        out.push(prev);
    }
    out
}

/// The title of an ATX heading.
fn heading_title(line: &str) -> Option<&str> {
    let rest = line.strip_prefix('#')?;
    let rest = rest.trim_start_matches('#');
    let title = rest.strip_prefix(' ')?;
    Some(title.trim())
}

/// `prefix` plus digits at the start of a heading title.
fn wp_id(title: &str, prefix: &str) -> Option<String> {
    let id = title.split_whitespace().next()?;
    let digits = id.strip_prefix(prefix)?;
    if digits.is_empty() || !digits.chars().all(|ch| ch.is_ascii_digit()) {
        None
    } else {
        Some(id.to_owned())
    }
}

/// Password rows that are live while [`NO_PASSWORDS`] is.
fn passwords_live(reqs: &[Requirement], features: &[Feature]) -> Vec<Finding> {
    let no_passwords_live = reqs
        .iter()
        .any(|req| req.id == NO_PASSWORDS && req.is_live());
    if !no_passwords_live {
        return Vec::new();
    }
    let mut findings = Vec::new();
    for req in reqs {
        if PASSWORD_ROWS.contains(&req.id.as_str()) && req.is_live() {
            findings.push(Finding::PasswordsLive { id: req.id.clone() });
        }
    }
    for feature in features {
        if PASSWORD_ROWS.contains(&feature.id.as_str()) && feature.is_built() {
            findings.push(Finding::PasswordsLive {
                id: feature.id.clone(),
            });
        }
    }
    findings
}

/// Whether the last-reviewed line names `tag` as a release tag. An empty
/// tag, as from an unset variable, names nothing.
fn names_tag(line: &str, tag: &str) -> bool {
    if tag.is_empty() {
        return false;
    }
    if !tag.contains('.') {
        return line.contains(&format!("for {tag}")) && token_eq(line, tag);
    }
    token_eq(line, tag)
}

/// Whether `needle` appears in `haystack` as a whole token.
fn token_eq(haystack: &str, needle: &str) -> bool {
    let mut rest = haystack;
    while let Some(at) = rest.find(needle) {
        let before = rest.get(..at).and_then(|h| h.chars().next_back());
        let after = rest.get(at.saturating_add(needle.len())..);
        let after_ch = after.and_then(|t| t.chars().next());
        let after_next = after.and_then(|t| t.chars().nth(1));
        let ok_after = match after_ch {
            None => true,
            Some('.') => after_next.is_none_or(|ch| !ch.is_ascii_digit()),
            Some(ch) => is_token_edge(ch),
        };
        if before.is_none_or(is_token_edge) && ok_after {
            return true;
        }
        rest = rest_after(rest, at, 1);
    }
    false
}

/// A character that may sit next to a release tag.
fn is_token_edge(ch: char) -> bool {
    !ch.is_ascii_alphanumeric() && ch != '.' && ch != '-'
}

/// Markdown table cells on `line`.
pub(crate) fn cells(line: &str) -> Option<Vec<String>> {
    let body = line.trim().strip_prefix('|')?;
    let body = body.strip_suffix('|').unwrap_or(body);
    let parts: Vec<String> = body.split('|').map(str::trim).map(str::to_owned).collect();
    if parts
        .iter()
        .all(|cell| !cell.is_empty() && cell.chars().all(|ch| matches!(ch, '-' | ':')))
    {
        return None;
    }
    Some(parts)
}

/// A `SEC-AREA-number` ID.
fn is_sec_id(id: &str) -> bool {
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

/// A feature ID (`ACC-001`), not a requirement.
fn is_feature_id(id: &str) -> bool {
    if id.starts_with("SEC-") {
        return false;
    }
    let Some((area, number)) = id.split_once('-') else {
        return false;
    };
    !area.is_empty()
        && area.chars().all(|ch| ch.is_ascii_uppercase())
        && !number.is_empty()
        && number.chars().all(|ch| ch.is_ascii_digit())
}

/// A trust-boundary ID.
fn is_tb(id: &str) -> bool {
    let Some(digits) = id.strip_prefix("TB") else {
        return false;
    };
    !digits.is_empty() && digits.chars().all(|ch| ch.is_ascii_digit())
}

/// A threat-register ID.
fn is_tmt(id: &str) -> bool {
    let Some(digits) = id.strip_prefix("TM-T") else {
        return false;
    };
    !digits.is_empty() && digits.chars().all(|ch| ch.is_ascii_digit())
}

/// Whether `text` names a TB ID.
fn has_tb(text: &str) -> bool {
    cited_ids(text).into_iter().any(|id| is_tb(&id))
}

/// Whether `text` names a TM-T ID.
fn has_tmt(text: &str) -> bool {
    cited_ids(text).into_iter().any(|id| is_tmt(&id))
}

/// SEC IDs cited in `text`, in order.
pub(crate) fn cited_sec(text: &str) -> Vec<String> {
    cited_ids(text)
        .into_iter()
        .filter(|id| is_sec_id(id))
        .collect()
}

/// SEC, TB and TM-T IDs cited in `text`, in order.
fn cited_ids(text: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i != bytes.len() {
        if let Some(id) = take_id(text, i) {
            ids.push(id.clone());
            i = i.saturating_add(id.len().max(1)).min(bytes.len());
            continue;
        }
        i = i.saturating_add(1);
    }
    ids
}

/// An ID starting at byte `at`, if the boundaries hold.
fn take_id(text: &str, at: usize) -> Option<String> {
    let before = text.get(..at).and_then(|h| h.chars().next_back());
    if before.is_some_and(|ch| ch.is_ascii_alphanumeric()) {
        return None;
    }
    let rest = text.get(at..)?;
    for prefix in ["SEC-", "TM-T", "TB"] {
        if let Some(id) = take_prefixed(rest, prefix)
            && rest
                .get(id.len()..)
                .and_then(|t| t.chars().next())
                .is_none_or(|ch| !ch.is_ascii_alphanumeric())
        {
            return Some(id);
        }
    }
    None
}

/// `prefix` plus its ID rest at the start of `rest`.
fn take_prefixed(rest: &str, prefix: &str) -> Option<String> {
    let after = rest.strip_prefix(prefix)?;
    if prefix == "SEC-" {
        let (area, tail) = after.split_once('-')?;
        if area.is_empty() || !area.chars().all(|ch| ch.is_ascii_uppercase()) {
            return None;
        }
        let number: String = tail.chars().take_while(char::is_ascii_digit).collect();
        if number.is_empty() {
            None
        } else {
            Some(format!("SEC-{area}-{number}"))
        }
    } else {
        let number: String = after.chars().take_while(char::is_ascii_digit).collect();
        if number.is_empty() {
            None
        } else {
            Some(format!("{prefix}{number}"))
        }
    }
}

/// The suffix of `text` starting at `at`, or empty when `at` is past the end.
pub(crate) fn rest_from(text: &str, at: usize) -> &str {
    text.get(at..).unwrap_or("")
}

/// The suffix after skipping `skip` bytes from `at`, saturating at the end.
pub(crate) fn rest_after(text: &str, at: usize, skip: usize) -> &str {
    rest_from(text, at.saturating_add(skip))
}

#[cfg(test)]
pub mod tests {
    //! The docs lint's fixtures, which the standards and traceability
    //! tests build on.

    use super::{Finding, Requirement, check, requirements, reviewed};
    use crate::standards;
    use crate::tree::Tree;
    use crate::tree::memory::Memory;
    use crate::{Failure, ROOT, dispatch};

    /// A tree that lists files it will not read.
    pub struct Hide {
        /// The files.
        pub inner: Memory,
        /// Paths `read` hides.
        pub hidden: Vec<&'static str>,
    }

    impl Tree for Hide {
        fn read(&self, path: &str) -> Option<String> {
            if self.hidden.contains(&path) {
                None
            } else {
                self.inner.read(path)
            }
        }

        fn files(&self, dir: &str) -> Vec<String> {
            self.inner.files(dir)
        }
    }

    /// A threat model that defines one boundary, one threat, the tables
    /// that own controls and parameters, and the release-scope table.
    pub const MODEL: &str = "\
# Threat model

Date: 2026-10-02. Status: proposed.
Last reviewed: 2026-10-02 (v0.1.0, the R1 baseline).

### Trust boundaries

| ID | Boundary | What crosses it | First release |
|---|---|---|---|
| TB1 | Internet to server | HTTP | R1 |

### Threat register

| ID | Threat | Who | Impact | Likelihood | Mitigated by |
|---|---|---|---|---|---|
| TM-T01 | A published bug is exploited | ADV-1 | High | High | 001 |

## Requirements

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-TM-001 | Every feature file must name TB and TM-T IDs. | SSDF PW.1.1; ASVS 15.1.5 | R1 | Docs lint |
| SEC-TM-013 | **Withdrawn 2026-10-02: merged into SEC-IAM-025.** | ASVS 6.1.3 | Withdrawn | Proved by the tests of SEC-IAM-025 |

### Control ownership

| Control | Owner | Withdrawn into it, or citing it |
|---|---|---|
| No passwords and no TOTP | SEC-IAM-025 | SEC-TM-013; SEC-API-001 cites it |
| Content Security Policy | SEC-API-044 | SEC-API-001 cites it |

### Security parameters

| Key | Value | Owner |
|---|---|---|
| cookie.session | `__Host-gm_session`; Secure, HttpOnly, SameSite=Lax, Path=/ | SEC-API-032 |
| session.admin | Separate `__Host-` cookie, SameSite=Strict | SEC-IAM-041 |
| identifier.bits | At least 128 random bits | SEC-API-023 |

### Egress inventory

| Purpose | Default | Destination | Data sent | Release | Owner |
|---|---|---|---|---|---|
| Naming: label registration and DNS-01 updates | On when the install chose the project name service (recommended, OD-1); allowed before the claim | The project name service | Random label, public key, TXT value, signature | R2 | SEC-TM-001 |
| ACME certificate issuance | On with project or own-domain naming; allowed before the claim | The configured CA | Certificate request; ACME account key | R1 | SEC-TM-001 |
| Certificate Transparency monitoring | On with project naming, from the claim | Two independent CT monitors | The server's own name | R2 | SEC-TM-001 |
| Update and advisory feed | The owner's first-run answer | The project feed (static files) | A plain GET with no identifiers | R1 | SEC-TM-001 |
| OIDC discovery, keys and tokens | When the owner configures a provider | The configured issuer | Standard OIDC requests | R1.2 | SEC-TM-001 |
| Metadata, artwork and lyrics providers | Off until the owner turns one on in the required setup step | The provider | The fields the setup screen lists | R1.1 | SEC-TM-001 |
| Relays, address lookup and the browser edge | Off until the owner turns on remote access | Configured relays and edge | As the relay requirements state | R2 | SEC-TM-001 |
| Plugins and webhooks | Off; per grant or allowlisted host | Granted hosts | Per grant | R2 | SEC-TM-001 |
| Live TV sources | Off; per source | The source | Per source | R3 | SEC-TM-001 |
";

    /// Requirements on identity, R1 and R1.2.
    pub const IDENTITY: &str = "\
# Identity

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-IAM-025 | The server must not offer account passwords. | ASVS 6.1.3; A07:2025; CWE-521 | R1 | Docs lint |
| SEC-IAM-026 | OIDC sign-in must use PKCE. | ASVS 10.2.1 | R1.2 | Integration test |
| SEC-IAM-041 | The admin session is held in its own `__Host-` cookie with SameSite=Strict. | ASVS 7.3.1 | R1 | Unit test |
";

    /// Requirements on the web client.
    pub const WEB: &str = "\
# Web

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-API-001 | Every route declares its policy; the no-passwords rule of SEC-IAM-025 holds. | API5:2023; Top 10 A01 | R1 | Route test |
| SEC-API-032 | The session cookie is `__Host-gm_session` with `SameSite=Lax`. The admin cookie of SEC-IAM-041 uses SameSite=Strict. | ASVS 3.3.1; MASVS-NETWORK-1 | R1 | Golden test |
| SEC-API-044 | Every HTML response carries `default-src 'none'; img-src 'self' blob:; style-src 'self'`. | ASVS 3.4.3 | R1 | Browser test |
| SEC-API-097 | Share links use the fragment pattern. | API1:2023 | R2 | Integration test |
| SEC-API-023 | Identifiers have 128 bits. | ASVS 8.2.2 | R1 | Unit test |
";

    /// A feature file with its security notes.
    pub const ACCOUNTS: &str = "\
# Accounts

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces | Security |
|---|---|---|---|---|---|---|---|---|---|
| ACC-001 | Passkeys | Sign in | Rare | High | R1 | Better | WebAuthn | Sign-in | SEC-IAM-025 |
| ACC-052 | Password sign-in as a fallback | Passwords | All | High | No | Never | None | None | SEC-TM-013 |
| ACC-053 | Two-factor codes | Codes | Plex | Medium | No | Never | None | None | |

## Security notes

Crosses TB1; threat TM-T01.
";

    /// The work packages.
    pub const PLAN: &str = "\
# Work packages

### WP-001 Workspace

- **Security.** Boundaries TB1; threats TM-T01. Verifies SEC-TM-001.

#### WP-002 Moved package

- **Security.** Boundaries TB1; threats TM-T01.

## Waves
";

    /// The pinned standards, cut down to the IDs the fixtures cite and a
    /// few more.
    pub const ASVS: &str =
        "# ASVS\n3.3.1 1\n3.4.3 2\n6.1.3 1\n7.3.1 1\n8.2.2 2\n10.2.1 2\n15.1.5 3\n";

    /// A repository whose documents pass every lint.
    pub fn repo() -> Memory {
        Memory::default()
            .with("docs/security/threat-model.md", MODEL)
            .with("docs/security/identity-and-access.md", IDENTITY)
            .with("docs/security/web-and-api-security.md", WEB)
            .with("docs/features/accounts.md", ACCOUNTS)
            .with(
                "docs/features/README.md",
                "# Feature map\n\nSee SEC-TM-001.\n",
            )
            .with("docs/plan/work-packages.md", PLAN)
            .with("CONTRIBUTING.md", "Verifies: SEC-TM-001\n")
            .with("crates/xtask/standards/asvs-5.0.0.txt", ASVS)
            .with(
                "crates/xtask/standards/top10-2025.txt",
                "A01 Access\nA07 Authentication\n",
            )
            .with(
                "crates/xtask/standards/api-top10-2023.txt",
                "API1 Objects\nAPI5 Functions\n",
            )
            .with(
                "crates/xtask/standards/masvs-2.1.0.txt",
                "MASVS-NETWORK-1\n",
            )
            .with("crates/xtask/standards/ssdf-1.1.txt", "PW.1\nPW.1.1\n")
            .with("crates/xtask/standards/cwe-4.20.txt", "1 deprecated\n521\n")
            .with("crates/xtask/standards/cwe-top25-2025.txt", "79\n")
    }

    /// `repo` with `old` replaced by `new` in the file at `path`.
    pub fn edited(path: &str, old: &str, new: &str) -> Memory {
        let text = crate::tree::Tree::read(&repo(), path).expect("the fixture has the file");
        assert!(text.contains(old), "the fixture holds the text to replace");
        repo().with(path, &text.replace(old, new))
    }

    #[test]
    fn a_consistent_repository_passes() {
        assert_eq!(check(&repo()), []);
    }

    #[test]
    fn reads_every_requirement_row_with_its_place() {
        let found = requirements(&repo());
        let ids: Vec<(&str, &str, &str, usize)> = found
            .iter()
            .map(|r| (r.id.as_str(), r.release.as_str(), r.path.as_str(), r.line))
            .collect();
        assert_eq!(
            ids,
            [
                (
                    "SEC-IAM-025",
                    "R1",
                    "docs/security/identity-and-access.md",
                    5
                ),
                (
                    "SEC-IAM-026",
                    "R1.2",
                    "docs/security/identity-and-access.md",
                    6
                ),
                (
                    "SEC-IAM-041",
                    "R1",
                    "docs/security/identity-and-access.md",
                    7
                ),
                ("SEC-TM-001", "R1", "docs/security/threat-model.md", 22),
                (
                    "SEC-TM-013",
                    "Withdrawn",
                    "docs/security/threat-model.md",
                    23
                ),
                (
                    "SEC-API-001",
                    "R1",
                    "docs/security/web-and-api-security.md",
                    5
                ),
                (
                    "SEC-API-032",
                    "R1",
                    "docs/security/web-and-api-security.md",
                    6
                ),
                (
                    "SEC-API-044",
                    "R1",
                    "docs/security/web-and-api-security.md",
                    7
                ),
                (
                    "SEC-API-097",
                    "R2",
                    "docs/security/web-and-api-security.md",
                    8
                ),
                (
                    "SEC-API-023",
                    "R1",
                    "docs/security/web-and-api-security.md",
                    9
                ),
            ]
        );
        assert_eq!(
            found[0],
            Requirement {
                id: "SEC-IAM-025".to_owned(),
                text: "The server must not offer account passwords.".to_owned(),
                standards: "ASVS 6.1.3; A07:2025; CWE-521".to_owned(),
                release: "R1".to_owned(),
                path: "docs/security/identity-and-access.md".to_owned(),
                line: 5,
            }
        );
        assert!(found[0].is_live());
        assert!(!found[4].is_live());
    }

    #[test]
    fn rows_that_are_not_requirements_are_skipped() {
        let tree = Memory::default()
            .with(
                "docs/security/a.md",
                "| SEC-TM-001 | short name |\n| SEC-TM-002 | a | b | R1 | c | extra |\n| TM-T01 | a | b | R1 | c |\n| SEC-TM-003 | a | b | R1 | c |\nSEC-TM-004 | a | b | R1 | c |\n",
            )
            .with("docs/security/notes.txt", "| SEC-TM-005 | a | b | R1 | c |\n");
        let ids: Vec<String> = requirements(&tree).into_iter().map(|r| r.id).collect();
        assert_eq!(ids, ["SEC-TM-003"]);
    }

    /// Verifies: SEC-TM-074
    #[test]
    fn an_unknown_release_value_fails() {
        let tree = edited("docs/security/identity-and-access.md", "| R1.2 |", "| R4 |").with(
            "docs/features/accounts.md",
            &ACCOUNTS
                .replace("| High | R1 |", "| High | Withdrawn |")
                .replace("| Plex | Medium | No |", "| Plex | Medium | Never |"),
        );
        assert_eq!(
            check(&tree),
            [
                Finding::UnknownRelease {
                    path: "docs/security/identity-and-access.md".to_owned(),
                    line: 6,
                    id: "SEC-IAM-026".to_owned(),
                    release: "R4".to_owned(),
                },
                Finding::UnknownRelease {
                    path: "docs/features/accounts.md".to_owned(),
                    line: 5,
                    id: "ACC-001".to_owned(),
                    release: "Withdrawn".to_owned(),
                },
                Finding::UnknownRelease {
                    path: "docs/features/accounts.md".to_owned(),
                    line: 7,
                    id: "ACC-053".to_owned(),
                    release: "Never".to_owned(),
                },
                // Any Release but No counts as built.
                Finding::PasswordsLive {
                    id: "ACC-053".to_owned(),
                },
            ]
        );
        let never = edited("docs/security/identity-and-access.md", "| R1.2 |", "| No |");
        assert_eq!(
            check(&never),
            [Finding::UnknownRelease {
                path: "docs/security/identity-and-access.md".to_owned(),
                line: 6,
                id: "SEC-IAM-026".to_owned(),
                release: "No".to_owned(),
            }]
        );
    }

    /// Verifies: SEC-TM-074
    #[test]
    fn every_release_value_of_the_register_is_allowed() {
        for release in ["R1", "R1.1", "R1.2", "R1.3", "R2", "R3", "Later"] {
            let requirement = edited(
                "docs/security/identity-and-access.md",
                "| R1.2 |",
                &format!("| {release} |"),
            );
            assert_eq!(check(&requirement), []);
            let feature = edited(
                "docs/features/accounts.md",
                "| High | R1 |",
                &format!("| High | {release} |"),
            );
            assert_eq!(check(&feature), []);
        }
    }

    #[test]
    fn a_requirement_defined_twice_fails() {
        let tree = edited(
            "docs/security/web-and-api-security.md",
            "| SEC-API-097 |",
            "| SEC-IAM-026 |",
        );
        assert_eq!(
            check(&tree),
            [Finding::Duplicate {
                path: "docs/security/web-and-api-security.md".to_owned(),
                line: 8,
                id: "SEC-IAM-026".to_owned(),
            }]
        );
    }

    /// Verifies: SEC-TM-001
    #[test]
    fn a_citation_of_an_id_no_table_defines_fails() {
        let tree = repo()
            .with(
                "docs/plan/api-needs.md",
                "Needs SEC-API-999 and SEC-IAM-025.\nCrosses TB1 and TB9, threat TM-T01 and TM-T77.\nNot IDs: SEC-API-, SEC-api-1, TB, TBx, TM-T, SEC--1, TB1a.\n",
            )
            .with("SECURITY.md", "Report under SEC-OPS-404.\n");
        let unknown = |path: &str, line, id: &str| Finding::UnknownId {
            path: path.to_owned(),
            line,
            id: id.to_owned(),
        };
        assert_eq!(
            check(&tree),
            [
                unknown("docs/plan/api-needs.md", 1, "SEC-API-999"),
                unknown("docs/plan/api-needs.md", 2, "TB9"),
                unknown("docs/plan/api-needs.md", 2, "TM-T77"),
                unknown("SECURITY.md", 1, "SEC-OPS-404"),
            ]
        );
    }

    /// Verifies: SEC-STD-006, SEC-TM-073
    #[test]
    fn a_live_row_that_cites_a_withdrawn_requirement_fails() {
        let requirement = edited(
            "docs/security/web-and-api-security.md",
            "the no-passwords rule of SEC-IAM-025",
            "the no-passwords rule of SEC-TM-013",
        );
        let feature = edited(
            "docs/features/accounts.md",
            "| Sign-in | SEC-IAM-025 |",
            "| Sign-in | SEC-TM-013 |",
        );
        assert_eq!(
            check(&requirement),
            [Finding::CitesWithdrawn {
                path: "docs/security/web-and-api-security.md".to_owned(),
                line: 5,
                id: "SEC-API-001".to_owned(),
                cites: "SEC-TM-013".to_owned(),
            }]
        );
        assert_eq!(
            check(&feature),
            [Finding::CitesWithdrawn {
                path: "docs/features/accounts.md".to_owned(),
                line: 5,
                id: "ACC-001".to_owned(),
                cites: "SEC-TM-013".to_owned(),
            }]
        );
        // A withdrawn row may point at another, and a feature that is never
        // built may cite one, as ACC-052 does in the fixture.
        let retired = edited(
            "docs/security/threat-model.md",
            "merged into SEC-IAM-025.**",
            "merged into SEC-IAM-025, as SEC-TM-013 says.**",
        );
        assert_eq!(check(&retired), []);
    }

    /// Verifies: SEC-TM-073
    #[test]
    fn a_requirement_merged_into_its_owner_is_withdrawn_and_points_at_it() {
        let live = edited(
            "docs/security/threat-model.md",
            "| ASVS 6.1.3 | Withdrawn |",
            "| ASVS 6.1.3 | R1 |",
        );
        assert_eq!(
            check(&live),
            [
                Finding::NotWithdrawn {
                    control: "No passwords and no TOTP".to_owned(),
                    id: "SEC-TM-013".to_owned(),
                },
                Finding::PasswordsLive {
                    id: "SEC-TM-013".to_owned(),
                },
            ]
        );
        let pointless = edited(
            "docs/security/threat-model.md",
            "merged into SEC-IAM-025.**",
            "merged.**",
        );
        assert_eq!(
            check(&pointless),
            [Finding::NoPointer {
                control: "No passwords and no TOTP".to_owned(),
                id: "SEC-TM-013".to_owned(),
            }]
        );
        // Every word before the last two of a part decides nothing: only a
        // part that ends "cite it", "cites it", "cite them" or "cites them"
        // lists citing rows.
        for ending in ["cite it", "cites them", "cite them"] {
            let citing = edited(
                "docs/security/threat-model.md",
                "SEC-API-001 cites it |\n| Content",
                &format!("SEC-API-001 {ending} |\n| Content"),
            );
            assert_eq!(check(&citing), []);
        }
        let merged = edited(
            "docs/security/threat-model.md",
            "SEC-API-001 cites it |\n| Content",
            "SEC-API-001 recites |\n| Content",
        );
        assert_eq!(
            check(&merged),
            [Finding::NotWithdrawn {
                control: "No passwords and no TOTP".to_owned(),
                id: "SEC-API-001".to_owned(),
            }]
        );
    }

    /// Verifies: SEC-TM-072
    #[test]
    fn a_conflicting_samesite_value_fails() {
        let tree = edited(
            "docs/security/identity-and-access.md",
            "| OIDC sign-in must use PKCE. |",
            "| OIDC sign-in must use PKCE; it returns to `__Host-gm_session` with SameSite=None. |",
        );
        assert_eq!(
            check(&tree),
            [Finding::Conflict {
                path: "docs/security/identity-and-access.md".to_owned(),
                line: 6,
                id: "SEC-IAM-026".to_owned(),
                key: "cookie.session".to_owned(),
                stated: "SameSite=None".to_owned(),
                owned: "SameSite=Lax".to_owned(),
            }]
        );
        let admin = edited(
            "docs/security/identity-and-access.md",
            "| OIDC sign-in must use PKCE. |",
            "| OIDC sign-in must use PKCE, then sets the admin `__Host-` cookie (SameSite=Lax). |",
        );
        assert_eq!(
            check(&admin),
            [Finding::Conflict {
                path: "docs/security/identity-and-access.md".to_owned(),
                line: 6,
                id: "SEC-IAM-026".to_owned(),
                key: "session.admin".to_owned(),
                stated: "SameSite=Lax".to_owned(),
                owned: "SameSite=Strict".to_owned(),
            }]
        );
    }

    /// Verifies: SEC-TM-072
    #[test]
    fn a_restated_value_that_agrees_or_belongs_elsewhere_passes() {
        for text in [
            // The same value, at the end of a sentence.
            "Sign-in returns to `__Host-gm_session` with SameSite=Lax.",
            // Another attribute the table does not set.
            "Sign-in returns to `__Host-gm_session` with Max-Age=0.",
            // A different value in a sentence that does not name the cookie.
            "Sign-in returns to `__Host-gm_session`. Its own cookie is SameSite=None.",
            "Sign-in returns to `__Host-gm_session`; its own cookie is SameSite=None.",
            // The name without its backticks is prose, not the cookie.
            "Sign-in returns to __Host-gm_session with SameSite=None.",
        ] {
            let tree = edited(
                "docs/security/identity-and-access.md",
                "OIDC sign-in must use PKCE.",
                text,
            );
            assert_eq!(check(&tree), []);
        }
        // A withdrawn row may say anything.
        let withdrawn = edited(
            "docs/security/threat-model.md",
            "merged into SEC-IAM-025.**",
            "merged into SEC-IAM-025.** It was `__Host-gm_session` with SameSite=None.",
        );
        assert_eq!(check(&withdrawn), []);
    }

    /// Verifies: SEC-TM-073
    #[test]
    fn a_conflicting_csp_directive_fails() {
        let tree = edited(
            "docs/security/web-and-api-security.md",
            "Share links use the fragment pattern.",
            "Share pages use `img-src * data:; style-src 'self'` and allow `img-src`.",
        );
        assert_eq!(
            check(&tree),
            [Finding::Conflict {
                path: "docs/security/web-and-api-security.md".to_owned(),
                line: 8,
                id: "SEC-API-097".to_owned(),
                key: "Content Security Policy".to_owned(),
                stated: "img-src * data:".to_owned(),
                owned: "img-src 'self' blob:".to_owned(),
            }]
        );
        let unowned = tree.with(
            "docs/security/threat-model.md",
            &MODEL.replace(
                "| Content Security Policy | SEC-API-044 |",
                "| Content Security Policy | none yet |",
            ),
        );
        assert_eq!(check(&unowned), []);
    }

    /// A conflict the egress lint reports on SEC-API-097 of the fixture.
    fn egress_conflict(key: &str, stated: &str, owned: &str) -> Finding {
        Finding::Conflict {
            path: "docs/security/web-and-api-security.md".to_owned(),
            line: 8,
            id: "SEC-API-097".to_owned(),
            key: key.to_owned(),
            stated: stated.to_owned(),
            owned: owned.to_owned(),
        }
    }

    /// The fixture with `sentence` put in place of SEC-API-097's text.
    fn stating(sentence: &str) -> Memory {
        edited(
            "docs/security/web-and-api-security.md",
            "Share links use the fragment pattern.",
            sentence,
        )
    }

    /// The fixture's inventory holds the threat model's own purposes and
    /// defaults, and the sentences are shaped like the baseline's.
    ///
    /// Verifies: SEC-TM-075
    #[test]
    fn a_restated_egress_default_fails() {
        assert_eq!(
            check(&stating(
                "The scanner must skip hidden files; a metadata provider must be on by default when the library holds only music."
            )),
            [egress_conflict(
                "Metadata, artwork and lyrics providers",
                "on by default",
                "Off until the owner turns one on in the required setup step"
            )]
        );
        assert_eq!(
            check(&stating(
                "Lyrics providers must be On by default. Share links use the fragment pattern."
            )),
            [egress_conflict(
                "Metadata, artwork and lyrics providers",
                "on by default",
                "Off until the owner turns one on in the required setup step"
            )]
        );
        assert_eq!(
            check(&stating(
                "Outbound webhooks must be on by default for the owner."
            )),
            [egress_conflict(
                "Plugins and webhooks",
                "on by default",
                "Off; per grant or allowlisted host"
            )]
        );
        assert_eq!(
            check(&stating(
                "Until the sandbox exists, a Live TV source and the browser edge must be on by default."
            )),
            [
                egress_conflict(
                    "Relays, address lookup and the browser edge",
                    "on by default",
                    "Off until the owner turns on remote access"
                ),
                egress_conflict("Live TV sources", "on by default", "Off; per source"),
            ]
        );
        assert_eq!(
            check(&stating(
                "ACME certificate issuance must be off by default, and Certificate Transparency monitoring with it."
            )),
            [
                egress_conflict(
                    "ACME certificate issuance",
                    "off by default",
                    "On with project or own-domain naming; allowed before the claim"
                ),
                egress_conflict(
                    "Certificate Transparency monitoring",
                    "off by default",
                    "On with project naming, from the claim"
                ),
            ]
        );
        assert_eq!(
            check(&stating("DNS-01 updates must be off by default.")),
            [egress_conflict(
                "Naming: label registration and DNS-01 updates",
                "off by default",
                "On when the install chose the project name service (recommended, OD-1); allowed before the claim"
            )]
        );
    }

    #[test]
    fn an_egress_default_that_agrees_or_is_about_something_else_passes() {
        for sentence in [
            // The same default as the inventory.
            "A metadata provider must be off by default.",
            "Label registration is on by default.",
            // The purpose and the default are in different sentences.
            "Lyrics providers are listed in setup; transcoding there must be on by default.",
            "Transcoding must be on by default. Webhooks are signed.",
            // No purpose of the inventory: real sentences of the baseline.
            "Until that sandbox exists on a platform, transcoding there must be off by default, and the health page must say isolation is reduced.",
            "\"Now playing\" updates must be a separate setting that is off by default.",
            "It matches Jellyfin 12, which turns legacy authorisation off by default and removed those prefixes.",
            // Part of a listed thing is not the thing.
            "The address book and the browser must be on by default.",
            "Certificate checks and issuance records are off by default.",
            // A word that only ends in a default.
            "Webhooks use encryption by default.",
            "Webhooks are on by design.",
            "Webhooks turn on to default hosts.",
            // A row whose default is neither on nor off.
            "The advisory feed is off by default.",
            "OIDC discovery is on by default.",
        ] {
            assert_eq!(check(&stating(sentence)), []);
        }
    }

    /// Verifies: SEC-TM-001
    #[test]
    fn a_work_package_or_feature_file_with_no_threat_ids_fails() {
        let plan = repo().with(
            "docs/plan/work-packages.md",
            "### WP-001 Workspace\n\n- **Security.** Boundaries TB1.\n\n#### WP-002 Moved\n\nThreats TM-T01.\n### Waves\nTB1 TM-T01\n### WP-003\n",
        );
        let unmodelled = |path: &str, entry: &str, missing| Finding::Unmodelled {
            path: path.to_owned(),
            entry: entry.to_owned(),
            missing,
        };
        assert_eq!(
            check(&plan),
            [
                unmodelled("docs/plan/work-packages.md", "WP-001", "TM-T"),
                unmodelled("docs/plan/work-packages.md", "WP-002", "TB"),
                unmodelled("docs/plan/work-packages.md", "WP-003", "TB"),
                unmodelled("docs/plan/work-packages.md", "WP-003", "TM-T"),
            ]
        );
        let client = repo().with(
            "docs/plan/client-packages.md",
            "### CP-001 Workspace\n\n- **Security.** Boundaries TB1.\n\n### CP-002 Gate\n\nBoundaries TB1; threats TM-T01.\n\n### WP-900 Not a client package\n\n### CP-x Not a package\n",
        );
        assert_eq!(
            check(&client),
            [unmodelled("docs/plan/client-packages.md", "CP-001", "TM-T")]
        );
        let feature = edited(
            "docs/features/accounts.md",
            "Crosses TB1; threat TM-T01.",
            "None.",
        )
        .with("docs/features/music.md", "Threat TM-T01.\n");
        assert_eq!(
            check(&feature),
            [
                unmodelled(
                    "docs/features/accounts.md",
                    "docs/features/accounts.md",
                    "TB"
                ),
                unmodelled(
                    "docs/features/accounts.md",
                    "docs/features/accounts.md",
                    "TM-T"
                ),
                unmodelled("docs/features/music.md", "docs/features/music.md", "TB"),
            ]
        );
    }

    /// Verifies: SEC-STD-006, SEC-IAM-025
    #[test]
    fn a_password_row_while_no_passwords_is_live_fails() {
        let tree = edited(
            "docs/features/accounts.md",
            "| Password sign-in as a fallback | Passwords | All | High | No |",
            "| Password sign-in as a fallback | Passwords | All | High | R1 |",
        );
        assert_eq!(
            check(&tree),
            [
                Finding::CitesWithdrawn {
                    path: "docs/features/accounts.md".to_owned(),
                    line: 6,
                    id: "ACC-052".to_owned(),
                    cites: "SEC-TM-013".to_owned(),
                },
                Finding::PasswordsLive {
                    id: "ACC-052".to_owned(),
                },
            ]
        );
        let totp = edited(
            "docs/features/accounts.md",
            "| Two-factor codes | Codes | Plex | Medium | No |",
            "| Two-factor codes | Codes | Plex | Medium | R2 |",
        );
        assert_eq!(
            check(&totp),
            [Finding::PasswordsLive {
                id: "ACC-053".to_owned(),
            }]
        );
        // With no-passwords itself withdrawn, nothing contradicts it; the
        // withdrawn owner then turns up as a withdrawn citation.
        let both = repo().with(
            "docs/security/identity-and-access.md",
            &IDENTITY.replace(
                "| ASVS 6.1.3; A07:2025; CWE-521 | R1 |",
                "| ASVS 6.1.3; A07:2025; CWE-521 | Withdrawn |",
            ),
        );
        let without = both.with(
            "docs/features/accounts.md",
            &ACCOUNTS.replace("| Plex | Medium | No |", "| Plex | Medium | R2 |"),
        );
        assert_eq!(
            check(&without),
            [
                Finding::CitesWithdrawn {
                    path: "docs/security/web-and-api-security.md".to_owned(),
                    line: 5,
                    id: "SEC-API-001".to_owned(),
                    cites: "SEC-IAM-025".to_owned(),
                },
                Finding::CitesWithdrawn {
                    path: "docs/features/accounts.md".to_owned(),
                    line: 5,
                    id: "ACC-001".to_owned(),
                    cites: "SEC-IAM-025".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn standards_problems_are_reported_by_the_docs_lint() {
        let tree = edited(
            "docs/security/identity-and-access.md",
            "| ASVS 10.2.1 |",
            "| ASVS 10.2.9 |",
        );
        assert_eq!(
            check(&tree),
            [Finding::Standard(standards::Finding::Unknown {
                path: "docs/security/identity-and-access.md".to_owned(),
                line: 6,
                id: "SEC-IAM-026".to_owned(),
                citation: "ASVS 10.2.9".to_owned(),
            })]
        );
    }

    /// Verifies: SEC-TM-002
    #[test]
    fn the_last_reviewed_line_must_name_the_release_tag() {
        assert_eq!(reviewed(&repo(), "v0.1.0"), []);
        let line = "Last reviewed: 2026-10-02 (v0.1.0, the R1 baseline).";
        for tag in ["v0.2.0", "R1", "v0.1", "2026"] {
            assert_eq!(
                reviewed(&repo(), tag),
                [Finding::NotReviewed {
                    tag: tag.to_owned(),
                    line: Some(line.to_owned()),
                }]
            );
        }
        let last = edited(
            "docs/security/threat-model.md",
            "(v0.1.0, the R1 baseline).",
            "for v0.3.0.",
        );
        assert_eq!(reviewed(&last, "v0.3.0"), []);
        for tag in ["", " "] {
            assert_eq!(
                reviewed(&last, tag),
                [Finding::NotReviewed {
                    tag: tag.to_owned(),
                    line: Some("Last reviewed: 2026-10-02 for v0.3.0.".to_owned()),
                }]
            );
        }
        let missing = edited(
            "docs/security/threat-model.md",
            "Last reviewed:",
            "Reviewed:",
        );
        assert_eq!(
            reviewed(&missing, "v0.1.0"),
            [Finding::NotReviewed {
                tag: "v0.1.0".to_owned(),
                line: None,
            }]
        );
    }

    #[test]
    fn unreadable_markdown_and_odd_ids_are_skipped() {
        let extra = || {
            repo()
                .with("docs/security/ghost.md", "| SEC-API-001 | a | b | R1 | c |\n")
                .with(
                    "docs/features/notes.MD",
                    "| ACC-099 | X | X | X | X | R1 | X | X | X | SEC-IAM-025 |\nCrosses TB1; threat TM-T01.\n",
                )
                .with("docs/features/nested/README.md", "index\n")
        };
        let unmodelled = |missing| Finding::Unmodelled {
            path: "docs/features/notes.MD".to_owned(),
            entry: "docs/features/notes.MD".to_owned(),
            missing,
        };
        let hidden = Hide {
            inner: extra(),
            hidden: vec!["docs/security/ghost.md", "docs/features/notes.MD"],
        };
        assert_eq!(requirements(&hidden), requirements(&repo()));
        assert_eq!(check(&hidden), [unmodelled("TB"), unmodelled("TM-T")]);
        let no_model = Hide {
            inner: repo(),
            hidden: vec!["docs/security/threat-model.md"],
        };
        assert_eq!(
            requirements(&no_model)
                .iter()
                .map(|req| req.id.as_str())
                .collect::<Vec<_>>(),
            [
                "SEC-IAM-025",
                "SEC-IAM-026",
                "SEC-IAM-041",
                "SEC-API-001",
                "SEC-API-032",
                "SEC-API-044",
                "SEC-API-097",
                "SEC-API-023",
            ]
        );
        let unknown = |path: &str, line, id: &str| Finding::UnknownId {
            path: path.to_owned(),
            line,
            id: id.to_owned(),
        };
        assert_eq!(
            check(&no_model),
            [
                unknown("docs/features/README.md", 3, "SEC-TM-001"),
                unknown("docs/features/accounts.md", 6, "SEC-TM-013"),
                unknown("docs/features/accounts.md", 11, "TB1"),
                unknown("docs/features/accounts.md", 11, "TM-T01"),
                unknown("docs/plan/work-packages.md", 5, "TB1"),
                unknown("docs/plan/work-packages.md", 5, "TM-T01"),
                unknown("docs/plan/work-packages.md", 5, "SEC-TM-001"),
                unknown("docs/plan/work-packages.md", 9, "TB1"),
                unknown("docs/plan/work-packages.md", 9, "TM-T01"),
                unknown("CONTRIBUTING.md", 1, "SEC-TM-001"),
            ]
        );
        let no_plan = Hide {
            inner: repo(),
            hidden: vec!["docs/plan/work-packages.md"],
        };
        assert_eq!(check(&no_plan), []);
        let skipped = Memory::default().with(
            "docs/security/a.md",
            "| SEC-TM | a | b | R1 | c |\n| SEC-API-001 | a | b | R1 | c |\n",
        );
        assert_eq!(
            requirements(&skipped)
                .iter()
                .map(|req| req.id.as_str())
                .collect::<Vec<_>>(),
            ["SEC-API-001"]
        );
        let features = repo().with(
            "docs/features/extra.md",
            "| SEC-API-001 | X | X | X | X | R1 | X | X | X | SEC-IAM-025 |\nCrosses TB1; threat TM-T01.\n",
        );
        assert_eq!(check(&features), []);
        assert!(!super::is_sec_id("TM-T01"));
        assert!(!super::is_feature_id("SEC-API-001"));
    }

    #[test]
    fn ownership_defaults_and_token_edges_are_named() {
        let ownership = edited(
            "docs/security/threat-model.md",
            "| No passwords and no TOTP | SEC-IAM-025 | SEC-TM-013; SEC-API-001 cites it |",
            "| Two cells | SEC-IAM-025 |\n| No passwords and no TOTP | SEC-IAM-025 | SEC-ZZZ-001; SEC-TM-013; SEC-API-001 cites it |",
        );
        assert_eq!(
            check(&ownership),
            [Finding::UnknownId {
                path: "docs/security/threat-model.md".to_owned(),
                line: 30,
                id: "SEC-ZZZ-001".to_owned(),
            }]
        );
        let owner_default = edited(
            "docs/security/threat-model.md",
            "| SEC-TM-001 | Every feature file must name TB and TM-T IDs. |",
            "| SEC-TM-001 | Every feature file must name TB and TM-T IDs; metadata providers are on by default. |",
        );
        assert_eq!(check(&owner_default), []);
        let heading = edited(
            "docs/plan/work-packages.md",
            "## Waves",
            "### WP-\n### WP-x\n## Waves",
        );
        assert_eq!(check(&heading), []);
        let last = edited(
            "docs/security/threat-model.md",
            "Last reviewed: 2026-10-02 (v0.1.0, the R1 baseline).",
            "Last reviewed: 2026-10-02 v0.1.0",
        );
        assert_eq!(reviewed(&last, "v0.1.0"), []);
        let for_r1 = edited(
            "docs/security/threat-model.md",
            "Last reviewed: 2026-10-02 (v0.1.0, the R1 baseline).",
            "Last reviewed: 2026-10-02 for R1.",
        );
        assert_eq!(reviewed(&for_r1, "R1"), []);
        let same = edited(
            "docs/security/identity-and-access.md",
            "| OIDC sign-in must use PKCE. |",
            "| OIDC sign-in must use SameSite=. |",
        );
        assert_eq!(check(&same), []);
        let csp = edited(
            "docs/security/web-and-api-security.md",
            "Share links use the fragment pattern.",
            "Share links mention ximg-src 'none' and preimg-src 'none' and img-src-extra 'none' and bare img-src and img-src none.",
        );
        assert_eq!(check(&csp), []);
        let at_end = edited(
            "docs/security/web-and-api-security.md",
            "Share links use the fragment pattern.",
            "Share links end with img-src",
        );
        assert_eq!(check(&at_end), []);
        let no_csp = edited(
            "docs/security/threat-model.md",
            "| Content Security Policy | SEC-API-044 | SEC-API-001 cites it |\n",
            "",
        );
        assert_eq!(check(&no_csp), []);
    }

    #[test]
    fn helper_edges_are_named() {
        assert_eq!(super::sentences(""), Vec::<&str>::new());
        assert_eq!(super::sentences("done. "), ["done"]);
        assert_eq!(super::sentences(". done"), ["done"]);
        assert_eq!(super::sentences("; x"), ["x"]);
        assert_eq!(
            super::words("The server's DNS-01 updates, and The Edges"),
            ["server", "", "dn", "01", "update", "and", "edge"]
        );
        assert_eq!(
            super::purpose_terms("Relays, and the edge: , address lookup and keys"),
            [
                vec![String::from("relay")],
                vec![String::from("edge")],
                vec![String::from("addres"), String::from("lookup")],
                vec![String::from("key")],
            ]
        );
        assert_eq!(super::default_on("maybe later"), None);
        assert_eq!(super::default_on("..."), None);
        assert_eq!(super::default_on("123"), None);
        assert_eq!(super::heading_title("###WP-001"), None);
        assert_eq!(super::heading_title("not-a-heading"), None);
        assert_eq!(super::wp_id("", "WP-"), None);
        assert_eq!(super::wp_id("WP-", "WP-"), None);
        assert_eq!(super::wp_id("WP-x", "WP-"), None);
        assert_eq!(super::rest_from("ab", 5), "");
        assert_eq!(super::rest_from("", 0), "");
        assert_eq!(super::rest_after("", usize::MAX, 4), "");
    }

    #[test]
    fn validator_edges_are_named() {
        assert_eq!(super::default_on("On until asked"), Some(true));
        assert_eq!(super::default_on("Off until asked"), Some(false));
        assert!(super::is_token_edge(' '));
        assert!(super::is_token_edge(';'));
        assert!(!super::is_token_edge('a'));
        assert!(!super::is_token_edge('0'));
        assert!(!super::is_token_edge('.'));
        assert!(!super::is_token_edge('-'));
        assert_eq!(super::cells("|---|---|"), None);
        assert_eq!(
            super::cells("| a | b |").as_deref(),
            Some([String::from("a"), String::from("b")].as_slice())
        );
        assert!(super::is_sec_id("SEC-API-001"));
        assert!(!super::is_sec_id("SEC--001"));
        assert!(!super::is_sec_id("SEC-Aa-001"));
        assert!(!super::is_sec_id("SEC-API-"));
        assert!(!super::is_sec_id("SEC-API-0a"));
        assert!(super::is_feature_id("ACC-001"));
        assert!(!super::is_feature_id("-001"));
        assert!(!super::is_feature_id("Acc-001"));
        assert!(!super::is_feature_id("ACC-"));
        assert!(!super::is_feature_id("ACC-0a"));
        assert!(super::is_tb("TB1"));
        assert!(!super::is_tb("TB"));
        assert!(!super::is_tb("TBx"));
        assert!(super::is_tmt("TM-T01"));
        assert!(!super::is_tmt("TM-T"));
        assert!(!super::is_tmt("TM-Tx"));
        assert_eq!(
            super::parameters(
                "| cookie.session | Strict | before |\n### Security parameters\n| Key | Value | Why |\n| cookie.session | Lax | why |\n### Next\n",
            ),
            [(String::from("cookie.session"), String::from("Lax"))]
        );
        assert_eq!(
            super::egress_rows(
                "| Metadata providers | On | n | n | n | SEC-TM-001 |\n### Egress inventory\n| Purpose | Default | Trigger | Dest | Auth | Control |\n| Metadata providers | Off | n | n | n | SEC-TM-001 |\n### Next\n",
            )
            .into_iter()
            .map(|(purpose, default, _)| (purpose, default))
            .collect::<Vec<_>>(),
            [(String::from("Metadata providers"), String::from("Off"))]
        );
        assert_eq!(super::find_directive("ximg-src 'self'", "img-src"), None);
        assert_eq!(super::find_directive("img-srcx 'self'", "img-src"), None);
        assert_eq!(super::find_directive("img-src-'self'", "img-src"), None);
        assert_eq!(super::find_directive("-img-src 'self'", "img-src"), None);
        assert_eq!(
            super::find_directive("img-src 'self'", "img-src").as_deref(),
            Some("'self'")
        );
        assert_eq!(
            super::find_directive("img-src'self'", "img-src").as_deref(),
            Some("'self'")
        );
        assert_eq!(
            super::find_directive("img-src*", "img-src").as_deref(),
            Some("*")
        );
        assert_eq!(
            super::find_directive("img-src\t'self'", "img-src").as_deref(),
            Some("'self'")
        );
        assert_eq!(super::cited_ids("SEC-API-001"), ["SEC-API-001"]);
        assert_eq!(super::cited_ids("xSEC-API-001"), Vec::<String>::new());
    }

    #[test]
    fn the_subcommands_are_wired_to_the_checks() {
        let run = |args: &[&str]| {
            let args: Vec<String> = args.iter().map(|&arg| arg.to_owned()).collect();
            dispatch(&args, ROOT, 0, &[], &mut Vec::new())
        };
        assert_eq!(run(&["docs-lint"]), Ok(()));
        assert_eq!(
            run(&["last-reviewed", "not-a-tag"]),
            Err(Failure::Findings(vec![format!(
                "NotReviewed {{ tag: \"not-a-tag\", line: Some({:?}) }}",
                crate::tree::Tree::read(&crate::tree::Disk::new(ROOT), super::THREAT_MODEL)
                    .expect("the threat model is readable")
                    .lines()
                    .find(|line| line.starts_with("Last reviewed:"))
                    .expect("the threat model has the line")
            )]))
        );
    }
}
