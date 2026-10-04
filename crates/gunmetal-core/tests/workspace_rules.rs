//! The rules every crate in the workspace compiles under, checked against
//! the files that define them.
//!
//! The gate enforces these files: Clippy reads `clippy.toml` and
//! `crates/gunmetal-core/clippy.toml`, cargo-deny reads `deny.toml`, cargo-vet
//! reads `supply-chain/`, and CI runs `scripts/gate.sh`. These tests pin what
//! the files must say, so that no rule can be dropped or loosened without a
//! failing test naming the requirement it proves. A rule that a file states
//! but nothing enforces would still pass here, which is why the pull request
//! that adds a rule also shows the gate refusing a change that breaks it.
//!
//! Every file is read at compile time, so the tests do no I/O; the one test
//! that needs today's date reads the clock. They live with the core, the one
//! crate every package builds on, until the repository's xtask crate exists
//! (WP-008).

use std::time::{SystemTime, UNIX_EPOCH};

const WORKSPACE_MANIFEST: &str = include_str!("../../../Cargo.toml");
const CORE_MANIFEST: &str = include_str!("../Cargo.toml");
const WORKSPACE_CLIPPY: &str = include_str!("../../../clippy.toml");
const CORE_CLIPPY: &str = include_str!("../clippy.toml");
const DENY: &str = include_str!("../../../deny.toml");
const TOOLCHAIN: &str = include_str!("../../../rust-toolchain.toml");
const GATE: &str = include_str!("../../../scripts/gate.sh");
const CI: &str = include_str!("../../../.github/workflows/ci.yml");
const DAILY_DENY: &str = include_str!("../../../.github/workflows/supply-chain.yml");
const VET_CONFIG: &str = include_str!("../../../supply-chain/config.toml");
const EXCEPTIONS: &str = include_str!("../../../supply-chain/exceptions.toml");
const CORE_ALLOWLIST: &str = include_str!("../../../supply-chain/core-allowlist.toml");

// ---------------------------------------------------------------------------
// Reading the files. Each file keeps one entry per line, so a line reader is
// enough, and it shares no code with the tools that enforce the files.
// ---------------------------------------------------------------------------

/// One inline table in a TOML array written one entry per line, such as
/// `{ path = "std::fs::read", reason = "... (SEC-MED-033)" }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Entry<'a> {
    /// The value of the entry's first key: a path or a crate name.
    name: &'a str,
    /// The entry's reason, which names the requirements it serves.
    reason: &'a str,
}

/// Parses one entry line, or returns `None` if it is not one.
fn entry(line: &str) -> Option<Entry<'_>> {
    let (_, rest) = line.trim().strip_prefix('{')?.split_once(" = \"")?;
    let (name, rest) = rest.split_once('"')?;
    let (_, reason) = rest.split_once("reason = \"")?;
    let (reason, _) = reason.split_once('"')?;
    Some(Entry { name, reason })
}

/// The lines of the TOML table `[header]`, up to the next table header. An
/// empty header means the lines before the first table.
fn table<'a>(toml: &'a str, header: &str) -> Vec<&'a str> {
    let opening = format!("[{header}]");
    let lines = toml.lines();
    if header.is_empty() {
        return lines.take_while(|line| !line.starts_with('[')).collect();
    }
    lines
        .skip_while(|line| *line != opening)
        .skip(1)
        .take_while(|line| !line.starts_with('['))
        .collect()
}

/// The lines between `key = [` and the closing `]` among `lines`.
fn array<'a>(lines: &[&'a str], key: &str) -> Vec<&'a str> {
    let opening = format!("{key} = [");
    lines
        .iter()
        .copied()
        .skip_while(|line| *line != opening)
        .skip(1)
        .take_while(|line| *line != "]")
        .collect()
}

/// The lines that start an inline table, whether or not they parse.
fn entry_lines<'a>(lines: &[&'a str]) -> Vec<&'a str> {
    lines
        .iter()
        .copied()
        .filter(|line| line.trim_start().starts_with('{'))
        .collect()
}

/// Every entry in `lines`, skipping comments.
fn entries<'a>(lines: &[&'a str]) -> Vec<Entry<'a>> {
    entry_lines(lines).into_iter().filter_map(entry).collect()
}

/// Whether `reason` cites requirement `id` as a whole word.
fn cites(reason: &str, id: &str) -> bool {
    reason
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .any(|word| word == id)
}

/// The names of the entries whose reason cites `id`, sorted.
fn citing<'a>(entries: &[Entry<'a>], id: &str) -> Vec<&'a str> {
    let mut names: Vec<&str> = entries
        .iter()
        .filter(|found| cites(found.reason, id))
        .map(|found| found.name)
        .collect();
    names.sort_unstable();
    names
}

/// `names`, sorted, so expected lists can be written in any order.
fn sorted<'a>(names: &[&'a str]) -> Vec<&'a str> {
    let mut names = names.to_vec();
    names.sort_unstable();
    names
}

/// The lines of `lines` that are neither blank nor comments, trimmed.
fn rules<'a>(lines: &[&'a str]) -> Vec<&'a str> {
    lines
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

/// The value of `key = value` among `lines`, as written.
fn value<'a>(lines: &[&'a str], key: &str) -> Option<&'a str> {
    let prefix = format!("{key} = ");
    lines
        .iter()
        .find_map(|line| line.trim().strip_prefix(prefix.as_str()))
}

/// Whether `text` holds `line` as a whole line, ignoring indentation.
fn has_line(text: &str, line: &str) -> bool {
    text.lines().any(|candidate| candidate.trim() == line)
}

/// The quoted strings in `text`, in order.
fn quoted(text: &str) -> Vec<&str> {
    text.split('"').skip(1).step_by(2).collect()
}

fn workspace_methods() -> Vec<Entry<'static>> {
    entries(&array(&table(WORKSPACE_CLIPPY, ""), "disallowed-methods"))
}

fn workspace_types() -> Vec<Entry<'static>> {
    entries(&array(&table(WORKSPACE_CLIPPY, ""), "disallowed-types"))
}

fn denied_crates() -> Vec<Entry<'static>> {
    entries(&array(&table(DENY, "bans"), "deny"))
}

/// The crates `deny.toml` lets build scripts run for.
fn build_script_crates(deny: &str) -> Vec<&str> {
    array(&table(deny, "bans.build"), "allow-build-scripts")
        .into_iter()
        .flat_map(quoted)
        .collect()
}

/// The advisories `deny.toml` ignores, whether written on one line or many.
fn ignored_advisories(deny: &str) -> Vec<&str> {
    let advisories = table(deny, "advisories");
    let listed = array(&advisories, "ignore");
    let inline = value(&advisories, "ignore").unwrap_or_default();
    let mut ids: Vec<&str> = quoted(inline)
        .into_iter()
        .chain(
            listed
                .iter()
                .filter_map(|line| quoted(line).first().copied()),
        )
        .collect();
    ids.sort_unstable();
    ids
}

/// The advisory IDs `exceptions.toml` records, one `id = "..."` line each.
fn excepted_advisories(exceptions: &str) -> Vec<&str> {
    let mut ids: Vec<&str> = exceptions
        .lines()
        .filter_map(|line| line.strip_prefix("id = "))
        .flat_map(quoted)
        .collect();
    ids.sort_unstable();
    ids
}

/// The ignored advisories with no exception, and the exceptions with no
/// ignored advisory.
fn unmatched_advisories<'a>(deny: &'a str, exceptions: &'a str) -> (Vec<&'a str>, Vec<&'a str>) {
    let ignored = ignored_advisories(deny);
    let excepted = excepted_advisories(exceptions);
    (
        ignored
            .iter()
            .copied()
            .filter(|id| !excepted.contains(id))
            .collect(),
        excepted
            .iter()
            .copied()
            .filter(|id| !ignored.contains(id))
            .collect(),
    )
}

/// Each table whose header line starts with `opening`, as its header and the
/// lines up to the next header. Commented-out headers do not count.
fn blocks<'a>(toml: &'a str, opening: &str) -> Vec<(&'a str, Vec<&'a str>)> {
    let lines: Vec<&str> = toml.lines().collect();
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.starts_with(opening))
        .map(|(at, header)| {
            let body = lines
                .iter()
                .skip(at)
                .skip(1)
                .take_while(|line| !line.starts_with('['))
                .copied()
                .collect();
            (*header, body)
        })
        .collect()
}

/// Whether `field` is exactly `width` ASCII digits.
fn digits(field: &str, width: usize) -> bool {
    field.len() == width && field.bytes().all(|byte| byte.is_ascii_digit())
}

