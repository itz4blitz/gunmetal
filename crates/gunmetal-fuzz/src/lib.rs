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

pub mod aiff;
pub mod ape;
pub mod base64;
pub mod detect;
pub mod ebml;
pub mod flac_frames;
pub mod flac_metadata;
pub mod flac_structure;
pub mod http_forwarded;
pub mod http_range;
pub mod id;
pub mod id3_structure;
pub mod id3v1;
pub mod id3v2_tag;
pub mod inflate;
pub mod link;
pub mod logframe;
pub mod lyrics;
pub mod mp4_probe;
pub mod mp4_sample_table;
pub mod mp4_structure;
pub mod mpa;
pub mod net;
pub mod ogg;
pub mod ogg_structure;
pub mod opus;
pub mod otp;
pub mod path;
pub mod registry;
pub mod riff;
pub mod search_segment;
pub mod tags_riff;
pub mod text;
pub mod time;
pub mod token;
pub mod userdata_codec;
pub mod values;
pub mod vorbis;
pub mod vorbis_comment;
pub mod webauthn_attestation;
pub mod webauthn_authdata;
pub mod webauthn_cbor;
pub mod webauthn_cose;
pub mod wire;
