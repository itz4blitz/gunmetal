//! Feeds arbitrary bytes to the streaming decompression helper through its
//! harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::inflate::run(data);
});
