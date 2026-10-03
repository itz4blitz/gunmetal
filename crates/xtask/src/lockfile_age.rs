//! `xtask lockfile-age`: a pull request must not add or change a crate
//! version published less than seven days earlier (SEC-SUP-027). Malicious
//! versions are usually found and yanked within hours, so a week's wait
//! leaves a wide margin (supply-chain-and-release.md, section 5).
//!
//! The xtask makes no network request. `requests` prints a curl
//! configuration that fetches the crates.io record of every version the
//! pull request adds; the workflow runs curl with it, then `check` reads the
//! saved records. Tests use recorded responses, never the network. A pull
//! request that carries the override label is checked by
//! `lockfile-age override` (`age_override.rs`) instead.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use crate::lockfile::{self, Package};
use crate::tree::Tree;

/// Seven days, in seconds.
pub const MIN_AGE: u64 = 604_800;

/// Something `lockfile-age` found wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finding {
    /// A name or version with characters a crates.io URL or a file name
    /// must not carry, so its record is never fetched.
    Malformed {
        /// The crate's name as locked.
        name: String,
        /// The version as locked.
        version: String,
    },
    /// No recorded crates.io response for the version.
    NoResponse {
        /// The crate.
        name: String,
        /// The version.
        version: String,
    },
    /// A response with no publish time the check can read.
    NoPublishTime {
        /// The crate.
        name: String,
        /// The version.
        version: String,
    },
    /// Published less than [`MIN_AGE`] before `now`, or after it.
    TooNew {
        /// The crate.
        name: String,
        /// The version.
        version: String,
        /// When crates.io says it was published, in seconds since the epoch.
        published: u64,
        /// The time of the check, in seconds since the epoch.
        now: u64,
    },
}

/// The registry crate versions in `head` that `base` does not lock.
fn added(base: &str, head: &str) -> Vec<Package> {
    let locked: BTreeSet<Package> = lockfile::packages(base).into_iter().collect();
    lockfile::packages(head)
        .into_iter()
        .filter(|package| package.source.is_some() && !locked.contains(package))
        .collect()
}

/// The versions `head` adds to `base` whose records are safe to fetch, and
/// a finding for each of the rest.
fn sorted(base: &str, head: &str) -> (Vec<Package>, Vec<Finding>) {
    let mut safe = Vec::new();
    let mut malformed = Vec::new();
    for package in added(base, head) {
        if is_safe(&package.name) && is_safe(&package.version) {
            safe.push(package);
        } else {
            malformed.push(Finding::Malformed {
                name: package.name,
                version: package.version,
            });
        }
    }
    (safe, malformed)
}

/// Whether `text` can go into a crates.io URL and a file name as it is:
/// ASCII letters, digits, `+`, `-`, `.` and `_`, and not empty.
fn is_safe(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "+-._".contains(character))
}

/// A curl configuration that saves the crates.io record of every version
/// `head` adds to `base` as `<name>@<version>.json`, or the versions whose
/// name or version is malformed.
pub fn requests(base: &str, head: &str) -> Result<String, Vec<Finding>> {
    let (safe, malformed) = sorted(base, head);
    if !malformed.is_empty() {
        return Err(malformed);
    }
    let mut config = String::new();
    for Package { name, version, .. } in safe {
        let _ = writeln!(
            config,
            "url = \"https://crates.io/api/v1/crates/{name}/{version}\""
        );
        let _ = writeln!(config, "output = \"{name}@{version}.json\"");
    }
    Ok(config)
}

/// Checks every version `head` adds to `base` against its record in
/// directory `responses`, at time `now`.
pub fn check(tree: &dyn Tree, responses: &str, base: &str, head: &str, now: u64) -> Vec<Finding> {
    let (safe, mut findings) = sorted(base, head);
    for Package { name, version, .. } in safe {
        let Some(response) = tree.read(&format!("{responses}/{name}@{version}.json")) else {
            findings.push(Finding::NoResponse { name, version });
            continue;
        };
        let Some(published) = published(&response) else {
            findings.push(Finding::NoPublishTime { name, version });
            continue;
        };
        if now.checked_sub(published).is_none_or(|age| age < MIN_AGE) {
            findings.push(Finding::TooNew {
                name,
                version,
                published,
                now,
            });
        }
    }
    findings
}

