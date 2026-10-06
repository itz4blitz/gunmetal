//! A bounded pool for one library root's blocking file calls.
//!
//! A read from a hung network share never returns, and nothing can
//! interrupt it. So the calls that touch a root's files run on that root's
//! own [`Pool`], never on a thread a request or another root waits for
//! (SEC-MED-041, LIB-207):
//!
//! - at most `limit` jobs are waited for at once, and one more is refused
//!   as [`PoolError::Busy`] instead of queueing behind them;
//! - a job that has not finished by its deadline is given up on: the caller
//!   gets [`PoolError::TimedOut`], and the worker is counted as lost and no
//!   longer holds a place. Its thread lives on until the call it is stuck
//!   in returns, because nothing can stop it;
//! - after [`LOST_LIMIT`] lost workers the root is paused. Its status is
//!   [`Status::NotResponding`], and every job is refused as
//!   [`PoolError::Paused`] until [`Pool::resume`]. Other roots have pools
//!   of their own and carry on;
//! - when the operating system will not start another thread, the job is
//!   refused as [`PoolError::NoThread`] and its place given back.
//!
//! So a hung share holds at most `limit` threads that the pool waits for,
//! and at most [`LOST_LIMIT`] stuck threads between one resume and the
//! next. A resume forgets the lost workers, not their threads: each resume
//! while the share is still hung lets [`LOST_LIMIT`] more threads get
//! stuck, so whoever resumes a root decides how often to retry (WP-082).
//!
//! The scan's worker processes are counted here too: the worker pool
//! (WP-078) calls [`Pool::report_lost`] when a worker is stuck reading this
//! root, so the rule that pauses a root is in one place.

use std::io;
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
    /// The operating system would not start a thread for the job, and the
    /// job was not started.
    NoThread {
        /// How the operating system refused.
        kind: io::ErrorKind,
    },
}

/// A job as the pool starts it: the caller's job and the channel its result
/// goes back through.
type Task = Box<dyn FnOnce() + Send>;

/// Starts `task` on a thread of its own. The operating system may refuse,
/// when the process or the system has as many threads as it allows, or no
/// memory for another.
fn spawn(task: Task) -> io::Result<()> {
    thread::Builder::new().spawn(task).map(drop)
}

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

    /// Takes a place for `task` and starts it with `spawn` on a thread of
    /// its own, or refuses it and drops it.
    fn start(&self, task: Task, spawn: fn(Task) -> io::Result<()>) -> Result<(), PoolError> {
        self.admit().and_then(|()| {
            spawn(task).map_err(|error| {
                self.release(0);
                PoolError::NoThread { kind: error.kind() }
            })
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
    /// Returns [`PoolError::Paused`], [`PoolError::Busy`] or
    /// [`PoolError::NoThread`] when the job was not started,
    /// [`PoolError::TimedOut`] when it missed the deadline, and
    /// [`PoolError::Panicked`] when it panicked.
    pub fn run<T, F>(&self, deadline: Duration, job: F) -> Result<T, PoolError>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        let (sender, receiver) = sync_channel(1);
        self.start(
            Box::new(move || {
                // The caller may have given up, and then nobody is listening.
                let _ = sender.send(job());
            }),
            spawn,
        )
        .and_then(|()| {
            receiver
                .recv_timeout(deadline)
                .inspect(|_| self.release(0))
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
    /// retry or once the storage is seen to answer. A lost worker's thread
    /// that is still stuck stays stuck, and is no longer counted.
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

/// The operating system's refusal to start a thread cannot be caused on
/// demand from outside, so this test hands the pool a stand-in for the
/// thread spawner that refuses as `pthread_create` does when the process
/// may have no more threads (`EAGAIN`).
#[cfg(test)]
mod tests {
    use super::*;

    /// A spawner that refuses every thread, and drops the task unrun.
    fn refuse(task: Task) -> io::Result<()> {
        drop(task);
        Err(io::Error::from(io::ErrorKind::WouldBlock))
    }

    /// A job that does nothing.
    fn job() {}

    #[test]
    fn gives_the_place_back_when_no_thread_can_be_started() {
        let pool = Pool::new(1);
        assert_eq!(
            pool.start(Box::new(job), refuse),
            Err(PoolError::NoThread {
                kind: io::ErrorKind::WouldBlock
            })
        );
        // The pool has one place, so had the refused job kept it this
        // would be refused as busy.
        assert_eq!(pool.run(Duration::from_secs(30), job), Ok(()));
        assert_eq!(pool.status(), Status::Running);
    }
}
