//! A planted hang. The fuzz workflow's canary job fuzzes this target with
//! the settings every harness uses and passes only when the run fails with
//! a timeout artifact, which proves the 5 s per-input limit fails the fuzz
//! job (SEC-MED-029). It is not a harness, and nothing ships it.

#![no_main]

use std::thread;
use std::time::Duration;

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    // One comparison per byte, so coverage guidance finds the prefix a byte
    // at a time.
    if data.first() == Some(&b'H')
        && data.get(1) == Some(&b'A')
        && data.get(2) == Some(&b'N')
        && data.get(3) == Some(&b'G')
    {
        thread::sleep(Duration::from_secs(60));
    }
});
