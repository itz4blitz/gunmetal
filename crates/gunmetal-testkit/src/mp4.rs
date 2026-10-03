//! Builders for MP4 audio files: boxes, the audio sample entries with their
//! codec configuration, and iTunes-style item lists.
//!
//! Written from ISO/IEC 14496-12 (boxes and sample entries), ISO/IEC
//! 14496-1 and 14496-14 (the `esds` descriptors), ISO/IEC 14496-3 (the
//! `AudioSpecificConfig`), Apple's ALAC magic cookie and `QuickTime` metadata
//! documents, and the FLAC-in-ISOBMFF and Opus-in-ISOBMFF mappings. It
//! shares no code with the MP4 parser in `gunmetal-core`.
//!
//! Each function returns one whole box, so a file reads as a tree of calls:
//!
//! ```
//! use gunmetal_testkit::mp4::{ftyp, mp4_box};
//!
//! let file = [ftyp(*b"M4A ", 0, &[]), mp4_box(*b"moov", &[])].concat();
//! assert_eq!(file.len(), 24);
//! ```

use crate::bytes::{Bits, Bytes};

/// A box with a 32-bit size.
///
/// # Panics
///
/// Panics when the box would not fit a 32-bit size.
#[must_use]
pub fn mp4_box(kind: [u8; 4], body: &[u8]) -> Vec<u8> {
    let mut written = Bytes::new();
    written.mp4_box(kind, body);
    written.into_vec()
}

/// A full box: a box whose body starts with a version and 24-bit flags.
///
/// # Panics
///
/// Panics when `flags` does not fit 24 bits or the box would not fit a
/// 32-bit size.
#[must_use]
pub fn full_box(kind: [u8; 4], version: u8, flags: u32, body: &[u8]) -> Vec<u8> {
    let mut written = Bytes::new();
    written.u8(version).u24_be(flags).bytes(body);
    mp4_box(kind, written.as_slice())
}

/// A box whose 32-bit size is 1, with its real size in the 64 bits after
/// the type.
///
/// # Panics
///
/// Panics when the box would not fit a 64-bit size.
#[must_use]
pub fn large_box(kind: [u8; 4], body: &[u8]) -> Vec<u8> {
    let size = u64::try_from(body.len() + 16).expect("a box fits a 64-bit size");
    let mut written = Bytes::new();
    written.u32_be(1).bytes(&kind).u64_be(size).bytes(body);
    written.into_vec()
}

/// A box whose size is 0, which runs to the end of its parent or file.
#[must_use]
pub fn open_box(kind: [u8; 4], body: &[u8]) -> Vec<u8> {
    let mut written = Bytes::new();
    written.u32_be(0).bytes(&kind).bytes(body);
    written.into_vec()
}

/// A file type box.
#[must_use]
pub fn ftyp(major: [u8; 4], minor: u32, compatible: &[[u8; 4]]) -> Vec<u8> {
    let mut written = Bytes::new();
    written
        .bytes(&major)
        .u32_be(minor)
        .bytes(&compatible.concat());
    mp4_box(*b"ftyp", written.as_slice())
}

/// The ISO 639-2 code "und", packed as three five-bit letters less 0x60.
const UNDETERMINED: u16 = 0x55C4;

/// A version 0 media header box.
#[must_use]
pub fn mdhd(timescale: u32, duration: u32) -> Vec<u8> {
    let mut written = Bytes::new();
    written
        .u32_be(0) // creation time
        .u32_be(0) // modification time
        .u32_be(timescale)
        .u32_be(duration)
        .u16_be(UNDETERMINED)
        .u16_be(0); // pre_defined
    full_box(*b"mdhd", 0, 0, written.as_slice())
}

/// A version 1 media header box, with 64-bit times and duration.
#[must_use]
pub fn mdhd_v1(timescale: u32, duration: u64) -> Vec<u8> {
    let mut written = Bytes::new();
    written
        .u64_be(0) // creation time
        .u64_be(0) // modification time
        .u32_be(timescale)
        .u64_be(duration)
        .u16_be(UNDETERMINED)
        .u16_be(0); // pre_defined
    full_box(*b"mdhd", 1, 0, written.as_slice())
}

