//! Helpers shared by this crate's unit tests.

use std::fmt::Write as _;

use crate::root::{ROOT_LEN, Root};

/// The bytes 0 to 31: the root secret the tests derive keys from. The
/// expected keys, tags and sealed values in the tests were computed from it
/// outside this crate, with Python's `hmac` and `hashlib` and with
/// libsodium.
pub const COUNTED: [u8; ROOT_LEN] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31,
];

/// A root whose secret is [`COUNTED`].
pub fn counted_root() -> Root {
    Root::read(&mut &COUNTED[..]).unwrap()
}

/// `bytes` in lower-case hexadecimal.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, byte| {
        write!(out, "{byte:02x}").unwrap();
        out
    })
}
