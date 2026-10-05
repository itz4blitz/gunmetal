//! MP4 files.

use gunmetal_testkit::mp4::{
    Esds, SampleEntry, audio_specific_config, data, esds, ftyp, mdhd, mp4_box, sample_entry, stsd,
    trak, udta,
};

use super::*;
use crate::catalog::{FileFacts, IdentityInputs};
use crate::formats::detect::Format;
use crate::formats::mp4::{
    FourCc, IlstItem, ItemKey, ItemValue, Mp4Error, PictureRef, SampleTableRanges,
};
use crate::parse::ParseFault;
use crate::probe::{SeekIndex, TagBlock};

/// The file type box of an M4A file: 24 octets.
fn m4a() -> Vec<u8> {
    ftyp(*b"M4A ", 0, &[*b"M4A ", *b"mp42"])
}

/// A sound track of 188 octets: 88,200 units at 44,100 a second, and an
/// `mp4a` sample entry that says 44.1 kHz stereo and holds `config`.
fn track(timescale: u32, config: &[u8]) -> Vec<u8> {
    let entry = sample_entry(&SampleEntry {
        format: *b"mp4a",
        version: 0,
        channels: 2,
        bits: 16,
        rate: 44_100,
        children: config,
    });
    trak(*b"soun", &mdhd(timescale, 88_200), &stsd(&[&entry]))
}

/// An elementary stream descriptor of 39 octets: MPEG-4 audio at 128,000
/// bits a second, with an `AudioSpecificConfig` for AAC LC at 48 kHz.
fn aac() -> Vec<u8> {
    esds(&Esds {
        object_type: 0x40,
        max_bitrate: 0,
        avg_bitrate: 128_000,
        specific: Some(&audio_specific_config(2, 3, 2)),
        width: 1,
    })
}

/// A whole file with its movie after its media data, laid out as:
///
/// | Octets | What |
/// |---|---|
/// | 0..24 | `ftyp` |
/// | 24..48 | `mdat` |
/// | 48..390 | `moov` |
/// | 56..244 | `trak`, holding `mdia` at 64 |
/// | 72..137 | `mdhd` (32 octets) and `hdlr` (33) |
/// | 137..244 | `minf`, `stbl` at 145 and `stsd` at 153 |
/// | 244..390 | `udta`, `meta` at 252, its `hdlr` at 264, `ilst` at 297 |
/// | 305..333 | `©nam`, whose `data` box is at 313 |
/// | 333..361 | `covr`, whose `data` box is at 341 and picture at 357..361 |
/// | 361..390 | `©lyr`, whose `data` box is at 369 |
fn whole() -> Vec<u8> {
    let items = [
        mp4_box(*b"\xA9nam", &data(1, b"Song")),
        mp4_box(*b"covr", &data(13, &[0xFF, 0xD8, 0xFF, 0xE0])),
        mp4_box(*b"\xA9lyr", &data(1, b"la la")),
    ]
    .concat();
    let movie = [track(44_100, &aac()), udta(false, &items)].concat();
    [
        m4a(),
        mp4_box(*b"mdat", &[0; 16]),
        mp4_box(*b"moov", &movie),
    ]
    .concat()
}

