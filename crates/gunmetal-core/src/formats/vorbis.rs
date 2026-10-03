//! Vorbis stream headers (the Vorbis I specification, sections 4.2 and
//! 5.2): the identification header and the framing of the comment header.
//!
//! Each entry point reads one packet that the Ogg layer has already put
//! together, so the offsets in its errors count from the start of the
//! packet. The setup header and audio packets are not read.
//!
//! The comment header's comment block is returned as it is, for the Vorbis
//! comment parser. This module only walks the length of the vendor string
//! and of each comment to find where the block ends, so that it can check
//! the framing bit after it; it decodes none of them. The Opus comment
//! header holds the same block, so the walk is shared with
//! [`opus`](super::opus).
//!
//! Work: [`vorbis_ident`] spends one step of the budget, whatever its
//! input. [`vorbis_comments`] spends one, and one more for each comment it
//! walks, which is at most `packet.len() / 4 + 2` steps.

use std::num::NonZeroU32;

use crate::parse::{Budget, Cursor, Depth, LimitKind, Limits, ParseFault};
use crate::problem::{Arg, Describe, Problem, ProblemCode};
use crate::values::{Channels, SampleRate, ValueError};

/// The packet type and signature that open an identification header.
const IDENT_MAGIC: [u8; 7] = *b"\x01vorbis";
/// The packet type and signature that open a comment header.
const COMMENT_MAGIC: [u8; 7] = *b"\x03vorbis";

/// The facts a Vorbis identification header holds (Vorbis I, section
/// 4.2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VorbisIdent {
    /// The channel count.
    pub channels: Channels,
    /// The sample rate.
    pub sample_rate: SampleRate,
    /// The largest bitrate the encoder promised, in bits per second. The
    /// specification counts a bitrate hint only when it is greater than
    /// zero, so any other value is `None`.
    pub bitrate_maximum: Option<NonZeroU32>,
    /// The average bitrate the encoder aimed for, in bits per second, when
    /// the header gives one.
    pub bitrate_nominal: Option<NonZeroU32>,
    /// The smallest bitrate the encoder promised, in bits per second, when
    /// the header gives one.
    pub bitrate_minimum: Option<NonZeroU32>,
    /// The short block size in samples: a power of two from 64 to 8,192.
    pub short_block: u16,
    /// The long block size in samples: a power of two from 64 to 8,192,
    /// and never less than the short block size.
    pub long_block: u16,
}

/// Why a Vorbis header could not be read. Offsets count from the start of
/// the packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VorbisError {
    /// A failure every parser shares: the packet ended early, or the
    /// parse ran out of budget, nesting depth or tag fields.
    Fault(ParseFault),
    /// The packet does not start with the packet type and `vorbis`
    /// signature of the header asked for.
    Magic {
        /// Where the signature starts.
        offset: u64,
        /// The octets found in its place.
        found: [u8; 7],
    },
    /// A Vorbis version other than 0, the only one Vorbis I defines.
    Version {
        /// Where the version starts.
        offset: u64,
        /// The version found.
        version: u32,
    },
    /// A channel count or sample rate outside its range.
    Value {
        /// Where the value starts.
        offset: u64,
        /// Which value it was and why it was refused.
        error: ValueError,
    },
    /// Block sizes that are not powers of two from 64 to 8,192 samples, or
    /// a short block longer than the long one.
    BlockSizes {
        /// Where the octet holding both sizes is.
        offset: u64,
        /// The octet, the short block's exponent in its low four bits.
        octet: u8,
    },
    /// The framing bit that ends the header is not set.
    Framing {
        /// Where the octet holding the framing bit is.
        offset: u64,
        /// The octet, the framing bit being its lowest bit.
        octet: u8,
    },
}

impl VorbisError {
    /// Where the parser was working when it stopped, in octets from the
    /// start of the packet.
    #[must_use]
    pub const fn offset(&self) -> u64 {
        match *self {
            Self::Fault(fault) => fault.offset(),
            Self::Magic { offset, .. }
            | Self::Version { offset, .. }
            | Self::Value { offset, .. }
            | Self::BlockSizes { offset, .. }
            | Self::Framing { offset, .. } => offset,
        }
    }
}

impl From<ParseFault> for VorbisError {
    fn from(fault: ParseFault) -> Self {
        Self::Fault(fault)
    }
}

impl Describe for VorbisError {
    /// Unreadable Vorbis headers, at the offset where reading stopped.
    fn problem(&self) -> Problem {
        Problem {
            code: ProblemCode::VorbisHeaderUnreadable,
            args: vec![("offset", Arg::Number(self.offset()))],
        }
    }
}

