//! Feeds arbitrary bytes to the search-segment reader through its harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::search_segment::run(data);
});
