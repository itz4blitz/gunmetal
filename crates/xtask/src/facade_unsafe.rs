//! `xtask facade-unsafe`: hand-written facade sources never use `unsafe`.
//!
//! The facade's manifest keeps the workspace's `unsafe_code = "forbid"`,
//! and its `tests/workspace_rules.rs` refuses any lower level, so the
//! compiler already refuses `unsafe` written in the facade, for the host and
//! for `wasm32`. This check refuses it in the text as well, so that lowering
//! the lint in the manifest would not be enough to let a hand-written
//! `unsafe` block in. It reads every Rust source under the facade path and
//! refuses the keyword outside comments, strings and character literals. It
//! supports SEC-MED-077, whose parsers and limits the browser runs through
//! this crate, and does not prove it.

use crate::native_code::uses_unsafe;
use crate::tree::{Tree, is_rust};

/// The facade directory relative to the repository root.
pub const FACADE: &str = "crates/gunmetal-wasm";

/// Something `facade-unsafe` found wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finding {
    /// A facade Rust source could not be read as text, so its safety cannot
    /// be established.
    Unreadable {
        /// Its path relative to the repository root.
        path: String,
    },
    /// A hand-written facade Rust source uses the `unsafe` keyword.
    Unsafe {
        /// Its path relative to the repository root.
        path: String,
    },
}

/// Finds every unreadable source and every hand-written `unsafe` keyword in
/// the WebAssembly facade.
pub fn check(tree: &dyn Tree) -> Vec<Finding> {
    tree.files(FACADE)
        .into_iter()
        .filter(|path| is_rust(path))
        .filter_map(|path| {
            let path = format!("{FACADE}/{path}");
            match tree.read(&path) {
                None => Some(Finding::Unreadable { path }),
                Some(source) if uses_unsafe(&source) => Some(Finding::Unsafe { path }),
                Some(_) => None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{FACADE, Finding, check};
    use crate::tree::Tree;
    use crate::tree::memory::Memory;

    /// Supports: SEC-MED-077
    #[test]
    fn refuses_hand_written_unsafe_in_each_facade_source_file() {
        let tree = Memory::default()
            .with("crates/gunmetal-wasm/src/lib.rs", "pub fn safe() {}\n")
            .with(
                "crates/gunmetal-wasm/src/generated.rs",
                "pub fn raw() { unsafe {} }\n",
            )
            .with(
                "crates/gunmetal-wasm-extra/src/lib.rs",
                "pub fn outside() { unsafe {} }\n",
            );
        assert_eq!(
            check(&tree),
            [Finding::Unsafe {
                path: "crates/gunmetal-wasm/src/generated.rs".to_owned(),
            }]
        );
    }

    /// Supports: SEC-MED-077
    #[test]
    fn allows_mentions_of_unsafe_outside_rust_code() {
        let tree = Memory::default().with(
            "crates/gunmetal-wasm/src/lib.rs",
            "// unsafe is refused\nconst NOTE: &str = \"unsafe\";\nconst MARK: char = 'u';\n",
        );
        assert_eq!(check(&tree), []);
    }

    struct Unreadable;

    impl Tree for Unreadable {
        fn read(&self, _: &str) -> Option<String> {
            None
        }

        /// One file, in the facade directory only.
        fn files(&self, directory: &str) -> Vec<String> {
            std::iter::once("src/lib.rs".to_owned())
                .filter(|_| directory == FACADE)
                .collect()
        }
    }

    /// Supports: SEC-MED-077
    #[test]
    fn refuses_an_unreadable_facade_source() {
        assert_eq!(
            check(&Unreadable),
            [Finding::Unreadable {
                path: "crates/gunmetal-wasm/src/lib.rs".to_owned(),
            }]
        );
    }
}
