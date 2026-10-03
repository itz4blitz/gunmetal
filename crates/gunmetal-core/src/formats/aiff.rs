//! AIFF and AIFF-C files.
//!
//! An AIFF file is an EA IFF 85 form (Apple's Audio Interchange File Format
//! 1.3, 1989): the ID `FORM`, a 32-bit big-endian size that counts
//! everything after it, the form type `AIFF`, then chunks laid out as in
//! RIFF ([`super::riff`]) but with big-endian sizes. AIFF-C (Apple's draft
//! of 1991) is the same form with the type `AIFC` and a compression type in
//! its `COMM` chunk.
//!
//! [`Aiff`] walks the chunks as [`Wav`](super::riff::Wav) does, reading the
//! first 30 octets at each: the chunk's header and the 22 octets of an
//! AIFF-C `COMM` chunk's fields. It reports:
//!
//! - the first `COMM` chunk, as a typed [`AiffFormat`]. Its sample rate is
//!   an 80-bit IEEE 754 extended number, converted with checked arithmetic
//!   and rounded down to the hertz;
//! - where the samples of the first `SSND` chunk are, after the offset its
//!   first field gives, and the block size its second gives;
//! - the body of the first `ID3 ` or `id3 ` chunk, an `ID3v2` tag, as a
//!   raw range for the tag parser.
//!
//! A later chunk with one of these IDs is skipped like any other. Damaged
//! files, limits, the step budget and nesting depth work as they do for
//! WAV files, and the same budget is always enough: [`STEPS_PER_OCTET`] ×
//! the file's length + [`STEPS_FIXED`]. An AIFF file whose sample count is
//! zero may leave out its `SSND` chunk, but holds nothing to play, so this
//! parser refuses it.

use super::riff::{ByteRange, Chunk, Halt, Layout, Parser, Walk, fault_reason, problem};
use crate::parse::{Budget, Cursor, Limits, ParseFault, SansIo, Step, Window};
use crate::problem::{Describe, Problem, ProblemCode};
use crate::values::{BitDepth, Channels, Field, SampleRate, ValueError};

pub use super::riff::{STEPS_FIXED, STEPS_PER_OCTET};

/// The ID of an IFF form.
const FORM: [u8; 4] = *b"FORM";
/// The form type of an AIFF file.
const AIFF: [u8; 4] = *b"AIFF";
/// The form type of an AIFF-C file.
const AIFC: [u8; 4] = *b"AIFC";
/// The chunk that describes the samples.
const COMM: [u8; 4] = *b"COMM";
/// The chunk that holds the samples.
const SSND: [u8; 4] = *b"SSND";
/// An `ID3v2` tag, as most writers name its chunk.
const ID3: [u8; 4] = *b"ID3 ";
/// An `ID3v2` tag, as some writers name its chunk.
const ID3_LOWER: [u8; 4] = *b"id3 ";

/// Octets of an AIFF `COMM` chunk's fields.
const COMM_FIELDS: u64 = 18;
/// Octets of an AIFF-C `COMM` chunk's fields before the compression name.
const COMM_C_FIELDS: u64 = 22;
/// Octets of an `SSND` chunk's fields before its samples.
const SSND_FIELDS: u64 = 8;

/// The sign bit of an extended number's first two octets.
const SIGN: u16 = 0x8000;
/// The exponent in an extended number's first two octets; all ones is an
/// infinity or not a number.
const EXPONENT: u16 = 0x7FFF;
/// The exponent at which an extended number equals its 64-bit significand:
/// the bias, 16,383, plus the 63 bits after the binary point.
const POINT: u16 = 16_446;

/// What a `COMM` chunk says about the samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiffFormat {
    /// The compression type of an AIFF-C file, such as `NONE`, `sowt` or
    /// `fl32`; `None` in an AIFF file, whose samples are big-endian
    /// integer PCM.
    pub compression: Option<[u8; 4]>,
    /// How many channels there are.
    pub channels: Channels,
    /// How many sample frames the `SSND` chunk holds, as written.
    pub sample_frames: u32,
    /// Bits in one sample, when from 1 to 64.
    pub sample_size: Option<BitDepth>,
    /// Frames a second.
    pub sample_rate: SampleRate,
}

/// What an AIFF or AIFF-C file holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiffFile {
    /// The first `COMM` chunk.
    pub format: AiffFormat,
    /// The samples of the first `SSND` chunk, after its offset, ending
    /// where the form does if the chunk runs past it.
    pub sound: ByteRange,
    /// The size of the blocks the samples are aligned to, as written; zero
    /// when they are not.
    pub block_size: u32,
    /// The body of the first `ID3 ` or `id3 ` chunk: an `ID3v2` tag.
    pub id3: Option<ByteRange>,
    /// Why the walk stopped before the end of the form, if it did.
    pub stopped: Option<ParseFault>,
}

/// Why an AIFF or AIFF-C file cannot be played.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiffError {
    /// A failure every parser shares, before the format and the samples
    /// were both found.
    Fault(ParseFault),
    /// The file does not start with `FORM`.
    NotForm {
        /// The first four octets.
        found: [u8; 4],
    },
    /// The form type, at offset 8, is not `AIFF` or `AIFC`.
    NotAiff {
        /// The form type.
        found: [u8; 4],
    },
    /// A chunk is shorter than the fixed fields it must hold.
    Short {
        /// The chunk's ID.
        id: [u8; 4],
        /// Where the chunk starts.
        offset: u64,
        /// The size of its body.
        len: u64,
        /// The size its fields need.
        needed: u64,
    },
    /// A `COMM` chunk holds a channel count or sample rate out of range.
    Value {
        /// Where the chunk starts.
        offset: u64,
        /// Which value, and why.
        error: ValueError,
    },
    /// An `SSND` chunk's samples would start after the chunk ends.
    SoundOffset {
        /// Where the chunk starts.
        offset: u64,
        /// The offset its first field gives.
        sound_offset: u32,
        /// The size of its body.
        len: u64,
    },
    /// The form has no chunk with this ID, which a playable file needs.
    Missing {
        /// The chunk's ID.
        id: [u8; 4],
    },
}

