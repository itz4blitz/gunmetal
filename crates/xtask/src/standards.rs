//! Pinned standard ID lists, citation checks, coverage and the version watch.
//!
//! [`citations`] fails when a requirement names an ID the pinned copy does
//! not have, or names a superseded edition (SEC-STD-001). [`coverage`] fails
//! when an ASVS item at or below its chapter target has neither a citing
//! requirement nor a complete register row, or when a review date has
//! expired (SEC-STD-002). [`watch`] compares recorded release feeds with the
//! pinned editions (SEC-STD-003).

use std::collections::{BTreeMap, BTreeSet};

use crate::docs_lint::{Requirement, rest_from};
use crate::tree::Tree;

/// Directory of the vendored ID lists.
pub const DIR: &str = "crates/xtask/standards";

/// The coverage file SEC-STD-002 regenerates against.
pub const COVERAGE: &str = "docs/security/standards-coverage.md";

/// The pinned ASVS list.
pub const ASVS: &str = "crates/xtask/standards/asvs-5.0.0.txt";

/// The pinned CWE list.
pub const CWE: &str = "crates/xtask/standards/cwe-4.20.txt";

/// The pinned Top 10 list.
pub const TOP10: &str = "crates/xtask/standards/top10-2025.txt";

/// The pinned API Top 10 list.
pub const API: &str = "crates/xtask/standards/api-top10-2023.txt";

/// The pinned MASVS list.
pub const MASVS: &str = "crates/xtask/standards/masvs-2.1.0.txt";

/// The pinned SSDF list.
pub const SSDF: &str = "crates/xtask/standards/ssdf-1.1.txt";

/// The pinned editions [`watch`] compares feeds against.
pub const PINNED: &[(&str, &str)] = &[
    ("asvs", "5.0.0"),
    ("top10", "2025"),
    ("api-top10", "2023"),
    ("masvs", "2.1.0"),
    ("cwe-top25", "2025"),
    ("ssdf", "1.1"),
];

/// Something wrong with a standards citation, coverage row or feed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finding {
    /// An ID the pinned version does not have.
    Unknown {
        /// The file.
        path: String,
        /// The line.
        line: usize,
        /// The requirement.
        id: String,
        /// The citation.
        citation: String,
    },
    /// A named edition the project has replaced.
    Superseded {
        /// The file.
        path: String,
        /// The line.
        line: usize,
        /// The requirement.
        id: String,
        /// The citation.
        citation: String,
        /// The pinned edition.
        pinned: String,
    },
    /// The coverage file is missing.
    MissingCoverage,
    /// An ASVS item at or below the chapter target is uncovered.
    Uncovered {
        /// The ASVS ID.
        asvs: String,
        /// The chapter target level.
        target: u8,
    },
    /// A register row is missing a required field.
    IncompleteRegister {
        /// The ASVS ID.
        asvs: String,
    },
    /// A register review date is older than one year before `now`.
    ExpiredReview {
        /// The ASVS ID.
        asvs: String,
        /// The recorded date, `YYYY-MM-DD`.
        date: String,
    },
    /// A feed names a newer final edition than the pin.
    Newer {
        /// The standard key.
        standard: String,
        /// The pinned edition.
        pinned: String,
        /// The newer edition in the feed.
        found: String,
    },
}

/// Citation problems in the Standards columns of `requirements`.
pub fn citations(tree: &dyn Tree, requirements: &[Requirement]) -> Vec<Finding> {
    let catalog = Catalog::load(tree);
    let mut findings = Vec::new();
    for req in requirements {
        for citation in parse_citations(&req.standards) {
            match catalog.status(&citation) {
                Status::Ok => {}
                Status::Unknown
                    if matches!(citation.kind, Kind::Ssdf)
                        && req.standards.to_ascii_lowercase().contains("draft") => {}
                Status::Unknown => findings.push(Finding::Unknown {
                    path: req.path.clone(),
                    line: req.line,
                    id: req.id.clone(),
                    citation: citation.display(),
                }),
                Status::Superseded(pinned) => findings.push(Finding::Superseded {
                    path: req.path.clone(),
                    line: req.line,
                    id: req.id.clone(),
                    citation: citation.display(),
                    pinned,
                }),
            }
        }
    }
    findings
}

/// Coverage of ASVS items at or below each chapter target, at time `now`.
pub fn coverage(tree: &dyn Tree, now: u64) -> Vec<Finding> {
    let Some(text) = tree.read(COVERAGE) else {
        return vec![Finding::MissingCoverage];
    };
    let catalog = Catalog::load(tree);
    let cited = cited_asvs(tree);
    let targets = chapter_targets(&text);
    let register = register_rows(&text);
    let mut findings = Vec::new();
    for (id, level) in &catalog.asvs_levels {
        let chapter = chapter_of(id);
        let Some(target) = targets.get(&chapter).copied() else {
            continue;
        };
        if *level > target {
            continue;
        }
        if cited.contains(id) {
            continue;
        }
        if let Some(row) = register.get(id) {
            if !row.complete() {
                findings.push(Finding::IncompleteRegister { asvs: id.clone() });
            }
            if let Some(date) = &row.review
                && expired(date, now)
            {
                findings.push(Finding::ExpiredReview {
                    asvs: id.clone(),
                    date: date.clone(),
                });
            }
            continue;
        }
        findings.push(Finding::Uncovered {
            asvs: id.clone(),
            target,
        });
    }
    findings
}

/// Newer final editions in the recorded feeds under `feeds`.
pub fn watch(tree: &dyn Tree, feeds: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    for name in tree.files(feeds) {
        let path = format!("{feeds}/{name}");
        let Some(text) = tree.read(&path) else {
            continue;
        };
        let Some((standard, pinned)) = standard_key(&name) else {
            continue;
        };
        for found in feed_versions(&text) {
            if is_draft(&found) {
                continue;
            }
            if newer(&found, pinned) {
                findings.push(Finding::Newer {
                    standard: standard.to_owned(),
                    pinned: pinned.to_owned(),
                    found,
                });
            }
        }
    }
    findings
}

