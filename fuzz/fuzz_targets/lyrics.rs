//! Feeds arbitrary bytes to the lyrics readers through their harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::lyrics::run(data);
});
