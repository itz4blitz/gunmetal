//! Statements against the task table. Every statement is static text with
//! bound values (SEC-API-066, SEC-TM-039, SEC-HIS-038).

use gunmetal_core::id::{IdKind, PublicId};
use gunmetal_core::time::Timestamp;
use gunmetal_fs::sqlite::{DbError, Query, Row, Value};
use gunmetal_store::readers::Reader;
use gunmetal_store::store::StoreError;
use gunmetal_store::writer::Transaction;

use super::error::TaskStatus;
use super::kind::TaskKind;
use super::runner::{TaskId, TaskSnapshot};

const SQLITE_CONSTRAINT: i32 = 19;

const FIND_OPEN: Query = Query::new(
    "SELECT id FROM tasks \
     WHERE kind = ?1 AND principal = ?2 AND path = ?3 \
     AND status IN ('waiting', 'running') LIMIT 1",
);

const INSERT: Query = Query::new(
    "INSERT INTO tasks (kind, principal, path, status, progress, requested_at) \
     VALUES (?1, ?2, ?3, 'waiting', 0, ?4)",
);

const LAST_ID: Query = Query::new("SELECT last_insert_rowid()");

const REQUEUE: Query = Query::new("UPDATE tasks SET status = 'waiting' WHERE status = 'running'");

const WAITING: Query = Query::new(
    "SELECT id, kind, principal, path, status, progress, checkpoint, error, \
     requested_at, started_at, finished_at, duration_ms \
     FROM tasks WHERE status = 'waiting' ORDER BY id",
);

const BY_ID: Query = Query::new(
    "SELECT id, kind, principal, path, status, progress, checkpoint, error, \
     requested_at, started_at, finished_at, duration_ms \
     FROM tasks WHERE id = ?1",
);

const ALL: Query = Query::new(
    "SELECT id, kind, principal, path, status, progress, checkpoint, error, \
     requested_at, started_at, finished_at, duration_ms \
     FROM tasks ORDER BY id",
);

const MARK_RUNNING: Query = Query::new(
    "UPDATE tasks SET status = 'running', started_at = ?1, error = NULL \
     WHERE id = ?2 AND status = 'waiting'",
);

const CHECKPOINT: Query = Query::new(
    "UPDATE tasks SET checkpoint = ?1, progress = ?2 WHERE id = ?3 AND status = 'running'",
);

const PROGRESS: Query =
    Query::new("UPDATE tasks SET progress = ?1 WHERE id = ?2 AND status = 'running'");

const FINISH: Query = Query::new(
    "UPDATE tasks SET status = ?1, progress = ?2, error = ?3, finished_at = ?4, duration_ms = ?5 \
     WHERE id = ?6 AND status = 'running'",
);

const CANCEL_WAITING: Query = Query::new(
    "UPDATE tasks SET status = 'cancelled', finished_at = ?1 WHERE id = ?2 AND status = 'waiting'",
);

fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}

fn int(value: i64) -> Value {
    Value::Integer(value)
}

fn blob(bytes: &[u8]) -> Value {
    Value::Blob(bytes.to_vec())
}

fn opt_text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, text)
}

fn opt_int(value: Option<i64>) -> Value {
    value.map_or(Value::Null, int)
}

fn constraint(error: &DbError) -> bool {
    matches!(error, DbError::Sqlite { code } if *code & 0xff == SQLITE_CONSTRAINT)
}

/// The identifier of an open job for this kind, principal and path, if any.
pub(crate) fn find_open(
    tx: &Transaction<'_>,
    kind: TaskKind,
    principal: &str,
    path: &str,
) -> Result<Option<TaskId>, StoreError> {
    let rows = tx.query(
        &FIND_OPEN
            .bind(text(kind.as_str()))
            .bind(text(principal))
            .bind(text(path)),
    )?;
    match rows.as_slice() {
        [] => Ok(None),
        [row, ..] => parse_id(row).map(Some),
    }
}

