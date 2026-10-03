//! Opus stream headers in Ogg (RFC 7845, section 5): the identification
//! header `OpusHead` and the framing of the comment header `OpusTags`.
//!
//! Each entry point reads one packet that the Ogg layer has already put
//! together, so the offsets in its errors count from the start of the
//! packet. Audio packets are not read.
//!
//! The comment header holds the same comment block as a Vorbis comment
//! header, and [`opus_tags`] returns it as it is, for the Vorbis comment
//! parser. Only the lengths in it are walked, to find where it ends, so
//! the padding `opusenc` leaves after the comments, or binary data an
//! editor must keep, is not part of it.
//!
//! Work: [`opus_head`] spends one step of the budget, whatever its input:
//! its work is bounded by the 276 octets the longest header holds.
//! [`opus_tags`] spends one, and one more for each comment it walks, which
//! is at most `packet.len() / 4 + 2` steps.

use super::vorbis::comment_block;
use crate::parse::{Budget, Cursor, Depth, Limits, ParseFault};
use crate::problem::{Arg, Describe, Problem, ProblemCode};
use crate::values::{Channels, GainDb, SampleRate, ValueError};

/// The signature that opens an identification header.
const HEAD_MAGIC: [u8; 8] = *b"OpusHead";
/// The signature that opens a comment header.
const TAGS_MAGIC: [u8; 8] = *b"OpusTags";
/// The highest version octet of major version 0.
const MAX_VERSION: u8 = 15;
/// The mapping entry for an output channel that plays silence.
const SILENT: u8 = 255;

/// The facts an Opus identification header holds (RFC 7845, section
/// 5.1).
#[derive(Debug, Clone, PartialEq)]
pub struct OpusHead {
    /// The version octet: 1 for RFC 7845, and anything from 0 to 15,
    /// which the RFC asks readers to treat as compatible.
    pub version: u8,
    /// The output channel count.
    pub channels: Channels,
    /// Samples at 48 kHz to discard from the start of the decoded stream
    /// (MUS-069).
    pub pre_skip: u16,
    /// The sample rate of the original input. It is informational only:
    /// Opus always decodes at 48 kHz. A value outside its range, such as
    /// zero, is dropped with the reason.
    pub input_sample_rate: Result<SampleRate, ValueError>,
    /// The gain every decoder applies to its output (MUS-085).
    pub output_gain: GainDb,
    /// How decoded channels map to output channels.
    pub mapping: ChannelMapping,
}

/// How the channels of the Opus streams in a packet map to output
/// channels (RFC 7845, section 5.1.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelMapping {
    /// Family 0: one stream, mono or stereo, with no table.
    Single,
    /// Family 1 (up to eight channels in Vorbis order) or 255 (up to 255
    /// channels with no defined layout), with its table.
    Table {
        /// The mapping family.
        family: u8,
        /// The number of Opus streams in each packet, at least 1.
        streams: u8,
        /// How many of those streams are coupled, carrying two channels;
        /// at most `streams`.
        coupled: u8,
        /// For each output channel, the decoded channel it plays: less
        /// than `streams + coupled`, or 255 for silence.
        mapping: Vec<u8>,
    },
    /// A family from 2 to 254. RFC 8486 gives families 2 and 3 to
    /// ambisonics, with a different table for family 3, so the table is
    /// left unread.
    Reserved {
        /// The mapping family.
        family: u8,
    },
}

/// Why an Opus header could not be read. Offsets count from the start of
/// the packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpusError {
    /// A failure every parser shares: the packet ended early, or the
    /// parse ran out of budget, nesting depth or tag fields.
    Fault(ParseFault),
    /// The packet does not start with the signature of the header asked
    /// for.
    Magic {
        /// Where the signature starts.
        offset: u64,
        /// The octets found in its place.
        found: [u8; 8],
    },
    /// A major version other than 0: a version octet of 16 or more.
    Version {
        /// Where the version octet is.
        offset: u64,
        /// The version octet.
        version: u8,
    },
    /// A channel count of zero.
    Value {
        /// Where the channel count is.
        offset: u64,
        /// Which value it was and why it was refused.
        error: ValueError,
    },
    /// More channels than the mapping family allows: 2 for family 0 and
    /// 8 for family 1.
    FamilyChannels {
        /// Where the channel count is.
        offset: u64,
        /// The mapping family.
        family: u8,
        /// The channel count.
        channels: u8,
    },
    /// A mapping table with no streams.
    NoStreams {
        /// Where the stream count is.
        offset: u64,
    },
    /// More coupled streams than streams.
    Coupled {
        /// Where the coupled stream count is.
        offset: u64,
        /// The stream count.
        streams: u8,
        /// The coupled stream count.
        coupled: u8,
    },
    /// More than 255 decoded channels, which no mapping can address.
    DecodedChannels {
        /// Where the stream count is.
        offset: u64,
        /// The stream count.
        streams: u8,
        /// The coupled stream count.
        coupled: u8,
    },
    /// A mapping entry that names no decoded channel and is not 255.
    MappingEntry {
        /// Where the entry is.
        offset: u64,
        /// The entry.
        entry: u8,
        /// The number of decoded channels, `streams + coupled`.
        decoded: u16,
    },
}

