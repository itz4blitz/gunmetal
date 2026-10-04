//! MP3 files.

use gunmetal_testkit::ape::{self, Ape};
use gunmetal_testkit::id3v1::Id3v1;
use gunmetal_testkit::id3v2::{self, Encoding, Tag, Version};
use gunmetal_testkit::mpa::{self, Frame, Lame, Mode, Xing};

use super::*;
use crate::catalog::{FileFacts, IdentityInputs, Trim};
use crate::formats::ape::{ApeError, ApeItem, ApeTag, ApeValue};
use crate::formats::detect::Format;
use crate::formats::id3v1::{Id3v1Error, Id3v1Tag};
use crate::formats::id3v2::{
    Frame as TagFrame, FrameBody, FrameId, Header, Id3v2Tag, LanguageText, PictureRef, Span,
    TagProblem,
};
use crate::formats::mpa::{MpaError, SeekIndex as MpegIndex, SeekPoint};
use crate::parse::ParseFault;
use crate::probe::{PartProblem, SeekIndex, TagBlock};

/// An MPEG-1 Layer III frame at 128 kbit/s and 44.1 kHz in stereo: 417
/// octets and 1,152 samples.
fn stereo() -> Frame {
    Frame::layer3(mpa::Version::Mpeg1, 9, 0, Mode::Stereo)
}

/// An MPEG-2 Layer III frame at 8 kbit/s and 24 kHz in mono: 24 octets and
/// 576 samples.
fn mono() -> Frame {
    Frame::layer3(mpa::Version::Mpeg2, 1, 1, Mode::Mono)
}

/// Three mono frames: 72 octets and 1,728 samples, which play for 72
/// milliseconds.
fn small() -> Vec<u8> {
    mpa::stream(&[mono(), mono(), mono()])
}

/// The seek index of a stream whose frames start at `offsets`, `samples`
/// samples apart.
fn index(samples: u64, offsets: &[u64]) -> SeekIndex {
    SeekIndex::Mpeg(MpegIndex {
        points: (0..)
            .zip(offsets)
            .map(|(frame, &offset)| SeekPoint {
                sample: frame * samples,
                offset,
            })
            .collect(),
        stride: 1,
    })
}

/// An `ID3v2.4` tag of 77 octets: `TIT2` at 10, `USLT` holding one LRC line
/// at 23, and `APIC` at 50 with four octets of picture at 73..77.
fn leading_tag() -> Vec<u8> {
    Tag::new(Version::V24)
        .frame(b"TIT2", 0, &id3v2::text(Encoding::Utf8, &["Hi"]))
        .frame(
            b"USLT",
            0,
            &id3v2::comment(Encoding::Utf8, *b"eng", "", "[00:01.00]la"),
        )
        .frame(
            b"APIC",
            0,
            &id3v2::picture(
                Encoding::Latin1,
                "image/png",
                3,
                "",
                &[0x89, b'P', b'N', b'G'],
            ),
        )
        .build()
}

/// What [`leading_tag`] holds.
fn leading_tag_read() -> Id3v2Tag {
    Id3v2Tag {
        header: Header {
            major: 4,
            revision: 0,
            flags: 0,
            size: 67,
            len: 77,
        },
        extended: None,
        frames: vec![
            TagFrame {
                id: FrameId::Four(*b"TIT2"),
                offset: 10,
                flags: 0,
                body: FrameBody::Text(vec![text("Hi")]),
            },
            TagFrame {
                id: FrameId::Four(*b"USLT"),
                offset: 23,
                flags: 0,
                body: FrameBody::Lyrics(LanguageText {
                    language: *b"eng",
                    description: text(""),
                    text: text("[00:01.00]la"),
                }),
            },
            TagFrame {
                id: FrameId::Four(*b"APIC"),
                offset: 50,
                flags: 0,
                body: FrameBody::Picture(PictureRef {
                    mime: text("image/png"),
                    picture_type: 3,
                    description: text(""),
                    data: Span {
                        start: 73,
                        end: 77,
                        unsynchronised: false,
                    },
                }),
            },
        ],
        problems: vec![],
    }
}

