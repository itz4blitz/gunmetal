//! Writers that format builders use to lay out bytes.

/// A growing byte string, written field by field.
///
/// Each writer appends to the end and returns the writer, so a header reads
/// top to bottom as the specification lays it out:
///
/// ```
/// use gunmetal_testkit::bytes::Bytes;
///
/// let mut header = Bytes::new();
/// header.bytes(b"ID3").u8(4).u8(0).u8(0).syncsafe32(257);
/// assert_eq!(header.as_slice(), b"ID3\x04\x00\x00\x00\x00\x02\x01");
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bytes(Vec<u8>);

impl Bytes {
    /// An empty byte string.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The bytes written so far.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// The bytes written so far, as an owned vector.
    #[must_use]
    pub fn into_vec(self) -> Vec<u8> {
        self.0
    }

    /// Appends `bytes` as they are.
    pub fn bytes(&mut self, bytes: &[u8]) -> &mut Self {
        self.0.extend_from_slice(bytes);
        self
    }

    /// Appends `count` zero octets.
    pub fn zeros(&mut self, count: usize) -> &mut Self {
        self.0.resize(self.0.len() + count, 0);
        self
    }

    /// Appends one octet.
    pub fn u8(&mut self, value: u8) -> &mut Self {
        self.0.push(value);
        self
    }

    /// Appends a 16-bit integer, most significant octet first.
    pub fn u16_be(&mut self, value: u16) -> &mut Self {
        self.bytes(&value.to_be_bytes())
    }

    /// Appends a 16-bit integer, least significant octet first.
    pub fn u16_le(&mut self, value: u16) -> &mut Self {
        self.bytes(&value.to_le_bytes())
    }

    /// Appends a 24-bit integer, most significant octet first, as FLAC
    /// writes metadata block lengths and MP4 writes box flags.
    ///
    /// # Panics
    ///
    /// Panics when `value` does not fit in 24 bits.
    pub fn u24_be(&mut self, value: u32) -> &mut Self {
        assert!(value < 1 << 24, "{value} does not fit in 24 bits");
        let [_, high, middle, low] = value.to_be_bytes();
        self.bytes(&[high, middle, low])
    }

    /// Appends a 32-bit integer, most significant octet first.
    pub fn u32_be(&mut self, value: u32) -> &mut Self {
        self.bytes(&value.to_be_bytes())
    }

    /// Appends a 32-bit integer, least significant octet first.
    pub fn u32_le(&mut self, value: u32) -> &mut Self {
        self.bytes(&value.to_le_bytes())
    }

    /// Appends a 64-bit integer, most significant octet first.
    pub fn u64_be(&mut self, value: u64) -> &mut Self {
        self.bytes(&value.to_be_bytes())
    }

    /// Appends a 64-bit integer, least significant octet first.
    pub fn u64_le(&mut self, value: u64) -> &mut Self {
        self.bytes(&value.to_le_bytes())
    }

    /// Appends a 32-bit syncsafe integer, as ID3 version 2 writes tag and
    /// frame sizes: four octets of seven bits each, most significant first,
    /// with the top bit of every octet clear.
    ///
    /// # Panics
    ///
    /// Panics when `value` does not fit in 28 bits.
    pub fn syncsafe32(&mut self, value: u32) -> &mut Self {
        assert!(
            value < 1 << 28,
            "{value} does not fit in a 28-bit syncsafe integer"
        );
        self.bytes(&[
            low_seven_bits(value >> 21),
            low_seven_bits(value >> 14),
            low_seven_bits(value >> 7),
            low_seven_bits(value),
        ])
    }
}

/// The seven least significant bits of `value`.
fn low_seven_bits(value: u32) -> u8 {
    let [.., low] = value.to_be_bytes();
    low & 0x7F
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_empty() {
        assert_eq!(Bytes::new().as_slice(), &[] as &[u8]);
        assert_eq!(Bytes::default().into_vec(), Vec::<u8>::new());
    }

    #[test]
    fn writes_octets_and_runs_of_bytes_in_order() {
        let mut written = Bytes::new();
        written.u8(0xAB).bytes(b"fLaC").zeros(3).u8(0x01).bytes(&[]);
        assert_eq!(
            written.into_vec(),
            vec![0xAB, b'f', b'L', b'a', b'C', 0x00, 0x00, 0x00, 0x01]
        );
    }

    #[test]
    fn writes_big_endian_integers_most_significant_octet_first() {
        let mut written = Bytes::new();
        written
            .u16_be(0x0102)
            .u24_be(0x03_0405)
            .u32_be(0x0607_0809)
            .u64_be(0x0A0B_0C0D_0E0F_1011);
        assert_eq!(
            written.as_slice(),
            &[
                0x01, 0x02, // u16
                0x03, 0x04, 0x05, // u24
                0x06, 0x07, 0x08, 0x09, // u32
                0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11, // u64
            ]
        );
    }

    #[test]
    fn writes_little_endian_integers_least_significant_octet_first() {
        let mut written = Bytes::new();
        written
            .u16_le(0x0102)
            .u32_le(0x0304_0506)
            .u64_le(0x0708_090A_0B0C_0D0E);
        assert_eq!(
            written.as_slice(),
            &[
                0x02, 0x01, // u16
                0x06, 0x05, 0x04, 0x03, // u32
                0x0E, 0x0D, 0x0C, 0x0B, 0x0A, 0x09, 0x08, 0x07, // u64
            ]
        );
    }

    #[test]
    fn writes_the_extremes_of_each_width() {
        let mut written = Bytes::new();
        written
            .u24_be(0xFF_FFFF)
            .u24_be(0)
            .u16_le(u16::MAX)
            .u32_be(u32::MAX)
            .u64_le(u64::MAX);
        let mut expected = vec![0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0xFF];
        expected.extend([0xFF; 4]);
        expected.extend([0xFF; 8]);
        assert_eq!(written.into_vec(), expected);
    }

    #[test]
    #[should_panic(expected = "16777216 does not fit in 24 bits")]
    fn refuses_a_24_bit_value_that_does_not_fit() {
        Bytes::new().u24_be(0x100_0000);
    }

    #[test]
    fn writes_syncsafe_integers_as_the_id3v2_specifications_show() {
        // ID3v2.3.0 section 3.1: "a 257 bytes long tag is represented as
        // $00 00 02 01". ID3v2.4.0 structure section 6.2: 255 is
        // %00000001 01111111.
        let cases: [(u32, [u8; 4]); 5] = [
            (0, [0x00, 0x00, 0x00, 0x00]),
            (255, [0x00, 0x00, 0x01, 0x7F]),
            (257, [0x00, 0x00, 0x02, 0x01]),
            (0x0123_4567, [0x09, 0x0D, 0x0A, 0x67]),
            (0x0FFF_FFFF, [0x7F, 0x7F, 0x7F, 0x7F]),
        ];
        for (value, expected) in cases {
            let mut written = Bytes::new();
            written.syncsafe32(value);
            assert_eq!(written.as_slice(), &expected, "value {value}");
        }
    }

    #[test]
    #[should_panic(expected = "268435456 does not fit in a 28-bit syncsafe integer")]
    fn refuses_a_syncsafe_value_that_does_not_fit() {
        Bytes::new().syncsafe32(0x1000_0000);
    }
}
