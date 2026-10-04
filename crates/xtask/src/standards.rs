//! Pinned standard ID lists, citation checks, coverage and the version watch.
//!
//! [`citations`] fails when a requirement names an ID the pinned copy does
//! not have, or names a superseded edition (SEC-STD-001). [`coverage`] fails
//! when an ASVS item at or below its chapter target has neither a citing
//! requirement nor a register row that gives a reason, a compensating
//! control, an owner and a review date, or when that date has passed
//! (SEC-STD-002). [`watch`] compares recorded release feeds with the pinned
//! editions, and fails when a feed no longer names the pinned edition,
//! because such a feed could not show a newer one either (SEC-STD-003).
//!
//! No test here carries a `Verifies:` line for SEC-STD-002, so the
//! traceability check keeps it open: the requirement asks CI to regenerate
//! the coverage tables and fail, and [`coverage`] only checks them, runs in
//! no workflow and does not yet pass on the real coverage file.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use crate::docs_lint::{Requirement, cells, rest_from, section};
use crate::tree::Tree;

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

/// The heading of the chapter target table in [`COVERAGE`].
pub const TARGETS: &str = "### Target level per chapter";

/// The heading of the register of deviations and not-applicable items in
/// [`COVERAGE`].
pub const REGISTER: &str = "### Recorded deviations";

/// Seconds in a day.
const DAY: u64 = 86_400;

/// The day number of 1970-01-01, counting 0000-03-01 as day 1.
const EPOCH_DAY: u64 = 719_469;

/// The pinned editions [`watch`] compares feeds against. The feed of each
/// is the file `<key>.txt`.
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
    /// A register row lacks a reason, a compensating control, an owner or
    /// a review date.
    IncompleteRegister {
        /// The ASVS ID.
        asvs: String,
    },
    /// The day a register row was due for review has ended.
    ExpiredReview {
        /// The ASVS ID.
        asvs: String,
        /// The recorded date, `YYYY-MM-DD`.
        date: String,
    },
    /// A feed is missing or does not name the pinned edition, so it cannot
    /// be trusted to show a newer one.
    NoFeed {
        /// The standard key.
        standard: String,
        /// The pinned edition.
        pinned: String,
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
                        && after_draft(&req.standards, &citation.id) => {}
                Status::Unknown => findings.push(Finding::Unknown {
                    path: req.path.clone(),
                    line: req.line,
                    id: req.id.clone(),
                    citation: citation.display(),
                }),
                Status::Superseded => findings.push(Finding::Superseded {
                    path: req.path.clone(),
                    line: req.line,
                    id: req.id.clone(),
                    citation: citation.display(),
                }),
            }
        }
    }
    findings
}

/// Whether the Standards column names `id` after the word `draft`: a task
/// of the SSDF draft that the pinned edition does not have yet.
fn after_draft(standards: &str, id: &str) -> bool {
    standards
        .split_once("draft")
        .is_some_and(|(_, after)| after.contains(id))
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
        let Some(row) = register.get(id) else {
            findings.push(Finding::Uncovered {
                asvs: id.clone(),
                target,
            });
            continue;
        };
        match date_seconds(&row.review).filter(|_| row.complete) {
            None => findings.push(Finding::IncompleteRegister { asvs: id.clone() }),
            Some(due) if now >= due + DAY => findings.push(Finding::ExpiredReview {
                asvs: id.clone(),
                date: row.review.clone(),
            }),
            Some(_) => {}
        }
    }
    findings
}