impl OpusError {
    /// Where the parser was working when it stopped, in octets from the
    /// start of the packet.
    #[must_use]
    pub const fn offset(&self) -> u64 {
        match *self {
            Self::Fault(fault) => fault.offset(),
            Self::Magic { offset, .. }
            | Self::Version { offset, .. }
            | Self::Value { offset, .. }
            | Self::FamilyChannels { offset, .. }
            | Self::NoStreams { offset }
            | Self::Coupled { offset, .. }
            | Self::DecodedChannels { offset, .. }
            | Self::MappingEntry { offset, .. } => offset,
        }
    }
}

impl From<ParseFault> for OpusError {
    fn from(fault: ParseFault) -> Self {
        Self::Fault(fault)
    }
}

impl Describe for OpusError {
    /// Unreadable Opus headers, at the offset where reading stopped.
    fn problem(&self) -> Problem {
        Problem {
            code: ProblemCode::OpusHeaderUnreadable,
            args: vec![("offset", Arg::Number(self.offset()))],
        }
    }
}

/// Reads an Opus identification header, `OpusHead`: the channel count,
/// pre-skip, input sample rate, output gain and channel mapping. Octets
/// after the header are ignored, as later minor versions may add fields.
///
/// `depth` is the nesting depth of the structure that holds the packet;
/// the header counts as one level below it.
///
/// # Errors
///
/// Returns [`OpusError::Magic`] for a packet that is not an identification
/// header, the error for the first field RFC 7845 forbids, and
/// [`OpusError::Fault`] when the packet ends early, `depth` is already at
/// its limit or `budget` has no step left.
pub fn opus_head(
    packet: &[u8],
    limits: &Limits,
    budget: &mut Budget,
    depth: Depth,
) -> Result<OpusHead, OpusError> {
    depth.descend(limits, 0)?;
    budget.charge(1, 0)?;
    let mut cursor = Cursor::new(packet);
    magic(&mut cursor, HEAD_MAGIC)?;
    let offset = cursor.offset();
    let version = cursor.u8()?;
    if version > MAX_VERSION {
        return Err(OpusError::Version { offset, version });
    }
    let channels_at = cursor.offset();
    let count = cursor.u8()?;
    let channels = Channels::new(count.into()).map_err(|error| OpusError::Value {
        offset: channels_at,
        error,
    })?;
    let pre_skip = cursor.u16_le()?;
    let input_sample_rate = SampleRate::new(cursor.u32_le()?);
    let output_gain = GainDb::from_q7_8(cursor.array().map(i16::from_le_bytes)?);
    let family = cursor.u8()?;
    let head = |mapping| OpusHead {
        version,
        channels,
        pre_skip,
        input_sample_rate,
        output_gain,
        mapping,
    };
    let most = match family {
        0 => 2,
        1 => 8,
        u8::MAX => u8::MAX,
        _ => return Ok(head(ChannelMapping::Reserved { family })),
    };
    if count > most {
        return Err(OpusError::FamilyChannels {
            offset: channels_at,
            family,
            channels: count,
        });
    }
    if family == 0 {
        return Ok(head(ChannelMapping::Single));
    }
    table(&mut cursor, family, count).map(head)
}

/// Reads the mapping table of a header with `channels` output channels
/// in mapping family `family`.
fn table(cursor: &mut Cursor<'_>, family: u8, channels: u8) -> Result<ChannelMapping, OpusError> {
    let streams_at = cursor.offset();
    let streams = cursor.u8()?;
    if streams == 0 {
        return Err(OpusError::NoStreams { offset: streams_at });
    }
    let offset = cursor.offset();
    let coupled = cursor.u8()?;
    if coupled > streams {
        return Err(OpusError::Coupled {
            offset,
            streams,
            coupled,
        });
    }
    // Two octets add up to at most 510.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "two u8 values as u16 add up to at most 510"
    )]
    let decoded = u16::from(streams) + u16::from(coupled);
    if decoded > u8::MAX.into() {
        return Err(OpusError::DecodedChannels {
            offset: streams_at,
            streams,
            coupled,
        });
    }
    let table_at = cursor.offset();
    let mapping = cursor.take(channels.into())?;
    let named = |&(_, &entry): &(u64, &u8)| entry == SILENT || u16::from(entry) < decoded;
    if let Some((offset, &entry)) = (table_at..).zip(mapping).find(|pair| !named(pair)) {
        return Err(OpusError::MappingEntry {
            offset,
            entry,
            decoded,
        });
    }
    Ok(ChannelMapping::Table {
        family,
        streams,
        coupled,
        mapping: mapping.to_vec(),
    })
}