/// IDs and levels from the vendored lists.
struct Catalog {
    asvs: BTreeSet<String>,
    asvs_levels: BTreeMap<String, u8>,
    cwe: BTreeSet<String>,
    top10: BTreeSet<String>,
    api: BTreeSet<String>,
    masvs: BTreeSet<String>,
    ssdf: BTreeSet<String>,
}

impl Catalog {
    fn load(tree: &dyn Tree) -> Self {
        debug_assert_eq!(ASVS, format!("{DIR}/asvs-5.0.0.txt"));
        let (asvs, asvs_levels) = load_asvs(tree);
        Self {
            asvs,
            asvs_levels,
            cwe: load_ids(tree, CWE),
            top10: load_ids(tree, TOP10),
            api: load_ids(tree, API),
            masvs: load_ids(tree, MASVS),
            ssdf: load_ids(tree, SSDF),
        }
    }

    fn status(&self, citation: &Citation) -> Status {
        if let Some(edition) = &citation.superseded {
            return Status::Superseded(edition.clone());
        }
        let known = match citation.kind {
            Kind::Asvs => self.asvs.contains(&citation.id),
            Kind::Cwe => self.cwe.contains(&citation.id),
            Kind::Top10 => self.top10.contains(&citation.id),
            Kind::Api => self.api.contains(&citation.id),
            Kind::Masvs => self.masvs.contains(&citation.id),
            Kind::Ssdf => self.ssdf.contains(&citation.id),
        };
        if known { Status::Ok } else { Status::Unknown }
    }
}

/// How a citation compares to the pin.
enum Status {
    Ok,
    Unknown,
    Superseded(String),
}

/// One citation of a pinned standard.
#[derive(Debug, Clone)]
struct Citation {
    kind: Kind,
    id: String,
    superseded: Option<String>,
}

impl Citation {
    fn display(&self) -> String {
        if let Some(edition) = &self.superseded {
            return edition.clone();
        }
        match self.kind {
            Kind::Asvs => format!("ASVS {}", self.id),
            Kind::Cwe => format!("CWE-{}", self.id),
            Kind::Top10 => format!("A{}", self.id.trim_start_matches('A')),
            Kind::Api | Kind::Masvs => self.id.clone(),
            Kind::Ssdf => format!("SSDF {}", self.id),
        }
    }
}

/// Which list a citation belongs to.
#[derive(Debug, Clone, Copy)]
enum Kind {
    Asvs,
    Cwe,
    Top10,
    Api,
    Masvs,
    Ssdf,
}

/// A deviation or not-applicable register row.
struct Register {
    reason: bool,
    compensating: bool,
    owner: bool,
    review: Option<String>,
}

impl Register {
    fn complete(&self) -> bool {
        self.reason && self.compensating && self.owner
    }
}

/// The first whitespace token of `line`, or the whole line.
fn first_token(line: &str) -> &str {
    first_token_from(line.split_whitespace().next(), line)
}

/// `token` if present, otherwise `line`.
fn first_token_from<'a>(token: Option<&'a str>, line: &'a str) -> &'a str {
    token.unwrap_or(line)
}

/// The first table cell, or an empty string.
fn first_cell(cells: &[String]) -> &str {
    cells.first().map_or("", String::as_str)
}

/// ASVS IDs and their levels.
fn load_asvs(tree: &dyn Tree) -> (BTreeSet<String>, BTreeMap<String, u8>) {
    let mut ids = BTreeSet::new();
    let mut levels = BTreeMap::new();
    for line in tree.read(ASVS).unwrap_or_default().lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let id = first_token_from(parts.next(), line);
        ids.insert(id.to_owned());
        if let Some(level) = parts.next().and_then(|n| n.parse().ok()) {
            levels.insert(id.to_owned(), level);
        }
    }
    (ids, levels)
}

/// First-field IDs from a vendored list.
fn load_ids(tree: &dyn Tree, path: &str) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for line in tree.read(path).unwrap_or_default().lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let id = first_token(line);
        ids.insert(id.to_owned());
    }
    ids
}

/// Citations of pinned standards in a Standards column.
fn parse_citations(standards: &str) -> Vec<Citation> {
    let mut found = Vec::new();
    found.extend(asvs_citations(standards));
    found.extend(cwe_citations(standards));
    found.extend(top10_citations(standards));
    found.extend(api_citations(standards));
    found.extend(masvs_citations(standards));
    found.extend(ssdf_citations(standards));
    found
}

/// ASVS numbers and superseded editions.
fn asvs_citations(standards: &str) -> Vec<Citation> {
    let mut found = Vec::new();
    let mut rest = standards;
    for _ in 0..standards.len() {
        let Some(at) = find_word(rest, "ASVS") else {
            break;
        };
        let after = rest_from(rest, at.saturating_add(4)).trim_start();
        let (superseded, after) = asvs_edition(after);
        if let Some(edition) = superseded {
            found.push(Citation {
                kind: Kind::Asvs,
                id: edition.clone(),
                superseded: Some(format!("ASVS {edition}")),
            });
        }
        for id in dotted_ids(after) {
            found.push(Citation {
                kind: Kind::Asvs,
                id,
                superseded: None,
            });
        }
        rest = after;
    }
    found
}

