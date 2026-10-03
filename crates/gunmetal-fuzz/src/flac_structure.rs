//! The structure-aware harness for FLAC (SEC-MED-031).
//!
//! Random octets rarely make a frame header: a sync code, a coded number
//! laid out as UTF-8 lays out a code point, and a CRC-8 that matches. This
//! harness reads its input as recipes instead and writes headers with that
//! framing always valid around fields taken from the input, so the fuzzer
//! spends its time on the fields: every code, reserved and forbidden ones
//! included, coded numbers of every width, end-of-header fields, and data
//! between headers that may hold false sync codes. The stream it writes is
//! then indexed through the [`flac_frames`](crate::flac_frames) harness,
//! which checks the index's invariants.
//!
//! A recipe is [`RECIPE_LEN`] octets:
//!
//! | Octets | Field |
//! |---|---|
//! | 0 | Bit 0: the blocking strategy bit. Bits 1 to 7: how many of the octets after the recipe follow the header as frame data. |
//! | 1 | The header's block size and sample rate codes, as written. |
//! | 2 | The header's channel and bit depth codes and reserved bit, as written. |
//! | 3 to 7 | The coded number, big-endian, kept to its low 31 bits for a fixed-blocking frame number or 36 bits for a sample number. |
//! | 8 and 9 | The end-of-header block size field, big-endian; an 8-bit field takes octet 9. |
//! | 10 and 11 | The end-of-header sample rate field, big-endian; an 8-bit field takes octet 11. |
//!
//! Octets after the last whole recipe are ignored.

use crate::flac_frames;

/// The octets of one recipe.
pub const RECIPE_LEN: usize = 12;

/// What the harness wrote and what the frame index reported for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The stream the recipes describe.
    pub stream: Vec<u8>,
    /// What the frame index reported for the stream.
    pub indexed: flac_frames::Outcome,
}

/// Writes the stream `data`'s recipes describe and indexes it through
/// [`flac_frames::run`].
///
/// # Panics
///
/// Panics as [`flac_frames::run`] does.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let stream = stream(data);
    let indexed = flac_frames::run(&stream);
    Outcome { stream, indexed }
}

/// The frames `data`'s recipes describe.
fn stream(data: &[u8]) -> Vec<u8> {
    let mut stream = Vec::new();
    let mut rest = data;
    while let Some((recipe, after)) = rest.split_first_chunk::<RECIPE_LEN>() {
        let [flags, codes, layout, n0, n1, n2, n3, n4, b0, b1, r0, r1] = *recipe;
        let variable = flags & 0x01;
        let mask = if variable == 1 {
            0xF_FFFF_FFFF
        } else {
            0x7FFF_FFFF
        };
        let number = u64::from_be_bytes([0, 0, 0, n0, n1, n2, n3, n4]) & mask;
        // The strategy bit is the sync code's last bit, which is clear.
        let mut header = vec![0xFF, 0xF8 + variable, codes, layout];
        header.extend(coded(number));
        match codes >> 4 {
            6 => header.push(b1),
            7 => header.extend([b0, b1]),
            _ => {}
        }
        match codes & 0x0F {
            12 => header.push(r1),
            13 | 14 => header.extend([r0, r1]),
            _ => {}
        }
        header.push(crc8(&header));
        stream.extend(header);
        let (data, tail) = after.split_at(usize::from(flags >> 1).min(after.len()));
        stream.extend(data);
        rest = tail;
    }
    stream
}

/// `number`, of at most 36 bits, as a coded number of one to seven
/// octets.
fn coded(number: u64) -> Vec<u8> {
    if number < 0x80 {
        return vec![low(number)];
    }
    // An n-octet form holds 7 - n bits in its first octet and six in each
    // of the others: 5n + 1 bits in all.
    let octets = (2..=7)
        .find(|&octets| number >> (5 * octets + 1) == 0)
        .unwrap_or(7);
    let continuations = octets - 1;
    // The marker bits and the number's bits never overlap, so adding them
    // sets both.
    let lead = (0xFF << (8 - octets)) + low(number >> (6 * continuations));
    let mut coded = vec![lead];
    for index in (0..continuations).rev() {
        coded.push(0x80 + low(number >> (6 * index) & 0x3F));
    }
    coded
}

/// The low octet of `value`, which the caller has narrowed to fit.
fn low(value: u64) -> u8 {
    u8::try_from(value).unwrap_or(u8::MAX)
}

/// FLAC's header CRC-8 of `octets`: polynomial 0x07, initial value 0.
fn crc8(octets: &[u8]) -> u8 {
    octets.iter().fold(0, |crc, &octet| {
        (0..8).fold(crc ^ octet, |crc: u8, _| {
            if crc & 0x80 == 0 {
                crc << 1
            } else {
                (crc << 1) ^ 0x07
            }
        })
    })
}
