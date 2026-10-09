//! Plugin host decisions.
//!
//! This crate is the rules a plugin must pass before anything runs: a closed
//! manifest, exact network grants, call and quota limits, a component
//! package that is never unpacked, and an install that leaves the previous
//! version in place when a check fails (SEC-EXT-025 to SEC-EXT-044).
//!
//! It does not link a WebAssembly runtime, open a socket, or spawn a
//! process. SEC-EXT-018 stays in force until SEC-EXT-019 to SEC-EXT-034
//! pass, including an engine built from [`engine::POLICY`] and the transcode
//! worker's sandbox profile. The server does not depend on this crate.

pub mod channel;
pub mod consent;
pub mod engine;
pub mod grant;
pub mod id;
pub mod index;
pub mod lifecycle;
pub mod manifest;
pub mod package;
pub mod record;
pub mod runtime;
pub mod scope;
pub mod session;
pub mod version;
pub mod world;

#[cfg(test)]
mod tests {
    /// Verifies: SEC-EXT-018
    #[test]
    fn this_crate_does_not_name_a_runtime() {
        let manifest = include_str!("../Cargo.toml");
        for banned in [
            "wasmtime",
            "wasmer",
            "wasmi",
            "wasm3",
            "extism",
            "libloading",
        ] {
            assert!(
                !manifest.contains(banned),
                "{banned} must not be a dependency"
            );
        }
    }
}
