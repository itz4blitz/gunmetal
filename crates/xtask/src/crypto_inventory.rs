//! `xtask crypto-inventory`: the cryptographic inventory in record 9 and the
//! code agree (SEC-STD-018).
//!
//! Record 9 ([`RECORD`]) lists the implementations that may perform
//! cryptography (its allow-list) and every key the server holds (its
//! inventory). This check reads both tables and fails when the code says
//! something they do not:
//!
//! - a file of one of the two crypto modules ([`MODULES`]) names a crate
//!   its own crate depends on, and that crate is not an implementation on
//!   the allow-list;
//! - a struct in the secrets crate holds a `Secret`, making it a key
//!   wrapper type, and its documentation has no `Inventory: <name>` line;
//! - an `Inventory: <name>` line in the secrets crate names no row of the
//!   inventory;
//! - the record, or either table, is missing, so a rewrite of the record
//!   cannot switch the check off.
//!
//! `clippy.toml` already rejects the cryptographic crates' types outside
//! the two modules; this check keeps the record honest about what is
//! inside them. It reads only code outside test modules: everything from a
//! file's first `#[cfg(test)]` on is skipped.

use crate::tree::{Tree, is_rust};

/// Record 9, which holds the allow-list and the inventory.
pub const RECORD: &str = "docs/adr/0009-cryptography.md";

/// The heading of the allow-list; its table's second column names the
/// implementations.
pub const ALLOW_LIST: &str = "### 2. The allow-list";

/// The heading of the inventory; its first table's first column names the
/// keys.
pub const INVENTORY: &str = "### 5. The cryptographic inventory";

/// The two crypto modules (owner decision 38): each crate's directory, and
/// the module inside it, a file or a directory ending in `/`.
pub const MODULES: [(&str, &str); 2] = [
    ("crates/gunmetal-core", "src/crypto.rs"),
    ("crates/gunmetal-secrets", "src/crypto/"),
];

/// The secrets crate's sources, where every key wrapper type lives.
pub const SECRETS: &str = "crates/gunmetal-secrets/src";

/// Something `crypto-inventory` found wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finding {
    /// The record cannot be read.
    NoRecord,
    /// The record has no table, or an empty one, under this heading.
    NoTable {
        /// The heading.
        heading: &'static str,
    },
    /// A crypto module names a crate that is not on the allow-list.
    UnlistedCrate {
        /// The module file's path from the repository root.
        path: String,
        /// The line, counted from 1.
        line: usize,
        /// The crate, as Rust code names it.
        krate: String,
    },
    /// An `Inventory:` line names no row of the inventory.
    UnlistedKey {
        /// The file's path from the repository root.
        path: String,
        /// The line, counted from 1.
        line: usize,
        /// The name it gives.
        name: String,
    },
    /// A struct that holds a `Secret` has no `Inventory:` line.
    Unnamed {
        /// The file's path from the repository root.
        path: String,
        /// The struct's line, counted from 1.
        line: usize,
    },
}

/// Checks the repository in `tree`.
pub fn check(tree: &dyn Tree) -> Vec<Finding> {
    let Some(record) = tree.read(RECORD) else {
        return vec![Finding::NoRecord];
    };
    let implementations = column(&record, ALLOW_LIST, 1);
    let keys = column(&record, INVENTORY, 0);
    let mut findings: Vec<Finding> = [(ALLOW_LIST, &implementations), (INVENTORY, &keys)]
        .into_iter()
        .filter(|(_, names)| names.is_empty())
        .map(|(heading, _)| Finding::NoTable { heading })
        .collect();
    for (krate, module) in MODULES {
        let dependencies = dependencies(
            &tree
                .read(&format!("{krate}/Cargo.toml"))
                .unwrap_or_default(),
        );
        for path in module_files(tree, krate, module) {
            let source = tree.read(&path).unwrap_or_default();
            findings.extend(unlisted_crates(
                &path,
                &source,
                &dependencies,
                &implementations,
            ));
        }
    }
    for file in tree.files(SECRETS).into_iter().filter(|file| is_rust(file)) {
        let path = format!("{SECRETS}/{file}");
        let source = tree.read(&path).unwrap_or_default();
        findings.extend(key_types(&path, &source, &keys));
    }
    findings
}

/// The names in backticks in column `index` of the first table after the
/// line `heading`, with `-` written as `_`, as Rust code names crates.
fn column(record: &str, heading: &str, index: usize) -> Vec<String> {
    record
        .lines()
        .skip_while(|line| *line != heading)
        .skip_while(|line| !line.starts_with('|'))
        .take_while(|line| line.starts_with('|'))
        // The header and the separator.
        .skip(2)
        .filter_map(|row| row.split('|').nth(index + 1))
        .flat_map(|cell| cell.split('`').skip(1).step_by(2))
        .map(|name| name.replace('-', "_"))
        .collect()
}