impl From<ParseFault> for AiffError {
    fn from(fault: ParseFault) -> Self {
        Self::Fault(fault)
    }
}

impl Describe for AiffError {
    /// An unreadable AIFF file, with the reason and where it was found.
    fn problem(&self) -> Problem {
        let (reason, offset) = match *self {
            Self::Fault(fault) => (fault_reason(fault), Some(fault.offset())),
            Self::NotForm { .. } => ("not_form", Some(0)),
            Self::NotAiff { .. } => ("not_aiff", Some(8)),
            Self::Short { offset, .. } => ("short_chunk", Some(offset)),
            Self::Value { offset, .. } => ("bad_value", Some(offset)),
            Self::SoundOffset { offset, .. } => ("sound_offset", Some(offset)),
            Self::Missing { .. } => ("missing_chunk", None),
        };
        problem(ProblemCode::AiffUnreadable, reason, offset)
    }
}

/// The parser for one AIFF or AIFF-C file: see the module documentation.
#[derive(Debug)]
pub struct Aiff(Parser<AiffChunks>);

impl Aiff {
    /// A parser for one file, under `limits`, spending `budget`.
    /// `Budget::for_input(file_len, STEPS_PER_OCTET, STEPS_FIXED)` is always
    /// enough.
    #[must_use]
    pub const fn new(limits: &Limits, budget: Budget) -> Self {
        Self(Parser::new(limits, budget))
    }
}

impl SansIo for Aiff {
    type Output = Result<AiffFile, AiffError>;

    fn resume(&mut self, window: Window<'_>) -> Step<Self::Output> {
        self.0.resume(window)
    }
}

/// What the walk over an AIFF file has found so far.
#[derive(Debug, Clone, Copy)]
struct AiffChunks {
    /// Whether the form is AIFF-C, whose `COMM` chunk names a compression.
    compressed: bool,
    /// The first `COMM` chunk.
    format: Option<AiffFormat>,
    /// The first `SSND` chunk's samples and block size.
    sound: Option<(ByteRange, u32)>,
    /// The first `ID3v2` tag.
    id3: Option<ByteRange>,
}

impl Layout for AiffChunks {
    type File = AiffFile;
    type Error = AiffError;
    const PEEK: u32 = 30;

    fn size(octets: [u8; 4]) -> u32 {
        u32::from_be_bytes(octets)
    }

    fn open(header: [u8; 12]) -> Result<(Self, u64), AiffError> {
        let [i0, i1, i2, i3, s0, s1, s2, s3, t0, t1, t2, t3] = header;
        let found = [i0, i1, i2, i3];
        if found != FORM {
            return Err(AiffError::NotForm { found });
        }
        let compressed = match [t0, t1, t2, t3] {
            AIFF => false,
            AIFC => true,
            found => return Err(AiffError::NotAiff { found }),
        };
        let chunks = Self {
            compressed,
            format: None,
            sound: None,
            id3: None,
        };
        Ok((chunks, u64::from(u32::from_be_bytes([s0, s1, s2, s3]))))
    }

    fn chunk(
        &mut self,
        chunk: Chunk,
        mut body: Cursor<'_>,
        walk: Walk,
    ) -> Result<Walk, Halt<AiffError>> {
        let size = u64::from(chunk.size);
        match chunk.id {
            COMM if self.format.is_none() => {
                let needed = if self.compressed {
                    COMM_C_FIELDS
                } else {
                    COMM_FIELDS
                };
                short(COMM, size, needed, walk)?;
                let next = walk.past(size)?;
                self.format = Some(common(self.compressed, &mut body, walk)?);
                Ok(next)
            }
            SSND if self.sound.is_none() => {
                short(SSND, size, SSND_FIELDS, walk)?;
                let [o0, o1, o2, o3, b0, b1, b2, b3] = body.array()?;
                let sound_offset = u32::from_be_bytes([o0, o1, o2, o3]);
                let skip = u64::from(sound_offset).saturating_add(SSND_FIELDS);
                if skip > size {
                    return Err(Halt::Fail(AiffError::SoundOffset {
                        offset: walk.at(),
                        sound_offset,
                        len: size,
                    }));
                }
                // A chunk cut short by the end of the form keeps the samples
                // that are there, which start no later than that end.
                let present = size.min(walk.room());
                let sound = ByteRange {
                    offset: walk.body().saturating_add(skip.min(present)),
                    len: present.saturating_sub(skip),
                };
                self.sound = Some((sound, u32::from_be_bytes([b0, b1, b2, b3])));
                Ok(walk.past(size)?)
            }
            ID3 | ID3_LOWER if self.id3.is_none() => {
                let next = walk.past(size)?;
                self.id3 = Some(ByteRange {
                    offset: walk.body(),
                    len: size,
                });
                Ok(next)
            }
            _ => Ok(walk.past(size)?),
        }
    }

    fn finish(self, stopped: Option<ParseFault>) -> Result<AiffFile, AiffError> {
        let missing = |id| stopped.map_or(AiffError::Missing { id }, AiffError::Fault);
        let format = self.format.ok_or_else(|| missing(COMM))?;
        let (sound, block_size) = self.sound.ok_or_else(|| missing(SSND))?;
        Ok(AiffFile {
            format,
            sound,
            block_size,
            id3: self.id3,
            stopped,
        })
    }
}

/// Refuses the chunk `walk` is at, of ID `id` and size `size`, when it is
/// shorter than the `needed` octets of its fields.
fn short(id: [u8; 4], size: u64, needed: u64, walk: Walk) -> Result<(), Halt<AiffError>> {
    if size < needed {
        return Err(Halt::Fail(AiffError::Short {
            id,
            offset: walk.at(),
            len: size,
            needed,
        }));
    }
    Ok(())
}