/// The days from 1970-01-01 to a `YYYY-MM-DD` date, or `None` if `date` is
/// not a real one. Howard Hinnant's `days_from_civil`, counting years from
/// March so that a leap day falls at the end of one.
fn days_since_epoch(date: &str) -> Option<i64> {
    let mut fields = date.split('-');
    let (year, month, day) = (fields.next()?, fields.next()?, fields.next()?);
    if fields.next().is_some() || !(digits(year, 4) && digits(month, 2) && digits(day, 2)) {
        return None;
    }
    let (year, month, day): (i64, i64, i64) =
        (year.parse().ok()?, month.parse().ok()?, day.parse().ok()?);
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let february = if leap { 29 } else { 28 };
    let lengths = [31, february, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let length = *lengths.get(usize::try_from(month.checked_sub(1)?).ok()?)?;
    if !(1..=length).contains(&day) {
        return None;
    }
    let (year, month) = if month <= 2 {
        (year.checked_sub(1)?, month.checked_add(9)?)
    } else {
        (year, month.checked_sub(3)?)
    };
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let day_of_year = month
        .checked_mul(153)?
        .checked_add(2)?
        .checked_div(5)?
        .checked_add(day)?
        .checked_sub(1)?;
    let day_of_era = year_of_era
        .checked_mul(365)?
        .checked_add(year_of_era.checked_div(4)?)?
        .checked_sub(year_of_era.checked_div(100)?)?
        .checked_add(day_of_year)?;
    era.checked_mul(146_097)?
        .checked_add(day_of_era)?
        .checked_sub(719_468)
}

/// Whether `id` is a `RustSec` advisory ID, `RUSTSEC-YYYY-NNNN`.
fn is_advisory_id(id: &str) -> bool {
    let mut parts = id.split('-');
    parts.next() == Some("RUSTSEC")
        && parts.next().is_some_and(|year| digits(year, 4))
        && parts.next().is_some_and(|number| digits(number, 4))
        && parts.next().is_none()
}

/// The exceptions SEC-SUP-022 refuses on day `today` (days since
/// 1970-01-01), by advisory ID, each with the first thing wrong with it: a
/// missing field, an ID that is not `RUSTSEC-YYYY-NNNN`, or a review-by date
/// that is not a date, has passed, or is more than 90 days ahead.
fn refused_exceptions(exceptions: &str, today: i64) -> Vec<(&str, &'static str)> {
    blocks(exceptions, "[[advisory]]")
        .into_iter()
        .filter_map(|(_, lines)| {
            let id = value(&lines, "id")
                .map_or("(no id)", |id| quoted(id).first().copied().unwrap_or(id));
            let review_by = value(&lines, "review-by").and_then(days_since_epoch);
            let problem = if ["id", "crate", "reason", "owner", "review-by"]
                .iter()
                .any(|key| value(&lines, key).is_none())
            {
                "a field is missing"
            } else if !is_advisory_id(id) {
                "the ID is not RUSTSEC-YYYY-NNNN"
            } else if review_by.is_none() {
                "review-by is not a date"
            } else if review_by < Some(today) {
                "review-by has passed"
            } else if review_by > Some(today.saturating_add(90)) {
                "review-by is more than 90 days ahead"
            } else {
                return None;
            };
            Some((id, problem))
        })
        .collect()
}

/// The byte offset of the first `wanted` in `text` that is outside a quoted
/// string.
fn unquoted_offset(text: &str, wanted: char) -> Option<usize> {
    let mut quote = None;
    let mut escaped = false;
    for (at, found) in text.char_indices() {
        match quote {
            None if found == wanted => return Some(at),
            None if found == '"' || found == '\'' => quote = Some(found),
            Some('"') if escaped => escaped = false,
            Some('"') if found == '\\' => escaped = true,
            Some(open) if found == open => quote = None,
            _ => {}
        }
    }
    None
}

/// `text` before and after its first unquoted `separator`.
fn split_unquoted(text: &str, separator: char) -> Option<(&str, &str)> {
    let (before, after) = text.split_at_checked(unquoted_offset(text, separator)?)?;
    Some((before, after.strip_prefix(separator)?))
}

/// The parts of a dotted TOML key, trimmed and unquoted:
/// `target.'cfg(unix)'.dependencies` is `target`, `cfg(unix)` and
/// `dependencies`.
fn key_path(key: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut rest = key;
    while let Some((part, after)) = split_unquoted(rest, '.') {
        parts.push(part);
        rest = after;
    }
    parts.push(rest);
    parts
        .into_iter()
        .map(|part| {
            let part = part.trim();
            ['"', '\'']
                .into_iter()
                .find_map(|quote| part.strip_prefix(quote)?.strip_suffix(quote))
                .unwrap_or(part)
        })
        .collect()
}

/// Every `key = value` line of a TOML file, with the key's full path from
/// the root: the path of the table it is in, then its own dotted key. A line
/// inside a multi-line string or array is read as if it stood alone, which
/// is enough for a Cargo manifest.
fn assignments(toml: &str) -> Vec<(Vec<&str>, &str)> {
    let mut table = Vec::new();
    let mut found = Vec::new();
    for line in rules(&toml.lines().collect::<Vec<_>>()) {
        if line.starts_with('[') {
            let header = line.trim_start_matches('[');
            table = split_unquoted(header, ']').map_or_else(Vec::new, |(path, _)| key_path(path));
        } else if let Some((key, value)) = split_unquoted(line, '=') {
            let mut path = table.clone();
            path.extend(key_path(key));
            found.push((path, value.trim()));
        }
    }
    found
}

/// The value of `key` in an inline table such as `{ version = "1", package
/// = "x" }`, as written.
fn inline_value<'a>(table: &'a str, key: &str) -> Option<&'a str> {
    let mut rest = table.trim().strip_prefix('{')?;
    loop {
        let (entry, after) = split_unquoted(rest, ',').unwrap_or((rest, ""));
        let found = split_unquoted(entry, '=').filter(|(name, _)| name.trim() == key);
        if let Some((_, value)) = found {
            return Some(value.trim().trim_end_matches('}').trim_end());
        }
        if after.is_empty() {
            return None;
        }
        rest = after;
    }
}

/// One dependency a manifest declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Declared<'a> {
    /// The name it is declared under.
    key: &'a str,
    /// The package it names, when `package = "..."` renames it.
    package: Option<&'a str>,
    /// Whether it takes its entry from `[workspace.dependencies]`.
    inherits: bool,
}

/// The dependencies declared in the tables of `toml` whose paths match one
/// of `tables`, in order, where `*` matches any one part of a path. Each is
/// found however it is written: a line in the table, a dotted key, or a
/// subtable of its own.
fn declared<'a>(toml: &'a str, tables: &[&[&str]]) -> Vec<Declared<'a>> {
    let mut found: Vec<Declared<'a>> = Vec::new();
    for (path, value) in assignments(toml) {
        let within = tables.iter().find_map(|table| {
            let (head, rest) = path.split_at_checked(table.len())?;
            let matches = head
                .iter()
                .zip(table.iter())
                .all(|(part, wanted)| *wanted == "*" || part == wanted);
            matches.then_some(rest)?.split_first()
        });
        let Some((&key, field)) = within else {
            continue;
        };
        let (package, inherits) = match field {
            [] => (
                inline_value(value, "package"),
                inline_value(value, "workspace") == Some("true"),
            ),
            ["package"] => (Some(value), false),
            ["workspace"] => (None, value == "true"),
            _ => (None, false),
        };
        let package = package.and_then(|name| quoted(name).first().copied());
        if let Some(known) = found.iter_mut().find(|known| known.key == key) {
            known.package = known.package.or(package);
            known.inherits |= inherits;
        } else {
            found.push(Declared {
                key,
                package,
                inherits,
            });
        }
    }
    found
}

/// The packages a manifest uses at run time, in every `[dependencies]` and
/// `[target.<cfg>.dependencies]` table, under the names cargo-deny and the
/// allowlist use: a renamed dependency by the package it names, and one
/// inherited from `workspace` by the package the workspace entry names.
fn normal_dependencies<'a>(manifest: &'a str, workspace: &'a str) -> Vec<&'a str> {
    let inherited = declared(workspace, &[&["workspace", "dependencies"]]);
    declared(
        manifest,
        &[&["dependencies"], &["target", "*", "dependencies"]],
    )
    .into_iter()
    .map(|dependency| {
        let from_workspace = || {
            inherited
                .iter()
                .find(|entry| entry.key == dependency.key)
                .and_then(|entry| entry.package)
        };
        dependency
            .package
            .or_else(|| dependency.inherits.then(from_workspace).flatten())
            .unwrap_or(dependency.key)
    })
    .collect()
}

/// The crates the core allowlist names, one `name = "..."` line each.
fn allowlisted(allowlist: &str) -> Vec<&str> {
    allowlist
        .lines()
        .filter_map(|line| line.strip_prefix("name = "))
        .flat_map(quoted)
        .collect()
}

/// The core's direct normal dependencies that are not on the allowlist.
fn unreviewed_dependencies<'a>(
    manifest: &'a str,
    workspace: &'a str,
    allowlist: &'a str,
) -> Vec<&'a str> {
    let allowed = allowlisted(allowlist);
    normal_dependencies(manifest, workspace)
        .into_iter()
        .filter(|name| !allowed.contains(name))
        .collect()
}

/// The cargo-vet exemptions that do not say when they must be reviewed, by
/// crate name.
fn undated_exemptions(config: &str) -> Vec<&str> {
    blocks(config, "[[exemptions.")
        .into_iter()
        .filter(|(_, lines)| {
            let criteria = value(lines, "criteria").unwrap_or_default();
            let notes = value(lines, "notes").unwrap_or_default();
            !(["\"safe-to-run\"", "\"safe-to-deploy\""].contains(&criteria)
                && notes.contains(", by 20")
                && notes.contains("(SEC-SUP-024)"))
        })
        .map(|(header, _)| {
            header
                .trim_start_matches("[[exemptions.")
                .trim_end_matches("]]")
        })
        .collect()
}

/// The values of every `toolchain:` line in a workflow.
fn workflow_toolchains(workflow: &str) -> Vec<&str> {
    workflow
        .lines()
        .filter_map(|line| line.trim().strip_prefix("toolchain: "))
        .collect()
}

/// The names of a workflow's jobs, in file order: the keys indented two
/// spaces under `jobs:`.
fn workflow_jobs(workflow: &str) -> Vec<&str> {
    workflow
        .lines()
        .skip_while(|line| *line != "jobs:")
        .filter_map(|line| line.strip_prefix("  ")?.strip_suffix(':'))
        .filter(|name| !name.starts_with([' ', '#']))
        .collect()
}

/// The trimmed lines of one job of a workflow, up to the next job.
fn workflow_job<'a>(workflow: &'a str, job: &str) -> Vec<&'a str> {
    let opening = format!("  {job}:");
    workflow
        .lines()
        .skip_while(|line| *line != opening)
        .skip(1)
        .take_while(|line| !is_job_opening(line))
        .map(str::trim)
        .collect()
}

/// Whether `line` opens a job: a key indented exactly two spaces.
fn is_job_opening(line: &str) -> bool {
    line.strip_prefix("  ")
        .is_some_and(|rest| !rest.starts_with([' ', '#']) && rest.ends_with(':'))
}

/// The commands in a shell script that run cargo, one per line.
fn cargo_commands(script: &str) -> Vec<&str> {
    script
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("cargo "))
        .collect()
}

