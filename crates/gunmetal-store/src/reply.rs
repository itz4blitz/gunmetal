//! The answer to one call on the store, delivered by the thread that did
//! the work.
//!
//! A [`Reply`] is a future, so async code awaits it without blocking its
//! executor, and it needs no particular runtime: the writer or reader
//! thread stores the result and wakes whoever polled. The work is queued
//! when the call is made, not when the reply is first polled, so dropping a
//! reply does not cancel it.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Waker};

use crate::store::StoreError;

/// What the two halves share.
struct Shared<R> {
    result: Option<Result<R, StoreError>>,
    waker: Option<Waker>,
}

/// The result of a call on the store, once the thread doing the work has
/// finished it.
///
/// It resolves to [`StoreError::Closed`] when that thread stopped before it
/// could answer.
#[must_use = "the reply carries the call's result"]
pub struct Reply<R> {
    shared: Arc<Mutex<Shared<R>>>,
}

/// The half that delivers the result. Dropping it without sending, as a
/// panic in the work does, delivers [`StoreError::Closed`], so a reply
/// never waits forever.
pub(crate) struct Completer<R> {
    shared: Arc<Mutex<Shared<R>>>,
    result: Option<Result<R, StoreError>>,
}

/// A connected completer and reply.
pub(crate) fn channel<R>() -> (Completer<R>, Reply<R>) {
    let shared = Arc::new(Mutex::new(Shared {
        result: None,
        waker: None,
    }));
    (
        Completer {
            shared: Arc::clone(&shared),
            result: None,
        },
        Reply { shared },
    )
}

/// Locks `mutex`. Nothing panics while holding one of the store's locks,
/// so a poisoned lock still holds consistent data.
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl<R> Completer<R> {
    /// Delivers `result` to the reply.
    pub(crate) fn send(mut self, result: Result<R, StoreError>) {
        self.result = Some(result);
    }
}

impl<R> Drop for Completer<R> {
    fn drop(&mut self) {
        let result = self.result.take().unwrap_or(Err(StoreError::Closed));
        let waker = {
            let mut shared = lock(&self.shared);
            shared.result = Some(result);
            shared.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

impl<R> Future for Reply<R> {
    type Output = Result<R, StoreError>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let mut shared = lock(&self.shared);
        if let Some(result) = shared.result.take() {
            Poll::Ready(result)
        } else {
            shared.waker = Some(context.waker().clone());
            Poll::Pending
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::pin::pin;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::task::Wake;

    /// Counts how often it is woken.
    struct Count(AtomicUsize);

    impl Wake for Count {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Polls `reply` once, with a waker that does nothing.
    fn poll_once<R>(reply: Reply<R>) -> Poll<Result<R, StoreError>> {
        pin!(reply).poll(&mut Context::from_waker(Waker::noop()))
    }

    #[test]
    fn a_sent_result_is_ready_at_the_first_poll() {
        let (completer, reply) = channel();
        completer.send(Ok(7));
        assert_eq!(poll_once(reply), Poll::Ready(Ok(7)));

        let (completer, reply) = channel::<i32>();
        completer.send(Err(StoreError::Catalogue));
        assert_eq!(poll_once(reply), Poll::Ready(Err(StoreError::Catalogue)));
    }

    #[test]
    fn a_completer_dropped_without_sending_answers_closed() {
        let (completer, reply) = channel::<i32>();
        drop(completer);
        assert_eq!(poll_once(reply), Poll::Ready(Err(StoreError::Closed)));
    }

    #[test]
    fn a_reply_polled_early_is_pending_and_is_woken_once_by_the_result() {
        let (completer, reply) = channel();
        let count = Arc::new(Count(AtomicUsize::new(0)));
        let waker = Waker::from(Arc::clone(&count));
        let mut context = Context::from_waker(&waker);
        let mut reply = pin!(reply);
        assert_eq!(reply.as_mut().poll(&mut context), Poll::Pending);
        assert_eq!(count.0.load(Ordering::Relaxed), 0);
        completer.send(Ok("done"));
        assert_eq!(count.0.load(Ordering::Relaxed), 1);
        assert_eq!(reply.as_mut().poll(&mut context), Poll::Ready(Ok("done")));
    }
}
