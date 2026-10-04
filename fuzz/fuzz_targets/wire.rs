//! Feeds arbitrary bytes to the wire codec's frame reader, payload decoder
//! and version negotiation through their harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::wire::run(data);
});
