//! The task table in the cache.

use gunmetal_core::schema::{Column, DataClass, SchemaPart};

/// The task runner's schema part, which the server registers when it opens
/// the cache.
pub const SCHEMA: SchemaPart = SchemaPart {
    name: "tasks",
    sql: "CREATE TABLE tasks (\
              id INTEGER PRIMARY KEY, \
              kind TEXT NOT NULL CHECK (kind IN ('library_scan','path_refresh','backup','purge','rebuild')), \
              principal TEXT NOT NULL, \
              path TEXT NOT NULL, \
              status TEXT NOT NULL CHECK (status IN ('waiting','running','succeeded','failed','cancelled')), \
              progress INTEGER NOT NULL DEFAULT 0 CHECK (progress >= 0 AND progress <= 100), \
              checkpoint BLOB, \
              error TEXT, \
              requested_at INTEGER NOT NULL, \
              started_at INTEGER, \
              finished_at INTEGER, \
              duration_ms INTEGER \
          ) STRICT; \
          CREATE UNIQUE INDEX tasks_open ON tasks (kind, principal, path) \
          WHERE status IN ('waiting', 'running');",
    columns: &[
        Column {
            table: "tasks",
            name: "id",
            class: DataClass::Activity,
        },
        Column {
            table: "tasks",
            name: "kind",
            class: DataClass::Activity,
        },
        Column {
            table: "tasks",
            name: "principal",
            class: DataClass::Identity,
        },
        Column {
            table: "tasks",
            name: "path",
            class: DataClass::Library,
        },
        Column {
            table: "tasks",
            name: "status",
            class: DataClass::Activity,
        },
        Column {
            table: "tasks",
            name: "progress",
            class: DataClass::Activity,
        },
        Column {
            table: "tasks",
            name: "checkpoint",
            class: DataClass::Activity,
        },
        Column {
            table: "tasks",
            name: "error",
            class: DataClass::Activity,
        },
        Column {
            table: "tasks",
            name: "requested_at",
            class: DataClass::Activity,
        },
        Column {
            table: "tasks",
            name: "started_at",
            class: DataClass::Activity,
        },
        Column {
            table: "tasks",
            name: "finished_at",
            class: DataClass::Activity,
        },
        Column {
            table: "tasks",
            name: "duration_ms",
            class: DataClass::Activity,
        },
    ],
};
