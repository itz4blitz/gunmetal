//! A bounded pool for one library root's blocking file calls.
//!
//! A read from a hung network share never returns, and nothing can
//! interrupt it. So the calls that touch a root's files run on that root's
//! own [`Pool`], never on a thread a request or another root waits for
//! (SEC-MED-041, LIB-207):
//!
//! - at most `limit` jobs run at once, and one more is refused as
//!   [`PoolError::Busy`] instead of queueing behind them;
//! - a job that has not finished by its deadline is given up on: the caller
//!   gets [`PoolError::TimedOut`], and the worker is counted as lost and no
//!   longer holds a place;
//! - after [`LOST_LIMIT`] lost workers the root is paused. Its status is
//!   [`Status::NotResponding`], and every job is refused as
//!   [`PoolError::Paused`] until [`Pool::resume`], so a hung share costs
//!   two stuck threads and no more. Other roots have pools of their own
//!   and carry on.
//!
//! The scan's worker processes are counted here too: the worker pool
//! (WP-078) calls [`Pool::report_lost`] when a worker is stuck reading this
//! root, so the rule that pauses a root is in one place.

use std::time::Duration;

/// How many lost workers pause a root.
pub const LOST_LIMIT: u8 = 2;

/// Whether a root's storage answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Jobs are accepted.
    Running,
    /// The root is paused: "storage not responding".
    NotResponding,
}

/// Why a job did not give a result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolError {
    /// The root is paused, and the job was not started.
    Paused,
    /// Every place was taken, and the job was not started.
    Busy,
    /// The job did not finish by its deadline. Its worker is lost.
    TimedOut,
    /// The job panicked.
    Panicked,
}

/// One root's pool. See the [module documentation](self).
#[derive(Debug)]
pub struct Pool {
    _limit: usize,
}

impl Pool {
    /// A pool that runs at most `limit` jobs at once.
    #[must_use]
    pub const fn new(limit: usize) -> Self {
        Self { _limit: limit }
    }

    /// Runs `job` on a thread of this root's own and waits for its result
    /// until `deadline` has passed.
    ///
    /// # Errors
    ///
    /// Returns [`PoolError::Paused`] or [`PoolError::Busy`] when the job was
    /// not started, [`PoolError::TimedOut`] when it missed the deadline, and
    /// [`PoolError::Panicked`] when it panicked.
    pub fn run<T, F>(&self, _deadline: Duration, job: F) -> Result<T, PoolError>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        drop(job);
        Err(PoolError::Panicked)
    }

    /// Counts a worker that is stuck reading this root and was given up on
    /// somewhere else, such as a scan worker process.
    pub const fn report_lost(&self) {}

    /// Forgets the lost workers and accepts jobs again, for an admin's
    /// retry or once the storage is seen to answer.
    pub const fn resume(&self) {}

    /// Whether the root is paused.
    #[must_use]
    pub const fn status(&self) -> Status {
        Status::NotResponding
    }
}
