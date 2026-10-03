//! `xtask check-harnesses` and `xtask fuzz-targets`.
//!
//! Every public function in a parser module of `gunmetal-core` must be
//! called by a registered fuzz harness, and every registered harness must
//! have its module, its exact-outcome replay test and its cargo-fuzz target
//! (SEC-MED-027, SEC-TM-033, SEC-HIS-036). Each container format needs a
//! structure-aware harness as well (SEC-MED-031). The seeds of every
//! harness are checked by `gunmetal-fuzz`'s own replay test (SEC-MED-028).
//!
//! The rule for what counts as a parse entry point needs no Rust parser: a
//! line that starts, after indentation, with `pub fn` or `pub const fn`, in
//! any module below `formats/` or on [`PARSER_FILES`] or [`PARSER_DIRS`].
//! Methods count too, so an entry point cannot hide in an `impl` block. A
//! harness calls an entry point when its source holds the function's name
//! followed by `(`, with no identifier character just before it.

use crate::toml;
use crate::tree::{Tree, is_rust};

/// Where the core's modules live.
const CORE: &str = "crates/gunmetal-core/src";

/// Core modules outside the parser directories that parse untrusted input:
/// the list in work package WP-008, plus `ebml.rs`, which stays where it is
/// until the Matroska work moves it below `formats/`.
pub const PARSER_FILES: &[&str] = &[
    "deeplink.rs",
    "ebml.rs",
    "inflate.rs",
    "logframe.rs",
    "lyrics.rs",
    "m3u.rs",
    "path.rs",
    "wire.rs",
];

/// Core directories in which every module parses untrusted input.
pub const PARSER_DIRS: &[&str] = &["formats/", "http/", "import/", "provider/", "webauthn/"];

/// Container formats that need a structure-aware harness named
/// `<format>_structure` (SEC-MED-031), each with the prefix its parser
/// modules' harness names start with. Matroska joins the list when it
/// arrives.
pub const CONTAINERS: &[&str] = &["flac", "id3", "mp4", "ogg"];

/// The prefix of cargo-fuzz targets that carry a planted bug, which the
/// fuzz workflow runs to prove it fails on a finding (SEC-MED-029). They are
/// not harnesses.
pub const CANARY: &str = "canary_";

/// Something `check-harnesses` found wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finding {
    /// A parser module has public functions but no registered harness.
    Unregistered {
        /// The module's path from the repository root.
        module: String,
        /// The harness name the module needs.
        harness: String,
    },
    /// A module's harness never calls one of its public functions.
    Uncalled {
        /// The module's path from the repository root.
        module: String,
        /// The module's harness.
        harness: String,
        /// The function the harness does not call.
        function: String,
    },
    /// A container format has parser modules but no structure-aware harness.
    Unstructured {
        /// The container format.
        format: &'static str,
        /// The harness name it needs.
        harness: String,
    },
    /// A registered harness lacks one of its files.
    Missing {
        /// The harness.
        harness: String,
        /// The missing file's path from the repository root.
        path: String,
    },
    /// A registered harness has no `[[bin]]` target in `fuzz/Cargo.toml`.
    NoTarget {
        /// The harness.
        harness: String,
    },
    /// A cargo-fuzz target is neither a registered harness nor a canary, so
    /// stable `cargo test` never replays its inputs.
    Unknown {
        /// The target's name.
        target: String,
    },
}

/// Checks the tree against the `registered` harness names.
pub fn check(tree: &dyn Tree, registered: &[&str]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut needed = Vec::new();
    for module in tree
        .files(CORE)
        .into_iter()
        .filter(|module| is_parser(module))
    {
        let path = format!("{CORE}/{module}");
        let functions = public_functions(&tree.read(&path).unwrap_or_default());
        if functions.is_empty() {
            continue;
        }
        let harness = harness_name(&module);
        if registered.contains(&harness.as_str()) {
            let source = tree
                .read(&format!("crates/gunmetal-fuzz/src/{harness}.rs"))
                .unwrap_or_default();
            for function in functions {
                if !calls(&source, &function) {
                    findings.push(Finding::Uncalled {
                        module: path.clone(),
                        harness: harness.clone(),
                        function,
                    });
                }
            }
        } else {
            findings.push(Finding::Unregistered {
                module: path,
                harness: harness.clone(),
            });
        }
        needed.push(harness);
    }
    for &format in CONTAINERS {
        let harness = format!("{format}_structure");
        if needed.iter().any(|name| name.starts_with(format))
            && !registered.contains(&harness.as_str())
        {
            findings.push(Finding::Unstructured { format, harness });
        }
    }
    let targets = targets(&tree.read("fuzz/Cargo.toml").unwrap_or_default());
    for &harness in registered {
        for path in [
            format!("crates/gunmetal-fuzz/src/{harness}.rs"),
            format!("crates/gunmetal-fuzz/tests/{harness}_corpus.rs"),
            format!("fuzz/fuzz_targets/{harness}.rs"),
        ] {
            if tree.read(&path).is_none() {
                findings.push(Finding::Missing {
                    harness: harness.to_owned(),
                    path,
                });
            }
        }
        if !targets.iter().any(|target| target == harness) {
            findings.push(Finding::NoTarget {
                harness: harness.to_owned(),
            });
        }
    }
    for target in targets {
        if !target.starts_with(CANARY) && !registered.contains(&target.as_str()) {
            findings.push(Finding::Unknown { target });
        }
    }
    findings
}