// ---------------------------------------------------------------------------
// The reader itself, on literal samples.
// ---------------------------------------------------------------------------

#[test]
fn the_reader_finds_every_entry_and_the_requirements_each_cites() {
    let sample = "top = 1\n\
        list = [\n    \
            # a comment { path = \"not::an::entry\" }\n    \
            { path = \"a::b\", reason = \"one (SEC-AB-001)\" },\n    \
            { crate = \"c\", wrappers = [\"w\"], reason = \"two (SEC-AB-001, SEC-AB-0010)\" },\n    \
            { path = \"d::e\", reason = \"three (SEC-AB-002)\", allow-invalid = true },\n\
        ]\n\
        [bans]\n\
        x = \"y\"\n";
    let found = entries(&array(&table(sample, ""), "list"));
    assert_eq!(
        found,
        [
            Entry {
                name: "a::b",
                reason: "one (SEC-AB-001)"
            },
            Entry {
                name: "c",
                reason: "two (SEC-AB-001, SEC-AB-0010)"
            },
            Entry {
                name: "d::e",
                reason: "three (SEC-AB-002)"
            },
        ]
    );
    assert_eq!(citing(&found, "SEC-AB-001"), ["a::b", "c"]);
    assert_eq!(citing(&found, "SEC-AB-002"), ["d::e"]);
    assert_eq!(citing(&found, "SEC-AB-00"), Vec::<&str>::new());
    assert_eq!(table(sample, "bans"), ["x = \"y\""]);
    assert_eq!(value(&table(sample, ""), "top"), Some("1"));
    assert_eq!(value(&table(sample, ""), "missing"), None);
    assert_eq!(entry("{ path = \"no reason\" }"), None);
}

#[test]
fn every_rule_line_in_the_policy_files_parses_as_one_entry() {
    let arrays = [
        array(&table(WORKSPACE_CLIPPY, ""), "disallowed-methods"),
        array(&table(WORKSPACE_CLIPPY, ""), "disallowed-types"),
        array(&table(CORE_CLIPPY, ""), "disallowed-methods"),
        array(&table(CORE_CLIPPY, ""), "disallowed-types"),
        array(&table(DENY, "bans"), "deny"),
    ];
    for lines in arrays {
        let unparsed: Vec<&str> = entry_lines(&lines)
            .into_iter()
            .filter(|line| entry(line).is_none())
            .collect();
        assert_eq!(unparsed, Vec::<&str>::new());
    }
}

#[test]
fn the_advisory_check_reports_each_side_of_a_mismatch() {
    let deny = "[advisories]\nignore = [\n    { id = \"RUSTSEC-2026-0001\", reason = \"r\" },\n    \"RUSTSEC-2026-0002\",\n]\n";
    let exceptions = "# [[advisory]]\n# id = \"RUSTSEC-0000-0000\"\n\n[[advisory]]\nid = \"RUSTSEC-2026-0002\"\ncrate = \"c\"\nreason = \"r\"\nowner = \"@o\"\nreview-by = 2026-12-31\n\n[[advisory]]\nid = \"RUSTSEC-2026-0003\"\ncrate = \"c\"\nreason = \"r\"\n";
    assert_eq!(
        unmatched_advisories(deny, exceptions),
        (vec!["RUSTSEC-2026-0001"], vec!["RUSTSEC-2026-0003"])
    );
    // 2026-10-03 is day 20729.
    assert_eq!(
        refused_exceptions(exceptions, 20_729),
        [("RUSTSEC-2026-0003", "a field is missing")]
    );
    assert_eq!(
        ignored_advisories("[advisories]\nignore = [\"RUSTSEC-2026-0004\"]\n"),
        ["RUSTSEC-2026-0004"]
    );
}

/// One `[[advisory]]` entry with every field, for the samples below.
fn advisory(id: &str, review_by: &str) -> String {
    format!(
        "[[advisory]]\nid = \"{id}\"\ncrate = \"c\"\nreason = \"r\"\nowner = \"@o\"\nreview-by = {review_by}\n\n"
    )
}

#[test]
fn the_exception_check_refuses_a_lapsed_date_a_far_date_and_a_malformed_id() {
    let exceptions = [
        // Due today, and due in exactly 90 days: both still good.
        advisory("RUSTSEC-2026-0001", "2026-10-03"),
        advisory("RUSTSEC-2026-0002", "2027-01-01"),
        advisory("RUSTSEC-2026-0003", "2026-10-02"),
        advisory("RUSTSEC-2026-0004", "2027-01-02"),
        advisory("RUSTSEC-2026-0005", "2026-02-30"),
        advisory("RUSTSEC-2026-0006", "\"2026-12-31\""),
        advisory("RUSTSEC-26-0007", "2026-12-31"),
        advisory("RUSTSEC-2026-00080", "2026-12-31"),
        advisory("RUSTSEC-2026-0009-1", "2026-12-31"),
        advisory("GHSA-2026-0010", "2026-12-31"),
        advisory("RUSTSEC-2026-001x", "2026-12-31"),
        "[[advisory]]\nid = \"RUSTSEC-2026-0012\"\ncrate = \"c\"\nreason = \"r\"\nreview-by = 2026-12-31\n\n"
            .to_owned(),
        "[[advisory]]\ncrate = \"c\"\n\n".to_owned(),
    ]
    .concat();
    // 2026-10-03 is day 20729.
    assert_eq!(
        refused_exceptions(&exceptions, 20_729),
        [
            ("RUSTSEC-2026-0003", "review-by has passed"),
            ("RUSTSEC-2026-0004", "review-by is more than 90 days ahead"),
            ("RUSTSEC-2026-0005", "review-by is not a date"),
            ("RUSTSEC-2026-0006", "review-by is not a date"),
            ("RUSTSEC-26-0007", "the ID is not RUSTSEC-YYYY-NNNN"),
            ("RUSTSEC-2026-00080", "the ID is not RUSTSEC-YYYY-NNNN"),
            ("RUSTSEC-2026-0009-1", "the ID is not RUSTSEC-YYYY-NNNN"),
            ("GHSA-2026-0010", "the ID is not RUSTSEC-YYYY-NNNN"),
            ("RUSTSEC-2026-001x", "the ID is not RUSTSEC-YYYY-NNNN"),
            ("RUSTSEC-2026-0012", "a field is missing"),
            ("(no id)", "a field is missing"),
        ]
    );
}

#[test]
fn the_date_reader_counts_days_from_the_epoch() {
    // Reference values from Python's datetime.date arithmetic.
    for (date, days) in [
        ("1970-01-01", 0),
        ("1969-12-31", -1),
        ("2000-02-29", 11_016),
        ("2000-03-01", 11_017),
        ("2024-02-29", 19_782),
        ("2026-10-03", 20_729),
        ("2027-01-01", 20_819),
        ("2100-03-01", 47_541),
        ("1600-03-01", -135_080),
        ("9999-12-31", 2_932_896),
    ] {
        assert_eq!(days_since_epoch(date), Some(days), "{date}");
    }
    for not_a_date in [
        "2026-02-29",
        "2100-02-29",
        "2026-04-31",
        "2026-01-32",
        "2026-13-01",
        "2026-00-10",
        "2026-01-00",
        "2026-1-01",
        "26-01-01",
        "+026-01-01",
        "2026-01-0x",
        "2026-01-01-01",
        "2026-01",
        "\"2026-01-01\"",
        "",
    ] {
        assert_eq!(days_since_epoch(not_a_date), None, "{not_a_date}");
    }
}

#[test]
fn the_allowlist_check_reports_a_dependency_nobody_reviewed() {
    let manifest = "[package]\nname = \"x\"\n\n[dependencies]\nserde.workspace = true\npostcard = { workspace = true }\n\n[dev-dependencies]\nproptest.workspace = true\n";
    let allowlist = "[[crate]]\nname = \"serde\"\nreason = \"r\"\n";
    assert_eq!(normal_dependencies(manifest, ""), ["serde", "postcard"]);
    assert_eq!(
        unreviewed_dependencies(manifest, "", allowlist),
        ["postcard"]
    );
}

#[test]
fn the_allowlist_check_reads_every_way_to_declare_a_normal_dependency() {
    let manifest = r#"dependencies.at-the-root = "1"

[package]
name = "x"

[dependencies.subtable]
version = "1"
features = [
    "a=b",
]

[dependencies.subtable-renamed]
package = "subtable-real"
version = "1"

[dependencies]
plain = "1" # a comment
inline-renamed = { version = "1", package = "inline-real" }
dotted-renamed.package = "dotted-real"
dotted-renamed.version = "1"
inherited = { workspace = true }
inherited-dotted.workspace = true
not-inherited = "1"
"quoted" = "1"

[target.'cfg(unix)'.dependencies]
single-quoted-target = "1"

[target."cfg(target_feature = \"sse4.1\")".dependencies.escaped-target]
version = "1"

[target.x86_64-unknown-linux-gnu.dependencies]
bare-target = "1"

[ dependencies . spaced ]
version = "1"

[[bin]]
name = "tool"
dependencies.not-a-dependency = "1"

[dev-dependencies]
dev-only = "1"

[build-dependencies]
build-only = "1"

[target.'cfg(unix)'.dev-dependencies]
target-dev-only = "1"
"#;
    let workspace = "[workspace.dependencies]\n\
        inherited = { version = \"1\", package = \"inherited-real\" }\n\
        inherited-dotted = { version = \"1\", package = \"inherited-dotted-real\" }\n\
        not-inherited = { version = \"1\", package = \"not-this\" }\n";
    let found = [
        "at-the-root",
        "subtable",
        "subtable-real",
        "plain",
        "inline-real",
        "dotted-real",
        "inherited-real",
        "inherited-dotted-real",
        "not-inherited",
        "quoted",
        "single-quoted-target",
        "escaped-target",
        "bare-target",
        "spaced",
    ];
    assert_eq!(normal_dependencies(manifest, workspace), found);
    assert_eq!(
        unreviewed_dependencies(manifest, workspace, "name = \"plain\"\n"),
        found
            .iter()
            .copied()
            .filter(|name| *name != "plain")
            .collect::<Vec<_>>()
    );
}