/// Newer final editions in the recorded feeds under `feeds`, and feeds that
/// do not name the pinned edition.
pub fn watch(tree: &dyn Tree, feeds: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    for &(standard, pinned) in PINNED {
        let text = tree
            .read(&format!("{feeds}/{standard}.txt"))
            .unwrap_or_default();
        let mut live = false;
        for found in feed_editions(&text) {
            match compare_version(found, pinned) {
                Ordering::Less => {}
                Ordering::Equal => live = true,
                Ordering::Greater => findings.push(Finding::Newer {
                    standard: standard.to_owned(),
                    pinned: pinned.to_owned(),
                    found: found.to_owned(),
                }),
            }
        }
        if !live {
            findings.push(Finding::NoFeed {
                standard: standard.to_owned(),
                pinned: pinned.to_owned(),
            });
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
        if citation.superseded.is_some() {
            return Status::Superseded;
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
    Superseded,
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
    /// Whether it gives a reason, a compensating control and an owner.
    complete: bool,
    /// Its Review date cell: the day it is due for review.
    review: String,
}

/// The first whitespace token of `line`, or the whole line.
fn first_token(line: &str) -> &str {
    first_token_from(line.split_whitespace().next(), line)
}

/// `token` if present, otherwise `line`.
fn first_token_from<'a>(token: Option<&'a str>, line: &'a str) -> &'a str {
    token.unwrap_or(line)
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
    let stop = after
        .find(';')
        .into_iter()
        .chain(find_word(after, "ASVS"))
        .min()
        .unwrap_or(after.len());
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

/// MASVS control IDs, and the first edition named as one.
fn masvs_citations(standards: &str) -> Vec<Citation> {
    let mut found = Vec::new();
    let mut rest = standards;
    while let Some(at) = rest.find("MASVS-") {
        let token: String = rest_from(rest, at)
            .chars()
            .take_while(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || *ch == '-')
            .collect();
        found.push(Citation {
            kind: Kind::Masvs,
            id: token,
            superseded: None,
        });
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

/// ASVS IDs cited by a live requirement.
fn cited_asvs(tree: &dyn Tree) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for req in crate::docs_lint::requirements(tree)
        .iter()
        .filter(|req| req.is_live())
    {
        for citation in parse_citations(&req.standards) {
            if matches!(citation.kind, Kind::Asvs) && citation.superseded.is_none() {
                ids.insert(citation.id);
            }
        }
    }
    ids
}

/// Chapter target levels from the table under [`TARGETS`].
fn chapter_targets(text: &str) -> BTreeMap<u8, u8> {
    let mut targets = BTreeMap::new();
    for row in section(text, TARGETS).into_iter().filter_map(cells) {
        let chapter = row
            .first()
            .and_then(|cell| cell.strip_prefix('V'))
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|number| number.parse::<u8>().ok());
        let level = row.get(2).and_then(|cell| target_level(cell));
        if let (Some(chapter), Some(level)) = (chapter, level) {
            targets.insert(chapter, level);
        }
    }
    targets
}

/// The level a Target cell starts with. A chapter that is not applicable
/// has none.
fn target_level(cell: &str) -> Option<u8> {
    ["**L1**", "**L2**", "**L3**"]
        .iter()
        .zip(1..)
        .find(|(mark, _)| cell.starts_with(**mark))
        .map(|(_, level)| level)
}

/// The rows of the register under [`REGISTER`], by ASVS number. The first
/// row of the table names the columns.
fn register_rows(text: &str) -> BTreeMap<String, Register> {
    let mut rows = BTreeMap::new();
    let mut headers: Vec<String> = Vec::new();
    for row in section(text, REGISTER).into_iter().filter_map(cells) {
        if headers.is_empty() {
            headers = row;
            continue;
        }
        let cell = |name: &str| {
            headers
                .iter()
                .position(|header| header == name)
                .and_then(|at| row.get(at))
                .map_or("", String::as_str)
        };
        let register = Register {
            complete: !cell("Deviation").is_empty()
                && !cell("Compensating control").is_empty()
                && !cell("Owner").is_empty(),
            review: cell("Review date").to_owned(),
        };
        rows.insert(cell("ASVS").to_owned(), register);
    }
    rows
}

/// The chapter number of an ASVS ID.
fn chapter_of(id: &str) -> u8 {
    id.split('.')
        .next()
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

/// Midnight UTC at the start of `YYYY-MM-DD`, in seconds since the epoch,
/// for the years 1970 to 9999. A day number up to 31 is accepted in every
/// month and counts on into the next one.
pub(crate) fn date_seconds(date: &str) -> Option<u64> {
    let parts: Vec<Option<u64>> = date.split('-').map(|part| part.parse().ok()).collect();
    let [Some(year), Some(month), Some(day)] = *parts.as_slice() else {
        return None;
    };
    if !(1970..=9999).contains(&year) || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    // Count years from March, so a leap day is the last day of its year.
    let (y, m) = if month > 2 {
        (year, month - 3)
    } else {
        (year - 1, month + 9)
    };
    let days = y * 365 + y / 4 - y / 100 + y / 400 + (153 * m + 2) / 5 + day;
    Some((days - EPOCH_DAY) * DAY)
}

/// The editions a recorded feed names: one per line, with any leading `v`
/// removed, drafts left out.
fn feed_editions(text: &str) -> Vec<&str> {
    text.lines()
        .map(|line| line.trim().trim_start_matches('v'))
        .filter(|edition| !is_draft(edition))
        .collect()
}

/// A pre-release marker.
fn is_draft(version: &str) -> bool {
    let lower = version.to_ascii_lowercase();
    lower.contains("draft") || lower.contains("rc") || lower.contains("beta")
}

/// Numeric edition comparison, component by component.
fn compare_version(left: &str, right: &str) -> Ordering {
    let left_parts = version_parts(left);
    let right_parts = version_parts(right);
    let n = left_parts.len().max(right_parts.len());
    for i in 0..n {
        let l = left_parts.get(i).copied().unwrap_or(0);
        let r = right_parts.get(i).copied().unwrap_or(0);
        match l.cmp(&r) {
            Ordering::Equal => {}
            other => return other,
        }
    }
    Ordering::Equal
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
    use std::cmp::Ordering;

    use super::{Finding, citations, coverage, watch};
    use crate::docs_lint::tests::{IDENTITY, edited, repo};
    use crate::tree::memory::Memory;

    /// The fixture repository with `standards` as the Standards column of
    /// SEC-IAM-026, on line 6 of the identity file.
    fn citing(standards: &str) -> Memory {
        edited(
            "docs/security/identity-and-access.md",
            "| ASVS 10.2.1 |",
            &format!("| {standards} |"),
        )
    }

    /// What `citations` finds in `tree`.
    fn cited(tree: &Memory) -> Vec<Finding> {
        citations(tree, &crate::docs_lint::requirements(tree))
    }

    /// An unknown-ID finding on SEC-IAM-026.
    fn unknown(citation: &str) -> Finding {
        Finding::Unknown {
            path: "docs/security/identity-and-access.md".to_owned(),
            line: 6,
            id: "SEC-IAM-026".to_owned(),
            citation: citation.to_owned(),
        }
    }

    /// A superseded-edition finding on SEC-IAM-026.
    fn superseded(citation: &str) -> Finding {
        Finding::Superseded {
            path: "docs/security/identity-and-access.md".to_owned(),
            line: 6,
            id: "SEC-IAM-026".to_owned(),
            citation: citation.to_owned(),
        }
    }

    /// Verifies: SEC-STD-001
    #[test]
    fn an_unknown_asvs_number_fails() {
        assert_eq!(cited(&citing("ASVS 10.2.9")), [unknown("ASVS 10.2.9")]);
    }

    /// Verifies: SEC-STD-001
    #[test]
    fn an_id_the_pinned_list_does_not_have_fails_for_every_standard() {
        let tree = citing(
            "ASVS 10.2.9, 10.2.1; CWE-999; CWE-521; A99; A07:2025; API9:2023; API1:2023; MASVS-AUTH-1; MASVS-NETWORK-1; SSDF PO.9.9, PW.1.1",
        );
        assert_eq!(
            cited(&tree),
            [
                unknown("ASVS 10.2.9"),
                unknown("CWE-999"),
                unknown("A99"),
                unknown("API9"),
                unknown("MASVS-AUTH-1"),
                unknown("SSDF PO.9.9"),
            ]
        );
    }

    /// Verifies: SEC-STD-001
    #[test]
    fn a_superseded_edition_fails_for_every_standard() {
        let tree = citing(
            "ASVS 4.0.3 10.2.1; Top 10:2021 A01; A07:2021; API1:2019; MASVS 1; SSDF 1.0 PW.1.1; ASVS v5.0.0 10.2.1",
        );
        assert_eq!(
            cited(&tree),
            [
                superseded("ASVS 4.0.3"),
                superseded("Top 10:2021"),
                superseded("A07:2021"),
                superseded("API1:2019"),
                superseded("MASVS 1"),
                superseded("SSDF 1.0"),
            ]
        );
    }

    /// Verifies: SEC-STD-001
    #[test]
    fn the_fixture_standards_columns_are_known() {
        assert_eq!(cited(&repo()), []);
        assert!(IDENTITY.contains("ASVS 6.1.3"));
    }

    #[test]
    fn only_a_task_named_after_the_word_draft_may_be_missing_from_the_pinned_ssdf() {
        assert_eq!(
            cited(&citing("SSDF PW.1.1; SSDF 1.2 draft PO.6.1, PO.6.2")),
            []
        );
        assert_eq!(
            cited(&citing("SSDF RV.9.9; SSDF 1.2 draft PO.6.1")),
            [unknown("SSDF RV.9.9")]
        );
    }

    /// A coverage file whose register is `register`.
    fn coverage_file(register: &str) -> String {
        format!(
            "\
# Coverage

### Target level per chapter

| Chapter | Items (L1/L2/L3) | Target | Why |
|---|---|---|---|
| V6 Authentication | 1/0/0 | **L1** | sign-in |
| V13 Configuration | 0/1/1 | **L2** | configuration |
| V14 Data Protection | 1/1/0 | **L1** | little is stored |
| V15 Secure Coding | 0/1/2 | **L3** with deviations | the host |
| V16 Logging | 1/0/0 | **N/A** | none |
| Vx Odd | 0 | **L3** | none |
| V18 |

### Recorded deviations

Each is argued elsewhere.

{register}
### Section by section

| V13 Decoy | 0 | **L3** | not the target table |
| 15.1.6 | L3 | why | how | docs | 2026-10-02 |
"
        )
    }

    /// The fixture repository with a pinned ASVS list of `asvs`, a coverage
    /// file whose register is `register`, and the withdrawn SEC-TM-013
    /// citing ASVS 6.9.9.
    fn covering(asvs: &str, register: &str) -> Memory {
        edited(
            "docs/security/threat-model.md",
            "| ASVS 6.1.3 | Withdrawn |",
            "| ASVS 6.9.9 | Withdrawn |",
        )
        .with(super::ASVS, asvs)
        .with(super::COVERAGE, &coverage_file(register))
    }

    /// An uncovered-item finding.
    fn uncovered(asvs: &str, target: u8) -> Finding {
        Finding::Uncovered {
            asvs: asvs.to_owned(),
            target,
        }
    }

    #[test]
    fn an_item_at_or_below_its_target_needs_a_citation_or_a_register_row() {
        assert_eq!(coverage(&repo(), 0), [Finding::MissingCoverage]);
        let tree = covering(
            "# ASVS 5.0.0\n\n13.1.1 2\n13.1.2 3\n14.1.1 2\n14.1.2 1\n15.1.1 2\n15.1.5 3\n15.1.6 3\n16.1.1 1\n17.1.1 1\n6.1.3 1\n6.9.9 1\nx 1\nunlevelled\n",
            "\
| ASVS | Level | Deviation | Compensating control | Owner | Review date |
|---|---|---|---|---|---|
| 15.1.1 | L2 | why | how | docs | 2026-10-02 |
",
        );
        // The last second of 2026-10-02, the day the register row is due.
        assert_eq!(
            coverage(&tree, 1_790_985_599),
            [
                uncovered("13.1.1", 2),
                uncovered("14.1.2", 1),
                uncovered("15.1.6", 3),
                uncovered("6.9.9", 1),
            ]
        );
    }

    #[test]
    fn a_register_row_whose_review_day_has_ended_fails() {
        let tree = covering(
            "15.1.1 2\n",
            "\
| ASVS | Level | Deviation | Compensating control | Owner | Review date |
|---|---|---|---|---|---|
| 15.1.1 | L2 | why | how | docs | 2026-10-02 |
",
        );
        assert_eq!(coverage(&tree, 1_790_985_599), []);
        // Midnight at the start of 2026-10-03.
        assert_eq!(
            coverage(&tree, 1_790_985_600),
            [Finding::ExpiredReview {
                asvs: "15.1.1".to_owned(),
                date: "2026-10-02".to_owned(),
            }]
        );
    }

    /// An incomplete-register finding.
    fn incomplete(asvs: &str) -> Finding {
        Finding::IncompleteRegister {
            asvs: asvs.to_owned(),
        }
    }

    #[test]
    fn a_register_row_needs_a_reason_a_control_an_owner_and_a_review_date() {
        let asvs = "15.1.1 2\n15.1.2 2\n15.1.3 2\n15.1.4 2\n15.1.7 2\n15.1.8 2\n";
        let tree = covering(
            asvs,
            "\
| ASVS | Level | Deviation | Compensating control | Owner | Review date |
|---|---|---|---|---|---|
| 15.1.1 | L2 |  | how | docs | 2026-10-02 |
| 15.1.2 | L2 | why |  | docs | 2026-10-02 |
| 15.1.3 | L2 | why | how |  | 2026-10-02 |
| 15.1.4 | L2 | why | how | docs |  |
| 15.1.7 | L2 | why | how | docs | soon |
| 15.1.8 | L2 | why | how | docs | 2026-10-02 |
",
        );
        assert_eq!(
            coverage(&tree, 0),
            [
                incomplete("15.1.1"),
                incomplete("15.1.2"),
                incomplete("15.1.3"),
                incomplete("15.1.4"),
                incomplete("15.1.7"),
            ]
        );
        // A register with no Owner and no Review date column.
        let unowned = covering(
            "15.1.8 2\n",
            "\
| ASVS | Level | Deviation | Compensating control | Argued in |
|---|---|---|---|---|
| 15.1.8 | L2 | why | how | this file |
",
        );
        assert_eq!(coverage(&unowned, 0), [incomplete("15.1.8")]);
    }

    #[test]
    fn dates_are_midnight_utc_in_seconds_since_the_epoch() {
        // Each stamp is what `date -u -d <date> +%s` prints.
        for (date, stamp) in [
            ("1970-01-01", 0),
            ("1971-01-01", 31_536_000),
            ("1972-02-29", 68_169_600),
            ("1972-03-01", 68_256_000),
            ("1999-12-31", 946_598_400),
            ("2000-02-29", 951_782_400),
            ("2000-03-01", 951_868_800),
            ("2024-02-29", 1_709_164_800),
            ("2026-01-31", 1_769_817_600),
            ("2026-10-01", 1_790_812_800),
            ("2026-10-02", 1_790_899_200),
            ("2100-02-28", 4_107_456_000),
            ("2100-03-01", 4_107_542_400),
            ("2400-02-29", 13_574_563_200),
            ("9999-12-31", 253_402_214_400),
        ] {
            assert_eq!(super::date_seconds(date), Some(stamp));
        }
        for not_a_date in [
            "",
            "2026",
            "2026-10",
            "2026-10-01-01",
            "xx-01-01",
            "2026-aa-01",
            "2026-10-aa",
            "1969-12-31",
            "10000-01-01",
            "2026-00-01",
            "2026-13-01",
            "2026-10-00",
            "2026-10-32",
        ] {
            assert_eq!(super::date_seconds(not_a_date), None);
        }
    }

    /// A newer-edition finding.
    fn newer(standard: &str, pinned: &str, found: &str) -> Finding {
        Finding::Newer {
            standard: standard.to_owned(),
            pinned: pinned.to_owned(),
            found: found.to_owned(),
        }
    }

    /// A finding that a feed does not name the pinned edition.
    fn no_feed(standard: &str, pinned: &str) -> Finding {
        Finding::NoFeed {
            standard: standard.to_owned(),
            pinned: pinned.to_owned(),
        }
    }

    /// Verifies: SEC-STD-003
    #[test]
    fn the_pinned_editions_are_not_newer_than_themselves() {
        let tree = Memory::default()
            .with("feeds/asvs.txt", "latest\nv5.0.0_release\nv4.0.3_release\n")
            .with("feeds/top10.txt", "2021\n2025\n")
            .with("feeds/api-top10.txt", "2023\n")
            .with("feeds/masvs.txt", "v2.1.0\nv2.0.0\n")
            .with("feeds/cwe-top25.txt", "2025\n")
            .with("feeds/ssdf.txt", "1.1\n");
        assert_eq!(watch(&tree, "feeds"), []);
    }

    /// Verifies: SEC-STD-003
    #[test]
    fn a_newer_final_edition_fails_and_a_draft_does_not() {
        let tree = Memory::default()
            .with(
                "feeds/asvs.txt",
                "v5.0.0_release\nv5.1.0_release\nv6.0.0-rc1\nv5.2.0-beta\n",
            )
            .with("feeds/top10.txt", "2025\n2026\n")
            .with("feeds/api-top10.txt", "2023\n")
            .with("feeds/masvs.txt", "v2.1.0\n")
            .with("feeds/cwe-top25.txt", "2025\n")
            .with("feeds/ssdf.txt", "1.1\n1.2-draft\n");
        assert_eq!(
            watch(&tree, "feeds"),
            [
                newer("asvs", "5.0.0", "5.1.0_release"),
                newer("top10", "2025", "2026"),
            ]
        );
    }

    /// Verifies: SEC-STD-003
    #[test]
    fn a_feed_that_does_not_name_the_pinned_edition_fails() {
        assert_eq!(
            watch(&Memory::default(), "feeds"),
            [
                no_feed("asvs", "5.0.0"),
                no_feed("top10", "2025"),
                no_feed("api-top10", "2023"),
                no_feed("masvs", "2.1.0"),
                no_feed("cwe-top25", "2025"),
                no_feed("ssdf", "1.1"),
            ]
        );
        let tree = Memory::default()
            .with("feeds/asvs.txt", "v5.0.0_release\n")
            .with("feeds/top10.txt", "2025\n")
            .with("feeds/api-top10.txt", "2019\n")
            .with("feeds/masvs.json", "v2.1.0\n")
            .with("feeds/cwe-top25.txt", "2026\n")
            .with("feeds/ssdf.txt", "\n");
        assert_eq!(
            watch(&tree, "feeds"),
            [
                no_feed("api-top10", "2023"),
                no_feed("masvs", "2.1.0"),
                newer("cwe-top25", "2025", "2026"),
                no_feed("cwe-top25", "2025"),
                no_feed("ssdf", "1.1"),
            ]
        );
    }

    #[test]
    fn editions_compare_number_by_number() {
        assert_eq!(super::compare_version("5.0", "5.0.0"), Ordering::Equal);
        assert_eq!(super::compare_version("5.0.1", "5.0"), Ordering::Greater);
        assert_eq!(super::compare_version("4.9", "5"), Ordering::Less);
        assert_eq!(super::compare_version("10", "9"), Ordering::Greater);
        assert_eq!(super::version_parts("99999999999x"), Vec::<u32>::new());
        assert_eq!(super::version_parts("1.99999999999"), vec![1]);
        assert_eq!(super::version_parts("1.x2"), vec![1, 2]);
        assert!(super::is_draft("1.2-rc1"));
        assert!(super::is_draft("1.2 Draft"));
        assert!(super::is_draft("1.2-BETA"));
        assert!(!super::is_draft("1.2"));
    }

    #[test]
    fn helper_edges_are_named() {
        assert!(!super::is_asvs_edition(""));
        assert!(!super::is_asvs_id(""));
        assert_eq!(super::chapter_of("x"), 0);
        assert_eq!(super::chapter_of("15.1.1"), 15);
        assert_eq!(super::first_token(""), "");
        assert_eq!(super::first_token("  id 2"), "id");
        assert_eq!(super::first_token_from(None, "id"), "id");
        let api = super::api_citations("API1:2024 API2:2023 API");
        assert_eq!(api.len(), 2);
        assert_eq!(api[0].superseded.as_deref(), Some("API1:2024"));
        assert_eq!(api[1].superseded, None);
        assert!(super::cwe_citations("CWE-").is_empty());
        assert!(super::ssdf_citations("PO.").is_empty());
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
        assert_eq!(super::target_level("**L1**"), Some(1));
        assert_eq!(super::target_level("**L2** in R1"), Some(2));
        assert_eq!(super::target_level("**L3**"), Some(3));
        assert_eq!(super::target_level("**N/A**"), None);
        assert_eq!(super::target_level("see **L3**"), None);
    }

    #[test]
    fn parse_edges_are_named() {
        assert_eq!(super::year_token("20x6"), None);
        assert_eq!(super::year_token("2026"), Some(String::from("2026")));
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
        assert_eq!(
            super::asvs_citations("ASVS 1.1.1, ASVS 9.9.9, 2.2.2; ASVS 3.3.3")
                .iter()
                .map(|c| (c.id.as_str(), c.superseded.as_deref()))
                .collect::<Vec<_>>(),
            [
                ("1.1.1", None),
                ("9.9.9", None),
                ("2.2.2", None),
                ("3.3.3", None),
            ]
        );
        assert_eq!(
            super::masvs_citations("MASVS-AUTH-1.2; MASVS-; MASVS v1")
                .iter()
                .map(|c| (c.id.as_str(), c.superseded.as_deref()))
                .collect::<Vec<_>>(),
            [
                ("MASVS-AUTH-1", None),
                ("MASVS-", None),
                ("MASVS 1", Some("MASVS 1")),
            ]
        );
        assert!(!super::after_draft("SSDF PO.6.1", "PO.6.1"));
        assert!(!super::after_draft("PO.6.1 draft", "PO.6.1"));
    }

    #[test]
    fn a_cited_asvs_item_covers_it_unless_the_citing_row_is_withdrawn() {
        let tree = covering("6.1.3 1\n6.9.9 1\n", "");
        assert_eq!(
            super::cited_asvs(&tree),
            [
                "10.2.1", "15.1.5", "3.3.1", "3.4.3", "6.1.3", "7.3.1", "8.2.2"
            ]
            .into_iter()
            .map(String::from)
            .collect()
        );
        assert_eq!(coverage(&tree, 0), [uncovered("6.9.9", 1)]);
        let old_edition = citing("ASVS 4.0 10.2.1");
        assert!(!super::cited_asvs(&old_edition).contains("4.0"));
        assert!(super::cited_asvs(&old_edition).contains("10.2.1"));
    }
}