/// A superseded ASVS edition at the start of `after`, and the remainder.
fn asvs_edition(after: &str) -> (Option<String>, &str) {
    let token = after
        .split(|ch: char| ch == ',' || ch == ';' || ch.is_whitespace())
        .next()
        .unwrap_or("");
    let stripped = token.trim_start_matches('v');
    if is_asvs_edition(stripped) {
        let rest = after.get(token.len()..).unwrap_or(after);
        if matches!(stripped, "5" | "5.0" | "5.0.0") {
            return (None, rest);
        }
        return (Some(stripped.to_owned()), rest);
    }
    (None, after)
}

/// An ASVS *edition* (`4`, `4.0`, `4.0.3`), not a requirement number.
fn is_asvs_edition(token: &str) -> bool {
    let mut parts = token.split('.');
    let major = parts.next().unwrap_or("");
    if !matches!(major, "3" | "4" | "5") {
        return false;
    }
    match (parts.next(), parts.next(), parts.next()) {
        (None | Some("0"), None, None) => true,
        (Some("0"), Some(patch), None) => {
            !patch.is_empty() && patch.chars().all(|ch| ch.is_ascii_digit())
        }
        _ => false,
    }
}

/// Dotted `x.y.z` IDs until the next standard keyword or semicolon.
fn dotted_ids(after: &str) -> Vec<String> {
    let stop = after.find(';').unwrap_or(after.len());
    let span = after.get(..stop).unwrap_or(after);
    let mut ids = Vec::new();
    for token in span.split([',', ' ', '(', ')', '`']) {
        let token = token.trim();
        if is_asvs_id(token) {
            ids.push(token.to_owned());
        }
    }
    ids
}

/// An ASVS requirement number, not the 5.0.0 edition.
fn is_asvs_id(token: &str) -> bool {
    let mut parts = token.split('.');
    let a = parts.next().unwrap_or("");
    let Some(b) = parts.next() else {
        return false;
    };
    let Some(c) = parts.next() else {
        return false;
    };
    parts.next().is_none()
        && !a.is_empty()
        && !b.is_empty()
        && !c.is_empty()
        && a.chars().all(|ch| ch.is_ascii_digit())
        && b.chars().all(|ch| ch.is_ascii_digit())
        && c.chars().all(|ch| ch.is_ascii_digit())
        && token != "5.0.0"
}

/// `CWE-` numbers.
fn cwe_citations(standards: &str) -> Vec<Citation> {
    let mut found = Vec::new();
    let mut rest = standards;
    while let Some(at) = rest.find("CWE-") {
        let after = rest_from(rest, at.saturating_add(4));
        let id: String = after.chars().take_while(char::is_ascii_digit).collect();
        if !id.is_empty() {
            found.push(Citation {
                kind: Kind::Cwe,
                id,
                superseded: None,
            });
        }
        rest = after;
    }
    found
}

/// Top 10 category IDs, with a superseded year when it is not 2025.
fn top10_citations(standards: &str) -> Vec<Citation> {
    let mut found = Vec::new();
    if let Some(year) = top10_year(standards)
        && year != "2025"
    {
        found.push(Citation {
            kind: Kind::Top10,
            id: year.clone(),
            superseded: Some(format!("Top 10:{year}")),
        });
    }
    let mut rest = standards;
    for _ in 0..standards.len() {
        let Some(at) = find_a_id(rest) else {
            break;
        };
        let id = rest
            .get(at..at.saturating_add(3))
            .map_or("", |s| s)
            .to_owned();
        let after = rest_from(rest, at.saturating_add(3));
        let superseded = after
            .strip_prefix(':')
            .and_then(year_token)
            .filter(|year| *year != "2025")
            .map(|year| format!("{id}:{year}"));
        found.push(Citation {
            kind: Kind::Top10,
            id,
            superseded,
        });
        rest = after;
    }
    found
}

/// `Top 10:YYYY` if present.
fn top10_year(standards: &str) -> Option<String> {
    let after = standards.split("Top 10:").nth(1)?;
    year_token(after)
}

/// `API` category IDs.
fn api_citations(standards: &str) -> Vec<Citation> {
    let mut found = Vec::new();
    let mut rest = standards;
    while let Some(at) = rest.find("API") {
        let after = rest_from(rest, at.saturating_add(3));
        let number: String = after.chars().take_while(char::is_ascii_digit).collect();
        if number.is_empty() {
            rest = after;
            continue;
        }
        let id = format!("API{number}");
        let after_num = rest_from(after, number.len());
        let superseded = after_num
            .strip_prefix(':')
            .and_then(year_token)
            .filter(|year| *year != "2023")
            .map(|year| format!("{id}:{year}"));
        found.push(Citation {
            kind: Kind::Api,
            id,
            superseded,
        });
        rest = after_num;
    }
    found
}

/// MASVS control IDs.
fn masvs_citations(standards: &str) -> Vec<Citation> {
    let mut found = Vec::new();
    let mut rest = standards;
    while let Some(at) = rest.find("MASVS-") {
        let token: String = rest_from(rest, at)
            .chars()
            .take_while(|ch| {
                ch.is_ascii_uppercase() || ch.is_ascii_digit() || *ch == '-' || *ch == '.'
            })
            .collect();
        if token != "MASVS-" {
            if token.contains("1.") || token.ends_with("-1.0") {
                found.push(Citation {
                    kind: Kind::Masvs,
                    id: token.clone(),
                    superseded: Some(token),
                });
            } else {
                found.push(Citation {
                    kind: Kind::Masvs,
                    id: token,
                    superseded: None,
                });
            }
        }
        rest = rest_from(rest, at.saturating_add(6));
    }
    if standards.contains("MASVS 1") || standards.contains("MASVS v1") {
        found.push(Citation {
            kind: Kind::Masvs,
            id: "MASVS 1".to_owned(),
            superseded: Some("MASVS 1".to_owned()),
        });
    }
    found
}

