//! A builder for the start of a FLAC stream: the `fLaC` marker and the
//! metadata blocks of RFC 9639, section 8.
//!
//! Every field is written as the specification lays it out, from values
//! the test chooses, so a test can build valid streams and, through
//! [`Block::Raw`] and [`header`], streams with any block type or length.
//! The builder checks only what it needs to lay the bytes out; it is the
//! parser's job, not the builder's, to refuse a stream.

use crate::bytes::{Bits, Bytes};

/// The four octets every FLAC stream starts with.
pub const MARKER: [u8; 4] = *b"fLaC";

/// The sample number of a placeholder seek point (RFC 9639, section
/// 8.5.1).
pub const PLACEHOLDER: u64 = u64::MAX;

/// The fields of a STREAMINFO block (RFC 9639, section 8.2, Table 3), as
/// numbers rather than as they are stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamInfo {
    /// The minimum block size in samples.
    pub min_block_size: u16,
    /// The maximum block size in samples.
    pub max_block_size: u16,
    /// The minimum frame size in octets, 0 when unknown; 24 bits.
    pub min_frame_size: u32,
    /// The maximum frame size in octets, 0 when unknown; 24 bits.
    pub max_frame_size: u32,
    /// The sample rate in hertz; 20 bits.
    pub sample_rate: u32,
    /// The number of channels, 1 to 8.
    pub channels: u8,
    /// Bits per sample, 1 to 32.
    pub bits_per_sample: u8,
    /// Interchannel samples in the stream, 0 when unknown; 36 bits.
    pub total_samples: u64,
    /// The MD5 signature of the decoded audio, all zeros when unknown.
    pub md5: [u8; 16],
}

impl StreamInfo {
    /// The 34-octet block body.
    ///
    /// # Panics
    ///
    /// Panics when a field does not fit its width, or when `channels` or
    /// `bits_per_sample` is zero, since the stream stores each minus one.
    #[must_use]
    pub fn body(&self) -> Vec<u8> {
        let mut fields = Bits::new();
        fields
            .put(16, self.min_block_size.into())
            .put(16, self.max_block_size.into())
            .put(24, self.min_frame_size.into())
            .put(24, self.max_frame_size.into())
            .put(20, self.sample_rate.into())
            .put(3, minus_one(self.channels, "channels"))
            .put(5, minus_one(self.bits_per_sample, "bits per sample"))
            .put(36, self.total_samples);
        let mut body = fields.into_vec();
        body.extend_from_slice(&self.md5);
        body
    }
}

/// `value` minus one, for the fields the stream stores that way.
fn minus_one(value: u8, field: &str) -> u64 {
    u64::from(value)
        .checked_sub(1)
        .unwrap_or_else(|| panic!("{field} must be at least 1"))
}

/// One seek point (RFC 9639, section 8.5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeekPoint {
    /// The first sample of the target frame, or [`PLACEHOLDER`].
    pub sample: u64,
    /// Octets from the first frame header to the target frame's header.
    pub offset: u64,
    /// Samples in the target frame.
    pub samples: u16,
}

/// The fields of a PICTURE block (RFC 9639, section 8.8, Table 12). Each
/// length the block stores is the length of the field given here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    /// The picture type (Table 13).
    pub picture_type: u32,
    /// The media type string, or `-->` for a link.
    pub media_type: Vec<u8>,
    /// The description, in UTF-8.
    pub description: Vec<u8>,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Colour depth in bits per pixel.
    pub depth: u32,
    /// Colours used by an indexed picture, or 0.
    pub colors: u32,
    /// The picture data.
    pub data: Vec<u8>,
}

impl Picture {
    /// The block body.
    ///
    /// # Panics
    ///
    /// Panics when a field is longer than a 32-bit length can say.
    #[must_use]
    pub fn body(&self) -> Vec<u8> {
        let mut body = Bytes::new();
        body.u32_be(self.picture_type)
            .u32_be(length(&self.media_type))
            .bytes(&self.media_type)
            .u32_be(length(&self.description))
            .bytes(&self.description)
            .u32_be(self.width)
            .u32_be(self.height)
            .u32_be(self.depth)
            .u32_be(self.colors)
            .u32_be(length(&self.data))
            .bytes(&self.data);
        body.into_vec()
    }
}

