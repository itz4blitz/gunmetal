//! The wrapper every byte or string from outside arrives in (SEC-TM-031).

/// A value received from outside: a media file, a request, a provider.
pub struct Untrusted<T>(T);

impl<T> Untrusted<T> {
    /// Wraps a value as it arrives.
    #[must_use]
    pub const fn new(value: T) -> Self {
        Self(value)
    }

    /// Hands the value to a validator in this crate. Nothing outside the
    /// crate can reach it.
    pub(crate) fn into_inner(self) -> T {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::Untrusted;
    use std::borrow::Borrow;
    use std::ffi::{OsStr, OsString};
    use std::fmt::{Debug, Display};
    use std::net::ToSocketAddrs;
    use std::ops::Deref;
    use std::path::{Path, PathBuf};

    /// Implemented for every type through `()`, and once more through each
    /// marker below for every type that has the marker's trait. Naming
    /// `<T as AmbiguousIfImpl<_>>::check` therefore compiles only while `T`
    /// has none of those traits; one more makes the call ambiguous (E0283).
    trait AmbiguousIfImpl<Marker> {
        fn check() {}
    }
    impl<T: ?Sized> AmbiguousIfImpl<()> for T {}
    struct ImplementsDisplay;
    impl<T: ?Sized + Display> AmbiguousIfImpl<ImplementsDisplay> for T {}
    struct ImplementsDebug;
    impl<T: ?Sized + Debug> AmbiguousIfImpl<ImplementsDebug> for T {}
    struct ImplementsAsRefPath;
    impl<T: ?Sized + AsRef<Path>> AmbiguousIfImpl<ImplementsAsRefPath> for T {}
    struct ImplementsAsRefOsStr;
    impl<T: ?Sized + AsRef<OsStr>> AmbiguousIfImpl<ImplementsAsRefOsStr> for T {}
    struct ImplementsAsRefStr;
    impl<T: ?Sized + AsRef<str>> AmbiguousIfImpl<ImplementsAsRefStr> for T {}
    struct ImplementsAsRefBytes;
    impl<T: ?Sized + AsRef<[u8]>> AmbiguousIfImpl<ImplementsAsRefBytes> for T {}
    struct ImplementsBorrowStr;
    impl<T: ?Sized + Borrow<str>> AmbiguousIfImpl<ImplementsBorrowStr> for T {}
    struct ImplementsBorrowBytes;
    impl<T: ?Sized + Borrow<[u8]>> AmbiguousIfImpl<ImplementsBorrowBytes> for T {}
    struct ImplementsDeref;
    impl<T: ?Sized + Deref> AmbiguousIfImpl<ImplementsDeref> for T {}
    struct ImplementsIntoPathBuf;
    impl<T: Into<PathBuf>> AmbiguousIfImpl<ImplementsIntoPathBuf> for T {}
    struct ImplementsIntoOsString;
    impl<T: Into<OsString>> AmbiguousIfImpl<ImplementsIntoOsString> for T {}
    struct ImplementsIntoString;
    impl<T: Into<String>> AmbiguousIfImpl<ImplementsIntoString> for T {}
    struct ImplementsIntoBytes;
    impl<T: Into<Vec<u8>>> AmbiguousIfImpl<ImplementsIntoBytes> for T {}
    struct ImplementsToSocketAddrs;
    impl<T: ?Sized + ToSocketAddrs> AmbiguousIfImpl<ImplementsToSocketAddrs> for T {}

    /// The wrapper has no conversion to a path, a process argument, an
    /// outbound address or a string that could reach a log line, SQL or
    /// HTML, whatever it holds. Each line stops compiling, failing the
    /// gate, as soon as such a conversion is added.
    ///
    /// Verifies: SEC-TM-031
    #[test]
    fn has_no_path_argument_address_or_display_conversion() {
        <Untrusted<String> as AmbiguousIfImpl<_>>::check();
        <Untrusted<&'static str> as AmbiguousIfImpl<_>>::check();
        <Untrusted<Vec<u8>> as AmbiguousIfImpl<_>>::check();
        <Untrusted<&'static [u8]> as AmbiguousIfImpl<_>>::check();
        <Untrusted<PathBuf> as AmbiguousIfImpl<_>>::check();
        <Untrusted<OsString> as AmbiguousIfImpl<_>>::check();
    }
}