/// A typed value from the `COMM` chunk `walk` is at, or the error for it.
fn value<T>(read: Result<T, ValueError>, walk: Walk) -> Result<T, Halt<AiffError>> {
    read.map_err(|error| {
        Halt::Fail(AiffError::Value {
            offset: walk.at(),
            error,
        })
    })
}

/// Reads a `COMM` chunk's fields from `body`, with the compression type of
/// an AIFF-C chunk when `compressed`.
fn common(
    compressed: bool,
    body: &mut Cursor<'_>,
    walk: Walk,
) -> Result<AiffFormat, Halt<AiffError>> {
    let [c0, c1, f0, f1, f2, f3, s0, s1, rate @ ..] = body.array::<18>()?;
    let compression = if compressed {
        Some(body.array()?)
    } else {
        None
    };
    Ok(AiffFormat {
        compression,
        channels: value(Channels::new(u32::from(u16::from_be_bytes([c0, c1]))), walk)?,
        sample_frames: u32::from_be_bytes([f0, f1, f2, f3]),
        sample_size: BitDepth::new(u32::from(u16::from_be_bytes([s0, s1]))).ok(),
        sample_rate: value(extended_rate(rate), walk)?,
    })
}

/// The sample rate an 80-bit IEEE 754 extended number holds, rounded down
/// to the hertz: a sign bit, a 15-bit exponent biased by 16,383 and a
/// 64-bit significand with its integer bit written out, all big-endian.
///
/// The value is the significand times two to the power of the exponent
/// less [`POINT`]. Below that point the fraction is shifted out; above it
/// the significand is shifted left while it still fits 64 bits, and is
/// `u64::MAX` beyond, which no sample rate reaches.
fn extended_rate(octets: [u8; 10]) -> Result<SampleRate, ValueError> {
    let [high, low, significand @ ..] = octets;
    let sign_and_exponent = u16::from_be_bytes([high, low]);
    let significand = u64::from_be_bytes(significand);
    let exponent = sign_and_exponent & EXPONENT;
    if (sign_and_exponent & SIGN) != 0 || exponent == EXPONENT {
        // A negative rate, an infinity or not a number is no rate at all.
        return Err(ValueError::Unusable {
            field: Field::SampleRate,
        });
    }
    let hz = if significand == 0 {
        0
    } else if let Some(right) = POINT.checked_sub(exponent) {
        significand.checked_shr(u32::from(right)).unwrap_or(0)
    } else {
        let left = u32::from(exponent.saturating_sub(POINT));
        match significand.checked_shl(left) {
            Some(shifted) if left <= significand.leading_zeros() => shifted,
            _ => u64::MAX,
        }
    };
    u32::try_from(hz)
        .map_err(|_| ValueError::OutOfRange {
            field: Field::SampleRate,
            value: hz,
        })
        .and_then(SampleRate::new)
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test oracles and generators work with small, bounded values"
)]
mod tests {
    use super::super::riff::tests::{chunks, on_small_stack, record};
    use super::*;
    use crate::parse::{LimitKind, ReadRequest};
    use crate::problem::Arg;
    use gunmetal_testkit::riff::{aiff, comm, comm_c, extended, ssnd};
    use proptest::collection::vec;
    use proptest::prelude::*;

    /// The length of `file`.
    fn len(file: &[u8]) -> u64 {
        u64::try_from(file.len()).unwrap()
    }

    /// Parses `file` under `limits` and `budget`, with every window clipped
    /// at `cut`, returning the result and the requests made.
    fn run_under(
        file: &[u8],
        cut: u64,
        limits: &Limits,
        budget: Budget,
    ) -> (Result<AiffFile, AiffError>, Vec<ReadRequest>) {
        record(Aiff::new(limits, budget), file, cut, limits)
    }

    /// The budget the parser documents as always enough for `file`.
    fn enough(file: &[u8]) -> Budget {
        Budget::for_input(len(file), STEPS_PER_OCTET, STEPS_FIXED)
    }

    /// Parses all of `file` under the default limits and the documented
    /// budget.
    fn parse(file: &[u8]) -> Result<AiffFile, AiffError> {
        run_under(file, u64::MAX, &Limits::DEFAULT, enough(file)).0
    }

    /// Parses `file` as if it shrank to `cut` octets while being read.
    fn parse_shrinking(file: &[u8], cut: u64) -> Result<AiffFile, AiffError> {
        run_under(file, cut, &Limits::DEFAULT, enough(file)).0
    }

    fn channels(count: u32) -> Channels {
        Channels::new(count).unwrap()
    }

    fn rate(hz: u32) -> SampleRate {
        SampleRate::new(hz).unwrap()
    }

    /// A depth of `count` bits, which the test knows is one.
    fn bits(count: u32) -> Option<BitDepth> {
        let depth = BitDepth::new(count).ok();
        assert!(depth.is_some(), "{count} bits is no depth");
        depth
    }

    fn range(offset: u64, len: u64) -> ByteRange {
        ByteRange { offset, len }
    }

    fn truncated(offset: u64, needed: u64, available: u64) -> ParseFault {
        ParseFault::Truncated {
            offset,
            needed,
            available,
        }
    }

    /// The format of three 8-bit mono frames at 8 kHz.
    fn mono_8_bit() -> AiffFormat {
        AiffFormat {
            compression: None,
            channels: channels(1),
            sample_frames: 3,
            sample_size: bits(8),
            sample_rate: rate(8_000),
        }
    }

    /// An AIFF file of `format` and `sound` in blocks of `block_size`,
    /// found whole.
    fn found(format: AiffFormat, sound: ByteRange, block_size: u32) -> AiffFile {
        AiffFile {
            format,
            sound,
            block_size,
            id3: None,
            stopped: None,
        }
    }

    /// The file ffmpeg writes for three 8-bit mono frames, 58 octets.
    fn minimal() -> Vec<u8> {
        aiff(
            AIFF,
            &chunks(|form| {
                form.aiff_chunk(COMM, &comm(1, 3, 8, 8_000))
                    .aiff_chunk(SSND, &ssnd(0, 0, &[0x00, 0x01, 0xFF]));
            }),
        )
    }