/// A handler reference box.
#[must_use]
pub fn hdlr(handler: [u8; 4]) -> Vec<u8> {
    let mut written = Bytes::new();
    written
        .u32_be(0) // pre_defined
        .bytes(&handler)
        .zeros(12) // reserved
        .u8(0); // an empty name
    full_box(*b"hdlr", 0, 0, written.as_slice())
}

/// The fields of an audio sample entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleEntry<'a> {
    /// The coding name: `mp4a`, `alac`, `fLaC`, `Opus` and so on.
    pub format: [u8; 4],
    /// The `QuickTime` sound description version: 0, or 1 for 16 more
    /// octets of fields.
    pub version: u16,
    /// The channel count.
    pub channels: u16,
    /// The sample size in bits.
    pub bits: u16,
    /// The integer part of the 16.16 sample rate.
    pub rate: u16,
    /// The boxes after the fields, such as the codec configuration.
    pub children: &'a [u8],
}

/// An audio sample entry. Version 1's extra fields are written as zeros.
#[must_use]
pub fn sample_entry(entry: &SampleEntry<'_>) -> Vec<u8> {
    let mut written = Bytes::new();
    written
        .zeros(6) // reserved
        .u16_be(1) // data reference index
        .u16_be(entry.version)
        .u16_be(0) // revision level
        .u32_be(0) // vendor
        .u16_be(entry.channels)
        .u16_be(entry.bits)
        .u16_be(0) // compression ID
        .u16_be(0) // packet size
        .u16_be(entry.rate)
        .u16_be(0); // the fraction of the rate
    if entry.version == 1 {
        written.zeros(16);
    }
    written.bytes(entry.children);
    mp4_box(entry.format, written.as_slice())
}

/// A sample description box holding `entries`.
///
/// # Panics
///
/// Panics when there are more than `u32::MAX` entries.
#[must_use]
pub fn stsd(entries: &[&[u8]]) -> Vec<u8> {
    let count = u32::try_from(entries.len()).expect("the entry count fits 32 bits");
    let mut written = Bytes::new();
    written.u32_be(count).bytes(&entries.concat());
    full_box(*b"stsd", 0, 0, written.as_slice())
}

/// An MPEG-4 descriptor whose length is written in `width` octets.
///
/// # Panics
///
/// Panics when the length of `body` does not fit `width` octets of seven
/// bits each.
#[must_use]
pub fn descriptor(tag: u8, width: usize, body: &[u8]) -> Vec<u8> {
    let len = body.len();
    assert!(
        len >> (7 * width) == 0,
        "{len} octets do not fit a {width}-octet length"
    );
    let mut written = Bytes::new();
    written.u8(tag);
    for index in (0..width).rev() {
        let more = if index == 0 { 0 } else { 0x80 };
        let group = u8::try_from((len >> (7 * index)) & 0x7F).expect("seven bits fit an octet");
        written.u8(more | group);
    }
    written.bytes(body);
    written.into_vec()
}

/// The fields of an elementary stream descriptor box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Esds<'a> {
    /// The object type indication: 0x40 for MPEG-4 audio, 0x6B for MPEG-1
    /// audio and so on.
    pub object_type: u8,
    /// The largest bit rate.
    pub max_bitrate: u32,
    /// The average bit rate.
    pub avg_bitrate: u32,
    /// The decoder specific information, such as an `AudioSpecificConfig`.
    pub specific: Option<&'a [u8]>,
    /// How many octets every descriptor length takes.
    pub width: usize,
}

/// The stream type of audio, 5, shifted past the upstream flag, with the
/// reserved bit set.
const AUDIO_STREAM: u8 = 0x15;

/// An elementary stream descriptor box: an `ES_Descriptor` with `ES_ID` 1
/// and no flags, holding a `DecoderConfigDescriptor` with the
/// `DecoderSpecificInfo`, if any, and an `SLConfigDescriptor` of the
/// predefined MP4 kind.
#[must_use]
pub fn esds(esds: &Esds<'_>) -> Vec<u8> {
    let mut config = Bytes::new();
    config
        .u8(esds.object_type)
        .u8(AUDIO_STREAM)
        .u24_be(0) // buffer size
        .u32_be(esds.max_bitrate)
        .u32_be(esds.avg_bitrate);
    if let Some(specific) = esds.specific {
        config.bytes(&descriptor(0x05, esds.width, specific));
    }
    let mut es = Bytes::new();
    es.u16_be(1) // ES_ID
        .u8(0) // flags
        .bytes(&descriptor(0x04, esds.width, config.as_slice()))
        .bytes(&descriptor(0x06, esds.width, &[0x02]));
    full_box(*b"esds", 0, 0, &descriptor(0x03, esds.width, es.as_slice()))
}