/// Queues a job, or returns the identifier of the open one it joins.
pub(crate) fn insert_or_join(
    tx: &Transaction<'_>,
    kind: TaskKind,
    principal: &str,
    path: &str,
    now: Timestamp,
) -> Result<TaskId, StoreError> {
    if let Some(id) = find_open(tx, kind, principal, path)? {
        return Ok(id);
    }
    match tx.execute(
        &INSERT
            .bind(text(kind.as_str()))
            .bind(text(principal))
            .bind(text(path))
            .bind(int(now.millis())),
    ) {
        Ok(_) => last_id(tx),
        Err(error) if constraint(&error) => {
            find_open(tx, kind, principal, path)?.ok_or(StoreError::Catalogue)
        }
        Err(error) => Err(StoreError::Db(error)),
    }
}

fn last_id(tx: &Transaction<'_>) -> Result<TaskId, StoreError> {
    let rows = tx.query(&LAST_ID)?;
    match rows.as_slice() {
        [row] => parse_id(row),
        _ => Err(StoreError::Catalogue),
    }
}

fn parse_id(row: &Row) -> Result<TaskId, StoreError> {
    match row.0.first() {
        Some(Value::Integer(id)) if *id > 0 => u64::try_from(*id)
            .map(TaskId)
            .map_err(|_| StoreError::Catalogue),
        _ => Err(StoreError::Catalogue),
    }
}

/// Running jobs become waiting so a restart resumes them from the checkpoint.
pub(crate) fn requeue_running(tx: &Transaction<'_>) -> Result<(), StoreError> {
    tx.execute(&REQUEUE)?;
    Ok(())
}

/// Waiting jobs, oldest first.
pub(crate) fn waiting(reader: &Reader<'_>) -> Result<Vec<TaskSnapshot>, StoreError> {
    parse_rows(&reader.query(&WAITING)?)
}

/// One job by identifier.
pub(crate) fn by_id(reader: &Reader<'_>, id: TaskId) -> Result<Option<TaskSnapshot>, StoreError> {
    let rows = reader.query(&BY_ID.bind(int(id.as_i64())))?;
    match rows.as_slice() {
        [] => Ok(None),
        [row, ..] => parse_row(row).map(Some),
    }
}

/// Every job, oldest first.
pub(crate) fn all(reader: &Reader<'_>) -> Result<Vec<TaskSnapshot>, StoreError> {
    parse_rows(&reader.query(&ALL)?)
}

/// Marks a waiting job running. Returns whether it still was waiting.
pub(crate) fn mark_running(
    tx: &Transaction<'_>,
    id: TaskId,
    now: Timestamp,
) -> Result<bool, StoreError> {
    let changed = tx.execute(&MARK_RUNNING.bind(int(now.millis())).bind(int(id.as_i64())))?;
    Ok(changed == 1)
}

/// Stores a checkpoint blob and progress on a running job.
pub(crate) fn checkpoint(
    tx: &Transaction<'_>,
    id: TaskId,
    blob_bytes: &[u8],
    progress: u8,
) -> Result<(), StoreError> {
    tx.execute(
        &CHECKPOINT
            .bind(blob(blob_bytes))
            .bind(int(i64::from(progress)))
            .bind(int(id.as_i64())),
    )?;
    Ok(())
}

/// Stores progress on a running job without touching the checkpoint.
pub(crate) fn set_progress(
    tx: &Transaction<'_>,
    id: TaskId,
    progress: u8,
) -> Result<(), StoreError> {
    tx.execute(
        &PROGRESS
            .bind(int(i64::from(progress)))
            .bind(int(id.as_i64())),
    )?;
    Ok(())
}

