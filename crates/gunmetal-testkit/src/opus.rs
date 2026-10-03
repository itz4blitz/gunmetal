//! Builders for the stream headers of the two Ogg audio codecs: Opus
//! (RFC 7845, section 5) and Vorbis (the Vorbis I specification, sections
//! 4.2 and 5.2).
//!
//! Every builder writes the field values it is given as they are, valid or
//! not, so a test can build exactly the header it means to refuse.

use crate::bytes::Bytes;

/// The channel mapping table of an Opus identification header (RFC 7845,
/// section 5.1.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappingTable {
    /// The number of Opus streams in each packet.
    pub streams: u8,
    /// How many of those streams are coupled, carrying two channels.
    pub coupled: u8,
    /// For each output channel, the decoded channel it plays.
    pub mapping: Vec<u8>,
}

/// The fields of an Opus identification header, `OpusHead` (RFC 7845,
/// section 5.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpusHead {
    /// The version octet.
    pub version: u8,
    /// The output channel count.
    pub channels: u8,
    /// Samples at 48 kHz to discard from the start of the decoded stream.
    pub pre_skip: u16,
    /// The sample rate of the original input, in hertz.
    pub input_sample_rate: u32,
    /// The output gain, a Q7.8 number of decibels.
    pub output_gain: i16,
    /// The channel mapping family.
    pub family: u8,
    /// The channel mapping table, written after the family when present.
    pub table: Option<MappingTable>,
}

impl OpusHead {
    /// The header `opusenc` writes for a 48 kHz stereo input: version 1,
    /// two channels, 312 samples of pre-skip, no gain and family 0.
    #[must_use]
    pub fn stereo() -> Self {
        Self {
            version: 1,
            channels: 2,
            pre_skip: 312,
            input_sample_rate: 48_000,
            output_gain: 0,
            family: 0,
            table: None,
        }
    }

    /// The header's octets.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut header = Bytes::new();
        header
            .bytes(b"OpusHead")
            .u8(self.version)
            .u8(self.channels)
            .u16_le(self.pre_skip)
            .u32_le(self.input_sample_rate)
            .bytes(&self.output_gain.to_le_bytes())
            .u8(self.family);
        if let Some(table) = &self.table {
            header
                .u8(table.streams)
                .u8(table.coupled)
                .bytes(&table.mapping);
        }
        header.into_vec()
    }
}

/// A Vorbis comment block: the vendor string and the comments, each
/// behind its 32-bit little-endian length, after the count of comments.
///
/// # Panics
///
/// Panics when a length or the count does not fit in 32 bits.
#[must_use]
pub fn comment_block(vendor: &[u8], comments: &[&[u8]]) -> Vec<u8> {
    let mut block = Bytes::new();
    block.u32_le(length(vendor.len())).bytes(vendor);
    block.u32_le(length(comments.len()));
    for comment in comments {
        block.u32_le(length(comment.len())).bytes(comment);
    }
    block.into_vec()
}

/// A length or count as the 32-bit field that holds it.
fn length(len: usize) -> u32 {
    u32::try_from(len).expect("a length fits in 32 bits")
}

/// An Opus comment header, `OpusTags` (RFC 7845, section 5.2): the magic
/// signature, then [`comment_block`].
///
/// # Panics
///
/// Panics as [`comment_block`] does.
#[must_use]
pub fn opus_tags(vendor: &[u8], comments: &[&[u8]]) -> Vec<u8> {
    let mut header = Bytes::new();
    header
        .bytes(b"OpusTags")
        .bytes(&comment_block(vendor, comments));
    header.into_vec()
}

/// The fields of a Vorbis identification header (Vorbis I, section
/// 4.2.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VorbisIdent {
    /// The Vorbis version.
    pub version: u32,
    /// The channel count.
    pub channels: u8,
    /// The sample rate in hertz.
    pub sample_rate: u32,
    /// The largest bitrate, in bits per second.
    pub bitrate_maximum: i32,
    /// The nominal bitrate, in bits per second.
    pub bitrate_nominal: i32,
    /// The smallest bitrate, in bits per second.
    pub bitrate_minimum: i32,
    /// The base-2 logarithms of the short and the long block size.
    pub block_exponents: [u8; 2],
    /// The last octet, whose lowest bit is the framing flag.
    pub framing: u8,
}

impl VorbisIdent {
    /// The header `libvorbis` writes for 44.1 kHz stereo at its default
    /// quality: a nominal 112,000 bits per second and blocks of 256 and
    /// 2,048 samples.
    #[must_use]
    pub fn stereo() -> Self {
        Self {
            version: 0,
            channels: 2,
            sample_rate: 44_100,
            bitrate_maximum: 0,
            bitrate_nominal: 112_000,
            bitrate_minimum: 0,
            block_exponents: [8, 11],
            framing: 1,
        }
    }

    /// The header's octets.
    ///
    /// # Panics
    ///
    /// Panics when a block exponent does not fit in four bits.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let [short, long] = self.block_exponents;
        assert!(
            short < 16 && long < 16,
            "a block exponent fits in four bits, not {short} or {long}"
        );
        let mut header = Bytes::new();
        header
            .u8(1)
            .bytes(b"vorbis")
            .u32_le(self.version)
            .u8(self.channels)
            .u32_le(self.sample_rate)
            .bytes(&self.bitrate_maximum.to_le_bytes())
            .bytes(&self.bitrate_nominal.to_le_bytes())
            .bytes(&self.bitrate_minimum.to_le_bytes())
            // Vorbis packs fields from the lowest bit up, so the short
            // block's exponent is the low nibble and the long one's the
            // high nibble.
            .u8(long * 16 + short)
            .u8(self.framing);
        header.into_vec()
    }
}