    #[test]
    fn reads_the_format_and_samples_of_a_minimal_file() {
        let file = minimal();
        let (result, requests) = run_under(&file, u64::MAX, &Limits::DEFAULT, enough(&file));
        assert_eq!(result, Ok(found(mono_8_bit(), range(54, 3), 0)));
        assert_eq!(
            requests,
            [
                ReadRequest { offset: 0, len: 12 },
                ReadRequest {
                    offset: 12,
                    len: 30
                },
                ReadRequest {
                    offset: 38,
                    len: 20
                },
            ]
        );
    }

    #[test]
    fn reads_the_compression_type_of_an_aiff_c_file() {
        // Little-endian samples, after Apple's version chunk.
        let file = aiff(
            AIFC,
            &chunks(|form| {
                form.aiff_chunk(COMM, &comm_c(2, 1, 16, 44_100, *b"sowt", b""))
                    .aiff_chunk(*b"FVER", &[0xA2, 0x80, 0x51, 0x40])
                    .aiff_chunk(SSND, &ssnd(0, 0, &[1, 2, 3, 4]));
            }),
        );
        assert_eq!(
            parse(&file),
            Ok(found(
                AiffFormat {
                    compression: Some(*b"sowt"),
                    channels: channels(2),
                    sample_frames: 1,
                    sample_size: bits(16),
                    sample_rate: rate(44_100),
                },
                range(72, 4),
                0
            ))
        );
    }

    #[test]
    fn starts_the_samples_after_the_offset_the_sound_chunk_gives() {
        let file = aiff(
            AIFF,
            &chunks(|form| {
                form.aiff_chunk(COMM, &comm(1, 3, 8, 8_000))
                    .aiff_chunk(SSND, &ssnd(4, 4_096, &[1, 2, 3]));
            }),
        );
        assert_eq!(parse(&file), Ok(found(mono_8_bit(), range(58, 3), 4_096)));
        // An offset that leaves no samples is not an error.
        let file = aiff(
            AIFF,
            &chunks(|form| {
                form.aiff_chunk(COMM, &comm(1, 0, 8, 8_000))
                    .aiff_chunk(SSND, &ssnd(3, 0, &[]));
            }),
        );
        let empty = AiffFormat {
            sample_frames: 0,
            ..mono_8_bit()
        };
        assert_eq!(parse(&file), Ok(found(empty, range(57, 0), 0)));
    }

    #[test]
    fn refuses_samples_that_would_start_after_the_sound_chunk() {
        let file = aiff(
            AIFF,
            &chunks(|form| {
                form.aiff_chunk(COMM, &comm(1, 3, 8, 8_000))
                    .aiff_chunk(SSND, &[0, 0, 0, 4, 0, 0, 0, 0]);
            }),
        );
        assert_eq!(
            parse(&file),
            Err(AiffError::SoundOffset {
                offset: 38,
                sound_offset: 4,
                len: 8,
            })
        );
        let file = aiff(
            AIFF,
            &chunks(|form| {
                form.aiff_chunk(COMM, &comm(1, 3, 8, 8_000))
                    .aiff_chunk(SSND, &[0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0]);
            }),
        );
        assert_eq!(
            parse(&file),
            Err(AiffError::SoundOffset {
                offset: 38,
                sound_offset: u32::MAX,
                len: 8,
            })
        );
    }

    /// The sample rate of an AIFF file whose rate is written as `octets`.
    fn with_rate(octets: [u8; 10]) -> Result<SampleRate, AiffError> {
        let common = chunks(|common| {
            common.u16_be(1).u32_be(0).u16_be(16).bytes(&octets);
        });
        let file = aiff(
            AIFF,
            &chunks(|form| {
                form.aiff_chunk(COMM, &common)
                    .aiff_chunk(SSND, &ssnd(0, 0, &[]));
            }),
        );
        parse(&file).map(|file| file.format.sample_rate)
    }

    fn value_error(error: ValueError) -> AiffError {
        AiffError::Value { offset: 12, error }
    }

    fn out_of_range(value: u64) -> AiffError {
        value_error(ValueError::OutOfRange {
            field: Field::SampleRate,
            value,
        })
    }

    /// Verifies: SEC-MED-004, SEC-MED-014
    #[test]
    fn converts_extended_sample_rates_with_checked_arithmetic() {
        let unusable = value_error(ValueError::Unusable {
            field: Field::SampleRate,
        });
        let cases: [([u8; 10], Result<SampleRate, AiffError>); 8] = [
            (extended(44_100), Ok(rate(44_100))),
            (extended(768_000), Ok(rate(768_000))),
            (extended(768_001), Err(out_of_range(768_001))),
            // Zero, and the negative zero.
            ([0; 10], Err(out_of_range(0))),
            ([0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0], Err(unusable)),
            // An infinity, a quiet NaN, and minus 44,100.
            ([0x7F, 0xFF, 0x80, 0, 0, 0, 0, 0, 0, 0], Err(unusable)),
            ([0x7F, 0xFF, 0xC0, 0, 0, 0, 0, 0, 0, 0], Err(unusable)),
            ([0xC0, 0x0E, 0xAC, 0x44, 0, 0, 0, 0, 0, 0], Err(unusable)),
        ];
        for (octets, expected) in cases {
            assert_eq!(with_rate(octets), expected, "{octets:02X?}");
        }
    }

