//! Feeds arbitrary bytes to the ID3v2 parser through its harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::id3v2_tag::run(data);
});
