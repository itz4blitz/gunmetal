//! `xtask native-code`: every crate in the shipped graph that brings native
//! code or `unsafe` with it is on a justified allow-list (SEC-TM-034).
//!
//! `deny.toml` bans the C media libraries by name, and a list of names
//! cannot catch a binding it does not name. This check looks at what is
//! there instead. A crate needs a place on the list when any of these holds:
//!
//! - its name ends in `-sys` or `_sys`, the convention for bindings to a
//!   native library;
//! - its manifest declares `links`, which is how a crate says it links a
//!   native library;
//! - any Rust source file it ships uses the `unsafe` keyword.
//!
//! The shipped graph is every workspace member that is not build tooling
//! ([`TOOLING`]) and every crate one of them reaches through normal
//! dependencies, for the targets Gunmetal builds. A new workspace member
//! ships unless it is added to [`TOOLING`], so the check fails closed. Build
//! and dev dependencies are not linked into what ships and are left to the
//! build-script allow-list (SEC-SUP-026). Nor is a procedural macro, or a
//! crate that only one reaches: the compiler runs the macro while it builds,
//! and what ships is the code the macro wrote, in the crate that used it.
//! The gate runs:
//!
//! ```text
//! cargo metadata "$locked" --format-version 1 --all-features \
//!   --filter-platform x86_64-unknown-linux-gnu \
//!   --filter-platform aarch64-unknown-linux-gnu \
//!   --filter-platform i686-unknown-linux-gnu \
//!   --filter-platform wasm32-unknown-unknown >target/native-code.json
//! cargo run "$locked" -q -p xtask -- native-code target/native-code.json
//! ```
//!
//! The targets are the ones `deny.toml` names. `--all-features` matters for
//! the reason it does in `core-deps`: one member can turn on an optional
//! dependency of another.
//!
//! The source search reads every `.rs` file beneath a crate's manifest,
//! tests, examples and build script included, so it errs towards listing a
//! crate. It skips comments, strings and character literals, so a crate
//! that only writes about `unsafe`, or forbids `unsafe_code`, is not
//! listed. A crate whose sources cannot be read fails the check: nothing is
//! assumed safe unread.
//!
//! The list stays exactly as long as the graph needs: an entry for a crate
//! that no longer ships, or no longer has native or unsafe code, fails too.

use std::collections::{BTreeMap, BTreeSet};

use crate::json::{self, Value};
use crate::toml;
use crate::tree::{Tree, is_rust};

/// The reviewed allow-list, under code-owner review with the rest of
/// `supply-chain/`. Each entry is a `[[crate]]` line followed by
/// `name = "<crate>"`, `reason = "<why it is needed and what its native or
/// unsafe code does>"` and `requirement = "<SEC-AREA-NNN>"`, the requirement
/// that the crate is there to meet; several are separated by `, `. A missing
/// file is an empty list, the strictest one.
pub const ALLOWLIST: &str = "supply-chain/native-allowlist.toml";

/// The workspace members that are build tooling and never ship. Each still
/// counts when a shipped crate depends on it.
pub const TOOLING: [&str; 3] = ["gunmetal-fuzz", "gunmetal-testkit", "xtask"];

/// Something `native-code` found wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finding {
    /// The input is not the output of the `cargo metadata` command above.
    NotMetadata,
    /// The allow-list's entry number `entry`, counting from one, has no
    /// readable `name`.
    Unnamed {
        /// The entry's position in the file.
        entry: usize,
    },
    /// An allow-list entry without a justification.
    Unjustified {
        /// The crate the entry names.
        name: String,
        /// What is absent or unreadable: `reason` or `requirement`.
        missing: &'static str,
    },
    /// A shipped crate with no Rust source the check can read, so nothing
    /// is known about it.
    Unreadable {
        /// The crate.
        name: String,
        /// Its version.
        version: String,
    },
    /// A shipped crate with native or unsafe code that the allow-list does
    /// not name.
    Unlisted {
        /// The crate.
        name: String,
        /// Its version.
        version: String,
        /// Whether its name marks it as bindings to a native library.
        sys: bool,
        /// The native library its manifest says it links.
        links: Option<String>,
        /// The first of its source files that uses `unsafe`, relative to
        /// its manifest.
        unsafe_in: Option<String>,
    },
    /// A crate the allow-list names that is not in the shipped graph, or
    /// has no native or unsafe code.
    Unused {
        /// The crate.
        name: String,
    },
}

/// Compares the shipped graph in `metadata`, the `cargo metadata` output
/// above, with `allowlist`, reading each crate's sources through `tree`.
pub fn check(tree: &dyn Tree, metadata: &str, allowlist: &str) -> Vec<Finding> {
    let Some(document) = json::parse(metadata) else {
        return vec![Finding::NotMetadata];
    };
    let Some(packages) = shipped(&document) else {
        return vec![Finding::NotMetadata];
    };
    let mut findings = unjustified(allowlist);
    let allowed: BTreeSet<&str> = entries(allowlist)
        .iter()
        .filter_map(|entry| entry.name)
        .collect();
    let mut needed = BTreeSet::new();
    for package in packages {
        let name = package.name.to_owned();
        let version = package.version.to_owned();
        let unsafe_in = match first_unsafe(tree, package.dir) {
            Scan::Unreadable => {
                needed.insert(package.name);
                findings.push(Finding::Unreadable { name, version });
                continue;
            }
            Scan::Safe => None,
            Scan::Unsafe(path) => Some(path),
        };
        let sys = package.name.ends_with("-sys") || package.name.ends_with("_sys");
        if sys || package.links.is_some() || unsafe_in.is_some() {
            needed.insert(package.name);
            if !allowed.contains(package.name) {
                findings.push(Finding::Unlisted {
                    name,
                    version,
                    sys,
                    links: package.links.map(str::to_owned),
                    unsafe_in,
                });
            }
        }
    }
    findings.extend(allowed.difference(&needed).map(|&name| Finding::Unused {
        name: name.to_owned(),
    }));
    findings
}