#[test]
fn the_exemption_check_reports_an_exemption_with_no_review_date() {
    let config = "[[exemptions.a]]\nversion = \"1\"\ncriteria = \"safe-to-run\"\nnotes = \"Dev-only, by 2026-12-31 (SEC-SUP-024).\"\n\n[[exemptions.b]]\nversion = \"1\"\ncriteria = \"safe-to-deploy\"\nnotes = \"No date.\"\n\n[[exemptions.c]]\nversion = \"1\"\ncriteria = \"anything\"\nnotes = \"by 2026-12-31 (SEC-SUP-024)\"\n";
    assert_eq!(undated_exemptions(config), ["b", "c"]);
}

// ---------------------------------------------------------------------------
// The core's own rules.
// ---------------------------------------------------------------------------

/// Verifies: SEC-MED-002
#[test]
fn the_core_refuses_every_construct_that_can_panic_or_overflow() {
    assert_eq!(
        rules(&table(CORE_MANIFEST, "lints.clippy")),
        [
            "all = { level = \"warn\", priority = -1 }",
            "pedantic = { level = \"warn\", priority = -1 }",
            "await_holding_lock = \"deny\"",
            // Spelled in two halves so that `xtask lint-exceptions`, which
            // fails on any source line naming a ban lint, does not match.
            concat!("disallowed", "_methods = \"deny\""),
            concat!("disallowed", "_types = \"deny\""),
            "unwrap_used = \"deny\"",
            "expect_used = \"deny\"",
            "panic = \"deny\"",
            "unreachable = \"deny\"",
            "todo = \"deny\"",
            "unimplemented = \"deny\"",
            "indexing_slicing = \"deny\"",
            "arithmetic_side_effects = \"deny\"",
            "large_stack_arrays = \"deny\"",
        ]
    );
    // Warnings fail the gate, so every lint above is an error there.
    assert!(has_line(
        GATE,
        "cargo clippy --locked --workspace --all-targets -- -D warnings"
    ));
}

#[test]
fn the_core_keeps_every_workspace_lint() {
    let core_rust = rules(&table(CORE_MANIFEST, "lints.rust"));
    let core_clippy = rules(&table(CORE_MANIFEST, "lints.clippy"));
    let dropped: Vec<&str> = rules(&table(WORKSPACE_MANIFEST, "workspace.lints.rust"))
        .into_iter()
        .filter(|lint| !core_rust.contains(lint))
        .chain(
            rules(&table(WORKSPACE_MANIFEST, "workspace.lints.clippy"))
                .into_iter()
                .filter(|lint| !core_clippy.contains(lint)),
        )
        .collect();
    assert_eq!(dropped, Vec::<&str>::new());
    assert_eq!(
        rules(&table(CORE_MANIFEST, "lints.rust")),
        ["unsafe_code = \"forbid\"", "missing_docs = \"warn\""]
    );
}

/// Verifies: SEC-MED-003
#[test]
fn the_core_cannot_size_an_allocation_from_a_declared_length() {
    let core_rules = rules(&table(CORE_CLIPPY, ""));
    let workspace_rules = rules(&table(WORKSPACE_CLIPPY, ""));
    // The core's file is the workspace's, plus these lines.
    let core_only: Vec<&str> = core_rules
        .iter()
        .copied()
        .filter(|line| !workspace_rules.contains(line))
        .collect();
    let presizing = "never size an allocation from a declared length; use the bounded capacity helper (SEC-MED-003)";
    let expected: Vec<String> = [
        "std::vec::Vec::with_capacity",
        "std::vec::Vec::reserve",
        "std::vec::Vec::reserve_exact",
        "std::vec::Vec::try_reserve",
        "std::vec::Vec::try_reserve_exact",
        "std::vec::Vec::resize",
        "std::vec::Vec::resize_with",
        "alloc::vec::from_elem",
        "std::string::String::with_capacity",
        "std::string::String::reserve",
        "std::string::String::reserve_exact",
        "std::string::String::try_reserve",
        "std::string::String::try_reserve_exact",
        "std::collections::VecDeque::with_capacity",
        "std::collections::VecDeque::reserve",
        "std::collections::VecDeque::reserve_exact",
        "std::collections::VecDeque::try_reserve",
        "std::collections::VecDeque::try_reserve_exact",
        "std::collections::VecDeque::resize",
        "std::collections::VecDeque::resize_with",
        "std::collections::HashMap::with_capacity",
        "std::collections::HashMap::with_capacity_and_hasher",
        "std::collections::HashMap::reserve",
        "std::collections::HashMap::try_reserve",
        "std::collections::HashSet::with_capacity",
        "std::collections::HashSet::with_capacity_and_hasher",
        "std::collections::HashSet::reserve",
        "std::collections::HashSet::try_reserve",
        "std::collections::BinaryHeap::with_capacity",
        "std::collections::BinaryHeap::reserve",
        "std::collections::BinaryHeap::reserve_exact",
        "std::collections::BinaryHeap::try_reserve",
        "std::collections::BinaryHeap::try_reserve_exact",
        "std::boxed::Box::new_uninit_slice",
        "std::boxed::Box::new_zeroed_slice",
        "std::rc::Rc::new_uninit_slice",
        "std::rc::Rc::new_zeroed_slice",
        "std::sync::Arc::new_uninit_slice",
        "std::sync::Arc::new_zeroed_slice",
        "slice::repeat",
        "str::repeat",
        "std::iter::repeat",
        "std::iter::repeat_n",
        "std::iter::repeat_with",
    ]
    .iter()
    .map(|path| format!("{{ path = \"{path}\", reason = \"{presizing}\" }},"))
    .chain(
        [
            "allow-unwrap-in-tests = true",
            "allow-expect-in-tests = true",
            "allow-panic-in-tests = true",
            "allow-indexing-slicing-in-tests = true",
        ]
        .map(String::from),
    )
    .collect();
    assert_eq!(core_only, expected);
}

#[test]
fn the_core_clippy_configuration_repeats_every_workspace_rule() {
    let core_rules = rules(&table(CORE_CLIPPY, ""));
    let missing: Vec<&str> = rules(&table(WORKSPACE_CLIPPY, ""))
        .into_iter()
        .filter(|line| !core_rules.contains(line))
        .collect();
    assert_eq!(missing, Vec::<&str>::new());
    // And only the two files exist: the gate refuses any other.
    assert!(has_line(
        GATE,
        "if [[ \"$clippy_configs\" != $'clippy.toml\\ncrates/gunmetal-core/clippy.toml' ]]; then"
    ));
}

/// Verifies: SEC-MED-004
#[test]
fn the_core_tests_also_run_where_usize_is_32_bits_wide() {
    assert!(has_line(CI, "targets: i686-unknown-linux-gnu"));
    assert!(has_line(
        CI,
        "- run: cargo test --locked -p gunmetal-core --target i686-unknown-linux-gnu"
    ));
    // Overflowing arithmetic and narrowing casts are refused in the core:
    // arithmetic_side_effects above, and Clippy's pedantic cast lints.
    let core_clippy = rules(&table(CORE_MANIFEST, "lints.clippy"));
    assert!(core_clippy.contains(&"arithmetic_side_effects = \"deny\""));
    assert!(core_clippy.contains(&"pedantic = { level = \"warn\", priority = -1 }"));
}

/// Verifies: SEC-SUP-025
#[test]
fn the_core_depends_only_on_reviewed_crates() {
    assert_eq!(
        unreviewed_dependencies(CORE_MANIFEST, WORKSPACE_MANIFEST, CORE_ALLOWLIST),
        Vec::<&str>::new()
    );
    // The owner accepted the core's first crates (D-02). The first one used,
    // sha2 for src/crypto.rs (WP-122), is listed with every crate it brings
    // in on any target; the others are listed when their first user needs
    // them.
    assert_eq!(
        allowlisted(CORE_ALLOWLIST),
        [
            "adler2",
            "block-buffer",
            "cfg-if",
            "cobs",
            "cpufeatures",
            "crypto-common",
            "digest",
            "hybrid-array",
            "libc",
            "miniz_oxide",
            "postcard",
            "proc-macro2",
            "quote",
            "serde",
            "serde_core",
            "serde_derive",
            "sha2",
            "syn",
            "thiserror",
            "thiserror-impl",
            "tinyvec",
            "typenum",
            "unicode-ident",
            "unicode-normalization",
        ]
    );
    // The manifest names only direct dependencies, so the gate also compares
    // every crate cargo resolves for the core, on every target and with every
    // feature on, with the allowlist, through WP-008's xtask core-deps.
    for line in [
        r#"cargo tree "$locked" -p gunmetal-core -e normal --target all --all-features --prefix none --format '{p}' >target/core-deps.txt"#,
        r#"cargo run "$locked" -q -p xtask -- core-deps target/core-deps.txt"#,
    ] {
        assert!(has_line(GATE, line), "{line}");
    }
}

// ---------------------------------------------------------------------------
// One door per risk: what only one module may do.
// ---------------------------------------------------------------------------

/// Verifies: SEC-MED-063
#[test]
fn only_the_sandbox_launcher_starts_programs() {
    assert_eq!(
        citing(&workspace_methods(), "SEC-MED-063"),
        ["std::process::Command::new", "tokio::process::Command::new"]
    );
}

