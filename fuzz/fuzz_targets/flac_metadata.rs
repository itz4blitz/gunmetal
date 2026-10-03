//! Feeds arbitrary bytes to the FLAC metadata parser through its harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::flac_metadata::run(data);
});