    /// Verifies: SEC-MED-004, SEC-MED-014
    #[test]
    fn rounds_a_fractional_rate_down_and_saturates_a_huge_one() {
        // The classic Macintosh rate of 22,254.545 Hz, as Sound Manager
        // wrote it, and half of it.
        let mac = [0xAD, 0xDD, 0x17, 0x45, 0xD1, 0x45, 0xD1, 0x74];
        let at = |exponent: u16, significand: [u8; 8]| {
            let mut octets = [0; 10];
            octets[..2].copy_from_slice(&exponent.to_be_bytes());
            octets[2..].copy_from_slice(&significand);
            extended_rate(octets)
        };
        let out_of_range = |value| {
            Err(ValueError::OutOfRange {
                field: Field::SampleRate,
                value,
            })
        };
        assert_eq!(at(0x400D, mac), Ok(rate(22_254)));
        assert_eq!(at(0x400C, mac), Ok(rate(11_127)));
        let one = 1_u64.to_be_bytes();
        // At the point, the significand is the value.
        assert_eq!(at(POINT, one), Ok(rate(1)));
        assert_eq!(
            at(POINT, (1_u64 << 40).to_be_bytes()),
            out_of_range(1 << 40)
        );
        // Above it, an unnormalised significand is shifted left: exactly
        // to the top bit, and past it.
        assert_eq!(at(POINT + 1, 3_u64.to_be_bytes()), Ok(rate(6)));
        assert_eq!(at(POINT + 63, one), out_of_range(1 << 63));
        assert_eq!(at(POINT + 64, one), out_of_range(u64::MAX));
        assert_eq!(at(0x7FFE, one), out_of_range(u64::MAX));
        assert_eq!(at(0x7FFE, [0; 8]), out_of_range(0));
        // Below it, the fraction goes: a denormal is zero.
        assert_eq!(at(POINT - 63, (1_u64 << 63).to_be_bytes()), Ok(rate(1)));
        assert_eq!(at(POINT - 64, u64::MAX.to_be_bytes()), out_of_range(0));
        assert_eq!(at(0, one), out_of_range(0));
    }

    /// An independent reading of an extended number as a whole number of
    /// hertz: double or halve the significand one step at a time.
    fn oracle(octets: [u8; 10]) -> Result<SampleRate, ValueError> {
        let sign_and_exponent = u16::from_be_bytes([octets[0], octets[1]]);
        let exponent = i32::from(sign_and_exponent & 0x7FFF);
        if sign_and_exponent >= 0x8000 || exponent == 0x7FFF {
            return Err(ValueError::Unusable {
                field: Field::SampleRate,
            });
        }
        let mut value = u128::from(u64::from_be_bytes(octets[2..].try_into().unwrap()));
        let mut power = exponent - 16_383 - 63;
        while power > 0 && value != 0 && value <= u128::from(u64::MAX) {
            value *= 2;
            power -= 1;
        }
        while power < 0 && value != 0 {
            value /= 2;
            power += 1;
        }
        let hz = u64::try_from(value).unwrap_or(u64::MAX);
        if (1..=768_000).contains(&hz) {
            Ok(rate(u32::try_from(hz).unwrap()))
        } else {
            Err(ValueError::OutOfRange {
                field: Field::SampleRate,
                value: hz,
            })
        }
    }

    #[test]
    fn the_oracle_agrees_on_each_kind_of_extended_number() {
        let cases = [
            extended(44_100),
            [0; 10],
            [0x7F, 0xFF, 0x80, 0, 0, 0, 0, 0, 0, 0],
            [0xC0, 0x0E, 0xAC, 0x44, 0, 0, 0, 0, 0, 0],
            [0x40, 0x3F, 0, 0, 0, 0, 0, 0, 0, 3],
            [0x40, 0x5F, 0x80, 0, 0, 0, 0, 0, 0, 0],
        ];
        let expected = [
            Ok(rate(44_100)),
            Err(ValueError::OutOfRange {
                field: Field::SampleRate,
                value: 0,
            }),
            Err(ValueError::Unusable {
                field: Field::SampleRate,
            }),
            Err(ValueError::Unusable {
                field: Field::SampleRate,
            }),
            Ok(rate(6)),
            Err(ValueError::OutOfRange {
                field: Field::SampleRate,
                value: u64::MAX,
            }),
        ];
        for (octets, expected) in cases.into_iter().zip(expected) {
            assert_eq!(oracle(octets), expected, "{octets:02X?}");
            assert_eq!(extended_rate(octets), expected, "{octets:02X?}");
        }
    }

    /// Extended numbers: mostly near the rates files hold, sometimes any
    /// exponent or sign at all.
    fn any_extended() -> impl Strategy<Value = [u8; 10]> {
        (
            prop_oneof![
                3 => 16_380_u16..16_520,
                1 => 0_u16..0x8000,
                1 => any::<u16>(),
                1 => Just(0x7FFF_u16),
            ],
            prop_oneof![any::<u64>(), 0_u64..1_000, Just(0)],
        )
            .prop_map(|(sign_and_exponent, significand)| {
                let mut octets = [0; 10];
                octets[..2].copy_from_slice(&sign_and_exponent.to_be_bytes());
                octets[2..].copy_from_slice(&significand.to_be_bytes());
                octets
            })
    }

    #[test]
    fn refuses_a_file_that_is_not_an_aiff_form() {
        let mut file = minimal();
        file[..4].copy_from_slice(b"RIFF");
        assert_eq!(parse(&file), Err(AiffError::NotForm { found: *b"RIFF" }));
        let mut file = minimal();
        file[8..12].copy_from_slice(b"8SVX");
        assert_eq!(parse(&file), Err(AiffError::NotAiff { found: *b"8SVX" }));
    }

    #[test]
    fn refuses_a_chunk_shorter_than_its_fields() {
        let short = |id, offset, len, needed| {
            Err(AiffError::Short {
                id,
                offset,
                len,
                needed,
            })
        };
        let with_common = |form_type, common: &[u8]| {
            aiff(
                form_type,
                &chunks(|form| {
                    form.aiff_chunk(COMM, common)
                        .aiff_chunk(SSND, &ssnd(0, 0, &[]));
                }),
            )
        };
        let plain = comm(1, 3, 8, 8_000);
        assert_eq!(
            parse(&with_common(AIFF, &plain[..17])),
            short(COMM, 12, 17, 18)
        );
        assert_eq!(parse(&with_common(AIFC, &plain)), short(COMM, 12, 18, 22));
        let compressed = comm_c(1, 3, 8, 8_000, *b"NONE", b"");
        assert_eq!(
            parse(&with_common(AIFC, &compressed[..21])),
            short(COMM, 12, 21, 22)
        );
        // An AIFF-C chunk without its name's count is long enough.
        assert_eq!(
            parse(&with_common(AIFC, &compressed[..22])),
            Ok(found(
                AiffFormat {
                    compression: Some(*b"NONE"),
                    ..mono_8_bit()
                },
                range(58, 0),
                0
            ))
        );
        let file = aiff(
            AIFF,
            &chunks(|form| {
                form.aiff_chunk(COMM, &plain).aiff_chunk(SSND, &[0; 7]);
            }),
        );
        assert_eq!(parse(&file), short(SSND, 38, 7, 8));
    }