/// The publish time in a crates.io version record: the first `created_at`,
/// which is the version's own (the publisher's comes later).
fn published(response: &str) -> Option<u64> {
    let (_, rest) = response.split_once("\"created_at\":\"")?;
    let (text, _) = rest.split_once('"')?;
    timestamp(text)
}

/// Seconds since the epoch of an RFC 3339 time in UTC from 1970 on, such
/// as `2026-06-04T18:55:34.718132Z` or `2026-06-04T18:55:34+00:00`.
fn timestamp(text: &str) -> Option<u64> {
    let (date, time) = text.split_once('T')?;
    let (clock, zone) = time.split_at_checked(8)?;
    let zone = zone.strip_prefix('.').map_or(zone, |fraction| {
        fraction.trim_start_matches(|character: char| character.is_ascii_digit())
    });
    if zone != "Z" && zone != "+00:00" {
        return None;
    }
    let [year, month, day] = numbers(date, '-')?;
    let [hour, minute, second] = numbers(clock, ':')?;
    if year < 1970 || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(days_from_civil(year, month, day) * 86_400 + hour * 3_600 + minute * 60 + second)
}

/// The three numbers in `text`, separated by `separator`.
fn numbers(text: &str, separator: char) -> Option<[u64; 3]> {
    let parts: Vec<&str> = text.split(separator).collect();
    let [first, second, third] = parts.as_slice() else {
        return None;
    };
    Some([
        first.parse().ok()?,
        second.parse().ok()?,
        third.parse().ok()?,
    ])
}

