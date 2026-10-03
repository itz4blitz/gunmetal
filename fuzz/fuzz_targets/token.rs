//! Feeds arbitrary bytes to the capability-token parser through its harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::token::run(data);
});