/// A Vorbis comment header (Vorbis I, section 5.2.1): the packet type 3
/// and `vorbis`, then [`comment_block`], then the octet `framing`, whose
/// lowest bit is the framing bit.
///
/// # Panics
///
/// Panics as [`comment_block`] does.
#[must_use]
pub fn vorbis_comments(vendor: &[u8], comments: &[&[u8]], framing: u8) -> Vec<u8> {
    let mut header = Bytes::new();
    header
        .u8(3)
        .bytes(b"vorbis")
        .bytes(&comment_block(vendor, comments))
        .u8(framing);
    header.into_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_the_stereo_header_opusenc_writes() {
        assert_eq!(
            OpusHead::stereo(),
            OpusHead {
                version: 1,
                channels: 2,
                pre_skip: 312,
                input_sample_rate: 48_000,
                output_gain: 0,
                family: 0,
                table: None,
            }
        );
        assert_eq!(
            OpusHead::stereo().to_bytes(),
            b"OpusHead\x01\x02\x38\x01\x80\xBB\x00\x00\x00\x00\x00"
        );
    }

    #[test]
    fn writes_a_mapping_table_after_the_family() {
        // libopus's 5.1 layout in Vorbis channel order: four streams, two
        // of them coupled, at 44.1 kHz with -1.5 dB of output gain.
        let head = OpusHead {
            version: 1,
            channels: 6,
            pre_skip: 3_840,
            input_sample_rate: 44_100,
            output_gain: -384,
            family: 1,
            table: Some(MappingTable {
                streams: 4,
                coupled: 2,
                mapping: vec![0, 4, 1, 2, 3, 5],
            }),
        };
        assert_eq!(
            head.to_bytes(),
            b"OpusHead\x01\x06\x00\x0F\x44\xAC\x00\x00\x80\xFE\x01\x04\x02\x00\x04\x01\x02\x03\x05"
        );
    }

    #[test]
    fn writes_a_comment_block_with_its_lengths() {
        assert_eq!(
            comment_block(b"libopus 1.4", &[b"TITLE=A", b""]),
            b"\x0B\x00\x00\x00libopus 1.4\x02\x00\x00\x00\x07\x00\x00\x00TITLE=A\x00\x00\x00\x00"
        );
        assert_eq!(comment_block(b"", &[]), b"\x00\x00\x00\x00\x00\x00\x00\x00");
    }

    #[test]
    fn writes_an_opus_comment_header() {
        assert_eq!(
            opus_tags(b"x", &[b"A=b"]),
            b"OpusTags\x01\x00\x00\x00x\x01\x00\x00\x00\x03\x00\x00\x00A=b"
        );
    }

    #[test]
    fn writes_the_stereo_header_libvorbis_writes() {
        assert_eq!(
            VorbisIdent::stereo(),
            VorbisIdent {
                version: 0,
                channels: 2,
                sample_rate: 44_100,
                bitrate_maximum: 0,
                bitrate_nominal: 112_000,
                bitrate_minimum: 0,
                block_exponents: [8, 11],
                framing: 1,
            }
        );
        assert_eq!(
            VorbisIdent::stereo().to_bytes(),
            b"\x01vorbis\x00\x00\x00\x00\x02\x44\xAC\x00\x00\x00\x00\x00\x00\x80\xB5\x01\x00\x00\x00\x00\x00\xB8\x01"
        );
    }

    #[test]
    fn writes_every_field_of_a_vorbis_identification_header_in_place() {
        let header = VorbisIdent {
            version: 0x0403_0201,
            channels: 0xFF,
            sample_rate: 0x0807_0605,
            bitrate_maximum: -2,
            bitrate_nominal: 0x0C0B_0A09,
            bitrate_minimum: i32::MIN,
            block_exponents: [0x0F, 0x06],
            framing: 0xFE,
        };
        assert_eq!(
            header.to_bytes(),
            b"\x01vorbis\x01\x02\x03\x04\xFF\x05\x06\x07\x08\xFE\xFF\xFF\xFF\x09\x0A\x0B\x0C\x00\x00\x00\x80\x6F\xFE"
        );
    }

    #[test]
    #[should_panic(expected = "a block exponent fits in four bits, not 16 or 0")]
    fn refuses_a_short_block_exponent_wider_than_four_bits() {
        let header = VorbisIdent {
            block_exponents: [16, 0],
            ..VorbisIdent::stereo()
        };
        let _ = header.to_bytes();
    }

    #[test]
    #[should_panic(expected = "a block exponent fits in four bits, not 0 or 16")]
    fn refuses_a_long_block_exponent_wider_than_four_bits() {
        let header = VorbisIdent {
            block_exponents: [0, 16],
            ..VorbisIdent::stereo()
        };
        let _ = header.to_bytes();
    }
    #[test]
    fn writes_a_vorbis_comment_header_with_its_framing_octet() {
        assert_eq!(
            vorbis_comments(b"Xiph", &[b"A=b"], 0x01),
            b"\x03vorbis\x04\x00\x00\x00Xiph\x01\x00\x00\x00\x03\x00\x00\x00A=b\x01"
        );
        assert_eq!(
            vorbis_comments(b"", &[], 0x00),
            b"\x03vorbis\x00\x00\x00\x00\x00\x00\x00\x00\x00"
        );
    }
}
