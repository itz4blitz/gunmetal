//! What holds for every input: the probe returns, asks only for reads the
//! limits allow, and says how many octets it read.

use gunmetal_testkit::bytes::Bytes;
use gunmetal_testkit::flac::{self, Block, StreamInfo};
use gunmetal_testkit::id3v1::Id3v1;
use gunmetal_testkit::id3v2::{self, Encoding, Tag, Version};
use gunmetal_testkit::mp4::{
    Esds, SampleEntry, data, esds, ftyp, mdhd, mp4_box, sample_entry, stsd, trak, udta,
};
use gunmetal_testkit::mpa::{self, Frame, Mode};
use gunmetal_testkit::ogg::{self, FIRST, LAST, Page};
use gunmetal_testkit::opus::{self, OpusHead};
use gunmetal_testkit::riff;
use gunmetal_testkit::vorbis_comment::CommentBlock;
use proptest::collection::vec;
use proptest::prelude::*;

use super::*;
use crate::parse::{ReadGuard, SansIo, Step, Window};

/// The most resumes any test input here needs. A probe that never stopped
/// asking fails the test instead of hanging it.
const RESUMES: u32 = 100_000;

/// Every extension a file may be probed under here: none, one of each
/// kind on the allowlist, and one outside it.
const NAMES: [Option<&str>; 12] = [
    None,
    Some("flac"),
    Some("mp3"),
    Some("m4a"),
    Some("ogg"),
    Some("opus"),
    Some("wav"),
    Some("aiff"),
    Some("png"),
    Some("lrc"),
    Some("m3u"),
    Some("txt"),
];

/// Probes `file` as a host does, on a thread with the 256 KiB stack
/// SEC-MED-001 names, and returns what the probe answered with the octets
/// the host served.
///
/// The host admits each request through a guard of its own, as [`drive`]
/// does, and fails the test at a request the limits refuse or at a probe
/// that is still asking after [`RESUMES`] resumes.
fn host(file: Vec<u8>, ext: Option<&'static str>, limits: Limits, steps: u64) -> (Outcome, u64) {
    std::thread::Builder::new()
        .stack_size(262_144)
        .spawn(move || {
            let file_len = len(&file);
            let mut budget = Budget::for_input(0, 0, steps);
            let mut probe = probe(ext, limits, &mut budget);
            let mut guard = ReadGuard::new(file_len, &limits);
            let mut window = Window::start(file_len);
            let mut resumes = 0;
            loop {
                resumes += 1;
                assert!(resumes <= RESUMES, "the probe did not stop");
                match probe.resume(window) {
                    Step::Done(outcome) => return (outcome, guard.bytes_read()),
                    Step::Need(request) => {
                        assert_eq!(guard.admit(request), Ok(()), "a refused read");
                        let start = usize::try_from(request.offset).expect("inside the file");
                        let end = start + usize::try_from(request.len).expect("a read fits");
                        window = Window {
                            offset: request.offset,
                            bytes: &file[start..end],
                            file_len,
                        };
                    }
                }
            }
        })
        .expect("the test thread starts")
        .join()
        .expect("the probe kept to the limits and returned")
}

/// What must hold of what a host saw of a probe that returned: when it
/// found a file, it reports the octets it was served.
fn check((outcome, served): &(Outcome, u64)) -> Result<(), TestCaseError> {
    let read = outcome
        .as_ref()
        .map_or(*served, |probed| probed.facts.bytes_read);
    prop_assert_eq!(read, *served);
    Ok(())
}

/// A small valid file of each format, with tags.
fn samples() -> Vec<(Vec<u8>, &'static str)> {
    let tag = Tag::new(Version::V24)
        .frame(b"TIT2", 0, &id3v2::text(Encoding::Utf8, &["Hi"]))
        .frame(
            b"USLT",
            0,
            &id3v2::comment(Encoding::Utf8, *b"eng", "", "[00:01.00]la"),
        )
        .build();
    let mut comment = CommentBlock::new(b"ref");
    comment.field("LYRICS", "la la");
    let mut flac = tag.clone();
    flac.extend(flac::stream(&[
        Block::StreamInfo(StreamInfo {
            min_block_size: 4_096,
            max_block_size: 4_096,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 44_100,
            channels: 2,
            bits_per_sample: 16,
            total_samples: 88_200,
            md5: [7; 16],
        }),
        Block::VorbisComment(comment.build()),
    ]));
    flac.extend([0xFF; 20]);

    let frame = Frame::layer3(mpa::Version::Mpeg2, 1, 1, Mode::Mono);
    let mp3 = [
        tag.clone(),
        mpa::stream(&[frame, frame, frame]),
        gunmetal_testkit::ape::Ape::new()
            .text("Lyrics", "la")
            .build(),
        Id3v1::new().title(b"One").build(),
    ]
    .concat();

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
            specific: None,
            width: 1,
        }),
    });
    let movie = [
        trak(*b"soun", &mdhd(44_100, 88_200), &stsd(&[&entry])),
        udta(false, &mp4_box(*b"\xA9lyr", &data(1, b"la la"))),
    ]
    .concat();
    let mp4 = [
        ftyp(*b"M4A ", 0, &[*b"M4A ", *b"mp42"]),
        mp4_box(*b"mdat", &[0; 16]),
        mp4_box(*b"moov", &movie),
    ]
    .concat();

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
    let ogg = [
        page(0, FIRST, 0, &OpusHead::stereo().to_bytes()),
        page(1, 0, 0, &opus::opus_tags(b"ref", &[b"LYRICS=la"])),
        page(2, LAST, 48_312, &[0; 10]),
    ]
    .concat();

    let mut chunks = Bytes::new();
    chunks
        .riff_chunk(*b"fmt ", &riff::format(1, 1, 8_000, 8))
        .riff_chunk(*b"data", &[0x80; 16])
        .riff_chunk(*b"LIST", b"INFOINAM\x02\0\0\0A\0")
        .riff_chunk(*b"id3 ", &tag);
    let wav = riff::wave(chunks.as_slice());

    let mut chunks = Bytes::new();
    chunks
        .aiff_chunk(*b"COMM", &riff::comm(1, 16, 8, 8_000))
        .aiff_chunk(*b"SSND", &riff::ssnd(0, 0, &[0x80; 16]))
        .aiff_chunk(*b"ID3 ", &tag);
    let aiff = riff::aiff(*b"AIFF", chunks.as_slice());

    vec![
        (flac, "flac"),
        (mp3, "mp3"),
        (mp4, "m4a"),
        (ogg, "opus"),
        (wav, "wav"),
        (aiff, "aiff"),
    ]
}

