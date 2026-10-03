//! MPEG audio streams: frame headers, whole frames, and the encoder headers
//! that LAME, Xing and Fraunhofer encoders write in a stream's first frame.
//!
//! Written from ISO/IEC 11172-3 and ISO/IEC 13818-3 (section 2.4 of each),
//! Fraunhofer's MPEG 2.5 extension, the Xing header layout, LAME's
//! `VbrTag.c` and the VBRI header layout, for the tests alone: nothing here
//! is shared with the parser in `gunmetal-core`. The tests below check the
//! writers against frames that LAME 4.0 wrote.

use crate::bytes::{Bits, Bytes};
use crate::checksum::crc16_flac;

/// The MPEG audio version a frame header declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    /// MPEG-1 (ISO/IEC 11172-3): 32, 44.1 and 48 kHz.
    Mpeg1,
    /// MPEG-2 (ISO/IEC 13818-3): 16, 22.05 and 24 kHz.
    Mpeg2,
    /// MPEG 2.5, Fraunhofer's extension: 8, 11.025 and 12 kHz.
    Mpeg25,
}

/// The layer a frame header declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    /// Layer I: 384 samples a frame.
    I,
    /// Layer II: 1,152 samples a frame.
    II,
    /// Layer III, the layer of MP3 files.
    III,
}

/// The channel mode a frame header declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Two independent channels.
    Stereo,
    /// Two channels coded together.
    JointStereo,
    /// Two unrelated mono channels.
    DualChannel,
    /// One channel.
    Mono,
}

/// The fields of one frame header, as ISO/IEC 11172-3 section 2.4.1.3 lays
/// them out after the twelve sync bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag is one bit of the header, named as the standard names it"
)]
pub struct Frame {
    /// The version.
    pub version: Version,
    /// The layer.
    pub layer: Layer,
    /// Whether a CRC follows the header, which the header says by clearing
    /// its protection bit.
    pub crc: bool,
    /// The bitrate index, 1 to 14 for a frame whose length is known.
    pub bitrate_index: u8,
    /// The sampling-frequency index, 0 to 2.
    pub rate_index: u8,
    /// Whether the frame carries one padding slot.
    pub padding: bool,
    /// The private bit.
    pub private: bool,
    /// The channel mode.
    pub mode: Mode,
    /// The two mode-extension bits.
    pub mode_extension: u8,
    /// The copyright bit.
    pub copyright: bool,
    /// The original bit.
    pub original: bool,
    /// The two emphasis bits.
    pub emphasis: u8,
}

impl Frame {
    /// A Layer III frame header with every other field clear: no CRC, no
    /// padding, mode extension 0, and neither private, copyrighted nor
    /// original.
    #[must_use]
    pub fn layer3(version: Version, bitrate_index: u8, rate_index: u8, mode: Mode) -> Self {
        Self {
            version,
            layer: Layer::III,
            crc: false,
            bitrate_index,
            rate_index,
            padding: false,
            private: false,
            mode,
            mode_extension: 0,
            copyright: false,
            original: false,
            emphasis: 0,
        }
    }

    /// The four header octets.
    ///
    /// # Panics
    ///
    /// Panics when the bitrate index, the sampling-frequency index, the mode
    /// extension or the emphasis does not fit its field.
    #[must_use]
    pub fn header(&self) -> [u8; 4] {
        let version = match self.version {
            Version::Mpeg1 => 0b11,
            Version::Mpeg2 => 0b10,
            Version::Mpeg25 => 0b00,
        };
        let layer = match self.layer {
            Layer::I => 0b11,
            Layer::II => 0b10,
            Layer::III => 0b01,
        };
        let mode = match self.mode {
            Mode::Stereo => 0b00,
            Mode::JointStereo => 0b01,
            Mode::DualChannel => 0b10,
            Mode::Mono => 0b11,
        };
        let mut bits = Bits::new();
        bits.put(11, 0x7FF)
            .put(2, version)
            .put(2, layer)
            .put(1, u64::from(!self.crc))
            .put(4, u64::from(self.bitrate_index))
            .put(2, u64::from(self.rate_index))
            .put(1, u64::from(self.padding))
            .put(1, u64::from(self.private))
            .put(2, mode)
            .put(2, u64::from(self.mode_extension))
            .put(1, u64::from(self.copyright))
            .put(1, u64::from(self.original))
            .put(2, u64::from(self.emphasis));
        let mut header = [0; 4];
        header.copy_from_slice(&bits.into_vec());
        header
    }

