//! Feeds arbitrary bytes to the one-time-code parsers through their harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::otp::run(data);
});
