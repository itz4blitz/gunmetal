//! The lint tables the facade compiles under, checked against the
//! workspace's.
//!
//! A `forbid` the workspace sets cannot be lowered by an attribute in
//! source, so this crate does not take `[lints] workspace = true`: its
//! manifest repeats every workspace lint, where record 12 (decision 14;
//! owner decision 34) allowed it to lower `unsafe_code` as far as generated
//! code needs. The generated code builds under `forbid`, so it needs no
//! lowering, and these tests allow none: they fail when a workspace lint is
//! missing from the repeated tables, when any lint differs from the
//! workspace's, `unsafe_code` included, and when the manifest sets a lint
//! the workspace does not have. Both files are read at compile time, so the
//! tests do no I/O.
//!
//! `.github/workflows/wasm.yml` is what fails if `unsafe_code` here stops
//! refusing `unsafe` written by hand in the `wasm32` build.

const WORKSPACE_MANIFEST: &str = include_str!("../../../Cargo.toml");
const FACADE_MANIFEST: &str = include_str!("../Cargo.toml");

/// The lines of the table that `header` opens in `manifest`, trimmed,
/// without blank lines and comments.
fn table<'a>(manifest: &'a str, header: &str) -> Vec<&'a str> {
    manifest
        .lines()
        .map(str::trim)
        .skip_while(|line| *line != header)
        .skip(1)
        .take_while(|line| !line.starts_with('['))
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

/// The lint that a table line sets: what stands before its ` = `.
fn lint(line: &str) -> &str {
    line.split(" = ").next().unwrap_or_default()
}

/// What is wrong with the lint tables of a facade `manifest`, compared with
/// the tables of the `workspace` manifest.
fn differences(workspace: &str, manifest: &str) -> Vec<String> {
    let mut found = Vec::new();
    for kind in ["rust", "clippy"] {
        let expected = table(workspace, &format!("[workspace.lints.{kind}]"));
        let actual = table(manifest, &format!("[lints.{kind}]"));
        for line in &expected {
            if !actual.iter().any(|own| lint(own) == lint(line)) {
                found.push(format!("{kind}: `{}` is missing", lint(line)));
            } else if !actual.contains(line) {
                found.push(format!("{kind}: `{}` differs", lint(line)));
            }
        }
        for own in &actual {
            if !expected.iter().any(|line| lint(line) == lint(own)) {
                found.push(format!("{kind}: `{}` is not a workspace lint", lint(own)));
            }
        }
    }
    found
}

/// A workspace manifest with two lints in each table.
const WORKSPACE: &str = "[workspace]\nmembers = []\n\n[workspace.lints.rust]\n\
    unsafe_code = \"forbid\"\nmissing_docs = \"warn\"\n\n[workspace.lints.clippy]\n\
    # Comments do not count.\npedantic = { level = \"warn\", priority = -1 }\n\
    await_holding_lock = \"deny\"\n\n[profile.release]\noverflow-checks = true\n";

/// A facade manifest whose tables hold the lines `rust` and `clippy`.
fn manifest(rust: &str, clippy: &str) -> String {
    format!(
        "[package]\nname = \"facade\"\n\n[lints.rust]\n{rust}\n# A comment.\n[lints.clippy]\n{clippy}\n\
         [features]\nextra = []\n"
    )
}

/// The clippy lines of [`WORKSPACE`], as a facade repeats them.
const CLIPPY: &str =
    "pedantic = { level = \"warn\", priority = -1 }\nawait_holding_lock = \"deny\"\n";

/// The rust lines of [`WORKSPACE`], as a facade repeats them.
const RUST: &str = "unsafe_code = \"forbid\"\nmissing_docs = \"warn\"\n";

#[test]
fn tables_that_repeat_the_workspace_s_exactly_pass() {
    assert_eq!(
        differences(WORKSPACE, &manifest(RUST, CLIPPY)),
        Vec::<String>::new()
    );
}

/// `unsafe_code` is held to the workspace's `forbid` like every other lint:
/// the code `wasm-bindgen` and `tsify` generate builds under it, so nothing
/// needs it lower.
#[test]
fn a_manifest_that_lowers_unsafe_code_fails() {
    for level in ["deny", "warn", "allow"] {
        let rust = format!("unsafe_code = \"{level}\"\nmissing_docs = \"warn\"\n");
        assert_eq!(
            differences(WORKSPACE, &manifest(&rust, CLIPPY)),
            ["rust: `unsafe_code` differs"],
            "{level}"
        );
    }
}

#[test]
fn a_manifest_missing_one_workspace_lint_fails() {
    assert_eq!(
        differences(
            WORKSPACE,
            &manifest(
                "unsafe_code = \"forbid\"\n",
                "pedantic = { level = \"warn\", priority = -1 }\n"
            )
        ),
        [
            "rust: `missing_docs` is missing",
            "clippy: `await_holding_lock` is missing"
        ]
    );
    assert_eq!(
        differences(WORKSPACE, &manifest("missing_docs = \"warn\"\n", CLIPPY)),
        ["rust: `unsafe_code` is missing"]
    );
}

#[test]
fn a_manifest_that_changes_any_other_lint_fails() {
    assert_eq!(
        differences(
            WORKSPACE,
            &manifest(
                "unsafe_code = \"forbid\"\nmissing_docs = \"allow\"\n",
                "pedantic = { level = \"warn\", priority = -1 }\nawait_holding_lock = \"warn\"\n"
            )
        ),
        [
            "rust: `missing_docs` differs",
            "clippy: `await_holding_lock` differs"
        ]
    );
}

#[test]
fn a_manifest_that_sets_a_lint_the_workspace_does_not_have_fails() {
    assert_eq!(
        differences(
            WORKSPACE,
            &manifest(
                "unsafe_code = \"forbid\"\nmissing_docs = \"warn\"\nunused = \"allow\"\n",
                CLIPPY
            )
        ),
        ["rust: `unused` is not a workspace lint"]
    );
}

#[test]
fn a_manifest_that_takes_the_workspace_table_or_none_fails() {
    let every_lint_missing = [
        "rust: `unsafe_code` is missing",
        "rust: `missing_docs` is missing",
        "clippy: `pedantic` is missing",
        "clippy: `await_holding_lock` is missing",
    ];
    assert_eq!(
        differences(
            WORKSPACE,
            "[package]\nname = \"facade\"\n\n[lints]\nworkspace = true\n"
        ),
        every_lint_missing
    );
    assert_eq!(
        differences(WORKSPACE, "[package]\nname = \"facade\"\n"),
        every_lint_missing
    );
}

/// The facade's own manifest, against the workspace's: every lint the
/// same, and `unsafe_code` at `forbid`.
#[test]
fn the_facade_repeats_every_workspace_lint_and_changes_none() {
    assert_eq!(
        differences(WORKSPACE_MANIFEST, FACADE_MANIFEST),
        Vec::<String>::new()
    );
    assert!(table(FACADE_MANIFEST, "[lints.rust]").contains(&"unsafe_code = \"forbid\""));
}
