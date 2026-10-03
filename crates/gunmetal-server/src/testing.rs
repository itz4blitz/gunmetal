//! Helpers the crate's unit tests share: a clock a test moves, an output a
//! test reads, and a security sink a test inspects.

use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use gunmetal_core::audit_event::{AuditUnavailable, SecurityEvent, SecuritySink};
use gunmetal_core::time::{Clock, Timestamp};
use gunmetal_testkit::clock::ManualClock;

/// 2026-10-03T12:00:00.000Z, where the tests' clocks start.
pub const NOON: i64 = 1_791_028_800_000;

/// The testkit's manual clock as the core's [`Clock`].
pub struct TestClock(pub Arc<ManualClock>);

impl Clock for TestClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_millis(self.0.now_ms()).expect("the test clock stays in range")
    }
}

/// A clock at [`NOON`] and the handle that moves it.
pub fn clock() -> (Arc<TestClock>, Arc<ManualClock>) {
    let manual = Arc::new(ManualClock::at(NOON));
    (Arc::new(TestClock(Arc::clone(&manual))), manual)
}

/// Lines written to a buffer the test can read.
#[derive(Clone, Default)]
pub struct Capture(Arc<Mutex<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().expect("capture").extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Capture {
    /// Everything written so far.
    pub fn text(&self) -> String {
        String::from_utf8(self.0.lock().expect("capture").clone()).expect("UTF-8 output")
    }
}

/// A sink that keeps every event, or refuses every one.
pub struct Recording {
    events: Mutex<Vec<SecurityEvent>>,
    accept: bool,
}

impl Recording {
    /// A sink that accepts every event, or none.
    pub fn new(accept: bool) -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            accept,
        }
    }

    /// The events accepted so far.
    pub fn events(&self) -> Vec<SecurityEvent> {
        self.events.lock().expect("events").clone()
    }
}

impl SecuritySink for Recording {
    fn record(&self, event: SecurityEvent) -> Result<(), AuditUnavailable> {
        if self.accept {
            self.events.lock().expect("events").push(event);
            Ok(())
        } else {
            Err(AuditUnavailable)
        }
    }
}

#[test]
fn the_capture_reads_back_what_was_written_and_flushes() {
    let mut capture = Capture::default();
    capture.write_all(b"one\n").expect("written");
    assert_eq!(capture.flush().ok(), Some(()));
    assert_eq!(capture.text(), "one\n");
}
