//! Feeds arbitrary bytes to the MP4 audio probe through its harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::mp4_probe::run(data);
});