/// Days from 1970-01-01 to a date from 1970 on in the Gregorian calendar,
/// by Howard Hinnant's `days_from_civil`, which counts years from March so
/// that the leap day comes last.
fn days_from_civil(year: u64, month: u64, day: u64) -> u64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year / 400;
    let year_of_era = year - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::{Finding, check, published, requests, timestamp};
    use crate::tree::memory::Memory;

    /// The crates.io index, as `Cargo.lock` names it.
    const CRATES_IO: &str = "registry+https://github.com/rust-lang/crates.io-index";

    /// 2026-10-03T00:00:00Z.
    const NOW: u64 = 1_790_985_600;

    /// A lock file holding the workspace and `crates`, as (name, version).
    fn lock(crates: &[(&str, &str)]) -> String {
        let mut text = String::from(
            "version = 4\n\n[[package]]\nname = \"gunmetal-core\"\nversion = \"0.0.0\"\n",
        );
        for (name, version) in crates {
            let _ = write!(
                text,
                "\n[[package]]\nname = \"{name}\"\nversion = \"{version}\"\nsource = \"{CRATES_IO}\"\n"
            );
        }
        text
    }

    /// A crates.io version record published at `created_at`.
    fn record(created_at: &str) -> String {
        format!(
            "{{\"version\":{{\"id\":1,\"crate\":\"x\",\"num\":\"1\",\"updated_at\":\"2026-10-02T00:00:00Z\",\"created_at\":\"{created_at}\",\"downloads\":0}}}}"
        )
    }

    /// Checks a pull request that adds only `serde 1.0.300`, published at
    /// `created_at`.
    fn check_serde_published_at(created_at: &str) -> Vec<Finding> {
        let tree = Memory::default().with("responses/serde@1.0.300.json", &record(created_at));
        check(
            &tree,
            "responses",
            &lock(&[]),
            &lock(&[("serde", "1.0.300")]),
            NOW,
        )
    }

    /// Verifies: SEC-SUP-027
    #[test]
    fn a_crate_published_three_days_ago_fails() {
        assert_eq!(
            check_serde_published_at("2026-09-30T00:00:00.000000Z"),
            [Finding::TooNew {
                name: "serde".to_owned(),
                version: "1.0.300".to_owned(),
                published: 1_790_726_400,
                now: NOW,
            }]
        );
    }

    /// Verifies: SEC-SUP-027
    #[test]
    fn a_crate_published_eight_days_ago_passes() {
        assert_eq!(check_serde_published_at("2026-09-25T00:00:00.000000Z"), []);
    }

    /// Verifies: SEC-SUP-027
    #[test]
    fn seven_days_to_the_second_is_old_enough() {
        assert_eq!(check_serde_published_at("2026-09-26T00:00:00Z"), []);
        assert_eq!(
            check_serde_published_at("2026-09-26T00:00:01Z"),
            [Finding::TooNew {
                name: "serde".to_owned(),
                version: "1.0.300".to_owned(),
                published: 1_790_380_801,
                now: NOW,
            }]
        );
    }

    /// Verifies: SEC-SUP-027
    #[test]
    fn a_publish_time_after_now_fails() {
        assert_eq!(
            check_serde_published_at("2026-10-04T00:00:00Z"),
            [Finding::TooNew {
                name: "serde".to_owned(),
                version: "1.0.300".to_owned(),
                published: 1_791_072_000,
                now: NOW,
            }]
        );
    }

    /// Verifies: SEC-SUP-027
    #[test]
    fn only_versions_the_pull_request_adds_or_changes_are_checked() {
        // `libc` keeps its version, `rand` moves to a new one and `zerocopy`
        // is new. Only the last two need a record, and both are too young.
        let base = lock(&[("libc", "0.2.190"), ("rand", "0.9.4")]);
        let head = lock(&[
            ("libc", "0.2.190"),
            ("rand", "0.9.5"),
            ("zerocopy", "0.8.59"),
        ]);
        let young = record("2026-10-01T00:00:00Z");
        let tree = Memory::default()
            .with("responses/libc@0.2.190.json", &young)
            .with("responses/rand@0.9.5.json", &young)
            .with("responses/zerocopy@0.8.59.json", &young);
        let too_new = |name: &str, version: &str| Finding::TooNew {
            name: name.to_owned(),
            version: version.to_owned(),
            published: 1_790_812_800,
            now: NOW,
        };
        assert_eq!(
            check(&tree, "responses", &base, &head, NOW),
            [too_new("rand", "0.9.5"), too_new("zerocopy", "0.8.59")]
        );
    }

    /// Verifies: SEC-SUP-027
    #[test]
    fn a_version_moved_to_another_source_is_checked_again() {
        let base = lock(&[("serde", "1.0.300")]);
        let head = base.replace(CRATES_IO, "sparse+https://index.crates.io/");
        assert_eq!(
            check(&Memory::default(), "responses", &base, &head, NOW),
            [Finding::NoResponse {
                name: "serde".to_owned(),
                version: "1.0.300".to_owned(),
            }]
        );
    }

    /// Verifies: SEC-SUP-027
    #[test]
    fn a_version_without_a_readable_record_fails() {
        let head = lock(&[("missing", "1.0.0"), ("garbled", "1.0.0")]);
        let tree = Memory::default().with("responses/garbled@1.0.0.json", "{\"errors\":[]}");
        assert_eq!(
            check(&tree, "responses", &lock(&[]), &head, NOW),
            [
                Finding::NoResponse {
                    name: "missing".to_owned(),
                    version: "1.0.0".to_owned(),
                },
                Finding::NoPublishTime {
                    name: "garbled".to_owned(),
                    version: "1.0.0".to_owned(),
                },
            ]
        );
    }

    /// Verifies: SEC-SUP-027
    #[test]
    fn a_name_or_version_that_could_escape_the_url_or_file_name_is_refused() {
        let head = lock(&[
            ("../../etc", "1.0.0"),
            ("serde", "1.0.0/../x"),
            ("spaced name", "1.0.0"),
            ("", "1.0.0"),
            ("ok-name_2", "1.0.0-rc.1+build.5"),
        ]);
        let malformed = |name: &str, version: &str| Finding::Malformed {
            name: name.to_owned(),
            version: version.to_owned(),
        };
        let expected = vec![
            malformed("../../etc", "1.0.0"),
            malformed("serde", "1.0.0/../x"),
            malformed("spaced name", "1.0.0"),
            malformed("", "1.0.0"),
        ];
        assert_eq!(requests(&lock(&[]), &head), Err(expected.clone()));
        let tree = Memory::default().with(
            "responses/ok-name_2@1.0.0-rc.1+build.5.json",
            &record("2026-01-01T00:00:00Z"),
        );
        assert_eq!(check(&tree, "responses", &lock(&[]), &head, NOW), expected);
    }

    #[test]
    fn requests_fetch_the_record_of_each_added_version_from_crates_io() {
        let base = lock(&[("libc", "0.2.190")]);
        let head = lock(&[("libc", "0.2.190"), ("rand", "0.9.5"), ("serde", "1.0.300")]);
        assert_eq!(
            requests(&base, &head),
            Ok("\
url = \"https://crates.io/api/v1/crates/rand/0.9.5\"
output = \"rand@0.9.5.json\"
url = \"https://crates.io/api/v1/crates/serde/1.0.300\"
output = \"serde@1.0.300.json\"
"
            .to_owned())
        );
        assert_eq!(requests(&base, &base), Ok(String::new()));
    }

    #[test]
    fn reads_the_publish_time_from_a_recorded_crates_io_response() {
        let recorded = include_str!("../fixtures/lockfile-age/responses/libfuzzer-sys@0.4.13.json");
        assert_eq!(published(recorded), Some(1_780_599_334));
        assert_eq!(published("{\"version\":{\"created_at\":\"soon\"}}"), None);
        assert_eq!(
            published("{\"version\":{\"created_at\":\"2026-06-04T18:55:34Z"),
            None
        );
        assert_eq!(published("{}"), None);
    }

    #[test]
    fn converts_utc_times_to_seconds_since_the_epoch() {
        // Reference values from GNU date, `date -u -d <time> +%s`.
        for (text, seconds) in [
            ("1970-01-01T00:00:00Z", 0),
            ("1970-03-01T00:00:00Z", 5_097_600),
            ("1972-02-29T12:00:00Z", 68_212_800),
            ("2000-02-29T23:59:59Z", 951_868_799),
            ("2000-03-01T00:00:00+00:00", 951_868_800),
            ("2026-01-31T01:02:03.5+00:00", 1_769_821_323),
            ("2026-06-04T18:55:34.718132Z", 1_780_599_334),
            ("2100-02-28T00:00:00Z", 4_107_456_000),
            ("2100-03-01T00:00:00Z", 4_107_542_400),
        ] {
            assert_eq!(timestamp(text), Some(seconds), "{text}");
        }
    }

    #[test]
    fn refuses_times_it_cannot_read_exactly() {
        for text in [
            "",
            "1969-12-31T23:59:59Z",
            "2026-00-10T00:00:00Z",
            "2026-13-10T00:00:00Z",
            "2026-01-00T00:00:00Z",
            "2026-01-32T00:00:00Z",
            "2026-06-04T18:55:34+01:00",
            "2026-06-04T18:55:34",
            "2026-06-04 18:55:34Z",
            "2026-06-04T18:55Z",
            "x026-06-04T18:55:34Z",
            "2026-x6-04T18:55:34Z",
            "2026-06-x4T18:55:34Z",
            "2026-06-04T18:55:3xZ",
            "2026-06-04T18:55:34:00Z",
            "2026-06-04-01T00:00:00Z",
            "2026-06T00:00:00Z",
        ] {
            assert_eq!(timestamp(text), None, "{text}");
        }
    }
}