/// Reads a Vorbis identification header: the channel count, the sample
/// rate, the bitrate hints and the block sizes.
///
/// `depth` is the nesting depth of the structure that holds the packet;
/// the header counts as one level below it.
///
/// # Errors
///
/// Returns [`VorbisError::Magic`] for a packet that is not an
/// identification header, the error for the first field Vorbis I forbids,
/// and [`VorbisError::Fault`] when the packet ends early, `depth` is
/// already at its limit or `budget` has no step left.
pub fn vorbis_ident(
    packet: &[u8],
    limits: &Limits,
    budget: &mut Budget,
    depth: Depth,
) -> Result<VorbisIdent, VorbisError> {
    depth.descend(limits, 0)?;
    budget.charge(1, 0)?;
    let mut cursor = Cursor::new(packet);
    magic(&mut cursor, IDENT_MAGIC)?;
    let offset = cursor.offset();
    let version = cursor.u32_le()?;
    if version != 0 {
        return Err(VorbisError::Version { offset, version });
    }
    let offset = cursor.offset();
    let channels =
        Channels::new(cursor.u8()?.into()).map_err(|error| VorbisError::Value { offset, error })?;
    let offset = cursor.offset();
    let sample_rate =
        SampleRate::new(cursor.u32_le()?).map_err(|error| VorbisError::Value { offset, error })?;
    let bitrate_maximum = bitrate(&mut cursor)?;
    let bitrate_nominal = bitrate(&mut cursor)?;
    let bitrate_minimum = bitrate(&mut cursor)?;
    let offset = cursor.offset();
    let octet = cursor.u8()?;
    // Vorbis packs fields from the lowest bit up, so the short block's
    // exponent is the low four bits and the long block's the high four.
    let (short_block, long_block) = match (block_size(octet & 0x0F), block_size(octet / 16)) {
        (Some(short), Some(long)) if short <= long => (short, long),
        _ => return Err(VorbisError::BlockSizes { offset, octet }),
    };
    framing(&mut cursor)?;
    Ok(VorbisIdent {
        channels,
        sample_rate,
        bitrate_maximum,
        bitrate_nominal,
        bitrate_minimum,
        short_block,
        long_block,
    })
}

/// Reads the framing of a Vorbis comment header and returns its comment
/// block: the vendor string and the comments, each behind its length, for
/// the Vorbis comment parser.
///
/// `depth` is the nesting depth of the structure that holds the packet;
/// the header counts as one level below it.
///
/// # Errors
///
/// Returns [`VorbisError::Magic`] for a packet that is not a comment
/// header, [`VorbisError::Framing`] when the framing bit after the block is
/// not set, and [`VorbisError::Fault`] when the packet ends early, the
/// block declares more comments than `limits` allows, `depth` is already at
/// its limit or `budget` runs out.
pub fn vorbis_comments<'a>(
    packet: &'a [u8],
    limits: &Limits,
    budget: &mut Budget,
    depth: Depth,
) -> Result<&'a [u8], VorbisError> {
    depth.descend(limits, 0)?;
    budget.charge(1, 0)?;
    let mut cursor = Cursor::new(packet);
    magic(&mut cursor, COMMENT_MAGIC)?;
    let block = comment_block(&mut cursor, limits, budget)?;
    framing(&mut cursor)?;
    Ok(block)
}

/// Reads the packet type and `vorbis` signature at `cursor` and checks
/// that they are `expected`.
fn magic(cursor: &mut Cursor<'_>, expected: [u8; 7]) -> Result<(), VorbisError> {
    let offset = cursor.offset();
    let found = cursor.array()?;
    if found == expected {
        Ok(())
    } else {
        Err(VorbisError::Magic { offset, found })
    }
}

/// Reads a bitrate hint, which counts only when it is greater than zero.
fn bitrate(cursor: &mut Cursor<'_>) -> Result<Option<NonZeroU32>, ParseFault> {
    cursor.array().map(|octets| {
        u32::try_from(i32::from_le_bytes(octets))
            .ok()
            .and_then(NonZeroU32::new)
    })
}

/// The block size of `2^exponent` samples, when Vorbis I allows it: from
/// 64 to 8,192.
fn block_size(exponent: u8) -> Option<u16> {
    match exponent {
        6 => Some(64),
        7 => Some(128),
        8 => Some(256),
        9 => Some(512),
        10 => Some(1_024),
        11 => Some(2_048),
        12 => Some(4_096),
        13 => Some(8_192),
        _ => None,
    }
}

/// Reads the octet that ends a header and checks its lowest bit, the
/// framing bit.
fn framing(cursor: &mut Cursor<'_>) -> Result<(), VorbisError> {
    let offset = cursor.offset();
    let octet = cursor.u8()?;
    if octet & 1 == 0 {
        return Err(VorbisError::Framing { offset, octet });
    }
    Ok(())
}

