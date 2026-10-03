//! Builders for Vorbis comment blocks, the tag format that FLAC, Ogg Vorbis
//! and Opus share, and for the picture a `METADATA_BLOCK_PICTURE` comment
//! holds.
//!
//! A comment block is laid out as the Vorbis I comment specification
//! (<https://xiph.org/vorbis/doc/v-comment.html>) says: a 32-bit
//! little-endian length and the vendor string, a 32-bit little-endian count,
//! then each comment as a 32-bit little-endian length and its `NAME=value`
//! octets. Ogg Vorbis adds a framing bit after the last comment and Opus may
//! add padding; neither is part of the block written here.
//!
//! A picture is the body of a FLAC `PICTURE` metadata block (RFC 9639,
//! section 8.8), every integer 32-bit big-endian, and goes into a comment
//! written in base64 (RFC 4648, section 4, with padding). The encoder here
//! shares nothing with the core's codec, so it can check it.

use crate::bytes::Bytes;

/// The standard base64 alphabet, RFC 4648 section 4.
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// A comment block, written comment by comment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommentBlock {
    /// The vendor string's octets.
    vendor: Vec<u8>,
    /// Every comment's octets, in order.
    comments: Vec<Vec<u8>>,
    /// A comment count to write instead of the real one.
    declared: Option<u32>,
}

impl CommentBlock {
    /// A block with the vendor string `vendor` and no comments.
    #[must_use]
    pub fn new(vendor: &[u8]) -> Self {
        Self {
            vendor: vendor.to_vec(),
            ..Self::default()
        }
    }

    /// Appends the comment `key=value`.
    pub fn field(&mut self, key: &str, value: &str) -> &mut Self {
        self.raw(format!("{key}={value}").as_bytes())
    }

    /// Appends a comment of exactly `octets`, which need not hold a `=` or
    /// be UTF-8.
    pub fn raw(&mut self, octets: &[u8]) -> &mut Self {
        self.comments.push(octets.to_vec());
        self
    }

    /// Writes `count` as the number of comments, whatever the block holds.
    pub fn declare(&mut self, count: u32) -> &mut Self {
        self.declared = Some(count);
        self
    }

    /// The block's octets.
    ///
    /// # Panics
    ///
    /// Panics when the vendor string, a comment or the number of comments
    /// does not fit a 32-bit length.
    #[must_use]
    pub fn build(&self) -> Vec<u8> {
        let count = self.declared.unwrap_or_else(|| length(self.comments.len()));
        let mut block = Bytes::new();
        block
            .u32_le(length(self.vendor.len()))
            .bytes(&self.vendor)
            .u32_le(count);
        for comment in &self.comments {
            block.u32_le(length(comment.len())).bytes(comment);
        }
        block.into_vec()
    }
}

/// The picture a `METADATA_BLOCK_PICTURE` comment holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Picture {
    /// The picture type; 3 is the front cover.
    pub kind: u32,
    /// The media type, such as `image/png`.
    pub mime: String,
    /// The description.
    pub description: String,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Bits per pixel.
    pub depth: u32,
    /// Colours in the palette of an indexed picture, otherwise zero.
    pub colours: u32,
    /// The image data.
    pub data: Vec<u8>,
}

impl Picture {
    /// The picture's octets, laid out as a FLAC `PICTURE` block body.
    ///
    /// # Panics
    ///
    /// Panics when the media type, the description or the data does not
    /// fit a 32-bit length.
    #[must_use]
    pub fn build(&self) -> Vec<u8> {
        let mut body = Bytes::new();
        body.u32_be(self.kind)
            .u32_be(length(self.mime.len()))
            .bytes(self.mime.as_bytes())
            .u32_be(length(self.description.len()))
            .bytes(self.description.as_bytes())
            .u32_be(self.width)
            .u32_be(self.height)
            .u32_be(self.depth)
            .u32_be(self.colours)
            .u32_be(length(self.data.len()))
            .bytes(&self.data);
        body.into_vec()
    }
}

/// `octets` in base64, standard alphabet, with padding (RFC 4648,
/// section 4).
#[must_use]
pub fn base64(octets: &[u8]) -> String {
    let mut text = String::new();
    for group in octets.chunks(3) {
        // The group's octets, most significant first, padded with zeros to
        // a 24-bit number.
        let mut packed = [0_u8; 3];
        for (slot, &octet) in packed.iter_mut().zip(group) {
            *slot = octet;
        }
        let [first, second, third] = packed;
        let bits = u32::from_be_bytes([0, first, second, third]);
        // A group of n octets is written as n + 1 characters, then padding.
        for (index, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            if index <= group.len() {
                let sextet = usize::try_from(bits >> shift & 0x3F).unwrap_or_default();
                let symbol = ALPHABET.get(sextet).copied().unwrap_or_default();
                text.push(char::from(symbol));
            } else {
                text.push('=');
            }
        }
    }
    text
}

