//! Feeds arbitrary bytes to the path rules through their harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::path::run(data);
});
