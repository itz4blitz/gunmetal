//! `xtask check-harnesses` and `xtask fuzz-targets`.
//!
//! Every entry point of a parser module of `gunmetal-core` must be reached
//! by a registered fuzz harness, and every registered harness must have its
//! module, its exact-outcome replay test and a cargo-fuzz target that builds
//! its own file and calls it (SEC-MED-027, SEC-TM-033, SEC-HIS-036). Each
//! container format needs a structure-aware harness as well (SEC-MED-031).
//! The seeds of every harness are checked by `gunmetal-fuzz`'s own replay
//! test (SEC-MED-028).
//!
//! The rule for what counts as a parse entry point needs no Rust parser. In
//! any module below `formats/` or on [`PARSER_FILES`] or [`PARSER_DIRS`],
//! an entry point is either of these:
//!
//! - a line that starts, after indentation, with `pub fn` or `pub const fn`.
//!   Methods count too, so an entry point cannot hide in an `impl` block. A
//!   harness reaches it by calling it by name.
//! - a line that starts, after indentation, with `impl` and names one of
//!   [`ENTRY_TRAITS`] as a whole word, because parsing behind `FromStr`,
//!   `TryFrom` or `Iterator` needs no public function. A harness reaches
//!   it by calling one of the trait's methods, which the list names. The
//!   word may stand in a bound rather than the implemented trait; such a
//!   module then needs a harness it might not have needed, which is the
//!   safe way to be wrong.
//!
//! A harness calls a function when a line of its source that is not a `//`
//! comment holds the function's name followed by `(` or by a turbofish
//! `::<`, with no identifier character just before the name.

use crate::toml;
use crate::tree::{Tree, is_rust};

/// Where the core's modules live.
const CORE: &str = "crates/gunmetal-core/src";

/// Core modules outside the parser directories that parse untrusted input:
/// the list in work package WP-008; `ebml.rs`, which stays where it is
/// until the Matroska work moves it below `formats/`; and the validators
/// that turn untrusted text and octets into typed values (`base64.rs`,
/// `link.rs`, `net.rs`, `text.rs`, `time.rs` and `values.rs`).
pub const PARSER_FILES: &[&str] = &[
    "base64.rs",
    "deeplink.rs",
    "ebml.rs",
    "inflate.rs",
    "link.rs",
    "logframe.rs",
    "lyrics.rs",
    "m3u.rs",
    "net.rs",
    "path.rs",
    "text.rs",
    "time.rs",
    "values.rs",
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

/// The standard-library traits through which a parser module can take
/// input without a public function, sorted, each with the methods through
/// which a harness reaches an implementation. An iterator is driven by a
/// `for` loop or an adapter, which text cannot tell from other code, so it
/// names none: its module needs a harness, and the call rule falls on
/// whatever builds the iterator.
pub const ENTRY_TRAITS: &[(&str, &[&str])] = &[
    ("FromStr", &["from_str", "parse"]),
    ("Iterator", &[]),
    ("TryFrom", &["try_from", "try_into"]),
];

/// Something `check-harnesses` found wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finding {
    /// A parser module has entry points but no registered harness.
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
    /// A module implements a parse trait and its harness calls none of the
    /// trait's methods.
    Unreached {
        /// The module's path from the repository root.
        module: String,
        /// The module's harness.
        harness: String,
        /// The line of the `impl`, counted from 1.
        line: usize,
        /// The trait.
        implemented: &'static str,
    },
    /// A registered harness's cargo-fuzz target file never calls
    /// `gunmetal_fuzz::<harness>::run(`, so fuzzing it fuzzes something
    /// else or nothing.
    Unwired {
        /// The harness.
        harness: String,
    },
    /// A `[[bin]]` target that does not build `fuzz_targets/<name>.rs`.
    Misplaced {
        /// The target's name.
        target: String,
        /// The file the block names, if any.
        path: Option<String>,
    },
}

