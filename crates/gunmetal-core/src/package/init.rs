//! The initialisation segment: what a player needs before the first media
//! segment (ISO/IEC 14496-12, with the FLAC mapping for the `fLaC` sample
//! entry and its `dfLa` box).
//!
//! It is a file type box and a movie box. The movie holds one audio track
//! and none of its samples: the sample tables are empty and a movie extends
//! box says the samples come in movie fragments. The movie and the media
//! count time in samples, so every time in a segment is exact.
//!
//! # Trim
//!
//! The track's one edit says which part of the decoded audio is the music:
//! it starts `delay` samples into the media and lasts for the samples that
//! are left once the padding is cut from the end. That is the edit list
//! form of the encoder delay and padding, and with it a player joins two
//! tracks without a gap (MUS-067, MUS-069).
//!
//! # Data references
//!
//! The one data reference is a `url ` entry flagged as self-contained, with
//! no location: the media data is in the segments themselves, and nothing
//! the packager writes points anywhere else (architecture record 4,
//! decision 6).

use std::num::NonZeroU64;

use crate::formats::flac::metadata::StreamInfo;

use super::boxes::{Body, TRACK_ID};
use super::index::FrameIndex;
use super::track::{PackTrack, frame_size};

/// The matrix of a movie or track header that changes nothing: 1.0 on the
/// diagonal, in 16.16 fixed point and, in the last column, 2.30.
const MATRIX: [u8; 36] = [
    0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0x40, 0, 0, 0,
];

/// The ISO 639-2 code `und`, for a language that is not stated, packed as
/// three letters of five bits.
const UNDETERMINED: u16 = 0x55C4;

/// The sample flags every sample of a fragment gets unless its run says
/// otherwise: the sample depends on no other, so each is a place to start.
const INDEPENDENT: u32 = 0x0200_0000;

/// Writes the initialisation segment of `track`: the file type box and the
/// movie box.
///
/// `index` gives the length of the audio, from which the edit list says how
/// many samples are left once the track's trim is cut.
#[must_use]
pub fn init_segment(track: &PackTrack, index: &FrameIndex) -> Vec<u8> {
    let timescale = track.timescale().get();
    let header = Body::full(0, 0)
        .u32(0) // creation time
        .u32(0) // modification time
        .u32(timescale)
        .u32(0) // duration: the fragments hold every sample
        .u32(0x0001_0000) // rate 1.0
        .u16(0x0100) // volume 1.0
        .bytes(&[0; 10]) // reserved
        .bytes(&MATRIX)
        .bytes(&[0; 24]) // pre_defined
        .u32(2) // the next track ID
        .boxed(*b"mvhd");
    let defaults = Body::full(0, 0)
        .u32(TRACK_ID)
        .u32(1) // the sample description every fragment uses
        .u32(0) // no default duration: every run states its own
        .u32(0) // no default size
        .u32(INDEPENDENT)
        .boxed(*b"trex");
    let movie = Body::new()
        .bytes(&header)
        .bytes(&track_box(track, index))
        .bytes(&Body::new().bytes(&defaults).boxed(*b"mvex"))
        .boxed(*b"moov");
    let mut segment = Body::new()
        .bytes(b"iso5") // the major brand
        .u32(0) // its minor version
        .bytes(b"iso5iso6mp41") // the compatible brands
        .boxed(*b"ftyp");
    segment.extend_from_slice(&movie);
    segment
}