/// Detection reads the 390 octets of the file. The probe then asks for a
/// header of up to 32 octets at each of 20 boxes: 32 at sixteen of them,
/// and what is left of the parent at the three `data` boxes (20, 20 and
/// 21) and at `©lyr` (29): 602. It asks for the bodies of `ftyp` (16),
/// `mdhd` (24), the track's `hdlr` (25) and `stsd` (83), the first eight
/// octets of `meta`, the bodies of the two text `data` boxes (12 and 13),
/// and the eight octets before the picture: 189. It never reads `mdat` or
/// the picture: 1,181 in all.
///
/// The codec configuration says 48 kHz, which comes before the 44.1 kHz of
/// the sample entry; the entry gives the channels.
///
/// Verifies: SEC-MED-010
#[test]
fn probes_an_mp4_file_whose_movie_is_at_the_end() {
    assert_eq!(
        run(&whole(), Some("m4a")),
        Ok(Probed {
            format: Format::Mp4,
            facts: FileFacts {
                tech: tech(
                    Codec::Aac,
                    Container::Mp4,
                    (48_000, None, 2),
                    Some(128_000),
                    Some(2_000),
                ),
                trim: None,
                artwork: vec![artwork(0, PictureType::FrontCover, 4)],
                lyrics: vec![lyrics(LyricsOrigin::Mp4Item, LyricsTiming::Plain)],
                identity: IdentityInputs {
                    audio_md5: None,
                    audio_window: range(0, 390),
                },
                parser_version: 1,
                bytes_read: 1_181,
            },
            tags: vec![TagBlock::Mp4(vec![
                IlstItem {
                    key: ItemKey::Atom(FourCc(*b"\xA9nam")),
                    values: vec![ItemValue::Text(text("Song"))],
                },
                IlstItem {
                    key: ItemKey::Atom(FourCc(*b"covr")),
                    values: vec![ItemValue::Picture(PictureRef {
                        type_code: 13,
                        offset: 357,
                        len: 4,
                    })],
                },
                IlstItem {
                    key: ItemKey::Atom(FourCc(*b"\xA9lyr")),
                    values: vec![ItemValue::Text(text("la la"))],
                },
            ])],
            seek: SeekIndex::Mp4(SampleTableRanges::default()),
            problems: vec![],
        })
    );
}

/// An `mp4a` entry with no codec configuration names no codec that is
/// read. The error says where the track starts: after `ftyp` and the
/// header of `moov`.
#[test]
fn fails_an_mp4_file_whose_codec_is_not_read() {
    let file = [m4a(), mp4_box(*b"moov", &track(44_100, &[]))].concat();
    assert_eq!(
        run(&file, Some("m4a")),
        Err(ProbeError::Unsupported {
            format: Format::Mp4,
            offset: 32,
        })
    );
}

/// A timescale of zero cannot time anything. The media header is at 48
/// and its timescale 20 octets in.
///
/// Verifies: SEC-MED-001
#[test]
fn fails_an_mp4_file_whose_media_header_is_damaged() {
    let file = [m4a(), mp4_box(*b"moov", &track(0, &aac()))].concat();
    assert_eq!(
        run(&file, Some("m4a")),
        Err(ProbeError::Mp4(Mp4Error::ZeroTimescale { offset: 68 }))
    );
}

/// `levels` `trak` boxes, one inside the other, around an empty `free`
/// box.
fn nested(levels: usize) -> Vec<u8> {
    (0..levels).fold(mp4_box(*b"free", &[]), |inner, _| mp4_box(*b"trak", &inner))
}

/// The movie is level 1. Thirty nested tracks reach level 31 and the box
/// inside them level 32, which is the limit. With 31, that box is at level
/// 33: after `ftyp` (24), the movie's header (8), the audio track (188)
/// and 31 headers.
///
/// Verifies: SEC-MED-005
#[test]
fn fails_an_mp4_file_nested_deeper_than_the_limit() {
    let file = |levels| {
        let movie = [track(44_100, &aac()), nested(levels)].concat();
        [m4a(), mp4_box(*b"moov", &movie)].concat()
    };
    assert_eq!(
        run(&file(30), Some("m4a")).map(|probed| (probed.facts.tech, probed.problems)),
        Ok((
            tech(
                Codec::Aac,
                Container::Mp4,
                (48_000, None, 2),
                Some(128_000),
                Some(2_000),
            ),
            vec![]
        ))
    );
    assert_eq!(
        run(&file(31), Some("m4a")),
        Err(ProbeError::Mp4(Mp4Error::Fault(ParseFault::TooDeep {
            limit: LimitKind::ContainerDepth,
            depth: 33,
            max: 32,
            offset: 468,
        })))
    );
}
