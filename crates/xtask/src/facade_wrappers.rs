//! `xtask facade-wrappers`: in the WebAssembly facade's own files, the only
//! code compiled for the browser alone is the wrappers one reviewed macro
//! writes.
//!
//! Mirror values cross to JavaScript through `tsify::Ts`, whose conversion
//! runs only inside a JavaScript host (record 12, the addition of
//! 2026-10-05). So every export of the facade is a plain function, tested on
//! the host under the usual rules, and a wrapper compiled only for `wasm32`,
//! which converts the types and calls it. Host coverage cannot see a
//! wrapper, because the host build does not compile it. Mutation testing
//! cannot test one either: the mutation tool reports a function compiled for
//! another target as a survivor, so the wrappers are written by a macro,
//! which the tool does not read. The wrappers are the one written exception
//! to those two rules, and this check keeps the exception from growing
//! through the facade's own files. It reads text and parses nothing: the
//! facade's manifest, [`MANIFEST`], and every `.rs` file beneath
//! `crates/gunmetal-wasm`. It holds them to four rules.
//!
//! **The macro is the reviewed one.** [`MACRO`] must be, to the character,
//! the text of [`EXPORT`]. That macro writes one function behind the `wasm32`
//! gate: it converts each `tsify::Ts` parameter, calls the named function
//! once with every parameter in the order written, and converts the answer.
//! What a line of the macro is given can only be names, types and a doc
//! comment, so no line can add a branch, arithmetic or a value. Changing
//! what a wrapper does therefore means changing [`EXPORT`] here, in review.
//!
//! **No other condition.** In every other source, a line that holds the
//! letters `cfg` anywhere, in code, a comment or a string, must be exactly
//! `#[cfg(test)]`, alone on its line and not indented. So no `#[cfg]`,
//! `cfg_attr` or `cfg!` written in the facade takes code out of the host
//! build, whether it is written directly or by a `macro_rules!` macro
//! defined there.
//!
//! **No other file brought in, and no export by hand.** No other source
//! holds in code any name in [`REFUSED`]: `include`, the built-in macro that
//! compiles another file as part of this one; `path`, the attribute that
//! points a module at any file, `.rs` or not, inside the facade or outside
//! it; and `wasm_bindgen`, without which nothing is exported but through
//! [`MACRO`]. "In code" means outside comments, strings and character
//! literals, as read by the scanner `facade-unsafe` uses, so spacing such as
//! `include !(` or `# [ path` changes nothing, and a mention in a comment or
//! a string is not refused. A name is matched whole, wherever it stands in
//! code: `include_str` and `wasm_bindgen_test` pass, and `r#include!`,
//! `use std::include as bring;`, a macro given `path` to write into an
//! attribute, and a variable, field or lifetime called `path` are refused.
//! A symbol exported with `no_mangle` or `export_name` needs `unsafe` in this
//! edition, which the crate's lint tables and `facade-unsafe` refuse.
//!
//! **No target table.** No line of [`MANIFEST`] holds the letters `target`,
//! in a header, a key, a value or a comment. So it has no table for one
//! target, written as a header or as a dotted key, and no dependency that
//! only `wasm32` builds.
//!
//! **What the check does not see.**
//!
//! - What a macro defined in another crate writes where the facade calls it.
//!   A derive, an attribute or a function-like macro from a dependency can
//!   write an item behind a `wasm32` gate, with logic in it, and no rule
//!   here reads that item. `tsify`'s and `wasm-bindgen`'s own macros write
//!   their glue that way, which is why the glue is outside host coverage
//!   too. The guard is the review that a new dependency, or a new macro in
//!   another workspace crate, gets.
//! - What another crate exports: a dependency can export to the browser
//!   itself.
//! - The manifest beyond the letters `target`: a key spelt with TOML's
//!   escapes; a `path` key that moves the library or a test out of the
//!   facade's directory; and a `build` key that names a build script, which
//!   the gate's own search (SEC-SUP-026) finds only when the file is named
//!   `build.rs`.
//! - A value the build supplies. `env!` and `option_env!` read the
//!   environment of the build, which a build script can set, so a value can
//!   differ between the host build and the `wasm32` build. That is another
//!   value in code the host still compiles and measures, not code left out
//!   of the host build.
//! - A directory reached through a symbolic link: the walk lists the link
//!   as a file and does not follow it, and the compiler does.
//! - What a wrapper does when it runs: the check runs nothing.