    /// The frame's length in octets, header included (ISO/IEC 11172-3 and
    /// ISO/IEC 13818-3, section 2.4.3.1 of each).
    ///
    /// # Panics
    ///
    /// Panics for free format (bitrate index 0), the forbidden bitrate index
    /// 15 and the reserved sampling-frequency index 3.
    #[must_use]
    pub fn size(&self) -> usize {
        let kbps = self.kbps();
        let hz = self.hz();
        let padding = usize::from(self.padding);
        no_larger_than_any_frame(match self.layer {
            // Layer I counts in four-octet slots of 32 bits.
            Layer::I => (kbps * 12_000 / hz + padding) * 4,
            // Layers II and III count in octets: samples / 8 bits a sample
            // at the bitrate, which is samples * kbit/s * 125 / Hz.
            Layer::II | Layer::III => {
                usize::try_from(self.samples()).expect("a frame's samples fit usize") * kbps * 125
                    / hz
                    + padding
            }
        })
    }

    /// The bitrate in kbit/s (ISO/IEC 11172-3 table 2.4.2.3 and ISO/IEC
    /// 13818-3 table 2.4.2.3).
    fn kbps(&self) -> usize {
        let table: [usize; 14] = match (self.version, self.layer) {
            (Version::Mpeg1, Layer::I) => [
                32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448,
            ],
            (Version::Mpeg1, Layer::II) => [
                32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384,
            ],
            (Version::Mpeg1, Layer::III) => [
                32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320,
            ],
            (Version::Mpeg2 | Version::Mpeg25, Layer::I) => [
                32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256,
            ],
            (Version::Mpeg2 | Version::Mpeg25, Layer::II | Layer::III) => {
                [8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160]
            }
        };
        let index = self.bitrate_index;
        assert!(
            (1..=14).contains(&index),
            "bitrate index {index} has no bitrate"
        );
        table[usize::from(index) - 1]
    }

    /// The sampling frequency in hertz.
    fn hz(&self) -> usize {
        let table = match self.version {
            Version::Mpeg1 => [44_100, 48_000, 32_000],
            Version::Mpeg2 => [22_050, 24_000, 16_000],
            Version::Mpeg25 => [11_025, 12_000, 8_000],
        };
        let index = self.rate_index;
        assert!(index < 3, "sampling-frequency index {index} is reserved");
        table[usize::from(index)]
    }

    /// The samples one frame holds per channel.
    #[must_use]
    pub fn samples(&self) -> u64 {
        match (self.layer, self.version) {
            (Layer::I, _) => 384,
            (Layer::II, _) | (Layer::III, Version::Mpeg1) => 1_152,
            (Layer::III, Version::Mpeg2 | Version::Mpeg25) => 576,
        }
    }

    /// The octets of Layer III side information. A Xing or Info header
    /// starts this many octets after the four header octets, whether or not
    /// the frame has a CRC: LAME writes it there, over the CRC's place, and
    /// decoders look for it there.
    #[must_use]
    pub fn side_info_len(&self) -> usize {
        match (self.version, self.mode) {
            (Version::Mpeg1, Mode::Mono) => 17,
            (Version::Mpeg1, _) => 32,
            (Version::Mpeg2 | Version::Mpeg25, Mode::Mono) => 9,
            (Version::Mpeg2 | Version::Mpeg25, _) => 17,
        }
    }

    /// The whole frame: the header, then `body`, then zeros up to
    /// [`Frame::size`].
    ///
    /// # Panics
    ///
    /// Panics when the header and `body` do not fit the frame, and for the
    /// headers [`Frame::size`] refuses.
    #[must_use]
    pub fn write(&self, body: &[u8]) -> Vec<u8> {
        let size = self.size();
        let used = 4 + body.len();
        assert!(
            used <= size,
            "a {}-octet body does not fit a {size}-octet frame",
            body.len()
        );
        let mut frame = Bytes::new();
        frame.bytes(&self.header()).bytes(body).zeros(size - used);
        frame.into_vec()
    }
}

/// The largest MPEG audio frame there is: MPEG 2.5 Layer II at 160 kbit/s
/// and 8 kHz, padded.
const LARGEST_FRAME: usize = 2_881;

/// `size`, checked against [`LARGEST_FRAME`], so that a mistake in the size
/// arithmetic fails at once instead of filling memory with a frame of
/// zeros.
///
/// # Panics
///
/// Panics when `size` is larger than any frame can be.
fn no_larger_than_any_frame(size: usize) -> usize {
    assert!(
        size <= LARGEST_FRAME,
        "no MPEG audio frame is {size} octets long"
    );
    size
}

/// A stream of `frames`, each written with an empty body.
#[must_use]
pub fn stream(frames: &[Frame]) -> Vec<u8> {
    frames.iter().flat_map(|frame| frame.write(&[])).collect()
}

/// A Xing header, or an Info header, which LAME writes in its place for a
/// constant bitrate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Xing {
    /// Write `Info` rather than `Xing`.
    pub info: bool,
    /// The frame count, if the header holds one.
    pub frames: Option<u32>,
    /// The byte count, if the header holds one.
    pub bytes: Option<u32>,
    /// The table of contents, if the header holds one.
    pub toc: Option<[u8; 100]>,
    /// The quality indicator, if the header holds one.
    pub quality: Option<u32>,
}

