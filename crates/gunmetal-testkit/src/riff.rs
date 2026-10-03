//! Builders for WAV and AIFF files.
//!
//! A WAV file is a RIFF form of type `WAVE` (Microsoft's Multimedia
//! Programming Interface and Data Specifications 1.0, 1991), or its 64-bit
//! variant RF64 (EBU Tech 3306). An AIFF file is an EA IFF 85 form of type
//! `AIFF` or, compressed, `AIFC` (Apple's Audio Interchange File Format 1.3,
//! 1989, and its AIFF-C draft, 1991). Both are chunks back to back; lay the
//! chunks out with [`Bytes::riff_chunk`] or [`Bytes::aiff_chunk`], then wrap
//! them in a form with [`wave`], [`rf64`] or [`aiff`].
//!
//! Each builder writes one structure as its specification lays it out, and
//! shares no code with the parsers it feeds.

use crate::bytes::Bytes;

/// The sub-format GUID of `WAVE_FORMAT_EXTENSIBLE` integer PCM,
/// `KSDATAFORMAT_SUBTYPE_PCM`, 00000001-0000-0010-8000-00AA00389B71, as
/// its octets appear in a file.
pub const PCM_SUBFORMAT: [u8; 16] = [
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
];

/// The sub-format GUID of `WAVE_FORMAT_EXTENSIBLE` IEEE floating point,
/// `KSDATAFORMAT_SUBTYPE_IEEE_FLOAT`, 00000003-0000-0010-8000-00AA00389B71.
pub const FLOAT_SUBFORMAT: [u8; 16] = [
    0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
];

/// A `fmt ` chunk body in the 16-octet `PCMWAVEFORMAT` layout: the format
/// tag, the channels, the sample rate, the average octets per second, the
/// block alignment and the bits per sample, all little-endian. The block
/// alignment is one frame, a sample of `bits` rounded up to whole octets
/// for each channel, and the octets per second are a second of frames.
#[must_use]
pub fn format(tag: u16, channels: u16, sample_rate: u32, bits: u16) -> Vec<u8> {
    let block_align = channels * bits.div_ceil(8);
    let mut body = Bytes::new();
    body.u16_le(tag)
        .u16_le(channels)
        .u32_le(sample_rate)
        .u32_le(sample_rate * u32::from(block_align))
        .u16_le(block_align)
        .u16_le(bits);
    body.into_vec()
}

/// A `fmt ` chunk body in the 40-octet `WAVEFORMATEXTENSIBLE` layout:
/// [`format`] with the tag 0xFFFE, then the size of the extension (22),
/// the valid bits per sample, the channel mask and the sub-format GUID.
#[must_use]
pub fn extensible(
    channels: u16,
    sample_rate: u32,
    bits: u16,
    valid_bits: u16,
    channel_mask: u32,
    sub_format: [u8; 16],
) -> Vec<u8> {
    let mut body = Bytes::new();
    body.bytes(&format(0xFFFE, channels, sample_rate, bits))
        .u16_le(22)
        .u16_le(valid_bits)
        .u32_le(channel_mask)
        .bytes(&sub_format);
    body.into_vec()
}

/// A `ds64` chunk body (EBU Tech 3306, section 3.2): the 64-bit sizes of
/// the RIFF form and of the `data` chunk, the sample count, and an empty
/// table of further chunk sizes.
#[must_use]
pub fn ds64(riff_size: u64, data_size: u64, sample_count: u64) -> Vec<u8> {
    let mut body = Bytes::new();
    body.u64_le(riff_size)
        .u64_le(data_size)
        .u64_le(sample_count)
        .u32_le(0);
    body.into_vec()
}

/// A WAV file: a `RIFF` form of type `WAVE` holding `chunks`, which are
/// laid out already.
///
/// # Panics
///
/// Panics when the form would not fit a 32-bit size.
#[must_use]
pub fn wave(chunks: &[u8]) -> Vec<u8> {
    let mut file = Bytes::new();
    file.riff_chunk(*b"RIFF", &[b"WAVE", chunks].concat());
    file.into_vec()
}

/// An RF64 WAV file: `RF64`, a size of all ones as EBU Tech 3306 asks
/// (the real size is in the `ds64` chunk that `chunks` must start with),
/// `WAVE`, then `chunks`.
#[must_use]
pub fn rf64(chunks: &[u8]) -> Vec<u8> {
    let mut file = Bytes::new();
    file.bytes(b"RF64")
        .u32_le(u32::MAX)
        .bytes(b"WAVE")
        .bytes(chunks);
    file.into_vec()
}