/// A file with every kind of tag, laid out as:
///
/// | Octets | What |
/// |---|---|
/// | 0..77 | [`leading_tag`] |
/// | 77..1328 | three stereo frames, at 77, 494 and 911 |
/// | 1328..1462 | APE: header, items at 1360, 1377 and 1413, footer at 1430 |
/// | 1462..1590 | `ID3v1` |
///
/// The APE items are `Title` (17 octets), a front cover whose ten octets
/// of value are at 1403..1413, and `Lyrics` (17 octets).
fn tagged() -> Vec<u8> {
    let mut file = leading_tag();
    file.extend(mpa::stream(&[stereo(), stereo(), stereo()]));
    file.extend(
        Ape::new()
            .text("Title", "Ape")
            .item(b"Cover Art (Front)", ape::BINARY, b"c.png\0\x89PNG")
            .text("Lyrics", "la")
            .build(),
    );
    file.extend(Id3v1::new().title(b"One").build());
    file
}

/// What the probe finds in [`tagged`].
///
/// Detection reads 512 octets at the start and 512 after the leading tag.
/// The stream parser reads the file from the start (1,590) and from the
/// end of the tag (1,513), the header after the first frame (4), the first
/// frame (417) and the header at 494, at 911 and at 1,328 (12). The
/// leading tag is read whole (77), then the last 160 octets, then the APE
/// tag from its start to the end of the file (262): 5,059.
///
/// The audio window is the 1,251 octets of the frames. Their 3,456 samples
/// play for 78 milliseconds, so the bitrate is 1,251 × 8,000 / 78.
fn tagged_probed() -> Probed {
    Probed {
        format: Format::Mpeg,
        facts: FileFacts {
            tech: tech(
                Codec::Mp3,
                Container::Mpeg,
                (44_100, None, 2),
                Some(128_307),
                Some(78),
            ),
            trim: None,
            artwork: vec![
                artwork(0, PictureType::FrontCover, 4),
                artwork(1, PictureType::FrontCover, 10),
            ],
            lyrics: vec![
                lyrics(LyricsOrigin::Id3Unsynced, LyricsTiming::Line),
                lyrics(LyricsOrigin::ApeItem, LyricsTiming::Plain),
            ],
            identity: IdentityInputs {
                audio_md5: None,
                audio_window: range(77, 1_328),
            },
            parser_version: 1,
            bytes_read: 5_059,
        },
        tags: vec![
            TagBlock::Id3v2 {
                offset: 0,
                tag: leading_tag_read(),
            },
            TagBlock::Ape(ApeTag {
                range: 1_328..1_462,
                items: vec![
                    ApeItem {
                        key: "Title".to_owned(),
                        value: ApeValue::Text(vec![text("Ape")]),
                    },
                    ApeItem {
                        key: "Cover Art (Front)".to_owned(),
                        value: ApeValue::Binary(1_403..1_413),
                    },
                    ApeItem {
                        key: "Lyrics".to_owned(),
                        value: ApeValue::Text(vec![text("la")]),
                    },
                ],
                problems: vec![],
            }),
            TagBlock::Id3v1(one()),
        ],
        seek: index(1_152, &[77, 494, 911]),
        problems: vec![],
    }
}

/// An `ID3v1` tag titled `One` at `start`.
fn one_at(start: u64) -> Id3v1Tag {
    Id3v1Tag {
        range: start..start + 128,
        title: text("One"),
        artist: text(""),
        album: text(""),
        year: text(""),
        comment: text(""),
        track: None,
        genre: 0,
    }
}

/// The `ID3v1` tag of [`tagged`].
fn one() -> Id3v1Tag {
    one_at(1_462)
}

/// Verifies: SEC-MED-010
#[test]
fn probes_an_mp3_file_and_leaves_every_tag_out_of_the_audio_window() {
    assert_eq!(run(&tagged(), Some("mp3")), Ok(tagged_probed()));
}