    /// Verifies: SEC-MED-014
    #[test]
    fn refuses_a_channel_count_out_of_range() {
        let file = aiff(
            AIFF,
            &chunks(|form| {
                form.aiff_chunk(COMM, &comm(0, 3, 8, 8_000))
                    .aiff_chunk(SSND, &ssnd(0, 0, &[]));
            }),
        );
        assert_eq!(
            parse(&file),
            Err(value_error(ValueError::OutOfRange {
                field: Field::Channels,
                value: 0,
            }))
        );
        // A sample size outside 1 to 64 is recorded as unknown.
        let file = aiff(
            AIFF,
            &chunks(|form| {
                form.aiff_chunk(COMM, &comm(1, 3, 0, 8_000))
                    .aiff_chunk(SSND, &ssnd(0, 0, &[]));
            }),
        );
        let format = AiffFormat {
            sample_size: None,
            ..mono_8_bit()
        };
        assert_eq!(parse(&file), Ok(found(format, range(54, 0), 0)));
    }

    #[test]
    fn a_playable_file_needs_its_format_and_its_samples() {
        let only_sound = aiff(
            AIFF,
            &chunks(|form| {
                form.aiff_chunk(SSND, &ssnd(0, 0, &[1]));
            }),
        );
        assert_eq!(parse(&only_sound), Err(AiffError::Missing { id: COMM }));
        let only_format = aiff(
            AIFF,
            &chunks(|form| {
                form.aiff_chunk(COMM, &comm(1, 0, 8, 8_000));
            }),
        );
        assert_eq!(parse(&only_format), Err(AiffError::Missing { id: SSND }));
    }

    #[test]
    fn records_the_first_id3_tag_and_keeps_the_first_of_each_chunk() {
        let file = aiff(
            AIFF,
            &chunks(|form| {
                form.aiff_chunk(ID3, b"ID3\x04")
                    .aiff_chunk(ID3_LOWER, b"ID3\x03")
                    .aiff_chunk(COMM, &comm(1, 3, 8, 8_000))
                    .aiff_chunk(SSND, &ssnd(0, 0, &[1, 2, 3]))
                    .aiff_chunk(COMM, &comm(2, 1, 16, 44_100))
                    .aiff_chunk(SSND, &ssnd(0, 7, &[4]));
            }),
        );
        assert_eq!(
            parse(&file),
            Ok(AiffFile {
                id3: Some(range(20, 4)),
                ..found(mono_8_bit(), range(78, 3), 0)
            })
        );
        let file = aiff(
            AIFF,
            &chunks(|form| {
                form.aiff_chunk(ID3_LOWER, b"ID3")
                    .aiff_chunk(COMM, &comm(1, 3, 8, 8_000))
                    .aiff_chunk(SSND, &ssnd(0, 0, &[]));
            }),
        );
        assert_eq!(parse(&file).map(|file| file.id3), Ok(Some(range(20, 3))));
    }

    /// Verifies: SEC-MED-001, SEC-TM-032
    #[test]
    fn keeps_samples_cut_short_and_says_where_the_walk_stopped() {
        let file = minimal();
        assert_eq!(
            parse(&file[..55]),
            Ok(AiffFile {
                stopped: Some(truncated(46, 11, 9)),
                ..found(mono_8_bit(), range(54, 1), 0)
            })
        );
        // Cut inside the gap the offset leaves, no samples are left, and
        // the empty range sits at the end of the file, not past it.
        let file = aiff(
            AIFF,
            &chunks(|form| {
                form.aiff_chunk(COMM, &comm(1, 3, 8, 8_000))
                    .aiff_chunk(SSND, &ssnd(4, 0, &[1, 2]));
            }),
        );
        assert_eq!(
            parse(&file[..56]),
            Ok(AiffFile {
                stopped: Some(truncated(46, 14, 10)),
                ..found(mono_8_bit(), range(56, 0), 0)
            })
        );
        // The same at the very start of the gap.
        assert_eq!(
            parse(&file[..54]),
            Ok(AiffFile {
                stopped: Some(truncated(46, 14, 8)),
                ..found(mono_8_bit(), range(54, 0), 0)
            })
        );
        // Cut inside the sound chunk's fields, there are none to keep.
        assert_eq!(
            parse(&minimal()[..50]),
            Err(AiffError::Fault(truncated(46, 8, 4)))
        );
        // A chunk that runs past the end of the form is not recorded.
        for id in [ID3, *b"ANNO"] {
            let mut file = minimal();
            file.extend(chunks(|tail| {
                tail.bytes(&id).u32_be(16).bytes(b"ID3\x04");
            }));
            file[7] += 12;
            assert_eq!(
                parse(&file),
                Ok(AiffFile {
                    stopped: Some(truncated(66, 16, 4)),
                    ..found(mono_8_bit(), range(54, 3), 0)
                }),
                "{id:?}"
            );
        }
        // A format chunk cut short leaves no format.
        assert_eq!(
            parse(&minimal()[..30]),
            Err(AiffError::Fault(truncated(20, 18, 10)))
        );
    }