/// Reads the framing of an Opus comment header, `OpusTags`, and returns
/// its comment block: the vendor string and the comments, each behind its
/// length, for the Vorbis comment parser.
///
/// `depth` is the nesting depth of the structure that holds the packet;
/// the header counts as one level below it.
///
/// # Errors
///
/// Returns [`OpusError::Magic`] for a packet that is not a comment header,
/// and [`OpusError::Fault`] when the packet ends early, the block declares
/// more comments than `limits` allows, `depth` is already at its limit or
/// `budget` runs out.
pub fn opus_tags<'a>(
    packet: &'a [u8],
    limits: &Limits,
    budget: &mut Budget,
    depth: Depth,
) -> Result<&'a [u8], OpusError> {
    depth.descend(limits, 0)?;
    budget.charge(1, 0)?;
    let mut cursor = Cursor::new(packet);
    magic(&mut cursor, TAGS_MAGIC)?;
    Ok(comment_block(&mut cursor, limits, budget)?)
}

/// Reads the signature at `cursor` and checks that it is `expected`.
fn magic(cursor: &mut Cursor<'_>, expected: [u8; 8]) -> Result<(), OpusError> {
    let offset = cursor.offset();
    let found = cursor.array()?;
    if found == expected {
        Ok(())
    } else {
        Err(OpusError::Magic { offset, found })
    }
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use super::super::vorbis::model::{
        block_end, cut_in, depth_at, field, on_small_stack, plenty, spent, u32_at, wide,
    };
    use super::*;
    use crate::parse::LimitKind;
    use crate::values::Field;
    use gunmetal_testkit::opus::{self as build, MappingTable};
    use proptest::collection::vec;
    use proptest::prelude::*;

    /// Reads `packet` as an identification header at the top level, under
    /// the default limits.
    fn head(packet: &[u8]) -> Result<OpusHead, OpusError> {
        opus_head(
            packet,
            &Limits::DEFAULT,
            &mut plenty(),
            Depth::CONTAINER_ROOT,
        )
    }

    /// Reads `packet` as a comment header at the top level, under the
    /// default limits.
    fn tags(packet: &[u8]) -> Result<&[u8], OpusError> {
        opus_tags(
            packet,
            &Limits::DEFAULT,
            &mut plenty(),
            Depth::CONTAINER_ROOT,
        )
    }

    /// An independent model of [`opus_head`], reading each field at its
    /// fixed place.
    fn model_head(bytes: &[u8]) -> Result<OpusHead, OpusError> {
        let found: [u8; 8] = field(bytes, 0, 8)?.try_into().unwrap();
        if found != *b"OpusHead" {
            return Err(OpusError::Magic { offset: 0, found });
        }
        let version = field(bytes, 8, 1)?[0];
        if version >= 16 {
            return Err(OpusError::Version { offset: 8, version });
        }
        let count = field(bytes, 9, 1)?[0];
        let channels =
            Channels::new(count.into()).map_err(|error| OpusError::Value { offset: 9, error })?;
        let pre_skip = u16::from_le_bytes(field(bytes, 10, 2)?.try_into().unwrap());
        let input_sample_rate = SampleRate::new(u32_at(bytes, 12)?);
        let gain = i16::from_le_bytes(field(bytes, 16, 2)?.try_into().unwrap());
        let output_gain = GainDb::new(f32::from(gain) / 256.0).unwrap();
        let family = field(bytes, 18, 1)?[0];
        let head = |mapping| OpusHead {
            version,
            channels,
            pre_skip,
            input_sample_rate,
            output_gain,
            mapping,
        };
        let most = match family {
            0 => 2,
            1 => 8,
            255 => 255,
            _ => return Ok(head(ChannelMapping::Reserved { family })),
        };
        if count > most {
            return Err(OpusError::FamilyChannels {
                offset: 9,
                family,
                channels: count,
            });
        }
        if family == 0 {
            return Ok(head(ChannelMapping::Single));
        }
        let streams = field(bytes, 19, 1)?[0];
        if streams == 0 {
            return Err(OpusError::NoStreams { offset: 19 });
        }
        let coupled = field(bytes, 20, 1)?[0];
        if coupled > streams {
            return Err(OpusError::Coupled {
                offset: 20,
                streams,
                coupled,
            });
        }
        let decoded = u16::from(streams) + u16::from(coupled);
        if decoded > 255 {
            return Err(OpusError::DecodedChannels {
                offset: 19,
                streams,
                coupled,
            });
        }
        let mapping = field(bytes, 21, count.into())?;
        if let Some(index) = mapping
            .iter()
            .position(|&entry| entry != 255 && u16::from(entry) >= decoded)
        {
            return Err(OpusError::MappingEntry {
                offset: wide(21 + index),
                entry: mapping[index],
                decoded,
            });
        }
        Ok(head(ChannelMapping::Table {
            family,
            streams,
            coupled,
            mapping: mapping.to_vec(),
        }))
    }

    /// An independent model of [`opus_tags`] under the default limits, as
    /// the block's start and end.
    fn model_tags(bytes: &[u8]) -> Result<(usize, usize), OpusError> {
        let found: [u8; 8] = field(bytes, 0, 8)?.try_into().unwrap();
        if found != *b"OpusTags" {
            return Err(OpusError::Magic { offset: 0, found });
        }
        Ok((8, block_end(bytes, 8, 4_096)?))
    }

    /// Checks that [`opus_head`] and its model both read `bytes` as
    /// `expected`, so every literal case also pins the model.
    #[track_caller]
    fn check_head(bytes: &[u8], expected: &Result<OpusHead, OpusError>) {
        assert_eq!(&head(bytes), expected, "parser on {bytes:02X?}");
        assert_eq!(&model_head(bytes), expected, "model on {bytes:02X?}");
    }

    /// Checks that [`opus_tags`] and its model both read `bytes` as
    /// `expected`, given as the block's start and end.
    #[track_caller]
    fn check_tags(bytes: &[u8], expected: Result<(usize, usize), OpusError>) {
        assert_eq!(
            tags(bytes),
            expected.map(|(start, end)| &bytes[start..end]),
            "parser on {bytes:02X?}"
        );
        assert_eq!(model_tags(bytes), expected, "model on {bytes:02X?}");
    }

    fn channels(count: u32) -> Channels {
        Channels::new(count).unwrap()
    }

    fn gain(db: f32) -> GainDb {
        GainDb::new(db).unwrap()
    }

    /// What [`build::OpusHead::stereo`] holds.
    fn stereo() -> OpusHead {
        OpusHead {
            version: 1,
            channels: channels(2),
            pre_skip: 312,
            input_sample_rate: SampleRate::new(48_000),
            output_gain: gain(0.0),
            mapping: ChannelMapping::Single,
        }
    }

    /// libopus's 5.1 header in Vorbis channel order: four streams, two of
    /// them coupled, at 44.1 kHz with -1.5 dB of output gain.
    fn surround() -> build::OpusHead {
        build::OpusHead {
            version: 1,
            channels: 6,
            pre_skip: 3_840,
            input_sample_rate: 44_100,
            output_gain: -384,
            family: 1,
            table: Some(MappingTable {
                streams: 4,
                coupled: 2,
                mapping: vec![0, 4, 1, 2, 3, 5],
            }),
        }
    }

    /// What [`surround`] holds.
    fn surround_read() -> OpusHead {
        OpusHead {
            version: 1,
            channels: channels(6),
            pre_skip: 3_840,
            input_sample_rate: SampleRate::new(44_100),
            output_gain: gain(-1.5),
            mapping: ChannelMapping::Table {
                family: 1,
                streams: 4,
                coupled: 2,
                mapping: vec![0, 4, 1, 2, 3, 5],
            },
        }
    }

    /// A header with a mapping table and the fields of [`stereo`] around
    /// it.
    fn with_table(family: u8, streams: u8, coupled: u8, mapping: &[u8]) -> Vec<u8> {
        build::OpusHead {
            channels: u8::try_from(mapping.len()).unwrap(),
            family,
            table: Some(MappingTable {
                streams,
                coupled,
                mapping: mapping.to_vec(),
            }),
            ..build::OpusHead::stereo()
        }
        .to_bytes()
    }

    /// What [`with_table`] holds when it is valid.
    fn table_read(family: u8, streams: u8, coupled: u8, mapping: &[u8]) -> OpusHead {
        OpusHead {
            channels: channels(u32::try_from(mapping.len()).unwrap()),
            mapping: ChannelMapping::Table {
                family,
                streams,
                coupled,
                mapping: mapping.to_vec(),
            },
            ..stereo()
        }
    }

    #[test]
    fn reads_the_stereo_header_opusenc_writes() {
        check_head(&build::OpusHead::stereo().to_bytes(), &Ok(stereo()));
    }

    #[test]
    fn reads_mapping_family_1_with_its_table() {
        check_head(&surround().to_bytes(), &Ok(surround_read()));
    }

    #[test]
    fn reads_mapping_family_255_with_its_table() {
        check_head(
            &with_table(255, 2, 1, &[0, 2, 255]),
            &Ok(table_read(255, 2, 1, &[0, 2, 255])),
        );
        // As many channels as one octet can count, each its own stream.
        let mapping: Vec<u8> = (0..=254).collect();
        check_head(
            &with_table(255, 255, 0, &mapping),
            &Ok(table_read(255, 255, 0, &mapping)),
        );
    }

    #[test]
    fn accepts_versions_0_to_15_and_refuses_16() {
        for version in [0, 1, 15] {
            let header = build::OpusHead {
                version,
                ..build::OpusHead::stereo()
            };
            check_head(
                &header.to_bytes(),
                &Ok(OpusHead {
                    version,
                    ..stereo()
                }),
            );
        }
        for version in [16, 255] {
            let header = build::OpusHead {
                version,
                ..build::OpusHead::stereo()
            };
            check_head(
                &header.to_bytes(),
                &Err(OpusError::Version { offset: 8, version }),
            );
        }
    }

    #[test]
    fn refuses_a_channel_count_of_zero() {
        let header = build::OpusHead {
            channels: 0,
            ..build::OpusHead::stereo()
        };
        check_head(
            &header.to_bytes(),
            &Err(OpusError::Value {
                offset: 9,
                error: ValueError::OutOfRange {
                    field: Field::Channels,
                    value: 0,
                },
            }),
        );
    }

    #[test]
    fn refuses_more_channels_than_the_family_allows() {
        let mono = build::OpusHead {
            channels: 1,
            ..build::OpusHead::stereo()
        };
        check_head(
            &mono.to_bytes(),
            &Ok(OpusHead {
                channels: channels(1),
                ..stereo()
            }),
        );
        let three = build::OpusHead {
            channels: 3,
            ..build::OpusHead::stereo()
        };
        check_head(
            &three.to_bytes(),
            &Err(OpusError::FamilyChannels {
                offset: 9,
                family: 0,
                channels: 3,
            }),
        );
        let eight = [0, 1, 2, 3, 4, 5, 6, 7];
        check_head(
            &with_table(1, 5, 3, &eight),
            &Ok(table_read(1, 5, 3, &eight)),
        );
        check_head(
            &with_table(1, 5, 4, &[0, 1, 2, 3, 4, 5, 6, 7, 8]),
            &Err(OpusError::FamilyChannels {
                offset: 9,
                family: 1,
                channels: 9,
            }),
        );
    }

    #[test]
    fn reports_a_reserved_family_without_reading_its_table() {
        for family in [2, 3, 254] {
            let header = build::OpusHead {
                channels: 255,
                family,
                ..build::OpusHead::stereo()
            };
            check_head(
                &header.to_bytes(),
                &Ok(OpusHead {
                    channels: channels(255),
                    mapping: ChannelMapping::Reserved { family },
                    ..stereo()
                }),
            );
        }
    }

    #[test]
    fn refuses_a_table_with_no_streams() {
        check_head(
            &with_table(1, 0, 0, &[0]),
            &Err(OpusError::NoStreams { offset: 19 }),
        );
    }

    #[test]
    fn refuses_more_coupled_streams_than_streams() {
        check_head(
            &with_table(1, 1, 2, &[0, 1]),
            &Err(OpusError::Coupled {
                offset: 20,
                streams: 1,
                coupled: 2,
            }),
        );
        check_head(
            &with_table(1, 1, 1, &[0, 1]),
            &Ok(table_read(1, 1, 1, &[0, 1])),
        );
    }

    #[test]
    fn refuses_more_than_255_decoded_channels() {
        check_head(
            &with_table(255, 200, 56, &[0]),
            &Err(OpusError::DecodedChannels {
                offset: 19,
                streams: 200,
                coupled: 56,
            }),
        );
        check_head(
            &with_table(255, 200, 55, &[254]),
            &Ok(table_read(255, 200, 55, &[254])),
        );
    }

    #[test]
    fn refuses_a_mapping_entry_that_names_no_decoded_channel() {
        check_head(
            &with_table(1, 1, 1, &[0, 2]),
            &Err(OpusError::MappingEntry {
                offset: 22,
                entry: 2,
                decoded: 2,
            }),
        );
        check_head(
            &with_table(1, 1, 1, &[3, 2]),
            &Err(OpusError::MappingEntry {
                offset: 21,
                entry: 3,
                decoded: 2,
            }),
        );
        // 255 plays silence.
        check_head(
            &with_table(1, 1, 1, &[1, 255]),
            &Ok(table_read(1, 1, 1, &[1, 255])),
        );
    }

    #[test]
    fn keeps_any_output_gain_negative_or_positive() {
        for (raw, db) in [
            (i16::MIN, -128.0),
            (-1, -0.003_906_25),
            (256, 1.0),
            // 32,767 / 256 dB, exact in an f32 but longer than its
            // precision as a decimal.
            (i16::MAX, 127.0 + 255.0 / 256.0),
        ] {
            let header = build::OpusHead {
                output_gain: raw,
                ..build::OpusHead::stereo()
            };
            check_head(
                &header.to_bytes(),
                &Ok(OpusHead {
                    output_gain: gain(db),
                    ..stereo()
                }),
            );
        }
    }

    #[test]
    fn keeps_a_pre_skip_larger_than_a_typical_file() {
        // 65,535 samples at 48 kHz is 1.37 seconds, longer than a short
        // sound effect; what to play of such a file is the player's call.
        let header = build::OpusHead {
            pre_skip: u16::MAX,
            ..build::OpusHead::stereo()
        };
        check_head(
            &header.to_bytes(),
            &Ok(OpusHead {
                pre_skip: 65_535,
                ..stereo()
            }),
        );
    }

    #[test]
    fn drops_an_input_sample_rate_outside_its_range_with_the_reason() {
        for hz in [0, 768_001, u32::MAX] {
            let header = build::OpusHead {
                input_sample_rate: hz,
                ..build::OpusHead::stereo()
            };
            check_head(
                &header.to_bytes(),
                &Ok(OpusHead {
                    input_sample_rate: Err(ValueError::OutOfRange {
                        field: Field::SampleRate,
                        value: hz.into(),
                    }),
                    ..stereo()
                }),
            );
        }
    }

    #[test]
    fn ignores_octets_after_the_header() {
        for (header, read) in [
            (build::OpusHead::stereo(), stereo()),
            (surround(), surround_read()),
        ] {
            let mut bytes = header.to_bytes();
            bytes.extend([0x00, 0xFF, 0x07]);
            check_head(&bytes, &Ok(read));
        }
    }

    #[test]
    fn refuses_a_packet_that_is_not_an_identification_header() {
        check_head(
            &build::opus_tags(b"", &[]),
            &Err(OpusError::Magic {
                offset: 0,
                found: *b"OpusTags",
            }),
        );
        let mut bytes = build::OpusHead::stereo().to_bytes();
        bytes[7] = b'D';
        check_head(
            &bytes,
            &Err(OpusError::Magic {
                offset: 0,
                found: *b"OpusHeaD",
            }),
        );
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn every_truncation_of_an_identification_header_names_the_field_it_cut() {
        let fields = [
            (0, 8),
            (8, 1),
            (9, 1),
            (10, 2),
            (12, 4),
            (16, 2),
            (18, 1),
            (19, 1),
            (20, 1),
            (21, 6),
        ];
        for header in [build::OpusHead::stereo(), surround()] {
            let bytes = header.to_bytes();
            for cut in 0..bytes.len() {
                let expected = Err(OpusError::Fault(cut_in(&fields, cut)));
                check_head(&bytes[..cut], &expected);
            }
        }
    }

    /// Verifies: SEC-MED-005
    #[test]
    fn an_identification_header_is_one_level_below_its_container() {
        let bytes = surround().to_bytes();
        let read = |depth| opus_head(&bytes, &Limits::DEFAULT, &mut plenty(), depth);
        assert_eq!(read(depth_at(31)), Ok(surround_read()));
        assert_eq!(
            read(depth_at(32)),
            Err(OpusError::Fault(ParseFault::TooDeep {
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
        let bytes = surround().to_bytes();
        let mut budget = Budget::for_input(0, 0, 1);
        let read = opus_head(&bytes, &Limits::DEFAULT, &mut budget, Depth::CONTAINER_ROOT);
        assert_eq!((read, budget.remaining()), (Ok(surround_read()), 0));
        assert_eq!(
            opus_head(&bytes, &Limits::DEFAULT, &mut budget, Depth::CONTAINER_ROOT),
            Err(OpusError::Fault(ParseFault::BudgetExceeded { offset: 0 }))
        );
    }

    /// A comment header holding a vendor string and two comments, and
    /// where its block starts and ends.
    fn two_comments() -> (Vec<u8>, (usize, usize)) {
        let bytes = build::opus_tags(b"ab", &[b"A=b", b"C"]);
        // 8 octets of signature, then 4 + 2 of vendor, 4 of count, and
        // 4 + 3 and 4 + 1 of comments.
        (bytes, (8, 30))
    }

    #[test]
    fn returns_the_comment_block_of_a_comment_header() {
        let (bytes, block) = two_comments();
        assert_eq!(bytes.len(), 30);
        check_tags(&bytes, Ok(block));
        assert_eq!(
            tags(&bytes),
            Ok(build::comment_block(b"ab", &[b"A=b", b"C"]).as_slice())
        );
        check_tags(&build::opus_tags(b"", &[]), Ok((8, 16)));
    }

    #[test]
    fn leaves_out_the_padding_and_data_after_the_comments() {
        // opusenc leaves 512 octets of zeros by default. Data whose first
        // octet has its lowest bit set is kept by editors, and is not
        // comments either.
        for after in [[0_u8; 512].as_slice(), &[0x01, 0xAA]] {
            let (mut bytes, block) = two_comments();
            bytes.extend(after);
            check_tags(&bytes, Ok(block));
        }
    }

    #[test]
    fn refuses_a_packet_that_is_not_a_comment_header() {
        check_tags(
            &build::OpusHead::stereo().to_bytes(),
            Err(OpusError::Magic {
                offset: 0,
                found: *b"OpusHead",
            }),
        );
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn every_truncation_of_a_comment_header_names_the_field_it_cut() {
        let (bytes, _) = two_comments();
        let fields = [
            (0, 8),
            (8, 4),
            (12, 2),
            (14, 4),
            (18, 4),
            (22, 3),
            (25, 4),
            (29, 1),
        ];
        for cut in 0..bytes.len() {
            let expected = Err(OpusError::Fault(cut_in(&fields, cut)));
            check_tags(&bytes[..cut], expected);
        }
    }

    /// Verifies: SEC-MED-006, SEC-TM-032
    #[test]
    fn refuses_a_count_of_u32_max_in_a_20_octet_block() {
        let mut bytes = b"OpusTags".to_vec();
        bytes.extend(
            b"\x00\x00\x00\x00\xFF\xFF\xFF\xFF\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",
        );
        check_tags(
            &bytes,
            Err(OpusError::Fault(ParseFault::LimitExceeded {
                limit: LimitKind::TagFields,
                value: 4_294_967_295,
                max: 4_096,
                offset: 12,
            })),
        );
    }

    /// Verifies: SEC-MED-006
    #[test]
    fn counts_comments_against_the_tag_field_limit_given() {
        let limits = Limits::DEFAULT
            .with_override(LimitKind::TagFields, 1)
            .unwrap();
        let read = |bytes: &[u8]| {
            opus_tags(bytes, &limits, &mut plenty(), Depth::CONTAINER_ROOT).map(<[u8]>::len)
        };
        assert_eq!(read(&build::opus_tags(b"ab", &[b"A=b"])), Ok(17));
        let (bytes, _) = two_comments();
        assert_eq!(
            read(&bytes),
            Err(OpusError::Fault(ParseFault::LimitExceeded {
                limit: LimitKind::TagFields,
                value: 2,
                max: 1,
                offset: 14,
            }))
        );
    }

    /// Verifies: SEC-MED-007
    #[test]
    fn a_comment_header_spends_one_step_and_one_per_comment() {
        let (bytes, (start, end)) = two_comments();
        let mut budget = Budget::for_input(0, 0, 3);
        let read = opus_tags(&bytes, &Limits::DEFAULT, &mut budget, Depth::CONTAINER_ROOT);
        assert_eq!((read, budget.remaining()), (Ok(&bytes[start..end]), 0));
        // Two steps reach the second comment, at offset 25, and stop there.
        let mut budget = Budget::for_input(0, 0, 2);
        assert_eq!(
            opus_tags(&bytes, &Limits::DEFAULT, &mut budget, Depth::CONTAINER_ROOT),
            Err(OpusError::Fault(ParseFault::BudgetExceeded { offset: 25 }))
        );
        let mut budget = Budget::for_input(0, 0, 0);
        assert_eq!(
            opus_tags(&bytes, &Limits::DEFAULT, &mut budget, Depth::CONTAINER_ROOT),
            Err(OpusError::Fault(ParseFault::BudgetExceeded { offset: 0 }))
        );
    }

    /// Verifies: SEC-MED-005
    #[test]
    fn a_comment_header_is_one_level_below_its_container() {
        let (bytes, (start, end)) = two_comments();
        let read = |depth| opus_tags(&bytes, &Limits::DEFAULT, &mut plenty(), depth);
        assert_eq!(read(depth_at(31)), Ok(&bytes[start..end]));
        assert_eq!(
            read(depth_at(32)),
            Err(OpusError::Fault(ParseFault::TooDeep {
                limit: LimitKind::ContainerDepth,
                depth: 33,
                max: 32,
                offset: 0,
            }))
        );
    }

    /// One error of each kind, at the offsets [`OFFSETS`] lists.
    fn one_of_each() -> [OpusError; 9] {
        [
            OpusError::Fault(ParseFault::BudgetExceeded { offset: 3 }),
            OpusError::Magic {
                offset: 5,
                found: [0; 8],
            },
            OpusError::Version {
                offset: 8,
                version: 16,
            },
            OpusError::Value {
                offset: 9,
                error: ValueError::OutOfRange {
                    field: Field::Channels,
                    value: 0,
                },
            },
            OpusError::FamilyChannels {
                offset: 10,
                family: 0,
                channels: 3,
            },
            OpusError::NoStreams { offset: 19 },
            OpusError::Coupled {
                offset: 20,
                streams: 1,
                coupled: 2,
            },
            OpusError::DecodedChannels {
                offset: 21,
                streams: 200,
                coupled: 56,
            },
            OpusError::MappingEntry {
                offset: 22,
                entry: 2,
                decoded: 2,
            },
        ]
    }

    /// Where each error of [`one_of_each`] happened.
    const OFFSETS: [u64; 9] = [3, 5, 8, 9, 10, 19, 20, 21, 22];

    #[test]
    fn reports_where_each_kind_of_error_happened() {
        assert_eq!(one_of_each().map(|error| error.offset()), OFFSETS);
    }

    #[test]
    fn describes_each_error_as_unreadable_opus_headers_at_its_offset() {
        let problems: Vec<Problem> = one_of_each().iter().map(Describe::problem).collect();
        let expected: Vec<Problem> = OFFSETS
            .iter()
            .map(|&offset| Problem {
                code: ProblemCode::OpusHeaderUnreadable,
                args: vec![("offset", Arg::Number(offset))],
            })
            .collect();
        assert_eq!(problems, expected);
    }

    /// Identification headers the builder writes from fields that are
    /// mostly valid and sometimes not, sometimes cut short and sometimes
    /// followed by more octets.
    fn head_like() -> impl Strategy<Value = Vec<u8>> {
        let table = (
            prop_oneof![8 => 1_u8..=8, 1 => Just(0_u8), 1 => 9_u8..=12],
            prop_oneof![4 => 1_u8..=4, 1 => any::<u8>()],
            prop_oneof![4 => 0_u8..=4, 1 => any::<u8>()],
        )
            .prop_flat_map(|(count, streams, coupled)| {
                let decoded = (u16::from(streams) + u16::from(coupled)).clamp(1, 255);
                let decoded = u8::try_from(decoded).unwrap();
                let entry = prop_oneof![4 => 0..decoded, 1 => Just(255_u8), 1 => any::<u8>()];
                (
                    Just(count),
                    Just(streams),
                    Just(coupled),
                    vec(entry, usize::from(count)),
                )
            });
        (
            prop_oneof![4 => 0_u8..=16, 1 => any::<u8>()],
            prop_oneof![Just(0_u8), Just(1_u8), Just(255_u8), any::<u8>()],
            table,
            (any::<u16>(), any::<u32>(), any::<i16>()),
            prop_oneof![3 => Just(None), 1 => (0_usize..32).prop_map(Some)],
            vec(any::<u8>(), 0..4),
        )
            .prop_map(
                |(version, family, (count, streams, coupled, mapping), fixed, cut, tail)| {
                    let (pre_skip, input_sample_rate, output_gain) = fixed;
                    let mut bytes = build::OpusHead {
                        version,
                        channels: if family == 0 { count.min(3) } else { count },
                        pre_skip,
                        input_sample_rate,
                        output_gain,
                        family,
                        table: (family != 0).then_some(MappingTable {
                            streams,
                            coupled,
                            mapping,
                        }),
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
    fn tags_like() -> impl Strategy<Value = Vec<u8>> {
        prop_oneof![
            (
                vec(any::<u8>(), 0..6),
                vec(vec(any::<u8>(), 0..6), 0..5),
                prop_oneof![3 => Just(None), 1 => (0_usize..48).prop_map(Some)],
                vec(any::<u8>(), 0..4),
            )
                .prop_map(|(vendor, list, cut, tail)| {
                    let list: Vec<&[u8]> = list.iter().map(Vec::as_slice).collect();
                    let mut bytes = build::opus_tags(&vendor, &list);
                    bytes.extend(tail);
                    bytes.truncate(cut.unwrap_or(bytes.len()));
                    bytes
                }),
            vec(any::<u8>(), 0..32).prop_map(|rest| [b"OpusTags".as_slice(), &rest].concat()),
        ]
    }

    proptest! {
        /// Verifies: SEC-MED-001, SEC-MED-007, SEC-TM-032
        #[test]
        fn reads_any_identification_header_exactly_as_the_model_does(
            bytes in prop_oneof![vec(any::<u8>(), 0..40), head_like()],
        ) {
            let expected = model_head(&bytes);
            let (read, steps) = on_small_stack(move || {
                let mut budget = plenty();
                let read = opus_head(&bytes, &Limits::DEFAULT, &mut budget, Depth::CONTAINER_ROOT);
                (read, spent(&budget))
            });
            prop_assert_eq!(read, expected);
            prop_assert_eq!(steps, 1);
        }

        /// Verifies: SEC-MED-001, SEC-MED-007, SEC-MED-008, SEC-TM-032
        #[test]
        fn reads_any_comment_header_exactly_as_the_model_does(
            bytes in prop_oneof![vec(any::<u8>(), 0..40), tags_like()],
        ) {
            let expected = model_tags(&bytes).map(|(start, end)| bytes[start..end].to_vec());
            let len = wide(bytes.len());
            let (read, steps) = on_small_stack(move || {
                let mut budget = plenty();
                let read = opus_tags(&bytes, &Limits::DEFAULT, &mut budget, Depth::CONTAINER_ROOT)
                    .map(<[u8]>::to_vec);
                (read, spent(&budget))
            });
            prop_assert_eq!(read, expected);
            prop_assert!(steps <= len / 4 + 2, "{} steps for {} octets", steps, len);
        }
    }
}
