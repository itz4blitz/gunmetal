//! The sign-in limiter: how often a source may try a credential, and how
//! long a wrong guess at a short secret makes it wait.
//!
//! The arithmetic is the core's (WP-032); this module gives it the server's
//! ceilings and a place to keep its state. It has two parts, and the
//! credential verifier (`crate::verifier`) is the one caller of both, so
//! every sign-in pathway is limited the same way (SEC-TM-014, SEC-HIS-046).
//!
//! - [`rates`] holds the per-source and server-wide ceilings every attempt
//!   passes first, on any pathway: the core's keyed GCRA under the keys it
//!   derives from the request's `ClientContext`, with the server-wide count
//!   under its global key (SEC-IAM-101). The ceilings are the ones
//!   registered in `limits.toml`, and each per-source ceiling is held below
//!   the server-wide one, so no one source can use that up (SEC-IAM-008).
//! - [`delay`] decides when a source that guessed a short secret wrong may
//!   guess again, on the schedule of SEC-API-056: 30 seconds, 1 minute,
//!   5 minutes, then 15 minutes, never longer and never for good. The
//!   verifier keeps the count it decides on in the identity store, so a
//!   restart forgives nothing.
//!
//! The only address either part takes is the `ClientContext` the listener
//! resolved; neither reads a socket peer or a header (SEC-OPS-037).
//!
//! This file is a registry: it holds only `mod` lines.

pub mod delay;
pub mod rates;
#[cfg(test)]
pub(crate) mod testing;
