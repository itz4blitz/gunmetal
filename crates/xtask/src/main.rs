//! Repository checks for Gunmetal: `cargo run --locked -p xtask -- <check>`.
//!
//! The security baseline asks for its CI checks as code under the same gate
//! as everything else, not as shell in workflow files
//! (supply-chain-and-release.md, section 6). This file is a registry: one
//! `mod` line and one dispatch line per subcommand, and each check lives in
//! its own file, owned by the package that adds it.
//!
//! - `check-harnesses`: every parser entry point in `gunmetal-core` has a
//!   registered fuzz harness, and every harness has its files (SEC-MED-027,
//!   SEC-MED-031).
//! - `core-deps <cargo-tree-output>`: `gunmetal-core`'s normal dependencies
//!   are exactly the reviewed allowlist (SEC-SUP-025).
//! - `fuzz-targets`: the registered harnesses as a JSON array, for the fuzz
//!   workflow's job matrix.
//! - `lint-exceptions`: only the modules on the written list turn a clippy
//!   ban off, and no cargo configuration file in the repository can.
//! - `lockfile-age requests <base-lock> <head-lock>` and
//!   `lockfile-age check <base-lock> <head-lock> <responses-dir>`: no crate
//!   version published less than seven days ago (SEC-SUP-027).
//! - `lockfile-age override <codeowners> <reviews> <head-sha>`: a code
//!   owner of every lock file approved commit `<head-sha>`, which a pull
//!   request carrying the override label needs instead (SEC-SUP-027).
//! - `js-deps`: every direct JavaScript dependency is listed with a reason
//!   (SEC-SUP-035).
//! - `repo`: repository protections, workflow pinning, REUSE, runbooks and
//!   CODEOWNERS (WP-124).
//! - `repo settings <live-dir>`: live GitHub dumps against the expected
//!   policy (SEC-SUP-001, SEC-SUP-003 to SEC-SUP-007, SEC-SUP-010,
//!   SEC-SUP-018).
//! - `repo scorecard <json>`: Scorecard thresholds (SEC-SUP-019).
//! - `repo advisories <json>`: every published advisory has a test named
//!   after it (SEC-TM-003).
//! - `repo codeql <sarif>`: no `CodeQL` result at `error` level or security
//!   severity 7.0 or more (SEC-SUP-018).
//!
//! Paths are relative to the repository root. A check that finds problems
//! exits with status 1 and lists them. `check-harnesses` and
//! `lint-exceptions` also run against the real repository in this crate's
//! tests, so the gate enforces them on every change.

mod age_override;
mod codeowners;
mod core_deps;
mod harnesses;
mod js_deps;
mod json;
mod lint_exceptions;
mod lockfile;
mod lockfile_age;
mod repo;
mod toml;
mod tree;

