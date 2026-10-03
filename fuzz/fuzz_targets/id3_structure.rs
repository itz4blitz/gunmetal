//! Feeds ID3v2 tags with valid framing around fuzzed fields to the parser
//! through the structure-aware harness (SEC-MED-031).

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::id3_structure::run(data);
});