/// An `AudioSpecificConfig` of the form AAC uses: an object type below 31, a
/// sampling frequency index below 15, a channel configuration and a
/// `GASpecificConfig` of three zero bits.
///
/// # Panics
///
/// Panics when a field does not fit its width.
#[must_use]
pub fn audio_specific_config(object_type: u8, rate_index: u8, channels: u8) -> Vec<u8> {
    let mut fields = Bits::new();
    fields
        .put(5, u64::from(object_type))
        .put(4, u64::from(rate_index))
        .put(4, u64::from(channels))
        .put(3, 0); // frame length, core coder and extension flags
    fields.into_vec()
}

/// The fields of an ALAC magic cookie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Alac {
    /// Samples per frame.
    pub frame_length: u32,
    /// Bits per sample.
    pub bits: u8,
    /// Channels.
    pub channels: u8,
    /// The largest frame, in octets.
    pub max_frame_bytes: u32,
    /// The average bit rate.
    pub avg_bitrate: u32,
    /// The sample rate in hertz.
    pub sample_rate: u32,
}

/// An `alac` box holding an ALAC magic cookie, with Apple's tuning values.
#[must_use]
pub fn alac(cookie: &Alac) -> Vec<u8> {
    let mut written = Bytes::new();
    written
        .u32_be(cookie.frame_length)
        .u8(0) // compatible version
        .u8(cookie.bits)
        .u8(40) // pb
        .u8(10) // mb
        .u8(14) // kb
        .u8(cookie.channels)
        .u16_be(255) // max run
        .u32_be(cookie.max_frame_bytes)
        .u32_be(cookie.avg_bitrate)
        .u32_be(cookie.sample_rate);
    full_box(*b"alac", 0, 0, written.as_slice())
}

/// The fields of a FLAC STREAMINFO block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamInfo {
    /// The smallest block size, in samples.
    pub min_block: u16,
    /// The largest block size, in samples.
    pub max_block: u16,
    /// The sample rate in hertz, at most 20 bits.
    pub sample_rate: u32,
    /// Channels, 1 to 8.
    pub channels: u8,
    /// Bits per sample, 1 to 32.
    pub bits: u8,
    /// Samples in the stream, at most 36 bits; 0 when unknown.
    pub total_samples: u64,
}

/// A `dfLa` box holding one STREAMINFO block, marked last, with unknown
/// frame sizes and an MD5 of zeros.
///
/// # Panics
///
/// Panics when a field does not fit its width.
#[must_use]
pub fn dfla(info: &StreamInfo) -> Vec<u8> {
    let mut fields = Bits::new();
    fields
        .put(1, 1) // the last block
        .put(7, 0) // STREAMINFO
        .put(24, 34) // its length
        .put(16, u64::from(info.min_block))
        .put(16, u64::from(info.max_block))
        .put(24, 0) // smallest frame
        .put(24, 0) // largest frame
        .put(20, u64::from(info.sample_rate))
        .put(3, u64::from(info.channels - 1))
        .put(5, u64::from(info.bits - 1))
        .put(36, info.total_samples)
        .put(64, 0) // MD5, not set
        .put(64, 0);
    full_box(*b"dfLa", 0, 0, &fields.into_vec())
}

/// The fields of an Opus specific box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Opus<'a> {
    /// Output channels.
    pub channels: u8,
    /// Samples to drop from the start, at 48 kHz.
    pub pre_skip: u16,
    /// The sample rate of the original input.
    pub input_rate: u32,
    /// The output gain, in Q7.8 decibels.
    pub gain: i16,
    /// The channel mapping family.
    pub family: u8,
    /// The stream count, coupled count and channel mapping, when the
    /// family is not 0.
    pub mapping: &'a [u8],
}

/// A `dOps` box, version 0.
#[must_use]
pub fn dops(opus: &Opus<'_>) -> Vec<u8> {
    let mut written = Bytes::new();
    written
        .u8(0) // version
        .u8(opus.channels)
        .u16_be(opus.pre_skip)
        .u32_be(opus.input_rate)
        .bytes(&opus.gain.to_be_bytes())
        .u8(opus.family)
        .bytes(opus.mapping);
    mp4_box(*b"dOps", written.as_slice())
}

