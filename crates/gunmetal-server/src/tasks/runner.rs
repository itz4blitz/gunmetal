//! The runner: requests, handlers, the worker thread, cancel and expiry.

use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};

use gunmetal_core::id::PublicId;
use gunmetal_core::time::{Clock, Timestamp};
use gunmetal_store::store::Store;

use super::ctx::TaskCtx;
use super::error::{Outcome, TaskError, TaskStatus};
use super::handler::Handler;
use super::kind::TaskKind;
use super::persist;
use super::wait;

/// The longest path a path-scoped request may name.
pub const MAX_PATH: usize = 4_096;

/// How often expiry sweeps run when the caller ticks the runner.
pub const SWEEP_EVERY_MS: i64 = 3_600_000;

/// Rotation and sweep limits. Tests shorten the sweep period.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Run registered expiry sweeps after this many milliseconds.
    pub sweep_every_ms: i64,
}

impl Limits {
    /// Production limits: a sweep every hour.
    #[must_use]
    pub const fn production() -> Self {
        Self {
            sweep_every_ms: SWEEP_EVERY_MS,
        }
    }

    /// Short limits so tests reach a sweep without waiting an hour.
    #[must_use]
    pub const fn test() -> Self {
        Self {
            sweep_every_ms: 1_000,
        }
    }
}

/// The identifier of one job. Repeated requests for the same open job
/// return this same value (SEC-API-064).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(pub(crate) u64);

impl TaskId {
    /// The identifier as a 64-bit number.
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    pub(crate) fn as_i64(self) -> i64 {
        i64::try_from(self.0).unwrap_or(i64::MAX)
    }
}

/// The handle a request returns: the identifier of the job, which a
/// repeated request shares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskHandle {
    /// The job.
    pub id: TaskId,
}

/// One job as a reader sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSnapshot {
    /// Its identifier.
    pub id: TaskId,
    /// Its kind.
    pub kind: TaskKind,
    /// Who requested it.
    pub principal: PublicId,
    /// The path it is scoped to, when it is path-scoped.
    pub path: Option<String>,
    /// How far it has got.
    pub status: TaskStatus,
    /// Progress as a percentage, 0 to 100.
    pub progress: u8,
    /// The latest checkpoint blob.
    pub checkpoint: Option<Vec<u8>>,
    /// The failure message, when it failed.
    pub error: Option<String>,
    /// When it was requested.
    pub requested_at: Timestamp,
    /// When a worker started it.
    pub started_at: Option<Timestamp>,
    /// When it finished, failed or was cancelled.
    pub finished_at: Option<Timestamp>,
    /// How long the run took, once it has finished.
    pub duration_ms: Option<i64>,
}

type HandlerMap = HashMap<TaskKind, Arc<dyn Handler>>;
type Sweep = Arc<dyn Fn(Timestamp) + Send + Sync>;
type CancelMap = HashMap<TaskId, Arc<AtomicBool>>;

/// Shared with the worker thread.
struct Inner {
    handlers: Mutex<HandlerMap>,
    sweeps: Mutex<Vec<Sweep>>,
    last_sweep: Mutex<Option<Timestamp>>,
    cancel: Mutex<CancelMap>,
    stopping: Arc<AtomicBool>,
    has_work: AtomicBool,
    lock: Mutex<()>,
    work: Condvar,
    limits: Limits,
}

/// The task runner. Dropping it stops the worker after the current
/// checkpoint and keeps a running job's checkpoint for resume.
pub struct Runner {
    store: Arc<Store>,
    clock: Arc<dyn Clock + Send + Sync>,
    inner: Arc<Inner>,
    worker: Option<JoinHandle<()>>,
}

fn recover<T>(result: Result<T, PoisonError<T>>) -> T {
    result.unwrap_or_else(PoisonError::into_inner)
}