use std::env;
use std::fmt::Debug;
use std::io::{self, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use gunmetal_fuzz::registry;

use crate::tree::{Disk, Tree};

/// The repository root, from where this crate was compiled.
const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

/// Why a run failed. `main` prints it and exits with status 1.
#[derive(Debug, PartialEq, Eq)]
enum Failure {
    /// The arguments name no subcommand. The crate documentation lists them.
    Usage,
    /// An input file named on the command line could not be read.
    Unreadable(String),
    /// The check found these problems.
    Findings(Vec<String>),
    /// Standard output could not be written.
    Output,
}

fn main() -> Result<(), Failure> {
    let args: Vec<String> = env::args().skip(1).collect();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let registered: Vec<&str> = registry::all().iter().map(|harness| harness.name).collect();
    dispatch(&args, ROOT, now, &registered, &mut io::stdout())
}

/// Runs the subcommand `args` names against the repository at `root`, at
/// time `now` in seconds since the epoch, with the `registered` fuzz
/// harnesses, writing any output to `out`.
fn dispatch(
    args: &[String],
    root: &str,
    now: u64,
    registered: &[&str],
    out: &mut dyn Write,
) -> Result<(), Failure> {
    let tree = Disk::new(root);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match *args.as_slice() {
        ["check-harnesses"] => report(harnesses::check(&tree, registered)),
        ["core-deps", output] => report(core_deps::check(
            &read(&tree, output)?,
            &tree.read(core_deps::ALLOWLIST).unwrap_or_default(),
        )),
        ["fuzz-targets"] => write(out, &harnesses::targets_json(registered).map_err(rendered)?),
        ["js-deps"] => report(js_deps::check(&tree)),
        ["lint-exceptions"] => report(lint_exceptions::check(&tree, lint_exceptions::EXCEPTIONS)),
        ["lockfile-age", "check", base, head, responses] => report(lockfile_age::check(
            &tree,
            responses,
            &read(&tree, base)?,
            &read(&tree, head)?,
            now,
        )),
        ["lockfile-age", "override", codeowners, reviews, head] => report(age_override::check(
            &read(&tree, codeowners)?,
            &read(&tree, reviews)?,
            head,
        )),
        ["lockfile-age", "requests", base, head] => write(
            out,
            &lockfile_age::requests(&read(&tree, base)?, &read(&tree, head)?).map_err(rendered)?,
        ),
        ["repo"] => report(repo::check(&tree, now)),
        ["repo", "advisories", json] => report(repo::advisories(&tree, &read(&tree, json)?)),
        ["repo", "codeql", sarif] => report(repo::codeql(&read(&tree, sarif)?)),
        ["repo", "scorecard", json] => report(repo::scorecard(&read(&tree, json)?)),
        ["repo", "settings", dir] => report(repo::settings(&tree, dir)),
        _ => Err(Failure::Usage),
    }
}

/// The text of the input file `path`.
fn read(tree: &dyn Tree, path: &str) -> Result<String, Failure> {
    tree.read(path)
        .ok_or_else(|| Failure::Unreadable(path.to_owned()))
}

/// Success when there are no `findings`.
fn report<T: Debug>(findings: Vec<T>) -> Result<(), Failure> {
    if findings.is_empty() {
        Ok(())
    } else {
        Err(rendered(findings))
    }
}

/// `findings`, written out for the person reading the failure.
fn rendered<T: Debug>(findings: impl IntoIterator<Item = T>) -> Failure {
    Failure::Findings(
        findings
            .into_iter()
            .map(|finding| format!("{finding:?}"))
            .collect(),
    )
}

/// Writes `text` to `out`.
fn write(out: &mut dyn Write, text: &str) -> Result<(), Failure> {
    out.write_all(text.as_bytes()).map_err(|_| Failure::Output)
}

#[cfg(test)]
mod tests {
    use std::io::{self, Write};

    use super::{Failure, ROOT, dispatch, main, read, rendered, report, write};

    /// Small repositories for the subcommands that read named inputs.
    const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures");

    /// The publish time in the recorded response for libfuzzer-sys 0.4.13:
    /// 2026-06-04T18:55:34Z.
    const LIBFUZZER_PUBLISHED: u64 = 1_780_599_334;

    /// One day, in seconds.
    const DAY: u64 = 86_400;

    /// Runs `args` against the repository at `root` and returns the result
    /// with everything written to standard output.
    fn run_in(
        root: &str,
        args: &[&str],
        now: u64,
        registered: &[&str],
    ) -> (Result<(), Failure>, String) {
        let args: Vec<String> = args.iter().map(|&arg| arg.to_owned()).collect();
        let mut out = Vec::new();
        let result = dispatch(&args, root, now, registered, &mut out);
        (result, String::from_utf8(out).expect("output is UTF-8"))
    }

    /// The harnesses registered in `gunmetal-fuzz`.
    fn registered() -> Vec<&'static str> {
        gunmetal_fuzz::registry::all()
            .iter()
            .map(|harness| harness.name)
            .collect()
    }

    /// The failure that lists `findings`.
    fn findings(findings: &[&str]) -> Failure {
        Failure::Findings(findings.iter().map(|&finding| finding.to_owned()).collect())
    }

    /// A standard output that is closed.
    struct Closed;

    impl Write for Closed {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("closed"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("closed"))
        }
    }

    #[test]
    fn main_without_a_subcommand_reports_usage() {
        assert_eq!(main(), Err(Failure::Usage));
    }

    #[test]
    fn arguments_that_name_no_subcommand_report_usage() {
        for args in [
            &[][..],
            &["unknown"],
            &["check-harnesses", "extra"],
            &["core-deps"],
            &["lockfile-age"],
            &["lockfile-age", "check", "base", "head"],
            &["lockfile-age", "override", "codeowners", "reviews"],
            &["lockfile-age", "requests", "base"],
            &["js-deps", "extra"],
            &["repo", "settings"],
            &["repo", "scorecard"],
            &["repo", "advisories"],
            &["repo", "codeql"],
            &["repo", "unknown"],
        ] {
            assert_eq!(
                run_in(FIXTURES, args, 0, &[]),
                (Err(Failure::Usage), String::new()),
                "{args:?}"
            );
        }
    }

    /// Verifies: SEC-MED-027, SEC-MED-031, SEC-TM-033, SEC-HIS-036
    #[test]
    fn the_repository_has_a_registered_harness_for_every_parser_entry_point() {
        assert_eq!(
            run_in(ROOT, &["check-harnesses"], 0, &registered()),
            (Ok(()), String::new())
        );
    }

    #[test]
    fn check_harnesses_lists_what_it_finds() {
        // The fixture repository has none of the files a harness needs.
        assert_eq!(
            run_in(FIXTURES, &["check-harnesses"], 0, &["ebml"]),
            (
                Err(findings(&[
                    r#"Missing { harness: "ebml", path: "crates/gunmetal-fuzz/src/ebml.rs" }"#,
                    r#"Missing { harness: "ebml", path: "crates/gunmetal-fuzz/tests/ebml_corpus.rs" }"#,
                    r#"Missing { harness: "ebml", path: "fuzz/fuzz_targets/ebml.rs" }"#,
                    r#"NoTarget { harness: "ebml" }"#,
                ])),
                String::new()
            )
        );
    }

    #[test]
    fn the_repository_turns_no_clippy_ban_off_outside_the_written_list() {
        assert_eq!(
            run_in(ROOT, &["lint-exceptions"], 0, &[]),
            (Ok(()), String::new())
        );
    }

    /// Verifies: SEC-SUP-025
    #[test]
    fn core_deps_compares_the_cargo_tree_output_with_the_allowlist() {
        assert_eq!(
            run_in(FIXTURES, &["core-deps", "core-deps/allowed.txt"], 0, &[]),
            (Ok(()), String::new())
        );
        assert_eq!(
            run_in(FIXTURES, &["core-deps", "core-deps/unlisted.txt"], 0, &[]),
            (
                Err(findings(&[r#"Unlisted { name: "digest" }"#])),
                String::new()
            )
        );
    }

    /// Verifies: SEC-SUP-025
    #[test]
    fn core_deps_reads_a_missing_allowlist_as_the_empty_list() {
        let root = format!("{FIXTURES}/core-deps");
        assert_eq!(
            run_in(&root, &["core-deps", "alone.txt"], 0, &[]),
            (Ok(()), String::new())
        );
        assert_eq!(
            run_in(&root, &["core-deps", "allowed.txt"], 0, &[]),
            (
                Err(findings(&[r#"Unlisted { name: "sha2" }"#])),
                String::new()
            )
        );
    }

    #[test]
    fn fuzz_targets_prints_the_registered_harnesses_as_json() {
        assert_eq!(
            run_in(FIXTURES, &["fuzz-targets"], 0, &["ebml", "flac_metadata"]),
            (Ok(()), "[\"ebml\",\"flac_metadata\"]\n".to_owned())
        );
        assert_eq!(
            run_in(FIXTURES, &["fuzz-targets"], 0, &["ebml", "../up"]),
            (Err(findings(&[r#""../up""#])), String::new())
        );
    }

    /// Verifies: SEC-SUP-027
    #[test]
    fn lockfile_age_checks_each_added_version_against_its_record() {
        let args = [
            "lockfile-age",
            "check",
            "lockfile-age/base.lock",
            "lockfile-age/head.lock",
            "lockfile-age/responses",
        ];
        assert_eq!(
            run_in(FIXTURES, &args, LIBFUZZER_PUBLISHED + 8 * DAY, &[]),
            (Ok(()), String::new())
        );
        let now = LIBFUZZER_PUBLISHED + 3 * DAY;
        assert_eq!(
            run_in(FIXTURES, &args, now, &[]),
            (
                Err(Failure::Findings(vec![format!(
                    r#"TooNew {{ name: "libfuzzer-sys", version: "0.4.13", published: {LIBFUZZER_PUBLISHED}, now: {now} }}"#
                )])),
                String::new()
            )
        );
    }

    /// Verifies: SEC-SUP-027
    #[test]
    fn lockfile_age_override_needs_a_code_owner_approval_of_the_head_commit() {
        let head = "ecdd80bb57125d7ba9641ffaa4d7d2c19d3f3091";
        let pushed_since = "6dcb09b5b57875f334f61aebed695e2e4193db5e";
        let args = |reviews: &'static str, head: &'static str| {
            [
                "lockfile-age",
                "override",
                "lockfile-age/override/CODEOWNERS",
                reviews,
                head,
            ]
        };
        let unapproved = |head: &str| {
            Err(Failure::Findings(vec![format!(
                r#"Unapproved {{ head: "{head}" }}"#
            )]))
        };
        assert_eq!(
            run_in(
                FIXTURES,
                &args("lockfile-age/override/approved.json", head),
                0,
                &[]
            ),
            (Ok(()), String::new())
        );
        assert_eq!(
            run_in(
                FIXTURES,
                &args("lockfile-age/override/approved.json", pushed_since),
                0,
                &[]
            ),
            (unapproved(pushed_since), String::new())
        );
        assert_eq!(
            run_in(
                FIXTURES,
                &args("lockfile-age/override/no-reviews.json", head),
                0,
                &[]
            ),
            (unapproved(head), String::new())
        );
    }

    /// Verifies: SEC-SUP-027
    #[test]
    fn the_repository_names_one_login_that_owns_every_lock_file() {
        let codeowners = read(&super::tree::Disk::new(ROOT), ".github/CODEOWNERS")
            .expect("CODEOWNERS is readable");
        let owners: Vec<Vec<&str>> = super::age_override::LOCKFILES
            .iter()
            .map(|path| {
                super::codeowners::owners(&codeowners, path).expect("every pattern is readable")
            })
            .collect();
        let logins: Vec<&str> = owners
            .iter()
            .flatten()
            .copied()
            .filter(|owner| owner.starts_with('@') && !owner.contains('/'))
            .collect();
        assert!(
            logins
                .iter()
                .any(|login| owners.iter().all(|named| named.contains(login))),
            "no login owns every lock file: {owners:?}"
        );
    }

    #[test]
    fn lockfile_age_requests_print_a_curl_configuration() {
        assert_eq!(
            run_in(
                FIXTURES,
                &["lockfile-age", "requests", "lockfile-age/base.lock", "lockfile-age/head.lock"],
                0,
                &[]
            ),
            (
                Ok(()),
                "url = \"https://crates.io/api/v1/crates/libfuzzer-sys/0.4.13\"\noutput = \"libfuzzer-sys@0.4.13.json\"\n"
                    .to_owned()
            )
        );
        assert_eq!(
            run_in(
                FIXTURES,
                &[
                    "lockfile-age",
                    "requests",
                    "lockfile-age/base.lock",
                    "lockfile-age/malformed.lock"
                ],
                0,
                &[]
            ),
            (
                Err(findings(&[
                    r#"Malformed { name: "../escape", version: "1.0.0" }"#
                ])),
                String::new()
            )
        );
    }

    #[test]
    fn a_missing_input_file_is_reported_by_name() {
        let base = "lockfile-age/base.lock";
        let head = "lockfile-age/head.lock";
        for args in [
            &["core-deps", "missing"][..],
            &[
                "lockfile-age",
                "check",
                "missing",
                head,
                "lockfile-age/responses",
            ],
            &[
                "lockfile-age",
                "check",
                base,
                "missing",
                "lockfile-age/responses",
            ],
            &["lockfile-age", "requests", "missing", head],
            &["lockfile-age", "requests", base, "missing"],
            &[
                "lockfile-age",
                "override",
                "missing",
                "lockfile-age/override/approved.json",
                "0",
            ],
            &[
                "lockfile-age",
                "override",
                "lockfile-age/override/CODEOWNERS",
                "missing",
                "0",
            ],
            &["repo", "scorecard", "missing"],
            &["repo", "advisories", "missing"],
            &["repo", "codeql", "missing"],
        ] {
            assert_eq!(
                run_in(FIXTURES, args, 0, &[]),
                (
                    Err(Failure::Unreadable("missing".to_owned())),
                    String::new()
                ),
                "{args:?}"
            );
        }
    }

    #[test]
    fn output_that_cannot_be_written_fails() {
        let args = vec!["fuzz-targets".to_owned()];
        assert_eq!(
            dispatch(&args, FIXTURES, 0, &["ebml"], &mut Closed),
            Err(Failure::Output)
        );
        assert_eq!(write(&mut Closed, "text"), Err(Failure::Output));
        assert!(Closed.flush().is_err(), "the closed output refuses a flush");
    }

    /// Verifies: SEC-SUP-005, SEC-SUP-007, SEC-SUP-010, SEC-SUP-028, SEC-SUP-030, SEC-SUP-035, SEC-SUP-055
    #[test]
    fn the_repository_passes_repo_and_js_deps() {
        assert_eq!(
            run_in(ROOT, &["repo"], 1_790_985_600, &[]),
            (Ok(()), String::new())
        );
        assert_eq!(run_in(ROOT, &["js-deps"], 0, &[]), (Ok(()), String::new()));
    }

    #[test]
    fn repo_scorecard_lists_what_it_finds() {
        assert_eq!(
            run_in(FIXTURES, &["repo", "scorecard", "tree/alpha.txt"], 0, &[]),
            (
                Err(findings(&[r#"Unreadable { path: "scorecard" }"#])),
                String::new()
            )
        );
    }

    /// Verifies: SEC-SUP-018
    #[test]
    fn repo_codeql_fails_on_a_high_severity_result_in_the_sarif_log() {
        assert_eq!(
            run_in(
                FIXTURES,
                &["repo", "codeql", "codeql/blocking.sarif"],
                0,
                &[]
            ),
            (
                Err(findings(&[
                    r#"CodeqlAlert { rule: "rust/sql-injection", path: "src/db.rs" }"#
                ])),
                String::new()
            )
        );
        assert_eq!(
            run_in(FIXTURES, &["repo", "codeql", "codeql/clean.sarif"], 0, &[]),
            (Ok(()), String::new())
        );
    }

    #[test]
    fn repo_advisories_and_settings_run_against_named_inputs() {
        assert_eq!(
            run_in(FIXTURES, &["repo", "advisories", "tree/alpha.txt"], 0, &[]),
            (
                Err(findings(&[r#"Unreadable { path: "advisories" }"#])),
                String::new()
            )
        );
        assert_eq!(
            run_in(FIXTURES, &["repo", "settings", "tree"], 0, &[]),
            (
                Err(findings(&[
                    r#"Unreadable { path: "tree/org.json" }"#,
                    r#"Unreadable { path: "tree/repo.json" }"#,
                    r#"Unreadable { path: "tree/private-vulnerability-reporting.json" }"#,
                    r#"Unreadable { path: "tree/actions-permissions.json" }"#,
                    r#"Unreadable { path: "tree/immutable-releases.json" }"#,
                    r#"Drift { setting: "ruleset.main" }"#,
                    r#"Drift { setting: "ruleset.tag" }"#,
                ])),
                String::new()
            )
        );
    }

    #[test]
    fn helpers_report_reads_findings_and_writes_exactly() {
        let tree = super::tree::Disk::new(FIXTURES);
        assert_eq!(read(&tree, "tree/alpha.txt"), Ok("alpha\n".to_owned()));
        assert_eq!(report::<u8>(vec![]), Ok(()));
        assert_eq!(report(vec![1_u8, 2]), Err(findings(&["1", "2"])));
        assert_eq!(rendered(vec!["a"]), findings(&[r#""a""#]));
        let mut out = Vec::new();
        assert_eq!(write(&mut out, "text"), Ok(()));
        assert_eq!(out, b"text");
    }
}