/// The 32-bit length of a field.
fn length(field: &[u8]) -> u32 {
    u32::try_from(field.len()).expect("a picture field fits a 32-bit length")
}

/// One metadata block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// STREAMINFO, type 0.
    StreamInfo(StreamInfo),
    /// PADDING, type 1: this many zero octets.
    Padding(usize),
    /// APPLICATION, type 2.
    Application {
        /// The registered application ID.
        id: [u8; 4],
        /// The application's data.
        data: Vec<u8>,
    },
    /// SEEKTABLE, type 3.
    SeekTable(Vec<SeekPoint>),
    /// `VORBIS_COMMENT`, type 4, with the body given whole.
    VorbisComment(Vec<u8>),
    /// CUESHEET, type 5, with the body given whole.
    CueSheet(Vec<u8>),
    /// PICTURE, type 6.
    Picture(Picture),
    /// Any type code, 0 to 127, with any body.
    Raw {
        /// The block type.
        code: u8,
        /// The body.
        body: Vec<u8>,
    },
}

impl Block {
    /// The block type code.
    #[must_use]
    pub fn code(&self) -> u8 {
        match self {
            Self::StreamInfo(_) => 0,
            Self::Padding(_) => 1,
            Self::Application { .. } => 2,
            Self::SeekTable(_) => 3,
            Self::VorbisComment(_) => 4,
            Self::CueSheet(_) => 5,
            Self::Picture(_) => 6,
            Self::Raw { code, .. } => *code,
        }
    }

    /// The block body, without its header.
    ///
    /// # Panics
    ///
    /// Panics when [`StreamInfo::body`] or [`Picture::body`] would.
    #[must_use]
    pub fn body(&self) -> Vec<u8> {
        match self {
            Self::StreamInfo(info) => info.body(),
            Self::Padding(len) => {
                let mut body = Bytes::new();
                body.zeros(*len);
                body.into_vec()
            }
            Self::Application { id, data } => [id.as_slice(), data].concat(),
            Self::SeekTable(points) => {
                let mut body = Bytes::new();
                for point in points {
                    body.u64_be(point.sample)
                        .u64_be(point.offset)
                        .u16_be(point.samples);
                }
                body.into_vec()
            }
            Self::VorbisComment(body) | Self::CueSheet(body) | Self::Raw { body, .. } => {
                body.clone()
            }
            Self::Picture(picture) => picture.body(),
        }
    }

    /// The block header and body, flagged as the last block when `last` is
    /// set.
    ///
    /// # Panics
    ///
    /// Panics when the body is longer than a 24-bit length can say, or when
    /// [`Block::body`] would.
    #[must_use]
    pub fn encode(&self, last: bool) -> Vec<u8> {
        let body = self.body();
        let len = u32::try_from(body.len()).expect("a block body fits a 24-bit length");
        [header(last, self.code(), len), body].concat()
    }
}

/// A block header (RFC 9639, section 8.1): the last-block flag, a seven-bit
/// type and a 24-bit body length, which need not match any body.
///
/// # Panics
///
/// Panics when `code` is over 127 or `len` over 24 bits.
#[must_use]
pub fn header(last: bool, code: u8, len: u32) -> Vec<u8> {
    let mut fields = Bits::new();
    fields.put(1, last.into()).put(7, code.into());
    let mut header = Bytes::new();
    header.bytes(&fields.into_vec()).u24_be(len);
    header.into_vec()
}

