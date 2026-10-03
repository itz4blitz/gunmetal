//! Feeds arbitrary bytes to the typed-value parsers through their harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::values::run(data);
});