/// An APE tag larger than a tag may be in memory is not read again from
/// its start. The file and its other tags are kept, and the audio window
/// ends at the `ID3v1` tag.
///
/// Verifies: SEC-MED-006, SEC-MED-017
#[test]
fn skips_an_ape_tag_larger_than_a_tag_may_be_in_memory() {
    let mut expected = tagged_probed();
    expected.tags.remove(1);
    expected.facts.artwork.truncate(1);
    expected.facts.lyrics.truncate(1);
    // 1,385 octets in 78 milliseconds.
    expected.facts.tech = tech(
        Codec::Mp3,
        Container::Mpeg,
        (44_100, None, 2),
        Some(142_051),
        Some(78),
    );
    expected.facts.identity.audio_window = range(77, 1_462);
    expected.facts.bytes_read = 4_797;
    expected.problems = vec![PartProblem::Fault(ParseFault::LimitExceeded {
        limit: LimitKind::Id3v2TagBytes,
        value: 262,
        max: 261,
        offset: 1_328,
    })];
    let limits = lowered(LimitKind::Id3v2TagBytes, 261);
    assert_eq!(run_under(&tagged(), Some("mp3"), limits), Ok(expected));
    // At 262 octets the tag is read.
    let limits = lowered(LimitKind::Id3v2TagBytes, 262);
    assert_eq!(
        run_under(&tagged(), Some("mp3"), limits),
        Ok(tagged_probed())
    );
}

/// A file whose first frame holds an Info header and a LAME extension,
/// then two audio frames, at 417 and 834.
fn encoded() -> Vec<u8> {
    let lame = Lame {
        encoder: *b"LAME3.100",
        revision_method: 0,
        lowpass: 0,
        peak: 0,
        track_gain: 0,
        album_gain: 0,
        flags_ath: 0,
        bitrate: 0,
        delay: 576,
        padding: 1_000,
        misc: 0,
        mp3_gain: 0,
        preset: 0,
        music_length: 1_251,
        music_crc: 0,
    };
    let xing = Xing {
        info: true,
        frames: Some(2),
        bytes: Some(1_251),
        toc: None,
        quality: None,
    };
    let mut file = mpa::xing_frame(&stereo(), &xing, Some(&lame));
    file.extend(mpa::stream(&[stereo(), stereo()]));
    file
}

/// The first frame holds no audio. The two after it hold 2,304 samples,
/// of which the encoder added 1,576, so 728 play, for 16 milliseconds.
///
/// Detection reads 512 octets. The stream parser reads the file (1,251),
/// the header after the first frame (4), the first frame (417) and the
/// headers at 417 and 834 (8). Then the last 160 octets are read: 2,352.
#[test]
fn takes_the_trim_of_an_mp3_file_from_its_lame_header() {
    assert_eq!(
        run(&encoded(), Some("mp3")),
        Ok(Probed {
            format: Format::Mpeg,
            facts: FileFacts {
                tech: tech(
                    Codec::Mp3,
                    Container::Mpeg,
                    (44_100, None, 2),
                    Some(625_500),
                    Some(16),
                ),
                trim: Some(Trim {
                    delay: 576,
                    padding: 1_000,
                }),
                artwork: vec![],
                lyrics: vec![],
                identity: IdentityInputs {
                    audio_md5: None,
                    audio_window: range(0, 1_251),
                },
                parser_version: 1,
                bytes_read: 2_352,
            },
            tags: vec![],
            seek: index(1_152, &[417, 834]),
            problems: vec![],
        })
    );
}

/// The first frame announces a Xing table of contents that its 24 octets
/// cannot hold: the name is at 13, the flags at 17 and the fields at 21.
///
/// The file is 72 octets. It is read once to detect it, once by the stream
/// parser, then the header at 24, the first frame (24), the headers at 24
/// and 48, and the whole file again as its own end: 252.
///
/// Verifies: SEC-MED-017
#[test]
fn keeps_an_mp3_file_whose_encoder_header_is_damaged() {
    let mut first = zeros(9);
    first.extend(b"Xing\0\0\0\x04");
    let mut file = mono().write(&first);
    file.extend(mpa::stream(&[mono(), mono()]));
    assert_eq!(
        run(&file, Some("mp3")),
        Ok(Probed {
            format: Format::Mpeg,
            facts: FileFacts {
                tech: tech(
                    Codec::Mp3,
                    Container::Mpeg,
                    (24_000, None, 1),
                    Some(12_000),
                    Some(48),
                ),
                trim: None,
                artwork: vec![],
                lyrics: vec![],
                identity: IdentityInputs {
                    audio_md5: None,
                    audio_window: range(0, 72),
                },
                parser_version: 1,
                bytes_read: 252,
            },
            tags: vec![],
            seek: index(576, &[24, 48]),
            problems: vec![PartProblem::Encoder(ParseFault::Truncated {
                offset: 21,
                needed: 100,
                available: 3,
            })],
        })
    );
}