/// SSDF practice and task IDs.
fn ssdf_citations(standards: &str) -> Vec<Citation> {
    let mut found = Vec::new();
    if standards.contains("SSDF 1.0") {
        found.push(Citation {
            kind: Kind::Ssdf,
            id: "1.0".to_owned(),
            superseded: Some("SSDF 1.0".to_owned()),
        });
    }
    for prefix in ["PO.", "PW.", "PS.", "RV."] {
        let mut rest = standards;
        while let Some(at) = rest.find(prefix) {
            let token: String = rest_from(rest, at)
                .chars()
                .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '.')
                .collect();
            if token.len() > prefix.len() {
                found.push(Citation {
                    kind: Kind::Ssdf,
                    id: token,
                    superseded: None,
                });
            }
            rest = rest_from(rest, at.saturating_add(prefix.len()));
        }
    }
    found
}

/// Byte index of word `needle` in `haystack`.
fn find_word(haystack: &str, needle: &str) -> Option<usize> {
    let mut rest = haystack;
    let mut base: usize = 0;
    for _ in 0..haystack.len() {
        let Some(at) = rest.find(needle) else {
            break;
        };
        let before = rest.get(..at).and_then(|h| h.chars().next_back());
        let after = rest
            .get(at.saturating_add(needle.len())..)
            .and_then(|t| t.chars().next());
        if before.is_none_or(|ch| !ch.is_ascii_alphanumeric())
            && after.is_none_or(|ch| !ch.is_ascii_alphanumeric())
        {
            return Some(base.saturating_add(at));
        }
        let step = at.saturating_add(1);
        base = base.saturating_add(step);
        rest = rest_from(rest, step);
    }
    None
}

/// Byte index of a Top 10 `A##` ID that is not `ASVS` or `API`.
fn find_a_id(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut i: usize = 0;
    while i.saturating_add(3) <= bytes.len() {
        let a = bytes.get(i).copied();
        let d1 = bytes.get(i.saturating_add(1)).copied();
        let d2 = bytes.get(i.saturating_add(2)).copied();
        if a == Some(b'A')
            && d1.is_some_and(|ch| ch.is_ascii_digit())
            && d2.is_some_and(|ch| ch.is_ascii_digit())
        {
            let before = if i == 0 {
                None
            } else {
                text.get(..i).and_then(|h| h.chars().next_back())
            };
            let after = text
                .get(i.saturating_add(3)..)
                .and_then(|t| t.chars().next());
            if before.is_none_or(|ch| !ch.is_ascii_alphanumeric())
                && after.is_none_or(|ch| !ch.is_ascii_alphanumeric())
            {
                return Some(i);
            }
        }
        i = i.saturating_add(1);
    }
    None
}

/// A four-digit year at the start of `text`.
fn year_token(text: &str) -> Option<String> {
    let year: String = text.chars().take(4).collect();
    if year.len() == 4 && year.chars().all(|ch| ch.is_ascii_digit()) {
        Some(year)
    } else {
        None
    }
}

/// ASVS IDs cited by any requirement.
fn cited_asvs(tree: &dyn Tree) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for req in crate::docs_lint::requirements(tree) {
        for citation in parse_citations(&req.standards) {
            if matches!(citation.kind, Kind::Asvs) && citation.superseded.is_none() {
                ids.insert(citation.id);
            }
        }
    }
    ids
}

/// Chapter target levels from the coverage file.
fn chapter_targets(text: &str) -> BTreeMap<u8, u8> {
    let mut targets = BTreeMap::new();
    for line in text.lines() {
        let Some(cells) = table_cells(line) else {
            continue;
        };
        let Some(chapter) = cells
            .first()
            .and_then(|c| c.strip_prefix('V'))
            .and_then(|n| n.split_whitespace().next())
        else {
            continue;
        };
        let Ok(number) = chapter.parse::<u8>() else {
            continue;
        };
        let Some(target_cell) = cells.get(2) else {
            continue;
        };
        if target_cell.contains("N/A") {
            continue;
        }
        if let Some(level) = target_level(target_cell) {
            targets.insert(number, level);
        }
    }
    targets
}

/// `L1`, `L2` or `L3` in a target cell.
fn target_level(cell: &str) -> Option<u8> {
    if cell.contains("L3") {
        Some(3)
    } else if cell.contains("L2") {
        Some(2)
    } else if cell.contains("L1") {
        Some(1)
    } else {
        None
    }
}

/// Register rows under "Recorded deviations" or a fixture table with those columns.
fn register_rows(text: &str) -> BTreeMap<String, Register> {
    let mut in_table = false;
    let mut rows = BTreeMap::new();
    let mut headers: Vec<String> = Vec::new();
    for line in text.lines() {
        if line.contains("Recorded deviations") || line.contains("### Register") {
            in_table = true;
            headers.clear();
            continue;
        }
        if in_table
            && line.starts_with("### ")
            && !line.contains("Register")
            && !line.contains("Recorded")
        {
            break;
        }
        let Some(cells) = table_cells(line) else {
            continue;
        };
        if !in_table {
            continue;
        }
        if headers.is_empty() {
            headers = cells;
            continue;
        }
        let asvs = first_cell(&cells).to_owned();
        if !is_asvs_id(&asvs) {
            continue;
        }
        let reason = cell_named(&headers, &cells, "Deviation").is_some_and(|c| !c.is_empty())
            || cell_named(&headers, &cells, "Reason").is_some_and(|c| !c.is_empty());
        let compensating =
            cell_named(&headers, &cells, "Compensating control").is_some_and(|c| !c.is_empty());
        let owner = cell_named(&headers, &cells, "Argued in").is_some_and(|c| !c.is_empty())
            || cell_named(&headers, &cells, "Owner").is_some_and(|c| !c.is_empty());
        let review = cell_named(&headers, &cells, "Review date")
            .or_else(|| cell_named(&headers, &cells, "Reviewed"))
            .filter(|c| !c.is_empty())
            .map(str::to_owned);
        rows.insert(
            asvs,
            Register {
                reason,
                compensating,
                owner,
                review,
            },
        );
    }
    rows
}