use crate::facade_unsafe::FACADE;
use crate::native_code::uses_unsafe;
use crate::tree::{Tree, is_rust};

/// The facade source that holds the macro, from the repository root.
pub const MACRO: &str = "crates/gunmetal-wasm/src/export.rs";

/// The facade's manifest, from the repository root.
pub const MANIFEST: &str = "crates/gunmetal-wasm/Cargo.toml";

/// The whole text of [`MACRO`].
const EXPORT: &str = r#"//! The one piece of this crate that is compiled only for the browser: the
//! wrapper around each exported function.
//!
//! A mirror value crosses to JavaScript through `tsify::Ts`, and that
//! conversion runs only inside a JavaScript host. So each export is a plain
//! function of mirror types and plain values, tested on the host like all
//! other code, and one line of `export!` beside it, which writes the
//! function the browser calls (record 12, the addition of 2026-10-05):
//!
//! ```text
//! crate::export::export! {
//!     /// The browser's `normaliseText`.
//!     "normaliseText": fn normalise_text_export = normalise_text(input: &str, cap: u32; lines: Lines) -> NormalisedText
//! }
//! ```
//!
//! The wrapper takes the plain parameters as they are and each parameter
//! after the `;` as a `tsify::Ts` of that mirror type. It calls the function
//! once, with every parameter in the order written, and answers with a
//! `tsify::Ts` of the mirror type or a JavaScript error. A line of `export!`
//! can name things and do nothing else, so a wrapper holds no branch, no
//! arithmetic and no value of its own.
//!
//! Host coverage and mutation testing cannot see the wrappers: the host
//! build does not compile them, and the mutation tool does not read what a
//! macro writes. They are the one written exception to those two rules.
//! `xtask facade-wrappers` holds this crate's own files to that exception:
//! it fails when this file differs from the copy it holds, and when another
//! source of the crate holds a condition other than its tests' gate, brings
//! in another file as code or names `wasm_bindgen`. What a macro of another
//! crate writes is not in those files, and the check does not see it; its
//! documentation lists what else it does not see. The `wasm32` build in
//! `.github/workflows/wasm.yml` compiles every wrapper.