/// The `trak` box: the track header, the edit that carries the trim, and
/// the media.
fn track_box(track: &PackTrack, index: &FrameIndex) -> Vec<u8> {
    let header = Body::full(0, 3) // enabled, and part of the movie
        .u32(0) // creation time
        .u32(0) // modification time
        .u32(TRACK_ID)
        .u32(0) // reserved
        .u32(0) // duration
        .bytes(&[0; 8]) // reserved
        .u16(0) // layer
        .u16(0) // alternate group
        .u16(0x0100) // volume 1.0
        .u16(0) // reserved
        .bytes(&MATRIX)
        .u32(0) // width
        .u32(0) // height
        .boxed(*b"tkhd");
    let delay = u64::from(track.trim.delay);
    let played = index
        .end
        .sample
        .saturating_sub(delay)
        .saturating_sub(u64::from(track.trim.padding));
    let edits = Body::full(1, 0)
        .u32(1) // one edit
        .u64(played) // how long it plays
        .u64(delay) // where it starts in the media
        .u16(1) // at rate 1.0
        .u16(0)
        .boxed(*b"elst");
    Body::new()
        .bytes(&header)
        .bytes(&Body::new().bytes(&edits).boxed(*b"edts"))
        .bytes(&media_box(track))
        .boxed(*b"trak")
}

/// The `mdia` box: the media header, the sound handler and the media
/// information with its one self-contained data reference.
fn media_box(track: &PackTrack) -> Vec<u8> {
    let header = Body::full(0, 0)
        .u32(0) // creation time
        .u32(0) // modification time
        .u32(track.timescale().get())
        .u32(0) // duration
        .u16(UNDETERMINED)
        .u16(0) // pre_defined
        .boxed(*b"mdhd");
    let handler = Body::full(0, 0)
        .u32(0) // pre_defined
        .bytes(b"soun")
        .bytes(&[0; 12]) // reserved
        .bytes(&[0]) // an empty name
        .boxed(*b"hdlr");
    let sound = Body::full(0, 0)
        .u16(0) // balance
        .u16(0) // reserved
        .boxed(*b"smhd");
    // Flag 1: the media data is in the same file as this box.
    let here = Body::full(0, 1).boxed(*b"url ");
    let references = Body::full(0, 0)
        .u32(1) // one reference
        .bytes(&here)
        .boxed(*b"dref");
    let information = Body::new()
        .bytes(&sound)
        .bytes(&Body::new().bytes(&references).boxed(*b"dinf"))
        .bytes(&sample_table(track))
        .boxed(*b"minf");
    Body::new()
        .bytes(&header)
        .bytes(&handler)
        .bytes(&information)
        .boxed(*b"mdia")
}

/// The `stbl` box: the sample description, and the four tables a movie
/// must have, each empty because the fragments hold every sample.
fn sample_table(track: &PackTrack) -> Vec<u8> {
    let description = Body::full(0, 0)
        .u32(1) // one sample entry
        .bytes(&flac_entry(&track.info))
        .boxed(*b"stsd");
    let empty = |kind| Body::full(0, 0).u32(0).boxed(kind);
    let sizes = Body::full(0, 0)
        .u32(0) // no size every sample shares
        .u32(0) // no samples
        .boxed(*b"stsz");
    Body::new()
        .bytes(&description)
        .bytes(&empty(*b"stts"))
        .bytes(&empty(*b"stsc"))
        .bytes(&sizes)
        .bytes(&empty(*b"stco"))
        .boxed(*b"stbl")
}

/// The `fLaC` sample entry with its `dfLa` box, which holds the stream's
/// STREAMINFO block as the last metadata block.
fn flac_entry(info: &StreamInfo) -> Vec<u8> {
    let channels = info.channels.get().get();
    let bits = info.bits_per_sample.get().get();
    let hz = info.sample_rate.hz().get();
    let [_, min_high, min_middle, min_low] = frame_size(info.min_frame_size).to_be_bytes();
    let [_, max_high, max_middle, max_low] = frame_size(info.max_frame_size).to_be_bytes();
    // The sample rate in 20 bits, the channels less one in 3, the bits per
    // sample less one in 5 and the sample count in 36. A track holds only
    // values that fit their fields, so the fields never overlap and adding
    // them sets each one.
    let packed = (u64::from(hz) << 44)
        .saturating_add(u64::from(channels.saturating_sub(1)) << 41)
        .saturating_add(u64::from(bits.saturating_sub(1)) << 36)
        .saturating_add(info.total_samples.map_or(0, NonZeroU64::get));
    let block = Body::full(0, 0)
        .bytes(&[0x80, 0, 0, 34]) // the last block: STREAMINFO, 34 octets
        .u16(info.min_block_size)
        .u16(info.max_block_size)
        .bytes(&[min_high, min_middle, min_low])
        .bytes(&[max_high, max_middle, max_low])
        .u64(packed)
        .bytes(&info.md5.map_or([0; 16], |md5| md5.0))
        .boxed(*b"dfLa");
    Body::new()
        .bytes(&[0; 6]) // reserved
        .u16(1) // the data reference this entry uses
        .bytes(&[0; 8]) // reserved
        .u16(low_half(channels))
        .u16(low_half(bits))
        .u16(0) // pre_defined
        .u16(0) // reserved
        .u16(entry_rate(hz))
        .u16(0) // the fraction of the rate
        .bytes(&block)
        .boxed(*b"fLaC")
}