impl Xing {
    /// The header: its name, a flag for each field present, then the
    /// fields present, in that order.
    #[must_use]
    pub fn write(&self) -> Vec<u8> {
        let mut flags = Bits::new();
        flags
            .put(28, 0)
            .put(1, u64::from(self.quality.is_some()))
            .put(1, u64::from(self.toc.is_some()))
            .put(1, u64::from(self.bytes.is_some()))
            .put(1, u64::from(self.frames.is_some()));
        let mut header = Bytes::new();
        header
            .bytes(if self.info { b"Info" } else { b"Xing" })
            .bytes(&flags.into_vec());
        if let Some(frames) = self.frames {
            header.u32_be(frames);
        }
        if let Some(bytes) = self.bytes {
            header.u32_be(bytes);
        }
        if let Some(toc) = self.toc {
            header.bytes(&toc);
        }
        if let Some(quality) = self.quality {
            header.u32_be(quality);
        }
        header.into_vec()
    }
}

/// The extension LAME writes after a Xing or Info header, as `VbrTag.c`
/// lays it out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lame {
    /// The encoder's short version string, such as `LAME3.100`.
    pub encoder: [u8; 9],
    /// The tag revision in the high four bits, the VBR method in the low.
    pub revision_method: u8,
    /// The lowpass filter frequency in hundreds of hertz.
    pub lowpass: u8,
    /// The peak sample as a fraction of full scale, times 2^23.
    pub peak: u32,
    /// The radio (track) `ReplayGain` field; see [`replay_gain`].
    pub track_gain: u16,
    /// The audiophile (album) `ReplayGain` field; see [`replay_gain`].
    pub album_gain: u16,
    /// The encoding flags in the high four bits, the ATH type in the low.
    pub flags_ath: u8,
    /// The ABR bitrate, or the minimum bitrate.
    pub bitrate: u8,
    /// Samples the encoder added at the start, below 4,096.
    pub delay: u16,
    /// Samples the encoder added at the end, below 4,096.
    pub padding: u16,
    /// Noise shaping, stereo mode, unwise settings and source frequency.
    pub misc: u8,
    /// The MP3 gain, in steps of 1.5 dB.
    pub mp3_gain: u8,
    /// Surround and preset.
    pub preset: u16,
    /// The octets of the stream from the first frame on.
    pub music_length: u32,
    /// The CRC of the audio after the first frame.
    pub music_crc: u16,
}

impl Lame {
    /// The 34 octets of the extension that come before its CRC.
    ///
    /// # Panics
    ///
    /// Panics when the delay or the padding does not fit twelve bits.
    #[must_use]
    pub fn write(&self) -> Vec<u8> {
        let mut delay_and_padding = Bits::new();
        delay_and_padding
            .put(12, u64::from(self.delay))
            .put(12, u64::from(self.padding));
        let mut extension = Bytes::new();
        extension
            .bytes(&self.encoder)
            .u8(self.revision_method)
            .u8(self.lowpass)
            .u32_be(self.peak)
            .u16_be(self.track_gain)
            .u16_be(self.album_gain)
            .u8(self.flags_ath)
            .u8(self.bitrate)
            .bytes(&delay_and_padding.into_vec())
            .u8(self.misc)
            .u8(self.mp3_gain)
            .u16_be(self.preset)
            .u32_be(self.music_length)
            .u16_be(self.music_crc);
        extension.into_vec()
    }
}

/// A `ReplayGain` field of the LAME extension: a three-bit name code (1 for
/// radio, 2 for audiophile), a three-bit originator code, a sign bit and the
/// gain's magnitude in tenths of a decibel.
///
/// # Panics
///
/// Panics when a code does not fit three bits or the magnitude nine.
#[must_use]
pub fn replay_gain(name: u8, originator: u8, tenths: i16) -> u16 {
    let mut field = Bits::new();
    field
        .put(3, u64::from(name))
        .put(3, u64::from(originator))
        .put(1, u64::from(tenths < 0))
        .put(9, u64::from(tenths.unsigned_abs()));
    let mut octets = [0; 2];
    octets.copy_from_slice(&field.into_vec());
    u16::from_be_bytes(octets)
}

/// CRC-16/ARC, which LAME computes over its first frame up to the
/// extension's CRC field: polynomial 0x8005 on reflected input and output,
/// initial value 0, no final XOR. It is FLAC's CRC-16 run over each octet's
/// bits in reverse, with the result reversed.
#[must_use]
pub fn crc16_arc(bytes: &[u8]) -> u16 {
    let reflected: Vec<u8> = bytes.iter().map(|octet| octet.reverse_bits()).collect();
    crc16_flac(&reflected).reverse_bits()
}

