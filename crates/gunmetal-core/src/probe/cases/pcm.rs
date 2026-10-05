//! WAV and AIFF files.

use gunmetal_testkit::bytes::Bytes;
use gunmetal_testkit::id3v2::{self, Encoding, Tag, Version};
use gunmetal_testkit::riff;

use super::*;
use crate::catalog::{FileFacts, IdentityInputs};
use crate::formats::aiff::AiffError;
use crate::formats::detect::Format;
use crate::formats::id3v2::{Frame, FrameBody, FrameId, Header, Id3v2Error, Id3v2Tag};
use crate::formats::riff::RiffError;
use crate::parse::ParseFault;
use crate::probe::{PartProblem, SeekIndex, TagBlock};

/// Sixteen octets of samples: two milliseconds of 8-bit mono at 8 kHz.
const SAMPLES: [u8; 16] = [0x80; 16];

/// An `ID3v2.4` tag of 23 octets holding one `TIT2` frame.
fn tag() -> Vec<u8> {
    Tag::new(Version::V24)
        .frame(b"TIT2", 0, &id3v2::text(Encoding::Utf8, &["Hi"]))
        .build()
}

/// What [`tag`] holds, as a block that starts at `offset`.
fn tag_at(offset: u64) -> TagBlock {
    TagBlock::Id3v2 {
        offset,
        tag: Id3v2Tag {
            header: Header {
                major: 4,
                revision: 0,
                flags: 0,
                size: 13,
                len: 23,
            },
            extended: None,
            frames: vec![Frame {
                id: FrameId::Four(*b"TIT2"),
                offset: 10,
                flags: 0,
                body: FrameBody::Text(vec![text("Hi")]),
            }],
            problems: vec![],
        },
    }
}

/// The facts of [`SAMPLES`] in `container`: 16 octets in 2 milliseconds
/// are 64,000 bits a second.
fn two_millis(container: Container) -> TechInfo {
    tech(
        Codec::Pcm,
        container,
        (8_000, Some(8), 1),
        Some(64_000),
        Some(2),
    )
}

/// A WAV file whose chunks are a format of tag `format_tag`, the samples,
/// then `rest`.
fn wav(format_tag: u16, bits: u16, rest: &[u8]) -> Vec<u8> {
    let mut chunks = Bytes::new();
    chunks
        .riff_chunk(*b"fmt ", &riff::format(format_tag, 1, 8_000, bits))
        .riff_chunk(*b"data", &SAMPLES)
        .bytes(rest);
    riff::wave(chunks.as_slice())
}

/// A whole WAV file, laid out as:
///
/// | Octets | What |
/// |---|---|
/// | 0..12 | `RIFF`, the size, `WAVE` |
/// | 12..36 | `fmt `: header and 16 octets |
/// | 36..60 | `data`: header, samples at 44..60 |
/// | 60..82 | `LIST`: header, `INFO`, sub-chunks at 72..82 |
/// | 82..114 | `id3 `: header, the tag at 90..113, a pad octet |
///
/// The chunk walk reads the form header (12), 48 octets at 12, at 36 and
/// at 60, and the 32 that are left at 82: 188. With the 114 of detection,
/// the tag (23) and the sub-chunks (10), that is 335.
///
/// Verifies: SEC-MED-010
#[test]
fn probes_a_wav_file_and_keeps_its_tag_and_its_info_list() {
    let mut rest = Bytes::new();
    rest.riff_chunk(*b"LIST", b"INFOINAM\x02\0\0\0A\0")
        .riff_chunk(*b"id3 ", &tag());
    assert_eq!(
        run(&wav(1, 8, rest.as_slice()), Some("wav")),
        Ok(Probed {
            format: Format::Wav,
            facts: FileFacts {
                tech: two_millis(Container::Wav),
                trim: None,
                artwork: vec![],
                lyrics: vec![],
                identity: IdentityInputs {
                    audio_md5: None,
                    audio_window: range(44, 60),
                },
                parser_version: 1,
                bytes_read: 335,
            },
            tags: vec![
                tag_at(90),
                TagBlock::RiffInfo {
                    offset: 72,
                    octets: b"INAM\x02\0\0\0A\0".to_vec(),
                },
            ],
            seek: SeekIndex::None,
            problems: vec![],
        })
    );
}

