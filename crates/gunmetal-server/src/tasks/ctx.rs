//! The context a handler sees: checkpoints, progress, cancel and resume.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use gunmetal_store::store::Store;

use super::error::TaskError;
use super::persist;
use super::runner::TaskId;
use super::wait;

/// What a running handler may do: persist a checkpoint, report progress,
/// and observe cancel. Cancel is honoured at the next checkpoint, not by
/// aborting the thread (WP-070).
pub struct TaskCtx {
    pub(crate) id: TaskId,
    pub(crate) store: Arc<Store>,
    pub(crate) cancel: Arc<AtomicBool>,
    pub(crate) stopping: Arc<AtomicBool>,
    pub(crate) loaded: Option<Vec<u8>>,
    pub(crate) progress: AtomicU8,
}

impl TaskCtx {
    /// The checkpoint restored after a restart, if the previous run wrote
    /// one.
    #[must_use]
    pub fn loaded(&self) -> Option<&[u8]> {
        self.loaded.as_deref()
    }

    /// Whether cancel has been requested for this job.
    #[must_use]
    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }

    /// Whether the runner is shutting down. A handler that sees this should
    /// return [`TaskError::Interrupted`] so the checkpoint is kept.
    #[must_use]
    pub fn stopping(&self) -> bool {
        self.stopping.load(Ordering::SeqCst)
    }

    /// Writes `bytes` as the resume point and `progress` as a percentage.
    /// Honours cancel and shutdown first.
    ///
    /// # Errors
    ///
    /// [`TaskError::Cancelled`] when cancel was requested,
    /// [`TaskError::Interrupted`] when the runner is shutting down,
    /// [`TaskError::Store`] when the cache could not keep the checkpoint.
    pub fn checkpoint(&self, bytes: &[u8], progress: u8) -> Result<(), TaskError> {
        self.guard()?;
        let progress = progress.min(100);
        self.progress.store(progress, Ordering::SeqCst);
        let id = self.id;
        let blob = bytes.to_vec();
        wait::wait(
            self.store
                .write(move |tx| persist::checkpoint(tx, id, &blob, progress)),
        )
        .map_err(TaskError::from)
    }

    /// Reports `progress` as a percentage without changing the checkpoint.
    ///
    /// # Errors
    ///
    /// The same as [`Self::checkpoint`].
    pub fn progress(&self, progress: u8) -> Result<(), TaskError> {
        self.guard()?;
        let progress = progress.min(100);
        self.progress.store(progress, Ordering::SeqCst);
        let id = self.id;
        wait::wait(
            self.store
                .write(move |tx| persist::set_progress(tx, id, progress)),
        )
        .map_err(TaskError::from)
    }

    fn guard(&self) -> Result<(), TaskError> {
        if self.stopping() {
            Err(TaskError::Interrupted)
        } else if self.cancelled() {
            Err(TaskError::Cancelled)
        } else {
            Ok(())
        }
    }
}
