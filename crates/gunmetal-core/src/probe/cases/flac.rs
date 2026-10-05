//! FLAC files.

use gunmetal_testkit::flac::{self, Block, Picture, SeekPoint as BuiltPoint, StreamInfo};
use gunmetal_testkit::id3v2::{self, Encoding, Tag, Version};
use gunmetal_testkit::vorbis_comment::CommentBlock;

use super::*;
use crate::catalog::{FileFacts, IdentityInputs};
use crate::formats::detect::{DetectError, Format};
use crate::formats::flac::metadata::{BlockProblem, FlacError, SeekPoint};
use crate::formats::id3v2::{Frame, FrameBody, FrameId, Header, Id3v2Error, Id3v2Tag};
use crate::formats::vorbis_comment::{Comments, Field};
use crate::parse::ParseFault;
use crate::probe::{PartProblem, SeekIndex, TagBlock};
use crate::values::{Field as ValueField, ValueError};

/// Twenty octets standing in for the frames.
const AUDIO: [u8; 20] = [0xFF; 20];

/// The MD5 the STREAMINFO of [`stream_info`] states.
const MD5: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

/// Two seconds of 16-bit stereo at 44.1 kHz.
fn stream_info() -> StreamInfo {
    StreamInfo {
        min_block_size: 4_096,
        max_block_size: 4_096,
        min_frame_size: 0,
        max_frame_size: 0,
        sample_rate: 44_100,
        channels: 2,
        bits_per_sample: 16,
        total_samples: 88_200,
        md5: MD5,
    }
}

/// A comment block with the vendor `ref`, a title and plain lyrics: 41
/// octets.
fn comment() -> Vec<u8> {
    let mut block = CommentBlock::new(b"ref");
    block.field("TITLE", "Song").field("LYRICS", "la la");
    block.build()
}

/// A front cover of four octets of data, in a block body of 45 octets.
fn cover() -> Picture {
    Picture {
        picture_type: 3,
        media_type: b"image/png".to_vec(),
        description: Vec::new(),
        width: 1,
        height: 1,
        depth: 24,
        colors: 0,
        data: vec![0x89, b'P', b'N', b'G'],
    }
}

/// A whole file, laid out as:
///
/// | Octets | What |
/// |---|---|
/// | 0..4 | `fLaC` |
/// | 4..42 | STREAMINFO: header and 34 octets |
/// | 42..82 | SEEKTABLE: header and two points |
/// | 82..127 | `VORBIS_COMMENT`: header and 41 octets |
/// | 127..176 | PICTURE: header, 41 octets of fields, data at 172..176 |
/// | 176..196 | audio |
fn whole() -> Vec<u8> {
    let mut file = flac::stream(&[
        Block::StreamInfo(stream_info()),
        Block::SeekTable(vec![
            BuiltPoint {
                sample: 0,
                offset: 0,
                samples: 4_096,
            },
            BuiltPoint {
                sample: 4_096,
                offset: 100,
                samples: 4_096,
            },
        ]),
        Block::VorbisComment(comment()),
        Block::Picture(cover()),
    ]);
    file.extend(AUDIO);
    file
}

/// What the probe finds in [`whole`].
///
/// It reads the 196 octets of the file once to detect it, then the marker
/// (4), four block headers (16), STREAMINFO (34), the seek points (36) and
/// the picture's fields (41), and last the comment block (41): 368. The 20
/// octets of audio in 2 seconds are 80 bits a second.
fn whole_probed() -> Probed {
    Probed {
        format: Format::Flac,
        facts: FileFacts {
            tech: tech(
                Codec::Flac,
                Container::Flac,
                (44_100, Some(16), 2),
                Some(80),
                Some(2_000),
            ),
            trim: None,
            artwork: vec![artwork(0, PictureType::FrontCover, 4)],
            lyrics: vec![lyrics(LyricsOrigin::VorbisComment, LyricsTiming::Plain)],
            identity: IdentityInputs {
                audio_md5: Some(MD5),
                audio_window: range(176, 196),
            },
            parser_version: 1,
            bytes_read: 368,
        },
        tags: vec![TagBlock::Vorbis(Comments {
            vendor: text("ref"),
            fields: vec![
                Field {
                    key: "TITLE".to_owned(),
                    value: text("Song"),
                },
                Field {
                    key: "LYRICS".to_owned(),
                    value: text("la la"),
                },
            ],
            pictures: vec![],
            problems: vec![],
            end: 127,
        })],
        seek: SeekIndex::Flac(vec![
            SeekPoint {
                sample: 0,
                offset: 0,
                samples: 4_096,
            },
            SeekPoint {
                sample: 4_096,
                offset: 100,
                samples: 4_096,
            },
        ]),
        problems: vec![],
    }
}