/// A `data` box of an item list: a well-known type, a locale of 0 and the
/// value.
#[must_use]
pub fn data(type_code: u32, value: &[u8]) -> Vec<u8> {
    let mut written = Bytes::new();
    written.u32_be(type_code).u32_be(0).bytes(value);
    mp4_box(*b"data", written.as_slice())
}

/// A freeform `----` item with an optional `mean`, an optional `name` and
/// its `data` boxes.
#[must_use]
pub fn freeform(mean: Option<&str>, name: Option<&str>, values: &[Vec<u8>]) -> Vec<u8> {
    let mut written = Bytes::new();
    for (kind, text) in [(*b"mean", mean), (*b"name", name)] {
        if let Some(text) = text {
            written.bytes(&full_box(kind, 0, 0, text.as_bytes()));
        }
    }
    written.bytes(&values.concat());
    mp4_box(*b"----", written.as_slice())
}

/// A user data box holding a `meta` box with an `mdir` handler and an
/// `ilst` of `items`. The `meta` box is a full box, as ISO/IEC 14496-12
/// defines it, unless `plain`, as `QuickTime` writes it.
#[must_use]
pub fn udta(plain: bool, items: &[u8]) -> Vec<u8> {
    let children = [hdlr(*b"mdir"), mp4_box(*b"ilst", items)].concat();
    let meta = if plain {
        mp4_box(*b"meta", &children)
    } else {
        full_box(*b"meta", 0, 0, &children)
    };
    mp4_box(*b"udta", &meta)
}

