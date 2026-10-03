//! Feeds arbitrary bytes to the Opus stream header parsers through their
//! harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::opus::run(data);
});
