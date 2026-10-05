//! User events: what a person or the household authored, as the user log
//! holds it (ADR 3, sections 4 and 5).
//!
//! An [`event::Event`] is the envelope: a random event ID, a hybrid logical
//! clock ([`hlc::Hlc`]), the device that authored it, the stream it belongs
//! to and a typed body. [`codec`] writes and reads the envelope's bytes,
//! keeping body types this version does not know whole. [`merge`] holds the
//! conflict rules as pure functions over event sets, and [`erasure`] the
//! selectors that remove history from them.
//!
//! This file is a registry: it holds only module lines.

pub mod codec;
pub mod compose;
pub mod erasure;
pub mod event;
pub mod hlc;
pub mod merge;
pub mod sink;
#[cfg(test)]
mod strategies;
