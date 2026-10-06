//! The session layer's tests, on a real identity store in a temporary data
//! directory and through the real request pipeline.
//!
//! This file is a registry: it holds only module lines.

mod http;
mod issue;
mod lifetimes;
mod requests;
mod revocation;
mod storage;
mod world;
