//! An in-process test client: it sends a request through a router and
//! returns the whole response, with no socket and no async runtime, so a
//! test can compare status, every header and the body at once.
//!
//! It calls the same [`axum::Router`] the listener serves, so what a test
//! sees is what a client would get from the pipeline. Limits that belong to
//! the connection (header size, timeouts, framing) are the listener's and
//! are not exercised here.

use core::future::Future;
use core::pin::pin;
use core::task::{Context, Poll, Waker};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::handler::Handler;
use axum::http::Request;
use axum::routing::any_service;

/// A whole response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestResponse {
    /// The status code.
    pub status: u16,
    /// Every header, lower-case name first, sorted.
    pub headers: Vec<(String, String)>,
    /// The body.
    pub body: Vec<u8>,
}

/// Sends requests to a router in the calling thread.
#[derive(Debug, Clone)]
pub struct TestClient {
    router: Router,
}

impl TestClient {
    /// A client for a router.
    #[must_use]
    pub const fn new(router: Router) -> Self {
        Self { router }
    }

    /// Sends one request and waits for the whole response.
    #[must_use]
    pub fn send(&self, request: Request<Body>) -> TestResponse {
        let service = any_service(self.router.clone());
        block_on(async move {
            let (parts, body) = service.call(request, ()).await.into_parts();
            let mut headers: Vec<(String, String)> = parts
                .headers
                .iter()
                .map(|(name, value)| {
                    (
                        name.as_str().to_owned(),
                        String::from_utf8_lossy(value.as_bytes()).into_owned(),
                    )
                })
                .collect();
            headers.sort();
            TestResponse {
                status: parts.status.as_u16(),
                headers,
                body: to_bytes(body, usize::MAX)
                    .await
                    .map(|bytes| bytes.to_vec())
                    .unwrap_or_default(),
            }
        })
    }
}

/// Runs a future to its end on this thread. Nothing in the pipeline waits
/// on I/O, so a future that is not ready is simply polled again.
fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
        std::thread::yield_now();
    }
}
