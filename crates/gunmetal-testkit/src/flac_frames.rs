//! FLAC audio frames (RFC 9639, section 9), for the frame index's tests.
//!
//! A frame here holds one constant subframe of silence per channel, so a
//! valid frame with no real audio is a few octets. The builder writes any
//! code a header field can hold, reserved and forbidden ones included, so a
//! test can build exactly the header it means to refuse. It shares no code
//! with the parser: the coded number follows the UTF-8 table of RFC 3629
//! extended to seven octets, and the checksums come from
//! [`checksum`](crate::checksum).

use crate::bytes::Bits;
use crate::checksum::{crc8_flac, crc16_flac};

/// Every field of a frame header (RFC 9639, section 9.1), as the codes the
/// header stores.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    /// The blocking strategy bit: `false` for fixed block sizes, whose
    /// coded number is a frame number, and `true` for variable ones, whose
    /// coded number is a sample number.
    pub variable: bool,
    /// The four block size bits. Code 6 adds an 8-bit field and code 7 a
    /// 16-bit one at the end of the header, holding the block size minus
    /// one.
    pub block_size: u8,
    /// The four sample rate bits. Code 12 adds an 8-bit field in kilohertz,
    /// code 13 a 16-bit field in hertz and code 14 a 16-bit field in tens of
    /// hertz at the end of the header.
    pub sample_rate: u8,
    /// The four channel bits.
    pub channels: u8,
    /// The three bit depth bits.
    pub bits: u8,
    /// The reserved bit, which a valid header leaves clear.
    pub reserved: bool,
    /// The coded frame or sample number.
    pub number: u64,
    /// The block size field that codes 6 and 7 add; ignored otherwise.
    pub block_size_field: u16,
    /// The sample rate field that codes 12 to 14 add; ignored otherwise.
    pub sample_rate_field: u16,
}

impl Header {
    /// A header for 4,096 samples of 16-bit stereo at 44.1 kHz, the shape
    /// the reference encoder writes for CD audio, with blocking strategy
    /// `variable` and coded number `number`.
    #[must_use]
    pub const fn cd(variable: bool, number: u64) -> Self {
        Self {
            variable,
            block_size: 0b1100,
            sample_rate: 0b1001,
            channels: 0b0001,
            bits: 0b100,
            reserved: false,
            number,
            block_size_field: 0,
            sample_rate_field: 0,
        }
    }

    /// The header's octets, from the sync code to its CRC-8.
    ///
    /// # Panics
    ///
    /// Panics when a code does not fit its field, when an 8-bit field holds
    /// more than eight bits, or when the number needs more than 36 bits.
    #[must_use]
    pub fn bytes(&self) -> Vec<u8> {
        let mut fields = Bits::new();
        fields
            .put(15, 0b111_1111_1111_1100) // sync code
            .put(1, u64::from(self.variable))
            .put(4, u64::from(self.block_size))
            .put(4, u64::from(self.sample_rate))
            .put(4, u64::from(self.channels))
            .put(3, u64::from(self.bits))
            .put(1, u64::from(self.reserved));
        for octet in coded_number(self.number) {
            fields.put(8, u64::from(octet));
        }
        match self.block_size {
            0b0110 => fields.put(8, u64::from(self.block_size_field)),
            0b0111 => fields.put(16, u64::from(self.block_size_field)),
            _ => &mut fields,
        };
        match self.sample_rate {
            0b1100 => fields.put(8, u64::from(self.sample_rate_field)),
            0b1101 | 0b1110 => fields.put(16, u64::from(self.sample_rate_field)),
            _ => &mut fields,
        };
        let mut header = fields.into_vec();
        header.push(crc8_flac(&header));
        header
    }
}

/// `value` as the header's coded number: one to seven octets laid out as
/// UTF-8 lays out a code point, with the seven-octet form `0xFE` followed
/// by six continuation octets for values up to 36 bits.
///
/// # Panics
///
/// Panics when `value` needs more than 36 bits.
#[must_use]
pub fn coded_number(value: u64) -> Vec<u8> {
    assert!(value >> 36 == 0, "{value} needs more than 36 bits");
    if value < 0x80 {
        return vec![low_octet(value)];
    }
    // An n-octet form holds 7 - n bits in its first octet and six in each
    // of the others: 5n + 1 bits in all.
    let octets = (2..=7)
        .find(|&octets| value >> (5 * octets + 1) == 0)
        .expect("seven octets hold 36 bits");
    let continuations = octets - 1;
    // The marker bits and the value's bits never overlap, so adding them
    // sets both.
    let mut coded = vec![(0xFF << (8 - octets)) + low_octet(value >> (6 * continuations))];
    for index in (0..continuations).rev() {
        coded.push(0x80 + low_octet(value >> (6 * index) & 0x3F));
    }
    coded
}