/// Whether `module`, a path below the core's `src/`, parses untrusted input.
fn is_parser(module: &str) -> bool {
    is_rust(module)
        && (PARSER_FILES.contains(&module) || PARSER_DIRS.iter().any(|dir| module.starts_with(dir)))
}

/// The name of the harness for `module`, a path below the core's `src/`.
fn harness_name(module: &str) -> String {
    let module = module.strip_prefix("formats/").unwrap_or(module);
    module
        .strip_suffix(".rs")
        .unwrap_or(module)
        .replace('/', "_")
}

/// Whether `character` can be part of a Rust identifier.
fn is_identifier(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

/// Whether `source` calls `function`: its name and `(`, with no identifier
/// character just before the name.
fn calls(source: &str, function: &str) -> bool {
    let call = format!("{function}(");
    source
        .match_indices(&call)
        .any(|(at, _)| !source[..at].ends_with(is_identifier))
}

/// The names of the public functions and methods defined in `source`.
fn public_functions(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let rest = line
                .strip_prefix("pub fn ")
                .or_else(|| line.strip_prefix("pub const fn "))?;
            rest.split(|character| !is_identifier(character))
                .next()
                .map(str::to_owned)
        })
        .collect()
}

/// The names of the `[[bin]]` targets in a cargo-fuzz manifest.
fn targets(manifest: &str) -> Vec<String> {
    let mut in_bin = false;
    let mut names = Vec::new();
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_bin = line == "[[bin]]";
        } else if in_bin && let Some(name) = toml::value(line, "name") {
            names.push(name.to_owned());
        }
    }
    names
}

/// The registered harness names as a JSON array on one line, for the fuzz
/// workflow's job matrix, or the names that are not made only of lowercase
/// ASCII letters, digits and underscores. The workflow uses each name in
/// paths, so a name from a pull request must not carry anything else.
pub fn targets_json(registered: &[&str]) -> Result<String, Vec<String>> {
    let invalid: Vec<String> = registered
        .iter()
        .filter(|name| !is_valid_name(name))
        .map(|&name| name.to_owned())
        .collect();
    if invalid.is_empty() {
        let quoted: Vec<String> = registered
            .iter()
            .map(|name| format!("\"{name}\""))
            .collect();
        Ok(format!("[{}]\n", quoted.join(",")))
    } else {
        Err(invalid)
    }
}

/// Whether `name` is made only of lowercase ASCII letters, digits and
/// underscores, and is not empty.
fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::{Finding, check, public_functions, targets, targets_json};
    use crate::tree::memory::Memory;

    /// The path of the core module `module`, given below `src/`.
    fn core(module: &str) -> String {
        format!("crates/gunmetal-core/src/{module}")
    }

    /// A cargo-fuzz manifest with one `[[bin]]` block per target.
    fn manifest(targets: &[&str]) -> String {
        let mut text = String::from(
            "[package]\nname = \"gunmetal-fuzz-targets\"\n\n[dependencies]\nlibfuzzer-sys = \"0.4.13\"\n",
        );
        for target in targets {
            let _ = write!(
                text,
                "\n[[bin]]\nname = \"{target}\"\npath = \"fuzz_targets/{target}.rs\"\ntest = false\n"
            );
        }
        text
    }

    /// `tree` with what a registered harness needs: its module holding
    /// `source`, its replay test and its cargo-fuzz target file.
    fn with_harness(tree: Memory, name: &str, source: &str) -> Memory {
        tree.with(&format!("crates/gunmetal-fuzz/src/{name}.rs"), source)
            .with(&format!("crates/gunmetal-fuzz/tests/{name}_corpus.rs"), "")
            .with(&format!("fuzz/fuzz_targets/{name}.rs"), "")
    }

    /// The shape of the real EBML module's entry points.
    const EBML: &str = "\
/// Decodes the variable-size integer at the start of `input`.
pub fn decode_vint(input: &[u8]) -> Result<Vint, VintError> {
}

