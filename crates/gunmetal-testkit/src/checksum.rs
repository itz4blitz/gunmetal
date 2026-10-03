//! Checksums that media formats embed.
//!
//! Each is written bit by bit from its definition, for the tests alone, and
//! shares nothing with any checksum in the code under test. A builder uses
//! them to write valid files; a test uses them to compute the checksum a
//! parser should accept or refuse.
//!
//! The CRCs are named by the formats that use them. Their parameters, in
//! the terms of the CRC `RevEng` catalogue, are given with each.

/// FLAC's frame header CRC-8: polynomial 0x07, initial value 0, not
/// reflected, no final XOR (CRC-8/SMBUS).
#[must_use]
pub fn crc8_flac(bytes: &[u8]) -> u8 {
    let mut crc = 0_u8;
    for &byte in bytes {
        crc ^= byte;
        for _ in 0..8 {
            crc = if crc & 0x80 == 0 {
                crc << 1
            } else {
                (crc << 1) ^ 0x07
            };
        }
    }
    crc
}

/// FLAC's whole-frame CRC-16: polynomial 0x8005, initial value 0, not
/// reflected, no final XOR (CRC-16/UMTS).
#[must_use]
pub fn crc16_flac(bytes: &[u8]) -> u16 {
    let mut crc = 0_u16;
    for &byte in bytes {
        crc ^= u16::from(byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 == 0 {
                crc << 1
            } else {
                (crc << 1) ^ 0x8005
            };
        }
    }
    crc
}

/// The CRC-32 of PNG, zlib's gzip trailer and many others: polynomial
/// 0x04C11DB7 reflected, initial value and final XOR all ones
/// (CRC-32/ISO-HDLC).
#[must_use]
pub fn crc32_ieee(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 0 {
                crc >> 1
            } else {
                (crc >> 1) ^ 0xEDB8_8320
            };
        }
    }
    !crc
}

/// The CRC-32 of an Ogg page, computed with the page's CRC field zeroed:
/// polynomial 0x04C11DB7, initial value 0, not reflected, no final XOR.
#[must_use]
pub fn crc32_ogg(bytes: &[u8]) -> u32 {
    let mut crc = 0_u32;
    for &byte in bytes {
        crc ^= u32::from(byte) << 24;
        for _ in 0..8 {
            crc = if crc & 0x8000_0000 == 0 {
                crc << 1
            } else {
                (crc << 1) ^ 0x04C1_1DB7
            };
        }
    }
    crc
}

/// The largest prime below 2^16, which Adler-32 reduces its sums by.
const ADLER_MODULUS: u32 = 65_521;