/// `value` as a 16-bit field. A track's channel count and bit depth are
/// far below its limit.
fn low_half(value: u32) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

/// The sample rate a sample entry states for a stream of `hz` hertz.
///
/// The field holds 16 bits. A rate that does not fit is halved until it
/// does, which is the greatest regular division the FLAC mapping asks for:
/// 48,000 for 96,000 and for 192,000. An odd rate that still does not fit
/// is written as 65,535. The `dfLa` box and the media header state the real
/// rate either way.
fn entry_rate(hz: u32) -> u16 {
    let mut rate = hz;
    loop {
        if let Ok(fits) = u16::try_from(rate) {
            return fits;
        }
        if rate & 1 == 1 {
            return u16::MAX;
        }
        rate >>= 1;
    }
}

#[cfg(test)]
mod tests {
    use std::num::{NonZeroU32, NonZeroU64};

    use super::super::testing::{cd_info, cd_track, index_of};
    use super::init_segment;
    use crate::catalog::Trim;
    use crate::formats::flac::metadata::{AudioMd5, StreamInfo};
    use crate::formats::mp4::{
        AudioEntry, AudioTrack, ChunkOffsets, CodecConfig, FIXED_STEPS, FileType, Flac, FourCc,
        Mp4Audio, Probe, STEPS_PER_BYTE, SampleSizes, SampleTableRanges,
    };
    use crate::package::{FrameIndex, PackTrack};
    use crate::parse::{Budget, Limits, drive};
    use crate::values::{BitDepth, Channels, SampleRate};
    use gunmetal_testkit::bytes::Bytes;
    use gunmetal_testkit::mp4::{self as kit, SampleEntry, ftyp, full_box, mp4_box};

    /// The identity matrix, as ISO/IEC 14496-12 section 8.2.2 writes it:
    /// 0x00010000, 0, 0, 0, 0x00010000, 0, 0, 0, 0x40000000.
    const IDENTITY: [u8; 36] = [
        0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // a, b, u
        0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, // c, d, v
        0, 0, 0, 0, 0, 0, 0, 0, 0x40, 0, 0, 0, // x, y, w
    ];

    /// `parts` joined, so a literal box reads one field per line.
    fn cat(parts: &[&[u8]]) -> Vec<u8> {
        parts.concat()
    }