/// The cell of `headers` named `name`.
fn cell_named<'a>(headers: &[String], cells: &'a [String], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .position(|header| header == name)
        .and_then(|i| cells.get(i).map(String::as_str))
}

/// Markdown table cells.
fn table_cells(line: &str) -> Option<Vec<String>> {
    let body = line.trim().strip_prefix('|')?;
    let body = body.strip_suffix('|').unwrap_or(body);
    let parts: Vec<String> = body.split('|').map(str::trim).map(str::to_owned).collect();
    if parts.is_empty()
        || parts
            .iter()
            .all(|cell| !cell.is_empty() && cell.chars().all(|ch| matches!(ch, '-' | ':')))
    {
        None
    } else {
        Some(parts)
    }
}

/// The chapter number of an ASVS ID.
fn chapter_of(id: &str) -> u8 {
    id.split('.')
        .next()
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

/// Whether `date` is more than 365 days before `now`.
fn expired(date: &str, now: u64) -> bool {
    let Some(stamp) = date_seconds(date) else {
        return false;
    };
    now.saturating_sub(stamp) > 31_536_000
}

/// Midnight UTC of `YYYY-MM-DD`, in seconds.
fn parse_u64(part: Option<&str>) -> Option<u64> {
    part?.parse().ok()
}

fn date_seconds(date: &str) -> Option<u64> {
    let mut parts = date.split('-');
    let year: u64 = parse_u64(parts.next())?;
    let month: u64 = parse_u64(parts.next())?;
    let day: u64 = parse_u64(parts.next())?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || year < 1970 {
        return None;
    }
    Some(
        year.saturating_sub(1970)
            .saturating_mul(31_536_000)
            .saturating_add(month.saturating_sub(1).saturating_mul(2_628_000))
            .saturating_add(day.saturating_sub(1).saturating_mul(86_400)),
    )
}

/// The pinned standard encoded in a feed file name, with its edition.
fn standard_key(name: &str) -> Option<(&'static str, &'static str)> {
    PINNED
        .iter()
        .copied()
        .find(|(key, _)| name.starts_with(key))
}

/// Versions listed in a feed file.
fn feed_versions(text: &str) -> Vec<String> {
    let trimmed = text.trim_start();
    if trimmed.starts_with('[') {
        return json_tag_names(trimmed);
    }
    let mut versions = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        let Some(first) = line.chars().next() else {
            continue;
        };
        if first == '#' {
            continue;
        }
        if line.starts_with('{') {
            versions.extend(json_tag_names(line));
            continue;
        }
        let token = line.trim_start_matches('v');
        if token.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
            versions.push(token.to_owned());
        }
    }
    versions
}

/// `tag_name` values from a GitHub releases JSON array.
fn json_tag_names(text: &str) -> Vec<String> {
    let mut versions = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("\"tag_name\"") {
        let after = rest_from(rest, at.saturating_add(10));
        let Some(colon) = after.find(':') else {
            rest = after;
            continue;
        };
        let after = rest_from(after, colon.saturating_add(1)).trim_start();
        if let Some(value) = json_string(after) {
            versions.push(value.trim_start_matches('v').to_owned());
        }
        rest = after;
    }
    versions
}

/// A JSON string at the start of `text`.
fn json_string(text: &str) -> Option<String> {
    let rest = text.strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(ch) = chars.next() {
        if ch == '"' {
            return Some(out);
        }
        if ch == '\\' {
            if let Some(escaped) = chars.next() {
                out.push(escaped);
            }
            continue;
        }
        out.push(ch);
    }
    None
}

/// A pre-release marker.
fn is_draft(version: &str) -> bool {
    let lower = version.to_ascii_lowercase();
    lower.contains("draft") || lower.contains("rc") || lower.contains("beta")
}

/// Whether `found` is a newer edition than `pinned`.
fn newer(found: &str, pinned: &str) -> bool {
    compare_version(found, pinned) == std::cmp::Ordering::Greater
}

