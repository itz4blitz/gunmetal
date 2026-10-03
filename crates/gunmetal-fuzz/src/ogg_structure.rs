//! The structure-aware harness for Ogg (SEC-MED-031).
//!
//! Random octets rarely make a page whose checksum holds, so the plain
//! harness seldom gets past the page layer. This one reads its input as a
//! description of a logical stream, lays the stream out on pages with
//! valid framing and checksums, damages one field when the description
//! asks for it, and feeds the result to [`ogg::packets`] and
//! [`ogg::last_granule`]. The page writer here shares no code with the
//! parser.
//!
//! The description is one octet for the layout, one for the damage, then
//! packets: a two-octet little-endian length and that many octets of
//! payload, the last one cut short where the input ends; one octet left
//! over is ignored. The layout octet's low three bits plus
//! one are the most segments a page carries. The damage octet's low three
//! bits pick a [`Damage`] by its number, and the rest pick the page it
//! applies to, counted modulo the pages written.

use gunmetal_core::formats::ogg::{self, STEPS_PER_OCTET};
use gunmetal_core::parse::{Budget, Cursor, Limits, ParseFault};

use crate::ogg::PacketItem;

/// The serial number of the stream the harness writes.
pub const SERIAL: u32 = 1;

/// What the harness wrote and what the parser read back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// How many pages the description laid out, before any damage.
    pub pages: usize,
    /// How many octets the stream written holds, after the damage.
    pub octets: usize,
    /// Every item [`ogg::packets`] yielded for [`SERIAL`]: each packet's
    /// page offset, granule position and length, or the error.
    pub read: Vec<PacketItem>,
    /// The other streams the walk recorded.
    pub others: Vec<u32>,
    /// [`ogg::last_granule`] of [`SERIAL`] over the stream.
    pub last: Result<Option<u64>, ParseFault>,
}

/// The damage a description can ask for, by its number in the low three
/// bits of the damage octet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Damage {
    /// 0: none; the packets must come back exactly as written.
    None,
    /// 1: one bit of the page's checksum is flipped.
    Checksum,
    /// 2: the page is left out.
    Dropped,
    /// 3: the page moves to the next serial number.
    Serial,
    /// 4: the page's sequence number goes up by one.
    Sequence,
    /// 5: the page's continuation flag is flipped.
    Continued,
    /// 6: the page says no packet ends on it.
    Granule,
    /// 7: the stream ends halfway through the page.
    Cut,
}

impl Damage {
    /// The damage numbered by the low three bits of `octet`.
    #[must_use]
    pub const fn of(octet: u8) -> Self {
        match octet & 7 {
            0 => Self::None,
            1 => Self::Checksum,
            2 => Self::Dropped,
            3 => Self::Serial,
            4 => Self::Sequence,
            5 => Self::Continued,
            6 => Self::Granule,
            _ => Self::Cut,
        }
    }
}

/// The header type flag of a page that continues a packet.
const CONTINUED: u8 = 0x01;

/// The header type flag of the first page of a logical stream.
const FIRST: u8 = 0x02;

/// One page as the harness lays it out.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Page {
    flags: u8,
    granule: u64,
    serial: u32,
    sequence: u32,
    lacing: Vec<u8>,
    body: Vec<u8>,
}

impl Page {
    /// The page's octets, with its checksum.
    fn bytes(&self) -> Vec<u8> {
        let segments = u8::try_from(self.lacing.len()).unwrap_or(u8::MAX);
        let mut page = [
            &b"OggS\x00"[..],
            &[self.flags],
            &self.granule.to_le_bytes(),
            &self.serial.to_le_bytes(),
            &self.sequence.to_le_bytes(),
            &[0; 4],
            &[segments],
            &self.lacing,
            &self.body,
        ]
        .concat();
        let crc = crc(&page);
        page[22..26].copy_from_slice(&crc.to_le_bytes());
        page
    }
}

/// The Ogg checksum, bit by bit: polynomial 0x04C11DB7, initial value 0,
/// not reflected, no final XOR.
fn crc(octets: &[u8]) -> u32 {
    let mut crc = 0_u32;
    for &octet in octets {
        crc ^= u32::from(octet) << 24;
        for _ in 0..8 {
            crc = if crc & 0x8000_0000 == 0 {
                crc << 1
            } else {
                (crc << 1) ^ 0x04C1_1DB7
            };
        }
    }
    crc
}