impl Runner {
    /// Opens a runner on `store`, requeues jobs left running, and starts
    /// the worker thread. `clock` is the time of requests, checkpoints and
    /// sweeps.
    ///
    /// # Errors
    ///
    /// [`TaskError::Store`] when the cache cannot requeue interrupted jobs.
    pub fn open(
        store: Arc<Store>,
        clock: Arc<dyn Clock + Send + Sync>,
        limits: Limits,
    ) -> Result<Self, TaskError> {
        wait::wait(store.write(persist::requeue_running))?;
        let inner = Arc::new(Inner {
            handlers: Mutex::new(HashMap::new()),
            sweeps: Mutex::new(Vec::new()),
            last_sweep: Mutex::new(None),
            cancel: Mutex::new(HashMap::new()),
            stopping: Arc::new(AtomicBool::new(false)),
            has_work: AtomicBool::new(false),
            lock: Mutex::new(()),
            work: Condvar::new(),
            limits,
        });
        let worker_store = Arc::clone(&store);
        let worker_clock = Arc::clone(&clock);
        let worker_inner = Arc::clone(&inner);
        let worker =
            thread::spawn(move || worker_loop(&worker_store, &worker_clock, &worker_inner));
        let runner = Self {
            store,
            clock,
            inner,
            worker: Some(worker),
        };
        runner.tick();
        Ok(runner)
    }

    /// Registers `handler` for its kind. A waiting job of that kind becomes
    /// runnable.
    pub fn register(&self, handler: Arc<dyn Handler>) {
        let kind = handler.kind();
        recover(self.inner.handlers.lock()).insert(kind, handler);
        self.wake();
    }

    /// Registers an expiry sweep. [`Self::tick`] runs every registered
    /// sweep when the period has elapsed.
    pub fn on_expiry(&self, sweep: impl Fn(Timestamp) + Send + Sync + 'static) {
        recover(self.inner.sweeps.lock()).push(Arc::new(sweep));
    }

    /// Runs due expiry sweeps at the clock's now.
    pub fn tick(&self) {
        tick(&self.inner, self.clock.now());
    }

    /// Requests a job of `kind` for `principal`, scoped to `path` when
    /// present. A request for an open job of the same kind, principal and
    /// path returns that job's identifier (SEC-API-064).
    ///
    /// # Errors
    ///
    /// [`TaskError::PathTooLong`] when `path` is longer than [`MAX_PATH`],
    /// [`TaskError::Store`] when the cache cannot keep the request.
    pub fn request(
        &self,
        kind: TaskKind,
        principal: PublicId,
        path: Option<&str>,
    ) -> Result<TaskHandle, TaskError> {
        if let Some(path) = path {
            if path.len() > MAX_PATH {
                return Err(TaskError::PathTooLong { length: path.len() });
            }
        }
        let now = self.clock.now();
        let principal = principal.to_string();
        let path = path.unwrap_or("").to_owned();
        let id = wait::wait(
            self.store
                .write(move |tx| persist::insert_or_join(tx, kind, &principal, &path, now)),
        )?;
        self.wake();
        Ok(TaskHandle { id })
    }

    /// Asks a waiting job to be cancelled, or a running job to stop at its
    /// next checkpoint.
    ///
    /// # Errors
    ///
    /// [`TaskError::Unknown`] when no job has `id`,
    /// [`TaskError::Store`] when the cache cannot record the cancel.
    pub fn cancel(&self, id: TaskId) -> Result<(), TaskError> {
        let now = self.clock.now();
        if let Some(flag) = recover(self.inner.cancel.lock()).get(&id) {
            flag.store(true, Ordering::SeqCst);
        }
        let cancelled = wait::wait(
            self.store
                .write(move |tx| persist::cancel_waiting(tx, id, now)),
        )?;
        if cancelled {
            return Ok(());
        }
        self.snapshot(id).map(|_| ())
    }

    /// One job as stored.
    ///
    /// # Errors
    ///
    /// [`TaskError::Unknown`] when no job has `id`,
    /// [`TaskError::Store`] when the cache cannot be read.
    pub fn snapshot(&self, id: TaskId) -> Result<TaskSnapshot, TaskError> {
        wait::wait(self.store.read(move |reader| persist::by_id(reader, id)))?
            .map_err(TaskError::from)?
            .ok_or(TaskError::Unknown)
    }

    /// Every job, oldest first.
    ///
    /// # Errors
    ///
    /// [`TaskError::Store`] when the cache cannot be read.
    pub fn list(&self) -> Result<Vec<TaskSnapshot>, TaskError> {
        wait::wait(self.store.read(persist::all))?.map_err(TaskError::from)
    }