/// Verifies: SEC-EXT-001, SEC-API-076
#[test]
fn only_the_egress_client_reaches_out() {
    let methods = sorted(&[
        "std::net::TcpStream::connect",
        "std::net::TcpStream::connect_timeout",
        "std::net::UdpSocket::connect",
        "std::net::UdpSocket::send_to",
        "std::net::ToSocketAddrs::to_socket_addrs",
        "tokio::net::TcpStream::connect",
        "tokio::net::TcpSocket::connect",
        "tokio::net::UdpSocket::connect",
        "tokio::net::UdpSocket::send_to",
        "tokio::net::UdpSocket::try_send_to",
        "tokio::net::UdpSocket::poll_send_to",
        "tokio::net::lookup_host",
        "socket2::Socket::connect",
        "socket2::Socket::connect_timeout",
        "socket2::Socket::send_to",
        "socket2::Socket::send_to_with_flags",
        "socket2::Socket::send_to_vectored",
        "socket2::Socket::send_to_vectored_with_flags",
        "socket2::Socket::sendmsg",
        "rustix::net::connect",
        "rustix::net::sendto",
        "rustix::net::sendmsg_addr",
        "reqwest::Client::new",
        "reqwest::Client::builder",
        "reqwest::ClientBuilder::new",
        "reqwest::get",
        "reqwest::blocking::Client::new",
        "reqwest::blocking::Client::builder",
        "reqwest::blocking::ClientBuilder::new",
        "reqwest::blocking::get",
        "ureq::Agent::new_with_defaults",
        "ureq::Agent::new_with_config",
        "ureq::Agent::config_builder",
        "ureq::get",
        "ureq::post",
        "ureq::put",
        "ureq::delete",
        "ureq::head",
        "ureq::patch",
        "hyper::client::conn::http1::handshake",
        "hyper::client::conn::http2::handshake",
        "hyper_util::client::legacy::Client::builder",
        "hyper_util::client::legacy::Builder::new",
    ]);
    let types = sorted(&[
        "reqwest::Client",
        "reqwest::blocking::Client",
        "ureq::Agent",
        "hyper_util::client::legacy::Client",
    ]);
    let crates = sorted(&[
        "reqwest",
        "ureq",
        "isahc",
        "attohttpc",
        "surf",
        "minreq",
        "ehttp",
        "curl",
    ]);
    for id in ["SEC-EXT-001", "SEC-API-076"] {
        assert_eq!(citing(&workspace_methods(), id), methods, "{id}");
        assert_eq!(citing(&workspace_types(), id), types, "{id}");
        assert_eq!(citing(&denied_crates(), id), crates, "{id}");
    }
}

/// Verifies: SEC-OPS-037
#[test]
fn only_the_listener_sees_where_a_request_came_from() {
    assert_eq!(
        citing(&workspace_methods(), "SEC-OPS-037"),
        sorted(&[
            "std::net::TcpListener::accept",
            "std::net::TcpStream::peer_addr",
            "std::net::UdpSocket::peer_addr",
            "std::net::UdpSocket::recv_from",
            "std::net::UdpSocket::peek_from",
            "std::os::unix::net::UnixListener::accept",
            "std::os::unix::net::UnixStream::peer_addr",
            "std::os::unix::net::UnixDatagram::peer_addr",
            "std::os::unix::net::UnixDatagram::recv_from",
            "tokio::net::TcpListener::accept",
            "tokio::net::TcpListener::poll_accept",
            "tokio::net::TcpStream::peer_addr",
            "tokio::net::tcp::OwnedReadHalf::peer_addr",
            "tokio::net::tcp::OwnedWriteHalf::peer_addr",
            "tokio::net::tcp::ReadHalf::peer_addr",
            "tokio::net::tcp::WriteHalf::peer_addr",
            "tokio::net::UdpSocket::peer_addr",
            "tokio::net::UdpSocket::recv_from",
            "tokio::net::UdpSocket::poll_recv_from",
            "tokio::net::UdpSocket::try_recv_from",
            "tokio::net::UdpSocket::recv_buf_from",
            "tokio::net::UdpSocket::try_recv_buf_from",
            "tokio::net::UdpSocket::peek_from",
            "tokio::net::UdpSocket::poll_peek_from",
            "tokio::net::UdpSocket::try_peek_from",
            "tokio::net::UdpSocket::peek_sender",
            "tokio::net::UdpSocket::poll_peek_sender",
            "tokio::net::UdpSocket::try_peek_sender",
            "tokio::net::UnixListener::accept",
            "tokio::net::UnixListener::poll_accept",
            "tokio::net::UnixStream::peer_addr",
            "tokio::net::unix::OwnedReadHalf::peer_addr",
            "tokio::net::unix::OwnedWriteHalf::peer_addr",
            "tokio::net::unix::ReadHalf::peer_addr",
            "tokio::net::unix::WriteHalf::peer_addr",
            "tokio::net::UnixDatagram::peer_addr",
            "tokio::net::UnixDatagram::recv_from",
            "tokio::net::UnixDatagram::poll_recv_from",
            "tokio::net::UnixDatagram::try_recv_from",
            "tokio::net::UnixDatagram::recv_buf_from",
            "tokio::net::UnixDatagram::try_recv_buf_from",
            "socket2::Socket::accept",
            "socket2::Socket::accept_raw",
            "socket2::Socket::peer_addr",
            "socket2::Socket::recv_from",
            "socket2::Socket::recv_from_with_flags",
            "socket2::Socket::recv_from_vectored",
            "socket2::Socket::recv_from_vectored_with_flags",
            "socket2::Socket::peek_from",
            "socket2::Socket::peek_sender",
            "socket2::Socket::recvmsg",
            "rustix::net::acceptfrom",
            "rustix::net::acceptfrom_with",
            "rustix::net::getpeername",
            "rustix::net::recvfrom",
            "rustix::net::recvmsg",
            "axum::serve::IncomingStream::remote_addr",
            "axum::serve::Listener::accept",
        ])
    );
    assert_eq!(
        citing(&workspace_types(), "SEC-OPS-037"),
        ["axum::extract::ConnectInfo"]
    );
    // Clippy cannot see a header name, so the gate searches for one.
    assert!(has_line(
        GATE,
        "git grep --untracked -n -i -E '\"(forwarded|x-forwarded-[a-z-]+|x-real-ip|x-client-ip|true-client-ip|cf-connecting-ip)\"|header::FORWARDED' -- 'crates/*.rs' ':!crates/*/tests/*' ':!crates/gunmetal-server/src/listener.rs' ':!crates/gunmetal-core/src/http/forwarded.rs' || status=$?"
    ));
}

/// Every function in `rustix::fs` that takes a path. With `CWD` as the
/// directory, the `*at` forms take one too.
const RUSTIX_PATH_FUNCTIONS: [&str; 41] = [
    "access",
    "accessat",
    "chmod",
    "chmodat",
    "chown",
    "chownat",
    "getxattr",
    "lgetxattr",
    "link",
    "linkat",
    "listxattr",
    "llistxattr",
    "lremovexattr",
    "lsetxattr",
    "lstat",
    "mkdir",
    "mkdirat",
    "mkfifoat",
    "mknodat",
    "open",
    "openat",
    "openat2",
    "readlink",
    "readlinkat",
    "readlinkat_raw",
    "removexattr",
    "rename",
    "renameat",
    "renameat_with",
    "rmdir",
    "setxattr",
    "stat",
    "statat",
    "statfs",
    "statvfs",
    "statx",
    "symlink",
    "symlinkat",
    "unlink",
    "unlinkat",
    "utimensat",
];

/// Verifies: SEC-MED-033
#[test]
fn only_the_filesystem_crate_opens_files() {
    let std_fs = [
        "canonicalize",
        "copy",
        "create_dir",
        "create_dir_all",
        "exists",
        "hard_link",
        "metadata",
        "read",
        "read_dir",
        "read_link",
        "read_to_string",
        "remove_dir",
        "remove_dir_all",
        "remove_file",
        "rename",
        "set_permissions",
        "soft_link",
        "symlink_metadata",
        "write",
        "File::create",
        "File::create_new",
        "File::open",
        "OpenOptions::open",
        "DirBuilder::create",
    ]
    .map(|name| format!("std::fs::{name}"));
    let tokio_fs = [
        "canonicalize",
        "copy",
        "create_dir",
        "create_dir_all",
        "hard_link",
        "metadata",
        "read",
        "read_dir",
        "read_link",
        "read_to_string",
        "remove_dir",
        "remove_dir_all",
        "remove_file",
        "rename",
        "set_permissions",
        "symlink",
        "symlink_metadata",
        "try_exists",
        "write",
        "File::create",
        "File::create_new",
        "File::open",
        "OpenOptions::open",
        "DirBuilder::create",
    ]
    .map(|name| format!("tokio::fs::{name}"));
    let path = [
        "canonicalize",
        "exists",
        "is_dir",
        "is_file",
        "is_symlink",
        "metadata",
        "read_dir",
        "read_link",
        "symlink_metadata",
        "try_exists",
    ]
    .map(|name| format!("std::path::Path::{name}"));
    let unix = ["chown", "lchown", "symlink"].map(|name| format!("std::os::unix::fs::{name}"));
    let rustix_fs = RUSTIX_PATH_FUNCTIONS.map(|name| format!("rustix::fs::{name}"));
    let expected: Vec<String> = std_fs
        .into_iter()
        .chain(tokio_fs)
        .chain(path)
        .chain(unix)
        .chain(rustix_fs)
        .chain(["cap_std::ambient_authority".to_owned()])
        .collect();
    let expected: Vec<&str> = expected.iter().map(String::as_str).collect();
    assert_eq!(
        citing(&workspace_methods(), "SEC-MED-033"),
        sorted(&expected)
    );
}

/// Verifies: SEC-HIS-015
#[test]
fn no_path_is_built_outside_the_filesystem_crate() {
    assert_eq!(
        citing(&workspace_methods(), "SEC-HIS-015"),
        sorted(&[
            "std::path::Path::new",
            "std::path::Path::join",
            "std::path::Path::with_extension",
            "std::path::Path::with_file_name",
            "std::path::PathBuf::push",
            "std::path::PathBuf::set_extension",
            "std::path::PathBuf::set_file_name",
        ])
    );
}

