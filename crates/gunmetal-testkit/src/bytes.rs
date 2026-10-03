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

/// Fields packed most significant bit first, as FLAC and MPEG audio pack
/// their headers.
///
/// ```
/// use gunmetal_testkit::bytes::Bits;
///
/// let mut header = Bits::new();
/// header.put(14, 0x3FFE).put(1, 0).put(1, 0);
/// assert_eq!(header.into_vec(), [0xFF, 0xF8]);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bits {
    /// Every octet whose eight bits are written.
    whole: Vec<u8>,
    /// The bits written after the last whole octet, in the low bits.
    partial: u8,
    /// How many bits `partial` holds, from 0 to 7.
    used: u8,
}

impl Bits {
    /// No fields yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends the low `width` bits of `value`, most significant first.
    ///
    /// # Panics
    ///
    /// Panics when `width` is over 64 or `value` does not fit in `width`
    /// bits.
    pub fn put(&mut self, width: u32, value: u64) -> &mut Self {
        assert!(width <= 64, "a field is at most 64 bits wide, not {width}");
        assert!(
            value.checked_shr(width).is_none_or(|rest| rest == 0),
            "{value} does not fit in {width} bits"
        );
        for shift in (0..width).rev() {
            self.push_bit((value >> shift) & 1 == 1);
        }
        self
    }

    /// Appends zero bits up to the next octet boundary, if not already on
    /// one.
    pub fn pad_to_byte(&mut self) -> &mut Self {
        while self.used != 0 {
            self.push_bit(false);
        }
        self
    }

    /// The packed octets.
    ///
    /// # Panics
    ///
    /// Panics when the fields do not end on an octet boundary; call
    /// [`Bits::pad_to_byte`] first where the format pads with zero bits.
    #[must_use]
    pub fn into_vec(self) -> Vec<u8> {
        assert!(
            self.used == 0,
            "the fields end {} bits into an octet; pad them first",
            self.used
        );
        self.whole
    }

