//! A planted memory exhaustion. The fuzz workflow's canary job fuzzes this
//! target with the settings every harness uses and passes only when the run
//! fails with an out-of-memory artifact, which proves the 1 GiB limit fails
//! the fuzz job (SEC-MED-029). It is not a harness, and nothing ships it.

#![no_main]

use std::hint::black_box;
use std::thread;
use std::time::Duration;

/// Twice the fuzz job's memory limit, in bytes.
const TWO_GIB: usize = 2 << 30;

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    // One comparison per byte, so coverage guidance finds the prefix a byte
    // at a time.
    if data.first() == Some(&b'B')
        && data.get(1) == Some(&b'I')
        && data.get(2) == Some(&b'G')
    {
        // Filling the block touches every page, so the process really holds
        // it. libFuzzer checks its memory once a second, well inside the
        // 5 s per-input limit.
        let block = vec![1_u8; TWO_GIB];
        black_box(&block);
        thread::sleep(Duration::from_secs(2));
    }
});