/// The first frame of a stream whose encoder wrote `xing` where the side
/// information ends and, when given, `lame` after it with the CRC that
/// covers every octet of the frame before the CRC.
///
/// # Panics
///
/// Panics when the headers do not fit the frame.
#[must_use]
pub fn xing_frame(frame: &Frame, xing: &Xing, lame: Option<&Lame>) -> Vec<u8> {
    let mut written = Bytes::new();
    written
        .bytes(&frame.header())
        .zeros(frame.side_info_len())
        .bytes(&xing.write());
    if let Some(lame) = lame {
        written.bytes(&lame.write());
        let crc = crc16_arc(written.as_slice());
        written.u16_be(crc);
    }
    frame.write(&written.as_slice()[4..])
}

/// A VBRI header, as Fraunhofer's encoders write it 32 octets after the
/// first frame's header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vbri {
    /// The header's version.
    pub version: u16,
    /// The delay field.
    pub delay: u16,
    /// The quality indicator.
    pub quality: u16,
    /// The byte count.
    pub bytes: u32,
    /// The frame count.
    pub frames: u32,
    /// The factor every table entry is multiplied by.
    pub scale: u16,
    /// The frames each table entry covers.
    pub frames_per_entry: u16,
    /// The table of contents, written two octets an entry.
    pub table: Vec<u16>,
}

/// The first frame of a stream whose encoder wrote `vbri` in it.
///
/// # Panics
///
/// Panics when the header does not fit the frame.
#[must_use]
pub fn vbri_frame(frame: &Frame, vbri: &Vbri) -> Vec<u8> {
    let entries = u16::try_from(vbri.table.len()).expect("the table fits a 16-bit count");
    let mut body = Bytes::new();
    body.zeros(32)
        .bytes(b"VBRI")
        .u16_be(vbri.version)
        .u16_be(vbri.delay)
        .u16_be(vbri.quality)
        .u32_be(vbri.bytes)
        .u32_be(vbri.frames)
        .u16_be(entries)
        .u16_be(vbri.scale)
        .u16_be(2)
        .u16_be(vbri.frames_per_entry);
    for &entry in &vbri.table {
        body.u16_be(entry);
    }
    frame.write(body.as_slice())
}