/// Takes the Vorbis comment block at `cursor` and moves past it: a 32-bit
/// little-endian vendor string length and the vendor string, a 32-bit
/// comment count, then each comment behind its own 32-bit length (Vorbis
/// I, section 5.2.1). Only the lengths are read.
///
/// Charges one step of `budget` for each comment.
///
/// # Errors
///
/// Returns [`ParseFault::LimitExceeded`] when the count is above the tag
/// field limit, [`ParseFault::Truncated`] when a length runs past the
/// input, and [`ParseFault::BudgetExceeded`] when `budget` runs out. The
/// cursor does not move.
pub(crate) fn comment_block<'a>(
    cursor: &mut Cursor<'a>,
    limits: &Limits,
    budget: &mut Budget,
) -> Result<&'a [u8], ParseFault> {
    let mut walk = *cursor;
    let vendor = walk.u32_le()?;
    walk.skip(vendor.into())?;
    let offset = walk.offset();
    let count = walk.u32_le()?;
    limits.check(LimitKind::TagFields, count.into(), offset)?;
    // Each comment moves the walk on by at least the four octets of its
    // length, or ends it.
    for _ in 0..count {
        budget.charge(1, walk.offset())?;
        let len = walk.u32_le()?;
        walk.skip(len.into())?;
    }
    // The walk read these octets from a copy of `cursor`, so they are
    // there to take.
    cursor.take(walk.offset().saturating_sub(cursor.offset()))
}

/// An independent model of the comment block walk and of truncated
/// fields, written with plain indexing for the tests of this module and of
/// the Opus headers.
#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "the model adds offsets inside inputs of a few hundred octets"
)]
pub(crate) mod model {
    use crate::parse::{Budget, Depth, LimitKind, Limits, ParseFault};

    /// A budget no header can spend.
    pub(crate) fn plenty() -> Budget {
        Budget::for_input(0, 0, u64::MAX)
    }

    /// Steps `budget` has spent, when it started as [`plenty`].
    pub(crate) fn spent(budget: &Budget) -> u64 {
        u64::MAX - budget.remaining()
    }

    /// The depth `level` levels below the root of a container.
    pub(crate) fn depth_at(level: u64) -> Depth {
        (0..level).fold(Depth::CONTAINER_ROOT, |depth, offset| {
            depth.descend(&Limits::DEFAULT, offset).unwrap()
        })
    }

    /// A count or offset as the `u64` a fault holds.
    pub(crate) fn wide(value: usize) -> u64 {
        u64::try_from(value).unwrap()
    }

