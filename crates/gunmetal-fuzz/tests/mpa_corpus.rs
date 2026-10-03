//! Replays the committed MPEG audio fuzz corpus through its harness on
//! stable Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/mpa` has a test here that pins its exact bytes
//! and the exact outcome the harness reports for it. Commit a fuzzing
//! reproducer by adding its file and its test together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::formats::mpa::{
    ChannelMode, FrameHeader, HeaderError, LameTag, Layer, MpaError, MpegStream, ReplayGain,
    SeekIndex, SeekPoint, Vbri, Version, Xing,
};
use gunmetal_core::parse::ParseFault;
use gunmetal_core::values::{GainDb, PeakRatio, SampleRate};
use gunmetal_fuzz::mpa::{Outcome, run};

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/mpa")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 11] = [
    "empty",
    "false-sync-in-id3",
    "free-format",
    "info-zero-frames",
    "junk-before-stream",
    "lame-delay-and-padding",
    "layer-ii",
    "three-small-frames",
    "truncated-id3",
    "vbri-header",
    "xing-mpeg2-mono",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

fn hz(value: u32) -> SampleRate {
    SampleRate::new(value).expect("seed sample rates are valid")
}

fn small_header() -> FrameHeader {
    FrameHeader {
        version: Version::Mpeg2,
        layer: Layer::III,
        crc: false,
        bitrate: 8,
        sample_rate: hz(24_000),
        padding: false,
        mode: ChannelMode::Mono,
        samples: 576,
        len: 24,
    }
}

fn middling_header() -> FrameHeader {
    FrameHeader {
        bitrate: 16,
        len: 48,
        ..small_header()
    }
}

fn roomy_header() -> FrameHeader {
    FrameHeader {
        bitrate: 64,
        len: 192,
        ..small_header()
    }
}

const fn point(sample: u64, offset: u64) -> SeekPoint {
    SeekPoint { sample, offset }
}

fn every_frame(count: u64, start: u64, len: u64) -> SeekIndex {
    SeekIndex {
        points: (0..count)
            .map(|frame| point(576 * frame, start + len * frame))
            .collect(),
        stride: 1,
    }
}

fn plain(start: u64, header: FrameHeader, frames: u64, end: u64, seek: SeekIndex) -> MpegStream {
    MpegStream {
        start,
        header,
        xing: None,
        lame: None,
        vbri: None,
        frames,
        end,
        seek,
    }
}

/// MPEG-2 Layer III at 8 kbit/s and 24 kHz, mono: 24 octets.
fn small_frame() -> Vec<u8> {
    let mut frame = vec![0xFF, 0xF3, 0x14, 0xC0];
    frame.extend([0; 20]);
    frame
}

/// The same stream at 16 kbit/s: 48 octets.
fn middling_frame() -> Vec<u8> {
    let mut frame = vec![0xFF, 0xF3, 0x24, 0xC0];
    frame.extend([0; 44]);
    frame
}

/// The same stream at 64 kbit/s: 192 octets.
fn roomy_frame() -> Vec<u8> {
    let mut frame = vec![0xFF, 0xF3, 0x84, 0xC0];
    frame.extend([0; 188]);
    frame
}

/// `count` copies of `item`.
fn repeated(item: &[u8], count: usize) -> Vec<u8> {
    item.repeat(count)
}

/// An `ID3v2.4` tag holding `body`, without a footer.
fn id3v2(body: &[u8]) -> Vec<u8> {
    let size = u32::try_from(body.len()).expect("the body fits a 32-bit length");
    let mut tag = b"ID3\x04\x00\x00".to_vec();
    tag.extend([
        u8::try_from((size >> 21) & 0x7F).expect("syncsafe octet"),
        u8::try_from((size >> 14) & 0x7F).expect("syncsafe octet"),
        u8::try_from((size >> 7) & 0x7F).expect("syncsafe octet"),
        u8::try_from(size & 0x7F).expect("syncsafe octet"),
    ]);
    tag.extend(body);
    tag
}

/// The table of contents `[0, 1, 2, ..., 99]`.
fn counting_toc() -> [u8; 100] {
    core::array::from_fn(|entry| u8::try_from(entry).expect("0..100 fits u8"))
}

