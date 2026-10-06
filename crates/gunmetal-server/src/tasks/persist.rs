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
        Err(error) => on_insert_error(tx, kind, principal, path, error),
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
        Some(Value::Integer(id)) => {
            let id = u64::try_from(*id).map_err(|_| StoreError::Catalogue)?;
            if id == 0 {
                return Err(StoreError::Catalogue);
            }
            Ok(TaskId(id))
        }
        _ => Err(StoreError::Catalogue),
    }
}

fn on_insert_error(
    tx: &Transaction<'_>,
    kind: TaskKind,
    principal: &str,
    path: &str,
    error: DbError,
) -> Result<TaskId, StoreError> {
    if constraint(&error) {
        find_open(tx, kind, principal, path)?.ok_or(StoreError::Catalogue)
    } else {
        Err(StoreError::Db(error))
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
        Value::Integer(id) => {
            let id = u64::try_from(*id).map_err(|_| StoreError::Catalogue)?;
            if id == 0 {
                return Err(StoreError::Catalogue);
            }
            TaskId(id)
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
        Value::Integer(value) => {
            let progress = u8::try_from(*value).map_err(|_| StoreError::Catalogue)?;
            if progress > 100 {
                return Err(StoreError::Catalogue);
            }
            progress
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

#[cfg(test)]
mod persist_tests {
    use super::{
        TaskKind, TaskStatus, constraint, find_open, insert_or_join, on_insert_error, parse_id,
        parse_row,
    };
    use crate::tasks::runner::TaskId;
    use crate::tasks::schema::SCHEMA;
    use crate::tasks::wait;
    use gunmetal_core::id::{IdKind, PublicId};
    use gunmetal_core::time::Timestamp;
    use gunmetal_fs::dataroot::{DataRoot, Policy};
    use gunmetal_fs::host::HostFacts;
    use gunmetal_fs::sqlite::{DbError, Row, Value};
    use gunmetal_store::store::{Generation, Store, StoreError};
    use gunmetal_testkit::tempdir::TempDir;
    use std::sync::Arc;

    const NOON: i64 = 1_791_028_800_000;
    const FIRST: Generation = Generation([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);

    fn user() -> String {
        PublicId::parse("usr_00000000000000000000000001", IdKind::User)
            .expect("id")
            .to_string()
    }

    fn now() -> Timestamp {
        Timestamp::from_millis(NOON).expect("noon")
    }

    fn store() -> (TempDir, Arc<Store>) {
        let dir = TempDir::new("persist-tasks").expect("scratch");
        let host = HostFacts::probe(dir.path()).expect("probe");
        let root = DataRoot::open(dir.path(), &host, Policy::DEFAULT)
            .expect("root")
            .root;
        let opened = Store::open(&root, &[SCHEMA], FIRST).expect("store");
        (dir, Arc::new(opened.store))
    }

    fn valid_cols() -> Vec<Value> {
        vec![
            Value::Integer(1),
            Value::Text("backup".to_owned()),
            Value::Text(user()),
            Value::Text(String::new()),
            Value::Text("waiting".to_owned()),
            Value::Integer(0),
            Value::Null,
            Value::Null,
            Value::Integer(NOON),
            Value::Null,
            Value::Null,
            Value::Null,
        ]
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "each column of parse_row has a typed case"
    )]
    fn parse_row_reads_a_waiting_job_and_rejects_bad_columns() {
        let snap = parse_row(&Row(valid_cols())).expect("valid");
        assert_eq!(snap.id, TaskId(1));
        assert_eq!(snap.kind, TaskKind::Backup);
        assert_eq!(snap.path, None);
        assert_eq!(snap.status, TaskStatus::Waiting);
        assert_eq!(snap.progress, 0);
        assert_eq!(snap.checkpoint, None);
        assert_eq!(snap.error, None);
        assert_eq!(snap.requested_at, now());
        assert_eq!(snap.started_at, None);
        assert_eq!(snap.finished_at, None);
        assert_eq!(snap.duration_ms, None);

        let mut path = valid_cols();
        path[3] = Value::Text("/music".to_owned());
        assert_eq!(
            parse_row(&Row(path)).expect("path").path.as_deref(),
            Some("/music")
        );

        let mut progress = valid_cols();
        progress[5] = Value::Integer(100);
        assert_eq!(parse_row(&Row(progress)).expect("100").progress, 100);

        let mut over = valid_cols();
        over[5] = Value::Integer(101);
        assert_eq!(parse_row(&Row(over)).err(), Some(StoreError::Catalogue));

        let mut zero = valid_cols();
        zero[0] = Value::Integer(0);
        assert_eq!(parse_row(&Row(zero)).err(), Some(StoreError::Catalogue));

        let mut negative = valid_cols();
        negative[0] = Value::Integer(-1);
        assert_eq!(parse_row(&Row(negative)).err(), Some(StoreError::Catalogue));

        assert_eq!(
            parse_row(&Row(valid_cols()[..11].to_vec())).err(),
            Some(StoreError::Catalogue)
        );
        assert_eq!(
            parse_id(&Row(vec![Value::Text("1".to_owned())])).err(),
            Some(StoreError::Catalogue)
        );
        assert_eq!(
            parse_id(&Row(vec![Value::Integer(0)])).err(),
            Some(StoreError::Catalogue)
        );
        assert_eq!(
            parse_id(&Row(vec![Value::Integer(2)])).expect("id"),
            TaskId(2)
        );

        let mut kind = valid_cols();
        kind[1] = Value::Text("analysis".to_owned());
        assert_eq!(parse_row(&Row(kind)).err(), Some(StoreError::Catalogue));
        let mut kind_ty = valid_cols();
        kind_ty[1] = Value::Integer(1);
        assert_eq!(parse_row(&Row(kind_ty)).err(), Some(StoreError::Catalogue));

        let mut principal = valid_cols();
        principal[2] = Value::Text("nope".to_owned());
        assert_eq!(
            parse_row(&Row(principal)).err(),
            Some(StoreError::Catalogue)
        );
        let mut principal_ty = valid_cols();
        principal_ty[2] = Value::Integer(1);
        assert_eq!(
            parse_row(&Row(principal_ty)).err(),
            Some(StoreError::Catalogue)
        );

        let mut path_ty = valid_cols();
        path_ty[3] = Value::Integer(1);
        assert_eq!(parse_row(&Row(path_ty)).err(), Some(StoreError::Catalogue));

        let mut status = valid_cols();
        status[4] = Value::Text("checkpointed".to_owned());
        assert_eq!(parse_row(&Row(status)).err(), Some(StoreError::Catalogue));
        let mut status_ty = valid_cols();
        status_ty[4] = Value::Integer(1);
        assert_eq!(
            parse_row(&Row(status_ty)).err(),
            Some(StoreError::Catalogue)
        );

        let mut progress_ty = valid_cols();
        progress_ty[5] = Value::Text("0".to_owned());
        assert_eq!(
            parse_row(&Row(progress_ty)).err(),
            Some(StoreError::Catalogue)
        );
        let mut progress_neg = valid_cols();
        progress_neg[5] = Value::Integer(-1);
        assert_eq!(
            parse_row(&Row(progress_neg)).err(),
            Some(StoreError::Catalogue)
        );

        let mut check = valid_cols();
        check[6] = Value::Blob(vec![1, 2]);
        assert_eq!(
            parse_row(&Row(check)).expect("blob").checkpoint.as_deref(),
            Some([1, 2].as_slice())
        );
        let mut check_ty = valid_cols();
        check_ty[6] = Value::Text("x".to_owned());
        assert_eq!(parse_row(&Row(check_ty)).err(), Some(StoreError::Catalogue));

        let mut err = valid_cols();
        err[7] = Value::Text("no space".to_owned());
        assert_eq!(
            parse_row(&Row(err)).expect("err").error.as_deref(),
            Some("no space")
        );
        let mut err_ty = valid_cols();
        err_ty[7] = Value::Integer(1);
        assert_eq!(parse_row(&Row(err_ty)).err(), Some(StoreError::Catalogue));

        let mut req = valid_cols();
        req[8] = Value::Text("now".to_owned());
        assert_eq!(parse_row(&Row(req)).err(), Some(StoreError::Catalogue));

        let mut started = valid_cols();
        started[9] = Value::Integer(NOON);
        assert_eq!(
            parse_row(&Row(started)).expect("started").started_at,
            Some(now())
        );
        let mut fin = valid_cols();
        fin[10] = Value::Integer(NOON);
        assert_eq!(
            parse_row(&Row(fin)).expect("finished").finished_at,
            Some(now())
        );
        let mut dur = valid_cols();
        dur[11] = Value::Integer(12);
        assert_eq!(parse_row(&Row(dur)).expect("dur").duration_ms, Some(12));
        let mut dur_ty = valid_cols();
        dur_ty[11] = Value::Text("12".to_owned());
        assert_eq!(parse_row(&Row(dur_ty)).err(), Some(StoreError::Catalogue));

        let mut id_ty = valid_cols();
        id_ty[0] = Value::Text("1".to_owned());
        assert_eq!(parse_row(&Row(id_ty)).err(), Some(StoreError::Catalogue));
    }

    #[test]
    fn a_constraint_on_insert_joins_the_open_job_and_another_error_does_not() {
        let (_dir, store) = store();
        let principal = user();
        let id = wait::wait(store.write({
            let principal = principal.clone();
            move |tx| insert_or_join(tx, TaskKind::Backup, &principal, "", now())
        }))
        .expect("insert");
        let joined = wait::wait(store.write({
            let principal = principal.clone();
            move |tx| {
                on_insert_error(
                    tx,
                    TaskKind::Backup,
                    &principal,
                    "",
                    DbError::Sqlite {
                        code: super::SQLITE_CONSTRAINT,
                    },
                )
            }
        }))
        .expect("join");
        assert_eq!(joined, id);
        let busy = wait::wait(store.write({
            let principal = principal.clone();
            move |tx| {
                on_insert_error(
                    tx,
                    TaskKind::Backup,
                    &principal,
                    "",
                    DbError::Sqlite { code: 5 },
                )
            }
        }));
        assert_eq!(busy, Err(StoreError::Db(DbError::Sqlite { code: 5 })));
        let missing = wait::wait(store.write(|tx| {
            on_insert_error(
                tx,
                TaskKind::Purge,
                "usr_00000000000000000000000002",
                "",
                DbError::Sqlite {
                    code: super::SQLITE_CONSTRAINT,
                },
            )
        }));
        assert_eq!(missing, Err(StoreError::Catalogue));
        assert!(constraint(&DbError::Sqlite {
            code: super::SQLITE_CONSTRAINT | (1 << 8)
        }));
        assert!(!constraint(&DbError::Sqlite { code: 5 }));
        assert!(!constraint(&DbError::MultipleStatements));
        let found = wait::wait(store.write({
            let principal = principal.clone();
            move |tx| find_open(tx, TaskKind::Backup, &principal, "")
        }))
        .expect("find");
        assert_eq!(found, Some(id));
        let none = wait::wait(
            store.write(|tx| find_open(tx, TaskKind::Purge, "usr_00000000000000000000000002", "")),
        )
        .expect("none");
        assert_eq!(none, None);
    }
}
