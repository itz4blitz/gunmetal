//! Hosts that answer a read with something other than the octets asked
//! for.

use gunmetal_testkit::ogg::{self, FIRST, LAST, Page};
use gunmetal_testkit::opus::{self, OpusHead};

use super::*;
use crate::catalog::{FileFacts, IdentityInputs, Trim};
use crate::formats::detect::Format;
use crate::formats::vorbis_comment::{Comments, Field};
use crate::parse::{ReadRequest, SansIo, Step, Window};
use crate::probe::{SeekIndex, TagBlock};

/// The most reads a probe of [`file`] makes here: one to detect it, one
/// for its header packets and one for its end, and one more when an
/// answer held only a part of its read.
const READS: usize = 4;

/// Each of those reads: the whole file.
const WHOLE: ReadRequest = ReadRequest {
    offset: 0,
    len: 146,
};

/// An Opus file of 146 octets on three pages: 47, 61 and 38 octets.
fn file() -> Vec<u8> {
    let page = |sequence, flags, granule, packet: &[u8]| {
        Page {
            flags,
            granule,
            serial: 7,
            sequence,
            lacing: ogg::lacing(packet.len()),
            body: packet.to_vec(),
        }
        .to_bytes()
    };
    [
        page(0, FIRST, 0, &OpusHead::stereo().to_bytes()),
        page(1, 0, 0, &opus::opus_tags(b"ref", &[b"TITLE=Song"])),
        page(2, LAST, 48_312, &[0; 10]),
    ]
    .concat()
}

/// Probes [`file`] as a host that answers read number `bad`, counting
/// from 0, with the window that starts at `offset` and holds `bytes`, and
/// every other read with the octets asked for. Returns the reads the
/// probe asked for and its answer.
///
/// A probe that asks for more than [`READS`] reads fails the test instead
/// of hanging it.
fn answered(bad: usize, (offset, bytes): (u64, &[u8])) -> (Vec<ReadRequest>, Outcome) {
    let file = file();
    let file_len = len(&file);
    let mut budget = Budget::for_input(0, 0, enough(&file));
    let mut probe = probe(Some("opus"), Limits::DEFAULT, &mut budget);
    let mut reads = Vec::new();
    let mut window = Window::start(file_len);
    loop {
        let request = match probe.resume(window) {
            Step::Done(outcome) => return (reads, outcome),
            Step::Need(request) => request,
        };
        assert!(reads.len() < READS, "the probe did not stop: {reads:?}");
        let start = usize::try_from(request.offset).unwrap();
        let end = start + usize::try_from(request.len).unwrap();
        window = if reads.len() == bad {
            Window {
                offset,
                bytes,
                file_len,
            }
        } else {
            Window {
                offset: request.offset,
                bytes: &file[start..end],
                file_len,
            }
        };
        reads.push(request);
    }
}

/// The answer of a probe that was given a window of `len` octets at
/// `offset` for a read of the whole file.
fn unanswered(offset: u64, len: u64) -> Outcome {
    Err(ProbeError::Unanswered {
        asked: WHOLE,
        offset,
        len,
    })
}

/// A file cut short after its length was taken answers a read with no
/// octets. Asking again would get no further, so the probe fails the file
/// at once and asks for nothing more, whether the read was for the header
/// packets or for the end of the file.
///
/// Verifies: SEC-MED-008
#[test]
fn fails_a_file_whose_read_is_answered_with_no_octets() {
    let none: &[u8] = &[];
    assert_eq!(
        answered(1, (0, none)),
        (vec![WHOLE, WHOLE], unanswered(0, 0))
    );
    assert_eq!(
        answered(2, (0, none)),
        (vec![WHOLE, WHOLE, WHOLE], unanswered(0, 0))
    );
}

/// A window that starts one octet late is not the octets asked for,
/// though it holds 145 of them. The probe takes none of it.
///
/// Verifies: SEC-MED-008
#[test]
fn fails_a_file_whose_read_is_answered_from_another_offset() {
    let file = file();
    let late: (u64, &[u8]) = (1, &file[1..]);
    assert_eq!(answered(1, late), (vec![WHOLE, WHOLE], unanswered(1, 145)));
    assert_eq!(
        answered(2, late),
        (vec![WHOLE, WHOLE, WHOLE], unanswered(1, 145))
    );
}

/// An answer that holds a part of its read is taken, and the probe asks
/// for the rest: here 100 of the 146 octets read for the header packets,
/// then the 46 that are left. The file is probed as it is when every read
/// is answered whole, but for the octets read: 146 to detect it, 146 and
/// 46 for its header packets and 146 for its end are 484.
#[test]
fn asks_for_the_rest_of_a_read_that_is_answered_in_part() {
    let file = file();
    let part: (u64, &[u8]) = (0, &file[..100]);
    let rest = ReadRequest {
        offset: 100,
        len: 46,
    };
    let probed = Probed {
        format: Format::Ogg,
        facts: FileFacts {
            tech: tech(
                Codec::Opus,
                Container::Ogg,
                (48_000, None, 2),
                Some(1_168),
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
                audio_window: range(0, 146),
            },
            parser_version: 1,
            bytes_read: 484,
        },
        tags: vec![TagBlock::Vorbis(Comments {
            vendor: text("ref"),
            fields: vec![Field {
                key: "TITLE".to_owned(),
                value: text("Song"),
            }],
            pictures: vec![],
            problems: vec![],
            end: 25,
        })],
        seek: SeekIndex::None,
        problems: vec![],
    };
    assert_eq!(
        answered(1, part),
        (vec![WHOLE, WHOLE, rest, WHOLE], Ok(probed))
    );
}

/// A window that holds more octets than the read asked for is no answer
/// to it either: the probe holds no octet that it did not ask for, and
/// that its own guard did not admit. Here one octet follows the 146 of
/// the file.
#[test]
fn fails_a_file_whose_read_is_answered_with_more_than_it_asked_for() {
    let mut long = file();
    long.push(0);
    let more: (u64, &[u8]) = (0, &long);
    assert_eq!(answered(1, more), (vec![WHOLE, WHOLE], unanswered(0, 147)));
    assert_eq!(
        answered(2, more),
        (vec![WHOLE, WHOLE, WHOLE], unanswered(0, 147))
    );
}
