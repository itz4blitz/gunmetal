//! Feeds arbitrary bytes to the MPEG audio parsers through their harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::mpa::run(data);
});
