//! Ogg files: Opus and Vorbis.

use gunmetal_testkit::ogg::{self, FIRST, LAST, Page};
use gunmetal_testkit::opus::{self, OpusHead, VorbisIdent};
use gunmetal_testkit::vorbis_comment::{Picture as BuiltPicture, base64};

use super::*;
use crate::catalog::{FileFacts, IdentityInputs, Trim};
use crate::formats::detect::Format;
use crate::formats::ogg::{PacketError, PageError};
use crate::formats::opus::OpusError;
use crate::formats::vorbis::VorbisError;
use crate::formats::vorbis_comment::{Comments, Field, Picture};
use crate::parse::ParseFault;
use crate::probe::{PartProblem, SeekIndex, TagBlock};

/// The serial number of the stream in every file here.
const SERIAL: u32 = 7;

/// A page of the stream that holds the one packet `packet`: 27 octets of
/// header, one lacing value for each 255 octets and one more, then the
/// packet.
fn page(sequence: u32, flags: u8, granule: u64, packet: &[u8]) -> Vec<u8> {
    Page {
        flags,
        granule,
        serial: SERIAL,
        sequence,
        lacing: ogg::lacing(packet.len()),
        body: packet.to_vec(),
    }
    .to_bytes()
}

/// An Opus comment header of 33 octets: the vendor `ref` and a title.
fn opus_tags() -> Vec<u8> {
    opus::opus_tags(b"ref", &[b"TITLE=Song"])
}

/// What the comment block of [`opus_tags`] holds: 25 octets.
fn title() -> Comments {
    Comments {
        vendor: text("ref"),
        fields: vec![Field {
            key: "TITLE".to_owned(),
            value: text("Song"),
        }],
        pictures: vec![],
        problems: vec![],
        end: 25,
    }
}

/// An Opus file whose pages hold the identification header (19 octets, at
/// 0..47), `tags` and `audio`, whose page ends at granule position 48,312:
/// one second after the 312 samples of pre-skip.
fn opus_file(tags: &[u8], audio: &[u8]) -> Vec<u8> {
    [
        page(0, FIRST, 0, &OpusHead::stereo().to_bytes()),
        page(1, 0, 0, tags),
        page(2, LAST, 48_312, audio),
    ]
    .concat()
}

/// A whole Opus file of 146 octets: pages at 0..47, 47..108 and 108..146.
fn whole() -> Vec<u8> {
    opus_file(&opus_tags(), &[0; 10])
}

/// What the probe finds in a one-second Opus file of `file_len` octets,
/// `bitrate` bits a second, after reading `bytes_read` octets.
fn opus_probed(file_len: u64, bitrate: u32, bytes_read: u64) -> Probed {
    Probed {
        format: Format::Ogg,
        facts: FileFacts {
            tech: tech(
                Codec::Opus,
                Container::Ogg,
                (48_000, None, 2),
                Some(bitrate),
                Some(1_000),
            ),
            trim: Some(Trim {
                delay: 312,
                padding: 0,
            }),
            artwork: vec![],
            lyrics: vec![],
            identity: IdentityInputs {
                audio_md5: None,
                audio_window: range(0, file_len),
            },
            parser_version: 1,
            bytes_read,
        },
        tags: vec![TagBlock::Vorbis(title())],
        seek: SeekIndex::None,
        problems: vec![],
    }
}

/// The file is read three times over: to detect it, for its headers, and
/// as its own end. 146 octets in one second are 1,168 bits a second.
///
/// Verifies: SEC-MED-010
#[test]
fn probes_an_opus_file() {
    assert_eq!(
        run(&whole(), Some("opus")),
        Ok(opus_probed(146, 1_168, 438))
    );
}