/// What is wrong with the allow-list `allowlist` on its own: entries
/// without a name, a reason or a requirement.
pub fn unjustified(allowlist: &str) -> Vec<Finding> {
    (1..)
        .zip(entries(allowlist))
        .filter_map(|(position, entry)| {
            let Some(name) = entry.name else {
                return Some(Finding::Unnamed { entry: position });
            };
            let missing = if entry.reason.is_none_or(|reason| reason.trim().is_empty()) {
                "reason"
            } else if entry.requirement.is_some_and(cites_requirements) {
                return None;
            } else {
                "requirement"
            };
            Some(Finding::Unjustified {
                name: name.to_owned(),
                missing,
            })
        })
        .collect()
}

/// One `[[crate]]` entry of the allow-list, as far as it can be read.
#[derive(Default)]
struct Entry<'a> {
    /// The crate.
    name: Option<&'a str>,
    /// Why it is allowed.
    reason: Option<&'a str>,
    /// The requirements it is there to meet.
    requirement: Option<&'a str>,
}

/// The entries of `allowlist`, in file order. Lines before the first entry
/// are ignored, and so is any line that is not one of the three keys.
fn entries(allowlist: &str) -> Vec<Entry<'_>> {
    let mut entries: Vec<Entry<'_>> = Vec::new();
    for line in allowlist.lines() {
        if line.trim() == "[[crate]]" {
            entries.push(Entry::default());
        } else if let Some(entry) = entries.last_mut() {
            entry.name = toml::value(line, "name").or(entry.name);
            entry.reason = toml::value(line, "reason").or(entry.reason);
            entry.requirement = toml::value(line, "requirement").or(entry.requirement);
        }
    }
    entries
}

/// Whether `text` is one or more requirement IDs separated by `, `.
fn cites_requirements(text: &str) -> bool {
    text.split(", ").all(is_requirement)
}

/// Whether `id` has the shape of a security requirement ID: `SEC-`, an
/// area in capital letters, `-` and three digits.
fn is_requirement(id: &str) -> bool {
    let Some((area, number)) = id
        .strip_prefix("SEC-")
        .and_then(|rest| rest.split_once('-'))
    else {
        return false;
    };
    !area.is_empty()
        && area.bytes().all(|byte| byte.is_ascii_uppercase())
        && number.len() == 3
        && number.bytes().all(|byte| byte.is_ascii_digit())
}

/// One package of the shipped graph.
struct Package<'a> {
    /// Its name.
    name: &'a str,
    /// Its version.
    version: &'a str,
    /// The native library its manifest says it links.
    links: Option<&'a str>,
    /// The directory that holds its manifest.
    dir: &'a str,
}

/// The packages of the shipped graph in `metadata`, sorted by name and
/// version, or `None` when `metadata` is not `cargo metadata` output.
fn shipped(metadata: &Value) -> Option<Vec<Package<'_>>> {
    let packages = by_id(metadata.get("packages")?)?;
    let nodes = by_id(metadata.get("resolve")?.get("nodes")?)?;
    let mut queue = Vec::new();
    for member in metadata.get("workspace_members")?.as_array()? {
        let id = member.as_str()?;
        if !TOOLING.contains(&text(packages.get(id)?, "name")?) {
            queue.push(id);
        }
    }
    let mut seen = BTreeSet::new();
    let mut found = Vec::new();
    while let Some(id) = queue.pop() {
        if seen.insert(id) {
            let described = packages.get(id)?;
            if !is_macro(described)? {
                found.push(package(described)?);
                for dep in nodes.get(id)?.get("deps")?.as_array()? {
                    if is_normal(dep)? {
                        queue.push(text(dep, "pkg")?);
                    }
                }
            }
        }
    }
    found.sort_by_key(|package| (package.name, package.version));
    Some(found)
}

/// The objects of the array `list`, by their `id`.
fn by_id(list: &Value) -> Option<BTreeMap<&str, &Value>> {
    list.as_array()?
        .iter()
        .map(|item| Some((text(item, "id")?, item)))
        .collect()
}

/// The string member `key` of `object`.
fn text<'a>(object: &'a Value, key: &str) -> Option<&'a str> {
    object.get(key)?.as_str()
}

/// Whether the package `described` is a procedural macro: one of its
/// targets has the kind `proc-macro`.
fn is_macro(described: &Value) -> Option<bool> {
    let kinds: Option<Vec<&[Value]>> = described
        .get("targets")?
        .as_array()?
        .iter()
        .map(|target| target.get("kind")?.as_array())
        .collect();
    let marker = Value::String("proc-macro".to_owned());
    Some(kinds?.iter().any(|kinds| kinds.contains(&marker)))
}

/// What the check reads of one entry of `packages`.
fn package(described: &Value) -> Option<Package<'_>> {
    let links = match described.get("links")? {
        Value::Null => None,
        named => Some(named.as_str()?),
    };
    Some(Package {
        name: text(described, "name")?,
        version: text(described, "version")?,
        links,
        dir: text(described, "manifest_path")?.strip_suffix("/Cargo.toml")?,
    })
}

/// Whether the dependency `dep` of a resolve node is a normal one on any
/// target, rather than only a build or dev dependency.
fn is_normal(dep: &Value) -> Option<bool> {
    let kinds: Option<Vec<&Value>> = dep
        .get("dep_kinds")?
        .as_array()?
        .iter()
        .map(|kind| kind.get("kind"))
        .collect();
    Some(kinds?.contains(&&Value::Null))
}

/// What reading a crate's sources found.
enum Scan {
    /// There is no Rust source, or one file is not text.
    Unreadable,
    /// No file uses `unsafe`.
    Safe,
    /// The first file that uses `unsafe`, relative to the crate.
    Unsafe(String),
}

/// Reads every Rust source file beneath `dir` until one uses `unsafe`.
fn first_unsafe(tree: &dyn Tree, dir: &str) -> Scan {
    let mut scan = Scan::Unreadable;
    for path in tree.files(dir) {
        if is_rust(&path) {
            let Some(source) = tree.read(&format!("{dir}/{path}")) else {
                return Scan::Unreadable;
            };
            if uses_unsafe(&source) {
                return Scan::Unsafe(path);
            }
            scan = Scan::Safe;
        }
    }
    scan
}