/// Lays `packets` out on pages of at most `per_page` segments, numbered
/// from 0, the first marked first. No page is marked last, as in a stream
/// still being written; the packet walk does not read that mark. Packet
/// `n` ends at granule position `n + 1`, and each page carries the position
/// of the last packet that ends on it, or −1.
fn paginate(packets: &[&[u8]], per_page: usize) -> Vec<Page> {
    // Every segment: its packet, its length, and whether it ends the packet.
    // A packet is whole segments of 255 octets and one shorter last one.
    let mut segments = Vec::new();
    for (packet, data) in packets.iter().enumerate() {
        let count = data.len() / 255 + 1;
        for number in 1..=count {
            let ends = number == count;
            let len = if ends { data.len() % 255 } else { 255 };
            segments.push((packet, len, ends));
        }
    }
    let chunks: Vec<_> = segments.chunks(per_page).collect();
    let mut laid = vec![0; packets.len()];
    let mut continued = false;
    let mut pages = Vec::new();
    for (number, chunk) in chunks.iter().enumerate() {
        let mut flags = if number == 0 { FIRST } else { 0 };
        if continued {
            flags |= CONTINUED;
        }
        let mut page = Page {
            flags,
            granule: u64::MAX,
            serial: SERIAL,
            sequence: u32::try_from(number).unwrap_or(u32::MAX),
            lacing: Vec::new(),
            body: Vec::new(),
        };
        for &(packet, len, ends) in *chunk {
            let start = laid[packet];
            page.lacing.push(u8::try_from(len).unwrap_or(u8::MAX));
            page.body
                .extend_from_slice(&packets[packet][start..start + len]);
            laid[packet] = start + len;
            if ends {
                page.granule = u64::try_from(packet).unwrap_or(u64::MAX) + 1;
            }
        }
        continued = chunk.last().is_some_and(|&(_, _, ends)| !ends);
        pages.push(page);
    }
    pages
}

/// Writes the stream `data` describes, damaged as it asks, and reads its
/// packets back.
///
/// # Panics
///
/// Panics when the description asks for no damage and the packets read
/// back are not exactly the packets written.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let (&layout, rest) = data.split_first().unwrap_or((&0, &[]));
    let (&damage, mut script) = rest.split_first().unwrap_or((&0, &[]));
    let mut packets = Vec::new();
    while let Some((&[low, high], rest)) = script.split_first_chunk::<2>() {
        let len = usize::from(u16::from_le_bytes([low, high]));
        let (packet, rest) = rest.split_at(len.min(rest.len()));
        packets.push(packet);
        script = rest;
    }
    let mut pages = paginate(&packets, usize::from(layout & 7) + 1);
    let laid_out = pages.len();
    let kind = Damage::of(damage);
    let at = usize::from(damage >> 3) % laid_out.max(1);
    match (kind, pages.get_mut(at)) {
        (Damage::Serial, Some(page)) => page.serial = SERIAL + 1,
        (Damage::Sequence, Some(page)) => page.sequence += 1,
        (Damage::Continued, Some(page)) => page.flags ^= CONTINUED,
        (Damage::Granule, Some(page)) => page.granule = u64::MAX,
        _ => {}
    }
    let mut stream = Vec::new();
    let mut end = None;
    for (number, page) in pages.iter().enumerate() {
        let start = stream.len();
        let bytes = page.bytes();
        match kind {
            Damage::Dropped if number == at => {}
            Damage::Checksum if number == at => {
                stream.extend_from_slice(&bytes[..22]);
                stream.push(bytes[22] ^ 1);
                stream.extend_from_slice(&bytes[23..]);
            }
            Damage::Cut if number == at => {
                stream.extend_from_slice(&bytes);
                end = Some(start + bytes.len() / 2);
            }
            _ => stream.extend_from_slice(&bytes),
        }
    }
    if let Some(end) = end {
        stream.truncate(end);
    }
    let octets = stream.len();
    // A slice never holds more than u64::MAX octets.
    let len = u64::try_from(octets).unwrap_or(u64::MAX);
    let mut budget = Budget::for_input(len, STEPS_PER_OCTET, 0);
    let mut walk = ogg::packets(Cursor::new(&stream), SERIAL, &Limits::DEFAULT, &mut budget);
    let read: Vec<_> = walk
        .by_ref()
        .take(octets.saturating_add(2))
        .map(|item| item.map(|packet| (packet.offset, packet.granule, packet.data.into_owned())))
        .collect();
    let others = walk.other_streams().to_vec();
    assert!(
        kind != Damage::None
            || read
                .iter()
                .map(|item| item.as_ref().ok().map(|(_, _, data)| &data[..]))
                .eq(packets.iter().map(|&packet| Some(packet))),
        "{packets:02X?} came back as {read:02X?}"
    );
    let last = ogg::last_granule(
        Cursor::new(&stream),
        SERIAL,
        &mut Budget::for_input(len, STEPS_PER_OCTET, 0),
    );
    Outcome {
        pages: laid_out,
        octets,
        read: read
            .into_iter()
            .map(|item| item.map(|(offset, granule, data)| (offset, granule, data.len())))
            .collect(),
        others,
        last,
    }
}