/// What a probe of `file` says about its tags: the blocks, the artwork,
/// the lyrics, the audio window and the problems.
type Tags = (
    Vec<TagBlock>,
    Vec<ArtworkRef>,
    Vec<LyricsSource>,
    ByteRange,
    Vec<PartProblem>,
);

fn tags_of(file: &[u8], limits: Limits) -> Tags {
    let probed = run_under(file, Some("mp3"), limits).unwrap();
    (
        probed.tags,
        probed.facts.artwork,
        probed.facts.lyrics,
        probed.facts.identity.audio_window,
        probed.problems,
    )
}

/// Two tags in front of the stream are both read, in file order.
#[test]
fn reads_every_leading_id3v2_tag() {
    let second = Tag::new(Version::V23)
        .frame(b"TIT2", 0, &id3v2::text(Encoding::Latin1, &["Yo"]))
        .build();
    let file = [leading_tag(), second, small()].concat();
    let (tags, ..) = tags_of(&file, Limits::DEFAULT);
    assert_eq!(
        tags,
        [
            TagBlock::Id3v2 {
                offset: 0,
                tag: leading_tag_read(),
            },
            TagBlock::Id3v2 {
                offset: 77,
                tag: Id3v2Tag {
                    header: Header {
                        major: 3,
                        revision: 0,
                        flags: 0,
                        size: 13,
                        len: 23,
                    },
                    extended: None,
                    frames: vec![TagFrame {
                        id: FrameId::Four(*b"TIT2"),
                        offset: 10,
                        flags: 0,
                        body: FrameBody::Text(vec![text("Yo")]),
                    }],
                    problems: vec![],
                },
            },
        ]
    );
}

/// A tag of 123 octets under a limit of 30: the probe reads the 50 octets
/// its parser can use, and the parser keeps the frame below the limit.
///
/// Detection reads the file (195) and the 72 octets after the tag: 267.
/// The stream parser reads the file (195) and from the end of the tag
/// (72), then 4 + 24 + 4 + 4: 303. The tag's first 50 octets are read,
/// then the last 160 octets: 780.
///
/// Verifies: SEC-MED-006
#[test]
fn reads_an_id3v2_tag_only_as_far_as_the_limit_keeps_it() {
    let tag = Tag::new(Version::V24)
        .frame(b"TIT2", 0, &id3v2::text(Encoding::Utf8, &["Hi"]))
        .padding(100)
        .build();
    let file = [tag, small()].concat();
    let probed = run_under(&file, Some("mp3"), lowered(LimitKind::Id3v2TagBytes, 30)).unwrap();
    assert_eq!(probed.facts.bytes_read, 780);
    assert_eq!(
        probed.tags,
        [TagBlock::Id3v2 {
            offset: 0,
            tag: Id3v2Tag {
                header: Header {
                    major: 4,
                    revision: 0,
                    flags: 0,
                    size: 113,
                    len: 123,
                },
                extended: None,
                frames: vec![TagFrame {
                    id: FrameId::Four(*b"TIT2"),
                    offset: 10,
                    flags: 0,
                    body: FrameBody::Text(vec![text("Hi")]),
                }],
                problems: vec![TagProblem::Fault(ParseFault::LimitExceeded {
                    limit: LimitKind::Id3v2TagBytes,
                    value: 113,
                    max: 30,
                    offset: 0,
                })],
            },
        }]
    );
    assert_eq!(probed.problems, []);
}

