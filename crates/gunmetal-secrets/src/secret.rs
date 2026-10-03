//! The wrapper every secret value lives in (SEC-OPS-013, SEC-IAM-095,
//! SEC-HIS-011).
//!
//! A [`Secret`] has no `Display`, no `PartialEq` and no serialisation, and
//! its `Debug` prints a fixed placeholder, so a secret cannot reach a log
//! line, an error message, an API response or a diagnostics bundle by being
//! formatted, compared or serialised. This crate depends on no serialisation
//! crate, and the orphan rule stops any other crate from implementing one
//! for `Secret`. Its value is wiped when it is dropped.
//!
//! [`Secret::expose`] is the one way to the value outside this crate, and a
//! `disallowed-methods` entry in `clippy.toml` rejects every call outside
//! the modules on the xtask exception list. Keyed operations happen inside
//! this crate, so no other crate needs it to sign, verify or hash.

use core::fmt;
use core::hint::black_box;

/// A value that can overwrite itself with zeros.
pub trait Wipe {
    /// Overwrites the value with zeros.
    fn wipe(&mut self);
}

impl<const N: usize> Wipe for [u8; N] {
    fn wipe(&mut self) {
        self.fill(0);
        // Without `unsafe` there is no volatile write, so the zeroed bytes
        // are handed to an opaque function, which keeps the compiler from
        // dropping the writes as dead stores.
        black_box(self);
    }
}

impl Wipe for Vec<u8> {
    fn wipe(&mut self) {
        self.fill(0);
        black_box(self);
    }
}

/// A secret value: never printed, compared with `==` or serialised, and
/// wiped when dropped.
pub struct Secret<T: Wipe>(T);

impl<T: Wipe> Secret<T> {
    /// Wraps `value`.
    #[must_use]
    pub const fn new(value: T) -> Self {
        Self(value)
    }

    /// The value. Outside this crate only the modules on the xtask
    /// exception list may call this, such as the egress client writing a
    /// replayed secret into a request.
    #[must_use]
    pub const fn expose(&self) -> &T {
        &self.0
    }

    /// The value, for this crate's keyed operations.
    pub(crate) const fn value(&self) -> &T {
        &self.0
    }

    /// The value, for filling it in place.
    pub(crate) const fn value_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T: Wipe> Drop for Secret<T> {
    fn drop(&mut self) {
        self.0.wipe();
    }
}

impl<T: Wipe> fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret([redacted])")
    }
}

/// Compile-fail tests: what no code may do with a secret. Rustdoc on stable
/// does not check which error a compile-fail test produced, so each shares
/// its imports with the control below, which compiles; a mistake in the
/// imports would fail the control.
#[cfg(doctest)]
mod compile_fail {
    /// Control: a secret can be made and debug-formatted to its placeholder.
    ///
    /// ```
    /// use gunmetal_secrets::Secret;
    ///
    /// let secret = Secret::new([7_u8; 32]);
    /// let other = Secret::new([7_u8; 32]);
    /// assert_eq!(format!("{secret:?} {other:?}"), "Secret([redacted]) Secret([redacted])");
    /// ```
    struct Control;

    /// Verifies: SEC-OPS-013, SEC-IAM-095, SEC-TM-049
    ///
    /// A secret has no `Display`, so it cannot be formatted into a log line
    /// or an error message.
    ///
    /// ```compile_fail
    /// use gunmetal_secrets::Secret;
    ///
    /// let secret = Secret::new([7_u8; 32]);
    /// let other = Secret::new([7_u8; 32]);
    /// let _ = format!("{secret} {other:?}");
    /// ```
    struct NoDisplay;

    /// Verifies: SEC-OPS-013
    ///
    /// A secret has no `==`, which would compare in variable time and tempt
    /// code into checking a guess against it.
    ///
    /// ```compile_fail
    /// use gunmetal_secrets::Secret;
    ///
    /// let secret = Secret::new([7_u8; 32]);
    /// let other = Secret::new([7_u8; 32]);
    /// let _ = format!("{secret:?} {other:?}");
    /// let _ = secret == other;
    /// ```
    struct NoEquality;

    /// A secret cannot be copied out by cloning, so its one copy is the one
    /// that is wiped.
    ///
    /// ```compile_fail
    /// use gunmetal_secrets::Secret;
    ///
    /// let secret = Secret::new([7_u8; 32]);
    /// let other = Secret::new([7_u8; 32]);
    /// let _ = format!("{secret:?} {other:?}");
    /// let _ = secret.clone();
    /// ```
    struct NoClone;
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::{Secret, Wipe};

