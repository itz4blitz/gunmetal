//! Feeds structure-aware MP4 trees to the audio probe through its harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::mp4_structure::run(data);
});