/// The lyrics of an `ID3v2` tag, whatever else the probe finds.
fn id3v2_lyrics(tag: &Tag, limits: Limits) -> (Vec<LyricsSource>, Vec<PartProblem>) {
    let file = [tag.build(), small()].concat();
    let (_, _, lyrics, _, problems) = tags_of(&file, limits);
    (lyrics, problems)
}

fn uslt(words: &str) -> Tag {
    Tag::new(Version::V24).frame(
        b"USLT",
        0,
        &id3v2::comment(Encoding::Utf8, *b"eng", "", words),
    )
}

fn sylt(content_type: u8, lines: &[(&str, u32)]) -> Tag {
    Tag::new(Version::V24).frame(
        b"SYLT",
        0,
        &id3v2::synced_lyrics(Encoding::Utf8, *b"eng", [2, content_type], "", lines),
    )
}

#[test]
fn times_text_lyrics_by_what_they_hold() {
    let cases = [
        ("la la", LyricsTiming::Plain),
        ("[00:01.00]la", LyricsTiming::Line),
        ("[00:01.00]<00:01.00>la <00:02.00>la", LyricsTiming::Word),
    ];
    for (words, timing) in cases {
        assert_eq!(
            id3v2_lyrics(&uslt(words), Limits::DEFAULT),
            (vec![lyrics(LyricsOrigin::Id3Unsynced, timing)], vec![]),
            "{words}"
        );
    }
}

#[test]
fn times_synchronised_lyrics_by_line_or_by_word() {
    assert_eq!(
        id3v2_lyrics(&sylt(1, &[("la", 0), ("li", 500)]), Limits::DEFAULT),
        (
            vec![lyrics(LyricsOrigin::Id3Synced, LyricsTiming::Line)],
            vec![]
        )
    );
    // A line feed at the start of a later entry starts a line, so every
    // entry is a word.
    assert_eq!(
        id3v2_lyrics(&sylt(1, &[("la", 0), ("\nli", 500)]), Limits::DEFAULT),
        (
            vec![lyrics(LyricsOrigin::Id3Synced, LyricsTiming::Word)],
            vec![]
        )
    );
}

/// Content type 1 is lyrics; 2 is a text transcription, and 5 chords.
#[test]
fn takes_only_synchronised_text_that_is_lyrics() {
    for content_type in [0, 2, 5] {
        assert_eq!(
            id3v2_lyrics(&sylt(content_type, &[("la", 0)]), Limits::DEFAULT),
            (vec![], vec![]),
            "content type {content_type}"
        );
    }
}

/// Lyrics of five octets in an APE item, which is not cut to the lyrics
/// limit when it is read, are skipped and recorded under a limit of four;
/// under a limit of five they are kept.
///
/// Verifies: SEC-MED-006, SEC-MED-017
#[test]
fn skips_lyrics_over_the_lyrics_limit() {
    let tag = Ape::new().without_header().text("Lyrics", "la la").build();
    let file = [small(), tag].concat();
    let lyrics_under = |limits| {
        let (_, _, lyrics, _, problems) = tags_of(&file, limits);
        (lyrics, problems)
    };
    assert_eq!(
        lyrics_under(lowered(LimitKind::LyricsBytes, 4)),
        (
            vec![],
            vec![PartProblem::Lyrics(ParseFault::LimitExceeded {
                limit: LimitKind::LyricsBytes,
                value: 5,
                max: 4,
                offset: 0,
            })]
        )
    );
    assert_eq!(
        lyrics_under(lowered(LimitKind::LyricsBytes, 5)),
        (
            vec![lyrics(LyricsOrigin::ApeItem, LyricsTiming::Plain)],
            vec![]
        )
    );
}

