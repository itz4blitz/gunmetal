//! The web client's WebAssembly facade over Gunmetal's core.
//!
//! The core cannot carry `wasm-bindgen`'s attributes (record 6), so each
//! core type that crosses to TypeScript has a mirror type here, with a
//! conversion that takes the core value apart field by field and no
//! catch-all: a core field renamed, retyped, added or removed stops this
//! crate compiling (record 12, decision 13). Every exported function is a
//! direct call into the core with conversion only, and no rule is written
//! here but one bound: the link filter reads no URL longer than the core's
//! long-text limit (`links.rs`).
//!
//! A mirror value crosses to JavaScript through `tsify::Ts`, and that
//! conversion runs only inside a JavaScript host. So each export is two
//! functions: a plain one, tested on the host like all other code, and a
//! wrapper compiled only for `wasm32`, which converts the types and calls
//! it (record 12, the addition of 2026-10-05). One macro writes every
//! wrapper; `export.rs` holds it and says how the exception it makes to the
//! coverage and mutation rules is kept small.
//!
//! This file is a registry: it holds only module lines, sorted.

mod export;
pub mod links;
pub mod text;
