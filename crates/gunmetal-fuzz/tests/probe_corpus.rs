//! Replays the committed file-probe fuzz corpus through its harness on
//! stable Rust, so that `cargo test` and the gate run every seed and every
//! reproducer of a past finding.
//!
//! Each file in `fuzz/seeds/probe` has a test here that pins its exact
//! bytes, built independently with the testkit's builders, and the exact
//! outcome the harness reports for it: for a file the probe reads, the
//! whole `Probed`, with the octets read worked out in the test's comment.
//! The harness probes with no extension, so the content alone decides the
//! format. Commit a fuzzing reproducer by adding its file and its test
//! together.
#![expect(
    clippy::disallowed_methods,
    reason = "replay reads the committed seed corpus, a repository fixture, by path (SEC-MED-028)"
)]

use std::fs;
use std::path::PathBuf;

use gunmetal_core::catalog::{
    AudioFormat, Bitrate, ByteRange, Codec, Container, FileFacts, IdentityInputs, TechInfo, Trim,
};
use gunmetal_core::formats::detect::{DetectError, Format};
use gunmetal_core::formats::flac::metadata::FlacError;
use gunmetal_core::formats::id3v1::Id3v1Tag;
use gunmetal_core::formats::mp4::SampleTableRanges;
use gunmetal_core::formats::mpa::{SeekIndex as MpegIndex, SeekPoint};
use gunmetal_core::formats::vorbis_comment::{Comments, Field};
use gunmetal_core::probe::{ProbeError, Probed, SeekIndex, TagBlock};
use gunmetal_core::text::Text;
use gunmetal_core::values::{BitDepth, Channels, Duration, SampleRate};
use gunmetal_fuzz::probe::{Outcome, run};
use gunmetal_testkit::bytes::Bytes;
use gunmetal_testkit::flac::{self, Block, StreamInfo};
use gunmetal_testkit::id3v1::Id3v1;
use gunmetal_testkit::mp4::{
    Esds, SampleEntry, audio_specific_config, esds, ftyp, mdhd, mp4_box, sample_entry, stsd, trak,
};
use gunmetal_testkit::mpa::{self, Frame, Mode};
use gunmetal_testkit::ogg::{self, FIRST, LAST, Page};
use gunmetal_testkit::opus::{self, OpusHead};
use gunmetal_testkit::riff;

/// The committed corpus, which the nightly fuzz job also starts from.
fn seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/probe")
}

/// Every file the corpus holds, in byte order of their names.
const SEEDS: [&str; 9] = [
    "aiff",
    "empty",
    "flac",
    "flac-short-stream-info",
    "mp3",
    "mp4-movie-at-end",
    "opus",
    "text",
    "wav",
];

/// Reads seed `name`, checks that it holds exactly `bytes`, and checks that
/// the harness reports `expected` for it.
fn replay(name: &str, bytes: &[u8], expected: &Outcome) {
    let file = fs::read(seeds_dir().join(name)).expect("seed file is readable");
    assert_eq!(file, bytes, "seed {name} holds different bytes");
    assert_eq!(&run(&file), expected, "seed {name}");
}

/// Text that was decoded whole, with nothing replaced.
fn text(value: &str) -> Text {
    Text {
        value: value.to_owned(),
        truncated: false,
        replaced: false,
    }
}

/// The technical facts of a file: its codec and container, its rate,
/// depth and channels, and its bitrate and duration.
fn tech(
    (codec, container): (Codec, Container),
    (hz, bits, channels): (u32, Option<u32>, u32),
    bitrate: u32,
    millis: u64,
) -> TechInfo {
    TechInfo::new(
        codec,
        container,
        AudioFormat {
            sample_rate: Some(SampleRate::new(hz).expect("a valid rate")),
            bit_depth: bits.map(|bits| BitDepth::new(bits).expect("a valid depth")),
            channels: Some(Channels::new(channels).expect("valid channels")),
            bitrate: Some(Bitrate::new(bitrate).expect("a valid bitrate")),
            duration: Some(Duration::from_millis(millis).expect("a valid duration")),
        },
    )
    .expect("facts that fit together")
}