/// Numeric edition comparison, component by component.
fn compare_version(left: &str, right: &str) -> std::cmp::Ordering {
    let left_parts = version_parts(left);
    let right_parts = version_parts(right);
    let n = left_parts.len().max(right_parts.len());
    for i in 0..n {
        let l = left_parts.get(i).copied().unwrap_or(0);
        let r = right_parts.get(i).copied().unwrap_or(0);
        match l.cmp(&r) {
            std::cmp::Ordering::Equal => {}
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}

/// Numeric components of an edition string.
fn version_parts(version: &str) -> Vec<u32> {
    let mut parts = Vec::new();
    let mut current = String::new();
    for ch in version.chars() {
        if ch.is_ascii_digit() {
            current.push(ch);
        } else if !current.is_empty() {
            if let Ok(n) = current.parse() {
                parts.push(n);
            }
            current.clear();
        }
    }
    if !current.is_empty()
        && let Ok(n) = current.parse()
    {
        parts.push(n);
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::{Finding, PINNED, citations, coverage, watch};
    use crate::docs_lint::tests::{IDENTITY, edited, repo};
    use crate::tree::memory::Memory;

    /// The citation string a finding names, when it has one.
    fn citation_text(finding: &Finding) -> Option<&str> {
        match finding {
            Finding::Unknown { citation, .. } | Finding::Superseded { citation, .. } => {
                Some(citation)
            }
            Finding::MissingCoverage
            | Finding::Uncovered { .. }
            | Finding::IncompleteRegister { .. }
            | Finding::ExpiredReview { .. }
            | Finding::Newer { .. } => None,
        }
    }

    /// Verifies: SEC-STD-001
    #[test]
    fn an_unknown_asvs_number_fails() {
        let tree = edited(
            "docs/security/identity-and-access.md",
            "| ASVS 10.2.1 |",
            "| ASVS 10.2.9 |",
        );
        let reqs = crate::docs_lint::requirements(&tree);
        assert_eq!(
            citations(&tree, &reqs),
            [Finding::Unknown {
                path: "docs/security/identity-and-access.md".to_owned(),
                line: 6,
                id: "SEC-IAM-026".to_owned(),
                citation: "ASVS 10.2.9".to_owned(),
            }]
        );
    }

    /// Verifies: SEC-STD-001
    #[test]
    fn a_superseded_edition_fails() {
        let tree = edited(
            "docs/security/identity-and-access.md",
            "| ASVS 10.2.1 |",
            "| ASVS 4.0 10.2.1 |",
        );
        let reqs = crate::docs_lint::requirements(&tree);
        assert_eq!(
            citations(&tree, &reqs),
            [Finding::Superseded {
                path: "docs/security/identity-and-access.md".to_owned(),
                line: 6,
                id: "SEC-IAM-026".to_owned(),
                citation: "ASVS 4.0".to_owned(),
                pinned: "ASVS 4.0".to_owned(),
            }]
        );
        let year = edited(
            "docs/security/identity-and-access.md",
            "A07:2025",
            "A07:2021",
        );
        let reqs = crate::docs_lint::requirements(&year);
        let year_findings = citations(&year, &reqs);
        assert!(
            year_findings.iter().any(|finding| matches!(
                finding,
                Finding::Superseded { citation, .. } if citation.contains("2021")
            )),
            "{year_findings:?}"
        );
    }

    /// Verifies: SEC-STD-001
    #[test]
    fn the_fixture_standards_columns_are_known() {
        let tree = repo();
        assert_eq!(citations(&tree, &crate::docs_lint::requirements(&tree)), []);
        assert!(IDENTITY.contains("ASVS 6.1.3"));
    }

    /// Verifies: SEC-STD-002
    #[test]
    fn coverage_fails_without_the_file_or_an_uncovered_item() {
        assert_eq!(coverage(&repo(), 0), [Finding::MissingCoverage]);
        let tree = repo()
            .with(super::ASVS, "# ASVS\n15.1.5 3\n15.1.6 3\n15.1.1 2\n")
            .with(
                super::COVERAGE,
                "\
### Levels

| Chapter | Items (L1/L2/L3) | Target | Why | Uncited | After |
|---|---|---|---|---|---|
| V15 Encoding | 0/0/1 | **L3** | Host | 0 | |

### Register

| ASVS | Level | Deviation | Compensating control | Owner | Review date |
|---|---|---|---|---|---|
| 15.1.1 | L2 | none | none | docs | 2020-01-01 |
",
            );
        let findings = coverage(&tree, 1_800_000_000);
        assert!(
            findings.iter().any(|finding| matches!(
                finding,
                Finding::Uncovered { asvs, target: 3 } if asvs == "15.1.6"
            )),
            "{findings:?}"
        );
        assert!(
            findings.iter().any(|finding| matches!(
                finding,
                Finding::ExpiredReview { asvs, date } if asvs == "15.1.1" && date == "2020-01-01"
            )),
            "{findings:?}"
        );
    }

    /// Verifies: SEC-STD-002
    #[test]
    fn a_complete_register_row_covers_an_uncited_item() {
        let tree = repo().with(super::ASVS, "# ASVS\n15.1.6 3\n").with(
            super::COVERAGE,
            "\
| Chapter | Items | Target |
|---|---|---|
| V15 Encoding | 0 | **L3** |

### Register

| ASVS | Deviation | Compensating control | Owner | Review date |
|---|---|---|---|---|
| 15.1.6 | N/A | none needed | docs | 2026-10-01 |
",
        );
        let findings = coverage(&tree, 1_790_000_000);
        assert_eq!(findings, []);
    }

    /// Verifies: SEC-STD-002
    #[test]
    fn an_incomplete_register_row_fails() {
        let tree = repo().with(super::ASVS, "# ASVS\n15.1.6 3\n").with(
            super::COVERAGE,
            "\
| Chapter | Items | Target |
|---|---|---|
| V15 Encoding | 0 | **L3** |

### Register

| ASVS | Deviation | Compensating control | Owner |
|---|---|---|---|
| 15.1.6 |   |   |   |
",
        );
        let incomplete = coverage(&tree, 0);
        assert!(
            incomplete.iter().any(|finding| matches!(
                finding,
                Finding::IncompleteRegister { asvs } if asvs == "15.1.6"
            )),
            "{incomplete:?}"
        );
    }

    /// Verifies: SEC-STD-003
    #[test]
    fn a_newer_final_edition_fails_and_a_draft_does_not() {
        let tree = Memory::default()
            .with("feeds/asvs.txt", "5.0.0\n5.1.0\n")
            .with("feeds/ssdf.txt", "1.1\n1.2-draft\n")
            .with(
                "feeds/top10.json",
                r#"[{"tag_name":"2025"},{"tag_name":"2026"}]"#,
            );
        let findings = watch(&tree, "feeds");
        assert!(
            findings.contains(&Finding::Newer {
                standard: "asvs".to_owned(),
                pinned: "5.0.0".to_owned(),
                found: "5.1.0".to_owned(),
            }),
            "{findings:?}"
        );
        assert!(
            findings.contains(&Finding::Newer {
                standard: "top10".to_owned(),
                pinned: "2025".to_owned(),
                found: "2026".to_owned(),
            }),
            "{findings:?}"
        );
        assert_eq!(findings.len(), 2);
        assert_eq!(PINNED.len(), 6);
        assert!(super::DIR.ends_with("standards"));
    }

    /// Verifies: SEC-STD-001
    #[test]
    fn every_citation_kind_is_reported() {
        let kinds = repo().with(
            "docs/security/identity-and-access.md",
            &IDENTITY
                .replace(
                    "| ASVS 10.2.1 |",
                    "| CWE-999; A99; API9:2023; MASVS-; MASVS-AUTH-1; MASVS-STORAGE-1.0; MASVS 1; SSDF 1.0; PO.9.9; ASVS 4.0.3; Top 10:2021; A07:20 |",
                )
                .replace(
                    "| ASVS 6.1.3; A07:2025; CWE-521 |",
                    "| ASVS 6.1.3; A07:2025; CWE-521; draft SSDF ZZ.1 |",
                ),
        );
        let reqs = crate::docs_lint::requirements(&kinds);
        let findings = citations(&kinds, &reqs);
        for needle in [
            "CWE-999",
            "A99",
            "API9",
            "MASVS-AUTH-1",
            "MASVS-STORAGE-1.0",
            "MASVS 1",
            "SSDF 1.0",
            "SSDF PO.9.9",
            "ASVS 4.0.3",
            "Top 10:2021",
        ] {
            assert!(
                findings.iter().any(|finding| citation_text(finding)
                    .is_some_and(|citation| citation.contains(needle))),
                "{needle} in {findings:?}"
            );
        }
        assert!(
            !findings
                .iter()
                .any(|finding| citation_text(finding)
                    .is_some_and(|citation| citation.contains("ZZ.1"))),
            "{findings:?}"
        );
        assert_eq!(citation_text(&Finding::MissingCoverage), None);
        assert_eq!(
            citation_text(&Finding::Uncovered {
                asvs: "1.1.1".to_owned(),
                target: 1,
            }),
            None
        );
        assert_eq!(
            citation_text(&Finding::IncompleteRegister {
                asvs: "1.1.1".to_owned(),
            }),
            None
        );
        assert_eq!(
            citation_text(&Finding::ExpiredReview {
                asvs: "1.1.1".to_owned(),
                date: "2020-01-01".to_owned(),
            }),
            None
        );
        assert_eq!(
            citation_text(&Finding::Newer {
                standard: "asvs".to_owned(),
                pinned: "5.0.0".to_owned(),
                found: "5.1.0".to_owned(),
            }),
            None
        );
    }

    #[test]
    fn helper_edges_are_named() {
        assert!(!super::is_asvs_edition(""));
        assert!(!super::is_asvs_id(""));
        assert_eq!(super::chapter_of("x"), 0);
        assert_eq!(super::first_token(""), "");
        assert_eq!(super::first_token("  id 2"), "id");
        assert_eq!(super::first_token_from(None, "id"), "id");
        assert_eq!(super::first_cell(&[]), "");
        assert_eq!(super::first_cell(&[String::from("15.1.1")]), "15.1.1");
        let api = super::api_citations("API1:2024 API2:2023 API");
        assert_eq!(api.len(), 2);
        assert_eq!(api[0].superseded.as_deref(), Some("API1:2024"));
        assert_eq!(api[1].superseded, None);
        let rows = super::register_rows(
            "\
Recorded deviations

| ASVS | Reason | Compensating control | Argued in | Reviewed |
|---|---|---|---|---|
| 15.1.6 | why | how | docs |  |
| 15.1.1 | why | how | docs | 2026-10-01 |

### Later
",
        );
        let empty_review = rows.get("15.1.6").expect("15.1.6 is a register row");
        assert!(empty_review.reason && empty_review.compensating && empty_review.owner);
        assert_eq!(empty_review.review.as_deref(), None);
        assert_eq!(
            rows.get("15.1.1")
                .expect("15.1.1 is a register row")
                .review
                .as_deref(),
            Some("2026-10-01")
        );
        assert!(super::cwe_citations("CWE-").is_empty());
        assert!(super::ssdf_citations("PO.").is_empty());
        assert_eq!(super::parse_u64(None), None);
        assert_eq!(super::date_seconds(""), None);
        assert_eq!(super::date_seconds("2026"), None);
        assert_eq!(super::date_seconds("2026-10"), None);
        assert_eq!(super::date_seconds("xx-01-01"), None);
        assert_eq!(super::date_seconds("2026-aa-01"), None);
        assert_eq!(super::date_seconds("2026-10-aa"), None);
        assert_eq!(super::date_seconds("1969-10-01"), None);
        assert_eq!(super::feed_versions("nope\nv\n"), Vec::<String>::new());
        assert_eq!(super::json_string("\"abc\\"), None);
        assert_eq!(super::version_parts("99999999999x"), Vec::<u32>::new());
        assert_eq!(super::version_parts("1.x2"), vec![1, 2]);
        assert!(!super::is_asvs_edition("4.0."));
        assert!(!super::is_asvs_edition("4.0.x"));
        assert!(super::is_asvs_edition("4.0.3"));
        assert!(!super::is_asvs_id("1.2.3.4"));
        assert!(!super::is_asvs_id(".1.1"));
        assert!(!super::is_asvs_id("1..1"));
        assert!(!super::is_asvs_id("1.1."));
        assert!(!super::is_asvs_id("a.1.1"));
        assert!(!super::is_asvs_id("1.b.1"));
        assert!(!super::is_asvs_id("1.1.c"));
        assert!(!super::is_asvs_id("5.0.0"));
    }

    #[test]
    fn parse_edges_are_named() {
        assert_eq!(super::year_token("20x6"), None);
        assert_eq!(super::year_token("2026"), Some(String::from("2026")));
        assert!(super::is_draft("1.2-rc1"));
        assert!(!super::is_draft("1.2"));
        let stamp = super::date_seconds("2026-10-01").expect("2026-10-01 is a date");
        assert!(!super::expired("2026-10-01", stamp));
        assert!(!super::expired(
            "2026-10-01",
            stamp.saturating_add(31_536_000)
        ));
        assert!(super::expired(
            "2026-10-01",
            stamp.saturating_add(31_536_001)
        ));
        assert!(super::date_seconds("1970-01-01").is_some());
        assert_eq!(super::date_seconds("2026-13-01"), None);
        assert_eq!(super::date_seconds("2026-10-32"), None);
        assert_eq!(super::table_cells("|---|---|"), None);
        assert_eq!(
            super::table_cells("| a | b |").as_deref(),
            Some([String::from("a"), String::from("b")].as_slice())
        );
        let comments = Memory::default()
            .with(super::ASVS, "# note\n\n15.1.1 1\n")
            .with(super::CWE, "# note\n\n79\n");
        assert_eq!(
            super::load_asvs(&comments).0,
            [String::from("15.1.1")].into_iter().collect()
        );
        assert_eq!(
            super::load_ids(&comments, super::CWE),
            [String::from("79")].into_iter().collect()
        );
        assert_eq!(
            super::asvs_citations("ASVS 4.0,10.2.1")
                .iter()
                .filter_map(|c| c.superseded.as_deref())
                .collect::<Vec<_>>(),
            ["ASVS 4.0"]
        );
        assert!(
            super::masvs_citations("MASVS-AUTH-1.2")
                .iter()
                .any(|c| c.superseded.is_some())
        );
        let partial = super::register_rows(
            "\
### Register

| ASVS | Deviation | Compensating control | Owner | Review date |
|---|---|---|---|---|
| 15.1.6 | why |  | docs | 2026-10-01 |
",
        );
        assert!(!partial.get("15.1.6").expect("row").complete());
        let kept = super::register_rows(
            "\
Recorded deviations

| ASVS | Deviation | Compensating control | Owner | Review date |
|---|---|---|---|---|
| 15.1.6 | why | how | docs | 2026-10-01 |
### The Register
| 15.1.1 | why | how | docs | 2026-10-01 |
### The Recorded
| 15.2.1 | why | how | docs | 2026-10-01 |
",
        );
        assert!(kept.contains_key("15.1.1"));
        assert!(kept.contains_key("15.2.1"));
    }

    #[test]
    fn a_cited_asvs_item_covers_the_chapter() {
        let tree = repo().with(super::ASVS, "10.2.1 1\n").with(
            super::COVERAGE,
            "\
| Chapter | Items | Target |
|---|---|---|
| V10 Encoding | 0 | **L1** |
",
        );
        assert_eq!(coverage(&tree, 0), []);
        assert!(super::cited_asvs(&tree).contains("10.2.1"));
        let superseded = edited(
            "docs/security/identity-and-access.md",
            "| ASVS 10.2.1 |",
            "| ASVS 4.0 10.2.1 |",
        );
        assert!(!super::cited_asvs(&superseded).contains("4.0"));
        assert!(super::cited_asvs(&superseded).contains("10.2.1"));
    }

    /// Verifies: SEC-STD-002, SEC-STD-003
    #[test]
    fn coverage_rows_and_feeds_cover_the_edges() {
        let tree = repo()
            .with(
                super::ASVS,
                "# ASVS\n\n15.2.1 2\n15.1.6 1\n14.1.1 2\n16.1.1 1\nnot-an-id\n",
            )
            .with(
                super::COVERAGE,
                "\
# Coverage

| Skip | This | Row |
|---|---|---|
| Intro | text | here |

| Chapter | Items | Target |
|---|---|---|
| Overview | 0 | **L3** |
| Vxx | 0 | **L3** |
| V1 Encoding | 0 | N/A |
| V2 Encoding | 0 | **L2** |
| V3 Encoding | 0 | **L1** |
| V4 Encoding | 0 | none |
| V5 | only-two |
| V14 Encoding | 0 | **L1** |
| V15 Encoding | 0 | **L3** |

### Register

| ASVS | Deviation | Compensating control | Owner | Review date |
|---|---|---|---|---|
| 15.1.6 | why | how | docs | 2026-13-01 |
| not-an-id | why | how | docs | 2026-10-01 |

### After
",
            );
        let findings = coverage(&tree, 0);
        assert!(
            findings.iter().any(|finding| matches!(
                finding,
                Finding::Uncovered { asvs, target: 3 } if asvs == "15.2.1"
            )),
            "{findings:?}"
        );
        let hidden = crate::docs_lint::tests::Hide {
            inner: Memory::default()
                .with("feeds/asvs.txt", "5.0.0\n")
                .with("feeds/notes.txt", "ignore\n")
                .with(
                    "feeds/masvs.txt",
                    "# comment\n\nv2.2.0\n{\"tag_name\" no-colon}\n{\"tag_name\":2026}\n{\"tag_name\": \"2\\\"x\"}\n{\"tag_name\": \"unterminated\n",
                ),
            hidden: vec!["feeds/asvs.txt"],
        };
        let watched = watch(&hidden, "feeds");
        assert!(
            watched.iter().any(|finding| matches!(
                finding,
                Finding::Newer { standard, found, .. } if *standard == "masvs" && found == "2.2.0"
            )),
            "{watched:?}"
        );
    }

    /// Verifies: SEC-STD-003
    #[test]
    fn the_pinned_editions_are_not_newer_than_themselves() {
        let tree = Memory::default()
            .with("feeds/asvs.txt", "5.0.0\n")
            .with("feeds/top10.txt", "2025\n")
            .with("feeds/api-top10.txt", "2023\n")
            .with("feeds/masvs.txt", "2.1.0\n")
            .with("feeds/cwe-top25.txt", "2025\n")
            .with("feeds/ssdf.txt", "1.1\n");
        assert_eq!(watch(&tree, "feeds"), []);
    }
}