/// Adler-32 (RFC 1950, section 8.2), the checksum that ends a zlib stream.
#[must_use]
pub fn adler32(bytes: &[u8]) -> u32 {
    let mut low = 1_u32;
    let mut high = 0_u32;
    for &byte in bytes {
        low = (low + u32::from(byte)) % ADLER_MODULUS;
        high = (high + low) % ADLER_MODULUS;
    }
    (high << 16) + low
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::collection::vec;
    use proptest::prelude::*;

    /// The input every entry in the CRC `RevEng` catalogue gives a check
    /// value for.
    const CHECK_INPUT: &[u8] = b"123456789";

    #[test]
    fn matches_the_crc_catalogue_check_values() {
        // CRC-8/SMBUS, CRC-16/UMTS and CRC-32/ISO-HDLC are FLAC's two CRCs
        // and the IEEE CRC under their catalogue names.
        assert_eq!(crc8_flac(CHECK_INPUT), 0xF4);
        assert_eq!(crc16_flac(CHECK_INPUT), 0xFEE8);
        assert_eq!(crc32_ieee(CHECK_INPUT), 0xCBF4_3926);
        // Ogg's CRC is CRC-32/CKSUM without the final inversion, and
        // CRC-32/CKSUM's check value is 0x765E7680.
        assert_eq!(crc32_ogg(CHECK_INPUT), !0x765E_7680);
    }

    #[test]
    fn checks_empty_input_as_the_initial_value() {
        assert_eq!(crc8_flac(&[]), 0x00);
        assert_eq!(crc16_flac(&[]), 0x0000);
        assert_eq!(crc32_ieee(&[]), 0x0000_0000);
        assert_eq!(crc32_ogg(&[]), 0x0000_0000);
        assert_eq!(adler32(&[]), 0x0000_0001);
    }

    /// The first frame libFLAC 1.5.0 wrote for 64 frames of 16-bit mono
    /// silence at 8 kHz: a six-octet header, its CRC-8, one constant
    /// subframe of value zero, and the frame's CRC-16.
    const FLAC_FRAME: [u8; 12] = [
        0xFF, 0xF8, 0x64, 0x08, 0x00, 0x3F, // header
        0x5E, // CRC-8 of the header
        0x00, 0x00, 0x00, // constant subframe
        0xC6, 0x3C, // CRC-16 of everything before it
    ];

    #[test]
    fn matches_the_crcs_in_a_real_flac_frame() {
        assert_eq!(crc8_flac(&FLAC_FRAME[..6]), 0x5E);
        assert_eq!(crc16_flac(&FLAC_FRAME[..10]), 0xC63C);
    }

    #[test]
    fn matches_the_crcs_of_real_png_chunks() {
        // A 1x1 grey PNG as `ffmpeg -pix_fmt gray` wrote it. Each chunk's
        // CRC covers its type and data.
        let ihdr = b"IHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x00\x00\x00\x00";
        assert_eq!(crc32_ieee(ihdr), 0x3A7E_9B55);
        let idat = b"IDAT\x78\x9C\x63\x64\x00\x00\x00\x04\x00\x02";
        assert_eq!(crc32_ieee(idat), 0x2164_AD6A);
        assert_eq!(crc32_ieee(b"IEND"), 0xAE42_6082);
    }

    #[test]
    fn matches_the_crcs_of_real_ogg_pages() {
        // The first and last pages `ffmpeg -c:a flac -f ogg` wrote for the
        // same silence, with each page's CRC field (octets 22 to 25) zeroed,
        // as the checksum is computed.
        let first = [
            &b"OggS\x00\x02"[..],
            &[0x00; 8],                // granule position
            &[0x6A, 0x4C, 0xA9, 0x4C], // serial number
            &[0x00; 4],                // page sequence number
            &[0x00; 4],                // CRC field, zeroed
            &[0x01, 0x33],             // one segment of 51 octets
            b"\x7FFLAC\x01\x00\x00\x01fLaC\x00\x00\x00\x22",
            &[0x00, 0x40, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x95],
            &[0x01, 0xF4, 0x00, 0xF0, 0x00, 0x00, 0x00, 0x00],
            &[0x00; 16],
        ]
        .concat();
        assert_eq!(first.len(), 79);
        assert_eq!(crc32_ogg(&first), 0x964E_C25E);

        let last = [
            &b"OggS\x00\x04"[..],
            &[0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // granule position
            &[0x6A, 0x4C, 0xA9, 0x4C],                         // serial number
            &[0x02, 0x00, 0x00, 0x00],                         // page sequence number
            &[0x00; 4],                                        // CRC field, zeroed
            &[0x01, 0x0C],                                     // one segment of 12 octets
            &FLAC_FRAME,
        ]
        .concat();
        assert_eq!(crc32_ogg(&last), 0xC76F_3F1D);
    }

    #[test]
    fn matches_published_and_real_adler32_values() {
        // The worked example in Wikipedia's Adler-32 article.
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
        // The zlib stream in the PNG above ends with the Adler-32 of the
        // two octets it inflates to: 00 04 00 02.
        assert_eq!(adler32(&[0x01, 0x00]), 0x0004_0002);
        // Sums past the modulus 65,521, as zlib computes them.
        assert_eq!(adler32(&[0xFF; 300]), 0xB90F_2AE4);
        assert_eq!(adler32(&[0xFF; 5_553]), 0x8E29_9C8B);
    }

    /// Adler-32 from its closed form, written only for these tests: the
    /// low sum is one plus every octet, and the high sum weights each octet
    /// by how many sums it is part of.
    fn adler32_closed_form(bytes: &[u8]) -> u32 {
        let count = u64::try_from(bytes.len()).unwrap();
        let low: u64 = 1 + bytes.iter().map(|&b| u64::from(b)).sum::<u64>();
        let high: u64 = count
            + (0..count)
                .zip(bytes)
                .map(|(i, &b)| (count - i) * u64::from(b))
                .sum::<u64>();
        u32::try_from((high % 65_521) << 16 | (low % 65_521)).unwrap()
    }

    proptest! {
        #[test]
        fn appending_a_flac_or_ogg_crc_leaves_a_zero_remainder(
            message in vec(any::<u8>(), 0..64),
        ) {
            let mut with_crc8 = message.clone();
            with_crc8.push(crc8_flac(&message));
            prop_assert_eq!(crc8_flac(&with_crc8), 0);

            let mut with_crc16 = message.clone();
            with_crc16.extend(crc16_flac(&message).to_be_bytes());
            prop_assert_eq!(crc16_flac(&with_crc16), 0);

            let mut with_ogg = message.clone();
            with_ogg.extend(crc32_ogg(&message).to_be_bytes());
            prop_assert_eq!(crc32_ogg(&with_ogg), 0);
        }

        #[test]
        fn appending_an_ieee_crc_leaves_the_published_residue(
            message in vec(any::<u8>(), 0..64),
        ) {
            let mut with_crc = message.clone();
            with_crc.extend(crc32_ieee(&message).to_le_bytes());
            prop_assert_eq!(crc32_ieee(&with_crc), 0x2144_DF1C);
        }

        #[test]
        fn adler32_matches_its_closed_form(
            message in vec(any::<u8>(), 0..1_024),
        ) {
            prop_assert_eq!(adler32(&message), adler32_closed_form(&message));
        }
    }
}