/// What the probe finds in a file of `format` with `tech`, whose audio is
/// at `start..end`, after reading `bytes_read` octets, before the parts
/// that differ from one format to another are filled in.
fn found(format: Format, tech: TechInfo, (start, end): (u64, u64), bytes_read: u64) -> Probed {
    Probed {
        format,
        facts: FileFacts {
            tech,
            trim: None,
            artwork: vec![],
            lyrics: vec![],
            identity: IdentityInputs {
                audio_md5: None,
                audio_window: ByteRange::new(start, end).expect("a valid range"),
            },
            parser_version: 1,
            bytes_read,
        },
        tags: vec![],
        seek: SeekIndex::None,
        problems: vec![],
    }
}

/// Sixteen octets of samples: two milliseconds of 8-bit mono at 8 kHz,
/// which are 64,000 bits a second.
const SAMPLES: [u8; 16] = [0x80; 16];

/// A WAV file of 60 octets: the form header, `fmt ` at 12..36 and `data`
/// at 36..60, whose samples are at 44..60.
fn wav() -> Vec<u8> {
    let mut chunks = Bytes::new();
    chunks
        .riff_chunk(*b"fmt ", &riff::format(1, 1, 8_000, 8))
        .riff_chunk(*b"data", &SAMPLES);
    riff::wave(chunks.as_slice())
}

/// An AIFF file of 70 octets: the form header, `COMM` at 12..38 and `SSND`
/// at 38..70, whose samples are at 54..70.
fn aiff() -> Vec<u8> {
    let mut chunks = Bytes::new();
    chunks
        .aiff_chunk(*b"COMM", &riff::comm(1, 16, 8, 8_000))
        .aiff_chunk(*b"SSND", &riff::ssnd(0, 0, &SAMPLES));
    riff::aiff(*b"AIFF", chunks.as_slice())
}

/// A FLAC file of 62 octets: the marker, a STREAMINFO block at 4..42 that
/// states two seconds of 16-bit stereo at 44.1 kHz and an MD5 of sevens,
/// then 20 octets standing in for the frames.
fn flac_file() -> Vec<u8> {
    let mut file = flac::stream(&[Block::StreamInfo(StreamInfo {
        min_block_size: 4_096,
        max_block_size: 4_096,
        min_frame_size: 0,
        max_frame_size: 0,
        sample_rate: 44_100,
        channels: 2,
        bits_per_sample: 16,
        total_samples: 88_200,
        md5: [7; 16],
    })]);
    file.extend([0xFF; 20]);
    file
}

/// An MP3 file of 200 octets: three MPEG-2 Layer III frames at 8 kbit/s
/// and 24 kHz in mono, 24 octets and 576 samples each, at 0, 24 and 48,
/// then an `ID3v1` tag titled `One` at 72..200.
fn mp3() -> Vec<u8> {
    let frame = Frame::layer3(mpa::Version::Mpeg2, 1, 1, Mode::Mono);
    [
        mpa::stream(&[frame, frame, frame]),
        Id3v1::new().title(b"One").build(),
    ]
    .concat()
}

/// An MP4 file of 244 octets whose movie comes after its media data:
///
/// | Octets | What |
/// |---|---|
/// | 0..24 | `ftyp` |
/// | 24..48 | `mdat` |
/// | 48..244 | `moov`, holding `trak` at 56 and `mdia` at 64 |
/// | 72..137 | `mdhd` (32 octets) and `hdlr` (33) |
/// | 137..244 | `minf`, `stbl` at 145 and `stsd` at 153 |
///
/// The `mp4a` entry says 44.1 kHz stereo; its descriptor says AAC LC at
/// 48 kHz and 128,000 bits a second. The media header says 88,200 units
/// at 44,100 a second.
fn mp4() -> Vec<u8> {
    let entry = sample_entry(&SampleEntry {
        format: *b"mp4a",
        version: 0,
        channels: 2,
        bits: 16,
        rate: 44_100,
        children: &esds(&Esds {
            object_type: 0x40,
            max_bitrate: 0,
            avg_bitrate: 128_000,
            specific: Some(&audio_specific_config(2, 3, 2)),
            width: 1,
        }),
    });
    let track = trak(*b"soun", &mdhd(44_100, 88_200), &stsd(&[&entry]));
    [
        ftyp(*b"M4A ", 0, &[*b"M4A ", *b"mp42"]),
        mp4_box(*b"mdat", &[0; 16]),
        mp4_box(*b"moov", &track),
    ]
    .concat()
}

