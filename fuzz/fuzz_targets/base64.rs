//! Feeds arbitrary bytes to the base64 decoder through its harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::base64::run(data);
});