/// A track: `trak` holding `mdia`, which holds `mdhd`, an `hdlr` naming
/// `handler`, and `minf` holding `stbl` with `stbl_children`.
#[must_use]
pub fn trak(handler: [u8; 4], mdhd: &[u8], stbl_children: &[u8]) -> Vec<u8> {
    let stbl = mp4_box(*b"stbl", stbl_children);
    let mdia = [mdhd.to_vec(), hdlr(handler), mp4_box(*b"minf", &stbl)].concat();
    mp4_box(*b"trak", &mp4_box(*b"mdia", &mdia))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `parts` joined, so a literal box reads one field per line.
    fn cat(parts: &[&[u8]]) -> Vec<u8> {
        parts.concat()
    }

    #[test]
    fn writes_a_box_whose_size_counts_its_header() {
        assert_eq!(mp4_box(*b"free", &[]), b"\x00\x00\x00\x08free");
        assert_eq!(
            mp4_box(*b"skip", &[0xAA, 0xBB]),
            b"\x00\x00\x00\x0Askip\xAA\xBB"
        );
    }

    #[test]
    fn writes_a_full_box_with_its_version_and_flags() {
        // ISO/IEC 14496-12 section 4.2: version(8) then flags(24).
        assert_eq!(
            full_box(*b"stco", 0, 0, &[0, 0, 0, 0]),
            b"\x00\x00\x00\x10stco\x00\x00\x00\x00\x00\x00\x00\x00"
        );
        assert_eq!(
            full_box(*b"tfhd", 1, 0x02_0304, b"x"),
            b"\x00\x00\x00\x0Dtfhd\x01\x02\x03\x04x"
        );
    }

    #[test]
    fn writes_a_large_box_with_a_64_bit_size() {
        // Size 1 says the real size follows the type as 64 bits.
        assert_eq!(
            large_box(*b"mdat", &[0xAA]),
            b"\x00\x00\x00\x01mdat\x00\x00\x00\x00\x00\x00\x00\x11\xAA"
        );
    }

    #[test]
    fn writes_an_open_box_with_size_zero() {
        assert_eq!(
            open_box(*b"mdat", &[0xAA, 0xBB]),
            b"\x00\x00\x00\x00mdat\xAA\xBB"
        );
    }

    #[test]
    fn writes_the_file_type_box_itunes_writes() {
        assert_eq!(
            ftyp(*b"M4A ", 0x200, &[*b"M4A ", *b"isom"]),
            b"\x00\x00\x00\x18ftypM4A \x00\x00\x02\x00M4A isom"
        );
        assert_eq!(
            ftyp(*b"isom", 0, &[]),
            b"\x00\x00\x00\x10ftypisom\x00\x00\x00\x00"
        );
    }

    #[test]
    fn writes_media_headers_of_both_versions() {
        assert_eq!(
            mdhd(44_100, 88_200),
            cat(&[
                b"\x00\x00\x00\x20mdhd\x00\x00\x00\x00",
                &[0, 0, 0, 0],             // creation time
                &[0, 0, 0, 0],             // modification time
                &[0x00, 0x00, 0xAC, 0x44], // timescale
                &[0x00, 0x01, 0x58, 0x88], // duration
                &[0x55, 0xC4],             // language "und", five bits a letter
                &[0, 0],                   // pre_defined
            ])
        );
        assert_eq!(
            mdhd_v1(48_000, 0x0002_0000_0001),
            cat(&[
                b"\x00\x00\x00\x2Cmdhd\x01\x00\x00\x00",
                &[0; 8],                                           // creation time
                &[0; 8],                                           // modification time
                &[0x00, 0x00, 0xBB, 0x80],                         // timescale
                &[0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01], // duration
                &[0x55, 0xC4, 0, 0],
            ])
        );
    }

    #[test]
    fn writes_a_handler_with_an_empty_name() {
        assert_eq!(
            hdlr(*b"soun"),
            cat(&[
                b"\x00\x00\x00\x21hdlr\x00\x00\x00\x00",
                &[0; 4], // pre_defined
                b"soun",
                &[0; 12], // reserved
                &[0],     // an empty name, NUL-terminated
            ])
        );
    }

    #[test]
    fn writes_audio_sample_entries_of_versions_0_and_1() {
        let fields = SampleEntry {
            format: *b"mp4a",
            version: 0,
            channels: 2,
            bits: 16,
            rate: 44_100,
            children: b"kids",
        };
        assert_eq!(
            sample_entry(&fields),
            cat(&[
                b"\x00\x00\x00\x28mp4a",
                &[0; 6],             // reserved
                &[0, 1],             // data reference index
                &[0, 0],             // version
                &[0, 0],             // revision level
                &[0; 4],             // vendor
                &[0, 2],             // channels
                &[0, 16],            // sample size
                &[0, 0],             // compression ID
                &[0, 0],             // packet size
                &[0xAC, 0x44, 0, 0], // 44,100.0 as 16.16
                b"kids",
            ])
        );
        let v1 = SampleEntry {
            version: 1,
            channels: 1,
            bits: 24,
            rate: 8_000,
            children: &[],
            ..fields
        };
        assert_eq!(
            sample_entry(&v1),
            cat(&[
                b"\x00\x00\x00\x34mp4a",
                &[0, 0, 0, 0, 0, 0, 0, 1],
                &[0, 1, 0, 0, 0, 0, 0, 0],
                &[0, 1, 0, 24, 0, 0, 0, 0, 0x1F, 0x40, 0, 0],
                &[0; 16], // samples per packet, bytes per packet, frame and sample
            ])
        );
    }

    #[test]
    fn writes_a_sample_description_with_its_entry_count() {
        assert_eq!(
            stsd(&[b"one", b"two"]),
            b"\x00\x00\x00\x16stsd\x00\x00\x00\x00\x00\x00\x00\x02onetwo"
        );
    }

    #[test]
    fn writes_descriptor_lengths_in_one_to_four_octets() {
        // ISO/IEC 14496-1 section 8.3.3: seven bits an octet, the top bit
        // set on every octet but the last.
        assert_eq!(descriptor(0x05, 1, &[0x12, 0x10]), [0x05, 0x02, 0x12, 0x10]);
        assert_eq!(descriptor(0x05, 2, &[0xAA]), [0x05, 0x80, 0x01, 0xAA]);
        assert_eq!(descriptor(0x06, 3, &[0x02]), [0x06, 0x80, 0x80, 0x01, 0x02]);
        assert_eq!(descriptor(0x03, 4, &[]), [0x03, 0x80, 0x80, 0x80, 0x00]);
        let long = vec![0x55; 200];
        let written = descriptor(0x04, 2, &long);
        assert_eq!(&written[..3], &[0x04, 0x81, 0x48]);
        assert_eq!(&written[3..], &long[..]);
    }

    #[test]
    #[should_panic(expected = "200 octets do not fit a 1-octet length")]
    fn refuses_a_length_too_long_for_its_width() {
        let _ = descriptor(0x04, 1, &[0; 200]);
    }

    #[test]
    fn writes_the_esds_ffmpeg_writes_for_aac() {
        // The descriptors `ffmpeg -c:a aac` writes for 44.1 kHz stereo,
        // with every length in four octets.
        let esds = esds(&Esds {
            object_type: 0x40,
            max_bitrate: 128_000,
            avg_bitrate: 128_000,
            specific: Some(&[0x12, 0x10]),
            width: 4,
        });
        assert_eq!(
            esds,
            cat(&[
                b"\x00\x00\x00\x33esds\x00\x00\x00\x00",
                &[0x03, 0x80, 0x80, 0x80, 0x22], // ES_Descriptor
                &[0x00, 0x01, 0x00],             // ES_ID 1, no flags
                &[0x04, 0x80, 0x80, 0x80, 0x14], // DecoderConfigDescriptor
                &[0x40, 0x15],                   // MPEG-4 audio; audio stream
                &[0x00, 0x00, 0x00],             // buffer size
                &[0x00, 0x01, 0xF4, 0x00],       // max bitrate
                &[0x00, 0x01, 0xF4, 0x00],       // average bitrate
                &[0x05, 0x80, 0x80, 0x80, 0x02, 0x12, 0x10], // DecoderSpecificInfo
                &[0x06, 0x80, 0x80, 0x80, 0x01, 0x02], // SLConfigDescriptor
            ])
        );
    }

    #[test]
    fn writes_an_esds_without_specific_info_in_one_octet_lengths() {
        // MP3 in MP4: MPEG-1 audio, which carries no specific info.
        assert_eq!(
            esds(&Esds {
                object_type: 0x6B,
                max_bitrate: 1,
                avg_bitrate: 2,
                specific: None,
                width: 1,
            }),
            cat(&[
                b"\x00\x00\x00\x23esds\x00\x00\x00\x00",
                &[0x03, 0x15, 0x00, 0x01, 0x00],
                &[0x04, 0x0D, 0x6B, 0x15, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 2],
                &[0x06, 0x01, 0x02],
            ])
        );
    }

    #[test]
    fn writes_the_audio_specific_config_of_aac_lc() {
        // AAC LC (2), 44.1 kHz (index 4), stereo (2): 00010 0100 0010 000.
        assert_eq!(audio_specific_config(2, 4, 2), [0x12, 0x10]);
        // AAC Main (1), 96 kHz (index 0), 7.1 (7): 00001 0000 0111 000.
        assert_eq!(audio_specific_config(1, 0, 7), [0x08, 0x38]);
        // 30, 14 and 15 fill every field.
        assert_eq!(audio_specific_config(30, 14, 15), [0xF7, 0x78]);
    }

    #[test]
    fn writes_an_alac_magic_cookie() {
        // Apple's ALACMagicCookieDescription.txt: the cookie follows a
        // full box header, with Apple's tuning values pb 40, mb 10, kb 14
        // and maxRun 255.
        assert_eq!(
            alac(&Alac {
                frame_length: 4_096,
                bits: 16,
                channels: 2,
                max_frame_bytes: 0x0102_0304,
                avg_bitrate: 0x0506_0708,
                sample_rate: 44_100,
            }),
            cat(&[
                b"\x00\x00\x00\x24alac\x00\x00\x00\x00",
                &[0x00, 0x00, 0x10, 0x00], // frameLength
                &[0x00, 0x10],             // compatibleVersion, bitDepth
                &[0x28, 0x0A, 0x0E],       // pb, mb, kb
                &[0x02],                   // numChannels
                &[0x00, 0xFF],             // maxRun
                &[0x01, 0x02, 0x03, 0x04], // maxFrameBytes
                &[0x05, 0x06, 0x07, 0x08], // avgBitRate
                &[0x00, 0x00, 0xAC, 0x44], // sampleRate
            ])
        );
    }

    #[test]
    fn writes_a_flac_specific_box_with_one_streaminfo_block() {
        assert_eq!(
            dfla(&StreamInfo {
                min_block: 4_096,
                max_block: 4_608,
                sample_rate: 44_100,
                channels: 2,
                bits: 16,
                total_samples: 0x0F_0102_0304,
            }),
            cat(&[
                b"\x00\x00\x00\x32dfLa\x00\x00\x00\x00",
                &[0x80, 0x00, 0x00, 0x22], // last block, STREAMINFO, 34 octets
                &[0x10, 0x00, 0x12, 0x00], // block sizes
                &[0; 6],                   // frame sizes, unknown
                // 44,100 in 20 bits, 1 (two channels) in 3, 15 (16 bits) in
                // 5, then the total in 36.
                &[0x0A, 0xC4, 0x42, 0xFF, 0x01, 0x02, 0x03, 0x04],
                &[0; 16], // MD5, not set
            ])
        );
    }

    #[test]
    fn writes_an_opus_specific_box_with_and_without_a_mapping() {
        // Opus in ISOBMFF section 4.3.2: every field big-endian.
        assert_eq!(
            dops(&Opus {
                channels: 2,
                pre_skip: 312,
                input_rate: 48_000,
                gain: -256,
                family: 0,
                mapping: &[],
            }),
            cat(&[
                b"\x00\x00\x00\x13dOps",
                &[0x00, 0x02, 0x01, 0x38],
                &[0x00, 0x00, 0xBB, 0x80],
                &[0xFF, 0x00, 0x00],
            ])
        );
        assert_eq!(
            dops(&Opus {
                channels: 3,
                pre_skip: 0,
                input_rate: 0,
                gain: 0,
                family: 1,
                mapping: &[2, 1, 0, 2, 1],
            }),
            b"\x00\x00\x00\x18dOps\x00\x03\x00\x00\x00\x00\x00\x00\x00\x00\x01\x02\x01\x00\x02\x01"
        );
    }

    #[test]
    fn writes_a_data_box_with_its_type_and_a_zero_locale() {
        assert_eq!(
            data(1, b"Title"),
            b"\x00\x00\x00\x15data\x00\x00\x00\x01\x00\x00\x00\x00Title"
        );
        assert_eq!(
            data(0x0001_0203, &[]),
            b"\x00\x00\x00\x10data\x00\x01\x02\x03\x00\x00\x00\x00"
        );
    }

    #[test]
    fn writes_freeform_items_with_and_without_mean_and_name() {
        let value = data(1, b"x");
        assert_eq!(
            freeform(
                Some("com.apple.iTunes"),
                Some("iTunSMPB"),
                std::slice::from_ref(&value)
            ),
            cat(&[
                b"\x00\x00\x00\x49----",
                b"\x00\x00\x00\x1Cmean\x00\x00\x00\x00com.apple.iTunes",
                b"\x00\x00\x00\x14name\x00\x00\x00\x00iTunSMPB",
                &value,
            ])
        );
        assert_eq!(
            freeform(None, Some("n"), &[]),
            b"\x00\x00\x00\x15----\x00\x00\x00\x0Dname\x00\x00\x00\x00n"
        );
        assert_eq!(
            freeform(Some("m"), None, &[value.clone(), value.clone()]),
            cat(&[
                b"\x00\x00\x00\x37----",
                b"\x00\x00\x00\x0Dmean\x00\x00\x00\x00m",
                &value,
                &value,
            ])
        );
    }

    #[test]
    fn writes_user_data_with_a_full_or_a_plain_meta_box() {
        let hdlr_mdir = hdlr(*b"mdir");
        assert_eq!(
            udta(false, b"items"),
            cat(&[
                b"\x00\x00\x00\x42udta",
                b"\x00\x00\x00\x3Ameta\x00\x00\x00\x00",
                &hdlr_mdir,
                b"\x00\x00\x00\x0Dilstitems",
            ])
        );
        assert_eq!(
            udta(true, &[]),
            cat(&[
                b"\x00\x00\x00\x39udta",
                b"\x00\x00\x00\x31meta",
                &hdlr_mdir,
                b"\x00\x00\x00\x08ilst",
            ])
        );
    }

    #[test]
    fn writes_a_track_down_to_its_sample_table() {
        let header = mdhd(1_000, 5);
        let handler = hdlr(*b"soun");
        assert_eq!(
            trak(*b"soun", &header, b"tables"),
            cat(&[
                b"\x00\x00\x00\x67trak",
                b"\x00\x00\x00\x5Fmdia",
                &header,
                &handler,
                b"\x00\x00\x00\x16minf",
                b"\x00\x00\x00\x0Estbltables",
            ])
        );
    }
}
