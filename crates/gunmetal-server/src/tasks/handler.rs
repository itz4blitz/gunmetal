//! The handler a module registers for one [`TaskKind`].

use super::ctx::TaskCtx;
use super::error::{Outcome, TaskError};
use super::kind::TaskKind;

/// Work for one kind. The runner calls [`Handler::run`] on an operating-system
/// thread and records a panic as a failure so the queue keeps going.
pub trait Handler: Send + Sync + 'static {
    /// The kind this handler runs.
    fn kind(&self) -> TaskKind;

    /// Does the work. Checkpoints and cancel are observed through `ctx`.
    ///
    /// # Errors
    ///
    /// [`TaskError::Cancelled`] when cancel was honoured,
    /// [`TaskError::Interrupted`] when the runner is shutting down,
    /// [`TaskError::Failed`] when the work itself failed.
    fn run(&self, ctx: &TaskCtx) -> Result<Outcome, TaskError>;
}
