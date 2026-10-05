//! The read limits and the step budget, across formats.

use gunmetal_testkit::bytes::Bytes;
use gunmetal_testkit::mp4::{
    Esds, SampleEntry, esds, ftyp, mdhd, mp4_box, sample_entry, stsd, trak,
};
use gunmetal_testkit::mpa::{self, Frame, Mode};
use gunmetal_testkit::ogg::{self, FIRST, LAST, Page};
use gunmetal_testkit::opus::{self, OpusHead};
use gunmetal_testkit::riff;

use super::*;
use crate::formats::detect::Format;
use crate::parse::ParseFault;
use crate::probe::PartProblem;

/// Three MPEG-2 Layer III mono frames of 24 octets each.
fn mp3() -> Vec<u8> {
    let frame = Frame::layer3(mpa::Version::Mpeg2, 1, 1, Mode::Mono);
    mpa::stream(&[frame, frame, frame])
}

/// An MP4 file of 216 octets: `ftyp` (24), and `moov` (192) holding one
/// AAC track whose descriptor has no specific information.
fn mp4() -> Vec<u8> {
    let config = esds(&Esds {
        object_type: 0x40,
        max_bitrate: 0,
        avg_bitrate: 0,
        specific: None,
        width: 1,
    });
    let entry = sample_entry(&SampleEntry {
        format: *b"mp4a",
        version: 0,
        channels: 2,
        bits: 16,
        rate: 44_100,
        children: &config,
    });
    let track = trak(*b"soun", &mdhd(44_100, 88_200), &stsd(&[&entry]));
    [
        ftyp(*b"M4A ", 0, &[*b"M4A ", *b"mp42"]),
        mp4_box(*b"moov", &track),
    ]
    .concat()
}

/// A WAV file of 60 octets: a PCM format and 16 octets of samples.
fn wav() -> Vec<u8> {
    let mut chunks = Bytes::new();
    chunks
        .riff_chunk(*b"fmt ", &riff::format(1, 1, 8_000, 8))
        .riff_chunk(*b"data", &[0x80; 16]);
    riff::wave(chunks.as_slice())
}

/// An AIFF file of 70 octets: a format and 16 octets of samples.
fn aiff() -> Vec<u8> {
    let mut chunks = Bytes::new();
    chunks
        .aiff_chunk(*b"COMM", &riff::comm(1, 16, 8, 8_000))
        .aiff_chunk(*b"SSND", &riff::ssnd(0, 0, &[0x80; 16]));
    riff::aiff(*b"AIFF", chunks.as_slice())
}

/// An Opus file of 146 octets on three pages: 47, 61 and 38 octets.
fn ogg() -> Vec<u8> {
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

/// A container parser that takes a budget of its own is given the
/// allowance it documents for a file of this length, out of the caller's
/// steps: one step an octet and 8 more for MP4, one an octet for WAV and
/// AIFF. One step fewer is not enough to start it. None of these files
/// has a tag to read, so the allowance alone is enough for the file.
///
/// Verifies: SEC-MED-007
#[test]
fn gives_each_container_parser_its_documented_allowance() {
    let cases = [
        (mp4(), "m4a", Format::Mp4, 224),
        (wav(), "wav", Format::Wav, 60),
        (aiff(), "aiff", Format::Aiff, 70),
    ];
    for (file, ext, format, allowance) in cases {
        let probe_with = |steps| {
            let (answer, left) = run_with(&file, Some(ext), Limits::DEFAULT, steps);
            (answer.unwrap().map(|probed| probed.format), left)
        };
        assert_eq!(
            probe_with(allowance - 1),
            (
                Err(ProbeError::Fault(ParseFault::BudgetExceeded { offset: 0 })),
                0
            ),
            "{ext}"
        );
        assert_eq!(probe_with(allowance), (Ok(format), 0), "{ext}");
    }
}

/// Detection asks for up to 512 octets at once, whatever the read limit
/// is. Under a limit of 60 the probe does not ask a host for the 72
/// octets of this file: it fails with the read it would have asked for.
///
/// Verifies: SEC-MED-010
#[test]
fn fails_rather_than_ask_for_a_read_longer_than_the_limit() {
    let limits = lowered(LimitKind::ReadBytes, 60);
    assert_eq!(
        run_with(&mp3(), Some("mp3"), limits, 1_000).0,
        Ok(Err(ProbeError::Read(DriveError::TooLong {
            offset: 0,
            len: 72,
            max: 60,
        })))
    );
}

/// Detection reads the 72 octets of the file, and the stream parser
/// starts by asking for them again, which a cap of 100 does not allow.
///
/// Verifies: SEC-MED-010
#[test]
fn fails_rather_than_read_past_the_file_cap_for_what_playback_needs() {
    let limits = lowered(LimitKind::FileBytes, 100);
    assert_eq!(
        run_with(&mp3(), Some("mp3"), limits, 1_000).0,
        Ok(Err(ProbeError::Read(DriveError::OverFileCap {
            offset: 0,
            len: 72,
            read: 72,
            max: 100,
        })))
    );
    // The headers of an Ogg file are needed too.
    let limits = lowered(LimitKind::FileBytes, 200);
    assert_eq!(
        run_with(&ogg(), Some("opus"), limits, 10_000).0,
        Ok(Err(ProbeError::Read(DriveError::OverFileCap {
            offset: 0,
            len: 146,
            read: 146,
            max: 200,
        })))
    );
}

/// Detection and the headers take 292 octets of this Ogg file. The read of
/// its end would pass a cap of 300, so it is not asked for: the file is
/// kept, without a duration, and the read is recorded. At a cap of 438 the
/// end is read.
///
/// Verifies: SEC-MED-010, SEC-MED-017
#[test]
fn skips_a_part_whose_read_would_pass_the_file_cap() {
    let probe_under = |max| {
        let probed = run_under(&ogg(), Some("opus"), lowered(LimitKind::FileBytes, max)).unwrap();
        (
            probed.facts.tech.format().duration,
            probed.facts.bytes_read,
            probed.problems,
        )
    };
    assert_eq!(
        probe_under(437),
        (
            None,
            292,
            vec![PartProblem::Read(DriveError::OverFileCap {
                offset: 0,
                len: 146,
                read: 292,
                max: 437,
            })]
        )
    );
    assert_eq!(
        probe_under(438),
        (Duration::from_millis(1_000).ok(), 438, vec![])
    );
}

/// With reads of at most 512 octets, the 8,192 octets of an Ogg file's
/// start and the 9,171 of its end arrive in pieces, and the probe finds
/// what it finds when they arrive whole.
///
/// Verifies: SEC-MED-010
#[test]
fn gathers_a_part_longer_than_one_read_from_several() {
    let mut file = ogg();
    file.truncate(108);
    file.extend(
        Page {
            flags: LAST,
            granule: 48_312,
            serial: 7,
            sequence: 2,
            lacing: ogg::lacing(9_000),
            body: zeros(9_000),
        }
        .to_bytes(),
    );
    let whole = run(&file, Some("opus")).unwrap();
    assert_eq!(whole.facts.bytes_read, 17_875);
    assert_eq!(
        whole.facts.tech.format().duration,
        Duration::from_millis(1_000).ok()
    );
    assert_eq!(
        run_under(&file, Some("opus"), lowered(LimitKind::ReadBytes, 512)),
        Ok(whole)
    );
}
