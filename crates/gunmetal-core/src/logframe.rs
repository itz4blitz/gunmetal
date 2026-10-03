//! Framing for user-log segment files (ADR 3).
//!
//! A segment is a sequence of records. Each record is a little-endian
//! payload length, a CRC-32C of the version and payload, the record format
//! version, and the payload:
//!
//! ```text
//! length:  u32 LE   payload octets, at most [`MAX_PAYLOAD`]
//! crc32c:  u32 LE   CRC-32C (Castagnoli) of `version || payload`
//! version: u8       [`RECORD_VERSION`] on write; any value on read
//! payload: [u8; length]
//! ```
//!
//! The first record of a segment is the header: a 16-octet stream identity,
//! a little-endian year and a month 1–12. Later records carry whatever
//! envelope WP-034 encodes. This module does not interpret payloads.
//!
//! A torn tail is cut back to the last whole record by [`recover_tail`].
//! Damage elsewhere is reported as [`Item::Damaged`] with the byte range
//! skipped, and the iterator resynchronises at the next whole record.
//! Records do not nest: the reader walks the file once and never recurses.

use std::ops::Range;

use crate::parse::Cursor;

/// The largest payload [`encode`] will write and [`records`] will accept.
///
/// One mebibyte holds a document snapshot of a large playlist. A length
/// field that claims more is damage, not a reason to skip gigabytes.
pub const MAX_PAYLOAD: u32 = 1_048_576;

/// The record format version [`encode`] writes.
pub const RECORD_VERSION: u8 = 1;

/// CRC-32C initial value and final XOR (CRC-32/ISCSI in the `RevEng` catalogue).
const CRC_INIT: u32 = 0xFFFF_FFFF;

/// The reflected Castagnoli polynomial 0x1EDC6F41.
const CRC_POLY: u32 = 0x82F6_3B78;

/// Why a payload could not be framed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodeError {
    /// The payload is longer than [`MAX_PAYLOAD`].
    TooLong {
        /// Octets the caller offered.
        len: u64,
        /// The largest payload the framer will write.
        max: u32,
    },
    /// A header month outside 1–12.
    BadMonth {
        /// The month that was refused.
        month: u8,
    },
}

/// Why a segment did not start with a header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderError {
    /// The segment held no record.
    Missing,
    /// The first record was damaged.
    Damaged {
        /// The skipped range in the segment.
        range: Range<usize>,
    },
    /// The first record's payload was not 19 octets.
    InvalidLength {
        /// Octets the payload actually held.
        len: usize,
    },
    /// The month octet was not 1–12.
    BadMonth {
        /// The month that was refused.
        month: u8,
    },
}

/// The identifying first record of a segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentHeader {
    /// The stream this segment belongs to, 16 opaque octets.
    pub stream: [u8; 16],
    /// The UTC year of the segment's month, as the writer stamped it.
    pub year: u16,
    /// The UTC month, 1 through 12.
    pub month: u8,
}

/// One whole record in a segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Record<'a> {
    /// The record format version the writer stamped.
    pub version: u8,
    /// The payload, borrowed from the segment.
    pub payload: &'a [u8],
}

/// One item [`records`] yields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item<'a> {
    /// A whole record whose CRC-32C matched.
    Record(Record<'a>),
    /// Bytes that were not a whole record, from the first bad octet to the
    /// next whole record or the end of the segment.
    Damaged {
        /// The skipped range in the segment.
        range: Range<usize>,
    },
}

/// An iterator of [`Item`]s over a segment.
#[derive(Debug)]
pub struct Records<'a> {
    /// The whole segment.
    segment: &'a [u8],
    /// The next octet to look at.
    pos: usize,
}

/// Frames `payload` as one record of version [`RECORD_VERSION`].
///
/// # Errors
///
/// [`EncodeError::TooLong`] when `payload` is longer than [`MAX_PAYLOAD`].
pub fn encode(payload: &[u8]) -> Result<Vec<u8>, EncodeError> {
    let Some(len) = u32::try_from(payload.len())
        .ok()
        .filter(|&len| len <= MAX_PAYLOAD)
    else {
        return Err(EncodeError::TooLong {
            len: u64::try_from(payload.len()).unwrap_or(u64::MAX),
            max: MAX_PAYLOAD,
        });
    };
    let crc = crc32c_frame(RECORD_VERSION, payload);
    let mut out = Vec::new();
    out.extend(len.to_le_bytes());
    out.extend(crc.to_le_bytes());
    out.push(RECORD_VERSION);
    out.extend(payload);
    Ok(out)
}

