//! Every registered harness. A registry file: a package that adds a harness
//! adds one entry here, in sorted position, and changes nothing else.
//!
//! The registry is how stable `cargo test` finds every harness's seeds
//! (SEC-MED-028) and how `xtask check-harnesses` knows which parser modules
//! have a harness (SEC-MED-027). The name is also the harness's module in
//! this crate, its cargo-fuzz target in `fuzz/Cargo.toml`, its target file
//! `fuzz/fuzz_targets/<name>.rs`, its seed directory `fuzz/seeds/<name>/`
//! and its exact-outcome replay test `tests/<name>_corpus.rs`; the check
//! fails when any of them is missing.

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

/// Every harness, sorted by name.
static HARNESSES: &[Harness] = &[Harness {
    name: "ebml",
    run: |data| {
        let _ = crate::ebml::run(data);
    },
}];

/// Every registered harness, sorted by name.
#[must_use]
pub fn all() -> &'static [Harness] {
    HARNESSES
}
