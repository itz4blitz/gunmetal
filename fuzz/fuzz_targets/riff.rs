//! Feeds arbitrary bytes to the WAV parser through its harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::riff::run(data);
});
