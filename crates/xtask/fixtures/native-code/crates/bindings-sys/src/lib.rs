//! A test fixture for `xtask native-code`: bindings to a native library.

unsafe extern "C" {
    pub fn bindings_version() -> i32;
}
