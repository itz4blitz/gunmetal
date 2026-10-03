//! Feeds recipes for FLAC frame headers with valid framing around fuzzed
//! fields to the frame index, through the structure-aware harness
//! (SEC-MED-031).

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::flac_structure::run(data);
});
