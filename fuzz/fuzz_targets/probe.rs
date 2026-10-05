//! Feeds arbitrary bytes to the file probe through its harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::probe::run(data);
});