/// Records the end of a run that was running.
pub(crate) fn finish(
    tx: &Transaction<'_>,
    id: TaskId,
    status: TaskStatus,
    progress: u8,
    error: Option<&str>,
    now: Timestamp,
    started_at: Option<Timestamp>,
) -> Result<(), StoreError> {
    let duration = started_at.map(|started| now.millis().saturating_sub(started.millis()));
    tx.execute(
        &FINISH
            .bind(text(status.as_str()))
            .bind(int(i64::from(progress)))
            .bind(opt_text(error))
            .bind(int(now.millis()))
            .bind(opt_int(duration))
            .bind(int(id.as_i64())),
    )?;
    Ok(())
}

/// Cancels a job that has not started. Returns whether it was waiting.
pub(crate) fn cancel_waiting(
    tx: &Transaction<'_>,
    id: TaskId,
    now: Timestamp,
) -> Result<bool, StoreError> {
    let changed = tx.execute(
        &CANCEL_WAITING
            .bind(int(now.millis()))
            .bind(int(id.as_i64())),
    )?;
    Ok(changed == 1)
}

fn parse_rows(rows: &[Row]) -> Result<Vec<TaskSnapshot>, StoreError> {
    rows.iter().map(parse_row).collect()
}

fn parse_row(row: &Row) -> Result<TaskSnapshot, StoreError> {
    let cols = &row.0;
    if cols.len() != 12 {
        return Err(StoreError::Catalogue);
    }
    let id = match &cols[0] {
        Value::Integer(id) if *id > 0 => {
            TaskId(u64::try_from(*id).map_err(|_| StoreError::Catalogue)?)
        }
        _ => return Err(StoreError::Catalogue),
    };
    let kind = match &cols[1] {
        Value::Text(text) => TaskKind::parse(text).ok_or(StoreError::Catalogue)?,
        _ => return Err(StoreError::Catalogue),
    };
    let principal = match &cols[2] {
        Value::Text(text) => {
            PublicId::parse(text, IdKind::User).map_err(|_| StoreError::Catalogue)?
        }
        _ => return Err(StoreError::Catalogue),
    };
    let path = match &cols[3] {
        Value::Text(text) if text.is_empty() => None,
        Value::Text(text) => Some(text.clone()),
        _ => return Err(StoreError::Catalogue),
    };
    let status = match &cols[4] {
        Value::Text(text) => TaskStatus::parse(text).ok_or(StoreError::Catalogue)?,
        _ => return Err(StoreError::Catalogue),
    };
    let progress = match &cols[5] {
        Value::Integer(value) if (0..=100).contains(value) => {
            u8::try_from(*value).map_err(|_| StoreError::Catalogue)?
        }
        _ => return Err(StoreError::Catalogue),
    };
    let checkpoint = match &cols[6] {
        Value::Null => None,
        Value::Blob(bytes) => Some(bytes.clone()),
        _ => return Err(StoreError::Catalogue),
    };
    let error = match &cols[7] {
        Value::Null => None,
        Value::Text(text) => Some(text.clone()),
        _ => return Err(StoreError::Catalogue),
    };
    let requested_at = timestamp(&cols[8])?;
    let started_at = opt_timestamp(&cols[9])?;
    let finished_at = opt_timestamp(&cols[10])?;
    let duration_ms = match &cols[11] {
        Value::Null => None,
        Value::Integer(value) => Some(*value),
        _ => return Err(StoreError::Catalogue),
    };
    Ok(TaskSnapshot {
        id,
        kind,
        principal,
        path,
        status,
        progress,
        checkpoint,
        error,
        requested_at,
        started_at,
        finished_at,
        duration_ms,
    })
}

fn timestamp(value: &Value) -> Result<Timestamp, StoreError> {
    match value {
        Value::Integer(ms) => Timestamp::from_millis(*ms).map_err(|_| StoreError::Catalogue),
        _ => Err(StoreError::Catalogue),
    }
}

fn opt_timestamp(value: &Value) -> Result<Option<Timestamp>, StoreError> {
    match value {
        Value::Null => Ok(None),
        other => timestamp(other).map(Some),
    }
}

#[cfg(test)]
pub(crate) fn sqlite_constraint() -> i32 {
    SQLITE_CONSTRAINT
}