/// Verifies: SEC-TM-039
#[test]
fn sql_runs_only_through_the_one_connection_opener() {
    let expected = [
        "open",
        "open_in_memory",
        "open_in_memory_with_flags",
        "open_in_memory_with_flags_and_vfs",
        "open_with_flags",
        "open_with_flags_and_vfs",
        "execute",
        "execute_batch",
        "prepare",
        "prepare_cached",
        "prepare_with_flags",
        "query_one",
        "query_row",
        "query_row_and_then",
        "pragma",
        "pragma_query",
        "pragma_query_value",
        "pragma_update",
        "pragma_update_and_check",
    ]
    .map(|method| format!("rusqlite::Connection::{method}"));
    let expected: Vec<&str> = expected.iter().map(String::as_str).collect();
    assert_eq!(
        citing(&workspace_methods(), "SEC-TM-039"),
        sorted(&expected)
    );
}

/// Verifies: SEC-STD-022
#[test]
fn randomness_comes_only_from_the_one_csprng_function() {
    assert_eq!(
        citing(&workspace_methods(), "SEC-STD-022"),
        sorted(&[
            "rand_core::SeedableRng::seed_from_u64",
            "rand_core::SeedableRng::from_seed",
            "rand_core::SeedableRng::from_rng",
            "rand_core::SeedableRng::try_from_rng",
            "rand_core::SeedableRng::from_os_rng",
            "rand_core::SeedableRng::try_from_os_rng",
            "rand::rng",
            "rand::random",
            "rand::random_bool",
            "rand::random_iter",
            "rand::random_range",
            "rand::random_ratio",
            "rand::fill",
            "rand::thread_rng",
            "getrandom::fill",
            "getrandom::fill_uninit",
            "getrandom::u32",
            "getrandom::u64",
            "fastrand::seed",
        ])
    );
    assert_eq!(
        citing(&workspace_types(), "SEC-STD-022"),
        sorted(&[
            "fastrand::Rng",
            "rand_xorshift::XorShiftRng",
            "rand::rngs::OsRng",
            "rand::rngs::StdRng",
            "rand::rngs::ThreadRng",
        ])
    );
    assert_eq!(
        citing(&denied_crates(), "SEC-STD-022"),
        sorted(&[
            "fastrand",
            "rand_xorshift",
            "oorandom",
            "nanorand",
            "wyrand",
            "turborand",
            "tinyrand",
            "frand",
            "rand_pcg",
            "rand_xoshiro",
            "rand_sfc",
            "rand_isaac",
        ])
    );
    // The two that test tooling needs are allowed only under that tooling.
    assert!(has_line(
        DENY,
        "{ crate = \"fastrand\", wrappers = [\"tempfile\"], reason = \"non-cryptographic RNG (SEC-STD-022)\" },"
    ));
    assert!(has_line(
        DENY,
        "{ crate = \"rand_xorshift\", wrappers = [\"proptest\"], reason = \"non-cryptographic RNG (SEC-STD-022)\" },"
    ));
    assert_eq!(
        rules(&table(DENY, "[bans.features]")),
        [
            "crate = \"rand\"",
            "deny = [\"small_rng\"]",
            "reason = \"SmallRng is a non-cryptographic generator (SEC-STD-022)\"",
        ]
    );
}

/// Verifies: SEC-STD-018
#[test]
fn cryptography_is_used_only_in_the_two_crypto_modules() {
    assert_eq!(
        citing(&workspace_methods(), "SEC-STD-018"),
        sorted(&[
            "rustls::ClientConfig::builder",
            "rustls::ClientConfig::builder_with_protocol_versions",
            "rustls::ClientConfig::builder_with_provider",
            "rustls::ServerConfig::builder",
            "rustls::ServerConfig::builder_with_protocol_versions",
            "rustls::ServerConfig::builder_with_provider",
            "rustls::crypto::CryptoProvider::install_default",
            "rustls::crypto::aws_lc_rs::default_provider",
            "rustls::crypto::ring::default_provider",
        ])
    );
    assert_eq!(
        citing(&workspace_types(), "SEC-STD-018"),
        sorted(&[
            "sha2::Sha224",
            "sha2::Sha256",
            "sha2::Sha384",
            "sha2::Sha512",
            "sha2::Sha512_224",
            "sha2::Sha512_256",
            "hmac::Hmac",
            "hmac::SimpleHmac",
            "hkdf::Hkdf",
            "hkdf::SimpleHkdf",
            "chacha20poly1305::ChaCha20Poly1305",
            "chacha20poly1305::XChaCha20Poly1305",
            "argon2::Argon2",
            "p256::PublicKey",
            "p256::SecretKey",
            "p256::ecdsa::Signature",
            "p256::ecdsa::SigningKey",
            "p256::ecdsa::VerifyingKey",
            "ed25519_dalek::Signature",
            "ed25519_dalek::SigningKey",
            "ed25519_dalek::VerifyingKey",
            "age::Decryptor",
            "age::Encryptor",
            "age::x25519::Identity",
            "age::x25519::Recipient",
        ])
    );
    // No crate of ours may depend on a TLS provider's primitives directly.
    assert_eq!(
        citing(&denied_crates(), "SEC-STD-018"),
        ["aws-lc-rs", "ring"]
    );
}

/// Verifies: SEC-STD-019
#[test]
fn broken_and_unauthenticated_cryptography_cannot_be_linked() {
    assert_eq!(
        citing(&denied_crates(), "SEC-STD-019"),
        sorted(&[
            "md2",
            "md4",
            "des",
            "rc2",
            "rc4",
            "blowfish",
            "cast5",
            "idea",
            "ecb",
            "cbc",
            "cfb-mode",
            "cfb8",
            "ofb",
            "block-modes",
            "rust-crypto",
            "ctr",
            "md-5",
            "md5",
            "sha1",
            "sha1_smol",
            "ring",
            "aws-lc-rs",
        ])
    );
}

/// Verifies: SEC-HIS-026
#[test]
fn tls_certificate_verification_cannot_be_switched_off() {
    assert_eq!(
        citing(&workspace_methods(), "SEC-HIS-026"),
        [
            "rustls::ClientConfig::dangerous",
            "rustls::ConfigBuilder::dangerous"
        ]
    );
    assert_eq!(
        citing(&workspace_types(), "SEC-HIS-026"),
        ["rustls::client::danger::DangerousClientConfig"]
    );
}

/// Verifies: SEC-MED-009
#[test]
fn only_the_streaming_helper_inflates() {
    assert_eq!(
        citing(&workspace_methods(), "SEC-MED-009"),
        sorted(&[
            "miniz_oxide::inflate::core::decompress",
            "miniz_oxide::inflate::core::decompress_with_limit",
            "miniz_oxide::inflate::decompress_slice_iter_to_slice",
            "miniz_oxide::inflate::decompress_to_vec",
            "miniz_oxide::inflate::decompress_to_vec_with_limit",
            "miniz_oxide::inflate::decompress_to_vec_zlib",
            "miniz_oxide::inflate::decompress_to_vec_zlib_with_limit",
            "miniz_oxide::inflate::stream::inflate",
        ])
    );
}

#[test]
fn public_ids_and_secret_values_each_have_one_door() {
    assert_eq!(
        citing(&workspace_methods(), "SEC-HIS-012"),
        ["gunmetal_core::id::Minted::from_os_random"]
    );
    assert_eq!(
        citing(&workspace_methods(), "SEC-OPS-013"),
        [
            "gunmetal_secrets::Secret::expose",
            "secrecy::ExposeSecret::expose_secret",
            "secrecy::ExposeSecretMut::expose_secret_mut",
        ]
    );
}

// ---------------------------------------------------------------------------
// Crates that must not be in the build at all.
// ---------------------------------------------------------------------------

/// Verifies: SEC-TM-006, SEC-HIS-052, SEC-NET-030, SEC-OPS-039
#[test]
fn no_crate_can_ask_a_router_to_open_ports() {
    let mapping = sorted(&[
        "igd",
        "igd-next",
        "easy-upnp",
        "natpmp",
        "crab_nat",
        "pcp-client",
        "portmapper",
        "rupnp",
        "upnp-client",
    ]);
    for id in ["SEC-HIS-052", "SEC-NET-030", "SEC-OPS-039"] {
        assert_eq!(citing(&denied_crates(), id), mapping, "{id}");
    }
    // SEC-TM-006 also covers revealing anything to discovery protocols.
    assert_eq!(
        citing(&denied_crates(), "SEC-TM-006"),
        sorted(&[
            "igd",
            "igd-next",
            "easy-upnp",
            "natpmp",
            "crab_nat",
            "pcp-client",
            "portmapper",
            "rupnp",
            "upnp-client",
            "ssdp",
            "ssdp-client",
        ])
    );
}

/// Verifies: SEC-NET-059
#[test]
fn no_ssdp_upnp_or_dlna_service_can_be_linked() {
    assert_eq!(
        citing(&denied_crates(), "SEC-NET-059"),
        sorted(&["rupnp", "upnp-client", "ssdp", "ssdp-client"])
    );
}

/// Verifies: SEC-NET-054, SEC-OPS-040
#[test]
fn no_iroh_endpoint_exists_in_r1() {
    assert_eq!(
        citing(&denied_crates(), "SEC-NET-054"),
        ["iroh", "iroh-net", "iroh-relay"]
    );
    assert_eq!(
        citing(&denied_crates(), "SEC-OPS-040"),
        ["iroh", "iroh-net", "iroh-relay", "portmapper"]
    );
}

/// Verifies: SEC-EXT-018, SEC-HIS-056
#[test]
fn no_plugin_runtime_or_dynamic_loader_can_be_linked() {
    let expected = sorted(&[
        "libloading",
        "dlopen",
        "dlopen2",
        "sharedlib",
        "abi_stable",
        "wasmtime",
        "wasmer",
        "wasmi",
        "wasm3",
        "extism",
        "mlua",
        "rlua",
        "pyo3",
        "deno_core",
        "deno_runtime",
        "v8",
        "rquickjs",
        "boa_engine",
        "rhai",
        "rune",
        "gluon",
    ]);
    for id in ["SEC-EXT-018", "SEC-HIS-056"] {
        assert_eq!(citing(&denied_crates(), id), expected, "{id}");
    }
}

