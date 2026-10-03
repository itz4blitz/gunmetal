//! Feeds arbitrary bytes to the RFC 3339 reader through its harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::time::run(data);
});
