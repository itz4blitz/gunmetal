//! Behaviour of the task runner: de-duplication, cancel, panic isolation,
//! resume after restart, and expiry sweeps.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use gunmetal_core::id::{IdKind, PublicId};
use gunmetal_core::time::Timestamp;
use gunmetal_fs::dataroot::{DataRoot, Policy};
use gunmetal_fs::host::HostFacts;
use gunmetal_store::store::{Generation, Store, StoreError};
use gunmetal_testkit::clock::ManualClock;
use gunmetal_testkit::tempdir::TempDir;

use super::TaskCtx;
use super::error::{Outcome, TaskError, TaskStatus};
use super::handler::Handler;
use super::kind::TaskKind;
use super::persist;
use super::runner::{Limits, MAX_PATH, Runner, TaskHandle, TaskId, TaskSnapshot};
use super::schema::SCHEMA;
use super::wait;
use crate::testing::{self, TestClock};

const BOUND: Duration = Duration::from_millis(400);
const FIRST: Generation = Generation([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);

struct World {
    runner: Runner,
    store: Arc<Store>,
    clock: Arc<TestClock>,
    manual: Arc<ManualClock>,
    root: DataRoot,
    _dir: TempDir,
}

fn user(n: u8) -> PublicId {
    PublicId::parse(&format!("usr_{n:026}"), IdKind::User).expect("a canonical user ID")
}

fn world() -> World {
    let dir = TempDir::new("server-tasks").expect("scratch");
    let host = HostFacts::probe(dir.path()).expect("probe");
    let root = DataRoot::open(dir.path(), &host, Policy::DEFAULT)
        .expect("data root")
        .root;
    let opened = Store::open(&root, &[SCHEMA], FIRST).expect("store");
    let store = Arc::new(opened.store);
    let (clock, manual) = testing::clock();
    let runner = Runner::open(Arc::clone(&store), clock.clone(), Limits::test()).expect("runner");
    World {
        runner,
        store,
        clock,
        manual,
        root,
        _dir: dir,
    }
}

fn reopen(world: World) -> World {
    let World {
        runner,
        store: _,
        clock,
        manual,
        root,
        _dir: dir,
    } = world;
    drop(runner);
    let opened = Store::open(&root, &[SCHEMA], FIRST).expect("reopen");
    let store = Arc::new(opened.store);
    let runner = Runner::open(Arc::clone(&store), clock.clone(), Limits::test()).expect("runner");
    World {
        runner,
        store,
        clock,
        manual,
        root,
        _dir: dir,
    }
}

fn settle(runner: &Runner, id: TaskId) -> TaskSnapshot {
    let deadline = Instant::now() + BOUND;
    loop {
        let snap = runner.snapshot(id).expect("snapshot");
        if matches!(
            snap.status,
            TaskStatus::Succeeded | TaskStatus::Failed | TaskStatus::Cancelled
        ) {
            return snap;
        }
        assert!(
            Instant::now() < deadline,
            "task {} did not finish",
            id.as_u64()
        );
        thread::sleep(Duration::from_millis(5));
    }
}

fn until(runner: &Runner, id: TaskId, want: impl Fn(&TaskSnapshot) -> bool) -> TaskSnapshot {
    let deadline = Instant::now() + BOUND;
    loop {
        let snap = runner.snapshot(id).expect("snapshot");
        if want(&snap) {
            return snap;
        }
        assert!(
            Instant::now() < deadline,
            "task {} did not reach the expected state: {:?}",
            id.as_u64(),
            snap.status
        );
        thread::sleep(Duration::from_millis(5));
    }
}

struct Completing {
    kind: TaskKind,
    runs: Arc<AtomicUsize>,
}

impl Handler for Completing {
    fn kind(&self) -> TaskKind {
        self.kind
    }

    fn run(&self, ctx: &TaskCtx) -> Result<Outcome, TaskError> {
        ctx.progress(100)?;
        self.runs.fetch_add(1, Ordering::SeqCst);
        Ok(Outcome::Succeeded)
    }
}

struct Holding {
    kind: TaskKind,
    at_checkpoint: Arc<AtomicBool>,
    hold: Arc<AtomicBool>,
    saw: Arc<Mutex<Vec<u8>>>,
}

impl Handler for Holding {
    fn kind(&self) -> TaskKind {
        self.kind
    }

    fn run(&self, ctx: &TaskCtx) -> Result<Outcome, TaskError> {
        let n = ctx
            .loaded()
            .and_then(|bytes| bytes.first().copied())
            .unwrap_or(0);
        self.saw.lock().expect("saw").push(n);
        ctx.checkpoint(&[n.saturating_add(1)], 50)?;
        self.at_checkpoint.store(true, Ordering::SeqCst);
        let deadline = Instant::now() + BOUND;
        while self.hold.load(Ordering::SeqCst) && !ctx.cancelled() && !ctx.stopping() {
            assert!(
                Instant::now() < deadline,
                "handler did not see cancel or stop"
            );
            thread::sleep(Duration::from_millis(2));
        }
        if ctx.stopping() {
            return Err(TaskError::Interrupted);
        }
        ctx.checkpoint(&[n.saturating_add(1)], 100)?;
        Ok(Outcome::Succeeded)
    }
}

struct Panicking {
    kind: TaskKind,
}

impl Handler for Panicking {
    fn kind(&self) -> TaskKind {
        self.kind
    }

    fn run(&self, _ctx: &TaskCtx) -> Result<Outcome, TaskError> {
        panic!("a bug in a task");
    }
}

struct Failing {
    kind: TaskKind,
}

impl Handler for Failing {
    fn kind(&self) -> TaskKind {
        self.kind
    }

    fn run(&self, _ctx: &TaskCtx) -> Result<Outcome, TaskError> {
        Err(TaskError::Failed {
            message: "no space".to_owned(),
        })
    }
}

struct Watch {
    kind: TaskKind,
    at_checkpoint: Arc<AtomicBool>,
    hold: Arc<AtomicBool>,
    saw_stop: Arc<AtomicBool>,
}

impl Handler for Watch {
    fn kind(&self) -> TaskKind {
        self.kind
    }

    fn run(&self, ctx: &TaskCtx) -> Result<Outcome, TaskError> {
        ctx.checkpoint(&[1], 10)?;
        self.at_checkpoint.store(true, Ordering::SeqCst);
        let deadline = Instant::now() + BOUND;
        while self.hold.load(Ordering::SeqCst) && !ctx.stopping() {
            assert!(Instant::now() < deadline, "handler did not see stopping");
            thread::sleep(Duration::from_millis(2));
        }
        self.saw_stop.store(ctx.stopping(), Ordering::SeqCst);
        Err(TaskError::Interrupted)
    }
}

#[test]
fn every_r1_kind_has_a_stable_name() {
    let names: Vec<(&str, Option<TaskKind>)> = [
        "library_scan",
        "path_refresh",
        "backup",
        "purge",
        "rebuild",
        "analysis",
        "",
    ]
    .into_iter()
    .map(|name| (name, TaskKind::parse(name)))
    .collect();
    assert_eq!(
        names,
        vec![
            ("library_scan", Some(TaskKind::LibraryScan)),
            ("path_refresh", Some(TaskKind::PathRefresh)),
            ("backup", Some(TaskKind::Backup)),
            ("purge", Some(TaskKind::Purge)),
            ("rebuild", Some(TaskKind::Rebuild)),
            ("analysis", None),
            ("", None),
        ]
    );
    assert_eq!(
        TaskKind::ALL.map(TaskKind::as_str),
        ["library_scan", "path_refresh", "backup", "purge", "rebuild"]
    );
    assert_eq!(TaskStatus::parse("waiting"), Some(TaskStatus::Waiting));
    assert_eq!(TaskStatus::parse("running"), Some(TaskStatus::Running));
    assert_eq!(TaskStatus::parse("succeeded"), Some(TaskStatus::Succeeded));
    assert_eq!(TaskStatus::parse("failed"), Some(TaskStatus::Failed));
    assert_eq!(TaskStatus::parse("cancelled"), Some(TaskStatus::Cancelled));
    assert_eq!(TaskStatus::parse("checkpointed"), None);
    assert_eq!(persist::sqlite_constraint(), 19);
}

/// Verifies: SEC-API-064
#[test]
fn two_requests_for_the_same_path_share_one_job() {
    let world = world();
    let runs = Arc::new(AtomicUsize::new(0));
    world.runner.register(Arc::new(Completing {
        kind: TaskKind::PathRefresh,
        runs: Arc::clone(&runs),
    }));
    let first = world
        .runner
        .request(TaskKind::PathRefresh, user(1), Some("/music/a"))
        .expect("first");
    let second = world
        .runner
        .request(TaskKind::PathRefresh, user(1), Some("/music/a"))
        .expect("second");
    assert_eq!(first, second);
    let done = settle(&world.runner, first.id);
    assert_eq!(done.status, TaskStatus::Succeeded);
    assert_eq!(done.path.as_deref(), Some("/music/a"));
    assert_eq!(done.principal, user(1));
    assert_eq!(done.kind, TaskKind::PathRefresh);
    assert_eq!(done.progress, 100);
    assert!(done.duration_ms.is_some());
    assert_eq!(runs.load(Ordering::SeqCst), 1);
}

/// Verifies: SEC-API-064
#[test]
fn concurrent_requests_return_the_same_identifier() {
    let world = world();
    let ids: Vec<TaskHandle> = thread::scope(|scope| {
        let handles: Vec<_> = (0..16)
            .map(|_| {
                scope.spawn(|| {
                    world
                        .runner
                        .request(TaskKind::LibraryScan, user(1), None)
                        .expect("request")
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("joined"))
            .collect()
    });
    let first = ids[0];
    assert!(ids.iter().all(|id| *id == first));
    world.runner.register(Arc::new(Completing {
        kind: TaskKind::LibraryScan,
        runs: Arc::new(AtomicUsize::new(0)),
    }));
    let done = settle(&world.runner, first.id);
    assert_eq!(done.status, TaskStatus::Succeeded);
    assert_eq!(done.path, None);
}

#[test]
fn a_finished_job_does_not_join_a_later_request() {
    let world = world();
    world.runner.register(Arc::new(Completing {
        kind: TaskKind::Backup,
        runs: Arc::new(AtomicUsize::new(0)),
    }));
    let first = world
        .runner
        .request(TaskKind::Backup, user(1), None)
        .expect("first");
    settle(&world.runner, first.id);
    let second = world
        .runner
        .request(TaskKind::Backup, user(1), None)
        .expect("second");
    assert_ne!(first, second);
    assert_eq!(first.id.as_u64(), 1);
    assert_eq!(second.id.as_u64(), 2);
    settle(&world.runner, second.id);
}

#[test]
fn different_paths_are_different_jobs() {
    let world = world();
    world.runner.register(Arc::new(Completing {
        kind: TaskKind::PathRefresh,
        runs: Arc::new(AtomicUsize::new(0)),
    }));
    let a = world
        .runner
        .request(TaskKind::PathRefresh, user(1), Some("/a"))
        .expect("a");
    let b = world
        .runner
        .request(TaskKind::PathRefresh, user(1), Some("/b"))
        .expect("b");
    assert_ne!(a, b);
    settle(&world.runner, a.id);
    settle(&world.runner, b.id);
}

#[test]
fn a_kind_with_no_handler_stays_waiting() {
    let world = world();
    let handle = world
        .runner
        .request(TaskKind::Purge, user(1), None)
        .expect("queued");
    thread::sleep(Duration::from_millis(40));
    let snap = world.runner.snapshot(handle.id).expect("snap");
    assert_eq!(snap.status, TaskStatus::Waiting);
    assert_eq!(snap.progress, 0);
    world.runner.register(Arc::new(Completing {
        kind: TaskKind::Purge,
        runs: Arc::new(AtomicUsize::new(0)),
    }));
    let done = settle(&world.runner, handle.id);
    assert_eq!(done.status, TaskStatus::Succeeded);
}

#[test]
fn a_registered_handler_starts_the_waiting_job_promptly() {
    let world = world();
    let handle = world
        .runner
        .request(TaskKind::Backup, user(1), None)
        .expect("queued");
    world.runner.register(Arc::new(Completing {
        kind: TaskKind::Backup,
        runs: Arc::new(AtomicUsize::new(0)),
    }));
    let deadline = Instant::now() + Duration::from_millis(100);
    loop {
        let snap = world.runner.snapshot(handle.id).expect("snap");
        if snap.status != TaskStatus::Waiting {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "job stayed waiting after a handler was registered"
        );
        thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn cancel_stops_at_the_next_checkpoint() {
    let world = world();
    let at_checkpoint = Arc::new(AtomicBool::new(false));
    let hold = Arc::new(AtomicBool::new(true));
    world.runner.register(Arc::new(Holding {
        kind: TaskKind::Rebuild,
        at_checkpoint: Arc::clone(&at_checkpoint),
        hold: Arc::clone(&hold),
        saw: Arc::new(Mutex::new(Vec::new())),
    }));
    let handle = world
        .runner
        .request(TaskKind::Rebuild, user(1), None)
        .expect("run");
    until(&world.runner, handle.id, |snap| {
        snap.status == TaskStatus::Running && snap.checkpoint.as_deref() == Some([1].as_slice())
    });
    world.runner.cancel(handle.id).expect("cancel");
    hold.store(false, Ordering::SeqCst);
    let done = settle(&world.runner, handle.id);
    assert_eq!(done.status, TaskStatus::Cancelled);
    assert_eq!(done.checkpoint.as_deref(), Some([1_u8].as_slice()));
    assert_eq!(done.error, None);
}

#[test]
fn a_panicking_task_is_failed_and_the_runner_keeps_going() {
    let world = world();
    world.runner.register(Arc::new(Panicking {
        kind: TaskKind::LibraryScan,
    }));
    let panicked = world
        .runner
        .request(TaskKind::LibraryScan, user(1), None)
        .expect("panic");
    let done = settle(&world.runner, panicked.id);
    assert_eq!(done.status, TaskStatus::Failed);
    assert_eq!(done.error.as_deref(), Some("panicked"));
    world.runner.register(Arc::new(Completing {
        kind: TaskKind::Backup,
        runs: Arc::new(AtomicUsize::new(0)),
    }));
    let next = world
        .runner
        .request(TaskKind::Backup, user(1), None)
        .expect("next");
    let ok = settle(&world.runner, next.id);
    assert_eq!(ok.status, TaskStatus::Succeeded);
}

#[test]
fn a_failed_handler_records_its_message() {
    let world = world();
    world.runner.register(Arc::new(Failing {
        kind: TaskKind::Backup,
    }));
    let handle = world
        .runner
        .request(TaskKind::Backup, user(2), None)
        .expect("fail");
    let done = settle(&world.runner, handle.id);
    assert_eq!(done.status, TaskStatus::Failed);
    assert_eq!(done.error.as_deref(), Some("no space"));
}

#[test]
fn a_checkpointed_task_resumes_after_the_runner_restarts() {
    let world = world();
    let at_checkpoint = Arc::new(AtomicBool::new(false));
    let hold = Arc::new(AtomicBool::new(true));
    let saw = Arc::new(Mutex::new(Vec::new()));
    world.runner.register(Arc::new(Holding {
        kind: TaskKind::LibraryScan,
        at_checkpoint: Arc::clone(&at_checkpoint),
        hold: Arc::clone(&hold),
        saw: Arc::clone(&saw),
    }));
    let handle = world
        .runner
        .request(TaskKind::LibraryScan, user(1), None)
        .expect("run");
    until(&world.runner, handle.id, |snap| {
        snap.checkpoint.as_deref() == Some([1].as_slice())
    });
    let id = handle.id;
    let world = reopen(world);
    world.runner.register(Arc::new(Holding {
        kind: TaskKind::LibraryScan,
        at_checkpoint: Arc::new(AtomicBool::new(false)),
        hold: Arc::new(AtomicBool::new(false)),
        saw: Arc::clone(&saw),
    }));
    let done = settle(&world.runner, id);
    assert_eq!(done.status, TaskStatus::Succeeded);
    assert_eq!(done.checkpoint.as_deref(), Some([2_u8].as_slice()));
    assert_eq!(*saw.lock().expect("saw"), vec![0, 1]);
}

#[test]
fn expiry_sweeps_run_on_the_built_in_schedule() {
    let world = world();
    let count = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&count);
    world.runner.on_expiry(move |_| {
        counted.fetch_add(1, Ordering::SeqCst);
    });
    world.runner.tick();
    assert_eq!(count.load(Ordering::SeqCst), 0);
    world.manual.advance(1_000);
    world.runner.tick();
    assert_eq!(count.load(Ordering::SeqCst), 1);
    world.runner.tick();
    assert_eq!(count.load(Ordering::SeqCst), 1);
    world.manual.advance(1_000);
    world.runner.tick();
    assert_eq!(count.load(Ordering::SeqCst), 2);
}

#[test]
fn a_path_longer_than_the_limit_is_refused() {
    let world = world();
    let long = "a".repeat(MAX_PATH + 1);
    assert_eq!(
        world
            .runner
            .request(TaskKind::PathRefresh, user(1), Some(&long)),
        Err(TaskError::PathTooLong {
            length: MAX_PATH + 1
        })
    );
    assert_eq!(world.runner.list().expect("list"), []);
    let exact = "a".repeat(MAX_PATH);
    let handle = world
        .runner
        .request(TaskKind::PathRefresh, user(1), Some(&exact))
        .expect("exact");
    assert_eq!(
        world
            .runner
            .snapshot(handle.id)
            .expect("snap")
            .path
            .as_deref(),
        Some(exact.as_str())
    );
}

#[test]
fn cancel_of_a_waiting_job_never_starts_it() {
    let world = world();
    let handle = world
        .runner
        .request(TaskKind::Rebuild, user(1), None)
        .expect("queued");
    world.runner.cancel(handle.id).expect("cancel");
    world.runner.register(Arc::new(Completing {
        kind: TaskKind::Rebuild,
        runs: Arc::new(AtomicUsize::new(0)),
    }));
    thread::sleep(Duration::from_millis(40));
    let snap = world.runner.snapshot(handle.id).expect("snap");
    assert_eq!(snap.status, TaskStatus::Cancelled);
}

#[test]
fn snapshot_of_an_unknown_job_is_unknown() {
    let world = world();
    assert_eq!(
        world.runner.snapshot(TaskId(99)).err(),
        Some(TaskError::Unknown)
    );
    assert_eq!(
        world.runner.cancel(TaskId(99)).err(),
        Some(TaskError::Unknown)
    );
}

#[test]
fn list_orders_jobs_by_identifier() {
    let world = world();
    let a = world
        .runner
        .request(TaskKind::Backup, user(1), None)
        .expect("a");
    let b = world
        .runner
        .request(TaskKind::Purge, user(2), None)
        .expect("b");
    let listed: Vec<TaskId> = world
        .runner
        .list()
        .expect("list")
        .into_iter()
        .map(|row| row.id)
        .collect();
    assert_eq!(listed, vec![a.id, b.id]);
}

#[test]
fn status_names_round_trip() {
    let names = TaskStatus::Waiting.as_str();
    assert_eq!(names, "waiting");
    assert_eq!(TaskStatus::Running.as_str(), "running");
    assert_eq!(TaskStatus::Succeeded.as_str(), "succeeded");
    assert_eq!(TaskStatus::Failed.as_str(), "failed");
    assert_eq!(TaskStatus::Cancelled.as_str(), "cancelled");
}

#[test]
fn requested_at_is_the_clock() {
    let world = world();
    let handle = world
        .runner
        .request(TaskKind::Backup, user(1), None)
        .expect("queued");
    let snap = world.runner.snapshot(handle.id).expect("snap");
    assert_eq!(
        snap.requested_at,
        Timestamp::from_millis(testing::NOON).expect("noon")
    );
    assert_eq!(snap.started_at, None);
    assert_eq!(snap.finished_at, None);
}

#[test]
fn progress_is_stored_before_the_job_finishes() {
    struct Reporting {
        at: Arc<AtomicBool>,
        hold: Arc<AtomicBool>,
    }
    impl Handler for Reporting {
        fn kind(&self) -> TaskKind {
            TaskKind::Backup
        }
        fn run(&self, ctx: &TaskCtx) -> Result<Outcome, TaskError> {
            ctx.progress(40)?;
            self.at.store(true, Ordering::SeqCst);
            let deadline = Instant::now() + BOUND;
            while self.hold.load(Ordering::SeqCst) && !ctx.cancelled() && !ctx.stopping() {
                assert!(
                    Instant::now() < deadline,
                    "handler did not see cancel or stop"
                );
                thread::sleep(Duration::from_millis(2));
            }
            Ok(Outcome::Succeeded)
        }
    }
    let world = world();
    let at = Arc::new(AtomicBool::new(false));
    let hold = Arc::new(AtomicBool::new(true));
    world.runner.register(Arc::new(Reporting {
        at: Arc::clone(&at),
        hold: Arc::clone(&hold),
    }));
    let handle = world
        .runner
        .request(TaskKind::Backup, user(1), None)
        .expect("run");
    until(&world.runner, handle.id, |snap| snap.progress == 40);
    let snap = world.runner.snapshot(handle.id).expect("mid");
    assert_eq!(snap.status, TaskStatus::Running);
    assert_eq!(snap.progress, 40);
    hold.store(false, Ordering::SeqCst);
    let done = settle(&world.runner, handle.id);
    assert_eq!(done.status, TaskStatus::Succeeded);
    assert_eq!(done.progress, 40);
}

#[test]
fn dropping_the_runner_sets_stopping_on_a_running_job() {
    let world = world();
    let at_checkpoint = Arc::new(AtomicBool::new(false));
    let hold = Arc::new(AtomicBool::new(true));
    let saw_stop = Arc::new(AtomicBool::new(false));
    world.runner.register(Arc::new(Watch {
        kind: TaskKind::LibraryScan,
        at_checkpoint: Arc::clone(&at_checkpoint),
        hold: Arc::clone(&hold),
        saw_stop: Arc::clone(&saw_stop),
    }));
    let handle = world
        .runner
        .request(TaskKind::LibraryScan, user(1), None)
        .expect("run");
    until(&world.runner, handle.id, |snap| {
        snap.checkpoint.as_deref() == Some([1].as_slice())
    });
    drop(world.runner);
    let deadline = Instant::now() + BOUND;
    while !saw_stop.load(Ordering::SeqCst) {
        assert!(
            Instant::now() < deadline,
            "dropping the runner did not stop the worker"
        );
        thread::sleep(Duration::from_millis(5));
    }
    assert!(saw_stop.load(Ordering::SeqCst));
}

#[test]
fn production_limits_sweep_every_hour() {
    assert_eq!(
        Limits::production().sweep_every_ms,
        super::runner::SWEEP_EVERY_MS
    );
    assert_eq!(Limits::test().sweep_every_ms, 1_000);
}

#[test]
fn a_zero_sweep_period_never_fires() {
    let dir = TempDir::new("server-tasks-zero-sweep").expect("scratch");
    let host = HostFacts::probe(dir.path()).expect("probe");
    let root = DataRoot::open(dir.path(), &host, Policy::DEFAULT)
        .expect("data root")
        .root;
    let opened = Store::open(&root, &[SCHEMA], FIRST).expect("store");
    let store = Arc::new(opened.store);
    let (clock, manual) = testing::clock();
    let runner =
        Runner::open(Arc::clone(&store), clock, Limits { sweep_every_ms: 0 }).expect("runner");
    let count = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&count);
    runner.on_expiry(move |_| {
        counted.fetch_add(1, Ordering::SeqCst);
    });
    runner.tick();
    manual.advance(1_000);
    runner.tick();
    assert_eq!(count.load(Ordering::SeqCst), 0);
}

#[test]
fn a_handler_error_without_a_message_is_failed() {
    struct UnknownKind;
    impl Handler for UnknownKind {
        fn kind(&self) -> TaskKind {
            TaskKind::Purge
        }
        fn run(&self, _ctx: &TaskCtx) -> Result<Outcome, TaskError> {
            Err(TaskError::Unknown)
        }
    }
    let world = world();
    world.runner.register(Arc::new(UnknownKind));
    let handle = world
        .runner
        .request(TaskKind::Purge, user(1), None)
        .expect("run");
    let done = settle(&world.runner, handle.id);
    assert_eq!(done.status, TaskStatus::Failed);
    assert_eq!(done.error.as_deref(), Some("failed"));
}

#[test]
fn progress_above_one_hundred_is_stored_as_one_hundred() {
    struct Over;
    impl Handler for Over {
        fn kind(&self) -> TaskKind {
            TaskKind::Backup
        }
        fn run(&self, ctx: &TaskCtx) -> Result<Outcome, TaskError> {
            ctx.progress(255)?;
            ctx.checkpoint(&[7], 101)?;
            Ok(Outcome::Succeeded)
        }
    }
    let world = world();
    world.runner.register(Arc::new(Over));
    let handle = world
        .runner
        .request(TaskKind::Backup, user(1), None)
        .expect("run");
    let done = settle(&world.runner, handle.id);
    assert_eq!(done.status, TaskStatus::Succeeded);
    assert_eq!(done.progress, 100);
    assert_eq!(done.checkpoint.as_deref(), Some([7_u8].as_slice()));
}

#[test]
fn a_task_id_that_does_not_fit_i64_is_clamped() {
    assert_eq!(TaskId(u64::MAX).as_i64(), i64::MAX);
    assert_eq!(TaskId(1).as_i64(), 1);
}

#[test]
fn a_closed_store_refuses_a_request_and_a_cancel() {
    let world = world();
    let store = Arc::clone(&world.store);
    let _ = wait::wait(store.write(|_tx| -> Result<(), StoreError> {
        panic!("stop the writer");
    }));
    assert_eq!(
        world.runner.request(TaskKind::Backup, user(1), None).err(),
        Some(TaskError::Store(StoreError::Closed))
    );
    assert_eq!(
        world.runner.cancel(TaskId(1)).err(),
        Some(TaskError::Store(StoreError::Closed))
    );
    for _ in 0..gunmetal_store::readers::READERS {
        let _ = wait::wait(store.read(|_reader| panic!("stop a reader")));
    }
    assert_eq!(
        world.runner.snapshot(TaskId(1)).err(),
        Some(TaskError::Store(StoreError::Closed))
    );
    assert_eq!(
        world.runner.list().err(),
        Some(TaskError::Store(StoreError::Closed))
    );
}

#[test]
fn a_waiting_row_the_catalogue_rejects_is_skipped() {
    let world = world();
    wait::wait(world.store.write(|tx| {
        tx.execute(&gunmetal_fs::sqlite::Query::new(
            "INSERT INTO tasks (kind, principal, path, status, progress, requested_at) \
             VALUES ('backup', 'not-an-id', '', 'waiting', 0, 1)",
        ))
        .map(|_| ())
        .map_err(StoreError::from)
    }))
    .expect("planted");
    world.runner.register(Arc::new(Completing {
        kind: TaskKind::Backup,
        runs: Arc::new(AtomicUsize::new(0)),
    }));
    thread::sleep(Duration::from_millis(40));
    assert_eq!(
        world.runner.list().err(),
        Some(TaskError::Store(StoreError::Catalogue))
    );
}

#[test]
fn checkpoint_after_cancel_is_cancelled() {
    struct WaitThenCheckpoint {
        ready: Arc<AtomicBool>,
    }
    impl Handler for WaitThenCheckpoint {
        fn kind(&self) -> TaskKind {
            TaskKind::Purge
        }
        fn run(&self, ctx: &TaskCtx) -> Result<Outcome, TaskError> {
            self.ready.store(true, Ordering::SeqCst);
            let deadline = Instant::now() + BOUND;
            while !ctx.cancelled() && !ctx.stopping() {
                assert!(
                    Instant::now() < deadline,
                    "handler did not see cancel or stop"
                );
                thread::sleep(Duration::from_millis(2));
            }
            ctx.checkpoint(&[1], 10)?;
            Ok(Outcome::Succeeded)
        }
    }
    let world = world();
    let ready = Arc::new(AtomicBool::new(false));
    world.runner.register(Arc::new(WaitThenCheckpoint {
        ready: Arc::clone(&ready),
    }));
    let handle = world
        .runner
        .request(TaskKind::Purge, user(1), None)
        .expect("run");
    let deadline = Instant::now() + BOUND;
    while !ready.load(Ordering::SeqCst) {
        assert!(Instant::now() < deadline, "handler did not start");
        thread::sleep(Duration::from_millis(5));
    }
    world.runner.cancel(handle.id).expect("cancel");
    let done = settle(&world.runner, handle.id);
    assert_eq!(done.status, TaskStatus::Cancelled);
}

#[test]
fn progress_after_cancel_is_cancelled() {
    struct WaitThenProgress {
        ready: Arc<AtomicBool>,
    }
    impl Handler for WaitThenProgress {
        fn kind(&self) -> TaskKind {
            TaskKind::Rebuild
        }
        fn run(&self, ctx: &TaskCtx) -> Result<Outcome, TaskError> {
            self.ready.store(true, Ordering::SeqCst);
            let deadline = Instant::now() + BOUND;
            while !ctx.cancelled() && !ctx.stopping() {
                assert!(
                    Instant::now() < deadline,
                    "handler did not see cancel or stop"
                );
                thread::sleep(Duration::from_millis(2));
            }
            ctx.progress(10)?;
            Ok(Outcome::Succeeded)
        }
    }
    let world = world();
    let ready = Arc::new(AtomicBool::new(false));
    world.runner.register(Arc::new(WaitThenProgress {
        ready: Arc::clone(&ready),
    }));
    let handle = world
        .runner
        .request(TaskKind::Rebuild, user(1), None)
        .expect("run");
    let deadline = Instant::now() + BOUND;
    while !ready.load(Ordering::SeqCst) {
        assert!(Instant::now() < deadline, "handler did not start");
        thread::sleep(Duration::from_millis(5));
    }
    world.runner.cancel(handle.id).expect("cancel");
    let done = settle(&world.runner, handle.id);
    assert_eq!(done.status, TaskStatus::Cancelled);
}

#[test]
fn checkpoint_after_stop_is_interrupted() {
    struct WaitThenCheckpointOnStop {
        ready: Arc<AtomicBool>,
    }
    impl Handler for WaitThenCheckpointOnStop {
        fn kind(&self) -> TaskKind {
            TaskKind::PathRefresh
        }
        fn run(&self, ctx: &TaskCtx) -> Result<Outcome, TaskError> {
            self.ready.store(true, Ordering::SeqCst);
            let deadline = Instant::now() + BOUND;
            while !ctx.stopping() {
                assert!(Instant::now() < deadline, "handler did not see stopping");
                thread::sleep(Duration::from_millis(2));
            }
            ctx.checkpoint(&[1], 10)?;
            Ok(Outcome::Succeeded)
        }
    }
    let world = world();
    let ready = Arc::new(AtomicBool::new(false));
    world.runner.register(Arc::new(WaitThenCheckpointOnStop {
        ready: Arc::clone(&ready),
    }));
    let handle = world
        .runner
        .request(TaskKind::PathRefresh, user(1), None)
        .expect("run");
    let deadline = Instant::now() + BOUND;
    while !ready.load(Ordering::SeqCst) {
        assert!(Instant::now() < deadline, "handler did not start");
        thread::sleep(Duration::from_millis(5));
    }
    drop(world.runner);
    assert!(ready.load(Ordering::SeqCst));
    let _ = handle;
}

#[test]
fn run_one_skips_a_job_that_is_not_waiting() {
    let world = world();
    let handle = world
        .runner
        .request(TaskKind::Backup, user(1), None)
        .expect("queued");
    world.runner.cancel(handle.id).expect("cancel");
    let runs = Arc::new(AtomicUsize::new(0));
    world.runner.run_one(
        handle.id,
        Arc::new(Completing {
            kind: TaskKind::Backup,
            runs: Arc::clone(&runs),
        }),
    );
    assert_eq!(runs.load(Ordering::SeqCst), 0);
}

#[test]
fn dropping_a_runner_without_a_worker_handle_is_quiet() {
    let world = world();
    let World {
        mut runner, store, ..
    } = world;
    let worker = runner.take_worker().expect("worker");
    drop(runner);
    worker.join().expect("joined");
    drop(store);
}

#[test]
fn a_dropped_table_refuses_open() {
    let world = world();
    wait::wait(world.store.write(|tx| {
        tx.execute(&gunmetal_fs::sqlite::Query::new("DROP TABLE tasks"))
            .expect("drop");
        Ok::<(), StoreError>(())
    }))
    .expect("dropped");
    assert_eq!(
        world
            .runner
            .list()
            .err()
            .map(|error| matches!(error, TaskError::Store(_))),
        Some(true)
    );
    assert_eq!(
        world
            .runner
            .snapshot(TaskId(1))
            .err()
            .map(|error| matches!(error, TaskError::Store(_))),
        Some(true)
    );
    let opened = Runner::open(
        Arc::clone(&world.store),
        world.clock.clone(),
        Limits::test(),
    );
    assert_eq!(
        opened
            .err()
            .map(|error| matches!(error, TaskError::Store(_))),
        Some(true)
    );
}

#[test]
fn mark_running_a_job_that_is_not_waiting_does_not_start() {
    let world = world();
    let handle = world
        .runner
        .request(TaskKind::Backup, user(1), None)
        .expect("queued");
    world.runner.cancel(handle.id).expect("cancel");
    let started = super::runner::take_running(
        &world.store,
        Timestamp::from_millis(testing::NOON).expect("noon"),
        handle.id,
    );
    assert_eq!(started, None);
}
