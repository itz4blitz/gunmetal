//! Feeds the Ogg packet parser streams with valid framing around damaged
//! fields, described by arbitrary bytes, through its structure-aware
//! harness (SEC-MED-031).

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::ogg_structure::run(data);
});
