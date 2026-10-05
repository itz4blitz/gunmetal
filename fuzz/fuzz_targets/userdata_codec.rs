//! Feeds arbitrary bytes to the user-event reader through its harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::userdata_codec::run(data);
});
