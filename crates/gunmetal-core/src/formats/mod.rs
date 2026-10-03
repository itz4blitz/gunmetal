//! Parsers for the media and sidecar formats Gunmetal reads.
//!
//! Every parser here follows the parse contract in [`crate::parse`]: it is
//! sans-I/O, runs under [`Limits`](crate::parse::Limits), a step
//! [`Budget`](crate::parse::Budget) and a nesting
//! [`Depth`](crate::parse::Depth), and returns `Ok` or a typed error for
//! every input. Containers return their raw tags and technical facts;
//! turning tags into the music model happens elsewhere.
//!
//! This file is a registry: it holds one `pub mod` line per format, sorted.

pub mod id3v2;
