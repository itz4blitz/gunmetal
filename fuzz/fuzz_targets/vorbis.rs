//! Feeds arbitrary bytes to the Vorbis stream header parsers through their
//! harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::vorbis::run(data);
});
