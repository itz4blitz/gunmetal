//! The per-root pool: a bounded number of jobs, and a root that stops
//! taking them once its storage has stopped answering.
//!
//! No test here sleeps to line threads up. A job that must not finish
//! waits on a channel the test holds, and every wait a test makes is
//! bounded, so a pool that lost a job fails the test instead of hanging it.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

use gunmetal_fs::pool::{LOST_LIMIT, Pool, PoolError, Status};

use crate::support::{LONG, SHORT};

/// A job that does not finish until the sender it comes with is dropped,
/// as a read from a hung mount does not finish.
fn stuck() -> (Sender<()>, impl FnOnce() -> u8 + Send + 'static) {
    let (hold, wait) = channel::<()>();
    (hold, move || {
        let _ = wait.recv();
        0
    })
}

/// Starts a job on another thread that holds a place in `pool` until the
/// sender is dropped, and waits until the job is running. The receiver
/// gets what the pool returned for it.
fn occupy(pool: &Arc<Pool>) -> (Sender<()>, Receiver<Result<u8, PoolError>>) {
    let (hold, job) = stuck();
    let (started, running) = channel();
    let (done, outcome) = channel();
    let pool = Arc::clone(pool);
    thread::spawn(move || {
        let ran = pool.run(LONG, move || {
            let _ = started.send(());
            job()
        });
        let _ = done.send(ran);
    });
    assert_eq!(running.recv_timeout(LONG), Ok(()), "the job starts");
    (hold, outcome)
}

/// Loses one worker of `pool` to a job that never finishes, and returns
/// what keeps that job from finishing.
fn lose_a_worker(pool: &Pool) -> Sender<()> {
    let (hold, job) = stuck();
    assert_eq!(pool.run(SHORT, job), Err(PoolError::TimedOut));
    hold
}

#[test]
fn runs_a_job_on_a_thread_of_its_own_and_returns_what_it_made() {
    let pool = Pool::new(1);
    let here = thread::current().id();
    assert_eq!(
        pool.run(LONG, move || (thread::current().id() == here, 6 * 7)),
        Ok((false, 42))
    );
    assert_eq!(pool.status(), Status::Running);
}

#[test]
fn gives_a_place_back_when_a_job_ends() {
    let pool = Pool::new(1);
    for n in 0..3 {
        assert_eq!(pool.run(LONG, move || n), Ok(n));
    }
}

/// Verifies: SEC-MED-041
#[test]
fn refuses_a_job_beyond_the_roots_limit_instead_of_queueing_it() {
    let pool = Arc::new(Pool::new(2));
    let (first, first_outcome) = occupy(&pool);
    let (second, second_outcome) = occupy(&pool);
    assert_eq!(pool.run(LONG, || 1), Err(PoolError::Busy));
    drop(first);
    assert_eq!(first_outcome.recv_timeout(LONG), Ok(Ok(0)));
    // One place is free again while the other job still runs.
    assert_eq!(pool.run(LONG, || 1), Ok(1));
    drop(second);
    assert_eq!(second_outcome.recv_timeout(LONG), Ok(Ok(0)));
}

/// The plan asks for a FUSE mount that never answers. Mounting one needs a
/// crate the workspace does not have, so the hang here is a job that never
/// returns, which is all the pool can see of a hung mount.
///
/// Verifies: SEC-MED-041
#[test]
fn pauses_a_root_after_two_lost_workers_and_leaves_other_roots_running() {
    assert_eq!(LOST_LIMIT, 2);
    let hung = Pool::new(1);
    let healthy = Pool::new(1);
    let first = lose_a_worker(&hung);
    assert_eq!(hung.status(), Status::Running);
    // The lost worker no longer holds the root's only place, so a second
    // job starts, and is lost too.
    let second = lose_a_worker(&hung);
    assert_eq!(hung.status(), Status::NotResponding);
    let ran = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&ran);
    assert_eq!(
        hung.run(LONG, move || flag.store(true, Ordering::SeqCst)),
        Err(PoolError::Paused)
    );
    assert!(!ran.load(Ordering::SeqCst), "a paused root starts nothing");
    assert_eq!(healthy.run(LONG, || 5), Ok(5));
    assert_eq!(healthy.status(), Status::Running);
    drop((first, second));
}

/// Verifies: SEC-MED-041
#[test]
fn takes_jobs_again_once_it_is_resumed_and_counts_from_nothing() {
    let pool = Pool::new(1);
    let held = [lose_a_worker(&pool), lose_a_worker(&pool)];
    assert_eq!(pool.status(), Status::NotResponding);
    pool.resume();
    assert_eq!(pool.status(), Status::Running);
    assert_eq!(pool.run(LONG, || 9), Ok(9));
    // One more lost worker is the first of a new count.
    let third = lose_a_worker(&pool);
    assert_eq!(pool.status(), Status::Running);
    assert_eq!(pool.run(LONG, || 9), Ok(9));
    drop((held, third));
}

/// Verifies: SEC-MED-041
#[test]
fn counts_workers_that_were_reported_lost_from_elsewhere() {
    let pool = Pool::new(4);
    pool.report_lost();
    assert_eq!(pool.status(), Status::Running);
    assert_eq!(pool.run(LONG, || 1), Ok(1));
    pool.report_lost();
    assert_eq!(pool.status(), Status::NotResponding);
    assert_eq!(pool.run(LONG, || 1), Err(PoolError::Paused));
    // A third report does not wake the root up again.
    pool.report_lost();
    assert_eq!(pool.status(), Status::NotResponding);
    pool.resume();
    // A worker lost here and one lost elsewhere count together.
    let held = lose_a_worker(&pool);
    assert_eq!(pool.status(), Status::Running);
    pool.report_lost();
    assert_eq!(pool.status(), Status::NotResponding);
    drop(held);
}

#[test]
fn reports_a_job_that_panicked_and_gives_its_place_back() {
    let pool = Pool::new(1);
    let outcome: Result<u8, PoolError> = pool.run(LONG, || panic!("the job failed"));
    assert_eq!(outcome, Err(PoolError::Panicked));
    assert_eq!(pool.status(), Status::Running);
    assert_eq!(pool.run(LONG, || 3), Ok(3));
}