    /// The `trak` box of a track of `timescale` units a second whose edit
    /// plays `played` units from `delay` and whose sample entry is `entry`,
    /// laid out by the testkit's box builders from the fields written out
    /// here.
    fn track_box(timescale: u32, played: u64, delay: u64, entry: &[u8]) -> Vec<u8> {
        let mut tkhd = Bytes::new();
        tkhd.zeros(8) // creation and modification times
            .u32_be(1) // track ID
            .zeros(4)
            .u32_be(0) // duration
            .zeros(8)
            .u16_be(0) // layer
            .u16_be(0) // alternate group
            .u16_be(0x0100) // volume
            .u16_be(0)
            .bytes(&IDENTITY)
            .zeros(8); // width and height
        let mut elst = Bytes::new();
        elst.u32_be(1) // one edit
            .u64_be(played)
            .u64_be(delay)
            .u16_be(1) // rate 1.0
            .u16_be(0);
        let edts = mp4_box(*b"edts", &full_box(*b"elst", 1, 0, elst.as_slice()));
        let url = full_box(*b"url ", 0, 1, &[]);
        let dref = full_box(*b"dref", 0, 0, &cat(&[&[0, 0, 0, 1], &url]));
        let stbl = cat(&[
            &kit::stsd(&[entry]),
            &full_box(*b"stts", 0, 0, &[0; 4]),
            &full_box(*b"stsc", 0, 0, &[0; 4]),
            &full_box(*b"stsz", 0, 0, &[0; 8]),
            &full_box(*b"stco", 0, 0, &[0; 4]),
        ]);
        let minf = cat(&[
            &full_box(*b"smhd", 0, 0, &[0; 4]),
            &mp4_box(*b"dinf", &dref),
            &mp4_box(*b"stbl", &stbl),
        ]);
        let mdia = cat(&[
            &kit::mdhd(timescale, 0),
            &kit::hdlr(*b"soun"),
            &mp4_box(*b"minf", &minf),
        ]);
        mp4_box(
            *b"trak",
            &cat(&[
                &full_box(*b"tkhd", 0, 3, tkhd.as_slice()),
                &edts,
                &mp4_box(*b"mdia", &mdia),
            ]),
        )
    }

    /// The whole initialisation segment of the track [`track_box`]
    /// describes.
    fn movie(timescale: u32, played: u64, delay: u64, entry: &[u8]) -> Vec<u8> {
        let mut mvhd = Bytes::new();
        mvhd.zeros(8) // creation and modification times
            .u32_be(timescale)
            .u32_be(0) // duration
            .u32_be(0x0001_0000) // rate
            .u16_be(0x0100) // volume
            .zeros(10)
            .bytes(&IDENTITY)
            .zeros(24)
            .u32_be(2); // next track ID
        let trex = full_box(
            *b"trex",
            0,
            0,
            &[
                0, 0, 0, 1, // track ID
                0, 0, 0, 1, // sample description index
                0, 0, 0, 0, // default duration
                0, 0, 0, 0, // default size
                0x02, 0, 0, 0, // default flags: depends on no other sample
            ],
        );
        let moov = cat(&[
            &full_box(*b"mvhd", 0, 0, mvhd.as_slice()),
            &track_box(timescale, played, delay, entry),
            &mp4_box(*b"mvex", &trex),
        ]);
        cat(&[
            &ftyp(*b"iso5", 0, &[*b"iso5", *b"iso6", *b"mp41"]),
            &mp4_box(*b"moov", &moov),
        ])
    }

    /// A `fLaC` sample entry of `channels` channels of `bits` bits that
    /// states `rate` and holds `dfla`.
    fn flac_sample_entry(channels: u16, bits: u16, rate: u16, dfla: &[u8]) -> Vec<u8> {
        kit::sample_entry(&SampleEntry {
            format: *b"fLaC",
            version: 0,
            channels,
            bits,
            rate,
            children: dfla,
        })
    }

    /// The sample entry of the test file's track.
    fn cd_entry() -> Vec<u8> {
        let dfla = kit::dfla(&kit::StreamInfo {
            min_block: 4_096,
            max_block: 4_096,
            sample_rate: 44_100,
            channels: 2,
            bits: 16,
            total_samples: 10_240,
        });
        flac_sample_entry(2, 16, 44_100, &dfla)
    }

    /// An index of one segment that ends at sample `samples`.
    fn ending_at(samples: u64) -> FrameIndex {
        index_of(&[(100, 0)], (142, samples))
    }

    fn track_of(info: &StreamInfo, delay: u32, padding: u32) -> PackTrack {
        PackTrack::flac(info, Trim { delay, padding }).expect("the test's track fits its boxes")
    }