/// Whether the Rust source `source` uses the `unsafe` keyword outside
/// comments, strings and character literals.
pub fn uses_unsafe(source: &str) -> bool {
    let mut rest = source;
    while let Some(first) = rest.chars().next() {
        if let Some(comment) = rest.strip_prefix("//") {
            rest = comment.split_once('\n').map_or("", |(_, after)| after);
        } else if let Some(comment) = rest.strip_prefix("/*") {
            rest = after_block_comment(comment);
        } else if let Some(string) = rest.strip_prefix('"') {
            rest = after_string(string);
        } else if let Some(quoted) = rest.strip_prefix('\'') {
            rest = after_quote(quoted);
        } else if is_word(first) {
            let after = rest.trim_start_matches(is_word);
            let word = rest.strip_suffix(after).unwrap_or_default();
            if word == "unsafe" {
                return true;
            }
            rest = if matches!(word, "r" | "br" | "cr") {
                after_raw_string(after)
            } else {
                after
            };
        } else {
            rest = rest.trim_start_matches(first);
        }
    }
    false
}

/// Whether `character` can be part of a name or keyword.
fn is_word(character: char) -> bool {
    character == '_' || character.is_alphanumeric()
}

/// What follows a block comment, given what follows its `/*`. Block
/// comments nest.
fn after_block_comment(comment: &str) -> &str {
    let mut rest = comment;
    let mut depth = 1_usize;
    loop {
        if let Some(inner) = rest.strip_prefix("/*") {
            depth += 1;
            rest = inner;
        } else if let Some(after) = rest.strip_prefix("*/") {
            depth -= 1;
            if depth == 0 {
                return after;
            }
            rest = after;
        } else {
            let mut characters = rest.chars();
            if characters.next().is_none() {
                return "";
            }
            rest = characters.as_str();
        }
    }
}

/// What follows a string, given what follows its opening quote.
fn after_string(string: &str) -> &str {
    let mut characters = string.chars();
    loop {
        match characters.next() {
            None | Some('"') => return characters.as_str(),
            Some('\\') => {
                characters.next();
            }
            Some(_) => {}
        }
    }
}

/// What follows a `'`: the rest of a character literal is skipped, and a
/// lifetime or label is left to be read as a name.
fn after_quote(quoted: &str) -> &str {
    let mut characters = quoted.chars();
    match (characters.next(), characters.next()) {
        (Some('\\'), Some(_)) => characters
            .as_str()
            .split_once('\'')
            .map_or("", |(_, after)| after),
        (Some(_), Some('\'')) => characters.as_str(),
        _ => quoted,
    }
}

/// What follows a raw string, given what follows its `r`, `br` or `cr`;
/// or `after` itself when no raw string starts there, because the letters
/// were an ordinary name.
fn after_raw_string(after: &str) -> &str {
    let hashes = after.chars().take_while(|&mark| mark == '#').count();
    let Some(body) = after.trim_start_matches('#').strip_prefix('"') else {
        return after;
    };
    let end = format!("\"{}", "#".repeat(hashes));
    body.split_once(&end).map_or("", |(_, rest)| rest)
}

#[cfg(test)]
mod tests {
    use super::{ALLOWLIST, Finding, check, unjustified, uses_unsafe};
    use crate::ROOT;
    use crate::tree::memory::Memory;
    use crate::tree::{Disk, Tree};

    /// One package of a test graph.
    struct Package {
        /// Its name.
        name: &'static str,
        /// Its version.
        version: &'static str,
        /// Its `links` value, as JSON.
        links: &'static str,
        /// Its dependencies: the package ID and the JSON `dep_kinds` array.
        deps: &'static [(&'static str, &'static str)],
        /// The kind of its one target: `lib`, `bin` or `proc-macro`.
        kind: &'static str,
    }

    /// A pure package with no dependencies.
    const fn package(name: &'static str, version: &'static str) -> Package {
        Package {
            name,
            version,
            links: "null",
            deps: &[],
            kind: "lib",
        }
    }

    /// A normal dependency, as `cargo metadata` writes its kinds.
    const NORMAL: &str = r#"[{"kind":null,"target":null}]"#;
    /// A dev-dependency.
    const DEV: &str = r#"[{"kind":"dev","target":null}]"#;
    /// A build dependency.
    const BUILD: &str = r#"[{"kind":"build","target":null}]"#;