/// `len` as a 32-bit length field.
fn length(len: usize) -> u32 {
    u32::try_from(len).expect("a length fits a 32-bit length")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_the_vendor_the_count_and_each_comment_with_its_length() {
        let mut block = CommentBlock::new(b"v");
        block.field("A", "1").field("Bc", "22");
        assert_eq!(
            block.build(),
            [
                1, 0, 0, 0, b'v', // vendor
                2, 0, 0, 0, // two comments
                3, 0, 0, 0, b'A', b'=', b'1', // A=1
                5, 0, 0, 0, b'B', b'c', b'=', b'2', b'2', // Bc=22
            ]
        );
    }

    #[test]
    fn writes_an_empty_block() {
        assert_eq!(CommentBlock::new(b"").build(), [0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn writes_lengths_of_more_than_one_octet_least_significant_first() {
        let vendor = [b'x'; 300];
        let mut expected = vec![0x2C, 0x01, 0, 0];
        expected.extend(vendor);
        expected.extend([0, 0, 0, 0]);
        assert_eq!(CommentBlock::new(&vendor).build(), expected);
    }

    #[test]
    fn writes_raw_comments_as_they_are() {
        let mut block = CommentBlock::new(b"\xFF");
        block.raw(b"").raw(b"no separator\xC3");
        assert_eq!(
            block.build(),
            [
                1, 0, 0, 0, 0xFF, // vendor
                2, 0, 0, 0, // two comments
                0, 0, 0, 0, // an empty comment
                13, 0, 0, 0, b'n', b'o', b' ', b's', b'e', b'p', b'a', b'r', b'a', b't', b'o',
                b'r', 0xC3,
            ]
        );
    }

    #[test]
    fn writes_a_declared_count_instead_of_the_real_one() {
        let mut block = CommentBlock::new(b"");
        block.field("K", "").declare(u32::MAX);
        assert_eq!(
            block.build(),
            [0, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 2, 0, 0, 0, b'K', b'=']
        );
    }

    #[test]
    fn writes_a_picture_as_a_flac_picture_block_body() {
        let picture = Picture {
            kind: 3,
            mime: "image/png".to_owned(),
            description: "é".to_owned(),
            width: 0x0102_0304,
            height: 2,
            depth: 24,
            colours: 0x10,
            data: vec![0x89, 0x50],
        };
        assert_eq!(
            picture.build(),
            [
                0, 0, 0, 3, // front cover
                0, 0, 0, 9, b'i', b'm', b'a', b'g', b'e', b'/', b'p', b'n',
                b'g', // media type
                0, 0, 0, 2, 0xC3, 0xA9, // description
                1, 2, 3, 4, // width
                0, 0, 0, 2, // height
                0, 0, 0, 24, // depth
                0, 0, 0, 0x10, // colours
                0, 0, 0, 2, 0x89, 0x50, // data
            ]
        );
    }

    #[test]
    fn writes_an_empty_picture_as_eight_zero_lengths_and_numbers() {
        assert_eq!(Picture::default().build(), [0; 32]);
    }

    /// RFC 4648, section 10, and octets whose encodings use `+`, `/` and
    /// the ends of the alphabet.
    #[test]
    fn encodes_base64_with_the_standard_alphabet_and_padding() {
        let cases: [(&[u8], &str); 10] = [
            (b"", ""),
            (b"f", "Zg=="),
            (b"fo", "Zm8="),
            (b"foo", "Zm9v"),
            (b"foob", "Zm9vYg=="),
            (b"fooba", "Zm9vYmE="),
            (b"foobar", "Zm9vYmFy"),
            (&[0xFB, 0xFF], "+/8="),
            (&[0xFF, 0xFF, 0xFF], "////"),
            (&[0x00, 0x10, 0x83], "ABCD"),
        ];
        for (octets, text) in cases {
            assert_eq!(base64(octets), text, "{octets:02X?}");
        }
    }

    #[test]
    fn encodes_every_character_of_the_alphabet_in_order() {
        // Forty-eight octets counting up six bits at a time.
        let octets = [
            0x00, 0x10, 0x83, 0x10, 0x51, 0x87, 0x20, 0x92, 0x8B, 0x30, 0xD3, 0x8F, 0x41, 0x14,
            0x93, 0x51, 0x55, 0x97, 0x61, 0x96, 0x9B, 0x71, 0xD7, 0x9F, 0x82, 0x18, 0xA3, 0x92,
            0x59, 0xA7, 0xA2, 0x9A, 0xAB, 0xB2, 0xDB, 0xAF, 0xC3, 0x1C, 0xB3, 0xD3, 0x5D, 0xB7,
            0xE3, 0x9E, 0xBB, 0xF3, 0xDF, 0xBF,
        ];
        assert_eq!(
            base64(&octets),
            "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
        );
    }

    #[test]
    fn keeps_a_length_that_fits_32_bits() {
        assert_eq!(length(0), 0);
        assert_eq!(length(0xFFFF_FFFF), u32::MAX);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    #[should_panic(expected = "fits a 32-bit length")]
    fn refuses_a_length_that_does_not_fit_32_bits() {
        let _ = length(0x1_0000_0000);
    }
}
