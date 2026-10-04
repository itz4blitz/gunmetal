//! Builders for Ogg streams (RFC 3533): pages with their checksum, and the
//! packets of one logical stream laid out across pages as an encoder lays
//! them out.
//!
//! Nothing here is shared with the parser in `gunmetal-core`. The checksum
//! comes from [`crc32_ogg`], and the layout is written from the RFC's
//! description of lacing: a packet is cut into segments of 255 octets and
//! one shorter final segment, which is empty when the packet's length is a
//! multiple of 255, and a page carries at most 255 segments.

use crate::bytes::Bytes;
use crate::checksum::crc32_ogg;

/// The header type flag of a page whose first segment continues a packet
/// from the page before it.
pub const CONTINUED: u8 = 0x01;
/// The header type flag of the first page of a logical stream.
pub const FIRST: u8 = 0x02;
/// The header type flag of the last page of a logical stream.
pub const LAST: u8 = 0x04;
/// The granule position of a page on which no packet ends, which the RFC
/// writes as −1.
pub const NO_GRANULE: u64 = u64::MAX;

/// One page, field by field (RFC 3533, section 6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    /// The header type flags: any of [`CONTINUED`], [`FIRST`] and [`LAST`].
    pub flags: u8,
    /// The granule position, or [`NO_GRANULE`].
    pub granule: u64,
    /// The serial number of the page's logical stream.
    pub serial: u32,
    /// The page's sequence number within its logical stream.
    pub sequence: u32,
    /// The segment table: one lacing value per segment.
    pub lacing: Vec<u8>,
    /// The segments, back to back. Nothing checks that their length is the
    /// sum of the lacing values, so a test can write a page that lies.
    pub body: Vec<u8>,
}

impl Page {
    /// The page as a file holds it: the capture pattern, version 0, the
    /// fields, the checksum of the whole page, then the segment table and
    /// the body.
    ///
    /// # Panics
    ///
    /// Panics when the segment table holds more than 255 lacing values.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let segments = u8::try_from(self.lacing.len()).expect("a page holds at most 255 segments");
        let mut page = Bytes::new();
        page.bytes(b"OggS")
            .u8(0)
            .u8(self.flags)
            .u64_le(self.granule)
            .u32_le(self.serial)
            .u32_le(self.sequence)
            .u32_le(0)
            .u8(segments)
            .bytes(&self.lacing)
            .bytes(&self.body);
        let mut page = page.into_vec();
        let crc = crc32_ogg(&page);
        page[22..26].copy_from_slice(&crc.to_le_bytes());
        page
    }
}

/// The lacing values of one packet of `len` octets: 255 for each whole 255
/// octets, then the length of what is left, which is 0 when `len` is a
/// multiple of 255. A reader knows a packet has ended when it meets a value
/// below 255.
#[must_use]
pub fn lacing(len: usize) -> Vec<u8> {
    let mut values = vec![255; len / 255];
    // What is left is below 255, so its lowest octet is all of it.
    let [remainder, ..] = (len % 255).to_le_bytes();
    values.push(remainder);
    values
}

/// One segment of a packet, as [`paginate`] places it.
struct Segment {
    /// Which packet it belongs to.
    packet: usize,
    /// Its lacing value.
    len: u8,
    /// Whether it is the packet's last segment.
    ends: bool,
}

/// Lays `packets` of the logical stream `serial` out across pages of at
/// most `max_segments` segments each, as an encoder does.
///
/// Each packet is a payload and the granule position at its end. The
/// pages are numbered from 0. The first carries [`FIRST`] and the last
/// [`LAST`]; a page whose first segment continues a packet from the page
/// before carries [`CONTINUED`]. A page's granule position is that of the
/// last packet that ends on it, or [`NO_GRANULE`] when none does. No
/// packets make no pages.
///
/// # Panics
///
/// Panics when `max_segments` is zero.
#[must_use]
pub fn paginate(serial: u32, packets: &[(Vec<u8>, u64)], max_segments: u8) -> Vec<Page> {
    let mut segments = Vec::new();
    for (packet, (data, _)) in packets.iter().enumerate() {
        let values = lacing(data.len());
        let count = values.len();
        for (n, len) in values.into_iter().enumerate() {
            segments.push(Segment {
                packet,
                len,
                ends: n + 1 == count,
            });
        }
    }
    let chunks: Vec<&[Segment]> = segments.chunks(usize::from(max_segments)).collect();
    // How much of each packet the pages so far have carried.
    let mut written = vec![0; packets.len()];
    let mut continued = false;
    let mut pages = Vec::new();
    for (number, chunk) in chunks.iter().enumerate() {
        let mut flags = 0;
        if continued {
            flags |= CONTINUED;
        }
        if number == 0 {
            flags |= FIRST;
        }
        if number + 1 == chunks.len() {
            flags |= LAST;
        }
        let mut page = Page {
            flags,
            granule: NO_GRANULE,
            serial,
            sequence: u32::try_from(number).expect("a test writes fewer than 2^32 pages"),
            lacing: Vec::new(),
            body: Vec::new(),
        };
        for segment in *chunk {
            let (data, granule) = &packets[segment.packet];
            let start = written[segment.packet];
            let end = start + usize::from(segment.len);
            page.lacing.push(segment.len);
            page.body.extend_from_slice(&data[start..end]);
            written[segment.packet] = end;
            if segment.ends {
                page.granule = *granule;
            }
        }
        continued = chunk.last().is_some_and(|segment| !segment.ends);
        pages.push(page);
    }
    pages
}