/// One way into a parser module.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Entry {
    /// A public function or method, reached by calling it by name.
    Function(String),
    /// An implementation of one of [`ENTRY_TRAITS`].
    Trait {
        /// The line of the `impl`, counted from 1.
        line: usize,
        /// The trait and the calls that reach it.
        implemented: (&'static str, &'static [&'static str]),
    },
}

/// One `[[bin]]` block of the cargo-fuzz manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Target {
    /// Its `name`.
    name: String,
    /// Its `path`, if it has one.
    path: Option<String>,
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
        let entries = entries(&tree.read(&path).unwrap_or_default());
        if entries.is_empty() {
            continue;
        }
        let harness = harness_name(&module);
        if registered.contains(&harness.as_str()) {
            let source = tree
                .read(&format!("crates/gunmetal-fuzz/src/{harness}.rs"))
                .unwrap_or_default();
            for entry in entries.into_iter().filter(|entry| !reaches(&source, entry)) {
                let module = path.clone();
                let harness = harness.clone();
                findings.push(match entry {
                    Entry::Function(function) => Finding::Uncalled {
                        module,
                        harness,
                        function,
                    },
                    Entry::Trait {
                        line,
                        implemented: (implemented, _),
                    } => Finding::Unreached {
                        module,
                        harness,
                        line,
                        implemented,
                    },
                });
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
        if let Some(source) = tree.read(&format!("fuzz/fuzz_targets/{harness}.rs"))
            && !calls(&source, &format!("gunmetal_fuzz::{harness}::run"))
        {
            findings.push(Finding::Unwired {
                harness: harness.to_owned(),
            });
        }
        if !targets.iter().any(|target| target.name == harness) {
            findings.push(Finding::NoTarget {
                harness: harness.to_owned(),
            });
        }
    }
    for Target { name: target, path } in targets {
        if path.as_deref() != Some(format!("fuzz_targets/{target}.rs").as_str()) {
            findings.push(Finding::Misplaced {
                target: target.clone(),
                path,
            });
        }
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

/// Whether `source` calls `function`: on a line that is not a `//` comment,
/// its name followed by `(` or `::<`, with no identifier character just
/// before the name.
fn calls(source: &str, function: &str) -> bool {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .any(|line| {
            line.match_indices(function).any(|(at, _)| {
                let (before, rest) = line.split_at(at);
                let after = &rest[function.len()..];
                !before.ends_with(is_identifier)
                    && (after.starts_with('(') || after.starts_with("::<"))
            })
        })
}

/// Whether `harness`, the source of a harness module, reaches `entry`.
fn reaches(harness: &str, entry: &Entry) -> bool {
    match entry {
        Entry::Function(function) => calls(harness, function),
        Entry::Trait {
            implemented: (_, methods),
            ..
        } => methods.is_empty() || methods.iter().any(|method| calls(harness, method)),
    }
}

/// The entry points defined in `source`.
fn entries(source: &str) -> Vec<Entry> {
    let mut found = Vec::new();
    for (index, line) in source.lines().enumerate() {
        let line = line.trim_start();
        let mut words = line.split(|character| !is_identifier(character));
        if let Some(rest) = line
            .strip_prefix("pub fn ")
            .or_else(|| line.strip_prefix("pub const fn "))
        {
            let name = rest.split(|character| !is_identifier(character)).next();
            found.extend(name.map(|name| Entry::Function(name.to_owned())));
        } else if words.next() == Some("impl") {
            let words: Vec<&str> = words.collect();
            found.extend(
                ENTRY_TRAITS
                    .iter()
                    .filter(|(name, _)| words.contains(name))
                    .map(|&implemented| Entry::Trait {
                        line: index + 1,
                        implemented,
                    }),
            );
        }
    }
    found
}

/// The `[[bin]]` targets in a cargo-fuzz manifest. A block without a name
/// is not a target cargo would build.
fn targets(manifest: &str) -> Vec<Target> {
    let mut blocks = Vec::new();
    let mut block: Option<(Option<String>, Option<String>)> = None;
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            blocks.extend(block.take());
            block = (line == "[[bin]]").then_some((None, None));
        } else if let Some((name, path)) = &mut block {
            if let Some(value) = toml::value(line, "name") {
                *name = Some(value.to_owned());
            }
            if let Some(value) = toml::value(line, "path") {
                *path = Some(value.to_owned());
            }
        }
    }
    blocks.extend(block);
    blocks
        .into_iter()
        .filter_map(|(name, path)| Some(Target { name: name?, path }))
        .collect()
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

    use super::{Entry, Finding, Target, check, entries, targets, targets_json};
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

    /// A cargo-fuzz target file that calls harness `name`.
    fn target(name: &str) -> String {
        format!(
            "#![no_main]\n\nlibfuzzer_sys::fuzz_target!(|data: &[u8]| {{\n    let _ = gunmetal_fuzz::{name}::run(data);\n}});\n"
        )
    }

    /// `tree` with what a registered harness needs: its module holding
    /// `source`, its replay test and its cargo-fuzz target file.
    fn with_harness(tree: Memory, name: &str, source: &str) -> Memory {
        tree.with(&format!("crates/gunmetal-fuzz/src/{name}.rs"), source)
            .with(&format!("crates/gunmetal-fuzz/tests/{name}_corpus.rs"), "")
            .with(&format!("fuzz/fuzz_targets/{name}.rs"), &target(name))
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

    /// The validators that turn untrusted text and octets into the core's
    /// typed values are parser modules too: links, base64, times, networks,
    /// typed values and text.
    ///
    /// Verifies: SEC-MED-027
    #[test]
    fn the_untrusted_input_validators_need_harnesses() {
        let entry = "pub fn parse(input: &[u8]) {}\n";
        let tree = Memory::default()
            .with(&core("base64.rs"), entry)
            .with(&core("link.rs"), entry)
            .with(&core("net.rs"), entry)
            .with(&core("text.rs"), entry)
            .with(&core("time.rs"), entry)
            .with(&core("values.rs"), entry)
            .with("fuzz/Cargo.toml", &manifest(&[]));
        let unregistered = |module: &str| Finding::Unregistered {
            module: core(&format!("{module}.rs")),
            harness: module.to_owned(),
        };
        assert_eq!(
            check(&tree, &[]),
            ["base64", "link", "net", "text", "time", "values"].map(unregistered)
        );
    }

    /// Verifies: SEC-MED-027, SEC-TM-033, SEC-HIS-036
    #[test]
    fn a_parser_module_whose_only_entry_point_is_a_trait_impl_needs_a_harness() {
        let tree = Memory::default()
            .with(
                &core("http/range.rs"),
                "pub struct Range;\n\nimpl core::str::FromStr for Range {\n    type Err = RangeError;\n    fn from_str(text: &str) -> Result<Self, RangeError> {\n    }\n}\n",
            )
            .with(
                &core("wire.rs"),
                "impl<'a> TryFrom<&'a [u8]> for Header<'a> {\n",
            )
            .with(
                &core("m3u.rs"),
                "pub struct Lines<'a>(pub &'a str);\n\nimpl<'a> Iterator for Lines<'a> {\n",
            )
            .with("fuzz/Cargo.toml", &manifest(&[]));
        let unregistered = |module: &str, harness: &str| Finding::Unregistered {
            module: core(module),
            harness: harness.to_owned(),
        };
        assert_eq!(
            check(&tree, &[]),
            [
                unregistered("http/range.rs", "http_range"),
                unregistered("m3u.rs", "m3u"),
                unregistered("wire.rs", "wire"),
            ]
        );
    }

    /// Verifies: SEC-MED-027
    #[test]
    fn a_harness_must_call_a_method_of_each_parse_trait_its_module_implements() {
        let module = "\
impl FromStr for Range {
}

impl TryFrom<&[u8]> for Header {
}

impl Iterator for Lines<'_> {
}
";
        let tree = Memory::default()
            .with(&core("http/range.rs"), module)
            .with(&core("wire.rs"), module)
            .with(&core("path.rs"), module)
            .with(
                "fuzz/Cargo.toml",
                &manifest(&["http_range", "path", "wire"]),
            );
        // `http_range` reaches both conversions the way callers write them,
        // `wire` through the traits' own methods and a turbofish, and `path`
        // only mentions them. An iterator is driven by a loop, so no call is
        // required of it.
        let tree = with_harness(
            tree,
            "http_range",
            "let range: Result<Range, _> = text.parse();\nlet header: Result<Header, _> = data.try_into();",
        );
        let tree = with_harness(
            tree,
            "wire",
            "let _ = text.parse::<Range>();\nlet _ = Header::try_from(data);\nlet _ = Range::from_str(text);",
        );
        let tree = with_harness(
            tree,
            "path",
            "// parse(text), try_from(data)\nreparse(text);",
        );
        let unreached = |line: usize, implemented: &'static str| Finding::Unreached {
            module: core("path.rs"),
            harness: "path".to_owned(),
            line,
            implemented,
        };
        assert_eq!(
            check(&tree, &["http_range", "path", "wire"]),
            [unreached(1, "FromStr"), unreached(4, "TryFrom")]
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
                "pub(crate) fn helper() {}\nfn private() {}\nimpl fmt::Display for Error {\n",
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
        let function = |name: &str| Entry::Function(name.to_owned());
        assert_eq!(
            entries(source),
            [
                function("plain"),
                function("generic"),
                function("constant"),
                function("method"),
                function("nested_const"),
            ]
        );
    }

    #[test]
    fn finds_the_parse_traits_an_impl_line_names() {
        let source = "\
impl FromStr for Range {
impl core::str::FromStr for Range {
  impl<'a> TryFrom<&'a [u8]> for Header<'a> {
impl<'a> Iterator for Elements<'a> {
impl From<TryFromIntError> for Error {
impl fmt::Display for FromStrError {
impl Range {
impls.push(Iterator);
// impl FromStr for Commented {
let iterator = impl_iterator();
";
        let from_str = ("FromStr", &["from_str", "parse"][..]);
        let iterator = ("Iterator", &[][..]);
        let try_from = ("TryFrom", &["try_from", "try_into"][..]);
        let entry = |line: usize, implemented| Entry::Trait { line, implemented };
        assert_eq!(
            entries(source),
            [
                entry(1, from_str),
                entry(2, from_str),
                entry(3, try_from),
                entry(4, iterator),
            ]
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

    /// Verifies: SEC-MED-027, SEC-TM-033
    #[test]
    fn a_fuzz_target_file_that_does_not_call_its_own_harness_fails() {
        // `lyrics` calls another harness, `m3u` is empty, and `wire` names
        // its own call only in a comment. `ebml` is wired correctly.
        let tree = with_harness(Memory::default(), "ebml", "").with(
            "fuzz/Cargo.toml",
            &manifest(&["ebml", "lyrics", "m3u", "wire"]),
        );
        let tree =
            with_harness(tree, "lyrics", "").with("fuzz/fuzz_targets/lyrics.rs", &target("ebml"));
        let tree = with_harness(tree, "m3u", "").with("fuzz/fuzz_targets/m3u.rs", "");
        let tree = with_harness(tree, "wire", "").with(
            "fuzz/fuzz_targets/wire.rs",
            "// gunmetal_fuzz::wire::run(data)\nlibfuzzer_sys::fuzz_target!(|data: &[u8]| {});\n",
        );
        let unwired = |harness: &str| Finding::Unwired {
            harness: harness.to_owned(),
        };
        assert_eq!(
            check(&tree, &["ebml", "lyrics", "m3u", "wire"]),
            [unwired("lyrics"), unwired("m3u"), unwired("wire")]
        );
    }

    /// Verifies: SEC-MED-027, SEC-TM-033
    #[test]
    fn a_bin_target_that_builds_another_file_fails() {
        // The `lyrics` block builds the EBML target's file, and the canary's
        // block names no file, so cargo would look in `src/bin/`.
        let manifest = "\
[[bin]]
name = \"canary_crash\"

[[bin]]
name = \"ebml\"
path = \"fuzz_targets/ebml.rs\"

[[bin]]
name = \"lyrics\"
path = \"fuzz_targets/ebml.rs\"
";
        let tree = with_harness(Memory::default(), "ebml", "").with("fuzz/Cargo.toml", manifest);
        let tree = with_harness(tree, "lyrics", "");
        assert_eq!(
            check(&tree, &["ebml", "lyrics"]),
            [
                Finding::Misplaced {
                    target: "canary_crash".to_owned(),
                    path: None,
                },
                Finding::Misplaced {
                    target: "lyrics".to_owned(),
                    path: Some("fuzz_targets/ebml.rs".to_owned()),
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
    fn reads_the_name_and_path_of_each_bin_target() {
        let manifest = "\
[package]
name = \"gunmetal-fuzz-targets\"
path = \"not-a-target.rs\"

[[bin]]
name = \"ebml\"
path = \"fuzz_targets/ebml.rs\"

[dependencies]
name = \"not-a-target\"
path = \"../not-a-target\"

[[bin]]
path = \"fuzz_targets/lyrics.rs\"
test = false
name = \"lyrics\"

[[bin]]
path = \"fuzz_targets/nameless.rs\"

[[bin]]
name = \"pathless\"
";
        let target = |name: &str, path: Option<&str>| Target {
            name: name.to_owned(),
            path: path.map(str::to_owned),
        };
        assert_eq!(
            targets(manifest),
            [
                target("ebml", Some("fuzz_targets/ebml.rs")),
                target("lyrics", Some("fuzz_targets/lyrics.rs")),
                target("pathless", None),
            ]
        );
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
