//! Feeds recipes to the ID3 family's structure-aware harness, which builds
//! tags with sound framing around fuzzed fields (SEC-MED-031).

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::id3_structure::run(data);
});