/// The marker, then each of `blocks` with the last-block flag on the final
/// one only.
///
/// # Panics
///
/// Panics when [`Block::encode`] would.
#[must_use]
pub fn stream(blocks: &[Block]) -> Vec<u8> {
    let mut stream = MARKER.to_vec();
    let final_index = blocks.len().saturating_sub(1);
    for (index, block) in blocks.iter().enumerate() {
        stream.extend(block.encode(index == final_index));
    }
    stream
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The STREAMINFO of RFC 9639, Appendix D.1: 4,096-sample blocks,
    /// 15-octet frames, 44.1 kHz, two channels of 16 bits, one sample.
    fn example_1_stream_info() -> StreamInfo {
        StreamInfo {
            min_block_size: 4_096,
            max_block_size: 4_096,
            min_frame_size: 15,
            max_frame_size: 15,
            sample_rate: 44_100,
            channels: 2,
            bits_per_sample: 16,
            total_samples: 1,
            md5: [
                0x3E, 0x84, 0xB4, 0x18, 0x07, 0xDC, 0x69, 0x03, 0x07, 0x58, 0x6A, 0x3D, 0xAD, 0x1A,
                0x2E, 0x0F,
            ],
        }
    }

    /// The metadata of RFC 9639, Appendix D.1, as section D.1.1 prints it:
    /// everything before the frame at 0x2A.
    const EXAMPLE_1_METADATA: [u8; 42] = [
        0x66, 0x4C, 0x61, 0x43, // fLaC
        0x80, 0x00, 0x00, 0x22, // last, STREAMINFO, 34 octets
        0x10, 0x00, 0x10, 0x00, // block sizes 4096 and 4096
        0x00, 0x00, 0x0F, 0x00, 0x00, 0x0F, // frame sizes 15 and 15
        0x0A, 0xC4, 0x42, 0xF0, 0x00, 0x00, 0x00, 0x01, // 44100 Hz, 2 x 16 bits, 1 sample
        0x3E, 0x84, 0xB4, 0x18, 0x07, 0xDC, 0x69, 0x03, // MD5
        0x07, 0x58, 0x6A, 0x3D, 0xAD, 0x1A, 0x2E, 0x0F,
    ];

    /// The metadata of RFC 9639, Appendix D.2, as section D.2.1 prints it:
    /// everything before the first frame at 0x88.
    const EXAMPLE_2_METADATA: [u8; 136] = [
        0x66, 0x4C, 0x61, 0x43, // fLaC
        0x00, 0x00, 0x00, 0x22, // STREAMINFO, 34 octets
        0x00, 0x10, 0x00, 0x10, // block sizes 16 and 16
        0x00, 0x00, 0x17, 0x00, 0x00, 0x44, // frame sizes 23 and 68
        0x0A, 0xC4, 0x42, 0xF0, 0x00, 0x00, 0x00, 0x13, // 44100 Hz, 2 x 16 bits, 19 samples
        0xD5, 0xB0, 0x56, 0x49, 0x75, 0xE9, 0x8B, 0x8D, // MD5
        0x8B, 0x93, 0x04, 0x22, 0x75, 0x7B, 0x81, 0x03, //
        0x03, 0x00, 0x00, 0x12, // SEEKTABLE, 18 octets
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // sample 0
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // offset 0
        0x00, 0x10, // 16 samples
        0x04, 0x00, 0x00, 0x3A, // VORBIS_COMMENT, 58 octets
        0x20, 0x00, 0x00, 0x00, // vendor string of 32 octets
        b'r', b'e', b'f', b'e', b'r', b'e', b'n', b'c', b'e', b' ', b'l', b'i', b'b', b'F', b'L',
        b'A', b'C', b' ', b'1', b'.', b'3', b'.', b'3', b' ', b'2', b'0', b'1', b'9', b'0', b'8',
        b'0', b'4', //
        0x01, 0x00, 0x00, 0x00, // one field
        0x0E, 0x00, 0x00, 0x00, // of 14 octets
        b'T', b'I', b'T', b'L', b'E', b'=', 0xD7, 0xA9, 0xD7, 0x9C, 0xD7, 0x95, 0xD7, 0x9D, //
        0x81, 0x00, 0x00, 0x06, // last, PADDING, 6 octets
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    #[test]
    fn writes_the_streaminfo_of_rfc_9639_example_1() {
        assert_eq!(example_1_stream_info().body(), EXAMPLE_1_METADATA[8..]);
    }

    #[test]
    fn writes_the_metadata_of_rfc_9639_example_1() {
        assert_eq!(
            stream(&[Block::StreamInfo(example_1_stream_info())]),
            EXAMPLE_1_METADATA
        );
    }

    #[test]
    fn writes_the_metadata_of_rfc_9639_example_2() {
        let info = StreamInfo {
            min_block_size: 16,
            max_block_size: 16,
            min_frame_size: 23,
            max_frame_size: 68,
            total_samples: 19,
            md5: [
                0xD5, 0xB0, 0x56, 0x49, 0x75, 0xE9, 0x8B, 0x8D, 0x8B, 0x93, 0x04, 0x22, 0x75, 0x7B,
                0x81, 0x03,
            ],
            ..example_1_stream_info()
        };
        let comment = EXAMPLE_2_METADATA[68..126].to_vec();
        assert_eq!(
            stream(&[
                Block::StreamInfo(info),
                Block::SeekTable(vec![SeekPoint {
                    sample: 0,
                    offset: 0,
                    samples: 16,
                }]),
                Block::VorbisComment(comment),
                Block::Padding(6),
            ]),
            EXAMPLE_2_METADATA
        );
    }

    /// RFC 9639, Appendix D.3: 32 kHz, one channel of 8 bits, 24 samples.
    #[test]
    fn writes_the_streaminfo_of_rfc_9639_example_3() {
        let info = StreamInfo {
            min_frame_size: 31,
            max_frame_size: 31,
            sample_rate: 32_000,
            channels: 1,
            bits_per_sample: 8,
            total_samples: 24,
            md5: [
                0xF8, 0xF9, 0xE3, 0x96, 0xF5, 0xCB, 0xCF, 0xC6, 0xDC, 0x80, 0x7F, 0x99, 0x77, 0x90,
                0x6B, 0x32,
            ],
            ..example_1_stream_info()
        };
        assert_eq!(
            info.body(),
            [
                0x10, 0x00, 0x10, 0x00, // block sizes 4096 and 4096
                0x00, 0x00, 0x1F, 0x00, 0x00, 0x1F, // frame sizes 31 and 31
                0x07, 0xD0, 0x00, 0x70, 0x00, 0x00, 0x00, 0x18, // 32 kHz, 1 x 8 bits, 24
                0xF8, 0xF9, 0xE3, 0x96, 0xF5, 0xCB, 0xCF, 0xC6, // MD5
                0xDC, 0x80, 0x7F, 0x99, 0x77, 0x90, 0x6B, 0x32,
            ]
        );
    }

    #[test]
    fn writes_the_widest_value_of_every_streaminfo_field() {
        let info = StreamInfo {
            min_block_size: u16::MAX,
            max_block_size: u16::MAX,
            min_frame_size: 0xFF_FFFF,
            max_frame_size: 0xFF_FFFF,
            sample_rate: 0xF_FFFF,
            channels: 8,
            bits_per_sample: 32,
            total_samples: 0xF_FFFF_FFFF,
            md5: [0xFF; 16],
        };
        assert_eq!(info.body(), [0xFF; 34]);
    }

    #[test]
    #[should_panic(expected = "channels must be at least 1")]
    fn refuses_zero_channels() {
        let info = StreamInfo {
            channels: 0,
            ..example_1_stream_info()
        };
        let _ = info.body();
    }

    #[test]
    #[should_panic(expected = "bits per sample must be at least 1")]
    fn refuses_zero_bits_per_sample() {
        let info = StreamInfo {
            bits_per_sample: 0,
            ..example_1_stream_info()
        };
        let _ = info.body();
    }

    #[test]
    #[should_panic(expected = "8 does not fit in 3 bits")]
    fn refuses_nine_channels() {
        let info = StreamInfo {
            channels: 9,
            ..example_1_stream_info()
        };
        let _ = info.body();
    }

    /// A picture laid out field by field from RFC 9639, Table 12.
    #[test]
    fn writes_a_picture_field_by_field() {
        let picture = Picture {
            picture_type: 3,
            media_type: b"image/png".to_vec(),
            description: b"Cover".to_vec(),
            width: 0x0102_0304,
            height: 0x0506_0708,
            depth: 24,
            colors: 0x0A0B_0C0D,
            data: vec![0x89, b'P', b'N', b'G'],
        };
        let mut expected = vec![
            0x00, 0x00, 0x00, 0x03, // front cover
            0x00, 0x00, 0x00, 0x09, // media type of 9 octets
        ];
        expected.extend(b"image/png");
        expected.extend([0x00, 0x00, 0x00, 0x05]); // description of 5 octets
        expected.extend(b"Cover");
        expected.extend([
            0x01, 0x02, 0x03, 0x04, // width
            0x05, 0x06, 0x07, 0x08, // height
            0x00, 0x00, 0x00, 0x18, // 24 bits per pixel
            0x0A, 0x0B, 0x0C, 0x0D, // colours
            0x00, 0x00, 0x00, 0x04, // data of 4 octets
            0x89, b'P', b'N', b'G',
        ]);
        assert_eq!(picture.body(), expected);
        let mut block = vec![0x86, 0x00, 0x00, 0x32]; // last, PICTURE, 50 octets
        block.extend(&expected);
        assert_eq!(Block::Picture(picture).encode(true), block);
    }

    #[test]
    fn writes_seek_points_and_placeholders() {
        let table = Block::SeekTable(vec![
            SeekPoint {
                sample: 0x0102_0304_0506_0708,
                offset: 0x1112_1314_1516_1718,
                samples: 0x2122,
            },
            SeekPoint {
                sample: PLACEHOLDER,
                offset: 0,
                samples: 0,
            },
        ]);
        let mut expected = vec![0x03, 0x00, 0x00, 0x24]; // SEEKTABLE, 36 octets
        expected.extend([0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
        expected.extend([0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18]);
        expected.extend([0x21, 0x22]);
        expected.extend([0xFF; 8]);
        expected.extend([0x00; 10]);
        assert_eq!(table.encode(false), expected);
    }

    /// An application block starts with its registered ID (RFC 9639,
    /// Table 5); `xmcd` is in the registry of section 12.2.
    #[test]
    fn writes_application_cuesheet_and_raw_blocks_with_their_codes() {
        let application = Block::Application {
            id: *b"xmcd",
            data: vec![0xAA, 0xBB],
        };
        assert_eq!(
            application.encode(false),
            [0x02, 0x00, 0x00, 0x06, b'x', b'm', b'c', b'd', 0xAA, 0xBB]
        );
        assert_eq!(
            Block::CueSheet(vec![0xCC]).encode(true),
            [0x85, 0x00, 0x00, 0x01, 0xCC]
        );
        let reserved = Block::Raw {
            code: 126,
            body: vec![0xDD, 0xEE],
        };
        assert_eq!(reserved.encode(false), [0x7E, 0x00, 0x00, 0x02, 0xDD, 0xEE]);
        let forbidden = Block::Raw {
            code: 127,
            body: vec![],
        };
        assert_eq!(forbidden.encode(true), [0xFF, 0x00, 0x00, 0x00]);
        assert_eq!(Block::Padding(0).encode(true), [0x81, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn names_each_block_by_its_type_code() {
        let codes: Vec<u8> = [
            Block::StreamInfo(example_1_stream_info()),
            Block::Padding(1),
            Block::Application {
                id: [0; 4],
                data: vec![],
            },
            Block::SeekTable(vec![]),
            Block::VorbisComment(vec![]),
            Block::CueSheet(vec![]),
            Block::Picture(Picture {
                picture_type: 0,
                media_type: vec![],
                description: vec![],
                width: 0,
                height: 0,
                depth: 0,
                colors: 0,
                data: vec![],
            }),
            Block::Raw {
                code: 9,
                body: vec![],
            },
        ]
        .iter()
        .map(Block::code)
        .collect();
        assert_eq!(codes, [0, 1, 2, 3, 4, 5, 6, 9]);
    }

    #[test]
    fn writes_any_header_the_tests_ask_for() {
        assert_eq!(header(false, 0, 33), [0x00, 0x00, 0x00, 0x21]);
        assert_eq!(header(true, 127, 0xFF_FFFF), [0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(header(true, 4, 0x01_0203), [0x84, 0x01, 0x02, 0x03]);
    }

    #[test]
    #[should_panic(expected = "128 does not fit in 7 bits")]
    fn refuses_a_type_code_over_127() {
        let _ = header(false, 128, 0);
    }

    #[test]
    fn flags_only_the_final_block_as_the_last() {
        assert_eq!(stream(&[]), b"fLaC");
        assert_eq!(
            stream(&[Block::Padding(0), Block::Padding(1), Block::Padding(0)]),
            [
                b'f', b'L', b'a', b'C', //
                0x01, 0x00, 0x00, 0x00, //
                0x01, 0x00, 0x00, 0x01, 0x00, //
                0x81, 0x00, 0x00, 0x00,
            ]
        );
    }
}
