//! Gunmetal's filesystem door.
//!
//! Every file the server keeps in its data directory is reached through the
//! [`dataroot::DataRoot`] handle, and every SQLite database is opened through
//! the one connection opener in [`sqlite`] (SEC-MED-033, SEC-HIS-016,
//! SEC-TM-039). Path-based `std::fs` and SQLite opens are allowed nowhere
//! else in the workspace.

pub mod dataroot;
pub mod fingerprint;
pub mod host;
pub mod open;
pub mod path;
pub mod pool;
pub mod root;
pub mod sqlite;
pub mod walk;