/// The crates named in the `[dependencies]` table of `manifest`, as Rust
/// code names them, leaving out the workspace's own.
fn dependencies(manifest: &str) -> Vec<String> {
    manifest
        .lines()
        .skip_while(|line| *line != "[dependencies]")
        .skip(1)
        .take_while(|line| !line.starts_with('['))
        .filter_map(|line| line.split([' ', '.', '=']).next())
        .filter(|name| !name.is_empty() && !name.starts_with('#') && !name.starts_with("gunmetal-"))
        .map(|name| name.replace('-', "_"))
        .collect()
}

/// The paths of the Rust files of crypto module `module` in crate
/// directory `krate`.
fn module_files(tree: &dyn Tree, krate: &str, module: &str) -> Vec<String> {
    match module.strip_suffix('/') {
        Some(dir) => tree
            .files(&format!("{krate}/{dir}"))
            .into_iter()
            .filter(|file| is_rust(file))
            .map(|file| format!("{krate}/{module}{file}"))
            .collect(),
        None => vec![format!("{krate}/{module}")],
    }
}

/// The lines of `source` outside its test module, numbered from 1.
fn code_lines(source: &str) -> impl Iterator<Item = (usize, &str)> {
    source
        .lines()
        .take_while(|line| !line.trim_start().starts_with("#[cfg(test)]"))
        .zip(1..)
        .map(|(line, number)| (number, line.trim()))
}

/// The places where `source`, the module file at `path`, names one of
/// `dependencies` that is not one of `implementations`.
fn unlisted_crates(
    path: &str,
    source: &str,
    dependencies: &[String],
    implementations: &[String],
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (line, text) in code_lines(source).filter(|(_, text)| !text.starts_with("//")) {
        for krate in dependencies
            .iter()
            .filter(|krate| names_crate(text, krate) && !implementations.contains(krate))
        {
            findings.push(Finding::UnlistedCrate {
                path: path.to_owned(),
                line,
                krate: krate.clone(),
            });
        }
    }
    findings
}

/// Whether `text` holds a path that starts with crate `krate`: its name
/// followed by `::`, not preceded by part of another name.
fn names_crate(text: &str, krate: &str) -> bool {
    text.match_indices(&format!("{krate}::")).any(|(at, _)| {
        !text[..at]
            .chars()
            .next_back()
            .is_some_and(|before| before.is_alphanumeric() || before == '_' || before == ':')
    })
}

/// What is wrong with the key wrapper types and `Inventory:` lines in
/// `source`, the secrets crate's file at `path`, given the inventory's
/// `keys`.
fn key_types(path: &str, source: &str, keys: &[String]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut named = false;
    let mut lines = code_lines(source);
    while let Some((line, text)) = lines.next() {
        if let Some(name) = text
            .strip_prefix("///")
            .and_then(|doc| doc.trim().strip_prefix("Inventory:"))
            .map(str::trim)
        {
            named = true;
            if !keys.iter().any(|key| key == name) {
                findings.push(Finding::UnlistedKey {
                    path: path.to_owned(),
                    line,
                    name: name.to_owned(),
                });
            }
        } else if let Some(declaration) = struct_declaration(text) {
            let body = if declaration.ends_with('{') {
                lines
                    .by_ref()
                    .map(|(_, text)| text)
                    .take_while(|text| *text != "}")
                    .collect::<Vec<_>>()
                    .concat()
            } else {
                declaration.to_owned()
            };
            if body.contains("Secret<") && !named {
                findings.push(Finding::Unnamed {
                    path: path.to_owned(),
                    line,
                });
            }
            named = false;
        } else if text.ends_with(['{', '}', ';']) {
            named = false;
        }
    }
    findings
}

/// What follows the name and generics of the struct `text` declares (its
/// fields, or the `{` that opens them), or `None` when it declares none.
fn struct_declaration(text: &str) -> Option<&str> {
    let (visibility, rest) = text.split_once("struct ")?;
    // A declaration whose `where` clause starts on the next line opens its
    // fields further down, so it is read as one that opens them here.
    (visibility.is_empty() || (visibility.starts_with("pub") && visibility.ends_with(' '))).then(
        || {
            rest.find(['{', '(', ';'])
                .map_or("{", |fields| &rest[fields..])
        },
    )
}

#[cfg(test)]
mod tests {
    use super::{ALLOW_LIST, Finding, INVENTORY, RECORD, check};
    use crate::tree::memory::Memory;