/// The least significant octet of `value`, which the caller has already
/// narrowed to fit.
fn low_octet(value: u64) -> u8 {
    u8::try_from(value).expect("the value fits in one octet")
}

/// A whole frame: `header`, then one constant subframe of silence per
/// entry of `subframe_bits`, each that many bits wide, then zero bits to
/// the next octet and the frame's CRC-16.
///
/// A side channel is one bit wider than the others, so a 16-bit stereo
/// frame coded left and side has subframes of 16 and 17 bits.
///
/// # Panics
///
/// Panics as [`Header::bytes`] does.
#[must_use]
pub fn frame(header: &Header, subframe_bits: &[u32]) -> Vec<u8> {
    let mut subframes = Bits::new();
    for &width in subframe_bits {
        // Zero padding bit, type 0 (constant), no wasted bits, then the
        // one sample value, zero.
        subframes.put(8, 0).put(width, 0);
    }
    subframes.pad_to_byte();
    let mut frame = header.bytes();
    frame.extend(subframes.into_vec());
    let crc = crc16_flac(&frame);
    frame.extend(crc.to_be_bytes());
    frame
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_the_utf8_examples_of_rfc_3629() {
        // RFC 3629, section 7, and its table: one octet below 0x80, then
        // two, three and four.
        assert_eq!(coded_number(0x41), [0x41]);
        assert_eq!(coded_number(0xE9), [0xC3, 0xA9]);
        assert_eq!(coded_number(0x20AC), [0xE2, 0x82, 0xAC]);
        assert_eq!(coded_number(0xD55C), [0xED, 0x95, 0x9C]);
        assert_eq!(coded_number(0x0002_33B4), [0xF0, 0xA3, 0x8E, 0xB4]);
    }

    #[test]
    fn writes_each_width_from_its_smallest_value_to_its_largest() {
        let cases: [(u64, &[u8]); 14] = [
            (0, &[0x00]),
            (0x7F, &[0x7F]),
            (0x80, &[0xC2, 0x80]),
            (0x7FF, &[0xDF, 0xBF]),
            (0x800, &[0xE0, 0xA0, 0x80]),
            (0xFFFF, &[0xEF, 0xBF, 0xBF]),
            (0x1_0000, &[0xF0, 0x90, 0x80, 0x80]),
            (0x1F_FFFF, &[0xF7, 0xBF, 0xBF, 0xBF]),
            (0x20_0000, &[0xF8, 0x88, 0x80, 0x80, 0x80]),
            (0x3FF_FFFF, &[0xFB, 0xBF, 0xBF, 0xBF, 0xBF]),
            (0x400_0000, &[0xFC, 0x84, 0x80, 0x80, 0x80, 0x80]),
            (0x7FFF_FFFF, &[0xFD, 0xBF, 0xBF, 0xBF, 0xBF, 0xBF]),
            (0x8000_0000, &[0xFE, 0x82, 0x80, 0x80, 0x80, 0x80, 0x80]),
            (0xF_FFFF_FFFF, &[0xFE, 0xBF, 0xBF, 0xBF, 0xBF, 0xBF, 0xBF]),
        ];
        for (value, expected) in cases {
            assert_eq!(coded_number(value), expected, "{value:#X}");
        }
    }

    #[test]
    #[should_panic(expected = "68719476736 needs more than 36 bits")]
    fn refuses_a_number_wider_than_36_bits() {
        let _ = coded_number(1 << 36);
    }

    /// The header of the first frame libFLAC 1.5.0 wrote for 64 samples of
    /// 16-bit mono silence at 8 kHz (the frame `checksum`'s tests check).
    const LIBFLAC_HEADER: Header = Header {
        variable: false,
        block_size: 0b0110,
        sample_rate: 0b0100,
        channels: 0b0000,
        bits: 0b100,
        reserved: false,
        number: 0,
        block_size_field: 63,
        sample_rate_field: 0,
    };

    #[test]
    fn writes_the_header_libflac_wrote() {
        assert_eq!(
            LIBFLAC_HEADER.bytes(),
            [0xFF, 0xF8, 0x64, 0x08, 0x00, 0x3F, 0x5E]
        );
    }

    #[test]
    fn writes_the_whole_frame_libflac_wrote() {
        assert_eq!(
            frame(&LIBFLAC_HEADER, &[16]),
            [
                0xFF, 0xF8, 0x64, 0x08, 0x00, 0x3F, 0x5E, // header
                0x00, 0x00, 0x00, // constant subframe of value zero
                0xC6, 0x3C, // CRC-16
            ]
        );
    }

    #[test]
    fn writes_16_bit_end_of_header_fields_after_a_seven_octet_number() {
        // Variable blocking, a 16-bit block size field, a 16-bit rate in
        // hertz, mid and side channels and 32-bit samples.
        let header = Header {
            variable: true,
            block_size: 0b0111,
            sample_rate: 0b1101,
            channels: 0b1010,
            bits: 0b111,
            reserved: false,
            number: 0x8000_0000,
            block_size_field: 0x1234,
            sample_rate_field: 0xABCD,
        };
        assert_eq!(
            header.bytes(),
            [
                0xFF, 0xF9, 0x7D, 0xAE, // sync, codes
                0xFE, 0x82, 0x80, 0x80, 0x80, 0x80, 0x80, // coded number
                0x12, 0x34, // block size minus one
                0xAB, 0xCD, // sample rate
                0x11, // CRC-8
            ]
        );
    }

    #[test]
    fn writes_an_8_bit_rate_in_kilohertz_and_the_reserved_bit() {
        let header = Header {
            variable: false,
            block_size: 0b0001,
            sample_rate: 0b1100,
            channels: 0b0001,
            bits: 0b001,
            reserved: true,
            number: 5,
            block_size_field: 0xFFFF,
            sample_rate_field: 0x30,
        };
        assert_eq!(header.bytes(), [0xFF, 0xF8, 0x1C, 0x13, 0x05, 0x30, 0xFE]);
    }

    #[test]
    fn writes_a_16_bit_rate_in_tens_of_hertz() {
        let header = Header {
            variable: true,
            block_size: 0b1100,
            sample_rate: 0b1110,
            channels: 0b1000,
            bits: 0b000,
            reserved: false,
            number: 0x7FF,
            block_size_field: 0xFFFF,
            sample_rate_field: 4_410,
        };
        assert_eq!(
            header.bytes(),
            [0xFF, 0xF9, 0xCE, 0x80, 0xDF, 0xBF, 0x11, 0x3A, 0xB1]
        );
    }

    #[test]
    fn writes_the_cd_shape() {
        let header = Header::cd(false, 1);
        assert_eq!(
            header,
            Header {
                variable: false,
                block_size: 0b1100,
                sample_rate: 0b1001,
                channels: 0b0001,
                bits: 0b100,
                reserved: false,
                number: 1,
                block_size_field: 0,
                sample_rate_field: 0,
            }
        );
        assert!(Header::cd(true, 0).variable);
    }

    #[test]
    fn pads_subframes_of_odd_widths_to_an_octet_before_the_crc() {
        // Left and side: 16 and 17 bits, after an 8-bit subframe header
        // each, are 49 bits, padded to seven octets.
        let header = Header {
            variable: false,
            block_size: 0b0001,
            sample_rate: 0b1001,
            channels: 0b1000,
            bits: 0b100,
            reserved: false,
            number: 1,
            block_size_field: 0,
            sample_rate_field: 0,
        };
        assert_eq!(
            frame(&header, &[16, 17]),
            [
                0xFF, 0xF8, 0x19, 0x88, 0x01, 0x0B, // header
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // two subframes
                0x56, 0xBD, // CRC-16
            ]
        );
    }

    #[test]
    #[should_panic(expected = "16 does not fit in 4 bits")]
    fn refuses_a_code_wider_than_its_field() {
        let _ = Header {
            block_size: 16,
            ..Header::cd(false, 0)
        }
        .bytes();
    }

    #[test]
    #[should_panic(expected = "256 does not fit in 8 bits")]
    fn refuses_an_8_bit_field_wider_than_eight_bits() {
        let _ = Header {
            block_size: 0b0110,
            block_size_field: 256,
            ..Header::cd(false, 0)
        }
        .bytes();
    }
}
