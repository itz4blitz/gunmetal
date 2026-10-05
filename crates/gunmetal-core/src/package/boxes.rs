//! The box writer of the audio packager (ISO/IEC 14496-12, section 4.2).
//!
//! A box is a 32-bit size that counts its own eight-octet header, a
//! four-character type and a body. [`Body`] collects a body field by field,
//! most significant octet first, and [`Body::boxed`] closes it. The
//! packager builds every box here from typed values, so no octet of a box
//! header or of a box's fields is ever copied from a source file
//! (architecture record 4, decision 6).

/// The ID of the one track a packaged stream holds.
pub(super) const TRACK_ID: u32 = 1;

/// How many items `items` holds.
pub(super) fn count<T>(items: &[T]) -> u64 {
    // A slice never holds more than u64::MAX items.
    u64::try_from(items.len()).unwrap_or(u64::MAX)
}

/// `value` as a 32-bit field.
///
/// The packager's sizes, counts and durations all fit: one segment's source
/// is at most the per-file read cap of 256 MiB, a frame takes at least one
/// octet of it, and no frame plays for more than 65,535 samples.
pub(super) fn narrow(value: u64) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// The body of a box, written field by field.
#[derive(Debug, Default)]
pub(super) struct Body(Vec<u8>);

impl Body {
    /// An empty body.
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// The body of a full box, which starts with its version and its 24
    /// bits of flags.
    pub(super) fn full(version: u8, flags: u32) -> Self {
        let [_, high, middle, low] = flags.to_be_bytes();
        Self::new().bytes(&[version, high, middle, low])
    }

    /// Appends `octets` as they are.
    pub(super) fn bytes(mut self, octets: &[u8]) -> Self {
        self.0.extend_from_slice(octets);
        self
    }

    /// Appends a 16-bit integer.
    pub(super) fn u16(self, value: u16) -> Self {
        self.bytes(&value.to_be_bytes())
    }

    /// Appends a 32-bit integer.
    pub(super) fn u32(self, value: u32) -> Self {
        self.bytes(&value.to_be_bytes())
    }

    /// Appends a 64-bit integer.
    pub(super) fn u64(self, value: u64) -> Self {
        self.bytes(&value.to_be_bytes())
    }

    /// The whole box of type `kind`: its size, its type and this body.
    pub(super) fn boxed(self, kind: [u8; 4]) -> Vec<u8> {
        let size = narrow(count(&self.0).saturating_add(8));
        let mut whole = Vec::new();
        whole.extend_from_slice(&size.to_be_bytes());
        whole.extend_from_slice(&kind);
        whole.extend_from_slice(&self.0);
        whole
    }
}