/// An `INFO` list of ten octets under a limit of nine is not read; under a
/// limit of ten it is.
///
/// Verifies: SEC-MED-006, SEC-MED-017
#[test]
fn skips_an_info_list_larger_than_a_tag_may_be_in_memory() {
    let mut rest = Bytes::new();
    rest.riff_chunk(*b"LIST", b"INFOINAM\x02\0\0\0A\0");
    let file = wav(1, 8, rest.as_slice());
    let probe_under = |max| {
        let limits = lowered(LimitKind::Id3v2TagBytes, max);
        let probed = run_under(&file, Some("wav"), limits).unwrap();
        (probed.tags, probed.problems, probed.facts.bytes_read)
    };
    // Detection reads the 82 octets; the walk 12 + 48 + 46 + 22.
    assert_eq!(
        probe_under(9),
        (
            vec![],
            vec![PartProblem::Fault(ParseFault::LimitExceeded {
                limit: LimitKind::Id3v2TagBytes,
                value: 10,
                max: 9,
                offset: 72,
            })],
            210
        )
    );
    assert_eq!(
        probe_under(10),
        (
            vec![TagBlock::RiffInfo {
                offset: 72,
                octets: b"INAM\x02\0\0\0A\0".to_vec(),
            }],
            vec![],
            220
        )
    );
}

/// `count` `ID3v2.3` tags with no frames, back to back: ten octets each.
fn empty_tags(count: usize) -> Vec<u8> {
    b"ID3\x03\0\0\0\0\0\0".repeat(count)
}

/// What one of the tags of [`empty_tags`] is read as, when it starts at
/// `offset`.
fn empty_tag_at(offset: u64) -> TagBlock {
    TagBlock::Id3v2 {
        offset,
        tag: Id3v2Tag {
            header: Header {
                major: 3,
                revision: 0,
                flags: 0,
                size: 0,
                len: 10,
            },
            extended: None,
            frames: vec![],
            problems: vec![],
        },
    }
}

/// A WAV file whose `id3 ` chunk, after the samples, holds `count` tags
/// with no frames. The chunk's header is at 60 and its body at 68.
fn wav_with_tags(count: usize) -> Vec<u8> {
    let mut rest = Bytes::new();
    rest.riff_chunk(*b"id3 ", &empty_tags(count));
    wav(1, 8, rest.as_slice())
}

/// An AIFF file whose `ID3 ` chunk, after the samples, holds `count` tags
/// with no frames. The chunk's header is at 70 and its body at 78.
fn aiff_with_tags(count: usize) -> Vec<u8> {
    let mut chunks = Bytes::new();
    chunks
        .aiff_chunk(*b"COMM", &riff::comm(1, 16, 8, 8_000))
        .aiff_chunk(*b"SSND", &riff::ssnd(0, 0, &SAMPLES))
        .aiff_chunk(*b"ID3 ", &empty_tags(count));
    riff::aiff(*b"AIFF", chunks.as_slice())
}

/// A chunk holds one tag. A second tag right after it is left unread, and
/// so is every tag after that: of 300 tags in a chunk, one is kept. What
/// is recorded is where the second tag starts.
///
/// Verifies: SEC-MED-017, SEC-TM-032
#[test]
fn reads_one_tag_from_a_chunk_and_records_a_second() {
    let cases: [(fn(usize) -> Vec<u8>, &str, u64, u64); 2] = [
        (wav_with_tags, "wav", 68, 78),
        (aiff_with_tags, "aiff", 78, 88),
    ];
    for (file_with, ext, first, second) in cases {
        let tags_of = |count| {
            let probed = run(&file_with(count), Some(ext)).unwrap();
            (probed.tags, probed.problems)
        };
        let one = vec![empty_tag_at(first)];
        let unread = vec![PartProblem::Fault(ParseFault::BudgetExceeded {
            offset: second,
        })];
        assert_eq!(tags_of(1), (one.clone(), vec![]), "{ext}");
        assert_eq!(tags_of(2), (one.clone(), unread.clone()), "{ext}");
        assert_eq!(tags_of(300), (one, unread), "{ext}");
    }
}

