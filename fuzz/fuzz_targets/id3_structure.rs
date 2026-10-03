//! Feeds one input to both ID3 structure-aware generators (SEC-MED-031).

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::id3_structure::run(data);
    let _ = gunmetal_fuzz::id3_structure::id3v2(data);
});