/// The documented budget is enough for every sample, and each is probed
/// without a problem: no step runs out anywhere in it.
///
/// Verifies: SEC-MED-007
#[test]
fn the_documented_budget_is_enough_for_a_valid_file_of_each_format() {
    for (file, ext) in samples() {
        let (answer, _) = run_with(&file, Some(ext), Limits::DEFAULT, enough(&file));
        let problems = answer.unwrap().map(|probed| probed.problems);
        assert_eq!(problems, Ok(vec![]), "{ext}");
    }
}

/// A budget that runs out anywhere in a valid file ends the probe with a
/// typed error or a recorded problem, never with a refused read or a
/// hang.
///
/// Verifies: SEC-MED-007, SEC-MED-008
#[test]
fn returns_for_every_budget_a_valid_file_can_run_out_of() {
    for (file, ext) in samples() {
        for steps in (0..enough(&file)).step_by(7) {
            let (outcome, served) = host(file.clone(), Some(ext), Limits::DEFAULT, steps);
            let read = outcome.map_or(served, |probed| probed.facts.bytes_read);
            assert_eq!(read, served, "{ext} with {steps} steps");
        }
    }
}

/// The limits a probe may run under here: the defaults, and read and file
/// caps low enough for a small file to reach.
fn limits() -> impl Strategy<Value = Limits> {
    (512_u64..2_048, 1_u64..8_192).prop_map(|(read, total)| {
        Limits::DEFAULT
            .with_override(LimitKind::ReadBytes, read)
            .and_then(|limits| limits.with_override(LimitKind::FileBytes, total))
            .unwrap_or(Limits::DEFAULT)
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Verifies: SEC-MED-001, SEC-MED-008, SEC-MED-010
    #[test]
    fn returns_for_any_bytes_under_any_name(
        bytes in vec(any::<u8>(), 0..1_024),
        name in 0..NAMES.len(),
        limits in limits(),
        steps in 0_u64..20_000,
    ) {
        let ext = NAMES.get(name).copied().flatten();
        check(&host(bytes, ext, limits, steps))?;
    }

    /// A valid file with any one octet changed, cut anywhere, and followed
    /// by anything.
    ///
    /// Verifies: SEC-MED-001, SEC-MED-008, SEC-MED-010
    #[test]
    fn returns_for_any_damage_to_a_valid_file(
        sample in 0..6_usize,
        at in any::<prop::sample::Index>(),
        octet in any::<u8>(),
        cut in any::<prop::sample::Index>(),
        tail in vec(any::<u8>(), 0..64),
        limits in limits(),
        steps in 0_u64..20_000,
    ) {
        let (mut file, ext) = samples().swap_remove(sample);
        let changed = at.index(file.len());
        file[changed] = octet;
        file.truncate(cut.index(file.len()) + 1);
        file.extend(tail);
        check(&host(file, Some(ext), limits, steps))?;
    }

}

/// A valid file probed whole reports exactly the octets its host served,
/// and passes the check the properties make, on every run rather than
/// only on runs that happen to generate a file the probe reads.
///
/// Verifies: SEC-MED-010
#[test]
fn reports_the_octets_a_host_served_of_a_valid_file() {
    for (file, ext) in samples() {
        let steps = enough(&file);
        let seen = host(file, Some(ext), Limits::DEFAULT, steps);
        check(&seen).unwrap();
        let (outcome, served) = seen;
        assert_eq!(
            outcome.map(|probed| probed.facts.bytes_read),
            Ok(served),
            "{ext}"
        );
    }
}