/// An `ID3v2.4` tag holding `body` as it is: the ten-octet header with a
/// syncsafe size, `body`, and when `footer` is set the footer, a copy of
/// the header that starts `3DI` (`ID3v2.4.0` structure, sections 3.1 and
/// 3.4). An MP3 file often starts with one.
///
/// # Panics
///
/// Panics when `body` does not fit a 28-bit size.
#[must_use]
pub fn id3v2_tag(body: &[u8], footer: bool) -> Vec<u8> {
    let size = u32::try_from(body.len()).expect("the body fits a 32-bit length");
    let flags = if footer { 0x10 } else { 0x00 };
    let mut tag = Bytes::new();
    tag.bytes(b"ID3")
        .u8(4)
        .u8(0)
        .u8(flags)
        .syncsafe32(size)
        .bytes(body);
    if footer {
        tag.bytes(b"3DI").u8(4).u8(0).u8(flags).syncsafe32(size);
    }
    tag.into_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// LAME's default stream: MPEG-1 Layer III at 128 kbit/s and 44.1 kHz,
    /// joint stereo with mid/side coding, marked original.
    fn lame_default() -> Frame {
        Frame {
            mode_extension: 0b10,
            original: true,
            ..Frame::layer3(Version::Mpeg1, 9, 0, Mode::JointStereo)
        }
    }

    #[test]
    fn writes_the_headers_lame_wrote() {
        // `lame -b 128`, `lame -m m --resample 22.05 -b 64` and
        // `lame -m m --resample 8 -V 9`.
        assert_eq!(lame_default().header(), [0xFF, 0xFB, 0x90, 0x64]);
        let mono22 = Frame {
            original: true,
            ..Frame::layer3(Version::Mpeg2, 8, 0, Mode::Mono)
        };
        assert_eq!(mono22.header(), [0xFF, 0xF3, 0x80, 0xC4]);
        let mono8 = Frame {
            original: true,
            ..Frame::layer3(Version::Mpeg25, 4, 2, Mode::Mono)
        };
        assert_eq!(mono8.header(), [0xFF, 0xE3, 0x48, 0xC4]);
    }

    #[test]
    fn writes_every_field_where_the_standard_puts_it() {
        let every_flag = Frame {
            version: Version::Mpeg1,
            layer: Layer::I,
            crc: true,
            bitrate_index: 0b1110,
            rate_index: 0b10,
            padding: true,
            private: true,
            mode: Mode::DualChannel,
            mode_extension: 0b11,
            copyright: true,
            original: true,
            emphasis: 0b11,
        };
        // Sync, MPEG-1, Layer I, protected; 1110, 10, padded, private;
        // dual channel, 11, copyright, original, 11.
        assert_eq!(every_flag.header(), [0xFF, 0xFE, 0xEB, 0xBF]);
        let layer2 = Frame {
            layer: Layer::II,
            ..Frame::layer3(Version::Mpeg2, 1, 1, Mode::Stereo)
        };
        assert_eq!(layer2.header(), [0xFF, 0xF5, 0x14, 0x00]);
    }

    #[test]
    #[should_panic(expected = "16 does not fit in 4 bits")]
    fn refuses_a_bitrate_index_wider_than_its_field() {
        let _ = Frame::layer3(Version::Mpeg1, 16, 0, Mode::Mono).header();
    }

    /// Frame sizes at the sampling frequency where each table's bitrates
    /// give whole numbers of octets, so every size is the bitrate times a
    /// small factor, for bitrate indices 1 to 14.
    #[test]
    fn sizes_frames_at_every_bitrate_of_every_table() {
        let tables: [(Version, Layer, u8, [usize; 14]); 8] = [
            // MPEG-1 at 48 kHz: Layer I frames hold kbit/s octets, Layers
            // II and III three times as many.
            (
                Version::Mpeg1,
                Layer::I,
                1,
                [
                    32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448,
                ],
            ),
            (
                Version::Mpeg1,
                Layer::II,
                1,
                [
                    96, 144, 168, 192, 240, 288, 336, 384, 480, 576, 672, 768, 960, 1152,
                ],
            ),
            (
                Version::Mpeg1,
                Layer::III,
                1,
                [
                    96, 120, 144, 168, 192, 240, 288, 336, 384, 480, 576, 672, 768, 960,
                ],
            ),
            // MPEG-2 at 24 kHz: twice, six times and three times kbit/s.
            (
                Version::Mpeg2,
                Layer::I,
                1,
                [
                    64, 96, 112, 128, 160, 192, 224, 256, 288, 320, 352, 384, 448, 512,
                ],
            ),
            (
                Version::Mpeg2,
                Layer::II,
                1,
                [
                    48, 96, 144, 192, 240, 288, 336, 384, 480, 576, 672, 768, 864, 960,
                ],
            ),
            (
                Version::Mpeg2,
                Layer::III,
                1,
                [
                    24, 48, 72, 96, 120, 144, 168, 192, 240, 288, 336, 384, 432, 480,
                ],
            ),
            // MPEG 2.5 at 12 kHz: four and six times kbit/s.
            (
                Version::Mpeg25,
                Layer::I,
                1,
                [
                    128, 192, 224, 256, 320, 384, 448, 512, 576, 640, 704, 768, 896, 1024,
                ],
            ),
            (
                Version::Mpeg25,
                Layer::III,
                1,
                [
                    48, 96, 144, 192, 240, 288, 336, 384, 480, 576, 672, 768, 864, 960,
                ],
            ),
        ];
        for (version, layer, rate_index, sizes) in tables {
            for (index, size) in (1..=14).zip(sizes) {
                let frame = Frame {
                    layer,
                    ..Frame::layer3(version, index, rate_index, Mode::Stereo)
                };
                assert_eq!(frame.size(), size, "{version:?} {layer:?} index {index}");
            }
        }
    }

    /// A frame's size unpadded and padded.
    type Sizes = (usize, usize);

    #[test]
    fn sizes_frames_at_every_sampling_frequency() {
        // (version, layer, bitrate index, then the size at each frequency
        // index, unpadded and padded)
        let cases: [(Version, Layer, u8, [Sizes; 3]); 6] = [
            // 128 kbit/s at 44.1, 48 and 32 kHz.
            (
                Version::Mpeg1,
                Layer::III,
                9,
                [(417, 418), (384, 385), (576, 577)],
            ),
            // 384 kbit/s.
            (
                Version::Mpeg1,
                Layer::II,
                14,
                [(1253, 1254), (1152, 1153), (1728, 1729)],
            ),
            // 448 kbit/s: a padding slot is four octets.
            (
                Version::Mpeg1,
                Layer::I,
                14,
                [(484, 488), (448, 452), (672, 676)],
            ),
            // 64 kbit/s at 22.05, 24 and 16 kHz.
            (
                Version::Mpeg2,
                Layer::III,
                8,
                [(208, 209), (192, 193), (288, 289)],
            ),
            // 32 kbit/s at 11.025, 12 and 8 kHz.
            (
                Version::Mpeg25,
                Layer::III,
                4,
                [(208, 209), (192, 193), (288, 289)],
            ),
            // 160 kbit/s, the largest Layer II frame there is at 8 kHz.
            (
                Version::Mpeg25,
                Layer::II,
                14,
                [(2089, 2090), (1920, 1921), (2880, 2881)],
            ),
        ];
        for (version, layer, bitrate_index, sizes) in cases {
            for (rate_index, (plain, padded)) in (0..3).zip(sizes) {
                let frame = Frame {
                    layer,
                    ..Frame::layer3(version, bitrate_index, rate_index, Mode::Mono)
                };
                assert_eq!(
                    (
                        frame.size(),
                        Frame {
                            padding: true,
                            ..frame
                        }
                        .size()
                    ),
                    (plain, padded),
                    "{version:?} {layer:?} rate index {rate_index}"
                );
            }
        }
    }

    #[test]
    fn allows_frames_up_to_the_largest_the_standards_define() {
        assert_eq!(no_larger_than_any_frame(2_881), 2_881);
    }

    #[test]
    #[should_panic(expected = "no MPEG audio frame is 2882 octets long")]
    fn refuses_a_frame_size_beyond_the_largest_the_standards_define() {
        let _ = no_larger_than_any_frame(2_882);
    }

    #[test]
    #[should_panic(expected = "bitrate index 0 has no bitrate")]
    fn refuses_to_size_a_free_format_frame() {
        let _ = Frame::layer3(Version::Mpeg1, 0, 0, Mode::Stereo).size();
    }

    #[test]
    #[should_panic(expected = "bitrate index 15 has no bitrate")]
    fn refuses_to_size_a_frame_with_the_forbidden_bitrate() {
        let _ = Frame::layer3(Version::Mpeg1, 15, 0, Mode::Stereo).size();
    }

    #[test]
    #[should_panic(expected = "sampling-frequency index 3 is reserved")]
    fn refuses_to_size_a_frame_with_the_reserved_frequency() {
        let _ = Frame::layer3(Version::Mpeg1, 9, 3, Mode::Stereo).size();
    }

    #[test]
    fn counts_the_samples_of_each_layer_and_version() {
        let samples: Vec<u64> = [
            (Version::Mpeg1, Layer::I),
            (Version::Mpeg1, Layer::II),
            (Version::Mpeg1, Layer::III),
            (Version::Mpeg2, Layer::I),
            (Version::Mpeg2, Layer::II),
            (Version::Mpeg2, Layer::III),
            (Version::Mpeg25, Layer::I),
            (Version::Mpeg25, Layer::II),
            (Version::Mpeg25, Layer::III),
        ]
        .into_iter()
        .map(|(version, layer)| {
            Frame {
                layer,
                ..Frame::layer3(version, 1, 0, Mode::Stereo)
            }
            .samples()
        })
        .collect();
        assert_eq!(samples, [384, 1152, 1152, 384, 1152, 576, 384, 1152, 576]);
    }

    #[test]
    fn measures_the_side_information_of_each_version_and_mode() {
        let lengths: Vec<usize> = [Version::Mpeg1, Version::Mpeg2, Version::Mpeg25]
            .into_iter()
            .flat_map(|version| {
                [
                    Mode::Stereo,
                    Mode::JointStereo,
                    Mode::DualChannel,
                    Mode::Mono,
                ]
                .map(|mode| Frame::layer3(version, 1, 0, mode).side_info_len())
            })
            .collect();
        assert_eq!(lengths, [32, 32, 32, 17, 17, 17, 17, 9, 17, 17, 17, 9]);
    }

    #[test]
    fn writes_a_frame_as_its_header_body_and_zeros() {
        let frame = Frame::layer3(Version::Mpeg2, 1, 1, Mode::Mono);
        let mut expected = vec![0xFF, 0xF3, 0x14, 0xC0, 0xAA, 0xBB];
        expected.extend([0; 18]);
        assert_eq!(frame.write(&[0xAA, 0xBB]), expected);
        // A body that fills the frame exactly.
        let full = frame.write(&[0x11; 20]);
        assert_eq!(full.len(), 24);
        assert_eq!(full[4..], [0x11; 20]);
    }

    #[test]
    #[should_panic(expected = "a 21-octet body does not fit a 24-octet frame")]
    fn refuses_a_body_longer_than_its_frame() {
        let _ = Frame::layer3(Version::Mpeg2, 1, 1, Mode::Mono).write(&[0; 21]);
    }

    #[test]
    fn writes_a_stream_frame_after_frame() {
        let small = Frame::layer3(Version::Mpeg2, 1, 1, Mode::Mono);
        let padded = Frame {
            padding: true,
            ..small
        };
        let mut expected = vec![0xFF, 0xF3, 0x14, 0xC0];
        expected.extend([0; 20]);
        expected.extend([0xFF, 0xF3, 0x16, 0xC0]);
        expected.extend([0; 21]);
        assert_eq!(stream(&[small, padded]), expected);
        assert_eq!(stream(&[]), Vec::<u8>::new());
    }

    /// The table of contents LAME 4.0 wrote for nine frames.
    const LAME_TOC: [u8; 100] = [
        0x00, 0x1C, 0x1C, 0x1C, 0x1C, 0x1C, 0x1C, 0x1C, 0x1C, 0x1C, 0x1C, 0x1C, 0x38, 0x38, 0x38,
        0x38, 0x38, 0x38, 0x38, 0x38, 0x38, 0x38, 0x38, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55,
        0x55, 0x55, 0x55, 0x55, 0x71, 0x71, 0x71, 0x71, 0x71, 0x71, 0x71, 0x71, 0x71, 0x71, 0x71,
        0x8E, 0x8E, 0x8E, 0x8E, 0x8E, 0x8E, 0x8E, 0x8E, 0x8E, 0x8E, 0x8E, 0xAA, 0xAA, 0xAA, 0xAA,
        0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xC7, 0xC7, 0xC7, 0xC7, 0xC7, 0xC7, 0xC7, 0xC7,
        0xC7, 0xC7, 0xC7, 0xE3, 0xE3, 0xE3, 0xE3, 0xE3, 0xE3, 0xE3, 0xE3, 0xE3, 0xE3, 0xE3, 0xFF,
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    ];

    #[test]
    fn writes_each_xing_field_its_flags_announce() {
        let mut every = Vec::new();
        every.extend(b"Info\x00\x00\x00\x0F\x00\x00\x00\x09\x00\x00\x10\x52");
        every.extend(LAME_TOC);
        every.extend([0x00, 0x00, 0x00, 0x38]);
        assert_eq!(
            Xing {
                info: true,
                frames: Some(9),
                bytes: Some(4178),
                toc: Some(LAME_TOC),
                quality: Some(56),
            }
            .write(),
            every
        );
        let none = Xing {
            info: false,
            frames: None,
            bytes: None,
            toc: None,
            quality: None,
        };
        assert_eq!(none.write(), b"Xing\x00\x00\x00\x00");
        let alone = |xing: Xing| xing.write();
        assert_eq!(
            alone(Xing {
                frames: Some(0x0102_0304),
                ..none.clone()
            }),
            b"Xing\x00\x00\x00\x01\x01\x02\x03\x04"
        );
        assert_eq!(
            alone(Xing {
                bytes: Some(0x0506_0708),
                ..none.clone()
            }),
            b"Xing\x00\x00\x00\x02\x05\x06\x07\x08"
        );
        let mut toc_alone = b"Xing\x00\x00\x00\x04".to_vec();
        toc_alone.extend(LAME_TOC);
        assert_eq!(
            alone(Xing {
                toc: Some(LAME_TOC),
                ..none.clone()
            }),
            toc_alone
        );
        assert_eq!(
            alone(Xing {
                quality: Some(0x090A_0B0C),
                ..none
            }),
            b"Xing\x00\x00\x00\x08\x09\x0A\x0B\x0C"
        );
    }

    #[test]
    fn writes_replay_gain_fields_as_lame_does() {
        // LAME 4.0's radio gain for a 440 Hz tone: +6.7 dB, determined
        // automatically.
        assert_eq!(replay_gain(1, 3, 67), 0x2C43);
        // An album gain of -6.5 dB set by a user.
        assert_eq!(replay_gain(2, 2, -65), 0x4A41);
        assert_eq!(replay_gain(7, 7, -511), 0xFFFF);
        assert_eq!(replay_gain(0, 0, 0), 0);
    }

    #[test]
    #[should_panic(expected = "512 does not fit in 9 bits")]
    fn refuses_a_gain_of_51_point_2_db() {
        let _ = replay_gain(1, 3, -512);
    }

    /// The extension LAME 4.0 wrote in the Info frame for `lame -b 128`.
    fn lame_tag() -> Lame {
        Lame {
            encoder: *b"LAME4.0 \x00",
            revision_method: 0x01,
            lowpass: 0xAA,
            peak: 0,
            track_gain: 0x2C43,
            album_gain: 0,
            flags_ath: 0x14,
            bitrate: 0x80,
            delay: 576,
            padding: 972,
            misc: 0x4E,
            mp3_gain: 0,
            preset: 0x0080,
            music_length: 4178,
            music_crc: 0xDEC8,
        }
    }

    #[test]
    fn writes_the_lame_extension_field_by_field() {
        let mut expected = b"LAME4.0 \x00".to_vec();
        expected.extend([0x01, 0xAA, 0x00, 0x00, 0x00, 0x00, 0x2C, 0x43, 0x00, 0x00]);
        expected.extend([0x14, 0x80, 0x24, 0x03, 0xCC, 0x4E, 0x00, 0x00, 0x80]);
        expected.extend([0x00, 0x00, 0x10, 0x52, 0xDE, 0xC8]);
        assert_eq!(lame_tag().write(), expected);
        let every_field = Lame {
            encoder: *b"Lavf61.7\x00",
            revision_method: 0x12,
            lowpass: 0x34,
            peak: 0x0080_0000,
            track_gain: 0x5678,
            album_gain: 0x9ABC,
            flags_ath: 0xDE,
            bitrate: 0xF0,
            delay: 0x0FFF,
            padding: 0x0ABC,
            misc: 0x11,
            mp3_gain: 0x22,
            preset: 0x3344,
            music_length: 0x5566_7788,
            music_crc: 0x99AA,
        };
        let mut expected = b"Lavf61.7\x00".to_vec();
        expected.extend([0x12, 0x34, 0x00, 0x80, 0x00, 0x00, 0x56, 0x78, 0x9A, 0xBC]);
        expected.extend([0xDE, 0xF0, 0xFF, 0xFA, 0xBC, 0x11, 0x22, 0x33, 0x44]);
        expected.extend([0x55, 0x66, 0x77, 0x88, 0x99, 0xAA]);
        assert_eq!(every_field.write(), expected);
    }

    #[test]
    #[should_panic(expected = "4096 does not fit in 12 bits")]
    fn refuses_a_delay_of_4096_samples() {
        let _ = Lame {
            delay: 4096,
            ..lame_tag()
        }
        .write();
    }

    #[test]
    fn computes_crc16_arc_as_the_crc_catalogue_and_lame_do() {
        // The catalogue's check value for CRC-16/ARC.
        assert_eq!(crc16_arc(b"123456789"), 0xBB3D);
        assert_eq!(crc16_arc(&[]), 0);
        // The CRC LAME 4.0 wrote over the first 190 octets of the Info
        // frame below.
        assert_eq!(crc16_arc(&lame_info_frame()[..190]), 0x47CE);
    }

    /// The Info frame LAME 4.0 wrote for 0.2 s of a 440 Hz tone with `lame
    /// -b 128`, octet for octet.
    fn lame_info_frame() -> Vec<u8> {
        let mut frame = vec![0xFF, 0xFB, 0x90, 0x64];
        frame.extend([0; 32]);
        frame.extend(b"Info\x00\x00\x00\x0F\x00\x00\x00\x09\x00\x00\x10\x52");
        frame.extend(LAME_TOC);
        frame.extend([0x00, 0x00, 0x00, 0x38]);
        frame.extend(b"LAME4.0 \x00");
        frame.extend([0x01, 0xAA, 0x00, 0x00, 0x00, 0x00, 0x2C, 0x43, 0x00, 0x00]);
        frame.extend([0x14, 0x80, 0x24, 0x03, 0xCC, 0x4E, 0x00, 0x00, 0x80]);
        frame.extend([0x00, 0x00, 0x10, 0x52, 0xDE, 0xC8, 0x47, 0xCE]);
        frame.extend([0; 225]);
        frame
    }

    #[test]
    fn writes_the_info_frame_lame_wrote() {
        let xing = Xing {
            info: true,
            frames: Some(9),
            bytes: Some(4178),
            toc: Some(LAME_TOC),
            quality: Some(56),
        };
        let written = xing_frame(&lame_default(), &xing, Some(&lame_tag()));
        assert_eq!(written.len(), 417);
        assert_eq!(written, lame_info_frame());
    }

    #[test]
    fn writes_a_xing_frame_without_the_lame_extension() {
        // A mono MPEG-2 frame puts the header nine octets after its own.
        let frame = Frame::layer3(Version::Mpeg2, 8, 1, Mode::Mono);
        let xing = Xing {
            info: false,
            frames: Some(3),
            bytes: None,
            toc: None,
            quality: None,
        };
        let mut expected = vec![0xFF, 0xF3, 0x84, 0xC0];
        expected.extend([0; 9]);
        expected.extend(b"Xing\x00\x00\x00\x01\x00\x00\x00\x03");
        expected.extend([0; 192 - 25]);
        assert_eq!(xing_frame(&frame, &xing, None), expected);
    }

    #[test]
    fn writes_a_vbri_frame_32_octets_after_the_header() {
        let vbri = Vbri {
            version: 1,
            delay: 0x0123,
            quality: 75,
            bytes: 0x0001_0203,
            frames: 0x0405_0607,
            scale: 1,
            frames_per_entry: 2,
            table: vec![0x0A0B, 0x0C0D],
        };
        let mut expected = vec![0xFF, 0xFB, 0x90, 0x64];
        expected.extend([0; 32]);
        expected.extend(b"VBRI\x00\x01\x01\x23\x00\x4B\x00\x01\x02\x03\x04\x05\x06\x07");
        expected.extend([0x00, 0x02, 0x00, 0x01, 0x00, 0x02, 0x00, 0x02]);
        expected.extend([0x0A, 0x0B, 0x0C, 0x0D]);
        expected.extend([0; 417 - 66]);
        assert_eq!(vbri_frame(&lame_default(), &vbri), expected);
    }

    #[test]
    fn writes_id3v2_tags_with_and_without_a_footer() {
        assert_eq!(
            id3v2_tag(&[0xAA, 0xBB], false),
            b"ID3\x04\x00\x00\x00\x00\x00\x02\xAA\xBB"
        );
        assert_eq!(
            id3v2_tag(&[0xCC; 200], true),
            [
                &b"ID3\x04\x00\x10\x00\x00\x01\x48"[..],
                &[0xCC; 200],
                b"3DI\x04\x00\x10\x00\x00\x01\x48",
            ]
            .concat()
        );
    }
}
