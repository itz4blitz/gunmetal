//! Feeds arbitrary bytes to the inbound-link reader through its harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::deeplink::run(data);
});
