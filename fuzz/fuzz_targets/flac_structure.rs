//! Feeds one input to both FLAC structure-aware generators (SEC-MED-031).

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::flac_structure::run(data);
    let _ = gunmetal_fuzz::flac_structure::frames(data);
});
