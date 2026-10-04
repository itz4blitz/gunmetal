//! Feeds arbitrary bytes to the `WebAuthn` CBOR reader through its harness.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = gunmetal_fuzz::webauthn_cbor::run(data);
});