/// Verifies: SEC-MED-010
#[test]
fn probes_a_flac_file_from_its_metadata_alone() {
    assert_eq!(run(&whole(), Some("flac")), Ok(whole_probed()));
}

#[test]
fn detects_a_flac_file_whatever_audio_name_it_has() {
    assert_eq!(run(&whole(), Some("mp3")), Ok(whole_probed()));
    assert_eq!(run(&whole(), None), Ok(whole_probed()));
}

/// The picture is over the limit, so it is skipped; everything else is as
/// before.
///
/// Verifies: SEC-MED-006, SEC-MED-017
#[test]
fn keeps_a_flac_file_whose_artwork_is_over_the_limit() {
    let mut expected = whole_probed();
    expected.facts.artwork = vec![];
    expected.problems = vec![PartProblem::Flac(BlockProblem::Fault {
        block: 127,
        fault: ParseFault::LimitExceeded {
            limit: LimitKind::PictureBytes,
            value: 4,
            max: 3,
            offset: 172,
        },
    })];
    assert_eq!(
        run_under(&whole(), Some("flac"), lowered(LimitKind::PictureBytes, 3)),
        Ok(expected)
    );
}

/// The picture declares five octets of data in a block that holds four.
///
/// Verifies: SEC-MED-017
#[test]
fn keeps_a_flac_file_whose_artwork_is_damaged() {
    let mut damaged = cover().body();
    // The data length is the four octets before the data.
    damaged[40] = 5;
    let mut file = flac::stream(&[
        Block::StreamInfo(stream_info()),
        Block::Raw {
            code: 6,
            body: damaged,
        },
    ]);
    file.extend(AUDIO);
    // The picture's header is at 42, its fields at 46..87 and its data at
    // 87..91, where the audio starts. The file is 111 octets: read once to
    // detect it, then 4 + 4 + 34 + 4 + 41.
    assert_eq!(
        run(&file, Some("flac")),
        Ok(Probed {
            format: Format::Flac,
            facts: FileFacts {
                tech: tech(
                    Codec::Flac,
                    Container::Flac,
                    (44_100, Some(16), 2),
                    Some(80),
                    Some(2_000),
                ),
                trim: None,
                artwork: vec![],
                lyrics: vec![],
                identity: IdentityInputs {
                    audio_md5: Some(MD5),
                    audio_window: range(91, 111),
                },
                parser_version: 1,
                bytes_read: 198,
            },
            tags: vec![],
            seek: SeekIndex::Flac(vec![]),
            problems: vec![PartProblem::Flac(BlockProblem::Fault {
                block: 42,
                fault: ParseFault::Truncated {
                    offset: 87,
                    needed: 5,
                    available: 4,
                },
            })],
        })
    );
}

