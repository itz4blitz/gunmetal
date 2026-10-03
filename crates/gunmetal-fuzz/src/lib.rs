//! Fuzz harnesses for the parsers in `gunmetal-core`.
//!
//! Each harness is a plain function (SEC-MED-027). The cargo-fuzz targets in
//! `fuzz/` call it on a nightly toolchain, and stable `cargo test` replays
//! the committed corpus in `fuzz/seeds/` through the same function
//! (SEC-MED-028), so a finding reproduces without nightly.
//!
//! A harness feeds its input to every entry point of one parser, asserts the
//! invariants that must hold for any input, and returns what the parser
//! reported so that the replay tests can check it exactly. A broken
//! invariant panics, which the fuzzer records as a crash.
//!
//! This file is a registry: it holds one `pub mod` line per harness module
//! and nothing else. Adding a harness takes one line here, one line in
//! [`registry`]'s list, one `[[bin]]` block in `fuzz/Cargo.toml`, and
//! files the harness's package owns: `src/<name>.rs`,
//! `tests/<name>_corpus.rs`, `fuzz/fuzz_targets/<name>.rs` and
//! `fuzz/seeds/<name>/`.

pub mod base64;
pub mod ebml;
pub mod http_forwarded;
pub mod http_range;
pub mod link;
pub mod net;
pub mod registry;
pub mod text;
pub mod time;
pub mod values;