    /// Runs `work` on a 256 KiB stack, so a parse that recurses too deeply
    /// fails instead of passing on the test runner's larger stack.
    ///
    /// # Panics
    ///
    /// Panics when the thread cannot start or when `work` panics.
    pub(crate) fn on_small_stack<T: Send + 'static>(
        work: impl FnOnce() -> T + Send + 'static,
    ) -> T {
        std::thread::Builder::new()
            .stack_size(262_144)
            .spawn(work)
            .expect("the test thread starts")
            .join()
            .expect("the code under test returned instead of panicking")
    }

    /// The fault for a field of `width` octets at `at` that `bytes` cuts
    /// short.
    pub(crate) fn truncated(bytes: &[u8], at: usize, width: usize) -> ParseFault {
        ParseFault::Truncated {
            offset: wide(at),
            needed: wide(width),
            available: wide(bytes.len() - at),
        }
    }

    /// The `width` octets at `at`, or the fault for the field they would
    /// have been.
    pub(crate) fn field(bytes: &[u8], at: usize, width: usize) -> Result<&[u8], ParseFault> {
        at.checked_add(width)
            .and_then(|end| bytes.get(at..end))
            .ok_or_else(|| truncated(bytes, at, width))
    }

    /// The little-endian 32-bit field at `at`.
    pub(crate) fn u32_at(bytes: &[u8], at: usize) -> Result<u32, ParseFault> {
        field(bytes, at, 4).map(|octets| u32::from_le_bytes(octets.try_into().unwrap()))
    }

    /// Where the comment block that starts at `start` ends, allowing at
    /// most `max_fields` comments.
    pub(crate) fn block_end(
        bytes: &[u8],
        start: usize,
        max_fields: u32,
    ) -> Result<usize, ParseFault> {
        let vendor = usize::try_from(u32_at(bytes, start)?).unwrap();
        field(bytes, start + 4, vendor)?;
        let mut at = start + 4 + vendor;
        let count = u32_at(bytes, at)?;
        if count > max_fields {
            return Err(ParseFault::LimitExceeded {
                limit: LimitKind::TagFields,
                value: count.into(),
                max: max_fields.into(),
                offset: wide(at),
            });
        }
        at += 4;
        for _ in 0..count {
            let len = usize::try_from(u32_at(bytes, at)?).unwrap();
            field(bytes, at + 4, len)?;
            at += 4 + len;
        }
        Ok(at)
    }

    /// The fault for input cut after `cut` octets, in a structure whose
    /// fields start and run as `fields` lists them, in order.
    pub(crate) fn cut_in(fields: &[(usize, usize)], cut: usize) -> ParseFault {
        let &(start, width) = fields
            .iter()
            .find(|&&(start, width)| (start..start + width).contains(&cut))
            .unwrap();
        ParseFault::Truncated {
            offset: wide(start),
            needed: wide(width),
            available: wide(cut - start),
        }
    }
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use super::model::{
        block_end, cut_in, depth_at, field, on_small_stack, plenty, spent, u32_at, wide,
    };
    use super::*;
    use crate::parse::LimitKind;
    use crate::values::Field;
    use gunmetal_testkit::opus::{self as build, VorbisIdent as Ident};
    use proptest::collection::vec;
    use proptest::prelude::*;

    /// Reads `packet` as an identification header at the top level, under
    /// the default limits.
    fn ident(packet: &[u8]) -> Result<VorbisIdent, VorbisError> {
        vorbis_ident(
            packet,
            &Limits::DEFAULT,
            &mut plenty(),
            Depth::CONTAINER_ROOT,
        )
    }

    /// Reads `packet` as a comment header at the top level, under the
    /// default limits.
    fn comments(packet: &[u8]) -> Result<&[u8], VorbisError> {
        vorbis_comments(
            packet,
            &Limits::DEFAULT,
            &mut plenty(),
            Depth::CONTAINER_ROOT,
        )
    }

    /// An independent model of [`vorbis_ident`] under the default limits,
    /// reading each field at its fixed place.
    fn model_ident(bytes: &[u8]) -> Result<VorbisIdent, VorbisError> {
        let found: [u8; 7] = field(bytes, 0, 7)?.try_into().unwrap();
        if found != *b"\x01vorbis" {
            return Err(VorbisError::Magic { offset: 0, found });
        }
        let version = u32_at(bytes, 7)?;
        if version != 0 {
            return Err(VorbisError::Version { offset: 7, version });
        }
        let channels = Channels::new(field(bytes, 11, 1)?[0].into())
            .map_err(|error| VorbisError::Value { offset: 11, error })?;
        let sample_rate = SampleRate::new(u32_at(bytes, 12)?)
            .map_err(|error| VorbisError::Value { offset: 12, error })?;
        let hint = |at| -> Result<Option<NonZeroU32>, ParseFault> {
            let value = i32::from_le_bytes(field(bytes, at, 4)?.try_into().unwrap());
            Ok(NonZeroU32::new(value.max(0).unsigned_abs()))
        };
        let (bitrate_maximum, bitrate_nominal, bitrate_minimum) = (hint(16)?, hint(20)?, hint(24)?);
        let octet = field(bytes, 28, 1)?[0];
        let (short, long) = (octet % 16, octet / 16);
        let allowed = 6..=13;
        if !allowed.contains(&short) || !allowed.contains(&long) || short > long {
            return Err(VorbisError::BlockSizes { offset: 28, octet });
        }
        let framing = field(bytes, 29, 1)?[0];
        if framing % 2 == 0 {
            return Err(VorbisError::Framing {
                offset: 29,
                octet: framing,
            });
        }
        Ok(VorbisIdent {
            channels,
            sample_rate,
            bitrate_maximum,
            bitrate_nominal,
            bitrate_minimum,
            short_block: 1 << short,
            long_block: 1 << long,
        })
    }

    /// An independent model of [`vorbis_comments`] under the default
    /// limits, as the block's start and end.
    fn model_comments(bytes: &[u8]) -> Result<(usize, usize), VorbisError> {
        let found: [u8; 7] = field(bytes, 0, 7)?.try_into().unwrap();
        if found != *b"\x03vorbis" {
            return Err(VorbisError::Magic { offset: 0, found });
        }
        let end = block_end(bytes, 7, 4_096)?;
        let framing = field(bytes, end, 1)?[0];
        if framing % 2 == 0 {
            return Err(VorbisError::Framing {
                offset: wide(end),
                octet: framing,
            });
        }
        Ok((7, end))
    }

    /// Checks that [`vorbis_ident`] and its model both read `bytes` as
    /// `expected`, so every literal case also pins the model.
    #[track_caller]
    fn check_ident(bytes: &[u8], expected: Result<VorbisIdent, VorbisError>) {
        assert_eq!(ident(bytes), expected, "parser on {bytes:02X?}");
        assert_eq!(model_ident(bytes), expected, "model on {bytes:02X?}");
    }

    /// Checks that [`vorbis_comments`] and its model both read `bytes` as
    /// `expected`, given as the block's start and end.
    #[track_caller]
    fn check_comments(bytes: &[u8], expected: Result<(usize, usize), VorbisError>) {
        assert_eq!(
            comments(bytes),
            expected.map(|(start, end)| &bytes[start..end]),
            "parser on {bytes:02X?}"
        );
        assert_eq!(model_comments(bytes), expected, "model on {bytes:02X?}");
    }

    fn channels(count: u32) -> Channels {
        Channels::new(count).unwrap()
    }

    fn rate(hz: u32) -> SampleRate {
        SampleRate::new(hz).unwrap()
    }

    fn bitrate(bits: u32) -> Option<NonZeroU32> {
        NonZeroU32::new(bits)
    }

    /// What [`Ident::stereo`] holds.
    fn stereo() -> VorbisIdent {
        VorbisIdent {
            channels: channels(2),
            sample_rate: rate(44_100),
            bitrate_maximum: None,
            bitrate_nominal: bitrate(112_000),
            bitrate_minimum: None,
            short_block: 256,
            long_block: 2_048,
        }
    }

    /// The fields of an identification header, as starts and widths.
    const IDENT_FIELDS: [(usize, usize); 9] = [
        (0, 7),
        (7, 4),
        (11, 1),
        (12, 4),
        (16, 4),
        (20, 4),
        (24, 4),
        (28, 1),
        (29, 1),
    ];

    #[test]
    fn reads_the_stereo_header_libvorbis_writes() {
        check_ident(&Ident::stereo().to_bytes(), Ok(stereo()));
    }

    #[test]
    fn keeps_each_bitrate_hint_only_when_it_is_greater_than_zero() {
        let header = Ident {
            bitrate_maximum: 320_000,
            bitrate_nominal: i32::MAX,
            bitrate_minimum: 1,
            ..Ident::stereo()
        };
        check_ident(
            &header.to_bytes(),
            Ok(VorbisIdent {
                bitrate_maximum: bitrate(320_000),
                bitrate_nominal: bitrate(2_147_483_647),
                bitrate_minimum: bitrate(1),
                ..stereo()
            }),
        );
        let header = Ident {
            bitrate_maximum: -1,
            bitrate_nominal: 0,
            bitrate_minimum: i32::MIN,
            ..Ident::stereo()
        };
        check_ident(
            &header.to_bytes(),
            Ok(VorbisIdent {
                bitrate_maximum: None,
                bitrate_nominal: None,
                bitrate_minimum: None,
                ..stereo()
            }),
        );
    }

    #[test]
    fn accepts_only_vorbis_version_0() {
        for version in [1, 0x0100_0000, u32::MAX] {
            let header = Ident {
                version,
                ..Ident::stereo()
            };
            check_ident(
                &header.to_bytes(),
                Err(VorbisError::Version { offset: 7, version }),
            );
        }
    }

    #[test]
    fn refuses_a_channel_count_of_zero_and_accepts_255() {
        let header = Ident {
            channels: 0,
            ..Ident::stereo()
        };
        check_ident(
            &header.to_bytes(),
            Err(VorbisError::Value {
                offset: 11,
                error: ValueError::OutOfRange {
                    field: Field::Channels,
                    value: 0,
                },
            }),
        );
        let header = Ident {
            channels: 255,
            ..Ident::stereo()
        };
        check_ident(
            &header.to_bytes(),
            Ok(VorbisIdent {
                channels: channels(255),
                ..stereo()
            }),
        );
    }

    #[test]
    fn refuses_a_sample_rate_outside_1_to_768_000_hertz() {
        for hz in [0, 768_001, u32::MAX] {
            let header = Ident {
                sample_rate: hz,
                ..Ident::stereo()
            };
            check_ident(
                &header.to_bytes(),
                Err(VorbisError::Value {
                    offset: 12,
                    error: ValueError::OutOfRange {
                        field: Field::SampleRate,
                        value: hz.into(),
                    },
                }),
            );
        }
        for hz in [1, 768_000] {
            let header = Ident {
                sample_rate: hz,
                ..Ident::stereo()
            };
            check_ident(
                &header.to_bytes(),
                Ok(VorbisIdent {
                    sample_rate: rate(hz),
                    ..stereo()
                }),
            );
        }
    }

    #[test]
    fn accepts_exactly_the_block_sizes_vorbis_i_allows() {
        let mut accepted = Vec::new();
        for short in 0..16 {
            for long in 0..16 {
                let header = Ident {
                    block_exponents: [short, long],
                    ..Ident::stereo()
                };
                let bytes = header.to_bytes();
                let read = ident(&bytes);
                assert_eq!(model_ident(&bytes), read, "{short} {long}");
                match read {
                    Ok(read) => accepted.push((read.short_block, read.long_block)),
                    Err(error) => assert_eq!(
                        error,
                        VorbisError::BlockSizes {
                            offset: 28,
                            octet: long * 16 + short,
                        }
                    ),
                }
            }
        }
        let sizes = [64, 128, 256, 512, 1_024, 2_048, 4_096, 8_192];
        let mut expected = Vec::new();
        for (index, &short) in sizes.iter().enumerate() {
            for &long in &sizes[index..] {
                expected.push((short, long));
            }
        }
        assert_eq!(accepted, expected);
    }

    #[test]
    fn refuses_a_header_whose_framing_bit_is_unset() {
        for framing in [0x00, 0xFE] {
            let header = Ident {
                framing,
                ..Ident::stereo()
            };
            check_ident(
                &header.to_bytes(),
                Err(VorbisError::Framing {
                    offset: 29,
                    octet: framing,
                }),
            );
        }
        // The framing bit is the lowest; the rest of its octet is padding.
        let header = Ident {
            framing: 0xFF,
            ..Ident::stereo()
        };
        check_ident(&header.to_bytes(), Ok(stereo()));
    }

    #[test]
    fn ignores_octets_after_the_framing_octet() {
        let mut bytes = Ident::stereo().to_bytes();
        bytes.extend([0x00, 0xFF, 0x05]);
        check_ident(&bytes, Ok(stereo()));
    }

    #[test]
    fn refuses_a_packet_that_is_not_an_identification_header() {
        check_ident(
            &build::vorbis_comments(b"", &[], 1),
            Err(VorbisError::Magic {
                offset: 0,
                found: *b"\x03vorbis",
            }),
        );
        let mut bytes = Ident::stereo().to_bytes();
        bytes[1] = b'V';
        check_ident(
            &bytes,
            Err(VorbisError::Magic {
                offset: 0,
                found: *b"\x01Vorbis",
            }),
        );
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn every_truncation_of_an_identification_header_names_the_field_it_cut() {
        let bytes = Ident::stereo().to_bytes();
        assert_eq!(bytes.len(), 30);
        for cut in 0..bytes.len() {
            let expected = Err(VorbisError::Fault(cut_in(&IDENT_FIELDS, cut)));
            check_ident(&bytes[..cut], expected);
        }
    }

    /// Verifies: SEC-MED-005
    #[test]
    fn an_identification_header_is_one_level_below_its_container() {
        let bytes = Ident::stereo().to_bytes();
        let read = |depth| vorbis_ident(&bytes, &Limits::DEFAULT, &mut plenty(), depth);
        assert_eq!(read(depth_at(31)), Ok(stereo()));
        assert_eq!(
            read(depth_at(32)),
            Err(VorbisError::Fault(ParseFault::TooDeep {
                limit: LimitKind::ContainerDepth,
                depth: 33,
                max: 32,
                offset: 0,
            }))
        );
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn an_identification_header_spends_exactly_one_step() {
        let bytes = Ident::stereo().to_bytes();
        let mut budget = Budget::for_input(0, 0, 1);
        let read = vorbis_ident(&bytes, &Limits::DEFAULT, &mut budget, Depth::CONTAINER_ROOT);
        assert_eq!((read, budget.remaining()), (Ok(stereo()), 0));
        assert_eq!(
            vorbis_ident(&bytes, &Limits::DEFAULT, &mut budget, Depth::CONTAINER_ROOT),
            Err(VorbisError::Fault(ParseFault::BudgetExceeded { offset: 0 }))
        );
    }

    /// A comment header holding a vendor string and two comments, and
    /// where its block starts and ends.
    fn two_comments() -> (Vec<u8>, (usize, usize)) {
        let bytes = build::vorbis_comments(b"Xiph", &[b"TITLE=A", b""], 0x01);
        // 7 octets of signature, then 4 + 4 of vendor, 4 of count, 4 + 7
        // and 4 + 0 of comments, then the framing octet.
        (bytes, (7, 34))
    }

    #[test]
    fn returns_the_comment_block_before_the_framing_bit() {
        let (bytes, block) = two_comments();
        assert_eq!(bytes.len(), 35);
        check_comments(&bytes, Ok(block));
        assert_eq!(
            comments(&bytes),
            Ok(build::comment_block(b"Xiph", &[b"TITLE=A", b""]).as_slice())
        );
        check_comments(&build::vorbis_comments(b"", &[], 0x01), Ok((7, 15)));
    }

    #[test]
    fn ignores_octets_after_the_framing_octet_of_a_comment_header() {
        let (mut bytes, block) = two_comments();
        bytes.extend([0x00, 0x05]);
        check_comments(&bytes, Ok(block));
    }

    #[test]
    fn refuses_a_comment_header_whose_framing_bit_is_unset_or_missing() {
        for framing in [0x00, 0x02, 0xFE] {
            check_comments(
                &build::vorbis_comments(b"Xiph", &[b"TITLE=A", b""], framing),
                Err(VorbisError::Framing {
                    offset: 34,
                    octet: framing,
                }),
            );
        }
        let (bytes, _) = two_comments();
        check_comments(
            &bytes[..34],
            Err(VorbisError::Fault(ParseFault::Truncated {
                offset: 34,
                needed: 1,
                available: 0,
            })),
        );
    }

    #[test]
    fn refuses_a_packet_that_is_not_a_comment_header() {
        check_comments(
            &Ident::stereo().to_bytes(),
            Err(VorbisError::Magic {
                offset: 0,
                found: *b"\x01vorbis",
            }),
        );
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn every_truncation_of_a_comment_header_names_the_field_it_cut() {
        let (bytes, _) = two_comments();
        let fields = [
            (0, 7),
            (7, 4),
            (11, 4),
            (15, 4),
            (19, 4),
            (23, 7),
            (30, 4),
            (34, 1),
        ];
        for cut in 0..bytes.len() {
            let expected = Err(VorbisError::Fault(cut_in(&fields, cut)));
            check_comments(&bytes[..cut], expected);
        }
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn refuses_a_count_of_u32_max_in_a_20_octet_block() {
        let mut bytes = b"\x03vorbis".to_vec();
        bytes.extend(
            b"\x00\x00\x00\x00\xFF\xFF\xFF\xFF\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",
        );
        check_comments(
            &bytes,
            Err(VorbisError::Fault(ParseFault::LimitExceeded {
                limit: LimitKind::TagFields,
                value: 4_294_967_295,
                max: 4_096,
                offset: 11,
            })),
        );
    }

    /// Verifies: SEC-MED-006, SEC-MED-007, SEC-MED-008
    #[test]
    fn walks_4_096_empty_comments_one_step_each_and_refuses_4_097() {
        let empty = |count| (0..count).map(|_| &b""[..]).collect::<Vec<_>>();
        let bytes = build::vorbis_comments(b"", &empty(4_096), 0x01);
        let mut budget = plenty();
        let read = vorbis_comments(&bytes, &Limits::DEFAULT, &mut budget, Depth::CONTAINER_ROOT);
        assert_eq!(read, Ok(&bytes[7..bytes.len() - 1]));
        assert_eq!(spent(&budget), 4_097);
        check_comments(
            &build::vorbis_comments(b"", &empty(4_097), 0x01),
            Err(VorbisError::Fault(ParseFault::LimitExceeded {
                limit: LimitKind::TagFields,
                value: 4_097,
                max: 4_096,
                offset: 11,
            })),
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn counts_comments_against_the_tag_field_limit_given() {
        let limits = Limits::DEFAULT
            .with_override(LimitKind::TagFields, 2)
            .unwrap();
        let read = |bytes: &[u8]| {
            vorbis_comments(bytes, &limits, &mut plenty(), Depth::CONTAINER_ROOT).map(<[u8]>::len)
        };
        let (bytes, _) = two_comments();
        assert_eq!(read(&bytes), Ok(27));
        let three = build::vorbis_comments(b"Xiph", &[b"A=1", b"B=2", b"C=3"], 0x01);
        assert_eq!(
            read(&three),
            Err(VorbisError::Fault(ParseFault::LimitExceeded {
                limit: LimitKind::TagFields,
                value: 3,
                max: 2,
                offset: 15,
            }))
        );
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn a_comment_header_spends_one_step_and_one_per_comment() {
        let (bytes, (start, end)) = two_comments();
        let mut budget = Budget::for_input(0, 0, 3);
        let read = vorbis_comments(&bytes, &Limits::DEFAULT, &mut budget, Depth::CONTAINER_ROOT);
        assert_eq!((read, budget.remaining()), (Ok(&bytes[start..end]), 0));
        // Two steps reach the second comment, at offset 30, and stop there.
        let mut budget = Budget::for_input(0, 0, 2);
        assert_eq!(
            vorbis_comments(&bytes, &Limits::DEFAULT, &mut budget, Depth::CONTAINER_ROOT),
            Err(VorbisError::Fault(ParseFault::BudgetExceeded {
                offset: 30
            }))
        );
        let mut budget = Budget::for_input(0, 0, 0);
        assert_eq!(
            vorbis_comments(&bytes, &Limits::DEFAULT, &mut budget, Depth::CONTAINER_ROOT),
            Err(VorbisError::Fault(ParseFault::BudgetExceeded { offset: 0 }))
        );
    }

    /// Verifies: SEC-MED-005
    #[test]
    fn a_comment_header_is_one_level_below_its_container() {
        let (bytes, (start, end)) = two_comments();
        let read = |depth| vorbis_comments(&bytes, &Limits::DEFAULT, &mut plenty(), depth);
        assert_eq!(read(depth_at(31)), Ok(&bytes[start..end]));
        assert_eq!(
            read(depth_at(32)),
            Err(VorbisError::Fault(ParseFault::TooDeep {
                limit: LimitKind::ContainerDepth,
                depth: 33,
                max: 32,
                offset: 0,
            }))
        );
    }

    #[test]
    fn takes_a_comment_block_at_absolute_offsets_and_moves_past_it() {
        let block = build::comment_block(b"v", &[b"A=b"]);
        let mut bytes = block.clone();
        bytes.push(0xAA);
        let mut cursor = Cursor::at(&bytes, 100);
        assert_eq!(
            comment_block(&mut cursor, &Limits::DEFAULT, &mut plenty()),
            Ok(block.as_slice())
        );
        assert_eq!((cursor.offset(), cursor.rest()), (116, &[0xAA][..]));
        let start = Cursor::at(&block[..15], 100);
        let mut cursor = start;
        assert_eq!(
            comment_block(&mut cursor, &Limits::DEFAULT, &mut plenty()),
            Err(ParseFault::Truncated {
                offset: 113,
                needed: 3,
                available: 2,
            })
        );
        assert_eq!(cursor, start);
    }

    /// One error of each kind, at the offsets [`OFFSETS`] lists.
    fn one_of_each() -> [VorbisError; 6] {
        [
            VorbisError::Fault(ParseFault::BudgetExceeded { offset: 3 }),
            VorbisError::Magic {
                offset: 5,
                found: [0; 7],
            },
            VorbisError::Version {
                offset: 7,
                version: 1,
            },
            VorbisError::Value {
                offset: 11,
                error: ValueError::OutOfRange {
                    field: Field::Channels,
                    value: 0,
                },
            },
            VorbisError::BlockSizes {
                offset: 28,
                octet: 0,
            },
            VorbisError::Framing {
                offset: 29,
                octet: 0,
            },
        ]
    }

    /// Where each error of [`one_of_each`] happened.
    const OFFSETS: [u64; 6] = [3, 5, 7, 11, 28, 29];

    #[test]
    fn reports_where_each_kind_of_error_happened() {
        assert_eq!(one_of_each().map(|error| error.offset()), OFFSETS);
    }

    #[test]
    fn describes_each_error_as_unreadable_vorbis_headers_at_its_offset() {
        let problems: Vec<Problem> = one_of_each().iter().map(Describe::problem).collect();
        let expected: Vec<Problem> = OFFSETS
            .iter()
            .map(|&offset| Problem {
                code: ProblemCode::VorbisHeaderUnreadable,
                args: vec![("offset", Arg::Number(offset))],
            })
            .collect();
        assert_eq!(problems, expected);
    }

    /// Identification headers the builder writes from fields that are
    /// mostly valid and sometimes not, sometimes cut short and sometimes
    /// followed by more octets.
    fn ident_like() -> impl Strategy<Value = Vec<u8>> {
        (
            prop_oneof![4 => Just(0_u32), 1 => any::<u32>()],
            prop_oneof![4 => 1_u8..=8, 1 => any::<u8>()],
            prop_oneof![4 => Just(48_000_u32), 1 => 0_u32..800_000, 1 => any::<u32>()],
            (any::<i32>(), any::<i32>(), any::<i32>()),
            prop_oneof![4 => (6_u8..=13, 6_u8..=13), 1 => (0_u8..16, 0_u8..16)],
            prop_oneof![4 => Just(1_u8), 1 => any::<u8>()],
            prop_oneof![3 => Just(None), 1 => (0_usize..30).prop_map(Some)],
            vec(any::<u8>(), 0..4),
        )
            .prop_map(
                |(version, channels, sample_rate, bitrates, (short, long), framing, cut, tail)| {
                    let (bitrate_maximum, bitrate_nominal, bitrate_minimum) = bitrates;
                    let mut bytes = Ident {
                        version,
                        channels,
                        sample_rate,
                        bitrate_maximum,
                        bitrate_nominal,
                        bitrate_minimum,
                        block_exponents: [short, long],
                        framing,
                    }
                    .to_bytes();
                    bytes.extend(tail);
                    bytes.truncate(cut.unwrap_or(bytes.len()));
                    bytes
                },
            )
    }

    /// Comment headers the builder writes, sometimes cut short and
    /// sometimes followed by more octets, and the signature followed by
    /// arbitrary octets, whose lengths are mostly too long.
    fn comments_like() -> impl Strategy<Value = Vec<u8>> {
        prop_oneof![
            (
                vec(any::<u8>(), 0..6),
                vec(vec(any::<u8>(), 0..6), 0..5),
                prop_oneof![3 => Just(1_u8), 1 => any::<u8>()],
                prop_oneof![3 => Just(None), 1 => (0_usize..48).prop_map(Some)],
                vec(any::<u8>(), 0..4),
            )
                .prop_map(|(vendor, list, framing, cut, tail)| {
                    let list: Vec<&[u8]> = list.iter().map(Vec::as_slice).collect();
                    let mut bytes = build::vorbis_comments(&vendor, &list, framing);
                    bytes.extend(tail);
                    bytes.truncate(cut.unwrap_or(bytes.len()));
                    bytes
                }),
            vec(any::<u8>(), 0..32).prop_map(|rest| [b"\x03vorbis".as_slice(), &rest].concat()),
        ]
    }

    proptest! {
        /// Verifies: SEC-MED-001, SEC-MED-007, SEC-TM-032
        #[test]
        fn reads_any_identification_header_exactly_as_the_model_does(
            bytes in prop_oneof![vec(any::<u8>(), 0..40), ident_like()],
        ) {
            let expected = model_ident(&bytes);
            let (read, steps) = on_small_stack(move || {
                let mut budget = plenty();
                let read = vorbis_ident(&bytes, &Limits::DEFAULT, &mut budget, Depth::CONTAINER_ROOT);
                (read, spent(&budget))
            });
            prop_assert_eq!(read, expected);
            prop_assert_eq!(steps, 1);
        }

        /// Verifies: SEC-MED-001, SEC-MED-007, SEC-MED-008, SEC-TM-032
        #[test]
        fn reads_any_comment_header_exactly_as_the_model_does(
            bytes in prop_oneof![vec(any::<u8>(), 0..40), comments_like()],
        ) {
            let expected = model_comments(&bytes).map(|(start, end)| bytes[start..end].to_vec());
            let len = wide(bytes.len());
            let (read, steps) = on_small_stack(move || {
                let mut budget = plenty();
                let read = vorbis_comments(&bytes, &Limits::DEFAULT, &mut budget, Depth::CONTAINER_ROOT)
                    .map(<[u8]>::to_vec);
                (read, spent(&budget))
            });
            prop_assert_eq!(read, expected);
            prop_assert!(steps <= len / 4 + 2, "{} steps for {} octets", steps, len);
        }
    }
}
