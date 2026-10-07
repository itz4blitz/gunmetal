//! Outcomes, statuses and errors of one task.

use gunmetal_store::store::StoreError;

/// What a handler returns when it finishes the work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The work completed.
    Succeeded,
}

/// How far a task has got, as readers see it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    /// Queued, including a kind that has no handler yet.
    Waiting,
    /// A worker thread is running it.
    Running,
    /// It completed.
    Succeeded,
    /// It failed or panicked.
    Failed,
    /// Cancel was honoured at a checkpoint.
    Cancelled,
}

impl TaskStatus {
    /// The inventory name stored in the cache.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Waiting => "waiting",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    /// The status whose inventory name is `text`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "waiting" => Some(Self::Waiting),
            "running" => Some(Self::Running),
            "succeeded" => Some(Self::Succeeded),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

/// Why a request, cancel, read or run failed.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskError {
    /// Cancel was honoured at a checkpoint.
    Cancelled,
    /// The runner is shutting down; the checkpoint is kept for resume.
    Interrupted,
    /// The handler reported a failure.
    Failed {
        /// What it said, without a panic payload.
        message: String,
    },
    /// The cache could not run the statement.
    Store(StoreError),
    /// No task has this identifier.
    Unknown,
    /// The path was longer than [`super::runner::MAX_PATH`].
    PathTooLong {
        /// The length that was offered.
        length: usize,
    },
}

impl From<StoreError> for TaskError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}

#[cfg(test)]
mod tests {
    use super::TaskError;
    use gunmetal_store::store::StoreError;

    #[test]
    fn maps_a_store_error() {
        assert_eq!(
            TaskError::from(StoreError::Closed),
            TaskError::Store(StoreError::Closed)
        );
        assert_eq!(TaskError::Cancelled, TaskError::Cancelled);
        assert_eq!(TaskError::Interrupted, TaskError::Interrupted);
        assert_eq!(TaskError::Unknown, TaskError::Unknown);
        assert_eq!(
            TaskError::Failed {
                message: "no space".to_owned()
            },
            TaskError::Failed {
                message: "no space".to_owned()
            }
        );
        assert_eq!(
            TaskError::PathTooLong { length: 4_097 },
            TaskError::PathTooLong { length: 4_097 }
        );
    }
}