/// The bytes of `pages`, back to back.
#[must_use]
pub fn write(pages: &[Page]) -> Vec<u8> {
    pages.iter().flat_map(Page::to_bytes).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 51-octet packet on the first page `ffmpeg -c:a flac -f ogg`
    /// wrote for 64 frames of 16-bit mono silence at 8 kHz: Ogg FLAC's
    /// mapping header and its STREAMINFO block.
    const FLAC_HEADER: [u8; 51] = [
        0x7F, b'F', b'L', b'A', b'C', 0x01, 0x00, 0x00, 0x01, b'f', b'L', b'a', b'C', 0x00, 0x00,
        0x00, 0x22, 0x00, 0x40, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x95, 0x01, 0xF4, 0x00,
        0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    /// The one FLAC frame on the last page of the same file.
    const FLAC_FRAME: [u8; 12] = [
        0xFF, 0xF8, 0x64, 0x08, 0x00, 0x3F, 0x5E, 0x00, 0x00, 0x00, 0xC6, 0x3C,
    ];

    /// The serial number ffmpeg chose for that file.
    const SERIAL: u32 = 0x4CA9_4C6A;

    #[test]
    fn writes_the_first_page_ffmpeg_wrote_octet_for_octet() {
        let page = Page {
            flags: FIRST,
            granule: 0,
            serial: SERIAL,
            sequence: 0,
            lacing: vec![51],
            body: FLAC_HEADER.to_vec(),
        };
        let expected = [
            &b"OggS\x00\x02"[..],
            &[0x00; 8],                // granule position
            &[0x6A, 0x4C, 0xA9, 0x4C], // serial number
            &[0x00; 4],                // page sequence number
            &[0x5E, 0xC2, 0x4E, 0x96], // CRC, as ffmpeg wrote it
            &[0x01, 0x33],             // one segment of 51 octets
            &FLAC_HEADER,
        ]
        .concat();
        assert_eq!(page.to_bytes(), expected);
    }

    #[test]
    fn writes_the_last_page_ffmpeg_wrote_octet_for_octet() {
        let page = Page {
            flags: LAST,
            granule: 64,
            serial: SERIAL,
            sequence: 2,
            lacing: vec![12],
            body: FLAC_FRAME.to_vec(),
        };
        let expected = [
            &b"OggS\x00\x04"[..],
            &[0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // granule position
            &[0x6A, 0x4C, 0xA9, 0x4C],                         // serial number
            &[0x02, 0x00, 0x00, 0x00],                         // page sequence number
            &[0x1D, 0x3F, 0x6F, 0xC7],                         // CRC, as ffmpeg wrote it
            &[0x01, 0x0C],                                     // one segment of 12 octets
            &FLAC_FRAME,
        ]
        .concat();
        assert_eq!(page.to_bytes(), expected);
    }

    #[test]
    fn writes_every_field_where_the_rfc_puts_it() {
        let page = Page {
            flags: CONTINUED | LAST,
            granule: NO_GRANULE,
            serial: 0x0403_0201,
            sequence: 0x0807_0605,
            lacing: vec![2, 0],
            body: vec![0xAA, 0xBB],
        };
        let bytes = page.to_bytes();
        assert_eq!(
            bytes[..22],
            [
                b'O', b'g', b'g', b'S', 0x00, 0x05, // capture, version, flags
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, // granule −1
                0x01, 0x02, 0x03, 0x04, // serial, least significant first
                0x05, 0x06, 0x07, 0x08, // sequence
            ]
        );
        assert_eq!(bytes[26..], [0x02, 0x02, 0x00, 0xAA, 0xBB]);
        // The checksum covers the page with its own field zeroed.
        let mut zeroed = bytes.clone();
        zeroed[22..26].fill(0);
        assert_eq!(bytes[22..26], crc32_ogg(&zeroed).to_le_bytes());
    }

    #[test]
    fn writes_a_page_that_lies_about_its_body() {
        // The table promises 9 octets; the body holds one.
        let page = Page {
            flags: 0,
            granule: 0,
            serial: 0,
            sequence: 0,
            lacing: vec![9],
            body: vec![0x01],
        };
        assert_eq!(page.to_bytes().len(), 27 + 1 + 1);
    }

    #[test]
    fn writes_a_full_segment_table() {
        let page = Page {
            flags: 0,
            granule: 0,
            serial: 0,
            sequence: 0,
            lacing: vec![0; 255],
            body: vec![],
        };
        let bytes = page.to_bytes();
        assert_eq!((bytes.len(), bytes[26]), (27 + 255, 255));
    }

    #[test]
    #[should_panic(expected = "a page holds at most 255 segments")]
    fn refuses_more_than_255_segments() {
        let page = Page {
            flags: 0,
            granule: 0,
            serial: 0,
            sequence: 0,
            lacing: vec![0; 256],
            body: vec![],
        };
        let _ = page.to_bytes();
    }

    #[test]
    fn laces_a_packet_as_the_rfc_shows() {
        assert_eq!(lacing(0), [0]);
        assert_eq!(lacing(1), [1]);
        assert_eq!(lacing(254), [254]);
        // A multiple of 255 ends with an empty segment.
        assert_eq!(lacing(255), [255, 0]);
        assert_eq!(lacing(256), [255, 1]);
        assert_eq!(lacing(510), [255, 255, 0]);
        assert_eq!(lacing(600), [255, 255, 90]);
    }

    /// `len` octets counting up from `first`.
    fn counting(first: u8, len: usize) -> Vec<u8> {
        (0..len)
            .map(|n| first.wrapping_add(u8::try_from(n % 256).unwrap()))
            .collect()
    }

    #[test]
    fn splits_a_long_packet_over_three_pages() {
        let packet = counting(0, 600);
        let pages = paginate(7, &[(packet.clone(), 1_000)], 1);
        assert_eq!(
            pages,
            [
                Page {
                    flags: FIRST,
                    granule: NO_GRANULE,
                    serial: 7,
                    sequence: 0,
                    lacing: vec![255],
                    body: packet[..255].to_vec(),
                },
                Page {
                    flags: CONTINUED,
                    granule: NO_GRANULE,
                    serial: 7,
                    sequence: 1,
                    lacing: vec![255],
                    body: packet[255..510].to_vec(),
                },
                Page {
                    flags: CONTINUED | LAST,
                    granule: 1_000,
                    serial: 7,
                    sequence: 2,
                    lacing: vec![90],
                    body: packet[510..].to_vec(),
                },
            ]
        );
    }

    #[test]
    fn puts_several_packets_on_one_page_with_the_last_ones_granule() {
        let pages = paginate(
            9,
            &[
                (vec![0xA1, 0xA2, 0xA3], 10),
                (vec![], 20),
                (vec![0xB1, 0xB2], 30),
            ],
            255,
        );
        assert_eq!(
            pages,
            [Page {
                flags: FIRST | LAST,
                granule: 30,
                serial: 9,
                sequence: 0,
                lacing: vec![3, 0, 2],
                body: vec![0xA1, 0xA2, 0xA3, 0xB1, 0xB2],
            }]
        );
    }

    #[test]
    fn puts_the_empty_segment_of_a_255_octet_packet_on_the_next_page() {
        let first = counting(1, 255);
        let pages = paginate(3, &[(first.clone(), 5), (vec![0xC1], 6)], 1);
        assert_eq!(
            pages,
            [
                Page {
                    flags: FIRST,
                    granule: NO_GRANULE,
                    serial: 3,
                    sequence: 0,
                    lacing: vec![255],
                    body: first,
                },
                // The empty segment ends the first packet, so this page
                // carries its granule position.
                Page {
                    flags: CONTINUED,
                    granule: 5,
                    serial: 3,
                    sequence: 1,
                    lacing: vec![0],
                    body: vec![],
                },
                // The packet before ended, so this page continues nothing.
                Page {
                    flags: LAST,
                    granule: 6,
                    serial: 3,
                    sequence: 2,
                    lacing: vec![1],
                    body: vec![0xC1],
                },
            ]
        );
    }

    #[test]
    fn fills_each_page_up_to_its_segment_limit() {
        // Five segments in pages of at most two: 2, 2, 1.
        let packets = [
            (vec![0x01], 1),
            (vec![0x02], 2),
            (vec![0x03], 3),
            (vec![0x04], 4),
            (vec![0x05], 5),
        ];
        let pages = paginate(0, &packets, 2);
        let summary: Vec<_> = pages
            .iter()
            .map(|page| (page.flags, page.granule, page.sequence, page.lacing.clone()))
            .collect();
        assert_eq!(
            summary,
            [
                (FIRST, 2, 0, vec![1, 1]),
                (0, 4, 1, vec![1, 1]),
                (LAST, 5, 2, vec![1]),
            ]
        );
    }

    #[test]
    fn lays_out_no_pages_for_no_packets() {
        assert_eq!(paginate(1, &[], 255), []);
    }

    #[test]
    #[should_panic(expected = "chunk size must be non-zero")]
    fn refuses_pages_of_no_segments() {
        let _ = paginate(1, &[(vec![0x01], 1)], 0);
    }

    #[test]
    fn writes_pages_back_to_back() {
        let pages = paginate(2, &[(vec![0x01], 1), (vec![0x02, 0x03], 2)], 1);
        let expected = [pages[0].to_bytes(), pages[1].to_bytes()].concat();
        assert_eq!(write(&pages), expected);
        assert_eq!(write(&pages).len(), 28 + 1 + 28 + 2);
        assert_eq!(write(&[]), []);
    }
}