/// Frames a segment header as the first record of a new segment.
///
/// # Errors
///
/// [`EncodeError::BadMonth`] when `header.month` is not 1–12.
pub fn encode_header(header: &SegmentHeader) -> Result<Vec<u8>, EncodeError> {
    if !is_month(header.month) {
        return Err(EncodeError::BadMonth {
            month: header.month,
        });
    }
    let mut payload = Vec::new();
    payload.extend(header.stream);
    payload.extend(header.year.to_le_bytes());
    payload.push(header.month);
    encode(&payload)
}

/// Reads the header record at the start of `segment`.
///
/// # Errors
///
/// [`HeaderError::Missing`] when the segment is empty,
/// [`HeaderError::Damaged`] when the first record is not whole,
/// [`HeaderError::InvalidLength`] when its payload is not 19 octets, and
/// [`HeaderError::BadMonth`] when the month is not 1–12.
pub fn header(segment: &[u8]) -> Result<SegmentHeader, HeaderError> {
    match records(segment).next() {
        Some(Item::Record(record)) => parse_header_payload(record.payload),
        Some(Item::Damaged { range }) => Err(HeaderError::Damaged { range }),
        None => Err(HeaderError::Missing),
    }
}

/// Walks `segment`, yielding each whole record or a damaged range.
#[must_use]
pub fn records(segment: &[u8]) -> Records<'_> {
    Records { segment, pos: 0 }
}

/// The length of the longest prefix of `segment` that is only whole records.
///
/// A torn write at the end is the octets after this length. Damage in the
/// middle is not repaired: this stops at the first octet that is not the
/// start of a whole record.
#[must_use]
pub fn recover_tail(segment: &[u8]) -> usize {
    let mut pos = 0;
    while let Some((_, end)) = record_at(segment, pos) {
        pos = end;
    }
    pos
}

impl<'a> Iterator for Records<'a> {
    type Item = Item<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let pos = self.pos;
        if pos >= self.segment.len() {
            return None;
        }
        if let Some((record, end)) = record_at(self.segment, pos) {
            self.pos = end;
            return Some(Item::Record(record));
        }
        for scan in pos.saturating_add(1)..self.segment.len() {
            if record_at(self.segment, scan).is_some() {
                self.pos = scan;
                return Some(Item::Damaged { range: pos..scan });
            }
        }
        self.pos = self.segment.len();
        Some(Item::Damaged {
            range: pos..self.segment.len(),
        })
    }
}

/// A whole record starting at `pos`, and the octet after it.
fn record_at(segment: &[u8], pos: usize) -> Option<(Record<'_>, usize)> {
    let rest = segment.get(pos..)?;
    let mut cursor = Cursor::new(rest);
    let len = cursor.u32_le().ok()?;
    if len > MAX_PAYLOAD {
        return None;
    }
    let crc = cursor.u32_le().ok()?;
    let version = cursor.u8().ok()?;
    let payload = cursor.take(u64::from(len)).ok()?;
    if crc32c_frame(version, payload) != crc {
        return None;
    }
    let consumed = rest.len().saturating_sub(cursor.rest().len());
    let end = pos.saturating_add(consumed);
    Some((Record { version, payload }, end))
}

/// Unpacks a 19-octet header payload.
fn parse_header_payload(payload: &[u8]) -> Result<SegmentHeader, HeaderError> {
    let Some((stream, rest)) = payload.split_first_chunk::<16>() else {
        return Err(HeaderError::InvalidLength { len: payload.len() });
    };
    let Some((year_octets, rest)) = rest.split_first_chunk::<2>() else {
        return Err(HeaderError::InvalidLength { len: payload.len() });
    };
    let Some((&month, rest)) = rest.split_first() else {
        return Err(HeaderError::InvalidLength { len: payload.len() });
    };
    if !rest.is_empty() {
        return Err(HeaderError::InvalidLength { len: payload.len() });
    }
    if !is_month(month) {
        return Err(HeaderError::BadMonth { month });
    }
    Ok(SegmentHeader {
        stream: *stream,
        year: u16::from_le_bytes(*year_octets),
        month,
    })
}