/// A Vorbis file of 156 octets: the identification header (30 octets) at
/// 0..58, the comment header (32) at 58..118, and a page that ends at
/// sample 88,200, two seconds in.
///
/// Verifies: SEC-MED-010
#[test]
fn probes_a_vorbis_file() {
    let file = [
        page(0, FIRST, 0, &VorbisIdent::stereo().to_bytes()),
        page(1, 0, 0, &opus::vorbis_comments(b"ref", &[b"LYRICS=la"], 1)),
        page(2, LAST, 88_200, &[0; 10]),
    ]
    .concat();
    assert_eq!(
        run(&file, Some("ogg")),
        Ok(Probed {
            format: Format::Ogg,
            facts: FileFacts {
                tech: tech(
                    Codec::Vorbis,
                    Container::Ogg,
                    (44_100, None, 2),
                    Some(624),
                    Some(2_000),
                ),
                trim: None,
                artwork: vec![],
                lyrics: vec![lyrics(LyricsOrigin::VorbisComment, LyricsTiming::Plain)],
                identity: IdentityInputs {
                    audio_md5: None,
                    audio_window: range(0, 156),
                },
                parser_version: 1,
                bytes_read: 468,
            },
            tags: vec![TagBlock::Vorbis(Comments {
                vendor: text("ref"),
                fields: vec![Field {
                    key: "LYRICS".to_owned(),
                    value: text("la"),
                }],
                pictures: vec![],
                problems: vec![],
                end: 24,
            })],
            seek: SeekIndex::None,
            problems: vec![],
        })
    );
}

/// The comment header is 9,023 octets, so its page ends at 9,133, past the
/// 8,192 octets read first. The read doubles and stops at the end of the
/// file, 9,171.
///
/// Detection reads 512 octets, the headers take the file and so does its
/// end: 18,854. 9,171 octets in one second are 73,368 bits a second.
#[test]
fn reads_on_until_it_holds_the_ogg_header_packets() {
    let value: String = (0..8_995).map(|_| 'x').collect();
    let note = format!("NOTE={value}");
    let file = opus_file(&opus::opus_tags(b"ref", &[note.as_bytes()]), &[0; 10]);
    let mut expected = opus_probed(9_171, 73_368, 18_854);
    expected.tags = vec![TagBlock::Vorbis(Comments {
        vendor: text("ref"),
        fields: vec![Field {
            key: "NOTE".to_owned(),
            value: text(&value),
        }],
        pictures: vec![],
        problems: vec![],
        end: 9_015,
    })];
    assert_eq!(run(&file, Some("opus")), Ok(expected));
}

/// The headers end at 108 and the page after them is 9,063 octets. The
/// first 8,192 octets hold the headers, so no more is read for them:
/// 512 + 8,192 + the 9,171 of the end.
///
/// Verifies: SEC-MED-010
#[test]
fn reads_no_further_once_it_holds_the_ogg_header_packets() {
    let file = opus_file(&opus_tags(), &zeros(9_000));
    assert_eq!(
        run(&file, Some("opus")),
        Ok(opus_probed(9_171, 73_368, 17_875))
    );
}

/// An Opus file of 9,171 octets with `head` as its identification header:
/// pages at 0..47, 47..9,133 and 9,133..9,171. The comment header is 9,023
/// octets and holds the one comment `NOTE`, of 8,995 letters.
fn long_tags(head: &OpusHead) -> Vec<u8> {
    let value: String = (0..8_995).map(|_| 'x').collect();
    let note = format!("NOTE={value}");
    [
        page(0, FIRST, 0, &head.to_bytes()),
        page(1, 0, 0, &opus::opus_tags(b"ref", &[note.as_bytes()])),
        page(2, LAST, 48_312, &[0; 10]),
    ]
    .concat()
}

/// What the comment header of [`long_tags`] holds: 9,015 octets.
fn note() -> Comments {
    let value: String = (0..8_995).map(|_| 'x').collect();
    Comments {
        vendor: text("ref"),
        fields: vec![Field {
            key: "NOTE".to_owned(),
            value: text(&value),
        }],
        pictures: vec![],
        problems: vec![],
        end: 9_015,
    }
}

/// What the probe finds in an Opus file of `file_len` octets when it
/// cannot tell how long the file plays, after reading `bytes_read`
/// octets, before its tags and its problems are filled in.
fn opus_unmeasured(file_len: u64, bytes_read: u64) -> Probed {
    Probed {
        format: Format::Ogg,
        facts: FileFacts {
            tech: tech(Codec::Opus, Container::Ogg, (48_000, None, 2), None, None),
            trim: Some(Trim {
                delay: 312,
                padding: 0,
            }),
            artwork: vec![],
            lyrics: vec![],
            identity: IdentityInputs {
                audio_md5: None,
                audio_window: range(0, file_len),
            },
            parser_version: 1,
            bytes_read,
        },
        tags: vec![],
        seek: SeekIndex::None,
        problems: vec![],
    }
}