/// Verifies: SEC-TM-038, SEC-HIS-034
#[test]
fn no_xml_parser_can_be_linked_in_r1() {
    let expected = sorted(&[
        "libxml",
        "minidom",
        "quick-xml",
        "roxmltree",
        "rxml",
        "serde-xml-rs",
        "sxd-document",
        "xml",
        "xml-rs",
        "xml5ever",
        "xmlparser",
        "xmltree",
        "xot",
        "yaserde",
    ]);
    for id in ["SEC-TM-038", "SEC-HIS-034"] {
        assert_eq!(citing(&denied_crates(), id), expected, "{id}");
    }
}

/// Verifies: SEC-HIS-019
#[test]
fn no_archive_extractor_can_be_linked() {
    assert_eq!(
        citing(&denied_crates(), "SEC-HIS-019"),
        sorted(&[
            "async-tar",
            "async_zip",
            "cab",
            "compress-tools",
            "rc-zip",
            "sevenz-rust",
            "sevenz-rust2",
            "tar",
            "tokio-tar",
            "unrar",
            "zip",
            "zip-extract",
        ])
    );
}

/// Verifies: SEC-HIS-035
#[test]
fn no_format_that_builds_arbitrary_types_can_be_linked() {
    assert_eq!(
        citing(&denied_crates(), "SEC-HIS-035"),
        sorted(&["serde-pickle", "jaded", "serde_yaml", "serde_yml"])
    );
}

/// Verifies: SEC-STD-010
#[test]
fn excluded_protocols_cannot_be_linked() {
    assert_eq!(
        citing(&denied_crates(), "SEC-STD-010"),
        sorted(&[
            "samael",
            "saml-rs",
            "ldap3",
            "ldap3_proto",
            "ldap-rs",
            "simple-ldap",
            "twilio",
            "async-graphql",
            "juniper",
            "graphql-parser",
            "graphql_client",
            "cynic",
            "apollo-parser",
            "apollo-compiler",
            "webrtc",
            "rtc",
            "str0m",
            "webrtc-ice",
            "webrtc-dtls",
            "stun",
            "stun-rs",
            "turn",
            "memcache",
            "sxd-xpath",
            "xrust",
            "libxslt",
        ])
    );
}

/// Verifies: SEC-STD-011
#[test]
fn only_a_linear_time_regex_engine_can_be_linked() {
    assert_eq!(
        citing(&denied_crates(), "SEC-STD-011"),
        sorted(&[
            "fancy-regex",
            "pcre2",
            "pcre2-sys",
            "pcre",
            "onig",
            "onig_sys",
            "regress",
        ])
    );
}

/// Verifies: SEC-PRV-009
#[test]
fn no_telemetry_or_crash_reporting_sdk_can_be_linked() {
    assert_eq!(
        citing(&denied_crates(), "SEC-PRV-009"),
        sorted(&[
            "amplitude",
            "bugsnag",
            "datadog-tracing",
            "honeybadger",
            "newrelic",
            "opentelemetry",
            "opentelemetry-otlp",
            "opentelemetry_sdk",
            "posthog-rs",
            "rollbar",
            "segment",
            "sentry",
            "sentry-core",
        ])
    );
}

/// Verifies: SEC-MED-025
#[test]
fn no_c_media_image_font_or_subtitle_library_can_be_linked() {
    // The same bans serve SEC-TM-034, but a list of names cannot fail for a
    // native crate it does not name, so this test does not claim it. The
    // gate's `xtask native-code` step proves it: see
    // `the_gate_compares_what_ships_with_the_native_code_allow_list`.
    let expected = sorted(&[
        "ffmpeg-next",
        "ffmpeg-sys",
        "ffmpeg-sys-next",
        "rsmpeg",
        "rusty_ffmpeg",
        "libwebp",
        "libwebp-sys",
        "webp",
        "libpng-sys",
        "mozjpeg",
        "mozjpeg-sys",
        "turbojpeg",
        "turbojpeg-sys",
        "libtiff-sys",
        "libheif-rs",
        "libheif-sys",
        "dav1d",
        "dav1d-sys",
        "magick_rust",
        "graphicsmagick",
        "graphicsmagick-sys",
        "librsvg",
        "gstreamer",
        "gstreamer-sys",
        "taglib",
        "taglib-sys",
        "freetype",
        "freetype-rs",
        "freetype-sys",
        "harfbuzz",
        "harfbuzz-sys",
        "harfbuzz_rs",
        "libass",
        "libass-sys",
    ]);
    for id in ["SEC-MED-025", "SEC-TM-034"] {
        assert_eq!(citing(&denied_crates(), id), expected, "{id}");
    }
    // And no crate of ours may hold unsafe code that could decode anything.
    assert_eq!(
        value(
            &table(WORKSPACE_MANIFEST, "workspace.lints.rust"),
            "unsafe_code"
        ),
        Some("\"forbid\"")
    );
}

/// Verifies: SEC-TM-034
#[test]
fn the_gate_compares_what_ships_with_the_native_code_allow_list() {
    // The gate hands `xtask native-code` every crate cargo resolves, with
    // every feature on, for exactly the targets deny.toml names. The check
    // itself is proved by xtask's tests (crates/xtask/src/native_code.rs).
    for line in [
        "cargo metadata \"$locked\" --format-version 1 --all-features --filter-platform x86_64-unknown-linux-gnu --filter-platform aarch64-unknown-linux-gnu --filter-platform i686-unknown-linux-gnu --filter-platform wasm32-unknown-unknown >target/native-code.json",
        "cargo run \"$locked\" -q -p xtask -- native-code target/native-code.json",
    ] {
        assert!(has_line(GATE, line));
    }
    assert_eq!(
        rules(&array(&table(DENY, "graph"), "targets")),
        [
            "\"x86_64-unknown-linux-gnu\",",
            "\"aarch64-unknown-linux-gnu\",",
            "\"i686-unknown-linux-gnu\",",
            "\"wasm32-unknown-unknown\",",
        ]
    );
}

/// Verifies: SEC-EXT-018, SEC-HIS-056
#[test]
fn sqlite_cannot_load_an_extension() {
    // rusqlite's `load_extension` feature lets a connection load a native
    // library by path. cargo-deny refuses the feature wherever it is turned
    // on, next to the one other feature ban.
    let bans: Vec<Vec<&str>> = DENY
        .split("\n[[bans.features]]\n")
        .skip(1)
        .map(|rest| rules(&table(rest, "")))
        .collect();
    assert_eq!(
        bans,
        [
            [
                "crate = \"rand\"",
                "deny = [\"small_rng\"]",
                "reason = \"SmallRng is a non-cryptographic generator (SEC-STD-022)\"",
            ],
            [
                "crate = \"rusqlite\"",
                "deny = [\"load_extension\"]",
                "reason = \"loads a native library into the server at run time (SEC-EXT-018, SEC-HIS-056)\"",
            ],
        ]
    );
}

// ---------------------------------------------------------------------------
// The supply chain.
// ---------------------------------------------------------------------------

/// Verifies: SEC-SUP-020
#[test]
fn every_cargo_command_in_the_gate_refuses_a_stale_lock_file() {
    // cargo fmt resolves nothing; every other command passes --locked, or
    // --frozen when GATE_OFFLINE=1 sets "$locked" to it.
    assert_eq!(
        cargo_commands(GATE),
        [
            "cargo fmt --all -- --check",
            "cargo clippy --locked --workspace --all-targets -- -D warnings",
            "cargo deny \"$locked\" check",
            "cargo vet --locked \"${vet_offline[@]}\"",
            "cargo tree \"$locked\" -p gunmetal-core -e normal --target all --all-features --prefix none --format '{p}' >target/core-deps.txt",
            "cargo run \"$locked\" -q -p xtask -- core-deps target/core-deps.txt",
            "cargo metadata \"$locked\" --format-version 1 --all-features --filter-platform x86_64-unknown-linux-gnu --filter-platform aarch64-unknown-linux-gnu --filter-platform i686-unknown-linux-gnu --filter-platform wasm32-unknown-unknown >target/native-code.json",
            "cargo run \"$locked\" -q -p xtask -- native-code target/native-code.json",
            "cargo llvm-cov --locked --workspace \\",
            "cargo test --locked --workspace --doc",
            "cargo mutants --workspace --no-shuffle \"${scope[@]}\" --cargo-arg=--locked",
        ]
    );
    assert!(has_line(GATE, "locked=--locked"));
    assert!(has_line(GATE, "locked=--frozen"));
    // CI proves it: a crate added without a lock file update fails the gate.
    assert!(has_line(CI, "locked-self-test:"));
    assert!(has_line(
        CI,
        "grep -F \"because --locked was passed to prevent this\" gate.log"
    ));
    assert!(has_line(
        CI,
        "- run: cargo install cargo-vet --version 0.10.2 --locked"
    ));
}