    /// `cargo metadata` output for a workspace of `members` and `packages`.
    /// A package's ID is `<name>@<version>` and its manifest is
    /// `/src/<name>-<version>/Cargo.toml`.
    fn metadata(members: &[&str], packages: &[Package]) -> String {
        let described: Vec<String> = packages
            .iter()
            .map(|package| {
                format!(
                    r#"{{"name":"{0}","version":"{1}","id":"{0}@{1}","links":{2},"manifest_path":"/src/{0}-{1}/Cargo.toml","targets":[{{"kind":["{3}"]}}]}}"#,
                    package.name, package.version, package.links, package.kind
                )
            })
            .collect();
        let nodes: Vec<String> = packages
            .iter()
            .map(|package| {
                let deps: Vec<String> = package
                    .deps
                    .iter()
                    .map(|(id, kinds)| format!(r#"{{"pkg":"{id}","dep_kinds":{kinds}}}"#))
                    .collect();
                format!(
                    r#"{{"id":"{}@{}","deps":[{}]}}"#,
                    package.name,
                    package.version,
                    deps.join(",")
                )
            })
            .collect();
        let members: Vec<String> = members.iter().map(|id| format!(r#""{id}""#)).collect();
        format!(
            r#"{{"packages":[{}],"workspace_members":[{}],"resolve":{{"nodes":[{}]}}}}"#,
            described.join(","),
            members.join(","),
            nodes.join(",")
        )
    }

    /// A server that depends on one crate, `name` at version 1.0.0, whose
    /// manifest declares `links`.
    fn server_with(name: &'static str, links: &'static str) -> String {
        let id: &'static str = Box::leak(format!("{name}@1.0.0").into_boxed_str());
        let deps: &'static [(&'static str, &'static str)] = Box::leak(Box::new([(id, NORMAL)]));
        metadata(
            &["gunmetal-server@0.0.0"],
            &[
                Package {
                    deps,
                    ..package("gunmetal-server", "0.0.0")
                },
                Package {
                    links,
                    ..package(name, "1.0.0")
                },
            ],
        )
    }

    /// A tree holding the server's own safe source.
    fn server_sources() -> Memory {
        Memory::default().with(
            "/src/gunmetal-server-0.0.0/src/main.rs",
            "#![forbid(unsafe_code)]\nfn main() {}\n",
        )
    }

    /// The finding for an unlisted crate at version 1.0.0.
    fn unlisted(name: &str, sys: bool, links: Option<&str>, unsafe_in: Option<&str>) -> Finding {
        Finding::Unlisted {
            name: name.to_owned(),
            version: "1.0.0".to_owned(),
            sys,
            links: links.map(str::to_owned),
            unsafe_in: unsafe_in.map(str::to_owned),
        }
    }

    /// An allow-list entry with a reason and a requirement.
    fn entry(name: &str) -> String {
        format!(
            "[[crate]]\nname = \"{name}\"\nreason = \"Reviewed for the test.\"\nrequirement = \"SEC-TM-034\"\n\n"
        )
    }

    #[test]
    fn a_graph_with_no_native_or_unsafe_code_passes_the_empty_list() {
        let tree = server_sources().with(
            "/src/plain-1.0.0/src/lib.rs",
            "//! Never unsafe.\npub fn add(a: u8, b: u8) -> u8 { a.wrapping_add(b) }\n",
        );
        assert_eq!(check(&tree, &server_with("plain", "null"), ""), []);
    }

    /// Verifies: SEC-TM-034
    #[test]
    fn a_crate_named_as_native_bindings_must_be_listed() {
        let safe = "pub const VERSION: u8 = 1;\n";
        for name in ["zlib-sys", "zlib_sys"] {
            let tree = server_sources().with(&format!("/src/{name}-1.0.0/src/lib.rs"), safe);
            assert_eq!(
                check(&tree, &server_with(name, "null"), ""),
                [unlisted(name, true, None, None)]
            );
            assert_eq!(check(&tree, &server_with(name, "null"), &entry(name)), []);
        }
        // The name has to end with the marker.
        let tree = server_sources().with("/src/sys-info-1.0.0/src/lib.rs", safe);
        assert_eq!(check(&tree, &server_with("sys-info", "null"), ""), []);
    }

    /// Verifies: SEC-TM-034
    #[test]
    fn a_crate_that_links_a_native_library_must_be_listed() {
        let tree =
            server_sources().with("/src/zlib-1.0.0/src/lib.rs", "pub const VERSION: u8 = 1;\n");
        assert_eq!(
            check(&tree, &server_with("zlib", "\"z\""), ""),
            [unlisted("zlib", false, Some("z"), None)]
        );
        assert_eq!(
            check(&tree, &server_with("zlib", "\"z\""), &entry("zlib")),
            []
        );
    }

    /// Verifies: SEC-TM-034
    #[test]
    fn a_crate_that_uses_unsafe_must_be_listed() {
        let tree = server_sources()
            .with("/src/fast-1.0.0/build.rs", "fn main() {}\n")
            .with("/src/fast-1.0.0/src/lib.rs", "mod raw;\n")
            .with(
                "/src/fast-1.0.0/src/raw/mod.rs",
                "pub fn first(bytes: &[u8]) -> u8 { unsafe { *bytes.get_unchecked(0) } }\n",
            )
            .with("/src/fast-1.0.0/src/zeta.rs", "unsafe impl Send for Z {}\n");
        assert_eq!(
            check(&tree, &server_with("fast", "null"), ""),
            [unlisted("fast", false, None, Some("src/raw/mod.rs"))]
        );
        assert_eq!(
            check(&tree, &server_with("fast", "null"), &entry("fast")),
            []
        );
    }

    /// Verifies: SEC-TM-034
    #[test]
    fn a_finding_names_every_reason_a_crate_needs_review() {
        let tree = server_sources().with(
            "/src/sqlite-sys-1.0.0/src/lib.rs",
            "unsafe extern \"C\" { pub fn sqlite3_libversion_number() -> i32; }\n",
        );
        assert_eq!(
            check(&tree, &server_with("sqlite-sys", "\"sqlite3\""), ""),
            [unlisted(
                "sqlite-sys",
                true,
                Some("sqlite3"),
                Some("src/lib.rs")
            )]
        );
    }

    #[test]
    fn unsafe_in_a_comment_a_string_or_another_language_does_not_count() {
        let tree = server_sources()
            .with(
                "/src/plain-1.0.0/src/lib.rs",
                "#![forbid(unsafe_code)]\n//! No unsafe here.\npub const NOTE: &str = \"unsafe\";\n",
            )
            .with("/src/plain-1.0.0/notes.md", "unsafe { }\n")
            .with("/src/plain-1.0.0/vendor/zlib.c", "/* unsafe */ int unsafe;\n")
            .with("/src/plain-1.0.0/src/lib.rs.orig", "unsafe fn old() {}\n");
        assert_eq!(check(&tree, &server_with("plain", "null"), ""), []);
    }

    /// Verifies: SEC-TM-034
    #[test]
    fn the_workspace_members_that_ship_are_checked_too() {
        let tree = Memory::default().with(
            "/src/gunmetal-server-0.0.0/src/main.rs",
            "fn main() { unsafe { std::hint::unreachable_unchecked() } }\n",
        );
        let graph = metadata(
            &["gunmetal-server@0.0.0"],
            &[package("gunmetal-server", "0.0.0")],
        );
        assert_eq!(
            check(&tree, &graph, ""),
            [Finding::Unlisted {
                name: "gunmetal-server".to_owned(),
                version: "0.0.0".to_owned(),
                sys: false,
                links: None,
                unsafe_in: Some("src/main.rs".to_owned()),
            }]
        );
    }

    /// A graph for the tests of which crates ship. The server reaches
    /// `through` by a normal dependency of `direct`; `tested` is only a
    /// dev-dependency and `builder` only a build dependency; `both` is a
    /// build dependency and a normal one. The tooling members depend on
    /// `tool-only`, and the server on the testkit.
    fn mixed_graph() -> String {
        metadata(
            &[
                "gunmetal-fuzz@0.0.0",
                "gunmetal-server@0.0.0",
                "gunmetal-testkit@0.0.0",
                "xtask@0.0.0",
            ],
            &[
                Package {
                    deps: &[("fuzz-only@1.0.0", NORMAL)],
                    ..package("gunmetal-fuzz", "0.0.0")
                },
                Package {
                    deps: &[
                        ("builder@1.0.0", BUILD),
                        ("tested@1.0.0", DEV),
                        ("direct@1.0.0", NORMAL),
                        (
                            "both@1.0.0",
                            r#"[{"kind":"build","target":null},{"kind":null,"target":"cfg(unix)"}]"#,
                        ),
                        ("gunmetal-testkit@0.0.0", NORMAL),
                    ],
                    ..package("gunmetal-server", "0.0.0")
                },
                Package {
                    deps: &[("kit-only@1.0.0", NORMAL)],
                    ..package("gunmetal-testkit", "0.0.0")
                },
                Package {
                    deps: &[("tool-only@1.0.0", NORMAL)],
                    ..package("xtask", "0.0.0")
                },
                package("both", "1.0.0"),
                package("builder", "1.0.0"),
                Package {
                    deps: &[("through@1.0.0", NORMAL)],
                    ..package("direct", "1.0.0")
                },
                package("fuzz-only", "1.0.0"),
                package("kit-only", "1.0.0"),
                package("tested", "1.0.0"),
                package("through", "1.0.0"),
                package("tool-only", "1.0.0"),
            ],
        )
    }

    /// Sources for [`mixed_graph`]: every third-party crate uses `unsafe`.
    fn mixed_sources() -> Memory {
        let mut tree = server_sources()
            .with("/src/gunmetal-fuzz-0.0.0/src/lib.rs", "")
            .with("/src/gunmetal-testkit-0.0.0/src/lib.rs", "")
            .with("/src/xtask-0.0.0/src/main.rs", "fn main() {}\n")
            .with("/src/direct-1.0.0/src/lib.rs", "");
        for name in [
            "both",
            "builder",
            "fuzz-only",
            "kit-only",
            "tested",
            "through",
            "tool-only",
        ] {
            tree = tree.with(
                &format!("/src/{name}-1.0.0/src/lib.rs"),
                "unsafe fn f() {}\n",
            );
        }
        tree
    }

    /// Verifies: SEC-TM-034
    #[test]
    fn the_shipped_graph_is_what_shipped_members_reach_by_normal_dependencies() {
        assert_eq!(
            check(&mixed_sources(), &mixed_graph(), ""),
            [
                unlisted("both", false, None, Some("src/lib.rs")),
                unlisted("kit-only", false, None, Some("src/lib.rs")),
                unlisted("through", false, None, Some("src/lib.rs")),
            ]
        );
    }

    /// A graph for the tests of procedural macros. The server depends on
    /// `data`, which depends on the macro `derive`. The macro depends on
    /// `parser` and `shared`, and the server reaches `shared` through `data`
    /// as well.
    fn macro_graph() -> String {
        metadata(
            &["gunmetal-server@0.0.0"],
            &[
                Package {
                    deps: &[("data@1.0.0", NORMAL)],
                    ..package("gunmetal-server", "0.0.0")
                },
                Package {
                    deps: &[("derive@1.0.0", NORMAL), ("shared@1.0.0", NORMAL)],
                    ..package("data", "1.0.0")
                },
                Package {
                    deps: &[("parser@1.0.0", NORMAL), ("shared@1.0.0", NORMAL)],
                    kind: "proc-macro",
                    ..package("derive", "1.0.0")
                },
                package("parser", "1.0.0"),
                package("shared", "1.0.0"),
            ],
        )
    }

    /// Sources for [`macro_graph`]: every crate but the server uses
    /// `unsafe`.
    fn macro_sources() -> Memory {
        let mut tree = server_sources();
        for name in ["data", "derive", "parser", "shared"] {
            tree = tree.with(
                &format!("/src/{name}-1.0.0/src/lib.rs"),
                "unsafe fn f() {}\n",
            );
        }
        tree
    }

    #[test]
    fn a_procedural_macro_and_what_only_it_reaches_are_not_linked_into_what_ships() {
        // The compiler runs a procedural macro while it builds; neither the
        // macro nor the crates only it uses are linked into the result.
        assert_eq!(
            check(&macro_sources(), &macro_graph(), ""),
            [
                unlisted("data", false, None, Some("src/lib.rs")),
                unlisted("shared", false, None, Some("src/lib.rs")),
            ]
        );
        let allowlist = [
            entry("data"),
            entry("derive"),
            entry("parser"),
            entry("shared"),
        ]
        .concat();
        assert_eq!(
            check(&macro_sources(), &macro_graph(), &allowlist),
            [
                Finding::Unused {
                    name: "derive".to_owned(),
                },
                Finding::Unused {
                    name: "parser".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn a_package_is_a_macro_when_any_of_its_targets_is_one() {
        // A library with a build script, or with any other target that is
        // not exactly a procedural macro, is linked.
        let graph = server_with("fast", "null");
        let tree = server_sources().with("/src/fast-1.0.0/src/lib.rs", "unsafe fn f() {}\n");
        let found = [unlisted("fast", false, None, Some("src/lib.rs"))];
        // The last package in the graph is `fast`.
        let (before, after) = graph
            .rsplit_once(r#"[{"kind":["lib"]}]"#)
            .expect("the graph has library targets");
        for (targets, expected) in [
            (
                r#"[{"kind":["lib"]},{"kind":["custom-build"]}]"#,
                &found[..],
            ),
            (r#"[{"kind":["cdylib","rlib"]},{"kind":["bench"]}]"#, &found),
            (r#"[{"kind":["proc-macros"]},{"kind":[7]}]"#, &found),
            ("[]", &found),
            (r#"[{"kind":["test"]},{"kind":["proc-macro"]}]"#, &[]),
            (r#"[{"kind":["lib","proc-macro"]}]"#, &[]),
        ] {
            let changed = format!("{before}{targets}{after}");
            assert_eq!(check(&tree, &changed, ""), expected);
        }
    }

    #[test]
    fn an_entry_for_a_crate_that_does_not_ship_or_needs_no_review_fails() {
        let allowlist = [
            entry("absent"),
            entry("both"),
            entry("builder"),
            entry("direct"),
            entry("kit-only"),
            entry("tested"),
            entry("through"),
            entry("tool-only"),
        ]
        .concat();
        let unused = |name: &str| Finding::Unused {
            name: name.to_owned(),
        };
        assert_eq!(
            check(&mixed_sources(), &mixed_graph(), &allowlist),
            [
                unused("absent"),
                unused("builder"),
                unused("direct"),
                unused("tested"),
                unused("tool-only"),
            ]
        );
    }

    #[test]
    fn each_crate_is_reported_once_and_each_version_on_its_own() {
        // The server reaches `shared` 2.0.0 twice, through `left` and
        // `right`, and `shared` 1.0.0 through `left`. The two depend on
        // each other as dev-dependencies do, in a cycle.
        let graph = metadata(
            &["gunmetal-server@0.0.0"],
            &[
                Package {
                    deps: &[("right@1.0.0", NORMAL), ("left@1.0.0", NORMAL)],
                    ..package("gunmetal-server", "0.0.0")
                },
                Package {
                    deps: &[
                        ("shared@2.0.0", NORMAL),
                        ("shared@1.0.0", NORMAL),
                        ("right@1.0.0", NORMAL),
                    ],
                    ..package("left", "1.0.0")
                },
                Package {
                    deps: &[("shared@2.0.0", NORMAL), ("left@1.0.0", NORMAL)],
                    ..package("right", "1.0.0")
                },
                package("shared", "1.0.0"),
                package("shared", "2.0.0"),
            ],
        );
        let tree = server_sources()
            .with("/src/left-1.0.0/src/lib.rs", "")
            .with("/src/right-1.0.0/src/lib.rs", "")
            .with("/src/shared-1.0.0/src/lib.rs", "")
            .with("/src/shared-1.0.0/src/old.rs", "unsafe fn f() {}\n")
            .with("/src/shared-2.0.0/src/lib.rs", "unsafe fn f() {}\n");
        assert_eq!(
            check(&tree, &graph, ""),
            [
                unlisted("shared", false, None, Some("src/old.rs")),
                Finding::Unlisted {
                    name: "shared".to_owned(),
                    version: "2.0.0".to_owned(),
                    sys: false,
                    links: None,
                    unsafe_in: Some("src/lib.rs".to_owned()),
                },
            ]
        );
        assert_eq!(check(&tree, &graph, &entry("shared")), []);
    }

    /// A tree whose files named `binary.rs` are listed and cannot be read,
    /// as a file that is not UTF-8 cannot.
    struct Binary(Memory);

    impl Tree for Binary {
        fn read(&self, path: &str) -> Option<String> {
            self.0.read(path).filter(|_| !path.ends_with("binary.rs"))
        }

        fn files(&self, dir: &str) -> Vec<String> {
            self.0.files(dir)
        }
    }

    /// Verifies: SEC-TM-034
    #[test]
    fn a_crate_whose_sources_cannot_be_read_fails() {
        let unreadable = [Finding::Unreadable {
            name: "opaque".to_owned(),
            version: "1.0.0".to_owned(),
        }];
        let graph = server_with("opaque", "null");
        // Nothing was downloaded, or only files that are not Rust.
        assert_eq!(check(&server_sources(), &graph, ""), unreadable);
        let no_rust = server_sources().with("/src/opaque-1.0.0/README.md", "");
        assert_eq!(check(&no_rust, &graph, ""), unreadable);
        // One source file is not text.
        let binary = Binary(
            server_sources()
                .with("/src/opaque-1.0.0/src/binary.rs", "")
                .with("/src/opaque-1.0.0/src/lib.rs", ""),
        );
        assert_eq!(
            binary.read("/src/opaque-1.0.0/src/lib.rs"),
            Some(String::new())
        );
        assert_eq!(check(&binary, &graph, ""), unreadable);
        // Listing the crate does not make it readable.
        assert_eq!(check(&binary, &graph, &entry("opaque")), unreadable);
    }

    /// Verifies: SEC-TM-034
    #[test]
    fn an_entry_needs_a_name_a_reason_and_a_requirement() {
        let allowlist = r#"# Comments and lines before the first entry are ignored.
name = "ignored"

[[crate]]
name = "complete"
reason = "Reviewed."
requirement = "SEC-HIS-016, SEC-MED-033"

[[crate]]
reason = "No name."
requirement = "SEC-TM-034"

[[crate]]
name = "no-reason"
requirement = "SEC-TM-034"

[[crate]]
name = "blank-reason"
reason = "  "
requirement = "SEC-TM-034"

[[crate]]
name = "no-requirement"
reason = "Reviewed."

[[crate]]
name = "hidden-requirement"
reason = "Reviewed."
requirement = "SEC-TM-034" # a comment hides the value

[[crate]]
name = "neither"

  [[crate]]
  name = "indented"
  reason = "Reviewed."
  requirement = "SEC-TM-034"

[[crate]]
"#;
        let lacks = |name: &str, missing: &'static str| Finding::Unjustified {
            name: name.to_owned(),
            missing,
        };
        assert_eq!(
            unjustified(allowlist),
            [
                Finding::Unnamed { entry: 2 },
                lacks("no-reason", "reason"),
                lacks("blank-reason", "reason"),
                lacks("no-requirement", "requirement"),
                lacks("hidden-requirement", "requirement"),
                lacks("neither", "reason"),
                Finding::Unnamed { entry: 9 },
            ]
        );
        assert_eq!(unjustified(""), []);
    }

    /// Verifies: SEC-TM-034
    #[test]
    fn a_requirement_reference_is_one_or_more_requirement_ids() {
        let with = |requirement: &str| {
            unjustified(&format!(
                "[[crate]]\nname = \"zlib\"\nreason = \"Reviewed.\"\nrequirement = \"{requirement}\"\n"
            ))
        };
        for accepted in [
            "SEC-TM-034",
            "SEC-HIS-016",
            "SEC-TM-034, SEC-MED-025, SEC-SUP-026",
        ] {
            assert_eq!(with(accepted), []);
        }
        for refused in [
            "",
            "see the review",
            "TM-034",
            "SEC-034",
            "SEC--034",
            "SEC-tm-034",
            "SEC-T1-034",
            "sec-TM-034",
            "SEC-TM-34",
            "SEC-TM-0344",
            "SEC-TM-03x",
            "SEC-TM-",
            "SEC-TM-034 ",
            "SEC-TM-034,SEC-MED-025",
            "SEC-TM-034, ",
            "SEC-TM-034, WP-126",
        ] {
            assert_eq!(
                with(refused),
                [Finding::Unjustified {
                    name: "zlib".to_owned(),
                    missing: "requirement",
                }]
            );
        }
    }

    /// Verifies: SEC-TM-034
    #[test]
    fn an_unjustified_entry_fails_even_when_its_crate_needs_listing() {
        let tree = server_sources().with("/src/zlib-sys-1.0.0/src/lib.rs", "");
        assert_eq!(
            check(
                &tree,
                &server_with("zlib-sys", "null"),
                "[[crate]]\nname = \"zlib-sys\"\n[[crate]]\nreason = \"Reviewed.\"\n"
            ),
            [
                Finding::Unjustified {
                    name: "zlib-sys".to_owned(),
                    missing: "reason",
                },
                Finding::Unnamed { entry: 2 },
            ]
        );
    }

    #[test]
    fn findings_come_in_order_the_list_then_the_graph_then_unused_entries() {
        let tree = server_sources().with("/src/zlib-sys-1.0.0/src/lib.rs", "");
        assert_eq!(
            check(
                &tree,
                &server_with("zlib-sys", "null"),
                "[[crate]]\nname = \"stale\"\n"
            ),
            [
                Finding::Unjustified {
                    name: "stale".to_owned(),
                    missing: "reason",
                },
                unlisted("zlib-sys", true, None, None),
                Finding::Unused {
                    name: "stale".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn input_that_is_not_cargo_metadata_fails() {
        let valid = server_with("zlib", "\"z\"");
        let tree = server_sources().with("/src/zlib-1.0.0/src/lib.rs", "");
        assert_eq!(
            check(&tree, &valid, ""),
            [unlisted("zlib", false, Some("z"), None)]
        );
        for (from, to) in [
            // Not JSON, or not an object with the three members.
            ("{\"packages\"", "\"packages\""),
            ("\"packages\"", "\"crates\""),
            ("\"workspace_members\"", "\"members\""),
            ("\"resolve\"", "\"resolved\""),
            ("\"nodes\"", "\"graph\""),
            // The members are not an array.
            (
                "\"workspace_members\":[\"gunmetal-server@0.0.0\"]",
                "\"workspace_members\":\"gunmetal-server@0.0.0\"",
            ),
            // A member that is not a string, or names no package.
            ("[\"gunmetal-server@0.0.0\"]", "[7]"),
            ("[\"gunmetal-server@0.0.0\"]", "[\"gunmetal-store@0.0.0\"]"),
            // A package without an ID, a name, a version, `links` or a
            // manifest, or with one of the wrong type.
            ("\"id\":\"zlib@1.0.0\",\"links\"", "\"links\""),
            ("\"id\":\"zlib@1.0.0\",\"links\"", "\"id\":7,\"links\""),
            ("\"name\":\"gunmetal-server\",", ""),
            ("\"name\":\"zlib\"", "\"name\":7"),
            ("\"version\":\"1.0.0\",", ""),
            ("\"links\":\"z\",", ""),
            ("\"links\":\"z\"", "\"links\":7"),
            (
                "\"manifest_path\":\"/src/zlib-1.0.0/Cargo.toml\"",
                "\"path\":7",
            ),
            ("/src/zlib-1.0.0/Cargo.toml", "/src/zlib-1.0.0/Cargo.lock"),
            ("/src/zlib-1.0.0/Cargo.toml", "Cargo.toml"),
            // A package without targets, or a target without kinds, or
            // either of the wrong type.
            (",\"targets\":[{\"kind\":[\"lib\"]}]", ""),
            ("\"targets\":[{\"kind\":[\"lib\"]}]", "\"targets\":7"),
            ("{\"kind\":[\"lib\"]}", "{\"name\":\"lib\"}"),
            ("\"kind\":[\"lib\"]", "\"kind\":\"lib\""),
            // A node without an ID or dependencies, or a dependency without
            // a package or kinds, or one that names no node.
            ("{\"id\":\"zlib@1.0.0\",\"deps\"", "{\"deps\""),
            ("\"deps\":[]", "\"dependencies\":[]"),
            ("\"deps\":[]", "\"deps\":7"),
            ("\"pkg\":\"zlib@1.0.0\",", ""),
            ("\"pkg\":\"zlib@1.0.0\"", "\"pkg\":\"zlib@2.0.0\""),
            (",{\"id\":\"zlib@1.0.0\",\"deps\":[]}", ""),
            (
                "\"dep_kinds\":[{\"kind\":null,\"target\":null}]",
                "\"kinds\":[]",
            ),
            (
                "\"dep_kinds\":[{\"kind\":null,\"target\":null}]",
                "\"dep_kinds\":7",
            ),
            ("{\"kind\":null,\"target\":null}", "{\"target\":null}"),
        ] {
            let broken = valid.replacen(from, to, 1);
            assert_ne!(broken, valid);
            assert_eq!(check(&tree, &broken, ""), [Finding::NotMetadata]);
        }
        // The packages or the nodes are not an array, or the document is
        // not an object.
        for document in [
            r#"{"packages":7,"workspace_members":[],"resolve":{"nodes":[]}}"#,
            r#"{"packages":[],"workspace_members":[],"resolve":{"nodes":7}}"#,
            r#"{"packages":[],"workspace_members":[],"resolve":[]}"#,
            "[]",
            "",
        ] {
            assert_eq!(check(&tree, document, ""), [Finding::NotMetadata]);
        }
        // A workspace with no members ships nothing.
        assert_eq!(
            check(
                &tree,
                r#"{"packages":[],"workspace_members":[],"resolve":{"nodes":[]}}"#,
                ""
            ),
            []
        );
    }

    #[test]
    fn finds_unsafe_in_code() {
        for source in [
            "unsafe { }",
            "unsafe{}",
            "pub unsafe fn f() {}",
            "unsafe impl Send for X {}",
            "unsafe extern \"C\" {}",
            "#[unsafe(no_mangle)]\nfn f() {}",
            "fn f() -> u8 {\n\tlet x = (unsafe{ g() });\n\tx\n}",
            "macro_rules! m { () => { unsafe $body } }",
            "é; unsafe {}",
        ] {
            assert!(uses_unsafe(source));
        }
    }

    #[test]
    fn finds_no_unsafe_in_safe_code() {
        for source in [
            "",
            "fn main() {}",
            "#![forbid(unsafe_code)]",
            "let not_unsafe = unsafely + unsafe1 + _unsafe + unsafe_;",
            "let unsafeé = Unsafe + UNSAFE;",
        ] {
            assert!(!uses_unsafe(source));
        }
    }

    #[test]
    fn skips_comments() {
        for source in [
            "// unsafe",
            "/// unsafe { }\nfn f() {}",
            "fn f() {} // unsafe {}",
            "/* unsafe */",
            "/* a\n unsafe\n */ fn f() {}",
            "/* a /* b */ unsafe */",
            "/* a /* b /* c */ */ unsafe */",
            "/*/ unsafe */",
            "/* unterminated unsafe",
            "/* a /* b */ unsafe",
            "/",
        ] {
            assert!(!uses_unsafe(source));
        }
        for source in [
            "// note\nunsafe {}",
            "/* a */ unsafe {}",
            "/**/unsafe {}",
            "/* a /* b */ c */ unsafe {}",
            "/* a /* b /* c */ */ d */ unsafe {}",
            "/* * / */ unsafe {}",
            "a / b; unsafe {}",
            "a * b; unsafe {}",
        ] {
            assert!(uses_unsafe(source));
        }
    }

    #[test]
    fn skips_strings() {
        for source in [
            r#""unsafe""#,
            r#"let a = "unsafe { }";"#,
            r#""a \" unsafe""#,
            r#""a \\\" unsafe""#,
            r#""unterminated unsafe"#,
            r#""ends with a backslash \"#,
            r#"b"unsafe""#,
            r#"c"unsafe""#,
            "\"two\nlines unsafe\"",
            r#""// " ; "unsafe""#,
            r#""/* " ; "unsafe""#,
        ] {
            assert!(!uses_unsafe(source));
        }
        for source in [
            r#""a"; unsafe {}"#,
            r#""a \" b"; unsafe {}"#,
            r#""a \\"; unsafe {}"#,
            r#""" ; unsafe {}"#,
            r#""// "; unsafe {}"#,
            r#""/* "; unsafe {}"#,
        ] {
            assert!(uses_unsafe(source));
        }
    }

    #[test]
    fn skips_raw_strings() {
        for source in [
            r#"r"unsafe""#,
            r##"r#"unsafe"#"##,
            r###"r##"a "# unsafe"##"###,
            r##"r#"a " unsafe"#"##,
            r#"br"unsafe""#,
            r##"br#"unsafe"#"##,
            r##"cr#"unsafe"#"##,
            r#"r#"unterminated unsafe""#,
            r#"r"a\" ; "unsafe""#,
        ] {
            assert!(!uses_unsafe(source));
        }
        for source in [
            r#"r"a" ; unsafe {}"#,
            r#"r"a\" ; unsafe {}"#,
            r##"r#"a"b"# ; unsafe {}"##,
            r###"br##"a"#b"## ; unsafe {}"###,
            r##"cr#"a"# ; unsafe {}"##,
            // `r`, `br` and `cr` are also ordinary names, and `r#` starts a
            // raw identifier.
            "let r = 1; unsafe {}",
            "let (br, cr) = (r, r); unsafe {}",
            "r#type; unsafe {}",
            "r # unsafe {}",
            "r",
        ] {
            assert_eq!(uses_unsafe(source), source.contains("unsafe {}"));
        }
        // Only those three prefixes are raw: after any other name a quote
        // opens an ordinary string, with escapes.
        assert!(!uses_unsafe(r#"b"a\" unsafe""#));
        assert!(!uses_unsafe(r#"xr"a\" unsafe {} ""#));
    }

    #[test]
    fn skips_character_literals_and_reads_past_lifetimes() {
        for source in [
            r#"'"' ; "unsafe""#,
            r#"'\'' ; "unsafe""#,
            r#"'\\' ; "unsafe""#,
            r#"'\u{22}' ; "unsafe""#,
            r#"b'"' ; "unsafe""#,
            r#"fn f<'a>(x: &'a str) -> &'static str { "unsafe" }"#,
            r#"'outer: loop { break 'outer; } "unsafe""#,
            r"'\n unterminated unsafe",
            "'",
            "'a",
            r"'\",
        ] {
            assert!(!uses_unsafe(source));
        }
        for source in [
            r#"'"' ; unsafe {}"#,
            r#"'\'' '"' ; unsafe {}"#,
            r#"'\\' '"' ; unsafe {}"#,
            r"'\u{22}' ; unsafe {}",
            r"'a' ; unsafe {}",
            r#"'é' '"' ; unsafe {}"#,
            r"fn f<'a>(x: &'a str) { unsafe {} }",
            r"fn f(x: &'static str, y: &'static str) { unsafe {} }",
        ] {
            assert!(uses_unsafe(source));
        }
    }

    #[test]
    fn the_repository_s_list_is_justified_sorted_and_names_each_crate_once() {
        let allowlist = Disk::new(ROOT)
            .read(ALLOWLIST)
            .expect("the allow-list is readable");
        assert_eq!(unjustified(&allowlist), []);
        let names: Vec<&str> = allowlist
            .lines()
            .filter_map(|line| crate::toml::value(line, "name"))
            .collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(names, sorted);
        assert!(names.contains(&"libsqlite3-sys"));
    }
}
