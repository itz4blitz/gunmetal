//! Structure-aware generator for MP4 audio (SEC-MED-031).
//!
//! Interprets the fuzzer's bytes as a choice of valid ISOBMFF framing with
//! mutated payloads in the sample-entry children, the item list, or the
//! media data, then feeds the file to the same probe as [`crate::mp4_probe`].
//! Random bytes rarely get past an `ftyp` box; generated trees do.

use crate::mp4_probe::{self, Outcome};

/// The most octets of mutated payload kept, so a generated file stays
/// inside one read of the default limits.
const MAX_PAYLOAD: usize = 4096;

/// Serialises `data` as an MP4 file with valid box headers around mutated
/// bodies.
#[must_use]
pub fn encode(data: &[u8]) -> Vec<u8> {
    let Some((&selector, rest)) = data.split_first() else {
        return Vec::new();
    };
    let rest = rest.split_at(rest.len().min(MAX_PAYLOAD)).0;
    match selector % 6 {
        0 => rest.to_vec(),
        1 => with_movie(rest, false, Size::Fixed),
        2 => with_movie(rest, true, Size::Fixed),
        3 => nested(rest),
        4 => with_movie(rest, false, Size::Open),
        _ => with_movie(rest, true, Size::Large),
    }
}

/// How the media-data box is sized.
#[derive(Clone, Copy)]
enum Size {
    /// A 32-bit size.
    Fixed,
    /// Size 0, running to the end of the file. Only used when `mdat` is last.
    Open,
    /// Size 1 with a 64-bit size after the type.
    Large,
}

/// A file with a movie and media data. `rest` is split between the sample
/// entry's children and the item list; the media data carries the same
/// octets so a `moov` after `mdat` still has something to skip.
fn with_movie(rest: &[u8], mdat_first: bool, size: Size) -> Vec<u8> {
    let mid = rest.len() / 2;
    let (entry, items) = rest.split_at(mid);
    let movie = box32(*b"moov", &[track(entry), user_data(items)].concat());
    let media = mdat(rest, size);
    let mut file = ftyp();
    if mdat_first {
        file.extend(media);
        file.extend(movie);
    } else {
        file.extend(movie);
        file.extend(media);
    }
    file
}

/// A movie with a valid audio track and `depth` nested boxes of `kind`
/// around a mutated free box, so the walk hits the depth limit while the
/// framing stays valid.
fn nested(rest: &[u8]) -> Vec<u8> {
    let (depth_kind, payload) = rest.split_first().map_or((0, rest), |(&b, rest)| (b, rest));
    let depth = u32::from(depth_kind >> 1).min(40);
    let kind = if depth_kind & 1 == 0 {
        *b"trak"
    } else {
        *b"udta"
    };
    let nested = (0..depth).fold(box32(*b"free", payload), |inner, _| box32(kind, &inner));
    let mut file = ftyp();
    file.extend(box32(*b"moov", &[track(&[]), nested].concat()));
    file
}

/// Feeds the generated file to the MP4 probe.
///
/// # Panics
///
/// Panics when the probe breaks an invariant, as [`mp4_probe::run`] does.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    mp4_probe::run(&encode(data))
}

fn ftyp() -> Vec<u8> {
    box32(*b"ftyp", b"M4A \x00\x00\x02\x00M4A mp42isom")
}

fn mdat(body: &[u8], size: Size) -> Vec<u8> {
    match size {
        Size::Fixed => box32(*b"mdat", body),
        Size::Open => box0(*b"mdat", body),
        Size::Large => box64(*b"mdat", body),
    }
}

fn track(children: &[u8]) -> Vec<u8> {
    let stbl = box32(*b"stbl", &stsd(children));
    let minf = box32(*b"minf", &stbl);
    let mdia = [mdhd(), hdlr(*b"soun"), minf].concat();
    box32(*b"trak", &box32(*b"mdia", &mdia))
}

fn user_data(items: &[u8]) -> Vec<u8> {
    let mut meta = Vec::from([0, 0, 0, 0]);
    meta.extend(hdlr(*b"mdir"));
    meta.extend(box32(*b"ilst", items));
    box32(*b"udta", &box32(*b"meta", &meta))
}

fn mdhd() -> Vec<u8> {
    let mut body = Vec::from([0, 0, 0, 0]);
    body.extend_from_slice(&[0; 8]);
    body.extend_from_slice(&44_100_u32.to_be_bytes());
    body.extend_from_slice(&441_000_u32.to_be_bytes());
    body.extend_from_slice(&[0x55, 0xC4, 0, 0]);
    box32(*b"mdhd", &body)
}

fn hdlr(handler: [u8; 4]) -> Vec<u8> {
    let mut body = Vec::from([0, 0, 0, 0]);
    body.extend_from_slice(&[0; 4]);
    body.extend_from_slice(&handler);
    body.extend_from_slice(&[0; 12]);
    body.push(0);
    box32(*b"hdlr", &body)
}

fn stsd(children: &[u8]) -> Vec<u8> {
    let mut body = Vec::from([0, 0, 0, 0]);
    body.extend_from_slice(&1_u32.to_be_bytes());
    body.extend(sample_entry(children));
    box32(*b"stsd", &body)
}

