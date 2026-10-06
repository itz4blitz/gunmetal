//! The session layer's tables in the identity store: enrolled devices and
//! the sessions they hold (ADR 7, section 8).
//!
//! A device row carries the device's class and its revocation epoch. A
//! session row carries the keyed hash of its token, never the token
//! (SEC-OPS-016), the kind of credential it is, which nothing can change
//! once the row is written (SEC-EXT-007), and the address it was issued to,
//! which goes when the row does (SEC-PRV-003).

use gunmetal_core::schema::{Column, DataClass, SchemaPart};

/// The devices and sessions tables.
pub const SESSIONS: SchemaPart = SchemaPart {
    name: "session",
    sql: "CREATE TABLE devices (\
              device TEXT PRIMARY KEY, \
              account TEXT NOT NULL, \
              class TEXT NOT NULL CHECK (class IN ('personal', 'limited')), \
              epoch INTEGER NOT NULL DEFAULT 0 CHECK (epoch >= 0), \
              enrolled INTEGER NOT NULL) WITHOUT ROWID; \
          CREATE INDEX devices_by_account ON devices (account); \
          CREATE TABLE sessions (\
              token_hash BLOB PRIMARY KEY, \
              handle BLOB NOT NULL UNIQUE, \
              kind TEXT NOT NULL, \
              lifetime TEXT NOT NULL CHECK (lifetime IN ('personal', 'shared')), \
              account TEXT NOT NULL, \
              device TEXT NOT NULL REFERENCES devices (device) ON DELETE CASCADE, \
              epoch INTEGER NOT NULL CHECK (epoch >= 0), \
              created INTEGER NOT NULL, \
              last_seen INTEGER NOT NULL, \
              address TEXT NOT NULL) WITHOUT ROWID; \
          CREATE INDEX sessions_by_account ON sessions (account); \
          CREATE INDEX sessions_by_device ON sessions (device); \
          CREATE TRIGGER sessions_kind_is_immutable BEFORE UPDATE OF kind ON sessions \
          BEGIN SELECT RAISE(ABORT, 'the kind of a credential never changes'); END;",
    columns: &[
        Column {
            table: "devices",
            name: "device",
            class: DataClass::Identity,
        },
        Column {
            table: "devices",
            name: "account",
            class: DataClass::Identity,
        },
        Column {
            table: "devices",
            name: "class",
            class: DataClass::Identity,
        },
        Column {
            table: "devices",
            name: "epoch",
            class: DataClass::Identity,
        },
        Column {
            table: "devices",
            name: "enrolled",
            class: DataClass::Identity,
        },
        Column {
            table: "sessions",
            name: "token_hash",
            class: DataClass::Secret,
        },
        Column {
            table: "sessions",
            name: "handle",
            class: DataClass::Secret,
        },
        Column {
            table: "sessions",
            name: "kind",
            class: DataClass::Identity,
        },
        Column {
            table: "sessions",
            name: "lifetime",
            class: DataClass::Identity,
        },
        Column {
            table: "sessions",
            name: "account",
            class: DataClass::Identity,
        },
        Column {
            table: "sessions",
            name: "device",
            class: DataClass::Identity,
        },
        Column {
            table: "sessions",
            name: "epoch",
            class: DataClass::Identity,
        },
        Column {
            table: "sessions",
            name: "created",
            class: DataClass::Identity,
        },
        Column {
            table: "sessions",
            name: "last_seen",
            class: DataClass::Identity,
        },
        Column {
            table: "sessions",
            name: "address",
            class: DataClass::Identity,
        },
    ],
};

/// Every schema part the session layer keeps, for whoever opens the
/// identity store.
pub const PARTS: [SchemaPart; 1] = [SESSIONS];
