//! Feeds arbitrary bytes to the outside-link and return-target validators
//! through their harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::link::run(data);
});