fn sample_entry(children: &[u8]) -> Vec<u8> {
    let mut body = Vec::from([0; 6]);
    body.extend_from_slice(&1_u16.to_be_bytes());
    body.extend_from_slice(&[0; 8]);
    body.extend_from_slice(&2_u16.to_be_bytes());
    body.extend_from_slice(&16_u16.to_be_bytes());
    body.extend_from_slice(&[0; 4]);
    body.extend_from_slice(&44_100_u16.to_be_bytes());
    body.extend_from_slice(&[0, 0]);
    body.extend_from_slice(children);
    box32(*b"mp4a", &body)
}

fn box32(kind: [u8; 4], body: &[u8]) -> Vec<u8> {
    let size = u32::try_from(8_usize.saturating_add(body.len())).unwrap_or(u32::MAX);
    let mut out = Vec::from(size.to_be_bytes());
    out.extend_from_slice(&kind);
    out.extend_from_slice(body);
    out
}

fn box64(kind: [u8; 4], body: &[u8]) -> Vec<u8> {
    let size = 16_u64.saturating_add(u64::try_from(body.len()).unwrap_or(u64::MAX));
    let mut out = Vec::from(1_u32.to_be_bytes());
    out.extend_from_slice(&kind);
    out.extend_from_slice(&size.to_be_bytes());
    out.extend_from_slice(body);
    out
}

fn box0(kind: [u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out = Vec::from(0_u32.to_be_bytes());
    out.extend_from_slice(&kind);
    out.extend_from_slice(body);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The file-type box every generated tree starts with.
    const FTYP: &[u8] = &[
        0, 0, 0, 28, b'f', b't', b'y', b'p', b'M', b'4', b'A', b' ', 0, 0, 2, 0, b'M', b'4', b'A',
        b' ', b'm', b'p', b'4', b'2', b'i', b's', b'o', b'm',
    ];

    /// Verifies: SEC-MED-031
    #[test]
    fn passthrough_copies_the_rest_up_to_the_payload_cap() {
        assert_eq!(encode(&[]), Vec::<u8>::new());
        assert_eq!(encode(&[0, 1, 2, 3]), vec![1, 2, 3]);
        assert_eq!(encode(&[6, 9, 8]), vec![9, 8]);
        let input: Vec<u8> = core::iter::once(0)
            .chain(core::iter::repeat_n(0xAA, 5_000))
            .collect();
        let wanted: Vec<u8> = core::iter::repeat_n(0xAA, MAX_PAYLOAD).collect();
        assert_eq!(encode(&input), wanted);
    }

    /// Verifies: SEC-MED-031
    #[test]
    fn movie_then_media_data_uses_a_32_bit_mdat_after_the_movie() {
        let file = encode(&[1]);
        assert_eq!(&file[..28], FTYP);
        assert_eq!(&file[28..32], &218_u32.to_be_bytes());
        assert_eq!(&file[32..36], b"moov");
        assert_eq!(&file[72..76], &44_100_u32.to_be_bytes());
        assert_eq!(&file[76..80], &441_000_u32.to_be_bytes());
        assert_eq!(
            &file[file.len() - 8..],
            &[0, 0, 0, 8, b'm', b'd', b'a', b't']
        );
        assert_eq!(file.len(), 254);
        assert_eq!(encode(&[7]), file);
    }

    /// Verifies: SEC-MED-031
    #[test]
    fn media_data_then_movie_puts_mdat_between_the_file_type_and_the_movie() {
        let file = encode(&[2]);
        assert_eq!(&file[..28], FTYP);
        assert_eq!(&file[28..36], &[0, 0, 0, 8, b'm', b'd', b'a', b't']);
        assert_eq!(&file[36..40], &218_u32.to_be_bytes());
        assert_eq!(&file[40..44], b"moov");
        assert_eq!(file.len(), 254);
        assert_eq!(encode(&[8]), file);
    }

    /// Verifies: SEC-MED-008, SEC-MED-031
    #[test]
    fn an_open_mdat_differs_from_a_32_bit_mdat_only_in_its_size_field() {
        let fixed = encode(&[1]);
        let open = encode(&[4]);
        assert_eq!(fixed.len(), open.len());
        assert_eq!(&fixed[..fixed.len() - 8], &open[..open.len() - 8]);
        assert_eq!(
            &open[open.len() - 8..],
            &[0, 0, 0, 0, b'm', b'd', b'a', b't']
        );
        assert_eq!(encode(&[10]), open);
    }

    /// Verifies: SEC-MED-008, SEC-MED-031
    #[test]
    fn a_64_bit_mdat_carries_size_one_and_a_sixteen_octet_header() {
        let file = encode(&[5]);
        assert_eq!(&file[..28], FTYP);
        assert_eq!(&file[28..36], &[0, 0, 0, 1, b'm', b'd', b'a', b't']);
        assert_eq!(&file[36..44], &16_u64.to_be_bytes());
        assert_eq!(&file[44..48], &218_u32.to_be_bytes());
        assert_eq!(&file[48..52], b"moov");
        assert_eq!(file.len(), 262);
        assert_eq!(encode(&[11]), file);
    }

    /// Verifies: SEC-MED-005, SEC-MED-031
    #[test]
    fn nested_boxes_use_trak_or_udta_and_cap_depth_at_40() {
        assert_eq!(&encode(&[3, 2])[189..193], b"trak");
        assert_eq!(&encode(&[3, 3])[189..193], b"udta");
        assert_eq!(encode(&[3, 0]).len(), 193);
        assert_eq!(encode(&[3, 80]).len(), 513);
        assert_eq!(encode(&[3, 200]).len(), 513);
        assert_eq!(encode(&[3, 78]).len(), 505);
    }
}