/// Whether `month` is 1 through 12.
const fn is_month(month: u8) -> bool {
    month >= 1 && month <= 12
}

/// CRC-32C of `version` followed by `payload`.
fn crc32c_frame(version: u8, payload: &[u8]) -> u32 {
    !crc32c_update(crc32c_update(CRC_INIT, &[version]), payload)
}

/// CRC-32C of `bytes`.
#[cfg(test)]
fn crc32c(bytes: &[u8]) -> u32 {
    !crc32c_update(CRC_INIT, bytes)
}

/// One reflected CRC-32C step over `bytes`, starting from `crc`.
fn crc32c_update(mut crc: u32, bytes: &[u8]) -> u32 {
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let bit = crc & 1;
            crc >>= 1;
            if bit != 0 {
                crc ^= CRC_POLY;
            }
        }
    }
    crc
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use super::*;
    use proptest::collection::vec;
    use proptest::prelude::*;

    /// The input every entry in the CRC `RevEng` catalogue gives a check
    /// value for.
    const CHECK_INPUT: &[u8] = b"123456789";

    /// CRC-32C from the unreflected Castagnoli polynomial, written only for
    /// these tests: each input octet is reflected, bits are shifted out the
    /// top, and the output is reflected.
    fn crc32c_unreflected(bytes: &[u8]) -> u32 {
        const POLY: u32 = 0x1EDC_6F41;
        let mut crc = 0xFFFF_FFFF;
        for &byte in bytes {
            crc ^= u32::from(byte.reverse_bits()) << 24;
            for _ in 0..8 {
                crc = if crc & 0x8000_0000 == 0 {
                    crc << 1
                } else {
                    (crc << 1) ^ POLY
                };
            }
        }
        (!crc).reverse_bits()
    }

    /// A record of version 1 whose CRC is the unreflected oracle's, not
    /// [`crc32c`]'s.
    fn frame(payload: &[u8]) -> Vec<u8> {
        frame_version(RECORD_VERSION, payload)
    }

    /// A record of `version` whose CRC is the unreflected oracle's.
    fn frame_version(version: u8, payload: &[u8]) -> Vec<u8> {
        let len = u32::try_from(payload.len()).expect("test payloads fit a u32");
        let mut hashed = vec![version];
        hashed.extend(payload);
        let crc = crc32c_unreflected(&hashed);
        let mut out = Vec::new();
        out.extend(len.to_le_bytes());
        out.extend(crc.to_le_bytes());
        out.push(version);
        out.extend(payload);
        out
    }

    /// Concatenates independently framed records.
    fn segment(payloads: &[&[u8]]) -> Vec<u8> {
        payloads.iter().flat_map(|payload| frame(payload)).collect()
    }

    /// Every item [`records`] yields over `bytes`.
    fn items(bytes: &[u8]) -> Vec<Item<'_>> {
        records(bytes).collect()
    }

    /// A whole record of version 1.
    fn rec(payload: &[u8]) -> Item<'_> {
        Item::Record(Record {
            version: RECORD_VERSION,
            payload,
        })
    }

    /// A damaged range.
    fn damaged(range: Range<usize>) -> Item<'static> {
        Item::Damaged { range }
    }

    /// `len` copies of `byte`, grown one octet at a time so the test does
    /// not call the banned `vec![x; n]`.
    fn filled(len: usize, byte: u8) -> Vec<u8> {
        let mut out = Vec::new();
        for _ in 0..len {
            out.push(byte);
        }
        out
    }

    /// Verifies: SEC-HIS-036
    #[test]
    fn crc32c_matches_published_check_values() {
        // CRC-32/ISCSI (CRC-32C) in the RevEng catalogue.
        assert_eq!(crc32c(CHECK_INPUT), 0xE306_9283);
        assert_eq!(crc32c_unreflected(CHECK_INPUT), 0xE306_9283);
        // Empty input is the identity of init XOR xorout.
        assert_eq!(crc32c(&[]), 0x0000_0000);
        // RFC 3720 appendix B.4 / Intel CRC-32C test vectors.
        assert_eq!(crc32c(&[0x00; 32]), 0x8A91_36AA);
        assert_eq!(crc32c(&[0xFF; 32]), 0x62A8_AB43);
        let sequential: Vec<u8> = (0..32).collect();
        assert_eq!(crc32c(&sequential), 0x46DD_794E);
        let reverse: Vec<u8> = (0..32).rev().collect();
        assert_eq!(crc32c(&reverse), 0x113F_DB5C);
    }

    #[test]
    fn the_unreflected_oracle_agrees_with_the_writer() {
        assert_eq!(crc32c(b"hi"), crc32c_unreflected(b"hi"));
        assert_eq!(
            crc32c(&[RECORD_VERSION]),
            crc32c_unreflected(&[RECORD_VERSION])
        );
        assert_eq!(
            crc32c(&[RECORD_VERSION, b'h', b'i']),
            crc32c_unreflected(&[RECORD_VERSION, b'h', b'i'])
        );
    }

    /// Verifies: SEC-MED-001
    #[test]
    fn encode_writes_the_independently_framed_bytes() {
        // length 2, CRC-32C of [0x01, 'h', 'i'] = 0xC1D99F14, version 1, "hi".
        assert_eq!(
            encode(b"hi"),
            Ok(vec![
                0x02,
                0x00,
                0x00,
                0x00,
                0x14,
                0x9F,
                0xD9,
                0xC1,
                RECORD_VERSION,
                b'h',
                b'i',
            ])
        );
        assert_eq!(encode(b"hi"), Ok(frame(b"hi")));
        assert_eq!(encode(b""), Ok(frame(b"")));
        assert_eq!(encode(b"abc"), Ok(frame(b"abc")));
    }

    /// Verifies: SEC-MED-003, SEC-TM-032
    #[test]
    fn encode_refuses_a_payload_above_the_cap() {
        let over = usize::try_from(u64::from(MAX_PAYLOAD) + 1).expect("cap fits usize");
        assert_eq!(
            encode(&filled(over, 0xAA)),
            Err(EncodeError::TooLong {
                len: u64::from(MAX_PAYLOAD) + 1,
                max: MAX_PAYLOAD,
            })
        );
        let max = usize::try_from(MAX_PAYLOAD).expect("cap fits usize");
        let payload = filled(max, 0xAA);
        let framed = encode(&payload).expect("a payload of the cap is accepted");
        assert_eq!(framed.len(), 9 + max);
        assert_eq!(items(&framed), [rec(&payload)]);
        assert_eq!(recover_tail(&framed), framed.len());
    }

    /// Verifies: SEC-MED-001, SEC-MED-008
    #[test]
    fn records_yields_encoded_payloads_in_order() {
        let bytes = segment(&[b"one".as_slice(), b"two", b""]);
        assert_eq!(items(&bytes), [rec(b"one"), rec(b"two"), rec(b"")]);
        assert_eq!(items(&[]), []);
        assert_eq!(items(&frame(b"hi")), [rec(b"hi")]);
    }

    /// Verifies: SEC-MED-001
    #[test]
    fn a_record_of_an_unknown_version_is_kept_when_its_crc_matches() {
        let bytes = frame_version(2, b"keep");
        assert_eq!(
            items(&bytes),
            [Item::Record(Record {
                version: 2,
                payload: b"keep",
            })]
        );
    }

    /// Verifies: SEC-MED-001, SEC-MED-008, SEC-HIS-036
    #[test]
    fn recover_tail_keeps_a_record_cut_at_every_byte() {
        let bytes = frame(b"tail");
        assert_eq!(bytes.len(), 13);
        assert_eq!(recover_tail(&[]), 0);
        assert_eq!(items(&[]), []);
        for cut in 1..bytes.len() {
            assert_eq!(recover_tail(&bytes[..cut]), 0, "cut {cut}");
            assert_eq!(items(&bytes[..cut]), [damaged(0..cut)], "cut {cut}");
        }
        assert_eq!(recover_tail(&bytes), bytes.len());
        assert_eq!(items(&bytes), [rec(b"tail")]);
    }

    /// Verifies: SEC-MED-001, SEC-MED-008
    #[test]
    fn recover_tail_of_two_records_stops_at_the_torn_second() {
        let first = frame(b"A");
        let second = frame(b"BC");
        let mut bytes = first.clone();
        bytes.extend(&second);
        let first_end = first.len();
        assert_eq!(recover_tail(&bytes), bytes.len());
        for extra in 0..second.len() {
            let cut = first_end + extra;
            assert_eq!(recover_tail(&bytes[..cut]), first_end, "cut {cut}");
        }
    }

    /// Verifies: SEC-MED-001, SEC-MED-008
    #[test]
    fn a_flipped_bit_in_a_middle_record_is_a_damaged_range() {
        let first = frame(b"one");
        let middle = frame(b"two");
        let last = frame(b"three");
        let mut bytes = first.clone();
        bytes.extend(&middle);
        bytes.extend(&last);
        let middle_at = first.len();
        let last_at = middle_at + middle.len();
        // Flip the first payload octet of the middle record (offset 9 in
        // that record: after length, CRC and version).
        bytes[middle_at + 9] ^= 0x01;
        assert_eq!(
            items(&bytes),
            [rec(b"one"), damaged(middle_at..last_at), rec(b"three"),]
        );
        // Tail recovery does not skip damage: it stops at the first bad
        // record, so a middle flip is not treated as a torn write.
        assert_eq!(recover_tail(&bytes), middle_at);
    }

    /// Verifies: SEC-MED-003, SEC-MED-006, SEC-TM-032
    #[test]
    fn a_length_field_claiming_four_gigabytes_is_damage() {
        let first = frame(b"ok");
        let last = frame(b"after");
        let mut bytes = first.clone();
        // A u32 of 0xFFFFFFFF is 4 GiB minus one octet, above the cap.
        bytes.extend([0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0, RECORD_VERSION, 0xAA]);
        let gap_at = first.len();
        let last_at = bytes.len();
        bytes.extend(&last);
        assert_eq!(
            items(&bytes),
            [rec(b"ok"), damaged(gap_at..last_at), rec(b"after"),]
        );
        // 16 octets of 0xFF as a lone segment: no whole record, no
        // allocation of 4 GiB.
        let huge = [0xFF; 16];
        assert_eq!(items(&huge), [damaged(0..16)]);
        assert_eq!(recover_tail(&huge), 0);
    }

    /// Verifies: SEC-MED-001, SEC-MED-008
    #[test]
    fn two_damaged_regions_are_reported_separately() {
        let first = frame(b"A");
        let second = frame(b"B");
        let third = frame(b"C");
        let fourth = frame(b"D");
        let fifth = frame(b"E");
        let mut bytes = first.clone();
        bytes.extend(&second);
        bytes.extend(&third);
        bytes.extend(&fourth);
        bytes.extend(&fifth);
        let second_at = first.len();
        let third_at = second_at + second.len();
        let fourth_at = third_at + third.len();
        let fifth_at = fourth_at + fourth.len();
        bytes[second_at + 9] ^= 0x01;
        bytes[fourth_at + 9] ^= 0x01;
        assert_eq!(
            items(&bytes),
            [
                rec(b"A"),
                damaged(second_at..third_at),
                rec(b"C"),
                damaged(fourth_at..fifth_at),
                rec(b"E"),
            ]
        );
    }

    /// Verifies: SEC-MED-001
    #[test]
    fn a_bad_crc_on_the_only_record_is_the_whole_segment() {
        let mut bytes = frame(b"x");
        let last = bytes.len() - 1;
        bytes[last] ^= 0x80;
        assert_eq!(items(&bytes), [damaged(0..bytes.len())]);
        assert_eq!(recover_tail(&bytes), 0);
    }

    #[test]
    fn encode_header_writes_stream_year_and_month() {
        let header = SegmentHeader {
            stream: [0x11; 16],
            year: 2026,
            month: 10,
        };
        let mut payload = [0x11; 16].to_vec();
        payload.extend(2026_u16.to_le_bytes());
        payload.push(10);
        assert_eq!(encode_header(&header), Ok(frame(&payload)));
        assert_eq!(super::header(&frame(&payload)), Ok(header));
        assert_eq!(
            encode_header(&SegmentHeader {
                stream: [0; 16],
                year: 2026,
                month: 0,
            }),
            Err(EncodeError::BadMonth { month: 0 })
        );
        assert_eq!(
            encode_header(&SegmentHeader {
                stream: [0; 16],
                year: 2026,
                month: 13,
            }),
            Err(EncodeError::BadMonth { month: 13 })
        );
        let january = SegmentHeader {
            stream: [0; 16],
            year: 1,
            month: 1,
        };
        let mut january_payload = [0_u8; 16].to_vec();
        january_payload.extend(1_u16.to_le_bytes());
        january_payload.push(1);
        assert_eq!(encode_header(&january), Ok(frame(&january_payload)));
        let december = SegmentHeader {
            stream: [0; 16],
            year: 9999,
            month: 12,
        };
        let mut december_payload = [0_u8; 16].to_vec();
        december_payload.extend(9999_u16.to_le_bytes());
        december_payload.push(12);
        assert_eq!(encode_header(&december), Ok(frame(&december_payload)));
    }

    #[test]
    fn header_reports_each_kind_of_refusal() {
        assert_eq!(super::header(&[]), Err(HeaderError::Missing));
        assert_eq!(
            super::header(&[0x00, 0x01, 0x02]),
            Err(HeaderError::Damaged { range: 0..3 })
        );
        assert_eq!(
            super::header(&frame(b"short")),
            Err(HeaderError::InvalidLength { len: 5 })
        );
        assert_eq!(
            super::header(&frame(&filled(16, 0))),
            Err(HeaderError::InvalidLength { len: 16 })
        );
        assert_eq!(
            super::header(&frame(&filled(17, 0))),
            Err(HeaderError::InvalidLength { len: 17 })
        );
        assert_eq!(
            super::header(&frame(&filled(18, 0))),
            Err(HeaderError::InvalidLength { len: 18 })
        );
        let mut long = filled(20, 0);
        long[18] = 6;
        assert_eq!(
            super::header(&frame(&long)),
            Err(HeaderError::InvalidLength { len: 20 })
        );
        let mut payload = filled(19, 0);
        payload[18] = 0;
        assert_eq!(
            super::header(&frame(&payload)),
            Err(HeaderError::BadMonth { month: 0 })
        );
        payload[18] = 12;
        assert_eq!(
            super::header(&frame(&payload)),
            Ok(SegmentHeader {
                stream: [0; 16],
                year: 0,
                month: 12,
            })
        );
    }

    /// Verifies: SEC-MED-001, SEC-MED-008
    #[test]
    fn a_single_garbage_octet_before_a_record_is_a_one_octet_range() {
        let mut bytes = vec![0xFF];
        bytes.extend(frame(b"ok"));
        assert_eq!(items(&bytes), [damaged(0..1), rec(b"ok")]);
        assert_eq!(recover_tail(&bytes), 0);
        assert!(record_at(b"abc", 3).is_none());
        assert!(record_at(b"abc", 4).is_none());
    }

    /// Verifies: SEC-MED-003, SEC-MED-006, SEC-TM-032
    #[test]
    fn a_payload_one_over_the_cap_is_damage_when_the_bytes_are_present() {
        let over = usize::try_from(u64::from(MAX_PAYLOAD) + 1).expect("cap fits usize");
        let payload = filled(over, 0xAA);
        let bytes = frame_version(RECORD_VERSION, &payload);
        assert_eq!(items(&bytes), [damaged(0..bytes.len())]);
        assert_eq!(recover_tail(&bytes), 0);
    }

    /// Verifies: SEC-MED-003, SEC-MED-006, SEC-TM-032
    #[test]
    fn a_declared_length_of_the_cap_plus_one_is_damage_in_a_short_buffer() {
        // length = MAX_PAYLOAD + 1 as u32, then eight more octets. The
        // buffer is 12 octets; nothing is allocated for the claimed size.
        let mut bytes = (MAX_PAYLOAD + 1).to_le_bytes().to_vec();
        bytes.extend([0, 0, 0, 0, RECORD_VERSION, 0x00, 0x00, 0x00]);
        assert_eq!(bytes.len(), 12);
        assert_eq!(items(&bytes), [damaged(0..12)]);
        assert_eq!(recover_tail(&bytes), 0);
    }

    /// Verifies: SEC-MED-001, SEC-MED-008
    #[test]
    fn a_crc_mismatch_with_a_honest_length_resyncs_at_the_next_record() {
        let first = frame(b"left");
        let mut middle = frame(b"mid");
        middle[8] ^= 0x01; // flip the version byte, CRC no longer matches
        let last = frame(b"right");
        let mut bytes = first.clone();
        bytes.extend(&middle);
        bytes.extend(&last);
        assert_eq!(
            items(&bytes),
            [
                rec(b"left"),
                damaged(first.len()..first.len() + middle.len()),
                rec(b"right"),
            ]
        );
    }

    /// Verifies: SEC-MED-005, SEC-MED-007, SEC-MED-008
    #[test]
    fn walking_any_prefix_of_a_framed_record_terminates() {
        let bytes = frame(&[0x55; 40]);
        for cut in 0..=bytes.len() {
            let yielded: Vec<_> = records(&bytes[..cut]).collect();
            assert!(
                yielded.len() <= bytes.len(),
                "{} items from {cut} octets",
                yielded.len()
            );
            assert!(recover_tail(&bytes[..cut]) <= cut);
        }
    }

    proptest! {
        /// Verifies: SEC-MED-001, SEC-MED-008
        #[test]
        fn encode_then_records_yields_the_payloads_in_order(
            payloads in vec(vec(any::<u8>(), 0..48), 0..8),
        ) {
            let mut bytes = Vec::new();
            for payload in &payloads {
                bytes.extend(encode(payload).expect("payloads are under the cap"));
            }
            let got: Vec<Vec<u8>> = records(&bytes)
                .map(|item| match item {
                    Item::Record(record) => record.payload.to_vec(),
                    Item::Damaged { range } => panic!("damage at {range:?}"),
                })
                .collect();
            prop_assert_eq!(got, payloads);
        }

        /// Verifies: SEC-MED-001, SEC-MED-008
        #[test]
        fn recover_tail_keeps_exactly_the_whole_records_before_a_cut(
            payloads in vec(vec(any::<u8>(), 0..24), 1..6),
            cut_seed in any::<u16>(),
        ) {
            let framed: Vec<Vec<u8>> = payloads
                .iter()
                .map(|payload| encode(payload).expect("payloads are under the cap"))
                .collect();
            let mut bytes = Vec::new();
            let mut ends = vec![0];
            for record in &framed {
                bytes.extend(record);
                ends.push(bytes.len());
            }
            let cut = usize::from(cut_seed) % (bytes.len() + 1);
            let expected = ends
                .iter()
                .copied()
                .filter(|&end| end <= cut)
                .max()
                .unwrap_or(0);
            prop_assert_eq!(recover_tail(&bytes[..cut]), expected);
        }

        /// Verifies: SEC-MED-001, SEC-MED-005, SEC-MED-007, SEC-MED-008, SEC-HIS-036
        #[test]
        fn never_panics_and_every_item_stays_inside_the_input(
            bytes in vec(any::<u8>(), 0..128),
        ) {
            let len = bytes.len();
            let yielded: Vec<_> = records(&bytes).collect();
            prop_assert!(yielded.len() <= len + 1);
            for item in &yielded {
                match item {
                    Item::Record(record) => {
                        prop_assert!(
                            record.payload.len()
                                <= usize::try_from(MAX_PAYLOAD).unwrap_or(usize::MAX)
                        );
                        prop_assert!(
                            record.payload.is_empty()
                                || bytes
                                    .windows(record.payload.len())
                                    .any(|window| window == record.payload)
                        );
                    }
                    Item::Damaged { range } => {
                        prop_assert!(range.start < range.end);
                        prop_assert!(range.end <= len);
                    }
                }
            }
            let tail = recover_tail(&bytes);
            prop_assert!(tail <= len);
            let _ = header(&bytes);
            let _ = encode(&bytes);
            let _ = encode_header(&SegmentHeader {
                stream: [0; 16],
                year: 2026,
                month: bytes.first().copied().unwrap_or(1),
            });
        }
    }
}
