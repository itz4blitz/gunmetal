//! Every registered harness. A registry file: a package that adds a harness
//! adds one line to the list at the end of this file, in sorted position,
//! and changes nothing else.
//!
//! The registry is how stable `cargo test` finds every harness's seeds
//! (SEC-MED-028) and how `xtask check-harnesses` knows which parser modules
//! have a harness (SEC-MED-027). The list holds module names, and each
//! entry's name and function come from that one token, so an entry cannot
//! carry one harness's name and run another's code. The name is also the
//! harness's cargo-fuzz target in `fuzz/Cargo.toml`, its target file
//! `fuzz/fuzz_targets/<name>.rs`, its seed directory `fuzz/seeds/<name>/`
//! and its exact-outcome replay test `tests/<name>_corpus.rs`; the check
//! fails when any of them is missing, when the target file does not call
//! `gunmetal_fuzz::<name>::run`, and when the target builds another file.

/// One fuzz harness: every entry point of one parser behind a plain
/// function of bytes (SEC-MED-027).
#[derive(Debug, Clone, Copy)]
pub struct Harness {
    /// The harness's name, made only of lowercase ASCII letters, digits and
    /// underscores. For a parser module it is the module's path below
    /// `gunmetal-core/src/` without a leading `formats/`, with `/` written
    /// as `_` (`formats/flac/metadata.rs` is `flac_metadata`). A
    /// structure-aware harness for a container format is named
    /// `<format>_structure` (SEC-MED-031).
    pub name: &'static str,
    /// Runs the harness on one input, discarding what the parser reported.
    /// It panics when the parser breaks an invariant.
    pub run: fn(&[u8]),
}

/// Every registered harness, sorted by name.
#[must_use]
pub fn all() -> &'static [Harness] {
    HARNESSES
}

/// Defines `HARNESSES` from the names of the harness modules: each entry is
/// named after its module and runs that module's `run`.
macro_rules! harnesses {
    ($($module:ident,)*) => {
        /// Every harness, sorted by name.
        static HARNESSES: &[Harness] = &[$(Harness {
            name: stringify!($module),
            run: |data| {
                let _ = crate::$module::run(data);
            },
        }),*];
    };
}

harnesses! {
    base64,
    ebml,
    id3_structure,
    id3v2_tag,
    link,
    net,
    text,
    time,
    values,
}