/// An AIFF `COMM` chunk body (18 octets, big-endian): the channels, the
/// sample frames, the bits per sample, and the sample rate as the 80-bit
/// extended number [`extended`] writes.
#[must_use]
pub fn comm(channels: u16, sample_frames: u32, sample_size: u16, sample_rate: u32) -> Vec<u8> {
    let mut body = Bytes::new();
    body.u16_be(channels)
        .u32_be(sample_frames)
        .u16_be(sample_size)
        .bytes(&extended(sample_rate));
    body.into_vec()
}

/// An AIFF-C `COMM` chunk body: [`comm`], then the compression type and
/// its name as a Pascal string (a count octet, then the text), padded with
/// one zero octet when the string's length is odd.
///
/// # Panics
///
/// Panics when `name` is longer than 255 octets.
#[must_use]
pub fn comm_c(
    channels: u16,
    sample_frames: u32,
    sample_size: u16,
    sample_rate: u32,
    compression: [u8; 4],
    name: &[u8],
) -> Vec<u8> {
    let count = u8::try_from(name.len()).expect("a Pascal string holds at most 255 octets");
    let mut body = Bytes::new();
    body.bytes(&comm(channels, sample_frames, sample_size, sample_rate))
        .bytes(&compression)
        .u8(count)
        .bytes(name)
        .zeros((name.len() + 1) % 2);
    body.into_vec()
}

/// An AIFF `SSND` chunk body: the offset of the first sample frame, the
/// block size, `offset` zero octets, then `samples`.
///
/// # Panics
///
/// Panics when `offset` does not fit a `usize`.
#[must_use]
pub fn ssnd(offset: u32, block_size: u32, samples: &[u8]) -> Vec<u8> {
    let mut body = Bytes::new();
    body.u32_be(offset)
        .u32_be(block_size)
        .zeros(usize::try_from(offset).expect("the offset fits a usize"))
        .bytes(samples);
    body.into_vec()
}

/// An AIFF file: a `FORM` of type `form_type`, `AIFF` or `AIFC`, holding
/// `chunks`, which are laid out already.
///
/// # Panics
///
/// Panics when the form would not fit a 32-bit size.
#[must_use]
pub fn aiff(form_type: [u8; 4], chunks: &[u8]) -> Vec<u8> {
    let mut file = Bytes::new();
    file.aiff_chunk(*b"FORM", &[&form_type[..], chunks].concat());
    file.into_vec()
}