/// A file with an `ID3v2` tag in front, laid out as:
///
/// | Octets | What |
/// |---|---|
/// | 0..23 | `ID3v2.4`: header, and a `TIT2` frame at 10 |
/// | 23..27 | `fLaC` |
/// | 27..65 | STREAMINFO, with no MD5 |
/// | 65..80 | `VORBIS_COMMENT`: header and 11 octets |
/// | 80..100 | audio |
fn tagged() -> Vec<u8> {
    let mut file = Tag::new(Version::V24)
        .frame(b"TIT2", 0, &id3v2::text(Encoding::Utf8, &["Hi"]))
        .build();
    file.extend(flac::stream(&[
        Block::StreamInfo(StreamInfo {
            md5: [0; 16],
            ..stream_info()
        }),
        Block::VorbisComment(CommentBlock::new(b"ref").build()),
    ]));
    file.extend(AUDIO);
    file
}

/// What the probe finds in [`tagged`].
///
/// Detection reads the 100 octets of the file, then the 77 after the tag.
/// The metadata takes 4 + 4 + 34 + 4, the comment block 11 and the tag 23:
/// 257. The file's own comment comes before the tag in front of it.
fn tagged_probed() -> Probed {
    Probed {
        format: Format::Flac,
        facts: FileFacts {
            tech: tech(
                Codec::Flac,
                Container::Flac,
                (44_100, Some(16), 2),
                Some(80),
                Some(2_000),
            ),
            trim: None,
            artwork: vec![],
            lyrics: vec![],
            identity: IdentityInputs {
                audio_md5: None,
                audio_window: range(80, 100),
            },
            parser_version: 1,
            bytes_read: 257,
        },
        tags: vec![
            TagBlock::Vorbis(Comments {
                vendor: text("ref"),
                fields: vec![],
                pictures: vec![],
                problems: vec![],
                end: 80,
            }),
            TagBlock::Id3v2 {
                offset: 0,
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
            },
        ],
        seek: SeekIndex::Flac(vec![]),
        problems: vec![],
    }
}

#[test]
fn probes_a_flac_file_with_an_id3v2_tag_in_front() {
    assert_eq!(run(&tagged(), Some("flac")), Ok(tagged_probed()));
}

/// A 2.2 tag with the compression flag cannot be read. The file is kept
/// and the tag recorded.
///
/// Verifies: SEC-MED-017
#[test]
fn keeps_a_flac_file_whose_leading_tag_cannot_be_read() {
    let mut file = Tag::new(Version::V22).flags(0x40).padding(13).build();
    file.extend(&tagged()[23..]);
    let mut expected = tagged_probed();
    expected.tags.truncate(1);
    expected.problems = vec![PartProblem::Id3v2 {
        offset: 0,
        error: Id3v2Error::Compressed { offset: 5 },
    }];
    assert_eq!(run(&file, Some("flac")), Ok(expected));
}

/// A file that starts with `count` `ID3v2.3` tags with no frames, ten
/// octets each. After them come `fLaC` and STREAMINFO, 42 octets, and the
/// 20 octets of audio.
fn after_empty_tags(count: usize) -> Vec<u8> {
    let mut file = empty_tags(count);
    file.extend(flac::stream(&[Block::StreamInfo(stream_info())]));
    file.extend(AUDIO);
    file
}

/// What the tags of [`after_empty_tags`] that start at `offsets` are read
/// as.
fn empty_tags_at(offsets: &[u64]) -> Vec<TagBlock> {
    offsets
        .iter()
        .map(|&offset| TagBlock::Id3v2 {
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
        })
        .collect()
}

