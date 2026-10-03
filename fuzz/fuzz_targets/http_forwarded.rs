//! Feeds arbitrary header sections to the forwarding-header parser and the
//! path-class function through their harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::http_forwarded::run(data);
});