/// Writes the browser's wrapper around one function of this crate.
macro_rules! export {
    (
        $(#[doc = $doc:literal])*
        $javascript:literal: fn $export:ident = $function:ident(
            $($plain:ident: $kind:ty),*
            $(; $($crossing:ident: $crossed:ident),+)?
        ) -> $mirror:ident
    ) => {
        $(#[doc = $doc])*
        #[cfg(target_arch = "wasm32")]
        #[wasm_bindgen::prelude::wasm_bindgen(js_name = $javascript)]
        pub fn $export(
            $($plain: $kind,)*
            $($($crossing: tsify::Ts<$crossed>,)+)?
        ) -> Result<tsify::Ts<$mirror>, wasm_bindgen::JsError> {
            Ok(tsify::Ts::from_rust(&$function(
                $($plain,)*
                $($($crossing.to_rust()?,)+)?
            ))?)
        }
    };
}

pub(crate) use export;
"#;

/// The one line of any other facade source that may hold `cfg`: the gate of
/// test code.
const TEST_GATE: &str = "#[cfg(test)]";

/// The names no other facade source may hold in code: the built-in macro
/// that compiles another file as part of the source, the attribute that
/// points a module at another file, and the crate that exports to the
/// browser. None of them occurs in `unsafe_`, which [`in_code`] relies on.
const REFUSED: [&str; 3] = ["include", "path", "wasm_bindgen"];

/// Something `facade-wrappers` found wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finding {
    /// The facade's manifest or one of its Rust sources could not be read
    /// as text, so what it compiles for the browser alone cannot be
    /// established.
    Unreadable {
        /// Its path relative to the repository root.
        path: String,
    },
    /// The macro's source is not the text this check holds.
    Rewritten {
        /// Its path relative to the repository root.
        path: String,
    },
    /// A line of another source holds `cfg` and is not `#[cfg(test)]`.
    Condition {
        /// The source's path relative to the repository root.
        path: String,
        /// The line, counted from 1.
        line: usize,
    },
    /// Another source holds, in code, a name in [`REFUSED`].
    Names {
        /// The source's path relative to the repository root.
        path: String,
        /// The name.
        name: &'static str,
    },
    /// A line of the manifest holds `target`.
    Target {
        /// The manifest's path relative to the repository root.
        path: String,
        /// The line, counted from 1.
        line: usize,
    },
}

/// Finds every unreadable file, a macro that was rewritten, every condition
/// other than the test gate, every refused name and every target table, in
/// the WebAssembly facade.
pub fn check(tree: &dyn Tree) -> Vec<Finding> {
    let mut findings = Vec::new();
    for file in tree.files(FACADE) {
        let path = format!("{FACADE}/{file}");
        if path != MANIFEST && !is_rust(&path) {
            continue;
        }
        match tree.read(&path) {
            None => findings.push(Finding::Unreadable { path }),
            Some(manifest) if path == MANIFEST => findings.extend(targets(&path, &manifest)),
            Some(source) if path == MACRO => {
                findings.extend((source != EXPORT).then_some(Finding::Rewritten { path }));
            }
            Some(source) => {
                findings.extend(conditions(&path, &source));
                findings.extend(names(&path, &source));
            }
        }
    }
    findings
}

/// The number, counted from 1, of each line of `text` that `holds`.
fn lines(text: &str, holds: fn(&str) -> bool) -> impl Iterator<Item = usize> {
    text.lines()
        .enumerate()
        .filter(move |(_, line)| holds(line))
        .map(|(index, _)| index + 1)
}

/// The lines of the facade source `source`, whose path is `path`, that hold
/// `cfg` and are not the test gate.
fn conditions(path: &str, source: &str) -> Vec<Finding> {
    lines(source, |line| line.contains("cfg") && line != TEST_GATE)
        .map(|line| Finding::Condition {
            path: path.to_owned(),
            line,
        })
        .collect()
}

/// The names in [`REFUSED`] that the facade source `source`, whose path is
/// `path`, holds in code.
fn names(path: &str, source: &str) -> Vec<Finding> {
    REFUSED
        .into_iter()
        .filter(|name| in_code(source, name))
        .map(|name| Finding::Names {
            path: path.to_owned(),
            name,
        })
        .collect()
}

/// Whether the Rust source `source` holds the name `name`, whole, outside
/// comments, strings and character literals.
///
/// The scanner `facade-unsafe` uses skips exactly those and looks for one
/// name, `unsafe`. So the source is handed to it with that name written as
/// `unsafe_` wherever it stands, and then `name` written as `unsafe`. Each
/// step puts the characters of a name where the characters of a name were,
/// so the scanner finds comments, strings and literals where it did before;
/// a longer name that holds `name`, such as `include_str`, becomes a longer
/// name that is not `unsafe`; and no name in [`REFUSED`] occurs in
/// `unsafe_`, so the first step neither makes nor hides one.
fn in_code(source: &str, name: &str) -> bool {
    uses_unsafe(&source.replace("unsafe", "unsafe_").replace(name, "unsafe"))
}

/// The lines of the manifest `manifest`, whose path is `path`, that hold
/// `target`.
fn targets(path: &str, manifest: &str) -> Vec<Finding> {
    lines(manifest, |line| line.contains("target"))
        .map(|line| Finding::Target {
            path: path.to_owned(),
            line,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{EXPORT, Finding, MACRO, MANIFEST, check};
    use crate::facade_unsafe::FACADE;
    use crate::tree::Tree;
    use crate::tree::memory::Memory;

    /// A facade source that is not the macro's.
    const LINKS: &str = "crates/gunmetal-wasm/src/links.rs";

    /// The gate, as the macro writes it.
    const GATE: &str = r#"#[cfg(target_arch = "wasm32")]"#;

    /// What the check finds in a facade that holds only `source`, at
    /// [`LINKS`].
    fn found_in(source: &str) -> Vec<Finding> {
        check(&Memory::default().with(LINKS, source))
    }

    /// The finding for a condition on `line` of the source at [`LINKS`].
    fn condition(line: usize) -> Finding {
        Finding::Condition {
            path: LINKS.to_owned(),
            line,
        }
    }

    /// The finding for the refused `name` in the source at [`LINKS`].
    fn named(name: &'static str) -> Finding {
        Finding::Names {
            path: LINKS.to_owned(),
            name,
        }
    }

    /// The finding for a target table on `line` of [`MANIFEST`].
    fn target(line: usize) -> Finding {
        Finding::Target {
            path: MANIFEST.to_owned(),
            line,
        }
    }

    /// What the macro writes for the browser, read from the copy this check
    /// holds: one gated function that converts and calls once. The gate
    /// appears nowhere else in the copy, and the macro has one arm.
    #[test]
    fn the_macro_this_check_holds_writes_one_gated_function_that_converts_and_calls_once() {
        let written: Vec<&str> = EXPORT
            .lines()
            .skip_while(|line| *line != "    ) => {")
            .skip(1)
            .take_while(|line| *line != "    };")
            .map(str::trim)
            .collect();
        assert_eq!(
            written,
            [
                "$(#[doc = $doc])*",
                GATE,
                "#[wasm_bindgen::prelude::wasm_bindgen(js_name = $javascript)]",
                "pub fn $export(",
                "$($plain: $kind,)*",
                "$($($crossing: tsify::Ts<$crossed>,)+)?",
                ") -> Result<tsify::Ts<$mirror>, wasm_bindgen::JsError> {",
                "Ok(tsify::Ts::from_rust(&$function(",
                "$($plain,)*",
                "$($($crossing.to_rust()?,)+)?",
                "))?)",
                "}",
            ]
        );
        assert_eq!(EXPORT.matches("cfg").count(), 1);
        assert_eq!(EXPORT.matches("=>").count(), 1);
    }

    /// What a line of the macro may be given, read from the same copy:
    /// doc comments, a JavaScript name, two function names, parameters that
    /// are a name and a type or a name and a mirror, and a mirror. Nothing
    /// in it takes an expression, a block or an attribute other than a doc
    /// comment.
    #[test]
    fn the_macro_this_check_holds_takes_only_names_types_and_doc_comments() {
        let taken: Vec<&str> = EXPORT
            .lines()
            .skip_while(|line| *line != "macro_rules! export {")
            .skip(1)
            .take_while(|line| *line != "    ) => {")
            .map(str::trim)
            .collect();
        assert_eq!(
            taken,
            [
                "(",
                "$(#[doc = $doc:literal])*",
                "$javascript:literal: fn $export:ident = $function:ident(",
                "$($plain:ident: $kind:ty),*",
                "$(; $($crossing:ident: $crossed:ident),+)?",
                ") -> $mirror:ident",
            ]
        );
    }

    #[test]
    fn the_macro_as_held_and_sources_that_only_gate_their_tests_pass() {
        let links = r#"//! Links.

/// Reads a URL with the core's link filter.
pub fn parse_link(raw: &str) -> LinkOutcome {
    convert(raw)
}

crate::export::export! {
    /// The browser's `parseLink`.
    "parseLink": fn parse_link_export = parse_link(raw: &str) -> LinkOutcome
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads() {}
}
"#;
        let manifest = "[package]\nname = \"gunmetal-wasm\"\n\n[lib]\n\
                        crate-type = [\"cdylib\", \"rlib\"]\n\n[dependencies]\n\
                        gunmetal-core = { path = \"../gunmetal-core\", version = \"0.0.0\" }\n\
                        wasm-bindgen.workspace = true\n";
        let tree = Memory::default()
            .with(MANIFEST, manifest)
            .with(MACRO, EXPORT)
            .with(LINKS, links)
            .with(
                "crates/gunmetal-wasm/src/lib.rs",
                "mod export;\npub mod links;\n",
            )
            .with(
                "crates/gunmetal-wasm/tests/rules.rs",
                "const MANIFEST: &str = include_str!(\"../Cargo.toml\");\n\
                 #[test]\nfn rules() {}\n",
            );
        assert_eq!(check(&tree), []);
    }

    #[test]
    fn a_macro_source_that_differs_from_the_text_held_fails() {
        let edits = [
            // Arithmetic, a default or another function in the wrapper.
            ("$($plain,)*\n", "$($plain + 1,)*\n"),
            (
                "$($($crossing.to_rust()?,)+)?\n",
                "$($($crossing.to_rust().unwrap_or_default(),)+)?\n",
            ),
            ("&$function(", "&decide($function("),
            // The gate widened, or gone.
            (GATE, "#[cfg(not(test))]"),
            ("        #[cfg(target_arch = \"wasm32\")]\n", ""),
            // A line of the macro allowed to carry more.
            ("$(#[doc = $doc:literal])*", "$(#[$attribute:meta])*"),
            ("$function:ident(", "$function:expr, ("),
            // Something beside the macro.
            (
                "pub(crate) use export;\n",
                "pub(crate) use export;\n\npub fn decide() {}\n",
            ),
            ("\n/// Writes", "\nuse tsify::Ts;\n\n/// Writes"),
            // A comment, or one space.
            ("    ) => {\n", "    ) => {\n        // A note.\n"),
            ("pub fn $export(", "pub  fn $export("),
        ];
        for (from, to) in edits {
            let edited = EXPORT.replacen(from, to, 1);
            assert_ne!(edited, EXPORT, "the edit of {from:?} changes the text");
            assert_eq!(
                check(&Memory::default().with(MACRO, &edited)),
                [Finding::Rewritten {
                    path: MACRO.to_owned()
                }],
                "{from:?}"
            );
        }
        let whole = [
            String::new(),
            EXPORT.trim_end().to_owned(),
            format!("{EXPORT}\n"),
            format!("\n{EXPORT}"),
            EXPORT.to_uppercase(),
        ];
        for text in whole {
            assert_eq!(
                check(&Memory::default().with(MACRO, &text)),
                [Finding::Rewritten {
                    path: MACRO.to_owned()
                }],
                "{text:?}"
            );
        }
    }

    /// Any line that holds `cfg` outside the macro's source, other than the
    /// test gate, is a condition the host build would not see through,
    /// wherever the letters stand.
    #[test]
    fn a_line_of_another_source_that_holds_cfg_and_is_not_the_test_gate_fails() {
        let source = [
            "//! A module.",
            "#[cfg(test)]",
            "mod tests {}",
            // Line 4: another condition.
            "#[cfg(not(test))]",
            "fn hidden() {}",
            // Lines 6 and 7: the test gate indented, and sharing its line.
            "    #[cfg(test)]",
            "#[cfg(test)] mod inline {}",
            // Line 8: a conditional attribute.
            "#[cfg_attr(test, derive(Debug))]",
            "struct Mirror;",
            // Line 10: the macro.
            "fn decide() -> bool { cfg!(test) }",
            // Lines 11 and 12: the browser's gate, and the same target
            // written another way.
            "#[cfg(target_arch = \"wasm32\")]",
            "#[cfg(target_family = \"wasm\")]",
            // Lines 13 and 14: a comment and a string.
            "// The cfg above.",
            "const NOTE: &str = \"cfg\";",
            // Lines 15 to 17: the test gate with a space after it, with a
            // space in it, and inside a wider condition.
            "#[cfg(test)] ",
            "#[cfg(test )]",
            "#[cfg(all(test, target_arch = \"wasm32\"))]",
            // Line 18: the whole file.
            "#![cfg(test)]",
            "fn last() {}",
        ]
        .join("\n");
        assert_eq!(
            found_in(&source),
            [4, 6, 7, 8, 10, 11, 12, 13, 14, 15, 16, 17, 18].map(condition)
        );
    }

    /// A second macro that gates what it writes is refused by its gate, and
    /// so is a copy of the reviewed one anywhere but in its own file.
    #[test]
    fn a_gate_in_any_other_source_fails_even_inside_a_macro() {
        let second = r#"//! Links.

macro_rules! browser {
    ($item:item) => {
        #[cfg(target_arch = "wasm32")]
        $item
    };
}
"#;
        assert_eq!(found_in(second), [condition(5)]);
        // The gate is line 45 of the macro's own source, and it names
        // `wasm-bindgen` in code.
        assert_eq!(found_in(EXPORT), [condition(45), named("wasm_bindgen")]);
        let elsewhere = Memory::default()
            .with("crates/gunmetal-wasm/src/nested/export.rs", GATE)
            .with("crates/gunmetal-wasm/tests/export.rs", GATE)
            .with("crates/gunmetal-wasm/src/export.rs.rs", GATE);
        assert_eq!(
            check(&elsewhere),
            [
                Finding::Condition {
                    path: "crates/gunmetal-wasm/src/export.rs.rs".to_owned(),
                    line: 1,
                },
                Finding::Condition {
                    path: "crates/gunmetal-wasm/src/nested/export.rs".to_owned(),
                    line: 1,
                },
                Finding::Condition {
                    path: "crates/gunmetal-wasm/tests/export.rs".to_owned(),
                    line: 1,
                },
            ]
        );
    }

    /// The check reads the facade's own manifest and its Rust sources. A
    /// file of another kind is not read: the compiler reads one only through
    /// `include!` or `#[path]`, which every source is refused. Nor is a
    /// manifest or a source outside the facade, or a manifest nested in it.
    #[test]
    fn only_the_manifest_and_the_rust_sources_of_the_facade_are_read() {
        let hidden =
            "#[cfg(not(test))]\n#[wasm_bindgen]\npub fn hidden() { include!(\"x.inc\") }\n";
        let table = "[target.wasm32-unknown-unknown.dependencies]\ngated = \"1\"\n";
        let tree = Memory::default()
            .with(MANIFEST, table)
            .with("crates/gunmetal-wasm/nested/Cargo.toml", table)
            .with("crates/gunmetal-wasm-extra/Cargo.toml", table)
            .with("crates/gunmetal-wasm/src/notes.md", hidden)
            .with("crates/gunmetal-wasm/src/gated.inc", hidden)
            .with("crates/gunmetal-wasm/types/links.d.ts", hidden)
            .with("crates/gunmetal-wasm-extra/src/lib.rs", hidden)
            .with("crates/gunmetal-wasm-extra/src/export.rs", hidden)
            .with("crates/gunmetal-core/src/lib.rs", hidden)
            .with("crates/gunmetal-wasm/src/nested/deep.rs", hidden)
            .with("crates/gunmetal-wasm/tests/rules.rs", hidden);
        let read = |path: &str| {
            [
                Finding::Condition {
                    path: path.to_owned(),
                    line: 1,
                },
                Finding::Names {
                    path: path.to_owned(),
                    name: "include",
                },
                Finding::Names {
                    path: path.to_owned(),
                    name: "wasm_bindgen",
                },
            ]
        };
        let mut expected = vec![target(1)];
        expected.extend(read("crates/gunmetal-wasm/src/nested/deep.rs"));
        expected.extend(read("crates/gunmetal-wasm/tests/rules.rs"));
        assert_eq!(check(&tree), expected);
    }

    /// A source that brings in another file as code is refused, however
    /// the macro or the attribute is spaced, called, renamed or assembled.
    /// The file it names would be one this check never reads: not Rust, or
    /// outside the facade.
    #[test]
    fn a_source_that_brings_in_another_file_as_code_fails() {
        for source in [
            "include!(\"gated.inc\");",
            "include !( \"gated.inc\" );",
            "include\n!\n(\"gated.inc\");",
            "fn f() { include /* a */ !(\"gated.inc\") }",
            "std::include! {\"gated.inc\"}",
            "core::include![\"../../elsewhere/gated.rs\"];",
            "r#include!(\"gated.inc\");",
            "include!(concat!(env!(\"OUT_DIR\"), \"/gated.rs\"));",
            "use std::include as bring;\nbring!(\"gated.inc\");",
            "macro_rules! bring {\n    ($m:ident) => { $m!(\"gated.inc\"); };\n}\nbring!(include);",
        ] {
            assert_eq!(found_in(source), [named("include")], "{source:?}");
        }
        for source in [
            "#[path = \"gated.inc\"]\nmod gated;",
            "# [ path = \"../../elsewhere/gated.rs\" ]\nmod gated;",
            "#\n[\npath\n=\n\"gated.inc\"\n]\nmod gated;",
            "#[ /* a */ path /* b */ = \"gated.inc\"]\nmod gated;",
            "macro_rules! point {\n    ($a:ident, $f:literal) => { #[$a = $f] mod gated; };\n}\n\
             point!(path, \"gated.inc\");",
        ] {
            assert_eq!(found_in(source), [named("path")], "{source:?}");
        }
        // A conditional `path` is refused by both rules.
        assert_eq!(
            found_in("#[cfg_attr(test, path = \"gated.inc\")]\nmod gated;"),
            [condition(1), named("path")]
        );
    }

    /// A source that exports by hand, or names anything of `wasm-bindgen`'s,
    /// is refused: an export is written by the reviewed macro or not at all.
    #[test]
    fn a_source_that_exports_by_hand_fails() {
        for source in [
            "#[wasm_bindgen]\npub fn parse(raw: &str) -> String {\n    raw.trim().to_owned()\n}",
            "# [ wasm_bindgen ( js_name = parseLink ) ]\npub fn parse() {}",
            "#[wasm_bindgen::prelude::wasm_bindgen]\npub fn parse() {}",
            "use wasm_bindgen::prelude::*;",
            "use ::wasm_bindgen as glue;",
            "pub fn parse(value: wasm_bindgen::JsValue) {}",
        ] {
            assert_eq!(found_in(source), [named("wasm_bindgen")], "{source:?}");
        }
        assert_eq!(
            found_in("#[wasm_bindgen]\npub fn parse() { include!(\"x.inc\") }"),
            [named("include"), named("wasm_bindgen")]
        );
    }

    /// The names count in code only, and whole: in a comment, a string or a
    /// character literal, or as part of a longer name, they pass. A name in
    /// code counts wherever it stands, so a variable, a field or a lifetime
    /// called `path` is refused too.
    #[test]
    fn a_name_counts_only_in_code_and_only_whole() {
        for source in [
            "// include!(\"gated.inc\") would bring in a file.",
            "/// Not `#[path = \"gated.inc\"]`, and not `#[wasm_bindgen]`.\nfn f() {}",
            "/* include!(\"a\") /* #[path = \"b\"] */ wasm_bindgen */",
            "const NOTE: &str = \"include!(\\\"x\\\") #[path = \\\"y\\\"] #[wasm_bindgen]\";",
            "const RAW: &str = r#\"#[path = \"gated.inc\"] include!(\"x\")\"#;",
            "#[doc = \"include!\"]\nfn f() {}",
            "const P: char = 'p';",
            "const MANIFEST: &str = include_str!(\"../Cargo.toml\");",
            "const BYTES: &[u8] = include_bytes!(\"../Cargo.toml\");",
            "fn paths(included: u8, wasm_bindgen_test: u8) -> u8 {\n    included + wasm_bindgen_test\n}",
            // The word the scanner itself looks for is none of the names.
            "fn f() { unsafe {} }",
        ] {
            assert_eq!(found_in(source), [], "{source:?}");
        }
        for source in [
            "let path = \"x\";",
            "struct Track {\n    path: String,\n}",
            "fn f<'path>(x: &'path str) -> &'path str {\n    x\n}",
        ] {
            assert_eq!(found_in(source), [named("path")], "{source:?}");
        }
    }

    /// A line of the manifest that holds `target` anywhere is refused: a
    /// table for one target, as a header or a dotted key, however it is
    /// spaced or quoted, and the word in a comment or a value too.
    #[test]
    fn a_manifest_line_that_holds_target_fails() {
        let manifest = [
            "[package]",
            "name = \"gunmetal-wasm\"",
            "",
            "[dependencies]",
            "gunmetal-core = { path = \"../gunmetal-core\", version = \"0.0.0\" }",
            // Lines 6 to 11: a table for one target, written every way.
            "[target.'cfg(target_arch = \"wasm32\")'.dependencies]",
            "[target.wasm32-unknown-unknown.dependencies]",
            "[ target . wasm32-unknown-unknown . dependencies ]",
            "[target]",
            "target.wasm32-unknown-unknown.dependencies.gated = \"1\"",
            "\"target\".\"wasm32-unknown-unknown\".dependencies.gated = \"1\"",
            // Lines 12 and 13: the word in a comment and in a value.
            "# For one target only.",
            "gated = { version = \"1\", package = \"gated-target\" }",
            "[lib]",
            "crate-type = [\"cdylib\", \"rlib\"]",
        ]
        .join("\n");
        assert_eq!(
            check(&Memory::default().with(MANIFEST, &manifest)),
            [6, 7, 8, 9, 10, 11, 12, 13].map(target)
        );
    }

    /// What `xtask facade-wrappers` prints for each new refusal, to the
    /// character: where it is, and the name or the line.
    #[test]
    fn each_refusal_is_printed_with_its_place() {
        let tree = Memory::default()
            .with(
                MANIFEST,
                "[package]\nname = \"gunmetal-wasm\"\n\n[target.wasm32-unknown-unknown.dependencies]\n",
            )
            .with(
                LINKS,
                "include!(\"a.inc\");\n#[path = \"b.inc\"]\nmod b;\n#[wasm_bindgen]\npub fn c() {}\n",
            );
        let printed: Vec<String> = check(&tree)
            .iter()
            .map(|finding| format!("{finding:?}"))
            .collect();
        assert_eq!(
            printed,
            [
                r#"Target { path: "crates/gunmetal-wasm/Cargo.toml", line: 4 }"#,
                r#"Names { path: "crates/gunmetal-wasm/src/links.rs", name: "include" }"#,
                r#"Names { path: "crates/gunmetal-wasm/src/links.rs", name: "path" }"#,
                r#"Names { path: "crates/gunmetal-wasm/src/links.rs", name: "wasm_bindgen" }"#,
            ]
        );
    }

    /// What the module's documentation says the check does not see passes
    /// it, for each limit a facade held in memory can show: an item a macro
    /// of another crate writes, a value read from the build's environment, a
    /// manifest key spelt with TOML's escapes, a `path` key that moves the
    /// library out of the facade, and a `build` key that names a build
    /// script. That no other crate is read is shown by
    /// `only_the_manifest_and_the_rust_sources_of_the_facade_are_read`. The
    /// last two limits, a directory behind a symbolic link and what a
    /// wrapper does when it runs, have no test.
    #[test]
    fn what_the_documentation_says_the_check_does_not_see_passes() {
        let tree = Memory::default()
            .with(
                MANIFEST,
                "[package]\nbuild = \"gen.rs\"\n\n[lib]\npath = \"../elsewhere/lib.rs\"\n\n\
                 [\"\\u0074arget\".\"wasm32-unknown-unknown\".dependencies]\ngated = \"1\"\n",
            )
            .with(
                LINKS,
                "other::gated! {\n    pub fn decide() -> bool {\n        true\n    }\n}\n\
                 const BUILT_FOR: &str = env!(\"BUILT_FOR\");\n\
                 const BUILT_BY: Option<&str> = option_env!(\"BUILT_BY\");\n",
            );
        assert_eq!(check(&tree), []);
    }

    /// A facade whose files are listed and cannot be read.
    struct Unreadable;

    impl Tree for Unreadable {
        fn read(&self, _: &str) -> Option<String> {
            None
        }

        /// The manifest, the macro's source, one other source and a file
        /// that is not read, in the facade directory only.
        fn files(&self, directory: &str) -> Vec<String> {
            [
                "Cargo.toml",
                "src/export.rs",
                "src/links.rs",
                "types/links.d.ts",
            ]
            .into_iter()
            .filter(|_| directory == FACADE)
            .map(str::to_owned)
            .collect()
        }
    }

    #[test]
    fn an_unreadable_facade_manifest_or_source_fails() {
        assert_eq!(
            check(&Unreadable),
            [
                Finding::Unreadable {
                    path: MANIFEST.to_owned(),
                },
                Finding::Unreadable {
                    path: MACRO.to_owned(),
                },
                Finding::Unreadable {
                    path: LINKS.to_owned(),
                },
            ]
        );
    }
}