    #[test]
    fn writes_the_file_type_and_the_movie_of_a_flac_track() {
        let init = init_segment(&cd_track(), &ending_at(10_240));
        assert_eq!(init.len(), 639);
        // The file type box, and the header of the movie box after it.
        assert_eq!(
            init.get(..36),
            Some(&b"\x00\x00\x00\x1Cftypiso5\x00\x00\x00\x00iso5iso6mp41\x00\x00\x02\x63moov"[..])
        );
        assert_eq!(init, movie(44_100, 10_240, 0, &cd_entry()));
    }

    /// Verifies: SEC-MED-032
    #[test]
    fn the_mp4_parser_reads_the_track_back() {
        let init = init_segment(&cd_track(), &ending_at(10_240));
        let budget = Budget::for_input(639, STEPS_PER_BYTE, FIXED_STEPS);
        let read = drive(Probe::new(Limits::DEFAULT, budget), &init, &Limits::DEFAULT);
        let rate = SampleRate::new(44_100).ok();
        let channels = Channels::new(2).ok();
        let bits = BitDepth::new(16).ok();
        let expected = Mp4Audio {
            file_type: Some(FileType {
                major: FourCc(*b"iso5"),
                minor: 0,
                compatible: vec![FourCc(*b"iso5"), FourCc(*b"iso6"), FourCc(*b"mp41")],
            }),
            track: AudioTrack {
                offset: 144,
                timescale: NonZeroU32::new(44_100).unwrap(),
                duration: Some(0),
                entry: AudioEntry {
                    format: FourCc(*b"fLaC"),
                    channels,
                    bits,
                    sample_rate: rate,
                    config: CodecConfig::Flac(Flac {
                        min_block: 4_096,
                        max_block: 4_096,
                        sample_rate: rate,
                        channels,
                        bits,
                        total_samples: Some(10_240),
                        blocks: 493..531,
                    }),
                },
            },
            items: Vec::new(),
            sample_tables: SampleTableRanges {
                stts: Some(539..547),
                stsc: Some(555..563),
                sizes: Some(SampleSizes::Stsz(571..583)),
                offsets: Some(ChunkOffsets::Stco(591..599)),
            },
            problems: Vec::new(),
        };
        assert_eq!(read, Ok(Ok(expected)));
    }

    #[test]
    fn carries_the_trim_as_the_one_edit_of_the_track() {
        // 2,112 samples of delay and 1,000 of padding leave 7,128 of 10,240.
        let trimmed = track_of(&cd_info(), 2_112, 1_000);
        let init = init_segment(&trimmed, &ending_at(10_240));
        let edit = cat(&[
            b"\x00\x00\x00\x2Cedts",
            b"\x00\x00\x00\x24elst\x01\x00\x00\x00", // version 1
            &[0, 0, 0, 1],                           // one edit
            &[0, 0, 0, 0, 0, 0, 0x1B, 0xD8],         // it plays 7,128 samples
            &[0, 0, 0, 0, 0, 0, 0x08, 0x40],         // from sample 2,112 of the media
            &[0, 1, 0, 0],                           // at rate 1.0
        ]);
        assert_eq!(init.get(244..288), Some(edit.as_slice()));
        assert_eq!(init, movie(44_100, 7_128, 2_112, &cd_entry()));
        // The same trim on a file 2^32 samples longer leaves that many more.
        assert_eq!(
            init_segment(&trimmed, &ending_at(4_294_977_536)),
            movie(44_100, 4_294_974_424, 2_112, &cd_entry())
        );
    }

    #[test]
    fn plays_nothing_when_the_trim_is_longer_than_the_audio() {
        let cases: [(u32, u32, u64); 4] = [
            // A delay of one sample more than there is.
            (10_241, 0, 0),
            // A padding of one sample more than the delay leaves.
            (240, 10_001, 0),
            // A trim of exactly the audio's length, and of one sample less.
            (240, 10_000, 0),
            (240, 9_999, 1),
        ];
        for (delay, padding, played) in cases {
            let track = track_of(&cd_info(), delay, padding);
            assert_eq!(
                init_segment(&track, &ending_at(10_240)),
                movie(44_100, played, u64::from(delay), &cd_entry()),
                "{delay} and {padding}"
            );
        }
    }