/// Detection skips up to seven tags in front of a stream, and the probe
/// reads as many. With seven the file is 132 octets. The metadata parser
/// is left one step an octet, and each tag costs one of the steps beyond
/// those, though a tag with no frames costs its parser nothing: with six
/// such steps the seventh tag, at 60, is not read. An eighth tag is more
/// than detection skips, so the file is not probed at all.
///
/// Verifies: SEC-MED-007, SEC-MED-017, SEC-TM-032
#[test]
fn reads_the_seven_leading_tags_detection_skips_at_a_step_each() {
    let probe_with = |count, steps| {
        run_with(
            &after_empty_tags(count),
            Some("flac"),
            Limits::DEFAULT,
            steps,
        )
        .0
        .unwrap()
        .map(|probed| (probed.tags, probed.problems))
    };
    assert_eq!(
        probe_with(7, 138),
        Ok((
            empty_tags_at(&[0, 10, 20, 30, 40, 50]),
            vec![PartProblem::Id3v2 {
                offset: 60,
                error: Id3v2Error::Fault(ParseFault::BudgetExceeded { offset: 0 }),
            }]
        ))
    );
    assert_eq!(
        probe_with(7, 139),
        Ok((empty_tags_at(&[0, 10, 20, 30, 40, 50, 60]), vec![]))
    );
    assert_eq!(
        probe_with(8, 10_000),
        Err(ProbeError::Detect(DetectError::Fault(
            ParseFault::BudgetExceeded { offset: 80 }
        )))
    );
}

/// A file holds at most 16 pictures, wherever they are. The `PICTURE`
/// blocks come first and the picture of the tag in front after them, so
/// with 15 blocks the tag's picture, of one octet, is the sixteenth, and
/// with 16 blocks it is one too many: it is left out, and the 17 found are
/// recorded against the limit.
///
/// Verifies: SEC-MED-006, SEC-MED-017, SEC-TM-032
#[test]
fn keeps_sixteen_pictures_of_a_flac_file_and_its_leading_tag() {
    let found_with = |blocks: usize| {
        let mut file = Tag::new(Version::V24)
            .frame(
                b"APIC",
                0,
                &id3v2::picture(Encoding::Latin1, "image/png", 3, "", &[0x89]),
            )
            .build();
        let mut metadata = vec![Block::StreamInfo(stream_info())];
        metadata.extend((0..blocks).map(|_| Block::Picture(cover())));
        file.extend(flac::stream(&metadata));
        file.extend(AUDIO);
        let probed = run(&file, Some("flac")).unwrap();
        (probed.facts.artwork, probed.problems)
    };
    let in_blocks = |count: u16| -> Vec<ArtworkRef> {
        (0..count)
            .map(|index| artwork(index, PictureType::FrontCover, 4))
            .collect()
    };
    let mut sixteen = in_blocks(15);
    sixteen.push(artwork(15, PictureType::FrontCover, 1));
    assert_eq!(found_with(15), (sixteen, vec![]));
    assert_eq!(
        found_with(16),
        (
            in_blocks(16),
            vec![PartProblem::Fault(ParseFault::LimitExceeded {
                limit: LimitKind::Pictures,
                value: 17,
                max: 16,
                offset: 0,
            })]
        )
    );
}

/// The comment block declares a vendor string longer than the block.
///
/// Verifies: SEC-MED-017
#[test]
fn keeps_a_flac_file_whose_comment_block_cannot_be_read() {
    let mut file = flac::stream(&[
        Block::StreamInfo(stream_info()),
        Block::VorbisComment(vec![9, 0, 0, 0, b'r']),
    ]);
    file.extend(AUDIO);
    // The comment's body is at 46..51 and the audio at 51..71. The file is
    // read once to detect it (71), then 4 + 4 + 34 + 4 and the block (5).
    assert_eq!(
        run(&file, Some("flac")),
        Ok(Probed {
            format: Format::Flac,
            facts: FileFacts {
                tech: tech(
                    Codec::Flac,
                    Container::Flac,
                    (44_100, Some(16), 2),
                    Some(80),
                    Some(2_000),
                ),
                trim: None,
                artwork: vec![],
                lyrics: vec![],
                identity: IdentityInputs {
                    audio_md5: Some(MD5),
                    audio_window: range(51, 71),
                },
                parser_version: 1,
                bytes_read: 122,
            },
            tags: vec![],
            seek: SeekIndex::Flac(vec![]),
            problems: vec![PartProblem::Comment(ParseFault::Truncated {
                offset: 50,
                needed: 9,
                available: 1,
            })],
        })
    );
}

