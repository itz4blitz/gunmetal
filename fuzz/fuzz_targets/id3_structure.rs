<<<<<<< HEAD
//! Feeds recipes to the ID3 family's structure-aware harness, which builds
//! tags with sound framing around fuzzed fields (SEC-MED-031).
=======
//! Feeds ID3v2 tags with valid framing around fuzzed fields to the parser
//! through the structure-aware harness (SEC-MED-031).
>>>>>>> origin/wp/wp-010

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::id3_structure::run(data);
});
