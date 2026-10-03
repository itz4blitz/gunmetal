//! A planted crash. The fuzz workflow's canary job fuzzes this target with
//! the settings every harness uses and passes only when the run fails with
//! a crash artifact, which proves the fuzz job fails on a crash
//! (SEC-MED-029). It is not a harness, and nothing ships it.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    // One comparison per byte, so coverage guidance finds the prefix a byte
    // at a time.
    if data.first() == Some(&b'F')
        && data.get(1) == Some(&b'U')
        && data.get(2) == Some(&b'Z')
        && data.get(3) == Some(&b'Z')
    {
        panic!("the planted crash was found");
    }
});