/// CRC-16/ARC of `bytes`, written for these tests from the catalogue
/// definition: polynomial 0x8005 reflected, initial value 0.
fn crc16_arc(bytes: &[u8]) -> u16 {
    bytes.iter().fold(0, |crc, &octet| {
        (0..8).fold(crc ^ u16::from(octet), |crc, _| {
            if crc & 1 == 0 {
                crc >> 1
            } else {
                (crc >> 1) ^ 0xA001
            }
        })
    })
}

/// Verifies: SEC-MED-028, SEC-MED-030
#[test]
fn the_corpus_holds_exactly_the_seeds_tested_here() {
    let mut names: Vec<String> = fs::read_dir(seeds_dir())
        .expect("seed directory is readable")
        .map(|entry| {
            entry
                .expect("seed directory entry is readable")
                .file_name()
                .into_string()
                .expect("seed names are UTF-8")
        })
        .collect();
    names.sort();
    assert_eq!(names, SEEDS);
}

/// Verifies: SEC-MED-028, SEC-HIS-036
#[test]
fn replays_the_empty_input() {
    replay(
        "empty",
        &[],
        &Outcome {
            header: None,
            stream: Err(MpaError::NoFrames { offset: 0 }),
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_three_confirmed_frames() {
    let bytes = repeated(&small_frame(), 3);
    replay(
        "three-small-frames",
        &bytes,
        &Outcome {
            header: Some(Ok(small_header())),
            stream: Ok(plain(0, small_header(), 3, 72, every_frame(3, 0, 24))),
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_stream_of_free_format_frames() {
    let mut frame = vec![0xFF, 0xF3, 0x04, 0xC0];
    frame.extend([0; 60]);
    let bytes = repeated(&frame, 3);
    replay(
        "free-format",
        &bytes,
        &Outcome {
            header: Some(Err(HeaderError::FreeFormat)),
            stream: Err(MpaError::FreeFormat { offset: 0 }),
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_an_id3_tag_that_holds_a_false_sync() {
    let mut bytes = id3v2(&repeated(&small_frame(), 2));
    bytes.extend(repeated(&middling_frame(), 3));
    replay(
        "false-sync-in-id3",
        &bytes,
        &Outcome {
            header: Some(Err(HeaderError::NoSync)),
            stream: Ok(plain(58, middling_header(), 3, 202, every_frame(3, 58, 48))),
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_junk_before_the_first_frame() {
    let mut bytes = vec![0; 16];
    bytes.extend(repeated(&small_frame(), 2));
    replay(
        "junk-before-stream",
        &bytes,
        &Outcome {
            header: Some(Err(HeaderError::NoSync)),
            stream: Ok(plain(16, small_header(), 2, 64, every_frame(2, 16, 24))),
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_xing_header_in_a_mono_mpeg2_frame() {
    let mut xing = vec![0xFF, 0xF3, 0x84, 0xC0];
    xing.extend([0; 9]);
    xing.extend(b"Xing\x00\x00\x00\x01\x00\x00\x00\x03");
    xing.extend([0; 192 - 25]);
    let mut bytes = xing;
    bytes.extend(repeated(&roomy_frame(), 2));
    replay(
        "xing-mpeg2-mono",
        &bytes,
        &Outcome {
            header: Some(Ok(roomy_header())),
            stream: Ok(MpegStream {
                start: 0,
                header: roomy_header(),
                xing: Some(Ok(Xing {
                    info: false,
                    frames: Some(3),
                    bytes: None,
                    toc: None,
                    quality: None,
                })),
                lame: None,
                vbri: None,
                frames: 2,
                end: 576,
                seek: SeekIndex {
                    points: vec![point(0, 192), point(576, 384)],
                    stride: 1,
                },
            }),
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_an_info_header_with_zero_frames() {
    let mut bytes = vec![0xFF, 0xF3, 0x84, 0xC0];
    bytes.extend([0; 9]);
    bytes.extend(b"Info\x00\x00\x00\x07\x00\x00\x00\x00\x00\x00\x00\xC0");
    bytes.extend(counting_toc());
    bytes.resize(192, 0);
    replay(
        "info-zero-frames",
        &bytes,
        &Outcome {
            header: Some(Ok(roomy_header())),
            stream: Ok(MpegStream {
                start: 0,
                header: roomy_header(),
                xing: Some(Ok(Xing {
                    info: true,
                    frames: Some(0),
                    bytes: Some(192),
                    toc: Some(counting_toc()),
                    quality: None,
                })),
                lame: None,
                vbri: None,
                frames: 0,
                end: 192,
                seek: SeekIndex {
                    points: vec![],
                    stride: 1,
                },
            }),
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_lame_tag_with_delay_and_padding() {
    let mut prefix = vec![0xFF, 0xF3, 0x84, 0xC0];
    prefix.extend([0; 9]);
    prefix.extend(b"Info\x00\x00\x00\x03\x00\x00\x00\x00\x00\x00\x00\xC0");
    prefix.extend(b"LAME3.100");
    prefix.extend([0x04, 0xB9, 0x00, 0x7E, 0x80, 0x00, 0x2C, 0x43, 0x4A, 0x41]);
    prefix.extend([0x15, 0x40, 0x24, 0x04, 0x80, 0x4D, 0x00, 0x01, 0xE0]);
    prefix.extend([0x00, 0x00, 0x00, 0xC0, 0x1A, 0x7E]);
    let crc = crc16_arc(&prefix);
    prefix.extend(crc.to_be_bytes());
    prefix.resize(192, 0);
    replay(
        "lame-delay-and-padding",
        &prefix,
        &Outcome {
            header: Some(Ok(roomy_header())),
            stream: Ok(MpegStream {
                start: 0,
                header: roomy_header(),
                xing: Some(Ok(Xing {
                    info: true,
                    frames: Some(0),
                    bytes: Some(192),
                    toc: None,
                    quality: None,
                })),
                lame: Some(Ok(LameTag {
                    encoder: *b"LAME3.100",
                    delay: 576,
                    padding: 1_152,
                    music_length: 192,
                    replay_gain: Ok(ReplayGain {
                        peak: Some(PeakRatio::new(0.988_281_25)),
                        track: Some(GainDb::new(6.7).expect("6.7 dB is in range")),
                        album: Some(GainDb::new(-6.5).expect("-6.5 dB is in range")),
                    }),
                })),
                vbri: None,
                frames: 0,
                end: 192,
                seek: SeekIndex {
                    points: vec![],
                    stride: 1,
                },
            }),
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_layer_ii_stream() {
    let mut frame = vec![0xFF, 0xFD, 0x90, 0x00];
    frame.extend([0; 518]);
    let bytes = repeated(&frame, 2);
    replay(
        "layer-ii",
        &bytes,
        &Outcome {
            header: Some(Ok(FrameHeader {
                version: Version::Mpeg1,
                layer: Layer::II,
                crc: false,
                bitrate: 160,
                sample_rate: hz(44_100),
                padding: false,
                mode: ChannelMode::Stereo,
                samples: 1_152,
                len: 522,
            })),
            stream: Err(MpaError::UnsupportedLayer {
                offset: 0,
                layer: Layer::II,
            }),
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_an_id3_tag_that_runs_past_the_file() {
    let bytes = id3v2(&[0; 100]);
    replay(
        "truncated-id3",
        &bytes[..30],
        &Outcome {
            header: Some(Err(HeaderError::NoSync)),
            stream: Err(MpaError::Fault(ParseFault::Truncated {
                offset: 0,
                needed: 110,
                available: 30,
            })),
        },
    );
}

/// Verifies: SEC-MED-028
#[test]
fn replays_a_vbri_header() {
    let mut vbri = vec![0xFF, 0xF3, 0x84, 0xC0];
    vbri.extend([0; 32]);
    vbri.extend(b"VBRI");
    vbri.extend([0x00, 0x01, 0x00, 0x00, 0x00, 0x4B, 0x00, 0x00, 0x02, 0x40]);
    vbri.extend([
        0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x01,
    ]);
    vbri.resize(192, 0);
    let mut bytes = vbri;
    bytes.extend(repeated(&roomy_frame(), 2));
    replay(
        "vbri-header",
        &bytes,
        &Outcome {
            header: Some(Ok(roomy_header())),
            stream: Ok(MpegStream {
                start: 0,
                header: roomy_header(),
                xing: None,
                lame: None,
                vbri: Some(Ok(Vbri {
                    version: 1,
                    quality: 75,
                    bytes: 576,
                    frames: 3,
                })),
                frames: 2,
                end: 576,
                seek: SeekIndex {
                    points: vec![point(0, 192), point(576, 384)],
                    stride: 1,
                },
            }),
        },
    );
}