/// The read for the header packets holds a tag block, so it grows no
/// further than a tag block may be held in memory. The comment header of
/// [`long_tags`] ends at 9,133. Under a limit of 9,133 octets the read
/// stops there and holds it: 512 octets to detect the file, those 9,133,
/// and the 9,171 of the file as its own end are 18,816.
///
/// Under a limit of 9,132 the header's page, which needs 9,086 octets
/// from 47, is one octet short. The file is kept without its comments,
/// and the page and the limit are both recorded.
///
/// Verifies: SEC-MED-006, SEC-MED-017, SEC-TM-032
#[test]
fn reads_for_the_ogg_header_packets_only_as_far_as_a_tag_may_be_held() {
    let file = long_tags(&OpusHead::stereo());
    let probe_under = |max| run_under(&file, Some("opus"), lowered(LimitKind::Id3v2TagBytes, max));
    let mut held = opus_probed(9_171, 73_368, 18_816);
    held.tags = vec![TagBlock::Vorbis(note())];
    assert_eq!(probe_under(9_133), Ok(held));
    let mut cut = opus_probed(9_171, 73_368, 18_815);
    cut.tags = vec![];
    cut.problems = vec![
        PartProblem::Ogg(PacketError::Page(PageError::Fault(ParseFault::Truncated {
            offset: 47,
            needed: 9_086,
            available: 9_085,
        }))),
        PartProblem::Fault(ParseFault::LimitExceeded {
            limit: LimitKind::Id3v2TagBytes,
            value: 9_171,
            max: 9_132,
            offset: 0,
        }),
    ];
    assert_eq!(probe_under(9_132), Ok(cut));
}

/// The first read is cut to the limit too. Under a limit of 100 octets
/// the 146 of [`whole`] are not all held: the comment header's page, which
/// needs 61 octets from 47, has 53. The file is read to detect it (146),
/// for its first 100 octets, and as its own end (146): 392.
///
/// Verifies: SEC-MED-006, SEC-MED-017, SEC-TM-032
#[test]
fn reads_the_start_of_an_ogg_file_only_as_far_as_a_tag_may_be_held() {
    let mut expected = opus_probed(146, 1_168, 392);
    expected.tags = vec![];
    expected.problems = vec![
        PartProblem::Ogg(PacketError::Page(PageError::Fault(ParseFault::Truncated {
            offset: 47,
            needed: 61,
            available: 53,
        }))),
        PartProblem::Fault(ParseFault::LimitExceeded {
            limit: LimitKind::Id3v2TagBytes,
            value: 146,
            max: 100,
            offset: 0,
        }),
    ];
    assert_eq!(
        run_under(
            &whole(),
            Some("opus"),
            lowered(LimitKind::Id3v2TagBytes, 100)
        ),
        Ok(expected)
    );
}

/// A read for more of the header packets that the limits refuse ends the
/// reading, and the file is kept with what the octets in hand hold: the
/// identification header, which playback needs. Detection reads 512
/// octets of [`long_tags`] and the first read for the headers 8,192:
/// 8,704. Under a file cap of 9,000 neither the 979 octets left of the
/// file nor, afterwards, its 9,171 octets as its own end may be read, so
/// the file has no comments and no duration, and what kept the probe from
/// each is recorded.
///
/// A file whose identification header is damaged fails, as it does when
/// no read is refused.
///
/// Verifies: SEC-MED-010, SEC-MED-017
#[test]
fn keeps_an_ogg_file_when_a_read_for_its_comment_header_is_refused() {
    let limits = lowered(LimitKind::FileBytes, 9_000);
    let refused = |offset, len| {
        PartProblem::Read(DriveError::OverFileCap {
            offset,
            len,
            read: 8_704,
            max: 9_000,
        })
    };
    let mut expected = opus_unmeasured(9_171, 8_704);
    expected.problems = vec![
        PartProblem::Ogg(PacketError::Page(PageError::Fault(ParseFault::Truncated {
            offset: 47,
            needed: 9_086,
            available: 8_145,
        }))),
        refused(8_192, 979),
        refused(0, 9_171),
    ];
    assert_eq!(
        run_under(&long_tags(&OpusHead::stereo()), Some("opus"), limits),
        Ok(expected)
    );
    let damaged = OpusHead {
        version: 16,
        ..OpusHead::stereo()
    };
    assert_eq!(
        run_under(&long_tags(&damaged), Some("opus"), limits),
        Err(ProbeError::Opus(OpusError::Version {
            offset: 8,
            version: 16,
        }))
    );
}

