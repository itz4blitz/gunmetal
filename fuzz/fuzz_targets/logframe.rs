//! Feeds arbitrary bytes to the log-segment framer through its harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::logframe::run(data);
});