/// An APE tag without a header, right after the 72 octets of [`small`].
/// A cover is a binary item and lyrics are a text item; the same keys with
/// the other kind of value are neither.
#[test]
fn takes_ape_pictures_and_lyrics_by_key_and_kind() {
    let tag = Ape::new()
        .without_header()
        .item(b"cover art (back)", ape::BINARY, b"b\0xy")
        .text("Cover Art (Front)", "text")
        .item(b"Lyrics", ape::BINARY, b"zz")
        .text("LYRICS", "[00:02.00]la")
        .item(b"Other", ape::BINARY, b"q")
        .build();
    let file = [small(), tag].concat();
    let (tags, artwork_found, lyrics_found, window, problems) = tags_of(&file, Limits::DEFAULT);
    // Each item is eight octets, its key, a zero and its value: 29 octets
    // from 72, 30 from 101, 17 from 131, 27 from 148 and 15 from 175. The
    // footer is at 190.
    assert_eq!(
        tags,
        [TagBlock::Ape(ApeTag {
            range: 72..222,
            items: vec![
                ApeItem {
                    key: "cover art (back)".to_owned(),
                    value: ApeValue::Binary(97..101),
                },
                ApeItem {
                    key: "Cover Art (Front)".to_owned(),
                    value: ApeValue::Text(vec![text("text")]),
                },
                ApeItem {
                    key: "Lyrics".to_owned(),
                    value: ApeValue::Binary(146..148),
                },
                ApeItem {
                    key: "LYRICS".to_owned(),
                    value: ApeValue::Text(vec![text("[00:02.00]la")]),
                },
                ApeItem {
                    key: "Other".to_owned(),
                    value: ApeValue::Binary(189..190),
                },
            ],
            problems: vec![],
        })]
    );
    assert_eq!(artwork_found, [artwork(0, PictureType::BackCover, 4)]);
    assert_eq!(
        lyrics_found,
        [lyrics(LyricsOrigin::ApeItem, LyricsTiming::Line)]
    );
    assert_eq!(window, range(0, 72));
    assert_eq!(problems, []);
}

/// A footer of version 3000 is no APE tag this parser reads. The footer
/// starts 32 octets before the `ID3v1` tag.
///
/// Verifies: SEC-MED-017
#[test]
fn keeps_an_mp3_file_whose_ape_tag_cannot_be_read() {
    let tag = Ape::new().without_header().version(3_000).build();
    let file = [small(), tag, Id3v1::new().title(b"One").build()].concat();
    assert_eq!(
        tags_of(&file, Limits::DEFAULT),
        (
            vec![TagBlock::Id3v1(one_at(104))],
            vec![],
            vec![],
            range(0, 104),
            vec![PartProblem::Ape(ApeError::UnknownVersion {
                offset: 72,
                version: 3_000,
            })]
        )
    );
}

/// The stream parser is given its documented allowance, four steps an
/// octet and 128 more, and what is left pays for the tags. Reading the
/// `ID3v1` tag of this 200-octet file costs one step.
///
/// Verifies: SEC-MED-007, SEC-MED-017
#[test]
fn spends_the_steps_left_after_the_stream_on_the_tags() {
    let file = [small(), Id3v1::new().title(b"One").build()].concat();
    let probe_with = |steps| {
        run_with(&file, Some("mp3"), Limits::DEFAULT, steps)
            .0
            .unwrap()
            .map(|probed| {
                (
                    probed.tags,
                    probed.facts.identity.audio_window,
                    probed.problems,
                )
            })
    };
    assert_eq!(
        probe_with(927),
        Err(ProbeError::Fault(ParseFault::BudgetExceeded { offset: 0 }))
    );
    assert_eq!(
        probe_with(928),
        Ok((
            vec![],
            range(0, 200),
            vec![PartProblem::Id3v1(Id3v1Error::Fault(
                ParseFault::BudgetExceeded { offset: 72 }
            ))]
        ))
    );
    assert_eq!(
        probe_with(929),
        Ok((vec![TagBlock::Id3v1(one_at(72))], range(0, 72), vec![]))
    );
}

/// One frame whose 417 octets run past the end of a file of 100 is no
/// stream. The error says where the frame starts.
///
/// Verifies: SEC-MED-001
#[test]
fn fails_an_mp3_file_whose_only_frame_is_cut_short() {
    let file = &stereo().write(&[])[..100];
    assert_eq!(
        run(file, Some("mp3")),
        Err(ProbeError::Mpeg(MpaError::Fault(ParseFault::Truncated {
            offset: 0,
            needed: 417,
            available: 100,
        })))
    );
}
