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

use std::sync::mpsc::{RecvTimeoutError, sync_channel};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread;
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

/// A job as the pool starts it: the caller's job and the channel its result
/// goes back through.
type Task = Box<dyn FnOnce() + Send>;

/// What a pool counts.
#[derive(Debug)]
struct Ledger {
    /// Jobs running now.
    running: usize,
    /// Workers lost since the pool was made or resumed.
    lost: u8,
}

/// One root's pool. See the [module documentation](self).
#[derive(Debug)]
pub struct Pool {
    limit: usize,
    ledger: Mutex<Ledger>,
}

impl Pool {
    /// A pool that runs at most `limit` jobs at once.
    #[must_use]
    pub const fn new(limit: usize) -> Self {
        Self {
            limit,
            ledger: Mutex::new(Ledger {
                running: 0,
                lost: 0,
            }),
        }
    }

    /// The counts. A job that panics does so on its own thread, so the
    /// lock is never poisoned with the counts half changed.
    fn ledger(&self) -> MutexGuard<'_, Ledger> {
        self.ledger.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Takes a place for one job.
    fn admit(&self) -> Result<(), PoolError> {
        let mut ledger = self.ledger();
        if ledger.lost >= LOST_LIMIT {
            return Err(PoolError::Paused);
        }
        if ledger.running >= self.limit {
            return Err(PoolError::Busy);
        }
        ledger.running = ledger.running.saturating_add(1);
        Ok(())
    }

    /// Takes a place for `task` and starts it on a thread of its own, or
    /// refuses it and drops it.
    fn start(&self, task: Task) -> Result<(), PoolError> {
        self.admit().map(|()| {
            thread::spawn(task);
        })
    }

    /// Gives a job's place back, and counts `lost` more lost workers.
    fn release(&self, lost: u8) {
        let mut ledger = self.ledger();
        ledger.running = ledger.running.saturating_sub(1);
        ledger.lost = ledger.lost.saturating_add(lost);
    }

    /// Settles a job that gave no result: one that missed its deadline is
    /// given up on and its worker counted as lost, and one whose thread
    /// ended without a result panicked.
    fn gave_up(&self, error: RecvTimeoutError) -> PoolError {
        match error {
            RecvTimeoutError::Timeout => {
                self.release(1);
                PoolError::TimedOut
            }
            RecvTimeoutError::Disconnected => {
                self.release(0);
                PoolError::Panicked
            }
        }
    }

    /// Runs `job` on a thread of this root's own and waits for its result
    /// until `deadline` has passed.
    ///
    /// # Errors
    ///
    /// Returns [`PoolError::Paused`] or [`PoolError::Busy`] when the job was
    /// not started, [`PoolError::TimedOut`] when it missed the deadline, and
    /// [`PoolError::Panicked`] when it panicked.
    pub fn run<T, F>(&self, deadline: Duration, job: F) -> Result<T, PoolError>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        let (sender, receiver) = sync_channel(1);
        self.start(Box::new(move || {
            // The caller may have given up, and then nobody is listening.
            let _ = sender.send(job());
        }))
        .and_then(|()| {
            receiver
                .recv_timeout(deadline)
                .map(|value| {
                    self.release(0);
                    value
                })
                .map_err(|error| self.gave_up(error))
        })
    }

    /// Counts a worker that is stuck reading this root and was given up on
    /// somewhere else, such as a scan worker process.
    pub fn report_lost(&self) {
        let mut ledger = self.ledger();
        ledger.lost = ledger.lost.saturating_add(1);
    }

    /// Forgets the lost workers and accepts jobs again, for an admin's
    /// retry or once the storage is seen to answer.
    pub fn resume(&self) {
        self.ledger().lost = 0;
    }

    /// Whether the root is paused.
    #[must_use]
    pub fn status(&self) -> Status {
        if self.ledger().lost >= LOST_LIMIT {
            Status::NotResponding
        } else {
            Status::Running
        }
    }
}
