//! Feeds one input to both MP4 structure-aware generators.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::mp4_structure::run(data);
    let _ = gunmetal_fuzz::mp4_sample_table::structured(data);
});