    /// A record with the two tables, in the shape record 9 writes them.
    const TABLES: &str = "\
# 9. Cryptography

### 2. The allow-list

| Primitive | Implementation | Used for |
|---|---|---|
| Security randomness | `getrandom`, through one function | Every key |
| SHA-256 | `sha2` (RustCrypto) | Hashes |
| TLS | `rustls` with one provider, `rustls-webpki`, and the bundled `webpki-roots` | HTTPS |

Never allowed: `md5`.

### 5. The cryptographic inventory

| Name | Algorithm | Purpose |
|---|---|---|
| `root` | 256-bit secret | Source of every derived key |
| `url_signing` | HMAC-SHA-256 | Capability URLs |

| Name | Algorithm |
|---|---|
| `identity_pub` | Ed25519 |
";

    const CORE_MANIFEST: &str = "\
[package]
name = \"gunmetal-core\"

[dependencies]
# SHA-256 only, in src/crypto.rs.
sha2.workspace = true

[dev-dependencies]
proptest.workspace = true
";

    const SECRETS_MANIFEST: &str = "\
[dependencies]
getrandom = { workspace = true }
gunmetal-core = { path = \"../gunmetal-core\" }
md-5.workspace = true
rustls-webpki.workspace = true
";

    /// A repository whose record and code agree.
    fn agreeing() -> Memory {
        Memory::default()
            .with(RECORD, TABLES)
            .with("crates/gunmetal-core/Cargo.toml", CORE_MANIFEST)
            .with(
                "crates/gunmetal-core/src/crypto.rs",
                "//! sha2 is the only crate here.\nuse sha2::{Digest as _, Sha256};\n\
                 #[cfg(test)]\nmod tests {\n    use proptest::prelude::*;\n}\n",
            )
            .with("crates/gunmetal-secrets/Cargo.toml", SECRETS_MANIFEST)
            .with(
                "crates/gunmetal-secrets/src/crypto/tls.rs",
                "use gunmetal_core::crypto;\npub fn verify() {\n    webpki::verify();\n    rustls_webpki::x();\n}\n",
            )
            .with(
                "crates/gunmetal-secrets/src/root.rs",
                "/// The root secret.\n///\n/// Inventory: root\n#[derive(Debug)]\npub struct Root {\n    \
                 #[expect(\n        dead_code,\n    )]\n    secret: Secret<[u8; 32]>,\n}\n\n\
                 /// Inventory: url_signing\npub(crate) struct Signing(Secret<[u8; 32]>);\n\n\
                 pub struct Secret<T: Wipe>(T);\nstruct Plain {\n    bytes: [u8; 4],\n}\n",
            )
            .with("crates/gunmetal-secrets/src/notes.txt", "struct Loose(Secret<u8>);\n")
    }

    /// The record's text with `from` replaced by `to`.
    fn record_with(from: &str, to: &str) -> Memory {
        agreeing().with(RECORD, &TABLES.replace(from, to))
    }

    #[test]
    fn a_repository_whose_record_and_code_agree_passes() {
        assert_eq!(check(&agreeing()), vec![]);
    }

    /// Verifies: SEC-STD-018
    #[test]
    fn a_crypto_module_naming_a_crate_off_the_allow_list_fails_at_its_line() {
        let tree = agreeing()
            .with(
                "crates/gunmetal-secrets/src/crypto/digest.rs",
                "// md_5::Md5 is in a comment.\nuse md_5::Md5;\nfn f() { let _ = (md_5::Md5::new(), sha2::x()); }\n\
                 fn g() { my_md_5::x(); a::md_5::y(); }\n#[cfg(test)]\nfn h() { md_5::z(); }\n",
            )
            .with("crates/gunmetal-secrets/src/crypto/README.md", "md_5::Md5\n");
        let unlisted = |line| Finding::UnlistedCrate {
            path: "crates/gunmetal-secrets/src/crypto/digest.rs".to_owned(),
            line,
            krate: "md_5".to_owned(),
        };
        assert_eq!(check(&tree), vec![unlisted(2), unlisted(3)]);
    }

    /// Verifies: SEC-STD-018
    #[test]
    fn the_core_module_is_held_to_its_own_crate_s_dependencies() {
        let tree = agreeing()
            .with(
                "crates/gunmetal-core/Cargo.toml",
                &CORE_MANIFEST.replace("sha2.workspace", "blake3.workspace = true\nsha2.workspace"),
            )
            .with(
                "crates/gunmetal-core/src/crypto.rs",
                "use blake3::hash;\nuse sha2::Sha256;\n",
            );
        assert_eq!(
            check(&tree),
            vec![Finding::UnlistedCrate {
                path: "crates/gunmetal-core/src/crypto.rs".to_owned(),
                line: 1,
                krate: "blake3".to_owned(),
            }]
        );
    }

    /// Verifies: SEC-STD-018
    #[test]
    fn a_crate_taken_off_the_allow_list_fails_where_the_module_uses_it() {
        assert_eq!(
            check(&record_with(
                "| SHA-256 | `sha2` (RustCrypto) | Hashes |\n",
                ""
            )),
            vec![Finding::UnlistedCrate {
                path: "crates/gunmetal-core/src/crypto.rs".to_owned(),
                line: 2,
                krate: "sha2".to_owned(),
            }]
        );
    }

    /// Verifies: SEC-STD-018
    #[test]
    fn a_key_wrapper_type_without_an_inventory_line_fails_at_its_line() {
        let tree = agreeing().with(
            "crates/gunmetal-secrets/src/keys.rs",
            "/// Inventory: root\nfn root() {}\n\npub struct Session {\n    key: Secret<[u8; 32]>,\n}\n\
             /// A pepper.\n#[derive(Debug)]\nstruct Pepper(Secret<[u8; 32]>);\n\
             pub(super) struct Hidden(Secret<u8>);\nimpl Wrapper { fn new() -> Secret<u8> { x } }\n\
             // a struct Secret<u8> in a comment\npub struct Keyed<T>\nwhere\n    T: Wipe,\n{\n    key: Secret<T>,\n}\n\
             pub(in crate) struct Odd(Secret<u8>);\n",
        );
        let unnamed = |line| Finding::Unnamed {
            path: "crates/gunmetal-secrets/src/keys.rs".to_owned(),
            line,
        };
        assert_eq!(
            check(&tree),
            vec![
                unnamed(4),
                unnamed(9),
                unnamed(10),
                unnamed(13),
                unnamed(19)
            ]
        );
    }

    /// Verifies: SEC-STD-018
    /// Only the inventory's first table names keys; the tables after it
    /// list public keys and algorithms.
    #[test]
    fn an_inventory_line_naming_no_row_fails_at_its_line() {
        let tree = agreeing().with(
            "crates/gunmetal-secrets/src/crypto/keys.rs",
            "/// Inventory: session_hash\npub struct Session(Secret<[u8; 32]>);\n///Inventory:identity_pub\nstruct Id;\n",
        );
        let unlisted = |line, name: &str| Finding::UnlistedKey {
            path: "crates/gunmetal-secrets/src/crypto/keys.rs".to_owned(),
            line,
            name: name.to_owned(),
        };
        assert_eq!(
            check(&tree),
            vec![unlisted(1, "session_hash"), unlisted(3, "identity_pub")]
        );
    }

    #[test]
    fn a_missing_record_or_table_fails() {
        assert_eq!(
            check(&agreeing().with(RECORD, "")),
            vec![
                Finding::NoTable {
                    heading: ALLOW_LIST
                },
                Finding::NoTable { heading: INVENTORY },
                Finding::UnlistedCrate {
                    path: "crates/gunmetal-core/src/crypto.rs".to_owned(),
                    line: 2,
                    krate: "sha2".to_owned(),
                },
                Finding::UnlistedCrate {
                    path: "crates/gunmetal-secrets/src/crypto/tls.rs".to_owned(),
                    line: 4,
                    krate: "rustls_webpki".to_owned(),
                },
                Finding::UnlistedKey {
                    path: "crates/gunmetal-secrets/src/root.rs".to_owned(),
                    line: 3,
                    name: "root".to_owned(),
                },
                Finding::UnlistedKey {
                    path: "crates/gunmetal-secrets/src/root.rs".to_owned(),
                    line: 12,
                    name: "url_signing".to_owned(),
                },
            ]
        );
        assert_eq!(
            check(&Memory::default().with("README.md", "")),
            vec![Finding::NoRecord]
        );
    }

    #[test]
    fn a_heading_with_no_rows_under_it_is_a_missing_table() {
        let tree = record_with(
            "| `root` | 256-bit secret | Source of every derived key |\n| `url_signing` | HMAC-SHA-256 | Capability URLs |\n",
            "",
        )
        .with("crates/gunmetal-secrets/src/root.rs", "pub struct Plain;\n");
        assert_eq!(check(&tree), vec![Finding::NoTable { heading: INVENTORY }]);
    }

    /// Verifies: SEC-STD-018
    #[test]
    fn the_repository_s_inventory_and_crypto_modules_agree() {
        let args = ["crypto-inventory".to_owned()];
        assert_eq!(
            crate::dispatch(&args, crate::ROOT, 0, &[], &mut Vec::new()),
            Ok(())
        );
    }

    #[test]
    fn missing_manifests_and_modules_name_no_crate() {
        let tree = Memory::default().with(RECORD, TABLES);
        assert_eq!(check(&tree), vec![]);
    }
}