/// The chunk walk is given its allowance of one step an octet, 78 for
/// this file, and the steps beyond it pay for the tag: one, since a tag
/// with no frames costs its parser nothing.
///
/// Verifies: SEC-MED-007, SEC-MED-017, SEC-TM-032
#[test]
fn charges_a_step_for_the_tag_of_a_chunk() {
    let file = wav_with_tags(1);
    let probe_with = |steps| {
        run_with(&file, Some("wav"), Limits::DEFAULT, steps)
            .0
            .unwrap()
            .map(|probed| (probed.tags, probed.problems))
    };
    assert_eq!(
        probe_with(77),
        Err(ProbeError::Fault(ParseFault::BudgetExceeded { offset: 0 }))
    );
    assert_eq!(
        probe_with(78),
        Ok((
            vec![],
            vec![PartProblem::Id3v2 {
                offset: 68,
                error: Id3v2Error::Fault(ParseFault::BudgetExceeded { offset: 0 }),
            }]
        ))
    );
    assert_eq!(probe_with(79), Ok((vec![empty_tag_at(68)], vec![])));
}

/// Floating-point samples are PCM too.
#[test]
fn reads_floating_point_wav_samples_as_pcm() {
    // Format tag 3, 32 bits: 32,000 octets a second, so the 16 octets play
    // for no whole millisecond and there is no bitrate.
    assert_eq!(
        run(&wav(3, 32, &[]), Some("wav")).map(|probed| probed.facts.tech),
        Ok(tech(
            Codec::Pcm,
            Container::Wav,
            (8_000, Some(32), 1),
            None,
            Some(0)
        ))
    );
}

/// Format tag 2 is ADPCM, which is not read. The error says where the
/// samples start.
#[test]
fn fails_a_wav_file_whose_codec_is_not_pcm() {
    assert_eq!(
        run(&wav(2, 4, &[]), Some("wav")),
        Err(ProbeError::Unsupported {
            format: Format::Wav,
            offset: 44,
        })
    );
}

/// A chunk after the samples declares 100 octets where the file ends. The
/// format and the samples were found, so the file is kept.
///
/// Verifies: SEC-MED-017
#[test]
fn keeps_a_wav_file_whose_chunks_stop_early() {
    let mut rest = Bytes::new();
    rest.bytes(b"JUNK").u32_le(100);
    // The form ends at 68. Detection reads it whole; the walk reads the
    // form header (12), 48 octets at 12, the 32 left at 36 and the 8 left
    // at 60.
    assert_eq!(
        run(&wav(1, 8, rest.as_slice()), Some("wav")),
        Ok(Probed {
            format: Format::Wav,
            facts: FileFacts {
                tech: two_millis(Container::Wav),
                trim: None,
                artwork: vec![],
                lyrics: vec![],
                identity: IdentityInputs {
                    audio_md5: None,
                    audio_window: range(44, 60),
                },
                parser_version: 1,
                bytes_read: 168,
            },
            tags: vec![],
            seek: SeekIndex::None,
            problems: vec![PartProblem::Stopped(ParseFault::Truncated {
                offset: 68,
                needed: 100,
                available: 0,
            })],
        })
    );
}

/// A format chunk of eight octets holds no format. The error says where
/// the chunk starts.
///
/// Verifies: SEC-MED-001
#[test]
fn fails_a_wav_file_whose_format_chunk_is_damaged() {
    let mut chunks = Bytes::new();
    chunks
        .riff_chunk(*b"fmt ", &[0; 8])
        .riff_chunk(*b"data", &SAMPLES);
    assert_eq!(
        run(&riff::wave(chunks.as_slice()), Some("wav")),
        Err(ProbeError::Wav(RiffError::Short {
            id: *b"fmt ",
            offset: 12,
            len: 8,
            needed: 16,
        }))
    );
}

