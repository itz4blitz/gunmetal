//! Feeds arbitrary bytes to the AIFF parser through its harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::aiff::run(data);
});
