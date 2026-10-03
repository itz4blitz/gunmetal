//! Feeds arbitrary bytes to the MP4 structure-aware generators through
//! their harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::mp4_structure::run(data);
});