/// A whole AIFF file, laid out as:
///
/// | Octets | What |
/// |---|---|
/// | 0..12 | `FORM`, the size, `AIFF` |
/// | 12..38 | `COMM`: header and 18 octets |
/// | 38..70 | `SSND`: header, offset and block size, samples at 54..70 |
/// | 70..102 | `ID3 `: header, the tag at 78..101, a pad octet |
///
/// The chunk walk reads the form header (12) and 30 octets at 12, at 38
/// and at 70. With the 102 of detection and the tag (23), that is 227.
///
/// Verifies: SEC-MED-010
#[test]
fn probes_an_aiff_file_and_keeps_its_tag() {
    let mut chunks = Bytes::new();
    chunks
        .aiff_chunk(*b"COMM", &riff::comm(1, 16, 8, 8_000))
        .aiff_chunk(*b"SSND", &riff::ssnd(0, 0, &SAMPLES))
        .aiff_chunk(*b"ID3 ", &tag());
    assert_eq!(
        run(&riff::aiff(*b"AIFF", chunks.as_slice()), Some("aiff")),
        Ok(Probed {
            format: Format::Aiff,
            facts: FileFacts {
                tech: two_millis(Container::Aiff),
                trim: None,
                artwork: vec![],
                lyrics: vec![],
                identity: IdentityInputs {
                    audio_md5: None,
                    audio_window: range(54, 70),
                },
                parser_version: 1,
                bytes_read: 227,
            },
            tags: vec![tag_at(78)],
            seek: SeekIndex::None,
            problems: vec![],
        })
    );
}

/// An AIFF-C file whose `COMM` chunk names `compression`: 24 octets of
/// body, so the samples are at 60..76.
fn aiff_c(compression: [u8; 4]) -> Vec<u8> {
    let mut chunks = Bytes::new();
    chunks
        .aiff_chunk(*b"COMM", &riff::comm_c(1, 16, 8, 8_000, compression, b""))
        .aiff_chunk(*b"SSND", &riff::ssnd(0, 0, &SAMPLES));
    riff::aiff(*b"AIFC", chunks.as_slice())
}

#[test]
fn reads_aiff_c_files_whose_compression_is_pcm() {
    for compression in [*b"NONE", *b"sowt", *b"fl32", *b"fl64"] {
        assert_eq!(
            run(&aiff_c(compression), Some("aifc"))
                .map(|probed| (probed.facts.tech, probed.facts.identity.audio_window)),
            Ok((two_millis(Container::Aiff), range(60, 76))),
            "{compression:?}"
        );
    }
}

/// IMA ADPCM, µ-law and a type in the wrong case are not read. The error
/// says where the samples start.
#[test]
fn fails_an_aiff_c_file_whose_compression_is_not_pcm() {
    for compression in [*b"ima4", *b"ulaw", *b"none", *b"SOWT"] {
        assert_eq!(
            run(&aiff_c(compression), Some("aifc")),
            Err(ProbeError::Unsupported {
                format: Format::Aiff,
                offset: 60,
            }),
            "{compression:?}"
        );
    }
}

/// A `COMM` chunk of eight octets holds no format. The error says where
/// the chunk starts.
///
/// Verifies: SEC-MED-001
#[test]
fn fails_an_aiff_file_whose_format_chunk_is_damaged() {
    let mut chunks = Bytes::new();
    chunks
        .aiff_chunk(*b"COMM", &[0; 8])
        .aiff_chunk(*b"SSND", &riff::ssnd(0, 0, &SAMPLES));
    assert_eq!(
        run(&riff::aiff(*b"AIFF", chunks.as_slice()), Some("aiff")),
        Err(ProbeError::Aiff(AiffError::Short {
            id: *b"COMM",
            offset: 12,
            len: 8,
            needed: 18,
        }))
    );
}
