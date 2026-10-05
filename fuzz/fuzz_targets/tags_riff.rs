//! Feeds arbitrary bytes to the `INFO` list mapper through its harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::tags_riff::run(data);
});