impl ElementError {
    /// The error's offset.
    pub const fn offset(&self) -> usize {
    }
}

pub fn elements(input: &[u8]) -> Elements<'_> {
}
";

    /// A harness that calls every EBML entry point.
    const EBML_HARNESS: &str = "\
    let vint = ebml::decode_vint(data);
    for (index, item) in ebml::elements(buffer).enumerate() {
        assert!(error.offset() < octets);
";

    /// Verifies: SEC-MED-027, SEC-TM-033, SEC-HIS-036
    #[test]
    fn a_parser_module_without_a_registered_harness_fails() {
        let tree = Memory::default()
            .with(&core("ebml.rs"), EBML)
            .with("fuzz/Cargo.toml", &manifest(&[]));
        assert_eq!(
            check(&tree, &[]),
            [Finding::Unregistered {
                module: "crates/gunmetal-core/src/ebml.rs".to_owned(),
                harness: "ebml".to_owned(),
            }]
        );
    }

    /// Verifies: SEC-MED-027, SEC-MED-031, SEC-TM-033, SEC-HIS-036
    #[test]
    fn a_tree_with_every_harness_registered_passes() {
        let tree = Memory::default()
            .with(&core("ebml.rs"), EBML)
            .with(&core("formats/mod.rs"), "pub mod flac;\n")
            .with(
                &core("formats/flac/metadata.rs"),
                "pub fn blocks(input: &[u8]) -> Blocks<'_> {\n",
            )
            .with(
                "fuzz/Cargo.toml",
                &manifest(&["canary_crash", "ebml", "flac_metadata", "flac_structure"]),
            );
        let tree = with_harness(tree, "ebml", EBML_HARNESS);
        let tree = with_harness(tree, "flac_metadata", "metadata::blocks(data)");
        let tree = with_harness(tree, "flac_structure", "metadata::blocks(&encode(&tree))");
        assert_eq!(
            check(&tree, &["ebml", "flac_metadata", "flac_structure"]),
            []
        );
    }

    /// Verifies: SEC-MED-027
    #[test]
    fn a_harness_that_skips_an_entry_point_fails() {
        // Calls of `reparse`, `try_parse` and a mention without a call do
        // not count as calls of `parse`.
        let harness = "reparse(data); try_parse(data); // parse\nlet _ = lyrics::lines(data);";
        let tree = Memory::default()
            .with(
                &core("lyrics.rs"),
                "pub fn parse(input: &str) {}\npub fn lines(input: &str) {}\n",
            )
            .with("fuzz/Cargo.toml", &manifest(&["lyrics"]));
        let tree = with_harness(tree, "lyrics", harness);
        assert_eq!(
            check(&tree, &["lyrics"]),
            [Finding::Uncalled {
                module: "crates/gunmetal-core/src/lyrics.rs".to_owned(),
                harness: "lyrics".to_owned(),
                function: "parse".to_owned(),
            }]
        );
    }

    /// Verifies: SEC-MED-027
    #[test]
    fn modules_in_parser_directories_and_on_the_list_need_harnesses() {
        let entry = "pub fn parse(input: &[u8]) {}\n";
        let tree = Memory::default()
            .with(&core("formats/riff.rs"), entry)
            .with(&core("http/range.rs"), entry)
            .with(&core("import/listenbrainz.rs"), entry)
            .with(&core("provider/musicbrainz.rs"), entry)
            .with(&core("webauthn/client_data.rs"), entry)
            .with(&core("m3u.rs"), entry)
            .with("fuzz/Cargo.toml", &manifest(&[]));
        let unregistered = |module: &str, harness: &str| Finding::Unregistered {
            module: core(module),
            harness: harness.to_owned(),
        };
        assert_eq!(
            check(&tree, &[]),
            [
                unregistered("formats/riff.rs", "riff"),
                unregistered("http/range.rs", "http_range"),
                unregistered("import/listenbrainz.rs", "import_listenbrainz"),
                unregistered("m3u.rs", "m3u"),
                unregistered("provider/musicbrainz.rs", "provider_musicbrainz"),
                unregistered("webauthn/client_data.rs", "webauthn_client_data"),
            ]
        );
    }

    #[test]
    fn other_modules_and_files_need_no_harness() {
        let entry = "pub fn parse(input: &[u8]) {}\n";
        let tree = Memory::default()
            .with(&core("queue.rs"), entry)
            .with(&core("lib.rs"), "pub mod ebml;\n")
            .with(&core("formats/notes.md"), entry)
            .with(&core("formats/mod.rs"), "pub mod flac;\n")
            .with(
                &core("ebml.rs"),
                "pub(crate) fn helper() {}\nfn private() {}\n",
            )
            .with("fuzz/Cargo.toml", &manifest(&[]));
        assert_eq!(check(&tree, &[]), []);
    }

    #[test]
    fn finds_public_functions_and_methods_but_not_private_ones() {
        let source = "\
pub fn plain(input: &[u8]) {}
pub fn generic<'a, T>(input: &'a [T]) {}
pub const fn constant() -> u8 { 0 }
    pub fn method(&self) {}
        pub const fn nested_const(&self) {}
pub(crate) fn crate_only() {}
fn private() {}
pub struct NotAFunction;
// pub fn commented_out() {}
let call = pub_fn_lookalike();
";
        assert_eq!(
            public_functions(source),
            ["plain", "generic", "constant", "method", "nested_const"]
        );
    }

    /// Verifies: SEC-MED-027, SEC-MED-028
    #[test]
    fn a_registered_harness_needs_its_files_and_its_target() {
        let tree = Memory::default().with("fuzz/Cargo.toml", &manifest(&[]));
        let missing = |path: &str| Finding::Missing {
            harness: "ebml".to_owned(),
            path: path.to_owned(),
        };
        assert_eq!(
            check(&tree, &["ebml"]),
            [
                missing("crates/gunmetal-fuzz/src/ebml.rs"),
                missing("crates/gunmetal-fuzz/tests/ebml_corpus.rs"),
                missing("fuzz/fuzz_targets/ebml.rs"),
                Finding::NoTarget {
                    harness: "ebml".to_owned()
                },
            ]
        );
    }

    /// Verifies: SEC-MED-028
    #[test]
    fn a_fuzz_target_that_is_neither_a_harness_nor_a_canary_fails() {
        let tree = with_harness(Memory::default(), "ebml", "").with(
            "fuzz/Cargo.toml",
            &manifest(&["canary_timeout", "ebml", "stray"]),
        );
        assert_eq!(
            check(&tree, &["ebml"]),
            [Finding::Unknown {
                target: "stray".to_owned()
            }]
        );
    }

    /// Verifies: SEC-MED-031
    #[test]
    fn a_container_format_needs_a_structure_aware_harness() {
        let entry = "pub fn parse(input: &[u8]) {}\n";
        let tree = Memory::default()
            .with(&core("formats/id3v2/frames.rs"), entry)
            .with(&core("formats/opus.rs"), entry)
            .with(
                "fuzz/Cargo.toml",
                &manifest(&["id3v2_frames", "mp4_structure", "opus"]),
            );
        let tree = with_harness(tree, "id3v2_frames", "frames::parse(data)");
        let tree = with_harness(tree, "mp4_structure", "");
        let tree = with_harness(tree, "opus", "opus::parse(data)");
        // Opus is not a container on the list, and a registered structure
        // harness for a format with no modules yet is allowed.
        assert_eq!(
            check(&tree, &["id3v2_frames", "mp4_structure", "opus"]),
            [Finding::Unstructured {
                format: "id3",
                harness: "id3_structure".to_owned(),
            }]
        );
    }

    #[test]
    fn reads_only_the_names_of_bin_targets() {
        let manifest = "\
[package]
name = \"gunmetal-fuzz-targets\"

[[bin]]
name = \"ebml\"
path = \"fuzz_targets/ebml.rs\"

[dependencies]
name = \"not-a-target\"

[[bin]]
test = false
name = \"lyrics\"
";
        assert_eq!(targets(manifest), ["ebml", "lyrics"]);
    }

    #[test]
    fn lists_the_targets_as_a_json_array() {
        assert_eq!(
            targets_json(&["ebml", "flac_metadata", "mp4_structure2"]),
            Ok("[\"ebml\",\"flac_metadata\",\"mp4_structure2\"]\n".to_owned())
        );
        assert_eq!(targets_json(&[]), Ok("[]\n".to_owned()));
    }

    #[test]
    fn refuses_target_names_that_could_escape_a_path_or_the_json() {
        assert_eq!(
            targets_json(&["ebml", "../up", "Upper", "", "quote\"", "ok_2"]),
            Err(vec![
                "../up".to_owned(),
                "Upper".to_owned(),
                String::new(),
                "quote\"".to_owned(),
            ])
        );
    }
}