/// `value` as an 80-bit IEEE 754 extended-precision number, as AIFF writes
/// its sample rate: a sign bit, a 15-bit exponent biased by 16,383, and a
/// 64-bit significand whose top bit is the integer bit, all big-endian.
/// Zero is all zero octets; any other value is normalised, so its top bit
/// is set.
#[must_use]
pub fn extended(value: u32) -> [u8; 10] {
    if value == 0 {
        return [0; 10];
    }
    // Shift the value's top bit up to the integer bit, and count the
    // exponent down from the 31 that a top bit in bit 31 needs.
    let shift = value.leading_zeros();
    // At most 16,414, so the exponent is the low two octets.
    let [_, _, high, low] = (16_383 + 31 - shift).to_be_bytes();
    let significand = u64::from(value) << (32 + shift);
    let mut octets = [0; 10];
    octets[..2].copy_from_slice(&[high, low]);
    octets[2..].copy_from_slice(&significand.to_be_bytes());
    octets
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// Chunks laid out with `write`.
    fn chunks(write: impl FnOnce(&mut Bytes)) -> Vec<u8> {
        let mut written = Bytes::new();
        write(&mut written);
        written.into_vec()
    }

    #[test]
    fn writes_the_format_of_a_cd_quality_file() {
        // The `fmt ` body of a 16-bit stereo file at 44.1 kHz, as every
        // writer lays it out: 176,400 octets a second in frames of four.
        assert_eq!(
            format(1, 2, 44_100, 16),
            [
                0x01, 0x00, 0x02, 0x00, 0x44, 0xAC, 0x00, 0x00, 0x10, 0xB1, 0x02, 0x00, 0x04, 0x00,
                0x10, 0x00,
            ]
        );
    }

    #[test]
    fn rounds_each_sample_up_to_whole_octets_in_a_frame() {
        // Six channels of 24 bits: frames of 18 octets, 864,000 a second.
        assert_eq!(
            format(1, 6, 48_000, 24),
            [
                0x01, 0x00, 0x06, 0x00, 0x80, 0xBB, 0x00, 0x00, 0x00, 0x2F, 0x0D, 0x00, 0x12, 0x00,
                0x18, 0x00,
            ]
        );
        // Twelve bits take two octets.
        assert_eq!(
            format(3, 1, 22_050, 12),
            [
                0x03, 0x00, 0x01, 0x00, 0x22, 0x56, 0x00, 0x00, 0x44, 0xAC, 0x00, 0x00, 0x02, 0x00,
                0x0C, 0x00,
            ]
        );
    }

    #[test]
    fn writes_wave_format_extensible() {
        // 24 valid bits in 32-bit containers, front left and right, at
        // 48 kHz, as Microsoft's WAVEFORMATEXTENSIBLE lays it out.
        let mut expected = vec![
            0xFE, 0xFF, 0x02, 0x00, 0x80, 0xBB, 0x00, 0x00, 0x00, 0xDC, 0x05, 0x00, 0x08, 0x00,
            0x20, 0x00, // the PCMWAVEFORMAT fields
            0x16, 0x00, // 22 octets of extension
            0x18, 0x00, // valid bits
            0x03, 0x00, 0x00, 0x00, // channel mask
        ];
        expected.extend(PCM_SUBFORMAT);
        assert_eq!(extensible(2, 48_000, 32, 24, 0x3, PCM_SUBFORMAT), expected);
    }

    #[test]
    fn the_sub_formats_are_the_guids_microsoft_publishes() {
        // KSDATAFORMAT_SUBTYPE_PCM and _IEEE_FLOAT: Data1 little-endian,
        // then Data2 0x0000 and Data3 0x0010 little-endian, then Data4.
        let tail = [
            0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
        ];
        assert_eq!(PCM_SUBFORMAT[..4], [0x01, 0x00, 0x00, 0x00]);
        assert_eq!(FLOAT_SUBFORMAT[..4], [0x03, 0x00, 0x00, 0x00]);
        assert_eq!(PCM_SUBFORMAT[4..], tail);
        assert_eq!(FLOAT_SUBFORMAT[4..], tail);
    }

    #[test]
    fn writes_a_ds64_body() {
        // A 4 GiB data chunk in a form 36 octets longer, of 2^30 frames.
        assert_eq!(
            ds64(0x1_0000_0024, 0x1_0000_0000, 0x4000_0000),
            [
                0x24, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, // RIFF size
                0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, // data size
                0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, // sample count
                0x00, 0x00, 0x00, 0x00, // no table
            ]
        );
    }

    #[test]
    fn wraps_chunks_in_a_riff_wave_form() {
        // The 8-bit mono file `ffmpeg -f wav -fflags +bitexact` wrote, as
        // in the byte writers' tests.
        let file = wave(&chunks(|wave| {
            wave.riff_chunk(*b"fmt ", &format(1, 1, 8_000, 8))
                .riff_chunk(*b"data", &[0x80, 0x81, 0x7F]);
        }));
        assert_eq!(
            file,
            [
                &b"RIFF\x28\x00\x00\x00WAVE"[..],
                b"fmt \x10\x00\x00\x00",
                &[0x01, 0x00, 0x01, 0x00, 0x40, 0x1F, 0x00, 0x00],
                &[0x40, 0x1F, 0x00, 0x00, 0x01, 0x00, 0x08, 0x00],
                b"data\x03\x00\x00\x00",
                &[0x80, 0x81, 0x7F, 0x00],
            ]
            .concat()
        );
    }

    #[test]
    fn wraps_chunks_in_an_rf64_form_whose_size_is_all_ones() {
        assert_eq!(
            rf64(b"ds64\x00\x00\x00\x00"),
            b"RF64\xFF\xFF\xFF\xFFWAVEds64\x00\x00\x00\x00"
        );
        assert_eq!(rf64(&[]), b"RF64\xFF\xFF\xFF\xFFWAVE");
    }

    #[test]
    fn writes_the_common_chunk_of_an_aiff_file() {
        // The COMM body ffmpeg wrote for three 8-bit mono frames at 8 kHz,
        // as in the byte writers' tests.
        assert_eq!(
            comm(1, 3, 8, 8_000),
            [
                0x00, 0x01, 0x00, 0x00, 0x00, 0x03, 0x00, 0x08, 0x40, 0x0B, 0xFA, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00,
            ]
        );
    }

    #[test]
    fn writes_the_compression_type_and_its_padded_name_in_aiff_c() {
        let common = [
            0x00, 0x02, 0x00, 0x01, 0x00, 0x00, 0x00, 0x10, 0x40, 0x0E, 0xAC, 0x44, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];
        // An empty name is its count alone, padded to two octets.
        assert_eq!(
            comm_c(2, 65_536, 16, 44_100, *b"sowt", b""),
            [&common[..], b"sowt", &[0x00, 0x00]].concat()
        );
        // A count and three octets are already even.
        assert_eq!(
            comm_c(2, 65_536, 16, 44_100, *b"NONE", b"abc"),
            [&common[..], b"NONE", b"\x03abc"].concat()
        );
        // Apple's name for `NONE`: a count and 14 octets, then a pad.
        assert_eq!(
            comm_c(2, 65_536, 16, 44_100, *b"NONE", b"not compressed"),
            [&common[..], b"NONE", b"\x0Enot compressed\x00"].concat()
        );
    }

    #[test]
    #[should_panic(expected = "a Pascal string holds at most 255 octets")]
    fn refuses_a_compression_name_longer_than_a_count_octet_holds() {
        let _ = comm_c(1, 0, 8, 8_000, *b"NONE", &[b'a'; 256]);
    }

    #[test]
    fn writes_the_sound_data_after_its_offset() {
        assert_eq!(
            ssnd(0, 0, &[0x00, 0x01, 0xFF]),
            [0, 0, 0, 0, 0, 0, 0, 0, 0x00, 0x01, 0xFF]
        );
        // Four octets of offset before the first frame, in blocks of 4,096.
        assert_eq!(
            ssnd(4, 4_096, &[0xAB]),
            [0, 0, 0, 4, 0, 0, 0x10, 0, 0, 0, 0, 0, 0xAB]
        );
    }

    #[test]
    fn wraps_chunks_in_an_aiff_form() {
        // The same file ffmpeg wrote, as in the byte writers' tests.
        let file = aiff(
            *b"AIFF",
            &chunks(|form| {
                form.aiff_chunk(*b"COMM", &comm(1, 3, 8, 8_000))
                    .aiff_chunk(*b"SSND", &ssnd(0, 0, &[0x00, 0x01, 0xFF]));
            }),
        );
        assert_eq!(
            file,
            [
                &b"FORM\x00\x00\x00\x32AIFF"[..],
                b"COMM\x00\x00\x00\x12",
                &[0x00, 0x01, 0x00, 0x00, 0x00, 0x03, 0x00, 0x08],
                &[0x40, 0x0B, 0xFA, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                b"SSND\x00\x00\x00\x0B",
                &[0x00; 8],
                &[0x00, 0x01, 0xFF, 0x00],
            ]
            .concat()
        );
        assert_eq!(aiff(*b"AIFC", &[]), b"FORM\x00\x00\x00\x04AIFC");
    }

    #[test]
    fn writes_sample_rates_as_80_bit_extended_numbers() {
        // 44,100 Hz is 0x400E AC44 0000 0000 0000 in every AIFF file at
        // that rate; the others follow the same layout.
        let cases: [(u32, [u8; 10]); 7] = [
            (0, [0x00; 10]),
            (1, [0x3F, 0xFF, 0x80, 0, 0, 0, 0, 0, 0, 0]),
            (8_000, [0x40, 0x0B, 0xFA, 0, 0, 0, 0, 0, 0, 0]),
            (22_050, [0x40, 0x0D, 0xAC, 0x44, 0, 0, 0, 0, 0, 0]),
            (44_100, [0x40, 0x0E, 0xAC, 0x44, 0, 0, 0, 0, 0, 0]),
            (48_000, [0x40, 0x0E, 0xBB, 0x80, 0, 0, 0, 0, 0, 0]),
            (u32::MAX, [0x40, 0x1E, 0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0]),
        ];
        for (value, octets) in cases {
            assert_eq!(extended(value), octets, "{value}");
        }
    }

    proptest! {
        #[test]
        fn every_extended_number_reads_back_as_its_value(value in any::<u32>()) {
            // An independent reading: shift the significand right until its
            // binary point, 63 bits after the integer bit, lines up with the
            // unbiased exponent.
            let octets = extended(value);
            let exponent = u16::from_be_bytes([octets[0], octets[1]]);
            let significand = u64::from_be_bytes(octets[2..].try_into().unwrap());
            if value == 0 {
                prop_assert_eq!((exponent, significand), (0, 0));
            } else {
                prop_assert!(significand >> 63 == 1, "not normalised: {:016X}", significand);
                let shift = 16_383 + 63 - u32::from(exponent);
                prop_assert_eq!(significand >> shift, u64::from(value));
                prop_assert_eq!(significand & ((1 << shift) - 1), 0);
            }
        }
    }
}
