//! Reads arbitrary bytes as a recipe for a FLAC stream with valid framing
//! and feeds the stream to the FLAC metadata parser (SEC-MED-031).

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::flac_structure::run(data);
});
