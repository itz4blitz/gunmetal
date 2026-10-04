//! Feeds arbitrary bytes to the public-identifier parser through its
//! harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::id::run(data);
});
