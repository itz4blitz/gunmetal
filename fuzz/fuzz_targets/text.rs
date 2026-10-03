//! Feeds arbitrary bytes to the text decoders and the ingest normaliser
//! through their harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::text::text(data);
});