/// The stream's two header pages end at 108, and 70,000 octets that are
/// no page follow them. The last 65,536 octets of the file, from 4,572,
/// hold no page of the stream, so how long the file plays is not known,
/// and that is recorded.
///
/// Detection reads 512 octets, the headers take the first 8,192 and the
/// end 65,536: 74,240.
///
/// Verifies: SEC-MED-017
#[test]
fn records_an_ogg_file_whose_end_holds_no_page_of_its_stream() {
    let file = [
        page(0, FIRST, 0, &OpusHead::stereo().to_bytes()),
        page(1, 0, 0, &opus_tags()),
        zeros(70_000),
    ]
    .concat();
    let mut expected = opus_unmeasured(70_108, 74_240);
    expected.tags = vec![TagBlock::Vorbis(title())];
    expected.problems = vec![PartProblem::NoLastPage {
        offset: 4_572,
        serial: SERIAL,
    }];
    assert_eq!(run(&file, Some("opus")), Ok(expected));
}

/// A picture in a comment is found among the file's artwork. The picture
/// is 45 octets, 41 of fields and four of data, so its base64 value is 60
/// octets. In the block, the vendor and the count take 11 octets, the
/// comment's length four and its name and `=` 23, so the value is at
/// 38..98.
#[test]
fn finds_a_picture_in_an_ogg_comment_header() {
    let picture = BuiltPicture {
        kind: 4,
        mime: "image/png".to_owned(),
        description: String::new(),
        width: 1,
        height: 2,
        depth: 24,
        colours: 0,
        data: vec![0x89, b'P', b'N', b'G'],
    };
    let comment = format!("METADATA_BLOCK_PICTURE={}", base64(&picture.build()));
    let file = opus_file(&opus::opus_tags(b"ref", &[comment.as_bytes()]), &[0; 10]);
    let probed = run(&file, Some("opus")).unwrap();
    assert_eq!(
        probed.tags,
        [TagBlock::Vorbis(Comments {
            vendor: text("ref"),
            fields: vec![],
            pictures: vec![Picture {
                offset: 38,
                len: 60,
                kind: 4,
                mime: text("image/png"),
                description: text(""),
                width: 1,
                height: 2,
                depth: 24,
                colours: 0,
                data_len: 4,
            }],
            problems: vec![],
            end: 98,
        })]
    );
    assert_eq!(
        probed.facts.artwork,
        [artwork(0, PictureType::BackCover, 4)]
    );
    assert_eq!(probed.problems, []);
}

/// What a probe says about the comment header of `file`: its tags, its
/// problems and how long it plays.
fn comments_of(file: &[u8]) -> (Vec<TagBlock>, Vec<PartProblem>, Option<Duration>) {
    let probed = run(file, Some("ogg")).unwrap();
    (
        probed.tags,
        probed.problems,
        probed.facts.tech.format().duration,
    )
}

fn second() -> Option<Duration> {
    Duration::from_millis(1_000).ok()
}

/// Verifies: SEC-MED-017
#[test]
fn keeps_an_ogg_file_whose_comment_header_cannot_be_read() {
    // An Opus stream whose second packet is not a comment header.
    assert_eq!(
        comments_of(&opus_file(b"NotTags!", &[0; 10])),
        (
            vec![],
            vec![PartProblem::Opus(OpusError::Magic {
                offset: 0,
                found: *b"NotTags!",
            })],
            second()
        )
    );
    // A Vorbis stream whose comment header lacks its framing bit, which
    // comes after the 7 octets of signature and the 11 of the block.
    let vorbis = [
        page(0, FIRST, 0, &VorbisIdent::stereo().to_bytes()),
        page(1, LAST, 44_100, &opus::vorbis_comments(b"ref", &[], 0)),
    ]
    .concat();
    assert_eq!(
        comments_of(&vorbis),
        (
            vec![],
            vec![PartProblem::Vorbis(VorbisError::Framing {
                offset: 18,
                octet: 0,
            })],
            second()
        )
    );
}