    fn push_bit(&mut self, bit: bool) {
        self.partial = (self.partial << 1) + u8::from(bit);
        self.used += 1;
        if self.used == 8 {
            self.whole.push(self.partial);
            self.partial = 0;
            self.used = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::collection::vec;
    use proptest::prelude::*;

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

    #[test]
    fn packs_a_real_flac_streaminfo_block() {
        // The STREAMINFO body libFLAC 1.5.0 wrote for 64 frames of 16-bit
        // mono silence at 8 kHz, encoded with `flac -b 64`.
        let mut fields = Bits::new();
        fields
            .put(16, 64) // minimum block size
            .put(16, 64) // maximum block size
            .put(24, 12) // minimum frame size
            .put(24, 12) // maximum frame size
            .put(20, 8_000) // sample rate
            .put(3, 0) // channels minus one
            .put(5, 15) // bits per sample minus one
            .put(36, 64) // total samples
            .put(64, 0xF09F_35A5_6378_3945) // MD5 of the samples, first half
            .put(64, 0x8E46_2E63_50EC_BCE4); // and second half
        assert_eq!(
            fields.into_vec(),
            vec![
                0x00, 0x40, 0x00, 0x40, // block sizes
                0x00, 0x00, 0x0C, 0x00, 0x00, 0x0C, // frame sizes
                0x01, 0xF4, 0x00, 0xF0, 0x00, 0x00, 0x00, 0x40, // rate to total samples
                0xF0, 0x9F, 0x35, 0xA5, 0x63, 0x78, 0x39, 0x45, // MD5
                0x8E, 0x46, 0x2E, 0x63, 0x50, 0xEC, 0xBC, 0xE4,
            ]
        );
    }

    #[test]
    fn packs_a_real_mpeg_audio_frame_header() {
        // The first frame header LAME wrote for 128 kbit/s joint stereo at
        // 44.1 kHz.
        let mut fields = Bits::new();
        fields
            .put(11, 0x7FF) // frame sync
            .put(2, 0b11) // MPEG-1
            .put(2, 0b01) // Layer III
            .put(1, 1) // no CRC
            .put(4, 0b1001) // 128 kbit/s
            .put(2, 0b00) // 44.1 kHz
            .put(1, 0) // no padding
            .put(1, 0) // private bit
            .put(2, 0b01) // joint stereo
            .put(2, 0b10) // mid/side stereo on, intensity stereo off
            .put(1, 0) // not copyrighted
            .put(1, 1) // original
            .put(2, 0b00); // no emphasis
        assert_eq!(fields.into_vec(), vec![0xFF, 0xFB, 0x90, 0x64]);
    }

    #[test]
    fn writes_nothing_for_a_field_of_no_bits() {
        let mut fields = Bits::new();
        fields.put(0, 0);
        assert_eq!(fields.into_vec(), Vec::<u8>::new());
        assert_eq!(Bits::default().into_vec(), Vec::<u8>::new());
    }

    #[test]
    fn writes_a_full_64_bit_field() {
        let mut fields = Bits::new();
        fields.put(1, 1).put(64, u64::MAX).put(7, 0);
        let mut expected = vec![0xFF; 8];
        expected.push(0x80);
        assert_eq!(fields.into_vec(), expected);
    }

    #[test]
    fn pads_the_last_octet_with_zero_bits() {
        let mut fields = Bits::new();
        fields
            .put(3, 0b101)
            .pad_to_byte()
            .put(4, 0b1111)
            .pad_to_byte();
        assert_eq!(fields.into_vec(), vec![0b1010_0000, 0b1111_0000]);
    }

    #[test]
    fn padding_an_aligned_field_adds_nothing() {
        let mut fields = Bits::new();
        fields.pad_to_byte().put(8, 0x5A).pad_to_byte();
        assert_eq!(fields.into_vec(), vec![0x5A]);
    }

    #[test]
    #[should_panic(expected = "a field is at most 64 bits wide, not 65")]
    fn refuses_a_field_wider_than_64_bits() {
        Bits::new().put(65, 0);
    }

    #[test]
    #[should_panic(expected = "8 does not fit in 3 bits")]
    fn refuses_a_value_wider_than_its_field() {
        Bits::new().put(3, 8);
    }

    #[test]
    #[should_panic(expected = "the fields end 5 bits into an octet; pad them first")]
    fn refuses_to_finish_part_way_through_an_octet() {
        let mut fields = Bits::new();
        fields.put(13, 0);
        let _ = fields.into_vec();
    }

    /// Reads `width` bits from `bytes` starting at bit `start`, most
    /// significant first. A reference reader written only for these tests.
    fn read_bits(bytes: &[u8], start: usize, width: usize) -> u64 {
        (start..start + width).fold(0, |value, bit| {
            let octet = u64::from(bytes[bit / 8]);
            (value << 1) | ((octet >> (7 - bit % 8)) & 1)
        })
    }

    proptest! {
        #[test]
        fn reads_back_every_field_it_packed(
            fields in vec((0_usize..=64, any::<u64>()), 0..24),
        ) {
            // Keep only the bits each field has room for.
            let fields: Vec<(usize, u64)> = fields
                .into_iter()
                .map(|(width, raw)| {
                    let value = if width == 64 { raw } else { raw % (1 << width) };
                    (width, value)
                })
                .collect();
            let mut packed = Bits::new();
            for &(width, value) in &fields {
                packed.put(u32::try_from(width).unwrap(), value);
            }
            packed.pad_to_byte();
            let bytes = packed.into_vec();

            let total: usize = fields.iter().map(|&(width, _)| width).sum();
            prop_assert_eq!(bytes.len(), total.div_ceil(8));
            let mut start = 0;
            for &(width, value) in &fields {
                prop_assert_eq!(read_bits(&bytes, start, width), value);
                start += width;
            }
            prop_assert_eq!(read_bits(&bytes, start, bytes.len() * 8 - start), 0);
        }
    }
}
