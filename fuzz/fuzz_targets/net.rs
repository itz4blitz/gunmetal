//! Feeds arbitrary bytes to the network parser and the address classifier
//! through their harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::net::run(data);
});