/// Verifies: SEC-MED-017
#[test]
fn keeps_an_ogg_file_with_no_comment_header() {
    let head = page(0, FIRST | LAST, 48_312, &OpusHead::stereo().to_bytes());
    // The stream ends after its identification header.
    assert_eq!(comments_of(&head), (vec![], vec![], second()));
    // Four octets that are no page follow it. They are what kept the
    // probe from a second packet, so they are recorded.
    assert_eq!(
        comments_of(&[head, b"junk".to_vec()].concat()),
        (
            vec![],
            vec![PartProblem::Ogg(PacketError::Page(PageError::Skipped {
                offset: 47,
                len: 4,
            }))],
            second()
        )
    );
}

/// The first packet names the codec, so a damaged one fails the file. The
/// offsets count from the start of the packet.
///
/// Verifies: SEC-MED-001
#[test]
fn fails_an_ogg_file_whose_first_packet_is_no_header_it_reads() {
    let opus = OpusHead {
        version: 16,
        ..OpusHead::stereo()
    };
    assert_eq!(
        run(&page(0, FIRST, 0, &opus.to_bytes()), Some("opus")),
        Err(ProbeError::Opus(OpusError::Version {
            offset: 8,
            version: 16,
        }))
    );
    // An Ogg FLAC stream is neither Opus nor Vorbis.
    assert_eq!(
        run(&page(0, FIRST, 0, b"\x7FFLAC\x01\x00\x00\x01"), Some("oga")),
        Err(ProbeError::Vorbis(VorbisError::Magic {
            offset: 0,
            found: *b"\x7FFLAC\x01\x00",
        }))
    );
}

/// Verifies: SEC-MED-001, SEC-MED-008
#[test]
fn fails_an_ogg_file_with_no_packet() {
    // The capture pattern and version that detection needs, and no page.
    assert_eq!(run(b"OggS\0", Some("ogg")), Err(ProbeError::Ogg(None)));
    // A page whose one packet goes on to a page that is not there: its
    // lacing value of 255 does not end the packet.
    let unfinished = Page {
        flags: FIRST,
        granule: 0,
        serial: SERIAL,
        sequence: 0,
        lacing: vec![255],
        body: zeros(255),
    }
    .to_bytes();
    assert_eq!(
        run(&unfinished, Some("ogg")),
        Err(ProbeError::Ogg(Some(PacketError::Unfinished { offset: 0 })))
    );
}

/// Reading the headers of [`whole`] costs 184 steps: one for the attempt,
/// 47 to find the first page, 108 for the pages of the two packets, one
/// for the identification header, two for the framing of the comment
/// header and 25 for its block. Finding the last page costs the 146 octets
/// of the three pages, of which the last is 38.
///
/// Verifies: SEC-MED-007, SEC-MED-017
#[test]
fn stops_looking_for_the_last_ogg_page_when_the_steps_run_out() {
    let duration_with = |steps| {
        let probed = run_with(&whole(), Some("opus"), Limits::DEFAULT, steps)
            .0
            .unwrap()
            .unwrap();
        (probed.facts.tech.format().duration, probed.problems)
    };
    assert_eq!(
        duration_with(329),
        (
            None,
            vec![PartProblem::Fault(ParseFault::BudgetExceeded {
                offset: 108
            })]
        )
    );
    assert_eq!(duration_with(330), (second(), vec![]));
}

/// Each attempt to read the header packets costs a step, so with no steps
/// there is none. The fault says how far the read had got.
///
/// Verifies: SEC-MED-007
#[test]
fn fails_an_ogg_file_with_no_step_to_read_its_headers() {
    assert_eq!(
        run_with(&whole(), Some("opus"), Limits::DEFAULT, 0).0,
        Ok(Err(ProbeError::Fault(ParseFault::BudgetExceeded {
            offset: 146
        })))
    );
}