    /// Verifies: SEC-MED-001
    #[test]
    fn reports_reads_cut_short_by_a_file_that_shrinks() {
        assert_eq!(
            parse_shrinking(&minimal(), 25),
            Err(AiffError::Fault(truncated(20, 18, 5)))
        );
        let file = aiff(
            AIFC,
            &chunks(|form| {
                form.aiff_chunk(COMM, &comm_c(1, 3, 8, 8_000, *b"NONE", b""))
                    .aiff_chunk(SSND, &ssnd(0, 0, &[]));
            }),
        );
        assert_eq!(
            parse_shrinking(&file, 40),
            Err(AiffError::Fault(truncated(38, 4, 2)))
        );
    }

    /// Verifies: SEC-MED-005, SEC-MED-006, SEC-MED-007, SEC-TM-032
    #[test]
    fn walks_under_the_same_limits_as_a_wav_file() {
        let file = minimal();
        let lowered = |kind, value| Limits::DEFAULT.with_override(kind, value).unwrap();
        let depth = lowered(LimitKind::ContainerDepth, 0);
        assert_eq!(
            run_under(&file, u64::MAX, &depth, enough(&file)).0,
            Err(AiffError::Fault(ParseFault::TooDeep {
                limit: LimitKind::ContainerDepth,
                depth: 1,
                max: 0,
                offset: 12,
            }))
        );
        let children = lowered(LimitKind::Children, 1);
        assert_eq!(
            run_under(&file, u64::MAX, &children, enough(&file)).0,
            Err(AiffError::Fault(ParseFault::LimitExceeded {
                limit: LimitKind::Children,
                value: 2,
                max: 1,
                offset: 38,
            }))
        );
        assert_eq!(
            run_under(
                &file,
                u64::MAX,
                &Limits::DEFAULT,
                Budget::for_input(0, 0, 1)
            )
            .0,
            Err(AiffError::Fault(ParseFault::BudgetExceeded { offset: 38 }))
        );
    }

    #[test]
    fn describes_every_error_as_an_unreadable_aiff_file() {
        let cases = [
            (AiffError::Fault(truncated(46, 8, 4)), "truncated", Some(46)),
            (
                AiffError::Fault(ParseFault::BudgetExceeded { offset: 38 }),
                "budget_exceeded",
                Some(38),
            ),
            (AiffError::NotForm { found: *b"RIFF" }, "not_form", Some(0)),
            (AiffError::NotAiff { found: *b"8SVX" }, "not_aiff", Some(8)),
            (
                AiffError::Short {
                    id: SSND,
                    offset: 38,
                    len: 7,
                    needed: 8,
                },
                "short_chunk",
                Some(38),
            ),
            (out_of_range(0), "bad_value", Some(12)),
            (
                AiffError::SoundOffset {
                    offset: 50,
                    sound_offset: 4,
                    len: 8,
                },
                "sound_offset",
                Some(50),
            ),
            (AiffError::Missing { id: COMM }, "missing_chunk", None),
        ];
        for (error, reason, offset) in cases {
            let mut args = vec![("reason", Arg::Name(reason))];
            args.extend(offset.map(|offset| ("offset", Arg::Number(offset))));
            assert_eq!(
                error.problem(),
                Problem {
                    code: ProblemCode::AiffUnreadable,
                    args,
                },
                "{error:?}"
            );
        }
    }

    /// One chunk of a generated AIFF file.
    #[derive(Debug, Clone)]
    enum Piece {
        /// A `COMM` chunk.
        Common {
            channels: u16,
            frames: u32,
            size: u16,
            rate: u32,
        },
        /// An `SSND` chunk.
        Sound {
            offset: u8,
            block_size: u32,
            samples: Vec<u8>,
        },
        /// An `ID3 ` or `id3 ` chunk.
        Id3 { lower: bool, body: Vec<u8> },
        /// A chunk this parser does not read.
        Junk(Vec<u8>),
    }

    impl Piece {
        /// The chunk's ID and body, in an AIFF-C form when `compressed`.
        fn encode(&self, compressed: bool) -> ([u8; 4], Vec<u8>) {
            match self {
                &Self::Common {
                    channels,
                    frames,
                    size,
                    rate,
                } if compressed => (
                    COMM,
                    comm_c(channels, frames, size, rate, *b"fl32", b"32-bit float"),
                ),
                &Self::Common {
                    channels,
                    frames,
                    size,
                    rate,
                } => (COMM, comm(channels, frames, size, rate)),
                Self::Sound {
                    offset,
                    block_size,
                    samples,
                } => (SSND, ssnd(u32::from(*offset), *block_size, samples)),
                Self::Id3 { lower, body } => (if *lower { ID3_LOWER } else { ID3 }, body.clone()),
                Self::Junk(body) => (*b"ANNO", body.clone()),
            }
        }
    }

    fn common_piece() -> impl Strategy<Value = Piece> {
        (1_u16..=8, any::<u32>(), 1_u16..=32, 1_u32..=192_000).prop_map(
            |(channels, frames, size, rate)| Piece::Common {
                channels,
                frames,
                size,
                rate,
            },
        )
    }

    fn sound_piece() -> impl Strategy<Value = Piece> {
        (0_u8..4, any::<u32>(), vec(any::<u8>(), 0..16)).prop_map(
            |(offset, block_size, samples)| Piece::Sound {
                offset,
                block_size,
                samples,
            },
        )
    }

    fn piece() -> impl Strategy<Value = Piece> {
        prop_oneof![
            common_piece(),
            sound_piece(),
            (any::<bool>(), vec(any::<u8>(), 0..12))
                .prop_map(|(lower, body)| Piece::Id3 { lower, body }),
            vec(any::<u8>(), 0..12).prop_map(Piece::Junk),
        ]
    }

    /// The AIFF or AIFF-C file of `pieces`.
    fn encode(compressed: bool, pieces: &[Piece]) -> Vec<u8> {
        let form_type = if compressed { AIFC } else { AIFF };
        aiff(
            form_type,
            &chunks(|form| {
                for piece in pieces {
                    let (id, body) = piece.encode(compressed);
                    form.aiff_chunk(id, &body);
                }
            }),
        )
    }

