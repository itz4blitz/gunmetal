//! A test fixture for `xtask native-code`: a crate that uses unsafe.

pub fn first(bytes: &[u8]) -> u8 {
    unsafe { *bytes.get_unchecked(0) }
}
