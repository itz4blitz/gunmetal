//! Feeds arbitrary bytes, and an extension hint chosen by the first of them,
//! to format detection through its harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::detect::run(data);
});
