//! Feeds arbitrary bytes to the `Range` header parser through its harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::http_range::run(data);
});