    /// A value that counts how often it is wiped.
    struct Probe<'a>(&'a Cell<u32>);

    impl Wipe for Probe<'_> {
        fn wipe(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    /// Verifies: SEC-OPS-013, SEC-IAM-095
    #[test]
    fn debug_prints_only_the_placeholder() {
        let secret = Secret::new(*b"canary-canary-canary-canary-cana");
        assert_eq!(format!("{secret:?}"), "Secret([redacted])");
        assert_eq!(format!("{secret:#?}"), "Secret([redacted])");
        assert_eq!(format!("{:?}", Some(&secret)), "Some(Secret([redacted]))");
    }

    #[test]
    fn wiping_an_array_zeroes_every_byte() {
        let mut bytes = [0xA5_u8; 32];
        bytes.wipe();
        assert_eq!(bytes, [0; 32]);
    }

    #[test]
    fn wiping_a_vector_zeroes_every_byte_and_keeps_its_length() {
        let mut bytes = vec![0xA5_u8; 40];
        bytes.wipe();
        assert_eq!(bytes, vec![0; 40]);
    }

    /// Verifies: SEC-TM-049
    #[test]
    fn dropping_a_secret_wipes_its_value_once() {
        let wiped = Cell::new(0);
        let secret = Secret::new(Probe(&wiped));
        assert_eq!(wiped.get(), 0);
        drop(secret);
        assert_eq!(wiped.get(), 1);
    }

    #[test]
    #[expect(
        clippy::disallowed_methods,
        reason = "the test of the one method that exposes a secret"
    )]
    fn the_value_is_reachable_through_expose_and_inside_the_crate() {
        let mut secret = Secret::new([1_u8, 2, 3]);
        assert_eq!(secret.expose(), &[1, 2, 3]);
        secret.value_mut()[1] = 9;
        assert_eq!(secret.value(), &[1, 9, 3]);
        assert_eq!(secret.expose(), &[1, 9, 3]);
    }

    /// Verifies: SEC-HIS-011
    ///
    /// No secret can be serialised into an API response: this crate depends
    /// on no serialisation crate, so it implements no serialisation trait
    /// for `Secret`, and the orphan rule forbids any other crate to.
    #[test]
    fn the_crate_depends_on_no_serialisation_crate() {
        let manifest = include_str!("../Cargo.toml");
        let dependencies: Vec<&str> = manifest
            .lines()
            .skip_while(|line| *line != "[dependencies]")
            .skip(1)
            .take_while(|line| !line.starts_with('['))
            .filter_map(|line| line.split([' ', '.', '=']).find(|name| !name.is_empty()))
            .collect();
        assert_eq!(
            dependencies,
            [
                "chacha20poly1305",
                "getrandom",
                "gunmetal-core",
                "gunmetal-fs",
                "hkdf",
                "hmac",
                "sha2"
            ]
        );
    }

    /// Verifies: SEC-TM-049, SEC-STD-023
    ///
    /// The cipher and MAC crates copy the key into their own state. Their
    /// `Drop` impls wipe that copy only when the `zeroize` feature is on.
    /// `zeroize` is not a direct dependency; the features pull it in.
    #[test]
    fn the_cipher_and_mac_crates_enable_zeroize() {
        let manifest = include_str!("../Cargo.toml");
        assert_eq!(
            manifest
                .lines()
                .filter(|line| {
                    line.contains("chacha20poly1305")
                        || line.starts_with("hmac ")
                        || line.starts_with("sha2")
                })
                .collect::<Vec<_>>(),
            [
                r#"chacha20poly1305 = { version = "0.11.0", default-features = false, features = ["zeroize"] }"#,
                r#"hmac = { version = "0.13.0", default-features = false, features = ["zeroize"] }"#,
                r#"sha2 = { workspace = true, features = ["zeroize"] }"#,
            ]
        );
        let lock = include_str!("../../../Cargo.lock");
        let chacha = lock
            .split("[[package]]\n")
            .find(|block| block.starts_with("name = \"chacha20poly1305\"\nversion = \"0.11.0\""))
            .unwrap_or("");
        let digest = lock
            .split("[[package]]\n")
            .find(|block| block.starts_with("name = \"digest\"\nversion = \"0.11.3\""))
            .unwrap_or("");
        assert_eq!(
            (
                chacha.contains("\"zeroize\""),
                digest.contains("\"zeroize\""),
                lock.split("[[package]]\n")
                    .any(|block| block.starts_with("name = \"missing\"\nversion = \"0.0.0\""))
            ),
            (true, true, false)
        );
    }
}