    /// An independent model of the parse of a whole file of `pieces`: it
    /// lays the chunks out and keeps the first of each kind.
    fn model(compressed: bool, pieces: &[Piece]) -> Result<AiffFile, AiffError> {
        let mut at = 12_u64;
        let (mut format, mut sound, mut id3) = (None, None, None);
        for piece in pieces {
            let body = at + 8;
            let len = len(&piece.encode(compressed).1);
            match piece {
                &Piece::Common {
                    channels: count,
                    frames,
                    size,
                    rate: hz,
                } if format.is_none() => {
                    format = Some(AiffFormat {
                        compression: compressed.then_some(*b"fl32"),
                        channels: channels(u32::from(count)),
                        sample_frames: frames,
                        sample_size: bits(u32::from(size)),
                        sample_rate: rate(hz),
                    });
                }
                Piece::Sound {
                    offset, block_size, ..
                } if sound.is_none() => {
                    let skip = 8 + u64::from(*offset);
                    sound = Some((range(body + skip, len - skip), *block_size));
                }
                Piece::Id3 { .. } if id3.is_none() => id3 = Some(range(body, len)),
                _ => {}
            }
            at = body + len + len % 2;
        }
        match (format, sound) {
            (None, _) => Err(AiffError::Missing { id: COMM }),
            (_, None) => Err(AiffError::Missing { id: SSND }),
            (Some(format), Some((sound, block_size))) => Ok(AiffFile {
                id3,
                ..found(format, sound, block_size)
            }),
        }
    }

    #[test]
    fn the_model_agrees_with_the_parser_on_each_kind_of_chunk() {
        let pieces = [
            Piece::Junk(vec![1]),
            Piece::Id3 {
                lower: true,
                body: vec![2, 3],
            },
            Piece::Common {
                channels: 2,
                frames: 9,
                size: 24,
                rate: 96_000,
            },
            Piece::Sound {
                offset: 2,
                block_size: 512,
                samples: vec![4, 5, 6],
            },
            Piece::Id3 {
                lower: false,
                body: vec![],
            },
        ];
        let format = |compression| AiffFormat {
            compression,
            channels: channels(2),
            sample_frames: 9,
            sample_size: bits(24),
            sample_rate: rate(96_000),
        };
        let plain = Ok(AiffFile {
            id3: Some(range(30, 2)),
            ..found(format(None), range(76, 3), 512)
        });
        assert_eq!(model(false, &pieces), plain);
        assert_eq!(parse(&encode(false, &pieces)), plain);
        // The AIFF-C chunk adds the type and a 14-octet name.
        let compressed = Ok(AiffFile {
            id3: Some(range(30, 2)),
            ..found(format(Some(*b"fl32")), range(94, 3), 512)
        });
        assert_eq!(model(true, &pieces), compressed);
        assert_eq!(parse(&encode(true, &pieces)), compressed);
        assert_eq!(
            model(false, &pieces[..3]),
            Err(AiffError::Missing { id: SSND })
        );
        assert_eq!(
            model(false, &pieces[3..]),
            Err(AiffError::Missing { id: COMM })
        );
    }

    /// Any octets at all: noise, noise after a form header, or a generated
    /// file with one octet changed and its end cut off.
    fn any_file() -> impl Strategy<Value = Vec<u8>> {
        prop_oneof![
            vec(any::<u8>(), 0..96),
            (
                prop::sample::select(vec![AIFF, AIFC]),
                any::<u32>(),
                vec(any::<u8>(), 0..96)
            )
                .prop_map(|(form_type, size, noise)| {
                    let mut file = chunks(|form| {
                        form.bytes(&FORM).u32_be(size).bytes(&form_type);
                    });
                    file.extend(noise);
                    file
                }),
            (
                any::<bool>(),
                vec(piece(), 0..3),
                common_piece(),
                sound_piece(),
                vec(piece(), 0..3),
                prop::option::of((any::<usize>(), any::<u8>(), any::<usize>()))
            )
                .prop_map(|(compressed, before, common, sound, after, damage)| {
                    let pieces = [before, vec![common, sound], after].concat();
                    let mut file = encode(compressed, &pieces);
                    if let Some((at, octet, cut)) = damage {
                        let at = at % file.len();
                        file[at] = octet;
                        file.truncate(cut % (file.len() + 1));
                    }
                    file
                }),
        ]
    }

    proptest! {
        /// Verifies: SEC-MED-001, SEC-MED-004, SEC-MED-014
        #[test]
        fn converts_any_extended_number_as_the_oracle_does(octets in any_extended()) {
            prop_assert_eq!(extended_rate(octets), oracle(octets));
        }

        /// Verifies: SEC-MED-001, SEC-MED-007, SEC-MED-008, SEC-TM-032, SEC-HIS-036
        #[test]
        fn returns_and_reads_only_inside_the_file_for_any_input(
            file in any_file(),
            cut in prop_oneof![Just(u64::MAX), 0_u64..128],
        ) {
            // run_under fails on a read outside the file and on more reads
            // than the file has chunks; the budget is the documented one.
            let length = len(&file);
            let (result, _) = on_small_stack(move || {
                run_under(&file, cut, &Limits::DEFAULT, enough(&file))
            });
            let inside = |range: ByteRange| range.offset + range.len <= length;
            match result {
                Ok(found) => {
                    prop_assert!(inside(found.sound), "{:?}", found);
                    prop_assert!(found.id3.is_none_or(inside), "{:?}", found);
                    prop_assert!(
                        !matches!(found.stopped, Some(ParseFault::BudgetExceeded { .. })),
                        "{:?}",
                        found
                    );
                }
                Err(error) => prop_assert!(
                    !matches!(error, AiffError::Fault(ParseFault::BudgetExceeded { .. })),
                    "{:?}",
                    error
                ),
            }
        }

        #[test]
        fn finds_the_first_of_each_chunk_wherever_it_lies(
            compressed in any::<bool>(),
            pieces in vec(piece(), 0..8),
        ) {
            prop_assert_eq!(parse(&encode(compressed, &pieces)), model(compressed, &pieces));
        }
    }
}