/// STREAMINFO without a sample count gives no duration and so no bitrate;
/// one whose samples play for more than 30 days has its duration dropped
/// and recorded.
///
/// Verifies: SEC-MED-017
#[test]
fn gives_no_duration_for_an_unknown_or_impossible_sample_count() {
    let probe_with = |sample_rate, total_samples| {
        let mut file = flac::stream(&[Block::StreamInfo(StreamInfo {
            sample_rate,
            total_samples,
            ..stream_info()
        })]);
        file.extend(AUDIO);
        run(&file, Some("flac")).map(|probed| (probed.facts.tech, probed.problems))
    };
    assert_eq!(
        probe_with(44_100, 0),
        Ok((
            tech(
                Codec::Flac,
                Container::Flac,
                (44_100, Some(16), 2),
                None,
                None
            ),
            vec![]
        ))
    );
    // 2,592,001 samples at one a second are a second past 30 days.
    assert_eq!(
        probe_with(1, 2_592_001),
        Ok((
            tech(Codec::Flac, Container::Flac, (1, Some(16), 2), None, None),
            vec![PartProblem::Value(ValueError::OutOfRange {
                field: ValueField::Duration,
                value: 2_592_001_000,
            })]
        ))
    );
}

/// A STREAMINFO block of 33 octets is not one. The error says where the
/// block starts.
///
/// Verifies: SEC-MED-001
#[test]
fn fails_a_flac_file_whose_stream_info_is_damaged() {
    let mut file = flac::MARKER.to_vec();
    file.extend(flac::header(true, 0, 33));
    file.extend([0; 33]);
    assert_eq!(
        run(&file, Some("flac")),
        Err(ProbeError::Flac(FlacError::StreamInfoLength {
            offset: 4,
            length: 33,
        }))
    );
}

/// The metadata parser charges the caller's budget itself, and is left
/// its allowance of one step an octet: 196 for [`whole`]. Every step
/// beyond that pays for the tags and the lyrics. The comment block costs
/// its 41 octets, and the lyrics twice their five octets and one.
///
/// Verifies: SEC-MED-007, SEC-MED-017
#[test]
fn leaves_the_flac_parser_its_allowance_and_spends_the_rest_on_tags() {
    let probe_with = |steps| {
        run_with(&whole(), Some("flac"), Limits::DEFAULT, steps)
            .0
            .unwrap()
    };
    // The parser has its 196 steps and the comment block has none.
    let mut expected = whole_probed();
    expected.tags = vec![];
    expected.facts.lyrics = vec![];
    expected.problems = vec![PartProblem::Comment(ParseFault::BudgetExceeded {
        offset: 86,
    })];
    assert_eq!(probe_with(196), Ok(expected));
    // The comment block is read, and no step is left for the lyrics.
    let mut expected = whole_probed();
    expected.facts.lyrics = vec![];
    expected.problems = vec![PartProblem::Lyrics(ParseFault::BudgetExceeded {
        offset: 0,
    })];
    assert_eq!(probe_with(237), Ok(expected));
    assert_eq!(probe_with(249), Ok(whole_probed()));
}

/// The metadata of [`whole`] costs six steps: one for each of the four
/// block headers and one for each of the two seek points. With five, the
/// budget runs out at the last header, at 127.
///
/// Verifies: SEC-MED-007
#[test]
fn fails_a_flac_file_when_the_steps_run_out_in_its_metadata() {
    let probe_with = |steps| {
        let (answer, left) = run_with(&whole(), Some("flac"), Limits::DEFAULT, steps);
        (answer.unwrap().map(|probed| probed.format), left)
    };
    assert_eq!(
        probe_with(5),
        (
            Err(ProbeError::Flac(FlacError::Fault(
                ParseFault::BudgetExceeded { offset: 127 }
            ))),
            0
        )
    );
    assert_eq!(probe_with(6), (Ok(Format::Flac), 0));
    // Of 200 steps, four move to the tags and the parser spends six.
    assert_eq!(probe_with(200), (Ok(Format::Flac), 190));
}