/// A page of stream 7 that holds the one packet `packet`.
fn page(sequence: u32, flags: u8, granule: u64, packet: &[u8]) -> Vec<u8> {
    Page {
        flags,
        granule,
        serial: 7,
        sequence,
        lacing: ogg::lacing(packet.len()),
        body: packet.to_vec(),
    }
    .to_bytes()
}

/// An Opus file of 146 octets: pages at 0..47 (the identification
/// header), 47..108 (a comment header with the vendor `ref` and a title)
/// and 108..146, which ends at granule position 48,312: one second after
/// the 312 samples of pre-skip.
fn opus_file() -> Vec<u8> {
    [
        page(0, FIRST, 0, &OpusHead::stereo().to_bytes()),
        page(1, 0, 0, &opus::opus_tags(b"ref", &[b"TITLE=Song"])),
        page(2, LAST, 48_312, &[0; 10]),
    ]
    .concat()
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

/// An empty file is no format on the allowlist.
///
/// Verifies: SEC-HIS-036, SEC-MED-011, SEC-MED-027, SEC-MED-028
#[test]
fn replays_the_empty_input() {
    replay(
        "empty",
        &[],
        &Outcome::Error(ProbeError::Detect(DetectError::Unknown { offset: 0 })),
    );
}

/// Text is no format on the allowlist either.
///
/// Verifies: SEC-HIS-036, SEC-MED-011, SEC-MED-027, SEC-MED-028
#[test]
fn replays_text() {
    replay(
        "text",
        b"hello",
        &Outcome::Error(ProbeError::Detect(DetectError::Unknown { offset: 0 })),
    );
}

/// Detection reads the 60 octets of the file; the chunk walk reads the
/// form header (12), 48 octets at 12 and the 24 left at 36: 144.
///
/// Verifies: SEC-HIS-036, SEC-MED-010, SEC-MED-027, SEC-MED-028
#[test]
fn replays_a_wav_file() {
    let facts = tech((Codec::Pcm, Container::Wav), (8_000, Some(8), 1), 64_000, 2);
    replay(
        "wav",
        &wav(),
        &Outcome::Found(Box::new(found(Format::Wav, facts, (44, 60), 144))),
    );
}

/// Detection reads the 70 octets of the file; the chunk walk reads the
/// form header (12) and 30 octets at 12 and at 38: 142.
///
/// Verifies: SEC-HIS-036, SEC-MED-010, SEC-MED-027, SEC-MED-028
#[test]
fn replays_an_aiff_file() {
    let facts = tech(
        (Codec::Pcm, Container::Aiff),
        (8_000, Some(8), 1),
        64_000,
        2,
    );
    replay(
        "aiff",
        &aiff(),
        &Outcome::Found(Box::new(found(Format::Aiff, facts, (54, 70), 142))),
    );
}

/// Detection reads the 62 octets of the file and the metadata parser the
/// 42 of the marker and the block: 104. The audio window is the 20 octets
/// after the block, which in two seconds are 80 bits a second.
///
/// Verifies: SEC-HIS-036, SEC-MED-010, SEC-MED-027, SEC-MED-028
#[test]
fn replays_a_flac_file() {
    let facts = tech(
        (Codec::Flac, Container::Flac),
        (44_100, Some(16), 2),
        80,
        2_000,
    );
    let mut expected = found(Format::Flac, facts, (42, 62), 104);
    expected.facts.identity.audio_md5 = Some([7; 16]);
    expected.seek = SeekIndex::Flac(vec![]);
    replay("flac", &flac_file(), &Outcome::Found(Box::new(expected)));
}

/// A STREAMINFO block of 33 octets is not one: what playback needs is
/// damaged, so the file fails, at the block.
///
/// Verifies: SEC-HIS-036, SEC-MED-001, SEC-MED-027, SEC-MED-028
#[test]
fn replays_a_flac_file_whose_stream_info_is_short() {
    let mut file = flac::MARKER.to_vec();
    file.extend(flac::header(true, 0, 33));
    file.extend([0; 33]);
    replay(
        "flac-short-stream-info",
        &file,
        &Outcome::Error(ProbeError::Flac(FlacError::StreamInfoLength {
            offset: 4,
            length: 33,
        })),
    );
}

/// Detection reads the 200 octets of the file and the stream parser the
/// file from the start (200), the header after the first frame (4), the
/// first frame (24) and the header at 24, at 48 and at 72 (12). The end of
/// the file is read for its tags (160): 600.
///
/// The `ID3v1` tag ends the audio window at 72. The 1,728 samples play for
/// 72 milliseconds, so the 72 octets of the frames are 8,000 bits a
/// second.
///
/// Verifies: SEC-HIS-036, SEC-MED-010, SEC-MED-027, SEC-MED-028
#[test]
fn replays_an_mp3_file_with_an_id3v1_tag() {
    let facts = tech((Codec::Mp3, Container::Mpeg), (24_000, None, 1), 8_000, 72);
    let mut expected = found(Format::Mpeg, facts, (0, 72), 600);
    expected.tags = vec![TagBlock::Id3v1(Id3v1Tag {
        range: 72..200,
        title: text("One"),
        artist: text(""),
        album: text(""),
        year: text(""),
        comment: text(""),
        track: None,
        genre: 0,
    })];
    expected.seek = SeekIndex::Mpeg(MpegIndex {
        points: vec![
            SeekPoint {
                sample: 0,
                offset: 0,
            },
            SeekPoint {
                sample: 576,
                offset: 24,
            },
            SeekPoint {
                sample: 1_152,
                offset: 48,
            },
        ],
        stride: 1,
    });
    replay("mp3", &mp3(), &Outcome::Found(Box::new(expected)));
}

/// Detection reads the 244 octets of the file. The probe then asks for a
/// header of 32 octets at each of the ten boxes (320) and for the bodies
/// of `ftyp` (16), `mdhd` (24), `hdlr` (25) and `stsd` (83): 712. It never
/// reads `mdat`.
///
/// The descriptor's 48 kHz comes before the entry's 44.1 kHz; the entry
/// gives the channels. Tags lie inside the movie, so the audio window is
/// the whole file.
///
/// Verifies: SEC-HIS-036, SEC-MED-010, SEC-MED-027, SEC-MED-028
#[test]
fn replays_an_mp4_file_whose_movie_is_at_the_end() {
    let facts = tech(
        (Codec::Aac, Container::Mp4),
        (48_000, None, 2),
        128_000,
        2_000,
    );
    let mut expected = found(Format::Mp4, facts, (0, 244), 712);
    expected.tags = vec![TagBlock::Mp4(vec![])];
    expected.seek = SeekIndex::Mp4(SampleTableRanges::default());
    replay(
        "mp4-movie-at-end",
        &mp4(),
        &Outcome::Found(Box::new(expected)),
    );
}

/// The file is read three times over: to detect it, for its headers and
/// as its own end: 438. Its 146 octets in one second are 1,168 bits a
/// second, and the pre-skip is the trim.
///
/// Verifies: SEC-HIS-036, SEC-MED-010, SEC-MED-027, SEC-MED-028
#[test]
fn replays_an_opus_file() {
    let facts = tech(
        (Codec::Opus, Container::Ogg),
        (48_000, None, 2),
        1_168,
        1_000,
    );
    let mut expected = found(Format::Ogg, facts, (0, 146), 438);
    expected.facts.trim = Some(Trim {
        delay: 312,
        padding: 0,
    });
    expected.tags = vec![TagBlock::Vorbis(Comments {
        vendor: text("ref"),
        fields: vec![Field {
            key: "TITLE".to_owned(),
            value: text("Song"),
        }],
        pictures: vec![],
        problems: vec![],
        end: 25,
    })];
    replay("opus", &opus_file(), &Outcome::Found(Box::new(expected)));
}