    fn wake(&self) {
        self.inner.has_work.store(true, Ordering::SeqCst);
        drop(recover(self.inner.lock.lock()));
        self.inner.work.notify_all();
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        self.inner.stopping.store(true, Ordering::SeqCst);
        self.wake();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn tick(inner: &Inner, now: Timestamp) {
    let due = {
        let last = recover(inner.last_sweep.lock());
        match *last {
            None => true,
            Some(at) => {
                inner.limits.sweep_every_ms != 0
                    && now.millis().saturating_sub(at.millis()) >= inner.limits.sweep_every_ms
            }
        }
    };
    if !due {
        return;
    }
    let sweeps = recover(inner.sweeps.lock()).clone();
    for sweep in sweeps {
        sweep(now);
    }
    *recover(inner.last_sweep.lock()) = Some(now);
}

fn worker_loop(store: &Arc<Store>, clock: &Arc<dyn Clock + Send + Sync>, inner: &Inner) {
    loop {
        if inner.stopping.load(Ordering::SeqCst) {
            break;
        }
        if let Some(job) = next_job(store, inner) {
            run_job(store, clock, inner, job);
        } else {
            // Inlined so a `wait_for_work -> false` mutant cannot turn
            // shutdown into a busy loop that outlives cargo-mutants.
            let mut guard: MutexGuard<'_, ()> = recover(inner.lock.lock());
            while !inner.has_work.load(Ordering::SeqCst) && !inner.stopping.load(Ordering::SeqCst) {
                guard = recover(inner.work.wait(guard));
            }
            inner.has_work.store(false, Ordering::SeqCst);
            if inner.stopping.load(Ordering::SeqCst) {
                break;
            }
        }
    }
}

struct Job {
    id: TaskId,
    handler: Arc<dyn Handler>,
    loaded: Option<Vec<u8>>,
}

fn next_job(store: &Arc<Store>, inner: &Inner) -> Option<Job> {
    let handlers = recover(inner.handlers.lock()).clone();
    if handlers.is_empty() {
        return None;
    }
    let Ok(Ok(waiting)) = wait::wait(store.read(persist::waiting)) else {
        return None;
    };
    waiting.into_iter().find_map(|snap| {
        handlers.get(&snap.kind).map(|handler| Job {
            id: snap.id,
            handler: Arc::clone(handler),
            loaded: snap.checkpoint,
        })
    })
}

fn run_job(store: &Arc<Store>, clock: &Arc<dyn Clock + Send + Sync>, inner: &Inner, job: Job) {
    let now = clock.now();
    let started = match wait::wait(store.write({
        let id = job.id;
        move |tx| persist::mark_running(tx, id, now)
    })) {
        Ok(true) => now,
        Ok(false) | Err(_) => return,
    };
    let cancel = Arc::new(AtomicBool::new(false));
    recover(inner.cancel.lock()).insert(job.id, Arc::clone(&cancel));
    let ctx = TaskCtx {
        id: job.id,
        store: Arc::clone(store),
        cancel: Arc::clone(&cancel),
        stopping: Arc::clone(&inner.stopping),
        loaded: job.loaded,
        progress: AtomicU8::new(0),
    };
    let result = catch_unwind(AssertUnwindSafe(|| job.handler.run(&ctx)));
    recover(inner.cancel.lock()).remove(&job.id);
    let progress = ctx.progress.load(Ordering::SeqCst);
    let finished_at = clock.now();
    let (status, error) = match result {
        Ok(Ok(Outcome::Succeeded)) => (TaskStatus::Succeeded, None),
        Ok(Err(TaskError::Cancelled)) => (TaskStatus::Cancelled, None),
        Ok(Err(TaskError::Interrupted)) => {
            // Leave the row running so a restart requeues it with its checkpoint.
            return;
        }
        Ok(Err(TaskError::Failed { message })) => (TaskStatus::Failed, Some(message)),
        Ok(Err(_)) => (TaskStatus::Failed, Some("failed".to_owned())),
        Err(_) => (TaskStatus::Failed, Some("panicked".to_owned())),
    };
    let id = job.id;
    let error_owned = error.clone();
    let _ = wait::wait(store.write(move |tx| {
        persist::finish(
            tx,
            id,
            status,
            progress,
            error_owned.as_deref(),
            finished_at,
            Some(started),
        )
    }));
}
