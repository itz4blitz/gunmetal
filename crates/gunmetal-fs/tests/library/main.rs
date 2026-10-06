//! The library door on a real filesystem: root handles, opening, the link
//! policy, walking, fingerprints, the check before serving and the per-root
//! pool. Nothing is mocked: every test builds its library in a scratch
//! directory of its own, made by the testkit.
#![expect(
    clippy::disallowed_methods,
    reason = "tests of the filesystem door build and inspect hostile layouts on the real filesystem, by path (SEC-MED-033)"
)]

mod confinement;
mod fingerprints;
mod links;
mod opening;
mod pool;
mod read_only;
mod roots;
mod serving;
mod support;
mod walking;