/// The full mutation run is split across jobs and loses no mutant: the
/// gate's switches cannot be combined into a run that tests nothing, the
/// shards partition one list, and the one required check fails unless every
/// job it waits for ended as it should.
#[test]
fn the_sharded_gate_runs_every_mutant_behind_one_required_check() {
    // The script refuses a switch it cannot read and any two together.
    for line in [
        r#"*) usage "GATE_SKIP_MUTANTS must be 0 or 1, not '$skip_mutants'" ;;"#,
        r#"if [[ -n "$mutants_shard" && ! "$mutants_shard" =~ ^(0|[1-9][0-9]{0,3})/([1-9][0-9]{0,3})$ ]]; then"#,
        r#"if [[ -n "$mutants_shard" ]] && ((BASH_REMATCH[1] >= BASH_REMATCH[2])); then"#,
        r"if ((${#switches[@]} > 1)); then",
        r#"usage "${switches[*]} cannot be combined: each one chooses which mutants this run tests""#,
        // A shard is one slice of the same list the unscoped run tests.
        r#"scope=(--shard "$mutants_shard" --sharding round-robin)"#,
        r#"cargo mutants --workspace --no-shuffle "${scope[@]}" --cargo-arg=--locked"#,
    ] {
        assert!(has_line(GATE, line), "{line}");
    }
    // What runs: every other step unless this is a shard, and mutation
    // testing unless it is skipped. Nothing follows that could undo it.
    assert!(GATE.ends_with(
        r#"
if [[ -z "$mutants_shard" ]]; then
  checks
fi
if [[ "$skip_mutants" == 1 ]]; then
  echo "==> mutation testing skipped (GATE_SKIP_MUTANTS=1)"
else
  mutants
fi
"#
    ));

    assert_eq!(
        workflow_jobs(CI),
        [
            "checks",
            "mutants",
            "core-32-bit",
            "locked-self-test",
            "gate-switches-self-test",
            "gate",
        ]
    );
    let diff_scoped = "github.event_name == 'pull_request' && startsWith(github.base_ref, 'wave-')";

    // Without a diff scope, the checks job leaves mutation testing to the
    // shards. With one, it is the whole gate and the shards do not run.
    let checks = workflow_job(CI, "checks");
    for line in [
        format!(
            "GATE_MUTANTS_DIFF: ${{{{ {diff_scoped} && format('origin/{{0}}', github.base_ref) || '' }}}}"
        ),
        format!("GATE_SKIP_MUTANTS: ${{{{ {diff_scoped} && '0' || '1' }}}}"),
        "- run: scripts/gate.sh".to_owned(),
    ] {
        assert!(checks.contains(&line.as_str()), "{line}");
    }

    // Shards 0 to 9 of 10: each index once, and a failed shard does not
    // cancel the others, so one run reports every missed mutant.
    let mutants = workflow_job(CI, "mutants");
    for line in [
        format!("if: ${{{{ !({diff_scoped}) }}}}"),
        "fail-fast: false".to_owned(),
        "shard: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]".to_owned(),
        "GATE_MUTANTS_SHARD: ${{ matrix.shard }}/10".to_owned(),
        "- run: scripts/gate.sh".to_owned(),
    ] {
        assert!(mutants.contains(&line.as_str()), "{line}");
    }

    // The required check waits for every other job, runs even when one of
    // them failed or was cancelled (a skipped job would count as passed),
    // and compares each result with the one this kind of run must have.
    let gate = workflow_job(CI, "gate");
    let others: Vec<&str> = workflow_jobs(CI)
        .into_iter()
        .filter(|job| *job != "gate")
        .collect();
    assert!(gate.contains(&"if: ${{ always() }}"));
    assert!(gate.contains(&format!("needs: [{}]", others.join(", ")).as_str()));
    for line in [
        format!("DIFF_SCOPED: ${{{{ {diff_scoped} }}}}"),
        "NEEDS: ${{ toJSON(needs) }}".to_owned(),
        "mutants=success".to_owned(),
        "if [[ \"$DIFF_SCOPED\" == true ]]; then".to_owned(),
        "mutants=skipped".to_owned(),
        r#"wrong=$(jq -r --arg mutants "$mutants" '[to_entries[] | select(.value.result != (if .key == "mutants" then $mutants else "success" end)) | "\(.key) ended as \(.value.result)"] | join(", ")' <<<"$NEEDS")"#.to_owned(),
        "if [[ -n \"$wrong\" ]]; then".to_owned(),
        "exit 1".to_owned(),
    ] {
        assert!(gate.contains(&line.as_str()), "{line}");
    }

    // No job can hang: each has a limit.
    for job in workflow_jobs(CI) {
        let limits = workflow_job(CI, job)
            .into_iter()
            .filter(|line| line.starts_with("timeout-minutes: "))
            .count();
        assert_eq!(limits, 1, "{job}");
    }
}

/// Verifies: SEC-SUP-021
#[test]
fn the_dependency_policy_runs_on_every_change_and_every_day() {
    let advisories = table(DENY, "advisories");
    assert_eq!(value(&advisories, "unmaintained"), Some("\"all\""));
    assert_eq!(value(&advisories, "unsound"), Some("\"all\""));
    assert_eq!(value(&advisories, "yanked"), Some("\"deny\""));
    assert_eq!(value(&table(DENY, "bans"), "wildcards"), Some("\"deny\""));
    assert_eq!(
        rules(&table(DENY, "sources")),
        [
            "unknown-registry = \"deny\"",
            "unknown-git = \"deny\"",
            "allow-registry = [\"https://github.com/rust-lang/crates.io-index\"]",
            "allow-git = []",
        ]
    );
    assert!(has_line(GATE, "cargo deny \"$locked\" check"));
    assert!(has_line(DAILY_DENY, "- cron: \"23 5 * * *\""));
    assert!(has_line(DAILY_DENY, "- run: cargo deny --locked check"));
}

/// Verifies: SEC-SUP-022
#[test]
fn every_ignored_advisory_has_a_recorded_exception() {
    assert_eq!(
        unmatched_advisories(DENY, EXCEPTIONS),
        (Vec::new(), Vec::new())
    );
    // Checked against today's date (UTC), so the gate starts failing on the
    // day an exception lapses, nightly runs included.
    let today = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock is set after 1970")
        .as_secs()
        .checked_div(86_400)
        .and_then(|days| i64::try_from(days).ok())
        .expect("today fits in i64 days");
    assert_eq!(refused_exceptions(EXCEPTIONS, today), Vec::new());
    // Nothing is ignored today.
    assert_eq!(ignored_advisories(DENY), Vec::<&str>::new());
}

/// Verifies: SEC-SUP-023
#[test]
fn a_failed_daily_check_starts_the_remediation_clock() {
    assert!(has_line(DAILY_DENY, "issues: write"));
    assert!(has_line(
        DAILY_DENY,
        "--body \"The scheduled cargo-deny check failed: $RUN_URL. Remediation deadlines run from today (SEC-SUP-023): a fixed release within 7 days for a critical or high advisory reachable from shipped code, 30 days for medium, 90 days for low, and 180 days to replace, fork or justify an unmaintained crate.\""
    ));
}

/// Verifies: SEC-SUP-024
#[test]
fn every_crate_is_audited_or_exempted_until_a_review_date() {
    assert!(has_line(GATE, "cargo vet --locked \"${vet_offline[@]}\""));
    assert_eq!(undated_exemptions(VET_CONFIG), Vec::<&str>::new());
    assert_eq!(
        quoted(&table(VET_CONFIG, "imports.mozilla").concat()),
        ["https://raw.githubusercontent.com/mozilla/supply-chain/main/audits.toml"]
    );
}

/// Verifies: SEC-SUP-026
#[test]
fn only_reviewed_crates_run_build_scripts() {
    assert_eq!(
        rules(&table(DENY, "bans.build"))
            .into_iter()
            .filter(|line| !line.starts_with('"') && *line != "]")
            .collect::<Vec<_>>(),
        [
            "executables = \"deny\"",
            "interpreted = \"deny\"",
            "include-dependencies = true",
            "include-workspace = true",
            "include-archives = true",
            "allow-build-scripts = [",
        ]
    );
    assert_eq!(
        build_script_crates(DENY),
        [
            "cap-primitives",
            "cap-std",
            "getrandom",
            "io-extras",
            "io-lifetimes",
            "libc",
            "libsqlite3-sys",
            "num-traits",
            "proc-macro2",
            "quote",
            "rustix",
            "serde",
            "serde_core",
            "serde_json",
            "thiserror",
            "zerocopy",
            "zmij",
        ]
    );
    // A bypass is reviewed for one version, so an update is reviewed again.
    assert_eq!(
        blocks(DENY, "[[bans.build.bypass]]")
            .into_iter()
            .map(|(_, lines)| rules(&lines))
            .collect::<Vec<_>>(),
        [
            [
                "crate = \"zerocopy@0.8.59\"",
                "allow-globs = [\"cargo.sh\", \"ci/*.sh\"]",
            ],
            [
                "crate = \"vcpkg@0.2.15\"",
                "allow-globs = [\"setup_vcp.sh\", \"tests/*.sh\"]",
            ],
        ]
    );
    // The workspace has no build script of its own.
    assert!(has_line(
        GATE,
        "if git ls-files --cached --others --exclude-standard -- 'build.rs' '*/build.rs' | grep .; then"
    ));
}

/// Verifies: SEC-SUP-029
#[test]
fn only_licences_compatible_with_the_agpl_are_allowed() {
    assert_eq!(
        quoted(&array(&table(DENY, "licenses"), "allow").concat()),
        [
            "0BSD",
            "AGPL-3.0-or-later",
            "Apache-2.0",
            "Apache-2.0 WITH LLVM-exception",
            "BSD-2-Clause",
            "BSD-3-Clause",
            "BSL-1.0",
            "CC0-1.0",
            "GPL-3.0-or-later",
            "ISC",
            "LGPL-2.1-or-later",
            "LGPL-3.0-or-later",
            "MIT",
            "MPL-2.0",
            "Unicode-3.0",
            "Unlicense",
            "Zlib",
        ]
    );
    // A clarification overrides what a crate says its licence is, so none
    // is made without a code owner adding it here.
    assert!(!has_line(DENY, "[[licenses.clarify]]"));
}

/// Verifies: SEC-SUP-038
#[test]
fn every_build_uses_one_exact_toolchain_release() {
    let channel = value(&table(TOOLCHAIN, "toolchain"), "channel")
        .and_then(|quoted_channel| quoted(quoted_channel).first().copied())
        .unwrap_or_default();
    let parts: Vec<&str> = channel.split('.').collect();
    assert_eq!(parts.len(), 3, "{channel} is not an exact release");
    assert!(
        parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit())),
        "{channel} is not an exact release"
    );
    assert_eq!(workflow_toolchains(CI), [channel; 4]);
    assert_eq!(workflow_toolchains(DAILY_DENY), [channel]);
}

/// Verifies: SEC-STD-033
#[test]
fn release_builds_trap_integer_overflow() {
    assert_eq!(
        rules(&table(WORKSPACE_MANIFEST, "profile.release")),
        ["overflow-checks = true"]
    );
}