    #[test]
    fn writes_every_field_of_the_stream_info_block() {
        // Mono, 24 bits, 96 kHz, with frame sizes, a sample count that
        // needs all 36 bits and an MD5.
        let md5 = [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD,
            0xEE, 0xFF,
        ];
        let info = StreamInfo {
            min_block_size: 0x0123,
            max_block_size: 0x4567,
            min_frame_size: NonZeroU32::new(0x00_000E),
            max_frame_size: NonZeroU32::new(0xAB_CDEF),
            sample_rate: SampleRate::new(96_000).unwrap(),
            channels: Channels::new(1).unwrap(),
            bits_per_sample: BitDepth::new(24).unwrap(),
            total_samples: NonZeroU64::new(0x9_8765_4321),
            md5: Some(AudioMd5(md5)),
        };
        let init = init_segment(&track_of(&info, 0, 0), &ending_at(48));
        let dfla = cat(&[
            b"\x00\x00\x00\x32dfLa\x00\x00\x00\x00",
            &[0x80, 0, 0, 34],   // the last block, STREAMINFO, 34 octets
            &[0x01, 0x23],       // the smallest block
            &[0x45, 0x67],       // the largest block
            &[0x00, 0x00, 0x0E], // the smallest frame
            &[0xAB, 0xCD, 0xEF], // the largest frame
            // 96,000 Hz is 0x17700 in 20 bits; then 000 for one channel,
            // 10111 for 24 bits, and 0x9 to start the sample count.
            &[0x17, 0x70, 0x01, 0x79],
            &[0x87, 0x65, 0x43, 0x21], // the rest of the sample count
            &md5,
        ]);
        assert_eq!(init.get(481..531), Some(dfla.as_slice()));
        assert_eq!(
            init,
            movie(96_000, 48, 0, &flac_sample_entry(1, 24, 48_000, &dfla))
        );
    }

    #[test]
    fn states_the_sample_rate_the_entry_can_hold() {
        let cases: [(u32, u16); 9] = [
            (8_000, 8_000),
            (44_100, 44_100),
            (65_535, 65_535),
            (65_536, 32_768),
            (96_000, 48_000),
            (192_000, 48_000),
            (705_600, 44_100),
            // Odd, so it has no half; and twice an odd rate that does not
            // fit either.
            (65_537, 65_535),
            (131_074, 65_535),
        ];
        for (hz, stated) in cases {
            let info = StreamInfo {
                sample_rate: SampleRate::new(hz).unwrap(),
                ..cd_info()
            };
            let init = init_segment(&track_of(&info, 0, 0), &ending_at(10_240));
            // The entry's 16.16 rate, and the timescales of the movie and
            // of the media, which are the real rate.
            let entry_rate = cat(&[&stated.to_be_bytes(), &[0, 0]]);
            assert_eq!(init.get(477..481), Some(entry_rate.as_slice()), "{hz}");
            assert_eq!(init.get(56..60), Some(&hz.to_be_bytes()[..]), "{hz}");
            assert_eq!(init.get(316..320), Some(&hz.to_be_bytes()[..]), "{hz}");
        }
    }

    /// Verifies: SEC-MED-074
    #[test]
    fn refers_to_no_data_outside_the_segment() {
        let init = init_segment(&cd_track(), &ending_at(10_240));
        let references = cat(&[
            b"\x00\x00\x00\x24dinf",
            b"\x00\x00\x00\x1Cdref\x00\x00\x00\x00",
            &[0, 0, 0, 1], // one reference
            // A URL entry with no URL, flagged "the media is in this file".
            b"\x00\x00\x00\x0Curl \x00\x00\x00\x01",
        ]);
        assert_eq!(init.get(385..421), Some(references.as_slice()));
        // No other box of the segment is a data reference.
        let locators = init.windows(4).filter(|kind| *kind == b"url ").count();
        let names = init.windows(4).filter(|kind| *kind == b"urn ").count();
        assert_eq!((locators + names, init.len()), (1, 639));
    }
}
