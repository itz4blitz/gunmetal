//! Drive a store [`Reply`] on this thread until the writer or a reader
//! answers. The work was queued when the call was made.

use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, Thread};
use std::time::Duration;

/// How long to stay parked if the waker never runs. The completer still
/// wakes on the fast path; this bound is so a `wake -> ()` mutant cannot
/// hang the gate (CONTRIBUTING.md: a hang is not a detection).
const PARK_BOUND: Duration = Duration::from_millis(80);

/// Parks the thread that is waiting for a reply.
struct Unpark(Thread);

impl Wake for Unpark {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

/// Polls `future` until it is ready.
pub(crate) fn wait<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(Unpark(thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
        thread::park_timeout(PARK_BOUND);
    }
}

#[cfg(test)]
mod tests {
    use super::wait;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::{Arc, Mutex, PoisonError};
    use std::task::{Context, Poll, Waker};
    use std::thread;
    use std::time::{Duration, Instant};

    struct State {
        ready: bool,
        waker: Option<Waker>,
    }

    struct ReadyAfterWake {
        state: Arc<Mutex<State>>,
    }

    impl Future for ReadyAfterWake {
        type Output = &'static str;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            if state.ready {
                Poll::Ready("ok")
            } else {
                state.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }

    #[test]
    fn wait_returns_when_the_waker_runs_without_waiting_out_the_park_timeout() {
        let state = Arc::new(Mutex::new(State {
            ready: false,
            waker: None,
        }));
        let future = ReadyAfterWake {
            state: Arc::clone(&state),
        };
        let worker = thread::spawn(move || wait(future));
        let deadline = Instant::now() + Duration::from_millis(100);
        loop {
            {
                let state = state.lock().unwrap_or_else(PoisonError::into_inner);
                if state.waker.is_some() {
                    break;
                }
            }
            assert!(
                Instant::now() < deadline,
                "wait did not poll and store a waker"
            );
            thread::sleep(Duration::from_millis(1));
        }
        {
            let mut state = state.lock().unwrap_or_else(PoisonError::into_inner);
            state.ready = true;
            state.waker.take().expect("waker").wake();
        }
        let started = Instant::now();
        assert_eq!(worker.join().expect("join"), "ok");
        assert!(
            started.elapsed() < Duration::from_millis(40),
            "wait stayed parked after the waker ran"
        );
    }
}
